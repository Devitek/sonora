// Thin wrapper around the Tauri API so the rest of the UI never imports
// @tauri-apps/* directly. Makes it trivial to stub in a browser/dev context.

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

export const isTauri = "__TAURI_INTERNALS__" in window;

export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri) {
    console.warn(`[invoke:noop] ${cmd}`, args);
    return undefined as T;
  }
  return tauriInvoke<T>(cmd, args);
}

export async function listen<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!isTauri) {
    return () => {};
  }
  return tauriListen<T>(event, (e) => handler(e.payload));
}
