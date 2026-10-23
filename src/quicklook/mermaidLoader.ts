// Quick Look's Mermaid loader. The preview script can't lazy-load module
// chunks over file://, so quicklook/build.sh ships Mermaid's single-file build
// next to it (the app loads that same file), added only when a document
// contains a diagram.

import { loadMermaidScript, type Mermaid } from "../lib/markdown/mermaidScript";

export function loadMermaid(): Promise<Mermaid> {
  return loadMermaidScript("mermaid.min.js");
}
