#!/usr/bin/env node
// Verify every locale catalogue has exactly the same keys as the French source
// (src/lib/i18n/locales/fr.json). Fails CI on drift so a translation can never
// ship with missing or stale keys. Run via `bun run check`.

import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const dir = join(here, "..", "src", "lib", "i18n", "locales");
const SOURCE = "fr";

const load = (code) =>
  JSON.parse(readFileSync(join(dir, `${code}.json`), "utf8"));

const source = load(SOURCE);
const sourceKeys = new Set(Object.keys(source));

const locales = readdirSync(dir)
  .filter((f) => f.endsWith(".json"))
  .map((f) => f.slice(0, -5))
  .filter((code) => code !== SOURCE);

let failed = false;
for (const code of locales) {
  const keys = new Set(Object.keys(load(code)));
  const missing = [...sourceKeys].filter((k) => !keys.has(k));
  const extra = [...keys].filter((k) => !sourceKeys.has(k));
  const empty = [...keys].filter((k) => !String(load(code)[k] ?? "").trim());
  if (missing.length || extra.length || empty.length) {
    failed = true;
    console.error(`✗ ${code}.json out of sync with ${SOURCE}.json:`);
    if (missing.length) console.error(`  missing: ${missing.join(", ")}`);
    if (extra.length) console.error(`  unknown: ${extra.join(", ")}`);
    if (empty.length) console.error(`  empty:   ${empty.join(", ")}`);
  }
}

if (failed) process.exit(1);
console.log(
  `i18n: ${sourceKeys.size} keys, ${locales.length} translation(s) in sync.`,
);
