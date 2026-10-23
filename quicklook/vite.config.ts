// Builds the Quick Look preview script as one classic script plus one
// stylesheet (WKWebView won't load ES modules or lazy chunks from file://).
// Run by quicklook/build.sh with --outDir.

import { resolve } from "node:path";
import { defineConfig } from "vite";

import { katexWoff2Only } from "../vite-plugins";

const root = resolve(import.meta.dirname, "..");

export default defineConfig({
  root,
  plugins: [katexWoff2Only()],
  resolve: {
    alias: [
      // The app's highlighter lazy-loads 300 grammars; use a bundled subset.
      { find: /^\.\/highlight$/, replacement: resolve(root, "src/quicklook/highlight.ts") },
      // Mermaid comes from a separate script file instead of a lazy chunk.
      { find: /^\.\/mermaidLoader$/, replacement: resolve(root, "src/quicklook/mermaidLoader.ts") },
    ],
  },
  build: {
    target: "safari16",
    emptyOutDir: true,
    cssCodeSplit: false,
    lib: {
      entry: resolve(root, "src/quicklook/main.ts"),
      formats: ["iife"],
      name: "MDViewerQuickLook",
      fileName: () => "preview.js",
      cssFileName: "preview",
    },
  },
});
