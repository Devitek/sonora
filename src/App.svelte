<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, listen } from "./lib/tauri";
  import { copyText } from "./lib/clipboard";
  import { loadHistory, saveHistory, newEntry } from "./lib/history";
  import {
    EVENT_CHANNEL,
    CONTROL_CHANNEL,
    type BackendEvent,
    type ControlEvent,
    type ControlAction,
    type RecordingState,
    type HistoryEntry,
  } from "./lib/types";

  let recState = $state<RecordingState>("idle");
  let partial = $state("");
  let finals = $state<string[]>([]);
  let level = $state(0);
  let errorMsg = $state("");

  let history = $state<HistoryEntry[]>([]);
  let showHistory = $state(false);
  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;

  const listening = $derived(recState === "listening" || recState === "starting");
  const finalsText = $derived(finals.join(" ").trim());
  const fullText = $derived([...finals, partial].filter(Boolean).join(" ").trim());

  onMount(() => {
    void invoke<string>("app_ready").then((v) => console.log("backend ready:", v));
    void loadHistory().then((h) => (history = h));

    const unlistenEvents = listen<BackendEvent>(EVENT_CHANNEL, (ev) => {
      switch (ev.kind) {
        case "state":
          recState = ev.state;
          if (ev.state === "idle") void finalizeSession();
          break;
        case "partial":
          partial = ev.text;
          break;
        case "final":
          if (ev.text.trim()) {
            finals = [...finals, ev.text.trim()];
            partial = "";
            void autoCopy();
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

    // Replay a first-launch CLI action (e.g. `transcript toggle`).
    void invoke<string | null>("take_pending_action").then((a) => {
      if (a) handleControl(a as ControlAction);
    });

    return () => {
      void unlistenEvents.then((fn) => fn());
      void unlistenControl.then((fn) => fn());
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

  /** Persist the just-finished session (called when the backend goes idle). */
  async function finalizeSession() {
    const text = finalsText;
    if (!text) return;
    history = [newEntry(text), ...history];
    await saveHistory(history);
  }

  async function toggle() {
    if (listening) {
      await invoke("stop_recording");
      recState = "idle"; // optimistic; backend confirms after finalize
    } else {
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

  function onMicKey(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      void toggle();
    }
  }

  function clearAll() {
    finals = [];
    partial = "";
  }

  async function copyEntry(entry: HistoryEntry) {
    await copyText(entry.text);
    flashCopied();
  }

  async function deleteEntry(id: string) {
    history = history.filter((e) => e.id !== id);
    await saveHistory(history);
  }

  async function clearHistory() {
    history = [];
    await saveHistory(history);
  }

  function fmtTime(ts: number): string {
    return new Date(ts).toLocaleString(undefined, {
      day: "2-digit",
      month: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  async function hideWindow() {
    await invoke("hide_window");
  }
</script>

<main class="hud" class:listening>
  <header class="bar" data-tauri-drag-region>
    <div class="title" data-tauri-drag-region>
      <span class="dot" class:on={listening} class:err={recState === "error"}></span>
      transcript
    </div>
    <div class="win-controls">
      <button
        class="icon"
        class:active={showHistory}
        title="Historique"
        onclick={() => (showHistory = !showHistory)}>≣</button
      >
      <button class="icon" title="Masquer (reste dans le tray)" onclick={hideWindow}>—</button>
    </div>
  </header>

  {#if showHistory}
    <section class="body history">
      {#if history.length === 0}
        <p class="placeholder">Aucun historique pour l'instant.</p>
      {:else}
        <div class="history-head">
          <span>{history.length} entrée{history.length > 1 ? "s" : ""}</span>
          <button class="link" onclick={clearHistory}>Tout effacer</button>
        </div>
        <ul class="history-list">
          {#each history as entry (entry.id)}
            <li class="history-item">
              <button class="entry-text" title="Copier" onclick={() => copyEntry(entry)}>
                <span class="entry-time">{fmtTime(entry.createdAt)}</span>
                <span class="entry-body">{entry.text}</span>
              </button>
              <button class="entry-del" title="Supprimer" onclick={() => deleteEntry(entry.id)}
                >✕</button
              >
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {:else}
    <section class="body">
      {#if errorMsg}
        <p class="error">{errorMsg}</p>
      {/if}
      {#if fullText}
        <p class="transcript">
          {#each finals as f}<span class="final">{f} </span>{/each}<span class="partial"
            >{partial}</span
          >
        </p>
      {:else}
        <div class="empty">
          <div
            class="big-mic"
            class:on={listening}
            role="button"
            tabindex="0"
            onclick={toggle}
            onkeydown={onMicKey}
            title={listening ? "Arrêter" : "Démarrer la dictée"}
            aria-label={listening ? "Arrêter" : "Démarrer la dictée"}
            style="position:relative;flex:none;width:88px;height:88px;border-radius:50%;cursor:pointer;background:{listening
              ? '#ef4444'
              : '#4f46e5'};color:#fff"
          >
            {#if listening}
              <svg viewBox="0 0 24 24" width="34" height="34" aria-hidden="true">
                <rect x="6" y="6" width="12" height="12" rx="2.5" fill="currentColor" />
              </svg>
            {:else}
              <svg
                viewBox="0 0 24 24"
                width="36"
                height="36"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <rect x="9" y="2" width="6" height="11" rx="3" />
                <path d="M5 10a7 7 0 0 0 14 0" />
                <line x1="12" y1="19" x2="12" y2="22" />
              </svg>
            {/if}
          </div>
          <p class="placeholder">
            {recState === "idle" ? "Cliquez le micro pour dicter" : "À l'écoute…"}
          </p>
        </div>
      {/if}
    </section>
  {/if}

  <footer class="actions">
    <div class="meter" aria-hidden="true">
      <span class="meter-fill" style={`width:${Math.min(100, Math.round(level * 100))}%`}></span>
    </div>
    {#if copied}<span class="copied">copié ✓</span>{/if}
    <button class="ghost" onclick={copyCurrent} disabled={!fullText} title="Copier">⧉</button>
    <button class="ghost" onclick={clearAll} disabled={!fullText} title="Effacer">⌫</button>
    <div
      class="mic"
      class:on={listening}
      role="button"
      tabindex="0"
      onclick={toggle}
      onkeydown={onMicKey}
      title="Démarrer / arrêter"
      aria-label="Démarrer / arrêter la dictée"
      style="position:relative;flex:none;width:50px;height:50px;border-radius:50%;cursor:pointer;background:{listening
        ? '#ef4444'
        : '#4f46e5'};color:#fff"
    >
      {#if listening}
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
          <rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" />
        </svg>
      {:else}
        <svg
          viewBox="0 0 24 24"
          width="22"
          height="22"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <rect x="9" y="2" width="6" height="11" rx="3" />
          <path d="M5 10a7 7 0 0 0 14 0" />
          <line x1="12" y1="19" x2="12" y2="22" />
        </svg>
      {/if}
    </div>
  </footer>
</main>

<style>
  .hud {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    overflow: hidden;
    transition: border-color 0.2s ease;
  }
  .hud.listening {
    box-shadow: inset 0 0 0 2px rgba(129, 140, 248, 0.65);
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    -webkit-app-region: drag;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    letter-spacing: 0.2px;
    color: var(--fg);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--fg-dim);
    transition: background 0.2s ease;
  }
  .dot.on {
    background: var(--ok);
    box-shadow: 0 0 8px var(--ok);
  }
  .dot.err {
    background: var(--danger);
    box-shadow: 0 0 8px var(--danger);
  }
  .win-controls {
    display: flex;
    gap: 4px;
  }
  .win-controls .icon {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    color: var(--fg-dim);
  }
  .win-controls .icon:hover,
  .win-controls .icon.active {
    background: var(--panel);
    color: var(--fg);
  }

  .body {
    flex: 1;
    padding: 8px 16px 12px;
    overflow-y: auto;
    line-height: 1.55;
    display: flex;
    flex-direction: column;
  }
  .placeholder {
    color: var(--fg-dim);
    text-align: center;
    font-size: 14px;
    max-width: 28ch;
  }
  .empty {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }
  .big-mic {
    box-shadow: 0 10px 28px rgba(79, 70, 229, 0.5);
    transition:
      transform 0.12s ease,
      background 0.2s ease;
  }
  /* Position icons out of flow so their (WebKitGTK-buggy) intrinsic SVG size
     can never collapse the mic button. */
  .big-mic svg,
  .mic svg {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    pointer-events: none;
  }
  .big-mic:hover {
    transform: scale(1.06);
  }
  .big-mic.on {
    background: var(--danger);
    animation: pulse 1.5s ease-out infinite;
  }
  @keyframes pulse {
    0% {
      box-shadow: 0 0 0 0 rgba(248, 113, 113, 0.55);
    }
    100% {
      box-shadow: 0 0 0 24px rgba(248, 113, 113, 0);
    }
  }
  .transcript {
    font-size: 17px;
  }
  .final {
    color: var(--fg);
  }
  .partial {
    color: var(--accent);
  }
  .error {
    color: var(--danger);
    font-size: 12px;
    margin-bottom: 6px;
  }

  /* history */
  .history-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: var(--fg-dim);
    margin-bottom: 6px;
  }
  .link {
    color: var(--fg-dim);
    font-size: 12px;
  }
  .link:hover {
    color: var(--danger);
  }
  .history-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .history-item {
    display: flex;
    align-items: stretch;
    gap: 6px;
  }
  .entry-text {
    flex: 1;
    text-align: left;
    background: var(--panel);
    border-radius: 8px;
    padding: 7px 9px;
    display: block;
    min-width: 0;
  }
  .entry-time {
    display: block;
    margin-bottom: 2px;
  }
  .entry-text:hover {
    background: rgba(255, 255, 255, 0.07);
  }
  .entry-time {
    font-size: 10px;
    color: var(--fg-dim);
  }
  .entry-body {
    color: var(--fg);
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .entry-del {
    width: 28px;
    border-radius: 8px;
    color: var(--fg-dim);
    background: var(--panel);
  }
  .entry-del:hover {
    color: var(--danger);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.18);
    background: #1b1e29;
  }
  .meter {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    background: var(--panel);
    overflow: hidden;
  }
  .meter-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.08s linear;
  }
  .copied {
    font-size: 11px;
    color: var(--ok);
    white-space: nowrap;
  }
  .ghost {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    color: var(--fg-dim);
    background: var(--panel);
  }
  .ghost:hover:not(:disabled) {
    color: var(--fg);
  }
  .ghost:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .mic {
    box-shadow: 0 4px 16px rgba(79, 70, 229, 0.55);
    transition:
      transform 0.1s ease,
      background 0.2s ease;
  }
  .mic:hover {
    transform: scale(1.06);
  }
  .mic.on {
    background: var(--danger);
    box-shadow: 0 4px 16px rgba(248, 113, 113, 0.55);
  }
</style>
