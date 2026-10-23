// Syntax highlighting with Shiki. Grammars are loaded on demand, so a document
// only pays for the languages it actually uses.

import { bundledLanguages } from "shiki/langs";
import githubDark from "shiki/themes/github-dark.mjs";
import githubLight from "shiki/themes/github-light.mjs";

import { createHighlighting } from "./highlightCore";

export { fenceLanguages } from "./highlightCore";

// The two themes are imported directly: going through `bundledThemes` would
// ship all ~60 of Shiki's themes with the app.
export const { prepareHighlighting, highlight } = createHighlighting({
  themes: [githubLight, githubDark],
  lazyLangs: bundledLanguages,
});
