<script lang="ts">
  import { onMount, tick } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Menu } from "@tauri-apps/api/menu";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { Idiomorph } from "idiomorph";

  import "katex/dist/katex.min.css";
  import "markdown-it-github-alerts/styles/github-base.css";
  import "markdown-it-github-alerts/styles/github-colors-light.css";
  import "markdown-it-github-alerts/styles/github-colors-dark-media.css";
  import "./styles/markdown.css";

  import EmptyState from "./components/EmptyState.svelte";
  import FindBar from "./components/FindBar.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import { Finder } from "./lib/find";
  import { DEFAULT_SETTINGS, ipc, on, type Doc, type MenuAction, type Settings } from "./lib/ipc";
  import { prepareHighlighting } from "./lib/markdown/highlight";
  import { renderDiagrams } from "./lib/markdown/mermaid";
  import { buildFragment } from "./lib/markdown/postprocess";
  import { renderMarkdown } from "./lib/markdown/render";
  import { basename, isMarkdownPath } from "./lib/paths";
  import { activeHeading, extractHeadings, type Heading } from "./lib/toc";

  const ZOOM_STEPS = [0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3];
  const CONTENT_WIDTHS = { narrow: "640px", normal: "780px", wide: "1000px", full: "none" };

  interface HistoryEntry {
    path: string;
    scrollTop: number;
  }

  let doc = $state<Doc | null>(null);
  let settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  let recent = $state<string[]>([]);
  let loadError = $state<string | null>(null);
  let missing = $state(false);
  let toast = $state<string | null>(null);
  let headings = $state<Heading[]>([]);
  let active = $state<string | null>(null);
  let scrolled = $state(false);
  let dropping = $state(false);
  let findOpen = $state(false);
  let findQuery = $state("");
  let findPosition = $state(0);
  let findCount = $state(0);
  let backStack: HistoryEntry[] = [];
  let forwardStack: HistoryEntry[] = [];

  let scroller: HTMLElement;
  let article: HTMLElement;
  let findBar = $state<ReturnType<typeof FindBar>>();
  let finder: Finder;
  let renderToken = 0;
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  let scrollFrame = 0;

  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
  const showSidebar = $derived(settings.showSidebar && doc !== null);

  // ── Loading and rendering ────────────────────────────────────────────────

  async function load(path: string, options: { push?: boolean; scrollTop?: number; hash?: string } = {}) {
    try {
      const next = await ipc.openDocument(path);
      if (options.push && doc) {
        backStack.push({ path: doc.path, scrollTop: scroller.scrollTop });
        forwardStack = [];
      }
      doc = next;
      missing = false;
      loadError = null;
      await tick();
      await render(next.content, "replace", options.scrollTop ?? 0, options.hash ?? "");
      scroller.focus({ preventScroll: true });
    } catch (error) {
      if (doc) showToast(String(error));
      else loadError = String(error);
    }
  }

  /**
   * "replace" shows a new document; "update" patches the current one in place
   * (live reload), which keeps the scroll position and avoids flicker.
   */
  async function render(source: string, mode: "replace" | "update", scrollTop = 0, hash = "") {
    const token = ++renderToken;
    await prepareHighlighting(source);
    if (token !== renderToken || !doc) return;
    const fragment = buildFragment(renderMarkdown(source), {
      baseDir: doc.dir,
      assetUrl: (path) => convertFileSrc(path),
      blockRemoteImages: settings.blockRemoteImages,
    });
    await renderDiagrams(fragment, darkQuery.matches ? "dark" : "light");
    if (token !== renderToken) return;

    if (mode === "replace") {
      article.replaceChildren(fragment);
      scroller.scrollTop = scrollTop;
      if (hash) scrollToId(hash, false);
    } else {
      const top = scroller.scrollTop;
      const atBottom = top > 0 && top + scroller.clientHeight >= scroller.scrollHeight - 4;
      Idiomorph.morph(article, [...fragment.childNodes], { morphStyle: "innerHTML" });
      scroller.scrollTop = atBottom ? scroller.scrollHeight : top;
    }

    headings = extractHeadings(article);
    updateActiveHeading();
    if (findOpen) {
      finder.refresh();
      syncFind();
    }
  }

  async function reload() {
    if (!doc) return;
    try {
      const fresh = await ipc.reloadDocument();
      doc = fresh;
      missing = false;
      await render(fresh.content, "update");
    } catch (error) {
      showToast(String(error));
    }
  }

  async function goBack() {
    const entry = backStack.pop();
    if (!entry || !doc) return;
    forwardStack.push({ path: doc.path, scrollTop: scroller.scrollTop });
    await load(entry.path, { scrollTop: entry.scrollTop });
  }

  async function goForward() {
    const entry = forwardStack.pop();
    if (!entry || !doc) return;
    backStack.push({ path: doc.path, scrollTop: scroller.scrollTop });
    await load(entry.path, { scrollTop: entry.scrollTop });
  }

  // ── Links, code blocks, context menu ─────────────────────────────────────

  function scrollToId(id: string, smooth = true) {
    let target: string;
    try {
      target = decodeURIComponent(id);
    } catch {
      target = id;
    }
    const element =
      document.getElementById(target) ?? article.querySelector(`[name="${CSS.escape(target)}"]`);
    element?.scrollIntoView({ behavior: smooth ? "smooth" : "auto", block: "start" });
  }

  async function followLink(link: HTMLAnchorElement, newWindow: boolean) {
    const { link: kind, path, hash } = link.dataset;
    const href = link.getAttribute("href") ?? "";
    try {
      if (kind === "anchor") {
        scrollToId(hash ?? "");
      } else if (kind === "external") {
        await openUrl(href);
      } else if (kind === "file" && path) {
        if (path === doc?.path) {
          if (hash) scrollToId(hash);
        } else if (isMarkdownPath(path)) {
          if (newWindow) await ipc.openPaths([path]);
          else await load(path, { push: true, hash });
        } else {
          await ipc.openExternalFile(path);
        }
      } else if (kind === "file" && hash) {
        scrollToId(hash);
      } else {
        showToast(`Can’t open “${href}”.`);
      }
    } catch (error) {
      showToast(String(error));
    }
  }

  async function copyCode(button: HTMLButtonElement) {
    const code = button.parentElement?.querySelector("pre")?.innerText ?? "";
    await ipc.copyText(code.replace(/\n$/, ""));
    button.textContent = "Copied";
    button.classList.add("copied");
    setTimeout(() => {
      button.textContent = "Copy";
      button.classList.remove("copied");
    }, 1200);
  }

  function onArticleClick(event: MouseEvent) {
    const target = event.target as Element;
    const copy = target.closest<HTMLButtonElement>(".copy-button");
    if (copy) {
      void copyCode(copy);
      return;
    }
    const link = target.closest<HTMLAnchorElement>("a");
    if (!link) return;
    // Never let the web view itself navigate.
    event.preventDefault();
    if (link.hasAttribute("href")) void followLink(link, event.metaKey);
  }

  async function onContextMenu(event: MouseEvent) {
    const target = event.target as Element;
    const link = target.closest<HTMLAnchorElement>("a[href]");
    if (link && article.contains(link)) {
      event.preventDefault();
      const opensMarkdown = link.dataset.link === "file" && isMarkdownPath(link.dataset.path ?? "");
      const menu = await Menu.new({
        items: [
          { id: "open-link", text: "Open Link", action: () => void followLink(link, false) },
          {
            id: "open-link-window",
            text: "Open Link in New Window",
            enabled: opensMarkdown,
            action: () => void followLink(link, true),
          },
          {
            id: "copy-link",
            text: "Copy Link",
            action: () => void ipc.copyText(link.dataset.path || link.getAttribute("href") || ""),
          },
        ],
      });
      await menu.popup();
      return;
    }
    // Keep WebKit's native menu (Copy, Look Up, Translate…) for selected text only;
    // its other entries (Reload, Open Image…) don't belong in a document viewer.
    const hasSelection = !!window.getSelection()?.toString();
    if (!article.contains(target) || !hasSelection || target.closest("img")) {
      event.preventDefault();
    }
  }

  // ── Find, sidebar, zoom, settings ────────────────────────────────────────

  async function openFind() {
    findOpen = true;
    await tick();
    findBar?.focus();
    if (findQuery) {
      finder.search(findQuery);
      syncFind();
    }
  }

  function closeFind() {
    findOpen = false;
    finder.clear();
    syncFind();
    scroller.focus({ preventScroll: true });
  }

  function syncFind() {
    findPosition = finder.position;
    findCount = finder.count;
  }

  function findStep(direction: 1 | -1) {
    if (!findOpen) {
      void openFind();
      return;
    }
    if (direction === 1) finder.next();
    else finder.previous();
    syncFind();
  }

  function selectHeading(id: string) {
    active = id;
    scrollToId(id);
  }

  function updateActiveHeading() {
    active = activeHeading(article, headings, scroller.getBoundingClientRect().top);
  }

  function onScroll() {
    scrolled = scroller.scrollTop > 0;
    cancelAnimationFrame(scrollFrame);
    scrollFrame = requestAnimationFrame(updateActiveHeading);
  }

  function saveSettings(next: Settings) {
    settings = next;
    void ipc.setSettings($state.snapshot(next));
  }

  async function setZoom(zoom: number) {
    // Keep the reading position while the text reflows.
    const ratio = scroller.scrollHeight ? scroller.scrollTop / scroller.scrollHeight : 0;
    saveSettings({ ...settings, zoom });
    await tick();
    scroller.scrollTop = ratio * scroller.scrollHeight;
  }

  function stepZoom(direction: 1 | -1) {
    const current = settings.zoom;
    const next =
      direction === 1
        ? ZOOM_STEPS.find((step) => step > current + 0.001)
        : ZOOM_STEPS.findLast((step) => step < current - 0.001);
    if (next) void setZoom(next);
  }

  function onMenuAction(action: MenuAction) {
    switch (action) {
      case "find":
        void openFind();
        break;
      case "find-next":
        findStep(1);
        break;
      case "find-prev":
        findStep(-1);
        break;
      case "reload":
        void reload();
        break;
      case "back":
        void goBack();
        break;
      case "forward":
        void goForward();
        break;
      case "toggle-sidebar":
        saveSettings({ ...settings, showSidebar: !settings.showSidebar });
        break;
      case "zoom-reset":
        void setZoom(1);
        break;
      case "zoom-in":
        stepZoom(1);
        break;
      case "zoom-out":
        stepZoom(-1);
        break;
    }
  }

  function onSettingsChanged(next: Settings) {
    const rerender = next.blockRemoteImages !== settings.blockRemoteImages;
    settings = next;
    if (rerender && doc) void render(doc.content, "update");
  }

  function showToast(message: string) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 3500);
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && findOpen) {
      event.preventDefault();
      closeFind();
    }
  }

  // ── Startup ──────────────────────────────────────────────────────────────

  async function start() {
    settings = await ipc.getSettings().catch(() => settings);
    const pending = await ipc.takePendingOpen().catch(() => null);
    if (pending) await load(pending);
    else recent = await ipc.getRecent().catch(() => []);
    await ipc.showWindow();
  }

  onMount(() => {
    finder = new Finder(article, scroller);
    const onThemeChange = () => {
      if (doc) void render(doc.content, "update");
    };
    darkQuery.addEventListener("change", onThemeChange);

    const unlisteners = [
      on("doc:open", async () => {
        const path = await ipc.takePendingOpen();
        if (path) await load(path, { push: true });
      }),
      on("doc:changed", (changed) => {
        if (!doc || changed.path !== doc.path) return;
        missing = false;
        if (changed.content === doc.content) return;
        doc = changed;
        void render(changed.content, "update");
      }),
      on("doc:missing", () => (missing = true)),
      on("menu:action", onMenuAction),
      on("settings:changed", onSettingsChanged),
      on("recent:changed", async () => {
        if (!doc) recent = await ipc.getRecent();
      }),
      getCurrentWebview().onDragDropEvent(({ payload }) => {
        if (payload.type === "enter") {
          dropping = payload.paths.length > 0;
        } else if (payload.type === "leave") {
          dropping = false;
        } else if (payload.type === "drop") {
          dropping = false;
          const paths = payload.paths.filter(isMarkdownPath);
          if (paths.length) void ipc.openPaths(paths);
          else if (payload.paths.length) showToast("MDViewer opens Markdown files (.md, .markdown, …).");
        }
      }),
    ];

    void start();

    return () => {
      darkQuery.removeEventListener("change", onThemeChange);
      for (const unlisten of unlisteners) void unlisten.then((fn) => fn());
    };
  });
</script>

<svelte:window oncontextmenu={onContextMenu} onkeydown={onKeydown} />

<div
  class="window"
  class:with-sidebar={showSidebar}
  style:--zoom={settings.zoom}
  style:--content-width={CONTENT_WIDTHS[settings.contentWidth]}
>
  {#if showSidebar}
    <Sidebar {headings} {active} onselect={selectHeading} />
  {/if}

  <main class="pane">
    <header class="toolbar" class:scrolled data-tauri-drag-region>
      {#if doc}
        <div class="title" data-tauri-drag-region>
          <span class="name" data-tauri-drag-region>{doc.name}</span>
          <span class="folder" data-tauri-drag-region>{basename(doc.dir)}</span>
        </div>
      {/if}
    </header>

    {#if findOpen}
      <FindBar
        bind:this={findBar}
        bind:query={findQuery}
        position={findPosition}
        count={findCount}
        onsearch={(query) => {
          finder.search(query);
          syncFind();
        }}
        onnext={() => findStep(1)}
        onprevious={() => findStep(-1)}
        onclose={closeFind}
      />
    {/if}
    {#if missing}
      <div class="banner" role="status">This file was moved or deleted. Showing the last version.</div>
    {/if}
    {#if doc?.lossy}
      <div class="banner" role="status">This file isn’t valid UTF-8, so some characters may look wrong.</div>
    {/if}

    <div class="scroller" bind:this={scroller} hidden={!doc} tabindex="-1" onscroll={onScroll}>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <article class="markdown" bind:this={article} onclick={onArticleClick}></article>
    </div>

    {#if !doc}
      <EmptyState
        {recent}
        error={loadError}
        onopen={() => void ipc.showOpenDialog()}
        onopenrecent={(path) => void load(path)}
      />
    {/if}
  </main>

  {#if doc}
    <button
      type="button"
      class="icon-button sidebar-toggle"
      onclick={() => onMenuAction("toggle-sidebar")}
      title={showSidebar ? "Hide Sidebar" : "Show Sidebar"}
      aria-label="Toggle sidebar"
    >
      <svg width="18" height="14" viewBox="0 0 18 14" aria-hidden="true">
        <rect x="0.75" y="0.75" width="16.5" height="12.5" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.3" />
        <path d="M6.25 1v12" stroke="currentColor" stroke-width="1.3" />
        <path d="M2.5 4h2M2.5 6.25h2M2.5 8.5h2" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" />
      </svg>
    </button>
  {/if}

  {#if dropping}
    <div class="drop-overlay" aria-hidden="true"></div>
  {/if}
  {#if toast}
    <div class="toast" role="status">{toast}</div>
  {/if}
</div>

<style>
  .window {
    position: relative;
    height: 100%;
    display: flex;
  }

  .pane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--content-bg);
  }

  .with-sidebar .pane {
    border-left: 1px solid var(--hairline);
  }

  .toolbar {
    height: var(--toolbar-height);
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 120px;
    border-bottom: 1px solid transparent;
    transition: border-color 0.15s;
  }

  .with-sidebar .toolbar {
    padding-left: 24px;
  }

  .toolbar.scrolled {
    border-bottom-color: var(--hairline);
  }

  .title {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 0;
    line-height: 1.25;
  }

  .name {
    font-weight: 600;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .folder {
    font-size: 11px;
    color: var(--text-secondary);
  }

  /* Sits right of the traffic lights (see TRAFFIC_LIGHTS in platform.rs). */
  .sidebar-toggle {
    position: absolute;
    top: 14px;
    left: 90px;
  }

  .banner {
    flex: none;
    padding: 6px 16px;
    font-size: 12px;
    background: var(--banner-bg);
    color: var(--banner-text);
    border-bottom: 1px solid var(--hairline);
  }

  .scroller {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    outline: none;
  }

  .scroller[hidden] {
    display: none;
  }

  .markdown {
    max-width: var(--content-width);
    margin: 0 auto;
    padding: 24px 48px 96px;
  }

  .drop-overlay {
    position: absolute;
    inset: 8px;
    border: 2px dashed var(--accent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    pointer-events: none;
  }

  .toast {
    position: absolute;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    max-width: min(520px, calc(100% - 48px));
    padding: 8px 14px;
    border-radius: 10px;
    background: var(--control-bg);
    box-shadow: var(--shadow);
    font-size: 12px;
  }

  @media print {
    .toolbar,
    .banner,
    .sidebar-toggle,
    .toast {
      display: none !important;
    }

    .window,
    .pane,
    .scroller {
      display: block;
      height: auto;
      overflow: visible;
      background: white;
      border: 0;
    }

    .markdown {
      max-width: none;
      padding: 0;
    }

    :global(html),
    :global(body) {
      height: auto;
      overflow: visible;
    }

    :global(.sidebar) {
      display: none !important;
    }
  }
</style>
