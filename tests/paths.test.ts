import { describe, expect, it } from "vitest";

import { basename, dirname, hasScheme, isMarkdownPath, normalize, resolveLocal, splitHash } from "../src/lib/paths";

describe("paths", () => {
  it("normalizes . and .. segments", () => {
    expect(normalize("/a/b/../c/./d.md")).toBe("/a/c/d.md");
    expect(normalize("/../x")).toBe("/x");
    expect(normalize("a/../../b")).toBe("../b");
  });

  it("resolves references relative to the document folder", () => {
    expect(resolveLocal("/docs/guide", "img/a.png")).toBe("/docs/guide/img/a.png");
    expect(resolveLocal("/docs/guide", "../assets/a%20b.png")).toBe("/docs/assets/a b.png");
    expect(resolveLocal("/docs/guide", "/abs/x.md")).toBe("/abs/x.md");
    expect(resolveLocal("/docs", "file:///Users/me/notes.md")).toBe("/Users/me/notes.md");
  });

  it("splits hashes and drops query strings", () => {
    expect(splitHash("other.md?raw=1#section")).toEqual(["other.md", "section"]);
    expect(splitHash("#top")).toEqual(["", "top"]);
    expect(splitHash("plain.md")).toEqual(["plain.md", ""]);
  });

  it("recognises Markdown files and URL schemes", () => {
    expect(isMarkdownPath("/a/README.md")).toBe(true);
    expect(isMarkdownPath("/a/notes.MARKDOWN")).toBe(true);
    expect(isMarkdownPath("/a/image.png")).toBe(false);
    expect(hasScheme("https://x.y")).toBe(true);
    expect(hasScheme("mailto:a@b.c")).toBe(true);
    expect(hasScheme("docs/a.md")).toBe(false);
  });

  it("splits folders and names", () => {
    expect(dirname("/a/b/c.md")).toBe("/a/b");
    expect(dirname("/c.md")).toBe("/");
    expect(basename("/a/b/c.md")).toBe("c.md");
  });
});
