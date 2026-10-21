import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import { fenceLanguages, prepareHighlighting } from "../src/lib/markdown/highlight";
import { buildFragment } from "../src/lib/markdown/postprocess";
import { renderMarkdown, slugify } from "../src/lib/markdown/render";
import { findRanges } from "../src/lib/find";
import { extractHeadings } from "../src/lib/toc";

const fixture = (name: string) => readFileSync(resolve("tests/fixtures", name), "utf8");

const options = {
  baseDir: "/docs",
  assetUrl: (path: string) => `asset://localhost${path}`,
  blockRemoteImages: false,
};

function toElement(markdown: string, overrides: Partial<typeof options> = {}): HTMLElement {
  const root = document.createElement("article");
  root.append(buildFragment(renderMarkdown(markdown), { ...options, ...overrides }));
  return root;
}

describe("renderMarkdown", () => {
  it("creates GitHub-style heading ids", () => {
    expect(slugify("Hello, World!")).toBe("hello-world");
    expect(slugify("  Ünïcode — ok ")).toBe("ünïcode--ok");
    const html = renderMarkdown("# Intro\n\n## Intro\n");
    expect(html).toContain('id="intro"');
    expect(html).toContain('id="intro-1"');
  });

  it("renders GFM tables, task lists, footnotes and alerts", () => {
    const html = renderMarkdown(
      "| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n- [ ] todo\n\nText[^n]\n\n[^n]: Note\n\n> [!NOTE]\n> Hi\n",
    );
    expect(html).toContain("<table>");
    expect(html).toMatch(/<input[^>]*checked[^>]*disabled|<input[^>]*disabled[^>]*checked/);
    expect(html).toContain('class="footnotes"');
    expect(html).toContain("markdown-alert-note");
  });

  it("renders math with KaTeX, including ```math fences", () => {
    expect(renderMarkdown("$x^2$")).toContain('class="katex"');
    expect(renderMarkdown("```math\n\\frac{1}{2}\n```")).toContain("katex-display");
  });

  it("emits Mermaid source for later rendering", () => {
    const html = renderMarkdown("```mermaid\ngraph TD; A-->B\n```");
    expect(html).toContain('<pre class="mermaid-source">graph TD; A--&gt;B\n</pre>');
  });

  it("shows front matter collapsed above the document", () => {
    const html = renderMarkdown("---\ntitle: x\n---\n# Doc\n");
    expect(html.startsWith('<details class="front-matter">')).toBe(true);
    expect(html).toContain("title: x");
    expect(renderMarkdown("# No front matter")).not.toContain('class="front-matter"');
  });

  it("highlights code once the grammar is loaded", async () => {
    const source = "```rust\nfn main() {}\n```\n";
    expect(fenceLanguages(source)).toEqual(["rust"]);
    await prepareHighlighting(source);
    const html = renderMarkdown(source);
    expect(html).toContain("shiki");
    expect(html).toContain('data-lang="rust"');
    expect(html).toContain("--shiki-dark");
  });

  it("falls back to plain code for unknown languages", async () => {
    const source = "```notalanguage\nhello\n```\n";
    await prepareHighlighting(source);
    expect(renderMarkdown(source)).toContain('<code class="language-notalanguage">hello');
  });
});

describe("buildFragment", () => {
  it("removes scripts and event handlers", () => {
    const root = toElement('<img src="x" onerror="alert(1)"><script>alert(2)</script><a href="javascript:alert(3)">x</a>');
    expect(root.querySelector("script")).toBeNull();
    expect(root.querySelector("img")!.getAttribute("onerror")).toBeNull();
    expect(root.querySelector("a")!.getAttribute("href")).toBeNull();
  });

  it("keeps KaTeX's TeX annotation", () => {
    const root = toElement("$x^2$");
    expect(root.querySelector("annotation[encoding='application/x-tex']")?.textContent).toBe("x^2");
  });

  it("points local images at the asset protocol", () => {
    const root = toElement("![a](img/a.png) ![b](https://example.com/b.png) ![c](data:image/png;base64,AA==)");
    const [local, remote, inline] = root.querySelectorAll("img");
    expect(local.getAttribute("src")).toBe("asset://localhost/docs/img/a.png");
    expect(local.dataset.localPath).toBe("/docs/img/a.png");
    expect(remote.getAttribute("src")).toBe("https://example.com/b.png");
    expect(inline.getAttribute("src")).toMatch(/^data:/);
  });

  it("rewrites <picture> sources", () => {
    const root = toElement('<picture><source srcset="dark.png 2x" media="(prefers-color-scheme: dark)"><img src="light.png"></picture>');
    expect(root.querySelector("source")!.getAttribute("srcset")).toBe("asset://localhost/docs/dark.png 2x");
  });

  it("can block remote images", () => {
    const root = toElement("![b](https://example.com/b.png)", { blockRemoteImages: true });
    const img = root.querySelector("img")!;
    expect(img.hasAttribute("src")).toBe(false);
    expect(img.classList.contains("blocked-image")).toBe(true);
  });

  it("classifies links", () => {
    const root = toElement(
      "[a](#x) [b](https://example.com) [c](other.md#part) [d](../up/file.pdf) [e](mailto:a@b.c) [f](ftp://x)",
    );
    const links = [...root.querySelectorAll("a")].map((a) => ({ ...a.dataset }));
    expect(links).toEqual([
      { link: "anchor", hash: "x" },
      { link: "external" },
      { link: "file", path: "/docs/other.md", hash: "part" },
      { link: "file", path: "/up/file.pdf", hash: "" },
      { link: "external" },
      { link: "unsupported" },
    ]);
  });

  it("wraps code blocks with a copy button, but not diagrams", () => {
    const root = toElement("```js\nlet a\n```\n\n```mermaid\ngraph TD; A-->B\n```");
    expect(root.querySelectorAll(".code-block")).toHaveLength(1);
    expect(root.querySelector(".code-block .copy-button")).not.toBeNull();
    expect(root.querySelector("pre.mermaid-source")!.closest(".code-block")).toBeNull();
  });

  it("renders the rich fixture end to end", async () => {
    const source = fixture("rich.md");
    await prepareHighlighting(source);
    const root = toElement(source);
    expect(root.querySelector("h1")!.id).toBe("rich-content");
    expect(root.querySelectorAll("pre.shiki")).toHaveLength(2);
    expect(root.querySelector("script")).toBeNull();
    expect(root.querySelector(".markdown-alert-warning")).not.toBeNull();
  });
});

describe("table of contents and find", () => {
  it("extracts h1–h3 headings with relative depth", () => {
    const root = toElement("## A\n### B\n#### C\n## D\n");
    expect(extractHeadings(root)).toEqual([
      { id: "a", text: "A", depth: 0 },
      { id: "b", text: "B", depth: 1 },
      { id: "d", text: "D", depth: 0 },
    ]);
  });

  it("finds case-insensitive matches across text nodes", () => {
    const root = toElement("Hello **hello** HELLO, and `hello()`");
    document.body.append(root);
    expect(findRanges(root, "hello")).toHaveLength(4);
    expect(findRanges(root, "missing")).toHaveLength(0);
    root.remove();
  });
});
