---
title: "Closure Recovery Gate Implementation Plan"
date: 2026-10-25
status: "implementing"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5TP0N6C1V5EV2H3WQ6WHR62"
---

# Closure Recovery Gate Implementation Plan

> **For agentic workers:** the owner approved implementation using
> `subagent-driven-development`. The controller owns delegation/review;
> bounded implementers do not spawn subagents. Follow
> `working-on-a-ticket`'s targeted-test exception for this non-Cycle ticket.

**Goal:** Land the three library prerequisites for C4/C5/D5: accurate closure
conflict classification, recoverable local context synchronization with shared
bindings, and parent-correct child-execution wording.

**Architecture:** A dedicated local operation owns typed companion evidence under
the existing local journal. Transport-neutral merge/validation/artifact primitives
are shared with remote synchronization; its orchestration stays unchanged. New
read DTOs and two unconfirmed mutation variants expose the local capability.

**Tech stack:** Existing Rust, git2, rusqlite, serde/serde_json and hashing support.
No new dependency or test target. Initial main was `189284e`, merged as `189bb36`.
Before draft review, documentation-only main `0464c42` was merged as `0cbeab0`;
the researched library source is unchanged by that advance.

**Authority:** [Contract](../Cycles/wave-03-closure-recovery-gate.md), then
[design](2026-10-25-closure-recovery-gate-design.md), then this plan. All are approved.
Owner approval: “Every is approved. Proceed using subagent driven development”.
This covers detailed rulings, E1–E9, implementation and ticket-branch commits/pushes;
start comment `01M5TZ0E62RMQPKBGRPX3CX92E` records it. PR, closure, merge, cleanup
and native CI remain separately gated.

## Global Constraints

**Current checkpoint (2026-10-27):** Tasks 0–8 are implemented and independently
approved at `03a27cb`. Task 9 is in progress: its documentation/source audit is
checkpointed separately from final verification. The controller owns the final
Devenv runs and independent whole-branch SPEC/QUALITY reviews. This docs-only
implementer runs no concurrent Rust commands. No review-ready or lifecycle
completion is asserted. See the [execution ledger](2026-10-25-closure-recovery-gate-execution.md#task-9-documentation-and-source-audit).

- Work only in ticket `01M57SKVF6TQ3QM6AXJ35JGGSY`'s existing worktree/branch.
  Fetch and **merge** `origin/main`; never rebase. Before checkpoints run
  `git fetch origin` and `git merge-tree --write-tree HEAD origin/main`.
- Every Rust command is `devenv shell -- cargo … --locked`; use the prescribed
  `devenv shell -- cargo fmt --check` syntax for formatting.
- New behavior is library-only. Do not edit `src/bin/`, `src/desktop/`, Cargo
  dependencies, existing global result/recovery-action enums, or unrelated code.
- Named old-test edits are E1–E9 in the design. Add new tests freely within owned
  modules; any other semantic edit or unexpected failure stops for owner review.
  Mechanical extraction must preserve old assertions and behavior.
- Test first: add a real failing assertion, run it, implement, rerun. A missing
  symbol compile error can establish the initial new API seam; runtime recovery
  assertions must also be exercised before claiming a contract checkpoint.
- Use **one positional filter per Cargo test command**. Inspect the selected
  count; zero tests is not evidence. Confirm custom SSH harness filtering before
  selecting cases; use the complete relevant target if its filter is unsupported.
- This is a non-Cycle ticket worked under `working-on-a-ticket`: targeted tests,
  whole-tree fmt/check/clippy, no full suite. Native CI is post-merge and separately
  approved. Do not inherit C3's full-suite execution instruction.
- Clippy always uses `--all-targets --all-features --locked -- -D warnings`.
  Desktop-only gates are unnecessary while no desktop code changes.
- Tests use isolated homes/data, fixture identity and autocrlf settings; never
  real application data, developer keys or live remotes. Close handles before
  replacing databases. Use actual cross-process lease tests, not sleep guesses.
- Never log source, token internals, raw SQL, private input or backend messages.
  Store sources only as canonical Git/worktree effects, never request/journal text.
- Use existing golden-update environment switches, not hand-edited output
  fixtures, and inspect each diff. Do not regenerate unrelated golden collections.
- Take document/comment IDs from `manyhands-cli id new`, timestamps from UTC.
  Canonical comments record each task, owner ruling, gate and review checkpoint.
- Each task ends with a scoped commit when authorized; trailers end with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Inspect status/diff/log,
  stage owned files only. Ticket-branch publication and other lifecycle actions
  follow explicit authorization, never force-push or infer permission to merge.
- Implementers do not spawn agents. With subagent-driven execution, controller
  owns fresh task reviewers and returns fixes to the implementer.

## Review Focus

1. Frozen parent snapshots and replay before fresh observation checks: self-written
   state must not invalidate the operation's own retry.
2. No dangling authority: pending local rows, lifecycle reservations, mutation
   request attempts, index owners and Git artifact owners must agree after crashes.
3. Resolution input is ephemeral: after a partial path write, generic resume must
   name the correct original **resolution request**, not reconstruct missing source.
4. Side-derived immutable closure comparison includes the ancestor. Do not weaken
   legacy resolution validation merely because fresh classification changes.
5. Read tokens are capabilities over enumerated entries, not supplied paths/OIDs.
   Unknown encoding/modes, symlink substitution and concurrent reads must refuse.
6. Cancellation acknowledgement must permit explicit continuation without erasing
   a concurrent new cancellation or pretending outstanding work is finished.
7. Named-edit inventory must cover closed-set contract tests; do not treat adding
   a new action as permission to alter CLI/desktop exhaustiveness handling.

## Task 0: Baseline, Inventory And Required-Outcome Checkpoint

**Files:** create `docs/plans/2026-10-25-closure-recovery-gate-execution.md` and
canonical checkpoint comments. Production/test edits are not part of Task 0.

1. Check status, comments, current ticket/PR state, main ancestry and active
   overlapping branches. Merge current main; record before/after SHAs. Confirm
   no newer gate decision or implementation already exists.
2. Recheck design Facts 1–24 by source. Read every function named in later tasks;
   record public/private boundaries and actual test module/filter names. Reconcile
   the sibling cleanup ticket's scope without changing its worktree.
3. Inventory all references to `Command::ALL`, `OperationAction::ALL`, `LocalStep`,
   `HOMES`, `DATA_ROOTS`, `contract_enumerations`, `MutationDataDto`,
   `RecoveryActionKind`, `cancel_requested`, `require_no_pending_local`,
   `held_by_another_journal`, and refresh takeover. Classify production exhaustiveness
   versus old test expectations. Recheck the explicit E7/E9 paths; an unlisted
   semantic edit is an owner question, not an implicit exemption.
4. Recheck `tests/support/ssh_harness.rs:97–133` selection: E1 runs as
   `devenv shell -- cargo test --all-features --locked --test ticket_closure true_race_distinct_identities_needs_external_recovery -- --exact`
   and must report one SSH case. An unmatched filter currently succeeds with zero
   cases, which is not accepted evidence. The no-network local acceptance
   fixtures must use local unit fixtures, not `ticket_closure::Pair`, which starts
   an SSH server even in its no-remote configuration.
5. Run baseline checks and targeted existing evidence:

   ```sh
   devenv shell -- cargo fmt --check
   devenv shell -- cargo check --all-features --locked
   devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
   devenv shell -- cargo test --all-features --locked --lib all_recorded_ticket_sides_constrain_retained_closure_before_writes
   devenv shell -- cargo test --all-features --locked --lib child_fence
   devenv shell -- cargo test --all-features --locked --lib conflicting_advance_after_the_checkpoint_needs_a_deliberate_synchronization
   ```

6. Write the design recovery table into the execution ledger as required outcomes
   with test owners. Record what is source-confirmed versus still to be proven:
   partial checkout recovery, cross-journal admission, cancellation acknowledgement,
   wire-token read stability and local conflict reconstruction are **not executed
   facts**. Their test-first checkpoint is Tasks 4–7.
7. Add the baseline ticket comment. Commit `docs: record closure recovery baseline`
   after authorization. Stop for any changed assumption, unexplained baseline
   failure, or additional contract question before Task 1.

## Task 1: Honest Incompatible-Closure Classification

**Modify:** `src/repository/remote/{merge.rs,sync.rs,merge_tests.rs,sync_tests.rs}`;
`tests/ticket_closure/races.rs` (E1/E2 only).

1. Write failing unit cases for two distinct nonempty closure pairs in local/
   incoming, timestamp-only, author-only and ancestor-only disagreement. Controls:
   all open, one closed pair, all same pair, status-text differences, mixed conflict
   set. Use parsed `OffsetDateTime` equality and exact author text.
2. Amend E1/E2's expected **fresh** classification/result before the fix, retaining
   every no-write assertion. Use `try_resolve_fixture` for the newly expected
   error; do not make `resolve_fixture` globally swallow errors.
3. Run the exact affected unit filter and the exact SSH command above. Observe
   expected failures, not unrelated fixture failures or a zero-case pass.
4. Add an internal distinct-closure check to same-ID ticket classification before
   it returns eligible. Represent the unsupported invariant explicitly in
   `ConflictStructure`; let existing whole-set eligibility handle it. Do not alter
   `preserves_item_invariants` or external-integration validation.
5. Rerun affected cases plus `retained_candidate_cannot_bypass_all_side_closure_validation`
   and `mixed_canonical_and_code_conflict_is_whole_merge_external_only_without_writes`.
   Preserve the legacy retained-attempt contract even where fresh inspection now
   refuses earlier.
6. Record source/diff and counts; commit `fix: classify incompatible closures as external recovery`.

## Task 2: Child Mismatch Wording By Parent Kind

**Modify:** `src/repository/promotion/lifecycle_record.rs`;
add tests in existing `src/repository/remote/reservation_tests.rs`.

1. Add focused tests obtaining the inner `RepositoryError` for wrong operation,
   target, parent disappearance, changed binding/attempt, disallowed stage and
   cancellation. Cover closure context/primary roles and promotion controls.
   Assert the exact parent-specific fixed text and unchanged error kind; do not
   assume masked outer Display exposes it.
2. Run `devenv shell -- cargo test --all-features --locked --lib child_fence` red.
3. Make the fence mismatch helper take `LifecycleKind`, and pass `self.kind` at
   each call site, including the absent-parent closure. Preserve the completed
   authoritative handoff exception and all validation predicates.
4. Rerun the filter and
   `devenv shell -- cargo test --all-features --locked --lib quality1_current_authority_handoff_and_ordinary_synchronization_remain_valid`.
5. Comment and commit `fix: use lifecycle parent wording for child mismatches`.

## Task 3: Extract Transport-Neutral Integration Mechanics

**Create:** `src/repository/integration/{mod.rs,prepare.rs,artifacts.rs,validate.rs,resolution.rs}`,
with focused adjacent unit tests where behavior is not already covered.
**Modify:** `src/repository.rs` module declaration and relevant remote
`merge.rs`, `sync.rs`, `reservation.rs`, `state.rs` visibility/adapters (E8).

1. Map the existing clean/fast-forward/conflict install and resolution intent,
   path-write, candidate/ref, lock/metadata-release sequences before moving code.
   Record exact preconditions and ownership callback contracts in module docs.
2. Move only neutral preparation/validation/artifact mechanics. Keep remote
   reservation acquisition, windows, journal schema and transport orchestration
   in the remote adapter. Each helper must be called by the old path in this task;
   no unused future-local scaffold or broad lint suppression. Do not invent an
   extracted per-path initial-checkout journal: decision 6 retains conservative
   ambiguous-state stops for those coarser backend operations.
3. Add a test owner adapter that deliberately loses authority between preparation
   and mutation; verify no Git effect after loss. Prove memory preparation writes
   no persistent objects/refs/index and rejects unproven mutable paths.
4. Run these regression anchors, separately:

   ```sh
   devenv shell -- cargo test --all-features --locked --lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally
   devenv shell -- cargo test --all-features --locked --lib external_repair_exact_ordered_parent_clean_only
   devenv shell -- cargo test --all-features --locked --lib clean_merge_integration_fault_matrix_resumes_each_durable_transition_once
   devenv shell -- cargo test --all-features --locked --lib cancellation_with_a_pending_conflict_is_a_recoverable_stop
   ```

5. Run fmt/check/clippy. Obtain an independent extraction review of actual moved
   code and ownership sequencing before building local execution. Unexpected remote
   behavior changes stop the task; do not update old assertions to fit.
6. Comment and commit `refactor: share owned integration mechanics without transport`.

## Task 4: Local Journal, Observation And Reciprocal Fences

**Create:** `src/repository/local_sync/{mod.rs,record.rs,observe.rs}` and adjacent
`*_tests.rs` modules; new local observation DTO/schema.
**Modify:** `src/repository.rs`; `recovery.rs`; `coordination.rs` only if the
existing guard interface needs a neutral wrapper; `closure/{record,observe,checkpoint}.rs`;
`promotion/lifecycle_record.rs`; `remote/reservation.rs`; `registration_removal.rs`;
read DTO/status modules and E4/E7 inventory entries.

1. Write tests first for atomic parent/companion creation, every operation-ID
   collision family, malformed/missing companion evidence and schema migration.
   Create/read a new operation only through its real store API, not raw rows that
   skip admission. Assert no fallback with a directory in place of the registry.
2. Define typed records and exact schema audits, with FKs to the local envelope,
   version checks, owner epoch and generation. Add action/step parsing/status
   coverage without altering old strings or family values.
3. Write observation tests for both kinds, missing context, wrong branch, dirty
   target, primary committed-versus-uncommitted input and concurrent snapshot
   movement. Implement separate read sessions A/B and Git observations as specified,
   closing A before B so authority-only changes are visible. Cover semantic revision
   changes with unchanged Git, owner-only takeover, serialized token reopen and
   historical content reads; no transport, writes or persistent token issuance.
4. Write the admission matrix as tests before changing fences. Matching written
   closure at Closing is allowed; wrong item/kind/stage, promotion, unrelated
   local/remote owners are refused. Include retained/interrupted remote work across
   contexts and check its reacquisition before phase/epoch update, in both
   acquisition orders. Add reciprocal closure preview/checkpoint/
   integration refusal while this operation is unfinished.
5. Add new-operation guards to refresh takeover, reservation, operation enumeration,
   registration removal and rebuild validation. Pending conflict evidence cannot
   be treated as an index-only handoff. Keep remote written-closure publication
   guards unchanged.
6. Run `devenv shell -- cargo test --all-features --locked --lib local_sync::`,
   then the exact existing action/step inventory tests E4, `child_fence`, and the
   named closure-admission controls discovered in Task 0. No zero-test result counts.
7. **Design checkpoint:** record actual admission/namespace/refusal answers. Any
   conflict with the approved table stops for the owner before execution code.
8. Comment and commit `feat: persist and observe local context synchronization`.

## Task 5: Local Integration Executor, Test First

**Create:** `local_sync/{run.rs,recover.rs}` and adjacent tests.
**Modify:** local records/observation and shared integration adapter; add narrowly
named local failure points in `src/repository.rs` where existing ones do not fit.

1. Write `local_sync_required_outcomes` cases for both kinds: equal/contained,
   fast-forward, clean merge, stale observation and frozen-parent retry. Add fault
   cases after durable intent, checkout progress, ref transition and index handoff.
   Tests assert exact ordered parents, one ref effect/commit, primary/tracking refs
   unchanged and journal/request authority. They must be red before implementation.
2. Implement the domain executor under the repository lease using the design's
   state machine. Generate deterministic scope/target identity, persist intents,
   reconcile exact pre/post state, and refuse foreign state. Use configured identity
   only at commit; never write identity configuration or use transport.
3. Add configured-unreachable-remote and transport-spy negative controls. Reads
   of primary are committed input only. Dirty target refusal must preserve files,
   including untracked paths and symlink substitutions.
4. Run `devenv shell -- cargo test --all-features --locked --lib local_sync::`.
   Assert produced effects match each design row; same code can have different
   failure class after a durable effect. Do not hard-code ordinary code classes.
5. Record the partial required-outcome table: conflict/resolution rows are explicitly
   outstanding for Task 6. A completed row returning differently stops for a ruling.
6. Comment and commit `feat: integrate local primary into editing contexts`.

## Task 6: Canonical Conflict Resolution And Safe Recovery

**Create:** `local_sync/{conflict.rs,resolve.rs}` and adjacent tests.
**Modify:** local recovery/record and neutral integration adapter; guarded abandon
through `src/repository/bridges/abandon.rs`; appropriate mutation cancellation
helpers only when used by the following boundary task.

1. Add failing tests for all remaining required-outcome rows: conflict installation,
   whole-set eligibility, complete multi-path resolution, source byte preservation,
   source mismatch under same attempt, partial path writes, candidate/ref crashes,
   release failures, exact external-repair parents and irreversible closure pairs.
2. Implement local inspection/token validation and source reads from recorded blobs,
   never mutable current paths. Match token to re-enumerated entries; tampering,
   wrong operation, wrong generation, wrong mode and stale fingerprint refuse.
3. Implement complete preflight before any path write. Bind the resolution attempt
   and input digest durably, then use owned per-path/index/ref/metadata execution.
   Resume partial progress by attempt before fresh token validation. Missing bytes
   return original-request-required, not fabricated source or a fresh attempt.
4. Implement acknowledged cancellation and guarded abandonment per design. Test
   cancellation before effects, at conflict pending, after partial resolution,
   after integration and during index handoff. Race a new cancellation with explicit
   resume; no lost cancellation and no endless cancellation loop.
5. Run `devenv shell -- cargo test --all-features --locked --lib local_sync::`
   and the Task 3 shared-resolution regression anchors affected by extraction.
6. **Design checkpoint:** all domain required-outcome rows must now pass. Record
   actual effects and owner states after each injected failure. Any contradictory
   row stops for the owner; neither table nor test is rewritten to fit silently.
7. Comment and commit `feat: resolve and recover owned local context conflicts`.

## Task 7: Shared Reads, Mutation Bindings And Request Recovery

**Create:** `src/repository/mutation/bind/local_sync.rs`,
`src/repository/read/local_sync.rs`; `tests/mutation_replay/local_sync.rs`;
`tests/read_items/local_sync.rs`, registered by an explicit path module in the
existing `tests/read_items.rs` target; published local schemas/goldens.
**Modify:** library exports; mutation `mod.rs`, `command.rs`, `dto.rs`, `identity.rs`
only for required typed fields, `bind/mod.rs`, `outcome.rs`, `replay.rs`,
`bind/resume.rs`, `progress.rs`, `records.rs` and request settlement helpers as
required for local-control lookup, counters and atomic dispositions; read
module/DTO registration; E5–E7/E9 test inventories and the existing target modules.

1. Add public-API tests for the approved input structs and read signatures from the
   design. Prove missing/wrong kind/context refusal, read-only observation/inspection,
   explicit side-content output and private diagnostics/storage. Register new
   modules in existing targets; no Cargo target addition.
2. Add the two mutation variants only with executable bindings and conformance
   homes. Start parses observation before effects, hashes unchanged semantic input,
   and uses the Local family. Resolve targets the parent operation with its own
   request/attempt association; no new family and no confirmation machinery.
3. Extend `operation resume` with exact local-sync routing and owner-request fencing.
   Test starting and resolution requests independently across parent completion,
   lost settlement, request mismatch, competing resolve/resume and original-input
   requirements. A partial resolution's retry must not fail merely because its own
   writes changed the live conflict token.
   Advance the control row's execution-attempt fence atomically with F2
   `enter_request`, preserving stable-ID association/effects/cancellation counters.
   Intended parent scope/ID is recorded at acceptance, before linking; include
   unlinked contenders in terminal-parent/abandon dispositions, as the design table
   requires. Do not turn an old-attempt control row into missing evidence.
4. Make cancellation acknowledgement visible through the existing request API,
   preserving old commands' behavior. Explicit continuation consumes only the
   acknowledged generation, not a newer cancellation. Add crash/reopen tests.
5. Map the design effects and existing result codes. Add only new local typed DTOs
   and OperationAction/step enum members; global ResultCode and RecoveryActionKind
   remain unchanged. Existing front ends must compile without edits. Do not add
   `conflict.show` to the global recovery enum: desktop matches it exhaustively.
6. Add schema roots, enum coverage, command homes and success/refusal/partial/no-op
   goldens. Prove explicit content-read schemas cannot be mistaken for stored
   mutation schemas. Use schema validation and existing golden update helpers.
7. Run:

   ```sh
   devenv shell -- cargo test --all-features --locked --test mutation_replay local_sync
   devenv shell -- cargo test --all-features --locked --test read_items local_sync
   devenv shell -- cargo test --all-features --locked --test mutation_contract
   devenv shell -- cargo test --all-features --locked --test read_contract
   ```

   Run `every_command_of_this_target_keeps_the_rules_of_the_boundary` as one
   positional filter in each of `--test mutation_replay`, `--test mutation_confirmation`,
   `--test mutation_remote`, and `--test mutation_bridges`. They must still account
   for every old and new command; record a nonzero selected count in each target.
8. Run whole-tree fmt/check/clippy. Comment and commit
   `feat: expose local synchronization and recovery through library boundaries`.

## Task 8: Closure Acceptance And Adversarial Evidence

**Modify:** E3's two closure run tests; add tests beside them and in
`local_sync/*_tests.rs`, mutation replay and read-item modules.

1. Replace the hand-merge step in E3 with the new operation. The fixture must
   actually produce a checkpoint-merge refusal; inspect it, resolve canonical
   conflicts through the library, finish local sync, then explicitly resume closure.
   Keep original closure_oid/closed_at/closed_by and adopted-context assertions.
2. Test primary moving again after the first local sync: frozen sync completes its
   pair, closure may require another explicit sync, and no hidden fetch or automatic
   close occurs. Test no-remote and configured-remote repos separately.
3. Add cross-process lease/owner races, request input/ID collisions, healthy rebuild,
   lost database with retained Git artifacts, malformed companion schema, index
   failure, immutable closure race and foreign metadata preservation. Every claimed
   no-write/no-second-effect outcome compares files, index, refs, rows and authority.
4. Privacy tests positively plant source in committed/owned files while proving it
   absent from request/journal stores, error strings and progress output. Do not
   scan the discovery title index as though it were a request store.
5. Run targeted local-sync/read/replay filters and the two exact E3 test filters.
   Run the affected ticket-closure race harness and shared remote regression filters
   from Tasks 1/3. Record platform limitations and independent Part review findings.
6. Comment and commit `test: prove closure recovery through local synchronization`.

### Mandatory named cases added after round 1

These are NEW tests under `src/repository/local_sync/` unless the target column
names an existing integration target. Include them in their owning task, then
run their table's group filter in Task 8. A failure cannot be waived by source
inspection or a different passing case.

| NEW case / owner | Setup and required outcome | Target/filter |
| --- | --- | --- |
| `unrelated_histories_refuse_before_acceptance` / 5 | Orphan context/primary histories; recovery_required with typed reason, no owner/refs/files changed. | `--lib local_sync::` |
| `immutable_closure_disagreement_never_fences_the_repository` / 5 | Two incompatible pairs in prepared merge; no accepted request or pending owner; unrelated new work succeeds. | `--lib local_sync::` |
| `identity_missing_then_set_retries_the_same_local_intent` / 5/7 | Initial missing identity leaves no accepted work; set identity and repeat same ID. Also remove identity after acceptance but before candidate: retain S, set identity, resume S; journaled candidate identity never changes. | `--lib local_sync::`; `--test mutation_replay local_sync` |
| `accepted_resolution_observation_is_part_of_request_identity` / 7 | Partial R, change only top-level observation; request_mismatch with historical effects and unchanged attempt/files. | `--test mutation_replay local_sync` |
| `cancelled_partial_resolution_remains_accepted_and_resendable` / 7 | One path written, acknowledge cancel, reopen; R/S remain Accepted; U identifies R; exact R completes once. | `--test mutation_replay local_sync` |
| `new_cancellation_survives_acknowledgement_and_resume` / 7 | Coordinate issued-counter increment during acknowledgement/U takeover; newer counter stays pending and is honored. | `--test mutation_replay local_sync` |
| `artifact_free_abandon_settles_all_associated_requests_atomically` / 7 | R integrated/released, index pending; abandon atomically terminates Accepted S/R/U, preserves finished results, exact replay works and changed-ID input mismatches. | `--test mutation_replay local_sync` |
| `unlinked_accepted_resolution_cannot_write_after_attempt_loss` / 7 | Kill after acceptance before parent link, compete with same-ID caller; old attempt cannot write, exact retry safely links. | `--test mutation_replay local_sync` |
| `request_attempt_handover_preserves_resolution_and_cancellation` / 7 | Partial R; advance F2/control attempt then exit before domain takeover; another retry preserves effects/counters, old executor cannot settle, current owner finishes once. | `--test mutation_replay local_sync` |
| `unlinked_contender_observes_parent_completion_or_abandonment` / 7 | Accept R0 then pause; R1 completes or reaches artifact-free index-pending and abandons; R0 takes the specified terminal/no-effect disposition and never remains dangling. | `--test mutation_replay local_sync` |
| `retained_remote_and_local_admission_never_deadlock` / 4/8 | Remote interrupted retained work on same/other context versus local start, both acquisition orders; refused owner changes no phase/epoch. | `--lib local_sync::` |
| `authority_only_change_is_seen_by_fresh_read_session` / 4/7 | Commit semantic revision/registration change between A and B while Git stays equal; external_change; owner-only takeover and historical token cases separately pinned. | `--test read_items local_sync` |
| `journal_permission_failure_precedes_git_effects` / 4/8 | Inject typed write PermissionDenied before atomic journal creation; index_unavailable, no Git effect or half-bound authority. | `--lib local_sync::` |
| `side_read_permission_failure_is_read_only` / 7 | Deny recorded blob/path read; repository_inaccessible with no side bytes or mutation. | `--test read_items local_sync` |
| `permission_loss_after_one_resolution_path_preserves_recovery` / 6/8 | Deny the next owned write; partial repository_inaccessible, retained R/evidence; restore permission and exact R completes once. | `--lib local_sync::` |
| `no_drop_backend_ambiguity_preserves_until_operator_proof` / 5/8 | Child exits without Drop inside checkout/conflict/ref-log/metadata windows; ambiguous artifacts persist. Invalid repair still refuses; exact baseline or validated result permits retry with no repeated commit. | `--lib local_sync::` |
| `no_drop_owned_metadata_retirement_finishes_automatically` / 6/8 | Exit after one owned metadata/sentinel member is removed; observe absent member, retire verified owned remainder, finish with no operator repair or second commit. Foreign metadata remains refused. | `--lib local_sync::` |
| `lost_journal_distinguishes_conflict_checkout_and_completed_state` / 8 | Separate fixtures for conflict metadata, dirty partial checkout, private lock artifacts, and clean committed result after database loss; refused unsafe adoption, no invented request identity, fresh clean observation allowed only in final case. | `--test mutation_replay local_sync` |

Use fixture-only barriers around the exact durable boundaries; do not rely on
sleep timing. The no-Drop child uses the current test executable and isolated
environment/data, exiting at the requested barrier without unwinding; the parent
inspects preserved artifacts. No developer process is terminated. A returned
injected error alone does not satisfy the no-Drop case. Permission injections
are typed I/O errors at the actual store/path seam; Unix real-permission controls
supplement them, without claiming native Windows ACL evidence.

## Task 9: Documentation, Verification And Review-Ready Handoff

1. Amend the documented old conflict classification, local recovery instructions,
   API audit and Wave prerequisite in the exact places the design lists. Preserve
   historical execution results; mark front-end adapters as downstream, not delivered.
2. Recheck every new exported type, DTO, command identity, schema, recovery outcome,
   named edit and test-case reference against actual code. No placeholder or
   “Task 0 will decide policy” survives implementation.
3. Run the final relevant gates, once on the final Rust tree:

   ```sh
   devenv shell -- cargo fmt --check
   devenv shell -- cargo check --all-features --locked
   devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
   devenv shell -- cargo test --all-features --locked --lib local_sync::
   devenv shell -- cargo test --all-features --locked --lib child_fence
   devenv shell -- cargo test --all-features --locked --lib all_recorded_ticket_sides_constrain_retained_closure_before_writes
   devenv shell -- cargo test --all-features --locked --test mutation_replay local_sync
   devenv shell -- cargo test --all-features --locked --test read_items local_sync
   devenv shell -- cargo test --all-features --locked --test mutation_contract
   devenv shell -- cargo test --all-features --locked --test read_contract
   ```

   Also run changed E3 tests, the Task 1 race harness selection and the Task 3
   extraction/remote regression anchors, with exact selectors recorded from Task 0.
   This is targeted verification, not `cargo test` over the full package. Stop
   unexplained failures; do not repeat a failing suite until it happens to pass.
4. Fetch/merge main as needed, run merge-tree and obtain independent spec and code
   quality review of the **whole** branch from its recorded base. Include an
   adversarial review of ownership, partial resolution, cancellation and cache loss.
5. Resolve findings and reverify affected tests. Record all counts, named exceptions,
   native limitations and review ranges in the execution ledger/ticket comment.
6. Mark review-ready only after required evidence and reviews. Commit/push within
   authorization. Ask separately for PR/closure/merge as applicable; close before
   merge only after owner acceptance. Do not remove the downstream gate prematurely.

## Plan Self-Review

| Acceptance/ruling | Task and proof |
| --- | --- |
| A1, R-1 | 1: all-side classification + real two-clone race; legacy retained attempts unchanged. |
| A2 | 2: every child fence reason/role + promotion controls. |
| A3–A4, R-4/R-5 | 4–5: observation, graph cases, identity and no transport/primary mutation. |
| A5, R-4/R-6 | 6–7: complete canonical resolution, wire tokens/content privacy and original request. |
| A6–A7, R-2/R-6/R-7/R-8 | 4–8: required-outcome checkpoints, fault/ownership/request/effect matrix. |
| A8–A9, R-3 | 4/8: admission and reciprocal fences; real closure resume without hand merge. |
| A10 | 6–8: capability/path validation, explicit content versus persisted output, positive privacy controls. |
| A11 | 4/6/8: schema/registration/rebuild/lost journal/abandon cases. |
| A12, R-9 | 7: closed boundary inventories, schemas/goldens, progress/cancellation semantics. |
| A13 | 0/3/9: baseline, extraction review, final targeted gates and whole-branch review. |
| A14 | 5/8: pre-effect immutable-conflict refusal and explicit backend-ambiguity/operator-proof cases. |
| R-10 | 3: old remote tests pass unchanged around neutral extraction. |
| E1–E9 | Inventory at 0; task-owned changes only; final diff audit at 9. |

## Verification Limitations And Open Review Work

No builds/tests ran during planning. Task 0 subsequently verified source,
extraction seams and exact selectors; Tasks 1–8 implemented the production library
boundary and its recovery evidence. The API audit records the actual public names,
schema shapes and recovery semantics. Task 8's 25-outcome and 18-case tables retain
the executed Linux proof. Task 9 final gates and independent whole-branch reviews
remain pending; no unresolved Task 0 policy placeholder is current work.

The following is the historical planning review record:
Round 1 found 3 Blocking/3 Important/1 Minor source findings and
1 Blocking/6 Important/1 Minor contract findings. The revised explicit protocols,
owner decisions 5–6 and targeted cases address them. Round 2 found 0 Blocking,
3 Important and 1 Minor across both reviews; the bounded handover, contender,
retirement and DTO revisions resolved them. Round 3 source and contract reviewers
both approved with zero findings, reviewing these uncommitted drafts against
`0cbeab0`. Only review/approval-status bookkeeping changed afterward. Detailed
owner approval and implementation authorization are recorded in the start comment.
