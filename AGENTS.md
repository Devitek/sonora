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

## Documentation & website — keep EN + FR in sync, only when needed

User-facing documentation exists in **English (default) and French**, and the
two must stay in sync. **Only touch docs when a change actually affects users** —
a new or changed feature, flag, permission, install step, provider, or shortcut.
**Skip doc updates for purely internal work** (refactors, CI, dependencies,
packaging internals, tests) — don't churn the docs needlessly.

When an update IS warranted, update **all** the relevant surfaces, in **both
languages**:

- `README.md` (EN) **and** `README.fr.md` (FR).
- `CONTRIBUTING.md` (EN) when the contributor workflow/commands change.
- The site under `docs/`: English at the root (`docs/index.html`,
  `docs/docs.html`) **and** French under `docs/fr/`. Keep anchors, the language
  switcher, `hreflang` tags and asset paths consistent (`fr/` pages reference
  `../assets/...`). Regenerate screenshots via `scripts/screenshots.sh` if the UI
  changed.
- **Release notes are written in English.**

The golden rule still applies on top of this: any user-granted permission must be
documented in the README(s) **and** the site, in both languages.

## Conventions

- **Languages:** agent replies follow the user; the app's **UI strings stay
  French**; user-facing **docs, website and release notes are English-default +
  French**, kept in sync (see the section above).
- Commit messages: conventional, concise.
- Don't auto-close issues from commit messages until the reporter confirms — use
  `Refs #N`, not `Fixes #N`, while a fix is pending verification.
