<script lang="ts">
  import { basename, dirname } from "../lib/paths";

  let {
    recent,
    error,
    onopen,
    onopenrecent,
  }: {
    recent: string[];
    error: string | null;
    onopen: () => void;
    onopenrecent: (path: string) => void;
  } = $props();
</script>

<div class="empty-state">
  <svg class="glyph" width="64" height="64" viewBox="0 0 64 64" aria-hidden="true">
    <rect x="10" y="4" width="44" height="56" rx="6" fill="none" stroke="currentColor" stroke-width="2.5" />
    <path d="M20 40V24l6 8 6-8v16M40 24v16m-5-5 5 5 5-5" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
  <h1>Open a Markdown file</h1>
  <p class="hint">Drop a file here, or</p>
  <button type="button" class="push-button primary" onclick={onopen}>Open…</button>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if recent.length}
    <div class="recent">
      <div class="recent-title">Recent</div>
      <ul>
        {#each recent.slice(0, 8) as path (path)}
          <li>
            <button type="button" onclick={() => onopenrecent(path)} title={path}>
              <span class="name">{basename(path)}</span>
              <span class="folder">{basename(dirname(path))}</span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .empty-state {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 24px 24px 64px;
    text-align: center;
  }

  .glyph {
    color: var(--text-tertiary);
    margin-bottom: 12px;
  }

  h1 {
    font-size: 17px;
    font-weight: 600;
    margin: 0 0 4px;
  }

  .hint {
    color: var(--text-secondary);
    margin: 0 0 12px;
  }

  .error {
    color: #d70015;
    margin: 16px 0 0;
    max-width: 420px;
  }

  .recent {
    margin-top: 32px;
    width: min(360px, 100%);
    text-align: left;
  }

  .recent-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-tertiary);
    padding: 0 8px 4px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    border: 0;
    background: transparent;
    padding: 5px 8px;
    border-radius: 6px;
    text-align: left;
  }

  li button:hover {
    background: var(--hover);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .folder {
    color: var(--text-tertiary);
    flex: none;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
