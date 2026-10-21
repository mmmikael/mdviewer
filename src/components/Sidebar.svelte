<script lang="ts">
  import type { Heading } from "../lib/toc";

  let {
    headings,
    active,
    onselect,
  }: { headings: Heading[]; active: string | null; onselect: (id: string) => void } = $props();
</script>

<aside class="sidebar">
  <div class="sidebar-top" data-tauri-drag-region></div>
  <nav aria-label="Table of contents">
    {#if headings.length}
      <div class="section-title">Contents</div>
      <ul>
        {#each headings as heading (heading.id)}
          <li>
            <button
              type="button"
              class="item"
              class:active={heading.id === active}
              style:padding-left="{10 + heading.depth * 14}px"
              title={heading.text}
              onclick={() => onselect(heading.id)}
            >
              {heading.text}
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty">No headings</p>
    {/if}
  </nav>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-width);
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .sidebar-top {
    height: var(--toolbar-height);
    flex: none;
  }

  nav {
    flex: 1;
    overflow-y: auto;
    padding: 0 10px 16px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-tertiary);
    padding: 4px 10px 6px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .item {
    display: block;
    width: 100%;
    text-align: left;
    border: 0;
    background: transparent;
    padding: 4px 10px;
    border-radius: 6px;
    line-height: 16px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .item:hover {
    background: var(--hover);
  }

  .item.active {
    background: var(--selected);
    font-weight: 500;
  }

  .empty {
    color: var(--text-tertiary);
    padding: 4px 10px;
    margin: 0;
  }
</style>
