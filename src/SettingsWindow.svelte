<script lang="ts">
  import { onMount } from "svelte";

  import { DEFAULT_SETTINGS, ipc, on, type ContentWidth, type Settings, type ThemePref } from "./lib/ipc";
  import { basename } from "./lib/paths";

  let settings = $state<Settings>({ ...DEFAULT_SETTINGS });

  const themes: { value: ThemePref; label: string }[] = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];
  const widths: { value: ContentWidth; label: string }[] = [
    { value: "narrow", label: "Narrow" },
    { value: "normal", label: "Normal" },
    { value: "wide", label: "Wide" },
    { value: "full", label: "Full Width" },
  ];

  function save(patch: Partial<Settings>) {
    settings = { ...settings, ...patch };
    void ipc.setSettings($state.snapshot(settings));
  }

  async function chooseEditor() {
    const app = await ipc.chooseEditorApp();
    if (app) save({ editorApp: app });
  }

  onMount(() => {
    const unlisten = on("settings:changed", (next) => (settings = next));
    void (async () => {
      settings = await ipc.getSettings();
      await ipc.showWindow();
    })();
    return () => void unlisten.then((fn) => fn());
  });
</script>

<form class="settings" onsubmit={(e) => e.preventDefault()}>
  <section>
    <div class="row">
      <span class="label">Appearance</span>
      <div class="segmented" role="radiogroup" aria-label="Appearance">
        {#each themes as theme (theme.value)}
          <button
            type="button"
            role="radio"
            aria-checked={settings.theme === theme.value}
            class:selected={settings.theme === theme.value}
            onclick={() => save({ theme: theme.value })}>{theme.label}</button
          >
        {/each}
      </div>
    </div>

    <div class="row">
      <label class="label" for="width">Text width</label>
      <select
        id="width"
        value={settings.contentWidth}
        onchange={(e) => save({ contentWidth: e.currentTarget.value as ContentWidth })}
      >
        {#each widths as width (width.value)}
          <option value={width.value}>{width.label}</option>
        {/each}
      </select>
    </div>

    <div class="row">
      <span class="label">Text size</span>
      <div class="inline">
        <span class="value">{Math.round(settings.zoom * 100)}%</span>
        <button type="button" class="push-button" disabled={settings.zoom === 1} onclick={() => save({ zoom: 1 })}
          >Reset</button
        >
      </div>
    </div>
  </section>

  <section>
    <div class="row">
      <span class="label">Sidebar</span>
      <label class="check">
        <input
          type="checkbox"
          checked={settings.showSidebar}
          onchange={(e) => save({ showSidebar: e.currentTarget.checked })}
        />
        Show table of contents
      </label>
    </div>
    <div class="row">
      <span class="label">Privacy</span>
      <label class="check">
        <input
          type="checkbox"
          checked={settings.blockRemoteImages}
          onchange={(e) => save({ blockRemoteImages: e.currentTarget.checked })}
        />
        Block images from the internet
      </label>
    </div>
  </section>

  <section>
    <div class="row">
      <span class="label">Open in Editor</span>
      <div class="inline">
        <span class="value" title={settings.editorApp ?? ""}>
          {settings.editorApp ? basename(settings.editorApp).replace(/\.app$/, "") : "Default text editor"}
        </span>
        <button type="button" class="push-button" onclick={chooseEditor}>Choose…</button>
        {#if settings.editorApp}
          <button type="button" class="push-button" onclick={() => save({ editorApp: null })}>Reset</button>
        {/if}
      </div>
    </div>
  </section>
</form>

<style>
  .settings {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  section {
    background: var(--content-bg);
    border: 0.5px solid var(--hairline);
    border-radius: 10px;
    padding: 2px 14px;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 40px;
  }

  .row + .row {
    border-top: 0.5px solid var(--hairline);
  }

  .label {
    font-weight: 500;
  }

  .inline {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .value {
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  input[type="checkbox"] {
    accent-color: var(--accent);
    margin: 0;
  }

  select {
    font: inherit;
    color: inherit;
  }

  .segmented {
    display: inline-flex;
    padding: 2px;
    border-radius: 7px;
    background: var(--hover);
  }

  .segmented button {
    border: 0;
    background: transparent;
    padding: 3px 12px;
    border-radius: 5px;
  }

  .segmented button.selected {
    background: var(--control-bg);
    box-shadow: 0 0.5px 1.5px rgba(0, 0, 0, 0.2);
  }

  .push-button:disabled {
    opacity: 0.5;
  }
</style>
