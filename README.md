<p align="center">
  <img src="brand/logo.svg" alt="Sonora" width="116" height="116">
</p>

<h1 align="center">Sonora</h1>

<p align="center">
  <em>Real-time speech-to-text on your desktop — cloud or 100% local models.</em>
</p>

<p align="center">
  <a href="https://github.com/Devitek/sonora/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Devitek/sonora/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/Devitek/sonora/actions/workflows/release.yml"><img alt="Release" src="https://github.com/Devitek/sonora/actions/workflows/release.yml/badge.svg"></a>
  <img alt="platform" src="https://img.shields.io/badge/macOS%20·%20Windows%20·%20Linux-0a0b12?style=flat-square">
  <img alt="built with" src="https://img.shields.io/badge/Tauri%20·%20Svelte%205%20·%20Rust-7C5CFF?style=flat-square">
  <img alt="license" src="https://img.shields.io/badge/license-MIT-22D3EE?style=flat-square">
</p>

<p align="center">
  <strong>English</strong> · <a href="README.fr.md">Français</a>
  &nbsp;·&nbsp;
  <strong><a href="https://devitek.github.io/sonora/">🌐 Website &amp; documentation</a></strong>
</p>

> 🤖 **Open source, fully built with AI — and we own it.**
> Sonora was designed and coded end to end by an AI agent (architecture, Rust/Tauri
> backend, Svelte 5 frontend, CI/CD, brand design), paired with a human who steered
> and tested on their machine. The code, the commits and this README are the result
> of that collaboration. Take it as a demonstration of what can be built this way —
> with its strengths… and the need to review anything security-related.

---

## ✦ What is it

A **Spotlight-style floating bar**, always a shortcut away. You press, you speak, the
text appears **live** — then it's copied to the clipboard or **typed straight at your
cursor**. Plug in the engine of your choice: **Gemini**, **Mistral**, **OpenAI**,
**Groq**, any **OpenAI-compatible** endpoint, or **fully local, offline Whisper**.

## Features

- 🎙️ **Streaming dictation** — text appears as you speak (with Gemini Live).
- ⌨️ **Type at cursor** — the result is typed where you are (or copied to the clipboard).
- ⌘ **Global shortcut** — start/stop without leaving your current app.
- 🔌 **Pluggable models** — Gemini Live, Mistral (Voxtral), OpenAI Whisper, Groq,
  OpenAI-compatible, local Whisper.
- ✦ **Hesitation cleanup** — optional pass that strips "uh", false starts, repetitions.
- ✍️ **Reformulation prompts** — transform a dictation through an LLM with your own
  prompts ("rewrite formally", "convert to a terminal command", "make it professional"…).
- 🕘 **History** — find and re-copy your previous dictations.
- 🌗 **Theme** — light / dark / system.
- 〰️ **Live waveform** driven by the real audio level.
- 🔐 **API keys in the keychain** — stored in the OS keyring (`0600` file fallback),
  never in plaintext on the frontend.
- 🖥️ **Cross-platform** — a lightweight binary (Tauri) on macOS, Windows and Linux.

## Providers

| Provider             | Transcription        | Reformulation | Key required | Offline |
| -------------------- | -------------------- | :-----------: | :----------: | :-----: |
| **Gemini Live**      | streaming, word-by-word | ✓          |     yes      |    —    |
| **Mistral (Voxtral)**| per segment          | ✓             |     yes      |    —    |
| **OpenAI Whisper**   | per segment          | ✓             |     yes      |    —    |
| **Groq Whisper**     | per segment          | ✓             |     yes      |    —    |
| **OpenAI-compatible**| per segment          | ✓             | host-dependent | possible |
| **Local Whisper** (ggml) | per segment      |       —       |      no      |    ✓    |

> "Per segment" transcription splits speech with voice-activity detection (VAD) and
> transcribes each segment; Gemini Live instead streams the text continuously.

## Install / development

Prerequisites: **Rust** (stable), **Bun**, and the
[Tauri system dependencies](https://tauri.app/start/prerequisites/) (WebKitGTK, etc.).

```bash
bun install
bun run tauri dev      # development
bun run tauri build    # production build (binary in src-tauri/target/release/sonora)
```

**NixOS / Nix** — the flake exposes a native package (recommended on NixOS over the
AppImage, which trips on EGL/bubblewrap drivers):

```bash
nix run github:Devitek/sonora      # run directly
nix build github:Devitek/sonora    # -> ./result/bin/sonora
```

…and a complete devshell (WebKitGTK, ALSA, cmake, wtype…) for development:

```bash
nix develop
bash scripts/dev.sh    # frees a stale Vite port, then runs `tauri dev`
```

## Configuration (⚙)

Open the **Settings** (⚙ icon on the bar):

1. Choose a transcription **provider**.
2. Paste the matching **API key** (stored in the OS keychain).
3. Optional: pick the **microphone** (capture source; "System" follows the OS default
   mic, ↻ refreshes the list).
4. Optional: model, language, base URL (compatible endpoints), `ggml` model path
   (local Whisper).
5. Optional: enable **automatic cleanup**, choose its **reformulation engine**, and
   define your custom **prompts**.

> For development, these settings can also come from a `.env` file (see
> [`.env.example`](.env.example)).

## Floating bar on Linux (Hyprland)

Sonora is a **transparent, decoration-less, focus-less** window. Under a tiling
compositor like Hyprland, float it and bind the global shortcut. Syntax for recent
Hyprland versions (`match:` selector):

```ini
windowrule = float on,            match:title ^(Sonora)$
windowrule = move (monitor_w/2)-240 40, match:title ^(Sonora)$
windowrule = border_size 0,       match:title ^(Sonora)$
windowrule = no_shadow on,        match:title ^(Sonora)$
windowrule = rounding 0,          match:title ^(Sonora)$
windowrule = no_blur on,          match:title ^(Sonora)$
windowrule = pin on,              match:title ^(Sonora)$
windowrule = no_initial_focus on, match:title ^(Sonora)$

# Push-to-talk: launch a 2nd instance that forwards the action to the running one (single-instance)
bind = SUPER, V, exec, sonora toggle
```

## Releases & CI

- **CI** ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)): on every push/PR,
  Svelte type-check + `cargo fmt` + `clippy -D warnings` + `cargo check` on **Windows
  and macOS**. Plus the **Nix** build, **CodeQL** analysis and dependency audits
  (`cargo audit` / `bun audit`).
- **Release** ([`.github/workflows/release.yml`](.github/workflows/release.yml)):
  *Actions → Release → Run workflow* with a **semver** bump (patch/minor/major) →
  versions, tags `vX.Y.Z`, and publishes **Linux** (`.AppImage`, `.deb`), **macOS**
  (`.dmg`, Apple Silicon + Intel) and **Windows** (`.msi`, NSIS) binaries on the
  GitHub Release.

## Contributing

Contributions are welcome! See the **[contributing guide](CONTRIBUTING.md)** (setup,
verification commands, conventions, the permissions/platforms golden rule). To report a
bug or request a feature, open an
[issue](https://github.com/Devitek/sonora/issues/new/choose) with the right template.

## Brand

Visual assets in [`brand/`](brand/):

- `logo.svg` — icon (gradient, rounded background) · `logo-mark.svg` /
  `logo-mark-white.svg` — symbol only
- Colors: purple `#7C5CFF` → cyan `#22D3EE` · dark background `#0A0B12`
- Typography: Space Grotesk (titles) · Inter (UI) · JetBrains Mono (details)

## License

[MIT](LICENSE) © 2026
