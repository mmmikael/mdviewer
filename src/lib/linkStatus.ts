// The text shown in the status overlay when hovering a link, like Safari's
// bottom-left URL preview.

export interface LinkInfo {
  /** `data-link` set by postprocess.ts: anchor, external, file or unsupported. */
  kind: string | undefined;
  href: string;
  path?: string;
  hash?: string;
}

export function linkStatus(link: LinkInfo, docPath: string, newWindow = false): string {
  const hash = link.hash ? `#${link.hash}` : "";
  switch (link.kind) {
    case "anchor":
      return hash || link.href;
    case "external":
      return link.href;
    case "file": {
      if (!link.path || link.path === docPath) return hash || link.href;
      const dir = docPath.slice(0, docPath.lastIndexOf("/") + 1);
      const shown = link.path.startsWith(dir) ? link.path.slice(dir.length) : link.path;
      return newWindow ? `Open “${shown}${hash}” in a new window` : shown + hash;
    }
    default:
      return link.href;
  }
}
