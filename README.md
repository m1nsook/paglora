# Paglora

<img src="apps/desktop/public/paglora.svg" alt="Paglora logo" width="88" />

**Beautiful documentation, built locally and published anywhere.**

[![npm alpha version](https://img.shields.io/npm/v/paglora/alpha?style=flat-square&logo=npm&label=npm%20alpha)](https://www.npmjs.com/package/paglora?activeTab=versions)
[![npm monthly downloads](https://img.shields.io/npm/dm/paglora?style=flat-square&logo=npm&label=downloads%2Fmonth)](https://www.npmjs.com/package/paglora)
[![GitHub Actions CI](https://img.shields.io/github/actions/workflow/status/m1nsook/paglora/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/m1nsook/paglora/actions/workflows/ci.yml)
[![MIT license](https://img.shields.io/github/license/m1nsook/paglora?style=flat-square)](LICENSE)

Paglora is an open-source, **local-first documentation studio**, built with Tauri, Rust, React, and TypeScript. Create documentation in Markdown, edit it visually, and export static sites you can host anywhere—without an account or cloud backend.

> [!IMPORTANT]
> **Early development:** Studio can be run from source on Windows. There is no stable desktop installer yet. The published `paglora` npm package is an alpha CLI for initializing and validating projects, **not** a website builder or Studio installer.

## Features

- **Visual Studio:** Markdown editor with autosave, pages and navigation, resizable Edit/Split/Canvas views, and desktop/tablet/mobile previews.
- **Site design:** Local branding, theme colors, fonts, navigation, CSS, and light/dark site appearance independently of Studio's own theme.
- **Documentation components:** Cards, callouts, steps, tabs, code groups, accordions, badges, and a searchable component library. The source-backed Components canvas supports exact-position insertion and bounded Undo/Redo (buttons or Ctrl/Cmd+Z and Ctrl/Cmd+Shift+Z); editing Markdown in Write/Source starts a fresh component history.
- **Local-first projects:** Offline documents and images, private drafts, local search, project backups and recoverable trash. Importing or exporting a portable `.paglora` backup shows its actual current stage without invented progress percentages; archive operations are serialized to prevent accidental duplicate imports.
- **Static publishing:** Portable HTML/CSS/JS, search, mobile navigation, metadata and sitemap; private pages are excluded from exports.
- **Languages:** Studio translations and independently configurable published-site language.

## Quick start

Install [Bun](https://bun.sh/), [Rust](https://rustup.rs/), and the [Tauri 2 Windows prerequisites](https://v2.tauri.app/start/prerequisites/). Open the repository with GitHub Desktop, then from the repository folder run:

```powershell
bun install
bun run desktop
```

The browser-only frontend can be started with `bun run dev`, but cannot access local Tauri project storage.

To try the **alpha CLI** (separate from desktop Studio):

```powershell
bunx --bun paglora@alpha init .\my-docs --name "My Docs"
bunx --bun paglora@alpha inspect .\my-docs
bunx --bun paglora@alpha validate .\my-docs
```

For experimental builds and release information, check [GitHub Actions](https://github.com/m1nsook/paglora/actions/workflows/nightly-studio.yml) and [Releases](https://github.com/m1nsook/paglora/releases).

## Documentation

**Full guides, screenshots, component examples, publishing steps and translation instructions live in the separate documentation project**, not this README:

- **[Documentation website](https://m1nsook.github.io/paglora-docs/)**
- **[paglora-docs repository](https://github.com/m1nsook/paglora-docs)** — editable docs source built with Paglora
- **[paglora-lang repository](https://github.com/m1nsook/paglora-lang)** — community language contributions
- **[Changelog](CHANGELOG.md)** — notable development changes

## Contribute

See [CONTRIBUTING.md](CONTRIBUTING.md), [Issues](https://github.com/m1nsook/paglora/issues), and [SECURITY.md](SECURITY.md). Paglora is licensed under [MIT](LICENSE) and created by [m1nsook](https://github.com/m1nsook).
