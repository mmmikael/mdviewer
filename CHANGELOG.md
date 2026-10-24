# Changelog

Downloads for every version are on the [releases page](https://github.com/mmmikael/mdviewer/releases).

## 0.3.2 — 2026-10-24

- **TL;DR as a diagram.** A List / Diagram switch in the summary card draws the whole document as a Mermaid flowchart, mind map or timeline. Its nodes link to their sections. A diagram that can't be drawn gets one automatic retry.
- **Huge documents on small models.** Notes on the parts of a long document are merged in rounds until they fit the model, and the list of sections is kept within budget. No request exceeds the model's context.
- **Zoom.** The summary zooms with the document and lines up with its text column.

## 0.3.1 — 2026-10-24

- **Signed and notarised** with a Developer ID: MDViewer opens like any other app, without "Open Anyway". The Quick Look extension is signed the same way.

## 0.3.0 — 2026-10-23

- **AI summaries (TL;DR).** View ▸ Summarize (⇧⌘S) writes a short summary above the document, with links to its sections. Bring your own key for OpenRouter, OpenAI, Anthropic, Google Gemini, xAI Grok, Mistral, DeepSeek or Groq, or run a model locally with Ollama or LM Studio. Editable instructions; keys stay in the macOS keychain; documents are only sent when you ask.
- **Smaller downloads:** 5.6 MB for Apple silicon, 5.7 MB for Intel, 8.9 MB universal.

## 0.2.2 — 2026-10-23

- Separate downloads for Apple silicon and Intel, about half the size of the universal one.
- Ticked task-list checkboxes are clearly visible, in the app and in Quick Look.

## 0.2.1 — 2026-10-22

- Native title bar: the window buttons no longer jump while resizing.
- The sidebar button sits where Mac apps put it.

## 0.2.0 — 2026-10-21

- **Quick Look** previews with code highlighting, math and Mermaid diagrams.
- **Export as PDF** and as a single self-contained **HTML** file.
- **Copy as Rich Text** for Mail, Docs and Slack, with math as TeX.
- Hovering a link shows where it goes, like Safari.

## 0.1.1 — 2026-10-21

- New bundle identifier (`com.mmmikael.mdviewer`) and an Install section in the README.

## 0.1.0 — 2026-10-21

- First release: GitHub-flavoured Markdown with live reload, a table of contents sidebar, find, zoom, recent files, light and dark mode, and "Make Default" for Markdown files.
