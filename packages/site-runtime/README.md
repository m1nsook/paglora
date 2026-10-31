# Paglora site runtime

The small first-party `@paglora/site-runtime` package progressively enhances static HTML exported by Paglora. It uses standard browser APIs and does not require Vue, jQuery, a backend, or a hosted service.

The initial release supports **Light / Dark / System** appearance preferences (stored per site root where storage is allowed) and **active table-of-contents heading** tracking. Navigation and content work without JavaScript; interactive enhancement is optional.

The Rust exporter embeds **minified browser bundles** generated from `src/index.js` and `src/search.js` by `bun packages/site-runtime/build.mjs`. Cargo invokes this build automatically before compiling the desktop backend (Bun is required for Rust builds). The bundles are emitted as `paglora.js` and `paglora-search.js` alongside exported public HTML, and public pages load them with deferred script tags. Studio previews intentionally strip both script tags because the current same-origin iframe is script-disabled, pending a separate secure interactive-preview design. Generated `dist/` bundles remain ignored by Git.

Run `bun packages/site-runtime/build.mjs` followed by `node --test packages/site-runtime/test/*.test.mjs` for focused tests.

The runtime is original Paglora source, **informed by VitePress's interaction patterns**, not copied Vue code. VitePress source is MIT-licensed (Copyright 2019–present Yuxi (Evan) You). If actual source is reused in the future, preserve the required notices in the distributed artifacts.

Static search indexes only public pages and loads locally; multilingual UI labels are planned separately.

## Local documentation search

The exported `paglora-search.js` adds a Ctrl+K / Cmd+K search interface. It lazy-loads `search-index.json`, generated **only from public pages** at build time using sanitized source content. Results link to local `.html` pages; all titles, excerpts and queries are inserted with `textContent`, not raw HTML. No user accounts, external services or JavaScript are required for ordinary page browsing.

Search now bundles **MiniSearch 7.2.0 (MIT)** into the local `paglora-search.js` asset. It ranks title matches, supports word prefixes, and falls back to fuzzy matching for longer Latin queries. A substring fallback preserves CJK queries without spaces. All search is offline; no CDN or search server is contacted. MiniSearch's full license is shipped in `licenses/MiniSearch-MIT.txt` with exported sites.

## Code example language preferences

Published `[!CODEGROUP]` examples are keyboard-accessible tabs when JavaScript is available. Choosing a labeled language (for example, **PowerShell**) selects that same language in other code groups on the current page, when offered. The preference is remembered under the exported site's own URL path, so separate documentation sites hosted on one domain do not affect one another. Other browser tabs on the same site also follow changes. Groups without the preferred language keep their own selection, and unrelated `[!TABS]` content blocks remain independent.

The preference uses optional local browser storage. If storage is disabled, code groups still synchronize on the current page; with JavaScript disabled (including Studio's safe preview), every labeled code example remains visible and readable. No network request or project format change is involved.

## Local image assets

In **Design site → Header**, import a PNG, JPEG, WebP or GIF image (up to 5 MiB) to use as the site logo. Paglora saves a uniquely named file inside the project's `assets/` directory and uses a relative `assets/...` URL. You can reuse that URL in Markdown images such as `![Diagram](assets/my-diagram.png)`. Exports copy **only images referenced by public page HTML or the public logo**; unreferenced/private project assets are not published. Studio's script-disabled preview inlines those referenced images as passive data URLs. SVG, remote HTTP, directory traversal and arbitrary file URLs are not supported. Missing or tampered assets fail the staged export instead of replacing a previous successful export.

## Canonical URLs and sitemap

In **Design site → Header**, optionally set the **Published site URL** to the HTTPS site root (for example, `https://username.github.io/project/`). Include the trailing slash and the GitHub Pages repository subpath when applicable. When configured, exported pages include canonical and Open Graph URL metadata, and the exporter writes `sitemap.xml` containing **only public pages**. Leave this field empty for offline or unknown hosting; no canonical URLs or sitemap will be generated. This setting does not change relative navigation or require a backend.

## Search result highlights

Published Ctrl/Cmd+K search highlights exact query terms in result titles and excerpts with semantic mark elements and text nodes, never executable HTML. Technical terms such as C++, CJK substrings, Unicode ligatures and case-folded matches map back to readable display text. Fuzzy matches remain readable when no exact substring is available. The site accent colors the highlights, with a forced-colors fallback. Search stays offline and public-page-only.

## Printing and saving public documentation as PDF

On any published page, use the browser's **Print** command (Ctrl+P / Cmd+P) and choose a printer or **Save as PDF**. Paglora's print stylesheet removes the navigation, table of contents, footer, page pager, search controls and other interactive chrome. It uses a high-contrast white paper layout regardless of the reader's selected dark theme, keeps code and tables readable, and respects the browser's normal page breaks. This works offline and requires no print service or account.

When JavaScript is enabled, the first-party runtime temporarily opens FAQ/disclosure answers and reveals **all** code-group and content-tab panels for printing, then restores their original open/selected state after printing. Without JavaScript, the print stylesheet still formats the visible static article and all script-free tab examples; closed native disclosures may remain collapsed. The same styles apply in exported HTML and the script-disabled Studio preview. Only the current page is printed; printing does not include private project pages or change saved Markdown.


## Long-page reading navigation

On long public documentation pages, the optional browser runtime shows a thin accent-colored **reading progress bar** at the top and a **Back to top** button after scrolling. The progress follows the article rather than the footer, and short pages do not show misleading controls. The button is keyboard accessible, moves focus back to the main content and respects reduced-motion preferences. The bar is decorative (not repeatedly announced to screen readers); RTL pages reverse its fill direction, and high-contrast palettes retain visibility.

These controls are local, require no account, storage, tracking or network access, and are not included in Print/Save as PDF. With JavaScript disabled, the article, native links and table of contents continue to work normally. Custom site CSS can override the first-party classes for reading progress and back-to-top.

## Deep links inside documentation tabs

Heading permalinks and table-of-contents links also work when the target is inside an inactive `[!TABS]` or `[!CODEGROUP]` panel. On an initial page load or later URL-fragment change, the exported runtime opens the required panels (including nested groups) before scrolling to the target. This does not change the reader's saved code-language preference or move keyboard focus. The active table-of-contents indicator ignores headings inside hidden panels. Without JavaScript, all tab content stays visible, so ordinary heading links remain usable. No network requests or project-format changes are needed.
