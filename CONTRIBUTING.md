# Contribuer à Sonora

Merci de l'intérêt que vous portez à **Sonora** — une barre de dictation flottante
multiplateforme (Tauri 2 + Svelte 5 + Rust). Toute contribution est la bienvenue :
rapport de bug, suggestion, doc, ou code.

> **Langue.** L'interface et les chaînes du produit sont en **français**. Vous
> pouvez ouvrir une issue ou une PR en **français ou en anglais** — on s'adaptera.

## Sommaire

- [Avant de commencer](#avant-de-commencer)
- [Mettre en place l'environnement](#mettre-en-place-lenvironnement)
- [Structure du projet](#structure-du-projet)
- [Lancer en développement](#lancer-en-développement)
- [Vérifications avant de proposer une PR](#vérifications-avant-de-proposer-une-pr)
- [La règle d'or : permissions & plateformes](#la-règle-dor--permissions--plateformes)
- [Conventions](#conventions)
- [Dépendances & sécurité](#dépendances--sécurité)
- [Processus de Pull Request](#processus-de-pull-request)
- [Releases (mainteneurs)](#releases-mainteneurs)
- [Signaler un bug / proposer une fonctionnalité](#signaler-un-bug--proposer-une-fonctionnalité)
- [Licence](#licence)

## Avant de commencer

- Pour un **changement non trivial**, ouvrez d'abord une **issue** pour en discuter
  (évite de coder dans une direction qui ne sera pas retenue).
- Cherchez dans les [issues existantes](https://github.com/Devitek/sonora/issues)
  avant d'en créer une nouvelle.
- Les petites corrections (typo, doc, lint) peuvent aller directement en PR.

## Mettre en place l'environnement

**Prérequis** : [Rust](https://rustup.rs/) stable, [Bun](https://bun.sh/), et les
[dépendances système Tauri](https://tauri.app/start/prerequisites/) (WebKitGTK,
ALSA, etc. sous Linux).

```bash
bun install
```

**Sous Nix / NixOS** (recommandé — fournit WebKitGTK, ALSA, cmake, wtype… dans un
devshell reproductible) :

```bash
nix develop            # entre dans le devshell
bun install
```

## Structure du projet

| Chemin | Rôle |
| --- | --- |
| `src/` | Frontend Svelte 5 (barre flottante + fenêtre Historique/Réglages) |
| `src/lib/` | Utilitaires partagés (providers, settings, clipboard, types…) |
| `src-tauri/src/` | Backend Rust (audio, providers de transcription, secrets, sortie clavier…) |
| `src-tauri/capabilities/` | Permissions ACL Tauri |
| `flake.nix` | Paquet Nix natif + devshell |
| `scripts/` | Outils (dev, screenshots, bump de version, hash des deps…) |
| `docs/` | Mini-site GitHub Pages |
| `.github/workflows/` | CI, release, sécurité, bots de mise à jour |

## Lancer en développement

```bash
bun run tauri dev          # ou: bash scripts/dev.sh  (libère le port Vite résiduel)
```

Build de production local :

```bash
bun run tauri build        # binaire dans src-tauri/target/release/sonora
```

## Vérifications avant de proposer une PR

Ces commandes correspondent **exactement** à ce que la CI exécute — faites-les
passer en local pour un aller-retour rapide :

```bash
# Frontend : 0 erreur attendue
bun run check

# Rust : format + lints (warnings = erreurs)
cargo fmt   --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# Paquet Nix (si vous touchez au packaging, au frontend ou aux deps)
nix build .#default
```

La CI ajoute en plus un **`cargo check` sur Windows et macOS** : une dépendance
qui ne compile que sur Linux ne suffit pas. Si votre changement touche au backend
audio/natif, gardez en tête les trois OS.

## La règle d'or : permissions & plateformes

Avant de considérer une fonctionnalité « terminée », vérifiez ce qui pourrait
**l'empêcher de tourner** sur chaque plateforme — **permissions OS** et
**capacités ACL Tauri**. Une feature qui marche en `tauri dev` échoue souvent
ailleurs (prompt micro, tray, EGL, position de fenêtre…).

En résumé, demandez-vous :

1. **ACL Tauri** (`src-tauri/capabilities/default.json`) — tout nouvel appel
   `core:*` ou plugin (`window.setPosition`, `clipboard`, `global-shortcut`,
   `store`…) doit avoir une règle `allow-*`, sinon il est **silencieusement
   refusé**. (Les commandes `#[tauri::command]` maison n'en ont pas besoin.)
2. **macOS** — ressource TCC (micro → `NS*UsageDescription` dans `Info.plist`),
   Accessibilité pour la frappe au curseur, Gatekeeper pour les builds non signés.
3. **Windows** — réglage de confidentialité micro, SmartScreen pour les installeurs.
4. **Linux** — outils/libs `dlopen`és : `wtype`/`xdotool` (frappe au curseur),
   `libayatana-appindicator` (tray), quirks EGL/DMABUF de WebKitGTK.

Le détail complet (et la matrice des permissions) est dans
[`AGENTS.md`](AGENTS.md) — à lire pour tout changement touchant micro, tray,
fenêtres, frappe au curseur ou packaging.

## Conventions

- **Langue** : chaînes d'UI et libellés en **français**.
- **Commits** : style [Conventional Commits](https://www.conventionalcommits.org/)
  concis (ex. `feat(audio): …`, `fix(nix): …`, `ci: …`, `docs: …`).
- **Issues dans les commits** : utilisez `Refs #N` tant qu'un correctif n'est pas
  confirmé par le rapporteur ; n'employez `Fixes/Closes #N` que lorsqu'on veut
  réellement clore l'issue au merge.
- **Style** : laissez `cargo fmt` et le formateur du frontend décider ; pas de
  reformatage massif non lié à votre changement.

## Dépendances & sécurité

- Les mises à jour sont automatisées : **Dependabot** (crates Rust + GitHub
  Actions) et des bots maison pour **bun** (`update-bun.yml`) et **`flake.lock`**
  (`update-flake-lock.yml`).
- **Si vous modifiez les dépendances frontend** (`package.json` / `bun.lock`),
  l'empreinte `bunDepsHash` du `flake.nix` doit suivre. Lancez :
  ```bash
  bash scripts/sync-bun-deps-hash.sh   # re-pin automatique (no-op si déjà bon)
  ```
- **Sécurité** : `cargo audit` (RustSec) et `bun audit` tournent en CI ; **CodeQL**
  analyse le code (JS/TS + Rust). Une PR qui introduit une vulnérabilité connue
  est bloquée. Merci de **ne jamais** committer de clé API / secret (les clés
  vivent dans le trousseau de l'OS, jamais en clair).

## Processus de Pull Request

1. Forkez et créez une branche depuis `main`
   (`feat/ma-feature`, `fix/le-bug`…).
2. Faites passer les [vérifications](#vérifications-avant-de-proposer-une-pr) en local.
3. Ouvrez la PR vers `main` en remplissant le gabarit. Décrivez le **quoi** et le
   **pourquoi**, et la/les **plateformes testées**.
4. La CI doit être **verte** (frontend, rust, `cargo check` Windows/macOS, Nix,
   CodeQL, audits). Répondez aux retours de revue.
5. Le merge se fait en **squash** une fois la PR validée.

## Releases (mainteneurs)

```bash
node scripts/bump-version.mjs <patch|minor|major>   # synchronise package.json / tauri.conf.json / Cargo.toml
git commit -am "chore(release): vX.Y.Z"
git tag -a vX.Y.Z -m "vX.Y.Z" && git push origin main vX.Y.Z
```

Le push du tag déclenche le build et la publication multiplateforme
(Linux / macOS / Windows).

## Signaler un bug / proposer une fonctionnalité

Ouvrez une [issue](https://github.com/Devitek/sonora/issues/new/choose) et choisissez
le gabarit adapté (**Rapport de bug** ou **Demande de fonctionnalité**). Plus le
contexte est précis (OS, mode d'installation, version, fournisseur de
transcription, logs), plus c'est rapide à traiter.

## Licence

En contribuant, vous acceptez que votre contribution soit publiée sous la licence
**MIT** du projet (voir [`LICENSE`](LICENSE)).
