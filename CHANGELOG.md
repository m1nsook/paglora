# Changelog

Paglora is in early development. Desktop milestone numbers are separate from the public npm CLI package.

## Unreleased — desktop source (after 0.2.0)

### Publishing and static-site packaging (release candidate)
- Introduced a deterministic content-addressed browser asset layout: `_paglora/chunks/<hash>.js` and `_paglora/chunks/css/<hash>.css`. The hashed files are referenced by exported HTML; legacy root assets remain temporarily available for compatibility.
- Added compact HTML page mirrors in `pages/` with canonical root-page bases and `noindex,follow`; existing public links, search results and canonical URLs continue pointing to the existing root HTML filenames. The final canonical `pages/` migration is deferred until end-to-end navigation, search and SEO verification.
- Added optional published-site favicons: HTTPS image URL or a locally generated transparent 64×64 PNG from uploaded PNG/JPEG/WebP/GIF. Local icons are included in exports and inlined for script-disabled preview.
- Updated packaging guards for the official headless GitHub Pages docs build (companion `paglora-docs` PR), without adding remote dependencies to generated sites.
- All changes are under review and are **not a stable release** until app CI, generated-site smoke tests, Windows Studio acceptance and the installer/update release gate pass.


### Studio workflow
- Offline backup import/export now shows real indeterminate stages (file read, native validation/import, workspace refresh, and native export). Duplicate archive actions and conflicting project removal are blocked while busy; a completed import with a failed list refresh is reported separately from a failed import.
- Added user-configurable keyboard shortcuts for local project/page search, project creation, project navigation and Studio Settings. A Radix shortcut recorder validates reserved/duplicate combinations, offers a reset action and stores settings locally without modifying documentation projects.

### Studio visual editing
- The source-owned Components canvas now supports Undo/Redo for inserted Markdown blocks, with exact original-source restoration, bounded history, keyboard shortcuts and explicit buttons. Editing in Write/Source invalidates stale component history rather than rolling back unrelated text. Windows interaction acceptance is still required.
- Added a searchable local image library: import and browse validated project-owned PNG/JPEG/WebP/GIF files, view bounded small-image thumbnails and selected-image previews offline, copy portable Markdown, or insert image nodes at the editor caret. Image nodes round-trip to Markdown without changing project formats; private-only images remain excluded from static exports.
- The Blocks outline now supports duplicating and deleting complete source-owned Markdown blocks, Alt+Up/Down reordering, explicit Undo/Redo, and caret-aware selection after changes. All actions use undoable ProseMirror transactions and existing autosave; no project schema changes.

### Print-ready published documentation
- Public pages now print as readable, high-contrast articles across Classic/Inset/Reading layouts and light/dark palettes, without site chrome or clipped code/tables. Browser Print/Save as PDF works offline.
- Before printing, the optional site runtime expands FAQ/disclosure answers and reveals all code/content tab examples; it restores the exact original state afterward. Script-disabled previews keep their existing readable fallback.

### Published documentation search
- Search results highlight exact query terms in titles and excerpts with safe semantic marks, Unicode-aware offsets, technical-token matching and accessible accent/forced-colors styling. Offline public-only search and keyboard controls remain intact.

### Published documentation navigation
- Code example groups now remember the reader's selected language per deployed site path, synchronize matching examples on the current page and across browser tabs, and retain accessible keyboard controls. Unmatched groups, unrelated content tabs and script-disabled previews keep their existing behavior.
- Improved small-screen Browse navigation with a scrollable, sticky, RTL-aware panel that overlays content rather than pushing long articles down. Native `<details>` keeps page links usable without scripts; progressive enhancement adds Escape dismissal, outside-click closing, and header-height tracking for wrapped headers.

### Documentation authoring
- Added source-owned `[!BADGES]` release/feature status labels with five validated color variants, Studio block/gallery insertion, responsive light/dark static styling and matching Studio/headless sanitization. Malformed groups stay readable as Markdown.
- Added source-owned `[!ACCORDION]` FAQ questions with Studio insertion, native keyboard-accessible `<details>` behavior, compact adjacent-question groups, and matching headless/Studio static output. Malformed markers remain visible Markdown.
- Added nested Markdown `[!TABS]` / `[!TAB]` content panels with Studio insertion, safe preview/export parity, script-free readable fallback, and keyboard-accessible published tabs (including RTL navigation).
- Added Markdown `[!CODEGROUP]` fenced-code examples with Studio insertion, safe language labels, script-free stacked fallback, and progressively enhanced accessible tabs in published sites. No new project schema.
- Added a source-owned `[!STEPS]` Markdown component with an ordered-list timeline, accessible static HTML, RTL-aware styling, and Studio block insertion. It requires no browser script and preserves existing project files.

## 0.2.0 — desktop development milestone (October 31, 2026)

### Website publishing and verification
- Added a publishing report to Studio, showing exported public pages, excluded private pages, and bundled local image count.
- Warn when a published page links to a **missing or private page**. Warnings do not silently block exporting valid public content.
- Export continues to stage all files before replacing the last successfully built website, preserving the previous working export if a build fails.
- Clarified that browser runtime features must be tested over HTTP(S), rather than opening exported HTML directly with `file://`.

### Features delivered since 0.1.0
- Minified, bundled browser scripts, offline MiniSearch with typo-tolerant/prefix queries, CJK substring support, and accessible keyboard search.
- Light, dark and system appearance switching, respecting the configured accent in all modes.
- Documentation heading permalinks, code-copy controls, callouts, task lists, disclosures and responsive navigation.
- Project-owned image imports (PNG/JPEG/WebP/GIF, up to 5 MiB), with only public-referenced assets included in exported sites.
- Document-language options, Arabic RTL, social metadata, optional canonical URLs and public-only `sitemap.xml`.
- Studio canvas improvements, responsive Tablet/Phone preview, and safeguards against stale project/page state.
- Windows Rust and Windows/Ubuntu frontend regression checks.

### Status
Development milestone only: no stable Windows installer, MDX/TSX source compiler, automated GitHub Pages deployment or general-purpose visual page builder yet. These capabilities remain roadmap work.

## 0.1.0 — initial foundation

- Tauri 2 desktop Studio, local-first project management, Markdown editing, page navigation, website design and static export.

## Public npm CLI (independent version)

- `paglora@0.0.1-alpha.3` was published October 31, 2026 under the npm `alpha` tag.
- Commands: `init`, `inspect`, and `validate`; no CLI website build yet.
