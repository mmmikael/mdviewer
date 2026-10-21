<script lang="ts">
  import { onMount } from "svelte";

  let {
    query = $bindable(""),
    position,
    count,
    onsearch,
    onnext,
    onprevious,
    onclose,
  }: {
    query: string;
    position: number;
    count: number;
    onsearch: (query: string) => void;
    onnext: () => void;
    onprevious: () => void;
    onclose: () => void;
  } = $props();

  let input: HTMLInputElement;

  export function focus() {
    input?.focus();
    input?.select();
  }

  onMount(focus);

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      if (event.shiftKey) onprevious();
      else onnext();
    } else if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }
</script>

<div class="find-bar" role="search">
  <div class="field">
    <svg width="12" height="12" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="6.5" cy="6.5" r="5" fill="none" stroke="currentColor" stroke-width="1.6" />
      <path d="M10.5 10.5 15 15" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    </svg>
    <input
      bind:this={input}
      bind:value={query}
      oninput={() => onsearch(query)}
      {onkeydown}
      type="search"
      placeholder="Find"
      spellcheck="false"
      autocomplete="off"
      aria-label="Find in document"
    />
    {#if query}
      <span class="count">{count ? `${position} of ${count}` : "Not found"}</span>
    {/if}
  </div>
  <div class="segmented">
    <button type="button" class="icon-button" onclick={onprevious} disabled={!count} aria-label="Previous match">
      <svg width="10" height="10" viewBox="0 0 10 10"><path d="M2 6.5 5 3.5l3 3" fill="none" stroke="currentColor" stroke-width="1.5" /></svg>
    </button>
    <button type="button" class="icon-button" onclick={onnext} disabled={!count} aria-label="Next match">
      <svg width="10" height="10" viewBox="0 0 10 10"><path d="M2 3.5 5 6.5l3-3" fill="none" stroke="currentColor" stroke-width="1.5" /></svg>
    </button>
  </div>
  <button type="button" class="push-button" onclick={onclose}>Done</button>
</div>

<style>
  .find-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--hairline);
    background: var(--content-bg);
  }

  .field {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    border-radius: 6px;
    background: var(--hover);
    color: var(--text-secondary);
  }

  .field:focus-within {
    outline: 3px solid color-mix(in srgb, var(--accent) 55%, transparent);
  }

  input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    font: inherit;
    color: var(--text);
    -webkit-user-select: text;
    user-select: text;
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  .count {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .segmented {
    display: flex;
  }

  .icon-button:disabled {
    opacity: 0.4;
  }
</style>
