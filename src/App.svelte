<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, listen, isTauri, dragWindow } from "./lib/tauri";
  import {
    getCurrentWindow,
    PhysicalPosition,
    availableMonitors,
  } from "@tauri-apps/api/window";
  import { copyText } from "./lib/clipboard";
  import { loadHistory, saveHistory, newEntry } from "./lib/history";
  import {
    getAutoType,
    getTheme,
    getBarPosition,
    setBarPosition,
    type BarPosition,
    type ThemePref,
  } from "./lib/settings";
  import {
    EVENT_CHANNEL,
    CONTROL_CHANNEL,
    type BackendEvent,
    type ControlEvent,
    type ControlAction,
    type RecordingState,
    type HistoryEntry,
    type Settings,
    type Prompt,
  } from "./lib/types";

  let recState = $state<RecordingState>("idle");
  let partial = $state("");
  let finals = $state<string[]>([]);
  let level = $state(0);
  let errorMsg = $state("");

  // Kept in memory only to append a finished dictation; the list now lives in
  // the dedicated panel window (Historique tab). Re-synced via the broadcast.
  let history = $state<HistoryEntry[]>([]);
  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  let autoType = $state(false);

  let settings = $state<Settings>({ provider: "gemini" });
  /** true once a usable transcription config resolves (drives onboarding). */
  let configured = $state(true);
  let cleaning = $state(false);
  let procLabel = $state("Reformulation…");
  let promptMenuOpen = $state(false);
  const prompts = $derived(settings.prompts ?? []);

  // Theme
  let theme = $state<ThemePref>("system");
  let systemDark = $state(true);
  const effectiveTheme = $derived(
    theme === "system" ? (systemDark ? "dark" : "light") : theme,
  );
  $effect(() => {
    document.documentElement.setAttribute("data-theme", effectiveTheme);
  });

  const cleanupEnabled = $derived(settings.cleanup_enabled === true);

  const listening = $derived(recState === "listening" || recState === "starting");
  const finalsText = $derived(finals.join(" ").trim());
  const fullText = $derived([...finals, partial].filter(Boolean).join(" ").trim());
  const hasText = $derived(fullText.length > 0);

  // Grow/shrink the transparent window to fit the bar + capsule + menu, so there
  // is no big transparent dead-zone capturing clicks (Spotlight-style).
  let lastBarHeight = 0;
  function measureAndResize() {
    let bottom = 0;
    for (const sel of [".bar", ".capsule", ".prompt-menu"]) {
      const el = document.querySelector(sel);
      if (!el) continue;
      const r = el.getBoundingClientRect();
      // Use scrollHeight so the window grows to the element's FULL content,
      // which removes the panel's internal scroll (WebKitGTK leaves black
      // repaint trails when scrolling a transparent window).
      bottom = Math.max(bottom, r.top + Math.max(el.scrollHeight, r.height));
    }
    const h = Math.ceil(bottom + 18);
    if (h > 0 && Math.abs(h - lastBarHeight) > 2) {
      lastBarHeight = h;
      void invoke("resize_bar", { height: h });
    }
  }

  $effect(() => {
    // Track everything that can change the visible height, then re-measure.
    void [
      promptMenuOpen,
      prompts.length,
      listening,
      errorMsg,
      configured,
      hasText,
      partial,
      finals.length,
    ];
    requestAnimationFrame(measureAndResize);
  });

  // ---- Move the floating bar -------------------------------------------------
  // Drag from any non-button area of the bar, or hold a modifier (⌘/Alt/Super)
  // and drag from anywhere (even over buttons) to reposition it. Uses the
  // OS-level window drag, so it works on macOS, Windows and Linux.
  function onBarPointerDown(e: PointerEvent) {
    if (e.button !== 0) return; // left button only
    const el = e.target as HTMLElement | null;
    const onControl = el?.closest("button, input, textarea, select, a");
    const modifier = e.altKey || e.metaKey;
    if (modifier || !onControl) {
      e.preventDefault();
      void dragWindow();
    }
  }

  // Persist the bar position so a moved bar stays put across launches.
  let unlistenMoved: (() => void) | undefined;
  let canPersistPos = false;
  let savePosTimer: ReturnType<typeof setTimeout> | undefined;

  async function isOnScreen(p: BarPosition): Promise<boolean> {
    try {
      const mons = await availableMonitors();
      if (!mons.length) return true;
      return mons.some((m) => {
        const { position: o, size: s } = m;
        return (
          p.x >= o.x - 24 &&
          p.y >= o.y - 24 &&
          p.x <= o.x + s.width - 48 &&
          p.y <= o.y + s.height - 48
        );
      });
    } catch {
      return true; // best effort — don't block restore on a query failure
    }
  }

  async function initBarPosition() {
    if (!isTauri) return;
    try {
      const win = getCurrentWindow();
      const saved = await getBarPosition();
      if (
        saved &&
        Number.isFinite(saved.x) &&
        Number.isFinite(saved.y) &&
        (await isOnScreen(saved))
      ) {
        await win.setPosition(new PhysicalPosition(saved.x, saved.y));
      }
      // Only start persisting after the restore + initial auto-resize settle, so
      // the centered default doesn't overwrite the saved spot.
      setTimeout(() => (canPersistPos = true), 700);
      unlistenMoved = await win.onMoved(({ payload }) => {
        if (!canPersistPos) return;
        clearTimeout(savePosTimer);
        const { x, y } = payload;
        savePosTimer = setTimeout(() => void setBarPosition({ x, y }), 400);
      });
    } catch (e) {
      console.error("bar position init failed:", e);
    }
  }

  /** Seed a realistic UI state for documentation screenshots (see onMount). */
  function seedShot(mode: string) {
    theme = "dark";
    configured = true;
    if (mode === "bar") {
      recState = "listening";
      level = 0.5;
      finals = ["Bonjour, ceci est une démonstration de Sonora,"];
      partial = "le texte s'écrit en temps réel.";
      return;
    }
    if (mode === "result") {
      recState = "idle";
      autoType = true;
      finals = ["Réécris l'intro, version plus concise."];
      partial = "";
      return;
    }
    // The history view now lives in the panel window (see Panel/History.svelte,
    // rendered via `?view=settings&tab=history` in the screenshot harness).
  }

  onMount(() => {
    // Docs screenshots: `?shot=bar|result|settings|history` renders the real UI
    // in a fixed, realistic state for captures. Completely inert without the
    // query param, so it never affects the shipped app.
    const shotParams = new URLSearchParams(location.search);
    const shotMode = shotParams.get("shot");
    if (shotMode) {
      seedShot(shotMode);
      const t = shotParams.get("theme");
      if (t === "light" || t === "dark") theme = t;
    }

    if (!shotMode) {
      void invoke<string>("app_ready").then((v) => console.log("backend ready:", v));
      void loadHistory().then((h) => (history = h));
      void getAutoType().then((v) => (autoType = v));
      void getTheme().then((v) => (theme = v));
      void loadSettings();
      void refreshConfigured();
      void initBarPosition();
    }

    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    systemDark = mq.matches;
    const onMq = (e: MediaQueryListEvent) => (systemDark = e.matches);
    mq.addEventListener("change", onMq);

    // Spotlight-style dismiss, made non-destructive: Escape closes the prompt
    // menu, then the settings panel. It only hides the whole bar when it is idle
    // and empty — never while there's a transcript to act on (apply a prompt,
    // copy…), a live session, or an error to read. This prevents the bar from
    // vanishing when Escape is pressed while the floating window happens to hold
    // focus (e.g. Hyprland focus-follows-mouse).
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (promptMenuOpen) promptMenuOpen = false;
      else if (!hasText && !listening && !cleaning && !errorMsg) void hideWindow();
    };
    window.addEventListener("keydown", onKey);

    const unlistenEvents = listen<BackendEvent>(EVENT_CHANNEL, (ev) => {
      switch (ev.kind) {
        case "state":
          recState = ev.state;
          if (ev.state === "idle") {
            void finalizeSession();
            void invoke("stop_recording"); // release mic if the session ended on its own
          }
          break;
        case "partial":
          partial = ev.text;
          break;
        case "final":
          if (ev.text.trim()) {
            const t = ev.text.trim();
            finals = [...finals, t];
            partial = "";
            void autoCopy();
            // When cleanup is on we type the cleaned text once at the end.
            if (autoType && !cleanupEnabled) {
              void invoke("type_text", { text: t + " " }).catch(
                (e) => (errorMsg = String(e)),
              );
            }
          }
          break;
        case "level":
          level = ev.rms;
          break;
        case "error":
          errorMsg = ev.message;
          recState = "error";
          void invoke("stop_recording"); // release mic + session on failure
          break;
      }
    });

    // Global hotkey / CLI / tray actions routed through the backend.
    const unlistenControl = listen<ControlEvent>(CONTROL_CHANNEL, (ev) =>
      handleControl(ev.action),
    );

    // The dedicated panel window broadcasts changes — reload what the bar
    // depends on (prompts, cleanup/auto-type toggles, theme, configured-state).
    const unlistenSettings = listen("sonora://settings-changed", () => {
      void loadSettings();
      void refreshConfigured();
      void getAutoType().then((v) => (autoType = v));
      void getTheme().then((v) => (theme = v));
    });

    // The panel's History tab (delete/clear) broadcasts too — keep the bar's
    // in-memory copy fresh so a finished dictation doesn't resurrect old entries.
    const unlistenHistory = listen("sonora://history-changed", () => {
      void loadHistory().then((h) => (history = h));
    });

    // Replay a first-launch CLI action (e.g. `transcript toggle`).
    void invoke<string | null>("take_pending_action").then((a) => {
      if (a) handleControl(a as ControlAction);
    });

    return () => {
      mq.removeEventListener("change", onMq);
      window.removeEventListener("keydown", onKey);
      void unlistenEvents.then((fn) => fn());
      void unlistenControl.then((fn) => fn());
      void unlistenSettings.then((fn) => fn());
      void unlistenHistory.then((fn) => fn());
      unlistenMoved?.();
      clearTimeout(savePosTimer);
    };
  });

  function handleControl(action: ControlAction) {
    switch (action) {
      case "toggle":
        void toggle();
        break;
      case "start":
        if (!listening) void toggle();
        break;
      case "stop":
        if (listening) void toggle();
        break;
      case "show":
        break; // backend already surfaced the window
    }
  }

  function flashCopied() {
    copied = true;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied = false), 1200);
  }

  async function autoCopy() {
    if (!finalsText) return;
    await copyText(finalsText);
    flashCopied();
  }

  async function copyCurrent() {
    if (!fullText) return;
    await copyText(fullText);
    flashCopied();
  }

  /** Clean `text` via the LLM, update the display + clipboard. Returns the text
   *  to persist (the cleaned version, or the original on failure). */
  async function runCleanup(text: string): Promise<string> {
    cleaning = true;
    procLabel = "Nettoyage…";
    try {
      const cleaned = (await invoke<string>("cleanup_text", { text }))?.trim();
      if (cleaned) {
        finals = [cleaned];
        partial = "";
        await copyText(cleaned);
        return cleaned;
      }
    } catch (e) {
      errorMsg = "Nettoyage : " + String(e);
    } finally {
      cleaning = false;
    }
    return text;
  }

  async function cleanupNow() {
    promptMenuOpen = false;
    if (!finalsText || cleaning) return;
    await runCleanup(finalsText);
  }

  /** Apply a user-defined reformulation prompt to the current transcript. */
  async function runTransform(promptText: string, text: string): Promise<string> {
    cleaning = true;
    procLabel = "Reformulation…";
    try {
      const out = (
        await invoke<string>("transform_text", { text, prompt: promptText })
      )?.trim();
      if (out) {
        finals = [out];
        partial = "";
        await copyText(out);
        return out;
      }
    } catch (e) {
      errorMsg = "Reformulation : " + String(e);
    } finally {
      cleaning = false;
    }
    return text;
  }

  async function applyPrompt(p: Prompt) {
    promptMenuOpen = false;
    if (!finalsText || cleaning) return;
    await runTransform(p.prompt, finalsText);
  }

  /** Persist the just-finished session (called when the backend goes idle). */
  async function finalizeSession() {
    let text = finalsText;
    if (!text) return;
    if (cleanupEnabled) {
      text = await runCleanup(text);
      if (autoType) {
        await invoke("type_text", { text: text + " " }).catch(
          (e) => (errorMsg = String(e)),
        );
      }
    }
    history = [newEntry(text), ...history];
    await saveHistory(history);
    void invoke("broadcast_history_changed"); // refresh the panel's History tab
  }

  async function toggle() {
    if (listening) {
      await invoke("stop_recording");
      recState = "idle"; // optimistic; backend confirms after finalize
    } else {
      // Not set up yet? Guide to settings instead of failing on start.
      await refreshConfigured();
      if (!configured) {
        openSettings();
        return;
      }
      // new session: clear the canvas (previous text is already in history)
      errorMsg = "";
      finals = [];
      partial = "";
      recState = "starting";
      try {
        await invoke("start_recording");
      } catch (e) {
        recState = "error";
        errorMsg = String(e);
      }
    }
  }

  function clearAll() {
    finals = [];
    partial = "";
  }

  async function loadSettings() {
    const s = await invoke<Settings>("get_settings");
    settings = s && Object.keys(s).length ? s : { provider: "gemini" };
    if (!settings.provider) settings.provider = "gemini";
    if (!settings.prompts) settings.prompts = [];
  }

  async function refreshConfigured() {
    configured = await invoke<boolean>("is_configured");
  }

  /** Open the dedicated panel window on the Historique tab. */
  function openHistory() {
    promptMenuOpen = false;
    void invoke("open_settings", { tab: "history" });
  }

  /** Open the dedicated panel window on the Réglages tab. */
  function openSettings() {
    promptMenuOpen = false;
    void invoke("open_settings", { tab: "settings" });
  }

  async function hideWindow() {
    await invoke("hide_window");
  }

  // --- Waveform capsule (driven by the real audio level) -------------------
  let canvasEl: HTMLCanvasElement | undefined = $state();
  let raf = 0;
  let animLevel = 0;

  $effect(() => {
    if (listening && canvasEl) {
      if (!raf) raf = requestAnimationFrame(drawLoop);
    } else {
      cancelAnimationFrame(raf);
      raf = 0;
      animLevel = 0;
    }
  });

  function drawLoop() {
    drawWave();
    raf = requestAnimationFrame(drawLoop);
  }

  function drawWave() {
    const cv = canvasEl;
    if (!cv) return;
    const w = cv.clientWidth,
      h = cv.clientHeight;
    if (!w || !h) return;
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    if (cv.width !== Math.round(w * dpr) || cv.height !== Math.round(h * dpr)) {
      cv.width = Math.round(w * dpr);
      cv.height = Math.round(h * dpr);
    }
    const ctx = cv.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // Smooth the RMS toward a lively target (RMS tends to be small).
    const target = Math.max(0.1, Math.min(1, level * 2.6));
    animLevel += (target - animLevel) * 0.3;

    const t = performance.now() / 1000;
    const c1 = "#7c5cff",
      c2 = "#22d3ee";
    const n = Math.max(11, Math.floor(w / 11));
    const mid = h / 2,
      bw = w / n;
    ctx.shadowColor = "rgba(124,92,255,0.5)";
    ctx.shadowBlur = 7;
    for (let i = 0; i < n; i++) {
      const osc =
        0.45 +
        0.55 *
          Math.abs(Math.sin(t * 7 + i * 0.55) + 0.4 * Math.sin(t * 4 - i * 0.9));
      const amp = Math.max(2, animLevel * osc * h * 0.4);
      const x = i * bw + bw * 0.28,
        ww = Math.max(2.5, bw * 0.44);
      const g = ctx.createLinearGradient(0, mid - amp, 0, mid + amp);
      g.addColorStop(0, c2);
      g.addColorStop(1, c1);
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.roundRect(x, mid - amp, ww, amp * 2, ww / 2);
      ctx.fill();
    }
    ctx.shadowBlur = 0;
  }
</script>

<main class="surface">
  <div class="anchor">
    <!-- THE BAR (drag a non-button area, or ⌘/Alt/Super + drag anywhere, to move it) -->
    <div
      class="bar"
      role="toolbar"
      tabindex="-1"
      aria-label="Barre Sonora — glissez pour déplacer"
      onpointerdown={onBarPointerDown}
      title="Glissez pour déplacer · ⌘/Alt + glisser depuis n'importe où"
    >
      <span class="grip" aria-hidden="true">
        <svg width="10" height="18" viewBox="0 0 10 18" fill="currentColor">
          <circle cx="2.5" cy="3" r="1.3" /><circle cx="7.5" cy="3" r="1.3" />
          <circle cx="2.5" cy="9" r="1.3" /><circle cx="7.5" cy="9" r="1.3" />
          <circle cx="2.5" cy="15" r="1.3" /><circle cx="7.5" cy="15" r="1.3" />
        </svg>
      </span>

      <button
        class="rec"
        class:on={listening}
        title={listening ? "Arrêter" : "Dicter"}
        onclick={toggle}
        aria-label={listening ? "Arrêter" : "Démarrer la dictée"}
      >
        {#if listening}
          <span class="stop"></span>
        {:else}
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <rect x="9" y="3" width="6" height="11" rx="3" fill="#fff" />
            <path d="M6 11a6 6 0 0 0 12 0" stroke="#fff" stroke-width="2" fill="none" stroke-linecap="round" />
            <line x1="12" y1="17" x2="12" y2="21" stroke="#fff" stroke-width="2" stroke-linecap="round" />
          </svg>
        {/if}
      </button>

      <span class="divider"></span>

      <div class="bar-text">
        {#if errorMsg}
          <span class="err">{errorMsg}</span>
        {:else if hasText}
          <p class="transcript">
            {#each finals as f}<span class="final">{f} </span>{/each}<span class="partial">{partial}</span>
          </p>
        {:else if !configured}
          <span class="ph">Ajoutez une clé API pour démarrer</span>
        {:else}
          <span class="ph">{listening ? "À l'écoute…" : "Dictez ou cliquez le micro"}</span>
        {/if}
      </div>

      {#if cleaning}<span class="proc">{procLabel}</span>{/if}

      {#if hasText && !listening}
        <button
          class="icon-btn"
          class:active={promptMenuOpen}
          onclick={() => (promptMenuOpen = !promptMenuOpen)}
          disabled={cleaning}
          title="Reformuler / nettoyer">✦</button
        >
        <button class="icon-btn" class:ok={copied} onclick={copyCurrent} title="Copier">
          {#if copied}✓{:else}⧉{/if}
        </button>
        <button class="icon-btn" onclick={clearAll} title="Effacer">⌫</button>
      {/if}

      <button class="icon-btn" onclick={openHistory} title="Historique" aria-label="Historique">
        <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 12a9 9 0 1 0 3-6.7L3 8" /><path d="M3 4v4h4" /><path d="M12 8v4l3 2" />
        </svg>
      </button>
      <button class="icon-btn menu" onclick={openSettings} title="Réglages" aria-label="Réglages">
        <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
    </div>

    <!-- FLOATING WAVEFORM CAPSULE -->
    {#if listening}
      <div class="capsule">
        <canvas bind:this={canvasEl}></canvas>
      </div>
    {/if}

    <!-- REFORMULATE / PROMPTS MENU -->
    {#if promptMenuOpen && hasText}
      <div class="prompt-menu">
        <button class="pm-item" onclick={cleanupNow} disabled={cleaning}>
          <span class="pm-ico">✦</span>
          <span>Nettoyer les hésitations</span>
        </button>
        {#each prompts as p (p.id)}
          {#if (p.name ?? "").trim() || (p.prompt ?? "").trim()}
            <button class="pm-item" onclick={() => applyPrompt(p)} disabled={cleaning}>
              <span class="pm-ico">↻</span>
              <span>{(p.name ?? "").trim() || "Sans nom"}</span>
            </button>
          {/if}
        {/each}
        <button
          class="pm-manage"
          onclick={() => {
            promptMenuOpen = false;
            openSettings();
          }}>+ Gérer les prompts</button
        >
      </div>
    {/if}
  </div>
</main>

<style>
  .surface {
    height: 100vh;
    width: 100vw;
    /* transparent window: the desktop shows through. Clicks pass through the
       empty area (pointer-events:none) — only the bar/capsule/menu catch them. */
    background: transparent;
    padding: 18px 20px;
    display: flex;
    justify-content: center;
    overflow: hidden;
    pointer-events: none;
  }
  .anchor {
    position: relative;
    width: 100%;
    max-width: 420px;
    align-self: flex-start;
    pointer-events: none;
  }
  .bar,
  .capsule,
  .prompt-menu {
    pointer-events: auto;
  }

  /* ---- reformulate / prompts menu ---- */
  .prompt-menu {
    position: absolute;
    top: calc(100% + 10px);
    right: 0;
    z-index: 8;
    width: 268px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-radius: 14px;
    background: var(--dd-bg);
    border: 1px solid var(--dd-border);
    box-shadow: var(--dd-shadow);
  }
  .pm-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    text-align: left;
    padding: 9px 10px;
    border-radius: 9px;
    color: var(--fg);
    font-size: 13px;
  }
  .pm-item:hover:not(:disabled) {
    background: var(--panel);
  }
  .pm-item:disabled {
    opacity: 0.5;
  }
  .pm-ico {
    color: var(--accent);
    font-size: 14px;
    width: 16px;
    text-align: center;
  }
  .pm-manage {
    margin-top: 4px;
    padding: 8px 10px;
    border-radius: 9px;
    color: var(--fg-dim);
    font-size: 12px;
    text-align: left;
    border-top: 1px solid var(--divider);
  }
  .pm-manage:hover {
    color: var(--fg);
  }
  .proc {
    flex: none;
    font-size: 12px;
    color: var(--accent);
    white-space: nowrap;
  }

  /* ---- bar ---- */
  .bar {
    position: relative;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 18px;
    background: var(--surface);
    border: 1px solid var(--surface-border);
    box-shadow: var(--surface-shadow);
    /* the bar background is a drag handle (buttons reset this to pointer) */
    cursor: grab;
  }
  .bar:active {
    cursor: grabbing;
  }
  /* drag handle: a subtle grip hinting the bar can be moved */
  .grip {
    flex: none;
    display: flex;
    align-items: center;
    margin: 0 -6px 0 -2px;
    color: var(--fg-dim);
    opacity: 0.5;
    cursor: grab;
    transition: opacity 0.12s ease;
  }
  .grip:hover {
    opacity: 0.9;
  }
  .rec {
    flex: none;
    width: 46px;
    height: 46px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(140deg, var(--accent), #6d5cff);
    box-shadow: 0 8px 22px rgba(124, 92, 255, 0.42);
    transition: transform 0.12s ease;
  }
  .rec:hover {
    transform: scale(1.05);
  }
  .rec.on {
    background: linear-gradient(140deg, #ef5da8, #7c5cff);
    box-shadow: 0 0 0 5px rgba(124, 92, 255, 0.16), 0 8px 24px rgba(239, 93, 168, 0.4);
    animation: pulse 1.5s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { box-shadow: 0 0 0 5px rgba(124, 92, 255, 0.16), 0 8px 24px rgba(239, 93, 168, 0.4); }
    50% { box-shadow: 0 0 0 9px rgba(124, 92, 255, 0.08), 0 8px 28px rgba(239, 93, 168, 0.5); }
  }
  .stop {
    width: 15px;
    height: 15px;
    border-radius: 4px;
    background: #fff;
  }
  .divider {
    flex: none;
    width: 1px;
    height: 24px;
    background: var(--divider);
  }
  .bar-text {
    flex: 1;
    min-width: 0;
    max-height: 84px;
    overflow-y: auto;
  }
  .ph {
    font-size: 15px;
    color: var(--fg-dim);
  }
  .err {
    font-size: 13px;
    color: var(--danger);
  }
  .transcript {
    margin: 0;
    font-size: 15.5px;
    line-height: 1.5;
  }
  .final {
    color: var(--fg);
  }
  .partial {
    color: var(--accent);
  }
  .icon-btn {
    flex: none;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 15px;
    color: var(--icon);
    background: var(--icon-bg);
  }
  .icon-btn:hover:not(:disabled) {
    color: var(--fg);
  }
  .icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .icon-btn.ok {
    color: var(--ok);
  }
  .icon-btn.menu {
    width: 36px;
    height: 36px;
    border-radius: 11px;
  }

  /* ---- capsule ---- */
  .capsule {
    position: absolute;
    left: 50%;
    top: calc(100% + 16px);
    transform: translateX(-50%);
    z-index: 4;
    width: 176px;
    height: 46px;
    border-radius: 15px;
    background: #15171f;
    box-shadow: 0 12px 30px rgba(40, 30, 90, 0.32), 0 0 0 1px rgba(124, 92, 255, 0.18),
      0 0 26px rgba(124, 92, 255, 0.22);
    overflow: hidden;
  }
  .capsule canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
