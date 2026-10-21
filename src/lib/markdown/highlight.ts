// Syntax highlighting with Shiki. Grammars are loaded on demand, so a document
// only pays for the languages it actually uses.

import { bundledLanguages } from "shiki/langs";
import { bundledThemes } from "shiki/themes";

import { createHighlighting } from "./highlightCore";

export { fenceLanguages } from "./highlightCore";

export const { prepareHighlighting, highlight } = createHighlighting({
  themes: [bundledThemes["github-light"], bundledThemes["github-dark"]],
  lazyLangs: bundledLanguages,
});
