// Keeps the reader's place across live reloads. A saved pixel offset drifts
// whenever text is added or removed above the viewport (an agent ticking boxes
// in a plan, say), so instead we remember the blocks at the top of the view and
// put the first one that survived the edit back where it was.

const BLOCKS = "h1, h2, h3, h4, h5, h6, p, li, pre, blockquote, tr, dt, dd, summary, .mermaid-diagram";
/** Fallbacks in case the block being read is the one that was edited. */
const MAX_ANCHORS = 4;

interface Anchor {
  tag: string;
  text: string;
  /** Which of the blocks with this tag and text it is, to tell repeats apart. */
  occurrence: number;
  /** Distance from the top of the viewport to the top of the block. */
  offset: number;
}

function blocks(root: ParentNode): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(BLOCKS)];
}

function text(el: HTMLElement): string {
  return (el.textContent ?? "").replace(/\s+/g, " ").trim();
}

function matches(all: HTMLElement[], tag: string, content: string): HTMLElement[] {
  return all.filter((el) => el.tagName === tag && text(el) === content);
}

/** Notes the blocks at the top of `scroller`'s viewport. */
export function captureAnchors(root: ParentNode, scroller: HTMLElement): Anchor[] {
  const view = scroller.getBoundingClientRect();
  const all = blocks(root);
  // The block cut by the top edge comes first; nested blocks follow their parent
  // in document order, so the last one cut is the innermost.
  let straddling = -1;
  const inView: number[] = [];
  for (const [i, el] of all.entries()) {
    const rect = el.getBoundingClientRect();
    if (rect.top < view.top) {
      if (rect.bottom > view.top) straddling = i;
    } else if (rect.top < view.bottom) {
      inView.push(i);
      if (inView.length >= MAX_ANCHORS) break;
    } else {
      break;
    }
  }
  const indices = straddling >= 0 ? [straddling, ...inView] : inView;
  return indices
    .map((index) => {
      const el = all[index];
      const content = text(el);
      return {
        tag: el.tagName,
        text: content,
        occurrence: matches(all.slice(0, index), el.tagName, content).length,
        offset: el.getBoundingClientRect().top - view.top,
      };
    })
    .filter((anchor) => anchor.text !== "");
}

/**
 * Scrolls so the first anchor still in the document is back at its old offset.
 * Returns false if none of them survived, leaving the scroll position alone.
 */
export function restoreAnchors(root: ParentNode, scroller: HTMLElement, anchors: Anchor[]): boolean {
  const all = blocks(root);
  for (const anchor of anchors) {
    const found = matches(all, anchor.tag, anchor.text);
    const el = found[Math.min(anchor.occurrence, found.length - 1)];
    if (!el) continue;
    const top = el.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
    scroller.scrollTop += top - anchor.offset;
    return true;
  }
  return false;
}
