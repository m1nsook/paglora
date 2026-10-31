import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const runtime = readFileSync(new URL("../src/index.js", import.meta.url), "utf8");
const css = readFileSync(new URL("../../../apps/desktop/src-tauri/src/sites.rs", import.meta.url), "utf8");

function fixture({ height = 2400, viewport = 800, reducedMotion = false, articlePresent = true, translations = null } = {}) {
  const listeners = new Map();
  const nodes = [];
  const focusCalls = [];
  const scrollCalls = [];
  let scroll = 0;
  let articleHeight = height;
  let resizeArticle;
  const contentAttrs = new Map();
  const main = {
    hasAttribute(name) { return contentAttrs.has(name); },
    setAttribute(name, value) { contentAttrs.set(name, value); },
    focus(options) { focusCalls.push(options); },
  };
  const article = {
    getBoundingClientRect() {
      return { top: 180 - scroll, bottom: 180 - scroll + articleHeight, height: articleHeight };
    },
  };
  function element(tag) {
    const attributes = new Map();
    const events = new Map();
    return {
      tag, attributes, events, children: [], hidden: false, style: { transform: "" },
      setAttribute(name, value) { attributes.set(name, value); },
      getAttribute(name) { return attributes.get(name); },
      addEventListener(name, fn) { events.set(name, fn); },
      appendChild(child) { this.children.push(child); },
      click() { events.get("click")?.(); },
    };
  }
  const context = {
    URL,
    location: { href: "https://example.test/docs/index.html" },
    localStorage: { getItem() { return null; } },
    document: {
      documentElement: { dataset: { colorMode: "system" }, clientHeight: viewport, scrollTop: 0 },
      body: { appendChild(node) { nodes.push(node); } },
      getElementById(id) {
        if (id === "content") return main;
        if (id === "paglora-i18n" && translations) return { textContent: JSON.stringify(translations) };
        return null;
      },
      querySelector(selector) {
        if (selector === ".top-bar") return { appendChild() {} };
        if (selector === ".docs") return articlePresent ? article : null;
        return null;
      },
      querySelectorAll() { return []; },
      createElement: element,
    },
    addEventListener(name, fn) { listeners.set(name, fn); },
    requestAnimationFrame(fn) { fn(); },
    innerHeight: viewport,
    scrollY: 0,
    scrollTo(options) { scrollCalls.push(options); },
    matchMedia(query) {
      assert.equal(query, "(prefers-reduced-motion: reduce)");
      return { matches: reducedMotion };
    },
    ResizeObserver: class {
      constructor(fn) { resizeArticle = fn; }
      observe(node) { assert.equal(node, article); }
    },
  };
  vm.runInNewContext(runtime, context);
  const progress = nodes.find(node => node.className === "paglora-reading-progress");
  const back = nodes.find(node => node.className === "paglora-back-to-top");
  return {
    progress, back, focusCalls, scrollCalls, contentAttrs,
    setScroll(value) {
      scroll = value;
      context.scrollY = value;
      context.document.documentElement.scrollTop = value;
      listeners.get("scroll")?.();
    },
    setHeight(value) { articleHeight = value; resizeArticle?.(); },
    setViewport(value) {
      context.innerHeight = value;
      context.document.documentElement.clientHeight = value;
      listeners.get("resize")?.();
    },
  };
}

test("long articles expose accurate bounded reading progress and delayed return control", () => {
  const app = fixture();
  assert.equal(app.progress.hidden, false);
  assert.equal(app.progress.attributes.get("aria-hidden"), "true");
  assert.equal(app.progress.children[0].style.transform, "scaleX(0)");
  assert.equal(app.back.hidden, true);
  app.setScroll(800);
  const fraction = Number(app.progress.children[0].style.transform.slice(7, -1));
  assert.ok(fraction > 0.3 && fraction < 0.6);
  assert.equal(app.back.hidden, false);
  app.setScroll(3000);
  assert.equal(app.progress.children[0].style.transform, "scaleX(1)");
});

test("short articles and resized short content do not show misleading controls", () => {
  const app = fixture({ height: 450 });
  assert.equal(app.progress.hidden, true);
  app.setScroll(900);
  assert.equal(app.back.hidden, true);
  app.setHeight(2400);
  assert.equal(app.progress.hidden, false);
  assert.equal(app.back.hidden, false);
  app.setViewport(2600);
  assert.equal(app.progress.hidden, true);
  assert.equal(app.back.hidden, true);
});

test("return-to-top restores keyboard focus and respects reduced-motion settings", () => {
  for (const reducedMotion of [false, true]) {
    const app = fixture({ reducedMotion, translations: { "site.navigation.backToTop": "Powrót na górę" } });
    app.setScroll(700);
    assert.equal(app.back.attributes.get("aria-label"), "Powrót na górę");
    app.back.click();
    assert.equal(app.contentAttrs.get("tabindex"), "-1");
    assert.equal(app.focusCalls.length, 1);
    assert.equal(app.focusCalls[0].preventScroll, true);
    assert.equal(app.scrollCalls[0].top, 0);
    assert.equal(app.scrollCalls[0].behavior, reducedMotion ? "auto" : "smooth");
  }
});

test("short/script-disabled pages remain ordinary documentation", () => {
  const app = fixture({ articlePresent: false });
  assert.equal(app.progress, undefined);
  assert.equal(app.back, undefined);
});

test("export styles include RTL, print, high-contrast and reduced-motion support", () => {
  assert.match(css, /\.paglora-reading-progress\[hidden\],\.paglora-back-to-top\[hidden\]/);
  assert.match(css, /:root\[dir="rtl"\] \.paglora-reading-progress-fill\{transform-origin:right center\}/);
  assert.match(css, /@media\(forced-colors:active\)\{\.paglora-reading-progress-fill/);
  assert.match(css, /@media\(prefers-reduced-motion:reduce\)\{\.paglora-reading-progress-fill/);
  assert.match(css, /\.paglora-reading-progress,\.paglora-back-to-top\{display:none!important\}/);
});
