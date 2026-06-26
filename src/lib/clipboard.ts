// Clipboard helper. Uses the Tauri clipboard-manager plugin in the app,
// falls back to the Web Clipboard API when running in a plain browser.

import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { isTauri } from "./tauri";

export async function copyText(text: string): Promise<void> {
  const value = text.trim();
  if (!value) return;
  try {
    if (isTauri) {
      await writeText(value);
    } else {
      await navigator.clipboard?.writeText(value);
    }
  } catch (e) {
    console.error("clipboard write failed:", e);
  }
}
