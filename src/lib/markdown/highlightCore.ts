// Shiki setup shared by the app (all languages, loaded on demand) and the
// Quick Look preview (a fixed set bundled in, see src/quicklook/highlight.ts).

import {
  createHighlighterCore,
  type HighlighterCore,
  type LanguageInput,
  type ThemeInput,
} from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";

const FENCE_INFO = /^[ \t]{0,3}(?:`{3,}|~{3,})[ \t]*([\w+#.-]+)/gm;

/** Languages named on fenced code blocks in `source`. */
export function fenceLanguages(source: string): string[] {
  const langs = new Set<string>();
  for (const match of source.matchAll(FENCE_INFO)) langs.add(match[1].toLowerCase());
  return [...langs];
}

export interface HighlightingOptions {
  themes: ThemeInput[];
  /** Grammars loaded up front. */
  langs?: LanguageInput[];
  /** Grammars loaded when a document first uses them, by id or alias. */
  lazyLangs?: Record<string, LanguageInput>;
}

export function createHighlighting({ themes, langs = [], lazyLangs = {} }: HighlightingOptions) {
  let highlighter: HighlighterCore | null = null;
  let starting: Promise<HighlighterCore> | null = null;
  const failed = new Set<string>();

  function start(): Promise<HighlighterCore> {
    starting ??= createHighlighterCore({ themes, langs, engine: createJavaScriptRegexEngine() }).then(
      (h) => (highlighter = h),
    );
    return starting;
  }

  /** Loads the grammars `source` needs. Call before rendering. */
  async function prepareHighlighting(source: string): Promise<void> {
    const wanted = fenceLanguages(source).filter((lang) => lang in lazyLangs && !failed.has(lang));
    if (wanted.length === 0 && highlighter) return;
    const h = await start();
    const loaded = new Set(h.getLoadedLanguages());
    await Promise.all(
      wanted
        .filter((lang) => !loaded.has(lang))
        .map((lang) =>
          h.loadLanguage(lazyLangs[lang]).catch(() => {
            failed.add(lang);
          }),
        ),
    );
  }

  /** Highlighted `<pre>` HTML, or null when the language isn't available. */
  function highlight(code: string, lang: string): string | null {
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

  return { prepareHighlighting, highlight };
}
