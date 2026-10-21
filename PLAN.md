# MDViewer — Implementation Plan (Tauri 2)

A small, fast, native-feeling macOS app for **viewing** Markdown files. No editing.

## Status (2026-10-21)

Phases 0–7 are implemented; see [README.md](README.md) for how to run and build it.

- **Verified in the real app:** rendering of all fixtures (code, Mermaid, KaTeX, local images, alerts) in light and dark mode under the release CSP; live reload with atomic saves; empty window with recent files; opening via Launch Services (cold start, same file twice focuses the existing window, second file gets its own window); Quit still works although closing the last window keeps the app running; release bundle 8.4 MB, DMG 5.8 MB.
- **Verified in the browser with mocked IPC:** find (count, next/previous, wrap-around), links to other documents and back with scroll restore, zoom, sidebar toggle, live updates that keep scroll and Mermaid nodes, Settings layout.
- **Implemented but not exercised by hand yet:** menu commands (Open dialog, Open Recent, Reveal in Finder, Open in Editor, Copy File Path, Print), native tab merging, drag and drop onto a window, the link context menu, Settings changes applied across windows.
- **Not done:** signing and notarisation (needs the Developer ID certificate, §12), the universal build (needs `rustup target add x86_64-apple-darwin`), and the optional auto-updater (§4, Phase 7).
- **Deviations from the plan:** Tauri pinned to 2.11 with matching plugins (Rust 1.89 on this machine); traffic lights positioned through AppKit in `platform.rs` because Tauri's option doesn't take effect; windows are shown by a `show_window` command once the first render is done.

## 1. Goals and non-goals

**Goals**
- Opens `.md` files from Finder, the Dock, drag and drop, the Open dialog and the command line.
- Renders GitHub-flavoured Markdown with good typography, syntax highlighting, Mermaid diagrams, math and local images.
- Re-renders live when the file changes on disk (edited in another editor), keeping the scroll position.
- Feels like a Mac app: translucent sidebar, inset traffic lights, native menus and shortcuts, native window tabs, follows light/dark mode, remembers window size and position.
- Small (< 15 MB), launches in under 300 ms, edit-to-reload loop of about a second during development.

**Non-goals (v1)**
- Editing Markdown.
- Windows/Linux builds (Tauri allows them later, but we won't test or polish them).
- Quick Look preview extension (needs a native Swift app extension; see §11).
- Browsing a whole folder as a workspace (possible v2; the sidebar is designed so it can be added).

## 2. Architecture

```
┌──────────────────────────── macOS app bundle (MDViewer.app) ────────────────────────────┐
│                                                                                         │
│  Rust core (src-tauri)                          Web UI (src, runs in WKWebView)         │
│  ─────────────────────                          ─────────────────────────────           │
│  • App lifecycle, RunEvent::Opened (Finder)     • Svelte 5 + TypeScript, built by Vite  │
│  • Native menu bar + Open Recent                • Markdown → HTML (markdown-it)         │
│  • Window manager (one window per doc,          • Sanitise (DOMPurify)                  │
│    native tabs, path → window map)              • Highlight (Shiki), Mermaid, KaTeX     │
│  • File reading + file watcher (notify)         • Sidebar: table of contents            │
│  • Asset scope for local images                 • Find bar, zoom, link handling         │
│  • Persistent settings (store plugin)           • Light/dark theme via CSS              │
│                                                                                         │
│            ◄──── commands (invoke) ────   ──── events (emit) ────►                      │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

**Key decisions**

| Decision | Choice | Why |
|---|---|---|
| Frontend framework | **Svelte 5 + TS** | Tiny runtime, little boilerplate, enough for sidebar/find/settings state. React is fine too if the team prefers it. |
| Where Markdown is rendered | **In the web view (JS)** | Best plugin ecosystem (alerts, footnotes, Mermaid, math), and changes hot-reload instantly. Rust (`comrak`) is a fallback if performance on huge files becomes a problem. |
| Markdown parser | **markdown-it** | Fast, CommonMark-compliant, many plugins, synchronous (simple pipeline). |
| Highlighting | **Shiki** (fine-grained bundle, languages loaded on demand) | Same grammars and themes as VS Code; light and dark themes from one render. |
| File I/O | **Our own Rust commands**, not the `fs` plugin | A viewer must read any file the user opens; scoping the `fs` plugin for files arriving from Finder is awkward. Keep the command surface tiny instead (read one text file, watch it). |
| Documents and windows | **One window per document**, with `tabbingIdentifier` | macOS then applies the user's "Prefer tabs" setting and gives us Window ▸ Merge All Windows, tab bar, etc. for free. |
| Package manager | **pnpm** | Already installed; fast. |

> API names below are from Tauri 2. Pin versions at scaffold time and check each config key against the docs for that version; a couple (e.g. `trafficLightPosition`, `tabbingIdentifier`, file-association `contentTypes`) arrived in 2.x minor releases.

## 3. Project layout

```
mdviewer/
├── package.json, pnpm-lock.yaml, vite.config.ts, tsconfig.json, index.html
├── src/                              # Web UI
│   ├── main.ts
│   ├── App.svelte                    # layout: sidebar | content, empty state
│   ├── lib/
│   │   ├── ipc.ts                    # typed wrappers around invoke/listen
│   │   ├── markdown/
│   │   │   ├── render.ts             # markdown-it instance + plugin setup
│   │   │   ├── sanitize.ts           # DOMPurify config
│   │   │   ├── highlight.ts          # Shiki, lazy languages
│   │   │   ├── mermaid.ts            # lazy-loaded, renders after mount
│   │   │   ├── links.ts              # rewrite image src, classify links
│   │   │   └── toc.ts                # heading extraction
│   │   ├── find.ts                   # CSS Custom Highlight API find-in-page
│   │   └── settings.ts               # zoom, theme, sidebar visibility
│   ├── components/
│   │   ├── Sidebar.svelte, Toc.svelte, FindBar.svelte,
│   │   ├── EmptyState.svelte, Document.svelte
│   └── styles/
│       ├── app.css                   # chrome: system font, no selection, cursors
│       └── markdown.css              # content typography (GitHub-like, Mac fonts)
├── src-tauri/
│   ├── Cargo.toml, build.rs, tauri.conf.json, Info.plist (merged into bundle)
│   ├── capabilities/default.json     # minimal permissions
│   ├── icons/
│   └── src/
│       ├── main.rs                   # calls lib::run()
│       ├── lib.rs                    # builder, plugins, RunEvent handling
│       ├── commands.rs               # open_document, take_pending_open, reveal, …
│       ├── documents.rs              # window ↔ path map, open/focus logic
│       ├── watcher.rs                # notify + debouncer, emits doc:changed
│       ├── menu.rs                   # menu bar, Open Recent, accelerators
│       └── recent.rs                 # recent files (store plugin)
└── tests/
    ├── fixtures/*.md                 # GFM, tables, code, mermaid, math, images, huge.md
    └── render.test.ts                # Vitest snapshot tests
```

## 4. Phases

Each phase ends with something runnable. Rough sizes assume one developer working with an AI assistant.

### Phase 0 — Scaffold and tooling (½ day)

1. `pnpm create tauri-app` → template **svelte-ts**, identifier e.g. `com.mmmikael.mdviewer`, product name `MDViewer`.
2. `git init`, `.gitignore` (target/, node_modules/, dist/).
3. Add Rust plugins: `tauri-plugin-dialog`, `tauri-plugin-opener`, `tauri-plugin-store`, `tauri-plugin-window-state`. Rust crates: `notify`, `notify-debouncer-full`, `serde`, `anyhow`/`thiserror`.
4. Add JS deps: `@tauri-apps/api`, plugin JS packages, `markdown-it` + plugins, `dompurify`, `shiki`, `mermaid`, `katex`, `github-markdown-css` (as a starting point).
5. Scripts: `pnpm tauri dev` (hot reload for UI, Rust auto-rebuild), `pnpm test` (Vitest), `pnpm lint` (eslint + prettier + `cargo clippy` + `cargo fmt --check`).
6. Toolchain note: Node 23 is installed and works, but it is end-of-life; pin Node 22 or 24 LTS in `.nvmrc` / `package.json#engines`.

**Done when:** `pnpm tauri dev` opens a window and editing a `.svelte` file hot-reloads.

### Phase 1 — Native window shell (1 day)

`tauri.conf.json`:
- `app.macOSPrivateApi: true` (required for a transparent window).
- Main window template: `titleBarStyle: "Overlay"`, `hiddenTitle: true`, `trafficLightPosition` aligned with the sidebar toolbar, `transparent: true`, `windowEffects: { effects: ["sidebar"], state: "followsWindowActiveState" }`, `minWidth/minHeight`, `tabbingIdentifier: "mdviewer-doc"`.
- Windows are created from Rust (not listed statically) so each document gets its own window; keep one template function in `documents.rs`.

UI shell:
- Two panes: translucent sidebar (body transparent so the vibrancy shows) and an opaque content pane using the system window background.
- A thin top strip with `data-tauri-drag-region` so the window can be dragged; check double-click zooms the window per the user's system setting.
- Chrome CSS for a native feel: `font: -apple-system-body` / `system-ui`, `user-select: none` and `cursor: default` everywhere except the document, `overscroll-behavior: none` on chrome, `color-scheme: light dark`, `AccentColor` for selection and focus rings, no web-style hover underline on buttons.
- Disable the default WebKit context menu outside the document; inside it, show a small native menu (Copy, Copy Link, Select All) via the Tauri menu `popup` API.
- `tauri-plugin-window-state` to remember size and position.

**Done when:** the window looks like a Finder/Notes-style window in both light and dark mode, with no visible "web page" tells.

### Phase 2 — Open and render a document (1–1½ days)

Rust:
- `open_document(path) -> Document { path, name, content, modified }` reads UTF-8 (lossy fallback with a warning banner for other encodings), refuses files over e.g. 20 MB with a friendly error.
- `documents.rs` keeps `HashMap<PathBuf, WindowLabel>`. Opening a path that is already open focuses its window instead of creating a new one. Paths are canonicalised first.
- Open paths:
  - **File ▸ Open… (⌘O)** → dialog plugin, filter `md, markdown, mdown, mkd, mdx, txt`, multi-select.
  - **Finder / Dock / `open -a MDViewer x.md`** → `RunEvent::Opened { urls }` in `lib.rs`.
  - **Drag and drop** onto a window → `onDragDropEvent` in the UI, which calls `open_document`.
- **Cold-start race:** when the app is launched by double-clicking a file, `Opened` can fire before any web view is ready. Store the paths in a `PendingOpens` state; each new window calls `take_pending_open()` once it has mounted. Test this case explicitly.
- When the app launches with no file: show one empty window with a drop zone and "Open…" button (like Preview's behaviour, but friendlier).

Web UI:
- `render.ts`: markdown-it with `html: true`, `linkify: true`, `typographer: true`; plugins for anchors (GitHub-style slugs), footnotes, task lists (read-only checkboxes), GitHub alerts (`> [!NOTE]`), front matter (hidden, or shown as a small table).
- Run output through DOMPurify (allow `details/summary`, `kbd`, `sup/sub`, `img`, task-list inputs as disabled; forbid scripts, iframes, event handlers, `style` attributes optional).
- `markdown.css`: start from github-markdown-css, switch to Mac fonts (`-apple-system`, `ui-monospace` / SF Mono), comfortable measure (~75ch), light and dark variants.
- Window title = file name; set the represented file so ⌘-clicking the title shows the path menu if Tauri exposes it (otherwise skip).

**Done when:** double-clicking a `.md` file in Finder (after a dev build is registered, see Phase 5) or using ⌘O renders it correctly, and opening the same file twice focuses the existing window.

### Phase 3 — Rich content (1½ days)

- **Code highlighting:** Shiki via `shiki/core` with the JavaScript regex engine, load grammars on demand per fenced language, use dual themes (`github-light` / `github-dark`) so theme switches need no re-render. Add a hover "Copy" button on code blocks.
- **Mermaid:** detect ` ```mermaid ` blocks; dynamically `import('mermaid')` only when present; render after mount; re-render on theme change; show the source and the error message if a diagram fails.
- **Math:** KaTeX through a markdown-it math plugin (`$…$`, `$$…$$`), CSS loaded with the document.
- **Local images:** resolve relative `src` against the document's folder, convert with `convertFileSrc`, and when opening a document call `app.asset_protocol_scope().allow_directory(doc_dir, true)` from Rust so only folders of opened documents are readable. Enable `app.security.assetProtocol`.
- **Remote images:** allowed by default (`img-src https:` in the CSP); setting to block them later if wanted.
- **Links** (`links.ts`, one click handler on the content):
  - `#heading` → smooth scroll within the page.
  - Relative `.md` link → open in the same window with a simple back/forward history (⌘[ / ⌘]); ⌘-click opens a new window/tab.
  - `http(s):`, `mailto:` → `opener` plugin (default browser/mail client).
  - Other local files → open with the default app (`opener`), or reveal in Finder if it can't be opened.
- **CSP** in `tauri.conf.json`: `default-src 'self'; img-src 'self' asset: http://asset.localhost https: data:; style-src 'self' 'unsafe-inline'; script-src 'self'` (Mermaid/KaTeX inline styles need `unsafe-inline` for styles only).

**Done when:** every fixture in `tests/fixtures/` renders correctly in light and dark mode.

### Phase 4 — Live reload (1 day)

- `watcher.rs`: one `notify-debouncer-full` watcher per open document, watching the **parent folder** (non-recursive) and filtering events for that file. This copes with editors that save atomically (write a temp file, then rename), like VS Code and Vim. Debounce ~150 ms.
- On change, re-read the file and emit `doc:changed` **to that window only** (`emit_to(label, …)`), with the new content.
- File deleted or renamed → show a non-blocking banner ("File was moved or deleted") and keep the last content; resume if it reappears.
- Scroll preservation: before re-render, remember the heading nearest the top of the viewport plus the offset within it; restore after render. Falls back to the scroll ratio when headings change. Consider `morphdom`/`idiomorph` to patch the DOM rather than replacing it, which avoids flicker and keeps Mermaid diagrams that didn't change.
- Stop the watcher when the window closes.

**Done when:** saving the file in VS Code, Vim and TextEdit updates the view within ~0.3 s without jumping.

### Phase 5 — macOS integration (1–1½ days)

**Menu bar** (`menu.rs`, built in Rust, events routed to the focused window):

| Menu | Items |
|---|---|
| MDViewer | About, Settings… ⌘, , Services, Hide ⌘H, Hide Others ⌥⌘H, Quit ⌘Q (predefined items) |
| File | Open… ⌘O, Open Recent ▸, Close Window ⌘W, Reveal in Finder ⇧⌘R, Open in Editor ⌘E, Copy File Path ⌥⌘C, Print… ⌘P |
| Edit | Copy ⌘C, Select All ⌘A, Find… ⌘F, Find Next ⌘G, Find Previous ⇧⌘G |
| View | Toggle Sidebar ⌃⌘S, Actual Size ⌘0, Zoom In ⌘+, Zoom Out ⌘−, Reload ⌘R, Enter Full Screen ⌃⌘F |
| Go | Back ⌘[, Forward ⌘] |
| Window | Minimize ⌘M, Zoom, tab items (register this submenu as the app's Windows menu so macOS adds the tab and window list automatically) |
| Help | MDViewer Help |

- Disable items that don't apply (e.g. Reveal in Finder in the empty window); update on window focus.

**Open Recent:** keep the last 15 paths in the store plugin, rebuild the submenu when it changes, include "Clear Menu". Optionally also register files with `NSDocumentController.noteNewRecentDocumentURL` (via `objc2`) so they show in the Dock icon's menu; small but nice.

**File associations** (`bundle.fileAssociations` in `tauri.conf.json`):
- Extensions `md, markdown, mdown, mkd, mkdn`, role **Viewer**, rank **Default**. Settings has a **Make Default** button that asks macOS to make MDViewer the handler for `net.daringfireball.markdown` (macOS never switches an existing default by itself).
- Declare the standard UTI `net.daringfireball.markdown` (via `contentTypes` if available in the pinned Tauri version, otherwise in `src-tauri/Info.plist`, which Tauri merges into the bundle).
- Note: Finder only learns about associations from a built `.app`; during development use `pnpm tauri build --debug` and run that bundle once.

**Settings window** (small, opened with ⌘,): theme (System/Light/Dark), default zoom, content width, block remote images, "Open in Editor" app (picks an app; default `open -t`).

**Command-line helper (optional):** document `open -a MDViewer file.md`, or ship a tiny `mdv` shell script.

**Done when:** MDViewer appears in Finder's "Open With" for `.md` files, menus and shortcuts behave like other Mac apps, and native tabs work.

### Phase 6 — Reading comfort (1 day)

- **Sidebar = table of contents**, built from headings (h1–h3, collapsible), highlights the current section while scrolling (IntersectionObserver), click to jump. Toggle with ⌃⌘S; state persisted.
- **Find in page** (⌘F): WKWebView's native find bar isn't exposed by Tauri, so build a small find bar: search text nodes in the content, mark matches with the **CSS Custom Highlight API** (no DOM changes), show "3 of 12", ⌘G / ⇧⌘G, Esc to close.
- **Zoom:** scale the content's font-size via a CSS variable (crisper than page zoom, and the chrome stays the same size); persisted per app.
- **Print / Export PDF:** print stylesheet (no sidebar, page breaks avoided inside code blocks), trigger printing from Rust via the webview's print API; if that is unreliable, export the HTML and print via the system.

### Phase 7 — Packaging and distribution (½–1 day, plus Apple account setup)

- App icon: 1024×1024 source → `pnpm tauri icon`.
- Universal build: `rustup target add x86_64-apple-darwin` then `pnpm tauri build --target universal-apple-darwin` → `.app` and `.dmg`.
- **Code signing and notarization** (needed so colleagues don't get Gatekeeper warnings): Apple Developer ID Application certificate; set `APPLE_SIGNING_IDENTITY` and either `APPLE_API_KEY`/`APPLE_API_ISSUER`/`APPLE_API_KEY_PATH` or `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID`; Tauri signs and notarises during `tauri build`.
- Hardened runtime is enabled by Tauri; the app needs no extra entitlements (no network server, no camera, etc.).
- CI: GitHub Actions with `tauri-apps/tauri-action` on a macOS runner, building on tags and attaching the `.dmg` to a GitHub release.
- Optional later: `tauri-plugin-updater` for auto-updates (needs a signing key and a hosted update manifest).

## 5. Rust ↔ UI interface

**Commands (UI → Rust)**

| Command | Purpose |
|---|---|
| `take_pending_open() -> Option<PathBuf>` | Window asks which file it was created for (handles cold start). |
| `open_document(path) -> Document` | Read file, register window ↔ path, allow its folder for assets, start watcher. |
| `open_paths(paths)` | Open files in new windows/tabs (used by drag and drop, ⌘-click on links). |
| `close_document()` | Stop watcher, unregister path (also run on window destroy). |
| `reveal_in_finder(path)`, `open_in_editor(path)` | File menu actions. |
| `get_settings()`, `set_settings(patch)` | Settings backed by the store plugin. |

**Events (Rust → UI)**

| Event | Payload |
|---|---|
| `doc:changed` | `{ content, modified }` |
| `doc:missing` / `doc:restored` | `{ path }` |
| `menu:action` | `{ id }` for actions handled in the UI (find, zoom, toggle sidebar, back/forward, print) |
| `settings:changed` | full settings object (broadcast to all windows) |

All payloads typed on both sides (`serde` structs; matching TS types in `ipc.ts`, or generated with `tauri-specta` if the surface grows).

## 6. Security

- Capabilities file grants only: core window/event/menu defaults, `dialog:allow-open`, `opener:allow-open-url` (http/https/mailto), `opener:allow-open-path`, `opener:allow-reveal-item-in-dir`, store and window-state defaults. No `fs` or `shell` plugin.
- Markdown can contain raw HTML: always sanitise with DOMPurify, never run scripts from documents, strict CSP as in Phase 3.
- Asset protocol scope only contains folders of documents the user opened.
- Custom commands validate input: canonical paths only, regular files only, size limit.

## 7. Testing

- **Rendering (Vitest, jsdom):** snapshot tests of `render()` + sanitiser output for each fixture; tests that hostile HTML (`<script>`, `onerror=`, `javascript:` links) is stripped; link classification and image path rewriting unit tests.
- **Rust (`cargo test`):** document registry (dedupe, canonicalisation), pending-open queue, watcher with temp files including atomic-rename saves.
- **UI in a browser:** run the Vite app in Playwright with Tauri's `mockIPC` (`@tauri-apps/api/mocks`) to test find, TOC, link clicks and live reload without the native shell. (Tauri's WebDriver support doesn't cover macOS, so true end-to-end tests aren't practical.)
- **Manual release checklist:** cold start by double-click in Finder, drop on Dock icon, open the same file twice, native tabs (System Settings ▸ Prefer tabs: Always), live reload from VS Code/Vim/TextEdit, file deleted while open, light/dark switch while open, 5 MB file performance, Gatekeeper on a clean Mac.

## 8. Performance targets

- Cold launch to rendered document: < 300 ms on Apple silicon for a typical README.
- Re-render on file change: < 50 ms for a 100 KB file (excluding Mermaid).
- Bundle: < 15 MB (Mermaid and Shiki grammars are the big parts; both load lazily).
- Very large files (> 2 MB): render the first part immediately, the rest in idle callbacks; consider moving parsing to a Web Worker if needed.

## 9. Milestones

| Milestone | Phases | Result |
|---|---|---|
| **M1 – It opens files** | 0–2 | Usable viewer via ⌘O, native-looking window |
| **M2 – Daily driver** | 3–5 | Rich rendering, live reload, Finder integration, menus |
| **M3 – Polished** | 6–7 | TOC, find, zoom, print, signed DMG for the team |

Total: roughly 8–10 working days for one developer, less with heavy AI assistance on the UI parts.

## 10. Risks and how we handle them

| Risk | Mitigation |
|---|---|
| Files opened from Finder before the web view is ready get lost | Pending-open queue pulled by the window (Phase 2); explicit test. |
| Atomic saves break file watching | Watch the parent folder and filter by path (Phase 4). |
| "Web app" feel creeps in (text selection on buttons, pointer cursors, bounce) | Chrome CSS rules in Phase 1 plus a review pass against Notes/Preview. |
| Large Mermaid/Shiki bundles slow startup | Dynamic imports; only load when a document needs them. |
| Some Tauri macOS options are new or change between 2.x versions | Pin versions; verify each config key during Phase 0/1. |
| No notarization → colleagues see "unidentified developer" warnings | Get the Developer ID certificate early (it can take a few days). |

## 11. Later ideas (not in v1)

- **Folder mode:** open a folder, file tree in the sidebar, quick-open (⌘P) across its Markdown files.
- **Quick Look extension:** a separate small Swift extension target embedded in the `.app` that reuses the same HTML/CSS renderer. Needs Xcode tooling for that one piece.
- Export to HTML/PDF, custom CSS themes, presentation mode, word count/reading time.

## 12. Decisions needed before starting

1. Bundle identifier and app name (proposed: `com.mmmikael.mdviewer`, "MDViewer").
2. Svelte vs React for the UI (plan assumes Svelte 5).
3. Who will own the Apple Developer ID certificate for signing and notarization?
4. ~~Should MDViewer offer to become the default app for `.md` files?~~ Yes: Settings ▸ Make Default.
