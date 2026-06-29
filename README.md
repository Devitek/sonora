<p align="center">
  <img src="brand/logo.svg" alt="Sonora" width="116" height="116">
</p>

<h1 align="center">Sonora</h1>

<p align="center">
  <em>Transcription vocale en temps réel, sur votre bureau — modèles cloud ou 100 % locaux.</em>
</p>

<p align="center">
  <a href="https://github.com/Devitek/sonora/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/Devitek/sonora/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/Devitek/sonora/actions/workflows/release.yml"><img alt="Release" src="https://github.com/Devitek/sonora/actions/workflows/release.yml/badge.svg"></a>
  <img alt="platform" src="https://img.shields.io/badge/macOS%20·%20Windows%20·%20Linux-0a0b12?style=flat-square">
  <img alt="built with" src="https://img.shields.io/badge/Tauri%20·%20Svelte%205%20·%20Rust-7C5CFF?style=flat-square">
  <img alt="license" src="https://img.shields.io/badge/license-MIT-22D3EE?style=flat-square">
</p>

<p align="center">
  <strong><a href="https://devitek.github.io/sonora/">🌐 Site &amp; documentation</a></strong>
</p>

> 🤖 **Projet open source, intégralement construit avec une IA — et c'est assumé.**
> Sonora a été conçu et codé de bout en bout par un agent IA (architecture, backend
> Rust/Tauri, frontend Svelte 5, CI/CD, design de marque), en binôme avec un humain qui
> orientait et testait sur sa machine. Le code, les commits et ce README sont le résultat
> de cette collaboration. À prendre comme une démonstration de ce qu'on peut bâtir ainsi —
> avec ses qualités… et la nécessité de relire ce qui touche à la sécurité.

---

## ✦ C'est quoi

Une **barre flottante façon Spotlight**, toujours à portée de raccourci. Vous appuyez, vous
parlez, le texte s'écrit **en direct** — puis il est copié dans le presse-papier ou **tapé
directement à l'endroit de votre curseur**. Branchez le moteur de votre choix : **Gemini**,
**Mistral**, **OpenAI**, **Groq**, n'importe quel endpoint **compatible OpenAI**, ou
**Whisper 100 % local et hors-ligne**.

## Fonctionnalités

- 🎙️ **Dictée en streaming** — le texte apparaît au fur et à mesure (avec Gemini Live).
- ⌨️ **Collage au curseur** — le résultat est tapé là où vous êtes (ou copié au presse-papier).
- ⌘ **Raccourci global** — démarrez/arrêtez sans quitter votre application en cours.
- 🔌 **Modèles enfichables** — Gemini Live, Mistral (Voxtral), OpenAI Whisper, Groq,
  OpenAI-compatible, Whisper local.
- ✦ **Nettoyage des hésitations** — passe optionnelle qui retire les « euh », faux départs,
  répétitions.
- ✍️ **Prompts de reformulation** — transformez une dictée via un LLM avec vos propres prompts
  (« reformuler de manière formelle », « convertir en commande terminal », « réécrire pro »…).
- 🕘 **Historique** — retrouvez et recopiez vos dictées précédentes.
- 🌗 **Thème** clair / sombre / système.
- 〰️ **Waveform en direct** pilotée par le vrai niveau audio.
- 🔐 **Clés API au trousseau** — stockées dans le keyring de l'OS (repli fichier `0600`),
  jamais en clair côté front.
- 🖥️ **Multiplateforme** — un binaire léger (Tauri) sur macOS, Windows et Linux.

## Providers

| Fournisseur          | Transcription      | Reformulation | Clé requise | Hors-ligne |
| -------------------- | ------------------ | :-----------: | :---------: | :--------: |
| **Gemini Live**      | streaming, mot-à-mot | ✓           |     oui     |     —      |
| **Mistral (Voxtral)**| par segment        | ✓             |     oui     |     —      |
| **OpenAI Whisper**   | par segment        | ✓             |     oui     |     —      |
| **Groq Whisper**     | par segment        | ✓             |     oui     |     —      |
| **OpenAI-compatible**| par segment        | ✓             | selon hôte  |  possible  |
| **Whisper local** (ggml) | par segment    |       —       |     non     |     ✓      |

> La transcription « par segment » découpe la parole via une détection d'activité vocale (VAD)
> et transcrit chaque segment ; Gemini Live, lui, renvoie le texte en continu.

## Installation / développement

Prérequis : **Rust** (stable), **Bun**, et les [dépendances système Tauri](https://tauri.app/start/prerequisites/)
(WebKitGTK, etc.).

```bash
bun install
bun run tauri dev      # développement
bun run tauri build    # build de production (binaire dans src-tauri/target/release/sonora)
```

**NixOS / Nix** — un flake fournit tout le devshell (WebKitGTK, ALSA, cmake, wtype…) :

```bash
nix develop
bash scripts/dev.sh    # libère le port Vite résiduel puis lance `tauri dev`
```

## Configuration (⚙)

Ouvrez le menu **☰ → Réglages** :

1. Choisissez un **fournisseur** de transcription.
2. Collez la **clé API** correspondante (stockée dans le trousseau de l'OS).
3. Optionnel : modèle, langue, URL de base (endpoints compatibles), chemin du modèle `ggml`
   (Whisper local).
4. Optionnel : activez le **nettoyage automatique**, choisissez son **moteur de reformulation**,
   et définissez vos **prompts** personnalisés.

> Pour le développement, ces réglages peuvent aussi venir d'un fichier `.env` (voir
> [`.env.example`](.env.example)).

## Barre flottante sous Linux (Hyprland)

Sonora est une fenêtre **transparente, sans décorations, sans focus**. Sous un compositeur
tuilant comme Hyprland, faites-la flotter et bindez le raccourci global. Syntaxe pour les
versions récentes de Hyprland (sélecteur `match:`) :

```ini
windowrule = float on,            match:title ^(Sonora)$
windowrule = move (monitor_w/2)-240 40, match:title ^(Sonora)$
windowrule = border_size 0,       match:title ^(Sonora)$
windowrule = no_shadow on,        match:title ^(Sonora)$
windowrule = rounding 0,          match:title ^(Sonora)$
windowrule = no_blur on,          match:title ^(Sonora)$
windowrule = pin on,              match:title ^(Sonora)$
windowrule = no_initial_focus on, match:title ^(Sonora)$

# Push-to-talk : lance une 2ᵉ instance qui transmet l'action à celle en cours (single-instance)
bind = SUPER, V, exec, sonora toggle
```

## Releases & CI

- **CI** ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) : sur chaque push/PR,
  type-check Svelte + `cargo fmt` + `clippy -D warnings`.
- **Release** ([`.github/workflows/release.yml`](.github/workflows/release.yml)) : *Actions →
  Release → Run workflow* avec un bump **semver** (patch/minor/major) → versionne, tague
  `vX.Y.Z`, et publie les binaires **Linux** (`.AppImage`, `.deb`), **macOS** (`.dmg`,
  Apple Silicon + Intel) et **Windows** (`.msi`, NSIS) sur la Release GitHub.

## Marque

Ressources visuelles dans [`brand/`](brand/) :

- `logo.svg` — icône (dégradé, fond arrondi) · `logo-mark.svg` / `logo-mark-white.svg` — symbole seul
- Couleurs : violet `#7C5CFF` → cyan `#22D3EE` · fond sombre `#0A0B12`
- Typographie : Space Grotesk (titres) · Inter (UI) · JetBrains Mono (détails)

## Licence

[MIT](LICENSE) © 2026
