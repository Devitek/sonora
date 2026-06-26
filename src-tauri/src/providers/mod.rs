//! Transcription providers.
//!
//! A *session* is a live transcription connection. The audio pipeline feeds it
//! 16 kHz mono PCM through a [`SessionSink`]; the provider emits `partial` /
//! `final` events to the HUD. Each provider (Gemini Live here; Deepgram,
//! Whisper API, whisper.cpp local in M5) implements its own `run_session`.

pub mod gemini;

use std::sync::Mutex;

use tauri::AppHandle;
use tokio::sync::mpsc;

/// Messages flowing from the audio pipeline to a provider session.
pub enum AudioMsg {
    /// A chunk of 16 kHz mono PCM (i16).
    Audio(Vec<i16>),
    /// End of stream — the user stopped dictating; flush and finalize.
    Eos,
}

/// Which backend to use and how to reach it.
#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub kind: String,
    pub model: String,
    pub api_key: String,
    #[allow(dead_code)] // consumed by Whisper / OpenAI-compatible providers in M5
    pub language: Option<String>,
}

impl ProviderConfig {
    /// M2 config source: environment variables. M7 replaces this with the
    /// settings UI + OS keyring (with an encrypted-file fallback).
    pub fn from_env() -> Result<Self, String> {
        let kind = std::env::var("TRANSCRIPT_PROVIDER").unwrap_or_else(|_| "gemini".into());

        let api_key = std::env::var("TRANSCRIPT_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .map_err(|_| {
                "Aucune clé API. Définis GEMINI_API_KEY (ou TRANSCRIPT_API_KEY).".to_string()
            })?;

        let model = std::env::var("TRANSCRIPT_MODEL")
            .unwrap_or_else(|_| default_model(&kind).to_string());

        let language = std::env::var("TRANSCRIPT_LANGUAGE").ok().filter(|s| !s.is_empty());

        Ok(Self {
            kind,
            model,
            api_key,
            language,
        })
    }
}

fn default_model(kind: &str) -> &'static str {
    match kind {
        "gemini" => "gemini-2.0-flash-live-001",
        _ => "gemini-2.0-flash-live-001",
    }
}

/// Cheap, cloneable handle the audio pipeline uses to push samples.
#[derive(Clone)]
pub struct SessionSink {
    tx: mpsc::UnboundedSender<AudioMsg>,
}

impl SessionSink {
    pub fn push(&self, pcm: &[i16]) {
        let _ = self.tx.send(AudioMsg::Audio(pcm.to_vec()));
    }

    pub fn eos(&self) {
        let _ = self.tx.send(AudioMsg::Eos);
    }
}

/// Tauri-managed handle to the current provider session.
#[derive(Default)]
pub struct SessionController {
    inner: Mutex<Option<SessionSink>>,
}

impl SessionController {
    /// Spawn a session for `cfg` and return the sink the pipeline feeds.
    pub fn start(&self, app: AppHandle, cfg: ProviderConfig) -> Result<SessionSink, String> {
        // Tear down any lingering session first.
        self.stop();

        let (tx, rx) = mpsc::unbounded_channel();
        let sink = SessionSink { tx };

        match cfg.kind.as_str() {
            "gemini" => {
                tauri::async_runtime::spawn(gemini::run_session(app, cfg, rx));
            }
            other => {
                return Err(format!("Fournisseur inconnu: '{other}'"));
            }
        }

        *self.inner.lock().unwrap() = Some(sink.clone());
        Ok(sink)
    }

    /// Signal the running session to flush and finalize.
    pub fn stop(&self) {
        if let Some(sink) = self.inner.lock().unwrap().take() {
            sink.eos();
        }
    }
}
