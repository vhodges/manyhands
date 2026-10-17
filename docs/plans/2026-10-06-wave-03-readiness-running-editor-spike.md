---
title: "Wave 03 Readiness: Running Editor Spike Amendment"
date: 2026-10-06
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M49A8TN9VE8CFJNG38HNN4BG"
---

# Running editor spike: temporary byte-change tolerance

[Contract](../research/wave-03-readiness-exploration.md) |
[Design](2026-10-06-wave-03-readiness-exploration-design.md) |
[Original tasks](2026-10-06-wave-03-readiness-exploration-implementation.md) |
[Ledger](2026-10-06-wave-03-readiness-exploration-execution.md) |
[Ticket](../../.manyhands/tickets/01M48S808PF2D8ZWVYM918RK2M/ticket.md)

## User authorization, not a product ruling

After reviewed source-only findings, the user said: "I might be okay with the
editor changing the bytes (I am NOT ruling that though). For this spike ticket,
assume it's okay, I'd like to learn the integration/adoption cost (and I'd like
to see it running with some of the docs in this repo since they're
representitive)".

For this experiment only, source normalization/serialization differences no
longer veto running the pinned candidate. **This does not amend the RFC's exact
source/untouched-block requirements or select an editor.** Record original and
actual candidate bytes, transformations and implications honestly. Do not change
goldens to candidate-generated strings or report preservation passing because
the experiment tolerates differences. Semantic content/edit loss, protected
metadata changes, unsafe resource access and unapproved core adaptation are
still distinct blockers, not authorized by this assumption.

Resume Zorite **exactly 0.10.0**, Kit 0.6.6 and GPUI-pre 0.3.6 under the original
small-host-adapter boundary. No fork, vendoring, core rewrite, GPUI upgrade,
other candidate version, executable Velotype extraction, production wiring,
publication, merge, closure or cleanup is authorized. The existing
subagent-driven execution method continues, one serialized component owner
per canonical ticket worktree and fresh independent component/final reviews.

## Safe, representative demonstration

Build the original opt-in `editor-probe` feature, explicit
`editor_feasibility` example and feature-gated test target in the single root
package. Domain library, CLI and production desktop remain unchanged and free
of new editor imports. All host GPUI imports use GPUI Kit; initialize Kit in
app.run and allocate Root before its child views. Transport initialization must
precede threads, as in the original design.

The selector should include scratch sessions from these inspected documents:

- `README.md`: short unmanaged Markdown, links and fenced code.
- `docs/RFC/cli-contract.md`: managed document, dense tables and code.
- `docs/Waves/wave-03-dogfooding.md`: managed, longer headings/lists/tables.
- `docs/plans/2026-10-06-wave-03-readiness-exploration-implementation.md`:
  managed plan, task/checklist/command material.

These are actual repo contents, not artificial demos claimed representative.
Read and hash originals, split immutable canonical frontmatter from editable
body where present, and retain independent scratch originals. Never save,
format, commit or publish editor-produced content into source documents.
Keep each document's scratch draft/editor when switching the selector. Rich/
source toggles must reuse the same editor entity and not call set_text/reset
history. Separate changed-on-load from user edits in the visible status; do not
silently treat normalized bytes as original bytes.

No automatic file/network/image/URL opener providers. Leave absent resource
providers deny-by-default and record denied link/image requests. Never read
private files, keys or real clipboard contents programmatically for tests.
Ordinary intentional user clipboard actions in the live editor are not
synthetic clipboard proof. Explicit snapshot capture may write only fresh
files in the run-owned ignored `target/editor-feasibility/<run-id>/` directory,
with original/header/candidate hashes, provenance and event/action status.
Do not add a production draft store or use live domain/SSH/API operations.

## Ordered implementation and evidence

### S1 — Restore reviewed pin and minimal evidence seam

Owner: Cargo optional feature/dependency/explicit test target;
`examples/editor_feasibility/{session,evidence}.rs`,
`tests/editor_feasibility.rs`, `tests/fixtures/editor_feasibility/`.
Do not register the example until its entry file exists.

Intentionally resolve via Devenv metadata, compare to baseline package IDs/
features, inspect and retain the lock delta; preserve the single Kit/core
identity and syntax-only markdown feature closure. Existing MIT/provenance
inspection is reusable when package checksums match. No baseline upgrades.

Keep meaningful independent counterexamples: no-op mismatch, CRLF, mixed math,
Unicode/partial UTF-8 range, immutable header, absent actual candidate evidence,
and dirty/stale-generation replacement. Include deterministic 100 KiB/larger
input with exact size/hash, but do not build a general fixture framework.
Tests must reject false preservation/evidence claims while allowing the spike
to record a non-preserving candidate as an observed result. Tests of pure
helpers are not actual editor/native tests. Run focused feature-gated tests and
commit a bounded handoff for independent review before S2.

### S2 — Actual Kit/editor host and representative catalog

Owner: `examples/editor_feasibility.rs`,
`examples/editor_feasibility/{host,adapter}.rs`, necessary Cargo example wiring,
plus focused adapter tests and a clearly dated report appendix. Consume S1's
reviewed helper contracts rather than rewrite them.

Implement selector, real rich/source toggle, visible scratch/normalization/
dirty state, real editor formatting/undo/table operations where supported,
explicit candidate readback capture and resource request policy. Use actual
pinned APIs, not a second editing model or pasted host strings. Record adapter
size, public API gaps, subscriptions/history/focus/resource costs and failures.
Stop and ask if compilation/containment needs excluded upgrades/core changes.

Compile the example and run the required final Rust checks through Devenv:
check all-features locked, fmt check, clippy all-targets/all-features locked
with -D warnings, test all-features locked; also focused probe tests, headless
CLI check and CLI smoke. Keep complete logs/exits/head/lockfile evidence and
obtain independent code review before the user-facing launch.

### S3 — Launch on this desktop, capture honest observations

Controller owns the user-facing persistent launch after reviewed S2. Use
`devenv shell -- cargo run --locked --features editor-probe --example editor_feasibility`.
An active Wayland/X display is advertised; actual availability is unproved.
Keep the demo open for the user if launch succeeds; identify only our own
process and provide reproducible launch/control instructions. Smoke the
unchanged production scaffold separately and stop only its owned smoke process.

Record actual load/readback/mode/edit/undo/resource observations where performed,
with snapshots/trace rather than claiming model tests are native proof. Invite
user examination of representative documents. Unobserved keyboard/table/IME/
clipboard/high-DPI/accessibility/native-platform behavior stays not-tested.
Responsiveness samples, if collected, must name their timing boundary; no
rendered p95 or full native acceptance follows from startup/handler timing.

### S4 — Adoption-cost recommendation and final review

Append running-spike findings to the editor report with exact graph/build/run
provenance, allowed byte transformations, named host/adoption work, resource/
undo/native limitations and upstream/fork costs. Reconcile the R1–R10 matrix:
production R3/R4 preservation remains blocked or unverified until an explicit
product ruling, even if this experiment runs successfully. API inventory stays
provisional and must be repeated on completed Wave 02 main.

Review the complete amended branch independently. Keep ticket open and provide
the running demonstration plus cost/risk recommendation, not an adoption or
production-readiness claim. Any publication, retention/cleanup or later editor
selection remains a separate decision.

## Decision audit and refreshed preflight

Settled: user authorizes temporary byte tolerance and a running cost experiment,
not a production preservation ruling. Ruling: reuse exact Zorite pin rather than
port Velotype; it already fits the graph and has narrower known adaptations.
Ruling: scratch body-only editing, retained immutable metadata and explicit
capture prevent changes to source docs. Cost if wrong: stop on unsupported
host policy/API, record the missing seam and seek approval before expansion.
Native input/performance/future adoption remain evidence gaps, not assumptions.

Fresh fetch/rebase before this amendment: base
`29f3f5a25957836a8513cba8a318306e1b928063`; before/after
`af380b01291d847c9661ecc1b6f958b3d0f504ec`. No-op rebase, ancestry and clean
worktree verified; local main agrees, AGENTS unchanged. Main's unrelated
`.superpowers/`, `devenv.nix~` and all other worktrees remain untouched.
