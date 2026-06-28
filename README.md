# transcript — refonte « barre flottante »

<!-- Remplacez <owner> par votre compte / organisation GitHub. -->
[![CI](https://github.com/<owner>/transcript/actions/workflows/ci.yml/badge.svg)](https://github.com/<owner>/transcript/actions/workflows/ci.yml)
[![Release](https://github.com/<owner>/transcript/actions/workflows/release.yml/badge.svg)](https://github.com/<owner>/transcript/actions/workflows/release.yml)

Portage du design validé dans l'app Svelte 5 + Tauri. **Aucune logique métier modifiée** :
providers, clés API, nettoyage, historique, saisie auto au curseur, hotkey global / tray,
événements backend — tout est préservé. Le design et le thème changent, plus un nouveau
réglage **Thème (Système / Clair / Sombre)** et une **waveform pilotée par le vrai niveau audio**.

## Fichiers à remplacer

Copiez ces fichiers par-dessus les vôtres (mêmes chemins) :

```
src/App.svelte            ← refonte complète (template + styles)
src/app.css               ← système de tokens clair/sombre
src/lib/settings.ts       ← + getTheme()/setTheme() (le reste inchangé)
src/lib/Select.svelte     ← fond de liste via variable (compatible thème clair)
src-tauri/tauri.conf.json ← fenêtre 460×440 (laisse la place à la barre + menu déroulant)
```

Aucune nouvelle dépendance. `git diff` recommandé avant commit.

## Ce qui change à l'écran

- **Barre flottante** : bouton record à gauche, texte en ligne, bouton **Options** (☰) à droite.
  Quand du texte existe : boutons ✦ nettoyer · ⧉ copier · ⌫ effacer apparaissent dans la barre.
- **Capsule waveform** : pastille sombre sous la barre pendant l'écoute, animée par le **vrai
  niveau RMS** (`level`) reçu du backend (canvas, lissé).
- **Menu déroulant** : deux bascules rapides (Coller au curseur, Nettoyage auto) + onglets
  **Historique** et **Réglages**. Les Réglages contiennent tous les champs existants
  (fournisseur, clé, modèle, base_url, whisper local, langue, section nettoyage) **plus** le
  sélecteur de thème en haut. Bouton « Masquer la fenêtre » en bas.

## Détails d'implémentation

- **Thème** : `theme` (`system|light|dark`) persisté via le store (`getTheme/setTheme`).
  `effectiveTheme` résout `system` via `matchMedia('(prefers-color-scheme: dark)')` (avec
  écoute des changements OS) et applique `data-theme` sur `<html>`. Tout le style passe par des
  variables CSS définies dans `app.css` pour les deux thèmes.
- **Nettoyage auto** : la bascule rapide écrit `settings.cleanup_enabled` et appelle
  `save_settings` immédiatement (la case à cocher dans Réglages reste pour configurer moteur/modèle).
- **Waveform** : `requestAnimationFrame` démarré/arrêté par un `$effect` sur `listening`.
  Couleurs néon (#7C5CFF→#22D3EE) indépendantes du thème, la capsule étant toujours sombre.

## À ajuster si besoin

- **Indice de niveau** : `target = level * 2.6` dans `drawWave()`. Si vos RMS sont plus
  faibles/forts, ajustez ce facteur.
- **Taille de fenêtre** : `tauri.conf.json` (460×440). La fenêtre est opaque
  (`transparent: false`) ; le fond dégradé simule le bureau. Pour une vraie barre flottante à
  coins arrondis transparents, passez `transparent: true` + coins arrondis (spécifique OS — non
  fait ici pour rester sûr).
- **Raccourci** : aucun libellé de hotkey n'est affiché dans la barre (le binding réel n'est pas
  connu côté front). Ajoutez un `<kbd>` dans `.bar-text` si vous voulez l'indiquer.

## TODO / Roadmap

- [ ] **Signature des binaires** (retirer les avertissements Gatekeeper / SmartScreen sur les
  builds publiés par la CI) :
  - **macOS** : certificat « Developer ID Application » + **notarisation** (`notarytool`). À
    câbler dans `tauri-action` via les secrets `APPLE_CERTIFICATE`,
    `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`,
    `APPLE_TEAM_ID`.
  - **Windows** : certificat **Authenticode** (code signing) via `WINDOWS_CERTIFICATE` +
    `WINDOWS_CERTIFICATE_PASSWORD`, ou **Azure Trusted Signing**.
  - Nécessite des certificats payants (Apple Developer Program / AC Windows) ; à ajouter quand
    disponibles, sans changer le reste du pipeline.
