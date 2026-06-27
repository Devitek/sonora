//! API-key storage: OS keyring first, local file fallback.
//!
//! On Linux the keyring needs an active Secret Service (gnome-keyring / kwallet).
//! When that's unavailable (common on minimal NixOS/Wayland setups) we fall back
//! to a `0600` file in the app config dir (base64, obfuscated — not encrypted;
//! it relies on file permissions). The account is the provider name.

use std::path::{Path, PathBuf};

use base64::Engine;

const SERVICE: &str = "transcript";

pub fn set_api_key(config_dir: &Path, provider: &str, key: &str) -> Result<(), String> {
    if keyring_set(provider, key).is_ok() {
        let _ = fallback_remove(config_dir, provider); // drop any stale copy
        return Ok(());
    }
    fallback_set(config_dir, provider, key)
}

pub fn get_api_key(config_dir: &Path, provider: &str) -> Option<String> {
    if let Ok(k) = keyring_get(provider) {
        if !k.is_empty() {
            return Some(k);
        }
    }
    fallback_get(config_dir, provider)
}

pub fn has_api_key(config_dir: &Path, provider: &str) -> bool {
    get_api_key(config_dir, provider)
        .map(|k| !k.is_empty())
        .unwrap_or(false)
}

pub fn delete_api_key(config_dir: &Path, provider: &str) {
    if let Ok(entry) = keyring::Entry::new(SERVICE, provider) {
        let _ = entry.delete_credential();
    }
    let _ = fallback_remove(config_dir, provider);
}

// --- keyring ---

fn keyring_set(provider: &str, key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, provider).map_err(|e| e.to_string())?;
    entry.set_password(key).map_err(|e| e.to_string())
}

fn keyring_get(provider: &str) -> Result<String, String> {
    let entry = keyring::Entry::new(SERVICE, provider).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}

// --- file fallback ---

fn fallback_path(config_dir: &Path) -> PathBuf {
    config_dir.join("secrets.json")
}

fn read_map(config_dir: &Path) -> serde_json::Map<String, serde_json::Value> {
    std::fs::read(fallback_path(config_dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn write_map(
    config_dir: &Path,
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let path = fallback_path(config_dir);
    let data = serde_json::to_vec_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    set_owner_only(&path);
    Ok(())
}

fn fallback_set(config_dir: &Path, provider: &str, key: &str) -> Result<(), String> {
    let mut map = read_map(config_dir);
    let enc = base64::engine::general_purpose::STANDARD.encode(key.as_bytes());
    map.insert(provider.to_string(), serde_json::Value::String(enc));
    write_map(config_dir, &map)
}

fn fallback_get(config_dir: &Path, provider: &str) -> Option<String> {
    let map = read_map(config_dir);
    let enc = map.get(provider)?.as_str()?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(enc).ok()?;
    String::from_utf8(bytes).ok()
}

fn fallback_remove(config_dir: &Path, provider: &str) -> Result<(), String> {
    let mut map = read_map(config_dir);
    if map.remove(provider).is_some() {
        write_map(config_dir, &map)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_owner_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) {}
