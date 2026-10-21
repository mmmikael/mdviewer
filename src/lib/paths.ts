// Small POSIX path helpers. Kept synchronous (the Tauri path API is async)
// because they run for every link and image while building the document.

export const MARKDOWN_EXTENSIONS = ["md", "markdown", "mdown", "mkd", "mkdn", "mdwn", "mdx", "txt"];

/** True for `https:`, `mailto:`, `data:` … but not for Windows-style `C:` paths. */
export function hasScheme(href: string): boolean {
  return /^[a-z][a-z0-9+.-]+:/i.test(href);
}

export function isMarkdownPath(path: string): boolean {
  const ext = path.split(/[?#]/)[0].split(".").pop()?.toLowerCase() ?? "";
  return MARKDOWN_EXTENSIONS.includes(ext);
}

/** Splits `a/b.md#intro` into `["a/b.md", "intro"]` (query strings are dropped). */
export function splitHash(href: string): [string, string] {
  const hashIndex = href.indexOf("#");
  const path = hashIndex === -1 ? href : href.slice(0, hashIndex);
  const hash = hashIndex === -1 ? "" : href.slice(hashIndex + 1);
  return [path.split("?")[0], hash];
}

export function normalize(path: string): string {
  const absolute = path.startsWith("/");
  const parts: string[] = [];
  for (const part of path.split("/")) {
    if (part === "" || part === ".") continue;
    if (part === "..") {
      if (parts.length && parts[parts.length - 1] !== "..") parts.pop();
      else if (!absolute) parts.push("..");
      continue;
    }
    parts.push(part);
  }
  return (absolute ? "/" : "") + parts.join("/");
}

function safeDecode(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

/** Resolves a link or image reference from a document in `baseDir` to an absolute path. */
export function resolveLocal(baseDir: string, ref: string): string {
  if (/^file:/i.test(ref)) {
    return normalize(safeDecode(new URL(ref).pathname));
  }
  const decoded = safeDecode(ref);
  return normalize(decoded.startsWith("/") ? decoded : `${baseDir}/${decoded}`);
}

export function dirname(path: string): string {
  const index = path.lastIndexOf("/");
  return index <= 0 ? "/" : path.slice(0, index);
}

export function basename(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}
