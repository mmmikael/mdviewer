import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { captureAnchors, restoreAnchors } from "../src/lib/scrollAnchor";

// jsdom has no layout, so fake one: every block is 100px tall and stacked in
// document order, and the scroller is a 300px window onto them.
const LINE = 100;
let scroller: HTMLElement;
let article: HTMLElement;

function layout(el: Element): DOMRect {
  if (el === scroller) return new DOMRect(0, 0, 800, 300);
  const index = [...article.children].indexOf(el);
  const top = index * LINE - scroller.scrollTop;
  return new DOMRect(0, top, 800, LINE);
}

function show(html: string) {
  article.innerHTML = html;
}

beforeEach(() => {
  scroller = document.createElement("div");
  article = document.createElement("article");
  scroller.append(article);
  document.body.append(scroller);
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
    return layout(this);
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  scroller.remove();
});

const paragraphs = (texts: string[]) => texts.map((t) => `<p>${t}</p>`).join("");

describe("scroll anchors", () => {
  it("keeps the same paragraph at the top when text is added above it", () => {
    show(paragraphs(["a", "b", "c", "d", "e"]));
    scroller.scrollTop = 250; // halfway through "c"
    const anchors = captureAnchors(article, scroller);
    show(paragraphs(["new 1", "new 2", "a", "b", "c", "d", "e"]));
    expect(restoreAnchors(article, scroller, anchors)).toBe(true);
    expect(scroller.scrollTop).toBe(450);
  });

  it("keeps it when text is removed above it", () => {
    show(paragraphs(["a", "b", "c", "d", "e"]));
    scroller.scrollTop = 300;
    const anchors = captureAnchors(article, scroller);
    show(paragraphs(["c", "d", "e"]));
    restoreAnchors(article, scroller, anchors);
    expect(scroller.scrollTop).toBe(100);
  });

  it("falls back to the next block when the one being read was edited", () => {
    show(paragraphs(["a", "b", "c", "d", "e"]));
    scroller.scrollTop = 250;
    const anchors = captureAnchors(article, scroller);
    show(paragraphs(["new", "a", "b", "c (edited)", "d", "e"]));
    restoreAnchors(article, scroller, anchors);
    expect(scroller.scrollTop).toBe(350); // "d" stays 50px below the top
  });

  it("tells identical blocks apart", () => {
    show(paragraphs(["TODO", "x", "TODO", "y", "TODO", "z"]));
    scroller.scrollTop = 400; // the third "TODO"
    const anchors = captureAnchors(article, scroller);
    show(paragraphs(["new", "TODO", "x", "TODO", "y", "TODO", "z"]));
    restoreAnchors(article, scroller, anchors);
    expect(scroller.scrollTop).toBe(500);
  });

  it("reports when nothing survived", () => {
    show(paragraphs(["a", "b", "c"]));
    scroller.scrollTop = 100;
    const anchors = captureAnchors(article, scroller);
    show(paragraphs(["x", "y", "z"]));
    expect(restoreAnchors(article, scroller, anchors)).toBe(false);
    expect(scroller.scrollTop).toBe(100);
  });
});
