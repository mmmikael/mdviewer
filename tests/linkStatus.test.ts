import { describe, expect, it } from "vitest";

import { linkStatus } from "../src/lib/linkStatus";

const doc = "/docs/guide/intro.md";

describe("linkStatus", () => {
  it("shows external URLs as they are", () => {
    expect(linkStatus({ kind: "external", href: "https://example.com/a?b=1" }, doc)).toBe(
      "https://example.com/a?b=1",
    );
    expect(linkStatus({ kind: "external", href: "mailto:a@b.c" }, doc)).toBe("mailto:a@b.c");
  });

  it("shows in-page anchors as #id", () => {
    expect(linkStatus({ kind: "anchor", href: "#setup", hash: "setup" }, doc)).toBe("#setup");
    expect(linkStatus({ kind: "file", href: "intro.md#x", path: doc, hash: "x" }, doc)).toBe("#x");
  });

  it("shows local files relative to the document's folder when inside it", () => {
    expect(linkStatus({ kind: "file", href: "a/b.md#c", path: "/docs/guide/a/b.md", hash: "c" }, doc)).toBe("a/b.md#c");
    expect(linkStatus({ kind: "file", href: "../x.pdf", path: "/docs/x.pdf", hash: "" }, doc)).toBe("/docs/x.pdf");
  });

  it("says when ⌘-click will open a new window", () => {
    expect(linkStatus({ kind: "file", href: "b.md", path: "/docs/guide/b.md", hash: "" }, doc, true)).toBe(
      "Open “b.md” in a new window",
    );
  });
});
