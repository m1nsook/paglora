import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import test from "node:test";
import ts from "typescript";
import { marked } from "marked";

const require = createRequire(import.meta.url);
const read = path => readFileSync(new URL("../" + path, import.meta.url), "utf8");
function load(path, resolver = require) {
  const exports = {};
  const code = ts.transpileModule(read(path), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText;
  new Function("require", "exports", code)(resolver, exports);
  return exports;
}
const history = load("src/lib/component-edit-history.ts");
const examples = load("src/lib/block-examples.ts");
const model = load("src/lib/component-source-model.ts", id => {
  if (id === "@/lib/block-examples") return examples;
  if (id === "marked") return { marked };
  return require(id);
});

test("every source-backed component insertion undoes and redoes exact Markdown", () => {
  const original = "# 시작 😀\r\n\r\nParagraph <em>raw</em>\r\n\r\n## Next\r\n";
  for (const kind of Object.keys(examples.BLOCK_MARKDOWN_EXAMPLES)) {
    const inserted = model.insertComponentAtBoundary(original, kind, 2);
    const first = history.recordComponentEdit(history.createComponentEditHistory(original), original, inserted);
    assert.equal(first.current, inserted);
    const undone = history.undoComponentEdit(first, inserted);
    assert.equal(undone.current, original, kind);
    assert.equal(undone.undo.length, 0);
    const redone = history.redoComponentEdit(undone, original);
    assert.equal(redone.current, inserted, kind);
    assert.equal(redone.redo.length, 0);
  }
});

test("multiple insertions are separate undo steps and redo follows reverse order", () => {
  const original = "# A\n\nFirst\n\n## B\n";
  const one = model.insertComponentAtBoundary(original, "note", 2);
  const two = model.insertComponentAtBoundary(one, "warning", 3);
  let state = history.createComponentEditHistory(original);
  state = history.recordComponentEdit(state, original, one);
  state = history.recordComponentEdit(state, one, two);
  assert.equal(state.undo.length, 2);
  state = history.undoComponentEdit(state, two);
  assert.equal(state.current, one);
  state = history.undoComponentEdit(state, one);
  assert.equal(state.current, original);
  state = history.redoComponentEdit(state, original);
  assert.equal(state.current, one);
  state = history.redoComponentEdit(state, one);
  assert.equal(state.current, two);
});

test("a new insertion after Undo discards the redo branch", () => {
  const original = "A\n";
  let state = history.recordComponentEdit(history.createComponentEditHistory(original), original, "A\nB\n");
  state = history.undoComponentEdit(state, "A\nB\n");
  state = history.recordComponentEdit(state, original, "A\nC\n");
  assert.equal(state.redo.length, 0);
  assert.equal(history.redoComponentEdit(state, "A\nC\n"), null);
});

test("stale Write/Source changes are never undone by component history", () => {
  const original = "# Original\n";
  const inserted = "# Original\n\nNew\n";
  const state = history.recordComponentEdit(history.createComponentEditHistory(original), original, inserted);
  assert.equal(history.undoComponentEdit(state, "# Externally edited\n"), null);
  assert.equal(history.redoComponentEdit(state, "# Externally edited\n"), null);
  const fresh = history.recordComponentEdit(state, "# Externally edited\n", "# Externally edited\n\nNew\n");
  assert.equal(fresh.undo.length, 1);
  assert.equal(history.undoComponentEdit(fresh, fresh.current).current, "# Externally edited\n");
});

test("history bounds both step count and retained source characters", () => {
  let state = history.createComponentEditHistory("0");
  for (let i = 1; i <= 70; i++) state = history.recordComponentEdit(state, state.current, String(i));
  assert.equal(state.undo.length, history.MAX_COMPONENT_UNDO_STEPS);
  const huge = "x".repeat(history.MAX_COMPONENT_HISTORY_CHARS + 1);
  state = history.recordComponentEdit(history.createComponentEditHistory(huge), huge, huge + "y");
  assert.equal(state.undo.length, 0);
  assert.equal(history.undoComponentEdit(state, state.current), null);
});

test("the composer uses source snapshots, scoped shortcuts and explicit controls", () => {
  const app = read("src/App.tsx");
  const canvas = read("src/components/editor/source-component-canvas.tsx");
  assert.match(app, /recordComponentEdit\(componentHistory\.current, before, nextSource\)/);
  assert.match(app, /undoComponentEdit\(componentHistory\.current, latest\.current\)/);
  assert.match(app, /redoComponentEdit\(componentHistory\.current, latest\.current\)/);
  assert.match(app, /componentHistory\.current = createComponentEditHistory\(latest\.current\)/);
  assert.match(app, /componentHistory\.current = createComponentEditHistory\(markdown\)/);
  assert.match(app, /target\.closest\('input, textarea, select,/);
  assert.match(app, /event\.preventDefault\(\)/);
  assert.match(canvas, /aria-label="Undo component change"/);
  assert.match(canvas, /aria-label="Redo component change"/);
  assert.match(canvas, /disabled=\{!canUndo\}/);
  assert.match(canvas, /disabled=\{!canRedo\}/);
});
