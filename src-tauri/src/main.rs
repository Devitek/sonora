// Prevents an additional console window on Windows in release. DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Load .env directly (dev convenience) so provider config reflects the file,
    // not whatever the shell exported when the devshell was entered. Override so
    // an edited .env wins over a stale exported value. (M7 replaces this with a
    // settings UI + keyring.)
    #[cfg(debug_assertions)]
    {
        let _ = dotenvy::dotenv_override();
    }

    // Linux/WebKitGTK runtime workarounds. Must run before GTK/WebKit init.
    #[cfg(target_os = "linux")]
    {
        // WebKitGTK's DMABUF renderer (>= 2.40) fails to create an EGL display on
        // many setups — VMs, some NixOS/Mesa combinations, and newer Intel Arc/Xe
        // GPUs — aborting at launch with:
        //   "Could not create default EGL display: EGL_BAD_PARAMETER. Aborting..."
        // Disabling it falls back to a working rendering path. Honour an explicit
        // user override so power users can re-enable it. (GitHub issue #1.)
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }

        // WebKitGTK mishandles Wayland *fractional* scaling (e.g. Hyprland scale
        // 1.33): it reports a negative devicePixelRatio, which collapses the whole
        // layout to 0×0 at huge negative offsets. Route through XWayland instead,
        // unless the user explicitly picked a GDK backend.
        if std::env::var_os("GDK_BACKEND").is_none()
            && std::env::var_os("WAYLAND_DISPLAY").is_some()
        {
            std::env::set_var("GDK_BACKEND", "x11");
        }
    }

    sonora_lib::run()
}
