// Loads Mermaid. The built app uses the copy inside its Quick Look extension,
// served by the app as mdvres:// (see src-tauri/src/resources.rs), so the
// bundle carries Mermaid once instead of also building it into the program.
// Quick Look swaps this module for src/quicklook/mermaidLoader.ts.

import { loadMermaidScript, type Mermaid } from "./mermaidScript";

export type { Mermaid };

export async function loadMermaid(): Promise<Mermaid> {
  // The dev server and the browser mock have no app bundle to load from.
  if (import.meta.env.DEV) return (await import("mermaid")).default;
  return loadMermaidScript("mdvres://localhost/mermaid.min.js");
}
