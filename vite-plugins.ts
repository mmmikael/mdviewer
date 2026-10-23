// Vite plugins shared by the app build (vite.config.ts) and the Quick Look
// build (quicklook/vite.config.ts).

import type { Plugin } from "vite";

/**
 * KaTeX lists woff2, woff and ttf files for every font. WebKit only needs
 * woff2, so drop the others (about 0.6 MB per copy of the app).
 */
export function katexWoff2Only(): Plugin {
  return {
    name: "katex-woff2-only",
    // Before Vite's CSS handling picks up the font URLs.
    enforce: "pre",
    transform(code, id) {
      // Also matches `katex.min.css?inline`, which HTML export uses.
      const path = id.split("?")[0];
      if (!path.includes("katex") || !path.endsWith(".css")) return null;
      return code.replace(/,\s*url\([^)]+\.(?:woff|ttf)\)\s*format\(["'](?:woff|truetype)["']\)/g, "");
    },
  };
}
