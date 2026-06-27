//! transcript — real-time speech-to-text with pluggable models.

mod audio;
mod events;
mod output;
mod providers;
mod secrets;
mod settings;

use std::path::PathBuf;
use std::sync::Mutex;

use audio::AudioController;
use providers::{ProviderConfig, SessionController};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};

/// Control channel: backend-initiated actions (global hotkey / CLI / tray)
/// that the frontend turns into the same start/stop flow as the mic button.
const CONTROL_CHANNEL: &str = "transcript://control";

/// A CLI action carried over from first launch until the webview is ready.
#[derive(Default)]
struct PendingAction(Mutex<Option<String>>);

/// Map CLI args to a control action: `transcript toggle|start|stop|show`.
fn parse_action(args: &[String]) -> Option<&'static str> {
    args.iter().find_map(|a| match a.as_str() {
        "toggle" => Some("toggle"),
        "start" => Some("start"),
        "stop" => Some("stop"),
        "show" => Some("show"),
        _ => None,
    })
}

/// Surface the HUD and forward an action to the frontend. Recording actions do
/// NOT grab focus, so dictated text keeps landing in the app the user was using.
fn dispatch_action(app: &AppHandle, action: &str) {
    surface_window(app, action == "show");
    let _ = app.emit(CONTROL_CHANNEL, serde_json::json!({ "action": action }));
}

/// Consumed by the frontend on mount to replay a first-launch CLI action.
#[tauri::command]
fn take_pending_action(state: State<'_, PendingAction>) -> Option<String> {
    state.0.lock().unwrap().take()
}

/// Type `text` into the currently focused window (dictation-to-cursor).
#[tauri::command]
fn type_text(text: String) -> Result<(), String> {
    output::type_text(&text)
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<settings::Settings, String> {
    Ok(settings::load(&config_dir(&app)?))
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: settings::Settings) -> Result<(), String> {
    settings::save(&config_dir(&app)?, &settings)
}

/// Whether a usable transcription config resolves (provider + required key/
/// fields). Drives the first-run onboarding so the user configures a model
/// before hitting a start error.
#[tauri::command]
fn is_configured(app: AppHandle) -> bool {
    config_dir(&app)
        .map(|dir| ProviderConfig::resolve(&dir).is_ok())
        .unwrap_or(false)
}

/// Whether an API key is stored for `provider` (the key itself is never returned).
#[tauri::command]
fn has_api_key(app: AppHandle, provider: String) -> Result<bool, String> {
    Ok(secrets::has_api_key(&config_dir(&app)?, &provider))
}

/// Store (or, when empty, delete) the API key for `provider`.
#[tauri::command]
fn save_api_key(app: AppHandle, provider: String, key: String) -> Result<(), String> {
    let dir = config_dir(&app)?;
    if key.is_empty() {
        secrets::delete_api_key(&dir, &provider);
        Ok(())
    } else {
        secrets::set_api_key(&dir, &provider, &key)
    }
}

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
    let cfg = ProviderConfig::resolve(&config_dir(&app)?)?;
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
    surface_window(app, true);
}

/// Show the HUD; only grab keyboard focus when `focus` is true.
fn surface_window(app: &AppHandle, focus: bool) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        if focus {
            let _ = win.set_focus();
        }
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
    let mut builder = tauri::Builder::default();

    // single-instance MUST be the first plugin: a second launch (e.g. the
    // Hyprland keybind running `transcript toggle`) forwards its argv here.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let args: Vec<String> = argv.into_iter().skip(1).collect();
            match parse_action(&args) {
                Some(action) => dispatch_action(app, action),
                None => show_window(app),
            }
        }));
    }

    builder = builder
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }

    builder
        .manage(AudioController::default())
        .manage(SessionController::default())
        .manage(PendingAction::default())
        .setup(|app| {
            setup_tray(app.handle())?;
            register_global_shortcut(app.handle());

            // Stash a first-launch CLI action for the frontend to replay.
            let args: Vec<String> = std::env::args().skip(1).collect();
            if let Some(action) = parse_action(&args) {
                *app.state::<PendingAction>().0.lock().unwrap() = Some(action.to_string());
            }
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
            hide_window,
            take_pending_action,
            type_text,
            get_settings,
            save_settings,
            is_configured,
            has_api_key,
            save_api_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running transcript");
}

/// Register a desktop global shortcut (Ctrl+Shift+Space) to toggle dictation.
/// On Wayland global capture is unavailable — prefer a Hyprland keybind running
/// `transcript toggle` (handled via the single-instance plugin). Failures here
/// are expected on Wayland and only logged.
#[cfg(desktop)]
fn register_global_shortcut(app: &AppHandle) {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
    let res = app
        .global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                dispatch_action(app, "toggle");
            }
        });
    if let Err(e) = res {
        eprintln!("[transcript] raccourci global indisponible (normal sous Wayland): {e}");
    }
}

#[cfg(not(desktop))]
fn register_global_shortcut(_app: &AppHandle) {}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle_i = MenuItem::with_id(app, "toggle", "Démarrer / arrêter la dictée", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "Afficher", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_i, &show_i, &quit_i])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("transcript")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => dispatch_action(app, "toggle"),
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
