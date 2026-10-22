<script lang="ts">
  import { Menu } from "@tauri-apps/api/menu";

  import { basename, dirname } from "../lib/paths";

  let {
    recent,
    error,
    onopen,
    onopenrecent,
    onremoverecent,
    onclearrecent,
  }: {
    recent: string[];
    error: string | null;
    onopen: () => void;
    onopenrecent: (path: string) => void;
    onremoverecent: (path: string) => void;
    onclearrecent: () => void;
  } = $props();

  async function showRowMenu(event: MouseEvent, path: string) {
    event.preventDefault();
    const menu = await Menu.new({
      items: [
        { id: "open", text: "Open", action: () => onopenrecent(path) },
        { id: "remove", text: "Remove from Recent", action: () => onremoverecent(path) },
        { item: "Separator" },
        { id: "clear", text: "Clear Recent", action: onclearrecent },
      ],
    });
    await menu.popup();
  }
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
      <div class="recent-header">
        <span class="recent-title">Recent</span>
        <button type="button" class="clear" onclick={onclearrecent}>Clear</button>
      </div>
      <ul>
        {#each recent.slice(0, 8) as path (path)}
          <li oncontextmenu={(event) => void showRowMenu(event, path)}>
            <button type="button" class="open" onclick={() => onopenrecent(path)} title={path}>
              <span class="name">{basename(path)}</span>
              <span class="folder">{basename(dirname(path))}</span>
            </button>
            <button
              type="button"
              class="remove"
              onclick={() => onremoverecent(path)}
              title="Remove from Recent"
              aria-label="Remove {basename(path)} from Recent"
            >
              <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
                <path d="M2 2l6 6M8 2 2 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              </svg>
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

  .recent-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    /* Right inset matches where the folder names end: the row's 4px padding
       plus the remove button (18px wide, 4px margin) shown on hover. */
    padding: 0 26px 4px 8px;
  }

  .recent-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-tertiary);
  }

  .clear {
    border: 0;
    background: transparent;
    padding: 0;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .clear:hover {
    color: var(--text);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: center;
    border-radius: 6px;
  }

  li:hover {
    background: var(--hover);
  }

  .open {
    flex: 1;
    min-width: 0;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    border: 0;
    background: transparent;
    padding: 5px 4px 5px 8px;
    text-align: left;
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

  /* Shown on hover, like the close button on a Safari tab. */
  .remove {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-right: 4px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    opacity: 0;
  }

  li:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .remove:hover {
    background: var(--selected);
    color: var(--text);
  }
</style>
