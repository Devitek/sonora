//! Sonora — real-time speech-to-text with pluggable models.

mod audio;
mod cleanup;
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
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
};

/// Control channel: backend-initiated actions (global hotkey / CLI / tray)
/// that the frontend turns into the same start/stop flow as the mic button.
const CONTROL_CHANNEL: &str = "transcript://control";

/// A CLI action carried over from first launch until the webview is ready.
#[derive(Default)]
struct PendingAction(Mutex<Option<String>>);

/// Map CLI args to a control action: `sonora toggle|start|stop|show`.
fn parse_action(args: &[String]) -> Option<&'static str> {
    args.iter().find_map(|a| match a.as_str() {
        "toggle" => Some("toggle"),
        "start" => Some("start"),
        "stop" => Some("stop"),
        "show" => Some("show"),
        _ => None,
    })
}

/// Surface the floating bar and forward an action to the frontend. The bar does
/// NOT grab focus, so dictated text keeps landing in the app the user was using.
fn dispatch_action(app: &AppHandle, action: &str) {
    show_window(app);
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

/// Post-process a transcript through an LLM to strip hesitation/filler markers.
#[tauri::command]
async fn cleanup_text(app: AppHandle, text: String) -> Result<String, String> {
    let dir = config_dir(&app)?;
    cleanup::run(&dir, &text).await
}

/// Reformulate a transcript with a user-defined prompt (formal, command, ...).
#[tauri::command]
async fn transform_text(app: AppHandle, text: String, prompt: String) -> Result<String, String> {
    let dir = config_dir(&app)?;
    cleanup::transform(&dir, &prompt, &text).await
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
    format!("Sonora v{}", env!("CARGO_PKG_VERSION"))
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
    let dir = config_dir(&app)?;
    let cfg = ProviderConfig::resolve(&dir)?;
    let device = settings::load(&dir).input_device.filter(|s| !s.is_empty());
    let sink = session.start(app.clone(), cfg)?;
    audio.start(app, Some(sink), device)
}

/// List the available microphones (input devices) for the settings picker.
#[tauri::command]
fn list_audio_inputs() -> Vec<audio::AudioInput> {
    audio::list_input_devices()
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

/// Hide the floating bar (app keeps running in the tray).
#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

/// Resize the bar window to fit its content height (Spotlight-style growth),
/// keeping the top-left fixed so the bar doesn't jump. Width stays constant.
#[tauri::command]
fn resize_bar(app: AppHandle, height: u32) {
    if let Some(win) = app.get_webview_window("main") {
        let h = height.clamp(80, 1000) as f64;
        let _ = win.set_size(tauri::LogicalSize::new(480.0, h));
    }
}

/// Open (or focus) the dedicated panel window (History + Settings tabs) on a
/// given tab. Keeping history/settings in their own normal, resizable window
/// avoids resizing/flickering the floating bar. `tab` is "history" | "settings".
#[tauri::command]
fn open_settings(app: AppHandle, tab: Option<String>) -> Result<(), String> {
    let tab = match tab.as_deref() {
        Some("settings") => "settings",
        _ => "history",
    };
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        // Window already up: ask it to switch to the requested tab.
        let _ = win.emit("sonora://panel-tab", tab);
        return Ok(());
    }
    // First open: seed the active tab via the URL (the listener isn't ready yet).
    let url = format!("index.html?tab={tab}");
    WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App(url.into()))
        .title("Sonora — Panneau")
        .inner_size(480.0, 680.0)
        .min_inner_size(380.0, 460.0)
        .resizable(true)
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Notify all windows that persisted settings changed, so the bar can reload
/// (provider, prompts, auto-type, cleanup, theme, configured-state).
#[tauri::command]
fn broadcast_settings_changed(app: AppHandle) {
    let _ = app.emit("sonora://settings-changed", ());
}

/// Notify all windows that the transcript history changed, so the bar and the
/// panel's History tab stay in sync (new dictation, delete, clear).
#[tauri::command]
fn broadcast_history_changed(app: AppHandle) {
    let _ = app.emit("sonora://history-changed", ());
}

/// Show the floating bar WITHOUT stealing focus, so the app the user is in
/// keeps it (dictation-to-cursor works). Clicking the bar focuses it naturally.
fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
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
    // Hyprland keybind running `sonora toggle`) forwards its argv here.
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
            // The floating bar only hides on close (it lives in the tray);
            // secondary windows (e.g. Settings) close/destroy normally.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_ready,
            start_recording,
            stop_recording,
            list_audio_inputs,
            hide_window,
            resize_bar,
            open_settings,
            broadcast_settings_changed,
            broadcast_history_changed,
            take_pending_action,
            type_text,
            cleanup_text,
            transform_text,
            get_settings,
            save_settings,
            is_configured,
            has_api_key,
            save_api_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sonora");
}

/// Register a desktop global shortcut (Ctrl+Shift+Space) to toggle dictation.
/// On Wayland global capture is unavailable — prefer a Hyprland keybind running
/// `sonora toggle` (handled via the single-instance plugin). Failures here
/// are expected on Wayland and only logged.
#[cfg(desktop)]
fn register_global_shortcut(app: &AppHandle) {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
    let res = app
        .global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                dispatch_action(app, "toggle");
            }
        });
    if let Err(e) = res {
        eprintln!("[sonora] raccourci global indisponible (normal sous Wayland): {e}");
    }
}

#[cfg(not(desktop))]
fn register_global_shortcut(_app: &AppHandle) {}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle_i = MenuItem::with_id(
        app,
        "toggle",
        "Démarrer / arrêter la dictée",
        true,
        None::<&str>,
    )?;
    let show_i = MenuItem::with_id(app, "show", "Afficher", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_i, &show_i, &quit_i])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Sonora")
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
