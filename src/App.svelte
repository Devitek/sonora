<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, listen } from "./lib/tauri";
  import { copyText } from "./lib/clipboard";
  import { loadHistory, saveHistory, newEntry } from "./lib/history";
  import {
    getAutoType,
    setAutoType,
    getTheme,
    setTheme,
    type ThemePref,
  } from "./lib/settings";
  import Select from "./lib/Select.svelte";
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

  let history = $state<HistoryEntry[]>([]);
  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  let autoType = $state(false);

  // Options dropdown
  let menuOpen = $state(false);
  let menuTab = $state<"history" | "settings">("history");

  let settings = $state<Settings>({ provider: "gemini" });
  let apiKey = $state("");
  let hasKey = $state(false);
  let settingsMsg = $state("");
  /** true once a usable transcription config resolves (drives onboarding). */
  let configured = $state(true);
  let cleaning = $state(false);
  let procLabel = $state("Reformulation…");
  let cleanupKey = $state("");
  let hasCleanupKey = $state(false);
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

  const PROVIDERS = [
    { id: "gemini", label: "Gemini Live (streaming)" },
    { id: "mistral", label: "Mistral (Voxtral)" },
    { id: "openai", label: "OpenAI Whisper" },
    { id: "groq", label: "Groq Whisper (rapide)" },
    { id: "openai-compatible", label: "OpenAI-compatible" },
    { id: "whisper-local", label: "Whisper local (offline)" },
  ];
  const CLEANUP_ENGINES = [
    { id: "gemini", label: "Gemini" },
    { id: "mistral", label: "Mistral" },
    { id: "groq", label: "Groq (rapide)" },
    { id: "openai", label: "OpenAI" },
    { id: "openai-compatible", label: "OpenAI-compatible (local…)" },
  ];
  const CLEANUP_MODEL_PLACEHOLDER: Record<string, string> = {
    gemini: "gemini-2.5-flash",
    mistral: "mistral-small-latest",
    groq: "llama-3.3-70b-versatile",
    openai: "gpt-4o-mini",
    "openai-compatible": "nom du modèle",
  };
  const MODEL_PLACEHOLDER: Record<string, string> = {
    gemini: "gemini-2.5-flash-native-audio-latest",
    mistral: "voxtral-mini-latest",
    openai: "whisper-1",
    groq: "whisper-large-v3",
    "openai-compatible": "whisper-1",
    "whisper-local": "",
  };
  const provider = $derived(settings.provider ?? "gemini");
  const needsKey = $derived(
    ["gemini", "mistral", "openai", "groq", "openai-compatible"].includes(provider),
  );
  const cleanupEnabled = $derived(settings.cleanup_enabled === true);
  const cleanupProvider = $derived(settings.cleanup_provider ?? "gemini");

  const listening = $derived(recState === "listening" || recState === "starting");
  const finalsText = $derived(finals.join(" ").trim());
  const fullText = $derived([...finals, partial].filter(Boolean).join(" ").trim());
  const hasText = $derived(fullText.length > 0);

  // Grow/shrink the transparent window to fit the bar + capsule + menu, so there
  // is no big transparent dead-zone capturing clicks (Spotlight-style).
  let lastBarHeight = 0;
  function measureAndResize() {
    let bottom = 0;
    for (const sel of [".bar", ".capsule", ".panel", ".prompt-menu"]) {
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
      menuOpen,
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
    // settings / history: open the dropdown with realistic config + data
    autoType = true;
    hasKey = true;
    hasCleanupKey = true;
    settings = {
      provider: "gemini",
      language: "fr",
      cleanup_enabled: true,
      cleanup_provider: "gemini",
      prompts: [
        { id: "p1", name: "Reformuler — pro", prompt: "Réécris ce texte dans un ton professionnel et concis." },
        { id: "p2", name: "Commande terminal", prompt: "Convertis cette demande en une commande shell." },
      ],
    };
    menuOpen = true;
    menuTab = mode === "history" ? "history" : "settings";
    if (mode === "history") {
      history = [
        { id: "h1", text: "Peux-tu préparer un résumé de la réunion de ce matin ?", createdAt: Date.now() - 1000 * 60 * 7 },
        { id: "h2", text: "Ajouter une section FAQ à la documentation du projet.", createdAt: Date.now() - 1000 * 60 * 60 * 3 },
        { id: "h3", text: "Réserve une salle pour la rétro de vendredi après-midi.", createdAt: Date.now() - 1000 * 60 * 60 * 27 },
      ];
    }
  }

  onMount(() => {
    // Docs screenshots: `?shot=bar|result|settings|history` renders the real UI
    // in a fixed, realistic state for captures. Completely inert without the
    // query param, so it never affects the shipped app.
    const shotMode = new URLSearchParams(location.search).get("shot");
    if (shotMode) seedShot(shotMode);

    if (!shotMode) {
      void invoke<string>("app_ready").then((v) => console.log("backend ready:", v));
      void loadHistory().then((h) => (history = h));
      void getAutoType().then((v) => (autoType = v));
      void getTheme().then((v) => (theme = v));
      void loadSettings();
      void refreshConfigured();
    }

    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    systemDark = mq.matches;
    const onMq = (e: MediaQueryListEvent) => (systemDark = e.matches);
    mq.addEventListener("change", onMq);

    // Spotlight-style dismiss: Escape closes the menu, or hides the bar.
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (promptMenuOpen) promptMenuOpen = false;
      else if (menuOpen) menuOpen = false;
      else void hideWindow();
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

    // Replay a first-launch CLI action (e.g. `transcript toggle`).
    void invoke<string | null>("take_pending_action").then((a) => {
      if (a) handleControl(a as ControlAction);
    });

    return () => {
      mq.removeEventListener("change", onMq);
      window.removeEventListener("keydown", onKey);
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

  function addPrompt() {
    settings.prompts = [
      ...(settings.prompts ?? []),
      { id: crypto.randomUUID(), name: "", prompt: "" },
    ];
  }

  function removePrompt(id: string) {
    settings.prompts = (settings.prompts ?? []).filter((p) => p.id !== id);
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
      menuOpen = false;
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

  async function toggleAutoType() {
    autoType = !autoType;
    await setAutoType(autoType);
  }

  async function toggleCleanup() {
    settings.cleanup_enabled = !settings.cleanup_enabled;
    await invoke("save_settings", { settings: $state.snapshot(settings) }).catch(
      (e) => (errorMsg = String(e)),
    );
  }

  async function chooseTheme(v: ThemePref) {
    theme = v;
    await setTheme(v);
  }

  async function loadSettings() {
    const s = await invoke<Settings>("get_settings");
    settings = s && Object.keys(s).length ? s : { provider: "gemini" };
    if (!settings.provider) settings.provider = "gemini";
    if (!settings.prompts) settings.prompts = [];
    await refreshHasKey();
    await refreshHasCleanupKey();
  }

  async function refreshHasKey() {
    hasKey = await invoke<boolean>("has_api_key", { provider: provider });
  }

  async function refreshHasCleanupKey() {
    hasCleanupKey = await invoke<boolean>("has_api_key", { provider: "cleanup" });
  }

  async function refreshConfigured() {
    configured = await invoke<boolean>("is_configured");
  }

  function toggleMenu() {
    menuOpen = !menuOpen;
    if (menuOpen) void loadSettings();
  }

  function openSettings() {
    menuOpen = true;
    menuTab = "settings";
    void loadSettings();
  }

  function openHistory() {
    menuOpen = true;
    menuTab = "history";
  }

  async function saveSettings() {
    await invoke("save_settings", { settings: $state.snapshot(settings) });
    if (apiKey) {
      await invoke("save_api_key", { provider: provider, key: apiKey });
      apiKey = "";
      await refreshHasKey();
    }
    if (cleanupKey) {
      await invoke("save_api_key", { provider: "cleanup", key: cleanupKey });
      cleanupKey = "";
      await refreshHasCleanupKey();
    }
    await refreshConfigured();
    settingsMsg = "Enregistré ✓";
    setTimeout(() => (settingsMsg = ""), 1500);
  }

  async function clearKey() {
    await invoke("save_api_key", { provider: provider, key: "" });
    apiKey = "";
    await refreshHasKey();
    await refreshConfigured();
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

<main class="surface" data-tauri-drag-region>
  <div class="anchor">
    <!-- THE BAR -->
    <div class="bar" data-tauri-drag-region>
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
          onclick={() => {
            menuOpen = false;
            promptMenuOpen = !promptMenuOpen;
          }}
          disabled={cleaning}
          title="Reformuler / nettoyer">✦</button
        >
        <button class="icon-btn" class:ok={copied} onclick={copyCurrent} title="Copier">
          {#if copied}✓{:else}⧉{/if}
        </button>
        <button class="icon-btn" onclick={clearAll} title="Effacer">⌫</button>
      {/if}

      <button class="icon-btn menu" class:active={menuOpen} onclick={toggleMenu} title="Options" aria-label="Options">
        <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <line x1="4" y1="7" x2="20" y2="7" /><circle cx="15" cy="7" r="2.4" fill="currentColor" stroke="none" />
          <line x1="4" y1="13" x2="20" y2="13" /><circle cx="9" cy="13" r="2.4" fill="currentColor" stroke="none" />
          <line x1="4" y1="19" x2="20" y2="19" /><circle cx="16" cy="19" r="2.4" fill="currentColor" stroke="none" />
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

    <!-- OPTIONS DROPDOWN -->
    {#if menuOpen}
      <div class="panel">
        <!-- quick toggles -->
        <div class="quick">
          <button class="quick-card" class:on={autoType} onclick={toggleAutoType}>
            <span class="q-ico">⌨</span>
            <span class="q-name">Coller au curseur</span>
            <span class="q-state">{autoType ? "activé" : "désactivé"}</span>
          </button>
          <button class="quick-card" class:on={cleanupEnabled} onclick={toggleCleanup}>
            <span class="q-ico">✦</span>
            <span class="q-name">Nettoyage auto</span>
            <span class="q-state">{cleanupEnabled ? "activé" : "désactivé"}</span>
          </button>
        </div>

        <!-- tabs -->
        <div class="tabs">
          <button class="tab" class:active={menuTab === "history"} onclick={() => (menuTab = "history")}>Historique</button>
          <button class="tab" class:active={menuTab === "settings"} onclick={() => (menuTab = "settings")}>Réglages</button>
        </div>

        {#if menuTab === "history"}
          <div class="tab-body">
            {#if history.length === 0}
              <p class="placeholder">Aucune dictée enregistrée.</p>
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
                    <button class="entry-del" title="Supprimer" onclick={() => deleteEntry(entry.id)}>✕</button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {:else}
          <div class="tab-body">
            <!-- Thème -->
            <div class="field">
              <span>Thème</span>
              <div class="seg">
                <button class="seg-btn" class:active={theme === "system"} onclick={() => chooseTheme("system")}>Système</button>
                <button class="seg-btn" class:active={theme === "light"} onclick={() => chooseTheme("light")}>Clair</button>
                <button class="seg-btn" class:active={theme === "dark"} onclick={() => chooseTheme("dark")}>Sombre</button>
              </div>
            </div>

            <div class="field">
              <span>Fournisseur</span>
              <Select
                bind:value={settings.provider}
                options={PROVIDERS}
                onChange={() => {
                  apiKey = "";
                  void refreshHasKey();
                }}
              />
            </div>

            {#if needsKey}
              <label class="field">
                <span>Clé API {hasKey ? "· enregistrée ✓" : ""}</span>
                <input
                  type="password"
                  bind:value={apiKey}
                  placeholder={hasKey ? "•••• (laisser vide pour garder)" : "Coller la clé"}
                />
              </label>
              {#if provider === "gemini" && apiKey.startsWith("AQ.")}
                <p class="warn">
                  ⚠ Ceci ressemble à un jeton temporaire (Live) qui expire vite. Utilise une clé
                  API AI Studio (commence par « AIza ») : aistudio.google.com/apikey
                </p>
              {/if}
              {#if hasKey}
                <button class="link" onclick={clearKey}>Supprimer la clé enregistrée</button>
              {/if}
            {/if}

            <label class="field">
              <span>Modèle (optionnel)</span>
              <input type="text" bind:value={settings.model} placeholder={MODEL_PLACEHOLDER[provider] || "défaut"} />
            </label>

            {#if provider === "openai-compatible"}
              <label class="field">
                <span>URL de base</span>
                <input type="text" bind:value={settings.base_url} placeholder="http://localhost:8000/v1" />
              </label>
            {/if}

            {#if provider === "whisper-local"}
              <label class="field">
                <span>Chemin du modèle ggml</span>
                <input type="text" bind:value={settings.whisper_model} placeholder="/chemin/ggml-base.bin" />
              </label>
            {/if}

            <label class="field">
              <span>Langue (optionnel)</span>
              <input type="text" bind:value={settings.language} placeholder="fr · vide = auto" />
            </label>

            <div class="section-sep">Nettoyage des hésitations</div>

            <label class="field check">
              <input type="checkbox" bind:checked={settings.cleanup_enabled} />
              <span>Nettoyer automatiquement (retirer « euh », faux départs…)</span>
            </label>

            {#if settings.cleanup_enabled}
              <div class="field">
                <span>Moteur de nettoyage</span>
                <Select bind:value={settings.cleanup_provider} options={CLEANUP_ENGINES} />
              </div>
              <label class="field">
                <span>Modèle de nettoyage</span>
                <input
                  type="text"
                  bind:value={settings.cleanup_model}
                  placeholder={CLEANUP_MODEL_PLACEHOLDER[cleanupProvider] ?? "défaut"}
                />
              </label>
              {#if cleanupProvider === "gemini"}
                <p class="hint">Utilise la clé Gemini configurée plus haut.</p>
              {:else}
                {#if cleanupProvider === "openai-compatible"}
                  <label class="field">
                    <span>URL de base</span>
                    <input type="text" bind:value={settings.cleanup_base_url} placeholder="http://localhost:8000/v1" />
                  </label>
                {/if}
                <label class="field">
                  <span>Clé API nettoyage {hasCleanupKey ? "· enregistrée ✓" : ""}</span>
                  <input
                    type="password"
                    bind:value={cleanupKey}
                    placeholder={hasCleanupKey ? "•••• (laisser vide pour garder)" : "Coller la clé"}
                  />
                </label>
              {/if}
            {/if}

            <div class="section-sep">Prompts de reformulation</div>
            <p class="hint">
              Appliquez-les à une dictée via le bouton ✦. Ils utilisent le moteur configuré
              ci-dessus.
            </p>
            {#each settings.prompts ?? [] as p (p.id)}
              <div class="prompt-row">
                <div class="prompt-head">
                  <input
                    class="prompt-name"
                    type="text"
                    bind:value={p.name}
                    placeholder="Nom (ex. Formel, Commande terminal…)"
                  />
                  <button class="prompt-del" title="Supprimer" onclick={() => removePrompt(p.id)}
                    >✕</button
                  >
                </div>
                <textarea
                  class="prompt-text"
                  rows="2"
                  bind:value={p.prompt}
                  placeholder="Instruction, ex. « Réécris ce texte de manière formelle et professionnelle. »"
                ></textarea>
              </div>
            {/each}
            <button class="add-prompt" onclick={addPrompt}>+ Ajouter un prompt</button>

            <div class="settings-actions">
              {#if settingsMsg}<span class="ok-msg">{settingsMsg}</span>{/if}
              <button class="save-btn" onclick={saveSettings}>Enregistrer</button>
            </div>

            <button class="hide-row" onclick={hideWindow}>Masquer la fenêtre (reste dans le tray)</button>
          </div>
        {/if}
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
  .panel,
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
    width: 38px;
    height: 38px;
    border-radius: 11px;
  }
  .icon-btn.menu.active {
    color: var(--accent);
    background: var(--accent-soft);
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

  /* ---- dropdown ---- */
  .panel {
    position: absolute;
    right: 0;
    top: calc(100% + 12px);
    z-index: 8;
    width: 340px;
    max-height: calc(100vh - 110px);
    overflow-y: auto;
    border-radius: 18px;
    background: var(--dd-bg);
    border: 1px solid var(--dd-border);
    box-shadow: var(--dd-shadow);
  }
  .quick {
    display: flex;
    gap: 8px;
    padding: 12px;
  }
  .quick-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 5px;
    align-items: flex-start;
    padding: 11px 12px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--panel);
    text-align: left;
  }
  .quick-card.on {
    background: var(--accent-soft);
    border-color: var(--accent-soft-border);
  }
  .q-ico {
    font-size: 16px;
  }
  .q-name {
    font-size: 11.5px;
    font-weight: 500;
    line-height: 1.2;
    color: var(--fg-mid);
  }
  .quick-card.on .q-name {
    color: var(--on-accent-text);
  }
  .q-state {
    font-family: ui-monospace, "JetBrains Mono", monospace;
    font-size: 9.5px;
    color: var(--fg-dim);
  }
  .quick-card.on .q-state {
    color: var(--accent);
  }
  .tabs {
    display: flex;
    gap: 18px;
    padding: 0 14px;
    border-bottom: 1px solid var(--divider);
  }
  .tab {
    position: relative;
    padding: 9px 2px;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--fg-dim);
  }
  .tab.active {
    color: var(--fg);
    font-weight: 600;
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 2px;
    border-radius: 2px;
    background: var(--accent);
  }
  .tab-body {
    padding: 12px;
  }
  .placeholder {
    text-align: center;
    color: var(--fg-dim);
    font-size: 12.5px;
    padding: 28px 0;
  }

  /* history */
  .history-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: var(--fg-dim);
    margin-bottom: 8px;
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
    border: 1px solid var(--border);
    border-radius: 11px;
    padding: 9px 11px;
    display: block;
    min-width: 0;
  }
  .entry-text:hover {
    border-color: var(--accent-soft-border);
  }
  .entry-time {
    display: block;
    margin-bottom: 3px;
    font-family: ui-monospace, "JetBrains Mono", monospace;
    font-size: 9.5px;
    color: var(--fg-dim);
  }
  .entry-body {
    color: var(--fg-mid);
    font-size: 12.5px;
    line-height: 1.45;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .entry-del {
    width: 28px;
    flex: none;
    border-radius: 11px;
    color: var(--fg-dim);
    background: var(--panel);
    border: 1px solid var(--border);
    font-size: 12px;
  }
  .entry-del:hover {
    color: var(--danger);
  }

  /* settings */
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 11px;
    font-size: 11px;
    color: var(--fg-dim);
  }
  .field > span {
    padding-left: 1px;
  }
  .field input:not([type="checkbox"]) {
    appearance: none;
    -webkit-appearance: none;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 9px 11px;
    color: var(--fg);
    font-size: 13px;
    outline: none;
    width: 100%;
  }
  .field input:focus {
    border-color: var(--accent);
  }
  .seg {
    display: flex;
    gap: 3px;
    padding: 3px;
    border-radius: 11px;
    background: var(--panel);
    border: 1px solid var(--border);
  }
  .seg-btn {
    flex: 1;
    padding: 8px 0;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--fg-dim);
    transition: all 0.15s ease;
  }
  .seg-btn.active {
    color: var(--fg);
    background: var(--seg-active);
    box-shadow: var(--seg-shadow);
  }
  .section-sep {
    margin: 6px 0 10px;
    padding-top: 12px;
    border-top: 1px solid var(--divider);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--fg-dim);
  }
  .field.check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--fg);
    font-size: 13px;
  }
  .field.check input {
    width: auto;
    accent-color: var(--accent-strong);
  }
  .hint {
    font-size: 12px;
    color: var(--fg-dim);
    margin: -4px 0 10px;
  }
  .warn {
    font-size: 12px;
    color: #d6a206;
    line-height: 1.45;
    margin: -2px 0 10px;
  }
  .settings-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 4px;
  }
  .ok-msg {
    color: var(--ok);
    font-size: 12px;
  }
  .save-btn {
    background: linear-gradient(140deg, var(--accent), #6d5cff);
    color: #fff;
    border-radius: 10px;
    padding: 9px 18px;
    font-size: 13px;
    font-weight: 600;
    box-shadow: 0 8px 20px rgba(124, 92, 255, 0.3);
  }
  .save-btn:hover {
    filter: brightness(1.06);
  }
  .prompt-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 10px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel);
  }
  .prompt-head {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .prompt-name,
  .prompt-text {
    appearance: none;
    -webkit-appearance: none;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 9px;
    color: var(--fg);
    font-size: 13px;
    font-family: inherit;
  }
  .prompt-name {
    flex: 1;
    width: 100%;
  }
  .prompt-text {
    width: 100%;
    resize: vertical;
    min-height: 46px;
    line-height: 1.45;
  }
  .prompt-name:focus,
  .prompt-text:focus {
    border-color: var(--accent);
    outline: none;
  }
  .prompt-del {
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    color: var(--fg-dim);
    background: var(--panel-2);
  }
  .prompt-del:hover {
    color: var(--danger);
  }
  .add-prompt {
    width: 100%;
    padding: 9px;
    border-radius: 9px;
    border: 1px dashed var(--border);
    color: var(--fg-dim);
    font-size: 13px;
    margin-bottom: 8px;
  }
  .add-prompt:hover {
    color: var(--fg);
    border-color: var(--accent-soft-border);
  }
  .hide-row {
    width: 100%;
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--divider);
    text-align: center;
    font-size: 12px;
    color: var(--fg-dim);
  }
  .hide-row:hover {
    color: var(--fg);
  }
</style>
