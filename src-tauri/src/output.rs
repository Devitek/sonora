//! Type transcribed text into the currently focused window ("dictation" mode).
//!
//! Platform layer:
//!  - Linux/Wayland -> `wtype` (wlroots virtual-keyboard protocol)
//!  - Linux/X11     -> `xdotool type`
//!  - macOS/Windows -> `enigo` synthetic input
//!
//! The text is typed (not pasted), so it doesn't clobber the clipboard.

#[cfg(target_os = "linux")]
pub fn type_text(text: &str) -> Result<(), String> {
    use std::process::Command;

    if text.is_empty() {
        return Ok(());
    }

    // The compositor is Wayland even though our window runs through XWayland,
    // so prefer wtype (types into the truly focused surface).
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();

    let result = if wayland {
        Command::new("wtype").arg(text).status()
    } else {
        Command::new("xdotool")
            .args(["type", "--clearmodifiers", "--", text])
            .status()
    };

    match result {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("outil de saisie a échoué (code {:?})", s.code())),
        Err(e) => Err(format!(
            "outil de saisie introuvable ({}): installe wtype (Wayland) ou xdotool (X11)",
            e
        )),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn type_text(text: &str) -> Result<(), String> {
    use enigo::{Enigo, Keyboard, Settings};

    if text.is_empty() {
        return Ok(());
    }
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo.text(text).map_err(|e| e.to_string())?;
    Ok(())
}
