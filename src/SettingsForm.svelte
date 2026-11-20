<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "./lib/tauri";
  import Select from "./lib/Select.svelte";
  import {
    getAutoType,
    setAutoType,
    getTheme,
    setTheme,
    type ThemePref,
  } from "./lib/settings";
  import {
    PROVIDERS,
    CLEANUP_ENGINES,
    CLEANUP_MODEL_PLACEHOLDER,
    MODEL_PLACEHOLDER,
    KEYED_PROVIDERS,
  } from "./lib/providers";
  import type { Settings } from "./lib/types";
  import { t } from "./lib/i18n";

  // The parent panel owns the window chrome; it passes a close callback.
  let { onClose }: { onClose?: () => void } = $props();

  interface AudioInput {
    id: string;
    name: string;
    is_default: boolean;
  }

  let settings = $state<Settings>({ provider: "gemini" });
  let apiKey = $state("");
  let hasKey = $state(false);
  let cleanupKey = $state("");
  let hasCleanupKey = $state(false);
  let settingsMsg = $state("");
  let autoType = $state(false);
  let audioInputs = $state<AudioInput[]>([]);

  // Microphone picker options: system default + each device, plus a synthetic
  // entry when the saved device is currently unavailable (e.g. unplugged USB).
  const micOptions = $derived.by(() => {
    const opts = [
      { id: "", label: t("settings.mic.systemDefault") },
      ...audioInputs.map((d) => ({
        id: d.id,
        label: d.is_default
          ? t("settings.mic.deviceDefaultSuffix", { name: d.name })
          : d.name,
      })),
    ];
    const sel = settings.input_device;
    if (sel && !audioInputs.some((d) => d.id === sel)) {
      opts.push({ id: sel, label: t("settings.mic.selectedUnavailable") });
    }
    return opts;
  });

  // Theme: only used here to highlight the active segment. The panel applies the
  // actual data-theme attribute (and reloads it on the broadcast below).
  let theme = $state<ThemePref>("system");

  const provider = $derived(settings.provider ?? "gemini");
  const needsKey = $derived(KEYED_PROVIDERS.includes(provider));
  const cleanupProvider = $derived(settings.cleanup_provider ?? "gemini");

  async function loadSettings() {
    const s = await invoke<Settings>("get_settings");
    settings = s && Object.keys(s).length ? s : { provider: "gemini" };
    if (!settings.provider) settings.provider = "gemini";
    if (!settings.prompts) settings.prompts = [];
    await refreshHasKey();
    await refreshHasCleanupKey();
  }
  async function refreshHasKey() {
    hasKey = await invoke<boolean>("has_api_key", { provider });
  }
  async function refreshHasCleanupKey() {
    hasCleanupKey = await invoke<boolean>("has_api_key", { provider: "cleanup" });
  }

  async function loadAudioInputs() {
    audioInputs = (await invoke<AudioInput[]>("list_audio_inputs")) ?? [];
  }

  /** Tell the bar (and the panel) to reload persisted settings. */
  function broadcast() {
    void invoke("broadcast_settings_changed");
  }

  async function saveSettings() {
    await invoke("save_settings", { settings: $state.snapshot(settings) });
    if (apiKey) {
      await invoke("save_api_key", { provider, key: apiKey });
      apiKey = "";
      await refreshHasKey();
    }
    if (cleanupKey) {
      await invoke("save_api_key", { provider: "cleanup", key: cleanupKey });
      cleanupKey = "";
      await refreshHasCleanupKey();
    }
    settingsMsg = t("settings.saved");
    setTimeout(() => (settingsMsg = ""), 1500);
    broadcast();
  }

  async function clearKey() {
    await invoke("save_api_key", { provider, key: "" });
    apiKey = "";
    await refreshHasKey();
    broadcast();
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

  async function chooseTheme(v: ThemePref) {
    theme = v;
    await setTheme(v);
    broadcast();
  }
  async function toggleAutoType() {
    autoType = !autoType;
    await setAutoType(autoType);
    broadcast();
  }
  async function toggleCleanup() {
    settings.cleanup_enabled = !settings.cleanup_enabled;
    await invoke("save_settings", { settings: $state.snapshot(settings) });
    broadcast();
  }

  onMount(() => {
    if (isTauri) {
      void loadSettings();
      void loadAudioInputs();
      void getAutoType().then((v) => (autoType = v));
      void getTheme().then((v) => (theme = v));
    } else {
      // Demo data for the screenshot harness / browser preview (no backend).
      hasKey = true;
      hasCleanupKey = true;
      autoType = true;
      audioInputs = [
        { id: "coreaudio:builtin", name: "Micro intégré (MacBook Pro)", is_default: true },
        { id: "coreaudio:jabra", name: "Casque USB — Jabra Evolve", is_default: false },
        { id: "coreaudio:c920", name: "Webcam C920", is_default: false },
      ];
      settings = {
        provider: "gemini",
        language: "fr",
        input_device: "coreaudio:jabra",
        cleanup_enabled: true,
        cleanup_provider: "gemini",
        prompts: [
          { id: "p1", name: "Reformuler — pro", prompt: "Réécris ce texte dans un ton professionnel et concis." },
          { id: "p2", name: "Commande terminal", prompt: "Convertis cette demande en une commande shell." },
        ],
      };
    }
  });
</script>

<div class="form-root">
  <div class="s-body">
    <!-- quick toggles -->
    <div class="quick">
      <button class="quick-card" class:on={autoType} onclick={toggleAutoType}>
        <span class="q-ico">⌨</span>
        <span class="q-name">{t("settings.quick.autotype")}</span>
        <span class="q-state">{autoType ? t("settings.state.on") : t("settings.state.off")}</span>
      </button>
      <button class="quick-card" class:on={settings.cleanup_enabled === true} onclick={toggleCleanup}>
        <span class="q-ico">✦</span>
        <span class="q-name">{t("settings.quick.cleanup")}</span>
        <span class="q-state">{settings.cleanup_enabled ? t("settings.state.on") : t("settings.state.off")}</span>
      </button>
    </div>

    <!-- Thème -->
    <div class="field">
      <span>{t("settings.theme.label")}</span>
      <div class="seg">
        <button class="seg-btn" class:active={theme === "system"} onclick={() => chooseTheme("system")}>{t("settings.theme.system")}</button>
        <button class="seg-btn" class:active={theme === "light"} onclick={() => chooseTheme("light")}>{t("settings.theme.light")}</button>
        <button class="seg-btn" class:active={theme === "dark"} onclick={() => chooseTheme("dark")}>{t("settings.theme.dark")}</button>
      </div>
    </div>

    <!-- Microphone (source de capture) -->
    <div class="field">
      <span>{t("settings.mic.label")}</span>
      <div class="mic-row">
        <div class="mic-select">
          <Select bind:value={settings.input_device} options={micOptions} />
        </div>
        <button class="mic-refresh" title={t("settings.mic.refreshTitle")} aria-label={t("settings.mic.refreshAria")} onclick={loadAudioInputs}>↻</button>
      </div>
      <span class="mic-hint">{t("settings.mic.hint")}</span>
    </div>

    <div class="field">
      <span>{t("settings.provider.label")}</span>
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
        <span>{t("settings.apiKey.label")} {hasKey ? t("settings.apiKey.savedSuffix") : ""}</span>
        <input
          type="password"
          bind:value={apiKey}
          placeholder={hasKey ? t("settings.apiKey.placeholderKeep") : t("settings.apiKey.placeholderPaste")}
        />
      </label>
      {#if provider === "gemini" && apiKey.startsWith("AQ.")}
        <p class="warn">{t("settings.apiKey.geminiWarn")}</p>
      {/if}
      {#if hasKey}
        <button class="link" onclick={clearKey}>{t("settings.apiKey.clear")}</button>
      {/if}
    {/if}

    <label class="field">
      <span>{t("settings.model.label")}</span>
      <input type="text" bind:value={settings.model} placeholder={MODEL_PLACEHOLDER[provider] || t("settings.placeholder.default")} />
    </label>

    {#if provider === "openai-compatible"}
      <label class="field">
        <span>{t("settings.baseUrl.label")}</span>
        <input type="text" bind:value={settings.base_url} placeholder="http://localhost:8000/v1" />
      </label>
    {/if}

    {#if provider === "whisper-local"}
      <label class="field">
        <span>{t("settings.whisper.label")}</span>
        <input type="text" bind:value={settings.whisper_model} placeholder={t("settings.whisper.placeholder")} />
      </label>
    {/if}

    <label class="field">
      <span>{t("settings.language.label")}</span>
      <input type="text" bind:value={settings.language} placeholder={t("settings.language.placeholder")} />
    </label>

    <div class="section-sep">{t("settings.cleanup.section")}</div>

    <label class="field check">
      <input type="checkbox" bind:checked={settings.cleanup_enabled} />
      <span>{t("settings.cleanup.autoLabel")}</span>
    </label>

    {#if settings.cleanup_enabled}
      <div class="field">
        <span>{t("settings.cleanup.engineLabel")}</span>
        <Select bind:value={settings.cleanup_provider} options={CLEANUP_ENGINES} />
      </div>
      <label class="field">
        <span>{t("settings.cleanup.modelLabel")}</span>
        <input
          type="text"
          bind:value={settings.cleanup_model}
          placeholder={CLEANUP_MODEL_PLACEHOLDER[cleanupProvider] ?? t("settings.placeholder.default")}
        />
      </label>
      {#if cleanupProvider === "gemini"}
        <p class="hint">{t("settings.cleanup.geminiHint")}</p>
      {:else}
        {#if cleanupProvider === "openai-compatible"}
          <label class="field">
            <span>{t("settings.baseUrl.label")}</span>
            <input type="text" bind:value={settings.cleanup_base_url} placeholder="http://localhost:8000/v1" />
          </label>
        {/if}
        <label class="field">
          <span>{t("settings.cleanup.apiKeyLabel")} {hasCleanupKey ? t("settings.apiKey.savedSuffix") : ""}</span>
          <input
            type="password"
            bind:value={cleanupKey}
            placeholder={hasCleanupKey ? t("settings.apiKey.placeholderKeep") : t("settings.apiKey.placeholderPaste")}
          />
        </label>
      {/if}
    {/if}

    <div class="section-sep">{t("settings.prompts.section")}</div>
    <p class="hint">{t("settings.prompts.hint")}</p>
    {#each settings.prompts ?? [] as p (p.id)}
      <div class="prompt-row">
        <div class="prompt-head">
          <input class="prompt-name" type="text" bind:value={p.name} placeholder={t("settings.prompts.namePlaceholder")} />
          <button class="prompt-del" title={t("settings.prompts.deleteTitle")} onclick={() => removePrompt(p.id)}>✕</button>
        </div>
        <textarea
          class="prompt-text"
          rows="2"
          bind:value={p.prompt}
          placeholder={t("settings.prompts.textPlaceholder")}
        ></textarea>
      </div>
    {/each}
    <button class="add-prompt" onclick={addPrompt}>{t("settings.prompts.add")}</button>
  </div>

  <footer class="s-actions">
    {#if settingsMsg}<span class="ok-msg">{settingsMsg}</span>{/if}
    <button class="cancel-btn" onclick={() => onClose?.()}>{t("common.close")}</button>
    <button class="save-btn" onclick={saveSettings}>{t("common.save")}</button>
  </footer>
</div>

<style>
  .form-root {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .s-body {
    flex: 1;
    overflow-y: auto;
    padding: 14px;
  }
  .s-actions {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    padding: 12px 14px;
    border-top: 1px solid var(--divider);
    background: var(--surface);
  }
  .cancel-btn {
    color: var(--fg-dim);
    font-size: 13px;
    padding: 9px 14px;
    border-radius: 10px;
    border: 1px solid var(--border);
  }
  .cancel-btn:hover {
    color: var(--fg);
  }

  /* quick toggles */
  .quick {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
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

  /* fields */
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
  .mic-row {
    display: flex;
    align-items: stretch;
    gap: 6px;
  }
  .mic-select {
    flex: 1;
    min-width: 0;
  }
  .mic-refresh {
    flex: none;
    width: 38px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--panel);
    color: var(--fg-dim);
    font-size: 15px;
  }
  .mic-refresh:hover {
    color: var(--fg);
    border-color: var(--accent-soft-border);
  }
  .mic-hint {
    font-size: 10.5px;
    color: var(--fg-dim);
    line-height: 1.4;
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
  .link {
    color: var(--fg-dim);
    font-size: 12px;
    align-self: flex-start;
    margin: -4px 0 10px;
  }
  .link:hover {
    color: var(--danger);
  }
  .ok-msg {
    color: var(--ok);
    font-size: 12px;
    margin-right: auto;
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
</style>
