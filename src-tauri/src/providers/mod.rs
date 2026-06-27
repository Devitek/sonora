//! Transcription providers.
//!
//! A *session* is a live transcription connection. The audio pipeline feeds it
//! 16 kHz mono PCM through a [`SessionSink`]; the provider emits `partial` /
//! `final` events to the HUD.
//!
//! Two provider shapes:
//!  - **streaming** (Gemini Live): consumes the continuous `Audio` stream and
//!    does its own endpointing.
//!  - **chunked** (OpenAI/Groq/OpenAI-compatible Whisper): consumes complete
//!    VAD `Segment`s and POSTs each to `/audio/transcriptions`.

pub mod gemini;
pub mod openai_compat;

use std::sync::Mutex;

use tauri::AppHandle;
use tokio::sync::mpsc;

/// Messages flowing from the audio pipeline to a provider session.
pub enum AudioMsg {
    /// A chunk of 16 kHz mono PCM (i16) — continuous, for streaming providers.
    Audio(Vec<i16>),
    /// A complete speech segment (16 kHz mono) — for chunked providers.
    Segment(Vec<i16>),
    /// End of stream — the user stopped dictating; flush and finalize.
    Eos,
}

/// Which backend to use and how to reach it.
#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub kind: String,
    pub model: String,
    pub api_key: String,
    /// Base URL for OpenAI-compatible REST providers (e.g. `.../v1`).
    pub base_url: Option<String>,
    pub language: Option<String>,
}

impl ProviderConfig {
    /// Config from environment (dev). M7 replaces this with the settings UI +
    /// OS keyring. Supported `TRANSCRIPT_PROVIDER` values:
    ///   gemini | openai | groq | openai-compatible
    pub fn from_env() -> Result<Self, String> {
        let kind = env_nonempty("TRANSCRIPT_PROVIDER").unwrap_or_else(|| "gemini".into());
        let language = env_nonempty("TRANSCRIPT_LANGUAGE");
        let model_override = env_nonempty("TRANSCRIPT_MODEL");
        let base_override = env_nonempty("TRANSCRIPT_BASE_URL");

        let cfg = match kind.as_str() {
            "gemini" => Self {
                model: model_override.unwrap_or_else(|| "gemini-2.5-flash-native-audio-latest".into()),
                api_key: require_key(
                    &["TRANSCRIPT_API_KEY", "GEMINI_API_KEY", "GOOGLE_API_KEY"],
                    "Définis GEMINI_API_KEY (ou TRANSCRIPT_API_KEY).",
                )?,
                base_url: None,
                language,
                kind,
            },
            "openai" => Self {
                model: model_override.unwrap_or_else(|| "whisper-1".into()),
                api_key: require_key(
                    &["OPENAI_API_KEY", "TRANSCRIPT_API_KEY"],
                    "Définis OPENAI_API_KEY.",
                )?,
                base_url: Some(base_override.unwrap_or_else(|| "https://api.openai.com/v1".into())),
                language,
                kind,
            },
            "groq" => Self {
                model: model_override.unwrap_or_else(|| "whisper-large-v3".into()),
                api_key: require_key(
                    &["GROQ_API_KEY", "TRANSCRIPT_API_KEY"],
                    "Définis GROQ_API_KEY.",
                )?,
                base_url: Some(
                    base_override.unwrap_or_else(|| "https://api.groq.com/openai/v1".into()),
                ),
                language,
                kind,
            },
            "openai-compatible" => Self {
                base_url: Some(
                    base_override.ok_or("Définis TRANSCRIPT_BASE_URL pour openai-compatible.")?,
                ),
                model: model_override
                    .ok_or("Définis TRANSCRIPT_MODEL pour openai-compatible.")?,
                api_key: env_nonempty("TRANSCRIPT_API_KEY").unwrap_or_default(),
                language,
                kind,
            },
            other => return Err(format!("Fournisseur inconnu: '{other}'")),
        };
        Ok(cfg)
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

fn require_key(keys: &[&str], hint: &str) -> Result<String, String> {
    keys.iter()
        .find_map(|k| env_nonempty(k))
        .ok_or_else(|| format!("Aucune clé API. {hint}"))
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

    pub fn segment(&self, pcm: Vec<i16>) {
        let _ = self.tx.send(AudioMsg::Segment(pcm));
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
        self.stop(); // tear down any lingering session first

        let (tx, rx) = mpsc::unbounded_channel();
        let sink = SessionSink { tx };

        match cfg.kind.as_str() {
            "gemini" => {
                tauri::async_runtime::spawn(gemini::run_session(app, cfg, rx));
            }
            "openai" | "groq" | "openai-compatible" => {
                tauri::async_runtime::spawn(openai_compat::run_session(app, cfg, rx));
            }
            other => return Err(format!("Fournisseur inconnu: '{other}'")),
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
