#!/usr/bin/env bash
# Re-pin flake.nix `bunDepsHash.<currentSystem>` to match the current bun.lock.
#
# The frontend-deps fixed-output derivation produces node_modules, whose hash
# changes whenever the bun dependencies change. Any workflow that mutates
# bun.lock (update-bun.yml) — or that can change the bun version via nixpkgs
# (update-flake-lock.yml) — must call this so the committed flake stays buildable.
#
# No-op (exit 0) when the hash is already correct. Requires nix (flakes).
set -euo pipefail

system="$(nix eval --impure --raw --expr 'builtins.currentSystem')"

# A hash mismatch makes the FOD build fail and print the correct `got:` value;
# a correct hash just builds. `|| true` so we can inspect the output either way.
out="$(nix build .#frontend-deps -L 2>&1 || true)"
got="$(printf '%s\n' "$out" | awk '/got:/{print $2; exit}')"

if [ -z "${got}" ]; then
  echo "bunDepsHash.${system} already up to date."
  exit 0
fi

# Replace the per-system line:  <system> = "sha256-....";
sed -i -E "s|(${system} = \")sha256-[A-Za-z0-9+/=]+(\";)|\1${got}\2|" flake.nix
echo "bunDepsHash.${system} updated -> ${got}"
