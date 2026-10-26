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
review. The owner answered both earlier named-edit questions with “yes”, explicitly
approving E4's companion local-action partition and E9's repair DTO refusal arm;
comment `01M5V0RYTAKJE3F155FH4QTP53` records that ruling. Tasks 1–4 are implemented
and independently approved. The owner also approved the narrow key-file scope
question `01M5V7SPS9W51SSF5XYTJR5J08`, recorded in approval comment
`01M5VWDPG57KWRAJ55Y2EMXRQR`. The latest controller handoff reports SPEC and QUALITY
approval with zero findings at `4377944`, covering the full Task 4 range
`37bd2c3..4377944`. Earlier pending-review entries below remain historical.
Task 5 SPEC `ses_e87b8a702ffe6HryC0itCXg4H5` **approved with zero remaining findings
at `f282335`**, as reported by the controller. QUALITY
`ses_e878dbd29ffednm05pK3O5zlGc` then requested four corrections. Those now have
source-verified implementer fixes and fresh verification in
`01M5WAQG5RHQ4ZG571AB2DDCZA`. Subsequent SPEC verification at `2b9ba0c` found two
additional P1 image gaps; their source-verified fixes and new evidence are recorded
in `01M5WFEQNTFFPE9ZHKK82ABC1C`. **SPEC verification and QUALITY re-review remain
pending**; prior approvals are not automatically extended.
The controller resolved `01M5W0R66A877J9213TC9WTN42` as an **internal staging
decision**, not a new owner approval: narrowly test-gate the real internal executor
until Task 7 supplies its production callers. Task 7's explicit activation obligation
is recorded below and in `01M5W3AMDN1J5BTZNCW0X798QF`. Tasks 4/7 retain ownership of
E4/E9 code. The ticket remains implementing; no lifecycle status edit was needed.

| Work | Evidence/state |
| --- | --- |
| Approval recording | `51a0e96da11f8bc1f2f0689e72c88ed99470c96a`, `docs: approve closure recovery gate implementation`; pushed to ticket branch. |
| Task 0 | Source/selector inventory and required baseline completed; checkpoint `9039e65`, comment `01M5TZXJZ2348Z8GRMAQ7BSHPR`. Both independent reviews approved; owner questions resolved below. |
| Task 1 | Implemented at `76a1ff547a20dc3b112d5cc9417ef29e287f4b3a`, after approval checkpoint `9b7879e3b03a888c8f6ab62608b0edede65a6982`; comment `01M5V15ZJFXBEMM7KQ4G18Y3RC`. Controller SPEC and QUALITY both approved with zero findings; sessions below. |
| Task 2 | Implemented at `1f25f56e6ce8c69f0f0b53c8939fe4443aa46c07`, from `76a1ff547a20dc3b112d5cc9417ef29e287f4b3a`; comment `01M5V1QEB9WYETHHG1H765DVH7`. Controller SPEC and QUALITY approved with zero findings; sessions below. |
| Task 3 | Implemented at `87431fcffe18dd7125bf99962795129c9d5d578b`, review-range checkpoint `37bd2c36ace84e56920d4cf15f55539d63c7d1c7`. Controller SPEC and QUALITY approved the full `1f25f56..37bd2c3` range, zero findings; sessions below. |
| Task 4 | **Complete and approved**, full range `37bd2c36ace84e56920d4cf15f55539d63c7d1c7..4377944cffa6dc9c00e43d97053d00f8a7fbdc19`. Controller reports SPEC `ses_e898c0189ffebSYYicNzmsZaXQ` and QUALITY `ses_e881a2165ffeEIJWyTo9j3HVR3` approved at that head, each with zero findings. |
| Task 5 | Initial implementation `7856c61..5fde757`; SPEC corrections `5fde757..f282335` approved with zero remaining findings. QUALITY corrections `f282335..2b9ba0c` followed. SPEC verification `ses_e87b8a702ffe6HryC0itCXg4H5` then found two P1 image gaps. Current fix base `2b9ba0cb72850e478094c310f92b2406c8221acc`, checkpoint `01M5WFEQNTFFPE9ZHKK82ABC1C`: **105 local tests + 39 affected controls pass**, whole-tree fmt/check/clippy pass. SPEC verification then QUALITY re-review pending; executor remains narrowly test-gated until Task 7. |
| Tasks 6–9 | Not started. Resolution and request-boundary outcomes remain unproven. |

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

## Scoped approval and Task 4 review fixes resumed

At `2026-10-26T10:34:43Z`, the controller supplied the owner's answer **“yes”**
to question `01M5V7SPS9W51SSF5XYTJR5J08`. This authorizes minimal edits to
`src/repository/keys/{generation,deletion,registry}.rs` solely for transactional
refusal of operation IDs already owned by local sync before journal/effects;
shared guard/audit stays in `local_sync/record.rs`, new tests in `local_sync`,
with no old assertion changes or other key behavior changes. The design scope
addendum and canonical comment `01M5VWDPG57KWRAJ55Y2EMXRQR` record that exact limit.

Restored ticket/ledger to implementing using the status-edit exception. Entry
HEAD `fc50316277db8228f3ed41513bf5fe96beab289a` was clean; fetched and merged
`origin/main` `0464c42dae19a6dde000c7e7660e82762a41fada`, already up to date.
This approval checkpoint changes no code and reruns no unchanged tests.
Next: fix all five verified findings with meaningful tests first, required
Devenv gates and relevant regression tests, then controller SPEC followed by
QUALITY. Task 4 is not approved; Task 5 remains unstarted. Existing branch
commit/push approval remains, without PR/merge/closure/native authorization.

## Task 4 SPEC fixes: awaiting re-review

Recorded `2026-10-26T11:00:53Z`, comment `01M5VXXM7XSZCBTGB0HTVKV246`.
Session entry `fc50316277db8228f3ed41513bf5fe96beab289a`; separate scope-approval
commit **`9b40deda19d7fee34a7956a54743e1483359d230`** is the code-fix base.
Both start and checkpoint fetch/merge found main `0464c42` already incorporated.
Precommit merge-tree passed with committed base tree
`4b1472af85ee8b09c25c8b168103a5f2db4167a2`. No subagents were used.

Review addressed: **SPEC `ses_e898c0189ffebSYYicNzmsZaXQ`**, initial range
`37bd2c36ace84e56920d4cf15f55539d63c7d1c7..00a75e7742b6a9cc9284c91d2e18c9d5aec37599`.
The table below records implementer dispositions and evidence, not reviewer
approval. Tasks 1–3 retain their earlier approvals; Task 5 remains unstarted.

### Item-by-item verified corrections

1. **P1, missing companion refresh bypass — addressed.** `refresh_allowed`
   audits before every permission result, including absent-companion handling.
   An ordinary ID can still refresh; an ID reserved by a local resolution cannot.
   A parent handoff additionally matches its registered root. The public
   `create_folder` regression deletes the companion of a real admitted local
   operation, then compares directory absence, registration freshness/timestamps,
   envelope state/step/epoch/timestamp and Git images. All remain unchanged.
2. **P2, reverse ID exclusion and duplicate audit — addressed.** Shared
   `owned_identity`/`require_foreign_id_available` recognize local-sync envelopes,
   companions and resolution IDs, including an envelope whose companion was lost.
   Guards run inside key generation reservation, the new immediate deletion-intent
   transaction, and marked key-registry change transactions; key preflight/replay
   reads also reject before opening the key store. These are the sole behavioral
   edits in the three newly approved key files. The existing local begin/refresh,
   remote/lifecycle identity, remote child-ID, lifecycle child minting and comment
   child minting seams call the same narrow guard where needed. It is not a new
   general namespace policy for old journals. Companion audit detects foreign
   journal duplicates for parent and resolution identities, excluding the parent's
   own legitimate envelope. Existing remote/local handoff and parent-child controls
   pass unchanged. New tests cover public key generation/deletion, marked registry
   mutation rollback, duplicate corruption in three foreign journals, resolution
   IDs across a second registered root (folder/remote/key/promotion/closure), and
   remote child-ID admission. The old fifteen-family test remains **foreign-first
   evidence**; it is not relabeled as fifteen public reverse-order executions.
3. **P2, public unfinished store API — addressed.** Removed both record
   re-exports and its unused public getter. Record and store methods are
   repository-internal. The real begin/read/takeover impl and its exclusive helpers
   have bounded `cfg(test)` while the approved executor caller is absent; migration,
   decoding, audit, namespace guards and observation remain active production
   paths. No broad dead-code allowance was introduced. Task 5 must enable the
   internal seams with its prepared executor and retain their internal visibility;
   Task 7 still owns request-attempt authority. Four compile-fail doctests reject
   the old public record/begin/read/takeover surfaces. The approved observation
   DTO/service API and its golden continue to pass.
4. **P2, incomplete Closing checkpoint proof — addressed.** Admission now
   invokes the existing `closure::checkpoint::checkpoint_authority` before
   descendant/field proof. The local fixture now builds a real `Close ticket ID`
   checkpoint with one open parent and matching author/closed_by. No existing
   assertions were weakened. New negatives cover wrong subject, wrong author,
   two parents and already-closed parent; positives validate checkpoint authority
   directly and admit a clean descendant retaining the original checkpoint. A
   changed-closure descendant refuses with Git unchanged. The existing authority,
   adoption and publication guards pass unchanged.
5. **P2, unvalidated step OIDs — addressed.** Audit decodes both optional step
   OIDs as canonical lowercase nonzero full Git OIDs. A `reference` intent requires
   both old/new endpoints even when prepared; other effect kinds may omit a ref
   transition but any stored OID must validate. Eight malformed/missing-field
   scenarios refuse both real internal read and takeover without changing epoch;
   valid reference and absent non-reference controls pass. This runtime audit
   strengthens existing v1 evidence validation without rewriting the table shape
   or inventing Task 5 effect execution.

### TDD evidence

New tests live in `src/repository/local_sync/review_tests.rs`; old assertions in
other test modules are unchanged. The only edits to existing local test helpers
are the genuine checkpoint construction and explicit import of the now-internal
record. Source verification preceded the fixes, as recorded in the blocker.

| Command/filter, all through Devenv | Red evidence | Green evidence |
| --- | --- | --- |
| `cargo test --all-features --locked --lib local_sync::review_tests` (first 3 cases) | 1 passed, 2 failed: public folder created directories after companion loss; malformed OID read succeeded. | 3 passed after audit fixes; all eight malformed cases exercised. |
| Same filter (5 cases) | 4 passed, 1 failed: wrong-subject checkpoint admitted. | All four invalid authority forms and both descendant cases pass in final selection. |
| `cargo test --all-features --locked --doc local_sync` | 0 passed, 4 failed: all forbidden public API examples compiled successfully despite `compile_fail`. | 4 passed after internalization; no unused warnings. |
| `cargo test --all-features --locked --lib local_sync::review_tests` (9 cases) | 5 passed, 4 failed: public key generation, public deletion, marked registry mutation, and duplicate-journal audit. | 9 passed after shared guard/audit and scoped key edits. |
| `cargo test --all-features --locked --lib local_resolution_id_is_reserved_across_repository_boundaries` | 1 failed on folder creation on a second registered root; after that guard, 1 failed on remote reservation. | Final test covers all five foreign admission paths. |
| `cargo test --all-features --locked --lib remote_child_id_guard_refuses_local_resolution_identity` | 1 failed: existing child-ID guard admitted a local resolution ID. | Passes for local parent/child refusals and a fresh-ID positive control. |

The initial removal of the fixture's old commit call left an unused import, and
internalization left a production-unused ResultCode import. Removed/scoped those
imports; no warning suppression or unrelated code changes. No unexpected old-test
failure or additional policy question arose.

### Final verification and counts

- `devenv shell -- cargo fmt --check`: exit 0.
- `devenv shell -- cargo check --all-features --locked`: exit 0 (8.24s).
- `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`:
  exit 0 (37.40s), no warnings.
- `devenv shell -- cargo test --all-features --locked --lib local_sync::`:
  **49 passed**, 0 failed/ignored, 1284 filtered. This includes **11 new review
  regressions** and the 38 earlier local tests.
- `devenv shell -- cargo test --all-features --locked --doc local_sync`:
  **4 passed**, 11 filtered.
- `devenv shell -- cargo test --all-features --locked --lib child_fence`:
  **8 passed**, 1325 filtered.

Each selector below ran individually as
`devenv shell -- cargo test --all-features --locked --lib <selector> -- --exact`;
each selected/passed **one** test, zero failed/ignored:

```text
repository::read::status::tests::a_local_action_is_one_the_recovery_store_writes
repository::read::status::tests::every_action_the_recovery_store_writes_is_one_the_read_knows
repository::recovery::abandoned_row_tests::every_step_is_stored_under_its_own_name_and_read_back
repository::closure::observe::tests::local_operation_pending
repository::closure::observe::tests::pending_local_operation_blocks
repository::closure::observe::tests::comment_publication_pending
repository::closure::observe::tests::pending_comment_publication_blocks
repository::closure::run_tests::a_written_checkpoint_refuses_a_manual_context_synchronization
repository::closure::run_tests::an_active_manual_context_synchronization_holds_off_the_checkpoint
repository::closure::run_tests::an_interrupted_manual_context_synchronization_cannot_restart_after_the_checkpoint
repository::closure::run_tests::a_manual_context_synchronization_is_refused_while_publication_is_pending
repository::closure::checkpoint::tests::adopt_written_closure_only_while_closing
repository::closure::checkpoint::tests::reconcile_adoptable_descendant_is_checkpoint_written
repository::closure::checkpoint::tests::checkpoint_authority_reads_commit_not_config
repository::remote::reservation::tests::sync_authoritative_same_id_refresh_replay_allows_only_exact_refresh_identity
repository::remote::reservation::tests::sync_incomplete_action_cannot_coexist_with_a_same_id_local_refresh
repository::remote::reservation::tests::conflict_release_fences_stale_owner_and_requires_explicit_matching_reacquisition
repository::promotion::run_tests::quality1_current_authority_handoff_and_ordinary_synchronization_remain_valid
repository::remote::sync::tests::public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally
```

Additional commands use `devenv shell -- cargo test --all-features --locked`:

| Target/filter | Passed |
| --- | ---: |
| `--test key_material` | 39 |
| `--test shared_key_registry` | 39 |
| `--test mutation_replay key_generation` | 21 |
| `--test mutation_replay keys::` | 31 |
| `--test mutation_bridges folder::` | 32 |
| `--test read_contract local_context_sync_observation_matches_its_schema_and_golden -- --exact` | 1 |

Unique selected total: **76 library + 4 compile-fail doctests + 163 integration
tests = 243 passing tests**. Repeated red/green selections are not counted twice;
no zero-selection result was used. The full package suite and native workflows
were not run. No front-end/dependency/global-enum changes, new test target, PR,
closure, merge or cleanup. Key-file changes are limited to the owner-approved
local-sync identity refusal and transaction boundaries.

### Re-review handoff

The code review range starts at **`9b40deda19d7fee34a7956a54743e1483359d230`**
and ends at the commit containing this checkpoint. The preceding approval-only
range starts at `fc50316`. Reviewer should check all five corrections, narrow key
scope, unchanged old handoffs, and bounded internal test-only seams. There are no
known unresolved implementation findings or policy questions, but **Task 4 awaits
controller SPEC then QUALITY re-review and is not done**. Future executor/request
control/historical content/abandonment evidence remains with Tasks 5–7; this commit
does not claim it. Linux evidence is not native Windows/macOS runtime evidence.

## Task 4 QUALITY fixes: awaiting re-review

Recorded `2026-10-26T11:36:41Z`, canonical checkpoint
`01M5VZZ5VXNW0PXCGTQWNXHYBV`. Code-fix base:
**`f578bf7a37b5c52e205ef5cb3e1aaae0c8e40f16`**. Entry was clean; start and
checkpoint fetch/merge of `origin/main` `0464c42dae19a6dde000c7e7660e82762a41fada`
were no-ops. Merge-tree passed with base tree
`df27f70b9aa98c8ea9e61681b2e96d8d66d1d331`. No subagents were used.

### Review record

- Controller reports **SPEC `ses_e898c0189ffebSYYicNzmsZaXQ` APPROVED all five
  earlier findings, zero findings at `f578bf7`**. That approval is preserved as
  evidence for that head, not extended automatically to this change.
- **QUALITY `ses_e881a2165ffeEIJWyTo9j3HVR3`: three P2 findings** on child-history
  deletion, child integer authority and closure-checkpoint OID decoding.
- Source verification confirmed each before editing: the child FKs omitted
  ON DELETE CASCADE while public removal deletes envelopes first; SQLite numeric
  affinity/CHECKs do not establish INTEGER storage and the audit read no child
  integer fields; closure_checkpoint used direct Oid::from_str instead of the
  existing strict nullable OID path.

### Corrections and proof

1. **Terminal child-history removal — addressed.** Both child tables now cascade
   from their local companion. The existing public removal transaction can delete
   envelope, companion, children and registration together without a normal-path
   FK failure or orphaned removal intent. Pending admission/refusal rules remain.
   A public regression starts through the real store, retains a step and resolution,
   and establishes an audited abandoned-before-effects historical fixture. Removal
   succeeds and leaves no registration/envelope/child rows, with Git unchanged.
   A pending control preserves all rows and creates no removal intent. An injected
   registration DELETE failure proves the cascade rolls back with the transaction;
   same-ID retry then succeeds.

   Existing v1 journals are not discarded or silently accepted under a changed
   definition. New marker **`local_context_sync_child_cascade_v2`** versions the FK
   upgrade. Migration audits the exact legacy layouts and all authority, rebuilds
   only the two child tables with identical columns/data plus cascading FKs, and
   records the marker inside the existing migration transaction. Tests cover
   pending and terminal evidence preservation, terminal removal after upgrade,
   both-table/marker rollback on injected failure, and refusal of corrupt legacy
   authority without repair. Exact schema audits recognize the appropriate
   migration stage. The old fresh-schema fixture now clears both migration markers;
   every existing assertion is retained.
2. **Child integer authority — addressed.** Both child queries now decode stored
   generation and owner_epoch as i64, and steps also decode ordinal as i64. Thus
   REAL/TEXT storage cannot masquerade as integer authority through SQLite affinity.
   Ordinal is nonnegative within the signed SQLite INTEGER range; generations and
   epochs are positive, generation must equal the parent generation, and a child
   epoch cannot exceed its parent epoch. These checks replace the SQL-only parent
   comparison rather than inventing a new numeric limit. **32 corruption scenarios**
   across all five child integer fields cover text, fractions, out-of-range numeric
   representation, negatives, zero where prohibited and values beyond parent
   authority. Both real internal read and takeover return RecoveryRequired with
   unchanged parent epoch/generation/revision and Git. Ordinals 0/i64::MAX and a
   retained older child epoch after takeover remain valid controls.
3. **Closure checkpoint OID — addressed.** Extracted one strict nonzero canonical
   full-OID decoder, reused by persisted parent/candidate/result/checkpoint OIDs,
   step OIDs and resolution candidates. The closure_checkpoint path now uses it.
   Four corrupt spellings (zero, uppercase, short, invalid) of a genuinely admitted
   Closing fixture refuse real read and takeover, preserving owner epoch and Git.
   Existing genuine-checkpoint, valid-descendant and changed-closure controls pass.

The only production file changed is `local_sync/record.rs`. New tests are in
`local_sync/quality_tests.rs`, registered by mod.rs. No executor, public API,
request-control, key behavior or global enum change. Historical-state fixture
edits test cleanup of valid retained journal history; they are not a Task 6/7
abandonment implementation. New local parents are created through the real store.

### Red/green evidence

All runs used
`devenv shell -- cargo test --all-features --locked --lib local_sync::quality_tests`:

| Stage | Result |
| --- | --- |
| Before production fixes, five tests | **2 passed / 3 failed**: public removal hit FK failure; noninteger ordinal read succeeded; zero checkpoint OID read succeeded. |
| After typed integer and strict OID fixes | **4 passed / 1 failed**: only the terminal-history removal FK failure remained. |
| Added upgrade/rollback/legacy-corruption tests before migration implementation | **5 passed / 3 failed**: removal FK failure, absent cascade upgrade, and absent marker-insertion failure path. |
| After versioned cascade migration | **8 passed**, zero failures. |
| Final group with public deletion-rollback control | All **9 QUALITY tests** passed within the full local_sync selection below. |

No test failure was worked around by weakening an assertion. No new owner policy
question arose. The migration-fixture marker cleanup is the sole change to an
existing test setup and preserves its exact missing-table refusal assertions.

### Verification

Required whole-tree gates, through Devenv, all exit 0:

- `cargo fmt --check`.
- `cargo check --all-features --locked` (2.94s).
- `cargo clippy --all-targets --all-features --locked -- -D warnings` (11.50s).

Targeted commands use `devenv shell -- cargo test --all-features --locked`:

| Selection | Passed |
| --- | ---: |
| `--lib local_sync::` (49 prior + 9 new QUALITY tests) | 58 |
| `--lib repository::registration_removal::tests` | 20 |
| `--test repository_enablement remove_registration` | 8 |
| `--lib child_fence` | 8 |
| `--doc local_sync` | 4 |

Each following selector ran individually with `--lib <selector> -- --exact`,
selecting/passing **one** test:

```text
repository::read::status::tests::a_local_action_is_one_the_recovery_store_writes
repository::read::status::tests::every_action_the_recovery_store_writes_is_one_the_read_knows
repository::recovery::abandoned_row_tests::every_step_is_stored_under_its_own_name_and_read_back
repository::closure::checkpoint::tests::checkpoint_authority_reads_commit_not_config
repository::closure::checkpoint::tests::adopt_written_closure_only_while_closing
repository::closure::checkpoint::tests::reconcile_adoptable_descendant_is_checkpoint_written
repository::remote::reservation::tests::sync_authoritative_same_id_refresh_replay_allows_only_exact_refresh_identity
repository::remote::reservation::tests::sync_incomplete_action_cannot_coexist_with_a_same_id_local_refresh
repository::remote::reservation::tests::conflict_release_fences_stale_owner_and_requires_explicit_matching_reacquisition
```

Unique total: **95 library + 8 integration + 4 compile-fail doctests = 107 passing
tests**, zero failures/ignored and no zero-selection evidence. The local group
finished in 66.18s; its 32-fixture integer corruption matrix accounted for the
long-running-test notice and completed successfully. No full suite or native run.

### Next gate

The three QUALITY findings have implementer fixes and evidence, not reviewer
approval. Review this commit from `f578bf7a37b5c52e205ef5cb3e1aaae0c8e40f16`.
Ticket remains implementing. **No Task 5 advance until QUALITY re-review is green
and SPEC has verified the changed fixes.** No known implementation blocker remains;
native platform runtime behavior and later executor/request protocols remain
outside this checkpoint. No PR, merge, closure or cleanup was performed.

## Task 4 final approvals and Task 5 staging clarification

Recorded `2026-10-26T11:50:21Z`, canonical comment
`01M5W0R66A877J9213TC9WTN42`. The fresh Task 5 controller handoff explicitly
reports both reviews **APPROVED, zero findings at `4377944`**:

- SPEC: `ses_e898c0189ffebSYYicNzmsZaXQ`.
- QUALITY: `ses_e881a2165ffeEIJWyTo9j3HVR3`.
- Full Task 4 review range:
  `37bd2c36ace84e56920d4cf15f55539d63c7d1c7..4377944cffa6dc9c00e43d97053d00f8a7fbdc19`.

These approvals supersede the pending-review status above; they are supplied by
the controller, not newly performed reviews by this implementer.

### Preflight and source findings

Work stayed in the assigned ticket worktree/branch. Entry was clean at
`4377944cffa6dc9c00e43d97053d00f8a7fbdc19`. Fetch and merge found main
`0464c42dae19a6dde000c7e7660e82762a41fada` already incorporated; HEAD was unchanged
and the ancestry check passed. Read current main AGENTS, the full authoritative
contract/design, implementation plan and execution ledger, plus actual local
record/observation and neutral integration interfaces. No subagents were used.

The remaining question is task staging, not a change to the accepted recovery
policy or a contradiction of Task 4's tested outcomes:

- `local_sync/record.rs:510–514` intentionally compiles its internal store impl
  only under `cfg(test)` until an executor calls it. The public observation API
  is read-only; its compile-fail examples preserve the private store boundary.
- The approved design keeps low-level execution library-owned; public execution
  belongs to Mutation bindings. Implementation-plan Task 7 owns those bindings,
  local resume routing and F2 request/control association.
- Current `mutation/bind/resume.rs:339–366` has only generic local handoff and
  original-request disposition, and `takes_up` at 399–413 has no local-sync
  executor route. Searches in `src/repository/mutation` found no local-sync
  command/type/route. Task 5 therefore has no existing production entry caller.
- Publishing a direct executor solely to avoid unused-code warnings would violate
  the explicit public-boundary restriction. Moving execution into observation
  would violate its read-only contract. Adding the production mutation/resume
  route now would bring forward Task 7 ownership work.

**Clarification requested:** may the real internal Task 5 executor and its
executor-only dependencies remain narrowly `cfg(test)` until Task 7 supplies the
approved production caller? This would exercise actual domain execution through
the adjacent unit and no-Drop child tests, preserve private visibility, and use
no dead-code lint allowance. It would explicitly deliver domain proof rather than
claim a production-reachable execution boundary at Task 5. The earlier store-only
staging approval is not silently extended to the whole executor.

### Evidence and outstanding outcomes

This is a documentation-only checkpoint before test/code edits. No Rust command
was run and no RED/GREEN count, Rust gate or recovery outcome is claimed. Task 4's
previous results remain historical evidence, not rerun results. All Task 5 rows
remain outstanding: equal/contained, fast-forward, clean merge, stale/frozen retry,
intent/checkout/candidate/ref/index interruptions, identity and unrelated-history
refusals, incompatible-closure preacceptance refusal, dirty/symlink preservation,
transport negatives and no-Drop/operator proof. Conflict/resolution rows remain
Task 6; request/control/replay rows remain Task 7; broader adversarial and closure
acceptance rows remain Task 8. No completed outcome row has been changed.

Only this ledger and the canonical clarification comment are changed. The feature
commit `feat: integrate local primary into editing contexts` is reserved for an
implemented and verified Task 5; this checkpoint is not that completion. No
ticket lifecycle edit, PR, native CI, dependency, frontend or source change.

## Task 5: local integration executor and exact recovery

Checkpoint ID/time issued at `2026-10-26T12:35:22Z`; final verification continued
after issuance. Comment `01M5W3AMDN1J5BTZNCW0X798QF` records the controller staging
decision and implementation evidence. Code base:
**`7856c619f157febf3c327f5d643cba1a4557cb8a`**. Original task-supplied Rust base:
`4377944cffa6dc9c00e43d97053d00f8a7fbdc19`; the intervening commit changes only the
ledger and question comment. Review the implementation commit from `7856c61`, or
the complete Task 5 documentation/code range from `4377944`.

### Controller decision and mandatory Task 7 activation

The controller directed: **“Proceed with temporary narrow cfg(test) gating for the
internal executor and exclusively executor-only dependencies until Task7 introduces
real production mutation/resume callers.”** This is an internal compile-staging
decision consistent with Task 4, **not a new owner policy, scope or authorization
approval**. The prior question is resolved. No owner escalation is needed for
bounded contract-consistent internal wiring. Ticket status was already implementing
and was left so.

The executed implementation is real domain/Git/SQL code, not a mock or success
stub. Its temporary entry gates are `local_sync/mod.rs::{run,recover}`, the two
local-only bounded Git helpers in `integration/artifacts.rs`, and the existing
executor-only store/observation helpers. No public execution bypass or dead-code
allowance was added. Production schema migration/audit/fences remain active.

**Task 7 MUST** remove the executor and exclusively executor-dependent gates when
adding the approved mutation/resume bindings; activate only the internal store
helpers those callers actually need; retain fixture-only hooks/tests under test
configuration; add F2 acceptance/linkage and current request-attempt checks to
creation, takeover, each effect and settlement; run production-boundary tests and
whole-tree fmt/check/all-target clippy. The current all-target clippy compiles the
real test path. This checkpoint does **not** claim a production-reachable local
execution feature, Accepted S/R/U controls, public DTO output, or request settlement.

### Implemented interfaces and sequencing

- `local_sync/run.rs::execute` is internal and returns content-free typed reason,
  optional retained phase and shared effects. It consults accepted intent before
  fresh observation checks. The store preparation callback runs under the common
  Git lease before the envelope/companion/execution identity transaction.
- New `local_context_sync_execution` has migration marker
  `local_context_sync_execution_v3`, exact SQL shape, FK cascade, CHECK/integrity
  audits and parent/object-intent consistency checks. It retains graph disposition,
  routing/index/log digests, effect flags and frozen commit identity/time metadata;
  never source or candidate object bytes. Existing v1/v2 history is preserved.
- Preparation refuses unrelated histories, incompatible closure conflicts, missing
  required identity and dirty targets before accepting domain ownership. Both
  context kinds use committed configured primary. Ignored prospective-path
  collisions, untracked/tracked edits and symlink substitution are preserved.
- Execution uses the common-directory lease and an exact scoped parent-row check
  (including epoch/generation). Owner takeover requires a successful epoch update;
  ignored intent/phase updates cannot authorize Git. Every bounded Git helper
  performs its authority callback before effects; ref movement rechecks under
  its actual backend lock. Recorded step kind/parents/generation must match retry.
- Equal/contained does no Git write and creates no commit. Fast-forward adopts the
  frozen primary tip. Clean merge prepares in the memory ODB, durably freezes one
  ordered `[context, primary]` candidate and usable configured signature, then
  imports that exact object. Before/after-import retries reconstruct/adopt the same
  OID even if configured identity changes. No identity configuration is written.
- `local_sync/recover.rs` compares full index entries and no-follow file bytes
  against the frozen baseline/result, force-reloading the disk index. Ref proof
  checks both branch and HEAD reflog prefixes plus the exact intended append.
  Backend lock/merge metadata or unproven partial images are preserved. Matching
  changed paths establish truthful partial-write effects, **not per-path resume
  authority**. There is no reset, rollback, automatic lock removal or new general
  checkout engine.
- Integration/release is recorded before indexing. Under the lease, two discovery
  observations must agree; existing `persist_observation` is reused. Discovery,
  companion completion and envelope completion commit atomically. Generic refresh
  refuses this executor-owned pending handoff because it can only settle the old
  envelope; exact local retry completes it without another ref effect.
- Stable scoped evidence reads retain historical effects after input mismatch,
  owner loss or a lease contender. `Outcome::of_mutation` and `Envelope` derive
  classes from those effects: the same RequestMismatch code is ordinary before
  effects and Incomplete after a write. Index failure is typed `index_pending` /
  `discovery_pending`, retaining complete integration and the result OID.
- The no-network control registers a fixture-only libgit2 transport that counts
  and refuses calls before any socket exists. Local execution uses a configured
  unreachable URL with zero calls; an explicit fixture fetch exercises the spy
  afterward as its positive control. Primary dirty bytes and tracking refs remain
  unchanged. No network progress source is present in the domain executor.

Files: new `local_sync/{run,recover,run_tests}.rs`; modified `local_sync/{mod,record}.rs`,
`integration/artifacts.rs` and `repository.rs` (narrow local failure points only).
The existing observation API needed no behavior change. No pre-existing test
assertion, dependency, frontend, global ResultCode/RecoveryActionKind, Cargo target
or native workflow changed. No Task 6 conflict resolution was implemented.

### TDD and debugging record

All commands were run from the ticket worktree through Devenv. Library selections
use one positional filter. New-seam compilation failed first for missing modules
and local failure points. A temporary always-refusing seam then established runtime
red; it was replaced by real execution and is absent from the final tree.

| Command/filter (prefix `devenv shell -- cargo test --all-features --locked --lib`) | Observed RED before correction | GREEN evidence |
| --- | --- | --- |
| `local_sync::run_tests`, initial 7 | 1 passed / 6 failed: graph effects, frozen retry, identity, immutable closure reason, retained authority and fault recovery absent. The unrelated-history case matched the refusing seam and was not counted as independent red proof. | All seven pass with the actual executor and positive graph controls. |
| `local_sync_required_outcomes_fault_matrix` | Retry after ref transition refused because the initial proof incorrectly expected HEAD's reflog unchanged. Diagnostic booleans confirmed the HEAD append. | Exact expected append now checked in both logs; 24 scenarios pass. |
| `local_sync::run_tests`, 12 | 10 passed / 2 failed: ignored prospective file was admitted, and child returned 101 instead of exiting at a real backend boundary. | Preaccept no-follow collision check; actual checkout progress/ref-lock hooks. |
| Same 12 after shared adapter wiring | 6 passed / 6 failed: separate Git handles left the owner's cached index stale after checkout. | Force reload of durable index before comparison; no assertion weakened. |
| `local_sync::run_tests`, 16 | 14 passed / 2 failed: partial checkout reported unchanged writes; ownership-loss hook not yet exercised. | Partial intended-image evidence retained, actual epoch/lease loss fenced with historical effects. |
| `local_sync_object_intent_retries_the_exact_candidate_across_identity_changes` | 1 failed: retry created a different candidate after identity changed across object intent. | Candidate OID/signature/ordered intent frozen atomically before import; both boundaries pass. |
| `local_sync_valid_oid_corruption_cannot_substitute_an_unrelated_candidate` | 1 failed: a syntactically valid substituted OID was accepted. | Candidate/intent/disposition audit and actual parent/signature/invariant proof refuse it. |
| `local_sync_ignored_epoch_or_intent_update_never_authorizes_git_effects` | 1 failed: ignored epoch update allowed execution. | Required one-row updates and exact intent checks; epoch and checkout-intent cases pass. |
| `local_sync_index_handoff_cannot_be_settled_by_generic_refresh` | 1 failed: generic refresh took the executor's handoff. | Executor-owned handoff remains with its atomic settlement adapter. |
| `local_sync_lease_contender_cannot_erase_historical_effects` | 1 failed: Busy dropped the earlier committed checkpoint effect. | One scoped evidence snapshot supplies historical effects even when the executor cannot acquire its lease. |

Compilation-only corrections were a missing existing validate_identity operation
argument, a typed rusqlite closure annotation and an unused import; these were not
counted as behavioral RED. No unexplained old-test failure or policy contradiction
was encountered. The last four no-Drop object/ref-log cases are supplementary
crash proofs over the corrected protocols, not a separate pre-fix RED claim.

### Partial required-outcome table at Task 5

This supplements the original 25-row ownership table; it does not mark later
conflict/request outcomes complete.

| Original row / requirement | Task 5 executed proof | Remaining owner |
| --- | --- | --- |
| 1: intent before Git effects | Both kinds, frozen pair and exact same operation/observation retry; one branch ref append. | Task 7 public S acceptance/replay. |
| 2: coarse checkout interruption | Actual child exit during partial checkout; preserve/refuse ambiguity; explicit operator clean-baseline restoration permits retry of the same candidate. Both kinds, merge path. | Task 6 conflict installation; Task 8 broader backend/platform cases. |
| 3: primary advances | Original frozen pair finishes; new primary file is absent from the completed context. | Task 8 closure follow-on sync; Task 7 public binding. |
| 4: foreign context progress | Foreign files/logs/locks, lost epoch/lease, changed input and invalid valid-OID candidate preserve state and refuse. | Task 6 partial-resolution foreign changes. |
| 7: candidate/ref before settlement | Before/after object import, checkout, ref and index boundaries; candidate identity frozen, ordered parents, no second ref/commit. Actual no-Drop after object import and after ref plus both logs. | Task 6 resolution/owned metadata retirement; Task 7 independent request settlement. |
| 20: incompatible closure preparation | Typed immutable-disagreement refusal, no local owner or Git change; unrelated refresh succeeds. | Task 7 no Accepted F2 request retained. |
| 21: no-Drop backend ambiguity | Child dies while actual libgit2 ref lock is held; lock and partial checkout persist. Incomplete operator repair refuses; exact baseline or full intended ref/log result permits retry with one candidate. | Task 8 deeper backend windows; Task 6 conflict metadata. |
| Additional graph/effect requirements | 8 kind/graph scenarios, 24 kind/graph/fault scenarios, exact ordered parents and primary/tracking invariance. | Production-boundary DTO/progress contracts in Task 7. |
| Additional identity requirements | Six kind/acceptance/candidate scenarios; two object-intent identity-change scenarios. No-op/FF manufacture no commit. | Task 7 repo.identity_set recovery action and request-ID reuse/retention. |
| Privacy/no transport/dirty targets | Real positive-controlled transport spy; committed-primary-only input; six dirty-file fixtures, two Unix symlink fixtures; source sentinel present in owned file but absent from journal/request tables and diagnostics. | Broader wire/source/path/native proofs in Tasks 7/8. |

**Outstanding:** rows 5–6 (tokens and partial resolution sources), 8–9
(cancellation and two-request settlement), 10–11 (complete closure acceptance),
12–13 (executor recovery across rebuild/full journal loss), 14–18 and 23–25 (F2
attempts, cancellation generations, unlinked contenders and atomic abandonment),
and 22 (proven per-member metadata retirement). Row 19's Task 4 admission proof is
retained, not relabeled as new Task 5 execution. All conflict installation,
inspection, source resolution and metadata retirement remain Task 6. Admissible
conflicts currently stop at the internal preparation seam; this is not a delivered
conflict-recovery API. Local completion never calls closure resume.

The two no-Drop parent tests launch **eight child processes**, all exiting with
the real configured status 86 without unwinding: checkout/ref-lock ambiguity and
object-import/ref-plus-log progress, each for document and ticket contexts. The
child entry itself returns immediately in a normal parent test run; that one
selected harness test is not an additional crash proof.

### Final gates and selected counts

Final required Rust gate commands, all exit 0 on the final Rust tree:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib local_sync::
```

Final check: 4.01s; clippy: 17.27s, no warnings; local selection: **81 passed**,
0 failed/ignored/measured, 1284 filtered, 155.77s. This is 58 prior local tests plus
23 new Task 5 tests (including the child entry). Long-running fixture-matrix notices
completed successfully; no timeout or zero-selection result is counted as a pass.

Additional relevant verification (same code paths; the later change only retained
local historical effects on lease failure and added supplementary child tests):

| Command (prefix `devenv shell -- cargo test --all-features --locked`) | Passed |
| --- | ---: |
| `--doc local_sync` | 4 compile-fail API-boundary tests |
| `--lib repository::integration::tests` | 3 |
| `--lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | 1 |
| `--lib external_repair_exact_ordered_parent_clean_only` | 1 |
| `--lib clean_merge_integration_fault_matrix_resumes_each_durable_transition_once` | 1 |
| `--lib cancellation_with_a_pending_conflict_is_a_recoverable_stop` | 1 |

Selected unique total: **88 library tests plus 4 compile-fail doctests = 92**.
Child-process assertions are part of their parent tests, not added to that total.
Full-suite/native execution remains separately gated; Linux evidence does not prove
Windows/macOS runtime behavior. No frontend smoke run was needed for this internal
library task.

Start and final fetch both found main `0464c42dae19a6dde000c7e7660e82762a41fada`
already merged. Final merge-tree succeeded with committed base tree
`3aa14f2f5204d69f14551e3a968d1c834a41302c`. Inspected status/diff/log and owned files;
`git diff --check` passed. This implementer used no subagents. Next is controller
**independent SPEC, then QUALITY**, covering the full Task 5 range and the mandatory
Task 7 activation obligation. No Task 6 implementation, PR, lifecycle action or
native dispatch is included.

## Task 5 SPEC corrections: awaiting re-review

Recorded `2026-10-26T13:35:08Z`, comment `01M5W6R1Q4F2SY8E86XKB838N9`.
Review: **`ses_e87b8a702ffe6HryC0itCXg4H5`, REQUEST CHANGES: 3 P1, 1 P2**.
Fix base: **`5fde75757182c5714a6069ceec7998a4f1f6ed33`**. The full Task 5 code
range continues to start at `7856c619f157febf3c327f5d643cba1a4557cb8a`; the original
task-supplied Rust base was `4377944`. Each finding was verified against source
before a regression and correction. These dispositions are implementer evidence,
not SPEC approval. No owner question or policy/scope change was needed.

### Findings, fixes and proof

1. **P1: same-tree integrations dead-end — addressed.** The old code skipped
   checkout when the candidate image already matched, then unconditionally set
   checkout_observed and observed a nonexistent step. It now records checkout
   completion only when checkout ran or an earlier recorded checkout is being
   reconciled. Equal-tree fast-forwards and divergent clean merges still perform
   exactly one ref transition, but report write unchanged and create no checkout
   intent. New proof covers **12 cases**: document/ticket × empty-commit FF,
   change/revert FF and same-tree clean divergence × immediate execution or retry
   after the ref fault. It asserts unchanged file/index bytes, one ref-log append,
   exact ordered merge parents, correct made/adopted checkpoint effects and replay.

2. **P1: lost execution row grants a generic handoff — addressed.** The execution
   table's own rows cannot prove that no row was lost. Added parent-owned
   `execution_required`, atomically set with real execution identity creation.
   `local_context_sync_execution_authority_v4` additively migrates the exact audited
   parent layout. Audit requires the parent marker and execution-row presence to
   agree; known integration/candidate phases cannot pretend to be store-only.
   `refresh_allowed` now uses that retained parent fact after audit, never child
   absence as permission. Deleting the execution row before effects or at
   index_pending refuses public same-ID refresh, internal read and reopen, with
   **all snapshotted authority/discovery rows, owner fields and Git unchanged**.

   Legitimate Task 4 store-only fixtures retain execution_required=false, because
   they claim no executor integration. Existing executable v3 records upgrade to
   true. Known legacy progress with lost execution authority causes refusal and
   transaction rollback rather than manufactured evidence. Two new migration tests
   cover store-only/executor preservation, injected marker-insertion rollback and
   lost-legacy-progress refusal. No old fixture assertion was weakened or relabeled.

3. **P1: later journal failure erases known effects — addressed.** The old final
   evidence-read error path returned an empty report and replaced the drive error.
   `Owner` now retains its last graph-validated execution evidence/phase plus
   positive in-call Git completions (object import, checkout and ref transition).
   These caches are **reporting evidence only**, never write/takeover authority.
   Failed final reads fall back to that knowledge and preserve the original drive
   error. A planned, not-yet-imported merge OID no longer masquerades as an actual
   commit receipt. Completed effects remain Partial/Incomplete where work failed.

   A persistent typed PermissionDenied injection at the actual owned journal
   access seams denies writes **and the final evidence read**. Counters prove the
   final read failed; this is not merely the old before-index failure hook with
   a readable store. **Ten cases** cover FF/merge checkout, ref and index failures,
   pre-effect controls, and merge before/after object import. They assert known
   phase/OID/effects, original discovery_pending/index_pending after index failure,
   no fabricated writes/commits before effects, privacy-safe diagnostics and exact
   retry after access is restored. This is portable typed I/O injection, not a
   claim of real OS ACL denial on native platforms.

4. **P2: stored disposition trusted without graph proof — addressed.** Readable,
   valid-shaped `noop` could previously replace accepted fast_forward, falsely
   finishing already_applied. Execution evidence is now checked against the frozen
   parent graph inside the fenced store transaction before any effect/settlement,
   and again before evidence is cached or used for final/replay reporting. This
   adds no fresh-primary observation and cannot change the frozen pair. **Sixteen
   pending corruption cases** cover both kinds and equal/contained/FF/merge graphs
   against every wrong valid disposition, asserting unchanged rows, epochs, files
   and logs. Two completed-noop controls also refuse corrupt replay without
   fabricating complete integration effects.

### Actual RED/GREEN record

All rows used the prefix
`devenv shell -- cargo test --all-features --locked --lib` with the single filter
shown below. New tests are confined to `local_sync/run_tests.rs`.

| Filter | Before fix | After fix |
| --- | --- | --- |
| `spec_same_tree_integrations_transition_once_without_claiming_file_writes` | 1 failed: empty-tree FF returned InternalError at CommitPrepared. | 1 passed, all 12 matrix cases. |
| `spec_missing_execution_authority_refuses_public_refresh_without_settlement` | 1 failed: public generic refresh accepted the lost-execution handoff. | 1 passed, four kind/phase compositions. |
| `spec_execution_authority_upgrade` | 2 failed at the missing new authority-column seam; these are schema-seam REDs, not a claim that the migration rollback assertion ran before the schema existed. | 2 passed: preservation, rollback and lost-progress refusal cases. |
| `spec_journal_access_loss_preserves_known_effects_and_original_drive_error` | 1 failed first at pre-effect phase loss; reordered the matrix and reran before implementation: 1 failed after actual FF checkout, with owned file bytes positively verified and final read failure counted. | 1 passed, all 10 access-loss/control cases. |
| `spec_corrupt_` | 2 failed: pending FF→noop returned AlreadyApplied; completed noop→FF replay returned Ok. | 2 passed, 18 corrupt-disposition cases. |

The v4 migration's first implementation run failed fixture creation because SQLite
inserts an ALTER-added column before table-level CHECK constraints. Inspected the
actual fixed schema definition, corrected the expected exact layout, removed the
temporary schema-only diagnostic, and reran both migration and missing-authority
tests green. No parser, constraint or assertion was relaxed. No unrelated test
failure was encountered.

### Final verification

Whole-tree commands on the final Rust tree, all exit 0:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

Check finished in 3.10s; clippy in 13.91s with no warnings. Test commands use
`devenv shell -- cargo test --all-features --locked`:

| Selection | Passed |
| --- | ---: |
| `--lib local_sync::` | **88**, 0 failed/ignored/measured, 1284 filtered; 184.42s. Includes 81 prior plus all 7 new SPEC tests and the existing eight actual no-Drop child exits. |
| `--lib repository::integration::tests` | 3 |
| `--lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | 1 |
| `--lib external_repair_exact_ordered_parent_clean_only` | 1 |
| `--lib clean_merge_integration_fault_matrix_resumes_each_durable_transition_once` | 1 |
| `--lib cancellation_with_a_pending_conflict_is_a_recoverable_stop` | 1 |
| `--lib repository::registration_removal::tests` | 20 |
| `--test repository_enablement remove_registration` | 8 |
| `--doc local_sync` | 4 compile-fail API-boundary controls |

Unique selected total: **115 library + 8 integration + 4 compile-fail = 127**.
Red/green repeats and child-process assertions are not added to that total. Fixture
matrix long-running notices completed successfully. No full suite or native run.

Changed Rust files are only `local_sync/{record,recover,run,run_tests}.rs`. The v4
marker/audit is production-active; executor and exclusively executor-only helpers
retain the controller-approved narrow test gating. **Task 7 must still activate
real production mutation/resume callers, remove exclusive executor gates, retain
fixture-only access hooks as test-only, enforce F2 request attempts and rerun
production-boundary tests plus whole-tree gates.** No public bypass or new lint
allowance was introduced. Task 6 and all previously listed conflict/resolution,
request-control and broader closure/native outcomes remain outstanding.

Start and checkpoint fetch/merge found main `0464c42dae19a6dde000c7e7660e82762a41fada`
already incorporated. Entry was clean at the fix base; merge-tree succeeded with
base tree `9b39ee2d8a1568c1f27f485e97921cc2ebce45f5`. Status/diff/log were inspected;
`git diff --check` passed. Ticket remains implementing; no owner question, subagent,
Task 6 work, PR, lifecycle action or native dispatch. Next gate: controller SPEC
re-review of all four fixes, then QUALITY on the full Task 5 range.

## Task 5 QUALITY corrections: awaiting re-review

Recorded `2026-10-26T14:44:44Z`, comment `01M5WAQG5RHQ4ZG571AB2DDCZA`.
Base **`f2823350120710cfaea4bb74dba4b2062d523899`**. The controller reports SPEC
**`ses_e87b8a702ffe6HryC0itCXg4H5` approved at this base with zero remaining
findings**. QUALITY **`ses_e878dbd29ffednm05pK3O5zlGc` requested three P1 and one P2
corrections** below. Each was verified in source before its failing regression.
These are implementer dispositions pending independent re-review.

### Verified findings and corrections

1. **P1, raw blobs versus filtered checkout images — addressed.** The original
   byte proof compared worktree content only to raw ODB blobs, while the backend
   applied filters. A narrow shared `integration/filtered.rs` helper now uses the
   **already-linked libgit2 public filter-list API in TO_WORKTREE direction** to
   compute the actual checkout image in memory. The git2 0.20 wrapper lacks this
   API, so the helper contains bounded C ABI declarations and paired RAII cleanup,
   with documented handle/ABI lifetimes. No dependency or Git subprocess was added.
   Neither input nor output bytes are stored or diagnosed.

   Proof accepts exact literal blob bytes (a legitimate clean LF baseline) or
   exact checkout-filtered bytes. It retains no-follow filesystem reads and complete
   index comparison; it does not rely only on cached stat/status or a lossy
   clean-filter roundtrip. **16 cases** cover both kinds, FF/merge, autocrlf=true or
   attributes eol=crlf, and LF/CRLF baselines, with actual CRLF checkout followed by
   retry, one ref effect and unchanged user configuration. A further ident test
   positively proves that a forged expansion cleans to the same blob in a memory
   ODB, while the executor's explicit byte proof refuses and preserves it.

2. **P1, introduced unsupported modes accepted before impossible proof — addressed.**
   Preparation validates every prospective mode before accepting ownership. The
   bounded executor currently proves regular 100644/100755 files; symlinks and
   gitlinks are refused before journal/Git effects using existing
   recovery_required / unsupported_conflict (unsupported structural integration).
   It does not label the target dirty or claim an installed conflict. **Eight
   document/ticket × FF/merge × symlink/gitlink cases** prove no owner, no target
   artifact, unchanged index/files/refs and successful unrelated refresh afterward.
   This is the contract-consistent preacceptance unsupported refusal authorized by
   the task, not a new product policy or a new global enum.

3. **P1, partial FIRST-file damage reported unchanged — addressed.** Source inspection
   of linked libgit2 1.9.7 `checkout.c:1520–1580` confirmed destination open with
   O_CREAT|O_TRUNC before filter loading and stream writes. The old progress probe
   only recognized completed output blobs/deletions. The executor now durably
   records a single aggregate digest of the actual pre-checkout byte/presence
   images of the changed-path set, under its lease and before the backend call.
   A later observed difference proves a filesystem effect even if the first file
   is empty or contains only a prefix. This is **effect evidence only**, not a
   per-path journal or permission to resume an ambiguous partial checkout.

   Additive marker `local_context_sync_checkout_image_v5` adds the nullable,
   shape-audited digest field. Migration preserves old history and leaves unknown
   preimages NULL rather than manufacturing them; legacy full-image proof remains.
   Upgrade/rollback and malformed-digest refusal tests pass. There is no new
   general checkout engine, reset or automatic path repair.

   **Actual first-file window proof:** an isolated Unix child arms RLIMIT_FSIZE
   immediately before the real libgit2 checkout, after SQL intents and authority
   checks. Limits 0 and 3 cause actual truncation or a three-byte prefix during the
   first file's backend write. SIGXFSZ either invokes async-signal-safe `_exit(86)`
   without any Drop, or is ignored so the actual OS write error returns through
   libgit2; returned-error children restore their own limit before further SQL.
   Parent snapshots verify the first file's exact damage, untouched second file,
   unchanged index, unchanged ref and no completed ref-log effect. Both kinds are
   covered: **four actual no-Drop exits plus four returned-error child cases**.
   FF has no made checkpoint, so Written / Partial comes from the observed file
   damage, not a commit effect. Retry preserves/refuses the ambiguous image; explicit
   fixture/operator baseline restoration then permits exactly one FF/ref effect.
   No fixture writes the damaged partial bytes on the executor's behalf.

4. **P2, disabled reflogs strand an already-moved ref — addressed.** Before accepting
   any ref-changing operation, preparation verifies that effective Git logging can
   provide this executor's required branch and HEAD append authority. The same
   prerequisite is re-read from a fresh configuration handle before effects on
   accepted work. Explicit false refuses; normal nonbare default and `always` are
   supported. No-op/contained needs no append and remains valid with logging off.
   No user configuration is changed. Refusal uses existing recovery_required /
   authority_missing: configured backend recovery authority is unavailable, rather
   than incorrectly calling valid Git configuration an invalid Manyhands config.

   **16 cases** cover both kinds, new FF/merge refusal, changes after acceptance or
   checkout, explicit caller re-enable/retry, and equal/contained controls. Three
   additional controls cover unset/default, `always`, and an in-call disable after
   checkout; the ref stays unchanged until the caller re-enables logging. Previously
   completed effects remain truthful. No new result/recovery enum or policy question.

### RED/GREEN commands and supplementary controls

Each core finding used
`devenv shell -- cargo test --all-features --locked --lib <filter>`:

| Filter | Observed RED | GREEN |
| --- | --- | --- |
| `quality_filtered_checkout_bytes_are_proven_for_lf_and_crlf_baselines` | 1 failed: actual CRLF checkout succeeded, but retry returned RecoveryRequired/AmbiguousBackend. | 1 passed, all 16 matrix cases. |
| `quality_introduced_symlink_and_gitlink_refuse_before_acceptance` | 1 failed: a pending owner existed for unsupported incoming mode. | 1 passed, eight cases. |
| `quality_first_file_partial_write_is_partial_without_granting_resume_authority` | 1 failed **after actual child `_exit(86)` and verified first-file truncation**: retry reported Write Unchanged with no checkpoint. | 1 passed, eight child cases, four of them actual no-Drop exits. |
| `quality_disabled_reflogs_refuse_before_effects_and_recheck_accepted_work` | 1 failed: disabled logging still installed a pending owner. | 1 passed, all 16 cases. |

The supplementary group `local_sync::run_tests::quality_` selects **seven** tests;
the nested first-write parent/child live under `checkout_write_tests` and are
included by the full local selection. That group initially had 5 pass / 2 fail from
new control-fixture assumptions: libgit2 status did not report the forged ident
file clean, and a stale config writer retained its cached true value after another
handle wrote false. Replaced the first assumption with an actual memory-ODB clean
filter positive control and reopened the second fixture's configuration writer.
The group then passed 7/7. No product assertion was weakened. The function-pointer
signal cast warning was corrected using an explicit pointer cast, not suppressed.

Additional tests cover v5 migration preservation/rollback, three malformed digest
forms, exact ident proof, and supported/default plus in-call logging behavior.
These are supplementary proofs, not additional claimed pre-fix RED executions.

### Final verification and review boundary

Required gates, all exit 0 on the final Rust tree:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib local_sync::
```

Check: 4.41s; clippy: 17.55s, no warnings. Local selection: **97 passed**, zero
failed/ignored/measured, 1284 filtered, 238.86s. This includes 88 previous tests plus
nine new QUALITY tests; one new selected test is the child entry's normal no-op
invocation, not an additional crash proof. Long-running fixture matrices finished
successfully. The local selection now proves **12 actual no-Drop exits** (eight
prior plus four first-file exits); child assertions are not double-counted.

Additional commands use `devenv shell -- cargo test --all-features --locked`:

| Selection | Passed |
| --- | ---: |
| `--lib repository::integration::tests` | 3 |
| `--lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | 1 |
| `--lib external_repair_exact_ordered_parent_clean_only` | 1 |
| `--lib clean_merge_integration_fault_matrix_resumes_each_durable_transition_once` | 1 |
| `--lib cancellation_with_a_pending_conflict_is_a_recoverable_stop` | 1 |
| `--lib repository::registration_removal::tests` | 20 |
| `--test repository_enablement remove_registration` | 8 |
| `--doc local_sync` | 4 |

Unique total: **124 library + 8 integration + 4 compile-fail = 136 passing tests**.
No full suite or native workflow. The first-file RLIMIT/SIGXFSZ fixtures are Unix
tests executed on Linux; no Windows/macOS runtime or ABI execution claim is made.

New files are `integration/filtered.rs` and `local_sync/checkout_write_tests.rs`.
Modified Rust files are `integration/mod.rs`, `local_sync/{record,recover,run,run_tests}.rs`
and `repository.rs` (two narrow backend-boundary failure points). No dependency,
frontend, public bypass, global result/recovery enum, test target or old semantic
assertion changed.
The filter helper is an exclusively executor-dependent, narrowly test-gated module.
**Task 7 still must activate production execution and its helper gates, keep fixture
hooks/tests test-only, add F2 attempt authority, and verify production boundaries
with whole-tree gates.** Source/privacy/zero-transport controls remain green.

Start/checkpoint fetches found main `0464c42dae19a6dde000c7e7660e82762a41fada`
already merged. Entry was clean at the fix base. Merge-tree passed with base tree
`c164c594ccb3ac1b703d590af07c55e46342add9`; status/diff/log inspected and diff checks
passed. Ticket remains implementing. No owner question, subagent, Task 6 work,
PR/lifecycle action or native dispatch. Await controller QUALITY re-review and SPEC
verification of these changed fixes; prior SPEC approval applies to `f282335` only.

## Task 5 SPEC image verification corrections

Recorded `2026-10-26T16:07:19Z`, comment `01M5WFEQNTFFPE9ZHKK82ABC1C`.
Base **`2b9ba0cb72850e478094c310f92b2406c8221acc`**. SPEC verifier
`ses_e87b8a702ffe6HryC0itCXg4H5` confirmed the preceding four fixtures were fixed,
but found **two further P1 image gaps**. The controller also reports that the
reviewer verified the filter helper's unsafe signatures, lifetimes, layout and
cleanup; `integration/filtered.rs` is unchanged in this correction.

Both findings were reproduced before correction. No new product policy or owner
question was needed; these are internal changes to the accepted exact-image proof.

### 1. Frozen attribute/image authority — addressed

Verified root cause: the old helper's commit argument supplies a repository owner;
it does not select the attribute tree. Safe checkout can replace `.gitattributes`
while retaining a CRLF file with unchanged blob/mode. Comparing that file to raw LF
or a fresh LF-filtered image cannot prove the operation's own legitimate result.

Added `local_sync/image_proof.rs` and additive, exact-shape-audited
`local_context_sync_frozen_images_v6` / `local_context_sync_images` evidence.
Creation is atomic with the accepted envelope/execution record. The manifest stores
only versioned paths, OIDs, modes and digests, bound to the ordered accepted parents,
expected result tree, generation and owner epoch. No source bytes are persisted.
The public observation/read API and global enums are unchanged.

The initial actual baseline image is captured after clean-input proof. Prospective
filtered images are enumerated using private repository handles with an empty
temporary workdir and an in-memory index selecting the **accepted baseline or
candidate attribute tree**. The ODB is memory-backed; `set_workdir(false)` changes
only the private handle and writes no user configuration. No source file is
materialized and no checkout runs in this preview. Both frozen filter alternatives
are retained because libgit2 may cache baseline attributes during safe preflight.

Accepted retry uses this immutable evidence, not live attribute reinterpretation:

- An unchanged blob **and** mode may retain its exact trusted baseline image.
- Changed entries must match an enumerated prospective image; a changed blob cannot
  borrow a baseline digest merely because its path is the same.
- Complete index/tree/path-set binding and no-follow file proof remain mandatory.
  A missing after-only entry is refused before checkout, not discovered after writes.
- Current external filter inputs are re-evaluated against the frozen alternatives
  before effects. Fresh foreign tracked attributes and `.git/info/attributes`
  changes refuse; they cannot redefine an accepted result.
- Forged ident expansions still fail exact byte proof, even when Git cleaning
  would collapse them to the same blob. Existing same-tree and filtered tests pass.

The image manifest is immutable input evidence, **not a per-path checkout journal**:
there is no path-progress resume, rollback, reset or automatic repair. Legacy
unfinished operations lacking this new proof are preserved/refused rather than
having authority reconstructed from current files; completed historical replay
does not restart Git work. Missing/corrupt/version-inconsistent proof is tested.

### 2. Mode-aware coarse effect evidence — addressed

Verified root cause: v1 coarse hashes bind paths, presence and bytes but no physical
executable mode. Actual mode-only checkout progress can therefore compare equal to
the preimage despite a durable filesystem change.

Added `local_sync/file_image.rs`: on Unix it reads the regular file's bytes and
normalized Git executable mode through the **same descriptor**, with pinned
no-follow parents and leaf. It checks metadata stability across the read. Windows
uses the existing safe byte reader and explicitly has no portable Unix-mode value;
this is not Windows ACL or executable-bit evidence.

New coarse images use domain `manyhands-local-coarse-checkout-image-v2`, including
the verified mode (or explicit platform-unavailable tag). They live in the separate
`baseline_v2` field of the new image table. The existing `baseline_worktree` v1 field
and v1 domain are unchanged, so an old hash is never silently compared to a v2 image.
The manifest's coarse-version discriminator is audited: version 1 cannot carry a
v2 digest, and a started version-2 checkout requires its v2 baseline. Unknown versions,
missing v2 authority and inconsistent combinations refuse. A valid legacy v1 control
shows no invented write effect from a format/domain change and completes normally.

**Actual mode-only no-Drop proof:** document and ticket FF fixtures change only
100644→100755 with identical blob bytes. The child exits without Drop after the
backend recreates/chmods the path but before the index is persisted. Parent snapshots
positively prove the new executable mode, unchanged bytes, old index and old ref.
The mode-aware probe reports Written; retry returns Partial/AmbiguousBackend and
preserves the mode instead of treating a partial image as baseline/result authority.
Only explicit fixture/operator mode restoration permits retry and one ref effect.
These two executions are Linux evidence; no Windows/macOS runtime claim.

### TDD and additional proof

All library commands used
`devenv shell -- cargo test --all-features --locked --lib <filter>`:

| Filter | RED before correction | GREEN evidence |
| --- | --- | --- |
| `spec_attribute_changes_preserve_trusted_unchanged_images_and_filter_changed_blobs` | 1 failed: document FF CRLF→LF with unchanged blob remained Applying/AmbiguousBackend after its own checkout. | 1 passed, **16 cases** across both kinds, FF/merge, both EOL directions and unchanged/modified blobs. |
| `spec_mode_only_no_drop_is_partial_and_requires_complete_image_proof` | Corrected-window run: 1 failed after actual no-Drop, verified executable mode and unchanged bytes/index/ref; progress probe returned false. | 1 passed, **two actual mode-only no-Drop child exits**, truthful Written/Partial and operator-proof retry. |
| `spec_missing_after_only_image_refuses_before_checkout` | 1 failed: an incomplete prospective path set was caught only after a file write. | 1 passed; complete result-tree/path-set validation precedes checkout and merge-object import. |

The first mode-child setup exited at libgit2's removal progress event, leaving the
path absent. That was not counted as the requested mode RED. The fixture now selects
the existing progress boundary only after the executable mode is actually present;
then the intended byte-only-digest failure was reproduced before its fix.

Five supplementary tests cover: retained ident after an attribute change; two fresh
foreign attribute sources; six manifest-loss/corruption/version cases; metadata-only
privacy plus refusal to reconstruct lost proof; and separate v1/v2 domains with valid
legacy handling. These controls are not claimed as independent pre-fix REDs.

An initial clippy run found the runtime-only baseline field unread by the production
auditor, a complex stored-row tuple type and a now-redundant `Ok(...?)`. The auditor
now decodes and validates the same typed baseline field; a private row alias and the
direct result remove the other findings. No lint allowance or public scaffold was
added. No old semantic assertion was weakened.

### Final verification and handoff

Final whole-tree gates, all exit 0:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib local_sync::
```

Check: 2.67s; clippy: 10.40s, no warnings. Final local selection: **105 passed**,
zero failed/ignored/measured, 1284 filtered, 256.91s. This includes 97 prior tests
and eight new tests. Same-tree, filtered, ident, first-file partial-byte, old journal
migration, source-privacy and transport-spy assertions all pass unchanged. Long-running
matrix notices completed successfully. There are now **14 actual no-Drop exits**
inside the parent tests: twelve prior plus two mode-only cases; not double-counted.

Additional relevant checks used `devenv shell -- cargo test --all-features --locked`:

| Selection | Passed |
| --- | ---: |
| `--lib repository::integration::tests` | 3 |
| `--lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | 1 |
| `--lib external_repair_exact_ordered_parent_clean_only` | 1 |
| `--lib clean_merge_integration_fault_matrix_resumes_each_durable_transition_once` | 1 |
| `--lib cancellation_with_a_pending_conflict_is_a_recoverable_stop` | 1 |
| `--lib repository::registration_removal::tests` | 20 |
| `--test repository_enablement remove_registration` | 8 |
| `--doc local_sync` | 4 |

These shared/registration paths were checked before the final local-only path-set
tightening; they were unchanged by it. Unique selected total: **132 library +
8 integration + 4 compile-fail = 144 passing tests**. No full suite or native run.

Rust scope: new `local_sync/{image_proof,file_image}.rs`, modified
`local_sync/{mod,record,recover,run,run_tests,checkout_write_tests}.rs`. The already
reviewed unsafe filter helper is unchanged. The image-table codec/audit is active
in production; executable proof construction and file-image helpers retain narrow
test gates. **Task 7 must activate them with real production mutation/resume callers,
preserve fixture-only hooks, enforce F2 authority and run production-boundary tests
and whole-tree gates.** No new dependency, frontend, global result/recovery enum,
test target, public bypass or Task 6 implementation.

Start/checkpoint fetches found main `0464c42dae19a6dde000c7e7660e82762a41fada`
already merged. Entry was clean; merge-tree passed with base tree
`e4463e4c46b80acef277803ded9cd2247ee4ae5a`. Status/diff/log inspected and diff checks
passed. Ticket remains implementing. No owner question, subagent, PR/lifecycle action
or native dispatch. Await SPEC verification of these two corrections, then QUALITY
re-review of the complete Task 5 range; no independent approval is claimed here.
