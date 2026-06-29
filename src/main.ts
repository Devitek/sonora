import { mount } from "svelte";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./app.css";
import App from "./App.svelte";
import Panel from "./Panel.svelte";
import { isTauri } from "./lib/tauri";

// The same bundle drives the floating bar and the dedicated panel window
// (Historique / Réglages tabs). Pick the component from the Tauri window label
// (or `?view=settings`, used by the screenshot harness which runs outside Tauri).
function isPanelView(): boolean {
  if (new URLSearchParams(location.search).get("view") === "settings") return true;
  if (!isTauri) return false;
  try {
    return getCurrentWindow().label === "settings";
  } catch {
    return false;
  }
}

const app = mount(isPanelView() ? Panel : App, {
  target: document.getElementById("app")!,
});

export default app;
