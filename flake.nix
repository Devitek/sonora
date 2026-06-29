{
  description = "Sonora — real-time speech-to-text with pluggable models";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;
        version = (builtins.fromTOML (builtins.readFile ./src-tauri/Cargo.toml)).package.version;
        # Directory path (not a path literal per-file) so we can append names
        # containing characters like `@` (e.g. 128x128@2x.png) in strings.
        iconsDir = ./src-tauri/icons;

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
          cmake # whisper-rs (whisper.cpp) build
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

        # ---- Frontend (Svelte/Vite) built with bun -----------------------------
        # Fixed-output derivation: it needs network for `bun install`, so we pin
        # the hash of the produced `dist/`. Bump `outputHash` whenever the
        # frontend sources or dependencies change (nix will print the new hash).
        frontendSrc = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./package.json
            ./bun.lock
            ./index.html
            ./svelte.config.js
            ./tsconfig.json
            ./tsconfig.node.json
            ./vite.config.ts
            ./src
          ];
        };

        frontend = pkgs.stdenvNoCC.mkDerivation {
          pname = "sonora-frontend";
          inherit version;
          src = frontendSrc;
          nativeBuildInputs = [ pkgs.bun pkgs.nodejs_22 pkgs.cacert ];
          dontConfigure = true;
          SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
          buildPhase = ''
            runHook preBuild
            export HOME="$TMPDIR"
            bun install --frozen-lockfile --no-progress
            # Vite's bin uses a `#!/usr/bin/env node` shebang, which fails in the
            # sandbox (no /usr/bin/env) — rewrite the dep shebangs to nix node.
            patchShebangs node_modules
            bun run build
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            cp -r dist "$out"
            runHook postInstall
          '';
          outputHashMode = "recursive";
          outputHashAlgo = "sha256";
          outputHash = "sha256-vh9hyphiyms2wgdiWNJduEXXFfmuns5UvsEF1/SDP6s=";
        };

        # Desktop launcher entry (menu / app grid integration).
        desktopItem = pkgs.makeDesktopItem {
          name = "sonora";
          desktopName = "Sonora";
          genericName = "Dictée vocale";
          comment = "Barre flottante de transcription vocale en temps réel";
          exec = "sonora";
          icon = "sonora";
          terminal = false;
          startupNotify = false;
          categories = [ "Utility" "Audio" "AudioVideo" ];
          keywords = [ "speech" "dictation" "transcription" "voice" "stt" ];
        };

        # ---- The app: Rust binary with the frontend embedded -------------------
        sonora = pkgs.rustPlatform.buildRustPackage {
          pname = "sonora";
          inherit version;
          src = ./.;
          cargoRoot = "src-tauri";
          buildAndTestSubdir = "src-tauri";
          cargoLock.lockFile = ./src-tauri/Cargo.lock;

          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake # whisper.cpp
            rustPlatform.bindgenHook # whisper-rs bindgen (libclang)
            wrapGAppsHook3
            gobject-introspection
            copyDesktopItems # installs `desktopItems` into share/applications
          ];

          desktopItems = [ desktopItem ];

          buildInputs = with pkgs; [
            glib
            gtk3
            webkitgtk_4_1
            cairo
            pango
            gdk-pixbuf
            librsvg
            harfbuzz
            atk
            libsoup_3
            glib-networking
            openssl
            dbus
            libsecret
            libayatana-appindicator
            alsa-lib
            libxcb
            libx11
            libxcursor
            libxrandr
            libxi
            wayland
            libxkbcommon
          ];

          # native-tls / openssl-sys: link the system OpenSSL, don't vendor.
          env.OPENSSL_NO_VENDOR = "1";

          # `tauri::generate_context!` embeds ../dist (relative to src-tauri).
          postPatch = ''
            rm -rf dist
            cp -r ${frontend} dist
            chmod -R u+w dist
          '';

          # Tauri only embeds the frontend when the `custom-protocol` feature is
          # enabled; otherwise `generate_context!` builds in dev mode (empty
          # assets + dev URL) → blank window. The tauri CLI turns this on for
          # `tauri build`; a plain cargo build (buildRustPackage) must opt in.
          buildFeatures = [ "tauri/custom-protocol" ];

          # Only the binary target is needed (the lib is also staticlib+cdylib
          # for mobile, which we don't ship).
          cargoBuildFlags = [ "--bin" "sonora" ];

          # Install hicolor icons (+ a scalable SVG) so the .desktop entry shows
          # an icon in app launchers / the GNOME-Shell app grid.
          postInstall = ''
            install -Dm644 "${iconsDir}/32x32.png"      "$out/share/icons/hicolor/32x32/apps/sonora.png"
            install -Dm644 "${iconsDir}/128x128.png"    "$out/share/icons/hicolor/128x128/apps/sonora.png"
            install -Dm644 "${iconsDir}/128x128@2x.png" "$out/share/icons/hicolor/256x256/apps/sonora.png"
            install -Dm644 "${./brand/logo.svg}"        "$out/share/icons/hicolor/scalable/apps/sonora.svg"
          '';

          # The system-tray crate (libappindicator-sys) dlopen()s
          # libayatana-appindicator3.so.1 at runtime, so it isn't in the binary's
          # RPATH — expose it on LD_LIBRARY_PATH via the GApps wrapper, otherwise
          # the app panics at launch ("Failed to load ayatana-appindicator3").
          preFixup = ''
            gappsWrapperArgs+=(
              --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath [ pkgs.libayatana-appindicator ]}"
            )
          '';

          doCheck = false;

          meta = {
            description = "Real-time speech-to-text floating bar with pluggable models";
            homepage = "https://github.com/Devitek/sonora";
            license = lib.licenses.mit;
            mainProgram = "sonora";
            platforms = lib.platforms.linux;
          };
        };
      in
      {
        packages.default = sonora;
        packages.sonora = sonora;
        packages.frontend = frontend;

        apps.default = {
          type = "app";
          program = "${sonora}/bin/sonora";
        };

        devShells.default = pkgs.mkShell {
          packages = buildTools ++ runtimeLibs;

          # cpal/whisper-rs sometimes need a libclang for bindgen later on.
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibs}:''${LD_LIBRARY_PATH:-}"
            export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" runtimeLibs}:''${PKG_CONFIG_PATH:-}"
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
            export GI_TYPELIB_PATH="${pkgs.lib.makeSearchPath "lib/girepository-1.0" giLibs}:''${GI_TYPELIB_PATH:-}"
            # NB: do NOT set WEBKIT_DISABLE_COMPOSITING_MODE — it disables the
            # compositor and breaks window transparency / backdrop-filter (the
            # floating-bar look). Re-enable only if you hit GPU rendering issues.
            # Credentials are loaded by the app itself from .env (see main.rs),
            # so editing .env takes effect on the next launch without re-sourcing.
            echo "Sonora devshell ready — run: bun install && bun run tauri dev"
          '';
        };
      });
}
