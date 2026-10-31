/*
 * Paglora site runtime. MIT licensed; see repository LICENSE.
 * Progressive enhancement for generated HTML, inspired by VitePress's
 * approach to navigation/appearance. No VitePress source is copied here.
 */
(() => {
  "use strict";
  const root = document.documentElement;
  const header = document.querySelector(".top-bar");
  if (!root || !header) return;

  // The export embeds reviewed translation strings as JSON, not executable code.
  let translations = {};
  try {
    translations = JSON.parse(document.getElementById?.("paglora-i18n")?.textContent || "{}");
  } catch { /* keep English defaults when the dictionary is unavailable */ }
  const tr = (key, english, args = {}) => {
    const value = typeof translations[key] === "string" ? translations[key] : english;
    return value.replace(/\{([a-z][a-zA-Z0-9_]*)\}/g, (match, name) =>
      Object.prototype.hasOwnProperty.call(args, name) ? String(args[name]) : match);
  };

  const modes = ["light", "dark", "system"];
  const configured = modes.includes(root.dataset.colorMode)
    ? root.dataset.colorMode : "system";
  // Prefix by deployed site root so independent Paglora sites do not share
  // preferences when hosted under the same origin at different paths.
  const siteRoot = new URL(".", location.href).pathname;
  const storageKey = "paglora:theme:" + siteRoot;
  // Code examples can share a preferred language within this exported site.
  // Content tabs remain independent: their labels may describe unrelated topics.
  const codePreferenceKey = "paglora:code-language:" + siteRoot;
  const codeGroups = [];
  // Fragment navigation can point inside a hidden code or content tab.
  const fragmentPanels = new Map();
  function codeLabel(value) {
    if (typeof value !== "string" || value.length > 100) return null;
    const normalized = value.trim().normalize("NFKC").toLocaleLowerCase();
    return normalized && normalized.length <= 80 && !/[\u0000-\u001f\u007f]/u.test(normalized)
      ? normalized : null;
  }
  let preferredCodeLabel = null;
  try { preferredCodeLabel = codeLabel(localStorage.getItem(codePreferenceKey)); }
  catch { /* preferences are optional when storage is blocked */ }
  function updateCodePreference(value, persist = false) {
    preferredCodeLabel = codeLabel(value);
    if (persist && preferredCodeLabel) {
      try { localStorage.setItem(codePreferenceKey, preferredCodeLabel); }
      catch { /* selection still works without storage */ }
    }
    for (const group of codeGroups) group.activateLabel(preferredCodeLabel);
  }
  function readPreference() {
    try {
      const stored = localStorage.getItem(storageKey);
      return modes.includes(stored) ? stored : configured;
    } catch {
      return configured;
    }
  }
  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "paglora-appearance-toggle";
  toggle.setAttribute("data-paglora-appearance", "");
  header.appendChild(toggle);
  function setAppearance(mode) {
    root.dataset.colorMode = mode;
    const label = mode === "dark" ? tr("site.appearance.dark", "Dark") :
      mode === "light" ? tr("site.appearance.light", "Light") : tr("site.appearance.system", "System");
    toggle.textContent = (mode === "dark" ? "☾ " : mode === "light" ? "☀ " : "◐ ") + label;
    const description = tr("site.appearance.change", "Appearance: {mode}. Change theme", { mode: label });
    toggle.setAttribute("aria-label", description);
    toggle.title = description;
  }
  setAppearance(readPreference());
  // Keep separately opened documentation tabs in sync without requiring reload.
  // Ignore storage changes from other Paglora sites hosted on this origin.
  addEventListener("storage", event => {
    if (event.key === storageKey) {
      setAppearance(modes.includes(event.newValue) ? event.newValue : configured);
    } else if (event.key === codePreferenceKey) {
      // A separate tab may change or clear the preference. Never steal focus.
      updateCodePreference(event.newValue);
    }
  });
  toggle.addEventListener("click", () => {
    const next = modes[(modes.indexOf(root.dataset.colorMode) + 1) % modes.length];
    setAppearance(next);
    try { localStorage.setItem(storageKey, next); } catch { /* storage disabled */ }
  });

  // Small-screen navigation stays native <details> without scripts. JavaScript
  // only enhances the scrollable panel with dismissal and header-height tracking.
  const mobileNav = document.querySelector(".mobile-nav");
  if (mobileNav) {
    const summary = mobileNav.querySelector(":scope > summary");
    const siteHeader = document.querySelector(".site-header");
    function syncMobileOffset() {
      const height = siteHeader?.getBoundingClientRect?.().height;
      if (Number.isFinite(height) && height > 0 && height < 400) {
        mobileNav.style?.setProperty("--paglora-header-height", Math.ceil(height) + "px");
      }
    }
    syncMobileOffset();
    if (siteHeader && typeof ResizeObserver !== "undefined") {
      new ResizeObserver(syncMobileOffset).observe(siteHeader);
    }
    mobileNav.addEventListener("keydown", event => {
      if (event.key !== "Escape" || !mobileNav.open) return;
      event.preventDefault();
      mobileNav.open = false;
      summary?.focus?.();
    });
    mobileNav.addEventListener("click", event => {
      if (event.target?.closest?.("a[href]")) mobileNav.open = false;
    });
    document.addEventListener("pointerdown", event => {
      if (mobileNav.open && !mobileNav.contains(event.target)) mobileNav.open = false;
    });
    addEventListener("resize", () => {
      syncMobileOffset();
      if (typeof innerWidth === "number" && innerWidth > 800) mobileNav.open = false;
    }, { passive: true });
  }

  // Give generated Markdown code blocks copy controls. Clipboard access
  // requires a secure context; a failure never affects readable code.
  for (const code of document.querySelectorAll?.(".docs pre > code") ?? []) {
    const pre = code.parentElement;
    if (!pre) continue;
    const copy = document.createElement("button");
    copy.type = "button";
    copy.className = "paglora-copy-code";
    copy.textContent = tr("site.copy.copy", "Copy");
    copy.setAttribute("aria-label", tr("site.copy.copyLabel", "Copy code block"));
    copy.setAttribute("aria-live", "polite");
    pre.appendChild(copy);
    pre.classList.add("paglora-has-copy");
    let resetTimer;
    function showFeedback(text, label) {
      copy.textContent = text;
      copy.setAttribute("aria-label", label);
      if (resetTimer !== undefined) clearTimeout(resetTimer);
      resetTimer = setTimeout(() => {
        copy.textContent = tr("site.copy.copy", "Copy");
        copy.setAttribute("aria-label", tr("site.copy.copyLabel", "Copy code block"));
        resetTimer = undefined;
      }, 2000);
    }
    let copying = false;
    copy.addEventListener("click", async () => {
      if (copying) return;
      if (!navigator.clipboard?.writeText) {
        showFeedback(tr("site.copy.unavailable", "Unavailable"), tr("site.copy.unavailableLabel", "Clipboard unavailable"));
        return;
      }
      // Cancel a previous success timer so it cannot erase the in-flight
      // status of a slower subsequent clipboard operation.
      if (resetTimer !== undefined) {
        clearTimeout(resetTimer);
        resetTimer = undefined;
      }
      copying = true;
      copy.disabled = true;
      copy.textContent = tr("site.copy.copying", "Copying…");
      copy.setAttribute("aria-label", tr("site.copy.copyingLabel", "Copying code to clipboard"));
      copy.setAttribute("aria-busy", "true");
      try {
        await navigator.clipboard.writeText(code.textContent ?? "");
        showFeedback(tr("site.copy.copied", "Copied"), tr("site.copy.copiedLabel", "Code copied to clipboard"));
      } catch {
        showFeedback(tr("site.copy.unavailable", "Unavailable"), tr("site.copy.unavailableLabel", "Clipboard unavailable"));
      } finally {
        copying = false;
        copy.disabled = false;
        copy.removeAttribute("aria-busy");
      }
    });
  }


  // Code groups are readable as labeled, stacked examples without scripts.
  // Only after the full tab interface exists do we hide inactive examples.
  let codeGroupIndex = 0;
  for (const group of document.querySelectorAll?.(".docs .paglora-code-group") ?? []) {
    const panels = Array.from(group.querySelectorAll(".paglora-code-group-item"));
    const container = group.querySelector(".paglora-code-group-panels");
    if (panels.length < 2 || !container) continue;
    const groupId = codeGroupIndex++;
    const labels = panels.map(panel =>
      codeLabel(panel.querySelector(".paglora-code-group-label")?.textContent));
    const tablist = document.createElement("div");
    tablist.className = "paglora-code-group-tabs";
    tablist.setAttribute("role", "tablist");
    tablist.setAttribute("aria-label", group.getAttribute("aria-label") || "Code examples");
    const tabs = panels.map((panel, index) => {
      const tab = document.createElement("button");
      tab.type = "button";
      tab.className = "paglora-code-group-tab";
      tab.textContent = panel.querySelector(".paglora-code-group-label")?.textContent || "Example " + (index + 1);
      const tabId = "paglora-code-tab-" + groupId + "-" + index;
      const panelId = "paglora-code-panel-" + groupId + "-" + index;
      tab.id = tabId;
      panel.id = panelId;
      tab.setAttribute("role", "tab");
      tab.setAttribute("aria-controls", panelId);
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-labelledby", tabId);
      panel.tabIndex = 0;
      tab.addEventListener("click", () => activate(index, false, true));
      tab.addEventListener("keydown", event => {
        const direction = root.dir === "rtl" ? -1 : 1;
        let next = index;
        if (event.key === "ArrowRight") next = (index + direction + tabs.length) % tabs.length;
        else if (event.key === "ArrowLeft") next = (index - direction + tabs.length) % tabs.length;
        else if (event.key === "Home") next = 0;
        else if (event.key === "End") next = tabs.length - 1;
        else return;
        event.preventDefault();
        activate(next, true, true);
      });
      tablist.appendChild(tab);
      return tab;
    });
    function activate(selected, focus = false, remember = false) {
      tabs.forEach((tab, index) => {
        const active = index === selected;
        tab.setAttribute("aria-selected", String(active));
        tab.tabIndex = active ? 0 : -1;
        panels[index].hidden = !active;
      });
      if (focus) tabs[selected].focus();
      if (remember && labels[selected]) updateCodePreference(labels[selected], true);
    }
    function activateLabel(label) {
      if (!label) { activate(0); return; }
      const matching = labels.indexOf(label);
      // Groups without this language retain their own current selection.
      if (matching !== -1) activate(matching);
    }
    container.before(tablist);
    codeGroups.push({ activateLabel });
    panels.forEach((panel, index) => fragmentPanels.set(panel, () => activate(index)));
    // Always establish a valid initial tab before applying a saved preference.
    // A different page may not offer the preferred language at all.
    activate(0);
    activateLabel(preferredCodeLabel);
    group.classList.add("paglora-code-group-enhanced");
  }

  // General content tabs share the script-free, stacked-panel fallback of
  // code groups. Never hide a panel until all controls are ready.
  let contentTabsIndex = 0;
  for (const group of document.querySelectorAll?.(".docs .paglora-tabs") ?? []) {
    const panels = Array.from(group.querySelectorAll(".paglora-tabs-item"));
    const container = group.querySelector(".paglora-tabs-panels");
    if (panels.length < 2 || !container) continue;
    const groupId = contentTabsIndex++;
    const tablist = document.createElement("div");
    tablist.className = "paglora-tabs-tablist";
    tablist.setAttribute("role", "tablist");
    tablist.setAttribute("aria-label", group.getAttribute("aria-label") || "Content tabs");
    const tabs = panels.map((panel, index) => {
      const tab = document.createElement("button");
      tab.type = "button";
      tab.className = "paglora-tabs-tab";
      tab.textContent = panel.querySelector(".paglora-tabs-label")?.textContent || "Tab " + (index + 1);
      const tabId = "paglora-content-tab-" + groupId + "-" + index;
      const panelId = "paglora-content-panel-" + groupId + "-" + index;
      tab.id = tabId;
      panel.id = panelId;
      tab.setAttribute("role", "tab");
      tab.setAttribute("aria-controls", panelId);
      panel.setAttribute("role", "tabpanel");
      panel.setAttribute("aria-labelledby", tabId);
      panel.tabIndex = 0;
      tab.addEventListener("click", () => activate(index));
      tab.addEventListener("keydown", event => {
        const direction = root.dir === "rtl" ? -1 : 1;
        let next = index;
        if (event.key === "ArrowRight") next = (index + direction + tabs.length) % tabs.length;
        else if (event.key === "ArrowLeft") next = (index - direction + tabs.length) % tabs.length;
        else if (event.key === "Home") next = 0;
        else if (event.key === "End") next = tabs.length - 1;
        else return;
        event.preventDefault();
        activate(next, true);
      });
      tablist.appendChild(tab);
      return tab;
    });
    function activate(selected, focus = false) {
      tabs.forEach((tab, index) => {
        const active = index === selected;
        tab.setAttribute("aria-selected", String(active));
        tab.tabIndex = active ? 0 : -1;
        panels[index].hidden = !active;
      });
      if (focus) tabs[selected].focus();
    }
    container.before(tablist);
    activate(0);
    panels.forEach((panel, index) => fragmentPanels.set(panel, () => activate(index)));
    group.classList.add("paglora-tabs-enhanced");
  }


  // Reveal a fragment target inside an inactive code/content tab. The browser
  // may have tried to scroll before deferred scripts enhanced those panels.
  // Walk from the target outward and activate ancestor panels outermost first.
  function revealFragmentTarget() {
    const hash = typeof location.hash === "string" ? location.hash : "";
    if (hash.length < 2 || hash.length > 2048) return;
    let id;
    try { id = decodeURIComponent(hash.slice(1)); } catch { return; }
    if (!id || id.length > 512) return;
    const target = document.getElementById?.(id);
    if (!target) return;
    const selections = [];
    for (let node = target; node; node = node.parentElement) {
      const activate = fragmentPanels.get(node);
      if (activate && node.hidden) selections.push(activate);
    }
    if (!selections.length) return;
    for (const activate of selections.reverse()) activate();
    // A permalink must not change the stored code-language preference, steal
    // keyboard focus, or override a more recent fragment navigation.
    requestAnimationFrame(() => {
      if (location.hash === hash) target.scrollIntoView?.({ block: "start" });
    });
  }
  addEventListener("hashchange", revealFragmentTarget);
  revealFragmentTarget();

  // Print and Save as PDF must include every example and FAQ answer, even
  // when interactive tabs/accordions are collapsed on screen. Keep an exact
  // snapshot and restore it after the browser's print dialog closes.
  // No network, storage or project data changes are involved.
  // The print lifecycle belongs to the browser Window. Using its explicit
  // event target also keeps the runtime usable in scriptless/test environments.
  if (typeof window !== "undefined" && typeof window.addEventListener === "function") {
  let printSnapshot = null;
  window.addEventListener("beforeprint", () => {
    if (printSnapshot) return; // browsers may dispatch beforeprint repeatedly
    const disclosures = Array.from(document.querySelectorAll?.(".docs details") ?? []);
    const panels = Array.from(document.querySelectorAll?.(
      ".docs .paglora-code-group-item, .docs .paglora-tabs-item") ?? []);
    printSnapshot = {
      disclosures: disclosures.map(node => [node, node.open]),
      panels: panels.map(node => [node, node.hidden]),
    };
    for (const node of disclosures) node.open = true;
    for (const node of panels) node.hidden = false;
  });
  window.addEventListener("afterprint", () => {
    if (!printSnapshot) return;
    for (const [node, open] of printSnapshot.disclosures) node.open = open;
    for (const [node, hidden] of printSnapshot.panels) node.hidden = hidden;
    printSnapshot = null;
  });
  }


  // Long articles get unobtrusive reading progress and a keyboard-accessible
  // return-to-top control. Short pages and script-disabled exports stay readable.
  const readingArticle = document.querySelector(".docs");
  if (readingArticle && document.body && typeof readingArticle.getBoundingClientRect === "function") {
    const progress = document.createElement("div");
    progress.className = "paglora-reading-progress";
    progress.setAttribute("aria-hidden", "true"); // decorative; never announce every scroll tick
    const progressFill = document.createElement("span");
    progressFill.className = "paglora-reading-progress-fill";
    progress.appendChild(progressFill);

    const backToTop = document.createElement("button");
    backToTop.type = "button";
    backToTop.className = "paglora-back-to-top";
    backToTop.textContent = "↑";
    backToTop.hidden = true;
    const backLabel = tr("site.navigation.backToTop", "Back to top");
    backToTop.setAttribute("aria-label", backLabel);
    backToTop.title = backLabel;
    document.body.appendChild(progress);
    document.body.appendChild(backToTop);

    let readingUpdateQueued = false;
    function updateReadingPosition() {
      readingUpdateQueued = false;
      const bounds = readingArticle.getBoundingClientRect();
      const viewport = typeof innerHeight === "number" ? innerHeight : root.clientHeight;
      const y = typeof scrollY === "number" ? scrollY : root.scrollTop;
      if (!Number.isFinite(viewport) || viewport <= 0 ||
          !Number.isFinite(y) || !Number.isFinite(bounds.top) ||
          !Number.isFinite(bounds.bottom) || !Number.isFinite(bounds.height)) {
        progress.hidden = true;
        backToTop.hidden = true;
        return;
      }
      // Hide both controls on short pages where a progress meter is misleading.
      const longArticle = bounds.height >= Math.max(900, viewport * 1.35);
      progress.hidden = !longArticle;
      backToTop.hidden = !longArticle || y < Math.max(320, viewport * 0.45);
      if (!longArticle) return;
      const articleStart = bounds.top + y;
      const articleEnd = bounds.bottom + y;
      const start = Math.max(0, articleStart - 140);
      const end = Math.max(start + 1, articleEnd - viewport + 100);
      const fraction = Math.max(0, Math.min(1, (y - start) / (end - start)));
      progressFill.style.transform = "scaleX(" + (Math.round(fraction * 1000) / 1000) + ")";
    }
    function scheduleReadingUpdate() {
      if (readingUpdateQueued) return;
      readingUpdateQueued = true;
      requestAnimationFrame(updateReadingPosition);
    }
    backToTop.addEventListener("click", () => {
      // Prevent keyboard/screen-reader focus from remaining on a hidden button.
      const main = document.getElementById?.("content");
      if (main?.focus) {
        if (!main.hasAttribute?.("tabindex")) main.setAttribute("tabindex", "-1");
        main.focus({ preventScroll: true });
      }
      let reduceMotion = false;
      try {
        reduceMotion = typeof matchMedia === "function" &&
          matchMedia("(prefers-reduced-motion: reduce)").matches;
      } catch { /* no motion preference API: browser smooth scrolling is fine */ }
      if (typeof scrollTo === "function") {
        scrollTo({ top: 0, behavior: reduceMotion ? "auto" : "smooth" });
      }
    });
    addEventListener("scroll", scheduleReadingUpdate, { passive: true });
    addEventListener("resize", scheduleReadingUpdate, { passive: true });
    if (typeof ResizeObserver !== "undefined") {
      new ResizeObserver(scheduleReadingUpdate).observe(readingArticle);
    }
    scheduleReadingUpdate();
  }

  // Expose sharable heading anchors without changing project Markdown.
  for (const heading of document.querySelectorAll?.(".docs h2[id], .docs h3[id], .docs h4[id]") ?? []) {
    const id = heading.getAttribute("id");
    if (!id) continue;
    const link = document.createElement("a");
    link.className = "paglora-heading-link";
    link.href = "#" + encodeURIComponent(id);
    link.textContent = "#";
    link.setAttribute("aria-label", tr("site.heading.linkTo", "Link to {heading}", {
      heading: (heading.textContent ?? tr("site.heading.section", "section")).trim().slice(0, 160)
    }));
    heading.appendChild(link);
  }

  // Highlight the TOC heading the reader is currently passing. This is a
  // progressive enhancement: anchor links work without JavaScript.
  const mobileOutline = document.querySelector(".mobile-toc");
  const outlines = [document.querySelector(".toc-inner"), mobileOutline].filter(Boolean);
  if (!outlines.length) return;
  // Desktop and mobile outlines point to the same headings. Keep both sets
  // of links synchronized, even when the desktop sidebar is CSS-hidden.
  const items = outlines.flatMap(outline => [...outline.querySelectorAll('a[href^="#"]')])
    .map(link => {
      let id = link.getAttribute("href").slice(1);
      try { id = decodeURIComponent(id); } catch { return null; }
      const heading = document.getElementById(id);
      return heading ? { link, heading } : null;
    })
    .filter(Boolean);
  if (!items.length) return;

  // Close the mobile "On this page" drawer after a heading is selected.
  mobileOutline?.addEventListener("click", event => {
    if (event.target?.closest?.('a[href^="#"]')) mobileOutline.open = false;
  });

  let scheduled = false;
  function markActiveHeading() {
    scheduled = false;
    let activeHeading = null;
    let closestPassed = -Infinity;
    let closestUpcoming = Infinity;
    // Desktop and mobile TOCs duplicate headings and may be ordered differently.
    // Ignore headings in inactive tab panels, even when they appear earlier.
    for (const item of items) {
      if (item.heading.closest?.("[hidden]")) continue;
      if (!activeHeading) activeHeading = item.heading;
      const top = item.heading.getBoundingClientRect().top;
      if (top <= 140 && top > closestPassed) {
        closestPassed = top;
        activeHeading = item.heading;
      } else if (closestPassed === -Infinity && top > 140 && top < closestUpcoming) {
        closestUpcoming = top;
        activeHeading = item.heading;
      }
    }
    for (const item of items) {
      if (item.heading === activeHeading) item.link.setAttribute("aria-current", "location");
      else item.link.removeAttribute("aria-current");
    }
  }
  function schedule() {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(markActiveHeading);
  }
  addEventListener("scroll", schedule, { passive: true });
  addEventListener("resize", schedule, { passive: true });
  addEventListener("hashchange", schedule);
  schedule();
})();
