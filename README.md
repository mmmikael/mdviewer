# MDViewer

A small, native-feeling Markdown viewer for macOS, built with [Tauri 2](https://v2.tauri.app) (Rust) and Svelte 5. See [PLAN.md](PLAN.md) for the design and the reasoning behind it.

- Opens `.md` files from Finder, the Dock, drag and drop, ⌘O, Open Recent and `open -a MDViewer file.md`; declares itself a default handler for Markdown, and Settings has a **Make Default** button for Macs where another app is already the default
- GitHub-flavoured Markdown: tables, task lists, footnotes, alerts, front matter, syntax highlighting (Shiki), Mermaid diagrams, KaTeX math, local and remote images
- Live reload when the file changes on disk, keeping the scroll position
- Native window chrome: translucent sidebar with table of contents, inset traffic lights, macOS menus and shortcuts, native window tabs, light/dark mode
- Find in page, zoom, back/forward between linked documents, print, Settings window

## Install

Requires macOS 13 or later; one download works on both Apple silicon and Intel Macs.

1. Download the `.dmg` from the [latest release](https://github.com/mmmikael/mdviewer/releases/latest).
2. Open it and drag **MDViewer** to **Applications**.
3. Open MDViewer. Releases aren't notarised by Apple yet, so the first time macOS blocks it: go to **System Settings ▸ Privacy & Security**, scroll down and click **Open Anyway**. You only need to do this once.
4. To open Markdown files with MDViewer when you double-click them, choose **MDViewer ▸ Settings…** and click **Make Default**.

## Development

Requirements: Rust (1.89+), Node 22+, pnpm, Xcode command line tools.

```bash
pnpm install
```

```bash
pnpm tauri dev
```

Open a file at startup: `pnpm tauri dev -- -- "$PWD/tests/fixtures/rich.md"`. Frontend changes hot-reload; Rust changes rebuild and restart the app.

**UI in a browser:** while `pnpm dev` (or `tauri dev`) runs, open <http://127.0.0.1:1420/?file=rich.md> in any browser. Tauri's IPC is mocked by `src/dev/mockTauri.ts` (dev builds only). Use `?window=settings` for the Settings window, and `mdv.menu("find")` / `mdv.edit("# new content")` in the console to simulate menu actions and file changes.

| Script | What it does |
|---|---|
| `pnpm test` | Vitest: Markdown pipeline, sanitising, links, images, TOC, find |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust: document registry, file reading, settings, watcher matching |
| `pnpm lint` | `svelte-check`, `cargo fmt --check`, `cargo clippy -D warnings` |
| `pnpm tauri:build` | Release `.app` and `.dmg` for this Mac's architecture |
| `pnpm tauri:build:universal` | Universal (Apple silicon + Intel) build; needs `rustup target add x86_64-apple-darwin` |

Build output: `src-tauri/target/release/bundle/` (about 8 MB app, 6 MB DMG).

## Project layout

```
src/                       Web UI (Svelte 5 + TypeScript)
  DocumentWindow.svelte    Document window: rendering, links, find, zoom, history
  SettingsWindow.svelte    Settings window
  lib/markdown/            markdown-it setup, Shiki, Mermaid, sanitising + DOM post-processing
  lib/ipc.ts               Typed wrappers for Rust commands and events
  dev/mockTauri.ts         IPC mocks for running the UI in a browser
src-tauri/src/
  lib.rs                   App setup and lifecycle (Finder opens, Dock reopen, window events)
  documents.rs             Window ↔ file registry, window creation, Open dialog, asset scope
  watcher.rs               File watching (watches the folder, so atomic saves work)
  menu.rs                  Menu bar, Open Recent, menu actions
  settings.rs              Settings and recent files (store plugin)
  platform.rs              AppKit bits: Dock recents, clipboard, traffic light position
tests/                     Vitest tests and Markdown fixtures
```

## Notes

- **Versions are pinned to Tauri 2.11 / plugin 2.4–2.7** because newer plugin crates need Rust 1.90. Keep the JS packages (`@tauri-apps/*`) on the same minor versions as their Rust crates when upgrading.
- **Traffic lights** are positioned in `platform.rs` (`TRAFFIC_LIGHTS`) because Tauri's `trafficLightPosition` doesn't take effect on current macOS for these windows. The sidebar toggle in `DocumentWindow.svelte` is placed to match.
- **Security:** documents are sanitised with DOMPurify, scripts never run, the web view can't navigate away from the app, and only folders of opened documents (or their git repository) are readable through the asset protocol. There is no `fs` or `shell` plugin.

## Releasing

Signing and notarisation need an Apple Developer ID certificate. Locally, set `APPLE_SIGNING_IDENTITY` plus either `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` or an App Store Connect API key before `pnpm tauri:build`. In CI, pushing a `v*` tag runs `.github/workflows/build.yml`, which builds a universal DMG and attaches it to a draft GitHub release; it expects the secrets listed in that file.

Unsigned builds work on the machine that built them; other Macs will show a Gatekeeper warning.

## License

[MIT](LICENSE)
