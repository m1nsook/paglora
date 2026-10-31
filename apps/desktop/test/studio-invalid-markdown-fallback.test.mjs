import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
const read = path => readFileSync(new URL("../" + path, import.meta.url), "utf8");
test("components no longer consult ProseMirror validity or a string-fingerprint fallback", () => {
  const app = read("src/App.tsx");
  const palette = read("src/components/editor/block-palette.tsx");
  const model = read("src/lib/component-source-model.ts");
  assert.match(app, /const before = latest\.current;/);
  assert.match(app, /insertComponentAtBoundary\(before, kind, boundary\)/);
  assert.doesNotMatch(app, /insertAtSourcePosition|appendComponentToMarkdown/);
  assert.doesNotMatch(palette, /invalidVisualTree|onInvalidDocument|insertBlockIntoEditor/);
  assert.doesNotMatch(model, /textContent|\.state\.doc|nodeDOM/);
});
test("existing documentation templates and published renderer remain source-compatible", () => {
  const templates = read("src/lib/block-examples.ts");
  const canvas = read("src/components/editor/source-component-canvas.tsx");
  assert.match(templates, /\[!BADGES\]/);
  assert.match(templates, /\[!CARD\]/);
  assert.match(templates, /\[!TABS\]/);
  assert.match(canvas, /renderPageContent\(/);
  assert.match(canvas, /dangerouslySetInnerHTML=\{\{ __html: block\.html \}\}/);
});
test("Source edits refresh the component canvas and never serialize a malformed editor tree", () => {
  const app = read("src/App.tsx");
  assert.match(app, /refreshComponents\(n => n \+ 1\)/);
  assert.match(app, /rawSourceFallbackActive\.current = true/);
  assert.match(app, /void persist\(\)\.catch/);
  assert.match(app, /onPageSourceSaved=\{/);
});
