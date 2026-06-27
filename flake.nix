{
  description = "transcript — real-time speech-to-text with pluggable models";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Runtime/link libraries required by Tauri (WebKitGTK), the system tray,
        # clipboard (X11/Wayland), and audio capture (ALSA/PipeWire via cpal).
        runtimeLibs = with pkgs; [
          webkitgtk_4_1
          gtk3
          cairo
          pango
          gdk-pixbuf
          glib
          dbus
          openssl
          librsvg
          libsoup_3
          libayatana-appindicator
          # audio (cpal -> ALSA, ALSA bridges to PipeWire on this system)
          alsa-lib
          # clipboard / windowing
          libxcb
          libx11
          libxcursor
          libxrandr
          libxi
          wayland
          libxkbcommon
        ];

        buildTools = with pkgs; [
          # Rust toolchain (matches the system rustc 1.95 line)
          rustc
          cargo
          rustfmt
          clippy
          rust-analyzer
          # JS toolchain
          nodejs_22
          bun
          # native build glue
          pkg-config
          cargo-tauri
          # Wayland output helpers (typing at cursor / clipboard CLI fallback)
          wtype
          wl-clipboard
          # Headless WebKitGTK harness for inspecting real layout (scripts/)
          gjs
          gobject-introspection
          xvfb-run
        ];

        # Typelibs so gjs can `imports.gi.WebKit2` / Gtk in the harness.
        giLibs = with pkgs; [
          glib
          gtk3
          webkitgtk_4_1
          pango
          harfbuzz
          gdk-pixbuf
          atk
          libsoup_3
          gobject-introspection
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          packages = buildTools ++ runtimeLibs;

          # cpal/whisper-rs sometimes need a libclang for bindgen later on.
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibs}:''${LD_LIBRARY_PATH:-}"
            export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" runtimeLibs}:''${PKG_CONFIG_PATH:-}"
            # WebKitGTK on Wayland/Nvidia can need software compositing.
            export WEBKIT_DISABLE_COMPOSITING_MODE=1
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
            export GI_TYPELIB_PATH="${pkgs.lib.makeSearchPath "lib/girepository-1.0" giLibs}:''${GI_TYPELIB_PATH:-}"
            # Credentials are loaded by the app itself from .env (see main.rs),
            # so editing .env takes effect on the next launch without re-sourcing.
            echo "transcript devshell ready — run: bun install && bun run tauri dev"
          '';
        };
      });
}
