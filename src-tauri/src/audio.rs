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

use std::str::FromStr;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, DeviceId, FromSample, Sample, SampleFormat, SizedSample, StreamConfig};
use serde::Serialize;
use tauri::AppHandle;

use crate::events::BackendEvent;
use crate::providers::SessionSink;

/// A selectable microphone (input device) surfaced to the settings UI.
#[derive(Serialize, Clone, Debug)]
pub struct AudioInput {
    /// Stable cpal device id (`DeviceId` as string) — the value persisted in
    /// `Settings::input_device`. Stable across reboots/reconnections.
    pub id: String,
    /// Human-readable label for the picker.
    pub name: String,
    /// Whether this is the host's current default input.
    pub is_default: bool,
}

/// Best-effort human label for a device (its description name, else its id).
fn device_label(d: &Device, id_str: &str) -> String {
    d.description()
        .map(|desc| desc.name().to_string())
        .unwrap_or_else(|_| id_str.to_string())
}

/// Enumerate available input devices (deduplicated by id), default first.
/// Best-effort: returns an empty list if the host can't be queried.
pub fn list_input_devices() -> Vec<AudioInput> {
    let host = cpal::default_host();
    let default_id = host.default_input_device().and_then(|d| d.id().ok());

    let mut out: Vec<AudioInput> = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            let Ok(id) = d.id() else { continue };
            let id_str = id.to_string();
            // ALSA's null/"discard" device is not a real mic.
            if id_str == "alsa:null" {
                continue;
            }
            let name = device_label(&d, &id_str);
            // Collapse exact id/name duplicates: ALSA exposes the same card
            // under several nodes (sysdefault/front/hw/plughw) with one label —
            // keep the first (most format-compatible) so the list stays legible.
            if out.iter().any(|a| a.id == id_str || a.name == name) {
                continue;
            }
            let is_default = default_id.as_ref() == Some(&id);
            out.push(AudioInput {
                id: id_str,
                name,
                is_default,
            });
        }
    }
    // Surface the default first so the UI can preselect it sensibly.
    out.sort_by_key(|a| !a.is_default);
    out
}

/// Resolve the cpal device to capture from: the one whose id matches `wanted`,
/// else the system default. A previously-selected device that vanished (e.g. an
/// unplugged USB mic) thus transparently falls back to the default.
fn pick_input_device(host: &cpal::Host, wanted: Option<&str>) -> Option<Device> {
    if let Some(id_str) = wanted.filter(|s| !s.is_empty()) {
        if let Ok(id) = DeviceId::from_str(id_str) {
            if let Some(d) = host.device_by_id(&id) {
                return Some(d);
            }
        }
    }
    host.default_input_device()
}

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
    /// session) when present. `device` is the preferred microphone name (empty
    /// or unknown = system default). Idempotent: a no-op if already running.
    pub fn start(
        &self,
        app: AppHandle,
        sink: Option<SessionSink>,
        device: Option<String>,
    ) -> Result<(), String> {
        let mut guard = self.inner.lock().unwrap();
        if guard.is_some() {
            return Ok(());
        }

        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = mpsc::channel::<Msg>();

        let capture = {
            let running = running.clone();
            let app = app.clone();
            thread::spawn(move || capture_loop(tx, running, app, device))
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
    eprintln!("[sonora:audio] {msg}");
    BackendEvent::Error { message: msg }.emit(app);
    running.store(false, Ordering::Relaxed);
}

fn capture_loop(
    tx: Sender<Msg>,
    running: Arc<AtomicBool>,
    app: AppHandle,
    device_name: Option<String>,
) {
    let host = cpal::default_host();
    let Some(device) = pick_input_device(&host, device_name.as_deref()) else {
        fail(
            &app,
            &running,
            "Aucun micro détecté (périphérique d'entrée).",
        );
        return;
    };
    let picked = device
        .id()
        .map(|id| device_label(&device, &id.to_string()))
        .unwrap_or_else(|_| "?".into());
    eprintln!("[sonora:audio] micro: {picked}");
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

    eprintln!("[sonora:audio] rate={in_rate} ch={channels} fmt={sample_format:?}");

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
    let err_fn = |e| eprintln!("[sonora:audio] stream error: {e}");
    // cpal 0.18 takes the StreamConfig by value (it is Copy; was &StreamConfig
    // in 0.17), so dereference our borrowed config to pass an owned copy.
    let res = match sample_format {
        SampleFormat::F32 => {
            let tx = tx.clone();
            device.build_input_stream(
                *config,
                move |data: &[f32], _| send_mono(data, channels, &tx),
                err_fn,
                None,
            )
        }
        SampleFormat::I16 => {
            let tx = tx.clone();
            device.build_input_stream(
                *config,
                move |data: &[i16], _| send_mono(data, channels, &tx),
                err_fn,
                None,
            )
        }
        SampleFormat::U16 => {
            let tx = tx.clone();
            device.build_input_stream(
                *config,
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
        // The final step can push read_pos past the buffer end; clamp so the
        // drain range is always valid (avoids an out-of-range slice panic).
        let consumed = (self.read_pos as usize).min(self.buf.len());
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
    /// 16 kHz mono PCM of the current speech segment (for chunked providers).
    seg_buf: Vec<i16>,
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
            seg_buf: Vec::new(),
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

        // Capture this frame's PCM only when it belongs to a speech segment.
        let frame_pcm: Option<Vec<i16>> = if self.in_speech || voiced {
            Some(
                frame
                    .iter()
                    .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                    .collect(),
            )
        } else {
            None
        };

        match (self.in_speech, voiced) {
            (false, true) => {
                self.in_speech = true;
                self.hangover = HANGOVER;
                self.seg_buf.clear();
                if let Some(p) = frame_pcm {
                    self.seg_buf.extend_from_slice(&p);
                }
            }
            (true, true) => {
                self.hangover = HANGOVER;
                if let Some(p) = frame_pcm {
                    self.seg_buf.extend_from_slice(&p);
                }
            }
            (true, false) => {
                if let Some(p) = frame_pcm {
                    self.seg_buf.extend_from_slice(&p);
                }
                if self.hangover == 0 {
                    let ms = self.seg_buf.len() as f32 / TARGET_RATE as f32 * 1000.0;
                    eprintln!("[sonora:audio] segment de parole: {ms:.0} ms");
                    self.emit_segment();
                    self.in_speech = false;
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

    /// Hand the buffered speech segment to a chunked provider (no-op otherwise).
    fn emit_segment(&mut self) {
        let seg = std::mem::take(&mut self.seg_buf);
        if seg.is_empty() {
            return;
        }
        if let Some(sink) = &self.sink {
            sink.segment(seg);
        }
    }

    fn finish(&mut self) {
        if self.in_speech && !self.seg_buf.is_empty() {
            let ms = self.seg_buf.len() as f32 / TARGET_RATE as f32 * 1000.0;
            eprintln!("[sonora:audio] segment final: {ms:.0} ms");
            self.emit_segment();
        }
        if let Some(sink) = &self.sink {
            sink.eos();
        }
        BackendEvent::Level { rms: 0.0 }.emit(&self.app);
    }
}
