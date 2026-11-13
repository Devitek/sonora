<p align="center">
  <img src="brand/logo.svg" alt="" width="116" height="116">
</p>

<h1 align="center">Sonora</h1>

<p align="center">Real-time voice dictation for macOS, Windows and Linux, typed at your cursor.</p>

<p align="center">
  <a href="https://github.com/Devitek/sonora/releases/latest"><img src="https://img.shields.io/github/v/release/Devitek/sonora?style=flat-square&label=release&labelColor=141517&color=F0703A" alt="Latest release"></a>
  <a href="https://github.com/Devitek/sonora/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Devitek/sonora/ci.yml?branch=main&style=flat-square&label=ci&labelColor=141517" alt="CI"></a>
  <a href="https://github.com/Devitek/sonora/releases"><img src="https://img.shields.io/github/downloads/Devitek/sonora/total?style=flat-square&label=downloads&labelColor=141517&color=F0703A" alt="Downloads"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-F6F5F2?style=flat-square&labelColor=141517" alt="License"></a>
</p>

<p align="center">
  <a href="https://sonora.devitek.fr/">Website</a> ·
  <a href="#installation">Installation</a> ·
  <a href="#usage">Usage</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="CONTRIBUTING.md">Contributing</a> ·
  <a href="README.fr.md">Français</a>
</p>

Sonora is a Spotlight-style floating bar, always a shortcut away: press, speak, and the text appears live, then lands on your clipboard or is typed straight at your cursor. Plug in the engine you prefer (Gemini, Mistral, OpenAI, Groq, any OpenAI-compatible endpoint) or run Whisper fully offline. A free, open-source alternative to Wispr Flow and superwhisper.

> [!NOTE]
> Sonora was designed and coded end to end by an AI agent, paired with a human who steered and tested it. Take it as a demonstration of what can be built this way, and review anything security-related.

## Installation

Download the build for your system from the [latest release](https://github.com/Devitek/sonora/releases/latest): `.dmg` (Apple Silicon and Intel), `.msi` or `.exe` (Windows), `.AppImage` or `.deb` (Linux).

**NixOS / Nix**: the flake provides a native package, recommended over the AppImage (which trips on EGL / bubblewrap drivers):

```sh
nix run github:Devitek/sonora
```

## Usage

Open the **Settings** (⚙ on the bar), choose a transcription provider and paste its API key (stored in the OS keychain). Optionally pick the microphone, model, language, base URL or local `ggml` model, enable automatic cleanup and define reformulation prompts. Then start and stop dictation with the global shortcut.

| Provider | Transcription | Reformulation | Key required | Offline |
| --- | --- | :---: | :---: | :---: |
| **Gemini Live** | streaming, word by word | ✓ | yes | — |
| **Mistral (Voxtral)** | per segment | ✓ | yes | — |
| **OpenAI Whisper** | per segment | ✓ | yes | — |
| **Groq Whisper** | per segment | ✓ | yes | — |
| **OpenAI-compatible** | per segment | ✓ | host-dependent | possible |
| **Local Whisper** (ggml) | per segment | — | no | ✓ |

"Per segment" splits speech with voice-activity detection and transcribes each segment; Gemini Live streams the text continuously.

## Features

- **Streaming dictation**: text appears as you speak (Gemini Live).
- **Type at cursor**, or copy to the clipboard.
- **Global shortcut**: start and stop without leaving your app.
- **Hesitation cleanup**: strips "uh", false starts and repetitions.
- **Reformulation prompts**: run a dictation through an LLM with your own prompts ("rewrite formally", "convert to a terminal command"…).
- **History** of past dictations, light / dark / system **theme**, live **waveform**.
- **API keys in the OS keychain** (`0600` file fallback), never in plaintext on the frontend.

## Floating bar on Hyprland

Sonora is a transparent, decoration-less, focus-less window. Under a tiling compositor, float it and bind the shortcut (recent Hyprland `match:` syntax):

```ini
windowrule = float on,            match:title ^(Sonora)$
windowrule = move (monitor_w/2)-240 40, match:title ^(Sonora)$
windowrule = border_size 0,       match:title ^(Sonora)$
windowrule = no_shadow on,        match:title ^(Sonora)$
windowrule = rounding 0,          match:title ^(Sonora)$
windowrule = no_blur on,          match:title ^(Sonora)$
windowrule = pin on,              match:title ^(Sonora)$
windowrule = no_initial_focus on, match:title ^(Sonora)$

# Push-to-talk: a 2nd instance forwards the action to the running one
bind = SUPER, V, exec, sonora toggle
```

## Development

Requires Rust (stable), Bun and the [Tauri system dependencies](https://tauri.app/start/prerequisites/).

```sh
bun install
bun run tauri dev      # development
bun run tauri build    # binary in src-tauri/target/release/sonora
```

On Nix, `nix develop` provides the full toolchain; then `bash scripts/dev.sh`. Settings can also come from a `.env` file (see [`.env.example`](.env.example)). CI type-checks the Svelte code and runs `cargo fmt`, `clippy -D warnings` and `cargo check` on Windows and macOS, plus the Nix build, CodeQL and dependency audits; releases are cut from *Actions → Release* with a SemVer bump.

## Documentation

- [Website and documentation](https://sonora.devitek.fr/)
- [Brand assets](brand/): logo, symbol, colours (purple `#7C5CFF` → cyan `#22D3EE`, background `#0A0B12`), typography

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) first: one issue per change, Conventional Commits, green CI.

## License

Released under the [MIT License](LICENSE).

---

<p align="center">
  <a href="https://github.com/Devitek">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset=".github/assets/devitek-wordmark-blanc.svg">
      <img src=".github/assets/devitek-wordmark-encre.svg" alt="Devitek" height="22">
    </picture>
  </a>
  <br>
  <sub>Built and maintained by <a href="https://github.com/Devitek">Devitek</a></sub>
</p>
