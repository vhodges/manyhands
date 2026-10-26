---
title: "Closure Recovery Gate Execution Ledger"
date: 2026-10-25
status: "waiting"
author: "GPT-6 Astra"
manyhands_managed: true
manyhands_kind: document
id: "01M5TZ0E74VAJA33WV8M3GYMWW"
---

# Closure Recovery Gate Execution Ledger

## Authority, approval and task state

Ticket: `01M57SKVF6TQ3QM6AXJ35JGGSY`. Sources, in order:
[contract](../Cycles/wave-03-closure-recovery-gate.md),
[design](2026-10-25-closure-recovery-gate-design.md),
[implementation plan](2026-10-25-closure-recovery-gate-implementation.md).

Owner approval supplied by the controller: **“Every is approved. Proceed using
subagent driven development”**. This answered approval asks for all three
documents, detailed rulings, E1–E9 existing-test edits, subagent execution,
implementation and ticket-branch commits/pushes. Start comment
`01M5TZ0E62RMQPKBGRPX3CX92E` records it at `2026-10-26T02:00:39Z` (UTC clock).
The historical planning comment `01M5TM3G781C6TXKA5BY50QPBC` is unchanged.
PR, closure, merge, cleanup and native CI require separate authorization.

Task 0 completed with no production/test/schema edits. Controller owns independent
review. The owner answered both earlier named-edit questions with “yes”, explicitly
approving E4's companion local-action partition and E9's repair DTO refusal arm;
comment `01M5V0RYTAKJE3F155FH4QTP53` records that ruling. Tasks 1–3 are implemented
and independently approved. Current status is **waiting**: Task 4 SPEC requested
changes on five source-verified findings, and the newly identified key-file scope
requires an owner answer to question `01M5V7SPS9W51SSF5XYTJR5J08`. No fresh
implementation proceeds while waiting. Tasks 4/7 retain ownership of E4/E9 code.

| Work | Evidence/state |
| --- | --- |
| Approval recording | `51a0e96da11f8bc1f2f0689e72c88ed99470c96a`, `docs: approve closure recovery gate implementation`; pushed to ticket branch. |
| Task 0 | Source/selector inventory and required baseline completed; checkpoint `9039e65`, comment `01M5TZXJZ2348Z8GRMAQ7BSHPR`. Both independent reviews approved; owner questions resolved below. |
| Task 1 | Implemented at `76a1ff547a20dc3b112d5cc9417ef29e287f4b3a`, after approval checkpoint `9b7879e3b03a888c8f6ab62608b0edede65a6982`; comment `01M5V15ZJFXBEMM7KQ4G18Y3RC`. Controller SPEC and QUALITY both approved with zero findings; sessions below. |
| Task 2 | Implemented at `1f25f56e6ce8c69f0f0b53c8939fe4443aa46c07`, from `76a1ff547a20dc3b112d5cc9417ef29e287f4b3a`; comment `01M5V1QEB9WYETHHG1H765DVH7`. Controller SPEC and QUALITY approved with zero findings; sessions below. |
| Task 3 | Implemented at `87431fcffe18dd7125bf99962795129c9d5d578b`, review-range checkpoint `37bd2c36ace84e56920d4cf15f55539d63c7d1c7`. Controller SPEC and QUALITY approved the full `1f25f56..37bd2c3` range, zero findings; sessions below. |
| Task 4 | Initial implementation `00a75e7742b6a9cc9284c91d2e18c9d5aec37599`, from `37bd2c36ace84e56920d4cf15f55539d63c7d1c7`; checkpoint comment `01M5V6K8THJGFZ8FFSC4M87PD7`. SPEC `ses_e898c0189ffebSYYicNzmsZaXQ`: REQUEST CHANGES, five unresolved findings (1 P1, 4 P2). **Not approved; waiting for scoped owner permission**, question `01M5V7SPS9W51SSF5XYTJR5J08`. |
| Tasks 5–9 | Not started. Executor, resolution and request-boundary outcomes remain unproven. |

All new commits use the requested trailer
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Preflight and overlapping work

- Only working checkout/branch:
  `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M57SKVF6TQ3QM6AXJ35JGGSY`,
  `manyhands/ticket/01M57SKVF6TQ3QM6AXJ35JGGSY`.
- Controller reported clean main and verified worktree. This worker independently
  checked `git worktree list --porcelain`, status, diff, recent log and branch refs
  from the ticket checkout. Initial untracked changes were precisely the three
  supplied planning documents and historical planning comment; no tracked edits.
- Initial fetch/merge: main `0464c42dae19a6dde000c7e7660e82762a41fada`;
  HEAD before **and** after `0cbeab084d252e3823d317a468eee1f29c92c8ce`.
  `git merge origin/main` said already up to date; ancestry exit 0. Current
  `origin/main:AGENTS.md` was read. No merge conflicts or rebase.
- Checkpoint fetch/merge: same main; HEAD before/after
  `51a0e96da11f8bc1f2f0689e72c88ed99470c96a`, already up to date; ancestry exit 0.
  `git merge-tree --write-tree HEAD origin/main` exited 0, tree
  `d2ded4d66aa1bef6fc10584fee6bc184f404bb21`.
- `gh pr view manyhands/ticket/01M57SKVF6TQ3QM6AXJ35JGGSY --json
  number,state,url,comments,reviews,commits` returned “no pull requests found”.
  Ticket was open, has no deps or closed_at; start changed it to implementing
  through the working-on-a-ticket exception. Baseline question changes it to waiting.
- The sole earlier comment is the historical planning record. No newer decision
  or gate implementation was found. Searches for the new local API/companion
  names and `mod local_sync` in `src` returned no matches.
- `git branch -avv` and `git log --all --oneline origin/main.. -- src tests schemas
  .github` showed no unmerged implementation commit in these areas, only the gate
  merge `189bb36` and D3 merge `98f49b7`. Active D3 branch
  `01M4CC0VN0MFTGKQ4HGTTC5H63` is at `9dd2bec` (planning review). Other registered
  downstream branches remain present; this is a committed-ref check, not a claim
  about another worker's uncommitted state.
- Sibling cleanup branch `manyhands/ticket/01M57SBA7MKR9BZR7PYECSJ05G` is at
  `c5f1fb9b630e86b916147da34f4209feec6dc267`, tracking the same remote tip. Its
  ticket was read via `git show` (it is not in this ticket checkout). Its diff
  against main adds only its ticket. Scope: save_ticket wrong-kind refusal before
  provisioning; deduplicate terminal/busy-supersede record logic; strengthen raw
  privacy negative controls. None is implemented here. Shared-file overlap in
  lifecycle_record.rs/closure record code is real, but this gate owns only the
  approved parent-kind fence and local-sync admission changes. No sibling worktree
  was changed.
- Branch diff already includes the earlier Wave 03 downstream-gate edit and ticket
  creation; these predate this worker. Task 0 did not alter those gate rules.

## Facts 1–24 rechecked against source

Paths below abbreviate `src/repository/` as `repo/`. Line numbers refer to the
unchanged Rust tree at `0cbeab0` / `51a0e96`. “Confirmed” means source inspection,
unless the baseline table explicitly records execution.

| Fact | Rechecked source and conclusion |
| --- | --- |
| 1 | `repo/remote/merge.rs:50–69`: `classify_integration` takes explicit OIDs and callbacks; equal, contained, fast-forward, merge-required, unrelated are distinct. Confirmed. |
| 2 | `repo/remote/sync.rs:719–814`: `prepare_clean_merge` opens a handle with priority-1000 mempack; `prepare_clean_merge_in` requires that caller setup; `import_prepared_tree` is the separate persistent ODB/tree write. Confirmed; not a new-local recovery test. |
| 3 | `repo/mutation/bind/sync.rs:1–27,100–103`: item/repo sync concern publication remote and set refuse_divergence=true. Confirmed. |
| 4 | `repo/remote/sync.rs:7577–7683`: bind/replay/refresh use tagged refresh identity and PublishPending; `repo/recovery.rs:648–655,758–768` authorizes refresh only. No dedicated local-primary-into-context integration executor exists here. |
| 5 | `repo/recovery.rs:497–537`: local envelope lacks integration evidence. `47–198` supplies exact-shape-audited comment companion precedent and FK to operation_records. Confirmed. |
| 6 | `repo/remote/state.rs:721–932`: merge windows/steps/attempts/artifacts FK to remote rows. `resolve_synchronization` at `sync.rs:5593–6215` requires publication remote/ref plan and remote conflict reservation. Cannot put local evidence in those rows. |
| 7 | `repo/coordination.rs:14–15,84–95,216–258`: common-directory exclusive file lease, 250ms limit, 5ms retries; SQL identity alone is insufficient. Confirmed. |
| 8 | `src/repository.rs:3314–3337`: registry-directory special case returns record id 0 without journal. New durable local integration must refuse it before effects. |
| 9 | `repo/recovery.rs:633–643,1334–1393`: pending local rows fence work. `repo/remote/reservation.rs:238–258` checks that fence in owned(). Lifecycle closure is separate. |
| 10 | `repo/closure/record.rs:741–760`, `remote/reservation.rs:310–323`: written closure context cannot be remotely published; reciprocal active remote synchronization blocks checkpoint. Preserve. |
| 11 | `repo/closure/checkpoint.rs:389–498,698–799`: attached descendant with exact checkpoint closure fields can be adopted. `closure/record.rs:610–645`: Closing can bind adoption, Integrating only accepts exact recorded pair. |
| 12 | `repo/closure/run_tests.rs:749–774` calls private `synchronize_context_by_hand` (`281–324`). Baseline test passes with that manual fixture helper, not with a local API. |
| 13 | `repo/remote/sync.rs:4430–4545`: checks regular UTF-8 same path, canonical kinds/IDs, but no closure-pair comparison. `remote/merge.rs:171–185` implements whole-set eligibility. E1 baseline still expects eligible. |
| 14 | `repo/remote/sync.rs:4556–4617,4742–4832`: submitted and external candidate validation preserves all three sides' immutable closure fields. `src/canonical.rs:106–118`: timestamp is Option<OffsetDateTime>, author Option<String>. Do not replace with raw-YAML comparison. |
| 15 | `repo/promotion/lifecycle_record.rs:508–594`: ChildExecution stores kind, but private fence_mismatch uses DocumentPromotion; five refusal call sites including absent parent. Existing inner mismatch kind stays OperationMismatch. |
| 16 | `repo/mutation/records.rs:102–109`: SQL closed family set local/remote/key_material. Local companion avoids fourth family, not new action/step mapping work. |
| 17 | `repo/mutation/bind/resume.rs:339–366`: found_local is a **private free function**, not an associated OperationResumeBinding method; handles final/hand-off/own-request cases. `bridges/abandon.rs:428–518` closes row/releases request without Git cleanup. Design's shorthand is not an existing local-sync method. |
| 18 | `repo/recovery.rs:765–768,1377–1380,1406–1450`: same-ID refresh may borrow non-refresh local work. Both begin and pre-effect refresh admission need local-sync-specific proof before index-only takeover. |
| 19 | `remote/merge.rs:75–133,188–233`: opaque typed capabilities, private fields, redacted Debug, no wire codec. `remote/sync.rs:5051–5131`: immutable stage reads; current=None; existing remote read takes a lease. New local dual-session wire read remains to implement. |
| 20 | `repo/mutation/identity.rs:117–247`: public typed byte input to length-prefixed BLAKE3 intent digest; stored digest is not stored source. Keep new mutation DTO content-free. |
| 21 | `src/results.rs:282–288,302–366`: Partial maps Incomplete; writes, committed checkpoints and complete integration are durable. A commit OID alone proves no made commit. |
| 22 | `src/results.rs:609–637`: existing change/recovery/original-input/identity/dirty/conflicted codes. MergeRequired is explicitly local/remote history divergence. No new global result code. |
| 23 | `mutation/command.rs`, `tests/mutation_contract.rs:1494–1705`, `tests/support/conformance.rs:59–90,354–377`, `tests/read_contract.rs:703–1035`: closed inventories confirmed. **E4/E9 omitted paths found below**; completeness assumption is not confirmed. |
| 24 | `.github/workflows/build.yml:114–115`: existing library/read_items/read_contract/mutation_replay/mutation_contract/mutation_confirmation/mutation_remote/mutation_bridges/remote_merge_recovery/ticket_closure targets already run natively. No new target needed; no native execution claim. |

## Function boundaries and extraction map

- Remote `classify_integration`, `conflict_eligibility`, ConflictCandidate/Structure
  are `pub(super)` inside private remote merge module. Preparation/import types
  and functions are `pub(in super::super)` (repository-visible). These are internal
  extraction seams, not public front-end APIs.
- `RepositoryService::{classify_conflict_entry,preserves_item_invariants,
  validates_resolution_sides,validates_prospective_context,validates_context_sources,
  validates_external_integration}` are private associated functions. Existing
  `resolve_synchronization`, `inspect_synchronization_recovery` and
  `read_synchronization_conflict` are public service methods with remote-bound
  opaque types. New local methods/input structs are absent, not aliases.
- ChildExecution and its validate_request/validate methods are repository-visible;
  fence_mismatch is private. ParentFacts::read dispatches by LifecycleKind.
  Authoritative child completion bypasses stage/cancellation only after exact
  item/expectation/attempt/child validation. Preserve that ordering.
- Local migration/begin/pending/refresh/held-by-other-journal helpers are
  `pub(super)`; `begin_or_reconcile` and `in_the_way` are private. LocalStep is a
  pub enum within recovery, also available through the hidden request-store test
  surface. Its parse/is_checkpoint/continued matches and repository.rs's
  advance_lifecycle match are production exhaustiveness obligations.
- `Accepted::read_journal_row` and `recorded_effects` are crate-visible. The former
  returns Absent for family=None; the latter stops at absent journal. New R/U
  control lookup must precede both shortcuts. `enter_request` is doc-hidden public
  and increments request attempt inside write_records; no control fence exists yet.
- Public `show_request` reads stored state only. Public `cancel_request` uses
  stops_at_safe_points, current boolean request cancellation, and remote final
  cancellation for acknowledgement. request_cancellation/cancellation_requested/
  take_cancellation are mutation-module-private (`pub(super)`). New local counters
  are scoped additions, not a blanket rewrite of old commands.
- Public `abandonable_operation`/`abandon_operation` presently inspect local row,
  refuse other journals, acquire bootstrap/repository/cache guards, then release
  operation owner and abandon row in one immediate transaction. No existing
  artifact proof or all-associated-local-control settlement may be assumed.
- Remote conflict reacquisition's production entry is
  `reacquire_synchronization_conflict_in_window` (`reservation.rs:1907–1964`);
  unsuffixed entry is cfg(test), window-zero wrapper. The transaction currently
  checks target/generation/conflict/other remote owners then raises epoch/phase,
  without checking pending local rows first. The approved reciprocal guard is
  essential; ordinary owned()'s later check cannot prevent double admission.
- Resolution sequence, read in `sync.rs:5593–6215`: retained-candidate reconciliation
  precedes fresh inspection; complete token/immutable/prospective/path preflight
  precedes attempt; remote reacquisition then Git lease/revalidation; immutable
  attempt/path intents; owned index lock/ref baseline; all-path validation then
  per-path digest-guarded writes and observation; candidate creation/recording;
  index persist and ref proof/CAS; lease reacquisition and final proof; checkpoint
  observation; owned metadata/sentinel retirement; final reservation release.
  This is Task 3's adapter boundary, not proof of a generic recoverable checkout.
  Private ResolutionIndexLock and ref-log artifacts remain source-private.
- `prepare_clean_merge_in` alone does not install conflicts (returns Conflict).
  Task 3 must keep memory preparation separate from ODB import, initial libgit2
  checkout/conflict installation, ref transition and metadata release. Exact
  extracted signatures/SQL manifests remain implementation details, constrained
  by approved outcomes and regression anchors; no per-path initial-checkout
  journal may be invented.

## Reference inventory and E1–E9 reconciliation

Literal search at baseline, including production and tests:

```sh
rg -n 'Command::ALL|OperationAction::ALL|\bLocalStep\b|\bHOMES\b|\bDATA_ROOTS\b|contract_enumerations|MutationDataDto|RecoveryActionKind|cancel_requested|require_no_pending_local|held_by_another_journal|takeover|take_over' src tests schemas .github --stats
```

Result: **842 occurrences, 812 matching lines, 111 files**, 956 files searched.
Counts include declarations, comments, imports and uses; they are not 842 edit
sites. Appendix lists all hit files by grouping. This exact query plus recorded
SHA preserves the line-level inventory independently of session output. Refresh
takeover additionally requires semantic inspection of `begin_or_reconcile`,
`in_the_way`, `require_refresh_can_begin`, `owes_only_its_hand_off` and found_local:
their permissive conditions need not contain the word takeover.

| Reference | Production obligation / old expectation |
| --- | --- |
| Command::ALL | `mutation/mod.rs:557` hidden bound_commands projects every command; macro in command.rs defines ALL. Test uses `command.rs:485,526,547,563,587,616,636,652`; `outcome_tests.rs:1374,1406,1537,1651,1819,2259`. E5 authorizes two exact tests, not arbitrary recovery expectation changes. Other loops must still pass unchanged. |
| OperationAction::ALL | Production DTO replay parser `mutation/dto.rs:758` finds stored action. Tests `read/status_tests.rs:80,84,305` split first 13 local actions; read_contract.rs:915 publishes all actions. E4's named test covers line 305, but not the second test at 79–91. |
| LocalStep | Declaration/parse and exhaustive checkpoint/continued semantics in recovery.rs:848–1011; local-row evidence/hand-off derivation; repository.rs:3465–3493 durable-write classification; registration_removal.rs:200 shape validation; repair/repo/remote binding effect derivation. E4 test at recovery.rs:2461–2511 enumerates 21 stored names. Preserve existing names and flags. |
| HOMES | `tests/support/conformance.rs:59–90` is 30 commands; run_part at 354–377 enforces exact full set and target rows. E6 adds both commands, E9 adds replay rows. Four target wrappers must retain nonzero selection. |
| DATA_ROOTS | `tests/mutation_contract.rs:1510–1557` has 46 data roots; reachability at 1621 and unique-golden-data ownership at 1665 must grow under E6. No root-only exemption. |
| contract_enumerations | `tests/read_contract.rs:703–1003`, asserted at 1018–1035; all schema enums tied to Rust and one schema-version exception. LOCAL_ACTIONS at 687–701 independently enumerates 13 local actions for abandon preview/output. E7 explicitly includes it through enum coverage. |
| MutationDataDto | Public untagged enum (`mutation/dto.rs:540`); replay decoding and authority matching are production exhaustive paths. Six explicit helpers in support/mutation.rs at 596/611/730/745/760/775 are E9. **repair_data at support/repair.rs:333–347 is seventh, unlisted, exhaustive.** Other references are imports, signatures, constructors, if-let or fallback matches; new content-read DTO must not become stored mutation data. |
| RecoveryActionKind | `src/results.rs:369–428` macro/serialization/argument contract; mutation outcomes and read errors construct registered actions. Desktop `model/operations.rs:529–563` exhaustively matches all 11 variants for controls/labels. Test registries: results_tests, read_contract and support/golden. Preserve enum; do not add conflict.show or front-end arms. |
| cancel_requested | Existing boolean in mutation request records; separate remote and lifecycle records. mutation records.rs:824–940 and progress.rs:366–488 show attempt/cancel APIs; reenter/execute/bind sync/resume and parent/remote safe points consume it. Existing trigger tests cover those old booleans. New local issued/acknowledged counters must preserve those old behaviors and stable request evidence. |
| require_no_pending_local | Definition recovery.rs:633. Production callers remote/reservation.rs:257,913,1146,1393,2440 and remote/poll.rs:1294. Tests comment_publication_tests.rs:1417,1446 and remote/reservation_tests.rs:1604. Missing from conflict reacquisition before epoch transition; add only approved reciprocal guard. |
| held_by_another_journal | Definition recovery.rs:1777; callers bridges/abandon.rs:391,466 and mutation/outcome.rs:1330. Six existing recovery test uses at 2714–2743. Checks remote/key/key-registry journals globally; local companion remains local but needs guarded abandonment and collision validation. |
| refresh takeover | production recovery.rs begin/matching predicates and require_refresh_can_begin, mutation/resume found_local, lifecycle local-identity fences; tests document_promotion/quality.rs refresh_takeover, remote/sync_tests local takeover/cancellation, mutation_remote ownership handover, closure adopted checkpoint controls. New unresolved local conflicts must never be reduced to index-only work. |

E1 exact function/selector verified below. E2 is the eight-row immutable-closure
test, with no-write/ref/index/attempt-count assertions to preserve. E3's two
functions were read; their helper is also used by
never_abandons_with_an_adopted_descendant, which is not an E3 semantic edit.
Do not globally rewrite the helper to change that control's behavior.

E7 verified actual paths: `schemas/v1/operation.schema.json` (20 actions),
`operation_abandon_preview.schema.json` and `operation_mutation.schema.json`
(13 local actions plus null), and read_contract enum coverage. Schema descriptions
of old abandonment currently describe deleting the starting request; Task 9 must
accurately distinguish the new local-specific terminalization while retaining old
semantics. No global recovery enum/schema expansion is authorized.

E9 verified actual registries `tests/support/golden.rs::CASES`,
`tests/support/mutation_golden.rs::{CASES,PROGRESS_CASES}` and
`tests/mutation_replay/conformance.rs` chaining all command rows. Golden helpers
use `MANYHANDS_UPDATE_GOLDEN`, reject CI updates and validate registered actions.
No existing golden was regenerated. New fixtures/roots must have executable owners.

### Historical Task 0 owner questions (resolved before Task 1)

1. **E9 addition:** approve an explicit new-local-variant refusal arm in
   `tests/support/repair.rs::repair_data`, retaining its existing Repair success
   and every old rejection? Recommended: add this exact seventh helper to E9.
   The closed match has no wildcard; adding MutationDataDto's required variant
   otherwise cannot compile all targets. This is source-confirmed, not a baseline
   compiler failure (the variant does not exist yet).
2. **E4 clarification/addition:** approve updating the local-action split in
   `repo/read/status_tests.rs::a_local_action_is_one_the_recovery_store_writes`
   (`80,84`) alongside the already named every_action test (`305`)? Recommended:
   explicitly include this companion inventory test. Inserting the new local
   action before/after the old 13 makes one side of the old split incorrect.
   E4 says “update exact inventory boundaries”, but names only the other test;
   this worker does not silently broaden that scope.

No extra recovery-policy question was discovered. A Task 7 watchpoint, not a
proven additional edit: command.rs:546's only_a_synchronization_and_a_resume_can_finish_as_a_synchronization
relates stops_at_safe_points to finishes_as_a_synchronization. The latter controls
old remote replay guidance. Preserve its assertion unless implementation proves
an additional semantic edit is necessary, then return for approval. Likewise the
six outcome_tests loops are unchanged regression contracts, not E5 exemptions.

## Test names, selection and executed baseline

All Rust commands ran from the ticket worktree through Devenv, on the unchanged
Rust tree. No full suite or native workflow was run.

| Command | Observed result |
| --- | --- |
| `devenv shell -- cargo fmt --check` | Exit 0. |
| `devenv shell -- cargo check --all-features --locked` | First invocation exceeded tool's 120000ms during dependency compilation, no compiler error reported. Same command retried with 1200000ms limit: exit 0, Finished dev in 19.31s. Timeout is not counted as a pass. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Exit 0, Finished dev in 1m43s. |
| `devenv shell -- cargo test --all-features --locked --lib all_recorded_ticket_sides_constrain_retained_closure_before_writes` | 1 passed, 0 failed/ignored, 1269 filtered; 2.53s test execution (first test build 4m34s). |
| `devenv shell -- cargo test --all-features --locked --lib child_fence` | 4 passed, 0 failed/ignored, 1266 filtered; 0.32s. |
| `devenv shell -- cargo test --all-features --locked --lib conflicting_advance_after_the_checkpoint_needs_a_deliberate_synchronization` | 1 passed, 0 failed/ignored, 1269 filtered; 0.65s. |
| `devenv shell -- cargo test --all-features --locked --test ticket_closure true_race_distinct_identities_needs_external_recovery -- --exact` | Exit 0; exact named case ok; **1 SSH cases passed**. Build 37.45s. This is the old EligibleCanonical/ValidationFailed assertion, not Task 1 success. |

Total selected baseline: **6 library tests + 1 SSH case**, zero failures and no
zero-selection acceptance. No baseline Rust changes were needed. One optional
inventory aggregation attempt found python3 unavailable; direct rg counts were
used instead, with no environment/configuration change.

`tests/support/ssh_harness.rs:97–133` compares exact registered case name with
--exact, otherwise substring; unrecognized non-option filter selects zero and
still exits successfully. E1 registration at `tests/ticket_closure/races.rs:5–10`
maps the long selector above to private true_race. Its output count is required.
`Pair::new_with_remote` at `tests/ticket_closure.rs:123–124` starts SSH before
testing the remote flag. New no-network local acceptance must use isolated local
unit fixtures, not Pair(false). closure/run_tests::Fixture is such an existing
local fixture; its fake lifecycle IO is not proof of new local transport behavior.

Actual unit module names (file names are not always module names):

| Purpose | Full unit name or module + function filter (one positional filter) |
| --- | --- |
| E2 | `repository::remote::sync::tests::all_recorded_ticket_sides_constrain_retained_closure_before_writes` |
| Task 1 controls | `repository::remote::sync::tests::{retained_candidate_cannot_bypass_all_side_closure_validation,mixed_canonical_and_code_conflict_is_whole_merge_external_only_without_writes}`; run separately. |
| Task 1 helpers | private `resolve_fixture` calls try_resolve_fixture(...).unwrap(); private try_resolve_fixture returns Result. Use the latter for new external-only errors; do not weaken the former. |
| Task 2 filter | `repository::remote::reservation::tests::child_fence_accepts_ticket_target_only_for_closure_parent`, `child_fence_refuses_document_target_for_closure_parent`, `child_fence_admits_closure_primary_only_while_integrated_or_publication_pending`, `child_fence_refuses_unreserved_closure_children_after_cancellation` (4 selected today). |
| Task 2 control | `repository::promotion::run_tests::quality1_current_authority_handoff_and_ordinary_synchronization_remain_valid` |
| Task 3 anchors | `repository::remote::sync::tests::public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally`; `external_repair_exact_ordered_parent_clean_only`; `clean_merge_integration_fault_matrix_resumes_each_durable_transition_once`; `cancellation_with_a_pending_conflict_is_a_recoverable_stop`. All source-read, not executed in Task 0. |
| E3 | `repository::closure::run_tests::conflicting_advance_after_the_checkpoint_needs_a_deliberate_synchronization`; `adopted_context_is_merged_reconciled_after_f4_and_retired`. Only first executed here. |
| E4 | `repository::read::status::tests::every_action_the_recovery_store_writes_is_one_the_read_knows`; `repository::recovery::abandoned_row_tests::every_step_is_stored_under_its_own_name_and_read_back` (module corrected by Task 4 execution); additional inventory question above. |
| E5 | `repository::mutation::command::tests::only_a_synchronization_stops_at_safe_points_and_none_needs_confirmation`; `only_a_command_whose_work_no_journal_records_has_no_operation`. |
| E6/E7 | mutation_contract: every_bound_command_has_a_golden, every_schema_is_reachable_from_a_published_root, every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root; read_contract: published_enumerations_list_exactly_the_registered_values. |
| Task 7 conformance | `every_command_of_this_target_keeps_the_rules_of_the_boundary` in each existing target mutation_replay, mutation_confirmation, mutation_bridges (module conformance), mutation_remote (SSH registered name matches function). Four separate commands; remote's host rows are separately homed by run_part. |

Closure admission regression selectors for Task 4 (source locations confirmed):
`repository::closure::observe::tests::{local_operation_pending,pending_local_operation_blocks,
comment_publication_pending,pending_comment_publication_blocks}`;
`repository::closure::run_tests::{a_written_checkpoint_refuses_a_manual_context_synchronization,
an_active_manual_context_synchronization_holds_off_the_checkpoint,
an_interrupted_manual_context_synchronization_cannot_restart_after_the_checkpoint,
a_manual_context_synchronization_is_refused_while_publication_is_pending}`;
`repository::closure::checkpoint::tests::{adopt_written_closure_only_while_closing,
reconcile_adoptable_descendant_is_checkpoint_written}`. Run each relevant full
name as one filter, never brace syntax in Cargo. Remote reservation controls:
`sync_authoritative_same_id_refresh_replay_allows_only_exact_refresh_identity`
and `sync_incomplete_action_cannot_coexist_with_a_same_id_local_refresh` under
`repository::remote::reservation::tests`.

## Required outcomes: test owners, not executed facts

Every row below is an approved **required outcome**, transcribed from the design
recovery table. None is proved for the new local operation by the old baseline.
Partial checkout recovery, cross-journal admission, cancellation acknowledgement,
wire-token read stability and local conflict reconstruction remain **not executed
facts**. Tasks 4–7 are mandatory test-first checkpoints; Task 8 supplies additional
adversarial/closure evidence. Any contradictory result stops for a ruling.

Owner shorthand: L = new `--lib local_sync::` tests, M = new
`--test mutation_replay local_sync`, R = new `--test read_items local_sync`,
C = the two existing E3 closure unit fixtures, requiring adaptation. The new
L/M/R local-sync modules and required-outcome cases do not exist yet.

| # | Sequence | Required outcome | Test owner |
| --- | --- | --- | --- |
| 1 | Crash after intent, before Git effects | Same-ID retry completes recorded pair once; no new observation. | Task 5 L; 7 M |
| 2 | Crash during initial checkout/conflict installation | Exact baseline or fully proven result resumes; ambiguous partial files/index/metadata preserved for operator repair. | Task 5 L; 8 no_drop_backend_ambiguity_preserves_until_operator_proof |
| 3 | Primary advances after acceptance | Finish frozen pair honestly; fresh operation incorporates newer primary. | Task 5 L; 8 C |
| 4 | Context changes outside owned progress | No destructive reconciliation; external_change/recovery_required retains original evidence. | Tasks 5–6 L |
| 5 | Stale/foreign/duplicate/missing conflict token | Refuse before resolution writes; files/index/refs unchanged. | Task 6 L; 7 R/M |
| 6 | Crash after some resolution paths, before complete candidate | Resume requires original resolution request; exact resend completes once. | Task 6 L; 7 M |
| 7 | Crash after candidate/ref transition, before settlement | Reconcile same candidate, retire only owned artifacts, no second commit/ref effect. | Task 6 L; 7 M |
| 8 | Cancel with conflict/resolution pending | Retain files/evidence; explicit continuation progresses; unsafe abandon refuses. | Task 6 L; 7 M |
| 9 | Resolution completes, starting output lost | Both requests replay their own recorded/provable answers, no Git effect. | Task 7 M |
| 10 | Closing closure + partial local sync | Closure/incompatible work refuse; reads and exact recovery remain available. | Task 4 L; 8 C/R |
| 11 | Local sync completes + explicit closure resume | Adopt validated descendant; original closure_oid/closed_at/closed_by retained. | Task 8 C |
| 12 | Healthy rebuild during recoverable state | Evidence retained; no manufactured authority or premature completion. | Tasks 4/8 L/M |
| 13 | Lost journal with conflict/lock metadata | Recovery required; no adoption, cleanup, fresh merge or silent abort. | Task 8 M lost_journal_distinguishes_conflict_checkout_and_completed_state |
| 14 | Cancellation acknowledged after R path write, death before reply | S/R stay Accepted; exact R continues; U identifies R; no missing-source invention/cancel loop. | Task 7 M cancelled_partial_resolution_remains_accepted_and_resendable |
| 15 | New cancellation races acknowledgement/U takeover | Newer issued counter survives; U checks S/active R and stops at safe point. | Tasks 6/7 new_cancellation_survives_acknowledgement_and_resume |
| 16 | R integrated/released, indexing fails, abandon | Atomic parent/attempt abandon and Accepted S/R/U cancelled; finished immutable, IDs spent. | Task 7 M artifact_free_abandon_settles_all_associated_requests_atomically |
| 17 | R accepted, death before parent link | No Git effect; exact retry links with current attempt/epoch; stale executor fenced. | Task 7 M unlinked_accepted_resolution_cannot_write_after_attempt_loss |
| 18 | Only R observation changed after partial writes | request_mismatch with historical effects; attempt/files unchanged. | Task 7 M accepted_resolution_observation_is_part_of_request_identity |
| 19 | Retained remote versus local start, both orders | Only one owner; refused remote reacquisition changes no phase/epoch; no stalemate. | Task 4 L; 8 retained_remote_and_local_admission_never_deadlock |
| 20 | Known incompatible closures at preparation | Refuse without pending owner/effects; unrelated work succeeds. | Task 5 L immutable_closure_disagreement_never_fences_the_repository |
| 21 | Child killed in backend ref/log/metadata window without Drop | Ambiguous artifacts preserved until operator establishes baseline/validated result. | Tasks 5/8 L no_drop_backend_ambiguity_preserves_until_operator_proof |
| 22 | Child exits during proven per-member metadata/sentinel retirement | Already absent counts retired; exact-owned remainder retires automatically; no new commit; foreign refuses. | Tasks 6/8 L no_drop_owned_metadata_retirement_finishes_automatically |
| 23 | Partial R retry advances F2 attempt then dies before domain takeover | Same association/effects/counters retained; only current attempt/epoch executes/settles. | Task 7 M request_attempt_handover_preserves_resolution_and_cancellation |
| 24 | Unlinked R0 races R1 completion and artifact-free abandonment | Intended-parent association includes R0 in atomic cancel, or terminal parent produces no-effect finish; no dangling Accepted. | Task 7 M unlinked_contender_observes_parent_completion_or_abandonment |
| 25 | Parent completes before unlinked R/U acquires attempt | external_change without own effects; release unlinked request; no new attempt/parent rewrite. | Task 7 M unlinked_contender_observes_parent_completion_or_abandonment |

Additional mandatory design proofs retain their approved owners: unrelated
histories and identity retries (5/7), two-session authority-only read changes and
serialized/historical tokens (4/7), typed journal/side-read/path PermissionDenied
(4/6/7/8), source privacy, no transport including unreachable configured remote,
cross-process leases and distinct lost-journal cases (8). These are not waived by
the narrower existing baseline. Commit identity can change before candidate
creation, not after a candidate is journaled; refresh takeover requires integrated
and released proof, not merely a step name.

## Appendix: complete reference-hit file inventory

All paths below are repository-relative. Braced groups are an inventory notation,
not a shell/Cargo command. Together these cover the 111 files in the query above.
Critical exact line locations and classification are in the reference table;
remaining matches are type uses, constructors, old cancellation/effect tests or
frontend consumers. Presence in this inventory grants no edit permission.

- `src/results.rs`, `src/results_tests.rs`, `src/repository.rs`.
- `src/repository/{recovery.rs,registration_removal.rs,comment_publication_tests.rs}`.
- `src/repository/bridges/{abandon.rs,repair.rs}`.
- `src/repository/mutation/{command.rs,mod.rs,dto.rs,records.rs,execute.rs,reenter.rs,
  replay.rs,settle.rs,reply.rs,confirm.rs,outcome.rs,outcome_tests.rs}`.
- `src/repository/mutation/bind/{mod.rs,ticket.rs,document.rs,save.rs,repo.rs,
  remote.rs,sync.rs,key.rs,repo_identity.rs,host.rs,abandon.rs,repair.rs,resume.rs}`.
- `src/repository/promotion/{lifecycle.rs,lifecycle_record.rs,lifecycle_tests.rs,
  record.rs,record_tests.rs,run.rs,run_tests.rs}`.
- `src/repository/closure/{record.rs,record_tests.rs,run_tests.rs}`.
- `src/repository/remote/{poll.rs,poll_tests.rs,reservation.rs,reservation_tests.rs,
  state.rs,state_tests.rs,sync_tests.rs}`.
- `src/repository/read/{mod.rs,resolve.rs,items.rs,status_tests.rs}`.
- `src/bin/manyhands-cli/{human_admin.rs,human_admin_tests.rs,render.rs,render_tests.rs,
  mutate.rs,mutate_tests.rs}`: existing frontend uses; no Task 0/later gate edit scope.
- `src/desktop/{mod_tests.rs,test_support/fixtures.rs}`;
  `src/desktop/model/{operations.rs,operations_tests.rs}`;
  `src/desktop/services/{mod.rs,worker.rs,worker_tests.rs,reads_tests.rs,drafts/save.rs}`;
  `src/desktop/view/{confirm.rs,confirm_tests.rs,operations_tests.rs}`;
  `src/desktop/view/settings/{repositories_tests.rs,keys_tests.rs,remotes_tests.rs,
  keys/commands.rs}`: preserve frontend matches, especially global recovery enum.
- `tests/{mutation_replay.rs,mutation_remote.rs,mutation_confirmation.rs,
  mutation_contract.rs,read_contract.rs,remote_observation.rs,remote_reservation.rs,
  remote_synchronization.rs,remote_merge_recovery.rs,remote_polling.rs}`.
- `tests/support/{conformance.rs,mutation.rs,confirmation.rs,repair.rs,golden.rs}`.
- `tests/document_promotion/{journeys.rs,advanced.rs,quality.rs}`.
- `tests/mutation_confirmation/abandon.rs`, `tests/mutation_remote/host.rs`.
- `tests/mutation_replay/{documents.rs,key_generation.rs,keys.rs,resume.rs,progress.rs,
  bindings.rs,reenter.rs,index.rs,remotes.rs,execute.rs,not_enabled.rs,cache_loss.rs}`.
- `tests/mutation_bridges/{folder.rs,identity.rs,repair.rs}`.

Semantic checks additionally read the three E7 schema files, E9 mutation_golden
and replay conformance registration, SSH harness, fixture and named regression
functions above; their absence from the literal query is not an exemption.

## Independent Task 0 review

Controller reported both reviews **APPROVED**, each with zero findings, for
`0cbeab0..9039e65` (full endpoints
`0cbeab084d252e3823d317a468eee1f29c92c8ce` and
`9039e65f882c169a70ae0f5b90da33b18760ea43`). Recorded at
`2026-10-26T02:28:46Z` in canonical checkpoint comment
`01M5V0KXPBH8XBCD5XP17C5CH6`.

| Review | Session | Result |
| --- | --- | --- |
| SPEC | `ses_e89f8c6f0ffe8XoInnuqmMjdsA` | APPROVED; zero findings. |
| QUALITY | `ses_e89f70347ffezlwXc1KFxZujfO` | APPROVED; zero findings. |

Both reviewers source-verified E9's `tests/support/repair.rs::repair_data` and
E4's companion first-13-action split as genuine omitted named edits, not Task 0
defects. Their approvals accept this baseline/inventory checkpoint; the owner
still must answer the two exact additions in question comment
`01M5TZXJZ2348Z8GRMAQ7BSHPR`. Ticket and ledger remain waiting; Task 1 is not
authorized to proceed past that stop.

Review-record preflight: clean ticket checkout; fetched origin and merged current
main `0464c42dae19a6dde000c7e7660e82762a41fada`, already up to date. HEAD before
and after was `9039e65f882c169a70ae0f5b90da33b18760ea43`; ancestry check passed.
This follow-up records reviews only; no implementation or baseline rerun.

## Owner inventory ruling and Task 1 start

At `2026-10-26T02:31:31Z`, recorded the controller-supplied latest owner answer
**“yes”**, explicitly approving both questions in `01M5TZXJZ2348Z8GRMAQ7BSHPR`:
E4 `src/repository/read/status_tests.rs::a_local_action_is_one_the_recovery_store_writes`
local-action partition and E9 `tests/support/repair.rs::repair_data` new DTO
rejection arm. Updated the design's named-edit table; no E4/E9 code edit belongs
to Task 1. Canonical approval comment: `01M5V0RYTAKJE3F155FH4QTP53`.

Restored ticket implementing using the working-on-a-ticket status-edit exception.
Clean entry HEAD `5e6e7ad5a90f80752b08432cd2b573087b7bea01`; fetched and merged
`origin/main` `0464c42dae19a6dde000c7e7660e82762a41fada`, already current, HEAD
unchanged. Merge-tree succeeded with tree `77010a32ecd6416d9aa38a2dd6cb94c101f68116`.
Task 0's approved baseline is retained without rerunning unchanged tests.

Task 1 follows the approved red/green classification matrix, E1/E2 fresh-result
changes, retained-attempt and mixed-set controls, then whole-tree fmt/check/clippy
and targeted tests. Controller reviews its actual commit range before Task 2.
PR, merge, closure, cleanup and native CI remain separately gated.

## Task 1: honest incompatible-closure classification

Recorded `2026-10-26T02:38:38Z`. Separate approval checkpoint/code base:
`9b7879e3b03a888c8f6ab62608b0edede65a6982`. This task's code and evidence are in
the commit titled `fix: classify incompatible closures as external recovery`.
Review that commit from the code base, separately from the preceding approval
commit (`5e6e7ad..9b7879e`). No subagents were used by this implementer.

### Source and diff audit

- `src/repository/remote/merge.rs`: added internal
  `ConflictStructure::IncompatibleClosures`. Existing whole-set eligibility
  already rejects every structure except RegularUtf8SamePath; it is unchanged.
- `src/repository/remote/sync.rs::classify_conflict_entry`: after canonical
  shape/kind/ID validation, compare all nonempty ticket closure pairs across
  ancestor/local/incoming. Equality uses parsed `OffsetDateTime` and exact
  `closed_by` text. More than one distinct pair returns the explicit structure.
  Production diff: **17 inserted lines across two files**.
- `src/repository/remote/sync_tests.rs`: six new classification tests cover both
  fields differing, timestamp only, author case only, ancestor disagreement
  (three placements), seven compatible controls (including UTC spelling and
  status-text differences), and a mixed canonical set. Fixtures explicitly parse
  before classification. E2 retains all eight original rows and every no-write
  assertion; only its incompatible row now requires the external-only error.
  `try_resolve_fixture` is used without changing either shared resolver helper.
- `src/repository/remote/merge_tests.rs`: one new test pins explicit unsupported
  structure rejection alone and with document/ticket/comment in either order.
- `tests/ticket_closure/races.rs`: E1 requires fresh external-only classification
  and refusal for either side; every image/ref/history assertion remains.
- Rust diff: **5 files, 213 insertions, 14 deletions**. Only E1/E2 pre-existing
  test semantics changed. `preserves_item_invariants`, external integration
  validation, retained-attempt reconciliation, dependencies, frontend, schemas,
  global ResultCode/RecoveryActionKind and E4/E9 code are unchanged.

### TDD and targeted verification

All commands below ran in the ticket worktree. Each library command has one
positional filter; `--exact` uses the complete E2 unit name. No zero-case result
was accepted.

| Command | Red before production change | Green after change |
| --- | --- | --- |
| `devenv shell -- cargo test --all-features --locked --lib repository::remote::sync::tests::closure_conflict_classification` | Corrected-fixture run: 1 control passed, 5 failed with EligibleCanonical versus ExternalResolutionRequired; 1270 filtered. | 6 passed, 0 failed/ignored; 1271 filtered. |
| `devenv shell -- cargo test --all-features --locked --lib repository::remote::sync::tests::all_recorded_ticket_sides_constrain_retained_closure_before_writes -- --exact` | 1 failed at fresh incompatible classification; 1275 filtered. | 1 passed, 0 failed/ignored; 1276 filtered; all eight rows exercised. |
| `devenv shell -- cargo test --all-features --locked --test ticket_closure true_race_distinct_identities_needs_external_recovery -- --exact` | 1 SSH case failed, 0 passed; privacy-safe panic observation 153 identifies the changed classification assertion in races.rs, not fixture setup. Harness hook at ssh_harness.rs:67–71 emits panic line numbers. | 1 SSH case passed, zero failures. |
| `devenv shell -- cargo test --all-features --locked --lib incompatible_closures_make_every_mixed_set_order_external_only` | Supplementary structure/ordering test added with the enum after behavioral red; no separate pre-fix run. | 1 passed, 0 failed/ignored; 1276 filtered. |
| `devenv shell -- cargo test --all-features --locked --lib retained_candidate_cannot_bypass_all_side_closure_validation` | Unchanged regression, not rerun before edits. | 1 passed, 0 failed/ignored; 1276 filtered. |
| `devenv shell -- cargo test --all-features --locked --lib mixed_canonical_and_code_conflict_is_whole_merge_external_only_without_writes` | Unchanged regression, not rerun before edits. | 1 passed, 0 failed/ignored; 1276 filtered. |

Final targeted total: **10 library tests + 1 SSH case passed**. The legacy retained
candidate still returns ValidationFailed and preserves refs/index/worktree/artifact
counts. Fresh incompatible sets refuse earlier without accepting an attempt.

The first new-unit run had five intended classification failures plus one invalid
new control fixture: a non-UTC offset. Systematic debugging traced it to
`canonical.rs::parse_time`'s existing UTC-only contract (1198–1203). Changed only
the new fixture to equivalent `.000+00:00` versus `Z` spelling, then reran red:
the compatible control passed and all five intended failures remained. No parser
change or existing assertion adjustment was needed.

### Gates and handoff

- `devenv shell -- cargo fmt --check`: final exit 0. Initial formatting check
  reported three new-code layout differences; applied exactly those layouts.
- `devenv shell -- cargo check --all-features --locked`: exit 0 (4.01s).
- `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`:
  exit 0 (18.33s), no warnings.
- `git diff --check`: exit 0. Inspected complete Task 1 source diff/status/recent
  log before staging only owned source, ledger and checkpoint comment.
- Checkpoint fetch still reports main `0464c42dae19a6dde000c7e7660e82762a41fada`;
  ancestry passed. Precommit merge-tree exit 0, tree
  `998da6e07931a0af25f590ab9be7708601ad330b` (committed approval HEAD).
- Full suite, native CI and app smoke tests were not run for this library-only
  targeted task. No native-platform claim; full suite/native remain post-merge.
- No unresolved implementation question or out-of-scope old-test change found.
  Immutable closure races remain unsupported; this task makes classification
  honest, not a winner-selection policy. Controller SPEC then QUALITY review is
  next. Ticket remains implementing; Task 2 has not started.

## Independent Task 1 review

Recorded at the Task 2 checkpoint, `2026-10-26T02:48:10Z`, from the controller's
explicit handoff. Both reviews **APPROVED with zero findings** for
`9b7879e..76a1ff5` (full endpoints
`9b7879e3b03a888c8f6ab62608b0edede65a6982` and
`76a1ff547a20dc3b112d5cc9417ef29e287f4b3a`).

| Review | Session | Result |
| --- | --- | --- |
| SPEC | `ses_e89ead5dbffeOxjtFK9DMXgdOi` | APPROVED; zero findings. |
| QUALITY | `ses_e89e93bd6ffeltNxvyfeRCKLIZ` | APPROVED; zero findings. |

Task 0 and E4/E9 approvals above remain authoritative. This checkpoint records
Task 1 review completion and Task 2 implementation; it grants no new lifecycle
permission.

## Task 2: child mismatch wording by parent kind

Recorded `2026-10-26T02:48:10Z`; task base
`76a1ff547a20dc3b112d5cc9417ef29e287f4b3a`. Bounded implementer used no subagents.
Only the assigned ticket worktree/branch was changed. Entry status was clean at
the controller's supplied HEAD. Read current AGENTS, approved contract/design/
plan, ticket, checkpoint and ledger, plus the parent/fence/test source.

Preflight fetched origin and merged main
`0464c42dae19a6dde000c7e7660e82762a41fada`: already up to date, task HEAD unchanged,
ancestry check passed, no conflicts. Checkpoint fetch found the same main;
`git merge-tree --write-tree HEAD origin/main` exited 0 with tree
`331438ddce7565143862638c2919e7ff4cf34620` (committed task-base HEAD).

### Implementation and coverage

- Root cause: private `fence_mismatch` hard-coded DocumentPromotion despite
  ChildExecution retaining LifecycleKind. It now accepts that kind, and all five
  refusal sites pass `self.kind`, including the absent-parent `ok_or_else`.
- Production changes are confined to `src/repository/promotion/lifecycle_record.rs`:
  helper signature/body, comment and five callers. Validation predicates, their
  ordering, completed authoritative handoff exception, error kinds, and privacy
  wrappers are unchanged.
- Added four tests and two fixture/assertion helpers to the existing
  `src/repository/remote/reservation_tests.rs`. No existing assertions changed.
  Both closure and promotion parents cover both context and primary roles:
  **48 mismatch scenarios** (13 per context kind, 11 per primary kind).
- Cases independently alter request operation/target role, repository/operation
  identity, parent disappearance, item/expectation binding, attempt counter,
  child ID, disallowed stage and unreserved cancellation; context cases also
  alter target kind and item. Each starts with successful request/durable fence
  validation of a parent issued through real record APIs. Durable mutations
  retain valid row shape so the asserted error is OperationMismatch, not decode
  recovery failure.
- Tests obtain the inner RepositoryError directly, retaining it inside the
  transaction result where needed. Every case asserts OperationMismatch and
  exact fixed parent-specific message. Collecting messages before comparison
  ensures the red run exercises all scenarios, rather than stopping at its first
  wrong string. Promotion controls pin the original text.

### TDD and gates

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --lib child_fence` (RED, before production edits) | **6 passed, 2 failed**, 0 ignored/measured, 1273 filtered; 2.45s. Both new closure tests failed only on promotion-versus-closure text, across all 24 closure scenarios. Two new promotion controls and four old tests passed. |
| Same `child_fence` command (GREEN) | **8 passed, 0 failed/ignored/measured**, 1273 filtered; 2.67s. All 48 new mismatch scenarios exercised. |
| `devenv shell -- cargo test --all-features --locked --lib quality1_current_authority_handoff_and_ordinary_synchronization_remain_valid` | **1 passed, 0 failed/ignored/measured**, 1280 filtered; 1.01s. |
| `devenv shell -- cargo fmt --check` | Final exit 0. Initial check identified only the new helper's tuple-return line wrapping; applied that layout and reran. |
| `devenv shell -- cargo check --all-features --locked` | Exit 0; 2.62s. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Exit 0; 10.99s, no warnings. |
| `git diff --check` | Exit 0. |

Final targeted total: **9 library tests passed**. No full suite or native CI was
run under this task's targeted-test scope. No dependency, frontend, global enum,
new target, schema, or existing-test semantic changes were needed. No unresolved
issue or owner question. Controller independently reviews SPEC then QUALITY for
the commit from the recorded task base; Task 3 is not started. Ticket remains
implementing. Commit/push are authorized; PR/lifecycle/native actions are not.

## Independent Task 2 review

Recorded from the controller's Task 3 handoff at `2026-10-26T03:14:25Z`:
both reviews **APPROVED, zero findings**, range `76a1ff5..1f25f56`
(`76a1ff547a20dc3b112d5cc9417ef29e287f4b3a` through
`1f25f56e6ce8c69f0f0b53c8939fe4443aa46c07`).

| Review | Session | Result |
| --- | --- | --- |
| SPEC | `ses_e89e1ee10ffel5fclITUqdJmJA` | APPROVED; zero findings. |
| QUALITY | `ses_e89e109a8ffeZq9AWjP725U5xx` | APPROVED; zero findings. |

Task 2 evidence remains **8 child_fence + 1 handoff test green**, all prescribed
fmt/check/clippy gates green. Those unchanged tests were not rerun for Task 3.

## Task 3: transport-neutral mechanics extraction

Recorded `2026-10-26T03:14:25Z`. Task/code base:
`1f25f56e6ce8c69f0f0b53c8939fe4443aa46c07`. This bounded implementer used no
subagents. Entry branch/HEAD matched the handoff and status was clean. Fetched
origin, read current main's AGENTS, and merged main
`0464c42dae19a6dde000c7e7660e82762a41fada`: already up to date, HEAD unchanged.
Checkpoint fetch found the same main; ancestry and merge-tree passed (committed
base tree `848c7660f04674ba55d044b651276f32c942df9d`).

### Source map and private boundaries

Before moving production code, read and mapped clean/fast-forward/conflict install
and retained/fresh resolution order in `remote/sync.rs`. The new
`integration/mod.rs` records the sequence, exact caller preconditions, callback
lease/authority contract, coarse-backend limitation and remaining adapter duties.

| Shared module | Actual moved mechanics / old production callers |
| --- | --- |
| `prepare.rs` | Ordered graph classification from `remote/merge.rs`; priority-1000 memory preparation, generated-object import, and recorded merge-index reconstruction from `remote/sync.rs`. Remote divergence/inspection/recovery use them; existing promotion/closure preview callers retain their adapter signatures. |
| `validate.rs` | Whole-set eligibility, Task 1 three-side closure classification, immutable canonical provenance, prospective context/dependency validation, non-conflicting stage-zero comparison, canonical tree snapshots and external-result canonical validation. Old inspection/fresh resolution/retained resolution/external-repair paths call them. |
| `resolution.rs` | Complete index/status/path preflight, ignored-file policy, dirty-path gate, no-follow preimage/result digests, already-applied/preimage/third-image decision, and result-blob/private-index staging. Old resolver and candidate recovery call them. |
| `artifacts.rs` | Safe old-HEAD checkout/ref transaction, coarse conflict installation, Unix descriptor-based index image/presence inspection, and Unix/Windows pinned merge-metadata digest/retirement. Old integration, private index-lock implementation and metadata recovery call them. |

Every extracted helper is reachable through an old production path; no future
local scaffold, dependency or lint suppression was added. Shared inputs are
explicit OIDs, paths already bound by an adapter, borrowed sources and digests;
there are **no remote tokens, reservations, SQL rows or transport imports** in the
new machinery. Its two content-free internal errors map to the same remote errors.

Public ConflictEligibility keeps its existing remote re-export. Internal shared
items are repository-visible using `pub(in super::super)`; prepared entries and
metadata directory descriptors remain private. Remote capability fields, stage/
window selection, journal schema, identity confirmation, request/attempt fencing,
ResolutionIndexLock and ref-log manifests remain private to the remote adapter.
Their stateful acquire/persist/ref-proof/checkpoint/release orchestration is not
replaced by a new generic owner. No visibility expansion in reservation.rs or
state.rs was necessary. The existing repository-level digest-guarded path writer
continues at the same point in the adapter's all-path-check/write/observe sequence.

The new import callback performs a fresh `reservation::owned` check under the
already-held lease before persistent object import, propagating its original
error. It adds no cancellation safe point or ownership acquisition. The generic
checkout callback can reject before backend locks; the compatibility wrapper's
callers retain their existing synchronization/poll/lifecycle owner gates. Its
after-checkout fixture hook stays at the original interleaving boundary.

Exact-parent proof and clean-worktree proof still precede external canonical
validation in the remote adapter. Retained-candidate reconciliation still precedes
fresh inspection, and metadata/sentinel retirement still follows final proof and
durable checkpoint. Shared content validation is explicitly **not** write authority.
There is no per-path initial-checkout journal or expanded no-Drop promise.

Only one existing-test edit (E8): `comment_invariants_hold_only_when_created_by_is_unchanged_wherever_it_is_read`
now calls `validate::preserves_item_invariants` directly. Its complete assertion
and all test inputs remain unchanged. All other existing assertions are untouched.

### Test-first evidence and debugging

Added three adjacent shared-seam tests and module documentation/declaration before
production extraction. Ran `devenv shell -- cargo test --all-features --locked
--lib repository::integration::tests`: **exit 101, four E0583 missing-module errors**
for the not-yet-created prepare/artifacts/validate/resolution seams, as permitted
by the approved new-seam TDD rule. No runtime test was selected in that red build.

After extraction, runtime was initially **2 passed, 1 failed**. The new path test
incorrectly treated `.git/config` and missing parents as rejected image reads.
Source tracing to `owned_file_bytes_with_mode_policy` showed that it checks real
relative no-follow filesystem images, while canonical/token authority belongs to
the caller, and missing parents deliberately represent an absent preimage. Fixed
only the new fixture: traversal/absolute/empty/directory and symlink refusal, with
a positive absent-preimage control. Added the complete preflight assertions and
an actual common-Git lease to the owner-loss test. No production behavior changed
to accommodate that mistaken fixture expectation.

The owner-loss adapter revokes authority after memory preparation, then both
import and checkout/ref effects refuse before objects/index/ref/worktree/lock
changes. The memory test produces a genuinely new merged blob, proves it absent
from the persistent ODB alongside unchanged refs/index/worktree, then separately
imports and proves that blob exists (positive control).

### Final selected verification

Each row used `devenv shell -- cargo test --all-features --locked --lib <filter>`
with **one positional filter**. All passed; zero failed/ignored/measured.

| Filter | Selected/passed | Filtered |
| --- | ---: | ---: |
| `repository::integration::tests` | 3 | 1281 |
| `public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | 1 | 1283 |
| `external_repair_exact_ordered_parent_clean_only` | 1 | 1283 |
| `clean_merge_integration_fault_matrix_resumes_each_durable_transition_once` | 1 | 1283 |
| `cancellation_with_a_pending_conflict_is_a_recoverable_stop` | 1 | 1283 |
| `all_recorded_ticket_sides_constrain_retained_closure_before_writes` | 1 | 1283 |
| `closure_conflict_classification` | 6 | 1278 |
| `comment_invariants_hold_only_when_created_by_is_unchanged_wherever_it_is_read` | 1 | 1283 |
| `merge_metadata_retirement_` | 2 | 1282 |
| `resolution_preflight_` | 2 | 1282 |

Total: **19 library tests passed**, including all four anchors run separately.
Required whole-tree gates: `devenv shell -- cargo fmt --check` exit 0;
`devenv shell -- cargo check --all-features --locked` exit 0 (2.90s);
`devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`
exit 0 (22.24s), no warnings. `git diff --check` passed.

An earlier clippy run found restricted `pub(in crate::repository)` paths invalid
in existing SSH targets that include lib.rs under `mod production`. Replaced only
new visibilities with the established relative `pub(in super::super)` convention;
no harness/assertion edit. A subsequent fmt check requested five line-wrap changes
after those shorter qualifiers; applied them and reran all three gates green.

No unexpected remote runtime change or unresolved owner question was found.
Controller must independently review the **full Task 3 range**, actual moved code
and ownership sequencing before local execution is built. Task 4 is not started.
No full suite, native run, PR or lifecycle action; native Windows/macOS behavior
has not been executed by this Linux-only checkpoint.

### Exact review handoff

- **Task base:** `1f25f56e6ce8c69f0f0b53c8939fe4443aa46c07`.
- **Implementation head:** `87431fcffe18dd7125bf99962795129c9d5d578b`.
- **Code review range:** `1f25f56..87431fc`; inspect the new integration files as
  well as removed bodies and adapters, not only the remaining remote diff.
- Implementation commit is pushed to
  `origin/manyhands/ticket/01M57SKVF6TQ3QM6AXJ35JGGSY`; post-push status was clean.
  Postcommit merge-tree passed, tree `199338911f5a3db620f97dd510ab29b06e2e5666`.
- This subsequent documentation-only checkpoint records the resolved SHA and
  publication evidence. It changes no Rust/test code or verification result.
- Controller review focus: priority-1000 memory lifetime versus persistent import;
  explicit callback preconditions and existing adapter gates; unchanged complete
  preflight/retained-candidate order; content proof versus exact-parent/clean-state
  proof; private index/ref orchestration versus extracted descriptor/metadata
  mechanics; preserved conservative coarse-backend stops.

## Independent Task 3 review

Recorded with the Task 4 checkpoint at `2026-10-26T04:13:16Z`, from the
controller's explicit handoff. Both reviews **APPROVED with zero findings** for
the full range `1f25f56..37bd2c3`:

- Base: `1f25f56e6ce8c69f0f0b53c8939fe4443aa46c07`.
- Head: `37bd2c36ace84e56920d4cf15f55539d63c7d1c7`.
- SPEC: `ses_e89c7d13fffe4pSnuvk28e0DCu`.
- QUALITY: `ses_e89c52231ffeq2JYI8dlaxLS1O`.

This is the completed full extraction review, including active production
callers and ownership sequencing, not just the documentation-only final commit.

## Task 4: local journal, observation and reciprocal fences

Checkpoint timestamp/ID issued at `2026-10-26T04:13:16Z`; verification continued
after issuance. Base: `37bd2c36ace84e56920d4cf15f55539d63c7d1c7`. This bounded
implementer used no subagents and worked only in the assigned ticket checkout.
Entry status was clean. Start and checkpoint fetch/merge both found main
`0464c42dae19a6dde000c7e7660e82762a41fada`, already merged; HEAD stayed at the
task base. Ancestry and merge-tree passed (base tree
`c0160e972bdc16d705bee9f549e074166efbc66c`).

### Implemented boundaries and file ownership

- Added `src/repository/local_sync/{mod,record,observe}.rs` and adjacent
  `record_tests.rs`, `observe_tests.rs`, `admission_tests.rs`.
- The live service seams are `observe_local_context_sync`,
  `begin_local_context_sync`, `read_local_context_sync_record`, and
  `take_over_local_context_sync`. The last three are domain journal APIs, not
  the Task 5 Git executor or Task 7 mutation acceptance. They create/read a real
  typed record through admission; tests never seed a new local parent by raw SQL.
  Adversarial old-journal fixtures and corruption/epoch injections are explicitly
  test-owned database edits.
- Added same-database v1 record, ordered-step and resolution companion tables.
  Exact table definitions, migration marker, SQLite CHECK/FK integrity, typed
  record decoding, envelope/scope equality, generation/epoch and attempt
  association are audited. Partial unmarked schemas, missing migrated tables,
  missing companions, malformed evidence and orphaned children refuse.
- Envelope plus frozen companion identity is one immediate transaction under the
  common-Git lease and cache write guard. Creation rechecks authority and Git
  inside that transaction. A writable connection is opened only after the cache
  guard, avoiding a stale connection across database replacement. No id=0 or
  directory-registry fallback. Typed PermissionDenied is injected at the actual
  registry-opening seam and maps to `index_unavailable` before either row exists.
- Original-observation re-entry reads scoped frozen evidence before fresh Git
  observation; changed input mismatches. Takeover requires the exact previous
  record/epoch/generation and a successful one-row CAS. It changes only
  `owner_epoch`; refused/stale/ignored updates return no fabricated owner.
- Read session A ends before Git reads. Session B is a new read-only transaction,
  followed by Git recheck. Hashing uses versioned, length-delimited BLAKE3 over
  registered identity, refs/parents, canonical item path, index/configuration and
  semantic facts. Effective Git identity is excluded. Canonical item presence is
  verified from the committed context blob, rather than trusting stale discovery.
  Committed primary is used even with dirty primary files. Metadata/index probes
  reuse the extracted pinned no-follow machinery; observation performs no Git,
  SQL or transport effect and writes no token-issuance table.
- Added `context_sync_local` and `local_sync_prepared` parsing/status coverage.
  Old stored strings and Local/Remote/KeyMaterial family values are unchanged.
  Operation enumeration, registration removal and rebuild validation audit the
  companion; same-ID refresh requires integrated/released proof. The generic
  row-only abandon path recognizes companion ownership and cannot split it.
  Its exhaustive Writes match gained the necessary local-item arm; the domain
  artifact-free abandonment protocol remains Task 6/7.
- Closure preview, checkpoint's existing shared guard, adoption and stage
  advancement recognize unfinished local ownership. Lifecycle parent admission
  also refuses new work behind that owner. `closure/checkpoint.rs` itself needed
  no edit: its existing calls now reach the strengthened shared guard. Existing
  remote written-closure publication guards are unchanged.
- Remote conflict reacquisition checks pending local-sync authority in its own
  transaction before phase/epoch changes. Existing restart/reservation guards
  already check the local envelope. Local admission checks retained remote step,
  resolution/release/metadata and index obligations, including interrupted work
  on another context. Only repository-internal reacquisition visibility expanded.
- Published the exact eight-field observation DTO/schema, registered its golden,
  schema root and enum inventory, and added the action to the E7 schema lists.
  Generated only the new golden using `MANYHANDS_UPDATE_GOLDEN=1`; inspected it.
  E4's approved two action inventories and step inventory are the only old
  library-test expectation changes. E6/E7/E9 registry additions give the new
  observation an executable contract owner. E9 `repair_data` remains Task 7.

### Design checkpoint: observed answers

| Question | Executed answer |
| --- | --- |
| Ordinary document and ticket contexts? | Admit a clean, current, existing registered context; journal creation leaves Git unchanged. |
| Closure exception? | Only matching ticket, `Closing`, written checkpoint, exact closure fields and checkpoint/descendant proof. Bind the closure ID and checkpoint without changing the closure. |
| Before checkpoint, wrong item/kind/stage, promotion? | Refuse with recovery semantics and no new local owner. Every integrating/cleanup stage is covered. |
| Complete/abandoned closure history? | No special exception or closure binding. A remaining context is subject to ordinary admission. |
| Other local/remote owners? | Refuse. Interrupted retained remote obligations fence even another context. After local refusal the remote can restart; local-first ownership prevents remote restart/reacquisition without altering its phase or epoch. |
| Reciprocal closure fence? | Preview is blocked, checkpoint guard reports pending synchronization, adoption and integration-stage changes refuse. |
| Global operation ID namespace? | Fifteen journal/child families exercised through new-local admission: local envelope, key material, key registry, promotion parent/context/primary, closure parent/context/primary, comment parent/child, remote parent, remote resolution, remote identity confirmation, local resolution. No closure ID reuse. |
| Refresh/index-only confusion? | Prepared/pending ownership is not an index-only handoff; generic refresh and removal refuse. Healthy rebuild refusal preserves the record rather than completing it. |
| Authority-only read race? | Registration or semantic-revision change with identical Git gives `external_change`; owner-only takeover leaves the observation unchanged. Serialization/reopen is stable. |
| Permission/schema loss? | Typed journal PermissionDenied and directory registry return `index_unavailable`, with no Git changes or half-created rows. Missing/malformed companion authority refuses instead of reconstruction. |

No observed admission/namespace/refusal result conflicts with the approved table.
This is the Task 4 foundation checkpoint, not proof of the executor recovery table.

### Test-first and debugging evidence

All commands ran through Devenv in the ticket checkout. Initial new-seam run of
`cargo test --all-features --locked --lib local_sync::` exited 101 for missing
modules/API types, before production journal/observation implementation. It also
exposed a new-test ambiguous glob import, corrected explicitly.

Subsequent runtime red/green checkpoints:

| Red run/filter | Observed failure before its fix |
| --- | --- |
| `local_sync::` (16 tests) | After correcting new fixture wiring, 14 passed / 2 failed: generic refresh accepted the local owner; closure preview ignored it. |
| `local_sync::` (19 tests) | 16 passed / 3 failed: the preceding two fences plus missing local action/step mapping. |
| `local_sync::` (23 tests) | 21 passed / 2 failed: malformed companion evidence was accepted and partial unmarked schema was repaired. |
| `local_sync::` (24 tests) | 23 passed / 1 failed: retained remote conflict reacquisition returned ownership while local work was pending. |
| `local_sync::` (27 tests) | 24 passed / 3 failed: changed routing, unowned merge metadata, and retained remote metadata after an applied step were accepted. |
| `pending_local_owner_refuses_new_lifecycle_parent_and_closure_integration` | One failure on new promotion admission, then one on closure integration after the first guard; both assertions subsequently passed. |
| `admission_rechecks_authority_in_its_creation_transaction` | 1 failed: registration changed after observation but creation still succeeded. |
| `accepted_journal_reentry_reads_frozen_parents_before_live_observation` | 1 failed: frozen retry was rejected after primary advanced. |
| `stale_discovery_cannot_observe_an_item_removed_from_the_context_commit` | 1 failed: stale discovery issued an observation for an absent committed item. |
| `refused_epoch_update_cannot_return_a_fabricated_owner` | 1 failed: ignored SQL CAS was reported as a new owner. |
| `accepted_reentry_cannot_borrow_a_different_registered_root` | 1 failed: accepted re-entry failed to scope the record lookup to the registered root. |
| read-contract `local_context_sync_observation_matches_its_schema_and_golden` | 1 failed because the new schema did not exist; generated golden and registered schema then passed. |

Initial runtime wiring failures were investigated rather than counted as intended
reds: discovery stores shorthand branch names (converted to full refs); the remote
fixture needed `state::configure`, as existing reservation fixtures do. The new
generic-abandon test initially expected Err, but the established refusal is
`Ok(AnotherJournal)`; its final assertion pins that refusal plus unchanged authority.
The orphan test first exposed nullable digest CHECKs that wrongly rejected NULL
(fixed with explicit nullable predicates), then required explicitly disabling
fixture FKs to inject an orphan. Unsupported ResultCode/ItemId helper spellings
and an `is_plain_relative` argument type were corrected from the existing source.
These changes touched new code/fixtures only; no old assertion was weakened.

### Final gates and exact selected tests

Final Rust gates after the last scope/CAS changes:

- `devenv shell -- cargo fmt --check`: exit 0.
- `devenv shell -- cargo check --all-features --locked`: exit 0, 2.42s.
- `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`:
  exit 0, 10.42s, no warnings. The only new lint allowance is the established
  `ReadError`-by-value `result_large_err` convention; no dead-code suppression.
- `devenv shell -- cargo test --all-features --locked --lib local_sync::`:
  **38 passed**, 0 failed/ignored/measured, 1284 filtered, 31.83s.
- `git diff --check`: passed before checkpoint documentation.

Each library selector below ran as
`devenv shell -- cargo test --all-features --locked --lib <selector> -- --exact`,
except `child_fence`, which deliberately uses substring selection. Each exact
selector reported **one** passing test; `child_fence` reported **eight**.

| Group | Exact selectors (one command per name) | Passed |
| --- | --- | ---: |
| E4 | `repository::read::status::tests::a_local_action_is_one_the_recovery_store_writes`; `repository::read::status::tests::every_action_the_recovery_store_writes_is_one_the_read_knows`; `repository::recovery::abandoned_row_tests::every_step_is_stored_under_its_own_name_and_read_back` | 3 |
| Child fence | `child_fence` | 8 |
| Closure observation | `repository::closure::observe::tests::local_operation_pending`; `repository::closure::observe::tests::pending_local_operation_blocks`; `repository::closure::observe::tests::comment_publication_pending`; `repository::closure::observe::tests::pending_comment_publication_blocks` | 4 |
| Written closure publication | `repository::closure::run_tests::a_written_checkpoint_refuses_a_manual_context_synchronization`; `repository::closure::run_tests::an_active_manual_context_synchronization_holds_off_the_checkpoint`; `repository::closure::run_tests::an_interrupted_manual_context_synchronization_cannot_restart_after_the_checkpoint`; `repository::closure::run_tests::a_manual_context_synchronization_is_refused_while_publication_is_pending` | 4 |
| Closure adoption | `repository::closure::checkpoint::tests::adopt_written_closure_only_while_closing`; `repository::closure::checkpoint::tests::reconcile_adoptable_descendant_is_checkpoint_written` | 2 |
| Remote ownership controls | `repository::remote::reservation::tests::sync_authoritative_same_id_refresh_replay_allows_only_exact_refresh_identity`; `repository::remote::reservation::tests::sync_incomplete_action_cannot_coexist_with_a_same_id_local_refresh`; `repository::remote::reservation::tests::conflict_release_fences_stale_owner_and_requires_explicit_matching_reacquisition` | 3 |

Task 0's step selector used `recovery::tests`, which selected zero with `--exact`.
That attempt is **not** pass evidence. Source inspection found
`recovery::abandoned_row_tests`; the corrected exact command selected/passed one,
and the inventory above is corrected. No zero-selection result is accepted.

Additional published-boundary checks:

- `devenv shell -- cargo test --all-features --locked --test read_contract`:
  **72 passed**, including the new golden and exact enum/object-schema audits.
- `devenv shell -- cargo test --all-features --locked --test mutation_contract every_schema_is_reachable_from_a_published_root -- --exact`:
  **1 passed**.
- `devenv shell -- cargo test --all-features --locked --test mutation_contract every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root -- --exact`:
  **1 passed**.

Selected unique total: **62 library + 74 contract tests = 136 passed**. Repeated
red/green runs and repeated remote controls are not added to that unique total.
No full package suite, native workflow, front-end smoke run, PR, merge, closure or
cleanup was performed. Linux checks do not establish native Windows/macOS runtime
behavior or real Windows ACL behavior.

### Handoff boundaries and follow-up evidence

Controller SPEC then QUALITY review is next, using the full Task 4 base above
through the commit containing this checkpoint, including the new files. Task 5
has not started. No unresolved policy ruling was discovered.

- Task 5 must prepare/classify before using the journal boundary, then provide
  deterministic Git execution and frozen-pair recovery. Creating a record alone
  is not an integration success or proof of the incompatible-closure pre-effect
  executor rule.
- Tasks 5/6 populate durable effect/resolution evidence and own artifact release;
  the current companion foundation is not proof of partial checkout/path recovery.
- Task 7 must atomically create S/R/U request controls at F2 acceptance, preserve
  intended-parent identity before FK linkage, fence request attempts, and connect
  domain creation/takeover/settlement to those controls. None of that is claimed
  by this Task 4 commit. The current journal APIs are the meaningful domain seam.
- Full conflict token/content/historical-side reads, public inspection DTOs,
  cancellation acknowledgement and atomic artifact-free abandonment remain their
  approved Tasks 6/7 owners. Task 4 proves observation serialization/reopen and
  owner/semantic freshness separately; it does not claim historical side reads.
- Tasks 7/8 retain the public authority-only race, permissions, no-Drop/cross-process
  lease and complete closure-resolution acceptance cases listed earlier.

## Task 4 SPEC review: waiting for scoped owner decision

Recorded `2026-10-26T04:34:16Z`. Canonical question:
`01M5V7SPS9W51SSF5XYTJR5J08`. Ticket and ledger are set to **waiting** through the
`working-on-a-ticket` status-edit exception. This checkpoint changes only status,
ledger and comment; no implementation, test or schema changes were made.

- Reviewer session: **`ses_e898c0189ffebSYYicNzmsZaXQ`**.
- Review result: **REQUEST CHANGES — five findings (1 P1, 4 P2), all unresolved**.
- Reviewed range: **`37bd2c36ace84e56920d4cf15f55539d63c7d1c7..00a75e7742b6a9cc9284c91d2e18c9d5aec37599`**.
- Tasks **1, 2 and 3 retain their recorded SPEC and QUALITY approvals**.
- Task 4's initial **136 selected tests and fmt/check/clippy gates passed**, as
  recorded above, but **Task 4 SPEC is not approved**. Those results do not prove
  the newly identified missing controls. No unchanged tests were rerun here.

### Source-verified findings, not yet fixed

| # | Priority | Finding and verified source | Required correction/evidence |
| --- | --- | --- | --- |
| 1 | P1 | `local_sync/record.rs:714–725`: `refresh_allowed` returns `Ok(true)` for an absent companion without auditing the surviving local-sync envelope. `bridges/folder.rs:465–609` reaches that check before marking discovery stale and creating directories, then can reach `close_refresh` after refresh fails. | Audit before granting refresh permission; distinguish ordinary operations from missing local-sync authority. Add a public `create_folder` regression asserting no directories, registration change or envelope settlement. |
| 2 | P2 | `operation_collision` tests foreign-first admission only. `keys/generation.rs:65–91,279–302` reserves a generation without a local-ID exclusion. `keys/deletion.rs:59–95` and `keys/registry.rs:270–305` also insert key journal identities directly. The mutation/local/remote admission guards alone cannot protect these public key entry points. | Transactional reverse exclusion of IDs already owned by local sync, plus audit rejection of existing cross-journal duplicates; local-first tests. Preserve legitimate old cross-journal handoffs. The prior fifteen-family test is one-direction evidence, not proof of a reciprocal global namespace. Key-file edits require the scoped permission below. |
| 3 | P2 | `local_sync/record.rs` publicly exposes begin/takeover/read methods and its record is re-exported. The approved design at lines 115–118 keeps low-level execution library-owned and publishes service reads plus Mutation variants. | Internalize store mutators/record, retain real internal store tests, and use bounded appropriate test-only cfg while the executor caller is pending. Do not use public exposure or broad unused-code allowances to bypass the boundary. |
| 4 | P2 | Closing admission checks ancestry/closure fields but omits `closure::checkpoint::checkpoint_authority` (`checkpoint.rs:514–546`), which checks subject, exactly one open parent, and author/closed_by agreement. The local fixture's `fixture` commit subject does not establish that authority. | Reuse checkpoint authority validation, build a genuine closure checkpoint fixture, and test a valid descendant plus changed-closure/invalid-checkpoint refusals. |
| 5 | P2 | `local_context_sync_steps.expected_old_oid` and `expected_new_oid` are unconstrained TEXT and are not decoded by the companion audit. Malformed reference evidence can therefore survive read/takeover. | Validate persisted step OIDs and applicable required-field relationships, with corruption tests through the real internal read/takeover APIs. No Task 5 execution is needed to reject malformed authority. |

Verification here was source inspection under receiving-code-review discipline,
not a claim that new regression tests were run or any finding was resolved.

### Exact owner question and authorization boundary

**Approve minimal edits to `src/repository/keys/{generation,deletion,registry}.rs`
solely for transactional refusal of IDs already owned by local sync before
journal insertion/effects; shared guard/audit in `local_sync/record.rs`, new tests
in `local_sync`, with no old assertion changes or other key behavior changes?**

The owner initially approved the whole plan, detailed rulings, implementation and
ticket-branch commits/pushes. These key files are newly identified scope, rather
than an implicit extension of that approval. The current request authorizes this
documentation/status/comment checkpoint only; it does not approve the proposed
key-file edits, other files, native CI, or PR/closure/merge/cleanup actions.

An explicit scoped approval permits resuming Task 4: meaningful tests first for
**all five** findings, preserve approved policies and old assertions, fix and run
required Devenv gates plus relevant tests, then controller **SPEC re-review,
followed by QUALITY**. Without that answer, remain waiting with no fresh
implementation. Task 5 remains unstarted. Answer in chat or a canonical reply
whose `parent_id` is `01M5V7SPS9W51SSF5XYTJR5J08`.

Checkpoint preflight: clean worktree at `00a75e7`; fetched origin and merged
`origin/main` (already up to date). No PR exists. No Rust tests, native CI or
lifecycle completion action was run for these documentation-only edits.
