---
title: "Wave 03 Editor and API Readiness Exploration"
date: 2026-10-06
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M48S808Q62BYSSWGM96D980T"
---

# Wave 03 Editor and API Readiness Exploration

## Purpose and authorization

Retire editor-integration uncertainty and identify front-end API gaps while
Wave 02 completes. This is a standalone readiness exploration, **not Cycle 00**,
and does not change the Wave 03 entry gate or ordered implementation Cycles.

The user authorized ticket creation and planning, and selected these boundaries:
editor plus provisional API audit; small host adapters; Zorite first; assess
Velotype on a blocker but request approval before executable extraction.
The user approved the contract, design and implementation plan on 2026-10-06
at reviewed revision `aa51f96`. On 2026-10-06 the user explicitly authorized
executing this plan using subagent-driven development. That authorization is
limited to the approved exploration; publication, merge, closure, worktree
cleanup and excluded adaptations remain unauthorized.

- [Ticket](../../.manyhands/tickets/01M48S808PF2D8ZWVYM918RK2M/ticket.md).
- [Design](../plans/2026-10-06-wave-03-readiness-exploration-design.md).
- [Implementation plan](../plans/2026-10-06-wave-03-readiness-exploration-implementation.md).
- Branch: `manyhands/ticket/01M48S808PF2D8ZWVYM918RK2M`.
- Worktree: `.manyhands/worktrees/01M48S808PF2D8ZWVYM918RK2M`.

## Running-spike amendment (2026-10-06)

After the reviewed stop findings, the user explicitly authorized assuming byte
changes acceptable **for this spike only**, to learn integration/adoption cost
and see actual repo documents running. This is not a product ruling or an RFC
change. The [running-spike amendment](../plans/2026-10-06-wave-03-readiness-running-editor-spike.md)
governs resumed executable work where it differs from the original preservation
stop: use the same pinned Zorite, record actual transformations, and edit scratch
copies only. All core/GPUI/resource/production/lifecycle exclusions remain.
Original exact-preservation failures remain evidence, not passing acceptance.

## Governing requirements

[Wave 03](../Waves/wave-03-dogfooding.md#entry-gate-and-readiness-work) requires
an editor investigation before editor-Cycle implementation planning and a
current-main API audit before the first implementation Cycle. Its complete
Wave 02 integration prerequisite remains in force.

The [desktop RFC](../RFC/desktop-information-architecture-and-editor.md) owns
rich/source fidelity, undo, metadata separation, minimum rich vocabulary,
unsupported-source preservation, resources, drafts and native input. The
[CLI RFC](../RFC/cli-contract.md) owns command coverage, result envelopes,
request replay and confirmation. The [runtime RFC](../RFC/application-runtime-and-polling.md)
owns shared progress/cancellation, credential sessions and safe-point stopping.
The [decision register](../RFC/wave-03-rfc-review.md#approved-wave-03-evidence-gate)
owns final native evidence and responsiveness targets. None is weakened here.

## Baseline and dependencies

Freshly fetched main: `29f3f5a25957836a8513cba8a318306e1b928063`.
Ticket before/after in-worktree rebase equals that SHA; ancestry passed.
The lockfile pins GPUI Kit 0.6.6 and GPUI-pre 0.3.6. Wave 02 Cycles 01–04 are
closed at this baseline; Cycles 05–10 remain. Their future behavior is a
contract, not a present API. Another worktree is actively implementing Cycle 05.

Read-only upstream inspection found the published Zorite 0.10.0 manifest uses
GPUI-pre ^0.3, GPUI-bidi ^0.1 and Zorite-markdown ^0.9. Its versioned API
provides EditorState, EditorEvent, style-based rich/source presentation and
table-alignment hooks. Current unversioned host docs advertise 0.11: they
cannot prove the pinned release's behavior. Compilation, fidelity and cost are
unverified. A Wayland display is present in this session; a later session must
recheck availability, and native launch/input success is not yet evidence.

## In scope

1. Pin and characterize published Zorite 0.10.0, its dependency/license closure
   and host API. A different release needs a recorded review checkpoint.
2. After authorization, build one opt-in example inside the existing root
   Cargo package, using GPUI only through GPUI Kit and small host adapters.
3. Evaluate actual editor readback and native interaction against synthetic
   golden fixtures: no-op bytes, focused edits, rich/source mode switching,
   shared undo, minimum rich vocabulary, tables, unsupported syntax, CRLF,
   Unicode/IME, metadata separation and host-controlled resources.
4. Exercise an in-memory dirty-draft/external-change seam without implementing
   the production draft store or save/synchronization lifecycle.
5. Record Linux input/performance evidence and explicit Windows/macOS and
   remaining architecture obligations. Gather other native editor evidence if
   access is available; absence is an open obligation, never a passing check.
6. Inventory every CLI taxonomy verb and desktop/runtime operation, mapping
   actual entry points, observations, replay, confirmation, progress,
   cancellation, credentials and partial outcomes. Assign gaps to their owning
   earlier Cycle before a consumer is planned; do not implement those gaps.
7. Produce a recommendation with costs and blockers. If Zorite is blocked,
   perform a pinned source-only Velotype assessment and seek approval before
   extraction or executable adaptation.

## Explicit exclusions and stop boundaries

- No early CLI/desktop implementation, Wave resequencing, production editor
  selection/integration, new authoritative content model or custom editor.
- No GPUI/GPUI Kit upgrade, extra core GPUI graph, direct `gpui` dependency,
  vendoring, fork, editor-core rewrite or executable Velotype extraction.
- No changes to `src/lib.rs`, domain algorithms, journals, registry schema,
  canonical save behavior or production entry points to make the probe pass.
- No production draft persistence, polling scheduler, transport-stall experiment,
  real remote contact, repository/key administration or real user content.
- No network/resource execution from rendered content, implicit file saves,
  Git checkpoints or automatic OS link opening in the probe.
- No source-only product fallback or relaxation of the approved native matrix.
- No pushing, merging, ticket closure or cleanup without later authorization.

Stop and retain evidence if the dependency graph requires an excluded change,
untouched source is lost, raw/source mode resets undo irreparably, a candidate
cannot be host-contained, or rich-text vocabulary requires editor-core work.
Thin adapters may translate events/styles and enforce host policy; they may
not reconstruct an editor engine or conceal data loss. Correct preservation
with unsupported rendering is recorded as a vocabulary gap, not full success.

## Acceptance and evidence

| ID | Required outcome | Proof and limitation |
| --- | --- | --- |
| R1 | Exact candidate and one core GPUI identity | Version/checksum, lockfile diff and resolved package graph; compile success also required. |
| R2 | Host-only persistence/resources | Source/API audit plus native negative interactions; no callbacks that open arbitrary URLs or save files. |
| R3 | No-op byte fidelity | Editor readback equals each original fixture after open, repeated mode switches and no-change snapshot. Host no-save behavior is not proof of production no-commit save. |
| R4 | Focused edits and shared undo | Actual editor edit/mode/undo/redo traces; compare exact output and untouched ranges. Include CRLF, Unicode and mixed/unsupported content. |
| R5 | Required rich vocabulary | Per-construct native edits; tables include cell edit, alignment, add/remove row and column, and undo. Source-only table fallback is not full rich-text acceptance. |
| R6 | Native host usability | Recorded Linux Wayland keyboard-only mode/format/table/focus journey, IME composition and clipboard checks. Automated or source claims do not substitute. |
| R7 | Dirty input preserved at external-change seam | Feed simulated newer base/completion to the host, retain dirty draft and expose explicit review; no production recovery claim. |
| R8 | Responsiveness characterization | At least 100 local input samples on a recorded reference machine with a 100 KiB mixed fixture; report p95 against the approved <100 ms target. Method/coverage limitations remain explicit. |
| R9 | Complete provisional API inventory | Every CLI verb and desktop/runtime capability has a row, actual symbol or explicit gap, evidence status, owner and re-audit condition. |
| R10 | Reviewable recommendation | Adopt-with-obligations, adapt-with-approval, reject or insufficient-evidence; include unresolved native checks, costs and next approval. |

Each evidence record names commit/tree identity, lockfile identity, OS/architecture,
compositor/display, candidate pin, fixture, exact command/action, expected and
observed result, artifacts and reviewer. Classifications are **source-inspected**,
**automated-tested**, **native-tested**, **blocked** or **not-tested**. Never treat
an inspected API, host model test, compile or mocked result as native fidelity
proof. Negative findings are a valid exploration outcome, not a reason to relax
requirements.

## Exit and downstream obligations

Exit only after evidence and recommendation receive review. Leave the ticket
open until code/PR approval and explicit lifecycle authorization. Selection
requires a separate explicit acceptance of the recommendation; the report does
not silently select a dependency.

Before Wave 03 Cycle 01, refresh against completed Wave 02 main and redo the
API audit. Before Cycle 09 planning, resolve editor blockers and accept the
integration approach, with remaining native obligations assigned. Cycles 08/09
must still implement real draft recovery, stale-save handling and no-commit
no-op saves. Cycle 12 owns transport cancellation characterization. Cycle 13
still requires the full approved native matrix and integrated journeys.
