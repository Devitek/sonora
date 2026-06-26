//! Backend → frontend event contract.
//!
//! Serialized as an internally-tagged union so the Svelte side can switch on
//! `kind` (see `src/lib/types.ts`).

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Channel name shared with the frontend (`EVENT_CHANNEL` in types.ts).
pub const EVENT_CHANNEL: &str = "transcript://event";

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
// Several variants are wired up in later milestones (audio levels, streaming
// partials/finals); keep the full contract defined from the start.
#[allow(dead_code)]
pub enum BackendEvent {
    /// Recording lifecycle: "idle" | "starting" | "listening" | "error".
    State { state: &'static str },
    /// Streaming partial hypothesis (may be revised).
    Partial { text: String },
    /// Finalized chunk of transcript.
    Final { text: String },
    /// Microphone RMS level in 0.0..~1.0 for the UI meter.
    Level { rms: f32 },
    /// Human-readable error surfaced in the HUD.
    Error { message: String },
}

impl BackendEvent {
    pub fn emit(self, app: &AppHandle) {
        if let Err(e) = app.emit(EVENT_CHANNEL, self) {
            eprintln!("[transcript] failed to emit event: {e}");
        }
    }
}
