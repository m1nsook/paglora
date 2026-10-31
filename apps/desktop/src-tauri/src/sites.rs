use serde::{Deserialize, Serialize};
use std::{collections::{hash_map::DefaultHasher, HashMap, HashSet}, fs, hash::{Hash, Hasher}, io::{Read, Write}, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Emitter, Manager};

include!("locale_catalogs.rs");

const CONFIG: &str = "site.json";
const EXPORTS: &str = "site-exports";
const MAX_ASSET_BYTES: usize = 5 * 1024 * 1024;
const MAX_PREVIEW_ASSET_BYTES: usize = 15 * 1024 * 1024;
// First-party, browser-only runtime. The Studio preview deliberately removes it.
struct SiteRuntimeAsset {
    filename: &'static str,
    source: &'static str,
}
const SITE_RUNTIME_ASSETS: &[SiteRuntimeAsset] = &[
    SiteRuntimeAsset { filename: "paglora.js", source: include_str!("../../../../packages/site-runtime/dist/index.js") },
    SiteRuntimeAsset { filename: "paglora-search.js", source: include_str!("../../../../packages/site-runtime/dist/search.js") },
];
// Reproducible source-content address, independent of Windows/Linux paths or
// release timestamps. URLs change automatically when bundle bytes change.
fn chunk_hash(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
fn runtime_chunk_name(asset: &SiteRuntimeAsset) -> String {
    format!("_paglora/chunks/{}.js", chunk_hash(asset.source.as_bytes()))
}
fn stylesheet_chunk_name() -> String {
    // The name is based on the actual exported CSS, including its relative
    // font URL. Preview continues to embed SITE_CSS directly.
    let css = SITE_CSS.replace("fonts/geist-latin.woff2", "../../../fonts/geist-latin.woff2");
    format!("_paglora/chunks/css/{}.css", chunk_hash(css.as_bytes()))
}
/// Compatibility mirror for modern /pages layout. Keep the canonical root
/// page accessible for existing Markdown links and configured search results.
/// Only compact Paglora's wrapper markup; NEVER minify the authored article,
/// which may contain significant whitespace inside pre/code elements.
fn minified_page_mirror(html: &str, page_file: &str) -> Result<String, String> {
    let start = html.find("<article class=\"docs\">").ok_or("Missing article start")?;
    let end = html[start..].find("</article>").ok_or("Missing article end")? + start + "</article>".len();
    let compact = |part: &str| part.lines().map(str::trim).collect::<String>();
    let mut wrapper = compact(&html[..start]);
    if !wrapper.contains("<head>") { return Err("Missing page head".into()); }
    // Base points to the existing canonical root page. In-page #anchors,
    // CSS, assets, and runtime search URLs resolve without broken paths.
    let base = format!("<head><base href=\"../{page_file}\"><meta name=\"robots\" content=\"noindex,follow\">");
    wrapper = wrapper.replacen("<head>", &base, 1);
    Ok(format!("{}{}{}", wrapper, &html[start..end], compact(&html[end..])))
}

fn site_runtime_script_tags() -> String {
    SITE_RUNTIME_ASSETS.iter()
        .map(|asset| format!("<script defer src=\"{}\"></script>\n", runtime_chunk_name(asset)))
        .collect()
}
const SITE_CSS: &str = r#"
@font-face{font-family:"Geist Website";src:url("fonts/geist-latin.woff2") format("woff2");font-weight:100 900;font-style:normal;font-display:swap}
:root{color-scheme:light;--bg:#ffffff;--fg:#151a24;--muted:#626b7c;--line:#e6eaf0;--subtle:#f6f8fb;--hover:#edf3fa;--brand:#2196f3;--brand-light:#eaf5ff;--code-bg:#101828;--code-fg:#e9eef8;--header-bg:rgba(255,255,255,.96)}
:root[data-color-mode="dark"]{color-scheme:dark;--bg:#11151d;--fg:#f1f4f9;--muted:#a5aec0;--line:#2a3342;--subtle:#191f2b;--hover:#222d40;--brand:#72baff;--brand-light:#152e48;--code-bg:#080d14;--code-fg:#e9eef8;--header-bg:rgba(17,21,29,.96)}
@media(prefers-color-scheme:dark){:root:not([data-color-mode="light"]){color-scheme:dark;--bg:#11151d;--fg:#f1f4f9;--muted:#a5aec0;--line:#2a3342;--subtle:#191f2b;--hover:#222d40;--brand:#72baff;--brand-light:#152e48;--code-bg:#080d14;--code-fg:#e9eef8;--header-bg:rgba(17,21,29,.96)}}
*{box-sizing:border-box}
html{scroll-behavior:smooth;scroll-padding-top:105px}
body{margin:0;background:var(--bg);color:var(--fg);font-family:"Geist Website",system-ui,-apple-system,"Segoe UI",sans-serif;font-size:15px;line-height:1.75;-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}
a{color:inherit}
a:hover{color:var(--brand)}
a:focus-visible,summary:focus-visible{outline:2px solid var(--brand);outline-offset:3px}
.skip-link{position:absolute;z-index:100;top:-60px;left:10px;background:var(--bg);padding:8px 14px;border:1px solid var(--line);border-radius:8px}
.skip-link:focus{top:10px}
.paglora-reading-progress[hidden],.paglora-back-to-top[hidden]{display:none!important}
.paglora-reading-progress{position:fixed;inset-block-start:0;inset-inline:0;z-index:55;height:3px;pointer-events:none}
.paglora-reading-progress-fill{display:block;width:100%;height:100%;background:var(--brand);transform:scaleX(0);transform-origin:left center;transition:transform .08s linear}
:root[dir="rtl"] .paglora-reading-progress-fill{transform-origin:right center}
.paglora-back-to-top{position:fixed;inset-inline-end:clamp(16px,3vw,36px);bottom:calc(18px + env(safe-area-inset-bottom,0px));z-index:28;display:grid;place-items:center;width:44px;height:44px;padding:0;border:1px solid var(--line);border-radius:50%;background:var(--bg);color:var(--fg);box-shadow:0 8px 28px rgba(0,0,0,.15);font:inherit;font-size:22px;line-height:1;cursor:pointer}
.paglora-back-to-top:hover{border-color:var(--brand);background:var(--subtle);color:var(--brand)}
.paglora-back-to-top:focus-visible{outline:2px solid var(--brand);outline-offset:3px}
@media(forced-colors:active){.paglora-reading-progress-fill{background:Highlight}.paglora-back-to-top{border-color:ButtonText}}
@media(prefers-reduced-motion:reduce){.paglora-reading-progress-fill{transition:none}}
.site-header{position:sticky;top:0;z-index:30;background:var(--header-bg);border-bottom:1px solid var(--line);backdrop-filter:blur(18px)}
.site-header.not-sticky{position:relative}
.paglora-appearance-toggle{flex-shrink:0;appearance:none;border:1px solid var(--line);border-radius:9px;background:var(--subtle);color:var(--fg);font:inherit;font-size:12px;font-weight:650;cursor:pointer;white-space:nowrap;padding:7px 11px;line-height:1.3}
.paglora-appearance-toggle:hover{border-color:var(--brand);background:var(--hover)}
.paglora-appearance-toggle:focus-visible{outline:2px solid var(--brand);outline-offset:2px}
.paglora-search-trigger{appearance:none;display:flex;align-items:center;justify-content:space-between;flex-shrink:0;width:160px;min-height:33px;padding:5px 11px;border:1px solid var(--line);border-radius:9px;background:var(--subtle);color:var(--muted);font:inherit;font-size:12px;text-align:left;cursor:pointer}
.paglora-search-trigger::after{content:"⌘K";font-size:11px;opacity:.65}
.paglora-search-trigger:hover{border-color:var(--brand);color:var(--fg)}
.paglora-search-trigger:focus-visible{outline:2px solid var(--brand);outline-offset:2px}
.paglora-search-open{overflow:hidden}
.paglora-search-backdrop[hidden]{display:none}
.paglora-search-backdrop{position:fixed;inset:0;z-index:500;display:flex;align-items:flex-start;justify-content:center;padding:12vh 18px 28px;background:rgba(5,10,18,.65);backdrop-filter:blur(5px)}
.paglora-search-dialog{width:min(640px,100%);max-height:min(76vh,750px);display:flex;flex-direction:column;border:1px solid var(--line);border-radius:15px;background:var(--bg);color:var(--fg);box-shadow:0 20px 80px rgba(0,0,0,.25);overflow:hidden}
.paglora-search-field{display:flex;align-items:center;gap:12px;padding:12px 14px;border-bottom:1px solid var(--line)}
.paglora-search-input{flex:1;min-width:0;outline:none;background:transparent;border:0;color:var(--fg);font:inherit;font-size:16px}
.paglora-search-input::-webkit-search-cancel-button{cursor:pointer}
.paglora-search-close{padding:5px 9px;border:1px solid var(--line);border-radius:6px;background:var(--subtle);color:var(--muted);font:inherit;font-size:11px;cursor:pointer}
.paglora-search-results{min-height:100px;max-height:55vh;overflow-y:auto;padding:8px}
.paglora-search-result{display:block;padding:11px 13px;border:1px solid transparent;border-radius:9px;color:var(--fg);text-decoration:none}
.paglora-search-result:hover,.paglora-search-result.is-selected{background:var(--hover);border-color:var(--line)}
.paglora-search-match{background:color-mix(in srgb,var(--brand) 22%,var(--bg));color:var(--fg);font-weight:750;border-radius:3px;padding:0 .03em}
@media(forced-colors:active){.paglora-search-match{background:Highlight;color:HighlightText}}
.paglora-search-result-title{display:block;font-size:13px;font-weight:700;line-height:1.5}
.paglora-search-result-summary{display:block;margin-top:4px;font-size:12px;color:var(--muted);line-height:1.5;overflow:hidden;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical}
.paglora-search-status{padding:22px 15px;color:var(--muted);font-size:13px}
.paglora-search-retry{margin:0 15px 16px;padding:8px 12px;border:1px solid var(--line);border-radius:7px;background:var(--subtle);color:var(--fg);font:inherit;font-size:12px;cursor:pointer}
.paglora-search-retry:hover,.paglora-search-retry:focus-visible{border-color:var(--brand);outline-offset:2px}
.paglora-search-footnote{padding:9px 15px;border-top:1px solid var(--line);font-size:11px;color:var(--muted)}
@media(max-width:680px){.paglora-search-trigger{width:auto;min-width:75px}.paglora-search-trigger::after{display:none}.paglora-search-backdrop{padding-top:7vh}}

.toc-inner a[aria-current="location"]{color:var(--brand);font-weight:650}
@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}}
.top-bar{max-width:1500px;min-height:72px;margin:auto;padding:0 clamp(22px,3vw,48px);display:flex;align-items:center;justify-content:space-between;gap:28px}
.brand-wrap{display:inline-flex;align-items:center;gap:12px;min-width:0}
.brand{font-size:17px;font-weight:750;letter-spacing:-.045em;text-decoration:none;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.brand-logo{height:34px;max-width:144px;object-fit:contain}
.header-links{display:flex;align-items:center;gap:24px;flex-wrap:wrap}
.header-links a{font-size:13px;color:var(--muted);text-decoration:none;font-weight:550;white-space:nowrap}
.header-links a:hover{color:var(--fg)}
.site-header.header-center .top-bar{justify-content:center;flex-wrap:wrap;padding-block:14px}
.mobile-nav{display:none}
.site-body{max-width:1500px;margin:0 auto;display:grid;grid-template-columns:minmax(230px,260px) minmax(0,1fr) minmax(190px,235px);min-height:calc(100vh - 165px)}
.site-nav{grid-column:1;min-width:0;min-height:0;border-right:1px solid var(--line);padding:32px 16px 40px clamp(18px,2.5vw,42px)}
.site-nav-inner{position:sticky;top:100px;max-height:calc(100vh - 115px);overflow:auto;scrollbar-width:thin}
.site-nav .nav-group{margin-bottom:23px}
.site-nav .nav-group:last-child{margin-bottom:0}
.site-nav .nav-group>summary,.mobile-nav .nav-group>summary{display:flex;align-items:center;justify-content:space-between;gap:9px;padding:6px 13px;margin:0 0 7px;border-radius:7px;color:var(--muted);font-size:11px;font-weight:750;line-height:1.5;letter-spacing:.07em;text-transform:uppercase;cursor:pointer;list-style:none;user-select:none}
.site-nav .nav-group>summary::-webkit-details-marker,.mobile-nav .nav-group>summary::-webkit-details-marker{display:none}
.site-nav .nav-group>summary::after,.mobile-nav .nav-group>summary::after{content:"";width:6px;height:6px;flex-shrink:0;border-right:1.5px solid currentColor;border-bottom:1.5px solid currentColor;transform:rotate(45deg);transition:transform .16s ease}
.site-nav .nav-group:not([open])>summary::after,.mobile-nav .nav-group:not([open])>summary::after{transform:rotate(-45deg)}
.site-nav .nav-group>summary:hover,.mobile-nav .nav-group>summary:hover{background:var(--hover);color:var(--fg)}
.site-nav .nav-group>summary:focus-visible,.mobile-nav .nav-group>summary:focus-visible{outline:2px solid var(--brand);outline-offset:2px}
.mobile-nav .nav-group{min-width:0}
.mobile-nav .nav-group>summary{margin:9px 0 5px}
.mobile-nav .nav-group-links{display:grid;gap:3px}
.site-body.nav-top .site-nav .nav-group{display:flex;align-items:center;gap:8px;margin:0}
.site-body.nav-top .site-nav .nav-group>summary{margin:0}
.site-body.nav-top .site-nav .nav-group-links{display:flex;align-items:center;gap:4px;flex-wrap:wrap}
@media(prefers-reduced-motion:reduce){.site-nav .nav-group>summary::after,.mobile-nav .nav-group>summary::after{transition:none}}
.site-nav a{display:block;padding:8px 13px;margin:2px 0;border-radius:9px;text-decoration:none;font-size:13px;font-weight:500;line-height:1.55;color:var(--muted);overflow-wrap:anywhere;transition:background-color .13s,color .13s}
.site-nav a:hover{background:var(--hover);color:var(--fg)}
.site-nav a[aria-current=page]{background:var(--brand-light);color:var(--brand);font-weight:700}
.site-main{grid-column:2;min-width:0;width:100%;padding:44px clamp(26px,5vw,80px) 110px}
.article-wrap{max-width:var(--paglora-content-width,790px);margin:0 auto}
.page-heading{margin:0 0 27px;font-size:12px;line-height:1.5;color:var(--muted);font-weight:500}
.page-heading .crumb-separator{margin:0 8px;color:var(--muted)}
.page-heading strong{font-weight:650;color:var(--fg)}
.docs{font-size:15.5px;line-height:1.82;overflow-wrap:anywhere}
.docs h1,.docs h2,.docs h3,.docs h4{line-height:1.3;font-weight:740;letter-spacing:-.035em;scroll-margin-top:100px}
.docs h1{font-size:clamp(2.1rem,3.2vw,2.85rem);letter-spacing:-.052em;line-height:1.14;margin:0 0 27px}
.docs h2{font-size:1.68rem;margin:53px 0 17px;padding-bottom:13px;border-bottom:1px solid var(--line)}
.docs h3{font-size:1.25rem;margin:32px 0 12px}
.docs h4{font-size:1.05rem;margin:22px 0 9px}
.docs p,.docs ul,.docs ol{margin:0 0 19px}
.docs ul,.docs ol{padding-left:1.55em}
.docs li{padding-left:3px;margin:.32em 0}
.docs li::marker{color:var(--muted)}
.docs li.paglora-task-item{list-style:none;padding-left:0}
.docs details.paglora-disclosure{margin:20px 0;border:1px solid var(--line);border-radius:10px;background:var(--subtle);padding:0 16px}
.docs details.paglora-disclosure>summary{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:13px 0;font-weight:650;cursor:pointer;list-style:none}
.docs details.paglora-disclosure>summary::-webkit-details-marker{display:none}
.docs details.paglora-disclosure>summary::after{content:"+";font-size:18px;color:var(--muted);font-weight:500}
.docs details.paglora-disclosure[open]>summary::after{content:"−"}
.docs details.paglora-disclosure>summary:focus-visible{outline:2px solid var(--brand);outline-offset:3px}
.docs details.paglora-disclosure>:last-child:not(summary){margin-bottom:15px}
.docs details.paglora-disclosure>details.paglora-disclosure{background:var(--bg)}

.docs .paglora-task-checkbox{margin:0 .55em 0 -1.5em;vertical-align:middle;accent-color:var(--brand);opacity:1;pointer-events:none}
.docs blockquote{margin:24px 0;border-left:3px solid var(--brand);padding:13px 20px;color:var(--muted);background:var(--subtle);border-radius:0 10px 10px 0}
/* Markdown cards share the safe source-to-preview-to-export pipeline. */
.docs .paglora-card{margin:24px 0;padding:20px 22px;border:1px solid var(--line);border-radius:13px;background:var(--subtle);box-shadow:0 2px 8px rgba(0,0,0,.04)}
.docs .paglora-card-title{margin:0 0 10px;color:var(--fg);font-size:16px;font-weight:750;line-height:1.4}
.docs .paglora-card>:last-child{margin-bottom:0}
.docs .paglora-card a:focus-visible{outline:2px solid var(--brand);outline-offset:3px}
.docs blockquote p:last-child{margin-bottom:0}

/* Consecutive Markdown cards become a group. The opt-in website setting
   chooses a true responsive grid; older projects remain vertically stacked.
   min(100%,...) prevents overflow within narrow article columns. */
.docs .paglora-card-grid{display:block;margin:24px 0}
:root[data-card-layout="grid"] .docs .paglora-card-grid{
  display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,245px),1fr));gap:16px;align-items:stretch
}
:root[data-card-layout="grid"] .docs .paglora-card-grid>.paglora-card{
  min-width:0;margin:0;padding:23px 24px;
  background:linear-gradient(145deg,color-mix(in srgb,var(--brand) 4%,var(--bg)),var(--subtle));
  transition:border-color .18s ease,box-shadow .18s ease,transform .18s ease
}
:root[data-card-layout="grid"] .docs .paglora-card-grid>.paglora-card:hover{
  border-color:color-mix(in srgb,var(--brand) 45%,var(--line));
  box-shadow:0 8px 26px rgba(0,0,0,.08);transform:translateY(-2px)
}
:root[data-card-layout="grid"] .docs .paglora-card-grid>.paglora-card .paglora-card-title{font-size:16px;letter-spacing:-.025em}
:root[data-card-layout="grid"] .docs .paglora-card-grid>.paglora-card a{overflow-wrap:anywhere}
@media(prefers-reduced-motion:reduce){
  :root[data-card-layout="grid"] .docs .paglora-card-grid>.paglora-card{transition:none}
  :root[data-card-layout="grid"] .docs .paglora-card-grid>.paglora-card:hover{transform:none}
}
/* Semantic Markdown steps: no JavaScript, portable to static HTML and safe previews. */
.docs .paglora-steps{margin:28px 0}
.docs .paglora-steps-title{font-size:17px;font-weight:740;letter-spacing:-.025em;margin-bottom:18px;color:var(--fg)}
.docs .paglora-steps-list{list-style:none;counter-reset:paglora-step;margin:0;padding:0}
.docs .paglora-steps-list>li{position:relative;counter-increment:paglora-step;margin:0;margin-inline-start:16px;padding:0;padding-inline-start:35px;padding-bottom:27px;border-inline-start:2px solid var(--line);min-height:50px}
.docs .paglora-steps-list>li:last-child{border-inline-start-color:transparent;padding-bottom:0}
.docs .paglora-steps-list>li::before{content:counter(paglora-step);position:absolute;inset-inline-start:-17px;top:0;display:grid;place-items:center;width:32px;height:32px;border:2px solid var(--bg);border-radius:50%;background:var(--brand);color:#fff;font-size:12px;font-weight:750;line-height:1}
.docs .paglora-steps-list>li>:last-child{margin-bottom:0}


/* Source-owned status badges: no script, custom accent and dark mode aware. */
.docs .paglora-badges{margin:22px 0;min-width:0}
.docs .paglora-badges-title{margin-bottom:10px;color:var(--fg);font-size:13px;font-weight:700}
.docs .paglora-badges-row{display:flex;flex-wrap:wrap;align-items:center;gap:8px}
.docs .paglora-badge{--badge-color:var(--fg);display:inline-flex;align-items:center;max-width:100%;padding:3px 11px;border:1px solid color-mix(in srgb,var(--badge-color) 35%,var(--line));border-radius:999px;background:color-mix(in srgb,var(--badge-color) 8%,var(--bg));color:var(--badge-color);font-size:12px;font-weight:680;line-height:1.55;overflow-wrap:anywhere}
.docs .paglora-badge-info{--badge-color:var(--brand)}
.docs .paglora-badge-success{--badge-color:#137a4c}
.docs .paglora-badge-warning{--badge-color:#906100}
.docs .paglora-badge-danger{--badge-color:#ad3549}
:root[data-color-mode="dark"] .docs .paglora-badge-success{--badge-color:#7bd8ac}
:root[data-color-mode="dark"] .docs .paglora-badge-warning{--badge-color:#f4c36b}
:root[data-color-mode="dark"] .docs .paglora-badge-danger{--badge-color:#ff9dad}
@media(prefers-color-scheme:dark){
  :root:not([data-color-mode="light"]) .docs .paglora-badge-success{--badge-color:#7bd8ac}
  :root:not([data-color-mode="light"]) .docs .paglora-badge-warning{--badge-color:#f4c36b}
  :root:not([data-color-mode="light"]) .docs .paglora-badge-danger{--badge-color:#ff9dad}
}

/* Code groups: every example remains readable with scripts disabled.
   The browser runtime progressively adds accessible, keyboard-driven tabs. */
.docs .paglora-code-group{margin:28px 0;min-width:0}
.docs .paglora-code-group-title{font-size:15px;font-weight:720;letter-spacing:-.015em;margin-bottom:12px}
.docs .paglora-code-group-panels{min-width:0;border:1px solid var(--line);border-radius:12px;overflow:hidden;background:var(--subtle)}
.docs .paglora-code-group-item{min-width:0}
.docs .paglora-code-group-item+.paglora-code-group-item{border-top:1px solid var(--line)}
.docs .paglora-code-group-label{padding:10px 16px;color:var(--muted);font-size:12px;font-weight:650}
.docs .paglora-code-group-item>pre{margin:0;border:0;border-radius:0}
.docs .paglora-code-group-tabs{display:flex;gap:4px;overflow-x:auto;padding:7px 9px;border:1px solid var(--line);border-bottom:0;border-radius:12px 12px 0 0;background:var(--subtle);scrollbar-width:thin}
.docs .paglora-code-group-tabs>button{flex:0 0 auto;appearance:none;border:0;border-bottom:2px solid transparent;border-radius:5px 5px 0 0;background:transparent;padding:8px 12px;color:var(--muted);font:inherit;font-size:12px;font-weight:650;cursor:pointer}
.docs .paglora-code-group-tabs>button:hover{color:var(--fg);background:color-mix(in srgb,var(--brand) 6%,transparent)}
.docs .paglora-code-group-tabs>button[aria-selected="true"]{border-bottom-color:var(--brand);color:var(--fg)}
.docs .paglora-code-group-tabs>button:focus-visible{outline:2px solid var(--brand);outline-offset:-2px}
.docs .paglora-code-group-enhanced .paglora-code-group-panels{border-radius:0 0 12px 12px}
.docs .paglora-code-group-enhanced .paglora-code-group-label{display:none}
.docs .paglora-code-group-enhanced .paglora-code-group-item[hidden]{display:none}
.docs .paglora-code-group-enhanced .paglora-code-group-item+.paglora-code-group-item{border-top:0}

/* Content tabs use nested Markdown blockquotes. In script-disabled previews
   and static HTML without JS, every labeled panel stays visible and readable. */
.docs .paglora-tabs{min-width:0;margin:28px 0}
.docs .paglora-tabs-title{font-size:15px;font-weight:720;letter-spacing:-.015em;margin-bottom:12px;color:var(--fg)}
.docs .paglora-tabs-panels{min-width:0;border:1px solid var(--line);border-radius:12px;overflow:hidden;background:var(--bg)}
.docs .paglora-tabs-item{min-width:0;padding:17px 20px}
.docs .paglora-tabs-item+.paglora-tabs-item{border-top:1px solid var(--line)}
.docs .paglora-tabs-item>:last-child{margin-bottom:0}
.docs .paglora-tabs-label{margin-bottom:10px;color:var(--muted);font-size:12px;font-weight:700}
.docs .paglora-tabs-tablist{display:flex;gap:4px;overflow-x:auto;padding:7px 9px;border:1px solid var(--line);border-bottom:0;border-radius:12px 12px 0 0;background:var(--subtle);scrollbar-width:thin}
.docs .paglora-tabs-tablist>button{flex:0 0 auto;appearance:none;border:0;border-bottom:2px solid transparent;border-radius:5px 5px 0 0;background:transparent;padding:8px 12px;color:var(--muted);font:inherit;font-size:13px;font-weight:650;cursor:pointer}
.docs .paglora-tabs-tablist>button:hover{color:var(--fg);background:color-mix(in srgb,var(--brand) 6%,transparent)}
.docs .paglora-tabs-tablist>button[aria-selected="true"]{border-bottom-color:var(--brand);color:var(--fg)}
.docs .paglora-tabs-tablist>button:focus-visible{outline:2px solid var(--brand);outline-offset:-2px}
.docs .paglora-tabs-enhanced .paglora-tabs-panels{border-radius:0 0 12px 12px}
.docs .paglora-tabs-enhanced .paglora-tabs-label{display:none}
.docs .paglora-tabs-enhanced .paglora-tabs-item[hidden]{display:none}
.docs .paglora-tabs-enhanced .paglora-tabs-item+.paglora-tabs-item{border-top:0}

/* Native FAQ accordions: safe Markdown blockquotes become accessible details.
   They work in the script-disabled Studio preview and offline static export. */
.docs .paglora-accordion{margin:20px 0;border:1px solid var(--line);border-radius:11px;background:var(--bg);overflow:hidden}
.docs .paglora-accordion>summary{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:15px 18px;list-style:none;cursor:pointer;font-size:14px;font-weight:680;line-height:1.55;color:var(--fg)}
.docs .paglora-accordion>summary::-webkit-details-marker{display:none}
.docs .paglora-accordion>summary::after{content:"+";flex:0 0 auto;font-size:20px;font-weight:450;line-height:1;color:var(--muted)}
.docs .paglora-accordion[open]>summary::after{content:"−"}
.docs .paglora-accordion[open]>summary{border-bottom:1px solid var(--line);background:var(--subtle)}
.docs .paglora-accordion>summary:hover{background:var(--hover)}
.docs .paglora-accordion>summary:focus-visible{outline:2px solid var(--brand);outline-offset:-3px}
.docs .paglora-accordion>:not(summary){margin-inline:18px}
.docs .paglora-accordion>:last-child:not(summary){margin-bottom:18px}
.docs .paglora-accordion-group{margin:24px 0;border:1px solid var(--line);border-radius:12px;overflow:hidden}
.docs .paglora-accordion-group>.paglora-accordion{margin:0;border:0;border-radius:0}
.docs .paglora-accordion-group>.paglora-accordion+.paglora-accordion{border-top:1px solid var(--line)}

/* Markdown callouts — sanitized in the shared preview/build pipeline. */
.docs .paglora-callout{--callout-accent:var(--brand);position:relative;display:block;margin:25px 0;padding:16px 19px 17px;border:1px solid color-mix(in srgb,var(--callout-accent) 38%,var(--line));border-left:3px solid var(--callout-accent);border-radius:11px;background:color-mix(in srgb,var(--callout-accent) 7%,var(--bg));box-shadow:0 1px 2px rgba(0,0,0,.025)}
.docs .paglora-callout-title{display:flex;align-items:center;gap:9px;margin:0 0 6px;color:var(--fg);font-size:13px;font-weight:730;line-height:1.5}
.docs .paglora-callout-title::before{content:"i";display:grid;place-items:center;flex:0 0 18px;width:18px;height:18px;border-radius:50%;background:var(--callout-accent);color:#fff;font-size:12px;font-family:system-ui,sans-serif;font-style:normal;font-weight:750}
.docs .paglora-callout > :last-child{margin-bottom:0}
.docs .paglora-callout p{margin:0 0 10px}
.docs .paglora-callout ul,.docs .paglora-callout ol{margin:5px 0 10px}
.docs .paglora-callout-tip{--callout-accent:#168950}
.docs .paglora-callout-important{--callout-accent:#805bd4}
.docs .paglora-callout-warning{--callout-accent:#b67909}
.docs .paglora-callout-caution{--callout-accent:#c14454}
.docs .paglora-callout-tip .paglora-callout-title::before{content:"✓"}
.docs .paglora-callout-important .paglora-callout-title::before{content:"!"}
.docs .paglora-callout-warning .paglora-callout-title::before,.docs .paglora-callout-caution .paglora-callout-title::before{content:"!"}

.docs pre{position:relative;overflow:auto;border-radius:12px;background:var(--code-bg);color:var(--code-fg);padding:22px 23px;line-height:1.65;font-size:13px;border:1px solid var(--line);tab-size:2}
.docs code{font-family:"Cascadia Code","JetBrains Mono",ui-monospace,SFMono-Regular,Consolas,monospace}
.docs :not(pre)>code{background:var(--subtle);border:1px solid var(--line);color:var(--fg);padding:2px 5px;border-radius:5px;font-size:.86em}
.docs pre code{color:inherit}
.docs pre.paglora-has-copy{padding-top:52px}
.docs .paglora-copy-code{position:absolute;top:10px;right:12px;z-index:1;appearance:none;padding:5px 10px;border:1px solid #4d5664;border-radius:6px;background:#253041;color:#e9eef8;font:inherit;font-size:11px;line-height:1.4;cursor:pointer}
.docs .paglora-copy-code:hover{background:#354359}
.docs .paglora-copy-code:focus-visible{outline:2px solid var(--brand);outline-offset:2px}
.docs .paglora-heading-link{margin-left:12px;display:inline-block;font-size:.7em;font-weight:550;color:var(--brand);text-decoration:none;opacity:0;vertical-align:baseline}
.docs :is(h2,h3,h4):hover .paglora-heading-link,.docs .paglora-heading-link:focus-visible{opacity:1}
@media(hover:none){.docs .paglora-heading-link{opacity:.8}}
.docs .paglora-heading-link:focus-visible{outline:2px solid var(--brand);outline-offset:3px}
.docs a{color:var(--brand);text-decoration-color:color-mix(in srgb,var(--brand) 40%,transparent);text-decoration-thickness:1px;text-underline-offset:3px}
.docs a:hover{text-decoration-color:var(--brand)}
.docs img{display:block;max-width:100%;height:auto;border-radius:12px;border:1px solid var(--line);margin:22px 0}
.docs table{border-collapse:separate;border-spacing:0;width:100%;display:block;overflow-x:auto;border:1px solid var(--line);border-radius:10px;margin:25px 0}
.docs th,.docs td{padding:12px 16px;border-right:1px solid var(--line);border-bottom:1px solid var(--line);text-align:left;white-space:normal;min-width:130px}
.docs th{background:var(--subtle);font-weight:700}
.docs tr:last-child td{border-bottom:0}
.docs th:last-child,.docs td:last-child{border-right:0}
.docs hr{border:0;border-top:1px solid var(--line);margin:38px 0}
.docs :is(h2,h3,h4) a{color:inherit}
.on-this-page{grid-column:3;min-width:0;padding:35px clamp(18px,2.2vw,32px) 40px 14px}
.toc-inner{position:sticky;top:105px;max-height:calc(100vh - 125px);overflow-y:auto;scrollbar-width:thin}
.toc-inner h2{margin:0 0 13px;font-size:11px;text-transform:uppercase;letter-spacing:.09em;color:var(--muted);font-weight:750}
.toc-inner a{display:block;text-decoration:none;color:var(--muted);font-size:12px;line-height:1.6;padding:6px 9px;border-left:1px solid var(--line);overflow-wrap:anywhere}
.toc-inner a:hover{border-left:2px solid var(--brand);color:var(--fg);background:var(--subtle)}
.toc-inner a.toc-h3{padding-left:20px;font-size:11.5px}
.mobile-toc{display:none}
.page-pager{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px;margin-top:64px;padding-top:24px;border-top:1px solid var(--line)}
.page-pager a{display:flex;flex-direction:column;gap:4px;min-height:86px;padding:15px 17px;border:1px solid var(--line);border-radius:11px;text-decoration:none;transition:border-color .15s,background .15s}
.page-pager a:hover{border-color:var(--brand);background:var(--subtle)}
.page-pager .next{grid-column:2;text-align:right}
.page-pager small{font-size:11px;color:var(--muted)}
.page-pager span{font-size:13px;font-weight:650;color:var(--fg)}
.site-footer{border-top:1px solid var(--line);background:var(--subtle)}
.site-footer-inner{max-width:1500px;margin:auto;padding:25px clamp(22px,3vw,48px);font-size:12px;color:var(--muted);display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:20px}
.footer-links{display:flex;flex-wrap:wrap;gap:17px}
.footer-links a{text-decoration:none}
.site-body.nav-right{grid-template-columns:minmax(190px,235px) minmax(0,1fr) minmax(230px,260px)}
.site-body.nav-right .site-nav{grid-column:3;grid-row:1;border-right:0;border-left:1px solid var(--line);padding:32px clamp(18px,2.5vw,42px) 40px 16px}
.site-body.nav-right .site-main{grid-column:2;grid-row:1}
.site-body.nav-right .on-this-page{grid-column:1;grid-row:1;padding-left:clamp(18px,2.2vw,32px);padding-right:14px}
.site-body.nav-top{grid-template-columns:minmax(0,1fr) minmax(190px,235px)}
.site-body.nav-top .site-nav{grid-column:1/-1;grid-row:1;border:0;border-bottom:1px solid var(--line);padding:10px clamp(22px,3vw,48px)}
.site-body.nav-top .site-nav-inner{position:static;max-height:none;display:flex;gap:6px;align-items:center;flex-wrap:wrap;overflow:visible}
.site-body.nav-top .site-nav h2{margin:0 8px 0 0;padding:0}
.site-body.nav-top .site-nav a{white-space:nowrap;margin:0}
.site-body.nav-top .site-main{grid-column:1;grid-row:2}
.site-body.nav-top .on-this-page{grid-column:2;grid-row:2}
@media(max-width:1180px){.site-body,.site-body.nav-right{grid-template-columns:245px minmax(0,1fr)}.site-body.nav-right{grid-template-columns:minmax(0,1fr) 245px}.site-body .on-this-page{display:none}.site-main{padding-inline:clamp(28px,6vw,78px)}.site-body.nav-top{grid-template-columns:1fr}.site-body.nav-top .site-main{grid-column:1}}
@media(max-width:800px){
  html{scroll-padding-top:78px}.top-bar{min-height:64px;padding-inline:20px}.brand{font-size:16px}.brand-logo{height:30px}
  .header-links{gap:12px}.header-links a{font-size:12px}
  .site-body,.site-body.nav-right,.site-body.nav-top{display:block;min-height:calc(100vh - 145px)}
  .site-body .site-nav{display:none}
  .site-main,.site-body.nav-right .site-main,.site-body.nav-top .site-main{width:100%;padding:34px 22px 68px}
  /* The native details element remains usable with JavaScript disabled.
     The open navigation floats over the article instead of shifting it. */
  .mobile-nav{display:block;position:sticky;top:0;z-index:29;max-width:100%;border-bottom:1px solid var(--line);padding:8px 20px;background:var(--bg)}
  .site-header:not(.not-sticky)+.mobile-nav{top:var(--paglora-header-height,64px)}
  .mobile-nav[open]{border-color:var(--brand)}
  .mobile-nav summary{cursor:pointer;list-style:none;display:flex;align-items:center;justify-content:space-between;min-height:44px;font-size:13px;font-weight:650;padding:5px 0}
  .mobile-nav summary::-webkit-details-marker{display:none}
  .mobile-nav summary::after{content:"+";font-size:19px;color:var(--muted)}.mobile-nav[open] summary::after{content:"−"}
  .mobile-nav summary:focus-visible{outline:2px solid var(--brand);outline-offset:3px}
  .mobile-nav-inner{position:absolute;top:100%;inset-inline:0;display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,220px),1fr));align-items:start;gap:8px;padding:16px 20px 22px;max-height:65vh;max-height:min(65vh,calc(100dvh - var(--paglora-header-height,64px) - 64px));overflow-y:auto;overscroll-behavior:contain;background:var(--bg);border-bottom:1px solid var(--line);box-shadow:0 18px 36px rgba(0,0,0,.17)}
  .mobile-nav:not([open]) .mobile-nav-inner{display:none}
  .mobile-nav a{display:block;padding:9px 11px;border-radius:8px;text-decoration:none;font-size:13px;color:var(--muted);overflow-wrap:anywhere}
  .mobile-nav a:hover{background:var(--hover);color:var(--fg)}
  .mobile-nav a:focus-visible{outline:2px solid var(--brand);outline-offset:2px}
  .mobile-nav a[aria-current=page]{background:var(--brand-light);color:var(--brand);font-weight:650}
  .mobile-toc{display:block;margin:0 0 30px;border:1px solid var(--line);border-radius:10px;background:var(--subtle);padding:8px 15px}
  .mobile-toc summary{cursor:pointer;font-size:12px;font-weight:650;padding:6px 0}
  .mobile-toc a{display:block;font-size:12px;padding:4px 6px;text-decoration:none;color:var(--muted)}
  .docs h1{font-size:2.15rem}.docs h2{font-size:1.45rem}.page-pager{gap:10px}.page-pager a{padding:12px}
}
@media(max-width:460px){.top-bar{gap:12px}.header-links a:nth-child(n+3){display:none}.page-pager{display:flex;flex-direction:column}.page-pager a.next{align-items:flex-end}}

/* Per-page public layout modes. Source CSS in theme/custom.css may override these. */
.site-body.layout-wide .on-this-page{display:none}
.site-body.layout-wide .site-main{grid-column:2/4}
.site-body.nav-right.layout-wide .site-main{grid-column:1/3}
.site-body.nav-top.layout-wide .site-main{grid-column:1/-1}
.layout-wide .article-wrap{max-width:1100px}
.layout-wide .docs{font-size:16px}
.site-body.layout-landing{display:block}
.site-body.layout-landing .site-nav,.site-body.layout-landing .on-this-page{display:none}
.layout-landing .site-main{width:100%;padding:76px clamp(24px,5vw,86px) 110px}
.layout-landing .article-wrap{max-width:1150px}
.layout-landing .page-heading,.layout-landing .page-pager{display:none}
.layout-landing .docs>h1:first-child{max-width:950px;margin:16px auto 28px;text-align:center;font-size:clamp(2.8rem,6vw,5.1rem);font-weight:770;line-height:1.06;letter-spacing:-.07em}
.layout-landing .docs>h1:first-child + p{max-width:700px;margin:0 auto 58px;text-align:center;color:var(--muted);font-size:clamp(1.05rem,1.7vw,1.32rem);line-height:1.7}
.layout-landing .docs h2{margin-top:68px;font-size:2rem}
.page-layout-landing .mobile-nav{display:none}
@media(max-width:1180px){
  .site-body.layout-wide .site-main,.site-body.nav-right.layout-wide .site-main{grid-column:2}
  .site-body.nav-right.layout-wide .site-main{grid-column:1}
  .site-body.nav-top.layout-wide .site-main{grid-column:1}
}
@media(max-width:800px){
  .site-body.layout-wide,.site-body.layout-landing{display:block}
  .site-body.layout-wide .site-main,.site-body.nav-right.layout-wide .site-main,.site-body.nav-top.layout-wide .site-main{display:block;width:100%}
  .layout-landing .site-main{padding:48px 22px 74px}
  .layout-landing .docs>h1:first-child{font-size:clamp(2.5rem,9vw,3.6rem)}
  .layout-landing .docs>h1:first-child + p{margin-bottom:42px}
}
@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}*,*::before,*::after{transition:none!important}}

/* Arabic documents use right-to-left reading order, including navigation and callouts. */
:root[dir="rtl"] .site-nav{border-right:0;border-left:1px solid var(--line);padding-right:clamp(18px,2.5vw,42px);padding-left:16px}
:root[dir="rtl"] .site-body.nav-right .site-nav{border-left:0;border-right:1px solid var(--line);padding-right:16px;padding-left:clamp(18px,2.5vw,42px)}
:root[dir="rtl"] .docs ul,:root[dir="rtl"] .docs ol{padding-left:0;padding-right:1.55em}
:root[dir="rtl"] .docs .paglora-task-checkbox{margin:0 -1.5em 0 .55em}
:root[dir="rtl"] .docs blockquote{border-left:0;border-right:3px solid var(--brand);border-radius:10px 0 0 10px}
:root[dir="rtl"] .docs .paglora-callout{border-left:1px solid var(--line);border-right:3px solid var(--callout-accent)}
:root[dir="rtl"] .docs .paglora-heading-link{margin-left:0;margin-right:12px}
:root[dir="rtl"] .docs th,:root[dir="rtl"] .docs td{text-align:right}
:root[dir="rtl"] .toc-inner a{border-left:0;border-right:1px solid var(--line)}
:root[dir="rtl"] .toc-inner a.toc-h3{padding-left:9px;padding-right:20px}
:root[dir="rtl"] .page-pager .next{text-align:left}
@media(max-width:800px){:root[dir="rtl"] .site-nav,:root[dir="rtl"] .site-body.nav-right .site-nav{border:0}}

/* Site layouts: additional project-owned presentations, no framework runtime.
   Classic remains byte-for-byte compatible with previous site designs. */
:root[data-site-layout="inset"] body{background:var(--subtle)}
:root[data-site-layout="inset"] .site-header{top:12px;max-width:min(1460px,calc(100% - 32px));margin:16px auto 0;border:1px solid var(--line);border-radius:16px;box-shadow:0 7px 28px rgba(0,0,0,.045)}
:root[data-site-layout="inset"] .site-header.not-sticky{top:auto}
:root[data-site-layout="inset"] .top-bar{min-height:66px}
:root[data-site-layout="inset"] .site-body{max-width:1460px;margin:14px auto 32px;border:1px solid var(--line);border-radius:16px;background:var(--bg);box-shadow:0 18px 65px rgba(0,0,0,.045);overflow:clip}
:root[data-site-layout="inset"] .site-nav{background:color-mix(in srgb,var(--subtle) 63%,var(--bg))}
:root[data-site-layout="inset"] .site-footer{background:transparent}
:root[data-site-layout="inset"] .site-footer-inner{max-width:1420px}
:root[data-site-layout="reading"] .site-header{border-bottom:1px solid var(--line)}
:root[data-site-layout="reading"] .top-bar{max-width:1330px;min-height:62px}
:root[data-site-layout="reading"] .site-body{max-width:1330px;grid-template-columns:minmax(195px,235px) minmax(0,1fr) minmax(155px,205px)}
:root[data-site-layout="reading"] .site-nav{border-right:0;padding-top:43px}
:root[data-site-layout="reading"] .site-main{padding-top:66px}
:root[data-site-layout="reading"] .docs{font-size:16px;line-height:1.9}
:root[data-site-layout="reading"] .docs h1{letter-spacing:-.055em}
:root[data-site-layout="reading"] .toc-inner a{border-left-color:transparent}
:root[data-site-layout="reading"] .site-body.nav-right{grid-template-columns:minmax(155px,205px) minmax(0,1fr) minmax(195px,235px)}
:root[data-site-layout="reading"] .site-body.nav-right .site-nav{border-left:0}
:root[data-site-layout="reading"] .site-body.nav-top{grid-template-columns:minmax(0,1fr) minmax(155px,205px)}
:root[data-site-layout="reading"] .site-body.nav-top .site-nav{border-bottom:1px solid var(--line)}
/* Each palette supplies a complete background/text/border/code family for both
   appearances. The independent custom accent is applied later in the head. */
:root[data-site-palette="graphite"]{--bg:#ffffff;--fg:#191a1d;--muted:#62646b;--line:#e4e5e8;--subtle:#f6f6f7;--hover:#ebebee;--code-bg:#18191d;--code-fg:#f7f7f8;--header-bg:rgba(255,255,255,.96)}
:root[data-site-palette="lavender"]{--bg:#fdfcff;--fg:#282035;--muted:#736780;--line:#e8e2f1;--subtle:#f7f3fc;--hover:#efe8f8;--code-bg:#231b32;--code-fg:#f4eefc;--header-bg:rgba(253,252,255,.96)}
:root[data-site-palette="linen"]{--bg:#fffdf9;--fg:#29251f;--muted:#736c61;--line:#eae5db;--subtle:#faf6ed;--hover:#f1eadc;--code-bg:#2b2925;--code-fg:#f8f3e9;--header-bg:rgba(255,253,249,.96)}
:root[data-site-palette="graphite"][data-color-mode="dark"]{--bg:#101114;--fg:#f0f0f2;--muted:#a7a8af;--line:#2d3036;--subtle:#191b20;--hover:#26282f;--code-bg:#090a0c;--code-fg:#eeeef1;--header-bg:rgba(16,17,20,.96)}
:root[data-site-palette="lavender"][data-color-mode="dark"]{--bg:#17121e;--fg:#f4eefa;--muted:#b5a9c1;--line:#362b44;--subtle:#231a2f;--hover:#32253e;--code-bg:#110d17;--code-fg:#f4eefa;--header-bg:rgba(23,18,30,.96)}
:root[data-site-palette="linen"][data-color-mode="dark"]{--bg:#1c1915;--fg:#f5eee4;--muted:#b6a99b;--line:#3b352d;--subtle:#272119;--hover:#373026;--code-bg:#110f0d;--code-fg:#f7f0e9;--header-bg:rgba(28,25,21,.96)}
@media(prefers-color-scheme:dark){
:root[data-site-palette="graphite"]:not([data-color-mode="light"]){--bg:#101114;--fg:#f0f0f2;--muted:#a7a8af;--line:#2d3036;--subtle:#191b20;--hover:#26282f;--code-bg:#090a0c;--code-fg:#eeeef1;--header-bg:rgba(16,17,20,.96)}
:root[data-site-palette="lavender"]:not([data-color-mode="light"]){--bg:#17121e;--fg:#f4eefa;--muted:#b5a9c1;--line:#362b44;--subtle:#231a2f;--hover:#32253e;--code-bg:#110d17;--code-fg:#f4eefa;--header-bg:rgba(23,18,30,.96)}
:root[data-site-palette="linen"]:not([data-color-mode="light"]){--bg:#1c1915;--fg:#f5eee4;--muted:#b6a99b;--line:#3b352d;--subtle:#272119;--hover:#373026;--code-bg:#110f0d;--code-fg:#f7f0e9;--header-bg:rgba(28,25,21,.96)}
}
@media(max-width:1180px){
:root[data-site-layout="reading"] .site-body,:root[data-site-layout="reading"] .site-body.nav-right{grid-template-columns:230px minmax(0,1fr)}
:root[data-site-layout="reading"] .site-body.nav-right{grid-template-columns:minmax(0,1fr) 230px}
:root[data-site-layout="reading"] .site-body.nav-top{grid-template-columns:1fr}
}
@media(max-width:800px){
:root[data-site-layout="inset"] .site-header{top:0;max-width:none;margin:0;border-radius:0;border-inline:0}
:root[data-site-layout="inset"] .site-body{margin:0;border:0;border-radius:0;box-shadow:none}
:root[data-site-layout="reading"] .site-body,:root[data-site-layout="reading"] .site-body.nav-right,:root[data-site-layout="reading"] .site-body.nav-top{display:block}
:root[data-site-layout="reading"] .site-main{padding-top:30px}
}

/* Browser Print / Save as PDF: readable public article, no application chrome.
   Use the same rules for every palette, layout and screen width. */
@page{margin:15mm}
@media print{
  :root,:root[data-color-mode="dark"],:root[data-site-palette]{
    color-scheme:light!important;--bg:#fff!important;--fg:#171717!important;
    --muted:#505050!important;--line:#d4d4d4!important;--subtle:#f7f7f7!important;
    --brand:#155d9b!important;--code-bg:#f7f7f7!important;--code-fg:#171717!important
  }
  html{scroll-behavior:auto!important;scroll-padding-top:0!important}
  body,:root[data-site-layout="inset"] body{background:#fff!important;color:#171717!important}
  .site-header,.mobile-nav,.site-nav,.on-this-page,.mobile-toc,.site-footer,
  .page-pager,.skip-link,.paglora-search-backdrop,.paglora-search-trigger,
  .paglora-appearance-toggle,.paglora-copy-code,.paglora-heading-link,
  .paglora-reading-progress,.paglora-back-to-top{display:none!important}
  :root .site-body,:root .site-body.nav-right,:root .site-body.nav-top,
  :root .site-body.layout-landing{
    display:block!important;width:100%!important;max-width:none!important;
    min-height:0!important;margin:0!important;border:0!important;
    border-radius:0!important;box-shadow:none!important;overflow:visible!important;
    background:#fff!important
  }
  :root .site-main,:root .site-body.nav-right .site-main,
  :root .site-body.nav-top .site-main,:root .layout-landing .site-main{
    display:block!important;width:100%!important;max-width:none!important;
    margin:0!important;padding:0!important;grid-column:auto!important
  }
  :root .article-wrap,:root .layout-wide .article-wrap,
  :root .layout-landing .article-wrap{
    width:100%!important;max-width:none!important;margin:0!important
  }
  .page-heading{margin:0 0 14pt!important;padding-bottom:7pt!important;
    border-bottom:1px solid #d4d4d4!important;color:#505050!important;font-size:9pt!important}
  .docs{font-size:10.5pt!important;line-height:1.55!important;
    color:#171717!important;overflow:visible!important}
  .docs h1,.docs h2,.docs h3,.docs h4{
    color:#171717!important;break-after:avoid-page;page-break-after:avoid;
    scroll-margin:0!important
  }
  .docs h1{font-size:23pt!important;margin:0 0 15pt!important}
  .docs h2{font-size:16pt!important;margin-top:22pt!important}
  .docs h3{font-size:13pt!important}
  .docs p,.docs li{orphans:3;widows:3}
  .docs a{color:#171717!important;text-decoration:underline!important}
  .docs img{max-width:100%!important;max-height:230mm!important;
    object-fit:contain!important;break-inside:avoid-page;page-break-inside:avoid}
  .docs pre,.docs pre.paglora-has-copy{
    white-space:pre-wrap!important;overflow:visible!important;
    overflow-wrap:anywhere!important;word-break:break-word!important;
    background:#f7f7f7!important;color:#171717!important;
    border:1px solid #d4d4d4!important;box-shadow:none!important;
    padding:10pt!important;break-inside:auto!important
  }
  .docs pre code{color:#171717!important;white-space:inherit!important}
  .docs table{display:table!important;width:100%!important;max-width:100%!important;
    overflow:visible!important;border-collapse:collapse!important}
  .docs th,.docs td{min-width:0!important;padding:6pt!important;overflow-wrap:anywhere!important}
  .docs .paglora-card,.docs .paglora-callout,.docs .paglora-steps-list>li{
    break-inside:avoid-page;page-break-inside:avoid
  }
  /* The runtime reveals all panels before print; these rules also handle
     browsers that honor author print CSS over the hidden attribute. */
  .docs .paglora-code-group-item[hidden],.docs .paglora-tabs-item[hidden]{display:block!important}
  .docs .paglora-code-group-tabs,.docs .paglora-tabs-tablist{display:none!important}
  .docs .paglora-code-group-enhanced .paglora-code-group-label,
  .docs .paglora-tabs-enhanced .paglora-tabs-label{display:block!important}
  .docs .paglora-code-group-item,.docs .paglora-tabs-item{
    break-inside:avoid-page;page-break-inside:avoid
  }
}

"#;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteLink {
    label: String,
    url: String,
}

fn nav_default() -> String { "left".into() }
fn align_default() -> String { "left".into() }
fn true_default() -> bool { true }
fn width_default() -> u16 { 790 }
fn theme_default() -> String { "system".into() }
fn language_default() -> String { "en".into() }
fn accent_default() -> String { "#2196f3".into() }
fn site_layout_default() -> String { "classic".into() }
fn site_palette_default() -> String { "original".into() }
fn card_layout_default() -> String { "stacked".into() }

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteSettings {
    title: String,
    description: String,
    footer_text: String,
    // Optional canonical site root, including a GitHub Pages repository path.
    #[serde(default)]
    site_url: String,
    #[serde(default = "nav_default")]
    navigation_position: String,
    #[serde(default = "align_default")]
    header_alignment: String,
    #[serde(default = "true_default")]
    sticky_header: bool,
    #[serde(default = "true_default")]
    show_footer: bool,
    #[serde(default = "width_default")]
    content_width: u16,
    #[serde(default)]
    logo_url: String,
    #[serde(default)]
    favicon_url: String,
    #[serde(default)]
    header_links: Vec<SiteLink>,
    #[serde(default)]
    footer_links: Vec<SiteLink>,
    #[serde(default = "theme_default")]
    theme_mode: String,
    #[serde(default = "accent_default")]
    accent_color: String,
    // Optional visual theme settings; old site.json files keep their design.
    #[serde(default = "site_layout_default")]
    site_layout: String,
    #[serde(default = "site_palette_default")]
    site_palette: String,
    // Card layout is independent of the site's shell and neutral palette.
    #[serde(default = "card_layout_default")]
    card_layout: String,
    #[serde(default = "language_default")]
    language: String,
    // Optional remote font on the published site. Blank keeps the default Geist.
    #[serde(default)]
    google_font_family: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPage {
    id: String,
    html: String,
    #[serde(default)]
    toc: Vec<TocHeading>,
    #[serde(default)]
    search_text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocHeading {
    id: String,
    title: String,
    level: u8,
}

/// Summary shown to the author after the export is safely installed.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteBuildResult {
    path: String,
    public_pages: usize,
    private_pages: usize,
    local_images: usize,
    warnings: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageManifest {
    format_version: u8,
    pages: Vec<crate::pages::Page>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 &&
        id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn project_folder(app: &AppHandle, project_id: &str) -> Result<(PathBuf, String), String> {
    if !valid_id(project_id) { return Err("Invalid project identifier".into()); }
    let dir = crate::storage::projects_root(app)?.join(project_id);
    let data: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir.join("paglora.json")).map_err(|e| e.to_string())?
    ).map_err(|e| e.to_string())?;
    if data["id"].as_str() != Some(project_id) { return Err("Project configuration mismatch".into()); }
    let name = data["name"].as_str().ok_or("Project name missing")?.to_string();
    Ok((dir, name))
}

fn default_settings(name: &str) -> SiteSettings {
    SiteSettings {
        title: name.into(),
        description: format!("Documentation for {name}"),
        footer_text: format!("{name} · Built with Paglora"),
        site_url: String::new(),
        navigation_position: nav_default(),
        header_alignment: align_default(),
        sticky_header: true,
        show_footer: true,
        content_width: width_default(),
        logo_url: String::new(),
        favicon_url: String::new(),
        header_links: Vec::new(),
        footer_links: Vec::new(),
        theme_mode: theme_default(),
        accent_color: accent_default(),
        site_layout: site_layout_default(),
        site_palette: site_palette_default(),
        card_layout: card_layout_default(),
        language: language_default(),
        google_font_family: String::new(),
    }
}

// Preserve valid settings when Windows is interrupted between rename steps.
const SETTINGS_STAGED: &str = "site.json.tmp";
const SETTINGS_BACKUP: &str = "site.json.previous";

fn regular_settings_file(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err("Site settings path must be a regular file".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

fn read_settings(dir: &Path, name: &str) -> Result<SiteSettings, String> {
    let current = dir.join(CONFIG);
    let backup = dir.join(SETTINGS_BACKUP);
    let path = if regular_settings_file(&current)? {
        current
    } else if regular_settings_file(&backup)? {
        backup
    } else {
        return Ok(default_settings(name));
    };
    serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

// Write a complete staged JSON file before replacing the last good copy.
// Retain the backup through Windows' non-atomic two-rename window.
fn write_settings(dir: &Path, settings: &SiteSettings) -> Result<(), String> {
    let current = dir.join(CONFIG);
    let staged = dir.join(SETTINGS_STAGED);
    let backup = dir.join(SETTINGS_BACKUP);
    let has_current = regular_settings_file(&current)?;
    regular_settings_file(&staged)?;
    let has_backup = regular_settings_file(&backup)?;
    let json = format!("{}\n", serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?);
    fs::write(&staged, json).map_err(|e| e.to_string())?;

    // Keep an existing recovery backup if the primary was already missing.
    if has_current && has_backup {
        fs::remove_file(&backup).map_err(|e| e.to_string())?;
    }
    if has_current {
        if let Err(error) = fs::rename(&current, &backup) {
            let _ = fs::remove_file(&staged);
            return Err(format!("Unable to stage existing site settings: {error}"));
        }
    }
    if let Err(error) = fs::rename(&staged, &current) {
        if regular_settings_file(&backup).unwrap_or(false) {
            let _ = fs::rename(&backup, &current);
        }
        let _ = fs::remove_file(&staged);
        return Err(format!("Unable to replace site settings: {error}"));
    }
    if regular_settings_file(&backup)? { let _ = fs::remove_file(&backup); }
    Ok(())
}

fn valid_google_font_family(value: &str) -> bool {
    value.is_empty() ||
    (value.len() <= 80 && value.trim() == value &&
     !value.contains("  ") &&
     value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b' ' || c == b'-'))
}

fn google_font_markup(value: &str) -> String {
    if value.is_empty() { return String::new(); }
    // This is called only after validate_settings. Use a strict font-name
    // character allowlist so the name cannot inject HTML, CSS or query args.
    let family = value.replace(' ', "+");
    format!(
        "<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n<link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\n<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family={family}&amp;display=swap\">\n<style>body{{font-family:\"{value}\",\"Geist Website\",system-ui,-apple-system,\"Segoe UI\",sans-serif}}</style>"
    )
}

// Accept only a canonical HTTPS root, with an optional static-hosting path.
// Reject credentials, ports, escapes, queries, fragments and traversal so
// canonical tags and sitemap entries cannot point to ambiguous destinations.
fn valid_site_url(value: &str) -> bool {
    if value.is_empty() { return true; }
    if value.len() > 240 || !value.starts_with("https://") || !value.ends_with('/') {
        return false;
    }
    let tail = &value["https://".len()..];
    let Some((host, path)) = tail.split_once('/') else { return false; };
    if host.is_empty() || host.len() > 253 || host.starts_with('.') || host.ends_with('.') ||
        host.contains("..") || !host.bytes().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'.' | b'-')) {
        return false;
    }
    if host.split('.').any(|label| label.is_empty() || label.starts_with('-') || label.ends_with('-')) {
        return false;
    }
    !path.starts_with('/') &&
        !path.split('/').any(|segment| segment == "." || segment == "..") &&
        !path.contains("//") &&
        path.bytes().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'/' | b'-' | b'_' | b'.' | b'~'))
}

fn sitemap_xml(site_url: &str, public_pages: &[&crate::pages::Page]) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for page in public_pages {
        xml.push_str(&format!("<url><loc>{}{}</loc></url>\n", escape(site_url), filename(&page.id)));
    }
    xml.push_str("</urlset>\n");
    xml
}

// Only reviewed language packages (or the historical built-in languages) may
// be selected. The strict tag shape also prevents injecting HTML attributes.
fn safe_site_language_tag(code: &str) -> bool {
    let bytes = code.as_bytes();
    (bytes.len() == 2 && bytes.iter().all(u8::is_ascii_lowercase)) ||
    (bytes.len() == 5 && bytes[0..2].iter().all(u8::is_ascii_lowercase) &&
        bytes[2] == b'-' && bytes[3..5].iter().all(u8::is_ascii_uppercase))
}
fn supported_site_language(code: &str) -> bool {
    if !safe_site_language_tag(code) { return false; }
    const BUILT_IN: &[&str] = &[
        "en", "ko", "pl", "de", "fr", "es", "ja", "zh-CN", "zh-TW", "pt-BR",
        "ru", "uk", "ar", "tr", "it", "cs", "hu", "th",
    ];
    BUILT_IN.contains(&code) || PAGLORA_LOCALE_SOURCES.iter().any(|(locale, _)| *locale == code)
}
fn site_text_direction(language: &str) -> &'static str {
    if let Some((_, raw)) = PAGLORA_LOCALE_SOURCES.iter().find(|(locale, _)| *locale == language) {
        if serde_json::from_str::<serde_json::Value>(raw).ok()
            .and_then(|value| value.get("direction").and_then(|direction| direction.as_str()).map(str::to_owned))
            .as_deref() == Some("rtl") {
            return "rtl";
        }
    }
    // Arabic remains RTL even before its community catalog is available.
    if language == "ar" { "rtl" } else { "ltr" }
}

fn validate_settings(settings: &SiteSettings) -> Result<(), String> {
    if !valid_site_url(&settings.site_url) {
        return Err("Published site URL must be an HTTPS site root ending in /, without credentials, ports, query, or fragments".into());
    }
    for (value, field, max) in [
        (&settings.title, "Site title", 100),
        (&settings.description, "Site description", 240),
        (&settings.footer_text, "Footer text", 240),
    ] {
        if (field == "Site title" && value.trim().is_empty()) ||
            value.chars().count() > max || value.chars().any(char::is_control) {
            return Err(format!("{field} must contain no control characters and fit within {max} characters"));
        }
    }
    if !["left", "right", "top"].contains(&settings.navigation_position.as_str()) {
        return Err("Unsupported navigation position".into());
    }
    if !["left", "center"].contains(&settings.header_alignment.as_str()) {
        return Err("Unsupported header alignment".into());
    }
    if !["system", "light", "dark"].contains(&settings.theme_mode.as_str()) {
        return Err("Unsupported website color mode".into());
    }
    if !["classic", "inset", "reading"].contains(&settings.site_layout.as_str()) {
        return Err("Unsupported website layout preset".into());
    }
    if !["original", "graphite", "lavender", "linen"].contains(&settings.site_palette.as_str()) {
        return Err("Unsupported website color palette".into());
    }
    if !["stacked", "grid"].contains(&settings.card_layout.as_str()) {
        return Err("Unsupported documentation card layout".into());
    }
    if !supported_site_language(&settings.language) {
        return Err("Unsupported website content language".into());
    }
    let accent = settings.accent_color.as_bytes();
    if accent.len() != 7 || accent[0] != b'#' || !accent[1..].iter().all(|value| value.is_ascii_hexdigit()) {
        return Err("Accent color must use a six-digit hexadecimal code".into());
    }
    if !valid_google_font_family(&settings.google_font_family) {
        return Err("Google Fonts family may contain only letters, numbers, spaces and hyphens (max 80 characters)".into());
    }
    if !(640..=1100).contains(&settings.content_width) {
        return Err("Content width must be between 640 and 1100 pixels".into());
    }
    if !settings.logo_url.is_empty() &&
        !safe_link(&settings.logo_url, true) && !valid_asset_url(&settings.logo_url) {
        return Err("Logo must use an HTTPS URL or a local assets/ image".into());
    }
    if !settings.favicon_url.is_empty() &&
        !(settings.favicon_url.starts_with("https://") && safe_link(&settings.favicon_url, true)) &&
        !valid_asset_url(&settings.favicon_url) {
        return Err("Favicon must use an HTTPS URL or a local assets/ image".into());
    }
    for (links, name) in [
        (&settings.header_links, "Header"),
        (&settings.footer_links, "Footer"),
    ] {
        if links.len() > 8 { return Err(format!("{name} has too many links")); }
        for link in links {
            if link.label.trim().is_empty() || link.label.chars().count() > 60 ||
                link.label.chars().any(char::is_control) || !safe_link(&link.url, false) {
                return Err(format!("{name} link label or URL is invalid"));
            }
        }
    }
    Ok(())
}

// Local assets are flat, passive image files only. Never accept traversal,
 // SVG, executable files or URLs with query strings/fragments.
fn valid_asset_name(name: &str) -> bool {
    let Some((stem, ext)) = name.rsplit_once('.') else { return false; };
    !stem.is_empty() && stem.len() <= 90 &&
        stem.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') &&
        matches!(ext, "png" | "jpg" | "webp" | "gif")
}

fn valid_asset_url(url: &str) -> bool {
    url.strip_prefix("assets/").is_some_and(valid_asset_name)
}

fn image_type(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") { Some(("png", "image/png")) }
    else if bytes.starts_with(b"\xff\xd8\xff") { Some(("jpg", "image/jpeg")) }
    else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") { Some(("gif", "image/gif")) }
    else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else { None }
}

fn asset_directory(project: &Path) -> Result<PathBuf, String> {
    let dir = project.join("assets");
    if dir.exists() {
        let kind = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?.file_type();
        if kind.is_symlink() || !kind.is_dir() {
            return Err("Project assets folder must be a real directory".into());
        }
    }
    Ok(dir)
}

fn read_asset(project: &Path, name: &str) -> Result<(Vec<u8>, &'static str), String> {
    if !valid_asset_name(name) { return Err("Invalid local image name".into()); }
    let path = asset_directory(project)?.join(name);
    let meta = fs::symlink_metadata(&path).map_err(|e| format!("Missing local image {name}: {e}"))?;
    if !meta.file_type().is_file() || meta.len() > MAX_ASSET_BYTES as u64 {
        return Err(format!("Local image {name} is not a regular file under 5 MiB"));
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_ASSET_BYTES { return Err("Local image exceeds 5 MiB".into()); }
    let (extension, mime) = image_type(&bytes).ok_or("Unsupported local image format")?;
    if !name.ends_with(&format!(".{extension}")) {
        return Err("Local image extension does not match its contents".into());
    }
    Ok((bytes, mime))
}

fn referenced_local_assets(html: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let mut remaining = html;
    while let Some(index) = remaining.find("src=\"assets/") {
        remaining = &remaining[index + "src=\"assets/".len()..];
        let Some(end) = remaining.find('"') else { break; };
        let name = &remaining[..end];
        if valid_asset_name(name) { names.insert(name.to_string()); }
        remaining = &remaining[end + 1..];
    }
    names
}


/// Only CSS url(...) values that point to passive local project images count
/// as public asset dependencies. Ignore comments, ordinary quoted strings,
/// CSS escapes, external URLs and malformed CSS. Spans are byte offsets to
/// the URL value, excluding surrounding quotes.
fn css_image_refs(css:&str)->Vec<(usize,usize,String)>{
    let bytes=css.as_bytes();
    let mut out=Vec::new();
    let mut i=0;
    while i<bytes.len(){
        if bytes[i..].starts_with(b"/*"){
            i+=2;
            while i+1<bytes.len() && !bytes[i..].starts_with(b"*/"){i+=1;}
            i=(i+2).min(bytes.len());
            continue;
        }
        if bytes[i]==b'\'' || bytes[i]==b'"'{
            let quote=bytes[i];
            i+=1;
            while i<bytes.len(){
                if bytes[i]==b'\\'{i=(i+2).min(bytes.len());continue;}
                if bytes[i]==quote{i+=1;break;}
                i+=1;
            }
            continue;
        }
        if i+4<=bytes.len() && bytes[i..i+4].eq_ignore_ascii_case(b"url("){
            let mut j=i+4;
            while j<bytes.len() && bytes[j].is_ascii_whitespace(){j+=1;}
            let quote=if j<bytes.len() && (bytes[j]==b'"'||bytes[j]==b'\''){let q=bytes[j];j+=1;Some(q)}else{None};
            let start=j;
            while j<bytes.len(){
                if bytes[j]==b'\\'{j=(j+2).min(bytes.len());continue;}
                if quote==Some(bytes[j]) || (quote.is_none() && bytes[j]==b')'){break;}
                j+=1;
            }
            let mut end=j;
            if quote.is_none(){while end>start && bytes[end-1].is_ascii_whitespace(){end-=1;}}
            if let Some(q)=quote {
                if j>=bytes.len() || bytes[j]!=q {i=j;continue;}
                j+=1;
                while j<bytes.len() && bytes[j].is_ascii_whitespace(){j+=1;}
            }
            if j<bytes.len() && bytes[j]==b')' {
                if let Some(name)=css.get(start..end).and_then(|v|v.strip_prefix("assets/")) {
                    if valid_asset_name(name) {out.push((start,end,name.to_owned()));}
                }
                i=j+1;
                continue;
            }
        }
        i+=1;
    }
    out
}
/// Preview srcDoc has no valid filesystem base URL. Substitute only verified,
/// passive local assets with self-contained image data URLs.
fn inline_css_images(project:&Path, css:&str)->Result<(String,usize),String>{
    let mut result=String::with_capacity(css.len());
    let mut last=0;
    let mut total=0usize;
    for (start,end,name) in css_image_refs(css){
        let (bytes,mime)=read_asset(project,&name)?;
        total=total.saturating_add(bytes.len());
        if total>MAX_PREVIEW_ASSET_BYTES{return Err("Preview CSS images exceed the 15 MiB limit".into());}
        result.push_str(&css[last..start]);
        result.push_str(&format!("data:{mime};base64,{}",base64_image(&bytes)));
        last=end;
    }
    result.push_str(&css[last..]);
    Ok((result,total))
}

/// Find broken links in sanitized public-page markup. Never treat links to
/// remote origins or same-page fragments as local page references.
fn site_export_link_warnings(
    pages: &[crate::pages::Page],
    rendered: &HashMap<&str, &RenderedPage>,
) -> Vec<String> {
    let visibility: HashMap<&str, bool> =
        pages.iter().map(|page| (page.id.as_str(), page.is_public)).collect();
    let mut warnings = HashSet::new();
    for page in pages.iter().filter(|page| page.is_public) {
        let Some(content) = rendered.get(page.id.as_str()) else { continue; };
        // This receives only DOM-sanitized HTML built by renderPageContent.
        // Literal quotation marks in text are escaped, so they cannot create
        // misleading href attributes.
        for part in content.html.split("href=\"").skip(1) {
            let Some(href) = part.split('"').next() else { continue; };
            let pathname = href.split('#').next().unwrap_or(href);
            let pathname = pathname.strip_prefix("./").unwrap_or(pathname);
            let Some(target) = pathname.strip_suffix(".html") else { continue; };
            if !valid_id(target) { continue; }
            match visibility.get(target) {
                Some(true) => {}
                Some(false) => {
                    warnings.insert(format!(
                        "{} links to private page {}.html (not published)", page.id, target
                    ));
                }
                None => {
                    warnings.insert(format!(
                        "{} links to missing page {}.html", page.id, target
                    ));
                }
            }
        }
    }
    let mut sorted: Vec<_> = warnings.into_iter().collect();
    sorted.sort();
    sorted
}

fn base64_image(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as usize;
        let b = chunk.get(1).copied().unwrap_or(0) as usize;
        let c = chunk.get(2).copied().unwrap_or(0) as usize;
        out.push(ABC[a >> 2] as char);
        out.push(ABC[((a & 3) << 4) | (b >> 4)] as char);
        out.push(if chunk.len() > 1 { ABC[((b & 15) << 2) | (c >> 6)] as char } else { '=' });
        out.push(if chunk.len() > 2 { ABC[c & 63] as char } else { '=' });
    }
    out
}


#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteAsset { url: String, bytes: u64, mime: String }

/// List passive, regular project images without following symlinks or loading
/// full files into memory. The preview command revalidates selected contents.
fn list_project_assets(project: &Path) -> Result<Vec<SiteAsset>, String> {
    let dir = asset_directory(project)?;
    if !dir.exists() { return Ok(Vec::new()); }
    let mut assets = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue; };
        if !valid_asset_name(name) { continue; }
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        if !meta.file_type().is_file() || meta.len() == 0 || meta.len() > MAX_ASSET_BYTES as u64 { continue; }
        let mut file = fs::File::open(entry.path()).map_err(|e| e.to_string())?;
        let mut header = [0u8; 12];
        let count = file.read(&mut header).map_err(|e| e.to_string())?;
        let Some((ext, mime)) = image_type(&header[..count]) else { continue; };
        if !name.ends_with(&format!(".{ext}")) { continue; }
        assets.push(SiteAsset { url: format!("assets/{name}"), bytes: meta.len(), mime: mime.into() });
        if assets.len() > 500 { return Err("Too many local images (maximum 500)".into()); }
    }
    assets.sort_by(|a, b| a.url.cmp(&b.url));
    Ok(assets)
}


/// The review is conservative and fail-closed: unknown sources are NOT proof
/// of an unused file. We scan all saved Markdown (including unlisted/private),
/// registered plus residual theme CSS, and project/site/page metadata.
/// A candidate means "no literal name in current supported sources", not a
/// guarantee about external integrations or unsaved third-party editors.
const MAX_ASSET_SOURCE_AUDIT: usize = 32 * 1024 * 1024;
const REMOVED_ASSETS: &str = "removed-assets";

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct UnusedAssetCandidate { url: String, bytes: u64, fingerprint: String }

fn asset_fingerprint(bytes: &[u8]) -> String {
    let mut hasher=DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}",hasher.finish())
}
fn read_candidate_sources(project:&Path)->Result<Vec<String>,String>{
    let pages=read_pages(project)?;
    let docs_dir=project.join("docs");
    if !fs::symlink_metadata(&docs_dir).map_err(|e|e.to_string())?.file_type().is_dir(){
        return Err("Docs folder is not a real directory".into());
    }
    let mut sources=Vec::new();
    let mut total=0usize;
    let mut listed:HashSet<String>=HashSet::new();
    for page in pages {listed.insert(format!("{}.md",page.id));}
    let mut found=HashSet::new();
    for entry in fs::read_dir(&docs_dir).map_err(|e|e.to_string())?{
        let entry=entry.map_err(|e|e.to_string())?;
        let name=entry.file_name().to_string_lossy().into_owned();
        let meta=fs::symlink_metadata(entry.path()).map_err(|e|e.to_string())?;
        if !name.ends_with(".md") || !meta.file_type().is_file() || meta.len()>5_000_000 {
            return Err("Unexpected or unsafe docs file; cleanup review is unavailable".into());
        }
        found.insert(name);
        let content=fs::read_to_string(entry.path()).map_err(|e|e.to_string())?;
        total+=content.len();
        if total>MAX_ASSET_SOURCE_AUDIT{return Err("Too much source content to safely review images".into());}
        sources.push(content);
    }
    if !listed.is_subset(&found){return Err("Some Markdown pages are missing; cleanup is blocked".into());}
    let dir=project.join("theme");
    // Validates manifest membership so no extra stylesheet silently vanishes.
    crate::source::portable_theme_files(project)?;
    if dir.exists() {
        for entry in fs::read_dir(&dir).map_err(|e|e.to_string())?{
            let entry=entry.map_err(|e|e.to_string())?;
            let meta=fs::symlink_metadata(entry.path()).map_err(|e|e.to_string())?;
            if !meta.file_type().is_file(){return Err("Unexpected theme file; cleanup blocked".into());}
            if meta.len()>512_000 {return Err("Oversized theme source; cleanup blocked".into());}
            let content=fs::read_to_string(entry.path()).map_err(|e|e.to_string())?;
            total+=content.len();
            if total>MAX_ASSET_SOURCE_AUDIT{return Err("Too much source content to review".into());}
            sources.push(content);
        }
    }
    // Scan metadata, not only the parsed logo field: unknown future metadata
    // fields can still reference an imported asset by name.
    for name in ["site.json","pages.json","paglora.json"] {
        let path=project.join(name);
        if name=="site.json" && !path.exists() {
            // Legacy projects may use default settings without site.json.
            continue;
        }
        if !fs::symlink_metadata(&path).map_err(|e|e.to_string())?.file_type().is_file(){
            return Err(format!("Missing or unsafe project metadata: {name}"));
        }
        let meta=fs::metadata(&path).map_err(|e|e.to_string())?;
        if meta.len()>256_000{return Err("Metadata too large for cleanup audit".into());}
        let content=fs::read_to_string(path).map_err(|e|e.to_string())?;
        total+=content.len();
        if total>MAX_ASSET_SOURCE_AUDIT{return Err("Too much source content to review".into());}
        sources.push(content);
    }
    Ok(sources)
}
fn removal_directory(project:&Path)->Result<PathBuf,String>{
    let dir=project.join(REMOVED_ASSETS);
    if dir.exists(){
        if !fs::symlink_metadata(&dir).map_err(|e|e.to_string())?.file_type().is_dir(){
            return Err("Unsafe image recovery directory".into());
        }
    }
    Ok(dir)
}
fn unused_candidates_at(project:&Path)->Result<Vec<UnusedAssetCandidate>,String>{
    let sources=read_candidate_sources(project)?;
    let recover=removal_directory(project)?;
    let mut unused=Vec::new();
    for asset in list_project_assets(project)?{
        let name=asset.url.strip_prefix("assets/").ok_or("Invalid image path")?;
        if sources.iter().any(|source|source.contains(name)){continue;}
        // Do not offer a cleanup that would collide with an older recovery.
        if recover.join(name).exists(){continue;}
        let (bytes,_)=read_asset(project,name)?;
        unused.push(UnusedAssetCandidate{
            url:asset.url, bytes:asset.bytes, fingerprint:asset_fingerprint(&bytes),
        });
    }
    Ok(unused)
}
fn recoverable_assets_at(project:&Path)->Result<Vec<SiteAsset>,String>{
    let dir=removal_directory(project)?;
    if !dir.exists(){return Ok(Vec::new());}
    let mut list=Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e|e.to_string())?{
        let entry=entry.map_err(|e|e.to_string())?;
        let name=entry.file_name().to_string_lossy().into_owned();
        if !valid_asset_name(&name){return Err("Unsafe image in recovery folder".into());}
        let metadata=fs::symlink_metadata(entry.path()).map_err(|e|e.to_string())?;
        if !metadata.file_type().is_file() || metadata.len()>MAX_ASSET_BYTES as u64 {
            return Err("Unsafe recovery file".into());
        }
        let bytes=fs::read(entry.path()).map_err(|e|e.to_string())?;
        let (ext,mime)=image_type(&bytes).ok_or("Invalid recovered image content")?;
        if !name.ends_with(&format!(".{ext}")){return Err("Recovered image extension mismatch".into());}
        list.push(SiteAsset{url:format!("assets/{name}"),bytes:metadata.len(),mime:mime.into()});
        if list.len()>500{return Err("Too many recovered images".into());}
    }
    list.sort_by(|a,b|a.url.cmp(&b.url));
    Ok(list)
}
fn move_unused_at(project:&Path,url:&str,fingerprint:&str,confirmation:&str)->Result<(),String>{
    let name=url.strip_prefix("assets/").ok_or("Invalid image path")?;
    if !valid_asset_name(name) || confirmation!=name {
        return Err("Type the exact filename to confirm recovery move".into());
    }
    if fingerprint.len()!=16 || !fingerprint.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err("Invalid image review fingerprint".into());
    }
    let candidate=unused_candidates_at(project)?.into_iter()
        .find(|item|item.url==url && item.fingerprint==fingerprint)
        .ok_or("Image was changed or newly referenced. Review images again.")?;
    let source=asset_directory(project)?.join(name);
    let recover=removal_directory(project)?;
    fs::create_dir_all(&recover).map_err(|e|e.to_string())?;
    let target=recover.join(name);
    if target.exists(){return Err("Image already exists in recovery".into());}
    let (bytes,_)=read_asset(project,name)?;
    if asset_fingerprint(&bytes)!=candidate.fingerprint {
        return Err("Image changed during review; no file moved".into());
    }
    fs::rename(source,target).map_err(|e|format!("Could not move image into recovery: {e}"))?;
    Ok(())
}
fn restore_image_at(project:&Path,url:&str)->Result<(),String>{
    let name=url.strip_prefix("assets/").ok_or("Invalid image path")?;
    if !valid_asset_name(name){return Err("Invalid recovered image".into());}
    let recover=removal_directory(project)?;
    let source=recover.join(name);
    let metadata=fs::symlink_metadata(&source).map_err(|e|e.to_string())?;
    if !metadata.file_type().is_file(){return Err("Unsafe recovered image".into());}
    let bytes=fs::read(&source).map_err(|e|e.to_string())?;
    let (ext,_)=image_type(&bytes).ok_or("Recovered image is invalid")?;
    if !name.ends_with(&format!(".{ext}")) || bytes.len()>MAX_ASSET_BYTES {
        return Err("Recovered image failed validation".into());
    }
    let assets=asset_directory(project)?;
    fs::create_dir_all(&assets).map_err(|e|e.to_string())?;
    let target=assets.join(name);
    if target.exists(){return Err("An active image with this name already exists; no overwrite".into());}
    fs::rename(source,target).map_err(|e|format!("Could not restore image: {e}"))?;
    Ok(())
}
fn verified_cleanup_project(app:&AppHandle,project_id:&str)->Result<PathBuf,String>{
    let (project,_)=project_folder(app,project_id)?;
    if !fs::symlink_metadata(&project).map_err(|e|e.to_string())?.file_type().is_dir(){
        return Err("Project folder must be a real directory".into());
    }
    Ok(project)
}
#[tauri::command]
pub fn review_unused_site_assets(app:AppHandle,project_id:String)->Result<Vec<UnusedAssetCandidate>,String>{
    let project=verified_cleanup_project(&app,&project_id)?;
    unused_candidates_at(&project)
}
#[tauri::command]
pub fn move_unused_site_asset(app:AppHandle,project_id:String,url:String,fingerprint:String,confirmation:String)->Result<(),String>{
    let project=verified_cleanup_project(&app,&project_id)?;
    move_unused_at(&project,&url,&fingerprint,&confirmation)
}
#[tauri::command]
pub fn list_recoverable_site_assets(app:AppHandle,project_id:String)->Result<Vec<SiteAsset>,String>{
    let project=verified_cleanup_project(&app,&project_id)?;
    recoverable_assets_at(&project)
}
#[tauri::command]
pub fn restore_site_asset(app:AppHandle,project_id:String,url:String)->Result<(),String>{
    let project=verified_cleanup_project(&app,&project_id)?;
    restore_image_at(&project,&url)
}

#[tauri::command]
pub fn list_site_assets(app: AppHandle, project_id: String) -> Result<Vec<SiteAsset>, String> {
    let (project, _) = project_folder(&app, &project_id)?;
    if fs::symlink_metadata(&project).map_err(|e| e.to_string())?.file_type().is_symlink() {
        return Err("Project folder cannot be a symlink".into());
    }
    list_project_assets(&project)
}

#[tauri::command]
pub fn preview_site_asset(app: AppHandle, project_id: String, url: String) -> Result<String, String> {
    let name = url.strip_prefix("assets/").ok_or("Invalid local image URL")?;
    if !valid_asset_name(name) { return Err("Invalid local image URL".into()); }
    let (project, _) = project_folder(&app, &project_id)?;
    if fs::symlink_metadata(&project).map_err(|e| e.to_string())?.file_type().is_symlink() {
        return Err("Project folder cannot be a symlink".into());
    }
    let (bytes, mime) = read_asset(&project, name)?;
    Ok(format!("data:{mime};base64,{}", base64_image(&bytes)))
}

#[tauri::command]
pub fn import_site_asset(app: AppHandle, project_id: String, filename: String, bytes: Vec<u8>) -> Result<String, String> {
    if bytes.is_empty() || bytes.len() > MAX_ASSET_BYTES || filename.len() > 256 {
        return Err("Images must be between 1 byte and 5 MiB".into());
    }
    let (extension, _) = image_type(&bytes).ok_or("Only PNG, JPEG, WebP and GIF images are supported")?;
    let (project, _) = project_folder(&app, &project_id)?;
    if fs::symlink_metadata(&project).map_err(|e| e.to_string())?.file_type().is_symlink() {
        return Err("Project folder cannot be a symlink".into());
    }
    let dir = asset_directory(&project)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // The source name is cosmetic only. No client-provided path is used.
    let base = filename.rsplit(|c| c == '/' || c == '\\').next().unwrap_or("image");
    let stem = base.rsplit_once('.').map(|(name, _)| name).unwrap_or(base);
    let stem: String = stem.to_ascii_lowercase().chars()
        .map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() { c } else { '-' })
        .take(36).collect();
    let stem = stem.trim_matches('-');
    let stem = if stem.is_empty() { "image" } else { stem };
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    for attempt in 0..32 {
        let name = format!("{stem}-{stamp:x}-{attempt}.{extension}");
        let path = dir.join(&name);
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(&bytes) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(format!("Unable to save image: {error}"));
                }
                return Ok(format!("assets/{name}"));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Unable to create image: {error}")),
        }
    }
    Err("Could not allocate a unique image filename".into())
}

fn safe_link(raw: &str, https_only: bool) -> bool {
    if raw.chars().any(|c| c.is_control() || c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>')) {
        return false;
    }
    if raw.starts_with("https://") && raw.len() > 8 { return true; }
    !https_only && (
        (raw.starts_with("mailto:") && raw.len() > 7) ||
        (raw.ends_with(".html") && raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'))
    )
}

fn valid_site_page_entry(page: &crate::pages::Page) -> bool {
    valid_id(&page.id) && crate::pages::valid_navigation_group(&page.nav_group) &&
        ["docs", "wide", "landing"].contains(&page.layout.as_str()) &&
        !page.title.trim().is_empty() && page.title.chars().count() <= 100 &&
        !page.title.chars().any(char::is_control)
}

fn read_pages(dir: &Path) -> Result<Vec<crate::pages::Page>, String> {
    let manifest: PageManifest = serde_json::from_str(
        &fs::read_to_string(dir.join("pages.json")).map_err(|e| e.to_string())?
    ).map_err(|e| e.to_string())?;
    if manifest.format_version != 1 || manifest.pages.is_empty() ||
        manifest.pages.len() > 500 || manifest.pages[0].id != "index" ||
        !manifest.pages[0].is_public {
        return Err("Invalid public site page manifest".into());
    }
    let mut seen = HashSet::new();
    for page in &manifest.pages {
        if !valid_site_page_entry(page) || !seen.insert(page.id.as_str()) {
            return Err("Invalid page identifier in site".into());
        }
    }
    Ok(manifest.pages)
}

fn escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&#39;")
}

fn filename(id: &str) -> String { format!("{id}.html") }

// The bundled catalogs are validated snapshots of paglora-lang.
// Missing keys/languages fall back to canonical English.
fn website_messages(locale: &str) -> serde_json::Map<String, serde_json::Value> {
    let parse = |text: &str| -> serde_json::Map<String, serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(text).ok()
            .and_then(|value| value.get("messages").and_then(|m| m.as_object()).cloned())
            .unwrap_or_default()
    };
    let canonical = PAGLORA_LOCALE_SOURCES.iter().find(|(code, _)| *code == "en")
        .map(|(_, content)| *content).unwrap_or("{}");
    let mut values = parse(canonical);
    if locale != "en" {
        if let Some((_, content)) = PAGLORA_LOCALE_SOURCES.iter().find(|(code, _)| *code == locale) {
            values.extend(parse(content));
        }
    }
    values
}
fn website_label<'a>(messages: &'a serde_json::Map<String, serde_json::Value>, key: &'a str) -> &'a str {
    messages.get(key).and_then(|value| value.as_str()).unwrap_or(key)
}


// Match browser chrome to the exported page's configured light/dark palette.
// System mode uses media-qualified tags so browser UI follows OS changes.
fn theme_color_meta(mode: &str, palette: &str) -> String {
    // Browser chrome matches the same light/dark surfaces as the chosen
    // palette. A visitor's appearance toggle may change after page load,
    // but the default chrome always matches the configured website theme.
    let (light, dark) = match palette {
        "graphite" => ("#ffffff", "#101114"),
        "lavender" => ("#fdfcff", "#17121e"),
        "linen" => ("#fffdf9", "#1c1915"),
        _ => ("#ffffff", "#11151d"),
    };
    match mode {
        "light" => format!(r#"<meta name="theme-color" content="{light}">"#),
        "dark" => format!(r#"<meta name="theme-color" content="{dark}">"#),
        _ => format!(r#"<meta name="theme-color" content="{light}" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="{dark}" media="(prefers-color-scheme: dark)">"#),
    }
}

// Open Graph uses language_region identifiers, unlike HTML's BCP 47 lang.
fn social_locale(language: &str) -> String {
    let locale = match language {
        "en" => "en_US", "ko" => "ko_KR", "pl" => "pl_PL", "de" => "de_DE", "fr" => "fr_FR",
        "es" => "es_ES", "ja" => "ja_JP", "zh-CN" => "zh_CN", "zh-TW" => "zh_TW",
        "pt-BR" => "pt_BR", "ru" => "ru_RU", "uk" => "uk_UA", "ar" => "ar_SA",
        "tr" => "tr_TR", "it" => "it_IT", "cs" => "cs_CZ", "hu" => "hu_HU",
        "th" => "th_TH", _ => return language.replace('-', "_"),
    };
    locale.to_string()
}
fn public_search_index(
    public_pages: &[&crate::pages::Page],
    html_by_id: &HashMap<&str, &RenderedPage>,
) -> serde_json::Value {
    let pages: Vec<_> = public_pages.iter().filter_map(|page| {
        let item = html_by_id.get(page.id.as_str())?;
        Some(serde_json::json!({
            "title": page.title,
            "url": filename(&page.id),
            "text": item.search_text.chars().take(15_000).collect::<String>()
        }))
    }).collect();
    serde_json::json!({"version": 1, "pages": pages})
}


// Use the sanitized public-page text for unique search/social descriptions.
// Fallback to the site description for empty or legacy pages.
fn page_meta_description(site_description: &str, search_text: Option<&str>) -> String {
    let normalized = search_text.unwrap_or("").split_whitespace()
        .collect::<Vec<_>>().join(" ");
    let excerpt: String = normalized.chars().filter(|ch| !ch.is_control()).take(160).collect();
    if excerpt.is_empty() { site_description.to_string() } else { excerpt }
}

// Convenience wrapper used by the Rust renderer regression tests only.
// Production rendering calls render_site_page_with_description directly.
#[cfg(test)]
fn render_site_page(
    settings: &SiteSettings,
    pages: &[crate::pages::Page],
    page: &crate::pages::Page,
    body: &str,
    toc: &[TocHeading],
) -> String {
    render_site_page_with_description(settings, pages, page, body, toc, None)
}

fn render_site_page_with_description(
    settings: &SiteSettings,
    pages: &[crate::pages::Page],
    page: &crate::pages::Page,
    body: &str,
    toc: &[TocHeading],
    search_text: Option<&str>,
) -> String {
    let public_pages: Vec<_> = pages.iter().filter(|p| p.is_public).collect();
    let messages = website_messages(&settings.language);
    let toc_title = escape(website_label(&messages, "site.navigation.onThisPage"));
    let previous_text = escape(website_label(&messages, "site.navigation.previous"));
    let next_text = escape(website_label(&messages, "site.navigation.next"));
    let browse_text = escape(website_label(&messages, "site.navigation.browse"));
    let skip_text = escape(website_label(&messages, "site.navigation.skip"));
    // Browser-only enhancements read plain JSON, never executable translator content.
    let mut browser_labels = serde_json::Map::new();
    for (key, value) in &messages {
        if key.starts_with("site.") { browser_labels.insert(key.clone(), value.clone()); }
    }
    let browser_json = serde_json::to_string(&browser_labels).unwrap_or_else(|_| "{}".into())
        .replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026");
    let mut links = String::new();
    let mut previous_group: Option<&str> = None;
    for item in &public_pages {
        let group = item.nav_group.as_str();
        if previous_group != Some(group) {
            if previous_group.is_some() { links.push_str("</div></details>"); }
            links.push_str(&format!(
                "<details class=\"nav-group\" open><summary>{}</summary><div class=\"nav-group-links\">",
                escape(group)
            ));
            previous_group = Some(group);
        }
        let current = if item.id == page.id { " aria-current=\"page\"" } else { "" };
        links.push_str(&format!(
            "<a href=\"{}\"{}>{}</a>",
            filename(&item.id), current, escape(&item.title)
        ));
    }
    if previous_group.is_some() { links.push_str("</div></details>"); }
    let mut toc_links = String::new();
    for heading in toc.iter().take(100) {
        // All heading metadata comes from the frontend's sanitized Markdown DOM.
        // Validate it again before placing it into an HTML attribute.
        if !matches!(heading.level, 2 | 3) || heading.title.trim().is_empty() ||
            heading.title.len() > 640 || heading.id.len() > 160 ||
            heading.id.is_empty() || !heading.id.chars().all(|ch| ch.is_alphanumeric() || ch == '-') {
            continue;
        }
        toc_links.push_str(&format!(
            "<a class=\"toc-h{}\" href=\"#{}\">{}</a>",
            heading.level, heading.id, escape(&heading.title)
        ));
    }
    let toc_aside = if toc_links.is_empty() { String::new() } else {
        format!("<div class=\"toc-inner\"><h2>{toc_title}</h2>{toc_links}</div>")
    };
    let mobile_toc = if toc_links.is_empty() { String::new() } else {
        format!("<details class=\"mobile-toc\"><summary>{toc_title}</summary>{toc_links}</details>")
    };
    let mut pager = String::new();
    if let Some(index) = public_pages.iter().position(|item| item.id == page.id) {
        if index > 0 {
            let previous = public_pages[index - 1];
            pager.push_str(&format!("<a class=\"previous\" href=\"{}\"><small>← {previous_text}</small><span>{}</span></a>",
                filename(&previous.id), escape(&previous.title)));
        }
        if let Some(next) = public_pages.get(index + 1) {
            pager.push_str(&format!("<a class=\"next\" href=\"{}\"><small>{next_text} →</small><span>{}</span></a>",
                filename(&next.id), escape(&next.title)));
        }
    }
    let pager = if pager.is_empty() { String::new() } else {
        format!("<nav class=\"page-pager\" aria-label=\"Previous and next pages\">{pager}</nav>")
    };
    let page_title = format!("{} | {}", page.title, settings.title);
    let logo = if settings.logo_url.is_empty() {
        String::new()
    } else {
        format!("<img class=\"brand-logo\" src=\"{}\" alt=\"\">", escape(&settings.logo_url))
    };
    let favicon = if settings.favicon_url.is_empty() { String::new() } else {
        let kind = if settings.favicon_url.ends_with(".png") { "image/png" }
            else if settings.favicon_url.ends_with(".jpg") || settings.favicon_url.ends_with(".jpeg") { "image/jpeg" }
            else if settings.favicon_url.ends_with(".webp") { "image/webp" }
            else if settings.favicon_url.ends_with(".gif") { "image/gif" }
            else { "image/png" };
        format!("<link rel=\"icon\" type=\"{kind}\" href=\"{}\">", escape(&settings.favicon_url))
    };
    let link_html = |items: &[SiteLink], class_name: &str| -> String {
        if items.is_empty() { return String::new(); }
        let mut out = format!("<nav class=\"{}\" aria-label=\"Site links\">", class_name);
        for link in items {
            out.push_str(&format!(
                "<a href=\"{}\"{}>{}</a>",
                escape(&link.url),
                if link.url.starts_with("https://") { " rel=\"noopener noreferrer\"" } else { "" },
                escape(&link.label),
            ));
        }
        out.push_str("</nav>");
        out
    };
    let header_class = format!("{} {}", if settings.sticky_header { "" } else { "not-sticky" },
        if settings.header_alignment == "center" { "header-center" } else { "" });
    let footer = if settings.show_footer {
        format!("<footer class=\"site-footer\"><div class=\"site-footer-inner\"><span>{}</span>{}</div></footer>",
            escape(&settings.footer_text), link_html(&settings.footer_links, "footer-links"))
    } else { String::new() };
    let description = escape(&page_meta_description(&settings.description, search_text));
    let theme_mode = &settings.theme_mode;
    let theme_color = theme_color_meta(theme_mode, &settings.site_palette);
    let language = &settings.language;
    let og_locale = social_locale(language);
    let direction = site_text_direction(language);
    let google_font = google_font_markup(&settings.google_font_family);
    // Hex color was checked by validate_settings for both preview and build.
    // Dark and system-dark palettes set --brand with higher specificity than :root.
    // Match those selectors *after* styles.css so the configured accent wins in
    // light, dark, system-dark, and when the runtime appearance toggle is used.
    let brand_vars = format!(
        "--brand:{};--brand-light:color-mix(in srgb,var(--brand) 13%,var(--bg))",
        settings.accent_color
    );
    let accent_style = format!(
        r#"<style>:root{{{brand_vars}}}:root[data-color-mode="dark"]{{{brand_vars}}}
@media(prefers-color-scheme:dark){{:root:not([data-color-mode="light"]){{{brand_vars}}}}}</style>"#
    );
    let page_title = escape(&page_title);
    let canonical = if settings.site_url.is_empty() { String::new() } else {
        let url = escape(&format!("{}{}", settings.site_url, filename(&page.id)));
        format!("<link rel=\"canonical\" href=\"{url}\">\n<meta property=\"og:url\" content=\"{url}\">")
    };
    let brand_title = escape(&settings.title);
    let breadcrumb_title = escape(&page.title);
    let header_links = link_html(&settings.header_links, "header-links");
    let nav_position = &settings.navigation_position;
    let site_layout = &settings.site_layout;
    let site_palette = &settings.site_palette;
    let card_layout = &settings.card_layout;
    let content_width = settings.content_width;
    let layout = &page.layout;
    let runtime_scripts = site_runtime_script_tags();
    let stylesheet_chunk = stylesheet_chunk_name();
    format!(r##"<!doctype html>
<html lang="{language}" dir="{direction}" data-color-mode="{theme_mode}" data-site-layout="{site_layout}" data-site-palette="{site_palette}" data-card-layout="{card_layout}">
<head>
<meta charset="utf-8">
{favicon}
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="description" content="{description}">
{canonical}
<meta property="og:type" content="article">
<meta property="og:locale" content="{og_locale}">
<meta property="og:title" content="{page_title}">
<meta property="og:description" content="{description}">
<meta property="og:site_name" content="{brand_title}">
<meta name="twitter:card" content="summary">
<meta name="twitter:title" content="{page_title}">
<meta name="twitter:description" content="{description}">
{theme_color}
<title>{page_title}</title>
<link rel="stylesheet" href="{stylesheet_chunk}">
{google_font}
{accent_style}
</head>
<body class="page-layout-{layout}">
<a class="skip-link" href="#content">{skip_text}</a>
<header class="site-header {header_class}"><div class="top-bar"><a class="brand brand-wrap" href="index.html">{logo}{brand_title}</a>{header_links}</div></header>
<details class="mobile-nav"><summary>{browse_text}</summary><nav class="mobile-nav-inner" aria-label="Mobile page navigation">{links}</nav></details>
<div class="site-body nav-{nav_position} layout-{layout}">
<nav class="site-nav" aria-label="Documentation navigation"><div class="site-nav-inner">{links}</div></nav>
<main class="site-main" id="content" style="--paglora-content-width:{content_width}px">
<div class="article-wrap">
<div class="page-heading"><a href="index.html">Docs</a><span class="crumb-separator">/</span><strong>{breadcrumb_title}</strong></div>
{mobile_toc}
<article class="docs">{body}</article>
{pager}
</div></main>
<aside class="on-this-page" aria-label="Table of contents">{toc_aside}</aside>
</div>
{footer}
<script type="application/json" id="paglora-i18n">{browser_json}</script>
{runtime_scripts}
</body>
</html>"##)
}
#[tauri::command]
pub fn load_site_settings(app: AppHandle, project_id: String) -> Result<SiteSettings, String> {
    let (dir, name) = project_folder(&app, &project_id)?;
    read_settings(&dir, &name)
}

#[tauri::command]
pub fn save_site_settings(app: AppHandle, project_id: String, settings: SiteSettings) -> Result<(), String> {
    validate_settings(&settings)?;
    let (dir, _) = project_folder(&app, &project_id)?;
    write_settings(&dir, &settings)
}

/// Preview uses the exact same HTML and CSS renderer as static export.
/// The caller supplies already sanitized HTML; no files are written.
#[tauri::command]
pub fn preview_site_page(app: AppHandle, project_id: String, rendered: RenderedPage, settings_override: Option<SiteSettings>) -> Result<String, String> {
    let (dir, project_name) = project_folder(&app, &project_id)?;
    let settings = if let Some(value) = settings_override { value } else { read_settings(&dir, &project_name)? };
    validate_settings(&settings)?;
    let pages = read_pages(&dir)?;
    let page = pages.iter().find(|p| p.id == rendered.id && p.is_public)
        .ok_or("Page is private or not in this project")?;
    if rendered.html.len() > 15_000_000 { return Err("Page HTML is too large".into()); }
    let theme = crate::source::site_css(&dir)?;
    let doc = render_site_page_with_description(&settings, &pages, page, &rendered.html, &rendered.toc, Some(&rendered.search_text));
    // CSS is user-authored project source. Embed in this script-disabled iframe
    // without letting a literal closing style tag break out of the CSS element.
    let (override_css, css_bytes) = if let Some(css) = theme {
        let (embedded, bytes)=inline_css_images(&dir,&css)?;
        let escaped = embedded.replace('<', "\\3c ");
        (format!("<style>{escaped}</style>"),bytes)
    } else {(String::new(),0)};
    let site_with_font = SITE_CSS.replace("fonts/geist-latin.woff2", &format!(
        "data:font/woff2;base64,{}", include_str!("../assets/geist-latin.b64")
    ));
    let doc = doc.replace(
        &format!("<link rel=\"stylesheet\" href=\"{}\">", stylesheet_chunk_name()),
        &format!("<style>{site_with_font}</style>"),
    );
    // Studio preview is intentionally script-disabled; remove the exported
    // runtime reference. Site interactions are tested in the built website.
    let mut doc = doc;
    for asset in SITE_RUNTIME_ASSETS {
        let tag = format!("<script defer src=\"{}\"></script>", runtime_chunk_name(asset));
        doc = doc.replace(&tag, "");
    }
    // Inline only referenced local images in the script-disabled srcDoc
    // preview. No filesystem paths or active SVG content reach the iframe.
    let mut doc = doc.replace("</head>", &format!("{override_css}</head>"));
    let mut names = referenced_local_assets(&doc);
    if let Some(name) = settings.logo_url.strip_prefix("assets/") { names.insert(name.to_string()); }
    if let Some(name) = settings.favicon_url.strip_prefix("assets/") { names.insert(name.to_string()); }
    let mut total_bytes = css_bytes;
    for name in names {
        let (bytes, mime) = read_asset(&dir, &name)?;
        total_bytes += bytes.len();
        if total_bytes > MAX_PREVIEW_ASSET_BYTES {
            return Err("Preview local images exceed the 15 MiB limit".into());
        }
        let from = format!("src=\"assets/{name}\"");
        let to = format!("src=\"data:{mime};base64,{}\"", base64_image(&bytes));
        doc = doc.replace(&from, &to);
        let icon_from = format!("href=\"assets/{name}\"");
        let icon_to = format!("href=\"data:{mime};base64,{}\"", base64_image(&bytes));
        doc = doc.replace(&icon_from, &icon_to);
    }
    // Project-owned CSS always comes last, overriding designer presets.
    Ok(doc)
}

#[tauri::command]
pub fn build_site(app: AppHandle, project_id: String, rendered: Vec<RenderedPage>) -> Result<SiteBuildResult, String> {
    let (dir, project_name) = project_folder(&app, &project_id)?;
    let exports_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join(EXPORTS);
    // Emit *completed* native operations. No fabricated percentages or
    // filesystem paths; the UI filters by current project and build request.
    let mut report=|stage:&str,completed:usize,total:usize| {
        let _=app.emit("paglora:build-progress",serde_json::json!({
            "projectId":project_id,"stage":stage,"completed":completed,"total":total
        }));
    };
    build_site_from_disk_progress(&dir,&project_name,&exports_dir.join(&project_id),rendered,&mut report)
}

/// The production exporter is shared between Desktop and the headless CLI.
fn build_site_from_disk(dir: &Path, project_name: &str, target: &Path, rendered: Vec<RenderedPage>) -> Result<SiteBuildResult, String> {
    build_site_from_disk_progress(dir,project_name,target,rendered,&mut |_,_,_| {})
}
/// Shared native stage counts. Headless callers use the original wrapper with
/// no Tauri event dependency and identical export bytes.
fn build_site_from_disk_progress(dir:&Path,project_name:&str,target:&Path,rendered:Vec<RenderedPage>,
    report:&mut dyn FnMut(&str,usize,usize))->Result<SiteBuildResult,String> {
    let settings = read_settings(dir, project_name)?;
    validate_settings(&settings)?;
    let pages = read_pages(&dir)?;
    let theme = crate::source::site_css(&dir)?;
    let public_pages: Vec<_> = pages.iter().filter(|p| p.is_public).collect();
    let expected: HashSet<_> = public_pages.iter().map(|p| p.id.as_str()).collect();
    if rendered.len() != public_pages.len() { return Err("Public page count changed. Build again".into()); }
    let mut html_by_id = HashMap::new();
    for (count,page) in rendered.iter().enumerate() {
        if !expected.contains(page.id.as_str()) || !valid_id(&page.id) ||
            page.html.len() > 15_000_000 || html_by_id.insert(page.id.as_str(), page).is_some() {
            return Err("Invalid page data for static site build".into());
        }
        report("checking",count+1,rendered.len());
    }
    let warnings = site_export_link_warnings(&pages, &html_by_id);
    let exports_dir = target.parent().ok_or("Export target needs a parent directory")?;
    fs::create_dir_all(exports_dir).map_err(|e| e.to_string())?;
    let name = target.file_name().and_then(|s| s.to_str()).ok_or("Invalid export target")?;
    let staged = exports_dir.join(format!(".{name}-building"));
    let backup = exports_dir.join(format!(".{name}-previous"));
    if staged.exists() { fs::remove_dir_all(&staged).map_err(|e| e.to_string())?; }
    fs::create_dir(&staged).map_err(|e| e.to_string())?;
    let write_result = (|| -> Result<usize, String> {
        // /_paglora/chunks uses stable content hashes. Generated files can
        // be cached indefinitely; legacy root aliases remain for existing
        // third-party integrations during the transition.
        let chunks = staged.join("_paglora/chunks");
        fs::create_dir_all(chunks.join("css")).map_err(|e| e.to_string())?;
        let exported_css = SITE_CSS.replace("fonts/geist-latin.woff2", "../../../fonts/geist-latin.woff2");
        fs::write(staged.join(stylesheet_chunk_name()), exported_css).map_err(|e| e.to_string())?;
        fs::write(staged.join("styles.css"), SITE_CSS).map_err(|e| e.to_string())?;
        for asset in SITE_RUNTIME_ASSETS {
            fs::write(staged.join(runtime_chunk_name(asset)), asset.source).map_err(|e| e.to_string())?;
            fs::write(staged.join(asset.filename), asset.source).map_err(|e| e.to_string())?;
        }
        // Preserve MiniSearch MIT attribution in every exported website.
        fs::create_dir(staged.join("licenses")).map_err(|e| e.to_string())?;
        fs::write(staged.join("licenses/MiniSearch-MIT.txt"),
            include_str!("../../../../packages/site-runtime/MINISEARCH-LICENSE.txt"))
            .map_err(|e| e.to_string())?;
        // Bundle the default Geist font; optional hosted Google Fonts remain supported.
        fs::create_dir(staged.join("fonts")).map_err(|e| e.to_string())?;
        fs::write(staged.join("fonts/geist-latin.woff2"), include_bytes!("../assets/geist-latin.woff2"))
            .map_err(|e| e.to_string())?;
        fs::write(staged.join("fonts/OFL.txt"), include_str!("../assets/GEIST-LICENSE.txt"))
            .map_err(|e| e.to_string())?;
        if let Some(css) = &theme {
            fs::write(staged.join("custom.css"), css).map_err(|e| e.to_string())?;
        }
        // Publish only images referenced by public page HTML or the site
        // logo. Unreferenced/private project assets never leave the project.
        let mut image_names = HashSet::new();
        if let Some(name) = settings.logo_url.strip_prefix("assets/") {
            image_names.insert(name.to_string());
        }
        if let Some(name) = settings.favicon_url.strip_prefix("assets/") {
            image_names.insert(name.to_string());
        }
        for page in &public_pages {
            let rendered_page = html_by_id.get(page.id.as_str()).ok_or("Missing public page")?;
            image_names.extend(referenced_local_assets(&rendered_page.html));
        }
        if let Some(css)=&theme {
            image_names.extend(css_image_refs(css).into_iter().map(|(_,_,name)|name));
        }
        if !image_names.is_empty() {
            fs::create_dir(staged.join("assets")).map_err(|e| e.to_string())?;
            for (count,name) in image_names.iter().enumerate() {
                let (bytes, _) = read_asset(&dir, &name)?;
                fs::write(staged.join("assets").join(name), bytes).map_err(|e| e.to_string())?;
                report("assets",count+1,image_names.len());
            }
        } else {
            report("assets",0,0);
        }
        // Index only explicitly public, already sanitized page content.
        // Never copy private Markdown or raw user HTML into the search index.
        let index = public_search_index(&public_pages, &html_by_id);
        fs::write(staged.join("search-index.json"),
            serde_json::to_vec(&index)
                .map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
        report("indexing",1,1);
        if !settings.site_url.is_empty() {
            // Only explicitly public pages receive canonical sitemap URLs.
            fs::write(staged.join("sitemap.xml"), sitemap_xml(&settings.site_url, &public_pages))
                .map_err(|error| error.to_string())?;
        }
        // /pages/ provides compact HTML shells while the root files remain
        // canonical, preserving all existing link and sitemap URLs.
        fs::create_dir_all(staged.join("pages")).map_err(|e| e.to_string())?;
        for (count,page) in public_pages.iter().enumerate() {
            let rendered_page = html_by_id.get(page.id.as_str()).ok_or("Missing public page")?;
            let mut output = render_site_page_with_description(&settings, &pages, page, &rendered_page.html, &rendered_page.toc, Some(&rendered_page.search_text));
            if theme.is_some() {
                output = output.replace("</head>", "<link rel=\"stylesheet\" href=\"custom.css\">\n</head>");
            }
            let page_file = filename(&page.id);
            let mirror = minified_page_mirror(&output, &page_file)?;
            fs::write(staged.join("pages").join(&page_file), mirror).map_err(|e| e.to_string())?;
            fs::write(staged.join(page_file), output).map_err(|e| e.to_string())?;
            report("html",count+1,public_pages.len());
        }
        Ok(image_names.len())
    })();
    let local_images = match write_result {
        Ok(count) => count,
        Err(error) => {
            let _ = fs::remove_dir_all(&staged);
            return Err(error);
        }
    };
    // Atomic install is not a measurable percent and remains indeterminate.
    report("installing",0,0);
    // Stage first so an unsuccessful generation never replaces a working export.
    if backup.exists() { fs::remove_dir_all(&backup).map_err(|e| e.to_string())?; }
    if target.exists() { fs::rename(&target, &backup).map_err(|e| e.to_string())?; }
    if let Err(error) = fs::rename(&staged, &target) {
        if backup.exists() { let _ = fs::rename(&backup, &target); }
        return Err(error.to_string());
    }
    if backup.exists() { let _ = fs::remove_dir_all(&backup); }
    report("native-complete",1,1);
    Ok(SiteBuildResult {
        path: target.display().to_string(),
        public_pages: public_pages.len(),
        private_pages: pages.len() - public_pages.len(),
        local_images,
        warnings,
    })
}

/// Build from a canonical local source project without starting a Tauri window.
/// Prepared JSON consists only of *sanitized public pages* from tools/headless.
pub fn build_headless(project_dir: &Path, target: &Path, render_json: &Path) -> Result<SiteBuildResult, String> {
    let project_dir = project_dir.canonicalize().map_err(|e| format!("Project not found: {e}"))?;
    if !project_dir.is_dir() { return Err("Expected a Paglora project directory".into()); }
    let target_parent = target.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(target_parent).map_err(|e| e.to_string())?;
    let parent = target_parent.canonicalize().map_err(|e| e.to_string())?;
    if parent.starts_with(&project_dir) { return Err("Export must not overwrite editable project sources".into()); }
    let output_name = target.file_name().ok_or("Expected an export directory name")?;
    let output = parent.join(output_name);
    let config: serde_json::Value = serde_json::from_slice(&fs::read(project_dir.join("paglora.json")).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let id = config["id"].as_str().ok_or("Missing project identifier")?;
    if id != project_dir.file_name().and_then(|v| v.to_str()).ok_or("Invalid project folder")? || !valid_id(id) {
        return Err("Project folder and manifest ID mismatch".into());
    }
    let name = config["name"].as_str().ok_or("Missing project name")?;
    let json = fs::read(render_json).map_err(|e| e.to_string())?;
    if json.len() > 80_000_000 { return Err("Rendered page payload too large".into()); }
    let rendered: Vec<RenderedPage> = serde_json::from_slice(&json).map_err(|e| format!("Invalid prepared pages: {e}"))?;
    build_site_from_disk(&project_dir, name, &output, rendered)
}

#[cfg(test)]
mod website_font_tests {
    use super::*;

    #[test]
    fn site_settings_survive_an_interrupted_windows_style_replace() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("paglora-site-settings-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let mut settings = default_settings("Docs");
        settings.accent_color = "#cc3355".into();
        write_settings(&dir, &settings).unwrap();
        assert_eq!(read_settings(&dir, "Docs").unwrap().accent_color, "#cc3355");

        // Simulate interruption between renaming the live file to backup
        // and moving the staged file into place.
        fs::rename(dir.join(CONFIG), dir.join(SETTINGS_BACKUP)).unwrap();
        fs::write(dir.join(SETTINGS_STAGED), b"{incomplete").unwrap();
        assert_eq!(read_settings(&dir, "Docs").unwrap().accent_color, "#cc3355");

        settings.accent_color = "#115599".into();
        write_settings(&dir, &settings).unwrap();
        assert_eq!(read_settings(&dir, "Docs").unwrap().accent_color, "#115599");
        assert!(!dir.join(SETTINGS_BACKUP).exists());
        assert!(!dir.join(SETTINGS_STAGED).exists());

        // Existing project files still deserialize with defaulted fields.
        let mut legacy = serde_json::to_value(&settings).unwrap();
        legacy.as_object_mut().unwrap().remove("accentColor");
        fs::write(dir.join(CONFIG), serde_json::to_string(&legacy).unwrap()).unwrap();
        assert_eq!(read_settings(&dir, "Docs").unwrap().accent_color, "#2196f3");
        fs::remove_dir_all(dir).unwrap();
    }



    #[test]
    fn export_report_warns_about_private_and_missing_page_links_only() {
        let pages = vec![
            crate::pages::Page {
                id: "index".into(), title: "Overview".into(), is_public: true,
                layout: "docs".into(), nav_group: "Docs".into(),
            },
            crate::pages::Page {
                id: "guide".into(), title: "Guide".into(), is_public: true,
                layout: "docs".into(), nav_group: "Docs".into(),
            },
            crate::pages::Page {
                id: "draft".into(), title: "Draft".into(), is_public: false,
                layout: "docs".into(), nav_group: "Docs".into(),
            },
        ];
        let home = RenderedPage {
            id: "index".into(),
            html: r##"<a href="guide.html#usage">Valid</a>
<a href="./draft.html">Private</a><a href="gone.html">Missing</a>
<a href="gone.html">Duplicate</a><a href="https://example.test/external.html">External</a>
<a href="#local-heading">Local heading</a>"##.into(),
            toc: vec![],
            search_text: String::new(),
        };
        let mut html_by_id = HashMap::new();
        html_by_id.insert("index", &home);
        assert_eq!(
            site_export_link_warnings(&pages, &html_by_id),
            vec![
                "index links to missing page gone.html".to_string(),
                "index links to private page draft.html (not published)".to_string(),
            ]
        );
    }


    #[test]
    fn canonical_urls_and_sitemap_include_only_public_pages() {
        let mut settings = default_settings("Docs");
        assert_eq!(settings.site_url, "");
        let mut legacy = serde_json::to_value(&settings).unwrap();
        legacy.as_object_mut().unwrap().remove("siteUrl");
        let restored: SiteSettings = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored.site_url, "");
        settings.site_url = "https://m1nsook.github.io/paglora/".into();
        assert!(validate_settings(&settings).is_ok());
        let home = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Docs".into(),
        };
        let private = crate::pages::Page {
            id: "draft".into(), title: "Private".into(), is_public: false,
            layout: "docs".into(), nav_group: "Docs".into(),
        };
        let html = render_site_page(&settings, &[home.clone(), private.clone()], &home, "<p>Docs</p>", &[]);
        assert!(html.contains(r#"<link rel="canonical" href="https://m1nsook.github.io/paglora/index.html">"#));
        assert!(html.contains(r#"<meta property="og:url" content="https://m1nsook.github.io/paglora/index.html">"#));
        let xml = sitemap_xml(&settings.site_url, &[&home]);
        assert!(xml.contains("<loc>https://m1nsook.github.io/paglora/index.html</loc>"));
        assert!(!xml.contains("draft.html"));
        settings.site_url.clear();
        assert!(!render_site_page(&settings, &[home.clone()], &home, "<p>Docs</p>", &[]).contains("rel=\"canonical\""));
    }

    #[test]
    fn published_site_root_rejects_ambiguous_or_unsafe_urls() {
        for url in [
            "http://example.com/", "https://example.com", "https://user@example.com/",
            "https://example.com:443/", "https://example.com/../draft/",
            "https://example.com/%2e%2e/", "https://example.com/path?query=1/",
            "https://example.com/path#fragment/", "https://example..com/",
            "https://example.com//other/", "https://example.com/./",
        ] {
            assert!(!valid_site_url(url), "{url}");
        }
        assert!(valid_site_url("https://docs.example.com/"));
        assert!(valid_site_url("https://m1nsook.github.io/paglora/"));
    }

    #[test]
    fn configured_accent_is_not_overridden_by_dark_and_system_palettes() {
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        let mut settings = default_settings("Docs");
        settings.accent_color = "#ee3366".into();
        for mode in ["light", "dark", "system"] {
            settings.theme_mode = mode.into();
            let html = render_site_page(&settings, &[page.clone()], &page, "<p>Docs</p>", &[]);
            let vars = "--brand:#ee3366;--brand-light:color-mix(in srgb,var(--brand) 13%,var(--bg))";
            assert!(html.contains(&format!(":root{{{vars}}}")), "{mode}: light");
            assert!(html.contains(&format!(r#":root[data-color-mode="dark"]{{{vars}}}"#)), "{mode}: dark");
            assert!(html.contains(&format!(
                r#"@media(prefers-color-scheme:dark){{:root:not([data-color-mode="light"]){{{vars}}}}}"#
            )), "{mode}: system dark");
            assert!(html.find(&format!(r#"<link rel="stylesheet" href="{}">"#, stylesheet_chunk_name())).unwrap()
                < html.find(&format!(r#":root[data-color-mode="dark"]{{{vars}}}"#)).unwrap());
        }
    }

    #[test]
    fn layout_and_palette_settings_preserve_legacy_projects() {
        let mut settings = default_settings("Docs");
        assert_eq!(settings.site_layout, "classic");
        assert_eq!(settings.site_palette, "original");
        assert_eq!(settings.card_layout, "stacked");
        let mut legacy = serde_json::to_value(&settings).unwrap();
        legacy.as_object_mut().unwrap().remove("siteLayout");
        legacy.as_object_mut().unwrap().remove("sitePalette");
        legacy.as_object_mut().unwrap().remove("cardLayout");
        let restored: SiteSettings = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored.site_layout, "classic");
        assert_eq!(restored.site_palette, "original");
        assert_eq!(restored.card_layout, "stacked");
        for layout in ["classic", "inset", "reading"] {
            settings.site_layout = layout.into();
            assert!(validate_settings(&settings).is_ok(), "{layout}");
        }
        for palette in ["original", "graphite", "lavender", "linen"] {
            settings.site_palette = palette.into();
            assert!(validate_settings(&settings).is_ok(), "{palette}");
        }
        for card_layout in ["stacked", "grid"] {
            settings.card_layout = card_layout.into();
            assert!(validate_settings(&settings).is_ok(), "{card_layout}");
        }
        settings.card_layout = r#"grid" onpointerover="alert(1)"#.into();
        assert!(validate_settings(&settings).is_err());
        settings.card_layout = "grid".into();
        settings.site_layout = r#"classic" onclick="alert(1)"#.into();
        assert!(validate_settings(&settings).is_err());
        settings.site_layout = "classic".into();
        settings.site_palette = "../unsafe".into();
        assert!(validate_settings(&settings).is_err());
    }

    // Both preview and export call the same render_site_page function.
    fn html_preview_and_grid_markup_are_supported() -> bool {
        let mut settings = default_settings("Docs");
        settings.card_layout = "grid".into();
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        let html = render_site_page(&settings, &[page.clone()], &page, r##"<div class="paglora-card-grid"></div>"##, &[]);
        html.contains(r#"data-card-layout="grid""#)
            && html.contains("paglora-card-grid")
            && SITE_CSS.contains(r#":root[data-card-layout="grid"] .docs .paglora-card-grid"#)
    }

    #[test]
    fn theme_presets_share_preview_and_export_renderer_and_support_dark() {
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        let mut settings = default_settings("Docs");
        settings.site_layout = "inset".into();
        settings.site_palette = "lavender".into();
        settings.accent_color = "#b047d9".into();
        for mode in ["light", "dark", "system"] {
            settings.theme_mode = mode.into();
            let html = render_site_page(&settings, &[page.clone()], &page, "<p>Docs</p>", &[]);
            assert!(html.contains(r#"data-site-layout="inset" data-site-palette="lavender""#));
            assert!(html.contains("--brand:#b047d9;"), "{mode}");
            assert!(html.contains(r##"<meta name="theme-color" content="#17121e""##) || mode == "light");
        }
        assert!(html_preview_and_grid_markup_are_supported());
        assert!(SITE_CSS.contains(r#":root[data-site-layout="inset"] .site-body"#));
        assert!(SITE_CSS.contains(r#":root[data-site-layout="reading"] .site-nav"#));
        assert!(SITE_CSS.contains(r#":root[data-site-palette="lavender"][data-color-mode="dark"]"#));
        assert!(SITE_CSS.contains(r#":root[data-site-palette="lavender"]:not([data-color-mode="light"])"#));
        assert!(SITE_CSS.contains(r#"@media(max-width:800px)"#));
    }

    #[test]
    fn website_chrome_translates_with_site_language_and_english_fallback() {
        let page = crate::pages::Page {
            id: "index".into(), title: "Example".into(), is_public: true,
            layout: "docs".into(), nav_group: "Docs".into(),
        };
        let mut settings = default_settings("Example");
        settings.language = "pl".into();
        let polish = render_site_page(&settings, &[page.clone()], &page, "<p>Author text</p>", &[]);
        assert!(polish.contains("Przeglądaj dokumentację"));
        assert!(polish.contains("Przejdź do treści"));
        assert!(polish.contains("Author text"));
        assert!(polish.contains(r#"id="paglora-i18n""#));
        settings.language = "ko".into();
        assert!(render_site_page(&settings, &[page.clone()], &page, "<p>Original</p>", &[])
            .contains("문서 탐색"));
        settings.language = "fr".into();
        let fallback = render_site_page(&settings, &[page.clone()], &page, "<p>Original</p>", &[]);
        assert!(fallback.contains("Browse documentation"));
        assert!(fallback.contains("Skip to content"));
    }

    #[test]
    fn theme_color_metadata_tracks_the_configured_appearance() {
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        let mut settings = default_settings("Docs");
        for (mode, expected) in [
            ("light", r##"<meta name="theme-color" content="#ffffff">"##),
            ("dark", r##"<meta name="theme-color" content="#11151d">"##),
        ] {
            settings.theme_mode = mode.into();
            let html = render_site_page(&settings, &[page.clone()], &page, "<p>Docs</p>", &[]);
            assert!(html.contains(expected), "{mode}");
            assert_eq!(html.matches(r#"<meta name="theme-color""#).count(), 1);
        }
        settings.theme_mode = "system".into();
        let html = render_site_page(&settings, &[page.clone()], &page, "<p>Docs</p>", &[]);
        assert!(html.contains(r#"media="(prefers-color-scheme: light)""#));
        assert!(html.contains(r#"media="(prefers-color-scheme: dark)""#));
        assert_eq!(html.matches(r#"<meta name="theme-color""#).count(), 2);
    }


    #[test]
    fn image_library_only_lists_valid_regular_passive_images() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let folder = std::env::temp_dir().join(format!("paglora-image-library-{stamp}"));
        let dir = folder.join("assets");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("good.png"), b"\x89PNG\r\n\x1a\ncontent").unwrap();
        fs::write(dir.join("fake.png"), b"<script>bad</script>").unwrap();
        fs::write(dir.join("wrong.jpg"), b"\x89PNG\r\n\x1a\ncontent").unwrap();
        fs::write(dir.join("evil.svg"), b"<svg></svg>").unwrap();
        let items = list_project_assets(&folder).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "assets/good.png");
        assert_eq!(items[0].mime, "image/png");
        assert_eq!(items[0].bytes, 15);
        assert!(list_project_assets(&folder.join("missing")).unwrap().is_empty());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn local_assets_allow_only_passive_images_and_flat_safe_names() {
        assert!(valid_asset_url("assets/my-logo.png"));
        assert!(valid_asset_url("assets/guide-2.webp"));
        for url in ["assets/../secret.png", "assets/a.svg", "assets/a.png?x=1",
                    "assets/UPPER.PNG", "assets/a/b.png", "file:///etc/passwd"] {
            assert!(!valid_asset_url(url), "{url}");
        }
        assert_eq!(image_type(b"\x89PNG\r\n\x1a\nrest"), Some(("png", "image/png")));
        assert_eq!(image_type(b"\xff\xd8\xffrest"), Some(("jpg", "image/jpeg")));
        assert_eq!(image_type(b"GIF89arest"), Some(("gif", "image/gif")));
        assert_eq!(image_type(b"RIFFxxxxWEBP"), Some(("webp", "image/webp")));
        assert_eq!(image_type(b"<svg onload=alert(1)>"), None);
    }

    #[test]
    fn only_explicitly_referenced_local_images_are_selected_for_export() {
        let html = r#"<img src="assets/public.png"><img src="https://example.test/a.png"><p>src=&quot;assets/private.png&quot;</p><img src="assets/../bad.png">"#;
        let names = referenced_local_assets(html);
        assert_eq!(names.len(), 1);
        assert!(names.contains("public.png"));
        assert_eq!(base64_image(b"Man"), "TWFu");
        assert_eq!(base64_image(b"Ma"), "TWE=");
        assert_eq!(base64_image(b"M"), "TQ==");
    }

    #[test]
    fn exported_page_titles_obey_manifest_character_and_control_limits() {
        let mut page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        assert!(valid_site_page_entry(&page));
        page.title = "📘".repeat(100);
        assert!(valid_site_page_entry(&page));
        page.title.push('📘');
        assert!(!valid_site_page_entry(&page));
        page.title = "Docs\u{85}Internal".into();
        assert!(!valid_site_page_entry(&page));
        page.title = "   ".into();
        assert!(!valid_site_page_entry(&page));
    }

    #[test]
    fn page_navigation_groups_appear_in_generated_html() {
        let settings = default_settings("Docs");
        let pages = vec![
            crate::pages::Page { id: "index".into(), title: "Overview".into(), is_public: true, layout: "docs".into(), nav_group: "Getting Started".into() },
            crate::pages::Page { id: "api".into(), title: "API Guide".into(), is_public: true, layout: "docs".into(), nav_group: "API Reference".into() },
            crate::pages::Page { id: "private".into(), title: "Draft".into(), is_public: false, layout: "docs".into(), nav_group: "Secret".into() },
        ];
        let output = render_site_page(&settings, &pages, &pages[0], "<p>Docs</p>", &[]);
        assert!(output.contains("<details class=\"nav-group\" open><summary>Getting Started</summary>"));
        assert!(output.contains("<details class=\"nav-group\" open><summary>API Reference</summary>"));
        assert!(output.contains("api.html"));
        assert!(!output.contains("Secret"));
        assert!(!output.contains("private.html"));
        assert!(output.contains("</div></details>"));
        assert!(output.contains("aria-current=\"page\""));
        // The responsive navigation reuses only validated public links and
        // works as native details even when the browser runtime is disabled.
        assert!(output.contains("<details class=\"mobile-nav\"><summary>"));
        assert!(output.contains("<nav class=\"mobile-nav-inner\" aria-label="));
        assert!(!output.contains("private.html"));
        for asset in SITE_RUNTIME_ASSETS {
            assert!(output.contains(&format!("<script defer src=\"{}\"></script>", runtime_chunk_name(asset))));
            assert!(!asset.source.is_empty());
        }
        assert!(SITE_RUNTIME_ASSETS[1].source.contains("search-index.json"));
        assert!(SITE_RUNTIME_ASSETS[0].source.contains("paglora:theme:"));
    }

    #[test]
    fn website_language_is_validated_and_defaults_for_existing_projects() {
        let settings = default_settings("Docs");
        assert_eq!(settings.language, "en");
        let mut legacy = serde_json::to_value(&settings).unwrap();
        legacy.as_object_mut().unwrap().remove("language");
        let restored: SiteSettings = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored.language, "en");

        let mut korean = settings.clone();
        korean.language = "ko".into();
        assert!(validate_settings(&korean).is_ok());
        let page = crate::pages::Page {
            id: "index".into(), title: "개요".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        let output = render_site_page(&korean, &[page.clone()], &page, "<p>안녕하세요</p>", &[]);
        assert!(output.contains(r#"<html lang="ko" dir="ltr" data-color-mode="#));

        let mut arabic = settings.clone();
        arabic.language = "ar".into();
        assert!(validate_settings(&arabic).is_ok());
        let rtl = render_site_page(&arabic, &[page.clone()], &page, "<p>مرحبا</p>", &[]);
        assert!(rtl.contains(r#"<html lang="ar" dir="rtl" data-color-mode="#));
        assert!(SITE_CSS.contains(r#":root[dir="rtl"] .docs blockquote"#));

        korean.language = r#"en" onload="alert(1)"#.into();
        assert!(validate_settings(&korean).is_err());
        korean.language = "xx".into(); // Unreviewed locale is not published.
        assert!(validate_settings(&korean).is_err());
        for unsafe_code in ["en-", "en-us", "EN", "sv-SE-any", "pl\" onload=\"alert(1)", "../pl"] {
            assert!(!safe_site_language_tag(unsafe_code), "{unsafe_code}");
        }
        for existing in ["en", "pl", "ko", "fr", "zh-CN", "ar", "pt-BR"] {
            assert!(supported_site_language(existing), "{existing}");
        }
        assert_eq!(site_text_direction("ar"), "rtl");
        assert_eq!(site_text_direction("ko"), "ltr");
        assert_eq!(social_locale("sv-SE"), "sv_SE");
        assert_eq!(social_locale("en"), "en_US");
    }

    #[test]
    fn compact_pages_mirror_keeps_preformatted_body_and_canonical_root_links() {
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Docs".into(),
        };
        let settings = default_settings("Docs");
        let html = render_site_page(&settings, &[page.clone()], &page,
            "<pre><code>  leading spaces\n\n  next line</code></pre>", &[]);
        let compact = minified_page_mirror(&html, "index.html").unwrap();
        assert!(compact.contains("<base href=\"../index.html\">"));
        assert!(compact.contains("name=\"robots\" content=\"noindex,follow\""));
        assert!(compact.contains("  leading spaces\n\n  next line"));
        assert!(compact.contains("href=\"_paglora/chunks/css/"));
        assert!(!compact.contains("</article>\n</div>"));
    }

    #[test]
    fn real_disk_export_includes_chunks_pages_favicon_and_no_private_content() {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("paglora-export-chunks-{unique}"));
        let project = root.join("project");
        fs::create_dir_all(project.join("assets")).unwrap();
        fs::write(project.join("pages.json"),
            r#"{"formatVersion":1,"pages":[{"id":"index","title":"Overview","isPublic":true,"layout":"docs","navGroup":"Docs"},{"id":"draft","title":"Secret","isPublic":false,"layout":"docs","navGroup":"Internal"}]}"#).unwrap();
        fs::write(project.join("assets/favicon.png"), b"\x89PNG\r\n\x1a\nlocal-icon").unwrap();
        let mut settings = default_settings("Docs");
        settings.favicon_url = "assets/favicon.png".into();
        write_settings(&project, &settings).unwrap();
        let rendered = vec![RenderedPage {
            id: "index".into(), html: "<p>Public docs</p>".into(),
            toc: vec![], search_text: "Public docs".into(),
        }];
        let output = root.join("public");
        let result = build_site_from_disk(&project, "Docs", &output, rendered).unwrap();
        assert_eq!(result.public_pages, 1);
        assert_eq!(result.private_pages, 1);
        let html = fs::read_to_string(output.join("index.html")).unwrap();
        assert!(html.contains(&stylesheet_chunk_name()));
        assert!(html.contains(&runtime_chunk_name(&SITE_RUNTIME_ASSETS[0])));
        assert!(html.contains(r#"href="assets/favicon.png""#));
        assert!(output.join("assets/favicon.png").is_file());
        assert!(output.join(stylesheet_chunk_name()).is_file());
        for asset in SITE_RUNTIME_ASSETS {
            assert!(output.join(runtime_chunk_name(asset)).is_file());
        }
        let mirror = fs::read_to_string(output.join("pages/index.html")).unwrap();
        assert!(mirror.contains(r#"<base href="../index.html">"#));
        assert!(mirror.contains(r#"name="robots" content="noindex,follow""#));
        assert!(output.join("search-index.json").is_file());
        assert!(!output.join("draft.html").exists());
        assert!(!output.join("pages/draft.html").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn published_runtime_and_css_use_deterministic_content_hashed_chunks() {
        let stylesheet = stylesheet_chunk_name();
        assert!(stylesheet.starts_with("_paglora/chunks/css/"));
        assert!(stylesheet.ends_with(".css"));
        assert_eq!(stylesheet, stylesheet_chunk_name());
        let mut changed = SITE_RUNTIME_ASSETS[0].source.as_bytes().to_vec();
        let before = chunk_hash(&changed);
        changed.push(1);
        assert_ne!(chunk_hash(&changed), before);
        for asset in SITE_RUNTIME_ASSETS {
            let path = runtime_chunk_name(asset);
            assert!(path.starts_with("_paglora/chunks/") && path.ends_with(".js"));
            assert!(site_runtime_script_tags().contains(&path));
        }
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Docs".into(),
        };
        let output = render_site_page(&default_settings("Docs"), &[page.clone()], &page, "<p>Hi</p>", &[]);
        assert!(output.contains(&format!("href=\"{stylesheet}\"")));
        assert!(!output.contains("<script defer src=\"paglora.js\""));
    }

    #[test]
    fn favicon_is_opt_in_safe_and_preserves_legacy_projects() {
        let mut settings = default_settings("Docs");
        assert!(settings.favicon_url.is_empty());
        let mut old = serde_json::to_value(&settings).unwrap();
        old.as_object_mut().unwrap().remove("faviconUrl");
        let legacy: SiteSettings = serde_json::from_value(old).unwrap();
        assert!(legacy.favicon_url.is_empty());
        settings.favicon_url = "assets/project-icon.png".into();
        assert!(validate_settings(&settings).is_ok());
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Docs".into(),
        };
        let output = render_site_page(&settings, &[page.clone()], &page, "<p>Hi</p>", &[]);
        assert!(output.contains(r#"<link rel="icon" type="image/png" href="assets/project-icon.png">"#));
        settings.favicon_url = "https://example.com/icon.webp".into();
        assert!(validate_settings(&settings).is_ok());
        for value in ["javascript:alert(1)", "http://example.com/favicon.png", "../favicon.png",
            "assets/../evil.png", "data:image/svg+xml,bad", "assets/icon.svg"] {
            settings.favicon_url = value.into();
            assert!(validate_settings(&settings).is_err(), "{value}");
        }
    }

    #[test]
    fn public_pages_include_escaped_social_preview_metadata() {
        let mut settings = default_settings("Docs & Friends");
        settings.title = "Docs & Friends".into();
        settings.description = "Guides <for> everyone".into();
        let pages = vec![crate::pages::Page {
            id: "index".into(),
            title: "Overview & Setup".into(),
            is_public: true,
            layout: "docs".into(),
            nav_group: "Documentation".into(),
        }];
        let html = render_site_page(&settings, &pages, &pages[0], "<p>Welcome</p>", &[]);
        assert!(html.contains("<meta property=\"og:type\" content=\"article\">"));
        assert!(html.contains("<meta property=\"og:title\" content=\"Overview &amp; Setup | Docs &amp; Friends\">"));
        assert!(html.contains("<meta property=\"og:description\" content=\"Guides &lt;for&gt; everyone\">"));
        assert!(html.contains("<meta property=\"og:site_name\" content=\"Docs &amp; Friends\">"));
        assert!(html.contains("<meta name=\"twitter:card\" content=\"summary\">"));
        assert!(!html.contains("content=\"Guides <for> everyone\""));
    }

    #[test]
    fn unicode_heading_anchors_are_safe_in_public_toc() {
        let settings = default_settings("Docs");
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        let toc = vec![TocHeading { id: "설치-방법".into(), title: "설치 방법".into(), level: 2 }];
        let html = render_site_page(&settings, &[page.clone()], &page, "<h2 id=\"설치-방법\">설치 방법</h2>", &toc);
        assert!(html.contains("href=\"#설치-방법\""));
        assert!(html.contains("설치 방법</a>"));
        let unsafe_toc = vec![TocHeading { id: "bad\" onclick=\"alert-1".into(), title: "Bad".into(), level: 2 }];
        let safe_html = render_site_page(&settings, &[page.clone()], &page, "<p>Safe</p>", &unsafe_toc);
        assert!(!safe_html.contains("onclick="));
    }

    #[test]
    fn public_page_descriptions_use_sanitized_excerpts_with_safe_fallback() {
        let mut settings = default_settings("Docs");
        settings.description = "Site-wide fallback".into();
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        let content = "  Unique   page <notes> & details. ";
        let html = render_site_page_with_description(&settings, &[page.clone()], &page,
            "<p>Welcome</p>", &[], Some(content));
        assert!(html.contains("<meta name=\"description\" content=\"Unique page &lt;notes&gt; &amp; details.\">"));
        assert!(html.contains("<meta property=\"og:description\" content=\"Unique page &lt;notes&gt; &amp; details.\">"));
        assert!(!html.contains("content=\"Site-wide fallback\""));
        let legacy = render_site_page_with_description(&settings, &[page.clone()], &page,
            "<p>Welcome</p>", &[], Some("  "));
        assert!(legacy.contains("<meta name=\"description\" content=\"Site-wide fallback\">"));
        let long = "x".repeat(500);
        assert_eq!(page_meta_description("fallback", Some(&long)).chars().count(), 160);
    }

    #[test]
    fn search_index_includes_only_public_pages() {
        let pages = vec![
            crate::pages::Page { id: "index".into(), title: "Public".into(), is_public: true, layout: "docs".into(), nav_group: "Docs".into() },
            crate::pages::Page { id: "draft".into(), title: "Secret Draft".into(), is_public: false, layout: "docs".into(), nav_group: "Internal".into() },
        ];
        let rendered = vec![
            RenderedPage { id: "index".into(), html: "<p>Visible</p>".into(), toc: vec![], search_text: "Visible page".into() },
            RenderedPage { id: "draft".into(), html: "<p>Secret</p>".into(), toc: vec![], search_text: "Private content".into() },
        ];
        let refs = pages.iter().filter(|p| p.is_public).collect::<Vec<_>>();
        let by_id = rendered.iter().map(|p| (p.id.as_str(), p)).collect::<HashMap<_, _>>();
        let index = public_search_index(&refs, &by_id);
        assert_eq!(index["version"], 1);
        assert_eq!(index["pages"].as_array().unwrap().len(), 1);
        assert_eq!(index["pages"][0]["url"], "index.html");
        let json = serde_json::to_string(&index).unwrap();
        assert!(!json.contains("Secret"));
        assert!(!json.contains("Private content"));
    }

    #[test]
    fn social_preview_metadata_uses_localized_and_escaped_values() {
        let mut settings = default_settings("Docs & Guides");
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview <Setup>".into(), is_public: true,
            layout: "docs".into(), nav_group: "Documentation".into(),
        };
        for (language, locale) in [
            ("en", "en_US"), ("ko", "ko_KR"), ("pl", "pl_PL"),
            ("zh-CN", "zh_CN"), ("zh-TW", "zh_TW"), ("ar", "ar_SA"),
        ] {
            settings.language = language.into();
            let html = render_site_page_with_description(
                &settings, &[page.clone()], &page, "<p>Docs</p>", &[], Some("Hello & <friends>")
            );
            assert!(html.contains(&format!(r#"<meta property="og:locale" content="{locale}">"#)));
            assert!(html.contains(r#"<meta name="twitter:title" content="Overview &lt;Setup&gt; | Docs &amp; Guides">"#));
            assert!(html.contains(r#"<meta name="twitter:description" content="Hello &amp; &lt;friends&gt;">"#));
            assert!(!html.contains(r#"<meta name="twitter:title" content="Overview <Setup>"#));
        }
    }

    #[test]
    fn google_fonts_are_opt_in_and_default_projects_keep_geist() {
        let settings = default_settings("Test Project");
        assert_eq!(settings.google_font_family, "");
        assert!(google_font_markup(&settings.google_font_family).is_empty());
        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn valid_google_font_appears_in_generated_static_html() {
        let mut settings = default_settings("Test Project");
        settings.google_font_family = "Noto Sans KR".into();
        assert!(validate_settings(&settings).is_ok());
        let page = crate::pages::Page {
            id: "index".into(), title: "Overview".into(), is_public: true, layout: "docs".into(), nav_group: "Documentation".into()
        };
        let output = render_site_page(&settings, &[page.clone()], &page, "<p>Test</p>", &[]);
        assert!(output.contains("https://fonts.googleapis.com/css2?family=Noto+Sans+KR&amp;display=swap"));
        assert!(output.contains("body{font-family:\"Noto Sans KR\""));
        assert!(output.contains(&format!("<link rel=\"stylesheet\" href=\"{}\">", stylesheet_chunk_name())));
    }

    #[test]
    fn custom_font_name_does_not_allow_html_or_query_injection() {
        for invalid in ["<script>", "Roboto&display=block", "Inter\";body{}", "Noto  Sans", " Noto Sans ", ""] {
            if !invalid.is_empty() { assert!(!valid_google_font_family(invalid)); }
        }
        assert!(valid_google_font_family(""));
        assert!(valid_google_font_family("Plus Jakarta Sans"));
        assert!(valid_google_font_family("M PLUS 1"));
    }
}

#[cfg(test)]
mod css_asset_tests {
    use super::*;
    #[test]
    fn only_actual_local_css_url_tokens_are_public_dependencies(){
        let css=r#"/* url(assets/hidden.png) */
            .hero{background:url("assets/hero.png")}
            .next{background:url('assets/next.webp')}
            .icon{background:URL( assets/icon.gif )}
            .remote{background:url(https://cdn.example.com/a.png)}
            .other{content:"url(assets/literal.png)"}
            .data{background:url(data:image/png;base64,AAAA)}
            .bad{background:url(../assets/traversal.png)}"#;
        let found:HashSet<_>=css_image_refs(css).into_iter().map(|(_,_,name)|name).collect();
        assert_eq!(found,["hero.png","next.webp","icon.gif"].iter().map(|s|s.to_string()).collect());
    }
    #[test]
    fn css_reference_span_replacement_does_not_touch_unrelated_text(){
        let css="/* url(assets/ignored.png) */ .hero{background:url(assets/cover.png)}";
        let refs=css_image_refs(css);
        assert_eq!(refs.len(),1);
        let (start,end,name)=&refs[0];
        assert_eq!(name,"cover.png");
        assert_eq!(&css[*start..*end],"assets/cover.png");
    }
}

#[cfg(test)]
mod asset_cleanup_tests {
    use super::*;
    fn fixture()->PathBuf{
        let nonce=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir=std::env::temp_dir().join(format!("paglora-asset-clean-{}-{nonce}",std::process::id()));
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::create_dir_all(dir.join("theme")).unwrap();
        fs::write(dir.join("paglora.json"),r#"{"formatVersion":1,"id":"test","name":"Test"}"#).unwrap();
        fs::write(dir.join("pages.json"),r#"{"formatVersion":1,"pages":[{"id":"index","title":"Home","isPublic":true,"layout":"docs","navGroup":"Documentation"},{"id":"private","title":"Private","isPublic":false,"layout":"docs","navGroup":"Documentation"}]}"#).unwrap();
        fs::write(dir.join("site.json"),r#"{"logoUrl":""}"#).unwrap();
        fs::write(dir.join("docs/index.md"),"# Home").unwrap();
        fs::write(dir.join("docs/private.md"),"# Secret").unwrap();
        for name in ["used.png","private.png","css.png","unused.png"] {
            fs::write(dir.join("assets").join(name),b"\x89PNG\r\n\x1a\ncontent").unwrap();
        }
        dir
    }
    #[test]
    fn no_cleanup_when_private_pages_styles_or_logo_reference_images(){
        let dir=fixture();
        fs::write(dir.join("docs/index.md"),"![hero](assets/used.png)").unwrap();
        fs::write(dir.join("docs/private.md"),"![private](assets/private.png)").unwrap();
        fs::write(dir.join("theme/custom.css"),"body{background:url('assets/css.png')}").unwrap();
        let options=unused_candidates_at(&dir).unwrap();
        assert_eq!(options.len(),1);
        assert_eq!(options[0].url,"assets/unused.png");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn confirmed_cleanup_is_rechecked_and_images_restore_without_overwrite(){
        let dir=fixture();let options=unused_candidates_at(&dir).unwrap();
        let candidate=options.iter().find(|x|x.url=="assets/unused.png").unwrap();
        assert!(move_unused_at(&dir,&candidate.url,&candidate.fingerprint,"WRONG").is_err());
        fs::write(dir.join("docs/private.md"),"assets/unused.png").unwrap();
        assert!(move_unused_at(&dir,&candidate.url,&candidate.fingerprint,"unused.png").is_err());
        fs::write(dir.join("docs/private.md"),"# Private").unwrap();
        move_unused_at(&dir,&candidate.url,&candidate.fingerprint,"unused.png").unwrap();
        assert!(!dir.join("assets/unused.png").exists());
        assert!(dir.join("removed-assets/unused.png").exists());
        assert_eq!(recoverable_assets_at(&dir).unwrap().len(),1);
        fs::write(dir.join("assets/unused.png"),b"\x89PNG\r\n\x1a\nreplacement").unwrap();
        assert!(restore_image_at(&dir,"assets/unused.png").is_err());
        fs::remove_file(dir.join("assets/unused.png")).unwrap();
        restore_image_at(&dir,"assets/unused.png").unwrap();
        assert!(dir.join("assets/unused.png").exists());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn legacy_project_without_site_settings_can_recover_unused_images(){
        let dir=fixture();
        fs::remove_file(dir.join("site.json")).unwrap();
        assert_eq!(unused_candidates_at(&dir).unwrap().len(),4);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn invalid_sources_block_cleanup_and_unchanged_files_remain(){
        let dir=fixture();
        fs::remove_file(dir.join("docs/private.md")).unwrap();
        assert!(unused_candidates_at(&dir).is_err());
        assert!(dir.join("assets/unused.png").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
