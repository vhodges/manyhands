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

**Goal:** recognize completed peer contexts and retire only proven-safe local
copies through `context retire` and explicit `poll once --retire-completed`.

**Architecture:** shared proof and durable per-context cleanup on existing guarded
primitives; mutation-boundary batch coordination around the unchanged default
domain poll; thin CLI adapters. Durable fixed worklists prevent old poll requests
from selecting new contexts. No resident worker or background cleanup.

**Tech Stack:** Rust 2024, git2, rusqlite, existing SSH transport, F2 mutation
records, clap builder catalog and existing CLI/SSH test harnesses. No dependency
or Cargo test-target addition is planned.

**Spec:** [Cycle](../Cycles/wave-03-completed-context-retirement.md) and
[design](2026-11-04-completed-context-retirement-design.md). The Cycle governs
the design, which governs this plan. D1–D14/R-1–R-8 apply; the expanded Q1 routing
and Q2 initial-state tables were included in the owner's document approval.

## Global Constraints

- All owned edits stay in the specified ticket worktree/branch. Fetch and merge
  `origin/main`, never rebase; repeat when main moves. Do not touch sibling
  worktrees, push other branches, create a PR, merge, close, delete real contexts
  or dispatch native CI without the owner's separate go-ahead.
- Cargo only through `devenv shell -- cargo ...`. The current `AGENTS.md` tiered
  gate overrides the older planning-template full-suite instruction: targeted
  tests between tasks; static gates, fast tier and changed-path SSH tier at the
  end; full suite/native CI after merge, native dispatch separately authorized.
- No implementation until the Cycle/design/plan, E-1–E-10, Q1/Q2 and execution method
  are approved. Drafting approval is not implementation authorization.
- Tests first: introduce each observable row, run it and see the behavioral
  failure, then implement minimally and rerun. A compile failure alone is not
  the behavioral red evidence. No dead-code scaffolding-only production commits.
- Use existing test targets. New harness-false SSH cases go into the module's
  `CASES` and the runner's chain, split below the existing watchdog. Confirm each
  filtered run executes nonzero cases; Cargo takes one positional filter.
- Only E-1–E-10 may change old test semantics. Additional semantic changes or
  inability to prove a promised row stop for the owner, not expectation fitting.
- Keep the headless library free of GPUI. No desktop Retire action here. E-10
  permits only the existing generic abandon-confirmation's new safety details
  and corresponding tests (D13), included in document approval. Run desktop
  check/release clippy with `--features desktop` alone, and confirmation tests
  with `desktop-test-support`; D4's all-targets desktop-only clippy baseline needs
  that test-support feature. No navigation/release or new desktop action scope.
- Preserve ordinary poll request digests/shape and existing release behavior.
  Never persist canonical sources, titles, secrets or raw backend diagnostics in
  retirement evidence. No direct recursive deletion or shell Git in domain code.
- Commit each green task with `Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>`;
  append its evidence/decisions/next step to the execution ledger and a canonical
  ticket comment. Push only this ticket branch, no force push. Golden updates use
  `MANYHANDS_UPDATE_GOLDEN=1` outside CI; inspect each resulting diff.

## Review Focus

Proof preserves all changed paths and modes, not only the ticket; partial pruning never
becomes clean absence without owned intent; new attempts make fresh observations
but completed requests replay; normal-poll identity is backward-compatible;
batch state survives another poll and never enrolls a new context, even one
recreated at the same tip; one pass's observation is reused without nested
reservation/lease acquisition; sequential local children complete their own
index handoffs rather than accumulating blocked rows. Each has a
test in Tasks 1–6. The mechanism checkpoints in Tasks 2/4 are mandatory.

## Task 0: Re-Ground Main, Pins And Ledger

**Grounding checkpoint:** source/pin reconnaissance at `74186d51` found the
ledger's H1–H3 (ordinary output is untagged despite the design's `poll` tag
wording; two unnamed SSH disposition expectations; one unnamed exact recovery
step enumeration). These are raised for owner resolution before Task 1. No
additional existing test semantics are approved by recording the findings.

**Files:** create `docs/plans/2026-11-04-completed-context-retirement-execution.md`;
update the three planning documents only where grounding changes facts.

- [ ] Verify implementation authorization and execution method, clean worktree,
  owner identity, and any newer ticket/PR feedback. Fetch/merge main and record
  base/before/after SHAs and conflicts. Recheck D5's newly merged overlap; do not
  consume its unmerged branch.
- [ ] Reconfirm every Baseline Fact, proposed interface integration point, schema
  envelope and E-1–E-10 include/name. Read current `recovery` begin/resume/abandon,
  reservation guards and observation replay. Re-ground the design's specified
  child fencing/capability checks at reserve/restart/owned, registration token
  installation and preserving-abandon index finalizer. Do not choose different
  semantics under this re-grounding task.
- [ ] Sweep for all `PollOnce`, `PollMutationDto`, `PollDisposition`,
  `PollTargetDisposition`, `ReleaseContext`, operation action and CLI count pins.
  Record exact existing test names and every constructor-only edit in the ledger.
  If E-1–E-10 is insufficient, stop and ask before editing a new expectation.
- [ ] Run baseline targeted commands below (not a whole suite), record counts and
  any pre-existing failures. Read the existing `CliPair` and harness cases before
  writing a new helper.
- [ ] Start the ledger with acceptance mapping, owner decisions and review record;
  commit `docs: start completed-context retirement execution`; ticket checkpoint.

```sh
devenv shell -- cargo test --all-features --locked --lib context_release
devenv shell -- cargo test --all-features --locked --test mutation_bridges release_
devenv shell -- cargo test --all-features --locked --test cli_remote c5_polling_deleted_context_preserved
```

## Task 1: Shared Proof, Poll Recognition And Disposition Upgrade

**Create:** `src/repository/context_retirement/{mod,proof,tests}.rs` (new names).
**Modify:** `src/repository.rs`; `src/repository/remote/{poll,state}.rs` and test
modules; `src/repository/mutation/{dto.rs,bind/poll.rs}`;
`schemas/v1/poll_mutation.schema.json`; E-1/E-2/E-5.

- [ ] Add table-driven `retirement_proof_` tests for design predicates and full
  changed-entry algorithm: ancestor, folded ticket, canonical normalization,
  status/body/unknown/comment/code mismatches, deletions/renames/modes (including
  both executable-bit directions on the exceptional ticket path), document
  presence only, malformed closure, different closed copies, missing objects,
  unrelated/shallow history and multiple merge bases. Use temporary repositories,
  never the registered checkout's real contexts.
- [ ] Run the new cases and see old behavior/no implementation fail. Implement
  the proof, with typed refusal reasons, tree-only reads and no effects.
- [ ] Add poll recognition rows for closed/promoted/open/unknown/dirty ticket and
  local-primary-not-advanced. Add exact old-schema migration and corrupt/partial
  envelope rollback rows. Implement recognition/guidance and every enum/SQL/DTO
  mapping, preserving legacy stored reports and all old enum values.
- [ ] Update only applicable existing closure goldens; keep open-deletion and
  still-published materialization controls. Verify and commit
  `feat: recognize completed remotely deleted contexts`; checkpoint.

```sh
devenv shell -- cargo test --all-features --locked --lib retirement_proof
devenv shell -- cargo test --all-features --locked --lib repository::remote::
devenv shell -- cargo test --all-features --locked --test read_contract
devenv shell -- cargo test --all-features --locked --test cli_remote c5_polling
```

## Task 2: Single-Context Durable Retirement

**Create:** `src/repository/context_retirement/{record,instance,run}.rs` and additional
tests in the new module; `tests/mutation_remote/context_retirement.rs` if the
existing shared SSH world is needed for domain tests.
**Modify:** module exports, `src/repository/{recovery.rs,read/{dto,status}.rs,
bridges/abandon.rs,discovery.rs}`;
`src/repository/remote/{observation,reservation,state}.rs` for the exact owned
advertisement receipt/capture and pin retention, not a current-snapshot guess;
`src/repository/cleanup.rs` only for narrowly shared guarded capability seams;
operation schemas and E-2/E-8. No release guard is relaxed. If the optional SSH
module is used, add its module declaration and `CASES` chain to
`tests/mutation_remote.rs` in this task, before its first test command.

- [ ] Write domain `retirement_` outcome/recovery rows first: fresh advertised
  absence, unknown history, reappeared ref, unavailable transport, primary wrong
  branch/item dirty, branch-only and already-absent, all cleanliness/ownership
  refusals, unrelated pending operation, own correlated observation retry.
- [ ] Implement the Q2 initial-state matrix, including branch-only full proof,
  all-absent local discovery settlement, and no-follow mixed/invalid identity
  refusals. Test stale index rows after externally removed contexts. No remote
  connection for all-absent/index-only results. Use noncheckpoint authoritative
  `ContextRetirementAbsent` for a fresh absent no-op; do not invent prior removal.
- [ ] Implement registration-lifetime token capture using exclusive no-follow
  admin-file installation and an owned intended token. Test interrupted install,
  malformed/colliding marker preservation, missing/replaced token and same-tip
  recreated worktree, specifically crash-before-install plus recreated admin
  directory. A missing token after interruption refuses the candidate, never
  installs the old intended token. A fully installed matching token may complete
  its interrupted SQL selection commit; a partial token is not repaired. Never
  refresh a captured old identity by installing a new token. Default/background
  polling installs none.
- [ ] Persist retirement/observation identities and fixed tip before effects;
  capture operation/epoch/batch/generation in the live advertisement transaction,
  add receipt pin retention through configuration changes, and prove that a
  completed public observation replay or a different operation's current batch
  can never authorize deletion;
  implement exact primary proof and shared cleanup with recorded intent before
  prune, then worktree/branch steps. Test crash between effects and step writes,
  mixed partial prune, tip/primary/config changes and replacement registration.
- [ ] Pin R-7 in tests: worktree removed/admin token gone, branch delete, crash
  before SQL receipt, branch recreated at the same OID, retry must preserve it.
  More generally a surviving branch-only/tokenless branch on a new executor
  cannot be deleted under the old request; use guarded abandon/new-request
  reproof. When all resources are absent after owned intent, reconcile metadata
  only. Keep release's pre-existing automatic branch retry tests unchanged.
- [ ] Implement D14 exact old materialization-intent capture and atomic
  compare-and-consume receipt when cleanup/absence is established. Refuse
  incomplete/ambiguous/replaced intent, preserve new intent, and do not rewrite
  an immutable old target OID. Test crash after Git absence/before SQL settlement
  and idempotent replay. Do not delete tracking refs or change current observation
  projections through the existing lifecycle retirement helper; do not change
  existing promotion/closure cleanup. Fresh all-absent settlement can consume
  only the exact completed old intent, without claiming an owned removal.
- [ ] Test the exact read-only child-replacement transaction: crash before/after
  advertisement and after batch commit, active older child, new parent's epoch,
  unrelated reservation, and stale old executor returning after replacement.
  Install the bound-parent capability checks at reserve, restart, every owned
  write and completion, not only initial reservation. Generation change refuses
  rebinding and offers the preserving-abandon path.
- [ ] Separate observer-execution authorization from batch receipt-consumption:
  receipts bind the batch/request attempt, generation and frozen worklist digest.
  Once observing child B finishes, authorized C can consume that same receipt;
  B cannot keep writing through its ended execution capability. Test one
  advertisement across B handoff/C begin, foreign/stale scope refusal and
  observation from a no-pending-child gap under the batch-attempt scope.
- [ ] **Design checkpoint:** run the required-outcome table against the proposed
  record/reservation transitions; obtain independent adversarial review before
  proceeding. A needed recovery-contract change goes to the owner.
- [ ] Implement completed replay, index handoff and resume/abandon visibility.
  Index-only reentry does no network or deletion. Guarded abandonment is delivered
  in Task 3; until then do not let generic abandonment release a retirement's
  request identity or hide a partial record. Add additive schema
  migration/corruption/audit rows. Verify and commit
  `feat: retire proven completed contexts with recoverable cleanup`; checkpoint.

```sh
devenv shell -- cargo test --all-features --locked --lib context_retirement
devenv shell -- cargo test --all-features --locked --lib cleanup
devenv shell -- cargo test --all-features --locked --lib recovery
devenv shell -- cargo test --all-features --locked --lib read::status
devenv shell -- cargo test --all-features --locked --test recovery_foundation_gate
devenv shell -- cargo test --all-features --locked --test mutation_remote retirement_
```

## Task 3: Shared Mutation, DTOs, Replay And Resume

**Create:** `src/repository/mutation/bind/retire.rs`,
`schemas/v1/context_retirement_mutation.schema.json` and new golden cases.
**Modify:** mutation `{mod,command,dto,outcome,progress,records,reenter}.rs`, binding
registration, `bind/{resume,abandon}.rs`, `bridges/abandon.rs`,
retirement run/record modules and claimed-refresh completion in `src/repository.rs`
and its discovery helper; E-2/E-3/E-7/E-8/E-10; existing CLI/desktop generic
confirmation rendering. Add missing `tests/mutation_remote.rs` case registration
now only for cases first introduced here; Task 2's cases are already registered.

- [ ] Write boundary rows before binding: same-ID retry, finished replay after
  context recreation, changed item/mode request mismatch, unrelated recovery,
  cancellation before/after worktree removal, auth/host errors with no implicit
  approval, partial cleanup and failed index handoff through `operation resume`.
- [ ] Add proposed input/mutation and exhaustive predicates: has local operation,
  unconfirmed, no checkpoint, changes items/discovery, session-backed read-only
  remote observation and truthful safe-point cancellation. Bind final replay to
  stored retirement evidence, not release's run-as-new rule. Do not globally mark
  `ContextBranchReleased` as a checkpoint.
- [ ] Implement typed data/schema and existing result-code/effects mappings from
  the approved full Q1/Q2 tables. Test Final versus Owed/no-effect/index-only,
  original-request routing and `resume::takes_up`, token and `request cancel`
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
- [ ] Implement the exact gap route: the last completed retirement child is a
  confirmed batch-stop handle only while its batch is unfinished and no child
  is pending. Special-case it before the generic completed-local no-op. Bind
  preview to batch/worklist/cursor/epoch; stop remaining targets atomically,
  without reopening the local row or adding aggregate index ownership. A raced
  new child makes the preview stale. Test stop after index-only resume, lost
  parent result-copy, cancelled A-complete/B-not-started gap, crash during stop,
  and old coordinator attempting B afterward. Generic remote P remains invalid
  for abandonment; ordinary completed operations remain no-ops.
- [ ] Add required-nullable preview retirement metadata to the existing schema
  and old fixtures as null; render its retained resources in existing CLI and
  desktop confirmation only. This is E-10, not a desktop Retire action. Verify
  and commit `feat: bind context retirement and guarded preserving recovery`;
  checkpoint.

```sh
devenv shell -- cargo test --all-features --locked --test mutation_remote retirement_
devenv shell -- cargo test --all-features --locked --test mutation_replay
devenv shell -- cargo test --all-features --locked --test mutation_contract
devenv shell -- cargo test --all-features --locked --test mutation_confirmation
devenv shell -- cargo test --all-features --locked --test mutation_bridges
devenv shell -- cargo check --features desktop --locked --bin manyhands
devenv shell -- cargo clippy --features desktop --locked --bin manyhands -- -D warnings
devenv shell -- cargo test --features desktop-test-support --locked --bin manyhands confirm
```

## Task 4: Explicit Poll Retirement Batch

**Create:** `src/repository/mutation/bind/poll_retirement.rs` and
`tests/mutation_remote/poll_retirement.rs` (proposed modules),
`schemas/v1/poll_retirement_mutation.schema.json`.
**Modify:** retirement record/run module, mutation registration/poll binding/DTO,
session seam, `src/repository/remote/{poll,reservation,state}.rs` only for the
opted-in evidence capture (no destructive domain-poll path),
`schemas/v1/poll_mutation.schema.json`, E-3/E-6/E-7/E-9.
Register the new test module and `CASES` chain in `tests/mutation_remote.rs`
in this task before running its filter.

- [ ] Write `retirement_batch_` rows: opted-in versus ordinary/background;
  successful/dirty/unmatched mixed targets; empty planned list; poll failed,
  cancelled, interrupted or index-pending; mode mismatch and normal legacy digest;
  one live advertisement for many targets; each completed local child's handoff
  precedes the next child's begin (D12).
- [ ] Write interrupted-parent cases before implementation: crash before/after
  selection in the poll-finalization transaction; crash after captured selection
  but before first child; latest-only poll report overwritten by another poll;
  changed tip or a context recreated at the same tip with a different admin token;
  second context created after selection; stopped partial child; new process
  resume uses one fresh observation but never a fresh worklist; completed replay
  with recreated contexts; child index failure and lost response after refresh;
  A removed/indexed but B not started; guarded abandonment ends the entire batch.
- [ ] Test original-handoff eligibility: failed original refresh, unrelated
  successful repair, original request retry still cannot retire; successful
  original refresh with loss before eligibility persistence conservatively
  cannot retire; loss after eligibility persistence can resume fixed selection.
  A replayed Complete wrapper/current stale flag is never live success evidence.
- [ ] Implement additive per-operation batch/target records and immutable report,
  selected child IDs and progress. Coordinate only around the existing explicit
  poll; do not add a destructive flag to its automatic domain request. During
  opted-in per-target capture, bind the existing registered instance token under
  that target's lease. Freeze the full original report/worklist/identity/child-ID
  image in owned poll finalization before its reservation is released; **that
  transaction is selection**, not a later current-state scan. A completed
  index-pending poll returns final `discovery_pending` without cleanup; only a new
  request can start it after repair. Before successful final capture, interrupted
  polls keep existing restart/refetch semantics with no retirement effects.
  Fence each executor, and preserve refusal
  versus pending/not-reached distinctions.
- [ ] Persist `unconfirmed` original-handoff eligibility with captured selection.
  Only this call's process-only owned-finalization capture token plus original
  successful Complete wrapper and current request epoch can mark it `eligible`
  before child begin. Persist `ineligible` for original pending/failure; after a
  crash, unconfirmed completed-poll handoff becomes final no-retirement
  `discovery_pending`, never inferred from later repair or public replay (R-8).
- [ ] Keep `P` as the existing remote poll/request operation, with `K_i` as
  distinct local operations, one active at a time. No batch-local journal row or
  aggregate index debt. Complete each child handoff before starting the next;
  reconcile an interrupted parent result copy from the child's stored outcome.
  Overlay pending batch state in opted-in request reentry, `operation list/show`
  and cancellation even though `P` is already complete. Direct child resume
  either names the original batch request or finishes its index-only handoff;
  it never starts another target. Preserve ordinary ended-poll visibility.
- [ ] **Design checkpoint:** independent adversarial review of exact ownership,
  cancellation, report/replay and index transitions. If this mechanism still has
  repeat blockers after three planning/review rounds, stop rewriting it; pin the
  required outcomes as tests and ask the owner for the design checkpoint.
- [ ] Keep ordinary `PollMutationDto` and its existing literals/output unchanged.
  Add a distinct `PollRetirementMutationDto`/`poll_retirement` tag and closed
  schema root requiring every field, retirement outcomes included. Register both
  roots for command `poll once`, decode old and new stored tags, and validate
  legacy ordinary, opted-in and invalid missing/mixed shapes. Do not add `oneOf`
  or change `tests/support/schema.rs` to support it. Verify, commit
  `feat: opt explicit polls into safe context retirement`;
  ticket checkpoint.

```sh
devenv shell -- cargo test --all-features --locked --test mutation_remote retirement_batch_
devenv shell -- cargo test --all-features --locked --test mutation_remote c5_poll
devenv shell -- cargo test --all-features --locked --test mutation_replay
devenv shell -- cargo test --all-features --locked --test mutation_contract
devenv shell -- cargo test --all-features --locked --test read_contract
```

## Task 5: CLI Command, Flag, Help And Rendering

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
devenv shell -- cargo test --all-features --locked --bin manyhands-cli retirement
devenv shell -- cargo test --all-features --locked --bin manyhands-cli polling
devenv shell -- cargo test --all-features --locked --test cli
devenv shell -- sh scripts/check-cli-no-gpui.sh
```

## Task 6: Two-Clone Binary Acceptance And Adversarial Evidence

**Create:** `tests/cli_remote/retirement.rs`. **Modify:**
`tests/cli_remote.rs` module/`CASES` chain, fixture/golden registries and E-5/E-9.
Reuse `CliPair`/`CliClone`, existing closure helpers/patterns and SSH harness;
add no new target, no real keys, no global Git identity or real profile writes.

- [ ] Add `retirement_cli_` cases for A1–A7: close in A/delete remotely, observe
  in B without removal, standalone retire, list/show/ready after refresh; folded
  nonancestor source copy; promoted document; dirty/unpublished code/comment/body
  variants; open/malformed/unknown; remote reappearance; primary not advanced;
  server unchanged and no checkpoint/push; optional mixed batch; replay/new context.
- [ ] Add a direct A9 preservation row: create a sentinel item/comment draft via
  `DraftStore::open_in(&p.b.env.data)` and `create`, targeting clone B's actual
  registered root/item. Release its claim, retire the clean context through the
  binary, reopen/list the isolated draft store, and assert ID, revision and full
  draft body/base remain recoverable and exportable. Use `tests/drafts/support.rs`
  and `src/drafts/export_tests.rs` as patterns, not developer-profile fixtures.
- [ ] Add D14's lifecycle row: B materializes document T; A promotes/deletes it;
  B retires and settles the exact old intent; A publishes a new same-ID edit Q;
  B's later default poll must materialize Q with a new intent, not resurrect T.
  Include a Git-absence/intent-settlement crash in the domain/boundary matrix;
  preserve new/foreign intent and unchanged observation/tracking state.
- [ ] Assert refs, registration, full working content and index before/after
  refusals, not only envelope codes. Successful rows prove context absent and
  primary item/readiness correct. Measure advertisement count for many targets,
  not only duration. Test stopped process and index-pending resume through binary
  where existing fault hooks allow it; otherwise record the exact library-only
  row, never claim it is binary evidence.
- [ ] Keep existing release and default/background poll controls green. Run,
  review and commit `test: prove peer-context retirement through two clones`;
  checkpoint with row-by-row results and evidence limits.

```sh
devenv shell -- cargo test --all-features --locked --test cli_remote retirement_cli_
devenv shell -- cargo test --all-features --locked --test cli_remote c5_polling
devenv shell -- cargo test --all-features --locked --test mutation_bridges release_
devenv shell -- cargo test --all-features --locked --test read_relationships
```

## Task 7: Contract And Downstream Amendments

**Modify:** `docs/PRD/mvp.md`,
`docs/RFC/{git-workflow-and-conflict-recovery,mvp-rfc,cli-contract}.md`,
`docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md`, the three
planning documents and execution ledger.

- [ ] Apply the Cycle's Document Amendments, with dated owner decision references.
  Document exact result/exit/recovery rows and delivered interfaces, explicit-only
  flag versus background prohibition, conservative proof limits and extra
  standalone round trip. Note D6 desktop action/draft obligations and unchanged
  reads; amend PRD `MH-COLLAB-006` and record this ticket's shared-library-plus-CLI
  exception to Wave track rule 5. Include the existing abandon-preview renderer
  scope without treating it as a desktop Retire action. Do not edit D6 or
  latest-copy tickets without permission.
- [ ] Re-run CLI taxonomy/recovery-table/schema inventory tests; grep old blanket
  cleanup claims and update only governing current contract text, not historical
  approval records. Commit `docs: specify completed-context retirement contract`;
  checkpoint.

## Task 8: Final Gates, Review And Owner Handoff

- [ ] Fetch/merge current main, re-ground affected references and record whether
  changes need renewed design approval. Run the static gates, fast tier below,
  and each changed-path SSH target. Remote and mutation paths imply at least
  `ssh_transport`, `remote_observation`, `remote_synchronization`,
  `remote_merge_recovery`, `remote_polling`, `cli_remote`, `mutation_remote`.
  If cleanup changes reach promotion/closure, also run `document_promotion` and
  `ticket_closure`; when unsure run them. Shared SSH-harness edits imply every
  SSH-harness target. Never describe a zero-case filter as passing evidence.
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

## Plan Self-Review

| Contract/ruling | Tasks and proof |
| --- | --- |
| A1/A3, D1, R-1 | 1 proof table; 2 cleanup; 6 folded/closed binary reads |
| A2/A5 | 1 strict predicates; 2 preservation/freshness/ownership; 6 negative SSH rows |
| A4, D3 | 1 document content evidence; 6 promotion row |
| A6, D2/D6/D8/D11/D13, R-6/R-7 | 2 durable single-context/instance/branch-lifetime table; 3 request/resume/preserving abandon and gap handle; 6 binary replay |
| A7, D7/D10/D12, R-3/R-5/R-8 | 4 frozen manifest/handoff eligibility/batch receipt scope/sequential child handoffs/adversarial checkpoint; 5 flag; 6 count/mixed batch |
| A4/A6, D14 | 2 exact old-intent consumption; 6 T retire/new-Q document materialization; existing cleanup issue only proposed follow-up |
| A8, R-2/R-4 | 1 schema upgrade; 2 record migrations; 3–5 inventories and legacy replay |
| A9, D4/D5 | 2 release controls; 4 ordinary/background controls; 6 draft sentinel; 7 D6/read exclusions |
| D9, Q1/Q2, E-1–E-10 | Approval gate/Task 0, named edits in owning tasks (E-10 existing confirmation only), final authorization audit |

All commands and outcomes above are planned. No Cargo command, new behavior,
native CI, real deletion, merge or close has been executed during planning.
