// Find in page using the CSS Custom Highlight API, which marks text ranges
// without touching the DOM (so it survives live reloads and doesn't break
// selection or links).

const ALL = "find-match";
const CURRENT = "find-current";

export class Finder {
  private ranges: Range[] = [];
  private index = -1;
  query = "";

  constructor(
    private readonly root: HTMLElement,
    private readonly scroller: HTMLElement,
  ) {}

  static get supported(): boolean {
    return typeof CSS !== "undefined" && "highlights" in CSS;
  }

  get count(): number {
    return this.ranges.length;
  }

  /** 1-based position of the current match, 0 when there is none. */
  get position(): number {
    return this.index + 1;
  }

  search(query: string): void {
    this.query = query;
    this.ranges = query ? findRanges(this.root, query) : [];
    this.index = this.ranges.length ? this.firstVisible() : -1;
    this.paint(true);
  }

  /** Re-runs the current query after the document changed, keeping the position. */
  refresh(): void {
    if (!this.query) return;
    const previous = this.index;
    this.ranges = findRanges(this.root, this.query);
    this.index = this.ranges.length ? Math.min(Math.max(previous, 0), this.ranges.length - 1) : -1;
    this.paint(false);
  }

  next(): void {
    this.step(1);
  }

  previous(): void {
    this.step(-1);
  }

  clear(): void {
    this.query = "";
    this.ranges = [];
    this.index = -1;
    if (Finder.supported) {
      CSS.highlights.delete(ALL);
      CSS.highlights.delete(CURRENT);
    }
  }

  private step(direction: 1 | -1): void {
    if (!this.ranges.length) return;
    this.index = (this.index + direction + this.ranges.length) % this.ranges.length;
    this.paint(true);
  }

  private firstVisible(): number {
    const top = this.scroller.getBoundingClientRect().top;
    const i = this.ranges.findIndex((r) => r.getBoundingClientRect().top >= top);
    return i === -1 ? 0 : i;
  }

  private paint(scroll: boolean): void {
    if (!Finder.supported) return;
    CSS.highlights.set(ALL, new Highlight(...this.ranges));
    const current = this.ranges[this.index];
    if (!current) {
      CSS.highlights.delete(CURRENT);
      return;
    }
    CSS.highlights.set(CURRENT, new Highlight(current));
    if (scroll) this.reveal(current);
  }

  private reveal(range: Range): void {
    const rect = range.getBoundingClientRect();
    const view = this.scroller.getBoundingClientRect();
    const margin = 80;
    if (rect.top < view.top + margin || rect.bottom > view.bottom - margin) {
      this.scroller.scrollBy({ top: rect.top - view.top - view.height / 3 });
    }
  }
}

export function findRanges(root: Node, query: string): Range[] {
  const needle = query.toLocaleLowerCase();
  const ranges: Range[] = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) =>
      node.parentElement?.closest("svg, .katex-mathml, button")
        ? NodeFilter.FILTER_REJECT
        : NodeFilter.FILTER_ACCEPT,
  });
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = (node.nodeValue ?? "").toLocaleLowerCase();
    let at = text.indexOf(needle);
    while (at !== -1) {
      const range = document.createRange();
      range.setStart(node, at);
      range.setEnd(node, at + needle.length);
      ranges.push(range);
      at = text.indexOf(needle, at + needle.length);
    }
  }
  return ranges;
}
