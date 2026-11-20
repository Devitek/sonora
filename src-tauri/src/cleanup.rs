//! LLM post-processing of a finished transcript:
//!  - built-in **cleanup** (strip hesitations / filler markers), and
//!  - user-defined **reformulation prompts** (formal, terminal command, ...).
//!
//! Both go through the same text engine, configured under "moteur de
//! reformulation" in settings: `gemini` (generateContent) or any
//! `openai-compatible` chat/completions endpoint (OpenAI, Groq, local LLMs).

use std::path::Path;

use serde_json::{json, Value};

use crate::{secrets, settings};

const CLEANUP_SYSTEM: &str = "Tu nettoies des transcriptions vocales. Supprime les marqueurs d'hésitation et mots de remplissage (euh, heu, hum, hmm, ben, bah, mmh, ainsi que les répétitions involontaires et les faux départs) SANS changer le sens, le vocabulaire ni la langue. Tu peux corriger la ponctuation et les majuscules. Réponds UNIQUEMENT avec le texte nettoyé, sans guillemets ni commentaire.";

/// Built-in hesitation cleanup.
pub async fn run(config_dir: &Path, text: &str) -> Result<String, String> {
    complete(config_dir, CLEANUP_SYSTEM, text).await
}

/// Apply a user-defined reformulation prompt to the transcript.
pub async fn transform(config_dir: &Path, instruction: &str, text: &str) -> Result<String, String> {
    let instruction = instruction.trim();
    if instruction.is_empty() {
        return Err("Prompt vide.".into());
    }
    let system = format!(
        "{instruction}\n\nTu reçois ci-dessous un texte issu d'une dictée vocale. Applique strictement l'instruction ci-dessus à ce texte. Réponds UNIQUEMENT avec le résultat, sans préambule, sans guillemets, ni commentaire."
    );
    complete(config_dir, &system, text).await
}

/// Run `text` through the configured text engine with the given `system` prompt.
async fn complete(config_dir: &Path, system: &str, text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(String::new());
    }

    let s = settings::load(config_dir);
    let provider = nonempty(s.cleanup_provider).unwrap_or_else(|| "gemini".into());
    let client = reqwest::Client::new();

    let out = match provider.as_str() {
        "gemini" => {
            let model = nonempty(s.cleanup_model).unwrap_or_else(|| "gemini-2.5-flash".into());
            let key = secrets::get_api_key(config_dir, "gemini")
                .or_else(|| first_env(&["GEMINI_API_KEY", "GOOGLE_API_KEY", "TRANSCRIPT_API_KEY"]))
                .ok_or("Clé Gemini absente (⚙ → moteur de reformulation).")?;
            gemini_generate(&client, &model, &key, system, text).await?
        }
        "openai-compatible" | "openai" | "groq" | "mistral" | "minimax" => {
            let base = nonempty(s.cleanup_base_url)
                .or_else(|| default_base(&provider))
                .ok_or("Définis l'URL de base du moteur de reformulation (⚙).")?;
            let model = nonempty(s.cleanup_model)
                .or_else(|| default_model(&provider))
                .ok_or("Définis le modèle du moteur de reformulation (⚙).")?;
            // Keyring "cleanup" entry first, then env: the generic CLEANUP_API_KEY,
            // then this engine's OWN vendor variable (never another vendor's — a
            // Mistral key must not be sent to Groq/OpenAI), then TRANSCRIPT_API_KEY.
            let mut env_keys = vec!["CLEANUP_API_KEY"];
            env_keys.extend(vendor_env(&provider));
            env_keys.push("TRANSCRIPT_API_KEY");
            let key = secrets::get_api_key(config_dir, "cleanup")
                .or_else(|| first_env(&env_keys))
                .unwrap_or_default();
            openai_chat(
                &client,
                &base,
                &model,
                &key,
                &extra_body(&provider),
                system,
                text,
            )
            .await?
        }
        other => return Err(format!("Moteur de reformulation inconnu: '{other}'")),
    };

    Ok(out.trim().to_string())
}

async fn gemini_generate(
    client: &reqwest::Client,
    model: &str,
    key: &str,
    system: &str,
    text: &str,
) -> Result<String, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
    );
    let body = json!({
        "systemInstruction": { "parts": [{ "text": system }] },
        "contents": [{ "role": "user", "parts": [{ "text": text }] }],
        "generationConfig": { "temperature": 0.2 }
    });

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Requête Gemini: {e}"))?;
    let status = resp.status();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = v
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("erreur");
        return Err(format!("Gemini {}: {msg}", status.as_u16()));
    }

    let out = v
        .pointer("/candidates/0/content/parts")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<String>()
        })
        .unwrap_or_default();
    if out.is_empty() {
        return Err("Réponse Gemini vide".into());
    }
    Ok(out)
}

async fn openai_chat(
    client: &reqwest::Client,
    base: &str,
    model: &str,
    key: &str,
    extra: &Value,
    system: &str,
    text: &str,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let mut body = json!({
        "model": model,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": text }
        ]
    });
    // Merge vendor-specific request fields (e.g. MiniMax's thinking toggle).
    if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
        body.extend(extra.clone());
    }

    let mut req = client.post(&url).json(&body);
    if !key.is_empty() {
        req = req.bearer_auth(key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Requête nettoyage: {e}"))?;
    let status = resp.status();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = v
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("erreur");
        return Err(format!("HTTP {}: {msg}", status.as_u16()));
    }

    let content = v
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let out = strip_think(content).trim().to_string();
    if out.is_empty() {
        return Err("Réponse vide".into());
    }
    Ok(out)
}

fn default_base(provider: &str) -> Option<String> {
    match provider {
        "openai" => Some("https://api.openai.com/v1".into()),
        "groq" => Some("https://api.groq.com/openai/v1".into()),
        "mistral" => Some("https://api.mistral.ai/v1".into()),
        "minimax" => Some("https://api.minimax.io/v1".into()),
        _ => None,
    }
}

fn default_model(provider: &str) -> Option<String> {
    match provider {
        "openai" => Some("gpt-4o-mini".into()),
        "groq" => Some("llama-3.3-70b-versatile".into()),
        "mistral" => Some("mistral-small-latest".into()),
        "minimax" => Some("MiniMax-M3".into()),
        _ => None, // generic openai-compatible: the user must pick a model
    }
}

/// The engine's own vendor API-key variable, used only as a fallback for that
/// engine. Generic `openai-compatible` has none (it points at an arbitrary host).
fn vendor_env(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" => Some("OPENAI_API_KEY"),
        "groq" => Some("GROQ_API_KEY"),
        "mistral" => Some("MISTRAL_API_KEY"),
        "minimax" => Some("MINIMAX_API_KEY"),
        _ => None,
    }
}

/// Vendor-specific fields merged into the chat/completions request body.
fn extra_body(provider: &str) -> Value {
    match provider {
        // MiniMax-M3 answers directly when thinking is disabled (low latency);
        // `reasoning_split` keeps any reasoning out of the `content` field.
        // https://platform.minimax.io/docs/api-reference/text-openai-api
        "minimax" => json!({ "thinking": { "type": "disabled" }, "reasoning_split": true }),
        _ => json!({}),
    }
}

/// Drop a leading `<think>…</think>` block that some reasoning models inline in
/// `content`, so their chain of thought never reaches the cursor.
fn strip_think(s: &str) -> &str {
    s.trim_start()
        .strip_prefix("<think>")
        .and_then(|rest| rest.split_once("</think>"))
        .map_or(s, |(_, answer)| answer)
}

fn nonempty(v: Option<String>) -> Option<String> {
    v.filter(|s| !s.is_empty())
}

fn first_env(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| std::env::var(k).ok().filter(|s| !s.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_think_removes_leading_reasoning() {
        assert_eq!(
            strip_think("<think>plan…</think>\nanswer.").trim(),
            "answer."
        );
        assert_eq!(strip_think("  <think>x</think>ok"), "ok");
    }

    #[test]
    fn strip_think_keeps_plain_or_unclosed_text() {
        assert_eq!(strip_think("plain text"), "plain text");
        assert_eq!(strip_think("<think>never closed"), "<think>never closed");
        assert_eq!(strip_think("a <think>b</think>"), "a <think>b</think>");
    }

    #[test]
    fn minimax_preset_defaults() {
        assert_eq!(
            default_base("minimax").as_deref(),
            Some("https://api.minimax.io/v1")
        );
        assert_eq!(default_model("minimax").as_deref(), Some("MiniMax-M3"));
        assert_eq!(vendor_env("minimax"), Some("MINIMAX_API_KEY"));
        assert_eq!(extra_body("minimax")["thinking"]["type"], "disabled");
        assert_eq!(extra_body("openai"), json!({}));
    }
}
