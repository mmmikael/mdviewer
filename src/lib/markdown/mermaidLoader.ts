// Loads Mermaid as a lazy chunk. Quick Look swaps this module for
// src/quicklook/mermaidLoader.ts (see quicklook/vite.config.ts).

export type Mermaid = (typeof import("mermaid"))["default"];

export async function loadMermaid(): Promise<Mermaid> {
  return (await import("mermaid")).default;
}
