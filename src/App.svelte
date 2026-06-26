<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, listen } from "./lib/tauri";
  import { EVENT_CHANNEL, type BackendEvent, type RecordingState } from "./lib/types";

  let recState = $state<RecordingState>("idle");
  let partial = $state("");
  let finals = $state<string[]>([]);
  let level = $state(0);
  let errorMsg = $state("");

  const listening = $derived(recState === "listening" || recState === "starting");
  const fullText = $derived([...finals, partial].filter(Boolean).join(" ").trim());

  onMount(() => {
    void invoke<string>("app_ready").then((v) => console.log("backend ready:", v));

    const unlisten = listen<BackendEvent>(EVENT_CHANNEL, (ev) => {
      switch (ev.kind) {
        case "state":
          recState = ev.state;
          if (ev.state !== "error") errorMsg = "";
          break;
        case "partial":
          partial = ev.text;
          break;
        case "final":
          if (ev.text.trim()) finals = [...finals, ev.text.trim()];
          partial = "";
          break;
        case "level":
          level = ev.rms;
          break;
        case "error":
          errorMsg = ev.message;
          recState = "error";
          break;
      }
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  });

  async function toggle() {
    if (listening) {
      await invoke("stop_recording");
      recState = "idle";
    } else {
      recState = "starting";
      try {
        await invoke("start_recording");
        // backend will emit "state: listening" once the stream is live
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
      <button class="icon" title="Masquer (reste dans le tray)" onclick={hideWindow}>—</button>
    </div>
  </header>

  <section class="body">
    {#if errorMsg}
      <p class="error">{errorMsg}</p>
    {/if}
    {#if fullText}
      <p class="transcript">
        {#each finals as f}<span class="final">{f} </span>{/each}<span class="partial">{partial}</span>
      </p>
    {:else}
      <p class="placeholder">
        {recState === "idle" ? "Prêt. Appuie sur le micro pour dicter." : "À l'écoute…"}
      </p>
    {/if}
  </section>

  <footer class="actions">
    <div class="meter" aria-hidden="true">
      <span class="meter-fill" style={`width:${Math.min(100, Math.round(level * 140))}%`}></span>
    </div>
    <button class="ghost" onclick={clearAll} disabled={!fullText} title="Effacer">⌫</button>
    <button class="mic" class:on={listening} onclick={toggle} title="Démarrer / arrêter">
      {listening ? "■" : "●"}
    </button>
  </footer>
</main>

<style>
  .hud {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 14px;
    overflow: hidden;
    backdrop-filter: blur(18px);
    transition: border-color 0.2s ease;
  }
  .hud.listening {
    border-color: rgba(129, 140, 248, 0.5);
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
  .win-controls .icon {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    color: var(--fg-dim);
  }
  .win-controls .icon:hover {
    background: var(--panel);
    color: var(--fg);
  }

  .body {
    flex: 1;
    padding: 6px 14px 10px;
    overflow-y: auto;
    line-height: 1.5;
  }
  .placeholder {
    color: var(--fg-dim);
  }
  .transcript {
    font-size: 15px;
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

  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
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
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--accent-strong);
    color: white;
    font-size: 16px;
    display: grid;
    place-items: center;
    transition: transform 0.1s ease, background 0.2s ease;
  }
  .mic:hover {
    transform: scale(1.05);
  }
  .mic.on {
    background: var(--danger);
  }
</style>
