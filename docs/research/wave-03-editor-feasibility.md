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
