export interface Heading {
  id: string;
  text: string;
  /** Indent level, 0 for the shallowest heading in the document. */
  depth: number;
}

/** Headings (h1–h3) of a rendered document, for the sidebar. */
export function extractHeadings(root: ParentNode): Heading[] {
  const elements = [...root.querySelectorAll<HTMLHeadingElement>("h1[id], h2[id], h3[id]")].filter(
    (h) => !h.closest(".markdown-alert, blockquote, .footnotes"),
  );
  const levels = elements.map((h) => Number(h.tagName[1]));
  const top = Math.min(...levels);
  return elements.map((h, i) => ({
    id: h.id,
    text: (h.textContent ?? "").trim(),
    depth: levels[i] - top,
  }));
}

/** The heading the reader is currently in: the last one above the top edge. */
export function activeHeading(root: ParentNode, headings: Heading[], viewportTop: number): string | null {
  let active: string | null = null;
  for (const heading of headings) {
    const el = root.querySelector(`#${CSS.escape(heading.id)}`);
    if (!el) continue;
    if (el.getBoundingClientRect().top - viewportTop > 24) break;
    active = heading.id;
  }
  return active ?? headings[0]?.id ?? null;
}
