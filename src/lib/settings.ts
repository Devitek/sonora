// Small persisted settings (Tauri store, localStorage fallback for browser dev).

import { load, type Store } from "@tauri-apps/plugin-store";
import { isTauri } from "./tauri";

const FILE = "transcript-settings.json";

let storePromise: Promise<Store> | null = null;
function getStore(): Promise<Store> {
  if (!storePromise) storePromise = load(FILE);
  return storePromise;
}

export async function getAutoType(): Promise<boolean> {
  try {
    if (!isTauri) return localStorage.getItem("transcript-autotype") === "1";
    return (await (await getStore()).get<boolean>("autoType")) ?? false;
  } catch {
    return false;
  }
}

export async function setAutoType(value: boolean): Promise<void> {
  try {
    if (!isTauri) {
      localStorage.setItem("transcript-autotype", value ? "1" : "0");
      return;
    }
    const store = await getStore();
    await store.set("autoType", value);
    await store.save();
  } catch (e) {
    console.error("settings save failed:", e);
  }
}

// --- Floating bar position (so a moved bar stays put across launches) -------

export interface BarPosition {
  x: number;
  y: number;
}

export async function getBarPosition(): Promise<BarPosition | null> {
  try {
    if (!isTauri) {
      const raw = localStorage.getItem("transcript-bar-pos");
      return raw ? (JSON.parse(raw) as BarPosition) : null;
    }
    return (await (await getStore()).get<BarPosition>("barPosition")) ?? null;
  } catch {
    return null;
  }
}

export async function setBarPosition(pos: BarPosition): Promise<void> {
  try {
    if (!isTauri) {
      localStorage.setItem("transcript-bar-pos", JSON.stringify(pos));
      return;
    }
    const store = await getStore();
    await store.set("barPosition", pos);
    await store.save();
  } catch (e) {
    console.error("bar position save failed:", e);
  }
}

// --- Theme preference ------------------------------------------------------

export type ThemePref = "system" | "light" | "dark";

export async function getTheme(): Promise<ThemePref> {
  try {
    if (!isTauri) {
      return (localStorage.getItem("transcript-theme") as ThemePref) || "system";
    }
    return (await (await getStore()).get<ThemePref>("theme")) ?? "system";
  } catch {
    return "system";
  }
}

export async function setTheme(value: ThemePref): Promise<void> {
  try {
    if (!isTauri) {
      localStorage.setItem("transcript-theme", value);
      return;
    }
    const store = await getStore();
    await store.set("theme", value);
    await store.save();
  } catch (e) {
    console.error("theme save failed:", e);
  }
}
