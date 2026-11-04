---
title: "Completed-Context Retirement Implementation Plan"
date: 2026-11-04
status: "draft"
author: "GPT-6.1 Sol"
manyhands_managed: true
manyhands_kind: document
id: "01M6M3B4V4GJQZJ82CS32N4W0V"
---

# Completed-Context Retirement Implementation Plan

> **For agentic workers:** after explicit implementation/method authorization,
> use `executing-plans` for checkpointed execution or the owner's selected
> `subagent-driven-development` workflow. This draft authorizes neither.

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
the design, which governs this plan. D1–D9/R-1–R-4 apply; Q1 still needs approval.

## Global Constraints

- All owned edits stay in the specified ticket worktree/branch. Fetch and merge
  `origin/main`, never rebase; repeat when main moves. Do not touch sibling
  worktrees, push other branches, create a PR, merge, close, delete real contexts
  or dispatch native CI without the owner's separate go-ahead.
- Cargo only through `devenv shell -- cargo ...`. The current `AGENTS.md` tiered
  gate overrides the older planning-template full-suite instruction: targeted
  tests between tasks; static gates, fast tier and changed-path SSH tier at the
  end; full suite/native CI after merge, native dispatch separately authorized.
- No implementation until the Cycle/design/plan, E-1–E-9, Q1 and execution method
  are approved. Drafting approval is not implementation authorization.
- Tests first: introduce each observable row, run it and see the behavioral
  failure, then implement minimally and rerun. A compile failure alone is not
  the behavioral red evidence. No dead-code scaffolding-only production commits.
- Use existing test targets. New harness-false SSH cases go into the module's
  `CASES` and the runner's chain, split below the existing watchdog. Confirm each
  filtered run executes nonzero cases; Cargo takes one positional filter.
- Only E-1–E-9 may change old test semantics. Additional semantic changes or
  inability to prove a promised row stop for the owner, not expectation fitting.
- Keep the headless library free of GPUI. No desktop implementation here. Any
  genuinely necessary desktop compile-only exhaustive arm is an owner checkpoint,
  and also needs `--features desktop` on its own plus the current desktop-test
  clippy configuration; do not silently broaden scope.
- Preserve ordinary poll request digests/shape and existing release behavior.
  Never persist canonical sources, titles, secrets or raw backend diagnostics in
  retirement evidence. No direct recursive deletion or shell Git in domain code.
- Commit each green task with `Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>`;
  append its evidence/decisions/next step to the execution ledger and a canonical
  ticket comment. Push only this ticket branch, no force push. Golden updates use
  `MANYHANDS_UPDATE_GOLDEN=1` outside CI; inspect each resulting diff.

## Review Focus

Proof preserves all changed paths, not only the ticket; partial pruning never
becomes clean absence without owned intent; new attempts make fresh observations
but completed requests replay; normal-poll identity is backward-compatible;
batch state survives another poll and never enrolls a new context; one pass's
observation is reused without nested reservation/lease acquisition. Each has a
test in Tasks 1–6. The mechanism checkpoints in Tasks 2/4 are mandatory.

## Task 0: Re-Ground Main, Pins And Ledger

**Files:** create `docs/plans/2026-11-04-completed-context-retirement-execution.md`;
update the three planning documents only where grounding changes facts.

- [ ] Verify implementation authorization and execution method, clean worktree,
  owner identity, and any newer ticket/PR feedback. Fetch/merge main and record
  base/before/after SHAs and conflicts. Recheck D5's newly merged overlap; do not
  consume its unmerged branch.
- [ ] Reconfirm every Baseline Fact, proposed interface integration point, schema
  envelope and E-1–E-9 include/name. Read current `recovery` begin/resume/abandon,
  reservation guards and observation replay. Prove the exact child-observation
  correlation needed for a pending retirement without weakening other blockers.
- [ ] Sweep for all `PollOnce`, `PollMutationDto`, `PollDisposition`,
  `PollTargetDisposition`, `ReleaseContext`, operation action and CLI count pins.
  Record exact existing test names and every constructor-only edit in the ledger.
  If E-1–E-9 is insufficient, stop and ask before editing a new expectation.
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
  status/body/unknown/comment/code mismatches, deletions/renames/modes, document
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

**Create:** `src/repository/context_retirement/{record,run}.rs` and additional
tests in the new module; `tests/mutation_remote/context_retirement.rs` if the
existing shared SSH world is needed for domain tests.
**Modify:** module exports, `src/repository/{recovery.rs,read/{dto,status}.rs,
bridges/abandon.rs,discovery.rs}`;
`src/repository/remote/{observation,reservation,state}.rs` for the exact owned
advertisement receipt/capture and pin retention, not a current-snapshot guess;
`src/repository/cleanup.rs` only for narrowly shared guarded capability seams;
operation schemas and E-2/E-8. No release guard is relaxed.

- [ ] Write domain `retirement_` outcome/recovery rows first: fresh advertised
  absence, unknown history, reappeared ref, unavailable transport, primary wrong
  branch/item dirty, branch-only and already-absent, all cleanliness/ownership
  refusals, unrelated pending operation, own correlated observation retry.
- [ ] Persist retirement/observation identities and fixed tip before effects;
  capture operation/epoch/batch/generation in the live advertisement transaction,
  add receipt pin retention through configuration changes, and prove that a
  completed public observation replay or a different operation's current batch
  can never authorize deletion;
  implement exact primary proof and shared cleanup with recorded intent before
  prune, then worktree/branch steps. Test crash between effects and step writes,
  mixed partial prune, tip/primary/config changes and replacement registration.
- [ ] **Design checkpoint:** run the required-outcome table against the proposed
  record/reservation transitions; obtain independent adversarial review before
  proceeding. A needed recovery-contract change goes to the owner.
- [ ] Implement completed replay, index handoff and resume/abandon visibility.
  Index-only reentry does no network or deletion. Interrupted partial removal
  cannot be abandoned into an apparently clean success. Add additive schema
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
**Modify:** mutation `{mod,command,dto,outcome,progress}.rs`, binding registration,
`bind/resume.rs`, E-2/E-3/E-7/E-8; `tests/mutation_remote.rs` case registration.

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
  the approved Q1 table. Partial effects remain visible; no source/title/secret
  enters persisted evidence or diagnostic. Verify and commit
  `feat: bind explicit context retirement and recovery`; checkpoint.

```sh
devenv shell -- cargo test --all-features --locked --test mutation_remote retirement_
devenv shell -- cargo test --all-features --locked --test mutation_replay
devenv shell -- cargo test --all-features --locked --test mutation_contract
devenv shell -- cargo test --all-features --locked --test mutation_confirmation
devenv shell -- cargo test --all-features --locked --test mutation_bridges
```

## Task 4: Explicit Poll Retirement Batch

**Create:** `src/repository/mutation/bind/poll_retirement.rs` and
`tests/mutation_remote/poll_retirement.rs` (proposed modules).
**Modify:** retirement record/run module, mutation registration/poll binding/DTO,
session seam, `src/repository/remote/{poll,reservation,state}.rs` only for the
opted-in evidence capture (no destructive domain-poll path),
`schemas/v1/poll_mutation.schema.json`, E-3/E-6/E-7/E-9.

- [ ] Write `retirement_batch_` rows: opted-in versus ordinary/background;
  successful/dirty/unmatched mixed targets; empty planned list; poll failed,
  cancelled, interrupted or index-pending; mode mismatch and normal legacy digest;
  one live advertisement for many targets; no refresh per target.
- [ ] Write interrupted-parent cases before implementation: crash before/after
  manifest; latest-only poll report overwritten by another poll; changed tip;
  second context created after selection; stopped partial child; new process
  resume uses one fresh observation but never a fresh worklist; completed replay
  with recreated contexts; final aggregate index failure.
- [ ] Implement additive per-operation batch/target records and immutable report,
  selected child IDs and progress. Coordinate only around the existing explicit
  poll; do not add a destructive flag to its automatic domain request. Freeze
  the original report/candidate tip image in owned poll finalization before its
  reservation is released, then the worklist before first removal. A completed
  index-pending poll returns final `discovery_pending` without cleanup; only a new
  request can start it after repair. Before successful final capture, interrupted
  polls keep existing restart/refetch semantics with no retirement effects.
  Fence each executor, and preserve refusal
  versus pending/not-reached distinctions.
- [ ] **Design checkpoint:** independent adversarial review of exact ownership,
  cancellation, report/replay and index transitions. If this mechanism still has
  repeat blockers after three planning/review rounds, stop rewriting it; pin the
  required outcomes as tests and ask the owner for the design checkpoint.
- [ ] Preserve ordinary DTO output (optional field omitted) and old stored result
  decoding. Verify, commit `feat: opt explicit polls into safe context retirement`;
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
  mutation request/JSON options; no `--input`, `--prepare`, `--confirm` or force.
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

**Modify:** `docs/RFC/{git-workflow-and-conflict-recovery,mvp-rfc,cli-contract}.md`,
`docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md`, the three
planning documents and execution ledger.

- [ ] Apply the Cycle's Document Amendments, with dated owner decision references.
  Document exact result/exit/recovery rows and delivered interfaces, explicit-only
  flag versus background prohibition, conservative proof limits and extra
  standalone round trip. Note D6 desktop action/draft obligations and unchanged
  reads; do not edit D6 or latest-copy tickets without permission.
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
- [ ] Self-review `git diff origin/main...HEAD` against A1–A9 and E-1–E-9. Obtain
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
| A6, D2/D6/D8 | 2 durable single-context table; 3 request/resume; 6 binary replay |
| A7, D7, R-3 | 4 frozen manifest/adversarial checkpoint; 5 flag; 6 count/mixed batch |
| A8, R-2/R-4 | 1 schema upgrade; 2 record migrations; 3–5 inventories and legacy replay |
| A9, D4/D5 | 2 release controls; 4 ordinary/background controls; 7 D6/read exclusions |
| D9, Q1, E-1–E-9 | Approval gate/Task 0, named edits in owning tasks, final authorization audit |

All commands and outcomes above are planned. No Cargo command, new behavior,
native CI, real deletion, merge or close has been executed during planning.
