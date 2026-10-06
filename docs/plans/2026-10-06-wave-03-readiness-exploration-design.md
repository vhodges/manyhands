---
title: "Wave 03 Readiness Exploration Design"
date: 2026-10-06
status: proposed
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M48S808QBNHZGCW88DVDE28F"
---

# Wave 03 Readiness Exploration Design

**Contract:** [Readiness exploration](../research/wave-03-readiness-exploration.md).
**Plan:** [Implementation](2026-10-06-wave-03-readiness-exploration-implementation.md).
**Ticket:** [01M48S808PF2D8ZWVYM918RK2M](../../.manyhands/tickets/01M48S808PF2D8ZWVYM918RK2M/ticket.md).

Proposed for review; execution remains unauthorized. The user selected editor
plus provisional API audit, small host adapters only, and source assessment
followed by a checkpoint before executable Velotype extraction.

## Architecture and containment

Keep the single root Cargo package. After approval, add an optional exact
`zorite-editor = "=0.10.0"` dependency, an opt-in `editor-probe` feature including
`desktop`, one explicitly declared example and one explicitly declared test
target requiring that feature. Do not create a workspace, second package or
production entry point. The headless default library/CLI remain free of GPUI;
no GPUI type enters `src/lib.rs`-reachable domain modules.

Proposed file layout:

```text
Cargo.toml, Cargo.lock                       # opt-in dependency/target only
examples/editor_feasibility.rs              # probe executable
examples/editor_feasibility/host.rs          # Root, focus, controls, trace
examples/editor_feasibility/adapter.rs       # pinned candidate seam
examples/editor_feasibility/session.rs       # pure fixture/draft policy
examples/editor_feasibility/evidence.rs      # synthetic readback and checks
tests/editor_feasibility.rs                  # explicit feature-gated tests
tests/fixtures/editor_feasibility/           # synthetic originals/expectations
docs/research/wave-03-editor-feasibility.md   # resulting evidence/recommendation
docs/research/wave-03-api-audit.md            # resulting operation inventory
```

Do not add these implementation files during planning. `--all-features` will enable the
probe dependency; therefore its resolution/build affects mandated checks even
though production binaries do not use it. Failed exploratory resolution must
not leave an unbuildable manifest/lockfile in a review-ready patch. Preserve the
failed graph/evidence, then restore only this ticket's own dependency edits,
never reset or clean someone else's work.

Initialize transport before any GPUI/worker threads; call `gpui_kit::init(cx)`
inside `app.run`. Allocate Root as the first view for every probe window,
creating its content during Root construction using the supported pinned API.
The existing scaffold creates its child before Root: it is not a copy-paste
proof of the required ordering. Verify the probe's ordering without modifying
production `src/main.rs`; if the pinned API cannot meet the rule, stop and ask.
All host GPUI imports use `gpui_kit::*`. The candidate's transitive core GPUI
must be the same package/version/source as Kit's GPUI-pre 0.3.6.

## Pinning and compatibility gate

Start with the published release referenced by the approved RFC, not an
unversioned upstream branch. Record registry checksum, license and actual
transitive closure. Versioned [manifest](https://docs.rs/crate/zorite-editor/0.10.0/source/Cargo.toml)
and [API](https://docs.rs/zorite-editor/0.10.0/zorite_editor/) are planning sources;
they are not executed evidence. Current host documentation advertises 0.11,
so verify every method against 0.10.0 source and do not mix release APIs.

Resolve dependencies while retaining GPUI Kit 0.6.6/GPUI-pre 0.3.6. Inspect
package IDs and reverse dependencies, not just crate display names or the
absence of unrelated duplicate libraries. Unifying Cargo version constraints
alone does not prove ABI/API/entity interoperability. Compile a real editor
entity in the Kit host before claiming compatibility.

A changed GPUI identity, required upstream patch or unavailable artifact stops
executable work. Network/download failure is environment blockage, not candidate
rejection. Preserve versions, errors and reproduction steps. A different Zorite
release, GPUI upgrade or fork requires a new decision checkpoint.

## Host/candidate seam

The adapter wraps a single EditorState per active synthetic draft. It exposes
fixture loading, exact text readback, rich/source presentation, focus, selection
observation, formatting/table actions and undo/redo. Build controls against
verified versioned APIs rather than promising an imagined method surface.

Versioned source gives concrete starting points:

- `EditorState::set_markdown_style` / `clear_markdown_style` toggle presentation.
  Keep the same entity/history; toggles must not reload source.
- `replace_range` is documented as one undoable byte-range edit. Validate
  UTF-8 boundaries and compare actual readback; never transform an entire
  document to make a focused edit appear lossless.
- `set_text` replaces the whole document and resets caret; use only for initial
  loading or explicit replacement of a clean draft, never a mode switch.
- Table methods include insert/delete columns and alignment operations. Verify
  row operations, selection and no-op behavior before wiring controls.
- `bind_keys`, EditorEvent and actual readback let the host observe candidate
  actions. Do not assume unversioned event fields exist in the pinned release.

Fixture sessions contain original canonical bytes, an exact body slice,
fixture ID, base observation/generation, dirty source and operation trace.
Metadata is immutable host-owned fixture state, not text the rich renderer
may reinterpret. Keep a full-canonical fixture alongside body-only editor
readback; byte comparisons must include the preserved header independently.
A source view for the body is not proof of production full-source repair.

The host has no real repository registration, selected key or save service.
A labeled **Capture evidence** action writes only synthetic source/trace files
under an explicitly selected run directory, normally
`target/editor-feasibility/<run-id>/`. There is no automatic persistence of
source or user data. Mode switching and close do not write a canonical item
or checkpoint. Evidence files are not a draft recovery store.

## Resource and interaction policy

Default image/block providers remain disabled unless a specific synthetic local
fixture is needed. A thin host policy may allow only an explicit fixture-owned
resource allowlist: reject absolute paths, traversal, symlink escapes, network
URLs, executable schemes and private-key locations. Never pass arbitrary editor
strings into an OS opener, filesystem read or remote loader. Link clicks show
a non-executing request in the probe; deliberate external link opening remains
a future production obligation. HTML/scripts remain source, not executable UI.

Audit internal/transitive behavior too: disabled host callbacks do not prove
that a renderer performs no independent network/file access. Record inspected
call paths and native negative-interaction observations separately. Any
uncontrollable access is a stop boundary, not an acceptable probe side effect.

Keyboard-only controls cover mode, focus return, formatting, table edits,
undo/redo and capture. IME composition must be tested through the native input
path, not merely by pasting Unicode. Clipboard tests use synthetic text only.
Rich paste and plain-text paste are separate observations. Do not claim full
screen-reader or high-DPI conformance without native evidence.

## Fidelity and dirty-state experiments

Golden fixtures and explicit edit expectations come before adapter code:

- Required vocabulary: paragraphs/headings, emphasis/strong, links,
  ordered/unordered/task lists, quotes, fenced code and tables.
- Formatting sensitivity: CRLF, mixed indentation, reference links, whitespace,
  empty body, final/no-final newline, Unicode/combining/non-BMP/bidi text.
- Preservation sensitivity: unknown nested YAML, HTML, custom directives,
  unsupported extensions, mixed supported/unsupported regions and malformed
  rich parsing with usable source fallback.
- A reproducible 100 KiB mixed document and a larger document for preservation
  and explicit usable fallback; no silent length cap.

No-op sequence: load -> readback -> rich/source switches -> readback -> evidence
capture. Compare exact bytes; then edit one known region, compare the approved
change and every untouched range, switch modes, undo back to original and redo
back to edited bytes. Drive actual candidate edits, not just session strings.
A parser/host-model test passing while the candidate fails is a failed candidate
case. Structural edits may reformat the changed table/block, with that boundary
stated in expectations; other regions stay byte-identical.

Feed a synthetic external version and stale generation into the host. A dirty
draft remains unchanged with explicit comparison; a clean reload uses a new
base observation only after explicit action. This validates a host seam, not
atomic draft files, polling integration, canonical observations or crash safety.

## API inventory design

The API audit is documentation, not a new adapter implementation. Derive its
checklist from every CLI taxonomy verb and the desktop/runtime RFC sections.
One row per verb/capability records:

1. Requirement and planned consumer Cycle.
2. Existing public symbol and source location, request/outcome types and tests;
   or an explicit missing symbol. Private/test-only helpers are not public APIs.
3. Target/context and expected-observation behavior.
4. Request/operation identity, replay/cache-loss and completed effects.
5. Preflight/confirmation/expiry, progress, cancellation and error/redaction.
6. Selected-key session/host-trust boundary where applicable.
7. Status: present-and-tested, present-but-unverified, partially-present,
   absent, or pending-Wave-02. Name known evidence gaps even for closed Cycles.
8. Owning earlier Cycle, acceptance proof and final-main re-audit condition.

Initial source anchors, **not complete coverage claims**:

| Capability | Baseline anchor | Important limitation |
| --- | --- | --- |
| Canonical item reads/IDs | `canonical::{ItemId::generate,parse_item,serialize_item,ordered_comment_threads}` | Parsing/serialization alone is not repository-targeted observation, repair, replay or byte-preserving no-op UI save. |
| Repository onboarding/remotes | `RepositoryService::{inspect,enable,create_and_enable,list_remotes,add_remote,remove_remote,set_publication_remote,remove_registration}` in `src/repository.rs` | Verify identity, target absence, config persistence and confirmation gaps separately. |
| Local authoring/discovery | `RepositoryService::{prepare_context,save_document,save_ticket,submit_comment,refresh_repository,rebuild_repository,repository_snapshot,recovery_inspection}` | Existing local comment checkpoint is not Wave 02 Cycle 07 publication. Snapshot/open calls need a side-effect audit before being assigned to inspect-only CLI verbs. |
| Credentials/host verification | `repository::keys`, `keys::session::SessionCredentials`, `RepositoryService::verify_ssh_transport` | Inventory exact public key/host methods and session lifetimes; no secret channels invented by this audit. |
| Remote observation | `RepositoryService::observe_publication_remote`, remote state/reservation exports | Advertised refs and policy state are not fetch, sync, materialization or lifecycle completion. |
| Startup | `runtime::initialize_git_transport_before_threads` | Existing initializer does not supply a headless progress/cancellation adapter or a total shutdown deadline. |

Do not open services or execute API calls merely to document them during this
exploration; constructors/inspectors can create cache or registration state.
Read source and existing tests. Any later executable API audit needs its own
bounded authorization/fixtures. Missing frontend observations/replay/results
normally belong to Wave 03 Cycles 01/02; unfinished collaboration behavior
belongs to its approved Wave 02 Cycle. Ambiguous ownership is a reported blocker,
not silently assigned to a later consumer.

## Measurement and evidence model

Separate pure fixture/model assertions, actual candidate readback tests, and
native input/rendering journeys. Prefer a supported GPUI test context exposed
through Kit for candidate tests, but do not enable a second direct GPUI graph
or copy an upstream test harness requiring an excluded dependency. If that
context is unavailable, use the native probe's candidate-produced snapshots
and reproducible action traces; label those as native tests, not headless tests.
Missing actual-editor evidence leaves fidelity unverified.

For performance, record a warm-up, at least 100 synthetic/native local input
samples, measurement start/end (input dispatch to corresponding displayed update),
reference hardware/display and p95 calculation. Event callback time alone is
not rendered response. If a trustworthy display timing seam is unavailable,
report handler timing as limited characterization and retain the approved
<100 ms rendered-input target as unverified. Large-document opening/scrolling
is recorded separately. The 1,000-item integrated app fixture and cancellation
feedback are downstream obligations; this probe has no repository navigation
or blocking transport and must not pretend to measure them.

Record tested commit plus dirty patch/tree identity where relevant, candidate
checksum/lockfile, fixture hashes and action IDs. Reports contain synthetic
content and redacted fixed-category errors only. Native absence, CI unavailable,
failed checks and source-only results remain visible.

## Review matrix and decision record

| Source/concern | Classification | Resolution or proof obligation |
| --- | --- | --- |
| User: scope and adapter allowance | Settled | Editor plus audit; thin adapters; no forks/upgrades; Velotype checkpoint before execution. |
| Wave entry gate | Settled | Exploration neither starts Cycle 01 nor changes order; final audit after completed Wave 02. |
| Published versus current Zorite documentation | Ruling | Start exact 0.10.0/versioned source; another release needs checkpoint. Cost if wrong: one blocked candidate, not silent dependency drift. |
| Single package versus isolated executable | Ruling | Feature-gated example/test within root package. Cost if wrong: remove ticket-owned probe wiring after preserving evidence; no production behavior added. |
| GPUI identity and Root ordering | Decision gate | Verify package graph and actual supported Root construction; stop rather than weakening AGENTS.md. |
| CRLF/source/undo fidelity | Evidence gap | Actual candidate readback and native edit trace; failure is blocker, not host normalization. |
| Resource access and licensing | Evidence gap | Audit actual closure/call paths and negative interactions; require controllable providers/notices. |
| Draft recovery versus toy host model | Settled boundary | In-memory seam only; production crash/store/save proof remains Cycles 08/09. |
| Test context and rendering measurements | Ruling | Use supported Kit seam or clearly labeled native evidence. Cost if unavailable: report unverified, do not fabricate headless/render metrics. |
| Native CI/platform feasibility | Evidence gap | Current workflow is manual-only and prior native failures were deferred; no automatic green CI promise or full-matrix claim. |
| Probe retention/selection | Decision gate | Recommend retention/removal and editor adoption separately; no cleanup, closure or selection by report alone. |

No further product-policy question is presently needed: the answered boundaries
allow a safe proposed plan. Material findings during execution return to the
user before excluded changes, dependency selection or scope expansion.
