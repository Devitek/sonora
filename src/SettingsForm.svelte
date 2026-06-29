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

  // The parent panel owns the window chrome; it passes a close callback.
  let { onClose }: { onClose?: () => void } = $props();

  let settings = $state<Settings>({ provider: "gemini" });
  let apiKey = $state("");
  let hasKey = $state(false);
  let cleanupKey = $state("");
  let hasCleanupKey = $state(false);
  let settingsMsg = $state("");
  let autoType = $state(false);

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
    settingsMsg = "Enregistré ✓";
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
      void getAutoType().then((v) => (autoType = v));
      void getTheme().then((v) => (theme = v));
    } else {
      // Demo data for the screenshot harness / browser preview (no backend).
      hasKey = true;
      hasCleanupKey = true;
      autoType = true;
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
    }
  });
</script>

<div class="form-root">
  <div class="s-body">
    <!-- quick toggles -->
    <div class="quick">
      <button class="quick-card" class:on={autoType} onclick={toggleAutoType}>
        <span class="q-ico">⌨</span>
        <span class="q-name">Coller au curseur</span>
        <span class="q-state">{autoType ? "activé" : "désactivé"}</span>
      </button>
      <button class="quick-card" class:on={settings.cleanup_enabled === true} onclick={toggleCleanup}>
        <span class="q-ico">✦</span>
        <span class="q-name">Nettoyage auto</span>
        <span class="q-state">{settings.cleanup_enabled ? "activé" : "désactivé"}</span>
      </button>
    </div>

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
    <p class="hint">Appliquez-les à une dictée via le bouton ✦. Ils utilisent le moteur configuré ci-dessus.</p>
    {#each settings.prompts ?? [] as p (p.id)}
      <div class="prompt-row">
        <div class="prompt-head">
          <input class="prompt-name" type="text" bind:value={p.name} placeholder="Nom (ex. Formel, Commande terminal…)" />
          <button class="prompt-del" title="Supprimer" onclick={() => removePrompt(p.id)}>✕</button>
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
  </div>

  <footer class="s-actions">
    {#if settingsMsg}<span class="ok-msg">{settingsMsg}</span>{/if}
    <button class="cancel-btn" onclick={() => onClose?.()}>Fermer</button>
    <button class="save-btn" onclick={saveSettings}>Enregistrer</button>
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
