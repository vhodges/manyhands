---
title: "Wave 03 Velotype — Pinned Source-Only Fallback Assessment"
date: 2026-10-06
status: blocked
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M48Z1S9ZNB89N3VVKYKVAGB4"
---

# Wave 03 Velotype — Task 7

## Disposition and authority

**Do not proceed to executable extraction of this pin under the thin-adapter
boundary.** Source inspection establishes load-time newline normalization and
whole-tree rich/source reserialization, incompatible with the approved exact
no-op and untouched-block requirements. The upstream core is also a different
GPUI package from Kit's core, and the editor directly owns file/resource access.
These findings require editor-core/extraction/GPUI-port decisions, not just host
adapters. This is a blocked **unmodified pinned path**, not a native result,
product infeasibility verdict or dependency selection.

Only the approved Task 7 source-only fallback assessment ran. Public HTTP source
retrieval and documentation/evidence checks were performed; no Rust command,
Cargo resolution, upstream test, build, executable extraction, integration,
patch, fork, GPUI upgrade, webview or new editor was performed. No Manyhands
service/API, key, domain SSH, publication remote or rendered content was opened.
Tasks 4–6 remain **not executed**. The sole committed project file is this report;
Cargo/source/tests/production/Devenv and controller-owned reports, API audit,
ticket and ledger are unchanged. No rebase or baseline rerun was needed.

[Contract](wave-03-readiness-exploration.md) |
[Design](../plans/2026-10-06-wave-03-readiness-exploration-design.md) |
[Task 7 plan](../plans/2026-10-06-wave-03-readiness-exploration-implementation.md#task-7--bounded-fallback-assessment-and-recommendation) |
[Execution ledger](../plans/2026-10-06-wave-03-readiness-exploration-execution.md) |
[Reviewed Zorite stop](wave-03-editor-feasibility.md#pinned-source-preservation-blocker-r3).

## Immutable provenance and review evidence

- Task base: `94672ab7585f558d87d998ed29d8f984c85e7054`.
- Branch: `manyhands/ticket/01M48S808PF2D8ZWVYM918RK2M`.
- Cwd: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M48S808PF2D8ZWVYM918RK2M`.
- Upstream: <https://github.com/manyougz/velotype>.
- Resolved revision: `ed65977be94f2f2703037fcb8b6cbab2e7579571`, upstream
  manifest version **0.7.2**, commit dated 2026-08-14T17:21:00Z, merge PR #117.
  This is a revision pin, not an asserted released/published editor library.
- Resolution performed on 2026-10-06 between **15:55:01Z and 15:55:15Z** using
  <https://api.github.com/repos/manyougz/velotype/commits/HEAD>. Saved response
  identifies Git tree `2ef80a35c5abaf565d87df7d7fb8feeb2087e725`; HEAD may move
  later. Inspection uses only the resolved revision, never current README APIs.
- Exact archive URL:
  <https://codeload.github.com/manyougz/velotype/tar.gz/ed65977be94f2f2703037fcb8b6cbab2e7579571>.
  Recursive tree URL:
  <https://api.github.com/repos/manyougz/velotype/git/trees/2ef80a35c5abaf565d87df7d7fb8feeb2087e725?recursive=1>.
  All **148** extracted file blobs were checked against GitHub's Git blob SHA-1
  entries and byte-compared with archive members; recursive response is not
  truncated. GitHub's API reports a verified signature; no independent PGP
  verification was performed. HTTPS/archive/hash evidence is not authorship proof.

All local evidence is under ignored
[`target/readiness-evidence/velotype-assessment/`](../../target/readiness-evidence/velotype-assessment/)
(**E/** below). The full extracted tree is
`E/velotype-ed65977be94f2f2703037fcb8b6cbab2e7579571/` (**S/**).
Source citations `editor/...`, `components/...`, `net/...`, etc. are relative to
**S/src/** with exact line numbers. Immutable web equivalent:
<https://github.com/manyougz/velotype/tree/ed65977be94f2f2703037fcb8b6cbab2e7579571/src>.
`E/numbered-source/` supplies line-numbered copies of the core cited modules for
shell-less review; originals, not numbered copies, are hashed. All are ignored
inspection evidence, not vendored build inputs. Preserve E/ with this review.

| SHA-256 identity | Digest |
| --- | --- |
| Source archive | `3e000aaae1c8f6780699b6049ed325e4bb8f135723837e555f22e3077c4bbe5d` |
| Upstream Cargo.toml | `090527048f3bf16ef8cbc9d8f8bcf5256c5c2b171907bbb7b0c58c679c7bb3af` |
| Upstream Cargo.lock | `341c1c648df0b73ec09d789158ab8c77bebb1ef3a09a98b11f0d143ef5d4e352` |
| LICENSE-APACHE | `af5cab59eaa95e6ccd5bf0394627497d756c41a419f77280f376135f29d1ca6f` |
| 61-module inventory (path, line count, SHA-256) | `db5effe6fb5d66d825bb3696eee4936c8e23698388b43c3ea987187c3999b70f` |
| Complete upstream/downloaded/baseline source hash manifest | `77cc7ea6ef82a1bfd53dd97f6df3a590e54d764aa8a5f3c2705d1657ba948503` |

`E/resolution.{json,headers}`, `E/tree.{json,headers}`,
`E/archive.headers`, `E/archive-members.txt` and `E/retrieval.log` record exact
request provenance. `E/verified-packages.json` records ten downloaded registry
archives, manifest licenses/notices and checksums verified against the upstream
lock; `E/upstream-lock-inventory.json` preserves all **818 upstream lock nodes**
and dependency edges. This is an upstream lock inventory, **not** a resolved
Manyhands extraction graph, platform closure or successful compatibility check.
`E/audit.py`, `E/audit.log`, `E/identity-hashes.sha256`,
`E/source-hashes.sha256`, `E/evidence-hashes.sha256` authenticate inspection.
`E/task-base.txt`, `E/task-commit.txt`, `E/task7.diff` and `E/clean-state.txt`
record the exact sole-file committed range and final unstaged/clean state;
`E/committed-report.md` is the committed report snapshot. Their post-commit
hashes are outside this document to avoid a self-referential commit/hash.

## Load → model → edit → source/export trace

### Proven preservation blockers (R3 and untouched ranges)

1. **Before parsing:** `Editor::from_markdown` at `editor/mod.rs:300–307`
   unconditionally executes `markdown.replace("\r\n", "\n").replace('\r', "\n")`.
   The stored stable history source is also normalized (`mod.rs:370–382`) and
   then refreshed from the model. The replacement load path repeats it in
   `editor/file_drop.rs:83–100`. There is no constructor policy/callback to opt
   out. Source-derived counterexample: body bytes `alpha\r\n` cannot be returned
   unchanged through this rich constructor even without editing. This is a
   source conclusion, not an executed-editor fixture.
   [Immutable load code](https://github.com/manyougz/velotype/blob/ed65977be94f2f2703037fcb8b6cbab2e7579571/src/editor/mod.rs#L300-L307).
2. **Semantic rather than original-source model:** `editor/document.rs:927–986`
   splits normalized text into lines, reduces blank runs to empty paragraph
   records, and does not retain their whitespace. Supported native blocks use
   `InlineTextTree::from_markdown` (`document.rs:825–834`). `BlockRecord`
   (`components/block/state.rs:485–494`) stores kind, title, table/HTML and UUID
   tree relationships, with `raw_fallback` only for designated opaque kinds,
   **not an original source range/lexeme for every native block**.
   `components/markdown/inline.rs:451–490` consumes formatting markers into
   fragments/style flags and normalizes code-span content. Some constructs keep
   source (reference/autolinks and math), but that is not a complete lossless CST.
3. **Readback/save serializes every rich block:** `DocumentTree::markdown_text`
   (`editor/tree.rs:136–148`) collects/rejoins generated lines. Root separator
   policy (`tree.rs:442–483`), regenerated fences/quotes/list nesting
   (`tree.rs:486–617`), `BlockRecord::markdown_line`
   (`components/block/state.rs:589–628`) and the table serializer
   (`components/markdown/table.rs:741–755`) reconstruct spelling, whitespace,
   indentation, ordinals and delimiters. A focused edit to one block still
   exports all the other native blocks through this serializer; there is no
   untouched-original-byte splice path.
4. **Counterexamples are in upstream tests, not inferred from feature claims:**
   `editor/document.rs:2031–2059` explicitly expects a Setext heading to become
   `## Heading`; `document.rs:3957–3966` expects loaded `alpha\n` to serialize as
   `alpha`. These tests were **read, not run**. Their expectations agree with
   the parser/serializer implementation. Changing another paragraph does not
   restore that untouched heading spelling or original final newline.
   [Immutable final-newline expectation](https://github.com/manyougz/velotype/blob/ed65977be94f2f2703037fcb8b6cbab2e7579571/src/editor/document.rs#L3957-L3966).
5. **Mode switch is conversion, not presentation-only:**
   `editor/window_state.rs:328–367` serializes rich content into a new single
   source block and replaces all document roots; reverse switching reparses the
   source and recreates rich blocks/table/image runtimes. The outer Editor and
   its undo vectors survive, but original block identities do not. The toggle
   refreshes the stable source snapshot and does not mark dirty; lack of a dirty
   flag is not exact-byte evidence. Selection mappings cannot recover discarded
   source. [Immutable toggle](https://github.com/manyougz/velotype/blob/ed65977be94f2f2703037fcb8b6cbab2e7579571/src/editor/window_state.rs#L328-L367).

Retaining the original canonical bytes in a host could prevent a no-change
**save**, and keeping the YAML header out of the editor is a thin host policy.
Neither makes editor readback/mode switching exact, nor makes focused rich
edits preserve all untouched blocks. Reconstructing lost spellings by comparing
semantic output to the original would be editor-engine work, not an adapter.
Removing only the newline replacement leaves the independent rich serializer
blockers. Loading everything as raw blocks/source would not satisfy R5's rich
vocabulary and is not an authorized product fallback.

### Unsupported syntax and source mappings

There is meaningful preservation machinery, not wholesale dropping of all
unknown nodes. The importer keeps reference definitions, unsupported footnotes,
HTML/raw fallbacks and malformed math as opaque regions
(`editor/document.rs:1008–1032,1118–1123`); raw block serialization uses
`raw_fallback` (`editor/tree.rs:579–600`). `BlockRecord` raw kinds include raw
Markdown, comments, HTML, math and Mermaid (`components/block/state.rs:574–586`).
HTML parsing/safety determines native versus raw rather than invoking a webview
(`document.rs` imports `HtmlSafetyClass`; `components/markdown/html.rs`).

However, opaque-region bytes are already downstream of CRLF normalization.
Root separators and nested indentation are regenerated around them. Not every
unknown extension is explicitly classified as opaque: ordinary paragraph import
still parses inline syntax (`document.rs:1137–1159`); invalid root table regions
become plain paragraphs (`document.rs:1095–1109`). Preservation of every custom
directive/mixed supported-unsupported boundary is therefore not established.

`editor/source_mapping.rs:128–155,309–375,676–739` builds content/source offset
maps and block ranges **from current generated Markdown** (tables call the
serializer at line 325). They support caret/selection conversion, not spans into
an immutable original buffer. Inline source/visible maps likewise cannot encode
lost heading/fence/blank-line spellings. A new original-span model, mutation
invalidation rules and exact splice export across structural edits would be
core/fork design work beyond current authority.

## Editing, shared undo, tables and native input

| Boundary | Actual inspected source | What remains unproved |
| --- | --- | --- |
| Event/edit engine | `editor/runtime_context.rs:69–85` creates Block entities and subscribes to BlockEvent. `editor/events.rs:1–21` centralizes split/merge/indent/outdent/paste/focus; `components/block/runtime/mod.rs:1626–1748` routes visible-range edits through projection/raw/inline parsing paths. | No Manyhands edit/readback adapter exists; UTF-8-safe target edit and untouched-range fixtures not run. This is an engine, not a host String replacement. |
| Shared history | `editor/history.rs:59–75,100–136` captures document source/selection, coalesces edits; `history.rs:273–356` restores/reparses in current mode and implements undo/redo. Limits/coalescing: `editor/mod.rs:296–298` (200 entries, 1-second window). `editor/tests.rs:2566–2670` specifies typing/coalescing/redo. | Outer history survives toggle, so do not claim an unconditional undo reset. Snapshots use generated/normalized source; undo cannot promise the original discarded bytes. Native cross-mode/IME/table histories not tested. |
| Rich vocabulary | `components/block/state.rs:108` BlockKind; importer `editor/document.rs:1034–1129` recognizes headings, image, code, list, quote, separator, tables and math. Inline fragments contain emphasis/strong/link styles (`components/markdown/inline.rs:451–490`). | Recognition is not native usability/fidelity acceptance. Heading/list/fence/inline normalization already blocks exact no-op. Required rich construct-by-construct actions remain not-tested. |
| Native tables | `editor/table_edit.rs:10–137` binds per-cell Block entities and syncs records; append column/row at 139/182, alignment 271, row/column move 308/365, row/header/column deletion 420/481/516. Actions capture document undo and rebuild runtimes. `components/markdown/table.rs:121–216,741–755` owns data mutation and whole-table output. | Cell edits and structural operations are real source seams but not public standalone editor APIs; independent changed-table boundaries, untouched blocks and undo need proof. No original cell/table layout spans retained. |
| Native text/IME | `components/block/input.rs:16–178` implements EntityInputHandler text/range/mark/replace bridges with UTF-16↔UTF-8 conversion, including cross-block replace events. Bounds/point geometry at 181–235 uses painted text layout. | Real Linux Wayland composition, combining/non-BMP/bidi selection and ported platform callbacks not-tested; implementation does not prove conformance. |
| Clipboard | `components/block/interactions.rs:886–978` copy/paste reads OS clipboard, handles images before raw-mode checks, flattens table newlines, keeps raw text or normalizes multiline rich paste. | No demonstrated rich-HTML clipboard import/plain-paste parity or keyboard-only journey. Native image paste can create files (below), including paths before source-raw checks. |
| Rendering/performance | `editor/window_state.rs:75–170` computes row windowing/focus islands; `editor/render.rs` and `components/block/{element,render}.rs` paint/focus/layout. | Source windowing/benches are not rendered p95 or responsiveness measurements. No 100 KiB/100-sample/larger-source run. |

## Persistence and resource ownership (R2)

**Embedding the intact upstream Editor is not deny-by-default host containment.**
There is no exposed policy object/disabled-provider constructor governing all
these paths. They could be made host-controlled only by approved extraction and
core policy edits; do not call them permanently uncontrollable in every future
adaptation. Passing `file_path=None` still permits cwd-derived resources and
save prompts.

- Markdown save chooses raw source or whole-tree Markdown
  (`editor/persistence.rs:50–55`), then writes directly via `std::fs::write`
  (`persistence.rs:87–114,116–174`). Render/window actions expose pending save,
  save-as and close flows. `editor/close.rs` and `file_drop.rs` own unsaved/save
  dialogs; dropped Markdown is directly read at `file_drop.rs:71–80` and can be
  saved before replacement at `file_drop.rs:231–281`. No Manyhands expected
  observation, protected draft store, replay or host persistence authority.
- Images: `components/markdown/image.rs:102–115` accepts HTTP(S), absolute paths
  and base-dir joins without an allowlist/traversal/symlink check.
  `editor/runtime_context.rs:88–93` falls back to cwd for base dir.
  `components/block/runtime/image.rs:29–40` computes image resources;
  `components/block/render.rs:460–461,999–1000` passes local/remote sources to
  GPUI `img`. Downloaded **gpui 0.2.2** `src/elements/img.rs:591–610` directly
  reads files for Resource::Path and performs HTTP client GET for Resource::Uri.
  `net/mod.rs:23–57` installs a reqwest transport with redirect following.
  Omitting that upstream client is not file denial or proof that a host client
  will not fetch. Policy must precede resource construction and handle every
  standalone/table/HTML image route, not only app-level click callbacks.
- Image paste materializes folders/copies/clipboard bytes using filesystem calls
  (`editor/events.rs:336–386`) and reads app preferences (`events.rs:21`).
  Merely not wiring canonical saves would leave these writes enabled.
- LaTeX/Mermaid render caches write/read SVG files and create directories
  (`components/latex/mod.rs:85–99,143`; `components/mermaid/mod.rs:153–206,508–523`).
  These are independent renderer persistence, not canonical saves. RaTeX
  standalone/font rendering also discovers system fonts (downloaded
  `ratex-unicode-font-0.1.14/src/lib.rs:1–36`); its manifest license is not a
  redistribution license for every discovered system font.
- Links emit Block requests (`components/block/interactions.rs:1432–1442`), but
  the **editor** handles a prompt then `cx.open_url` (`editor/render.rs:525–548`).
  A confirmation prompt is not the approved non-executing request seam.
  About links also open URLs (`render.rs:17–24`). Update checks use background
  reqwest against upstream manifests (`editor/update.rs:19–32`, `net/update.rs:14–17,212`).
- Visual HTML/PDF export is distinct from Markdown readback:
  `editor/export.rs:46–89` transforms the serialized document, writes files and
  depends on app theme/title/base-dir; `export/pdf.rs:25–76` starts a Tokio
  runtime, writes temporary HTML/profile data and launches headless Chromium.
  It must be excluded, not adopted as editor core or as a webview workaround.
- Config/theme/i18n/workspace/recent-file/window/menu behavior is bound into
  Editor and Block renderers, not just `main.rs`. Examples:
  `editor/render.rs:10–15`, `editor/context_menu.rs:608–609`,
  `editor/status_bar.rs:8–10,119`, `components/block/render.rs:21–22,1683`.
  App settings loaders/writers and window ownership must not be copied into a
  probe to satisfy globals. Host-owned static styling/text can replace those
  UI data contracts; removing persistent/global behavior is extraction work.

These are source-inspected stop risks, not negative-interaction tests. No
rendered image, URL, clipboard image, SVG cache or exported file was executed.
The downloaded source is not a sandbox and GPUI's normal platform/font/clipboard
access also remains a later native obligation.

## GPUI identity, dependencies and notices

Upstream `Cargo.toml:60–80` depends directly on registry **gpui ^0.2** with
`runtime_shaders`; its dev dependency enables `test-support`. Upstream lock
`Cargo.lock:2457–2460` pins:

```text
registry+https://github.com/rust-lang/crates.io-index#gpui@0.2.2
checksum 979b45cfa6ec723b6f42330915a1b3769b930d02b2d505f9697f8ca602bee707
```

Manyhands retains **Kit 0.6.6 / gpui-pre 0.3.6**, registry identity/checksums as
recorded in the reviewed Zorite report. Kit's actual cached normalized manifest
`Cargo.toml:354–356` aliases `gpui-pre` with `=0.3.6`; baseline copies are in
`E/baseline-source/`. These are **different package names and versions**, not
constraints Cargo can unify. Upstream Entity/Context/Window/Action/Element types
cannot simply be passed to Kit. No Manyhands graph was resolved for Velotype;
this is manifest/lock source evidence, not a compiler diagnostic.

A port would have to replace imports/derive/macro crate paths throughout Editor,
Block, renderers, actions and helpers with the Kit-compatible surface, eliminate
the old core graph and validate actual APIs. Changing a manifest alias alone
would not establish that. Concrete input API delta: downloaded gpui 0.2.2
`src/input.rs:10–74` versus cached gpui-pre 0.3.6 `src/input.rs:13–121` adds
platform paste and optional selection/text-length/configuration/editable-range
hooks. Their defaults avoid claiming every addition is a compile error, but
Velotype's custom paste/selection path would require port review and native
proof. Custom Block Element paint/layout, `Window::handle_input`, image/cache,
action derives, prompt/async and scroll interfaces are further named port
contracts, not proven drop-in compatibility. Root-first/Kit initialization and
supported test context must be established separately. No port is authorized.

Upstream is an application: modules are declared by `main.rs:20–34`; no `src/lib.rs`
or standalone editor Cargo target exists. Features chiefly select syntax/HTML
highlighting; save/file/network ownership is not a Cargo feature boundary.
`build.rs:1–11` compiles Windows resources with embed-resource. It was read,
not run, and should not become an editor-library build script.

### Dependency cost, not a trial resolved graph

`E/upstream-lock-inventory.json` contains the original application lock's 818
nodes/edges (including platform/development packages); an extraction closure
must be designed before re-resolution under a new authorization. The following
are concrete source dependency groups, not a promise that unused dependencies
can already be dropped without edits:

| Group | Source contract and consequence |
| --- | --- |
| Base editor | GPUI, unicode-segmentation, UUID, serde/schemars actions, anyhow; inline/tree/table/HTML helpers also use crate theme/GPUI types. Not a GPUI-free domain library. |
| Highlighting/HTML | Default features include tree-sitter core, official/config languages and html-native; upstream lock tree-sitter 0.26.12 plus grammar/native build dependencies. Separate syntax parsing from native renderer/build cost; do not execute scripts during source assessment. |
| Math/diagrams | Manifest RaTeX ^0.1.9 actually locks layout/parser/svg/font/font-loader/katex/unicode packages at 0.1.14; Mermaid ^0.3.0 locks 0.3.1. Caches, font discovery and assets are part of this closure. |
| Application resources | reqwest 0.13.4/default-tls, futures, directories, URL, config/TOML and locale support implement network/settings/workspace behavior. Their removal requires policy seams and code separation. |
| Visual export only | chromiumoxide 0.9.1, Tokio, pulldown-cmark 0.13.4, CSS/HTML/theme exports. Do not retain this browser/process closure for body edit/readback. |
| Native/backend/build | Original gpui 0.2.2 backend/transitive/native scripts are separate from baseline Kit/pre graph. No full transitive build-script or license audit of all 818 nodes is claimed. |

### License/provenance obligations

- Upstream manifest declares **Apache-2.0**; `LICENSE-APACHE` names
  **Copyright 2025–2026 manyougz** and includes the full license. No upstream
  NOTICE/COPYING or separately licensed bundled asset file was found among the
  148 pinned blobs. README's `LICENSE` link does not match the actual filename;
  license evidence is the file, not the badge. Extraction/redistribution must
  retain license/copyright/attribution and applicable notices, identify modified
  files and avoid implying upstream endorsement. Pin/module hashes and a
  reproducible upstream-to-extraction patch would become maintenance records.
- Downloaded gpui 0.2.2 declares Apache-2.0 and carries LICENSE-APACHE.
  Downloaded pulldown-cmark 0.13.4 and mermaid-rs-renderer 0.3.1 declare MIT and
  carry LICENSE texts. All ten archive hashes equal upstream lock checksums;
  `E/verified-packages.json` names the actual notice files.
- Downloaded RaTeX 0.1.14 packages declare MIT but their inspected archives
  do not include a general MIT LICENSE file. That declaration alone is not a
  complete notice bundle for redistribution; obtain authoritative copyright/
  permission notices before retaining this group. **ratex-katex-fonts** contains
  `fonts/FONT_NOTICE.txt` and full `fonts/OFL.txt`: KaTeX TTFs are **SIL OFL 1.1**,
  not software MIT. Embedding/bundling/modified reserved-name/font notice rules
  need separate handling. Dropping optional rich math rendering would still
  require preserving unsupported source and removing its active call paths.
- Remaining transitive licenses/assets/scripts and exact retained feature closure
  are substantive unknowns, not legal clearance. E/ preserves downloaded
  notices/font source; no notice obligations are added to the product by this
  documentation-only source retrieval.

## Bounded work packages and authority split

These are prospective deliverables/contracts, **not implementation approval or
time estimates**. The upstream editor directory alone has **20 Rust modules /
23,492 lines including tests**; the full app has 61 Rust modules. Those counts
show coupled surface size, not effort forecasts or a claim all must be copied.

| Package | Exact module surface / deliverable | Authority and maintenance cost |
| --- | --- | --- |
| P1 source-preservation design | `editor/{document,tree,source_mapping,history,window_state}.rs`; `components/block/{state,runtime/mod,runtime/projection}.rs`; `components/markdown/{inline,table}.rs`. Specify immutable original spans, changed-block invalidation, structural move/table boundaries, CRLF/final-newline and exact export/history behavior. | **Core/fork work**, not adapter. Independent normalization/serialization losses require design review before executable effort. Every future parser/edit/undo change needs byte-golden regressions and upstream reconciliation. |
| P2 extracted editor boundary | Retain/review P1's model/history modules plus `editor/{mod,events,runtime_context,selection,table_edit,render,context_menu,window_state}.rs` and Block `mod/state/input/element/render/interactions/runtime/{mod,projection,table,code,image}.rs`, `components/actions.rs`, Markdown `inline/table/link/footnote/html/image/paste/code_highlight`, module re-export roots `components/mod.rs` and `components/markdown/mod.rs`. LaTeX/Mermaid parsing/render helpers (`components/{latex,mermaid}/mod.rs`) are called directly by importer/Block renderer (`document.rs:20`, `components/block/render.rs:13–20`); retain a host-contained implementation or explicitly remove/delegate their render paths under P3. Strip app action/UI/data ownership. | No reusable library today. Core UI and app concerns cross these modules; new readback/action/event API and tests are extraction/API changes. Do not pretend copying only `document.rs` produces a usable editor. |
| P3 host-only I/O seam | Remove/delegate `editor/{persistence,close,file_drop,export,update,workspace,status_bar}.rs`, render menu/link handling and image paste from `events.rs`; audit `config`, `app_menu`, `net`, `window_chrome`, theme/i18n dependencies. Replace content image construction/cache behavior in Block/Markdown/LaTeX/Mermaid. | **Extraction/policy patch** work. Host interfaces below are proposals, not existing injectable hooks. Denial must cover internal file reads/cache writes as well as save callbacks. Keep/re-audit provenance and policy patch on upstream updates. |
| P4 GPUI port | All retained `use gpui::*`, qualified derives/macros/types; Block input/Element/layout/paint, render/image/async/scroll/action contracts; original framework tests. Produce one Kit/pre identity and Root-first compatible host proof. | **Framework/core port** beyond thin adapters. No GPUI upgrade/direct dependency allowed now; source similarity is insufficient. Size unknown until API inventory and approved compilation. |
| P5 thin host adapters | Once P1–P4 have an accepted seam: immutable canonical header/body, single draft/base/generation, dirty status/readback, non-executing resource/link requests, action/focus/style translation and explicit synthetic capture. | Fits existing allowance **only after** an acceptable editor API exists. Host policy cannot manufacture source fidelity or own an alternate engine. Real save/draft/replay services stay Cycles 08/09. |
| P6 meaningful proof | Independent byte goldens then actual candidate load/readback, focused native edit/undo/redo across modes, table cell/alignment/row/column edits, unsupported/custom/HTML/parse-failure mixed source; containment negatives, stale-generation/dirty seam; native IME/clipboard and measured rendered response. | Future Tasks 4–6 equivalent only with new authorization. Existing upstream tests are starting specifications, not accepted fixtures/native results. Windows/macOS/other architectures, high-DPI/accessibility, integrated performance remain obligations. |

The proposed **host-owned interfaces** would be body-only load/readback with
explicit clean replacement, shared edit/undo/mode actions, typed source-change /
selection / resource-request events, host-provided static style/text configuration,
a deny-by-default resolver returning approved decoded resources, and an in-memory
draft base/generation contract. Canonical save/draft/crash recovery and repository
operations never belong to that extracted UI core. Exposing these interfaces,
removing private app calls and preserving spans are changes to upstream core/API;
no existing supported injection surface was found covering them all.

Substantive unknowns: whether upstream will support a byte-preserving model/API;
full source-span semantics through structural edits; exact retained dependency/
notice closure; scope of GPUI API port beyond input; real test-context access;
composition/selection/clipboard behavior; safe resource/cache cancellation;
rendered p95 and large-source usability. A build alone would not answer these.

## Requirements and comparison

| Requirement | This Task 7 result |
| --- | --- |
| R1 exact pin / one core | **Source-inspected; blocked for unchanged embedding**: immutable archive/tree/manifest/lock identity recorded; gpui 0.2.2 differs from Kit/pre. No extracted/resolved/compiled graph or entity interoperability test. |
| R2 host-only resources/persistence | **Source-inspected; blocked for intact Editor**: direct save/drop/paste/cache/image/URL/update/export paths above, not controllable by merely omitting save callbacks. No native denial tests or sanctioned policy patch. |
| R3 exact no-op | **Blocked by source inspection**: constructor CRLF replacement, generated rich readback, explicit final-newline/Setext normalization expectations. No candidate snapshot or native test. |
| R4 focused edit/shared undo | **Source-inspected; preservation blocked, executable not-tested**: real shared document history/actions exist, but snapshots/reparse use generated source and export rewrites untouched native blocks. Cross-mode/CRLF/unsupported undo proof absent. |
| R5 rich vocabulary/tables | **Source-inspected; not-tested**: native kinds, per-cell editor and structural table methods characterized. No required rich/native edit acceptance; source-only fallback not counted. |
| R6 native usability | **Not-tested**: inspected input/actions/clipboard implementation does not prove Linux Wayland keyboard/real IME/Unicode/bidi, other OS/architecture, accessibility or high-DPI behavior. |
| R7 dirty/external-change | **Source-inspected; not-tested**: upstream dirty/drop/close logic is app file ownership, not a host generation/stale-completion seam. No synthetic newer-base experiment or production recovery proof. |
| R8 responsiveness | **Not-tested**: source windowing is not 100 KiB/100-sample rendered p95, larger-source/scroll usability or integrated 1,000-item evidence. |
| R9 API inventory | **Already complete provisionally**, accepted Task 2 [106-row audit](wave-03-api-audit.md); unchanged/not rerun here. Final completed-Wave-02-main re-audit remains mandatory. |
| R10 recommendation | **Source-inspected stop/adapt-only-with-new-approval** below. No editor chosen; independent report review and user checkpoint required. |

The reviewed **Zorite 0.10.0** result demonstrated a resolved single Kit/pre
identity and small potential host/style/provider seams, but stopped at
unconditional math-fence load normalization. It did not compile or test a native
editor and was rolled back to the original Cargo files. Velotype's pinned path
has independent normalization/semantic reserialization losses, a different core
identity and application-owned I/O; it is therefore a broader adaptation surface
than that Zorite finding, not a drop-in fallback. Neither result establishes
that every revision/use case/editor or the approved product is infeasible.
No performance/native-quality comparison is possible from these source findings.

## Recommendation and next user decision

**Recommend retaining both negative reports and not authorizing executable
Velotype extraction merely to confirm already explicit source blockers.** R1–R4
cannot pass unchanged; executable work would immediately need excluded ports/core
policy/preservation changes. The completed-wave entry gate and full product
fidelity requirements remain unchanged.

The next user checkpoint should choose a bounded direction, separately from
approval of this report:

1. **Smaller next investigation (recommended if continuing editor exploration):**
   authorize source-only assessment of a specifically immutable Zorite revision
   or documented upstream preservation/load API that may avoid the reviewed
   normalization, before any resolution/build. Deliver a diff of load/readback/
   edit/history/resource paths and renewed license/core identity evidence. Stop
   if removing normalization needs a patch or hides other losses. This is
   worthwhile only with a concrete revision/API lead; availability is unknown,
   not a promise another version passes. Cost is source/provenance review plus
   a later independent actual-editor/native proof lane if it passes.
2. **If Velotype is strategically preferred:** explicitly authorize a separate
   **design-only** core/extraction/GPUI-port investigation for P1–P4, limited to
   source/API mapping and preservation/policy contracts at this pin, no executable
   work. Decision deliverables: required retained modules/patch list, feasibility
   of exact source spans across edits/modes/undo, host I/O ownership and Kit port
   inventory. This is worthwhile only if the user is willing to consider
   ongoing fork/port/notice maintenance; it cannot be recast as thin adapters.
   Any subsequent fork/patch/build still needs a separate approval.
3. **Pause, or authorize another bounded candidate source assessment:** retain
   evidence, select no dependency and incur no extraction maintenance now.
   Another candidate must have a named provenance and the same fidelity/core/
   containment gates. No custom editor, webview, GPUI upgrade or source-only
   product fallback follows automatically from either negative result.

Future executable proof, if a path is approved, must begin with independent
original-byte fixtures and counterexamples (CRLF, `alpha\n`, Setext headings,
unsupported/mixed regions and untouched table/list/code neighbors), not a
candidate's own normalized expected strings. Then prove actual editor snapshots
and cross-mode native edit/undo/resource behavior. A host-only no-save check or
pasted-string test cannot retire the preservation blocker.

## Documentation verification and limitations

`E/validate.py` / `E/validation.log` check canonical frontmatter and globally
unique valid ULID, local links/anchors, exact source line/range and key excerpts,
archive/tree/registry checksums, quoted identity hashes and unchanged baseline
Cargo/lock/Devenv files. `git diff --check` passes. `E/audit.log` records the
source/archive inventory validation (exit 0). These are documentation/evidence
assertions, **not candidate tests**; no tests were added. Ten public registry
archives and upstream HTTP requests succeeded; no external download failure was
classified as incompatibility. Outer PATH lacks `python3`; the existing explicit
Nix-store Python executable was used for evidence checks, not Rust execution.
No native environment/display access was exercised and no automated/native CI
result is claimed. The controller owns final Task 8 bookkeeping and independent
review; ticket stays open, no publication/merge/closure/cleanup authorized.
