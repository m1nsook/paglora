import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import test from "node:test";
import ts from "typescript";
import { marked } from "marked";
const require = createRequire(import.meta.url);
const read = path => readFileSync(new URL("../" + path, import.meta.url), "utf8");
function load(source, resolver = require) {
  const exports = {};
  const compiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText;
  new Function("require", "exports", compiled)(resolver, exports);
  return exports;
}
const examples = load(read("src/lib/block-examples.ts"));
const model = load(read("src/lib/component-source-model.ts"), id => {
  if (id === "@/lib/block-examples") return examples;
  if (id === "marked") return { marked };
  return require(id);
});
const source = "# Overview\n\nWelcome to the documentation.\n\n## Installation\n\nInstall Bun.\n";
const blocks = model.parseSourceComponents(source).blocks;

test("real Markdown tokens, not fake ProseMirror nodes, own all editor positions", () => {
  assert.equal(blocks.length, 4);
  assert.deepEqual(blocks.map(b => source.slice(b.start,b.end).trim()), [
    "# Overview", "Welcome to the documentation.", "## Installation", "Install Bun.",
  ]);
  assert.deepEqual(blocks.map(b => b.markdownType), ["heading","paragraph","heading","paragraph"]);
  assert.equal(blocks[0].start, 0);
  assert.equal(source.slice(blocks[2].start,blocks[2].end).trim(), "## Installation");
});
test("all 20 components insert exactly between the selected source blocks", () => {
  assert.equal(Object.keys(examples.BLOCK_MARKDOWN_EXAMPLES).length, 20);
  for (const [kind, snippet] of Object.entries(examples.BLOCK_MARKDOWN_EXAMPLES)) {
    const actual = model.insertComponentAtBoundary(source, kind, 2);
    const expected = "# Overview\n\nWelcome to the documentation.\n\n" +
      snippet + "\n\n## Installation\n\nInstall Bun.\n";
    assert.equal(actual, expected, kind);
    assert.ok(actual.indexOf(snippet) < actual.indexOf("## Installation"), kind);
    assert.equal(model.parseSourceComponents(actual).blocks.length, 5, kind);
  }
});
test("top, before a heading, and bottom insertions respect exact source offsets", () => {
  assert.equal(model.insertComponentAtBoundary(source,"paragraph",0),
    "Write something...\n\n" + source);
  assert.equal(model.insertComponentAtBoundary(source,"paragraph",2),
    "# Overview\n\nWelcome to the documentation.\n\nWrite something...\n\n## Installation\n\nInstall Bun.\n");
  assert.equal(model.insertComponentAtBoundary(source,"paragraph",4),
    source + "\nWrite something...\n");
});
test("existing Paglora badges, cards, warning and nested tabs remain unchanged", () => {
  const documented = "# Components\n\n> [!BADGES] Release status\n>\n> - [success] Stable\n> - [warning] Preview\n\n" +
    "> [!CARD] Existing card\n>\n> Preserve this card.\n\n" +
    "> [!TABS] Platform\n>\n> > [!TAB] Windows\n> >\n> > Bun\n\n## Next\n";
  const original = model.parseSourceComponents(documented);
  assert.equal(original.blocks.length, 5);
  const next = model.insertComponentAtBoundary(documented,"note",3);
  const offset = original.blocks[3].start;
  assert.ok(next.startsWith(documented.slice(0, offset)));
  assert.ok(next.endsWith(documented.slice(offset)));
  const inserted = next.slice(offset, next.length - (documented.length - offset));
  assert.match(inserted, /> \[!NOTE\] Note/);
  for (const marker of ["[!BADGES]", "[!CARD]", "[!TABS]", "[!TAB]"]) {
    assert.equal(next.split(marker).length, documented.split(marker).length, marker);
  }
});
test("CRLF, Unicode, inline HTML and original formatting are never rewritten", () => {
  const doc = "# 漢字 😀\r\n\r\nBefore <em>HTML</em>\r\n\r\nAfter\r\n";
  const result = model.insertComponentAtBoundary(doc,"orderedList",2);
  assert.equal(result,
    "# 漢字 😀\r\n\r\nBefore <em>HTML</em>\r\n\r\n1. First step\r\n\r\nAfter\r\n");
  assert.equal(model.parseSourceComponents(result).blocks.length,4);
});
test("empty docs work and invalid boundary or component cannot mutate the source", () => {
  assert.equal(model.insertComponentAtBoundary("","paragraph",0), "Write something...\n");
  assert.throws(() => model.insertComponentAtBoundary(source, "note", 99), /position is no longer valid/i);
  assert.throws(() => model.insertComponentAtBoundary(source, "invalid", 2), /unknown documentation component/i);
});
test("Studio composer uses the SAME real source boundaries for canvas and insertion", () => {
  const app = read("src/App.tsx");
  const palette = read("src/components/editor/block-palette.tsx");
  const canvas = read("src/components/editor/source-component-canvas.tsx");
  assert.match(app, /const before = latest\.current;/);
  assert.match(app, /insertComponentAtBoundary\(before, kind, boundary\)/);
  assert.match(app, /SourceComponentCanvas source=\{latest\.current\}/);
  assert.match(app, /onInsert=\{insertSourceComponent\}/);
  assert.match(canvas, /parseSourceComponents\(source\)\.blocks/);
  assert.match(canvas, /data-component-position=\{boundary\}/);
  assert.match(palette, /data-component-block-index/);
  assert.doesNotMatch(app, /insertAtSourcePosition|insertComponentAtMarkdownBoundary/);
  assert.doesNotMatch(palette, /state\.doc|insertBlockIntoEditor/);
});
