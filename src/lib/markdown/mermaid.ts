// Mermaid diagrams. The library is large, so it is only loaded when a document
// contains a diagram. Results are cached so live reloads don't flicker.

import { escapeHtml } from "./render";

type Mermaid = (typeof import("mermaid"))["default"];

let mermaid: Mermaid | null = null;
let initializedFor: "light" | "dark" | null = null;
let counter = 0;
const cache = new Map<string, string>();

async function load(theme: "light" | "dark"): Promise<Mermaid> {
  mermaid ??= (await import("mermaid")).default;
  if (initializedFor !== theme) {
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      theme: theme === "dark" ? "dark" : "default",
      fontFamily: "-apple-system, BlinkMacSystemFont, 'Helvetica Neue', sans-serif",
    });
    initializedFor = theme;
  }
  return mermaid;
}

/** Replaces every `pre.mermaid-source` in `root` with its rendered diagram. */
export async function renderDiagrams(root: ParentNode, theme: "light" | "dark"): Promise<void> {
  const blocks = [...root.querySelectorAll<HTMLPreElement>("pre.mermaid-source")];
  if (blocks.length === 0) return;
  const m = await load(theme);

  for (const pre of blocks) {
    const source = pre.textContent ?? "";
    const key = `${theme}\n${source}`;
    const figure = document.createElement("div");
    figure.className = "mermaid-diagram";

    let svg = cache.get(key);
    if (svg === undefined) {
      const id = `mermaid-${++counter}`;
      try {
        ({ svg } = await m.render(id, source));
        cache.set(key, svg);
      } catch (error) {
        // Mermaid leaves its scratch element behind when parsing fails.
        document.getElementById(`d${id}`)?.remove();
        figure.classList.add("mermaid-error");
        figure.innerHTML =
          `<div class="mermaid-error-message">Diagram error: ${escapeHtml(String((error as Error)?.message ?? error))}</div>` +
          `<pre>${escapeHtml(source)}</pre>`;
        pre.replaceWith(figure);
        continue;
      }
    }
    figure.innerHTML = svg;
    pre.replaceWith(figure);
  }

  // Keep the cache from growing without bound while editing diagrams live.
  while (cache.size > 200) cache.delete(cache.keys().next().value!);
}
