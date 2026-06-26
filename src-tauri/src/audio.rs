//! Audio capture pipeline (M1).
//!
//! Flow: cpal input stream (any sample format, any channel count, any rate)
//!   -> downmix to mono f32 in the realtime callback
//!   -> worker thread: linear resample to 16 kHz mono
//!   -> energy VAD (adaptive noise floor + hangover) + RMS level metering
//!
//! The resampled 16 kHz mono i16 stream and the VAD segment boundaries are the
//! inputs the transcription providers (M2+) consume. For now the worker drives
//! the HUD level meter and logs detected speech segments.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError, Sender},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, Sample, SampleFormat, SizedSample, StreamConfig};
use tauri::AppHandle;

use crate::events::BackendEvent;
use crate::providers::SessionSink;

/// Target sample rate fed to every transcription backend.
pub const TARGET_RATE: u32 = 16_000;
/// VAD analysis frame: 20 ms @ 16 kHz.
const FRAME_SIZE: usize = (TARGET_RATE as usize / 1000) * 20;
/// Speech must exceed `noise_floor * SPEECH_FACTOR` (and an absolute floor).
const SPEECH_FACTOR: f32 = 3.0;
const ABS_THRESHOLD: f32 = 0.012;
/// Keep "in speech" for this long after energy drops, to bridge short pauses.
const HANGOVER: u32 = 12; // 12 * 20 ms = 240 ms

enum Msg {
    Config { in_rate: u32 },
    Data(Vec<f32>),
}

/// Tauri-managed handle to the running capture, if any.
#[derive(Default)]
pub struct AudioController {
    inner: Mutex<Option<Running>>,
}

struct Running {
    running: Arc<AtomicBool>,
    capture: Option<JoinHandle<()>>,
    worker: Option<JoinHandle<()>>,
}

impl AudioController {
    /// Start capturing, forwarding 16 kHz mono PCM to `sink` (the provider
    /// session) when present. Idempotent: a no-op if already running.
    pub fn start(&self, app: AppHandle, sink: Option<SessionSink>) -> Result<(), String> {
        let mut guard = self.inner.lock().unwrap();
        if guard.is_some() {
            return Ok(());
        }

        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::channel::<Msg>();

        let capture = {
            let running = running.clone();
            let app = app.clone();
            thread::spawn(move || capture_loop(tx, running, app))
        };
        let worker = {
            let running = running.clone();
            let app = app.clone();
            thread::spawn(move || worker_loop(rx, running, app, sink))
        };

        *guard = Some(Running {
            running,
            capture: Some(capture),
            worker: Some(worker),
        });
        Ok(())
    }

    /// Stop capturing and join the worker threads.
    pub fn stop(&self) {
        let running = {
            let mut guard = self.inner.lock().unwrap();
            guard.take()
        };
        if let Some(mut r) = running {
            r.running.store(false, Ordering::Relaxed);
            if let Some(h) = r.capture.take() {
                let _ = h.join();
            }
            if let Some(h) = r.worker.take() {
                let _ = h.join();
            }
        }
    }

    #[allow(dead_code)] // used by the global hotkey toggle in M4
    pub fn is_running(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }
}

fn fail(app: &AppHandle, running: &AtomicBool, msg: impl Into<String>) {
    let msg = msg.into();
    eprintln!("[transcript:audio] {msg}");
    BackendEvent::Error { message: msg }.emit(app);
    running.store(false, Ordering::Relaxed);
}

fn capture_loop(tx: Sender<Msg>, running: Arc<AtomicBool>, app: AppHandle) {
    let host = cpal::default_host();
    let Some(device) = host.default_input_device() else {
        fail(&app, &running, "Aucun micro détecté (périphérique d'entrée).");
        return;
    };
    let supported = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            fail(&app, &running, format!("Config micro indisponible: {e}"));
            return;
        }
    };

    let sample_format = supported.sample_format();
    let channels = supported.channels() as usize;
    let in_rate = supported.sample_rate();
    let config: StreamConfig = supported.config();

    eprintln!("[transcript:audio] rate={in_rate} ch={channels} fmt={sample_format:?}");

    let _ = tx.send(Msg::Config { in_rate });

    let stream = match build_stream(&device, &config, sample_format, channels, tx) {
        Ok(s) => s,
        Err(e) => {
            fail(&app, &running, format!("Ouverture du flux audio: {e}"));
            return;
        }
    };
    if let Err(e) = stream.play() {
        fail(&app, &running, format!("Démarrage du flux audio: {e}"));
        return;
    }

    BackendEvent::State { state: "listening" }.emit(&app);

    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(50));
    }
    drop(stream); // stops capture, drops the callback's Sender
}

fn build_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    channels: usize,
    tx: Sender<Msg>,
) -> Result<cpal::Stream, String> {
    let err_fn = |e| eprintln!("[transcript:audio] stream error: {e}");
    let res = match sample_format {
        SampleFormat::F32 => {
            let tx = tx.clone();
            device.build_input_stream(
                config,
                move |data: &[f32], _| send_mono(data, channels, &tx),
                err_fn,
                None,
            )
        }
        SampleFormat::I16 => {
            let tx = tx.clone();
            device.build_input_stream(
                config,
                move |data: &[i16], _| send_mono(data, channels, &tx),
                err_fn,
                None,
            )
        }
        SampleFormat::U16 => {
            let tx = tx.clone();
            device.build_input_stream(
                config,
                move |data: &[u16], _| send_mono(data, channels, &tx),
                err_fn,
                None,
            )
        }
        other => return Err(format!("format d'échantillon non supporté: {other:?}")),
    };
    res.map_err(|e| e.to_string())
}

/// Downmix an interleaved frame to mono f32 and forward it to the worker.
fn send_mono<T>(data: &[T], channels: usize, tx: &Sender<Msg>)
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    if channels == 0 {
        return;
    }
    let mut mono = Vec::with_capacity(data.len() / channels);
    for frame in data.chunks(channels) {
        let mut sum = 0.0f32;
        for &s in frame {
            sum += f32::from_sample(s);
        }
        mono.push(sum / channels as f32);
    }
    let _ = tx.send(Msg::Data(mono));
}

fn worker_loop(
    rx: Receiver<Msg>,
    running: Arc<AtomicBool>,
    app: AppHandle,
    sink: Option<SessionSink>,
) {
    let mut pipeline: Option<Pipeline> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Msg::Config { in_rate }) => {
                pipeline = Some(Pipeline::new(app.clone(), in_rate, sink.clone()));
            }
            Ok(Msg::Data(mono)) => {
                if let Some(p) = pipeline.as_mut() {
                    p.process(&mono);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if !running.load(Ordering::Relaxed) {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    if let Some(mut p) = pipeline {
        p.finish();
    }
    // Note: the "idle" state is emitted by the provider session once it has
    // fully finalized (so the UI saves the complete transcript to history).
}

/// Streaming linear resampler (in_rate -> 16 kHz), carrying state across chunks.
struct LinearResampler {
    step: f64, // input samples consumed per output sample
    buf: Vec<f32>,
    read_pos: f64,
}

impl LinearResampler {
    fn new(in_rate: u32) -> Self {
        Self {
            step: in_rate as f64 / TARGET_RATE as f64,
            buf: Vec::new(),
            read_pos: 0.0,
        }
    }

    fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        self.buf.extend_from_slice(input);
        while (self.read_pos as usize) + 1 < self.buf.len() {
            let idx = self.read_pos as usize;
            let frac = self.read_pos - idx as f64;
            let a = self.buf[idx] as f64;
            let b = self.buf[idx + 1] as f64;
            out.push((a * (1.0 - frac) + b * frac) as f32);
            self.read_pos += self.step;
        }
        let consumed = self.read_pos as usize;
        if consumed > 0 {
            self.buf.drain(0..consumed);
            self.read_pos -= consumed as f64;
        }
    }
}

struct Pipeline {
    app: AppHandle,
    sink: Option<SessionSink>,
    resampler: LinearResampler,
    acc: Vec<f32>,
    noise_floor: f32,
    in_speech: bool,
    hangover: u32,
    seg_samples: usize,
    resampled: Vec<f32>,
    level_peak: f32,
    last_level: Instant,
}

impl Pipeline {
    fn new(app: AppHandle, in_rate: u32, sink: Option<SessionSink>) -> Self {
        Self {
            app,
            sink,
            resampler: LinearResampler::new(in_rate),
            acc: Vec::with_capacity(FRAME_SIZE * 2),
            noise_floor: ABS_THRESHOLD,
            in_speech: false,
            hangover: 0,
            seg_samples: 0,
            resampled: Vec::with_capacity(4096),
            level_peak: 0.0,
            last_level: Instant::now(),
        }
    }

    fn process(&mut self, mono: &[f32]) {
        self.resampled.clear();
        self.resampler.process(mono, &mut self.resampled);
        // borrow-safe: move out the freshly resampled samples
        let chunk = std::mem::take(&mut self.resampled);

        // Stream the full 16 kHz mono signal to the provider (it does its own
        // endpointing); our VAD below only drives the meter / segment logs.
        if let Some(sink) = &self.sink {
            let pcm: Vec<i16> = chunk
                .iter()
                .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect();
            sink.push(&pcm);
        }

        self.acc.extend_from_slice(&chunk);
        self.resampled = chunk;

        while self.acc.len() >= FRAME_SIZE {
            let frame: Vec<f32> = self.acc.drain(0..FRAME_SIZE).collect();
            self.analyze_frame(&frame);
        }
    }

    fn analyze_frame(&mut self, frame: &[f32]) {
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();

        if !self.in_speech {
            // Track the ambient noise floor only while silent.
            self.noise_floor = 0.97 * self.noise_floor + 0.03 * rms;
        }
        let threshold = (self.noise_floor * SPEECH_FACTOR).max(ABS_THRESHOLD);
        let voiced = rms > threshold;

        match (self.in_speech, voiced) {
            (false, true) => {
                self.in_speech = true;
                self.hangover = HANGOVER;
                self.seg_samples = frame.len();
            }
            (true, true) => {
                self.hangover = HANGOVER;
                self.seg_samples += frame.len();
            }
            (true, false) => {
                self.seg_samples += frame.len();
                if self.hangover == 0 {
                    let ms = self.seg_samples as f32 / TARGET_RATE as f32 * 1000.0;
                    eprintln!("[transcript:audio] segment de parole: {ms:.0} ms");
                    self.in_speech = false;
                    self.seg_samples = 0;
                } else {
                    self.hangover -= 1;
                }
            }
            (false, false) => {}
        }

        // Level meter: normalized 0..1, emitted ~every 80 ms (peak-hold).
        self.level_peak = self.level_peak.max((rms * 6.0).min(1.0));
        if self.last_level.elapsed() >= Duration::from_millis(80) {
            BackendEvent::Level {
                rms: self.level_peak,
            }
            .emit(&self.app);
            self.level_peak = 0.0;
            self.last_level = Instant::now();
        }
    }

    fn finish(&mut self) {
        if self.in_speech && self.seg_samples > 0 {
            let ms = self.seg_samples as f32 / TARGET_RATE as f32 * 1000.0;
            eprintln!("[transcript:audio] segment final: {ms:.0} ms");
        }
        if let Some(sink) = &self.sink {
            sink.eos();
        }
        BackendEvent::Level { rms: 0.0 }.emit(&self.app);
    }
}
