// Syntax highlighting with Shiki. Grammars are loaded on demand, so a document
// only pays for the languages it actually uses.

import { createHighlighterCore, type HighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import { bundledLanguages } from "shiki/langs";
import { bundledThemes } from "shiki/themes";

type LanguageId = keyof typeof bundledLanguages;

let highlighter: HighlighterCore | null = null;
let starting: Promise<HighlighterCore> | null = null;
const failed = new Set<string>();

function start(): Promise<HighlighterCore> {
  starting ??= createHighlighterCore({
    themes: [bundledThemes["github-light"], bundledThemes["github-dark"]],
    langs: [],
    engine: createJavaScriptRegexEngine(),
  }).then((h) => (highlighter = h));
  return starting;
}

const FENCE_INFO = /^[ \t]{0,3}(?:`{3,}|~{3,})[ \t]*([\w+#.-]+)/gm;

/** Languages named on fenced code blocks in `source`. */
export function fenceLanguages(source: string): string[] {
  const langs = new Set<string>();
  for (const match of source.matchAll(FENCE_INFO)) langs.add(match[1].toLowerCase());
  return [...langs];
}

/** Loads the grammars `source` needs. Call before rendering. */
export async function prepareHighlighting(source: string): Promise<void> {
  const wanted = fenceLanguages(source).filter(
    (lang): lang is LanguageId => lang in bundledLanguages && !failed.has(lang),
  );
  if (wanted.length === 0 && highlighter) return;
  const h = await start();
  const loaded = new Set(h.getLoadedLanguages());
  await Promise.all(
    wanted
      .filter((lang) => !loaded.has(lang))
      .map((lang) =>
        h.loadLanguage(bundledLanguages[lang]).catch(() => {
          failed.add(lang);
        }),
      ),
  );
}

/** Highlighted `<pre>` HTML, or null when the language isn't available. */
export function highlight(code: string, lang: string): string | null {
  if (!highlighter || !lang) return null;
  const id = lang.toLowerCase();
  if (!highlighter.getLoadedLanguages().includes(id)) return null;
  try {
    return highlighter.codeToHtml(code, {
      lang: id,
      themes: { light: "github-light", dark: "github-dark" },
      defaultColor: false,
      transformers: [
        {
          pre(node) {
            node.properties["data-lang"] = id;
            delete node.properties.tabindex;
          },
        },
      ],
    });
  } catch {
    return null;
  }
}
