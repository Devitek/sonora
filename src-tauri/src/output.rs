//! Deliver transcribed text to the currently focused window ("dictation" mode).
//!
//! Platform layer:
//!  - Linux/Wayland -> `wtype` types the text (its virtual keyboard carries its
//!    own keymap, so Unicode is exact).
//!  - Linux/X11     -> clipboard + paste keystroke. `xdotool type` can only
//!    press keys that exist in the active layout; for anything else (CJK,
//!    typographic punctuation, accented input) it rebinds a scratch keycode per
//!    character, and that rebinding races the receiving client — characters
//!    come out dropped or wrong. Pasting moves the text through the X selection
//!    instead, which is exact.
//!  - macOS/Windows -> `enigo` synthetic input (Unicode-safe).
//!
//! Sonora already copies every transcript to the clipboard, so the X11 path
//! does not lose anything the user had there that the app wasn't overwriting.

use tauri::AppHandle;

#[cfg(target_os = "linux")]
use std::sync::Mutex;

/// Serialises delivery: two concurrent pastes would race one call's clipboard
/// write against the other's keystroke and paste the same segment twice.
#[cfg(target_os = "linux")]
static DELIVERY: Mutex<()> = Mutex::new(());

/// Send `text` to the currently focused window (dictation-to-cursor).
#[cfg(target_os = "linux")]
pub fn deliver_text(app: &AppHandle, text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    let _guard = DELIVERY.lock().map_err(|e| e.to_string())?;

    // The compositor is Wayland even though our window runs through XWayland,
    // so prefer wtype (types into the truly focused surface).
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return run("wtype", &[text]);
    }
    paste_x11(app, text)
}

/// Put `text` on the clipboard, then press the focused app's paste shortcut.
#[cfg(target_os = "linux")]
fn paste_x11(app: &AppHandle, text: &str) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;

    app.clipboard()
        .write_text(text.to_string())
        .map_err(|e| format!("clipboard write failed: {e}"))?;
    // Let the selection ownership settle before the target asks for it.
    std::thread::sleep(std::time::Duration::from_millis(40));

    let combo = if focused_window_is_terminal() {
        "ctrl+shift+v"
    } else {
        "ctrl+v"
    };
    run("xdotool", &["key", "--clearmodifiers", combo])?;

    // A paste is asynchronous: the target requests the selection after the
    // keystroke. Hold the lock until it has had time to do so, otherwise the
    // next segment's clipboard write races it and that paste comes out empty.
    std::thread::sleep(std::time::Duration::from_millis(150));
    Ok(())
}

/// Terminals paste with Ctrl+Shift+V — plain Ctrl+V inserts a literal "v" there.
/// Unknown (non-matching) windows get Ctrl+V.
#[cfg(target_os = "linux")]
fn focused_window_is_terminal() -> bool {
    // Case-insensitive regex over WM_CLASS, as `xdotool search` matches it.
    const TERMINAL_CLASS_RE: &str = "terminal|konsole|xterm|urxvt|alacritty|kitty|wezterm|tilix|foot|ghostty|guake|terminator|terminology|rio";

    let active = match std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .output()
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => return false,
    };
    if active.is_empty() {
        return false;
    }
    // xdotool 3.2016 (Ubuntu) has no `getwindowclassname`, so list the windows
    // whose class looks like a terminal and check whether the active one is in
    // that set.
    match std::process::Command::new("xdotool")
        .args(["search", "--class", TERMINAL_CLASS_RE])
        .output()
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .any(|l| l.trim() == active),
        Err(_) => false,
    }
}

#[cfg(target_os = "linux")]
fn run(program: &str, args: &[&str]) -> Result<(), String> {
    match std::process::Command::new(program).args(args).status() {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{program} a échoué (code {:?})", s.code())),
        Err(e) => Err(format!(
            "{program} introuvable ({e}) : installe wtype (Wayland) ou xdotool (X11)"
        )),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn deliver_text(_app: &AppHandle, text: &str) -> Result<(), String> {
    use enigo::{Enigo, Keyboard, Settings};

    if text.is_empty() {
        return Ok(());
    }
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.text(text).map_err(|e| e.to_string())?;
    Ok(())
}
