//! Gemini Live API provider (reference streaming implementation).
//!
//! Opens a bidirectional WebSocket, enables transcription of the *input* audio
//! (`inputAudioTranscription`), streams 16 kHz mono PCM as base64
//! `realtimeInput.audio`, and turns `serverContent.inputTranscription.text`
//! deltas into HUD `partial`/`final` events. The model's own text response is
//! intentionally ignored — we only want the dictation transcript.

use std::time::{Duration, Instant};

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, Message},
};

use super::{AudioMsg, ProviderConfig};
use crate::events::BackendEvent;

const WS_BASE: &str = "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";
/// ~100 ms of 16 kHz mono audio per outbound chunk (Gemini's recommended size).
const SEND_THRESHOLD: usize = 1600;
/// How long to keep reading trailing transcription after the user stops.
const FINALIZE_GRACE: Duration = Duration::from_millis(1500);

pub async fn run_session(
    app: AppHandle,
    cfg: ProviderConfig,
    mut rx: UnboundedReceiver<AudioMsg>,
) {
    let url = format!("{WS_BASE}?key={}", cfg.api_key);

    let (ws, _resp) = match connect_async(url.as_str()).await {
        Ok(x) => x,
        Err(e) => {
            BackendEvent::Error {
                message: format!("Connexion Gemini Live échouée: {e}"),
            }
            .emit(&app);
            return;
        }
    };
    eprintln!("[gemini] WebSocket connecté (model={})", cfg.model);
    let (mut write, mut read) = ws.split();

    // The available Live models are native-audio: they only accept the AUDIO
    // response modality. We don't care about the spoken reply (discarded) —
    // `inputAudioTranscription` is what yields the user's dictation transcript.
    let setup = json!({
        "setup": {
            "model": format!("models/{}", cfg.model),
            "generationConfig": { "responseModalities": ["AUDIO"] },
            "inputAudioTranscription": {}
        }
    });
    if let Err(e) = write.send(text_msg(setup.to_string())).await {
        BackendEvent::Error {
            message: format!("Envoi du setup Gemini: {e}"),
        }
        .emit(&app);
        return;
    }
    eprintln!("[gemini] setup envoyé: {}", setup);

    let mut ready = false;
    let mut pending: Vec<i16> = Vec::new(); // audio captured before setupComplete
    let mut send_buf: Vec<i16> = Vec::new();
    let mut transcript = String::new();

    // ---- streaming phase ----
    'main: loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(AudioMsg::Audio(samples)) => {
                    if !ready {
                        pending.extend_from_slice(&samples);
                    } else {
                        send_buf.extend_from_slice(&samples);
                        if send_buf.len() >= SEND_THRESHOLD {
                            if send_audio(&mut write, &send_buf).await.is_err() {
                                break 'main;
                            }
                            send_buf.clear();
                        }
                    }
                }
                // Streaming provider does its own endpointing — ignore VAD segments.
                Some(AudioMsg::Segment(_)) => {}
                Some(AudioMsg::Eos) | None => {
                    // Flush and tell Gemini the stream ended.
                    if ready && !send_buf.is_empty() {
                        let _ = send_audio(&mut write, &send_buf).await;
                    }
                    let _ = write
                        .send(text_msg(json!({ "realtimeInput": { "audioStreamEnd": true } }).to_string()))
                        .await;
                    break 'main;
                }
            },
            maybe = read.next() => match maybe {
                Some(Ok(Message::Text(txt))) => {
                    log_server(txt.as_str());
                    if process_server(txt.as_str(), &app, &mut transcript, &mut ready) && !pending.is_empty() {
                        let _ = send_audio(&mut write, &pending).await;
                        pending.clear();
                    }
                }
                Some(Ok(Message::Binary(bin))) => {
                    let txt = String::from_utf8_lossy(&bin);
                    log_server(&txt);
                    if process_server(&txt, &app, &mut transcript, &mut ready) && !pending.is_empty() {
                        let _ = send_audio(&mut write, &pending).await;
                        pending.clear();
                    }
                }
                Some(Ok(Message::Close(frame))) => {
                    let reason = frame
                        .map(|f| format!("code {} — {}", u16::from(f.code), f.reason))
                        .unwrap_or_else(|| "sans raison".into());
                    eprintln!("[gemini] fermeture serveur: {reason} (ready={ready})");
                    // Closed before setupComplete -> the producer refused the
                    // session (bad model/key/fields); surface it in the HUD.
                    if !ready {
                        BackendEvent::Error {
                            message: format!("Gemini a refusé la session: {reason}"),
                        }
                        .emit(&app);
                    }
                    break 'main;
                }
                None => break 'main,
                Some(Ok(_)) => {} // ping / pong / frame
                Some(Err(e)) => {
                    BackendEvent::Error { message: format!("Flux Gemini interrompu: {e}") }.emit(&app);
                    break 'main;
                }
            },
        }
    }

    // ---- finalize phase: drain trailing transcription for a short grace window ----
    let deadline = Instant::now() + FINALIZE_GRACE;
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, read.next()).await {
            Ok(Some(Ok(Message::Text(txt)))) => {
                process_server(txt.as_str(), &app, &mut transcript, &mut ready);
            }
            Ok(Some(Ok(Message::Binary(bin)))) => {
                let txt = String::from_utf8_lossy(&bin);
                process_server(&txt, &app, &mut transcript, &mut ready);
            }
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => break,
            Ok(Some(Ok(_))) => {}
            Err(_) => break, // grace window elapsed
        }
    }

    let leftover = transcript.trim().to_string();
    if !leftover.is_empty() {
        BackendEvent::Final { text: leftover }.emit(&app);
    }
    let _ = write.send(Message::Close(None)).await;

    // Session fully finalized — tell the UI it can persist the transcript.
    BackendEvent::State { state: "idle" }.emit(&app);
}

/// Parse one server message. Returns `true` when this message was
/// `setupComplete` (so the caller flushes buffered audio).
fn process_server(txt: &str, app: &AppHandle, transcript: &mut String, ready: &mut bool) -> bool {
    let Ok(v) = serde_json::from_str::<Value>(txt) else {
        return false;
    };

    if v.get("setupComplete").is_some() {
        let became_ready = !*ready;
        *ready = true;
        return became_ready;
    }

    if let Some(sc) = v.get("serverContent") {
        if let Some(t) = sc
            .pointer("/inputTranscription/text")
            .and_then(Value::as_str)
        {
            transcript.push_str(t);
            BackendEvent::Partial {
                text: transcript.clone(),
            }
            .emit(app);
        }
        if sc.get("turnComplete").and_then(Value::as_bool).unwrap_or(false) {
            let final_text = transcript.trim().to_string();
            if !final_text.is_empty() {
                BackendEvent::Final { text: final_text }.emit(app);
            }
            transcript.clear();
        }
    }

    false
}

/// Log a server frame (truncated) for diagnostics, skipping bulky audio chunks.
fn log_server(txt: &str) {
    if txt.contains("inlineData") {
        return; // discarded audio response — don't spam the log
    }
    let snippet: String = txt.chars().take(400).collect();
    eprintln!("[gemini] reçu: {snippet}");
}

async fn send_audio<S>(write: &mut S, samples: &[i16]) -> Result<(), WsError>
where
    S: SinkExt<Message, Error = WsError> + Unpin,
{
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let msg = json!({
        "realtimeInput": {
            "audio": { "data": data, "mimeType": "audio/pcm;rate=16000" }
        }
    });
    write.send(text_msg(msg.to_string())).await
}

/// Build a text WebSocket frame, compatible across tungstenite payload types.
fn text_msg(s: String) -> Message {
    Message::Text(s.into())
}
