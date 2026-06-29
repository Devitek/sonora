# AGENTS.md — guidelines for AI agents working on Sonora

Sonora is a Tauri 2 + Svelte 5 + Rust floating-bar dictation app shipped on
macOS, Windows and Linux (and as a native Nix package).

## ⚠️ Golden rule: verify platform prerequisites & permissions

Before considering any feature "done", check what would **block it from running
normally on each target platform** — both **OS permissions** and **Tauri ACL
capabilities**. A feature that works in `tauri dev` on one machine routinely
fails elsewhere because of a missing permission, usage-description, entitlement,
runtime library, or ACL allow-rule. This has bitten us repeatedly (mic prompt,
system tray, EGL, window-position persistence).

When you add or change a feature, ask:

1. **Tauri ACL capabilities** — `src-tauri/capabilities/default.json`.
   Does the frontend call any new `core:*` or plugin command (e.g.
   `window.setPosition`, `availableMonitors`, `clipboard`, `global-shortcut`,
   `store`)? Each needs an explicit `allow-*` entry or it is **silently denied
   at runtime**. App-defined `#[tauri::command]`s do **not** need ACL entries.
   Valid permission ids are listed in `src-tauri/gen/schemas/desktop-schema.json`.

2. **macOS** — does it touch a TCC-gated resource or a private API?
   - Microphone / Camera / etc. → add the matching `NS*UsageDescription` to
     `src-tauri/Info.plist` (Tauri auto-merges it), otherwise the system prompt
     **never appears** and access is silently denied.
   - Synthesising input / reading other apps (enigo "type at cursor") → requires
     **Accessibility** permission, granted by the user in System Settings
     (no Info.plist key triggers it). Document it; consider an in-app hint.
   - Unsigned builds → Gatekeeper (`right-click → Open`).

3. **Windows** — the microphone is gated by a system privacy setting
   ("let desktop apps access your microphone"); unsigned installers trigger
   SmartScreen. Document both.

4. **Linux** — runtime tools/libs the app shells out to or `dlopen()`s:
   `wtype` (Wayland) / `xdotool` (X11) for type-at-cursor,
   `libayatana-appindicator` for the tray, and WebKitGTK DMABUF/EGL quirks.
   Declare/bundle them; the Nix package must add `dlopen`'d libs to the wrapper
   `LD_LIBRARY_PATH` (they are not in the binary RPATH).

Always **document** a user-granted permission in the docs site + README, and
prefer a clear onboarding / in-app hint when one is required.

## Current permission / prerequisite matrix

| Need | Platform | How it's handled |
| --- | --- | --- |
| Microphone | macOS | `NSMicrophoneUsageDescription` in `src-tauri/Info.plist` |
| Microphone | Windows | system privacy setting (documented) |
| Accessibility (type-at-cursor) | macOS | user-granted in System Settings (documented) |
| type-at-cursor backend | Linux | `wtype` (Wayland) / `xdotool` (X11) |
| system tray | Linux (Nix) | `libayatana-appindicator` on wrapper `LD_LIBRARY_PATH` |
| EGL `EGL_BAD_PARAMETER` crash | Linux | `WEBKIT_DISABLE_DMABUF_RENDERER=1` set in `src-tauri/src/main.rs` |
| Wayland fractional-scale | Linux | force `GDK_BACKEND=x11` in `main.rs` |
| window move + persist position | all | `core:window:allow-set-position` + `allow-available-monitors` |

## Build / verify before claiming done

- Frontend type-check: `bun run check` (must be 0 errors).
- Rust: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
  and `cargo fmt --manifest-path src-tauri/Cargo.toml --check`.
- Native Nix package: `nix build .#default` (also runs in CI via `.github/workflows/nix.yml`).
- Real WebKitGTK layout / screenshots harness: `scripts/inspect-layout.sh`,
  `scripts/screenshots.sh` (run via `nix develop --command`).
- Releases: bump with `node scripts/bump-version.mjs <patch|minor|major>`, then
  tag `vX.Y.Z` and push (tag-push builds & publishes all platforms).

## Conventions

- Replies and UI strings: **French**. Commit messages: conventional, concise.
- Don't auto-close issues from commit messages until the reporter confirms — use
  `Refs #N`, not `Fixes #N`, while a fix is pending verification.
