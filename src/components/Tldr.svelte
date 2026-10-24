<script lang="ts">
  import { tick, untrack } from "svelte";

  import {
    EMPTY_PROVIDER_SETTINGS,
    ipc,
    type AiProvider,
    type Doc,
    type Settings,
    type SummaryStyle,
  } from "../lib/ipc";
  import { checkDiagram, renderDiagrams } from "../lib/markdown/mermaid";
  import { classifyLinks, sanitize } from "../lib/markdown/postprocess";
  import { renderMarkdown } from "../lib/markdown/render";
  import {
    diagramLinks,
    diagramSource,
    estimateTokens,
    hashText,
    linkDiagramNodes,
    loadSummary,
    maxRequestChars,
    saveSummary,
    summarize as writeSummary,
    type Complete,
  } from "../lib/tldr";
  import type { Heading } from "../lib/toc";

  interface Props {
    doc: Doc;
    headings: Heading[];
    settings: Settings;
    /** Link clicks and hovers go through the document's own handlers. */
    onclick: (event: MouseEvent) => void;
    onmousemove: (event: MouseEvent) => void;
    onmouseleave: () => void;
    onsavesettings: (settings: Settings) => void;
    ontoast: (message: string) => void;
  }

  let { doc, headings, settings, onclick, onmousemove, onmouseleave, onsavesettings, ontoast }: Props = $props();

  type Status = "hidden" | "confirm" | "running" | "done" | "error";

  let status = $state<Status>("hidden");
  let text = $state("");
  let notice = $state<string | null>(null);
  let error = $state<string | null>(null);
  /** Shown while a long document is read in parts. */
  let part = $state<{ index: number; count: number } | null>(null);
  /** Round of merging notes, when the notes on all parts don't fit one request. */
  let merging = $state<number | null>(null);
  /** Set when reading the API key takes a moment, i.e. macOS is asking for keychain access. */
  let keychainWait = $state(false);
  let keychainTimer: ReturnType<typeof setTimeout> | undefined;
  /** Style of the summary shown (the setting may change before it's rewritten). */
  let shownStyle = $state<SummaryStyle>("list");
  /** Set while a diagram Mermaid couldn't draw is being written again. */
  let repairing = $state(false);
  /** Why the diagram can't be drawn, when the repair didn't help either. */
  let invalid = $state<string | null>(null);
  let diagramBody = $state<HTMLElement>();
  let drawToken = 0;
  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
  let dark = $state(darkQuery.matches);
  /** Who wrote the summary, e.g. "OpenRouter · some/model". */
  let source = $state("");
  /** Hash of the document the shown summary was written from. */
  let summaryHash = $state<string | null>(null);
  let consent = $state<{ provider: string; model: string; tokens: number; cost: number | null } | null>(null);

  let run = 0;
  const requests = new Set<number>();
  let providersPromise: Promise<AiProvider[]> | null = null;

  const path = $derived(doc.path);
  const style = $derived<SummaryStyle>(settings.ai.style ?? "list");
  const hash = $derived(hashText(doc.content));
  const outdated = $derived(status === "done" && summaryHash !== null && summaryHash !== hash);
  // Diagrams are drawn once complete; while one streams in, the placeholder shows.
  const html = $derived(shownStyle === "list" ? renderSummary(text) : "");

  function summaryFragment(markdown: string): DocumentFragment {
    const fragment = sanitize(renderMarkdown(markdown));
    // A summary has no business loading images (they could be remote trackers).
    for (const img of fragment.querySelectorAll("img")) img.remove();
    classifyLinks(fragment, doc.dir);
    return fragment;
  }

  function renderSummary(markdown: string): string {
    if (!markdown) return "";
    const box = document.createElement("div");
    box.append(summaryFragment(markdown));
    return box.innerHTML;
  }

  async function drawDiagram(element: HTMLElement, markdown: string, theme: "light" | "dark") {
    const token = ++drawToken;
    const fragment = summaryFragment(markdown);
    await renderDiagrams(fragment, theme);
    linkDiagramNodes(fragment, diagramLinks(diagramSource(markdown) ?? ""), (id) => !!document.getElementById(id));
    if (token === drawToken) element.replaceChildren(fragment);
  }

  $effect(() => {
    if (shownStyle !== "diagram" || status !== "done" || !diagramBody) return;
    const element = diagramBody;
    const markdown = text;
    const theme = dark ? "dark" : "light";
    untrack(() => void drawDiagram(element, markdown, theme));
  });

  $effect(() => {
    const onChange = () => (dark = darkQuery.matches);
    darkQuery.addEventListener("change", onChange);
    return () => darkQuery.removeEventListener("change", onChange);
  });

  // A different file or style: stop, and show the saved summary if there is one.
  $effect(() => {
    const current = path;
    const currentStyle = style;
    untrack(() => {
      cancelRequests();
      const saved = loadSummary(current, currentStyle);
      error = null;
      part = null;
      merging = null;
      repairing = false;
      invalid = null;
      consent = null;
      shownStyle = currentStyle;
      if (saved) {
        text = saved.text;
        notice = saved.notice ?? null;
        source = `${saved.provider} · ${saved.model}`;
        summaryHash = saved.hash;
        status = "done";
      } else {
        text = "";
        notice = null;
        summaryHash = null;
        status = "hidden";
      }
    });
  });

  function providers(): Promise<AiProvider[]> {
    providersPromise ??= ipc.aiProviders().catch((e) => {
      providersPromise = null;
      throw e;
    });
    return providersPromise;
  }

  function stopKeychainWait() {
    clearTimeout(keychainTimer);
    keychainWait = false;
  }

  function cancelRequests() {
    run++;
    for (const id of requests) void ipc.aiCancel(id);
    requests.clear();
  }

  function fail(message: string) {
    error = message;
    status = "error";
  }

  /** View ▸ Summarize, the toolbar button, and the card's own buttons. */
  export async function summarize(confirmed = false) {
    if (status === "running") return;
    const all = await providers().catch(() => []);
    const ai = settings.ai;
    const provider = all.find((p) => p.id === ai.provider);
    if (!provider) return fail("Choose an AI provider in Settings.");
    const config = { ...EMPTY_PROVIDER_SETTINGS, ...ai.providers[provider.id] };
    const model = config.model.trim() || provider.defaultModel || "";
    if (!model) return fail(`Choose a model for ${provider.name} in Settings.`);

    // Ask once per cloud provider before a document leaves this Mac.
    if (!provider.local && !ai.allowed.includes(provider.id) && !confirmed) {
      const tokens = estimateTokens(doc.content);
      const cost =
        config.promptPrice !== null && config.completionPrice !== null
          ? tokens * config.promptPrice + 1_000 * config.completionPrice
          : null;
      consent = { provider: provider.name, model, tokens, cost };
      status = "confirm";
      return;
    }
    if (confirmed && !ai.allowed.includes(provider.id)) {
      onsavesettings({ ...settings, ai: { ...ai, allowed: [...ai.allowed, provider.id] } });
    }

    const token = ++run;
    const docPath = doc.path;
    const docHash = hash;
    const runStyle = style;
    shownStyle = runStyle;
    repairing = false;
    invalid = null;
    status = "running";
    text = "";
    notice = null;
    error = null;
    consent = null;
    part = null;
    merging = null;
    source = `${provider.name} · ${model}`;

    const complete: Complete = async (prompt, onText, onNotice) => {
      const id = Math.floor(Math.random() * 2 ** 31);
      requests.add(id);
      try {
        await ipc.aiComplete(id, prompt.system, prompt.prompt, (message) => {
          if (token !== run) return;
          if (message.type === "keychain") {
            clearTimeout(keychainTimer);
            keychainTimer = setTimeout(() => (keychainWait = true), 400);
            return;
          }
          stopKeychainWait();
          if (message.type === "text") onText(message.text);
          else onNotice(message.text);
        });
      } finally {
        stopKeychainWait();
        requests.delete(id);
      }
    };

    try {
      const result = await writeSummary({
        name: doc.name,
        source: doc.content,
        headings,
        instructions: ai.instructions,
        style: runStyle,
        check: runStyle === "diagram" ? checkSummaryDiagram : undefined,
        maxChars: maxRequestChars(config.contextTokens, provider.local),
        concurrency: provider.local ? 1 : 3,
        complete,
        onProgress: (progress) => {
          if (token !== run) return;
          part = progress.parts ? { index: progress.part ?? 0, count: progress.parts } : null;
          merging = progress.merging ?? null;
          repairing = progress.repairing ?? false;
          text = progress.text;
          notice = progress.notice ?? null;
        },
      });
      if (token !== run) return;
      if (!result.text.trim()) return fail(result.notice ?? "The model returned an empty summary.");
      summaryHash = docHash;
      invalid = result.invalid ?? null;
      status = "done";
      saveSummary(
        docPath,
        { hash: docHash, text: result.text, notice: result.notice, provider: provider.name, model, at: Date.now() },
        runStyle,
      );
    } catch (e) {
      if (token !== run) return;
      cancelRequests();
      fail(String(e));
    }
  }

  async function checkSummaryDiagram(summary: string): Promise<string | null> {
    const diagram = diagramSource(summary);
    return diagram ? checkDiagram(diagram) : "The reply had no Mermaid diagram.";
  }

  /** The List / Diagram switch: shows the saved summary in that style, or writes one. */
  async function setStyle(next: SummaryStyle) {
    if (next === style) return;
    onsavesettings({ ...settings, ai: { ...settings.ai, style: next } });
    await tick();
    if (status === "hidden") void summarize();
  }

  function openSettings() {
    // The Settings window opens on (or switches to) its AI tab.
    try {
      localStorage.setItem("mdviewer.settingsTab", "ai");
    } catch {
      // Opens on the last tab instead.
    }
    void ipc.showSettings();
  }

  function stop() {
    cancelRequests();
    part = null;
    merging = null;
    if (text.trim()) {
      notice = "Stopped before the summary was finished.";
      summaryHash = null;
      status = "done";
    } else {
      status = "hidden";
    }
  }

  function close() {
    cancelRequests();
    status = "hidden";
  }

  async function copy() {
    try {
      await ipc.copyText(text.trim());
      ontoast("Copied the summary");
    } catch (e) {
      ontoast(String(e));
    }
  }

  const meta = $derived.by(() => {
    if (status === "running") {
      if (keychainWait) return "Waiting for keychain access… (choose Always Allow if macOS asks)";
      if (repairing) return "Fixing the diagram…";
      if (merging && !text) return `Combining notes on the parts (round ${merging})…`;
      if (part && !text) return `Reading part ${part.index + 1} of ${part.count}…`;
      if (text) return `${shownStyle === "diagram" ? "Drawing" : "Writing"} · ${source}`;
      return `Reading · ${source}`;
    }
    if (status === "confirm") return "Before you summarize";
    if (status === "error") return "Couldn’t summarize";
    return source;
  });

  function formatCost(cost: number): string {
    if (cost === 0) return "free";
    if (cost < 0.01) return "less than $0.01";
    return `about $${cost.toFixed(2)}`;
  }
</script>

{#if status !== "hidden"}
  <section class="tldr" class:running={status === "running"} aria-label="Summary" aria-busy={status === "running"}>
    <header>
      <span class="badge">TL;DR</span>
      <span class="meta" title={source}>{meta}</span>
      <div class="styles" role="radiogroup" aria-label="Summary style">
        {#each [["list", "List"], ["diagram", "Diagram"]] as [value, label] (value)}
          <button
            type="button"
            role="radio"
            aria-checked={style === value}
            class:selected={style === value}
            disabled={status === "running"}
            onclick={() => void setStyle(value as SummaryStyle)}>{label}</button
          >
        {/each}
      </div>
      <div class="actions">
        {#if status === "running"}
          <button type="button" class="icon-button" title="Stop" aria-label="Stop" onclick={stop}>
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><rect x="1.5" y="1.5" width="9" height="9" rx="1.5" fill="currentColor" /></svg>
          </button>
        {:else if status === "done"}
          <button type="button" class="icon-button" title="Copy Summary" aria-label="Copy summary" onclick={copy}>
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.2">
              <rect x="4.5" y="4.5" width="8" height="8.5" rx="1.5" /><path d="M9.5 2.5V2a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v7a1 1 0 0 0 1 1h.5" />
            </svg>
          </button>
          <button type="button" class="icon-button" title="Summarize Again" aria-label="Summarize again" onclick={() => summarize()}>
            <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round">
              <path d="M12 7a5 5 0 1 1-1.5-3.55" /><path d="M11 1.5v2.5H8.5" />
            </svg>
          </button>
        {/if}
        <button type="button" class="icon-button" title="Close Summary" aria-label="Close summary" onclick={close}>
          <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" stroke="currentColor" stroke-width="1.4" stroke-linecap="round">
            <path d="M1.5 1.5l7 7M8.5 1.5l-7 7" />
          </svg>
        </button>
      </div>
    </header>

    {#if status === "confirm" && consent}
      <p class="message">
        Summarizing sends “{doc.name}” (about {consent.tokens.toLocaleString()} tokens) to {consent.provider}, to be
        read by {consent.model}{#if consent.cost !== null}, for {formatCost(consent.cost)}{/if}. Documents are only sent
        when you ask for a summary.
      </p>
      <div class="buttons">
        <button type="button" class="push-button" onclick={close}>Cancel</button>
        <button type="button" class="push-button primary" onclick={() => summarize(true)}>Summarize</button>
      </div>
    {:else if status === "error"}
      <p class="message error">{error}</p>
      <div class="buttons">
        <button type="button" class="push-button" onclick={openSettings}>Open Settings</button>
        <button type="button" class="push-button" onclick={() => summarize()}>Try Again</button>
      </div>
    {:else}
      {#if shownStyle === "diagram" && status === "done"}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="markdown body diagram" bind:this={diagramBody} {onclick} {onmousemove} {onmouseleave}></div>
      {:else if html}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="markdown body" {onclick} {onmousemove} {onmouseleave}>{@html html}</div>
      {:else}
        <div class="placeholder" aria-hidden="true"><span></span><span></span><span></span></div>
      {/if}
      {#if notice}
        <p class="notice">{notice}</p>
      {/if}
      {#if invalid && status === "done"}
        <p class="notice">
          The model’s diagram couldn’t be drawn, even after a second try.
          <button type="button" class="link" onclick={() => void setStyle("list")}>Show as a List</button>
        </p>
      {/if}
      {#if outdated}
        <div class="outdated">
          <span>The document changed since this summary.</span>
          <button type="button" class="push-button" onclick={() => summarize()}>Update</button>
        </div>
      {/if}
    {/if}
  </section>
{/if}

<style>
  /* The whole card (text, controls, diagram) zooms with the document. */
  .tldr {
    zoom: var(--zoom, 1);
    font-size: 13px;
    margin: 20px 0 4px;
    padding: 10px 14px 12px;
    border: 1px solid color-mix(in srgb, var(--accent) 22%, var(--hairline));
    border-radius: 10px;
    background: color-mix(in srgb, var(--accent) 4%, var(--content-bg));
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 24px;
  }

  .badge {
    flex: none;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 1px 6px;
    border-radius: 4px;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .meta {
    flex: 1;
    min-width: 0;
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .styles {
    flex: none;
    display: inline-flex;
    padding: 1px;
    border-radius: 6px;
    background: var(--hover);
  }

  .styles button {
    border: 0;
    background: transparent;
    padding: 1px 8px;
    border-radius: 5px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .styles button.selected {
    background: var(--control-bg);
    color: var(--text);
    box-shadow: 0 0.5px 1.5px rgba(0, 0, 0, 0.2);
  }

  .styles button:disabled {
    opacity: 0.6;
  }

  .body.diagram :global(.mermaid-diagram) {
    margin: 4px 0 8px;
  }

  .body.diagram :global(a.diagram-link) {
    cursor: pointer;
  }

  .body.diagram :global(a.diagram-link:hover) {
    opacity: 0.75;
  }

  .link {
    border: 0;
    padding: 0;
    background: none;
    color: var(--accent);
    font-size: inherit;
  }

  .actions {
    display: flex;
    gap: 2px;
    margin-right: -6px;
  }

  .body {
    --md-font-size: 14px;
    margin-top: 6px;
  }

  .body :global(ul),
  .body :global(ol) {
    padding-left: 1.5em;
  }

  .message {
    margin: 8px 0 10px;
    line-height: 1.5;
  }

  .message.error {
    white-space: pre-line;
    color: #d70015;
  }

  @media (prefers-color-scheme: dark) {
    .message.error {
      color: #ff6961;
    }
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .notice {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .outdated {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 10px;
    padding-top: 8px;
    border-top: 1px solid var(--hairline);
    font-size: 12px;
    color: var(--text-secondary);
  }

  .placeholder {
    display: grid;
    gap: 8px;
    margin: 10px 0 4px;
  }

  .placeholder span {
    height: 10px;
    border-radius: 5px;
    background: linear-gradient(90deg, var(--hover) 0%, var(--selected) 50%, var(--hover) 100%);
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
  }

  .placeholder span:nth-child(2) {
    width: 85%;
  }

  .placeholder span:nth-child(3) {
    width: 60%;
  }

  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .placeholder span {
      animation: none;
    }
  }

  @media print {
    .tldr {
      display: none;
    }
  }
</style>
