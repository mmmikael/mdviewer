// The Quick Look preview page (quicklook/preview.html). The Swift extension
// loads it and calls `mdvRender(source, baseDir)` with the file's contents.
// It uses the app's own Markdown pipeline so previews match MDViewer,
// including Mermaid diagrams (see ./mermaidLoader.ts).

import "katex/dist/katex.min.css";
import "markdown-it-github-alerts/styles/github-base.css";
import "markdown-it-github-alerts/styles/github-colors-light.css";
import "markdown-it-github-alerts/styles/github-colors-dark-media.css";
import "../styles/markdown.css";
import "./preview.css";

import { renderDiagrams } from "../lib/markdown/mermaid";
import { buildFragment } from "../lib/markdown/postprocess";
import { renderMarkdown } from "../lib/markdown/render";
import { prepareHighlighting } from "./highlight";

async function mdvRender(source: string, baseDir: string): Promise<void> {
  await prepareHighlighting(source);
  const fragment = buildFragment(renderMarkdown(source), {
    baseDir,
    assetUrl: (path) => `file://${encodeURI(path)}`,
    blockRemoteImages: false,
  });

  // Quick Look may only read the previewed file itself, so images next to it
  // can't load: show their description instead of a broken image.
  for (const img of fragment.querySelectorAll<HTMLImageElement>("img[data-local-path]")) {
    const placeholder = document.createElement("span");
    placeholder.className = "image-placeholder";
    placeholder.textContent = img.alt || img.dataset.localPath!.split("/").pop()!;
    img.replaceWith(placeholder);
  }
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  await renderDiagrams(fragment, dark ? "dark" : "light");

  document.getElementById("content")!.replaceChildren(fragment);
}

Object.assign(window, { mdvRender });
