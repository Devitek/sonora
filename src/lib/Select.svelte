<script lang="ts">
  // Custom dropdown: WebKitGTK's native <select> popup renders detached in a
  // screen corner and is unclickable under Wayland/XWayland. This stays in the
  // DOM (inline expand) so it positions and clicks correctly.
  interface Option {
    id: string;
    label: string;
  }

  let {
    value = $bindable(),
    options,
    onChange,
  }: {
    value?: string;
    options: Option[];
    onChange?: (v: string) => void;
  } = $props();

  let open = $state(false);

  const currentLabel = $derived(
    options.find((o) => o.id === value)?.label ?? options[0]?.label ?? "",
  );
  const selectedId = $derived(value ?? options[0]?.id);

  function pick(id: string) {
    value = id;
    open = false;
    onChange?.(id);
  }
</script>

<div class="select" class:open>
  <button type="button" class="select-btn" onclick={() => (open = !open)} aria-haspopup="listbox">
    <span class="label">{currentLabel}</span>
    <span class="caret">▾</span>
  </button>
  {#if open}
    <ul class="select-list" role="listbox">
      {#each options as o}
        <li>
          <button
            type="button"
            class="opt"
            class:sel={o.id === selectedId}
            onclick={() => pick(o.id)}>{o.label}</button
          >
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .select {
    position: relative;
  }
  .select-btn {
    position: relative;
    display: block;
    width: 100%;
    text-align: left;
    padding: 9px 28px 9px 11px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--fg);
    font-size: 13px;
  }
  .select.open .select-btn {
    border-color: var(--accent);
  }
  .caret {
    position: absolute;
    right: 11px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--fg-dim);
    font-size: 11px;
  }
  .select-list {
    list-style: none;
    margin: 6px 0 0;
    padding: 4px;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    box-shadow: var(--dd-shadow);
  }
  .opt {
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px 9px;
    border-radius: 7px;
    color: var(--fg);
    font-size: 13px;
  }
  .opt:hover {
    background: var(--panel);
  }
  .opt.sel {
    background: var(--accent-strong);
    color: #fff;
  }
</style>
