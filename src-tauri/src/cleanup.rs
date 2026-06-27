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
        "openai-compatible" | "openai" | "groq" => {
            let base = nonempty(s.cleanup_base_url)
                .or_else(|| default_base(&provider))
                .ok_or("Définis l'URL de base du moteur de reformulation (⚙).")?;
            let model = nonempty(s.cleanup_model)
                .or_else(|| default_model(&provider))
                .ok_or("Définis le modèle du moteur de reformulation (⚙).")?;
            let key = secrets::get_api_key(config_dir, "cleanup")
                .or_else(|| first_env(&["CLEANUP_API_KEY", "TRANSCRIPT_API_KEY"]))
                .unwrap_or_default();
            openai_chat(&client, &base, &model, &key, system, text).await?
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
    system: &str,
    text: &str,
) -> Result<String, String> {
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": text }
        ]
    });

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

    let out = v
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if out.is_empty() {
        return Err("Réponse vide".into());
    }
    Ok(out)
}

fn default_base(provider: &str) -> Option<String> {
    match provider {
        "openai" => Some("https://api.openai.com/v1".into()),
        "groq" => Some("https://api.groq.com/openai/v1".into()),
        _ => None,
    }
}

fn default_model(provider: &str) -> Option<String> {
    match provider {
        "openai" => Some("gpt-4o-mini".into()),
        "groq" => Some("llama-3.3-70b-versatile".into()),
        _ => None, // generic openai-compatible: the user must pick a model
    }
}

fn nonempty(v: Option<String>) -> Option<String> {
    v.filter(|s| !s.is_empty())
}

fn first_env(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| std::env::var(k).ok().filter(|s| !s.is_empty()))
}
