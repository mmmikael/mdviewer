// Turns rendered HTML into a safe DOM fragment ready to show: sanitises it,
// points local images at the asset protocol, tags links by kind and adds
// copy buttons to code blocks.

import DOMPurify from "dompurify";

import { hasScheme, resolveLocal, splitHash } from "../paths";

export interface PostprocessOptions {
  /** Folder of the document, used to resolve relative links and images. */
  baseDir: string;
  /** Maps an absolute file path to a URL the web view may load. */
  assetUrl: (path: string) => string;
  blockRemoteImages: boolean;
}

const EXTERNAL_SCHEMES = /^(https?|mailto|tel):/i;
const INLINE_SCHEMES = /^(data|blob):/i;

export function sanitize(html: string): DocumentFragment {
  return DOMPurify.sanitize(html, {
    RETURN_DOM_FRAGMENT: true,
    FORBID_TAGS: ["style", "form", "button", "textarea", "select"],
    ADD_ATTR: ["data-lang"],
  });
}

export function buildFragment(html: string, options: PostprocessOptions): DocumentFragment {
  const fragment = sanitize(html);
  rewriteImages(fragment, options);
  classifyLinks(fragment, options.baseDir);
  decorateCodeBlocks(fragment);
  return fragment;
}

function localUrl(ref: string, options: PostprocessOptions): string {
  return options.assetUrl(resolveLocal(options.baseDir, ref));
}

export function rewriteImages(root: ParentNode, options: PostprocessOptions): void {
  for (const img of root.querySelectorAll("img")) {
    img.loading = "lazy";
    const src = img.getAttribute("src");
    if (!src || INLINE_SCHEMES.test(src)) continue;
    if (/^https?:/i.test(src)) {
      if (options.blockRemoteImages) {
        img.removeAttribute("src");
        img.classList.add("blocked-image");
        img.title = `Remote image blocked: ${src}`;
      }
      continue;
    }
    if (hasScheme(src) && !/^file:/i.test(src)) continue;
    img.dataset.localPath = resolveLocal(options.baseDir, src);
    img.src = localUrl(src, options);
  }

  // <picture><source srcset="…"> is common in READMEs for dark-mode images.
  for (const source of root.querySelectorAll("source[srcset]")) {
    const srcset = source.getAttribute("srcset")!;
    const rewritten = srcset
      .split(",")
      .map((candidate) => {
        const [url, ...descriptor] = candidate.trim().split(/\s+/);
        if (!url || hasScheme(url)) return candidate.trim();
        return [localUrl(url, options), ...descriptor].join(" ");
      })
      .join(", ");
    source.setAttribute("srcset", rewritten);
  }
}

export function classifyLinks(root: ParentNode, baseDir: string): void {
  for (const link of root.querySelectorAll<HTMLAnchorElement>("a[href]")) {
    const href = link.getAttribute("href")!;
    if (href.startsWith("#")) {
      link.dataset.link = "anchor";
      link.dataset.hash = href.slice(1);
    } else if (EXTERNAL_SCHEMES.test(href)) {
      link.dataset.link = "external";
      link.title ||= href;
    } else if (!hasScheme(href) || /^file:/i.test(href)) {
      const [path, hash] = splitHash(href);
      link.dataset.link = "file";
      link.dataset.path = path ? resolveLocal(baseDir, path) : "";
      link.dataset.hash = hash;
    } else {
      link.dataset.link = "unsupported";
    }
  }
}

function codeLanguage(pre: Element): string {
  const fromShiki = pre.getAttribute("data-lang");
  if (fromShiki) return fromShiki;
  const code = pre.querySelector("code");
  const match = code?.className.match(/language-([\w+#.-]+)/);
  return match?.[1] ?? "";
}

export function decorateCodeBlocks(root: ParentNode): void {
  for (const pre of root.querySelectorAll("pre")) {
    if (pre.classList.contains("mermaid-source") || pre.closest(".front-matter")) continue;
    const wrapper = document.createElement("div");
    wrapper.className = "code-block";
    const lang = codeLanguage(pre);
    if (lang) wrapper.dataset.lang = lang;
    const copy = document.createElement("button");
    copy.type = "button";
    copy.className = "copy-button";
    copy.textContent = "Copy";
    copy.setAttribute("aria-label", "Copy code");
    pre.replaceWith(wrapper);
    wrapper.append(pre, copy);
  }
}
