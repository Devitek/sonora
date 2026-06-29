<script lang="ts">
  import { onMount } from "svelte";
  import { listen, isTauri } from "./lib/tauri";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import History from "./History.svelte";
  import SettingsForm from "./SettingsForm.svelte";
  import { getTheme, type ThemePref } from "./lib/settings";

  type Tab = "history" | "settings";

  function initialTab(): Tab {
    return new URLSearchParams(location.search).get("tab") === "settings"
      ? "settings"
      : "history";
  }
  let tab = $state<Tab>(initialTab());

  // Theme: this window owns the data-theme attribute (reloaded on broadcast).
  let theme = $state<ThemePref>("system");
  let systemDark = $state(true);
  const effectiveTheme = $derived(
    theme === "system" ? (systemDark ? "dark" : "light") : theme,
  );
  $effect(() => {
    document.documentElement.setAttribute("data-theme", effectiveTheme);
  });

  // Keep the OS window title in sync with the active tab so it never collides
  // with the bar's "Sonora" title (Hyprland/wm rules target the exact bar title).
  const TITLES: Record<Tab, string> = {
    history: "Sonora — Historique",
    settings: "Sonora — Réglages",
  };
  $effect(() => {
    if (!isTauri) return;
    try {
      void getCurrentWindow().setTitle(TITLES[tab]);
    } catch {
      /* set-title best effort */
    }
  });

  function closeWindow() {
    if (!isTauri) return;
    void getCurrentWindow().close();
  }

  onMount(() => {
    if (isTauri) void getTheme().then((v) => (theme = v));

    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    systemDark = mq.matches;
    const onMq = (e: MediaQueryListEvent) => (systemDark = e.matches);
    mq.addEventListener("change", onMq);

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeWindow();
    };
    window.addEventListener("keydown", onKey);

    // The bar asks an already-open panel to switch tab; reload the theme when
    // settings change in another window.
    const unlistenTab = listen<string>("sonora://panel-tab", (t) => {
      if (t === "settings" || t === "history") tab = t;
    });
    const unlistenSettings = listen("sonora://settings-changed", () => {
      void getTheme().then((v) => (theme = v));
    });

    return () => {
      mq.removeEventListener("change", onMq);
      window.removeEventListener("keydown", onKey);
      void unlistenTab.then((fn) => fn());
      void unlistenSettings.then((fn) => fn());
    };
  });
</script>

<div class="panel-root">
  <header class="p-header" data-tauri-drag-region>
    <div class="tabs" role="tablist">
      <button
        class="tab"
        class:active={tab === "history"}
        role="tab"
        aria-selected={tab === "history"}
        onclick={() => (tab = "history")}
      >
        Historique
      </button>
      <button
        class="tab"
        class:active={tab === "settings"}
        role="tab"
        aria-selected={tab === "settings"}
        onclick={() => (tab = "settings")}
      >
        Réglages
      </button>
    </div>
    <button class="p-close" onclick={closeWindow} title="Fermer" aria-label="Fermer">✕</button>
  </header>

  <div class="p-body">
    <!-- Both panes stay mounted (toggled via display) so switching tabs never
         drops in-progress settings edits or reloads the history list. -->
    <div class="pane" class:hidden={tab !== "history"}>
      <History />
    </div>
    <div class="pane" class:hidden={tab !== "settings"}>
      <SettingsForm onClose={closeWindow} />
    </div>
  </div>
</div>

<style>
  .panel-root {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-solid);
    color: var(--fg);
  }
  .p-header {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px 8px 12px;
    border-bottom: 1px solid var(--divider);
    background: var(--surface);
  }
  .tabs {
    display: flex;
    gap: 3px;
    padding: 3px;
    border-radius: 11px;
    background: var(--panel);
    border: 1px solid var(--border);
  }
  .tab {
    padding: 7px 16px;
    border-radius: 8px;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--fg-dim);
    transition: all 0.15s ease;
  }
  .tab:hover {
    color: var(--fg-mid);
  }
  .tab.active {
    color: var(--fg);
    background: var(--seg-active);
    box-shadow: var(--seg-shadow);
  }
  .p-close {
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    color: var(--fg-dim);
    background: var(--icon-bg);
    font-size: 13px;
  }
  .p-close:hover {
    color: var(--fg);
  }
  .p-body {
    flex: 1;
    min-height: 0;
    position: relative;
  }
  .pane {
    position: absolute;
    inset: 0;
  }
  .pane.hidden {
    display: none;
  }
</style>
