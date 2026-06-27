//! Local, offline transcription via whisper.cpp (whisper-rs).
//!
//! Loads a ggml model once (`TRANSCRIPT_WHISPER_MODEL`), then transcribes each
//! VAD speech segment on a blocking thread. 100% local — no network.

use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::mpsc::UnboundedReceiver;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{AudioMsg, ProviderConfig};
use crate::audio::TARGET_RATE;
use crate::events::BackendEvent;

const MIN_SEGMENT_SAMPLES: usize = TARGET_RATE as usize / 10;

pub async fn run_session(app: AppHandle, cfg: ProviderConfig, mut rx: UnboundedReceiver<AudioMsg>) {
    let ctx = match WhisperContext::new_with_params(&cfg.model, WhisperContextParameters::default())
    {
        Ok(c) => Arc::new(c),
        Err(e) => {
            BackendEvent::Error {
                message: format!("Chargement du modèle whisper '{}' échoué: {e}", cfg.model),
            }
            .emit(&app);
            BackendEvent::State { state: "idle" }.emit(&app);
            return;
        }
    };
    eprintln!("[whisper-local] modèle chargé: {}", cfg.model);
    let lang = cfg.language.clone();

    while let Some(msg) = rx.recv().await {
        match msg {
            AudioMsg::Segment(pcm) => {
                if pcm.len() < MIN_SEGMENT_SAMPLES {
                    continue;
                }
                BackendEvent::Partial { text: "…".into() }.emit(&app);
                let audio: Vec<f32> = pcm.iter().map(|s| *s as f32 / 32768.0).collect();
                let ctx = ctx.clone();
                let lang = lang.clone();
                let res =
                    tokio::task::spawn_blocking(move || transcribe(&ctx, &audio, lang.as_deref()))
                        .await;
                match res {
                    Ok(Ok(text)) if !text.is_empty() => {
                        BackendEvent::Final { text }.emit(&app);
                    }
                    Ok(Ok(_)) => BackendEvent::Partial { text: String::new() }.emit(&app),
                    Ok(Err(e)) => {
                        eprintln!("[whisper-local] {e}");
                        BackendEvent::Error { message: e }.emit(&app);
                    }
                    Err(e) => BackendEvent::Error {
                        message: format!("Tâche whisper: {e}"),
                    }
                    .emit(&app),
                }
            }
            AudioMsg::Audio(_) => {}
            AudioMsg::Eos => break,
        }
    }

    BackendEvent::State { state: "idle" }.emit(&app);
}

fn transcribe(ctx: &WhisperContext, audio: &[f32], lang: Option<&str>) -> Result<String, String> {
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(lang);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state.full(params, audio).map_err(|e| e.to_string())?;

    let n = state.full_n_segments();
    let mut text = String::new();
    for i in 0..n {
        if let Some(seg) = state.get_segment(i) {
            if let Ok(s) = seg.to_str_lossy() {
                text.push_str(&s);
            }
        }
    }
    Ok(text.trim().to_string())
}
