//! Chunked transcription against an OpenAI-compatible REST endpoint.
//!
//! Covers OpenAI Whisper, Groq, and any `/audio/transcriptions`-compatible
//! server. Driven by VAD: each complete speech [`AudioMsg::Segment`] is encoded
//! as a 16 kHz mono WAV and POSTed; the returned text is emitted as a `final`.

use tauri::AppHandle;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{AudioMsg, ProviderConfig};
use crate::audio::TARGET_RATE;
use crate::events::BackendEvent;

/// Ignore sub-100 ms blips to avoid spending an API call on noise.
const MIN_SEGMENT_SAMPLES: usize = TARGET_RATE as usize / 10;

pub async fn run_session(app: AppHandle, cfg: ProviderConfig, mut rx: UnboundedReceiver<AudioMsg>) {
    let client = reqwest::Client::new();
    eprintln!(
        "[{}] prêt (model={}, base={})",
        cfg.kind,
        cfg.model,
        cfg.base_url.as_deref().unwrap_or("?")
    );

    while let Some(msg) = rx.recv().await {
        match msg {
            AudioMsg::Segment(pcm) => {
                if pcm.len() < MIN_SEGMENT_SAMPLES {
                    continue;
                }
                // Show activity while the request is in flight.
                BackendEvent::Partial { text: "…".into() }.emit(&app);
                match transcribe(&client, &cfg, &pcm).await {
                    Ok(text) if !text.is_empty() => {
                        BackendEvent::Final { text }.emit(&app);
                    }
                    Ok(_) => {
                        // Empty result — clear the "…" placeholder.
                        BackendEvent::Partial {
                            text: String::new(),
                        }
                        .emit(&app);
                    }
                    Err(e) => {
                        eprintln!("[{}] {e}", cfg.kind);
                        BackendEvent::Error { message: e }.emit(&app);
                    }
                }
            }
            AudioMsg::Audio(_) => {} // chunked provider uses segments only
            AudioMsg::Eos => break,
        }
    }

    BackendEvent::State { state: "idle" }.emit(&app);
}

async fn transcribe(
    client: &reqwest::Client,
    cfg: &ProviderConfig,
    pcm: &[i16],
) -> Result<String, String> {
    let wav = pcm_to_wav(pcm, TARGET_RATE);
    let base = cfg
        .base_url
        .as_deref()
        .unwrap_or_default()
        .trim_end_matches('/');
    let url = format!("{base}/audio/transcriptions");

    let part = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;
    let mut form = reqwest::multipart::Form::new()
        .text("model", cfg.model.clone())
        .text("response_format", "json")
        .part("file", part);
    if let Some(lang) = &cfg.language {
        form = form.text("language", lang.clone());
    }

    let mut req = client.post(&url).multipart(form);
    if !cfg.api_key.is_empty() {
        req = req.bearer_auth(&cfg.api_key);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Requête transcription échouée: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let snippet: String = body.chars().take(200).collect();
        return Err(format!("HTTP {} sur {url}: {snippet}", status.as_u16()));
    }

    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Réponse JSON invalide: {e}"))?;
    Ok(v.get("text")
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .trim()
        .to_string())
}

/// Wrap 16-bit mono PCM in a minimal WAV container.
fn pcm_to_wav(pcm: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let byte_rate = sample_rate * 2; // mono * 16-bit
    let mut buf = Vec::with_capacity(44 + data_len as usize);

    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_len).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // channels = mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}
