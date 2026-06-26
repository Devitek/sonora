//! transcript — real-time speech-to-text with pluggable models.
//!
//! M0: application shell — translucent always-on-top HUD window, system tray
//! with show/quit, and the command surface the frontend will drive.
//! Recording commands are stubs here; real audio lands in M1.

mod audio;
mod events;
mod providers;

use audio::AudioController;
use providers::{ProviderConfig, SessionController};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State,
};

/// Liveness probe used by the frontend on mount to confirm the IPC bridge.
#[tauri::command]
fn app_ready() -> String {
    format!("transcript v{}", env!("CARGO_PKG_VERSION"))
}

/// Start a dictation session: spin up the provider stream, then open the mic
/// (16 kHz mono) and feed it. Emits `state: listening` once live, `error` on
/// failure (e.g. missing API key).
#[tauri::command]
fn start_recording(
    app: AppHandle,
    audio: State<'_, AudioController>,
    session: State<'_, SessionController>,
) -> Result<(), String> {
    let cfg = ProviderConfig::from_env()?;
    let sink = session.start(app.clone(), cfg)?;
    audio.start(app, Some(sink))
}

/// Stop the current dictation session and release the microphone.
#[tauri::command]
fn stop_recording(
    audio: State<'_, AudioController>,
    session: State<'_, SessionController>,
) -> Result<(), String> {
    // Stopping the mic flushes the pipeline, which sends EOS to the session;
    // stop() is an idempotent safety net. The session emits `state: idle` once
    // it has drained and finalized the transcript.
    audio.stop();
    session.stop();
    Ok(())
}

/// Hide the HUD window (app keeps running in the tray).
#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn toggle_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            show_window(app);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }

    builder
        .manage(AudioController::default())
        .manage(SessionController::default())
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window only hides it — quit happens from the tray.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_ready,
            start_recording,
            stop_recording,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running transcript");
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "Afficher", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("transcript")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
