import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const source = readFileSync(new URL("../src/index.js", import.meta.url), "utf8");

function node(tag = "div") {
  const attrs = new Map();
  const classes = new Set();
  return {
    tag, attrs, children: [], parentElement: null, listeners: {},
    hidden: false, id: "", textContent: "", focused: false, scrollCount: 0, top: 80,
    classList: { add(value) { classes.add(value); } },
    setAttribute(key, value) { attrs.set(key, value); },
    getAttribute(key) { return attrs.get(key) ?? null; },
    removeAttribute(key) { attrs.delete(key); },
    appendChild(child) { child.parentElement = this; this.children.push(child); },
    before(child) {
      const parent = this.parentElement;
      child.parentElement = parent;
      parent.children.splice(parent.children.indexOf(this), 0, child);
    },
    addEventListener(event, fn) { this.listeners[event] = fn; },
    focus() { this.focused = true; },
    scrollIntoView() { this.scrollCount++; },
    getBoundingClientRect() { return { top: this.top }; },
    closest(selector) {
      assert.equal(selector, "[hidden]");
      for (let current = this; current; current = current.parentElement) {
        if (current.hidden) return current;
      }
      return null;
    },
  };
}

function run({ hash = "", nested = false, withToc = false } = {}) {
  const headings = new Map();
  const events = new Map();
  const storage = new Map();
  const location = { href: "https://example.test/docs/index.html", hash };
  function makeGroup(kind, ids, labels) {
    const group = node("section");
    const container = node();
    const panels = ids.map((id, index) => {
      const panel = node();
      const label = node("span");
      label.textContent = labels[index];
      panel.querySelector = selector => selector ===
        (kind === "code" ? ".paglora-code-group-label" : ".paglora-tabs-label") ? label : null;
      const heading = node("h3");
      heading.id = id;
      heading.top = index ? 100 : 30;
      panel.appendChild(heading);
      container.appendChild(panel);
      headings.set(id, heading);
      return panel;
    });
    group.appendChild(container);
    group.setAttribute("aria-label", "Examples");
    group.querySelector = selector => selector ===
      (kind === "code" ? ".paglora-code-group-panels" : ".paglora-tabs-panels") ? container : null;
    group.querySelectorAll = selector => selector ===
      (kind === "code" ? ".paglora-code-group-item" : ".paglora-tabs-item") ? panels : [];
    return { group, panels, get tabs() { return group.children[0].children; } };
  }
  const code = makeGroup("code", ["bash", "powershell"], ["Bash", "PowerShell"]);
  const content = makeGroup("content", ["windows", "macos"], ["Windows", "macOS"]);
  if (nested) code.panels[1].appendChild(content.group);
  const links = withToc ? ["macos", "windows"].map(id => {
    const link = node("a");
    link.setAttribute("href", "#" + id);
    return link;
  }) : [];
  const outline = withToc ? { querySelectorAll() { return links; } } : null;
  const document = {
    documentElement: { dataset: { colorMode: "light" }, dir: "ltr" },
    querySelector(selector) {
      if (selector === ".top-bar") return { appendChild() {} };
      if (selector === ".toc-inner") return outline;
      return null;
    },
    querySelectorAll(selector) {
      if (selector === ".docs .paglora-code-group") return [code.group];
      if (selector === ".docs .paglora-tabs") return [content.group];
      return [];
    },
    getElementById(id) { return headings.get(id) ?? null; },
    createElement: node,
  };
  vm.runInNewContext(source, {
    document, location, URL,
    localStorage: {
      getItem() { return null; },
      setItem(key, value) { storage.set(key, value); },
    },
    addEventListener(name, fn) {
      const handlers = events.get(name) ?? [];
      handlers.push(fn);
      events.set(name, handlers);
    },
    requestAnimationFrame(fn) { fn(); },
  });
  return {
    code, content, headings, links, location, storage,
    navigate(hash) {
      location.hash = hash;
      for (const fn of events.get("hashchange") ?? []) fn();
    },
  };
}

test("initial permalink reveals a hidden content tab without stealing focus", () => {
  const app = run({ hash: "#macos" });
  assert.equal(app.content.panels[0].hidden, true);
  assert.equal(app.content.panels[1].hidden, false);
  assert.equal(app.content.tabs[1].attrs.get("aria-selected"), "true");
  assert.equal(app.headings.get("macos").scrollCount, 1);
  assert.equal(app.content.tabs[1].focused, false);
});

test("hash changes reveal code and content panels without saving a language", () => {
  const app = run();
  app.navigate("#powershell");
  assert.equal(app.code.panels[1].hidden, false);
  assert.equal(app.code.panels[0].hidden, true);
  assert.equal(app.storage.size, 0);
  app.navigate("#macos");
  assert.equal(app.content.panels[1].hidden, false);
  assert.equal(app.headings.get("macos").scrollCount, 1);
  app.navigate("#macos");
  assert.equal(app.headings.get("macos").scrollCount, 1, "visible targets do not jump again");
});

test("nested tab fragments reveal the outer code panel before inner content", () => {
  const app = run({ hash: "#macos", nested: true });
  assert.equal(app.code.panels[0].hidden, true);
  assert.equal(app.code.panels[1].hidden, false);
  assert.equal(app.content.panels[0].hidden, true);
  assert.equal(app.content.panels[1].hidden, false);
  assert.equal(app.headings.get("macos").scrollCount, 1);
});

test("malformed, missing, and oversized hashes cannot change tabs", () => {
  const app = run({ hash: "#%ZZ" });
  for (const hash of ["#unknown", "#" + "x".repeat(2100), "#%ZZ"]) app.navigate(hash);
  assert.equal(app.content.panels[0].hidden, false);
  assert.equal(app.content.panels[1].hidden, true);
  assert.equal(app.headings.get("macos").scrollCount, 0);
});

test("TOC active marker ignores a closer heading inside a hidden tab", () => {
  const app = run({ withToc: true });
  assert.equal(app.content.panels[1].hidden, true);
  assert.equal(app.links[0].attrs.has("aria-current"), false, "hidden macOS heading is skipped");
  assert.equal(app.links[1].attrs.get("aria-current"), "location");
  app.navigate("#macos");
  assert.equal(app.links[0].attrs.get("aria-current"), "location", "revealed heading becomes active");
  assert.equal(app.links[1].attrs.has("aria-current"), false);
});
