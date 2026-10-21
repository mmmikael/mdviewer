// Turning the rendered document into something that works outside the app:
// a self-contained HTML file, or HTML for the clipboard.

import katexCss from "katex/dist/katex.min.css?inline";
import alertsBaseCss from "markdown-it-github-alerts/styles/github-base.css?inline";
import alertsDarkCss from "markdown-it-github-alerts/styles/github-colors-dark-media.css?inline";
import alertsLightCss from "markdown-it-github-alerts/styles/github-colors-light.css?inline";
import markdownCss from "../styles/markdown.css?inline";
import { escapeHtml } from "./markdown/render";

// The few app-wide tokens markdown.css relies on, plus a page layout.
const PAGE_CSS = `
:root { color-scheme: light dark; --text: #1f2328; --content-bg: #ffffff; --accent: #0969da; }
@media (prefers-color-scheme: dark) { :root { --text: #e8e8ed; --content-bg: #1e1e1e; --accent: #4493f8; } }
html { background: var(--content-bg); }
body { margin: 0; }
.markdown { max-width: 52em; margin: 0 auto; padding: 32px 24px 64px; }
`;

/**
 * Removes what only makes sense inside the app: copy buttons, and the
 * absolute file paths the app keeps on links (exports would leak them).
 * Links keep the href written in the Markdown.
 */
function stripAppChrome(root: ParentNode) {
  for (const el of root.querySelectorAll(".copy-button")) el.remove();
  for (const link of root.querySelectorAll<HTMLAnchorElement>("a[data-link]")) {
    for (const key of Object.keys(link.dataset)) delete link.dataset[key];
  }
}

async function toDataUrl(url: string): Promise<string | null> {
  try {
    const response = await fetch(url);
    if (!response.ok) return null;
    const blob = await response.blob();
    return await new Promise((resolve) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => resolve(null);
      reader.readAsDataURL(blob);
    });
  } catch {
    return null;
  }
}

/** Embeds images that only the app can load (local files). Remote ones keep their URL. */
async function embedLocalImages(root: ParentNode) {
  await Promise.all(
    [...root.querySelectorAll<HTMLImageElement>("img[data-local-path]")].map(async (img) => {
      const data = await toDataUrl(img.getAttribute("src") ?? "");
      // An image that can't be read keeps only its alt text, not the app's internal URL.
      if (data) img.setAttribute("src", data);
      else img.removeAttribute("src");
      img.removeAttribute("data-local-path");
      img.removeAttribute("loading");
    }),
  );
  for (const source of root.querySelectorAll("source[srcset]")) {
    if (!/^https?:/i.test(source.getAttribute("srcset") ?? "")) source.remove();
  }
}

/** KaTeX fonts, keeping only the woff2 versions, embedded so math renders offline. */
async function embeddedKatexCss(): Promise<string> {
  const fonts = new Map<string, string>();
  for (const match of katexCss.matchAll(/url\(([^)]+?\.woff2)\)/g)) fonts.set(match[1], "");
  await Promise.all(
    [...fonts.keys()].map(async (url) => fonts.set(url, (await toDataUrl(url.replace(/^["']|["']$/g, ""))) ?? url)),
  );
  return katexCss.replace(
    /src:\s*url\(([^)]+?\.woff2)\)\s*format\(["']woff2["']\)[^;}]*/g,
    (_, url: string) => `src:url(${fonts.get(url) ?? url}) format("woff2")`,
  );
}

/** A complete, self-contained HTML document of the rendered page. */
export async function standaloneHtml(article: HTMLElement, title: string): Promise<string> {
  const content = article.cloneNode(true) as HTMLElement;
  stripAppChrome(content);
  await embedLocalImages(content);
  const usesMath = content.querySelector(".katex") !== null;
  const css = [
    PAGE_CSS,
    markdownCss,
    alertsBaseCss,
    alertsLightCss,
    alertsDarkCss,
    usesMath ? await embeddedKatexCss() : "",
  ].join("\n");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="generator" content="MDViewer">
<title>${escapeHtml(title)}</title>
<style>
${css}
</style>
</head>
<body>
<article class="markdown">
${content.innerHTML}
</article>
</body>
</html>
`;
}

/**
 * HTML for pasting into mail, documents and chat. Those apps ignore stylesheets
 * and CSS variables, so code colours are written inline and math becomes TeX.
 */
export async function clipboardHtml(root: ParentNode): Promise<string> {
  const container = document.createElement("div");
  container.append(...[...root.childNodes].map((node) => node.cloneNode(true)));
  // Links inside the document point nowhere once pasted elsewhere.
  for (const link of container.querySelectorAll<HTMLAnchorElement>("a[data-link='anchor']")) {
    link.removeAttribute("href");
  }
  stripAppChrome(container);
  await embedLocalImages(container);

  for (const el of container.querySelectorAll<HTMLElement>("[style*='--shiki-light']")) {
    const color = el.style.getPropertyValue("--shiki-light");
    const fontStyle = el.style.getPropertyValue("--shiki-light-font-style");
    el.removeAttribute("style");
    if (color && !el.matches("pre")) el.style.color = color;
    if (fontStyle) el.style.fontStyle = fontStyle;
  }
  for (const pre of container.querySelectorAll<HTMLElement>("pre")) {
    pre.style.cssText =
      "background:#f6f8fa;padding:12px 16px;border-radius:6px;font-family:'SF Mono',Menlo,monospace;font-size:13px;line-height:1.45;white-space:pre-wrap";
  }
  for (const math of container.querySelectorAll(".katex-display, .katex")) {
    // Inline .katex inside an already replaced display block is gone.
    if (!container.contains(math)) continue;
    const tex = (math.querySelector("annotation[encoding='application/x-tex']")?.textContent ?? "").trim();
    const display = math.classList.contains("katex-display");
    const replacement = document.createElement(display ? "pre" : "code");
    replacement.textContent = display ? `$$\n${tex}\n$$` : `$${tex}$`;
    // Block math sits in its own <p class="katex-block">; a <pre> can't go inside a <p>.
    const wrapper = math.parentElement;
    if (display && wrapper?.matches("p.katex-block") && wrapper.childElementCount === 1) wrapper.replaceWith(replacement);
    else math.replaceWith(replacement);
  }
  return container.innerHTML;
}
