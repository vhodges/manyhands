---
title: "Closure Recovery Gate Execution Ledger"
date: 2026-10-25
status: "implementing"
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
review. Status is **implementing**: the owner answered both named-edit questions
with “yes”, explicitly approving E4's companion local-action partition and E9's
repair DTO refusal arm. Comment `01M5V0RYTAKJE3F155FH4QTP53` records the ruling.
Task 1 is now authorized; Tasks 4/7 retain ownership of E4/E9 code respectively.

| Work | Evidence/state |
| --- | --- |
| Approval recording | `51a0e96da11f8bc1f2f0689e72c88ed99470c96a`, `docs: approve closure recovery gate implementation`; pushed to ticket branch. |
| Task 0 | Source/selector inventory and required baseline completed; checkpoint `9039e65`, comment `01M5TZXJZ2348Z8GRMAQ7BSHPR`. Both independent reviews approved; owner questions resolved below. |
| Task 1 | Starting after the separate E4/E9 approval checkpoint; bounded implementer, no subagents. |
| Tasks 2–9 | Not started. New local-operation outcomes below remain unproven. |

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

### Owner questions: stop before Task 1

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
| E4 | `repository::read::status::tests::every_action_the_recovery_store_writes_is_one_the_read_knows`; `repository::recovery::tests::every_step_is_stored_under_its_own_name_and_read_back`; additional inventory question above. |
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
