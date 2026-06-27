//! Non-secret user settings, persisted as JSON in the app config dir.
//! (API keys live in the OS keyring — see `secrets.rs`.)

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub language: Option<String>,
    pub base_url: Option<String>,
    pub whisper_model: Option<String>,

    // Post-processing text engine (cleanup + reformulation prompts).
    pub cleanup_enabled: Option<bool>,
    pub cleanup_provider: Option<String>, // gemini | openai | groq | openai-compatible
    pub cleanup_model: Option<String>,
    pub cleanup_base_url: Option<String>,

    /// User-defined reformulation prompts applied to a finished transcript.
    pub prompts: Vec<Prompt>,
}

fn path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

pub fn load(config_dir: &Path) -> Settings {
    std::fs::read(path(config_dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save(config_dir: &Path, s: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let data = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(path(config_dir), data).map_err(|e| e.to_string())
}
