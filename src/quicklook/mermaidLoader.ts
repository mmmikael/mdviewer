// Quick Look's Mermaid loader. The preview script can't lazy-load module
// chunks over file://, so quicklook/build.sh ships Mermaid's single-file build
// next to it, added only when a document contains a diagram.

import type { Mermaid } from "../lib/markdown/mermaidLoader";

let loading: Promise<Mermaid> | null = null;

export function loadMermaid(): Promise<Mermaid> {
  loading ??= new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = "mermaid.min.js";
    script.onload = () => resolve((globalThis as unknown as { mermaid: Mermaid }).mermaid);
    script.onerror = () => reject(new Error("Couldn’t load the diagram renderer."));
    document.head.append(script);
  });
  return loading;
}
