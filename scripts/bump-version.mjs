#!/usr/bin/env node
// Bump the version across the three files that must stay in sync:
//   package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml (+ Cargo.lock).
//
//   node scripts/bump-version.mjs <patch|minor|major|X.Y.Z>
//
// Prints the new version to stdout (the release workflow captures it).

import { readFileSync, writeFileSync, existsSync } from "node:fs";

const arg = process.argv[2] || "patch";
const root = new URL("..", import.meta.url).pathname;
const p = (rel) => root + rel;

const PKG = p("package.json");
const TAURI = p("src-tauri/tauri.conf.json");
const CARGO = p("src-tauri/Cargo.toml");
const LOCK = p("src-tauri/Cargo.lock");

const pkg = JSON.parse(readFileSync(PKG, "utf8"));
const cur = String(pkg.version || "0.0.0");

let [maj, min, pat] = cur.split(".").map((n) => parseInt(n, 10));
if (arg === "major") {
  maj += 1;
  min = 0;
  pat = 0;
} else if (arg === "minor") {
  min += 1;
  pat = 0;
} else if (arg === "patch") {
  pat += 1;
} else if (/^\d+\.\d+\.\d+$/.test(arg)) {
  [maj, min, pat] = arg.split(".").map((n) => parseInt(n, 10));
} else {
  console.error(`invalid bump: '${arg}' (expected patch|minor|major|X.Y.Z)`);
  process.exit(1);
}
const version = `${maj}.${min}.${pat}`;

// package.json (preserve 2-space indent)
pkg.version = version;
writeFileSync(PKG, JSON.stringify(pkg, null, 2) + "\n");

// tauri.conf.json
const tauri = JSON.parse(readFileSync(TAURI, "utf8"));
tauri.version = version;
writeFileSync(TAURI, JSON.stringify(tauri, null, 2) + "\n");

// Cargo.toml — only the [package] version, not dependency versions.
let cargo = readFileSync(CARGO, "utf8");
cargo = cargo.replace(/(\[package\][\s\S]*?\nversion\s*=\s*")[^"]+(")/, `$1${version}$2`);
writeFileSync(CARGO, cargo);

// Cargo.lock — keep the local package entry in sync (avoids a dirty lockfile).
if (existsSync(LOCK)) {
  let lock = readFileSync(LOCK, "utf8");
  lock = lock.replace(
    /(name = "sonora"\nversion = ")[^"]+(")/,
    `$1${version}$2`,
  );
  writeFileSync(LOCK, lock);
}

process.stdout.write(version + "\n");
