import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { createMarkdownRenderer } from "vitepress";

const docs = new URL("../docs/", import.meta.url);
const markdown = await createMarkdownRenderer(fileURLToPath(docs));

for (const page of ["index.md", "en/index.md"]) {
  const html = markdown.render(await readFile(new URL(page, docs), "utf8"));

  for (const section of ["yv-grid", "yv-roadmap"]) {
    test(page + ": " + section + " renders as HTML rather than a code block", () => {
      assert.ok(
        html.includes('<div class="' + section + '">'),
        section + " must render as an HTML element",
      );
      assert.ok(
        !html.includes('&lt;div class=&quot;' + section + '&quot;&gt;'),
        section + " must not display escaped HTML source",
      );
    });
  }
}
