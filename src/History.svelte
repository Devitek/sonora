<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, listen, isTauri } from "./lib/tauri";
  import { copyText } from "./lib/clipboard";
  import { loadHistory, saveHistory } from "./lib/history";
  import type { HistoryEntry } from "./lib/types";

  let history = $state<HistoryEntry[]>([]);
  let copied = $state("");
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;

  function flashCopied(id: string) {
    copied = id;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied = ""), 1200);
  }

  async function copyEntry(entry: HistoryEntry) {
    await copyText(entry.text);
    flashCopied(entry.id);
  }

  async function deleteEntry(id: string) {
    history = history.filter((e) => e.id !== id);
    await saveHistory(history);
    void invoke("broadcast_history_changed");
  }

  async function clearHistory() {
    history = [];
    await saveHistory(history);
    void invoke("broadcast_history_changed");
  }

  function fmtTime(ts: number): string {
    return new Date(ts).toLocaleString(undefined, {
      day: "2-digit",
      month: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  onMount(() => {
    if (isTauri) {
      void loadHistory().then((h) => (history = h));
      // The bar (and delete/clear from elsewhere) broadcast on every change.
      const unlisten = listen("sonora://history-changed", () => {
        void loadHistory().then((h) => (history = h));
      });
      return () => void unlisten.then((fn) => fn());
    }
    // Demo data for the screenshot harness / browser preview (no backend).
    history = [
      { id: "h1", text: "Peux-tu préparer un résumé de la réunion de ce matin ?", createdAt: Date.now() - 1000 * 60 * 7 },
      { id: "h2", text: "Ajouter une section FAQ à la documentation du projet.", createdAt: Date.now() - 1000 * 60 * 60 * 3 },
      { id: "h3", text: "Réserve une salle pour la rétro de vendredi après-midi.", createdAt: Date.now() - 1000 * 60 * 60 * 27 },
    ];
  });
</script>

<div class="hist-root">
  {#if history.length === 0}
    <p class="placeholder">Aucune dictée enregistrée pour l'instant.<br />Vos dictées apparaîtront ici.</p>
  {:else}
    <div class="hist-head">
      <span>{history.length} entrée{history.length > 1 ? "s" : ""}</span>
      <button class="link" onclick={clearHistory}>Tout effacer</button>
    </div>
    <ul class="hist-list">
      {#each history as entry (entry.id)}
        <li class="hist-item">
          <button class="entry-text" title="Copier" onclick={() => copyEntry(entry)}>
            <span class="entry-time">
              {fmtTime(entry.createdAt)}
              {#if copied === entry.id}<span class="entry-copied">copié ✓</span>{/if}
            </span>
            <span class="entry-body">{entry.text}</span>
          </button>
          <button class="entry-del" title="Supprimer" onclick={() => deleteEntry(entry.id)}>✕</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .hist-root {
    height: 100%;
    overflow-y: auto;
    padding: 14px;
  }
  .placeholder {
    text-align: center;
    color: var(--fg-dim);
    font-size: 12.5px;
    line-height: 1.6;
    padding: 40px 0;
  }
  .hist-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    color: var(--fg-dim);
    margin-bottom: 10px;
  }
  .link {
    color: var(--fg-dim);
    font-size: 12px;
  }
  .link:hover {
    color: var(--danger);
  }
  .hist-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hist-item {
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
    padding: 10px 12px;
    display: block;
    min-width: 0;
  }
  .entry-text:hover {
    border-color: var(--accent-soft-border);
  }
  .entry-time {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 3px;
    font-family: ui-monospace, "JetBrains Mono", monospace;
    font-size: 9.5px;
    color: var(--fg-dim);
  }
  .entry-copied {
    color: var(--ok);
  }
  .entry-body {
    color: var(--fg-mid);
    font-size: 13px;
    line-height: 1.5;
    display: block;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .entry-del {
    width: 30px;
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
</style>
