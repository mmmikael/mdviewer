// Markdown → HTML string. Synchronous; call `prepareHighlighting` first so the
// needed grammars are loaded.

import MarkdownIt, { type MarkdownIt as MarkdownItInstance } from "markdown-it";
import anchor from "markdown-it-anchor";
import footnote from "markdown-it-footnote";
import frontMatter from "markdown-it-front-matter";
import alerts from "markdown-it-github-alerts";
import taskLists from "markdown-it-task-lists";
import katexPlugin, { type MarkdownKatexOptions } from "@vscode/markdown-it-katex";

import { highlight } from "./highlight";

/** GitHub-style heading ids: lower case, punctuation removed, spaces → dashes. */
export function slugify(text: string): string {
  return text
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\p{M}\s_-]/gu, "")
    .replace(/\s/g, "-");
}

export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** CommonJS plugins may arrive wrapped in `{ default }`, depending on the bundler. */
function interop<T>(mod: T): T {
  const wrapped = mod as { default?: T };
  return typeof mod === "function" ? mod : (wrapped.default ?? mod);
}

// The KaTeX plugin is typed against the older @types/markdown-it.
const katex = interop(katexPlugin) as unknown as (md: MarkdownItInstance, options: MarkdownKatexOptions) => void;

// Filled by the front-matter plugin during `md.render`.
const frontMatterBox: { raw: string | null } = { raw: null };

const md = new MarkdownIt({
  html: true,
  linkify: true,
  typographer: false,
  highlight: (code, lang) => highlight(code, lang) ?? "",
});

md.use(interop(frontMatter), (raw: string) => {
  frontMatterBox.raw = raw;
})
  .use(anchor, { slugify, tabIndex: false })
  .use(interop(footnote))
  .use(interop(taskLists), { enabled: false, label: true })
  .use(alerts)
  .use(katex, { enableFencedBlocks: true, throwOnError: false });

// Mermaid blocks are rendered to SVG later (see mermaid.ts); emit their source.
const defaultFence = md.renderer.rules.fence!;
md.renderer.rules.fence = (tokens, idx, options, env, self) => {
  const token = tokens[idx];
  const lang = token.info.trim().split(/\s+/)[0].toLowerCase();
  if (lang === "mermaid") {
    return `<pre class="mermaid-source">${escapeHtml(token.content)}</pre>\n`;
  }
  return defaultFence(tokens, idx, options, env, self);
};

export function renderMarkdown(source: string): string {
  frontMatterBox.raw = null;
  const html = md.render(source);
  const raw = frontMatterBox.raw as string | null;
  if (raw === null || raw.trim() === "") return html;
  return (
    `<details class="front-matter"><summary>Front matter</summary>` +
    `<pre><code>${escapeHtml(raw)}</code></pre></details>\n` +
    html
  );
}
