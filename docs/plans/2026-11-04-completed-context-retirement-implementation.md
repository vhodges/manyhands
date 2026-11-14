---
title: "Completed-Context Retirement Implementation Plan"
date: 2026-11-04
status: "approved"
author: "GPT-6.1 Sol"
manyhands_managed: true
manyhands_kind: document
id: "01M6M3B4V4GJQZJ82CS32N4W0V"
---

# Completed-Context Retirement Implementation Plan

> **For agentic workers:** the owner approved the documents and implementation
> on 2026-11-04 and selected `subagent-driven-development`: sequential task
> ownership, spec review, then quality review, with the named adversarial checks.
> D15, 2026-11-06, approves the simpler live-pass-only contract. This engineering
> revision passed both same-reviewer source/contract re-reviews through `08aa5465`
> after correction of findings against `43214ff7`; unchanged approvals/method stand.
> Implementation resumes under that existing authorization, not new lifecycle authority.

**Goal:** recognize completed peer contexts and retire only proven-safe local
copies through `context retire` and explicit `poll once --retire-completed`.

**Architecture:** shared proof and ordinary durable per-context retirement;
a thin original-live opted-in poll loop supplies one sealed initial-observation
input to independently requested children. Captured identity/report evidence
prevents reselection, but old-pass replay never restarts deletion. No batch
recovery engine, aggregate debt, resident worker or background cleanup.

**Tech Stack:** Rust 2024, git2, rusqlite, existing SSH transport, F2 mutation
records, clap builder catalog and existing CLI/SSH test harnesses. No dependency
or Cargo test-target addition is planned.

**Spec:** [Cycle](../Cycles/wave-03-completed-context-retirement.md) and
[design](2026-11-04-completed-context-retirement-design.md). The Cycle governs
the design, which governs this plan. D15 amends Q1/R-8 and the batch continuation
promises only; Q2, D1–D14 and proof/standalone safety remain. R-9–R-12 ground the
private seams/forward unwiring. This is the existing plan refreshed, not a fourth
plan or a request to redo unchanged approvals.

## Delivered Checkpoints And Remaining Work — 2026-11-06

**Integration re-grounding, 2026-11-07 UTC:** the controller's authorized
merge now brings in `origin/main` `1e5616af22630b08a84dc0fb42037135b385f81d`
(approved/closed D5) from premerge HEAD `f69d341b5caac47c95d50b64c685f6073d61c958`.
This is the current baseline for the existing plan and `AGENTS.md` rules;
`79c3931b` and the Task 7 grounding below remain historical. Preserve both
products at the shared mutation/progress/resume seams, then same whole-feature
SPEC delta → same QUALITY before restarting final gates. No task restart,
new policy, broad interim test tier or desktop Retire action is authorized.

Source-grounded at clean Task 7 base
`76a852a7d5d4fcd757a95451601cdbb407b99bcc`; controller's fresh fetch/merge of
main `79c3931b01d225ddf3434ae9cd4fc51eb607fd2e` was already up to date.
Accepted without another fetch, worktree or main edit. Existing approval/method
stands; recording delivered contract is not a new owner decision round.

| Task | Current bounded delivery / exact names | Review and next gate |
| --- | --- | --- |
| 3 | Ordinary `Mutation::ContextRetire(ContextRetireInput)`, `ContextRetirementMutationDto`, item-owned reentry/resume, confirmed preserving/index settlement; forward obsolete-gap unwiring and independently frozen old catalogs | SPEC/QUALITY through `989a5807`; no whole-feature pass implied |
| 4 | `Mutation::PollOnceRetiring(PollOnceInput)`, `bind/poll_retirement.rs`, private original-live initial input, production `poll_retirement_passes`/`targets`, exact child-source membership, consumers-v1 migration and immutable cut | SPEC/QUALITY through `c514d160`; old selected-pass reentry is report-only, no loop |
| 5–6 | CLI `context retire --repo --kind ticket\|document --id ULID`, explicit poll flag; closed roots, ordered current-action rendering and bounded local/binary/one-new-SSH acceptance | SPEC through `1619e100`, QUALITY through `37b64d17`; latest changed SSH assertion at `4df41831` (1 case, 42.31 s total) |
| 7 | Existing governing contracts amended against those actual APIs, DTOs, schemas and `src/results.rs`; canonical checkpoint and scoped prose/inventory checks | Locally delivered, **independent SPEC then QUALITY pending** on the full Task 7 range from `76a852a7` |
| 8 | Final whole-tree static/fast/changed-path SSH gates once at review-ready, actual retained-44 coverage classification, whole-feature reviews and owner handoff | **Not started by Task 7; still owed**, not waived or replaced by bounded approvals |

The Task 3–6 checklists/commands below retain the **approved delivery and test
scope**, including originally proposed exact test names; they are not claims
that the mapped APIs remain missing, new execution instructions, or an assertion
that every retained fault/platform permutation has evidence. The ledger supplies
actual names/SHAs/results and remaining E/O/D/S rows. Task 8 must classify
essential uncovered behavior against actual coverage; this amendment authorizes
neither a broad new matrix nor a waiver. Historical source-feasibility and review
records keep their original dates/SHAs. No new plan/document is introduced.

The [API audit](../research/wave-03-api-audit.md#completed-context-retirement-delivery-2026-11-06)
and [CLI contract](../RFC/cli-contract.md#completed-context-retirement) are the
current delivered-name/wire/result references. One shared authoritative Fetch
input is not one total wire connection; ordinary Fetch authentication/reconnect
uses two upload-pack commands for both default and the measured two-child pass.
D6's explicit desktop Retire/editor/draft/keyboard/recovery journeys remain owed;
only the existing generic abandon-preview metadata display is delivered here.

## Global Constraints

- All owned edits stay in the specified ticket worktree/branch. Fetch and merge
  `origin/main`, never rebase; repeat when main moves. Do not touch sibling
  worktrees, push other branches, create a PR, merge, close, delete real contexts
  or dispatch native CI without the owner's separate go-ahead.
- Cargo only through `devenv shell -- cargo ...`. Current merged main's `AGENTS.md`
  Testing Strategy overrides old commands: between steps run only just-written/
  changed tests by exact full name, with check **or** Clippy + fmt at coherent
  checkpoints. Never rerun unchanged green evidence without a named doubt; stop
  and narrow a test command at about 15 minutes, record the slow target.
- Full static gates + fast tier + changed-path SSH tier run **once at review-ready**;
  rerun thereafter only targets whose code/tests changed. No whole SSH target,
  broad retirement filter or overlapping repeated matrix between steps. No bare
  full suite premerge. Full suite/native CI after merge; native dispatch separately
  authorized. The D15 docs-only revision ran no Cargo/build/tests; Task 7 runs
  only an existing exact inventory/recovery parser case when its text pin changes.
- Owner has approved implementation/method and D15. The same source/contract
  reviewers' re-review through `08aa5465` passed; routine internal representations
  do not require another product approval. Stop for a genuine policy change.
- Tests first: introduce each observable row, run it and see the behavioral
  failure, then implement minimally and rerun. A compile failure alone is not
  the behavioral red evidence. No dead-code scaffolding-only production commits.
- Cheapest layer first: table-driven library tests with local Git/temp registries
  and injected clocks; non-SSH API/CLI with local/file remotes. At most **one new
  SSH E2E case for this feature**, only to prove authenticated advertisement/host
  trust/output privacy connect end to end; edge/fault variants stay local. Existing
  105 retirement SSH cases are branch-era evidence, not permission for more matrix.
  Review any forward consolidation/rehome with coverage mapping before removing
  old cases. Extend existing targets; no real waits. Verify each exact filter
  executes nonzero cases; record full names/counts/SHAs in each ticket checkpoint.
- Only E-1–E-10 may change old test semantics. Additional semantic changes or
  inability to prove a promised external safety row stop for the owner, not
  expectation fitting. D15's design-named branch-era retirement test adaptations
  are included; routine internal refactoring uses SPEC/QUALITY, not renewed owner
  approval. Preserve every pre-feature assertion outside approved additions.
- Keep the headless library free of GPUI. No desktop Retire action here. E-10
  permits only the existing generic abandon-confirmation's new safety details
  and behavioral tests (D13), included in document approval. No appearance/render
  assertions, navigation/release changes or new desktop action scope. Follow
  AGENTS' current feature/gate rules, not earlier D4 workaround commands.
- Preserve ordinary poll request digests/shape and existing release behavior.
  Never persist canonical sources, titles, secrets or raw backend diagnostics in
  retirement evidence. No direct recursive deletion or shell Git in domain code.
- Commit each green task with `Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>`;
  append its evidence/decisions/next step to the execution ledger and a canonical
  ticket comment. This bounded revision does **not** push. Future pushes need the
  controller's existing authorization and stay on this ticket branch, no force
  push. Golden updates use `MANYHANDS_UPDATE_GOLDEN=1` outside CI; inspect each diff.

## Review Focus

Proof preserves every changed path/mode; partial pruning is not absence without
owned intent; R-7/nonce/ASCII/configuration/current-ad/ownership safeguards survive
the refactor. Ordinary child IDs, accepted attempts and index epochs are independent
of parent R. One uninterrupted pass uses one advertisement; stopped R can be Final
with R_i Owed, but replay never observes, allocates children or starts cleanup.
Pending child debt blocks next item; refusals do not; no earlier success rolls back.
Standard preserving receipt/index finalization must ship with the bindings.

Delivery is consolidated into **Task 3 ordinary binding + safe recovery,
Task 4 thin live loop/report, Tasks 5–6 CLI + acceptance, Tasks 7–8 contracts/final
gates**. Tasks 3–6 are now bounded-approved at the checkpoints above; Task 7 is
locally delivered/review pending and Task 8 still owed. Tasks 0–2 below are history/reuse boundaries, not a prerequisite ladder
of resumed-C/gap/new-F2 batch frameworks. SPEC then QUALITY review follows each
coherent delivery chunk; adversarial focus remains in the ordinary/shared-input
and live-loop checkpoints. Test exact new names, not the old command blocks.

## Task 0: Re-Ground Main, Pins And Ledger

**Completed historical checkpoint:** `74186d51..6a8b4259`, independent SPEC/QUALITY
passed; owner H1–H3 approved. Ordinary output stays untagged and exact E-2/E-5 pins
are recorded in the ledger. Do not rerun those unchanged baselines.

**D15 re-grounding:** clean entry `b2fe68c93580a85ec8c41768a79fcc535e1fdc1e`;
controller has just fetched/merged main `79c3931b01d225ddf3434ae9cd4fc51eb607fd2e`,
already up to date. No new fetch/merge or other-worktree inspection here. Current
source feasibility is the design's dated historical table; its read-only
source/contract re-reviews passed through `08aa5465`. Recheck main when it moves.

## Task 1: Shared Proof, Poll Recognition And Disposition Upgrade

**Completed and reviewed:** `834415b9..2f2f9b9d`; proof, independent ticket
completion, `closed`/`promoted`, strict old15→new17 migration and nested visibility
have SPEC/QUALITY approval. Keep all assertions/evidence; no duplicate proof or
recognition implementation task. Files/pins/results remain in the ledger.

## Task 2: Single-Context Durable Retirement

**Reuse, not full completion:** reviewed proof/absence/branch-only/present and
domain recovery, current-F2/exact-map/index claims, original-P source/handoff and
original-live safety increments are recorded through published `8e960885`.
Private projections are not production Task 4 coordination. No full Task 2 pass
is claimed; retained obligations in the ledger's amended 44-row register feed
Tasks 3/4/6 rather than blocking delivery on obsolete batch mechanisms.

Unpublished `2acac179` → `f196dfff` → `b2fe68c9` is historical gap work, **not
SPEC/QUALITY approved**. D15 removes resumed C/frozen remainder, gap observers,
advancing resumed cursors, special new-F2 batch/index recovery, completed-child
batch-stop handles and parent-Owed-until-all-finish from required delivery.
Forward unwiring is in Task 3; never reset/revert user work or rewrite history.
Source shows the gap catalog installs via `record::migrate`, so simply abandoning
its caller is insufficient. Ordinary poll pre-selection recovery and all retained
proof/identity/M/Z safety still require coverage and final review.

## Task 3: Shared Mutation, DTOs, Replay And Resume

**Bounded implementation delivered and independently approved through `989a5807`.**
The original scope below is retained for traceability/Task 8 coverage review;
the delivery map above and ledger name current interfaces and exact evidence.

**Create:** `src/repository/mutation/bind/retire.rs`,
`schemas/v1/context_retirement_mutation.schema.json` and new golden cases.
**Modify:** mutation `{mod,command,dto,outcome,progress,records,reenter,execute}.rs`, binding
registration, `bind/{resume,abandon}.rs`, `bridges/abandon.rs`,
retirement `{run,authority,request_scope,reentry,record,source,handoff}.rs` and
claimed-refresh completion in `src/repository.rs`
and its discovery helper; E-2/E-3/E-7/E-8/E-10; existing CLI/desktop generic
confirmation behavior. Extend local units and existing non-SSH
`tests/{mutation_replay,mutation_confirmation,mutation_bridges}/` modules, not
another SSH matrix. Deliver reusable ordinary retirement + safe exit as one
coherent chunk, with sequential SPEC then QUALITY before Task 4 integration.

- [ ] First map retained 44-row obligations to existing tests; extend table-driven
  local coverage only where a regression is unpinned. In forward commits unwire
  `context_retirement/gap.rs`, its reservation/poll hooks, record catalog/marker/
  audits/ID reservations and obsolete batch-only support as source-reachable.
  Preserve history; use synthetic exact prior and installed-empty-gap envelopes,
  and fail closed without data loss for unsupported nonempty experimental
  histories. No real profile inspection/repair. Retain published standalone
  migration/receipt tests. **Before any DDL constant/catalog changes**, freeze
  independent v1/v2/pre-gap index and exact installed-gap catalog definitions;
  keep the existing fingerprints in design R-12 rather than deriving old catalogs
  from edited `objects()`/`RECEIPTS`. Task 4 owns the coherent consumer-envelope
  migration and initial binding; do not claim today's provenance UNIQUE is reusable.
  Design's D15 named-test list governs semantic changes.

- [ ] Write boundary rows before binding: same-ID retry, finished replay after
  context recreation, changed item/mode request mismatch, unrelated recovery,
  cancellation before/after worktree removal, auth/host errors with no implicit
  approval, partial cleanup and failed index handoff through `operation resume`.
- [ ] Add proposed input/mutation and exhaustive predicates: has local operation,
  unconfirmed, no checkpoint, changes items/discovery, session-backed read-only
  remote observation and truthful safe-point cancellation. Bind final replay to
  stored retirement evidence, not release's run-as-new rule. Do not globally mark
  `ContextBranchReleased` as a checkpoint.
- [ ] Bind the actual existing `retire_context_for_request` /
  `reenter_context_for_request` seams (`request_scope.rs:339–367`) to ordinary
  R_s/S. Use existing Accepted/current-F2 map, standalone epoch replacement,
  M-only and I-only paths; do not duplicate their engine. Factor observation
  acquisition from the guarded local executor as needed for Task 4's private
  initial input, without fabricating a RemoteReservation or exposing receipts.
  Parent association must never enter the child's digest or permanent owner.
- [ ] Implement typed data/schema and existing result-code/effects mappings from
  the approved full Q1/Q2 tables. Test Final versus Owed/no-effect/index-only,
  own ordinary-request routing and `resume::takes_up`, token and `request cancel`
  routing, stable refused-request replay and no run-as-new after final refusal.
  Partial effects remain visible; no source/title/secret enters persisted evidence
  or diagnostic. Commit only after the preserving-abandon rows also pass.
- [ ] Implement D11 preserving abandonment under the existing prepare/confirm
  workflow: retirement-specific metadata in its existing preview variant,
  retained old request identity (never `release_operation_owner`), immediate
  fencing of destructive executor/observer, noncheckpoint authoritative
  `ContextRetirementPreserved`, claimed-refresh completion that atomically
  persists abandoned/partial receipts before releasing the blocker. Old request
  finishes/replays `external_change`; index failure stays owed to preserving
  continuation only. Test all no-removal/partial/config-changed/reappeared-remote,
  failed refresh/finalization and lost-result rows, including stale executor
  return. Generic non-retirement abandonment stays unchanged.
- [ ] No completed-child gap abandon route. Completed local operations remain
  generic no-ops; remote P is not a retirement-abandon target. Preserve/index
  recovery settles only real current item debt and retains its request identity.
- [ ] Add required-nullable preview retirement metadata to the existing schema
  and old fixtures as null; render its retained resources in existing CLI and
  desktop confirmation only. This is E-10, not a desktop Retire action. Verify
  and commit `feat: bind context retirement and guarded preserving recovery`;
  checkpoint.

At each test-first step run only its changed full name. Proposed names below must
be implemented/registered at those paths before use; they are not existing APIs
or completed evidence. Also run check **or** Clippy and fmt at a coherent checkpoint.

```sh
devenv shell -- cargo test --all-features --locked --test mutation_replay retirement::ordinary_retirement_replay_and_child_only_recovery -- --exact
devenv shell -- cargo test --all-features --locked --test mutation_confirmation retirement::preserving_receipt_and_index_finalization_are_atomic -- --exact
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
```

## Task 4: Thin Original-Live Poll Retirement Loop And Report

**Bounded production delivery and corrections independently approved through
`c514d160`.** `PollOnceRetiring`, its ordinary-child producer/migration and
report-only recovery are implemented; proposed names below record approved
planning scope, not still-missing APIs or fresh test executions.

**Create:** `src/repository/mutation/bind/poll_retirement.rs` (proposed),
`schemas/v1/poll_retirement_mutation.schema.json`.
**Modify:** retirement `{record,run,authority,request_scope,reentry,source,handoff}.rs`,
mutation `{execute,records,progress}.rs`/registration/poll binding/DTO,
session seam, `src/repository/remote/{poll,reservation,state}.rs` only for the
opted-in evidence capture (no destructive domain-poll path),
E-3/E-6/E-7/E-9 and existing local `context_retirement`/non-SSH mutation test
modules. Keep ordinary poll schema unchanged. Implement production capture/
selection/association/report here, not another preparatory coordinator layer.

- [ ] Write `retirement_batch_` rows: opted-in versus ordinary/background;
  successful/dirty/unmatched mixed targets; empty planned list; poll failed,
  cancelled, interrupted or index-pending; mode mismatch and normal legacy digest;
  one live advertisement for many targets; each completed local child's handoff
  precedes the next child's begin (D12).
- [ ] Local table-driven interrupted-parent cases: original capture loss before
  first child, durable-eligible crash, overwritten latest report, changed/recreated
  candidate and new context after selection, settled refusal then success, partial
  child and index-only child, lost parent copy after child completes, A indexed/B
  not started. Old R reentry must be summary-only with **zero** observations/new
  requests/cleanup. Final stopped parent may point to Owed R_i/K_i. New explicit
  pass handles remaining current contexts; no previous success is rolled back.
- [ ] Local sharing/lineage proof: K1 binds original P provenance, completes and
  releases only its own hold while the pass hold remains valid; K2 binds the SAME
  provenance through a distinct receipt/local epoch and ordinary R_i. Retain K1's
  immutable receipt. Stop parent with a pending ordinary child, then recover that
  item and release independently after parent Final. Cover exact selected present-W
  → fresh O/proof and owned-absence → M/I lineages from initial NULL observer;
  foreign/missing membership still refuses, neither route advances parent.
- [ ] Test original-handoff eligibility: failed original refresh, unrelated
  successful repair, original request retry still cannot retire; successful
  original refresh with loss before eligibility persistence conservatively
  cannot retire; loss after eligibility persistence also cannot restart the loop.
  A replayed Complete wrapper/current stale flag is never live success evidence.
- [ ] Implement minimal audited pass/target report records and immutable report,
  selected ordinary request IDs and provenance association. Coordinate only around
  the existing explicit poll; do not add a destructive automatic-domain flag. During
  opted-in per-target capture, install/read the validated registered-instance token
  under that target's lease using the existing intended-token safeguards (missing
  provisional token after restart never installs). Freeze the full original report/candidate identity/R_i
  image in owned poll finalization before its reservation is released; **that
  transaction is selection**, not a later current-state scan. A completed
  index-pending poll returns final `discovery_pending` without cleanup; only a new
  request can start it after repair. Before successful final capture, interrupted
  polls keep existing restart/refetch semantics with no retirement effects.
  Preserve ordinary poll restart/materialization behavior before **any** selection;
  an interrupted old invocation can finish ordinary P only, not mint retirement
  authority from repaired/replayed handoff. Release/fence its exact old source
  holds through normal recovery; retirement needs a new explicit pass.
  Once capture or retirement phase existed, recovery is report/settlement-only.
  Fence the live loop; preserve refusal versus actual pending/not-reached distinctions.
- [ ] Persist `unconfirmed` original-handoff eligibility with captured selection.
  Only this call's process-only owned-finalization capture token plus original
  successful Complete wrapper and current request epoch can mark it `eligible`
  before child begin. Persist `ineligible` for original pending/failure; after a
  crash, unconfirmed completed-poll handoff becomes final no-retirement
  `discovery_pending`, never inferred from later repair or public replay (R-8).
- [ ] Use design's feasible private execute-with-binding seam through ordinary
  request identity/accept/reenter/settle and the existing Session, with a sealed
  original-live initial input only in the private retire binding. Persist R_i
  before invoking; ordinary first acceptance fixes sole local K_i before effects.
  Add only a narrow transactional acceptance/admission callback to serialize
  current live start fence + association with normal request insertion. Reject
  stale stop/start races before effects; do not bypass the boundary with direct
  row fabrication or `Accepted::taking_up` (that is a remote-synchronization seam).
  Reserve-only entries stay not reached after stop. Metadata-only settlement is
  limited to accepted R_i whose ordinary local selection/claim provably NEVER
  established; no saved selected identity means no reconstruction/reselection/
  run-as-new (missing/corrupt evidence is not a never-claimed proof). A DURABLY
  SELECTED original present-W before Git keeps normal standalone reentry with
  fresh O/proof/nonce/lifetime where permitted; R-7 branch-only/tokenless limits
  stay unchanged. Neither recovery route starts another item. Persist pre-acceptance refusal rows
  before advancing; lost answers are reported only to the extent durable records
  prove them, never recovered by re-invoking cleanup.
- [ ] Route initial source consumption through the common ordinary local executor:
  deliver R-12's necessary one-to-many consumer representation, not just wider
  audits. Keep one P/source-epoch provenance; remove receipt provenance UNIQUE,
  add `UNIQUE(repository_id, consumer_kind, consumer_operation_ulid, consumer_attempt_epoch)`
  with child-local epoch distinct from R_i/F2, index and P epochs. Proposed
  immutable `poll_retirement_child_sources` ties exact R/P/image/selected identity
  to R_i/K_i/initial receipt + local epoch/F2. Ordinary O sources retain old
  source/consumer-request equality; initial poll children use this exact membership,
  never equate parent R with child R_i. Preserve receipt IDs, no fake P duplicates.
  Use fixed tip/nonce/M and
  current configuration/observation checks, ordinary child RequestFence. Keep
  `batch_operation_ulid`/`batch_attempt_epoch` NULL on new children. Today's batch
  rows cannot use those fences (`request_scope.rs:169–174`, `reentry.rs:119`);
  replace/unwire capture-bound batch execution/index support, do not merely turn
  off loop resume. No public caller receipt, N observations, fabricated O, separate
  recovery driver or permanent parent-epoch dependence.
- [ ] Implement the explicit `context_retirement_consumers_v1` marker/new exact
  envelope and whole dependency-closure migration from independently frozen prior
  catalogs (design R-12). Preserve all old receipt IDs and intent/M/absence/
  resumption/pin/release references and supported histories; validate new uniqueness
  against old rows. Rebuild known incoming-FK/trigger/index closure in ONE immediate
  transaction, without destructive parent DROP/cascade or shadow-FK retargeting;
  unknown incoming FK/catalog refuses. Commit marker only after full row/FK/history
  checks; fault rollback and idempotent reopen must preserve the entire old closure.
  Use synthetic pre-gap and exact installed-empty-gap upgrades, explicitly retire
  only empty obsolete gap objects/marker; unsupported nonempty gap data refuses
  unchanged. No profile access. This belongs to the existing Task 4 delivery chunk,
  not another coordinator/preparatory framework or an already implemented API.
- [ ] Replace provenance-wide fixed pin counts/reason assumptions with each exact
  holder/binding's live obligation or release receipt. Existing Z releases only
  the pass/source or child it owns and collects only without any holders/windows.
  Admit initial NULL-observer→ordinary reentry ONLY via exact initial association/
  receipt/epoch/F2/effect/M lineage (`record.rs:1331–1349`), not a generic NULL case.
  Recovery validates historical membership without current parent authority, then
  uses ordinary fresh O/M/I and unchanged R-7 rules.
- [ ] Stop/cancel serializes with admission under the common lease/transaction,
  revokes live starts/effects and atomically persists ONE immutable stop cut with
  child result/debt snapshots and exact not-reached membership BEFORE stopped-parent Final.
  Keep accepted child
  identity and real debt; settle owned no-effect holds, never release debt as a
  parent-finality shortcut. R may finish truthful non-success while R_i remains
  Owed. Replay/copy reconstruction reads the cut, NOT today's independently changed
  child standing; handles expose current child status. If death preceded any cut,
  first metadata reconciliation under lease/immediate transaction creates a cut
  from states durably known then, never invents an old snapshot or fills gaps by
  work. If no stopped cut exists, EVERY SELECTED target already has a durable
  settled outcome (success/legitimate refusal, handoffs settled) in its exact
  same-pass R_i/K_i association and only final copy was lost, complete from those
  outcomes at exit 0. A settled prefix with B/C not reached is non-success; do not
  count another pass's child IDs or rewrite a persisted stopped cut. R-8 handoff
  eligibility is not bypassed by an empty selected image. Ordinary child
  reentry/index uses normal current R_i and may never start C. New destructive
  child recovery needs its own O and ordinary nonce/proof/R-7 checks; M/I only
  uses recorded authority without new ad. Exact source/consumer Z still applies.
- [ ] **Design checkpoint:** independent adversarial review of exact ownership,
  cancellation, original-handoff proof, independent child recovery after R ends,
  stop/admission races, stable as-of-pass replay and exact hold release. Sequential
  SPEC then QUALITY at this delivery chunk; routine internal fixes do not reopen
  approved policy. No extra multi-stage batch lifecycle framework.
- [ ] Keep ordinary `PollMutationDto` and its existing literals/output unchanged.
  Add a distinct `PollRetirementMutationDto`/`poll_retirement` tag and closed
  schema root requiring every field, retirement outcomes included. Register both
  roots for command `poll once`, decode the old untagged shape and new stored tag, and validate
  legacy ordinary, opted-in and invalid missing/mixed shapes. Do not add `oneOf`
  or change `tests/support/schema.rs` to support it. Verify, commit
  `feat: opt explicit polls into safe context retirement`;
  ticket checkpoint.

Example exact **proposed** local table names; run only the just-changed one:

```sh
devenv shell -- cargo test --all-features --locked --lib repository::context_retirement::execution_tests::shared_provenance_two_children_keep_independent_holders -- --exact
devenv shell -- cargo test --all-features --locked --lib repository::context_retirement::store_tests::migration_tests::consumer_envelope_preserves_receipt_ids_and_rolls_back_dependency_closure -- --exact
devenv shell -- cargo test --all-features --locked --lib repository::context_retirement::execution_tests::live_initial_input_never_recreates_authority_after_stop -- --exact
devenv shell -- cargo test --all-features --locked --test mutation_replay retirement::live_pass_stops_replays_and_new_pass_handles_remainder -- --exact
devenv shell -- cargo test --all-features --locked --test mutation_replay retirement::stopped_parent_does_not_own_child_cleanup_or_index_recovery -- --exact
devenv shell -- cargo test --all-features --locked --test mutation_contract every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root -- --exact
```

## Task 5: CLI Command, Flag, Help And Rendering

**Delivered with Task 6; bounded SPEC/QUALITY approval through `37b64d17`.**
Actual grammar, required fields and numerical recovery are in the CLI contract.

**Create:** `src/bin/manyhands-cli/retirement.rs` and unit tests in the usual
path-module pattern. **Modify:** CLI entrypoint module list,
`manyhands-cli/{catalog,dispatch,polling,human_admin}.rs` as needed;
`tests/cli/{inventory,recovery_tables}.rs`, E-4/E-6/E-9 and golden registries.

- [ ] Test parser/help/rendering before implementation. Grammar:
  `context retire --repo <root> --kind <ticket|document> --id <ULID>` plus existing
  mutation request/JSON options; no `--input` or force. Declare the common
  `--prepare`/`--confirm` options but return `not_confirmable`, as other
  unconfirmed mutations do; do not turn them into parser errors.
  `poll once` gains boolean `--retire-completed`, default false.
- [ ] Use existing CLI mutation driver and session/transport initialization;
  include standalone retirement in the transport-requiring verb routing. JSON
  writes one envelope; human output names every retired/refused/pending target
  and exact continuation. Show ticket slug where the established contract does.
- [ ] Register one new verb under existing C5 follow-up attribution, input-free
  catalog/schema inventories and recovery tables; preserve release exemptions.
  Read every affected golden diff. Verify and commit
  `feat: expose context retire and poll retirement through the CLI`; checkpoint.

```sh
devenv shell -- cargo test --all-features --locked --test cli retirement::retirement_command_flag_and_stopped_report_contract -- --exact
devenv shell -- cargo test --all-features --locked --test cli inventory::every_verb_s_data_has_an_output_schema_and_its_goldens_match_it -- --exact
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

The `retirement::` table name is proposed; add its module to `tests/cli.rs`.
Run only new/changed full-name cases at each step, not an entire CLI target.

## Task 6: Local Acceptance And One Transport Connection Check

**Delivered useful CLI/acceptance chunk, not a full A1–A9/fault/platform gate.**
The ledger records the original 19 distinct exact results, source scopes and
changed-case reruns, including one new SSH case. Task 8 still classifies essential
remaining permutations; no blanket all-green retained-44 claim is made here.

**Extend:** local `context_retirement` unit tables and non-SSH
`tests/{mutation_replay,mutation_confirmation,cli}/retirement.rs` (proposed
modules, register in existing targets). **One optional new SSH case:**
`tests/cli_remote/retirement.rs`, register declaration/`CASES` in existing runner;
fixture/golden registries E-5/E-9. No new target or real profile/global identity.
Tasks 5–6 form one useful CLI/acceptance delivery chunk with SPEC then QUALITY,
not another sequence of preparatory milestones.

- [ ] Prove A1–A9 in cheapest relevant local tables: two local Git clones/bare or
  file remote for setup; closed/folded/promoted proof, dirt/unmatched/open/unknown,
  M/D14 and read readiness; independent R_i recovery after stopped R, mixed-complete
  exit 0, no rollback, new-pass remainder, frozen identity and no old-loop restart.
  Existing production authentication is SSH-specific, so a file URL is **not**
  advertised as a supported live retirement transport. Inject advertisements at
  the owned test seam for local domain/binding behavior; real binary no-network
  replay/index/preserving/contract rows use isolated fixtures. State the evidence
  boundary, not a new public file-transport feature.
- [ ] At most one new `retirement_cli_authenticated_live_pass` SSH E2E connects
  real authentication/host trust/Fetch observation to the two shipping entrypoints
  and privacy. Reuse `CliPair` and existing closure helpers, check default preserve,
  standalone fresh observe and one-ad live loop plus final reads/server no-push.
  It needs SSH because local injected advertisements cannot prove credentials,
  trust and transport framing. Edge/failure combinations stay in local tables.
  Existing 105 retirement SSH cases remain historical/current-branch legacy;
  map overlap and obtain review before forward consolidation/rehome, never add a
  parallel SSH retirement matrix or remove coverage merely for runtime savings.
- [ ] Add a direct A9 preservation row: create a sentinel item/comment draft via
  `DraftStore::open_in` and `create` in the isolated local acceptance data directory,
  targeting its registered root/item. Release its claim, retire through the guarded
  path, reopen/list the isolated draft store, and assert ID, revision and full
  draft body/base remain recoverable and exportable. Use `tests/drafts/support.rs`
  and `src/drafts/export_tests.rs` as patterns, not developer-profile fixtures.
- [ ] Add D14's lifecycle row: B materializes document T; A promotes/deletes it;
  B retires and settles the exact old intent; A publishes a new same-ID edit Q;
  B's later default poll must materialize Q with a new intent, not resurrect T.
  Use local Git/injected observation, including a Git-absence/intent-settlement
  crash in the domain/boundary table;
  preserve new/foreign intent and unchanged observation/tracking state.
- [ ] Assert refs, registration, full working content and index before/after
  refusals, not only envelope codes. Successful rows prove context absent and
  primary item/readiness correct. Measure advertisement count for many targets,
  not only duration. Binary replay/index recovery may use isolated persisted
  fixtures; transport interruption belongs only in the one justified E2E if needed.
  Otherwise record exact library-only fault rows, never claim binary/live-SSH proof.
- [ ] Keep existing release and default/background poll controls green. Run,
  review and commit `test: prove peer-context retirement through two clones`;
  checkpoint with row-by-row results and evidence limits.

```sh
devenv shell -- cargo test --all-features --locked --test cli retirement::retirement_replay_index_and_preserving_binary_contract -- --exact
devenv shell -- cargo test --all-features --locked --test cli_remote retirement_cli_authenticated_live_pass -- --exact
```

Both names are proposed; implement/register before use. Run a changed case alone,
not both automatically at every checkpoint. No real waits; injected clocks/durations.

## Task 7: Contract And Downstream Amendments

**Modify:** `docs/PRD/mvp.md`,
`docs/RFC/{git-workflow-and-conflict-recovery,mvp-rfc,cli-contract}.md`,
`docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md`, the three
planning documents and execution ledger.

- [x] Apply the Cycle's Document Amendments, with dated owner decision references.
  Document exact result/exit/recovery rows and delivered interfaces, explicit-only
  flag versus background prohibition, conservative proof limits and extra
  standalone round trip. Note D6 desktop action/draft obligations and unchanged
  reads; amend PRD `MH-COLLAB-006` and record this ticket's shared-library-plus-CLI
  exception to Wave track rule 5. Include the existing abandon-preview renderer
  scope without treating it as a desktop Retire action. Do not edit D6 or
  latest-copy tickets without permission.
- [x] Check the amended relative links, whitespace, source/schema/result consistency
  and old blanket cleanup claims; run only the existing exact taxonomy/recovery
  parsing test whose text pin actually changed. No schema/fixture/code changes
  or broad Rust/GPUI/SSH/fast/native reruns for prose. Exact taxonomy pin: 1 passed;
  357 relative link occurrences, no issues; frontmatter/history/scope and actual
  schema/result/source consistency checked. Original instruction:
  run only just-changed taxonomy/recovery-table/schema full-name tests; grep old blanket
  cleanup claims and update only governing current contract text, not historical
  approval records. Commit `docs: specify completed-context retirement contract`;
  checkpoint.
- [ ] Independent SPEC then QUALITY of the **full** Task 7 range from `76a852a7`
  before controller publication; Task 8 is not started by this documentation task.

## Task 8: Final Gates, Review And Owner Handoff

- [ ] Fetch/merge current main, re-ground affected references and record whether
  changes need renewed design approval. Run the static gates, fast tier below,
  and each changed-path SSH target **once at review-ready**. Remote and mutation
  paths imply at least
  `ssh_transport`, `remote_observation`, `remote_synchronization`,
  `remote_merge_recovery`, `remote_polling`, `cli_remote`, `mutation_remote`.
  If cleanup changes reach promotion/closure, also run `document_promotion` and
  `ticket_closure`; when unsure run them. Shared SSH-harness edits imply every
  SSH-harness target. Never describe a zero-case filter as passing evidence.
  Do not additionally run the overlapping `retirement_` filter or a bare full
  suite. Record per-target source SHA and results; after fixes rerun only targets
  whose source/tests changed, not unchanged green tiers.
- [ ] Self-review `git diff origin/main...HEAD` against A1–A9 and E-1–E-10. Obtain
  fresh independent spec/source and quality/adversarial reviews; verify findings
  against code, fix, re-review. No Blocking/Important findings remain at handoff.
- [ ] `git merge-tree --write-tree HEAD origin/main`; resolve conflicts by merge,
  rerun affected tests and record exact evidence. Mark ticket `review_ready`, add
  canonical checkpoint, commit/push only this branch. Request owner review;
  do not create a PR unless separately authorized.
- [ ] Only after code-review approval **and separate close/merge authorization**,
  set resolution status and follow the owner's current registered-checkout
  `ticket close` procedure; with a protected key the owner runs it in a terminal.
  It performs merge/publication/cleanup. Do not separately merge/delete anything,
  and do not treat document approval as closure authorization. Native CI remains
  post-merge, per-run separately authorized.

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib --bins \
  --test canonical_foundation --test cli --test discovery_rebuild \
  --test drafts --test journal_rejection --test key_material \
  --test key_storage --test local_authoring --test mutation_bridges \
  --test mutation_confirmation --test mutation_contract \
  --test mutation_relationships --test mutation_replay \
  --test read_boundary --test read_characterization --test read_comments \
  --test read_contract --test read_credentials --test read_items \
  --test read_relationships --test read_repository --test read_status \
  --test recovery_foundation_gate --test remote_reservation \
  --test repository_enablement --test session_credentials \
  --test shared_key_registry -- --skip repository::local_sync::
# Run each selected SSH target separately, for example:
devenv shell -- cargo test --all-features --locked --test remote_polling
git merge-tree --write-tree HEAD origin/main
```

### Approved Task 8 Correction — Deterministic Refresh-Claim Clock

Owner approved this bounded correction on 2026-11-07 after the whole
`mutation_replay` target returned **383 passed / 1 failed**. Its resume-contention
fixture stamps an `indexing` row once with real UTC but supplies no renewing
owner. A legitimate stale takeover can therefore defeat its intended live-owner
precondition. The historical run did not capture the claim-time cutoff; this is
a demonstrated fixture weakness, **not proof of that run's cause or a production
exclusion defect**. No further performance profiling is authorized.

**Files:** `src/repository.rs`, `tests/mutation_replay/resume.rs`, and the existing
execution ledger/canonical ticket checkpoint. Reuse `TestClock` and
`with_clock_for_testing`; add no clock trait, public test hook, dependency,
integration target, real wait or future-dated heartbeat.

1. Extend the existing exact case
   `resume::a_resume_of_a_row_another_call_is_indexing_under_is_busy_and_leaves_it`
   before changing production source. Inject a fixed clock at heartbeat `H`,
   using an ordinary fixed past timestamp, and retain every Busy, transient
   failure, retry, absent-request and unchanged-row assertion. Exercise both
   fresh `H` and equality `H + 3` clock values; advancing the injected clock to
   `H + 4` must permit the same request to reclaim and finish the hand-off.
   This pins the existing strict `<` cutoff without sleeps or a larger TTL.
2. Run only that exact case through Devenv and capture a compiled behavioral
   RED with the old real-UTC cutoff. A compile error is not RED evidence:
   ```sh
   devenv shell -- cargo test --all-features --locked --test mutation_replay resume::a_resume_of_a_row_another_call_is_indexing_under_is_busy_and_leaves_it -- --exact
   ```
3. Change only the cutoff argument in `claim_refresh_indexing` to
   `self.clock.now().unix_timestamp() - INDEX_OWNER_STALE_AFTER`. Leave
   `claim_lifecycle_indexing`, claim SQL, epoch fencing, heartbeat writes,
   production `SystemClock`, and the three-second constant unchanged.
4. Run the same exact case for GREEN, then strict whole-tree Clippy and fmt at
   the coherent checkpoint. Record exact commands, nonzero counts, source
   scopes and both RED/GREEN logs. Stop and report any need to broaden the
   production change instead of silently expanding clock plumbing.
5. Obtain the same whole-feature SPEC review, then the same QUALITY review,
   before further runtime gates. Reassess the affected verification scope:
   prior source-frozen greens remain evidence at their original SHAs, not
   automatically current-source passes after shared Rust source changes.
   Refresh required static/runtime evidence once after review, preserve valid
   unaffected coverage, and retain the first-failure admission rule.

The existing WAIT60 test accommodation, GPUI per-park guard, default test
concurrency, production leases/cache/SQLite/confirmation limits, retirement
authority and all lifecycle authorization boundaries remain unchanged. This
amendment extends the existing Task 8; it does not restart the Cycle or authorize
native CI, real-context retirement, PR creation, main integration or closure.

### Approved Task 8 Correction — Owned Poll Write Failure Points

Owner approved this bounded correction and verification selection on 2026-11-08:
add two precise points to the existing failure-injection mechanism, replace the
partial-batch yield trigger with the existing poll-boundary hook, verify only
the four failed SSH cases plus static checks, then resume the unrun gates.
Retain the 80 passing polling cases and every other completed green at its
recorded source scope; do not repeat whole targets or synthesize a fresh 84/0.

The failed fixtures attach unknown triggers to `remote_poll_dispositions`.
M1 correctly rejects those attachments during configuration/key preflight,
before fetch or the intended owned-write window. Keep the closed-catalog guard
unchanged. The historical parent log does not expose the underlying repository
error kind; its panic hook records source line numbers, not error payloads.

**Files:** `src/repository.rs`, `src/repository/remote/poll.rs`,
`src/repository/remote/state.rs`, `tests/remote_polling.rs`, and existing
ledger/canonical checkpoint metadata. No SSH harness, case inventory, dependency,
schema, new public setter or callback framework change.

1. Adapt only the four failed cases and their necessary shared fixture helpers:
   `poll_whole_review_disposition_fault_cancel_keeps_index_pending`,
   `poll_partial_batch_yield_report_and_restart`,
   `poll_batch_commit_failure_atomic_restart`, and
   `poll_fast_forward_before_disposition_failure_restart`.
   Preserve every original ref/OID, pending-index, cancellation, atomicity,
   restart, replay, no-push and output-privacy assertion. Require the three
   injected-write errors to be `InjectedFailure`, not a broad repository error
   that also accepts premature schema rejection.
2. Add only the declarations of two doc-hidden `FailurePoint` variants needed
   to compile those tests, before wiring any new runtime check. Run the three
   write-fault cases individually by exact name for compiled behavioral RED:
   the configured new points must not fire in the still-unmodified write paths.
   A compile error is not RED. The prior whole 80/4 run supplies the recorded
   partial-batch fixture RED; do not repeat that unchanged case for ceremony.
3. Add `PollBeforeDispositionInsert` at the actual first disposition INSERT,
   **after the transaction's disposition DELETE**, with advertisement/context
   capture already inside the ownership-validated batch transaction. Retain the
   existing `replace_poll_dispositions` signature through a thin internal
   no-op wrapper/check parameter so its other callers and private unit fault
   hooks keep their behavior. Call only the existing service-local one-shot
   failure mechanism; add no external callback under the transaction guard.
4. Add `PollBeforeDispositionWrite` inside the ownership-validated disposition
   update transaction, immediately before its UPDATE, after local Git effects
   and the separately durable `refresh_required` mark. Neither fault point
   changes default operation when no failure is configured.
5. Replace the partial-batch trigger with the existing guard-free
   `PollBeforeTarget / Some(1)` hook. Before requesting yield, prove committed
   primary FF/OIDs, untouched context and no discovery relative to the fixture's
   saved local-operation count. Bind the yield update to the exact operation and
   actual owner epoch, require one changed row, and prove one-shot consumption
   and normal restart ownership/flag clearing.
6. Run each of the four cases individually once for GREEN:
   ```sh
   devenv shell -- cargo test --all-features --locked --test remote_polling <exact-failed-case-name> -- --exact
   ```
   Do not rerun the other 80 cases or completed library, desktop, headless,
   local-sync or SSH gates. Run fresh required whole-tree check, strict Clippy
   and fmt once on the coherent amended Rust tree. Obtain same whole SPEC,
   then same QUALITY review before resuming the five unrun SSH targets/smokes.

The new points return `InjectedFailure` Rust errors at the same public effect
windows, not SQLite-generated `RAISE(ABORT)` or constraint errors. Retained private
library INSERT-fault coverage proves real SQLite transaction rollback after actual
advertisement/context writes, the old-report DELETE and INSERT0, followed by a Rust
`Err(state::recovery_required())` before INSERT1. It retains complete-image
restoration and legitimate reopen/replay proof, not an engine-generated constraint
error or its propagation. No new engine-specific error proof has been established
by this correction. Do not relabel old runtime evidence as newly executed on the
amended source. This owner-approved narrow selection adds no performance
investigation, safety-budget relaxation, native dispatch, push, PR, merge or closure
authority.

### Approved Task 8 Integration — Remote History Repair Main, 2026-11-08

Controller authorizes the required merge of `origin/main`
`8da9afd4ec82c145632988687cbd71bead046051` into clean entry
`c48bc2cf2d9d9bbe68621278bb56fedd7276d94c` (base `d70142b6`). Fresh preflight is
accepted; refs/cleanliness were reconfirmed and the actual merge has the assessed
12 conflicted files/22 hunks. No rebase or main-checkout edit. Owner additionally
authorizes necessary test repairs/expansions, not weakened guards/assertions,
new product behavior, production retirement authority or performance work.

1. Resolve additive DTO/export/module/test-helper/RFC conflicts structurally.
   Independently count command/home/schema inventories; retain strict pins and
   both closed stored decoders. Convert ticket-only configuration test callers to
   the upstream reason enum. Keep the reserved retirement ID guard before the
   invalid-configuration fence and preserve endpoint-versus-credentials checks.
2. Before compatibility wiring, extend the existing cheap migration tests with
   independent literal historical15/16/17 catalogs and exact marker profiles.
   Exercise valid rows, owner/time/row preservation, reopen, source-invalid rows,
   dependencies and marker inconsistencies. Observe compiled behavioral RED;
   conflict markers or compilation failures are not RED. Then compose one
   exact-profile upgrade to combined18. All source validation precedes DDL; do
   not retain main's unchecked unmarked rebuild or broaden the old15 row check.
3. Extend exact pin/Z tests to assert original batch/children/holder identities,
   rollback and copied-current-batch independence. Observe RED on the mechanical
   merge, then retain main's carry plus retirement/window exclusions in BOTH GC
   predicates. Never rebind old evidence to the copied batch. Current generation
   and exact receipt checks remain unchanged. No callbacks or new fault seams.
4. Preserve the closed remote-history enum, adding closed/promoted alongside
   upstream's members and null; run the existing exact enum contract check.
   Recognition's table must distinguish fetched deletion from unpublished and
   unknown history without deleting resources. Retain local disposition rollback.
5. Run only exact changed interoperability tests through Devenv, with names and
   literal argv recorded before admission in the ledger/new evidence. Run fresh
   coherent-tree `cargo check --all-features --locked`,
   `cargo clippy --all-targets --all-features --locked -- -D warnings` and
   `cargo fmt --check`. No broad library/fast/desktop/SSH rerun; 15-minute interim
   cap, no waits or thread/safety-budget changes. Do not repeat unchanged greens.
6. Record source/RED/GREEN/static evidence and qualified retained scopes, append
   the canonical checkpoint preserving its complete prior body, and commit the
   merge/resolutions with the required coauthor. Return clean/unpushed to the
   controller for SAME whole SPEC `ses_e4cc1a00bffeO0qOz2Do4Qx1Bd`, then QUALITY
   `ses_e4c8be2f1ffegiPUEUuB0QgyBE`, over complete c48-to-returned-HEAD range.

The three invalid-trigger retirement SSH fixtures and their approved cfg(test)
seam remain UNFIXED here. `document_promotion`, `ticket_closure`,
`comment_publication`, isolated smokes and final runtime reviews remain owed.
Retain prior cli_remote165/0, mutation805/3, polling80/4 and all other historical
evidence at their actual scopes; no combined-tree whole-pass claim. Upstream
ledger-reported gates are not reauthenticated or combined passes. Manual timing
benchmark stays ignored/NOT MEASURED. No push/PR/closure/native/real-context action.

### Owner-Accepted Deferred Marker-Race P2 — 2026-11-08

The owner supersedes the proposed production repair: record the registry-wide
marker race as an accepted deferred known issue and move on. Historical QUALITY
P2 remains **not fixed/not disproven**, source-derived/not reproduced, with
unmeasured likelihood. Per-repository flags do not make the current marker
per-repository; the possible interleave is resolution versus registration,
rebuild or cache recovery, not necessarily two syncs/processes. No production
fix/default normalization/exclusive-guard change/new hook or follow-up ticket is
authorized. Revisit with per-repository recovery-boundary or relevant concurrency
work. Carry integration SPEC CLEAR; require SAME QUALITY confirmation **with the
owner-accepted P2 deferred**, not zero findings, before controller-admitted
post-prune cfg(test) seam/three failed SSH-fixture work. Broad necessary-test-fix
authorization remains, not authorization to repair this production race. Prior29
exact checks/statics retain their unchanged scopes; no repeats or further gates
are admitted by this metadata amendment. Remaining SSH/smokes/final runtime
SPEC/QUALITY remain owed. The ledger/canonical append records the owner's words.

### Approved Post-Prune Seam / Three Fixture Correction — 2026-11-09 UTC

Owner says PROCEED. Controller carries SPEC CLEAR and reports SAME QUALITY
`ses_e4c8be2f1ffegiPUEUuB0QgyBE` QUALITY/SOURCE CLEAR through clean entry
`5af8ed2bf15ccb2eeaf7f647a3f14e122083b458`, with **one owner-accepted deferred P2,
zero blocking/new findings**, not zero issues. The registry-wide marker race and
performance work stay deferred. Broad necessary TEST repair/expansion authority
remains; an actual production bug or incoming production semantics/conflict must
return to controller rather than become an automatic policy fix.

1. Fresh fetch/merge from this ticket worktree: main remains
   `8da9afd4ec82c145632988687cbd71bead046051`; incoming diff empty; actual
   `git merge origin/main` exit0, Already up to date; ancestry/clean checks pass.
   Main checkout untouched. Preserve prior runtime integration `7447beb1` and all
   scoped historical approvals/results. This amendment precedes source changes.
2. Own only `src/repository/context_retirement/run.rs` and
   `tests/mutation_remote/context_retirement.rs`, plus these existing
   design/plan/ledger/canonical checkpoints. Add cfg(test)-only declarations for
   an operation/actual-owner-epoch keyed thread-local one-shot Rust fault with a
   non-Send scoped consumption/drop guard. Start with its checker unwired/inert.
   Use the existing outside-SQL worktree hook in a fixture helper to read the
   actual durable owner epoch and arm it. No production field, public API,
   runtime FailurePoint, dependency, arbitrary callback under SQL or schema edit.
3. Replace only `resumed_gap`, `reentry_ignore_w_observation` and
   `retirement_present_prune_receipt` trigger producers in
   `reentry_resumed_gap_m`, `reentry_worktree_only`, `present_sql_gap(false)`.
   Preserve every old assertion and branch=true D14 trigger semantics. Assert
   intended CleanupPending/Observed/None before checking consumption, so an
   unwired seam yields a meaningful compiled producer RED, not an unused-flag,
   compile or zero-match failure. Strengthen actual prune absence, retained
   unobserved worktree intent, no invented branch intent and legitimate reopen.
4. Add one cheapest-layer unit case for exact operation/epoch matching, one-shot
   consumption, drop cleanup and old-guard/new-arm separation. Observe its RED
   and each amended SSH case's compiled RED before behavior wiring. Never rerun
   the historical805/3 target to rediscover the known trigger problem.
5. Implement only the matching fault checker and cfg(test) call immediately
   before the worktree observation UPDATE in `prune_present`, after successful
   `verified.prune` consumed uninvoked and set permit. Let the existing transaction
   unwind: actual Git removal persists, SQL observation rolls back, live result
   Observed/None, durable possible/unobserved intent remains. No SQLite RAISE or
   historical payload attribution. Run the same exact unit/three SSH cases GREEN.
6. New-only evidence under
   `/tmp/opencode/task8-retirement-prune-gap-fix-20261109T013252Z/`: admitted literal
   argv/input hashes, full raw logs, actual counts/exits/times, compiled RED versus
   wired source proof, shipped-code exclusion, assertion/case-inventory preservation
   and final manifest. All Rust through Devenv/default threads, per-command900s
   cap. No waits/time aging/thread/timeout-budget changes; ordinary further TEST
   repairs are authorized but production semantics require controller return.
7. At the coherent tree run fresh all-tree check, strict Clippy and fmt-check.
   No whole808/old805/library/fast/Desktop/old29 regression repeat, remaining SSH
   targets, smokes, profiling or other gates. Append actual outcomes/current
   standing and canonical checkpoint preserving the entire prefix/ID/parent/time;
   commit bounded source/test/metadata with required coauthor, clean UNPUSHED.
8. Return full entry/runtime-freeze/base/main and evidence to controller for SAME
   SPEC `ses_e4cc1a00bffeO0qOz2Do4Qx1Bd`, then SAME QUALITY
   `ses_e4c8be2f1ffegiPUEUuB0QgyBE`, before any remaining gate. Still owed:
   `document_promotion`, `ticket_closure`, `comment_publication`, isolated smokes
   and final runtime SPEC/QUALITY. No status save/new IDs/push/PR/main closure/
   native/real profile/key/context/draft/cleanup action. Marker P2 remains
   accepted/deferred, not fixed; manual characterization ignored/NOT MEASURED.

Local checkpoint result: the one unit and three amended SSH cases each produced
a compiled behavioral RED, then each passed exactly1/0, exit0, after the minimal
cfg(test) wiring. An earlier Rust2024 closure-capture compile error was repaired
with precise `use<>` captures and is **not RED**. Fresh check/strict Clippy/fmt
pass; all GREEN/static input hashes match. Removing only the new cfg(test)
additions reconstructs entry `run.rs` byte-for-byte; all19 old assertions in the
three fixture functions, unchanged CASES/harness and branch=true D14 controls are
preserved. Actual counts/times/source proof are in the execution ledger/new
evidence. Return for SAME SPEC then QUALITY; no remaining gate admitted.

### Approved P1 Review Fix — Frozen Unpublished Reports, 2026-11-09 UTC

Owner returns an actionable P1 after published metadata HEAD
`62f79eec9c7b4a22855cb360113f17913b0b6737`, prior runtime
`a42460bb2d03bcba0db885657257d8ea0f62d66c`. **Review-ready is temporarily held**
until the bounded fix and controller's SAME SPEC then QUALITY review. This is
existing-contract compatibility, not a new policy or renewed implementation gate.
This amendment is recorded before test/production edits; no new planning document.

1. Fresh main `8da9afd4ec82c145632988687cbd71bead046051` is unchanged; actual
   `git merge origin/main` exit0, Already up to date, on the existing ticket
   branch. Current main `AGENTS.md` read; no rebase or main-checkout edit.
2. `NeverPublished` evidence produces normal `PollDisposition::Unpublished`.
   `FullPass::report` intentionally rejects only `NotReached` and
   `MaterializationIncomplete`; opted-in freeze captures the full report even
   with no candidates. Completion's record→handoff audit decodes that report
   inside the transaction. Its missing `Unpublished` member causes rollback /
   RecoveryRequired. Add **only `PollDisposition::Unpublished`** to the decoder
   allowlist after a compiled behavioral RED. Preserve the closed image,
   canonical encoding/schema audit, default poll shape and D15 report-only replay.
3. Extend existing cheapest local case
   `repository::context_retirement::execution_tests::mutation_tests::retirement_batch_public_live_pass_retires_real_context_and_replays_without_transport`.
   Retain its original normal/lost-copy assertions, add unpublished-only empty
   selection and mixed unpublished/real-retirement rows. Use the actual upstream
   `record_context_resolved` publication-evidence producer for never-advertised
   fixture contexts, not fabricated snapshot/authority rows. Assert the normal
   `Unpublished` target, completed empty or mixed selection, preserved unpublished
   branch/worktree/admin state, legitimate reopen and immutable no-fetch/no-new-
   loop replay. Any necessary fixture extension remains in the existing test file.
4. Literal RED/GREEN command, default threads, each capped at900s:
   `devenv shell -- cargo test --all-features --locked --lib repository::context_retirement::execution_tests::mutation_tests::retirement_batch_public_live_pass_retires_real_context_and_replays_without_transport -- --exact`.
   A compile failure or zero matches is not RED. Then fresh coherent-tree
   `cargo check --all-features --locked`, strict locked all-target/all-feature
   Clippy `-D warnings`, and `cargo fmt --check`, all through Devenv.
5. No whole poll84/mutation808/library/fast/Desktop/CLI/old-green repetition,
   SSH runtime or smoke: this decoder/capture/reopen regression is local, not a
   transport change. New-only full argv/raw/count/exit/input/source/RED→GREEN
   evidence: `/tmp/opencode/task8-unpublished-frozen-report-20261109T104659Z/`.
6. Update only this plan, ledger current standing/append and existing canonical
   comment append (preserve entire prefix/ID/parent/time), plus the one production
   line and existing test file. Required coauthor; return clean **UNPUSHED** for
   SAME SPEC `ses_e4cc1a00bffeO0qOz2Do4Qx1Bd`, then QUALITY
   `ses_e4c8be2f1ffegiPUEUuB0QgyBE` after controller verification, before publishing.
   Ticket remains implementing/review-fix-pending; no agents, PR/main close/native,
   real operations or cleanup. Marker P2 stays accepted/deferred **UNFIXED**;
   desktop PARTIAL/window-unverified accepted; manual benchmark **NOT MEASURED**,
   performance/perf-ticket/promotion-closure-materialization follow-up **STOP**.

Local P1 outcome, 2026-11-09 UTC: the unchanged-decoder exact test compiled and
failed **0/1**, exit101, **29.602632690s** total /5.69s harness: after verifying
the normal Unpublished target, unpublished-only original capture returned
RecoveryRequired rather than Ok. The original lost-copy row's caught panic was
expected. After the single enum addition, identical test bytes passed **1/0**,
exit0, **19.027548225s** total /8.58s harness, all four table rows reached.
Empty/mixed completion, unpublished resources, reopen and no-new-pass/no-fetch
replay assertions pass. No compile failure/zero match was counted as RED.

Fresh all-tree statics on the same1,723 source/build input hashes as GREEN:
check **exit0/4.978976427s**, strict Clippy **exit0/25.700913590s**, fmt-check
**exit0/17.939688986s**. The only RED→GREEN input change is `remote/poll.rs`;
removing its one new decoder line reconstructs published production source
byte-for-byte. No closed-image/schema audit, incomplete-disposition policy,
default poll shape, D15 or other production code changes. Existing green records
keep old scopes; this is not a new whole poll/library/SSH/desktop/CLI pass.
Return the bounded commit clean **UNPUSHED**, review-fix pending SAME SPEC then
QUALITY after controller evidence verification. Do not publish the fix yet.

## Plan Self-Review

| Contract/ruling | Tasks and proof |
| --- | --- |
| A1/A3, D1, R-1 | Retain 1/2 proof/cleanup; 3–6 local/binding/CLI folded/closed reads and one authenticated E2E connection check |
| A2/A5 | Retain 1/2 predicates/preservation/freshness/ownership; 3–6 table-driven local negative cases, not a new SSH matrix |
| A4, D3 | 1 document content evidence; 6 promotion row |
| A6, D2/D6/D8/D11/D13/D15, R-6/R-7/R-9 | Reuse 2's ordinary recovery; 3 request/resume/atomic preserving exit, no completed-child gap handle; 4 independent child after stopped parent; 6 binary/local replay |
| A7, D7/D10/D12/D15, R-3/R-5/R-8/R-10 | 4 initial fixed image/live handoff, thin one-ad loop, stopped-parent replay, child debt and new-pass remainder; 5 flag; 6 local mixed/count assertions + one SSH E2E |
| A4/A6, D14 | 2 exact old-intent consumption; 6 T retire/new-Q document materialization; existing cleanup issue only proposed follow-up |
| A8, R-2/R-4 | 1 schema upgrade; 2 record migrations; 3–5 inventories and legacy replay |
| A9, D4/D5 | 2 release controls; 4 ordinary/background controls; 6 draft sentinel; 7 D6/read exclusions |
| D9, Q1/Q2, E-1–E-10 | Approval gate/Task 0, named edits in owning tasks (E-10 existing confirmation only), final authorization audit |
| R-11 / obsolete gap and batch-specific machinery | 3 forward runtime/catalog/support unwiring with exact isolated-envelope proof; 4 replaces parent-capture execution/index dependence; no history rewrite or real-profile repair |
| R-12 / source review | 3 freeze independent historical catalogs first; 4 versioned one-to-many closure migration, two-child sharing, per-holder Z and exact initial-poll→ordinary O/M/I recovery; local tests only |
| Contract review C1/C2/C3 | 4 every-selected versus prefix outcomes, never-claimed versus selected original present-W, immutable cut/first-cut/all-selected lost-copy reconciliation; no new owner policy |

Remaining commands/outcomes above are planned. Earlier Task 0–2 executions and
approvals are preserved in the ledger at their actual source scope. This D15
revision is documents-only: no Cargo/build/test, Rust/schema change, native CI,
real deletion, push, merge or close. Read-only re-reviews by
`ses_e4ecf24b9ffeEuiFGI34Nf4fmm` (source) and
`ses_e4ecf24b3ffeJBNQqLYF0ceEPn` (contract) both passed through `08aa5465`,
each with 0 Blocking/Important/Minor findings. Corrections above are internal
under approved D15; implementation resumes without a renewed owner question or
full Task 2 readiness claim. Final implementation verification remains owed.
