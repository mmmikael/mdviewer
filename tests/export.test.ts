import { describe, expect, it } from "vitest";

import { clipboardHtml, standaloneHtml } from "../src/lib/export";
import { prepareHighlighting } from "../src/lib/markdown/highlight";
import { buildFragment } from "../src/lib/markdown/postprocess";
import { renderMarkdown } from "../src/lib/markdown/render";

const SOURCE = [
  "# Title",
  "",
  "[other](other.md#part) [site](https://example.com) [top](#title)",
  "",
  "![missing](images/missing.png)",
  "",
  "Inline $x^2$ and block:",
  "",
  "$$",
  "\\frac{1}{2}",
  "$$",
  "",
  "```rust",
  "fn main() {}",
  "```",
].join("\n");

async function rendered(): Promise<HTMLElement> {
  await prepareHighlighting(SOURCE);
  const article = document.createElement("article");
  article.append(
    buildFragment(renderMarkdown(SOURCE), {
      baseDir: "/Users/someone/docs",
      assetUrl: (path) => `asset://localhost/${encodeURIComponent(path)}`,
      blockRemoteImages: false,
    }),
  );
  return article;
}

describe("standaloneHtml", () => {
  it("is a complete page without app-only markup or local paths", async () => {
    const html = await standaloneHtml(await rendered(), "Notes <draft>");
    expect(html.startsWith("<!doctype html>")).toBe(true);
    expect(html).toContain("<title>Notes &lt;draft&gt;</title>");
    expect(html).not.toContain("/Users/someone");
    expect(html).not.toContain("asset:");
    expect(html).not.toContain('class="copy-button"');
    expect(html).not.toContain("data-link");
    expect(html).toContain('href="other.md#part"');
    expect(html).toContain('<img alt="missing">');
  });
});

describe("clipboardHtml", () => {
  it("inlines code colours, turns math into TeX and keeps links relative", async () => {
    const html = await clipboardHtml(await rendered());
    const root = document.createElement("div");
    root.innerHTML = html;

    expect(html).not.toContain("--shiki");
    expect(root.querySelector("pre span[style*='color']")).not.toBeNull();
    expect(root.querySelector(".katex")).toBeNull();
    expect(root.querySelector("code")?.textContent).toBe("$x^2$");
    expect([...root.querySelectorAll("pre")].some((pre) => pre.textContent === "$$\n\\frac{1}{2}\n$$")).toBe(true);
    expect(root.querySelector("p > pre")).toBeNull();

    const hrefs = [...root.querySelectorAll("a")].map((a) => a.getAttribute("href"));
    expect(hrefs).toEqual(["other.md#part", "https://example.com", null]);
    expect(html).not.toContain("/Users/someone");
  });
});
