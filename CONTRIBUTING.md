# Contributing to Sonora

Thanks for your interest in **Sonora** — a cross-platform floating dictation bar
(Tauri 2 + Svelte 5 + Rust). All contributions are welcome: bug reports,
suggestions, docs, or code.

> **Language.** The app's UI strings and product copy are in **French** (keep new
> user-facing strings in French). You may open issues and PRs in **English or
> French** — whatever is easier for you.

## Table of contents

- [Before you start](#before-you-start)
- [Setting up the environment](#setting-up-the-environment)
- [Project layout](#project-layout)
- [Running in development](#running-in-development)
- [Checks before opening a PR](#checks-before-opening-a-pr)
- [The golden rule: permissions & platforms](#the-golden-rule-permissions--platforms)
- [Conventions](#conventions)
- [Dependencies & security](#dependencies--security)
- [Pull request process](#pull-request-process)
- [Releases (maintainers)](#releases-maintainers)
- [Reporting a bug / requesting a feature](#reporting-a-bug--requesting-a-feature)
- [License](#license)

## Before you start

- For a **non-trivial change**, open an **issue** first to discuss it (avoids
  building in a direction that won't be accepted).
- Search [existing issues](https://github.com/Devitek/sonora/issues) before
  opening a new one.
- Small fixes (typo, docs, lint) can go straight to a PR.

## Setting up the environment

**Prerequisites:** [Rust](https://rustup.rs/) stable, [Bun](https://bun.sh/), and
the [Tauri system dependencies](https://tauri.app/start/prerequisites/)
(WebKitGTK, ALSA, etc. on Linux).

```bash
bun install
```

**On Nix / NixOS** (recommended — provides WebKitGTK, ALSA, cmake, wtype… in a
reproducible devshell):

```bash
nix develop            # enter the devshell
bun install
```

## Project layout

| Path | Purpose |
| --- | --- |
| `src/` | Svelte 5 frontend (floating bar + History/Settings window) |
| `src/lib/` | Shared utilities (providers, settings, clipboard, types…) |
| `src-tauri/src/` | Rust backend (audio, transcription providers, secrets, keystroke output…) |
| `src-tauri/capabilities/` | Tauri ACL permissions |
| `flake.nix` | Native Nix package + devshell |
| `scripts/` | Tooling (dev, screenshots, version bump, deps hash…) |
| `docs/` | GitHub Pages mini-site |
| `.github/workflows/` | CI, release, security, update bots |

## Running in development

```bash
bun run tauri dev          # or: bash scripts/dev.sh  (frees a stale Vite port first)
```

Local production build:

```bash
bun run tauri build        # binary in src-tauri/target/release/sonora
```

## Checks before opening a PR

These commands match **exactly** what CI runs — run them locally for a fast
feedback loop:

```bash
# Frontend: expect 0 errors
bun run check

# Rust: format + lints (warnings are errors)
cargo fmt    --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# Nix package (if you touch packaging, the frontend, or dependencies)
nix build .#default
```

CI additionally runs a **`cargo check` on Windows and macOS**: a dependency that
only builds on Linux isn't enough. If your change touches the audio/native
backend, keep all three OSes in mind.

## The golden rule: permissions & platforms

Before considering a feature "done", check what could **prevent it from running**
on each platform — both **OS permissions** and **Tauri ACL capabilities**. A
feature that works in `tauri dev` often fails elsewhere (mic prompt, tray, EGL,
window position…).

In short, ask yourself:

1. **Tauri ACL** (`src-tauri/capabilities/default.json`) — any new `core:*` or
   plugin call (`window.setPosition`, `clipboard`, `global-shortcut`, `store`…)
   needs an `allow-*` rule, or it is **silently denied** at runtime. (App-defined
   `#[tauri::command]`s don't need one.)
2. **macOS** — TCC-gated resource (microphone → `NS*UsageDescription` in
   `Info.plist`), Accessibility for type-at-cursor, Gatekeeper for unsigned builds.
3. **Windows** — microphone privacy setting, SmartScreen for unsigned installers.
4. **Linux** — `dlopen`'d tools/libs: `wtype`/`xdotool` (type-at-cursor),
   `libayatana-appindicator` (tray), WebKitGTK EGL/DMABUF quirks.

The full details (and the permission matrix) live in [`AGENTS.md`](AGENTS.md) —
read it for any change touching microphone, tray, windows, type-at-cursor, or
packaging.

## Conventions

- **Language:** keep UI strings and labels in **French**.
- **Commits:** concise [Conventional Commits](https://www.conventionalcommits.org/)
  (e.g. `feat(audio): …`, `fix(nix): …`, `ci: …`, `docs: …`).
- **Issues in commits:** use `Refs #N` while a fix isn't yet confirmed by the
  reporter; only use `Fixes/Closes #N` when you really want to close the issue on
  merge.
- **Style:** let `cargo fmt` and the frontend formatter decide; avoid unrelated
  mass-reformatting.

## Dependencies & security

- Updates are automated: **Dependabot** (Rust crates + GitHub Actions) plus
  in-house bots for **bun** (`update-bun.yml`) and **`flake.lock`**
  (`update-flake-lock.yml`).
- **If you change frontend dependencies** (`package.json` / `bun.lock`), the
  `bunDepsHash` in `flake.nix` must follow. Run:
  ```bash
  bash scripts/sync-bun-deps-hash.sh   # auto re-pin (no-op if already correct)
  ```
- **Security:** `cargo audit` (RustSec) and `bun audit` run in CI; **CodeQL**
  analyses the code (JS/TS + Rust). A PR introducing a known vulnerability is
  blocked. Please **never** commit an API key / secret (keys live in the OS
  keychain, never in plaintext).

## Pull request process

1. Fork and create a branch off `main` (`feat/my-feature`, `fix/the-bug`…).
2. Make the [checks](#checks-before-opening-a-pr) pass locally.
3. Open the PR against `main` and fill in the template. Describe the **what** and
   the **why**, and the **platform(s) tested**.
4. CI must be **green** (frontend, rust, `cargo check` Windows/macOS, Nix,
   CodeQL, audits). Address review feedback.
5. Merge is done via **squash** once the PR is approved.

## Releases (maintainers)

```bash
node scripts/bump-version.mjs <patch|minor|major>   # syncs package.json / tauri.conf.json / Cargo.toml
git commit -am "chore(release): vX.Y.Z"
git tag -a vX.Y.Z -m "vX.Y.Z" && git push origin main vX.Y.Z
```

Pushing the tag triggers the cross-platform build and publish (Linux / macOS /
Windows).

## Reporting a bug / requesting a feature

Open an [issue](https://github.com/Devitek/sonora/issues/new/choose) and pick the
right template (**Bug report** or **Feature request**). The more context (OS,
install method, version, transcription provider, logs), the faster it gets
triaged.

## License

By contributing, you agree that your contribution is published under the
project's **MIT** license (see [`LICENSE`](LICENSE)).
