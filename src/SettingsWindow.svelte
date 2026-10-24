<script lang="ts">
  import { onMount, tick } from "svelte";
  import { LogicalSize } from "@tauri-apps/api/dpi";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import {
    DEFAULT_SETTINGS,
    EMPTY_PROVIDER_SETTINGS,
    ipc,
    on,
    type AiModel,
    type AiProvider,
    type AiSettings,
    type ContentWidth,
    type DefaultViewer,
    type ProviderSettings,
    type Settings,
    type ThemePref,
  } from "./lib/ipc";
  import { basename } from "./lib/paths";
  import { DEFAULT_INSTRUCTIONS, maxDocumentChars } from "./lib/tldr";

  let settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  let defaultViewer = $state<DefaultViewer | null>(null);
  let makingDefault = $state(false);
  let defaultError = $state<string | null>(null);

  const WIDTH = 520;
  const TAB_KEY = "mdviewer.settingsTab";
  type Tab = "general" | "ai";

  function storedTab(): Tab {
    // `?tab=ai` is used by debug builds (MDV_DEBUG_SETTINGS) to open a given tab.
    const asked = new URLSearchParams(location.search).get("tab");
    if (asked === "ai" || asked === "general") return asked;
    try {
      return localStorage.getItem(TAB_KEY) === "ai" ? "ai" : "general";
    } catch {
      return "general";
    }
  }

  let tab = $state<Tab>(storedTab());
  let form = $state<HTMLFormElement>();

  // ── AI summary settings ──
  let providers = $state<AiProvider[]>([]);
  let hasKey = $state<boolean | null>(null);
  let keyInput = $state("");
  let keyError = $state<string | null>(null);
  let models = $state<AiModel[]>([]);
  let modelsError = $state<string | null>(null);
  let loadingModels = $state(false);
  let instructionsTimer: ReturnType<typeof setTimeout> | undefined;
  /** What's typed in the instructions box; saving echoes back older text while typing continues. */
  let instructionsDraft = $state<string | null>(null);

  const provider = $derived(providers.find((p) => p.id === settings.ai.provider) ?? null);
  const config = $derived<ProviderSettings>({
    ...EMPTY_PROVIDER_SETTINGS,
    ...(provider ? settings.ai.providers[provider.id] : {}),
  });
  const selectedModel = $derived(models.find((m) => m.id === (config.model || provider?.defaultModel)));
  /** What was saved about the chosen model, shown until the model list is loaded. */
  const savedModel = $derived<AiModel | undefined>(
    config.model
      ? {
          id: config.model,
          name: null,
          contextTokens: config.contextTokens,
          promptPrice: config.promptPrice,
          completionPrice: config.completionPrice,
        }
      : undefined,
  );
  const instructions = $derived(settings.ai.instructions ?? DEFAULT_INSTRUCTIONS);


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

  function saveAi(patch: Partial<AiSettings>) {
    save({ ai: { ...settings.ai, ...patch } });
  }

  function saveProvider(patch: Partial<ProviderSettings>) {
    if (!provider) return;
    saveAi({ providers: { ...settings.ai.providers, [provider.id]: { ...config, ...patch } } });
  }

  function selectTab(next: Tab) {
    tab = next;
    try {
      localStorage.setItem(TAB_KEY, next);
    } catch {
      // Not remembered.
    }
  }

  async function selectProvider(id: string) {
    saveAi({ provider: id });
    await refreshProvider();
  }

  /**
   * Key status and model list for the selected provider. The model list is only
   * fetched by itself when that needs no key: reading a saved key may make
   * macOS ask for keychain access, so that waits until the user asks (focusing
   * the model field, reloading, or just after saving the key).
   */
  async function refreshProvider(loadWithKey = false) {
    await tick();
    const current = provider;
    keyInput = "";
    keyError = null;
    models = [];
    modelsError = null;
    hasKey = null;
    if (!current) return;
    hasKey = await ipc.aiKeyStatus(current.id).catch(() => false);
    if (current.id !== provider?.id) return;
    if (current.needsKey && !hasKey) return;
    if (current.editableUrl && !(config.baseUrl?.trim() || current.baseUrl)) return;
    if (hasKey && !loadWithKey) return;
    await loadModels();
  }

  function onModelFocus() {
    if (!models.length && !loadingModels && !modelsError) void loadModels();
  }

  async function loadModels() {
    const current = provider;
    if (!current) return;
    loadingModels = true;
    modelsError = null;
    try {
      const list = await ipc.aiModels(current.id);
      if (current.id === provider?.id) models = list;
    } catch (error) {
      if (current.id === provider?.id) modelsError = String(error);
    } finally {
      loadingModels = false;
    }
  }

  async function saveKey() {
    if (!provider) return;
    keyError = null;
    try {
      await ipc.aiSetKey(provider.id, keyInput);
      await refreshProvider(true);
    } catch (error) {
      keyError = String(error);
    }
  }

  async function removeKey() {
    if (!provider) return;
    try {
      await ipc.aiSetKey(provider.id, "");
    } catch (error) {
      keyError = String(error);
    }
    await refreshProvider();
  }

  function chooseModel(id: string) {
    const info = models.find((m) => m.id === id.trim());
    saveProvider({
      model: id.trim(),
      contextTokens: info?.contextTokens ?? null,
      promptPrice: info?.promptPrice ?? null,
      completionPrice: info?.completionPrice ?? null,
    });
  }

  function saveInstructions(value: string) {
    instructionsDraft = value;
    clearTimeout(instructionsTimer);
    instructionsTimer = setTimeout(() => {
      const trimmed = value.trim();
      saveAi({ instructions: !trimmed || trimmed === DEFAULT_INSTRUCTIONS.trim() ? null : value });
    }, 400);
  }

  function perMillion(price: number): string {
    return `$${(price * 1_000_000).toFixed(2)}`;
  }

  function modelHint(model: AiModel | undefined): string {
    if (!model) return "";
    const parts: string[] = [];
    if (model.contextTokens) parts.push(`${model.contextTokens.toLocaleString()}-token context`);
    if (model.promptPrice !== null && model.completionPrice !== null) {
      parts.push(
        model.promptPrice === 0 && model.completionPrice === 0
          ? "free"
          : `${perMillion(model.promptPrice)} in / ${perMillion(model.completionPrice)} out per million tokens`,
      );
    }
    return parts.join(" · ");
  }

  async function chooseEditor() {
    const app = await ipc.chooseEditorApp();
    if (app) save({ editorApp: app });
  }

  async function refreshDefaultViewer() {
    defaultViewer = await ipc.getDefaultViewer().catch(() => null);
  }

  async function makeDefault() {
    makingDefault = true;
    defaultError = null;
    try {
      defaultViewer = await ipc.makeDefaultViewer();
    } catch (error) {
      defaultError = String(error);
    } finally {
      makingDefault = false;
    }
  }

  // The window grows and shrinks with the tab's content, like System Settings panes.
  let fitting: Promise<void> | null = null;
  let fitAgain = false;

  function fitWindow() {
    if (fitting) {
      fitAgain = true;
      return;
    }
    fitting = fit().finally(() => {
      fitting = null;
      if (fitAgain) {
        fitAgain = false;
        fitWindow();
      }
    });
  }

  /** Resolves on the next window resize, or after a moment if the size didn't change. */
  function resized(): Promise<void> {
    return new Promise((resolve) => {
      const timer = setTimeout(resolve, 250);
      window.addEventListener(
        "resize",
        () => {
          clearTimeout(timer);
          resolve();
        },
        { once: true },
      );
    });
  }

  /**
   * How much taller the window is than its page. Tauri sets the size including
   * the title bar here, so this is learnt from the first resize and reused.
   */
  let titlebar = 0;

  async function fit() {
    if (!form) return;
    const wanted = Math.min(Math.ceil(form.scrollHeight), Math.floor(screen.availHeight * 0.85));
    if (Math.abs(window.innerHeight - wanted) <= 1) return;
    const win = getCurrentWindow();
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        await win.setSize(new LogicalSize(WIDTH, wanted + titlebar));
      } catch {
        return;
      }
      await resized();
      const missing = wanted - window.innerHeight;
      if (Math.abs(missing) <= 1) return;
      titlebar += missing;
    }
  }

  function onStorage(event: StorageEvent) {
    // A document window's "Open Settings" asks for the AI tab.
    if (event.key === TAB_KEY && event.newValue === "ai") tab = "ai";
  }

  onMount(() => {
    const unlisten = on("settings:changed", (next) => (settings = next));
    // The default app can also be changed in Finder while this window is open.
    window.addEventListener("focus", refreshDefaultViewer);
    window.addEventListener("storage", onStorage);
    const observer = new ResizeObserver(fitWindow);
    if (form) observer.observe(form);
    void (async () => {
      settings = await ipc.getSettings();
      providers = await ipc.aiProviders().catch(() => []);
      await refreshDefaultViewer();
      await tick();
      fitWindow();
      await ipc.showWindow();
      await refreshProvider();
    })();
    return () => {
      observer.disconnect();
      window.removeEventListener("focus", refreshDefaultViewer);
      window.removeEventListener("storage", onStorage);
      void unlisten.then((fn) => fn());
    };
  });
</script>

<form class="settings" bind:this={form} onsubmit={(e) => e.preventDefault()}>
  <div class="tabs">
    <div class="segmented" role="tablist" aria-label="Settings">
      <button
        type="button"
        role="tab"
        aria-selected={tab === "general"}
        class:selected={tab === "general"}
        onclick={() => selectTab("general")}>General</button
      >
      <button
        type="button"
        role="tab"
        aria-selected={tab === "ai"}
        class:selected={tab === "ai"}
        onclick={() => selectTab("ai")}>AI Summary</button
      >
    </div>
  </div>

  {#if tab === "ai"}
    {@render aiPane()}
  {:else}
    {@render generalPane()}
  {/if}
</form>

{#snippet generalPane()}
  {#if defaultViewer}
    <section>
      <div class="row">
        <span class="label">Markdown files</span>
        <div class="inline">
          {#if defaultViewer.isDefault}
            <span class="value">Open in MDViewer</span>
          {:else}
            <span class="value">Open in {defaultViewer.currentApp ?? "another app"}</span>
            <button
              type="button"
              class="push-button"
              disabled={!defaultViewer.canChange || makingDefault}
              title={defaultViewer.canChange ? "" : "Only available in the built app"}
              onclick={makeDefault}>Make Default</button
            >
          {/if}
        </div>
      </div>
      {#if defaultError}
        <p class="error">{defaultError}</p>
      {/if}
    </section>
  {/if}

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
{/snippet}

{#snippet aiPane()}
  <section>
    <div class="row">
      <label class="label" for="ai-provider">Provider</label>
      <select id="ai-provider" value={settings.ai.provider} onchange={(e) => void selectProvider(e.currentTarget.value)}>
        <optgroup label="Cloud">
          {#each providers.filter((p) => !p.local && p.id !== "custom") as p (p.id)}
            <option value={p.id}>{p.name}</option>
          {/each}
        </optgroup>
        <optgroup label="On This Mac">
          {#each providers.filter((p) => p.local) as p (p.id)}
            <option value={p.id}>{p.name}</option>
          {/each}
        </optgroup>
        <optgroup label="Other">
          <option value="custom">Custom…</option>
        </optgroup>
      </select>
    </div>

    {#if provider?.editableUrl}
      <div class="row">
        <label class="label" for="ai-url">Address</label>
        <input
          id="ai-url"
          class="field wide"
          type="url"
          spellcheck="false"
          placeholder={provider.baseUrl || "https://example.com/v1"}
          value={config.baseUrl ?? ""}
          onchange={(e) => {
            saveProvider({ baseUrl: e.currentTarget.value.trim() || null });
            void refreshProvider();
          }}
        />
      </div>
    {/if}
    {#if provider?.id === "custom"}
      <div class="row">
        <label class="label" for="ai-format">API format</label>
        <select
          id="ai-format"
          value={config.format ?? "openai"}
          onchange={(e) => {
            saveProvider({ format: e.currentTarget.value as "openai" | "anthropic" });
            void refreshProvider();
          }}
        >
          <option value="openai">OpenAI-compatible</option>
          <option value="anthropic">Anthropic Messages</option>
        </select>
      </div>
    {/if}

    {#if provider && (provider.needsKey || provider.id === "custom")}
      <div class="row">
        <label class="label" for="ai-key">API key</label>
        {#if hasKey}
          <div class="inline">
            <span class="value">Saved in your keychain</span>
            <button type="button" class="push-button" onclick={removeKey}>Remove</button>
          </div>
        {:else}
          <div class="inline">
            <input
              id="ai-key"
              class="field"
              type="password"
              autocomplete="off"
              spellcheck="false"
              placeholder={provider.needsKey ? "Paste your key" : "Optional"}
              bind:value={keyInput}
              onkeydown={(e) => e.key === "Enter" && keyInput.trim() && void saveKey()}
            />
            <button type="button" class="push-button" disabled={!keyInput.trim()} onclick={saveKey}>Save</button>
          </div>
        {/if}
      </div>
      {#if keyError}
        <p class="error">{keyError}</p>
      {:else if hasKey}
        <p class="hint">
          When MDViewer first uses the key, macOS asks to let it into your keychain: choose Always Allow, and it
          won’t ask again.
        </p>
      {:else if hasKey === false}
        <p class="hint">
          {#if provider.keyUrl}
            Get a key from <button type="button" class="link" onclick={() => void openUrl(provider!.keyUrl!)}
              >{new URL(provider.keyUrl).host}</button
            >.
          {/if}
          MDViewer keeps it in your macOS keychain, which macOS may ask you to allow, and only sends it to {provider.name}.
        </p>
      {/if}
    {/if}

    {#if provider}
      <div class="row">
        <label class="label" for="ai-model">Model</label>
        <div class="inline">
          <input
            id="ai-model"
            class="field wide"
            list="ai-models"
            spellcheck="false"
            placeholder={provider.defaultModel ?? (models.length ? "Choose a model" : "Model name")}
            value={config.model}
            onfocus={onModelFocus}
            onchange={(e) => chooseModel(e.currentTarget.value)}
          />
          <button
            type="button"
            class="icon-button"
            title="Reload Models"
            aria-label="Reload models"
            disabled={loadingModels}
            onclick={loadModels}
          >
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round">
              <path d="M12 7a5 5 0 1 1-1.5-3.55" /><path d="M11 1.5v2.5H8.5" />
            </svg>
          </button>
        </div>
        <datalist id="ai-models">
          {#each models as model (model.id)}
            <option value={model.id}>{model.name ?? ""}</option>
          {/each}
        </datalist>
      </div>
      {#if modelsError}
        <p class="error">{modelsError}</p>
      {:else if loadingModels}
        <p class="hint">Loading models…</p>
      {:else if modelHint(selectedModel ?? savedModel)}
        <p class="hint">{modelHint(selectedModel ?? savedModel)}</p>
      {:else if models.length}
        <p class="hint">{models.length} models available.</p>
      {/if}

      <div class="row">
        <label class="label" for="ai-context">Context window</label>
        <div class="inline">
          <input
            id="ai-context"
            class="field number"
            type="number"
            min="1024"
            step="1024"
            placeholder={(config.contextTokens ?? (provider.local ? 8192 : 128000)).toLocaleString("en-US").replaceAll(",", "")}
            value={selectedModel?.contextTokens ? "" : (config.contextTokens ?? "")}
            disabled={!!selectedModel?.contextTokens}
            onchange={(e) => saveProvider({ contextTokens: Number(e.currentTarget.value) || null })}
          />
          <span class="value">tokens</span>
        </div>
      </div>
      <p class="hint">
        Longer documents are read in parts of about {(
          Math.round(maxDocumentChars(config.contextTokens, provider.local) / 4 / 1000) * 1000
        ).toLocaleString()} tokens, then summarized from notes on each part.
      </p>
    {/if}
  </section>

  <section class="stack">
    <div class="row">
      <label class="label" for="ai-instructions">Instructions</label>
      <button
        type="button"
        class="push-button"
        disabled={settings.ai.instructions === null}
        onclick={() => {
          clearTimeout(instructionsTimer);
          instructionsDraft = null;
          saveAi({ instructions: null });
        }}>Reset to Default</button
      >
    </div>
    <textarea
      id="ai-instructions"
      rows="8"
      spellcheck="true"
      value={instructionsDraft ?? instructions}
      oninput={(e) => saveInstructions(e.currentTarget.value)}
    ></textarea>
    <p class="hint">
      What the summary should contain and how long it should be. MDViewer adds the document and links to its sections.
    </p>
  </section>

  <p class="footnote">
    {#if provider?.local}
      {provider.name} runs on this Mac, so documents never leave it.
    {:else}
      A document is only sent to {provider?.name ?? "the provider"} when you choose View ▸ Summarize, and MDViewer asks before the
      first time.
    {/if}
  </p>
{/snippet}

<style>
  /* Only scrolls if the screen is too short for the whole tab. */
  :global(body.settings-window #app) {
    overflow-y: auto;
  }

  .settings {
    padding: 14px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .tabs {
    display: flex;
    justify-content: center;
  }

  .stack {
    padding-bottom: 12px;
  }

  .field {
    font: inherit;
    color: inherit;
    width: 180px;
    padding: 3px 7px;
    border: 0.5px solid var(--control-border);
    border-radius: 5px;
    background: var(--control-bg);
    -webkit-user-select: text;
    user-select: text;
  }

  .field.wide {
    width: 260px;
  }

  .field.number {
    width: 110px;
  }

  textarea {
    display: block;
    width: 100%;
    font: inherit;
    font-size: 12px;
    line-height: 1.45;
    color: inherit;
    padding: 6px 8px;
    border: 0.5px solid var(--control-border);
    border-radius: 6px;
    background: var(--control-bg);
    resize: vertical;
    -webkit-user-select: text;
    user-select: text;
  }

  .hint {
    margin: -4px 0 10px;
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-secondary);
  }

  .stack .hint {
    margin: 6px 0 0;
  }

  .footnote {
    margin: -4px 4px 0;
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-secondary);
  }

  .link {
    border: 0;
    padding: 0;
    background: none;
    color: var(--accent);
    font-size: inherit;
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

  .error {
    margin: 0 0 10px;
    color: #d70015;
    font-size: 12px;
    white-space: pre-line;
    -webkit-user-select: text;
    user-select: text;
  }

  @media (prefers-color-scheme: dark) {
    .error {
      color: #ff6961;
    }
  }

  .push-button:disabled,
  .icon-button:disabled {
    opacity: 0.5;
  }
</style>
