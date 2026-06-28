<!-- Remplacez la bannière par celle du nom retenu : banner-ditto.png ou banner-sonora.png -->
<p align="center">
  <img src="brand/png/banner-sonora.png" alt="Sonora" width="100%">
</p>

<p align="center">
  <em>Transcription vocale en temps réel, sur votre bureau — modèles cloud ou locaux.</em>
</p>

<p align="center">
  <img alt="platform" src="https://img.shields.io/badge/macOS%20·%20Windows%20·%20Linux-0a0b12?style=flat-square">
  <img alt="built with" src="https://img.shields.io/badge/Tauri%20·%20Svelte%205%20·%20Rust-7C5CFF?style=flat-square">
  <img alt="license" src="https://img.shields.io/badge/license-MIT-22D3EE?style=flat-square">
</p>

---

## ✦ C'est quoi

Une barre flottante, toujours à portée de raccourci. Vous appuyez, vous parlez, le texte
s'écrit en direct — puis il est copié dans le presse-papier ou **collé directement à l'endroit
de votre curseur**. Branchez le moteur de votre choix : Gemini, OpenAI, Groq, n'importe quel
endpoint compatible OpenAI, ou Whisper **100 % local et hors-ligne**.

## Fonctionnalités

- 🎙️ **Dictée en streaming** — le texte apparaît au fur et à mesure que vous parlez.
- ⌨️ **Collage au curseur** — le résultat est tapé là où vous êtes (ou copié au presse-papier).
- ⌘ **Raccourci global** — démarrez/arrêtez sans quitter votre application en cours.
- 🔌 **Modèles enfichables** — Gemini Live, OpenAI Whisper, Groq, OpenAI-compatible, Whisper local.
- ✦ **Nettoyage des hésitations** — passe optionnelle qui retire les « euh », faux départs, répétitions.
- 🕘 **Historique** — retrouvez et recopiez vos dictées précédentes.
- 🌗 **Thème clair / sombre / système.**
- 〰️ **Waveform en direct** pilotée par le vrai niveau audio.
- 🖥️ **Multiplateforme** — un seul binaire léger (Tauri) sur macOS, Windows et Linux.

<p align="center">
  <img src="brand/png/logo-256.png" alt="" width="96">
</p>

## Captures

<!-- Ajoutez vos captures ici, p.ex. : -->
<!-- <img src="brand/png/screenshot-light.png" width="49%"> <img src="brand/png/screenshot-dark.png" width="49%"> -->

## Installation

```bash
# Prérequis : Rust (stable), Bun, et les dépendances système Tauri
# https://tauri.app/start/prerequisites/

bun install
bun run tauri dev      # développement
bun run tauri build    # build de production (binaire dans src-tauri/target/release)
```

## Configuration

Ouvrez le menu **☰ → Réglages** :

1. Choisissez un **fournisseur**.
2. Collez la **clé API** correspondante (stockée de façon sécurisée, jamais en clair côté front).
3. Optionnel : modèle, langue, URL de base (pour les endpoints compatibles), chemin du modèle
   `ggml` (Whisper local).
4. Optionnel : activez le **nettoyage automatique** et son moteur.

| Fournisseur          | Clé requise | Hors-ligne |
| -------------------- | :---------: | :--------: |
| Gemini Live          |     oui     |     —      |
| OpenAI Whisper       |     oui     |     —      |
| Groq Whisper         |     oui     |     —      |
| OpenAI-compatible    |  selon hôte |  possible  |
| Whisper local (ggml) |     non     |     ✓      |

## Raccourcis

| Action                 | Raccourci            |
| ---------------------- | -------------------- |
| Démarrer / arrêter     | `⌘⇧Espace` *(à adapter)* |
| Masquer la fenêtre     | depuis le menu / tray |

## Marque

Les ressources visuelles sont dans [`brand/`](brand/) :

- `logo.svg` — icône (vectoriel, dégradé)
- `logo-mark.svg` / `logo-mark-white.svg` — symbole seul (sur fond transparent)
- `png/` — exports PNG (32 → 512 px) + bannières
- Couleurs : violet `#7C5CFF` → cyan `#22D3EE` · fond sombre `#0A0B12`
- Typographie : Space Grotesk (titres) · Inter (UI) · JetBrains Mono (détails)

## Licence

MIT © 2026
