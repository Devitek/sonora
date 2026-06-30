<!--
Merci pour votre contribution ! Décrivez le QUOI et le POURQUOI.
Voir CONTRIBUTING.md pour les détails.
-->

## Description

<!-- Que fait cette PR, et pourquoi ? -->

## Issue liée

<!-- ex. Refs #123  (ou Closes #123 si la PR clôt définitivement l'issue) -->
Refs #

## Type de changement

- [ ] 🐞 Correction de bug
- [ ] ✨ Nouvelle fonctionnalité
- [ ] 🧹 Refactor / interne
- [ ] 📖 Documentation
- [ ] 🔧 CI / outillage

## Vérifications

- [ ] `bun run check` passe (0 erreur)
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passe
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passe
- [ ] `nix build .#default` passe (si packaging / frontend / deps modifiés)
- [ ] Dépendances frontend modifiées → `bash scripts/sync-bun-deps-hash.sh` lancé
- [ ] J'ai vérifié les **permissions / ACL** impactées (cf. `AGENTS.md`)

## Plateformes testées

- [ ] Linux
- [ ] macOS
- [ ] Windows

## Notes pour la revue

<!-- Captures d'écran, points d'attention, choix de conception… -->
