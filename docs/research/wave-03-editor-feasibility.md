---
title: "Wave 03 Editor Feasibility — Zorite Dependency Gate"
date: 2026-10-06
status: blocked
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M48Y44BK0XK7XJXSEYT78ZD8"
---

# Wave 03 Editor Feasibility — Task 3

## Disposition and scope

**Historical Task 3 stop findings are preserved below.** The separately
approved spike-only byte tolerance now permits the isolated compiled native
host; see the [2026-10-06 S2 running-spike appendix](#2026-10-06-s2-running-spike-appendix).
This is not an editor selection or production preservation ruling.
For the later user-requested host update/release comparison, see the
[2026-10-07 scroll-performance appendix](#2026-10-07-edit-driven-host-updates-and-release-comparison).

**STOP the Zorite executable lane.** The exact published 0.10.0 dependency
resolved with the required single GPUI identity, but its documented load paths
unconditionally rewrite some source before any edit. This is a **source-inspected
preservation incompatibility** with the approved no-op/unsupported-source
contract, not a native test result or a claim that every editor use is infeasible.
The controller confirmed the preservation stop boundary and report-only rollback.
No adoption or production selection is made.

Only Task 3 ran. No fixture, test target, example target, host, production import,
core patch, fork, vendor integration, alternate candidate or GPUI upgrade was
introduced. No upstream example, editor build, native input, domain API, real
remote/key or content-resource loader was executed. Registry download was the
only candidate network activity. The six-command accepted baseline was not
rerun; no fetch/rebase occurred in this task.

[Contract](wave-03-readiness-exploration.md) |
[Design](../plans/2026-10-06-wave-03-readiness-exploration-design.md) |
[Plan](../plans/2026-10-06-wave-03-readiness-exploration-implementation.md) |
[Ledger](../plans/2026-10-06-wave-03-readiness-exploration-execution.md).
The controller owns ticket/ledger/comments and later fallback assignment.

## Reproduction identity and evidence

- Task base: `9060ac13ad479b656bce82603a054e7c858fa9e2`.
- Branch: `manyhands/ticket/01M48S808PF2D8ZWVYM918RK2M`.
- Cwd: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M48S808PF2D8ZWVYM918RK2M`.
- Accepted baseline: `a416d83`; accepted API inventory: `0906c3e`.
- All evidence below is under ignored
  [`target/readiness-evidence/editor-dependency/`](../../target/readiness-evidence/editor-dependency/)
  in that cwd (abbreviated **E/**). These artifacts are local, not Git-tracked;
  preserve them for independent review. This document is the sole committed
  project change. `E/task3.diff` contains the exact task-base..task-commit diff;
  `E/task-commit.txt` records its SHA after commit.

| SHA-256 identity | Starting/restored | Trial resolved |
| --- | --- | --- |
| Cargo.toml | `26daedc86bf5cbeb7f7a720ffa793f107881081fd5498f8a9b5d7aafb79ff613` | `b46f4920106db312da9008efa6ffaa6a1c142357b3ba99bcf7826da4d265eeb5` |
| Cargo.lock | `ac2e1977c8311a607b15a4329008ffd1fba9245ac56d37f1c0b4d3c2cef7f26c` | `51c46d1c4abb87e2ab011d8e1f4f6a58d709f8c099ab412dee8afa1201a8b5fb` |
| devenv.lock (unchanged) | `770a1b63f55bed5ad23a3e1d922a7c95c473084ffa483be71ebf03f777dc64a7` | same |

`E/original-{Cargo.toml,Cargo.lock}`, `E/resolved-{Cargo.toml,Cargo.lock}`,
`E/{original,resolved,restored}-hashes.txt` and `E/resolution.diff` preserve the
trial independently of rollback. Restore copied only the two saved original
files, and `cmp` passed for both. No reset/clean or other-file recovery occurred.

### Exact Cargo commands

All exited **0**, using Devenv. No compiler or upstream build script was run.
Each `.meta` contains exact command, exit, elapsed seconds, task base and the
resolved lock SHA above. Each has full separate stdout/stderr, not an excerpt.

| Exact command | Seconds | Artifact stem |
| --- | --- | --- |
| `devenv shell -- cargo metadata --format-version 1 --features editor-probe` | 4 | `E/01-metadata` |
| `devenv shell -- cargo tree --locked --features editor-probe -i gpui-pre@0.3.6` | 1 | `E/02-inverse` |
| `devenv shell -- cargo tree --locked --features editor-probe --duplicates` | 0 | `E/03-duplicates` |
| `devenv shell -- cargo metadata --format-version 1 --features desktop --locked` | 0 | `E/04-desktop-metadata` |

Metadata stdout suffix is `.stdout.json`, tree stdout is `.stdout.log`;
all stderr suffixes are `.stderr.log`. Devenv prints two greeting lines before
metadata JSON: the parser selects the JSON object beginning `{"packages":`,
retains the raw stdout, and writes `E/metadata.parsed.json`. The extra locked
desktop metadata command measures **feature/graph delta**, not a rerun of the
six-command compiled baseline. Cargo metadata is unfiltered across platforms
and includes root development dependencies; this is not a Linux build closure.

`E/audit.py` compares TOML lock entries, parsed resolved nodes/features/core
edges, archive hashes and build targets. Its exact invocation was:

```text
/nix/store/0r6k8xa2kgqyp3r4v2w7yrb80ma2iawm-python3-3.13.12/bin/python3 target/readiness-evidence/editor-dependency/audit.py
```

The first audit exited 1 due to an incorrect local cache-directory calculation
(`E/05-audit.{meta,stdout.json,stderr.log}`, saved `E/audit-initial.py`). Correcting
that evidence-helper path produced exit **0** in 2 seconds
(`E/06-audit.{meta,stdout.json,stderr.log}`). An earlier attempted `python3`
launcher was unavailable on the outer PATH (exit 127); no Cargo command ran in
that attempt. These were tooling failures, not source or dependency failures.
No registry/environment failure blocked the successful four Cargo commands.

## Exact pin, dependency graph and lock delta

The trial manifest added only:

```toml
editor-probe = ["desktop", "dep:zorite-editor"]
# under [dependencies]
zorite-editor = { version = "=0.10.0", optional = true }
```

Default features remained empty; there was one root package, no direct `gpui`,
no missing example/test declaration, and no library/headless boundary change.
These two manifest additions and the generated lock delta are **removed** in
the committed result.

The shared core package ID is exactly:

```text
registry+https://github.com/rust-lang/crates.io-index#gpui-pre@0.3.6
```

Parsed metadata has direct normal dependency edges named `gpui` from both
`gpui-kit@0.6.6` and `zorite-editor@0.10.0` to that same ID (also from
`gpui-bidi@0.1.1`). There is no package named `gpui` and no second `gpui-pre`
identity. Kit's constraint is `=0.3.6`; the editor's published normalized
manifest aliases `gpui-pre` with constraint `0.3`. The source/checksum of
Kit/core remain the baseline registry source and respectively
`42732dc20f50926278b2abf7a99d427a7741ab28bca6bfb49685f9ce3a53a1c7` /
`a0437c0b83e636a92bd1a39fa1d05fb632ae671289537497b35871ffbe231b84`.
`E/02-inverse.stdout.log` shows the Kit/base/component/assets/platform/Linux/wgpu
and editor/bidi paths sharing this core. **Resolution is not compiled type,
entity, ABI, fidelity or native compatibility proof.**

Lock entries and resolved nodes increased **993 -> 996**. No baseline package
version/source/checksum/dependency list changed or disappeared; only the root
`manyhands` dependency list gained `zorite-editor`. The lock diff is 29 inserted
lines (three package blocks plus the root edge); manifest diff is two lines.
New names were absent at baseline, so they introduce no new same-name duplicate.
The duplicates log retains unrelated inherited duplicates; their presence is
not a graph failure. `E/graph-audit.json` contains exact machine-checked deltas.

All new packages use `registry+https://github.com/rust-lang/crates.io-index`:

| Package | Archive SHA-256 / lock checksum | License | Activated features | Build script |
| --- | --- | --- | --- | --- |
| zorite-editor 0.10.0 | `1c9976727e00bcbb76a2cba95aeb3d79812a4a34c0bd02c0787257a9eecbddec` | MIT | none | none |
| zorite-markdown 0.9.0 | `13e5fc0166008b2b8ae11140a908aabf97aab27aca69c565c0405f799d1e4fb7` | MIT | none | none |
| gpui-bidi 0.1.1 | `12ca465817de1670c6c1a830c0f4a0a665434e4d02c30a7ffc6362e0194f9cc9` | MIT | none | none |

Editor runtime dependencies are GPUI-pre, gpui-bidi, existing
unicode-segmentation 1.13.3 and syntax-only zorite-markdown. Bidi uses the same
core and existing unicode-bidi 0.3.18 with `default-features=false`,
`hardcoded-data`; its resulting features are already activated at baseline.
Zorite-markdown is requested with `default-features=false`; its `view`, optional
`markdown` parser, reader GPUI edge and reader resource/link execution paths are
**not enabled**. The editor's gpui-pre-platform demo dependency is dev-only and
not pulled in as dependency-of-dependency. Published manifests, rather than
workspace-only `Cargo.toml.orig`, govern this graph.

The only existing-node feature delta is root `manyhands` adding `editor-probe`
to `[default, desktop]`. No baseline dependency feature changed. No new package
has declared features activated. `E/closure-inventory.json` enumerates all 996
resolved nodes with IDs/sources/checksums/licenses/features/notices/build-script
paths and hashes/dependency edges; it identifies the three new nodes. The
135 inherited build-script targets are inventoried, **not a fresh whole-baseline
script audit or executed scripts**. No new script targets or build dependencies
are introduced. All three new normalized manifests explicitly say `build=false`,
and their source packages contain no build script.

## Provenance, licenses and read-accessible source

`E/{gpui-bidi-0.1.1,zorite-editor-0.10.0,zorite-markdown-0.9.0}.crate` preserve
published downloaded archives. SHA-256 of each archive equals its lock checksum.
`E/published-source/<package-version>/` preserves corresponding source,
`Cargo.toml`, `Cargo.toml.orig`, `.cargo_vcs_info.json`, API.md, README and LICENSE
for the read-only reviewer. These are ignored inspection evidence, not vendored
build inputs. `E/source-evidence-hashes.txt` authenticates the copies; archive
comparison/rollback checks are in `E/07-validation.stdout.log`.

The three LICENSE texts were read: all are the MIT license, copyright
**2026 Will Lehnertz**. Redistribution must retain copyright and permission
notices; preserve warranty disclaimer. No separate NOTICE/COPYING file or
additional bundled asset license was found in these published packages. A
future distribution would still need existing GPUI/Kit Apache-2.0 notices and
all inherited transitive obligations (full expressions/notice paths in the
inventory, including alternatives and Unicode licenses). This bounded audit
is not whole-product legal approval. There is no new license/build-script stop
finding; the stop is preservation behavior below.

## Pinned-source preservation blocker (R3)

References below are relative to `E/published-source/`; original download paths
are `/home/vhodges/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/<package>/`.
Versioned web references:
[editor source](https://docs.rs/crate/zorite-editor/0.10.0/source/src/lib.rs),
[markdown syntax](https://docs.rs/crate/zorite-markdown/0.9.0/source/src/syntax.rs).
No unversioned 0.11 API claims are used.

1. `zorite-editor-0.10.0/src/lib.rs:1103–1107` (`with_text`) and
   `1151–1167` (`set_text`) assign `normalize_loaded` output to the buffer.
2. `lib.rs:318–332` explicitly canonicalizes loaded content containing `$$`,
   calling `zorite_markdown::syntax::normalize_math_fences`. There is no style
   check, provider switch or host opt-out: raw mode loads use this too.
3. `zorite-markdown-0.9.0/src/syntax.rs:951–1013` splits a words-mixed pair,
   trims adjacent whitespace and returns owned source joined by newlines.
   The bundled test at `syntax.rs:1041–1043` states this exact transformation:

   ```text
   input bytes (no final newline): "What if words $$E=mc^2$$ more"
   output bytes:                  "What if words\n$$E=mc^2$$\nmore"
   ```

This is a synthetic **source-derived counterexample**, not an executed fixture
or candidate snapshot. `text()` returns that modified buffer (`lib.rs:1117`),
and `set_text` clears undo/redo (`1161–1164`), so these load paths do not retain
the exact original for no-op readback/undo. The required exact-byte preservation
includes unsupported extensions/mixed regions; source-only rendering and no
host save do not excuse changing the buffer before editing. The original source
was never passed to a running editor here and no production source was lost.

This establishes incompatibility of the pinned load paths with the approved
contract, not a compile failure or a general product-feasibility verdict. No
normalization reconstruction, special load bypass, candidate change or core
patch was implemented/evaluated. Controller direction is to stop rather than
work around this finding and to assess fallback source only after review.

## Versioned API / probable adapter seams (source-inspected only)

| Concern | Pinned source and finding | Remaining limit |
| --- | --- | --- |
| Entity, readback, focus | editor `lib.rs:1005` new; `1117` text; `1878` value; `1884` focus; `4945–4953` Focusable/EventEmitter/Render | Actual Kit entity interoperability/Root-first construction not compiled. |
| Presentation | `1199–1218` set_markdown_style / clear_markdown_style only mutate style and notify | Same-entity style toggle appears thin; actual readback/history untested. SyntaxStyle is re-exported at `50`; fields at markdown_syntax.rs:27–86 require host palette/font, optional icon/callback fields can stay None. |
| Whole load | `1103`, `1151` with_text/set_text normalize; set_text resets selection and history, increments generation | Proven source stop above; never use set_text for mode switches. |
| Structural edit | `1125–1148` replace_range records undo, clamps/snaps invalid byte boundaries, clears marked range, moves caret, notifies | Host must reject stale/invalid UTF-8 ranges rather than rely on silent snapping. No Changed emit here: explicit action/readback needed. |
| Undo / format | `2674–2705` private undo/redo restore snapshots and notify; `2433–2453` private format handlers; public Undo/Redo/Bold/Italic/etc action declarations `61–96`, bind_keys `101`, Render action listeners `4988–5000` | Route real focused GPUI actions, not invented public undo methods. Undo/redo do not emit Changed; dirty tracking cannot depend solely on that event. Native action delivery untested. |
| Selection / events | `447–498` Changed, SelectionChanged, OpenLink, OpenWikiLink, EditMath, MathMenu, EditProperties, PreviewImage; `1652–1679` cursor/set_cursor; EntityInputHandler `4803` | No public selected_range getter found; native selection seam needs actual input/context evidence. No invented unversioned payloads. |
| Tables | tables.rs:437/456 alignment; 526/555 row insert/delete; 897/915 column insert/delete | Alignment reads painted table_rows (header getter only), so context/frame matters. Delete row excludes header/separator; last-column deletion no-ops. Columns rewrite entire table block with `| … |` / LF (931–968); independent changed-block expectations required. Cell editing is native input. |
| Clipboard / IME | lib.rs:2540–2588 paste; 2606 copy_plain; 2618–2625 optional clipboard writer; 4803–4898 EntityInputHandler | File/image paste propagates instead of loading files. Text paste normalizes CRLF/bare CR and rich math; table paste flattens newlines/escapes pipes. IME/rich-vs-plain/native clipboard behavior untested. |

These seams characterize probable adapter cost; they are not a host implementation
or permission to repair core fidelity behavior.

## Resource and internal-I/O audit (R2 partial source characterization)

- `EditorState::new` defaults image/chip/embed/mermaid/math/highlight/suggestion
  providers and clipboard writer to None (`lib.rs:1013,1049–1060`). Provider
  setters at `1228–1329` are optional host closures. Image providers return
  already-decoded `Arc<RenderImage>`, not URLs for an internal downloader.
- `element.rs:2600–2669,2692,3228–3245,3744–3765` call the host image provider
  if present and paint decoded images (`1934`). No arbitrary path/URL loader was
  found in editor source, bidi source or enabled syntax-only markdown source.
  No direct filesystem save/read, subprocess or network calls were found in
  those enabled new crates. This is inspected call-path evidence, not a runtime
  denial proof or an audit of every inherited GPUI/backend path.
- Clicks emit OpenLink/OpenWikiLink/PreviewImage requests (`lib.rs:3037–3102`),
  not OS opens. A future host must record/reject these, not bind an opener.
  SyntaxStyle alert/property icons resolve host-supplied AssetSource paths
  (`markdown_syntax.rs:47–50,83–84`, `element.rs:1716,4176`); leave them None
  along with all resource/block-label/ref-count providers for deny-by-default.
- There **are** direct OS clipboard calls and character-palette UI
  (`lib.rs:2540,2614,2623,2665–2669`); host callbacks do not disable those.
  No persistence callback is required; synthetic-only native clipboard testing
  and host capture-directory controls remain later obligations.
- The downloaded markdown **reader** includes automatic `cx.open_url`
  (`view.rs:1917,3016`), but `lib.rs:26–29` gates that module on `view`, which is
  absent in this graph. Do not enable reader/default features as a containment
  shortcut. Default GPUI OS/font/display/clipboard and inherited platform code
  are not sandboxed by an absent editor provider.

No uncontrolled rendered-content I/O was proven in the enabled new closure;
R2 is **not accepted** without native negative interactions and host policy.
No upstream demo was executed (nor used as containment proof).

## Report-only validation

The exact evidence validation command (exit **0**, 0 seconds) was:

```text
/nix/store/0r6k8xa2kgqyp3r4v2w7yrb80ma2iawm-python3-3.13.12/bin/python3 target/readiness-evidence/editor-dependency/validate.py
```

`E/07-validation.{meta,stdout.log,stderr.log}` records byte-exact rollback against
saved files **and task-base Git contents**, all 33 published source files matching
the three checksum-verified archives, canonical managed frontmatter/unique ULID,
local links/whitespace and existence of the quoted load paths/counterexample.
These Python assertions inspect evidence; they do not execute candidate Rust
or replace real editor tests. `git diff --check` passed; no tests were added.
Broad Rust checks/builds/native smoke were intentionally not run for this
graph-only task; after rollback the production Rust tree/lock are unchanged from
the already accepted baseline. Independent review is still required.

## Requirements and downstream obligations

| Requirement | Task 3 status |
| --- | --- |
| R1 exact pin/single core | **Partial, graph-only**: checksum/package ID/closure pass; actual Kit-host compilation and type interoperability not-tested. Trial dependency rolled back. |
| R2 resource/persistence containment | Source-inspected only; native/host negative checks **not-tested**. |
| R3 no-op bytes | **Blocked by pinned-source inspection** above; no actual-editor/native readback test run. |
| R4 edit/shared undo | **Not-tested**; inspected undo/edit/mode APIs are not fidelity proof. |
| R5 vocabulary/tables | **Not-tested**; public table operations characterized only. |
| R6 native usability | **Not-tested**: Linux/Wayland focus, keyboard, real IME, Unicode/bidi and both clipboard modes remain obligations; no Windows/macOS/architecture matrix result. |
| R7 dirty/external-change seam | **Not-tested**: no session/host/stale-event experiment. |
| R8 responsiveness | **Not-tested**: no 100 KiB input samples, rendered p95, opening/scrolling or larger-case measurement. |
| R9 API inventory | Owned by accepted Task 2, not repeated or strengthened here; completed-Wave-02-main re-audit remains mandatory. |
| R10 recommendation | This is a bounded **stop** finding, pending independent review and Task 7 source-only fallback recommendation, not adoption or final integration approval. |

Tasks 4–6 are not started for this stopped candidate. No executable Velotype
extraction is authorized; controller will assign the approved source-only Task 7
after review. Production Cycles 08/09 still owe crash-safe drafts, canonical
metadata/body separation, stale-save handling, no-commit no-op saves and accepted
editor approach. Cycle 12 owns transport cancellation characterization; Cycle 13
owns the full native matrix/integrated journeys (including 1,000-item app data).
The exploration does not start Wave 03 or weaken its completed-Wave-02 entry gate.

## 2026-10-06 S2 running-spike appendix

Authority: [running-spike amendment](../plans/2026-10-06-wave-03-readiness-running-editor-spike.md).
This appendix adds **compiled actual entity/API seam evidence**, not observed
native usability, editor adoption, or R3/R4 production acceptance. The earlier
pinned-source normalization finding remains valid. No source findings above
were replaced with candidate-generated goldens. No S1 contract was changed.

### Compiled host and reproducible demonstration

From this ticket worktree's repository root, after independent S2 review:

```sh
devenv shell -- cargo run --locked --features editor-probe --example editor_feasibility
# Explicit opt-in: capture all actual initial editor readbacks on launch:
devenv shell -- cargo run --locked --features editor-probe --example editor_feasibility -- --capture-initial
```

The fixed selector reads actual README, CLI-contract RFC, Wave03 dogfooding
and readiness implementation-plan files at launch, plus the independently
specified mixed-math counterexample. Only their bodies enter the editor.
The host retains one real `Entity<EditorState>`, S1 `Draft`, rich/source state
and `ScrollHandle` per document. Selector changes never recreate or reload
entities. `set_markdown_style` / `clear_markdown_style` switch presentation
on that same entity; no `set_text`, buffer replacement or history reset occurs.
The actual entity's Render implementation is the editor (there is no separate
public `Editor` wrapper in 0.10.0); it is rendered directly as a Kit child view.

Controls: sidebar selects a document; Rich/Source and Focus return editor
focus; Undo/Redo and Bold/Italic/Code dispatch actual pinned GPUI actions;
Row+/Col+/Row−/Col− invoke actual public table methods at the caret (may no-op
outside a table). Native editor context menus remain candidate-owned.
Ctrl-Alt-N cycles documents, M toggles mode, F returns focus, S captures all
scratch drafts. Editor Ctrl-Z/Ctrl-Shift-Z and Ctrl-B/I/E use pinned bindings.
Buttons are tab stops. Native delivery/focus/scroll behavior is **not yet observed**.
Source mode is the fallback for constructs lacking rich providers.

Transport initialization is first; Kit initialization is inside app.run.
The outer `cx.new` reserves Root's entity before its closure allocates Host
and editor children, using the supported pinned Context/App APIs. Source
anchor: GPUI-pre `app.rs:2967–2980` reserves before `build_entity`.
This intentionally does not copy production main.rs's child-first ordering.
The exact Zorite 0.10.0 / Kit 0.6.6 / GPUI-pre 0.3.6 seam now **checks and
builds**. Cargo.lock is unchanged from accepted S1, SHA-256
`51c46d1c4abb87e2ab011d8e1f4f6a58d709f8c099ab412dee8afa1201a8b5fb`.
No dependency, graph, feature-closure, core or production-source change.

### Readback, scratch policy and resources

Initial `EditorState::text` readback establishes S1's load observation;
entity notifications (not just Changed events), explicit mode/actions and
capture read actual text again. Undo/redo notify without Changed;
Bold/Italic/Code edits emit Changed and notify. Visible status separates
changed-on-load, later byte edits versus first readback, and conservative
byte-dirty. Undo to normalized initial bytes still cannot make a normalized
draft clean relative to the immutable original. No reload/replacement surface
or scratch-discard workaround is offered. This is not production dirty/store
or external-change integration.

Subscriptions capture stable document ID/generation and verify entity ID;
missing documents, wrong entities and stale generations are ignored. Owned
subscriptions drop with Host; Context callbacks use weak Host ownership.
Three new pure tests cover wrong-document/stale routing, denied-request
categories/redaction and actual fixed catalog/header separation. These do
not construct an editor, authenticate native provenance, or exercise undo.
Kit's test context requires the currently disabled `test-support` feature;
no feature/dependency expansion was made. Actual load/edit/undo snapshots
and native input tests remain S3 obligations.

Capture all drafts creates a fresh S1 `CaptureRun` under ignored
`target/editor-feasibility/native-<pid>-<nanoseconds>/`. Default launch writes
no snapshots. The explicit opt-in captures actual initial readbacks, not
host/pure-helper output. Manifests record originals/header/candidate hashes,
normalization/edit/dirty state, actual readback provenance, startup checkout
HEAD/lock and bounded action/event counts. Dispatch requests are labeled
requests, not performed edits; captured bytes are independent actual readback.
HEAD/lock are runtime checkout provenance, explicitly not build attestation.
Host-recombined header proof is **not editor metadata round-trip**. Captures
never write repo docs, and failed partial captures remain inspectable.
Capture path guards are local single-writer safeguards, not an OS sandbox.

Providers stay absent: images, chips, embeds, mermaid, math, syntax highlighting,
spelling suggestions, clipboard writer, label/ref-count resolvers and icons.
OpenLink/OpenWikiLink/PreviewImage events log only denied request categories
and counts, not URLs/content and not performed opens. Math/property structural
requests are labeled unsupported, not handled by an alternate editing model.
No URL/OS opener or arbitrary path CLI exists. Reused pinned internal-I/O
inspection above remains relevant: enabled editor/bidi/syntax code has no
independent content file/network/command loader; absent image callbacks cannot
resolve arbitrary strings. GPUI assets/fonts/display and ordinary intentional
clipboard input remain inherited OS access, **not sandboxed**. Native resource
negative tests and all-platform containment are still unverified.

### Named integration/adoption costs and exact API gaps

Physical LOC (including tests/comments/blank lines): entry **63**, host **383**,
adapter **126**, catalog **66**; total S2 Rust **638**. Separately, reused S1
probe-only bookkeeping is session **176** + evidence **230** = **406** LOC,
plus its existing tests/fixtures. These counts are code size, not a time or
production adoption estimate. Adapter tests are 35 of its 126 lines; catalog
tests are 19 of its 66 lines. No fork/upstream patch is required to compile this
spike; faithful product acceptance still has the known upstream/core gap.

- **Theme/host:** 0.10.0 `SyntaxStyle` requires explicit colors, mono font and
  optional fields; no Kit theme adapter is provided. This spike uses a fixed
  dark palette with absent icons. Production theme/font/accessibility/high-DPI
  integration is untested, not solved by compilation.
- **History/selection:** undo/redo/format handlers are private; route public
  actions into focused rendered entities. `cursor()` exists, but a public
  full selected-range getter was not found. Native selection/cross-mode undo
  fidelity remains to be observed. Core caps full-snapshot history at 256;
  retaining five independent histories can multiply document memory.
- **Readback/subscriptions:** Changed alone misses undo/programmatic operations;
  notify observers plus explicit/deferred samples are needed. This simple host
  clones full text on notifications, including some selection changes; S1
  retains full original/load/current Strings. No event-to-render timings or
  efficient incremental production draft bridge have been established.
- **Tables/scroll/focus:** pinned public row/column operations may rewrite the
  table block and no-op without context; alignment depends on painted table
  rows. No independent row/table normalization or native table acceptance is
  claimed. Host owns per-doc scrolling and focus return; caret auto-reveal,
  async scroll compensation, mode-relative viewport and platform input need
  native journeys. The available scroll-compensator hook is not installed
  because this spike has no async content providers.
- **Resources/metadata:** absent providers bound this demo, but adoption would
  need owned decoding/resolution/cache policy, safe actions, notices, and
  containment tests. Rich math/properties need dedicated structural-editor
  integration; source remains usable without inventing those cores. Immutable
  frontmatter is a host split, not canonical metadata editing/repair/round-trip.
- **Normalization/product:** with_text normalizes mixed math before history;
  the visible synthetic counterexample allows inspection after launch. Actual
  candidate captures must retain original differences. Byte tolerance here
  does not resolve the RFC's exact-source/untouched-block or lossless undo
  requirements; R3 remains source-blocked for production, R4 unverified.
- **Production/native:** crash-safe draft recovery, observation/stale-save,
  no-op save, domain wiring, license packaging, keyboard/IME/clipboard, native
  matrix, 100 KiB/larger behavior and rendered p95 are downstream obligations.
  Nothing here measures responsiveness or selects an editor. API re-audit
  against completed Wave02 main is unchanged.

### S2 verification evidence and boundaries

Task base `41267b9ca3c3c42c8e06e4e7edfbee7c36040b7d`.
Full command/exit/HEAD/tree/lock/duration logs and raw/numbered owned-source
snapshots are in ignored `target/readiness-evidence/running-spike/s2/`, with
`task.diff`, `task-commit.txt`, `review-index.json`, `review-proof.json` and
`artifact-hashes.txt` for shell-less review. The first focused check failed
on two host API mistakes (FocusHandle::focus requires App; Bounds::centered
needs synchronous App, not AsyncApp) and an unused import. Both were fixed
within S2, with diagnostics retained; no core/dependency workaround.

Focused example check/build and its three pure tests passed; nine S1 probe
tests passed. Required all-feature check/fmt/clippy(-D warnings)/tests,
headless CLI check and CLI smoke passed on the S2 Rust tree. Full tests report
**568** standard-harness passes (including 9 doctests and 9 S1 tests), zero
failures/ignored, plus existing isolated SSH harnesses 15/31/66. Example tests
are separately run: Cargo's normal all-feature test command does not execute
these three tests. Committed-tree gate logs are retained separately from initial
working-tree logs. No GUI process was started by S2; no native keyboard, IME,
clipboard, rendering, platform or production-desktop smoke pass is claimed.
Controller owns reviewed S3 persistent launch and production smoke separately.

## S3 native observations and adoption-cost handoff

### What actually ran

Controller launched the reviewed `2c53127` example through Devenv with
`--capture-initial`, owned process group **3247260**, and left it open for the
user. It subsequently exited with code0/no signal at2026-10-06T21:13:37Z;
no exit cause is inferred and no relaunch was performed. Five real EditorState
entities were constructed; startup body text was
read from those entities, not a substituted string model. Evidence is in
ignored `target/readiness-evidence/running-spike/s3/launch-1791317716369/` and
`target/editor-feasibility/native-3247260-1791317722697818257/`.

The user explicitly confirmed **Visible and working** in response to the
controller's question about seeing/exploring the documents. This is reported
basic visibility/usability, not a measured or per-requirement native suite.
The session advertises DISPLAY=:1 and WAYLAND_DISPLAY=wayland-1; backend choice
was not instrumented. No other desktop pixels or clipboard contents were read.

Parent independently compared initial original/candidate-full capture bytes:

| Scratch source | Original/candidate bytes | Initial comparison |
| --- | --- | --- |
| README | 1790 / 1790 | exact |
| CLI-contract RFC | 22992 / 22992 | exact |
| Wave03 dogfooding | 38177 / 38177 | exact |
| implementation plan | 22438 / 22438 | exact |
| labeled mixed-math counterexample | 29 / 29 | **different**, changed-on-load/dirty |

All five manifests declare ActualEditorReadback. Full candidates combine the
protected original header with actual body readback: metadata never traveled
through the editor. Source repo hashes still match launch originals. Equal
length is not preservation: mixed math retains the known normalization failure
and is not relabeled a pass under temporary tolerance. Four initial no-op loads
do not establish untouched-block edits, full-fixture fidelity or semantic safety.

Separately, the unchanged production desktop scaffold was built/launched through
Devenv at `7ab466f` using `cargo run --locked --features desktop --bin manyhands`.
Its binary was observed alive without startup diagnostics and only owned smoke
group **3310636** was stopped with controlled SIGTERM. Logs/observation/exit:
`target/readiness-evidence/running-spike/s3/desktop-smoke-1791321361661/`.
This is a startup smoke, not graceful-close or visual/input acceptance; the
user's editor process was not stopped.

### Concrete integration cost, not an adoption estimate

The experiment required **no fork, vendoring, GPUI upgrade, core rewrite or
production changes**. Pin/feature/test/example registration adds three reviewed
optional packages; Kit/pre type identity works in real entity construction.
S2 totals **638 Rust LOC**, including UI/capture controls/tests: actual adapter
126 (35 test LOC), catalog 66 (19 test LOC), host 383, entry 63. S1's **406 LOC**
of pure session/evidence bookkeeping is experiment infrastructure, not editor
porting. Prototype LOC is evidence of integration surface, not days of product
work or a performance guarantee.

| Adoption area | Learned now | Remaining work / decision |
| --- | --- | --- |
| embedding/theme | Kit-native entity/render/focus/style seam compiles and runs with a small adapter | integrate production layout/focus/theme refresh and platform checks; no core port demonstrated necessary |
| presentation/history | same per-doc entity/draft/mode/scroll owned by host; mode changes use style, not text reload | captured cross-mode edit/undo/redo/untouched-block journeys and caret/selection acceptance; snapshot history does not recover load loss |
| metadata/drafts/save | body-only boundary, normalized-on-load distinction, generations and readback comparisons are practicable | shared headless crash-safe draft store, stale/no-op save, reload/discard/conflict lifecycle and canonical service wiring; this demo saves nothing |
| resources/rich coverage | provider setters allow deny-by-default prototype; source fallback when rich services absent | explicit trusted resource ownership and negative traces; highlighting/math/HTML/image/suggestion/wiki/property services, licensing/packaging as needed; SDK/native hook gaps may need design/upstream work |
| bytes/meaning | four representative initial loads exact; math counterexample actually changes | product fidelity ruling or approved upstream/preservation repair if exact source stays mandatory; semantic/metadata safety and lossless undo still required, no tolerance waiver here |
| responsiveness | largest running repo sample is 38 KiB; basic user exploration reported working | actual 100 KiB/larger rendered p95/input/IME/clipboard/table matrix; whole-body notification readback and 256 full-text undo snapshots are costs to measure, not benchmark results |
| API/version integration | fixed 0.10 public seam is usable despite unversioned documentation drift | version/theme/provider maintenance and provisional API re-audit on completed Wave02 main before Wave03 Cycle01 |

**Conclusion:** the off-the-shelf embedding cost is bounded and much smaller than
an editor extraction/GPUI port. This pin is useful for continuing the experiment,
not selected or shipping-ready. The major adoption obligations are production
state/services, resource/rich contracts, native/performance proof and unresolved
fidelity policy—not getting a component to appear. No time estimate is justified
until those decisions/tests are bounded. Original strict RFC/Cycle requirements,
Wave02 completion/ordered entry gates and Velotype's source-only limitations
remain unchanged.

### Final corrected source verification

Two accepted S2 reporting errors were fixed narrowly in `3ba5ff2..7ab466f`:
568 standard passes, and formatting emits Changed while undo/redo only notify.
Only three report lines and two host comment lines changed; no behavior changed.
Fresh amended-tree Devenv fmt/check/clippy/test and focused example check passed
at tree `400e8dfa80f87db9784c50ce9629ff747c38d373`, mapped exactly to commit
`7ab466ff048f60af9413a058bb2033024934e6e1`; 568 standard passes and SSH 15/31/66,
no failures/ignored. Retained reviewer `d7536642` confirmed both findings resolved,
no issues, correction READY / OK. Old erroneous evidence remains immutable and
superseded, not rewritten. Current code differs from the originally launched reviewed binary
only in that accurate comment; its recorded launch/provenance remains `2c53127`.
Further report/ledger bookkeeping does not invalidate final Rust source gates.

## 2026-10-07 edit-driven host updates and release comparison

Authority: [scroll-performance continuation](../plans/2026-10-07-wave-03-editor-scroll-performance.md).
User liked appearance/behavior, but reported README scrolling quickly and Wave03
lagging, then exited the demo. This is subjective native feedback, not a timing
benchmark or a proven bottleneck. The prior binary was an unoptimized dev build.
Document size, wrapping and rich complexity covary; this change does not assign
all lag to the host or claim wholly unvirtualized editor rendering.

### Rebased source and bounded host change

Task base `e1fc6cb1293f2cd84cf65185686e11f56d5ce195` follows the controller's
fresh rebase onto main `40fc971c29a1722d9b4e5284e1da5401c95b15f1`.
Earlier trees/counts/commit IDs above remain historical (S2 code rebased to
`239a79`, comment correction to `9c88`); they are not verification of new main.
Both upstream remote_synchronization and existing probe target registrations
remain intact. This continuation changes only example host/adapter/entry,
a small pure observation/metrics module and this appendix; it changes no
Cargo/dependency/lock/Devenv/domain/CLI/production source or original/golden.
Exact Zorite 0.10.0 / Kit 0.6.6 / GPUI-pre 0.3.6 remain locked.

Previously every editor notification and subscribed event owned full text,
replaced Draft.current even if unchanged, then requested a Host redraw. The
new borrowed gate compares actual `EditorState::text()` bytes with the current
Draft **before** `to_owned`, Draft.observe or Host.notify. Same-length edits
are detected by byte equality, not length/hash heuristics. Duplicate Changed,
notify and deferred-hook signals for the same bytes cannot repeat the owned
readback, draft replacement or edit-status redraw. Cached DraftStatus is
refreshed only after actual draft changes, not recomputed across full bodies
inside Host.render. Initial load and explicit captures retain actual readbacks.

Changed is the preferred edit signal (typing/deletion/paste/IME/suggestions and
formatting). Supported host edit controls also have deferred hooks. Complete
edit-only coverage is **not available in this pin**: native keyboard/menu undo
and redo notify without Changed. The generic observer is therefore retained
with the same lazy borrowed equality gate. It still does callback/guard work
and, on valid generic notifications, a potentially O(body bytes) comparison.
This is **not zero notification work**, an SDK edit subscription, or a claim
that scrolling can never notify the candidate. No private/core hooks are used.

Selection and other non-edit event hints never call the text getter. Unchanged
scroll/caret/focus/blink/presentation notifications do no owned body readback,
draft replacement or Host edit-status notify. Native vertical wheel handling
at pinned `tables.rs:60–75` exits for dx=0 without notifying; horizontal table
scroll and other notifications can still reach the borrowed fallback. Layout
in `element.rs:114` uses cached shaping; later code shapes/paints a visible band.
No supported per-frame profiler was found. Release profile, larger/wrapping
bodies and rich layout cost remain hypotheses to compare, not assigned causes.

Routing still verifies stable document ID, actual entity ID and generation
before any read. Missing/stale/wrong-entity events are ignored. Valid inactive
documents retain their changes/status without redrawing the active document;
selection later shows cached status on the existing editor/history. Mode and
selector changes never reload/set_text/reset history. Mode/changed selector,
explicit capture and changed denied-request messages may redraw their chrome;
focus and unchanged selection do not request redundant Host redraws. Resource
requests remain denied and logged by category, without body reads or openers.

### Optional aggregate counters and runtime controls

Diagnostics are **off by default**. `--diagnostics` allocates per-document
aggregate integer counters. Press **Print counters** or **Ctrl-Alt-D** for an
explicit stderr summary; the request reads no body, writes no files, does not
notify Host or start a timer. No per-frame/edit diagnostic printing remains.
Counters change without notifying Host on ignored/unchanged callbacks.
Existing bounded action/request trace counts remain separate capture metadata.

Counts include valid notifications, Changed events, non-edit hints, deferred
edit hooks, capture samples, borrowed comparisons, owned body readbacks, actual
draft changes and Host.notify requests. Initial load contributes one owned
readback/one draft change per document. An explicit capture always adds its
owned evidence payload; it updates the draft only if bytes differ. Host notify
counts include actual chrome changes as well as edit-status changes, attributed
to the active document; they exclude candidate/Root/window refresh and are NOT
frame counts, allocator instrumentation, latency or p95. Compare counter deltas
around a scroll-only interval, excluding deliberate mode/select/capture actions.

After independent review, launch the already-built release binary from this
worktree root (controller owns user-facing launch and native observations):

```sh
devenv shell -- env -u ZORITE_WHEEL_DEBUG ./target/release/examples/editor_feasibility --diagnostics
# Optional, explicit initial actual-readback captures:
devenv shell -- env -u ZORITE_WHEEL_DEBUG ./target/release/examples/editor_feasibility --diagnostics --capture-initial
```

Unsetting the upstream wheel-debug variable avoids inherited per-wheel logging;
it is unrelated to our off-by-default aggregate diagnostics. Flags may be used
independently; unknown/duplicate flags and arbitrary paths are rejected.
Ctrl-Alt-N/M/F/S continue next document/mode/focus/capture; native editing keys
and table controls are retained. Use the **same document** in Rich and Source
mode for a user-owned comparison. No worker GUI launch/global input automation,
clipboard inspection, automatic captures or timer/frame logging is authorized.
All builds/tests finish before handoff so comparison does not compete with Cargo.

### Pure regression proof and remaining native obligations

Six new pure observation tests exercise unchanged scroll-like/caret/focus/blink
notifications (zero owned readback/draft-change/notify decision), lazy non-edit
hints (getter never called), same-length edits, duplicate signals in both orders,
notify-only undo/redo and table-hook changes, normalized-on-load dirty state,
inactive-document retention, stale generations and disabled metrics. Adapter
routing tests also reject a wrong entity as well as wrong document/generation.
These are source-policy tests with simulated bodies, **not native hook/event
coverage, actual scroll/undo journeys, allocation measurements or rendered p95**.

New-main focused example tests, required all-feature check/fmt/clippy/test,
headless CLI check/CLI smoke and the exact locked release build have their full
command/status/HEAD/index-tree/source-hash/duration logs under ignored
`target/readiness-evidence/running-spike/scroll-performance/`. That evidence's
`test-counts.json` derives actual new-main counts from complete logs rather than
reusing historical 568. `review-proof.json`, raw/numbered snapshots and exact
`task.diff` map the tested staged source to the local commit; `release-binary.json`
records the optimized binary's hash/size and explicit runtime flags. This is
compiled release readiness for reviewed comparison, not observed smoothness.
Prior evidence remains unchanged. No duplicate postcommit suite is needed for
the exact tested source. The controller still owes native scroll-only counters,
edit/undo/redo/resource checks and user feedback; keyboard/IME/clipboard/platform,
large-document memory/history, semantic fidelity and true rendered response
measurements remain unverified. Byte tolerance remains spike-only: no editor
selection, production fidelity ruling or adoption/maintenance time estimate.
