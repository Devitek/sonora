<!--
Thanks for your contribution! Describe the WHAT and the WHY.
See CONTRIBUTING.md for details.
-->

## Description

<!-- What does this PR do, and why? -->

## Related issue

<!-- e.g. Refs #123  (or Closes #123 if the PR fully resolves the issue) -->
Refs #

## Type of change

- [ ] 🐞 Bug fix
- [ ] ✨ New feature
- [ ] 🧹 Refactor / internal
- [ ] 📖 Documentation
- [ ] 🔧 CI / tooling

## Checks

- [ ] `bun run check` passes (0 errors)
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --check` passes
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` passes
- [ ] `nix build .#default` passes (if packaging / frontend / deps changed)
- [ ] Frontend deps changed → ran `bash scripts/sync-bun-deps-hash.sh`
- [ ] I checked the impacted **permissions / ACL** (see `AGENTS.md`)

## Platforms tested

- [ ] Linux
- [ ] macOS
- [ ] Windows

## Notes for reviewers

<!-- Screenshots, points of attention, design decisions… -->
