// Persistent transcript history. Backed by the Tauri store plugin in the app,
// with a localStorage fallback for browser dev.

import { load, type Store } from "@tauri-apps/plugin-store";
import { isTauri } from "./tauri";
import type { HistoryEntry } from "./types";

const STORE_FILE = "transcript-history.json";
const KEY = "entries";
const MAX_ENTRIES = 200;
const LS_KEY = "transcript-history";

let storePromise: Promise<Store> | null = null;
function getStore(): Promise<Store> {
  // We persist explicitly via save(); no autosave needed.
  if (!storePromise) storePromise = load(STORE_FILE);
  return storePromise;
}

export async function loadHistory(): Promise<HistoryEntry[]> {
  try {
    if (!isTauri) {
      const raw = localStorage.getItem(LS_KEY);
      return raw ? (JSON.parse(raw) as HistoryEntry[]) : [];
    }
    const store = await getStore();
    return (await store.get<HistoryEntry[]>(KEY)) ?? [];
  } catch (e) {
    console.error("history load failed:", e);
    return [];
  }
}

export async function saveHistory(entries: HistoryEntry[]): Promise<void> {
  const trimmed = entries.slice(0, MAX_ENTRIES);
  try {
    if (!isTauri) {
      localStorage.setItem(LS_KEY, JSON.stringify(trimmed));
      return;
    }
    const store = await getStore();
    await store.set(KEY, trimmed);
    await store.save();
  } catch (e) {
    console.error("history save failed:", e);
  }
}

export function newEntry(text: string): HistoryEntry {
  return {
    id: crypto.randomUUID(),
    text: text.trim(),
    createdAt: Date.now(),
  };
}
