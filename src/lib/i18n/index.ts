// Minimal, zero-dependency runtime i18n.
//
// Why not a library: Sonora's frontend is built offline in a Nix sandbox, which
// rules out compile-first tools (e.g. Paraglide) that fetch plugins from a CDN
// at build time. A tiny runtime over JSON catalogs needs no network, no compile
// step and no new dependency.
//
// French is the source/default language. Phase 1 ships only `fr`; a language
// selector, OS-locale detection, persistence and reactivity land in later
// phases (see the i18n epic). Add a language by dropping `locales/<code>.json`
// (same keys as `fr.json` — enforced by scripts/check-i18n.mjs) and registering
// it in `catalogs` below.

import fr from "./locales/fr.json";

export type Messages = typeof fr;
export type MessageKey = keyof Messages;

/** Registered catalogues, keyed by locale code. `fr` is the source of truth. */
const catalogs: Record<string, Partial<Messages>> = { fr };

/** Human-readable names for the (future) language picker. */
export const LOCALE_NAMES: Record<string, string> = { fr: "Français" };

export const DEFAULT_LOCALE = "fr";

let locale = DEFAULT_LOCALE;

export function availableLocales(): string[] {
  return Object.keys(catalogs);
}

export function getLocale(): string {
  return locale;
}

/** Switch the active locale (no-op for an unknown code). */
export function setLocale(code: string): void {
  if (catalogs[code]) locale = code;
}

/**
 * Translate `key`, replacing `{name}` placeholders with `params.name`.
 * Falls back to the French source string, then to the key itself, so a missing
 * translation degrades to French rather than to a blank.
 */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const msg = (catalogs[locale]?.[key] ?? fr[key] ?? key) as string;
  if (!params) return msg;
  return msg.replace(/\{(\w+)\}/g, (_, name) =>
    name in params ? String(params[name]) : `{${name}}`,
  );
}
