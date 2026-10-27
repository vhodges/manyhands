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
in `01M5WFEQNTFFPE9ZHKK82ABC1C`. SPEC at `ef746ba` reports those two findings resolved
and identifies one further P1: a retained image could borrow an unbound before-side
OID. Its verified correction is recorded in `01M5WHV3951QADA81HKTX5B7XY`.
The controller subsequently reports **full Task 5 SPEC approval at `3177dba`**.
QUALITY `ses_e878dbd29ffednm05pK3O5zlGc` resolved its original four findings but found
one new P2 worktree-attribute-source gap. Its tested correction is recorded in
`01M5WNVFNPVN42302WYENZCYEB`; independent re-review remains pending, and the SPEC
approval is not automatically extended to the correction. **Superseding controller
handoff for Task 6:** SPEC `ses_e87b8a702ffe6HryC0itCXg4H5` and QUALITY
`ses_e878dbd29ffednm05pK3O5zlGc` both **APPROVED, zero outstanding findings at
`7283ad210abbb0e9f6fd108362c48d488ee7fad4`**, over the full Task 5 base `4377944`
(implementation-code base `7856c619`). Its final recorded evidence is 113 local,
7 shared and 4 compile-fail tests, and 14 actual no-Drop exits. These are controller
review reports, not reviews performed by this implementer.
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
| Task 5 | **Complete and approved at `7283ad210abbb0e9f6fd108362c48d488ee7fad4`**, full range from `4377944` (code base `7856c619`), both controller reviews above, zero outstanding. 124 selected tests and 14 actual no-Drop exits; Task 7 activation obligation retained. |
| Task 6 | **Complete and approved at `41bce3eb23ae4e95316f287cd832d38be00164aa`**, full range `7283ad2..41bce3e`. Task 7 controller handoff reports SPEC `ses_e864b2dfbffemfrKBcgnvISO55` and QUALITY `ses_e8613a466ffe3Dlw50236N05X9` both approved with zero findings. Latest recorded verification: **159 local + 4 shared = 163 passing tests**, including **21 actual no-Drop exit assertions**. This supersedes the historical pending-review entries below. |
| Task 7 | **Complete and approved at `0d1b8d006980bf0da116828db54b16728bf55cbd`**, full range `41bce3e..0d1b8d0`. Task 8 controller handoff reports SPEC `ses_e8554c7d2ffe8o3xfJnBqcRrQO` and QUALITY `ses_e846d6706ffeMZlWpMYjwDMnGU` both approved with zero findings. Prior 182 selected checks remain recorded evidence. |
| Task 8 | **Complete and approved at `03a27cbb7252062429aabfed31ae6c35e2f143fb`**, full range `0d1b8d0..03a27cb`. Task 9 controller handoff reports SPEC `ses_e84247053ffeQkt4obeGjIa5Xy` and QUALITY `ses_e84216efcffeqZ4Q0rR0960XJX` both approved with zero findings. This includes the bounded-child/Windows-lock correction; historical 243-test and six-test delta evidence remain separately scoped below. |
| Task 9 | **In progress; final controller gates and independent whole-branch review pending.** Documentation/source audit checkpoint `01M5YF4JK2VQ94PE52V7T91TJJ`, base `03a27cb`. This worker runs no Rust gates. Ticket remains implementing, not review-ready or closed. |

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

## Historical Task 0 required outcomes: test owners, not then executed facts

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

## Task 5 SPEC: complete two-sided image bindings

Recorded `2026-10-26T16:49:02Z`, comment `01M5WHV3951QADA81HKTX5B7XY`.
Fix base **`ef746ba7d060e8b0d087213998f4c656c1f2dd13`**. Controller reports SPEC
`ses_e87b8a702ffe6HryC0itCXg4H5` resolved the prior two P1s but found one new P1
in their composition. This is an implementer correction pending independent review.

### Source finding and centralized correction

Verified: `matches` returned false on a live-index mismatch before checking the
selected mapping, then candidate proof could use a `before` digest whose OID had
not been bound to the frozen context tree. `filters_match` checked only the after
mapping. After FF checkout, changing only before.oid to after.oid and restoring
old file bytes could therefore move the ref and settle despite a dirty old file.

The complete mapping gate is now `Proof::validate_bindings`. Both full path/OID/mode
projections are encoded as canonical Git trees and hashed with Git's tree object
hashing. The before projection must equal the immutable frozen context commit's
tree; the after projection must equal the accepted result tree, which candidate
proof also binds to the actual candidate. The complete after projection is checked
at decode/audit too. Duplicate/prefix-conflicting paths cannot form a valid tree.

This is metadata-only validation: it reads no blobs or filters, creates no objects,
and reconstructs no merge. Directory processing is iterative and uses Git's
directory-versus-file ordering. Projection hashes are computed once per immutable
decoded proof and cached in a nonserialized Cell; stored input cannot provide the
cache. Removed the old partial selected-side and after-only mapping checks.

`Execution::validate_graph` invokes the complete gate before an owner transaction
can take over, write intents, cache authority or settle. The same gate precedes
both `matches` and `filters_match`, so early live-index mismatches and retained-image
shortcuts cannot skip either side. Baseline digest trust remains limited to its
verified immutable tree identity under the cooperative-writer contract.

### TDD and exact composition evidence

Command:

```sh
devenv shell -- cargo test --all-features --locked --lib spec_both_manifest_sides_bind_before_reconciliation_or_owner_change
```

- **RED:** one test failed. The real FF had changed `ordinary`, stopped after
  checkout with candidate index and parent ref, then the fixture changed only
  before.oid to after.oid, retained the original baseline digest and restored old
  bytes. Retry incorrectly returned Ok/Complete/integration complete.
- **GREEN:** one test passed, exercising **eight cases** across documents/tickets
  and before OID, before mode, missing before path and missing after path.
  The before variants retain the candidate index; the after-path variant retains
  baseline index to exercise the opposite early-return path. Every refusal preserves
  authority/discovery rows, owner epoch, intents, manifest bytes, files, index,
  context/primary/tracking refs and reflog counts. No takeover or settlement occurs.
- Supplementary `spec_complete_bindings_match_git_directory_and_file_ordering`
  passes a genuine FF/replay with nested paths, Unicode, and ordering-sensitive
  siblings such as `a.c`, `a/…`, `a0`, `a/sub.c`, `a/sub/…`, `a/sub0`. It checks the
  projection against actual Git-created trees, not a second copy of the algorithm.

No pre-existing assertion was weakened. No new schema or stored-format migration
was needed: validation is stronger; the in-memory cache is excluded from storage.
The previous attribute, ident, same-tree, partial-byte, mode-only, legacy and
no-Drop tests all remain green.

### Final verification and handoff

All required Devenv gates exited 0:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib local_sync::
```

Check: 3.78s; clippy: 14.10s, no warnings. Local tests: **107 passed**, zero
failed/ignored/measured, 1284 filtered, 287.28s. This includes 105 previous tests
plus the two new tests. Long-running matrices completed normally; their 14 actual
no-Drop exits remain parent-test assertions, not extra test counts.

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

Unique selected total: **134 library + 8 integration + 4 compile-fail = 146**.
No full suite or native run. Rust edits are confined to
`local_sync/{image_proof,recover,run_tests}.rs`. Unsafe filter code, executable
staging gates, dependencies, frontends and global result/recovery enums are unchanged.
Task 7 still owns production activation, real F2 callers/attempt authority and
production-boundary verification. No Task 6 implementation or owner question.

Entry was clean at the fix base. Start/checkpoint fetches found main `0464c42` already
merged; merge-tree passed with base tree `6fdeb937f58d9f9314f757248295bfe2b8edb689`.
Status/diff/log inspected and diff checks passed. Ticket remains implementing;
no subagent, PR/lifecycle action or native dispatch. Await SPEC verification of
this correction, then QUALITY re-review of the full Task 5 range.

## Task 5 QUALITY: unowned worktree attribute sources

Recorded `2026-10-26T17:59:09Z`, comment `01M5WNVFNPVN42302WYENZCYEB`.
Base **`3177dba4a44f8f13abeb8aa8ac936942eca4151d`**. Controller reports full Task 5
SPEC approval at this head. QUALITY `ses_e878dbd29ffednm05pK3O5zlGc` reports its
original four findings resolved and one new P2. This checkpoint records the new
finding's implementer correction, not reviewer approval of the changed range.

### Verification and minimal correction

Confirmed in source: frozen previews select tree attributes through an empty
workdir/in-memory index, while actual checkout reads real worktree `.gitattributes`
before index fallback. An ignored source absent from both frozen trees can transform
incoming LF bytes to CRLF, even though target status is clean and no prospective
path collision exists. The previous executor could accept ownership and perform
checkout before discovering that its own result was outside the frozen proof.

Added a narrow `Proof::require_worktree_attributes` guard. It enumerates only
`.gitattributes` paths in the ancestor directories of the frozen file set (root
and applicable nested sources), not all ignored files or a recursive directory
scan. An existing applicable source must be a no-follow regular file, tracked in
the live index, with its OID/mode belonging to a frozen before/after entry. Otherwise
the executor refuses using existing recovery_required / authority_missing: this
is an unrepresented filter input, not an installed conflict or invalid Manyhands
configuration. Source files and symlinks are preserved, not removed or tracked.

The guard runs before accepting ref-changing work, during the existing before-effect
filter checks, and again after the object-import interruption boundary before any
candidate objects are imported. Accepted retries therefore cannot silently adopt a
new ignored source. Historical effects remain truthful where checkout previously
completed. Tracked attribute transitions continue through their existing frozen
image proof; `.git/info/attributes` continues through the existing external-filter
validation. No source proof format, schema, FFI or configuration change was needed.

Equal/contained operations need no checkout source and remain valid. Ordinary
ignored files and ignored attribute files in directories outside the frozen path
ancestry remain allowed. Ordinary nonignored untracked attributes retain the
existing dirty-worktree refusal. This is the contract-consistent unsupported-input
guard requested by the controller, without a new product policy or owner question.

### RED/GREEN and controls

All library commands use
`devenv shell -- cargo test --all-features --locked --lib <filter>`:

| Filter | RED / control before implementation | GREEN |
| --- | --- | --- |
| `quality_ignored_worktree_attributes_refuse_before_acceptance` | 1 failed: ignored source produced a pending owner. A real-worktree filter positive control had already proved CRLF versus the committed LF blob. | 1 passed: eight kind × FF/merge × root/nested cases, no owner or Git effect, preserved source and successful unrelated refresh. After refusal assertions, an independent actual safe checkout confirms the CRLF transformation. |
| `quality_new_worktree_attribute_sources_cannot_change_accepted_execution` | 1 failed: after acceptance the executor wrote files before rejecting its result as ForeignChange. | 1 passed: eight kind × source-location × accepted/checked-out cases; no new Git effect, historical effects retained, exact retry completes after caller removes the source. |
| `quality_ignored_symlink_attribute_sources_are_preserved_and_refused` | 1 failed: execution wrote files before rejecting the result. | 1 passed: root and nested symlink sources preserved, outside target bytes unchanged, no owner/effects. Unix fixture executed on Linux. |
| `quality_ordinary_ignored_files_and_irrelevant_attribute_directories_remain_allowed` | 1 passed as an unchanged-behavior positive control. | 1 passed: four kind/graph cases preserve ignored data and unrelated ignored attribute files. |

Two supplementary tests pass: in-call source arrival at the before-object-import
and after-candidate boundaries (including proof that the merge candidate was not
imported), and ordinary untracked-source refusal plus equal/contained controls.
They are not counted as additional pre-fix REDs. Total new coverage: **six tests,
27 matrix/control cases**.

The first fixture attempt overwrote `.git/info/exclude`, accidentally dropping the
fixture's existing Manyhands-worktree exclusion and failing setup during a primary
commit. Corrected the new helper to append exclusions, then reran the intended RED.
That setup failure is not behavioral RED evidence. No old assertion was weakened.

### Final checks and handoff

All required whole-tree gates passed through Devenv:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

Check: 3.04s; clippy: 14.76s, no warnings. Final relevant tests use
`devenv shell -- cargo test --all-features --locked`:

| Selection | Passed |
| --- | ---: |
| `--lib local_sync::` | **113**, zero failed/ignored/measured, 1284 filtered, 280.32s |
| `--lib repository::integration::tests` | 3 |
| `--lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | 1 |
| `--lib external_repair_exact_ordered_parent_clean_only` | 1 |
| `--lib clean_merge_integration_fault_matrix_resumes_each_durable_transition_once` | 1 |
| `--lib cancellation_with_a_pending_conflict_is_a_recoverable_stop` | 1 |
| `--doc local_sync` | 4 |

Unique selected total: **120 library + 4 compile-fail = 124 passing tests**. Existing
tracked-attribute transitions, foreign-info attributes, immutable/private history,
same-tree, filtered/ident, legacy, privacy, no-transport and 14 actual no-Drop child
assertions remain green within the local group. No standalone storage/removal suite
was repeated for this execution-only guard; no schema/removal behavior changed.
No full suite or native run.

Rust edits are confined to `local_sync/{image_proof,run,run_tests}.rs`. The temporary
executor-only staging and Task 7 production activation/F2 obligations remain. No
Task 6 implementation, public bypass, dependency, frontend or global enum change.
Entry was clean; start/checkpoint fetches found main `0464c42` already incorporated.
Merge-tree passed with base tree `76000e7c2f650004b7972bb1db42d5db7bc58f2f`; status,
diff and log inspected, diff checks passed. Ticket remains implementing. No owner
question, subagent, PR/lifecycle action or native dispatch. Await independent
verification of this guard before Task 6.

## Task 6: canonical conflict resolution and safe domain recovery

Checkpoint `01M5WTE43WQ76N6VDJXB70519K`, ID/time issued at
**2026-10-26T19:19:14Z**; the final malformed-token correction and verification
continued after issuance. Base **`7283ad210abbb0e9f6fd108362c48d488ee7fad4`**.
Review from that base through the feature commit containing this checkpoint,
`feat: resolve and recover owned local context conflicts`.

### Authority and staging

The Task 6 handoff explicitly preserves the owner-approved plan, E4/E9 and narrow
key-ID guard scope, ticket-branch commits/pushes, and the controller's narrow
internal `cfg(test)` staging. No new staging approval was requested. Work remained
in this ticket worktree/branch, without subagents. Entry was clean. Start and final
fetch/merge found `origin/main` at `0464c42dae19a6dde000c7e7660e82762a41fada`
already incorporated; HEAD stayed at the supplied base. Final merge-tree exited 0,
base tree `41328daafed68b35880d236101608f9ca917c021`. Current main AGENTS was checked.

New files: `local_sync/{conflict,conflict_record,resolve,control,conflict_tests,
control_tests}.rs`. Modified: `local_sync/{mod,record,recover,run,image_proof}.rs`
and `bridges/abandon.rs`. All paths are under `src/repository/`.
The neutral integration helpers are reused directly; no remote orchestration or
old semantic test was changed. No Cargo/dependency/target, frontend, global
ResultCode/RecoveryActionKind or public execution bypass was introduced.

The real domain code performs Git/SQL/file effects under the common lease and
existing `Owner` authority checks. `conflict`, `resolve` and `control` join the
approved test-gated executor; production schema/audit/admission and the generic
abandon guard remain active. All-target clippy compiles these real domain paths.
This is not a claim that the public mutation commands or F2 protocol are delivered.

### Implemented sequencing and evidence

- Admissible canonical or mixed-code conflicts are installed through the neutral
  backend after durable intent. The content-free installed image binds complete
  index stages, raw paths, file digests/modes and owned merge-member digests to the
  frozen pair. Before acceptance, incompatible closures still refuse without an
  owner. Existing clean integration, two-sided image bindings, filtered-image and
  ref/reflog proofs remain in force. Conflict preparation/retry also retains the
  worktree-attribute-source guard.
- Additive marker `local_context_sync_conflicts_v7` versions the exact audited
  conflict and resolution-cancellation tables. Missing marker/table/required row,
  invalid generations/counters or broken associations refuse. The cancellation
  table cascades from the parent and audits its attempt association, preserving
  the existing legacy child-table upgrade/rollback contract.
- Conflict capabilities use versioned, length-delimited BLAKE3 over repository
  scope, operation, immutable generation, raw paths, modes and stage OIDs. Tokens
  select re-enumerated frozen entries, never caller-decoded paths. Malformed tokens
  are invalid_input; valid foreign tokens are external_change. Side strings come
  only from immutable ODB blobs; accepted partial attempts retain historical reads.
  Completed/abandoned operations invalidate those reads. Separate read sessions
  recheck registration/semantic facts and parent/active-R cancellation counters;
  Git image/HEAD and routing are rechecked afterward. No read-token table exists.
- Full canonical-set, all-side immutable-field and prospective-context dependency
  validation precedes acceptance/writes. Every path is preflighted before the
  first replacement. Attempts persist the top-level observation and canonical
  ordered token/source digest, per-path preimage/output digests and progress, never
  source text. Exact accepted input is matched before fresh-token validation;
  incomplete attempts require their original resolution request ID and bytes.
- Per-path replacements reuse no-follow/digest-guarded owned writes. Candidate
  intent freezes one ordered-parent OID and identity before import. Referenced
  objects are imported before the commit object; retry reconstructs the same
  candidate after an intent interruption, even after configured identity changes.
  Index pre/post images, original nonconflict entries, canonical invariants,
  candidate identity, context CAS and both expected reflog appends are checked.
- Metadata release preflights all members, records intent, treats an absent member
  as already retired, and removes only matching owned members. A foreign member
  refuses before retiring the remainder. Unknown backend locks are preserved for
  operator repair. No automatic reset, rollback, marker deletion or unsafe abandon
  exists. External repair requires a clean exact ordered-parent commit and the
  shared all-side invariant validation, and does not claim a made checkpoint.
- Domain S/active-R cancellation counters are monotonic. Safe points acknowledge
  the captured vector, retaining a concurrently newer issuance. Pre-effect stop
  is terminal; conflict/path/integration/index stops retain their phase and accepted
  intent. Final domain indexing serializes the cancellation check with parent,
  attempt and envelope completion. Exact continuation consumes acknowledged values
  without a cancellation loop. **These are domain counters, not F2 controls.**
- Internal abandonment proves either artifact-free Prepared or integrated/released
  IndexPending state, revokes epochs, and abandons parent/attempt without changing
  Git. It refuses an associated public operation owner. The production generic
  bridge has an explicit local-sync guard so its old request-deletion path cannot
  bypass the future all-associated disposition. Terminal domain resume cannot
  resurrect an abandoned operation.
- Positive in-call and last-proven evidence survive later journal access loss.
  The original failure is retained; durable effects remain Partial. Actual owned
  filesystem PermissionDenied maps to repository_inaccessible. Index failure maps
  to discovery_pending with integrated OID, and retry performs only discovery.

### Test-first and debugging record

Every command used Devenv and one positional test filter. Missing new read/resolve,
resume and control symbols first established API-seam compilation failures; these
are not counted as runtime recovery REDs. Runtime REDs before their corrections:

| Test/filter | Observed failure before correction | Final result |
| --- | --- | --- |
| `conflict_installation_is_owned_partial_for_both_context_kinds` | Unsupported preaccept refusal instead of owned ConflictPending. | Both kinds pass. |
| `mixed_conflict_requires_exact_ordered_clean_external_repair` | Valid external repair initially refused; then reversed parents exposed the adapter's missing ordered-parent check. | Exact clean parents pass; wrong order preserves/refuses. |
| `candidate_intent_rebuilds_same_identity_and_foreign_release_precedes_no_write` | Missing imported candidate could not resume with exact R. | Frozen candidate/identity retry and foreign-metadata no-write control pass. |
| `resolution_authority_loss_and_missing_companion_never_authorize_writes` | A write followed an injected owner-epoch loss. | Reauthorization at the actual write seam, plus missing marker/manifest/control controls pass. |
| `no_drop_conflict_backend_window_preserves_until_exact_operator_baseline` | Actual child exit left changed index/files but reply claimed Unchanged. | Written/Partial ambiguity, preserved owner and explicit operator-baseline retry pass. |
| `accepted_resolution_refuses_changed_mode_and_unrelated_untracked_before_more_effects` | Unrelated untracked content was accepted after candidate preparation. | Complete dirtiness/nonconflict proof refuses it before additional effects. |
| `conflict_preparation_keeps_frozen_attribute_source_guards` | Ignored unbound attributes admitted and installed conflict. | Owner-free/no-write refusal passes. |
| `cancellation_after_integration_and_at_index_handoff_keeps_proven_effects` | Cancellation arriving at index handoff finished Ok instead of stopping. | Both integrated and index-pending cancellation/continuation pass. |
| `only_artifact_free_domain_abandonment_can_revoke_authority` | Terminal prepared abandon resumed as RecoveryRequired instead of terminal cancellation. | Artifact-free cases and terminal replay pass. |
| `resolution_index_transaction_failure_preserves_integration_and_retries_only_discovery` | Actual index-transaction boundary failure returned RecoveryRequired. | DiscoveryPending/complete integration and discovery-only retry pass. |
| `conflict_read_rechecks_resolution_counters_in_a_fresh_session` | Active-R counter changed between sessions but read succeeded. | ExternalChange on the raced read; historical token remains usable afterward. |
| `unix_directory_permission_loss_keeps_partial_resolution_resendable` | Actual directory PermissionDenied became InternalError. | Typed local I/O mapping and exact resend pass, executed as UID 1000. |
| `resolution_journal_access_loss_retains_last_proven_effects` | Failed final evidence read erased actual earlier writes. | Path/candidate cases retain effects and resume after access restoration. |
| `ignored_resolution_takeover_does_not_authorize_a_path_write` | Ignored SQL epoch update still allowed resolution. | Required one-row fenced takeover refuses before writes. |
| `complete_preflight_refuses_every_invalid_set_before_any_path_write` | Final code-specific assertion exposed malformed token returning ExternalChange. | Malformed InvalidInput, foreign ExternalChange and no-write controls pass. |

Supplementary real proofs cover complete multi-path CRLF byte preservation in files
and blobs; serialized/reopened tokens; historical reads despite mutable-path edits;
same-ID source and top-level observation mismatch; all-side and ancestor-only
irreversible closures; affected comment dependencies; accepted-source privacy;
mode/path/metadata/lock refusal; rebuild/lost-registry preservation; and fault matrices.

Fixture/debug corrections were not policy changes: the first dependency probe used
an unknown `depends_on` key, which canonical parsing intentionally preserves; the
final test exercises actual comment-parent dependencies. The ancestor-only fixture
initially passed a commit handle from another repository handle into Git commit
creation; reopening the parent through each committing handle and asserting both
parent IDs fixed the fixture. New cancellation evidence initially referenced a
child table that legacy upgrade fixtures rename; changing its FK to the stable
parent plus audited attempt association preserved all old assertions. Clippy's
tuple-complexity finding was resolved with a private type alias, not an allowance.
An early 120-second local selection timed out and is not counted as passing;
subsequent complete selections used a 600-second tool limit.

### Design checkpoint: actual outcomes and retained owners

At the initial checkpoint the selected **Task 6 domain-owned** tests passed;
SPEC subsequently identified four missing compositions (whole-result preflight,
early backend evidence, import-before-observation recovery, and non-resolution
abandonment). Thus this is historical selected-test evidence, **not a current
all-domain-outcomes completion claim** until the corrections are reviewed. This table supplements,
and does not rewrite, the original 25 required outcomes or earlier evidence.
W/C/I/D below mean write/checkpoint/integration/discovery effects.

| Original row / boundary | Executed domain outcome and authority | Public/later obligation |
| --- | --- | --- |
| 1–3: intent, backend interruption, frozen pair | Task 5 controls remain green. New conflict backend child exit retains Applying and ambiguous Git evidence; explicit exact-baseline repair permits retry. | S acceptance/replay: Task 7; closure follow-on: Task 8. |
| 4–5: foreign state and bad complete set | Refuse before any new resolution path write, preserving files/index/refs and parent. Whole-set mixed conflict is external-only; malformed/foreign/duplicate/missing/stale and invalid immutable/dependency input refuse. | Public DTO/schema/error presentation: Task 7. |
| 6: after one path / missing bytes | ResolutionPrepared + same active attempt; W Written, C/I/D Pending. Resume names R with original_request_required; exact R completes once. Changed source or only observation gives request_mismatch with historical effects. | F2 R/U ownership and IDs: Task 7. |
| 7: candidate intent/import/index/ref | Before import, exact R reconstructs the frozen candidate. After import/index, C Committed, I Pending. After ref, Integrated with I Complete and release pending. Retry uses the same OID/one ref append. | Independent S/R settlement: Task 7. |
| 8, 14–15: cancellation | Prepared/no-effect stop is terminal Cancelled with no durable effects; conflict/path stops retain underlying phase/attempt. Integrated/index stops retain complete I/OID. Issued 2 versus acknowledged 1 survives the coordinated race; next retry acknowledges 2, then continuation completes. Unsafe abandon refuses. | Accepted S/R/U, reply-loss crashes, authoritative F2 counters and U takeover remain Task 7. |
| 9: output loss after completion | Domain R replay returns the completed parent evidence without new Git effects. | Both independent finished request receipts and lost settlement: Task 7, not claimed here. |
| 10–11: closure | Existing Closing admission/reciprocal-fence controls and all-side immutable validation remain green. | End-to-end closure refusal → local resolution → explicit adoption retaining closure_oid/fields is Task 8, still unproven here. |
| 12–13: rebuild/authority loss | Rebuild refuses pending real conflict/partial R without changing evidence. Replaced registry cannot resume or read historical sides; files/index/merge artifacts remain unchanged. | Full public three-way lost-journal distinction and registration behavior: Task 8. |
| 16: integrated/released abandon | IndexPending, W Written/C Committed/I Complete/D Pending. Artifact-free domain abandonment atomically marks parent/attempt Abandoned and revokes epoch; exact domain resume stays terminal. | Finish ALL Accepted S/R/U, including unlinked intended-parent contenders, atomically: Task 7. |
| 17–18, 23–25: acceptance/link/handover | Domain digest/attempt identity and owner loss are tested, with no new attempt on exact partial retry. | Public acceptance-before-link, stable-ID evidence, F2 handover, unlinked terminal dispositions and immutable request results are wholly Task 7. |
| 19–20: admission/closure disagreement | Task 4 reciprocal ownership and Task 5 preaccept immutable-disagreement controls remain green, without changed old assertions. | F2 acceptance disposition and broader public compositions: Tasks 7/8. |
| 21: backend ambiguity | Actual child exit after conflict backend effect, before observed image, preserves metadata/index/files and refuses automatic repair. Existing Task 5 actual checkout/ref/log/lock controls stay green. | Broader platform/adversarial coverage: Task 8/native later. |
| 22: owned-member retirement | Actual child exits **after MERGE_HEAD unlink**, leaving MERGE_MSG/MERGE_MODE. Resume observes absence, verifies and retires remainder, completes without another commit/ref effect. FOREIGN replacement refuses and survives. | Public request composition: Task 7; native runtime still unproven. |
| Permission loss | Mandatory portable typed PermissionDenied occurs at the actual second owned-write seam: two attempted writes, exactly one observed path. Partial repository_inaccessible retains R. Actual Unix directory denial corroborates it; restoration + exact R finishes once. | Windows ACL evidence is not claimed. |

There are **three new actual no-Drop child exits** (two owned-retirement cases,
including FOREIGN control, and one initial-install backend case), each asserted
exit status 76 without Drop. Together with Task 5's fourteen, the final local group
contains **17 actual no-Drop exits**. The ordinary child-entry test is not another
crash proof. Candidate/ref/path returned-fault tests are not mislabeled no-Drop.

### Final verification and Task 7 handoff

Final whole-tree Devenv gates all exit 0:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib local_sync::
```

Final local run: **142 passed**, zero failures/ignored/measured, 1284 filtered,
393.79s. This is the 113 existing local tests plus **29 new tests** (25 conflict/
resolution tests including one child entry, and four control tests). Check and
clippy finished in 0.30s and 6.57s respectively; no warnings. The earlier complete
run also passed 142 tests; the last change only corrected malformed-token result
classification and was followed by this fresh full local selection and all gates.

Additional targeted commands (each separate invocation, one positional filter),
prefix `devenv shell -- cargo test --all-features --locked`:

| Selection | Passed |
| --- | ---: |
| `--lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | 1 |
| `--lib external_repair_exact_ordered_parent_clean_only` | 1 |
| `--lib clean_merge_integration_fault_matrix_resumes_each_durable_transition_once` | 1 |
| `--lib cancellation_with_a_pending_conflict_is_a_recoverable_stop` | 1 |
| `--lib repository::integration::tests` | 3 |
| `--doc local_sync` | 4 |

These shared/private-boundary controls were unchanged by the final local token
parser correction. Unique selected total: **149 library + 4 compile-fail = 153**.
No full suite, native CI, PR or lifecycle action. Linux/UID-1000 permission evidence
does not establish native Windows/macOS behavior.

**Mandatory Task 7 obligations, not delivered by this checkpoint:**

1. Activate the existing executor and new conflict/resolve/control internals with
   real production mutation/read/resume callers; remove exclusive-dependency gates
   while leaving all fixture hooks and tests test-only. No direct public journal
   bypass may replace the approved boundary.
2. Create authoritative F2 S/R/U control associations atomically with acceptance,
   including immutable intended scoped parent before linkage; check current request
   attempt plus domain epoch at linking, takeover, every effect and settlement.
3. Atomically advance the control attempt fence with `enter_request`, preserving
   stable-ID evidence, original digest, effects and cancellation generations. Do
   not use the domain-only cancellation table as proof that F2 is implemented;
   bridge/migrate its observed counters without losing an outstanding issuance.
4. Route partial resume to original R and preserve Accepted/Owed on recoverable
   cancellation; handle linked and unlinked terminal dispositions exactly. Complete
   the bridge's guarded route with atomic cancellation of ALL associated Accepted
   S/R/U, including unlinked intended-parent records, preserving Finished results.
5. Publish the exact designed DTOs, closed enums/schemas/goldens, content-read
   privacy, command inventories, progress, request retry/cancel/abandon presentation
   and stable two-session reads. Current conflict-domain inspection is an internal
   building block, not the complete public operation-inspection DTO.
6. Run the named F2/read/contract cases and production gates. Task 8 still owns the
   complete closure acceptance fixture and broader lost-journal/platform evidence.

Next gate is controller **independent SPEC, then QUALITY**, over the full Task 6
range from the supplied base, including these activation and required-outcome
boundaries. This implementer claims no independent review approval and starts no
Task 7 implementation. Ticket remains implementing.

## Task 6 SPEC corrections: four Important findings, awaiting re-review

Recorded **2026-10-26T20:04:52Z**, comment `01M5WX1PHGK4REHFEHJGBQ5PAW`.
Fix base **`3dc7d15616819fcdef49164182614b2195ce1877`**. Review
**`ses_e864b2dfbffemfrKBcgnvISO55`, REQUEST CHANGES, four Important findings**.
All four were verified against the actual ordering/proof calls before test/code
changes. These are implementer corrections, not independent approval. The initial
all-domain-required-outcomes claim is explicitly qualified above until re-review.

### Source verification and dispositions

| Finding | Verified cause at base | Correction and proof | Disposition |
| --- | --- | --- | --- |
| 1. Whole-result invariants checked after path writes | `validate` constrained conflict entries and live stage-zero prospective dependencies; the frozen-tree all-side check only ran in `make_candidate`, after the path loop. An unrelated automatically merged closed ticket could therefore invalidate an already accepted/written R. | Shared private `prepared_candidate` builds the full candidate in a memory-only ODB and validates it against the three frozen trees before R acceptance or any path replacement. Real candidate creation uses the same routine. The counterexample proves zero attempts, zero owned-write calls, unchanged files/index/refs/logs/metadata and unchanged persistent object set. | Addressed; SPEC re-review pending. |
| 2. Backend locks and same-OID reflog changes checked too late | `verify_paths` omitted backend lock/ref-log evidence. `backend_unlocked` and the exact before-ref proof ran only after paths/objects or index work. | `verify_backend` reuses the existing lock and exact two-log pre/post proofs before fresh acceptance, partial-path reconciliation, each actual path write, object import, candidate/index continuation and ref transition. It runs outside the backend primitive's own transient locks; a proven owned post-ref append remains valid. Fourteen fresh/partial cases and nine in-call boundary cases prove no additional file/index/object effects and preserved foreign evidence. | Addressed; SPEC re-review pending. |
| 3. Imported candidate without observed objects step lost checkpoint attribution | The commit ODB write precedes step observation. Existing-candidate continuation validated the candidate but never reconciled that missing observation, so successful completion/replay reported Checkpoint Unchanged. | After frozen signature/parents, full invariants, accepted path/output and complete result checks, continuation also proves the active resolution's candidate binding and observes its existing objects intent before proceeding. Four actual child exits occur after commit import and before SQL observation; exact R and source-free resume both complete with Checkpoint Committed, same candidate, one ref append and unchanged object set. Foreign-signature substitution and missing attempt binding refuse without observing the object. | Addressed; SPEC re-review pending. |
| 4. Released non-resolution integrations could not be abandoned | `control::abandon` always called the canonical-resolution verifier, requiring a result/manifest/path intents/owned candidate append that other proven dispositions do not have. | `verify_released_integration` chooses the proof from recorded disposition/provenance: unchanged baseline/logs for no-op, frozen candidate/image/CAS/log evidence for FF/clean merge, existing canonical-R proof for R, or recorded external-result observation plus exact ordered-parent/all-side clean proof for external repair. Both integration/release flags and artifact-free checks remain mandatory. Ten two-kind positive cases and ten dirty-file/lock controls pass. | Addressed; SPEC re-review pending. |

The full-memory preflight is not a new persistent checkpoint or source store.
Prepared source blobs, trees and preview commits stay in the temporary ODB; the
regression enumerates actual persistent OIDs before and after refused input.
The original source digest/attempt matching still precedes fresh-token validation
on exact partial retry. No existing immutable-field predicate was weakened.

Backend checks distinguish owned progress from foreign state. The before-ref case
requires both recorded log baselines. The after-ref case requires the recorded
candidate, started ref intent and exact expected appends in both logs. Merge
metadata remains owned by the existing manifest/release protocol. No unproven lock
is removed or claimed; no reset/rollback was added. Restoring the foreign fixture
evidence permits the original complete R input to finish normally.

The new `CandidateImported` hook is **after** the successful real commit-object
write and **before** observing the objects step. Parent tests positively inspect
the existing commit and ordered parents, the still-prepared objects step and old
context ref. Four child processes exit with status **79 without Drop** (document/
ticket × exact-R/source-free continuation). Successful R and starting-operation
domain replays retain Committed and perform no further Git effects. These are
domain replay proofs, not claims about F2 request result settlement. There are now
**21 actual no-Drop exits** in the full local group (17 previous + 4 new); the
ordinary child-entry invocation is not counted as another exit.

### Test-first evidence

Commands below use the prefix
`devenv shell -- cargo test --all-features --locked --lib` and the single filter
listed. Every finding had a runtime failure before its correction; supplementary
negative controls are distinguished from RED claims.

| Filter | RED | GREEN |
| --- | --- | --- |
| `spec_whole_result_immutable_preflight_precedes_attempt_and_path_writes` | 1 failed: one resolution attempt existed instead of zero after the auto-stage-zero closure-author violation. | 1 passed; also asserts zero write calls and no persistent Git effects. |
| `spec_foreign_` | 2 failed: a pre-first-path index.lock and a lock arriving at Path(0) still allowed file/object effects. | 2 passed, **23 cases**: seven lock/log variants × fresh/partial R, plus three boundary points × index-lock/branch-log/HEAD-log. |
| `spec_no_drop_import_before_observation_reconciles_owned_checkpoint_on_resume_and_replay` | 1 failed after an actual child exit at the exact window: successful resolution reported Checkpoint Unchanged. | 1 passed, **four actual no-Drop cases**; Committed across completion and both domain replays, one ref effect. |
| `spec_artifact_free_abandon_uses_each_integration_disposition_and_provenance` | 1 failed: document/equal returned AuthorityMissing despite released IndexPending proof. | 1 passed, **ten cases**: both kinds × equal/contained/FF/clean merge/external repair. |
| `spec_disposition_abandon_preserves_foreign_files_and_backend_artifacts` | 1 failed: after restoring the no-op's foreign file, safe abandonment still refused. | 1 passed, **ten cases**: each disposition × foreign file/index lock, no Git or authority changes on refusal, success after restoration. |

Supplementary `spec_existing_external_object_cannot_be_observed_as_the_owned_candidate`
passes two negative controls: an external same-tree/ordered-parent object with a
foreign signature cannot borrow frozen candidate ownership, and an absent
resolution-candidate binding cannot be treated as observation authority. The
objects step remains prepared and Git unchanged in both cases. It is not claimed
as a separate pre-fix RED. The child harness entry is the eighth new selected test.

### Actual artifact-free abandonment matrix

All rows start at **IndexPending**, with `integration_proven=true` and
`artifacts_released=true`, following injected discovery failure.

| Provenance | Result/candidate evidence used | Successful domain abandon |
| --- | --- | --- |
| Equal / contained | No result or candidate; frozen context image and unchanged branch/HEAD logs. | Abandoned, epoch revoked; no Git effect. |
| Fast-forward | Adopted primary result equals frozen candidate; exact image and owned ref/log transition. | Abandoned, adopted OID retained; no Git effect. |
| Clean merge | Made candidate signature, ordered parents and all-side proof; frozen image and owned ref/log transition. | Abandoned, made OID retained; no second commit/ref effect. |
| Canonical resolution | Existing accepted-path/result/candidate/ref proof, plus full owned candidate validation. | Existing canonical-R controls remain green; no unsafe release. |
| Exact external repair | Recorded observed external reference step; clean result with exact ordered frozen parents and all-side invariants. No owned-candidate append is invented. | Abandoned, external OID retained; no made checkpoint or Git effect. |

Dirty files and backend artifacts refuse before parent/epoch/attempt changes for
every newly covered provenance. Existing prepared-before-effects and unsafe
conflict/partial-R abandonment controls remain green. The production generic
bridge guard and refusal of an associated public operation owner remain in place.
**Atomic S/R/U settlement, including unlinked intended-parent requests, is still
Task 7 and is not implemented or claimed by these fixes.**

### Final gates, scope and next review

All final whole-tree gates exited 0 through Devenv:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib local_sync::
```

Check: **2.65s**; clippy: **18.97s**, no warnings. Final local selection:
**150 passed**, zero failed/ignored/measured, 1284 filtered, **483.54s**. This is
142 existing local tests plus eight new SPEC tests. Long-running matrix notices
completed normally. Existing permission-loss, foreign-member retirement,
source/privacy, cancellation and Task 5 frozen-image/ref controls remain green.

Each required shared anchor ran separately with the same Devenv test prefix and
one positional filter, selecting/passing **one** test:

```text
public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally
external_repair_exact_ordered_parent_clean_only
clean_merge_integration_fault_matrix_resumes_each_durable_transition_once
cancellation_with_a_pending_conflict_is_a_recoverable_stop
```

**154 selected library tests pass** at this correction checkpoint. No unrelated
test target, full suite, native workflow or additional old gate was run.

Rust scope: new `local_sync/spec_tests.rs`; modified `local_sync/{resolve,recover,
control,mod}.rs`, plus mechanical visibility of the existing fixture setup helper
in `conflict_tests.rs`. No old assertion changed. No schema, dependency, frontend,
global enum, public API or F2 change. Narrow internal staging and all Task 7
activation/association/cancellation/atomic-disposition obligations remain.

Entry was clean at the fix base. Start/checkpoint fetch and merge found main
`0464c42dae19a6dde000c7e7660e82762a41fada` already incorporated; merge-tree exited 0,
base tree `2c1cd4472a8f3b543d5e08ed143d65b6de90db15`. Status/diff/log and owned scope
were inspected; diff checks passed. No subagents, owner-policy changes, PR or
lifecycle actions. **Next: SPEC re-review of all four findings on the full Task 6
range, then QUALITY.** The blanket completion claim remains qualified until those
independent gates approve the corrections.

## Task 6 SPEC re-review: candidate ownership and external provenance composition

Recorded **2026-10-26T20:31:21Z**, comment `01M5WYJ5NR8CQ09A8JMGPDX847`.
Fix base **`4104895b4ccb90c4410eb64cc035146cace7cbc9`**. The controller reports
SPEC `ses_e864b2dfbffemfrKBcgnvISO55` closed the original whole-result preflight
and early lock/log findings, but retained **two Important findings** below.
Both were verified in source and reproduced before correction. These are verified
implementer fixes awaiting SPEC re-review, not approval. **No QUALITY review yet.**

### 1. Shared candidate ownership, including S's error observer

Verified composition: `complete_candidate` checked the active R's candidate only
when `commit_observed` was false. A failed S execution then called
`observe_object_progress`, whose parent candidate validation did not require the
R binding. That observer could mark the objects step, allowing the next S retry to
skip the conditional binding check and integrate an inconsistent attempt.

The binding is now an unconditional invariant in the shared **production-active
authority audit**: the parent candidate and active-resolution candidate must agree,
including NULL versus non-NULL. The existing shared audit already verifies the
operation/generation association and now refuses missing/substituted candidate
ownership before any reader/owner transaction can treat it as authority. This
covers S's error observer, fresh and repeated R/resume execution, direct objects
step observation and already-observed steps. Candidate and active-R intent are
created atomically, so ordinary pre-candidate NULL/NULL records remain valid.

Removed the weaker conditional binding query from the resolver. Full candidate
signature/parents/tree/path/invariant checks remain before continuation; the
observed objects flag is never a substitute for association authority. No schema
shape, source storage, F2 control or public API changed.

New `spec_candidate_binding_is_required_on_repeated_start_retry_and_observed_recovery`
executes **four compositions**: prepared/observed objects step × missing/substituted
R candidate. Each makes **two real `run::execute` S retries**, then source-free
resume and exact R retry. Both S calls must refuse, the objects phase must stay as
it was, and files/index/ref/logs/metadata/persistent objects must not change.
The prior foreign-signature and missing-attempt resolver controls remain green.

### 2. External authority is not installation history

Verified cause: the abandonment dispatcher used `conflict_started` and required
`conflict_observed` to recognize external integration. A legitimate exact-parent
external repair after S's intent but before conflict installation took the ordinary
merge branch and failed for lack of an owned candidate. A repair after actual
installation but before manifest observation failed the observed-manifest test.

External provenance is now selected by the **observed external reference evidence**:
operation, ordinal/kind, observed phase, generation, frozen old OID and recorded
result OID must match. `external_integration_recorded` supplies that shared lookup;
`verify_external_integrated` still verifies the complete exact ordered-parent,
all-side canonical, clean-result and artifact-free proof. The dispatcher no longer
requires a prior conflict-install intent or observed manifest to recognize this
proven external result. It still refuses to borrow external provenance for an
owned merge candidate, and retains the ordinary no-op/FF/clean/R proof branches.

`spec_external_repair_without_observed_installation_can_abandon` covers **four
positive cases**: both context kinds × pre-install intent / actual backend install
interrupted before manifest observation. Each explicitly creates the valid external
commit, resumes through observed external-reference adoption, injects discovery
failure, and proves released IndexPending abandonment with retained result OID,
revoked epoch and no Git effect. Terminal domain resume remains cancelled.

`spec_external_abandon_requires_observed_reference_authority_and_no_foreign_artifacts`
covers **eight controls**: both installation histories × missing external step,
unobserved external step, wrong recorded result, or a foreign index lock. Refusal
preserves parent/epoch and the complete Git image. Restoring the foreign-lock
fixture permits abandonment, proving the guard is not a blanket refusal.
The interrupted-install seam is an injected return after real backend effects;
it is **not** claimed as an additional no-Drop exit.

### Exact RED/GREEN and final gates

Test command prefix: `devenv shell -- cargo test --all-features --locked --lib`,
one positional filter per invocation.

| Filter | RED before fix | GREEN |
| --- | --- | --- |
| `spec_candidate_binding_is_required_on_repeated_start_retry_and_observed_recovery` | 1 failed: the second S retry returned Ok in the prepared/substituted-R-candidate composition. | 1 passed, four compositions, both S retries and R/resume controls preserved. |
| `spec_external_` | 2 failed: pre-install external result refused abandonment; the foreign-lock control still refused after restoration. | 2 passed: four positive cases and eight negative/restoration controls. |
| `spec_existing_external_object_cannot_be_observed_as_the_owned_candidate` | Retained regression control, not a new RED claim. | 1 passed, foreign-signature and missing-attempt cases. |

Final whole-tree commands, all exit 0:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib local_sync::
```

Check: **2.63s**; clippy: **11.93s**, no warnings. Local selection: **153 passed**,
zero failed/ignored/measured, 1284 filtered, **507.63s**. This includes all 150 prior
tests and three new regression tests. All existing 21 actual no-Drop exit assertions
remain green; this correction adds no new process-exit case.

Each shared anchor ran separately with the same Devenv prefix and selected/passed
exactly **one** test:

```text
public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally
external_repair_exact_ordered_parent_clean_only
clean_merge_integration_fault_matrix_resumes_each_durable_transition_once
cancellation_with_a_pending_conflict_is_a_recoverable_stop
```

Unique selected total: **157 passing library tests**. No full suite, unrelated
target, native run, frontend/dependency/global-enum edit, subagent, PR or lifecycle
action. No Task 7/F2 implementation or all-associated S/R/U settlement claim.
Narrow executor staging and production authority/admission guards remain in force.

Rust files: `local_sync/{record,recover,resolve,run,spec_tests}.rs`. The run change
documents its shared audit gate; existing test assertions are unchanged. Entry was
clean at the supplied base. Start/checkpoint fetch/merge found main `0464c42` already
incorporated; merge-tree passed with base tree
`23c13023a64a35900ba5e82d90415924c53891ae`. Status/diff/log and owned scope were
inspected, and diff checks passed. **Next gate: SPEC re-review must be green before
QUALITY begins.** Overall Task 6 completion remains qualified pending that review.

## Task 6 QUALITY corrections: continuation and representation boundaries

Recorded **2026-10-26T21:22:06Z**, comment `01M5X0P7E9XE73WJYHYSSQN09A` (ID
issued earlier by the main checkout's existing CLI). Fix base:
**`fd1b356eb67403bc1dbcf6a7fad7ac8425dcecdc`**.

The controller reports Task 6 SPEC **APPROVED with zero outstanding findings at
this base**, session **`ses_e864b2dfbffemfrKBcgnvISO55`**. QUALITY
**`ses_e8613a466ffe3Dlw50236N05X9` requests three P2 corrections**. All three
were verified from source and reproduced before correction. The following are
implementer fixes pending SPEC delta verification and QUALITY re-review.

### Verified findings, corrections and actual owner states

| Finding | Verified base behavior | Correction / resulting proof |
| --- | --- | --- |
| 1. Stale integration proof at index handoff | Result/ref proof ran before `LocalSyncBeforeIndex`; the common index function then trusted two agreeing discovery observations and SQL owner checks. Both observations could agree on a foreign ref/log/lock image and incorrectly complete the parent, R and envelope. | The shared `run::index` rechecks `verify_released_integration` after both index boundaries and discovery observations, immediately before terminal SQL. This covers canonical R, external result, equal, contained, FF and clean merge. Foreign evidence leaves IndexPending, `index_complete=false`, and parent/R/envelope nonterminal; exact restoration permits discovery-only completion without another Git effect. |
| 2. Stale metadata-release preflight after one removal | Backend/ref and complete member checks ran once. After a normal-return `Retired(0)` hook changed a lock/log/ref or later member, the loop could unlink additional owned members before a late failure. | `release_preflight` checks current authority, backend locks, integrated ref/log/index/file proof, and the complete remaining metadata set before each removal. Foreign reappearance at an already-retired name is checked too. A foreign arrival after MERGE_HEAD removal now stops with only that member retired, Integrated/release-pending authority and the release step still prepared. Restoration resumes the verified remainder without another commit/ref effect. |
| 3. Conflict early return skipped prospective structural checks | The conflict branch returned execution preparation before the existing whole-index UTF-8/regular-mode/collision checks. An incoming nonconflicting symlink or non-UTF-8 path could therefore install effects and leave Applying with an unobserved image. | Removed the early return. Conflicts now pass the same complete prospective-entry checks, including every conflict stage and stage-zero addition, before journal acceptance. Unsupported representations refuse owner-free and Git-unchanged. Representable code, executable-file and binary conflicts still install as whole-set external-only; canonical eligibility policy is unchanged. |

External integration has no library-made reflog append. To detect a **same-OID**
foreign log change after adoption, reference step 13 now binds the validated
post-adoption branch and HEAD log digests in its existing metadata-digest fields.
Only digests are stored, not log contents or source. Verification requires those
exact retained digests in addition to the recorded external result, ordered
parents, all-side invariants, clean image and released artifacts. This preserves
external provenance without inventing an owned candidate/ref append.

First binding requires a prepared reference intent with the original unbound
metadata sentinel. A retained branch digest with a missing HEAD digest is partial
authority loss, not permission to capture fresh foreign logs. Existing observed
external records lacking the proof likewise refuse rather than reconstructing
authority. The SQL schema shape and global enums are unchanged. A regression
positively reproduced improper rebaselining of a partially lost prepared proof
during development; the sentinel check corrected it before this checkpoint.

### Test-first record and fixture matrix

All commands used Devenv and one positional filter. Test prefix:
`devenv shell -- cargo test --all-features --locked --lib`.

| Filter | Observed RED before correction | Final GREEN |
| --- | --- | --- |
| `quality_completion_revalidates_all_dispositions_after_index_boundaries` | 1 failed: changing canonical R's ref at LocalSyncBeforeIndex still returned Ok/Complete. | 1 passed, **48 cases**: six provenances × both index boundaries × ref/branch-log/HEAD-log/lock mutation. Parent, attempt and envelope remain pending; foreign Git image is unchanged; exact restoration finishes with no second Git effect. |
| `quality_metadata_retirement_rechecks_remaining_evidence_before_each_unlink` | 1 failed: retirement events were `[0,1,2]` rather than `[0]` after the normal-return hook introduced a foreign lock. | 1 passed, **six cases**: lock/ref/both logs/later MERGE_MODE/current MERGE_MSG. No additional member is removed; foreign evidence survives; repaired continuation finishes the same candidate. |
| `quality_conflict_` | 2 failed: symlink and non-UTF-8 cases returned retained Applying/Written instead of owner-free refusal. | 2 passed: **eight** kind × symlink/gitlink × stage-zero/conflict-stage cases, plus **two** Unix non-UTF-8 cases. Every refusal preserves files/index/refs/metadata and persistent object IDs. |
| `quality_representable_unsupported_conflicts_remain_external_only` | Passed as an unchanged-policy control before the structural fix. | 1 passed, **three** code/executable/binary controls. Not counted as a RED. |
| `quality_external_log_authority_is_not_recreated_from_current_git` | The added partially-lost-prepared composition failed by returning Ok and adopting fresh foreign logs. | 1 passed, **three** missing/corrupt/partially-lost-proof controls. Two retries and abandon refuse, preserving authority and Git. |

These are **six new tests, 70 matrix/control cases**, in
`local_sync/task6_quality_tests.rs`, registered as an adjacent child test module
of the existing SPEC fixtures. Existing semantic assertions were not edited.
The retirement hooks return normally: an injected error is not what stops the
loop. These new tests add no child-exit count; all **21 existing actual no-Drop
exit assertions** remain green in the complete local selection.

### Final verification and next gates

Final whole-tree checks, all exit 0:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked --lib local_sync::
```

Check: **0.32s**; clippy: **6.94s**, no warnings. Final local selection:
**159 passed**, zero failed/ignored/measured, 1284 filtered, **608.94s**. This
includes the 153 previous local tests plus all six new QUALITY tests. The final
run followed the additional prepared-log-proof-loss correction; earlier passing
runs are not substituted for that final verification.

Each required shared anchor then ran separately with the same Devenv test prefix,
selecting/passing exactly **one** test:

```text
public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally
external_repair_exact_ordered_parent_clean_only
clean_merge_integration_fault_matrix_resumes_each_durable_transition_once
cancellation_with_a_pending_conflict_is_a_recoverable_stop
```

Unique selected total: **163 passing library tests**. No full suite, unrelated
target, native workflow, frontend/dependency/global-enum edit, PR, lifecycle action
or subagent. The non-UTF-8 filesystem cases execute on Unix/Linux; no native Windows
or macOS runtime evidence is claimed.

Rust scope: `local_sync/{run,recover,resolve,spec_tests}.rs` and new
`local_sync/task6_quality_tests.rs`. Preparation and recovery reuse the existing
neutral primitives and disposition/image/ref proofs. No Task 7 production caller,
F2 association/cancellation/settlement or public DTO was implemented; its explicit
activation obligations remain. Source/privacy and all prior authority, external
repair, metadata retirement and frozen-image regressions remain green.

Entry was clean at `fd1b356`. Start/final fetch/merge found main `0464c42` already
incorporated; merge-tree passed with base tree
`ee50257d353a5c79342342168947a950ce3f89ef`. Status/diff/log and owned scope were
inspected; diff checks passed. **Next: SPEC delta verification, then QUALITY
re-review of these three corrections.** Prior SPEC approval applies to the base;
no approval of the changed tree or Task 7 advance is claimed.

## Task 7 start: request-control foundation and named-edit stop

Recorded **2026-10-26T21:47:49Z**, canonical question
`01M5X2Y6RPMMT9BMS75Y0EXTYQ`. Implementation base:
**`41bce3eb23ae4e95316f287cd832d38be00164aa`**. Initial implementation is committed
as **`ee9e485`**, `wip: stage local request acceptance and attempt controls`.
This is an incomplete checkpoint, not Task 7 delivery.

### Authority and preflight

The controller supplied final Task 6 SPEC and QUALITY approval at this base,
zero findings over the full `7283ad2..41bce3e` range, with sessions and counts
recorded in the task table above. These are controller reports, not fresh reviews
performed here. Read the entire authoritative approved design and contract, Task 7
plan, Task 0 source/selector inventory and implemented Task 4–6 seam/handoff records.

Worked inline without subagents, only in the specified ticket checkout. Entry
status was clean, branch and HEAD matched the handoff. Fetched origin, read
`origin/main:AGENTS.md`, merged main `0464c42` (already up to date), and verified
ancestry. HEAD did not move. No dependency, frontend, global enum or target edit.

### Implemented so far, and explicit limits

- Added internal production `mutation/local_control.rs`. The additive v1 table
  holds request FK, immutable intended parent/scope/role, nullable domain links,
  execution-attempt fence, cancellation generations and content-free effect fields.
  Migration checks marker/table consistency, exact shape and association integrity.
- `records::insert_request` creates S/R controls in the acceptance transaction;
  local-target U detection uses the existing scoped parent. Ignored insertion
  rolls back request and operation rows. No source text is added to persistence.
- `records::enter_request` advances the control fence in the same transaction as
  F2's attempt. Missing or stale S/R controls and ignored control updates refuse
  without committing the F2 advance. Stable intended identity, counters and effects
  survive unlinked handover and reopening.
- Registered nine tests in the requested existing `mutation_replay/local_sync.rs`
  module. They exercise the existing public hidden request-store API; **they are
  not public mutation-binding recovery tests** and do not satisfy the mandatory
  named full-protocol cases. Their deliberate corruption/trigger writes are
  isolated fixture injections, not production authority creation.

The production executor/conflict/control/filtered-image `cfg(test)` gates are
still present. No new public input, Mutation variant, read method, schema/golden,
domain linkage/fencing, authoritative cancellation integration, local resume or
atomic abandonment disposition has been delivered. Task 6 domain counters remain
distinct from this new F2 table. This checkpoint does not claim API activation.

### TDD and verification evidence

An initial two-test public-binding probe failed compilation because the approved
input and Mutation variant are absent. That temporary probe was replaced with
the focused acceptance/handover tests while building their prerequisite; it is
not retained or counted as passed public API coverage.

Five initial request-control tests ran RED: four encountered the missing table,
and one showed that F2 incorrectly advanced an attempt without control authority.
After implementation all five passed. Four supplementary rollback/reopen/legacy
controls were added and passed; no separate RED is claimed for those controls.

All Rust commands used Devenv from this checkout:

| Command | Observed result |
| --- | --- |
| `cargo test --all-features --locked --test mutation_replay local_sync` | **9 passed**, 0 failed, 308 filtered. |
| `cargo fmt --check` | Exit 0 after the final Rust edits. `git diff --check` also passed. |
| `cargo check --all-features --locked` | Exit 0, 4.44s. |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Exit 0, 45.65s, no warnings. |
| `cargo test --all-features --locked --test mutation_replay` | **316 passed, 1 failed**, 473.33s. All nine new tests and the existing conformance test passed. |
| `cargo test --all-features --locked --test mutation_replay the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else -- --exact` | **1 selected, 1 failed**, reproducing the same old assertion at line 430. |

The full target was selected because migration and request acceptance/handover are
shared by every binding. It was not the package-wide test suite. No golden updates,
native run, Task 8 work or completion feature commit occurred. The controller
subsequently directed committing/pushing the partial work and recording `waiting`;
the checkpoint publication procedure is recorded below.

### Actual old-test scope issue: owner approval required

`tests/mutation_replay.rs::the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`
constructs a pre-F2 fixture by dropping only `request_operations`, `request_records`
and `confirmation_records` (lines 415–423). It then asserts that no remaining
schema object's name contains `request` (line 430). The newly required
`local_context_sync_request_control` table remains, so that assertion fails before
reopening. Its later exact additive inventory is also explicitly the original
three tables (`REQUEST_TABLES`, line 347), with all other objects/rows unchanged.

This is an actual newly failing old assertion, not a compiler-staging issue or a
baseline failure. E5/E6/E7/E9 do not name this migration fixture or its private
inventory. The design explicitly requires owner review for an additional old-test
semantic edit, so the fixture remains unchanged and implementation pauses here.

**Requested narrow approval:** change only
`tests/mutation_replay.rs::the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`
and its private `REQUEST_TABLES` fixture/removal/migration-marker inventory for the
new local request-control table. Preserve **all** existing old-column,
unrelated-schema/data and privacy assertions; verify the control table/marker as
the exact additional migration effects. No general migration-test exemption or
unrelated fixture/golden rewrite is requested. The controller verified that this
is a genuine additional named edit, but expressly did not grant owner approval.

After the owner answers, complete the remaining Task 7 production binding/read/
control work and all required named tests/gates, then use the specified feature
commit and trailer and push. Independent controller SPEC then QUALITY reviews
still precede Task 8. The existing SSH conformance selector remains exactly
`every_command_of_this_target_keeps_the_rules_of_the_boundary`; four separate
target invocations and nonzero counts are still owed by Task 7 completion.

### Controller-directed WIP publication and hold

The controller instructed committing all owned partial work before pausing,
separating code from waiting documentation where practical, then pushing the ticket
branch. Status/diff/recent log and both new Rust files were inspected. Fetch/merge
again found main already incorporated. No PR exists. No Rust edits or repeated
gates were made on the unchanged verified tree.

- **Code checkpoint `ee9e485`:** only `src/repository/mutation/{local_control,mod,records}.rs`,
  `tests/mutation_replay.rs` (new module registration only), and
  `tests/mutation_replay/local_sync.rs`. The existing migration fixture and private
  inventory remain unchanged. The commit explicitly records the known test failure
  and incomplete scope and includes the required Co-Authored-By trailer.
- **Separate waiting checkpoint:** this ledger, question comment
  `01M5X2Y6RPMMT9BMS75Y0EXTYQ`, and the ticket's status-only change from implementing
  to waiting, using the `working-on-a-ticket` exception. The existing question ID
  and UTC creation time are retained. Both checkpoints are to be pushed to the
  existing ticket branch; remote parity is verified after push.

**Hold:** owner input is required for the exact fixture/inventory edit above.
Task 7 is not complete or independently reviewed. Public bindings, production
executor activation, DTO/schema publication and all required full-protocol cases
are still owed. Task 8 remains unstarted. No PR, closure, merge into main, native
CI or cleanup is authorized by this checkpoint.

## Task 7 scoped approval and implementation resumed

At **2026-10-26T21:59:30Z**, recorded the owner answer **“yes”** to question
`01M5X2Y6RPMMT9BMS75Y0EXTYQ`, approving exactly the additional legacy migration
fixture and private inventory edit stated there. Approval comment:
`01M5X3KKAFXVGD84SYBH3TBGFB`; design named-edit addendum records the same scope.
All old-column, unrelated-data and privacy assertions must remain. This is scope
approval, not independent review of Task 7.

Entry clean at `0b751846ff4fdb78a3e6b0f0c3d046d152bc05d2`; fetch/merge found main
`0464c42` already current. Restored ticket implementing via the skill's status
procedure. The WIP and nine-test evidence are retained; the already reproduced
fixture RED will be followed by its approved correction and GREEN. The remainder
of full Task 7, including mandatory production activation, is still required.

## Task 7: shared boundaries, authoritative request recovery and production activation

Recorded **2026-10-26T23:57:48Z**, comment **`01M5X9EC8F4RPJQYTX7Z45JXPZ`**
(ID issued earlier by the main checkout's existing CLI). Full review base:
**`41bce3eb23ae4e95316f287cd832d38be00164aa`**. Review through the commit containing
this checkpoint, titled
`feat: expose local synchronization and recovery through library boundaries`.
Do not review only the last commit: the range also contains the initial WIP,
waiting evidence and owner-approved migration-fixture addendum.

### Resumption and ownership

The usage-limit interruption occurred after substantial implementation and local
verification, before this durable final checkpoint and publication. The controller
reported no intervening edits and only a no-op fetch/merge. On resumption, HEAD was
still **`16e8780c246fa0f4e9b1e84be2e651703ef63463`**. Status, recent log, the complete
tracked-file inventory and untracked read-test directory matched this implementer's
prior work: **37 modified tracked source/test files**, three new production modules,
four schemas, five mutation goldens, two read goldens and the new read-test module.
No unfamiliar/unexplained edit was found or overwritten. Existing WIP work and
observed RED/GREEN evidence were retained rather than restarted.

Start, pre-handoff and post-interruption fetch/merge found `origin/main` at
`0464c42dae19a6dde000c7e7660e82762a41fada`, already incorporated. Merge-tree passed
at approval HEAD with tree `2f41d8d895d6fd6852fc499ddc20ab299ea5cc3e`. Work remained
inline in this ticket worktree/branch, with no subagents. Ticket stays implementing
while the controller reviews this task; Task 8 has not started.

### Actual production activation and public surface

- `mutation/bind/local_sync.rs` implements the exact three approved input structs
  and two unconfirmed variants/command names. Inputs have no Debug, Display or
  Serialize implementation. Start uses Local family; resolution has its own F2
  request/attempt association and no new parent-operation family. Its digest
  includes the unchanged top-level observation and every token/source pair in
  canonical token order; duplicate/malformed inputs refuse.
- `read/local_sync.rs` publishes the closed typed DTO/enumeration surface and
  inspection/content methods. The existing exact observation method remains the
  third read. Existing repository re-exports expose these boundaries. Immutable
  side source is explicitly serialized only by the content-read DTO, which has no
  Debug implementation. Mutation data stays content-free.
- Removed the executor-only module gates in `local_sync/mod.rs` for conflict,
  control, file image, resolve, recover and run. Activated `image_proof` execution,
  `observe::from_facts`, the required record/admission/store helpers,
  `integration::filtered` and `artifacts::{local_checkout,local_reference}`.
  `local_sync/request.rs` bridges F2 authority into the real leased executor.
  Production bindings call `run::execute_request` / `resolve::execute_request`;
  the old standalone execution/claim wrappers remain fixture-only. Actual Git,
  conflict, filtering and control logic is no longer hidden behind cfg(test).
- Fixture-only thread-local hooks remain test-gated. The two existing domain test
  Owner initializers gained only `request: None`; their assertions are unchanged.
  No public journal/executor bypass or broad unused/dead-code allowance was added.
  Four compile-fail tests still prove the journal/claim/takeover surface is private.

### Request authority, cancellation, disposition and read consistency

- Acceptance stores immutable intended scoped parent identity before nullable
  linkage. Domain creation or resolution-attempt acceptance links it in the same
  domain transaction under the repository lease. Resolution is not linked merely
  by reading the parent before input validation.
- Exact F2 entry advances the request/control attempt fence atomically, preserving
  stable-ID association, resolution attempt, effects and issued/acknowledged
  counters. Missing/stale control, including local-target U, refuses rather than
  recreating authority or advancing only one fence. Stable-ID journal/effect lookup
  precedes the family-None/absent-row shortcuts.
- Owner transactions verify current F2 attempt plus scoped domain epoch before
  effects; local settlement also verifies its control epoch. Resume has an exact
  local route. Partial R and an unimported candidate require the original R before
  U takeover or acceptance. A real U continuation transfers domain ownership; an
  unlinked contender sees completed-parent external_change/release or terminal
  cancellation without its own Git effects.
- Production S/R cancellation uses authoritative request-control generations;
  U checks S/active-R and routes its synchronous token to S. Safe points acknowledge
  the observed vector, not an unconditionally reread newer generation. Recoverable
  stops keep Accepted records and the underlying phase; request reads reflect
  pending counters. Old commands retain their boolean cancellation behavior.
  Local progress uses existing non-network stages. Cancellation keeps the needed
  canonical/original-input/external-only recovery choice.
- Artifact-free abandonment proves the current domain image under the lease, then
  abandons parent/attempt and finishes every Accepted intended-parent S/R/U in one
  transaction, including unlinked contenders. It stores each control's proven
  historical effects, preserves Finished rows, and reserves all terminal IDs.
  Settlement returns an already-terminalized local request's stored result.
- Finished local replay is immutable. The special handling is scoped to the two
  local commands/local resume data, so a forged local type tag on a legacy ticket
  result cannot bypass the legacy reachability check.
- Read views compare fresh SQL authority sessions and recheck Git. Registration
  and semantic-revision changes are detected with unchanged Git; owner-only epoch
  changes preserve tokens. Explicit historical sides remain readable under their
  original tokens after a real path write and reopening. Typed PermissionDenied
  at the recorded-blob read seam returns repository_inaccessible with no side bytes
  or Git/worktree changes. This injection is not native Windows ACL evidence.

The request control retains the specified request FK and nullable parent FK. Its
resolution identity is **audited against parent/request/attempt**, rather than a
SQL FK to the child table: the latter broke the existing legacy child-table
reconstruction protocol. The unshipped WIP v1 definition was corrected before
feature delivery; malformed/mismatched shapes still refuse. This follows the
existing Task 6 cancellation-companion association pattern, not a new migration
or transaction engine.

### Mandatory public tests and additional coverage

All names below are executable tests in `tests/mutation_replay/local_sync.rs`,
except the two marked Read, in the explicit-path `tests/read_items/local_sync.rs`.

| Required case | Observed proof |
| --- | --- |
| `identity_missing_then_set_retries_the_same_local_intent` | Pre-domain identity refusal releases S; accepted pre-candidate S survives identity removal and resumes after the public identity-set binding; journaled candidate retains its original identity. |
| `accepted_resolution_observation_is_part_of_request_identity` | Changing only top-level observation after a path write mismatches with historical written effects, unchanged file and unchanged F2 attempt. Exact R completes. |
| `cancelled_partial_resolution_remains_accepted_and_resendable` | Partial path cancellation/reopen retains Accepted S/R; U identifies R and leaves no request record; exact R finishes. |
| `new_cancellation_survives_acknowledgement_and_resume` | Coordinated second issuance during U acknowledgement leaves issued 2 / acknowledged 1. Next U honors the second generation; subsequent explicit continuation completes. |
| `artifact_free_abandon_settles_all_associated_requests_atomically` | Released/index-pending R plus unlinked R/U: public confirmed abandonment finishes all associated Accepted records cancelled/abandoned; exact R replay and changed-input mismatch retain spent IDs. |
| `unlinked_accepted_resolution_cannot_write_after_attempt_loss` | Handover between acceptance and domain entry fences the old invocation without writing; exact retry links and finishes. |
| `request_attempt_handover_preserves_resolution_and_cancellation` | Partial R handover interrupted before domain takeover preserves attempt association/counters/effects; a stale finish cannot settle, and later retry completes one resolution attempt. |
| `unlinked_contender_observes_parent_completion_or_abandonment` | Accepted R0/U before R1 completion get no-effect external_change and release; the abandonment composition terminalizes intended-parent contenders. |
| Read: `authority_only_change_is_seen_by_fresh_read_session` | Actual registration and semantic-revision changes between sessions refuse; owner-only change preserves observation and original side token. |
| Read: `side_read_permission_failure_is_read_only` | Typed denial at the actual blob-read seam gives repository_inaccessible, no returned side data and unchanged Git/worktree snapshot. |

Additional public cases cover no-op replay, independent S/R lost-settlement
receipts, unimported-candidate original-input requirements and fixed identity,
missing U control, source exclusion from every local companion plus request stores,
explicit read privacy, historical sides, external-only cancellation guidance, and
legacy replay isolation. Four actual subprocess exits without Drop cover R accepted
before link, after an owned path, F2 re-entry before domain takeover, and after
durable cancellation acknowledgement before response. Each asserts
`FAILURE_POINT_EXIT_STATUS`; the child-entry test itself is not another exit proof.
The prior **21** actual domain no-Drop assertions also remain green.

### RED/GREEN, corrections and approved old-test edits

The initial missing-public-API compile RED and five WIP control REDs are recorded
above. Further observed behavioral REDs were: guarded abandonment preview refused
the new parent; missing U control advanced F2 without its control; cancellation of
an unsupported conflict incorrectly advertised resolve; and a forged local data
tag let an unreachable legacy ticket result replay successfully. Each received a
production correction and a passing regression. Other new protocol tests are
supplementary composition proofs; no separate pre-fix RED is claimed for every row.

The first expanded domain run had **156 pass / 3 fail**. Two failures traced to the
new child FK and the legacy reconstruction fixture; one to premature epoch change
on lost external-log authority. Fixed production association/takeover wiring, not
the old assertions. All three exact selectors passed, followed by a fresh complete
**159/159** domain run. Fixture/debug corrections included using the ticket helper's
third argument as YAML extras rather than body text, relative internal visibility
for SSH targets that embed production modules, the schema checker's supported
single-value enum instead of `const`, and operation.resume's existing empty argument
set. No global enum or schema-checker behavior was expanded.

The owner-approved migration fixture now removes the new control table and marker,
asserts that exactly one marker returns, and excludes only that marker from its
unchanged-data comparison. Every old-column, unrelated-schema/data and privacy
assertion remains. Other old-test edits are E5's two command inventories, E6/E7's
homes/roots/enums, E9's seven explicit DTO refusal arms and golden/conformance
registrations, plus the two mechanical Owner fixture initializers above.

Published four new schema files (three roots and the referenced conflict entry),
five mutation goldens and two read goldens. The existing observation root remains.
Only selected local tests ran with `MANYHANDS_UPDATE_GOLDEN=1`; all seven generated
files were inspected. No pre-existing golden or unrelated fixture collection
changed. Explicit content data is rejected by the stored mutation schema.

### Final local gates and exact counts

Every Rust command ran through `devenv shell -- cargo ...` in this ticket checkout.
An earlier parallel Devenv startup reported a transient load-exports cleanup race;
the affected read-contract command was rerun sequentially with clean startup and
72 passes. No failed startup or earlier failing selection is counted as a final pass.

| Command after the Devenv prefix | Passing selection |
| --- | ---: |
| `fmt --check` | Exit 0 |
| `check --all-features --locked` | Exit 0 (latest 2.79s) |
| `clippy --all-targets --all-features --locked -- -D warnings` | Exit 0 (latest 14.41s), no warnings |
| `test --all-features --locked --test mutation_replay local_sync` | **28**, 308 filtered, latest 55.33s |
| `test --all-features --locked --test read_items local_sync` | **6**, 76 filtered, latest 4.45s |
| `test --all-features --locked --test mutation_contract` | **44**, zero failures |
| `test --all-features --locked --test read_contract` | **72**, zero failures |
| `test --all-features --locked --lib local_sync::` | **159**, zero failures, 641.28s |
| `test --all-features --locked --lib repository::mutation::command::tests` | **10** |
| `test --all-features --locked --test mutation_replay the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else -- --exact` | **1** |
| `test --all-features --locked --doc local_sync` | **4** compile-fail privacy/boundary proofs |

Ran **four separate commands**, each with the single positional selector
`every_command_of_this_target_keeps_the_rules_of_the_boundary`, under the same
`test --all-features --locked` prefix:

| Target | Actual selected output / homes |
| --- | --- |
| `--test mutation_replay` | **1 passed**; 18 command homes including both new bindings, no exemptions. |
| `--test mutation_confirmation` | **1 passed**; 6 homes. |
| `--test mutation_remote` | Exact SSH harness selector above; **1 SSH cases passed**; 4 homes. |
| `--test mutation_bridges` | **1 passed**; 4 homes. |

Together the home inventory accounts for **all 32 commands**. Selected unique
total is **328 tests/cases** across the listed groups, without adding repeated
RED/GREEN runs, the repeated identity-golden check or helper invocations. These
are local Linux results, not package-wide or native-platform CI evidence.

### Review handoff and remaining Task 8/9 outcomes

Task 7 implementation is ready for the controller's **independent SPEC, then
QUALITY** review over the full base range. No independent review or approval of
Task 7 is claimed. Preserve the ticket's implementing status until the remaining
Cycle work and review gates are satisfied.

Task 8 still owns the complete closure-refusal → public local resolution → explicit
closure-adoption acceptance cases, broader adversarial/lost-journal/platform
compositions and its approved E3 fixture changes. Task 9 owns the remaining document
amendments and whole-branch delivery/review. No closure is automatically resumed.
No frontend, Cargo/dependency/target, global ResultCode/RecoveryActionKind or native
workflow was changed; no PR, closure, merge into main or cleanup was performed.

## Task 7 SPEC corrections: seven Important findings, awaiting re-review

Checkpoint **`01M5XFVS1XWNMG79KS7DYXZ7GH`**, timestamp **2026-10-27T01:33:41Z**.
Fix base: **`44e3969f80907b34ab85a6c8bc3bb96f32783863`**. The controller supplied
SPEC session **`ses_e8554c7d2ffe8o3xfJnBqcRrQO`**, **REQUEST CHANGES**, with seven
Important findings. Each was verified against the base source before correction.
The original Task 7 delivery evidence above is historical, not approval of these
missing compositions. Review this fix delta and the full Task 7 range from
`41bce3eb23ae4e95316f287cd832d38be00164aa` through the commit containing this
checkpoint. QUALITY follows successful SPEC re-review; Task 8 remains unstarted.

### Verified findings and dispositions

1. **Complete production S/active-R authority — addressed locally.**
   `Fence::verify` checked only the executing request, `Owner::transaction` audited
   only the domain tables, and cancellation accepted any nonempty surviving
   control subset. The initial public R barrier reproduced continued execution
   after deleting S's control. Production owner checks now also run the F2 control
   audit and require exactly the scoped S association plus the exact active R,
   when present, with matching request/operation/attempt identities. Final indexing
   explicitly distinguishes authoritative F2 controls from fixture-only domain
   counters; an empty/incomplete F2 vector cannot fall back to those counters.
   Inspection and bound abandonment reuse the complete association proof. Public
   barriers delete S during R and active R during U: both stop before further Git
   effects, retain requests and domain evidence, leave the post-deletion authority
   snapshot unchanged and never recreate the missing control.

2. **Atomic unlinked-parent disposition — addressed locally.**
   The base checked a previously read parent before `attach`; a parent that
   completed between that read and linkage let U attach, inherit effects and
   finish successfully. The regression reproduced `ok` instead of external_change.
   `Fence::attach` now checks terminal state in the leased domain transaction and
   refuses first linkage to complete/cancelled/abandoned parents. The binding maps
   the resulting unlinked terminal disposition without own effects. A real public
   progress barrier after U's initial read lets a separate thread complete R before
   linkage: U reports external_change, leaves no request record and creates no
   attempt. Already-linked U completion/replay stays valid.
   The same gate now checks source-free candidate capability before first U link:
   a concurrent partial R appearing after U's initial read yields
   original_request_required/R, no own effects and release of unlinked U. That
   additional composition first reproduced inherited parent write effects. Tests
   coordinate through bounded channels, not recursive library calls from a sink.

3. **Read actions from executable evidence — addressed locally.**
   Base inspection used candidate-OID presence, so an unimported memory candidate
   incorrectly advertised resume/S; a stable index.lock incorrectly advertised
   fresh resolve. Both counterexamples ran RED. Read-only candidate validation now
   shares actual commit existence, ordered parents, frozen identity and invariant
   proof with source-free execution. Backend/ref/log checks have shared read-only
   cores; inspection also checks accepted path intents, modes, index composition
   and remaining metadata without acquiring a write lease or changing authority.
   A journaled/unimported R candidate gives retry_resolution/R/false. Stable foreign
   locks, files or candidate-identity disagreement give external_repair/false.
   Tests hold the repository lease while reading and compare Git plus full relevant
   SQL snapshots, including historical side reads under original tokens.
   A related external-result control verified that a clean exact-parent external
   repair can be inspected/resumed, including index-only continuation and missing
   current commit identity. External provenance uses its recorded reference/log
   proof, not an owned-candidate assumption. It first reproduced external_repair
   instead of resume; a follow-up missing-identity check reproduced IdentityRequired
   instead of IndexPending. Both now pass. No closure continuation is involved.

4. **Cancellation reply generation — addressed locally.**
   Base `cancel_request` set acknowledged from `ack > 0`; after `(1,1)`, issuing 2
   incorrectly returned true. The new regression reproduced that exact false
   acknowledgement. Local acknowledgement now requires a positive issued generation
   equal to the current acknowledged generation. Repeated issuance remains false
   until a safe point acknowledges it. The existing coordinated newer-cancellation/
   U-takeover case still passes, as do all seven old progress/cancellation cases.

5. **Canonical local resume routing — addressed locally.**
   The local query used the raw caller root before resolution; the first `root/.`
   deterministic-resume case reproduced OriginalRequestRequired instead of Ok.
   Registered local routing now resolves canonical repository scope before querying,
   and position reads use that same root. Missing/unregistered roots continue to
   the pre-existing legacy routes. Tests exercise root/. plus Unix relative and
   symlink spellings for deterministic U and partial-R input requirements, then
   repeat the same request ID through the canonical spelling and compare envelopes.
   All **33 existing resume tests** pass unchanged.

6. **Actual typed permission failures — addressed locally.**
   `file_image::read` already returned typed EACCES, but `conflict::image` replaced
   every I/O error with foreign change. A real chmod-unreadable conflict file at
   **UID 1000** reproduced external_change instead of repository_inaccessible.
   The conflict image now preserves typed I/O classification, and the non-Unix
   owned-file adapter retains its underlying I/O kind rather than replacing it
   with InvalidInput. The Unix test positively verifies PermissionDenied from the
   unreadable file, calls the explicit content API with a valid recorded token,
   restores permissions, and verifies no returned source and unchanged Git/SQL.
   The existing portable recorded-blob seam test also passes. No Windows ACL or
   native macOS execution evidence is claimed.

7. **Written-before-observed historical effects — addressed locally.**
   Base path replacement preceded the SQL observation, but changed-input lookup
   immediately returned the stale control effect cache. A new actual no-Drop exit
   at `LocalSyncAfterResolutionWriteBeforeObservation` leaves a prepared path and
   cached Unchanged despite the replaced file; the first mismatch test reproduced
   Unchanged instead of Written. Stable-ID effect reads now supplement the cache
   only from scoped accepted resolution identity, durable pre/output digests and
   a proven no-follow regular-file output image, with fresh SQL recheck and file
   recheck. This is reporting evidence only: no lease, phase update, attempt advance,
   token issuance or cache write. Changing only the observation now gives
   request_mismatch/Written/Partial while request, files, refs and every relevant
   SQL row remain unchanged. Exact R finishes once and then replays; no fresh
   context/attempt is adopted from the observed file.

### Test-first evidence and exact test homes

The 12 new tests live in **`tests/mutation_replay/local_sync_spec.rs`**, an explicit
child of the existing local_sync module and existing mutation_replay target. The
parent helper only gains the new child-exit boundary; no existing assertion was
changed. All selectors below use
`devenv shell -- cargo test --all-features --locked --test mutation_replay <selector>`.

| Finding | Selector / observed RED | Final proof |
| --- | --- | --- |
| 1 | `spec_missing_start_or_active_resolution_control_fences_the_live_executor`: R returned Ok after S-control deletion. | Both R/S-loss and U/active-R-loss barriers preserve Git and remaining SQL authority. |
| 2 | `spec_parent_completion_between_resume_read_and_link_releases_unlinked_u`: returned Ok instead of ExternalChange. | Completion between read/link produces no-effect release. |
| 2 composition | `spec_partial_resolution_between_resume_read_and_link_requires_r_without_linking_u`: inherited Written/Pending parent effects. | Original R required; no U effects or retained U record. |
| 2 control | `spec_linked_resume_keeps_its_receipt_after_another_request_completes` | Linked U finishes and replays its receipt; supplementary control, no separate RED claimed. |
| 3 | `spec_inspection_requires_an_imported_candidate_and_refuses_foreign_locks`: Resume instead of RetryResolution for unimported R candidate. | Required R, no fresh resolution, no SQL changes, successful read while lease held. |
| 3 | `spec_inspection_stable_foreign_locks_are_external_only`: Resolve for foreign index.lock. | index.lock and HEAD.lock give external-only recovery without writes or lease acquisition. |
| 3 controls | `spec_inspection_foreign_images_and_candidate_identity_are_read_only` | Fresh/partial/imported foreign file images and corrupt candidate identity give external-only recovery; historical sides stay immutable. Supplementary controls. |
| 3 composition | `spec_read_and_resume_share_external_result_proof`: ExternalRepair instead of Resume, then IdentityRequired instead of IndexPending after identity removal. | Valid external adoption and index-only retry share proof and make no library checkpoint. |
| 4 | `spec_cancel_reply_does_not_acknowledge_a_new_generation`: second issued generation returned acknowledged=true. | Second/third issues remain pending until explicit safe acknowledgement. |
| 5 | `spec_resume_aliases_use_canonical_local_scope`: first alias gave OriginalRequestRequired for resumable candidate. | Six alias/phase combinations, canonical same-ID replay and R selection pass on Linux. |
| 6 | `spec_actual_conflict_path_permission_denial_is_repository_inaccessible`: actual EACCES became ExternalChange. | Typed RepositoryInaccessible, no source, restored permissions and unchanged SQL/Git; executed at UID 1000. |
| 7 | `spec_no_drop_written_unobserved_path_has_read_only_mismatch_history`: actual exit before SQL observation gave Unchanged mismatch effects. | Written/Partial mismatch, no authority mutation, exact R completion/replay, one resolution attempt. |

The initial five-test SPEC selection failed all five intended assertions. The
terminal-race and no-Drop cases then established their own REDs before their fixes.
Supplementary controls are not represented as additional pre-fix failures. Routine
compile/lint corrections were limited to a borrow split and an octal permission
literal. The read/link barrier reports a real existing Observing progress stage;
it adds no public testing or journal-bypass method and does not consume the existing
cancellation-observation hook. The latter's older race test stays unchanged.

### Verification for this fix series

All commands ran through Devenv from the ticket checkout. Each test invocation had
one positional selector where filtered; no zero-selection result was accepted.

| Command after `devenv shell -- cargo` | Result |
| --- | --- |
| `fmt --check` | Exit 0. |
| `check --all-features --locked` | Exit 0; latest 4.76s. |
| `clippy --all-targets --all-features --locked -- -D warnings` | Exit 0; latest 25.97s, no warnings. |
| `test --all-features --locked --test mutation_replay local_sync` | **40 passed**, including all mandatory Task 7 names and all 12 new SPEC tests; 121.07s in the complete selection. |
| `test --all-features --locked --test read_items local_sync` | **6 passed**. |
| `test --all-features --locked --test mutation_replay resume::` | **33 passed**, old assertions unchanged. |
| `test --all-features --locked --test mutation_replay progress::` | **7 passed**, old assertions unchanged. |
| `test --all-features --locked --test mutation_contract` | **44 passed**. |
| `test --all-features --locked --test read_contract` | **72 passed**. |
| `test --all-features --locked --lib local_sync::` | **159 passed**, 838.28s; relevant domain regression group, not the full package suite. |
| `test --all-features --locked --doc local_sync` | **4 passed**, private-boundary compile-fail proofs. |

After the narrow final external-result/identity correction, reran its exact public
composition, all whole-tree gates and the complete affected 159-test domain group.
All passed. No unrelated baseline was rerun to replace the targeted evidence.

The exact single selector **`every_command_of_this_target_keeps_the_rules_of_the_boundary`**
also ran in four separate `test --all-features --locked` invocations:

- `--test mutation_replay`: **1 passed**, all 18 homes including both local commands.
- `--test mutation_confirmation`: **1 passed**, 6 homes.
- `--test mutation_remote`: **1 SSH cases passed**, exact SSH harness selector above,
  4 homes. This is not a zero-selected Rust test invocation.
- `--test mutation_bridges`: **1 passed**, 4 homes.

The home inventory remains **32 commands**. Unique selected total for this fix
series: **369 tests/cases** (12 SPEC tests are included in 40; exact re-runs are not
added again). The new path-window proof adds **one actual no-Drop exit** to the
existing four public and 21 domain exit assertions. Thread barriers are not counted
as process deaths. No golden/schema rewrite, new target, dependency or frontend edit
was needed. The existing portable permission injection remains; native platforms
were not run.

### Scope and re-review handoff

Entry was clean at the fix base. Fetch/merge at entry and checkpoint found main
`0464c42dae19a6dde000c7e7660e82762a41fada` already incorporated. Merge-tree passed at
base with tree `0b482a3468a29cdb038e659fe6847ec9c181b2c8`. Production module gates stay
active and the new shared proof/history helpers remain repository-internal.
No public journal bypass, generic transaction engine or unused-code allowance was
introduced. Existing tests were extended only with the new SPEC module and child
boundary; their old semantic assertions were not rewritten.

All seven findings are **locally addressed**, pending independent verification.
Next is controller SPEC re-review of the full correction delta, then QUALITY.
There is no approval of these fixes yet. Ticket remains implementing; Task 8 has
not started. No PR, lifecycle closure, merge into main, native CI or cleanup was
performed. Remaining unknowns are independent review and native runtime/ACL behavior,
not a claimed successful result for those unrun gates.

## Task 7 SPEC #3 re-review: preimage phases and ordinary image authority

Checkpoint **`01M5XPDY946AP10ZTA617CNMBV`**, recorded **2026-10-27T03:41:41Z**
(ID issued earlier by the main checkout's existing CLI). Fix base:
**`3836310a8471795187311fa4e730bfc2c17b53c7`**. The controller reports that six of
the original seven findings are closed; finding #3 still had **two Important
inspection gaps**. This checkpoint addresses those two, pending SPEC verification.
The full Task 7 review base remains `41bce3eb23ae4e95316f287cd832d38be00164aa`.

### 1. Exact preimage encoding and per-path phase — locally addressed

Source verification confirmed `read_resolution_image` compared raw BLAKE3 file
digests with `baseline_metadata_digest`, whose stored format includes the domain
separator, presence byte, length and content. It also did not load the path phase.
The new public fixture creates **two actual canonical conflicts** (ticket and
document), rather than relying on the previous one-path fixture.

- Extracted the existing preimage encoding and path-image classification from
  `integration/resolution.rs` into shared pure helpers. The executor still validates
  the submitted source digest and performs its no-follow read, then uses the same
  classifier that inspection now uses. The pre-existing digest format, presence
  distinction and execution behavior are preserved; no new journal format exists.
- Inspection reads actual bytes/mode through the no-follow image reader, verifies
  that they match its sampled Git image, and uses the stored phase explicitly.
  Prepared paths may match their exact framed preimage or their intended raw output;
  observed paths must match only output. A complete candidate still requires every
  path's output image, not an untouched preimage.
- Public regressions show accepted R cancelled before its first write and R stopped
  after one of two paths both produce `retry_resolution`, required R and no fresh
  resolution capability. The partial test covers both the observed-path failure
  boundary and an actual no-Drop write-before-observation exit; the second path is
  positively checked to be untouched. Exact R then completes.
- Restoring an already-observed first path's original baseline produces
  `external_repair`. This control was already refused at the base and remains so;
  the correction does not broadly permit baseline bytes after observation.

### 2. Ordinary S baseline/result proof — locally addressed

Source verification confirmed that the non-R branch returned success from the read
proof without consulting `owned_image`, so a stable foreign tracked file was advised
as resumable. A clean-merge S stopped after intent reproduced exactly that error.

- `recover::read_ordinary_image` now checks the retained full baseline/result proof
  used by `owned_image`, including index entries, bytes, modes and the recorded
  disposition. No-op/contained work uses its frozen baseline. Integrated/index-only
  work needs the recorded candidate/result, integration/release evidence and full
  intended image. Coarse work accepts a full result behind the baseline ref only
  with the recorded checkout intent; partial progress is not resume authority.
- Before a future checkout, frozen filter expectations are compared with the same
  filter pipeline. A clean prospective candidate can be recomputed from the frozen
  pair in a memory-only ODB for comparison. This never calls `Proof::build` to adopt
  a fresh baseline, persists an object, or replaces stored proof. Existing complete
  result images remain usable when no further checkout is needed.
- Missing/malformed image authority refuses. The snapshot recheck now compares the
  retained execution/image evidence, excluding only the lazy derived binding cache,
  and reevaluates the image proof after session B. Losing proof or changing filters
  between read sessions therefore refuses rather than using stale read-cache data.
  The existing owner-epoch-only observation control still passes.
- Unobserved conflict-install progress cannot bypass the ordinary proof by setting
  the reporting-only `conflict_observed` inference. Observed conflict-pending state
  continues through its dedicated immutable conflict-image proof. Lost recorded
  artifact release also refuses for ordinary and external-result index handoffs.

### Public test matrix and actual RED/GREEN evidence

New module: **`tests/mutation_replay/local_sync_inspection_spec.rs`**, registered
under the existing local_sync module/target. The earlier SQL snapshot helper gains
only parent-module visibility for reuse. No old assertion was edited or weakened.

Every `inspect_unchanged` call holds the **already-acquired repository lease** and
compares repository/worktree state, **all Git administrative file bytes including
objects/reflogs**, relevant SQL rows and stored image proofs before/after. These
are executable no-write/no-lease-acquisition controls, not merely source claims.

| Test | RED / control at the prior implementation | Final proof |
| --- | --- | --- |
| `spec_accepted_two_path_resolution_cancelled_before_first_write_is_retryable` | ExternalRepair instead of RetryResolution. | Two untouched prepared paths, Accepted R, required R, unchanged Git/SQL. |
| `spec_partial_two_path_resolution_keeps_untouched_preimages_retryable` | ExternalRepair after the first observed write. | First output + second untouched baseline; observed and no-Drop/unobserved modes both retry R and complete. |
| `spec_observed_resolution_path_cannot_return_to_its_preimage` | Passed as an unchanged-policy negative control. | Restored baseline on an observed path stays external-only. |
| `spec_non_resolution_foreign_file_or_index_requires_external_repair` | Resume for stable foreign file; reproduced for both contained and clean-merge prepared examples before correction. | **36 cases:** contained/FF/merge × prepared/integrated/index-pending × plain/CRLF × file/index mutation. Original/restored images resume S; foreign images are external-only. |
| `spec_non_resolution_images_require_coarse_intent_and_retained_authority` | Resume for a full result installed without this operation's checkout intent. A later test also exposed missing proof returning a view rather than the required fail-closed read error. | Four valid checkout/ref-window controls, two unowned-result refusals, and missing/corrupt proof refusals, all read-only. |
| `spec_non_resolution_filter_change_never_rebaselines_the_retained_image` | Prepared work still advertised Resume after filter change. | Future-checkout phases refuse changed filter expectations; already-proven checkout/index results can continue without reinterpretation. |
| `spec_non_resolution_read_rechecks_filter_and_image_authority_after_session_b` | Returned a resumable view after read-time filter change. | Filter change and proof disappearance between sessions both return ExternalChange; no proof is recreated. |
| `spec_unobserved_conflict_progress_is_not_resume_authority` | Resume after loss of the install observation while Git effects remained. | Retained intent/progress is external-only, not fabricated observed authority. |
| `spec_non_resolution_integrated_state_requires_recorded_release_proof` | Resume with recorded release removed. | Contained and clean-merge index handoffs refuse missing release evidence. |
| `spec_external_index_handoff_requires_recorded_release_proof` | Resume with external-result release evidence removed. | External handoff refuses until the fixture restores its original recorded proof. |

The first five-test selection produced four intended failures and the observed-path
negative control passed. Subsequent filter/coarse/snapshot/release counterexamples
were separately reproduced before their corrections. No separate RED is claimed for
every matrix row. The two-path unobserved mode adds **one actual no-Drop exit**; the
unobserved-conflict observation-loss control is a corruption fixture, not another
process-death claim.

A filtered fixture initially set autocrlf before the legacy save and then manually
wrote CRLF, which correctly encountered that save's exact observation/cleanliness
rules. The fixture was corrected to create its context first and use libgit2's real
forced checkout for its CRLF baseline, matching the established domain fixture.
Production authoring expectations were not changed. Clippy's by-value ReadError
warning on the new read-test helper uses the existing narrow `result_large_err`
convention; no unused/dead-code suppression was added.

### Targeted verification

All Rust commands used `devenv shell -- cargo` from this ticket worktree. There was
no package-wide test run or native CI. The following passed for this correction:

| Command after the Devenv prefix | Result |
| --- | --- |
| `fmt --check` | Exit 0. |
| `check --all-features --locked` | Exit 0; latest 3.16s. |
| `clippy --all-targets --all-features --locked -- -D warnings` | Exit 0; latest 16.51s. |
| `test --all-features --locked --test mutation_replay local_sync` | **50 passed**, 308 filtered; latest 177.85s. Includes all mandatory Task 7 cases and 10 new inspection tests. |
| `test --all-features --locked --test read_items local_sync` | **6 passed**, including owner-only epoch and historical-side controls. |
| `test --all-features --locked --test mutation_contract` | **44 passed**. |
| `test --all-features --locked --test read_contract` | **72 passed**. |
| `test --all-features --locked --lib local_sync::` | **159 passed**, 707.95s. Relevant domain regression group for shared image/hash helpers. |
| `test --all-features --locked --lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | **1 passed**; shared remote resolution behavior. |
| `test --all-features --locked --lib external_repair_exact_ordered_parent_clean_only` | **1 passed**. |
| `test --all-features --locked --lib repository::integration::tests` | **3 passed**. |
| `test --all-features --locked --doc local_sync` | **4 passed**, private-boundary compile-fail controls. |

Also ran `every_command_of_this_target_keeps_the_rules_of_the_boundary` as the single
positional selector in four separate `test --all-features --locked` commands:
mutation_replay **1 passed**, mutation_confirmation **1 passed**, mutation_remote
**1 SSH cases passed** using that exact harness selector, mutation_bridges **1 passed**.
All 32 command homes remain accounted for. Unique selected total: **344 tests/cases**,
not counting repeated RED/GREEN invocations or individual matrix rows again.

After the final narrow release-proof guard, reran whole-tree gates and the complete
50-test mutation/6-test read selections. The shared executor/hash implementation was
unchanged after its 159 domain plus five shared-anchor passes. No schema or golden
updates, Cargo/dependency changes, frontend work or new target was needed.

### Scope and next gate

Entry was clean at the fix base. Fetch/merge at entry and handoff found main
`0464c42dae19a6dde000c7e7660e82762a41fada` already incorporated; merge-tree passed at
base with tree `bc959085025a3495ed8818f16df685ec09bc5633`. Work stayed in the ticket
worktree, inline without subagents. All production activation remains in place;
new helpers are internal and read-only. No journal/executor bypass was published.

Both remaining inspection observations are locally addressed, **not independently
approved**. Await controller SPEC re-review of this delta, then QUALITY. Task 8 has
not started, and the ticket remains implementing. No PR, closure, merge into main,
native workflow or cleanup was performed. Native runtime/filter/ACL evidence remains
outside these local Linux checks.

## Task 7 SPEC: fresh-conflict dirty-path policy

Checkpoint **`01M5XRX8YK4GFJNZ6YEXD99GGV`**, recorded **2026-10-27T04:11:48Z**.
Fix base: **`8e7d6dad0de081883a757293f47d68682f4e6069`**. The controller reports
the two preceding inspection observations closed, but identifies a new gap:
fresh observed ConflictPending with no R still advertised resolve after an unrelated
nonignored untracked file appeared. This is a local correction pending SPEC
re-review, not approval of the changed tree.

### Source verification and minimal correction

Verified that `request::inspection` exempted the fresh observed conflict from the
ordinary image proof, while `conflict::image` described indexed paths and merge
metadata, not unrelated untracked paths. The executor's `verify_paths` separately
calls `integration::resolution::only_paths_are_dirty`; that predicate includes
WT_NEW and deliberately permits unrelated ignored files.

The production change is confined to **`local_sync/request.rs`**. Inspection retains
the immutable conflict entries' raw paths as its allowlist and calls that **same
existing predicate** in the fresh-conflict proof branch. It does not decode display
paths, duplicate a weaker status check, acquire an executor/write lease or create R
authority. Nonignored unrelated dirtiness selects external_repair and clears
can_start_resolution. The existing proof closure also reruns the check after
session B. The predicate and execution behavior themselves are unchanged.

### Public RED/GREEN proof

Added two tests to the existing `local_sync_inspection_spec.rs` module:

- `spec_fresh_conflict_untracked_policy_ticket`
- `spec_fresh_conflict_untracked_policy_document`

Both ran RED before the fix: inspection returned **Resolve** where ExternalRepair
was required. Each fixture positively asserts that `foreign-scratch.txt` is
**WT_NEW**, not IGNORED, and that `status_should_ignore` is false. Each also creates
an unrelated ignored scratch file, asserts its IGNORED status, and verifies that
the original conflict remains resolvable with that file present.

Every inspection uses the existing held-repository-lease helper, comparing Git
administrative files (including objects/reflogs), worktree state, relevant SQL and
image proofs before/after. With the nonignored file present, the read gives
external_repair/can_start_resolution=false, no active R, and zero resolution rows.
After the fixture removes only that foreign file, the original conflict again
gives resolve/can_start_resolution=true with its original observation. A real public
resolution then succeeds while the ignored scratch file remains byte-identical.
Both tests passed after the correction. No old assertion was edited or weakened.

### Verification and scope

All Rust commands ran from this ticket checkout through `devenv shell -- cargo`:

| Command after the Devenv prefix | Result |
| --- | --- |
| `fmt --check` | Exit 0. |
| `check --all-features --locked` | Exit 0, 3.00s. |
| `clippy --all-targets --all-features --locked -- -D warnings` | Exit 0, 15.73s. |
| `test --all-features --locked --test mutation_replay spec_fresh_conflict_untracked_policy` | RED: **2 failed**; GREEN: **2 passed**. |
| `test --all-features --locked --test mutation_replay local_sync` | **52 passed**, 308 filtered, 200.38s. |
| `test --all-features --locked --test read_items local_sync` | **6 passed**. |
| `test --all-features --locked --test mutation_contract` | **44 passed**. |
| `test --all-features --locked --test read_contract` | **72 passed**. |
| `test --all-features --locked --test mutation_replay every_command_of_this_target_keeps_the_rules_of_the_boundary` | **1 passed**, including both local command homes. |

Unique selected total for this narrow delta: **175 tests**, not adding the two
focused cases again. No executor/predicate/schema implementation changed,
so the prior domain/shared and other conformance selections were not redundantly
rerun. No full suite, native workflow, golden regeneration or new target was used.

Entry was clean at the fix base. Fetch/merge at entry and checkpoint found main
`0464c42dae19a6dde000c7e7660e82762a41fada` already incorporated; merge-tree passed at
base with tree `2d0fa5addded6664238ef7883dc96f62ae7bc77d`. Work stayed inline in this
ticket checkout, without subagents. Production activation is retained, with no
public journal bypass, frontend/dependency change or lifecycle action.

Next: controller SPEC re-review, then QUALITY. No approval is claimed. Ticket stays
implementing and Task 8 has not started. No PR, closure, merge into main, native CI
or cleanup was performed.

## Task 7 QUALITY: live routing and terminal request DTOs

Checkpoint **`01M5XTX949G7GTX75FGCSB1SXZ`**, recorded **2026-10-27T04:46:45Z**.
Fix base: **`077f421f6cec3ee9fdc255c89ea3b2da2483a951`**. The controller reports
**full Task 7 SPEC approval at that base**, session
**`ses_e8554c7d2ffe8o3xfJnBqcRrQO`**, followed by QUALITY session
**`ses_e846d6706ffeMZlWpMYjwDMnGU`** with **two P2 findings**. These are controller
review reports, not reviews performed by this implementer. Approval applies to the
base; SPEC delta verification and QUALITY re-review remain required for this commit.

### 1. Live routing in ordinary inspection — locally addressed

Verified that ordinary inspection compared stored registration/configuration facts
but not `recover::routing(repo)` against the retained execution routing digest.
The executor and conflict inspection already make that live-file comparison. The
new tests reproduced both failures: a stable change to primary's
`.manyhands/config.toml` still gave Resume, and a change between the first read and
session B still returned a stale resumable DTO.

Inspection now requires the same retained/live routing digest before building its
view and rechecks it after session B. It neither adopts the changed routing nor
refreshes the stored configuration. A stable mismatch gives the existing
ExternalChange ReadError; an in-call change also gives ExternalChange.

Public tests cover **contained, fast-forward and clean merge**, each in **Prepared
and IndexPending**, for both stable and in-call changes: twelve routing cases.
Stable reads run through the existing already-held-repository-lease snapshot helper.
In-call tests hold the same lease, restore only the fixture's deliberate config
change, then compare Git administration/worktree state and SQL. Restoring the
original routing makes the same original S inspect as resumable again.

### 2. Truthful unlinked terminal R/U data — locally addressed

Verified that `terminal_unlinked` served both R and U but always used the executing
request ID as `resolution_request_id`, and mapped both Cancelled and Abandoned
parents to reason Abandoned. The new U test reproduced a fake R identity; the new
R test reproduced Abandoned where the terminal parent was Cancelled.

The helper now reads the actual scoped control role, checked against its request
command/target, and sets `resolution_request_id` only for an actual resolve request.
It selects the exact existing state/reason pair: Cancelled/Cancelled,
Abandoned/Abandoned, or Complete/ForeignChange. No schema or global enum was added.

The new tests exercise **R and U independently under cancelled, abandoned and
complete parents**. Cancelled/abandoned requests finish without own effects, retain
their IDs, and replay immutable receipts; changed input mismatches. Complete-parent
unlinked requests retain the approved no-effect ExternalChange/release behavior.
Resolution-attempt counts and Git/worktree state remain unchanged in all six cases.
The existing atomic-abandonment and unlinked-contender tests also pass. No existing
Finished result is rewritten, and no settlement/retention policy was changed.

### RED/GREEN and verification

New module **`tests/mutation_replay/local_sync_quality.rs`**, registered in the
existing local_sync mutation test module. Two earlier fixture helpers gain only
parent-module visibility for reuse; no old assertion was changed or weakened.

The first command
`devenv shell -- cargo test --all-features --locked --test mutation_replay local_sync::quality`
selected **four tests, all failing at the intended assertions**. After correction,
all four passed:

- `quality_ordinary_inspection_refuses_stable_live_routing_change`
- `quality_ordinary_inspection_rechecks_live_routing_after_session_b`
- `quality_terminal_unlinked_resolve_identity_and_reason`
- `quality_terminal_unlinked_resume_identity_and_reason`

All Rust commands used Devenv in the ticket checkout:

| Command after `devenv shell -- cargo` | Result |
| --- | --- |
| `fmt --check` | Exit 0. |
| `check --all-features --locked` | Exit 0, 2.97s. |
| `clippy --all-targets --all-features --locked -- -D warnings` | Exit 0, 11.68s. |
| `test --all-features --locked --test mutation_replay local_sync` | **56 passed**, 308 filtered, 207.99s. |
| `test --all-features --locked --test read_items local_sync` | **6 passed**. |
| `test --all-features --locked --test mutation_contract` | **44 passed**. |
| `test --all-features --locked --test read_contract` | **72 passed**. |

Ran the single positional selector
`every_command_of_this_target_keeps_the_rules_of_the_boundary` separately in each
existing conformance target: mutation_replay **1 passed**, mutation_confirmation
**1 passed**, mutation_remote **1 SSH cases passed**, mutation_bridges **1 passed**.
The exact SSH harness selector is the same string above. All 32 command homes remain
accounted for. Unique selected total for this delta: **182 tests/cases**; the four
focused tests are included in 56 and are not counted twice.

The production changes are limited to the inspection routing check and terminal
binding DTO construction. No executor/journal disposition implementation changed,
so its prior domain selections were not redundantly repeated. No full suite,
schema/golden rewrite, new target, dependency or frontend change was introduced.
Production activation remains intact and no public journal bypass was added.

### Handoff

Entry was clean at the fix base. Fetch/merge at entry and checkpoint found main
`0464c42dae19a6dde000c7e7660e82762a41fada` already incorporated. Merge-tree passed at
base with tree `aa87a5efc08509c7e414dd99c7f5e7d68b77cd69`. Work stayed inline in the
ticket worktree without subagents. Both P2 findings are locally addressed, pending
**SPEC delta verification and QUALITY re-review**. No approval of this delta is
claimed. Ticket remains implementing; Task 8 has not started. No PR, closure, merge
into main, native workflow or cleanup was performed.

## Task 8: closure acceptance and adversarial evidence

Checkpoint **`01M5XX9Y76ZM2D6FJQKBJS2D13`**, recorded with UTC clock
`2026-10-27T05:28:37Z`; ID minted by the main checkout's existing
`target/debug/manyhands-cli --json id new`. Implementation/review base:
**`0d1b8d006980bf0da116828db54b16728bf55cbd`**. Review the whole delta from that
base, not just the production correction. All work stayed in this ticket worktree
on its existing branch; this implementer used no subagents.

### Prerequisite reviews and preflight

The controller explicitly reports **APPROVED, zero findings** for both Task 7
reviews at the base above, over **`41bce3e..0d1b8d0`**:

- SPEC: `ses_e8554c7d2ffe8o3xfJnBqcRrQO`.
- QUALITY: `ses_e846d6706ffeMZlWpMYjwDMnGU`.

These are controller-supplied reviews, not reviews performed here. They supersede
the historical pending-review status immediately above. The actual Task 0–7 source,
ledger and covering assertions were inspected; completed proof was not reimplemented.
The approved E4 companion, E9 repair DTO arm, three key guards, migration fixture
and REQUEST_TABLES decisions remain intact.

Entry status was clean at the supplied base. Fetch and `git merge origin/main`
at entry and checkpoint both reported already up to date; main remains
`0464c42dae19a6dde000c7e7660e82762a41fada`. Entry ancestry check passed. Before
publication `git merge-tree --write-tree HEAD origin/main` passed with tree
`101c92905b2aeb65bfab0cd3afdb5415f5a8a301` (the committed Task 8 base). No rebase.

### Implemented acceptance and adversarial composition

1. **E3, exactly the two authorized old tests.** Both now reach a real written
   closure checkpoint, advance primary into an actual canonical ticket conflict,
   and observe closure's merge refusal. `synchronize_closing_context` uses production
   `execute(LocalContextSync)`, inspection, explicit immutable-side content read,
   and `execute(LocalContextResolve)`. It proves pending local work fences closure,
   the ordered parents are `[context, primary]`, primary stays put, and the closure
   record is unchanged until an explicit `close_ticket` continuation. The original
   adopted context, `closure_oid`, `closed_at`, F4 one-merge and retirement assertions
   remain, with explicit `closed_by` assertions added. The hand-merge helper and its
   other old consumer were not changed.
2. **Newer primary requires explicit work.** Two new closure tests exercise another
   conflicting primary advance after the first local sync, then a second explicit
   sync and closure resume. No-remote and configured-remote fixtures are separate;
   neither starts SSH. Configured closure uses existing injected child IO only;
   local synchronization and resolution are production public calls. Its tracking
   updates model already-explicit primary synchronization, preserving C9 D2 readiness;
   local sync itself changes neither tracking nor closure state. Two additional public
   mutation tests stop after accepted intent, advance primary, retry the frozen pair,
   replay with exact Git/SQL snapshots, then explicitly sync the newer primary.
   The configured case has a live, refusing, fixture-only transport callback with
   a positive explicit-fetch control and zero calls from local synchronization.
3. **Real process contention.** Isolated current-test-executable children use flushed
   stdout/stdin pipe barriers, no sleep-based ordering. One child holds the common
   Git lease while public resolution returns Busy without any authority/image change;
   after release the same input succeeds. Another pauses after accepting R; the parent
   takes over the same ID and completes, then the stale child returns the winner's
   immutable receipt. Files, raw indexes, all refs, objects/reflogs/Git artifacts,
   request rows, eight local companion tables and registration/context/routing rows
   are identical before/after the losing continuation. This is not simulated process
   isolation or a claim that a returned injected fault is process death.
4. **Healthy versus lost database.** Healthy rebuild during public conflict and
   partial-R states refuses without changing Git, any retained authority row or the
   public inspection, and exact R still completes. Four separate lost-DB fixtures
   retain (a) real conflict metadata/index, (b) actual partially applied clean
   checkout, with Written effect and dirty state but no conflict/merge metadata,
   (c) a deliberately planted private index lock on an otherwise clean result, and
   (d) a clean committed result. Rebuild does not invent S/R or local companions.
   Old inspection/content reads return RecoveryRequired in all four. Fresh observation
   succeeds **only** for (d); the precise earlier refusals are ItemNotFound for
   unindexable conflict-marked canonical content, WorktreeNotClean for partial checkout,
   and RecoveryRequired for the lock. Git/files/index/refs/artifacts remain identical.
5. **Additional public edges.** Malformed companion shape refuses inspection and R
   before acceptance, with all authority/Git snapshots equal. Independently committed
   incompatible closures produce the immutable-disagreement DTO before a retained
   request/owner or Git effect; unrelated refresh still succeeds. Existing public
   input-collision, lost-output, index-failure, foreign-metadata and cancellation
   regressions are retained and selected, not duplicated.
6. **Positive privacy controls.** The new public privacy test plants a resolution
   sentinel in a genuinely owned partial path, then a committed result blob. The
   original source sentinel is actually returned by the explicit immutable-side DTO.
   Ordinary inspection, partial/mismatch/success/replay/request-show envelopes,
   nonempty progress output and ReadError Debug/Display contain none of the source,
   body or title sentinels. Base request/journal stores and **all eight local
   companions including images** are scanned at partial and completed states.
   Discovery titles are not misclassified as request storage.

### RED/GREEN correction and fixture debugging

The strict four-case lost-journal regression ran RED: public inspection returned
**InternalError instead of RecoveryRequired** after database loss. Source tracing
found `local_sync/run.rs::read_evidence` propagating `QueryReturnedNoRows` through
the generic read error conversion. The narrow production correction uses
`OptionalExtension` and maps only an absent scoped local row to RecoveryRequired,
matching `record.rs::read_local_context_sync_record`. No recovery identity, parent,
content, candidate or authority is synthesized. The exact test then passed all
four fixtures and both read APIs; the full relevant local/public groups pass after
the correction. Other SQL errors retain their existing mapping.

Earlier test-development failures were fixture assumptions, not waived contract
failures: missing imports; conflict-marked content absent from discovery; a stale
same-ID caller replaying its completed winner rather than returning a new refusal;
the public remote setter correctly rejecting the test-only protocol; and remote
closure requiring already-synchronized tracking-primary readiness. The transport
fixture now creates its context first, directly configures the isolated test-only
protocol as the existing domain spy does, commits routing and refreshes discovery.
No production behavior was changed to accommodate those fixtures.

### All 25 required outcomes: actual assertion owners

This table supersedes the historical Task 0 planned-ownership table. D denotes
domain/library tests under `src/repository/local_sync/`; P denotes production
public mutation/read composition in `tests/mutation_replay/local_sync*.rs`; R is
`tests/read_items/local_sync.rs`; C is `src/repository/closure/run_tests.rs`.
Names below were checked against their source/assertions and selected in the
executed groups below. **D is not relabeled as wire-level evidence**. All are local
Linux evidence; no row claims native Windows/macOS verification.

| # | Actual owner(s) and assertion evidence |
| --- | --- |
| 1 | D `run_tests::local_sync_required_outcomes_fault_matrix`: durable-intent retry keeps both parents and one ref append. P `local_sync_public_frozen_pair_without_any_remote` and `local_sync_public_frozen_pair_with_unreachable_configured_remote`: accepted public intent, later primary, exact retry and immutable replay. |
| 2 | D `run_tests::no_drop_backend_ambiguity_preserves_until_operator_proof`, `conflict_tests::no_drop_conflict_backend_window_preserves_until_exact_operator_baseline`, and actual first-file/mode no-Drop controls: preserved ambiguous backend and invalid repair refusal; exact operator baseline/result permits continuation. P lost-journal case independently preserves actual partial checkout after DB loss. |
| 3 | D `local_sync_required_outcomes_stale_and_frozen_parent_retry`; P frozen-pair tests assert old incoming parent and absence of later file until fresh explicit sync; C two new second-advance tests assert closure still requires explicit sync/resume. |
| 4 | D `local_sync_required_outcomes_foreign_change_preserves_historical_effects`; P `inspection_spec::spec_non_resolution_foreign_file_or_index_requires_external_repair` and `spec::spec_inspection_foreign_images_and_candidate_identity_are_read_only`: foreign image retained, advice/effects constrained by original evidence. |
| 5 | D `conflict_tests::complete_preflight_refuses_every_invalid_set_before_any_path_write` and `opaque_tokens_refuse_wrong_operation_generation_mode_and_malformed_input`: complete-set/stale/foreign/duplicate/missing checks before writes. R historical-side test and P inspection-spec phase tests prove wire tokens retain only their recorded authority. |
| 6 | D `permission_loss_after_one_resolution_path_preserves_recovery`; P `inspection_spec::spec_partial_two_path_resolution_keeps_untouched_preimages_retryable`, including no-Drop written-before-observation: U requires R, retained original per-path images, exact R finishes. |
| 7 | D `resolution_candidate_ref_index_and_release_interruptions_finish_once` and `spec_no_drop_import_before_observation_reconciles_owned_checkpoint_on_resume_and_replay`: same candidate, one log append, release evidence. P `local_sync_unimported_candidate_requires_exact_resolution_before_resume_takeover` and lost-settlement receipts protect public continuation. |
| 8 | D `control_tests::cancellation_before_effects_is_terminal_and_conflict_cancellation_retains_owner`, `only_artifact_free_domain_abandonment_can_revoke_authority`; P `cancelled_partial_resolution_remains_accepted_and_resendable`: preserved effects and recoverable Accepted S/R, exact resend. |
| 9 | P `local_sync_lost_resolution_settlement_and_start_receipts_are_independent`, `local_sync_public_resolution_and_start_settle_independently`: lost R settlement does not invent S's answer or repeat Git. New frozen-pair replay additionally compares full SQL and Git snapshots. |
| 10 | C E3 helper actually attempts closure while local conflict is pending and verifies unchanged closure authority; inspection and explicit side reads remain usable. D admission tests preserve reciprocal incompatible-owner fences. |
| 11 | C both modified E3 tests: public canonical resolution then explicit closure resume, exact adopted OID, original closure OID/time/author, F4 reconciliation and one retirement. |
| 12 | P `local_sync_healthy_rebuild_preserves_public_conflict_and_partial_resolution`: conflict/partial-R fixtures, full authority/Git/inspection equality across refused rebuild, then successful exact resolution. D prior enumeration/rebuild controls remain selected. |
| 13 | P `lost_journal_distinguishes_conflict_checkout_and_completed_state`: four independent fixtures and exact missing-authority read errors; no cleanup, no reconstructed S/R/companion; fresh observation only on clean committed result. |
| 14 | P `local_sync_no_drop_acceptance_handover_and_acknowledgement_preserve_requests`: actual process exits at acceptance, owned path, handover acceptance and acknowledgement; reopened S/R remain Accepted; exact R finishes. `cancelled_partial_resolution_remains_accepted_and_resendable` pins U's required R. |
| 15 | P `new_cancellation_survives_acknowledgement_and_resume` and `spec::spec_cancel_reply_does_not_acknowledge_a_new_generation`: a coordinated newer issuance leaves counters `(2,1)`, then another stop, then explicit progress; no boolean-loss shortcut. |
| 16 | P `artifact_free_abandon_settles_all_associated_requests_atomically`: index-pending R plus unlinked R/U are settled together, cancelled results replay, changed input mismatches. D `spec_artifact_free_abandon_uses_each_integration_disposition_and_provenance` and public terminal quality tests preserve existing finished receipts. |
| 17 | P `unlinked_accepted_resolution_cannot_write_after_attempt_loss`, actual no-Drop acceptance case, and new cross-process same-ID winner: stale attempt cannot add an effect/settle over the current owner; exact retry or immutable winner receipt is used. |
| 18 | P `accepted_resolution_observation_is_part_of_request_identity` and `spec_no_drop_written_unobserved_path_has_read_only_mismatch_history`: changing only observation yields RequestMismatch with historical effects and unchanged request attempt/files/authority. |
| 19 | D `admission_tests::retained_remote_and_local_admission_never_deadlock` tests same/other context and both acquisition orders; `retained_conflict_reacquisition_checks_local_before_phase_or_epoch` verifies refused remote phase/epoch and local record unchanged; `retained_remote_metadata_blocks_local_even_after_step_applied` checks retained release debt. These are domain admission proofs, not a claim of a live remote transport race. |
| 20 | D `immutable_closure_disagreement_never_fences_the_repository`; new P `local_sync_public_immutable_closure_race_leaves_no_owner_and_allows_other_work`: explicit typed reason, no retained request or local owner, full files/index/refs/artifact/row equality, subsequent refresh succeeds. |
| 21 | D `no_drop_backend_ambiguity_preserves_until_operator_proof` exits inside real checkout/ref lock windows; first-file/mode controls and conflict-installation child prove actual effects without Drop, preserved private artifacts, invalid-repair refusal, and exact operator-proof continuation. Returned faults alone are not counted as this proof. |
| 22 | D `no_drop_owned_metadata_retirement_finishes_automatically`: two child exits after MERGE_HEAD removal; absence counts as retired, exact remainder retires without a new commit/log effect; substituted MERGE_MSG remains refused and preserved. `quality_metadata_retirement_rechecks_remaining_evidence_before_each_unlink` covers later-member changes. |
| 23 | P `request_attempt_handover_preserves_resolution_and_cancellation`: partial R, attempt advancement before domain link, counters retained, old finish rejected, one resolution row. Actual no-Drop request-child case covers death at that handover boundary. |
| 24 | P `unlinked_contender_observes_parent_completion_or_abandonment` plus `artifact_free_abandon_settles_all_associated_requests_atomically`: unlinked R0/U included in cancellation or released on completed parent; terminal quality tests cover all terminal states without dangling Accepted work. |
| 25 | P `spec::spec_parent_completion_between_resume_read_and_link_releases_unlinked_u`, unlinked contender and terminal R/U quality cases: completed parent produces ExternalChange/no own effects, releases unlinked request, creates no new resolution/parent transition. |

### Mandatory Task 8 named-case audit (all 18 exist and were selected)

These are the plan's exact names, with actual file owner and coverage, not a
zero-selection claim. Names shared between D and P/R are listed separately.

| Mandatory name | Actual owner / assertion coverage |
| --- | --- |
| `unrelated_histories_refuse_before_acceptance` | D run_tests, Task 5; both kinds, typed unrelated refusal, no owner/image change. |
| `immutable_closure_disagreement_never_fences_the_repository` | D run_tests, Task 5; same-ID conflicting pairs before acceptance; Task 8 public companion above adds request boundary and complete preservation. |
| `identity_missing_then_set_retries_the_same_local_intent` | D run_tests (5) and P local_sync (7): missing before acceptance, removed after intent, and journaled candidate cases; public confirmed identity setting permits same-ID retry and candidate author stays frozen. |
| `accepted_resolution_observation_is_part_of_request_identity` | P local_sync (7); top-level observation participates in exact R identity with historical effects. |
| `cancelled_partial_resolution_remains_accepted_and_resendable` | P local_sync (7); Accepted S/R, required R, explicit continuation. |
| `new_cancellation_survives_acknowledgement_and_resume` | P local_sync (7), supplemented by P spec cancellation generation regression. |
| `artifact_free_abandon_settles_all_associated_requests_atomically` | P local_sync (7), with D all-disposition/provenance and P terminal-state controls. |
| `unlinked_accepted_resolution_cannot_write_after_attempt_loss` | P local_sync (7); exact handover fences, plus Task 8 actual-process contender. |
| `request_attempt_handover_preserves_resolution_and_cancellation` | P local_sync (7); old attempt cannot settle, counters/source association retained, one resolution; real process death supplied by the request-child companion. |
| `unlinked_contender_observes_parent_completion_or_abandonment` | P local_sync (7); R/U terminal completion and delegated atomic-abandon scenario, supplemented by P quality terminal matrices. |
| `retained_remote_and_local_admission_never_deadlock` | D admission_tests (4); both orders and same/other context, retained reacquisition/metadata companions. |
| `authority_only_change_is_seen_by_fresh_read_session` | D observe_tests (4) and R local_sync (7); semantic/registration change between A/B refuses, owner-only epoch does not invalidate the token. |
| `journal_permission_failure_precedes_git_effects` | D record_tests (4); typed PermissionDenied at actual journal write seam, IndexUnavailable, no envelope or Git change. |
| `side_read_permission_failure_is_read_only` | R local_sync (7); typed side-read seam, RepositoryInaccessible and no side/mutation; P spec adds real Unix path-permission denial. |
| `permission_loss_after_one_resolution_path_preserves_recovery` | D conflict_tests (6); actual next-write seam, one observed path, retained R, original-request requirement, exact continuation; Unix real-directory-permission companion selected too. |
| `no_drop_backend_ambiguity_preserves_until_operator_proof` | D run_tests (5), with actual conflict backend/first-file/mode companions from 5/6. Full assertions retained. |
| `no_drop_owned_metadata_retirement_finishes_automatically` | D conflict_tests (6), two actual no-Drop children including foreign metadata refusal. Full assertions retained. |
| `lost_journal_distinguishes_conflict_checkout_and_completed_state` | New P local_sync (8), four independent states described above; strict read-error assertion drove the narrow production fix. |

### Commands, counts and limitations

All Rust commands used Devenv. Final relevant group reruns after the production
correction are intentional; earlier successful unchanged Task 5–7 baseline work
was not recreated. Counts below are **distinct test functions/cases**, not loop
iterations or an invented total of recovery scenarios. Child entry tests are in
the library/harness counts; their parent assertions establish actual process exits.

| Command after `devenv shell -- cargo` | Observed result |
| --- | --- |
| `fmt --check` | Exit 0 on final Rust tree. |
| `check --all-features --locked` | Exit 0. |
| `clippy --all-targets --all-features --locked -- -D warnings` | Exit 0, no warnings. |
| `test --all-features --locked --lib local_sync:: -- --test-threads=4` | **159 passed**, 1286 filtered; 289.56s final group run. Includes existing genuine no-Drop/backend/retirement and Linux permission proofs. |
| `test --all-features --locked --test mutation_replay local_sync -- --test-threads=4` | **66 passed**, 308 filtered; 99.74s. Prior 56 plus 10 new tests (one child entry, nine adversarial/public cases). Later additional partial-state privacy scan rerun by its exact selector. |
| `test --all-features --locked --test read_items local_sync` | **6 passed**, 76 filtered. |
| `test --all-features --locked --lib repository::closure::run_tests::conflicting_advance_after_the_checkpoint_needs_a_deliberate_synchronization -- --exact` | **1 passed**, 1444 filtered. |
| `test --all-features --locked --lib repository::closure::run_tests::adopted_context_is_merged_reconciled_after_f4_and_retired -- --exact` | **1 passed**, 1444 filtered. |
| `test --all-features --locked --lib repository::closure::run_tests::local_sync` | **2 passed**, 1443 filtered; separate no-remote/configured-remote second-advance cases. |
| `test --all-features --locked --lib public_resolution_checkpoints_document_ticket_comment_and_multi_path_conflicts_locally` | **1 passed**. |
| `test --all-features --locked --lib external_repair_exact_ordered_parent_clean_only` | **1 passed**. |
| `test --all-features --locked --lib clean_merge_integration_fault_matrix_resumes_each_durable_transition_once` | **1 passed**. |
| `test --all-features --locked --lib cancellation_with_a_pending_conflict_is_a_recoverable_stop` | **1 passed**. |
| `test --all-features --locked --lib retained_candidate_cannot_bypass_all_side_closure_validation` | **1 passed**. |
| `test --all-features --locked --lib all_recorded_ticket_sides_constrain_retained_closure_before_writes` | **1 passed**, all retained eight-row assertions. |
| `test --all-features --locked --lib mixed_canonical_and_code_conflict_is_whole_merge_external_only_without_writes` | **1 passed**. |
| `test --all-features --locked --test ticket_closure true_race_distinct_identities_needs_external_recovery -- --exact` | **1 SSH cases passed**, actual exact harness selection. |

Total: **159 + 66 + 6 + 4 + 7 = 242 Rust tests, plus 1 SSH case = 243**.
Focused repetitions are not added again. One concurrently entered Devenv shell
reported a transient `.devenv/load-exports` chmod race even though Cargo's retained
candidate test passed; that exact test was rerun serially with clean shell setup
and passed. Subsequent commands ran serially. No test assertion failure remains.

Platform evidence is **Linux x86_64, UID 1000**. Actual non-root Unix permissions,
typed permission seams and real isolated child processes were exercised. No native
Windows ACL/macOS behavior or broad arbitrary-backend auto-recovery is inferred.
The no-network acceptance fixtures do not use `ticket_closure::Pair(false)`.
No dependency, frontend, schema/global enum, Cargo/test target or native workflow
changed. Only E3's two approved old tests changed semantics. Full-suite/native
verification remains outside this bounded task.

Task 8's promised Linux domain/public outcomes now have assertion-level evidence.
**Independent Part review remains required: controller SPEC, then QUALITY, before
Task 9.** This is implementer verification, not an independent approval. No material
contract contradiction or additional owner scope decision is pending. Ticket remains
implementing. PR, lifecycle closure, merge, cleanup and native dispatch were not run.

## Task 8 QUALITY: bounded child ownership and lock-safe snapshots

Recorded **2026-10-27T10:26:43Z**, checkpoint `01M5YEBSNAVTE45DEJF9QFP5MF`.
Fix/re-review base: **`65541cc48ed00d8f1f6463249924bc70c645dbc6`**.

### Review authority and verified findings

Controller reports Task 8 SPEC **APPROVED, zero findings** at that base, session
`ses_e84247053ffeQkt4obeGjIa5Xy`. QUALITY session
`ses_e84216efcffeqZ4Q0rR0960XJX` then identified two P2 test-harness findings:

1. Both new process tests blocked in `read_line` and `wait_with_output`, with no
   deadline or child-cleanup guard. An assertion before release could leave an
   unreaped child while its repository/data fixture was removed. A deadline checked
   only by the main thread would not help if its lease-contending library call blocked.
2. The lease test recursively read all Git artifacts while the child held the
   exclusive `manyhands-operation.lock`. Windows mandatory locking can reject that
   read before the intended Busy assertion. Source inspection of the test and the
   existing coordination metadata strategy confirmed the issue; Linux success was
   not evidence that the held-file read was portable.

Both findings were verified in the actual base source before edits. SPEC approval
applies to the base, not automatically to this correction. No independent review
of the correction is claimed here.

### Corrections and exact scope

All Rust edits are inside **`tests/mutation_replay/local_sync.rs`**, confined to
Task 8's new child entry/two process tests and their adjacent shared harness/fault
controls. No production, old support helper, dependency or platform configuration
changed.

- `GuardedLocalChild` launches only the isolated current-test executable. One
  supervisor thread exclusively owns that `Child`, accepts explicit Release/Abort
  messages, and enforces an absolute lifetime deadline independently of the caller.
  It polls child exit/deadline with timed channel receives, kills/reaps on expiry
  or abort, and never searches for or kills a developer process.
- Readiness still requires the child's exact flushed stdout pipe marker; release
  is still a byte on stdin. The readiness receive is bounded by the same deadline.
  No sleep guesses establish ordering. Ordinary process tests have a 60-second
  watchdog; deliberately stalled fault controls use short deadlines.
- Concurrent stdout/stderr drainers retain diagnostics without pipe-fill deadlock.
  Readiness failures report both streams and termination status. Completion reports
  distinguish natural exit, deadline and abort. Main guard Drop sends Abort and joins
  cleanup before outer fixtures can be removed. A second process-owner guard also
  kills/reaps if the supervisor unwinds; cleanup avoids a second panic during unwind.
- The watchdog owns the handle and can terminate/reap the child even while the main
  thread is blocked in a lease-acquiring call; it does not depend on borrowing a
  main-thread mutex or running code after that call returns.
- The lease test captures rows, files/index/refs and the **complete** Git artifact
  map before spawning the lease holder. It compares them only after release and
  reaping. The exact lease-file entry and every other artifact remain in the
  comparison: no blanket lock/Git exclusion was introduced.

### Fault controls and verification

Four new tests were written to exercise harness failures, then run together with
the two affected acceptance tests. No production behavioral RED/GREEN claim is
made for this test-infrastructure change. A preliminary compilation caught a fault
branch placed at the earlier child's similar JSON parse; it was moved into Task 8's
owned child entry, leaving the old child unchanged. No runtime failure was waived.

| New control | Actual assertion evidence |
| --- | --- |
| `local_sync_child_harness_bounds_readiness_and_captures_early_exit` | Two isolated children: one emits diagnostics but never signals readiness, one exits with code 23. Both are reaped and both stdout/stderr markers are present in the failure report. |
| `local_sync_child_harness_bounds_completion_after_release` | Child signals readiness and consumes release but deliberately never completes. Supervisor terminates it at the deadline, reports failure and retained stderr, and reaps it. |
| `local_sync_child_harness_reaps_on_unwind_before_fixture_cleanup` | Child owns the real repository lease; a caught parent assertion unwinds the guard before release. Reaped flag is set, the lease is immediately reacquirable, and the full Git artifact map including the lease file is unchanged. |
| `local_sync_child_harness_watchdog_releases_lease_while_caller_is_blocked` | After the exact readiness barrier, a nonblocking probe proves the child still holds the OS lease. Main then performs a real blocking acquisition without sending release. Watchdog expiry kills/reaps the child, the blocked acquisition returns, and the full artifact map remains identical. |

Commands used the ticket worktree and Devenv:

| Command after `devenv shell -- cargo` | Observed result |
| --- | --- |
| `test --all-features --locked --test mutation_replay local_sync_child_harness -- --test-threads=2` | **4 passed**, 374 filtered; 5.00s. Five isolated child failure scenarios. |
| `test --all-features --locked --test mutation_replay local_sync_cross_process -- --test-threads=2` | **2 passed**, 376 filtered; 2.98s. Original same-ID winner and Busy/lease assertions preserved; two isolated children. |
| `test --all-features --locked --test mutation_replay local_sync::local_sync_child_harness_reaps_on_unwind_before_fixture_cleanup -- --exact` | **1 passed**, after strengthening the post-unwind lease-reacquisition assertion. |
| `test --all-features --locked --test mutation_replay local_sync::local_sync_child_harness_watchdog_releases_lease_while_caller_is_blocked -- --exact` | **1 passed**, after adding the positive still-held lease probe before blocking. |
| `fmt --check` | Exit 0 on final Rust tree. |
| `check --all-features --locked` | Exit 0. |
| `clippy --all-targets --all-features --locked -- -D warnings` | Exit 0, no warnings. |

Distinct selection: **six tests, seven child scenarios**; the two focused reruns
are not added to that count. Existing domain/closure/SSH evidence was not redundantly
rerun for this test-only correction. No full suite or native workflow was run.

Entry was clean at the fix base. Fetch/merge found main `0464c42` already incorporated.
The Windows fix is a source-level ordering correction; the fault/containment and
preservation tests ran locally on Linux, not Windows ACLs or a native Windows runner.
The explicit barrier and complete artifact comparisons are retained on all platforms.

Both P2 findings are locally addressed, pending **controller SPEC delta verification,
then QUALITY re-review**. Task 9 remains unstarted; ticket remains implementing.
No PR, lifecycle closure, merge into main, cleanup or native dispatch was performed.

## Task 9: Documentation And Source Audit

Checkpoint **`01M5YF4JK2VQ94PE52V7T91TJJ`**, UTC **2026-10-27T10:40:15Z**.
The ID came from the main checkout's existing
`target/debug/manyhands-cli --json id new`, invoked from this ticket worktree;
no Cargo command or main-checkout edit was made. Docs-only base:
**`03a27cbb7252062429aabfed31ae6c35e2f143fb`**. Work is inline without subagents.

### Superseding review state and preflight

The controller reports **Task 8 SPEC and QUALITY APPROVED, zero findings at
`03a27cb`**, over the full Task 8 range
`0d1b8d006980bf0da116828db54b16728bf55cbd..03a27cb`:

- SPEC `ses_e84247053ffeQkt4obeGjIa5Xy`.
- QUALITY `ses_e84216efcffeqZ4Q0rR0960XJX`.

This supersedes the preceding historical pending-review text, including its two
P2 harness findings. It is a controller-supplied approval, not a review performed
by this implementer. Tasks 0–8's accepted work and verified tests were trusted;
none was redispatched or rerun for this documentation checkpoint. Runtime
activation is already present in Task 7 production callers and remains intact.

Entry status was clean at the exact supplied HEAD, tracking the same remote tip.
Inspected full branch status/diff inventory and log, fetched `origin main`, and
verified it remains **`0464c42dae19a6dde000c7e7660e82762a41fada`**, already an ancestor
of HEAD. No branch/tree move was needed while the controller runs final Rust gates.
`git merge-tree --write-tree HEAD origin/main` passed with tree
`0f6cc810d355e8950f87e8a755ac0a5e3d14c77b` at the committed audit base.
The **final review comparison is `0464c42..branch HEAD`** (equivalently the
merge-base diff here), not merely Task 8 or `HEAD~1`. Historical implementation
planning base **`0cbeab084d252e3823d317a468eee1f29c92c8ce`** remains recorded; the
full-main diff also includes the earlier ticket creation and Wave prerequisite.

### Documentation amendments and actual boundary validation

Updated the exact document families named by the design: Cycle 06 design/protocol,
Cycle 10 design's post-checkpoint instructions, Git workflow synchronization and
recovery sections, CLI conflict/recovery qualification, Wave prerequisite/ownership
row, and API audit. Current gate contract/design/plan/task table now distinguish
implemented Tasks 0–8 from Task 9's pending final controller gates/reviews. Original
Cycle execution ledgers and this ledger's dated results remain historical.

The API audit now records the complete three input structs, two Mutation variants,
three service read signatures, five DTO/schema field inventories (including nulls),
five public local enums, action/step additions, eight companion tables, command
homes and goldens. Source tracing followed exports and production dispatch,
read A/B and Git rechecks, intent hashing, DTO builders, recovery choice/effect
mapping, request controls, resume and guarded abandonment—not just symbol names.
No new global result code or recovery-action kind was found. `src/bin/`,
`src/desktop/`, Cargo/dependencies and workflow/test-target declarations have no
gate diff. New doc-hidden failure points remain test seams, not public recovery
commands. Internal module/table subdivisions implement the approved evidence
categories; they are not an additional operation family or a new product feature.

One documentation precision correction: `LocalContextConflictContentDto.historical`
is **true once R is accepted**, including before its first write, because
`conflict.rs::read` tests `active_resolution.is_some()`. The former design phrase
"after an accepted resolution changed live stages" was too narrow. The design and
audit now describe the actual accepted-attempt meaning; no field, code or policy
changed. The original source remains readable only through the explicit immutable
side DTO and never grants a fresh request write authority.

### Full-branch acceptance self-review

This is a documentation/source-and-evidence self-review over the **full branch
inventory and changed pre-existing test hunks**, with targeted implementation
traces and the accepted Task 0–8 proof. It is not the controller's independent
whole-branch code review or a fresh execution of the tests.

| Acceptance | Source/evidence cross-check and disposition |
| --- | --- |
| A1 | Shared `integration/validate.rs` classification is used by remote and local paths; E1/E2 plus new timestamp/author/ancestor/open/same-pair controls retain no-write assertions. Task 1/8 recorded real two-clone SSH race proof. Accounted for. |
| A2 | All five `ChildExecution` refusal sites pass the recorded parent kind; added context/primary × closure/promotion controls cover inner wording and privacy wrappers. Accounted for. |
| A3 | Production start calls local run/recover and neutral mechanics, not transport. Task 5 both-kind graph cases and Task 8 public frozen-pair/no-remote/unreachable-remote spy evidence distinguish configured remote from network use. Accounted for. |
| A4 | Observation binds registered branch/parents/routing/image, with lease recheck and effective identity at commit. Task 4/5/7 stale, dirty, wrong-context/branch, identity and unrelated-history cases plus public identity retry retain pre-effect refusals. Accounted for. |
| A5 | Conflict/token enumeration and resolve preflight enforce whole-set source/immutable/prospective validity before writes; explicit content DTO is scoped and historical. Task 6/7 invalid-set, mode/token and public read cases cover it. Accounted for. |
| A6 | Request-attempt and domain-epoch fences, owned image/candidate checks, safe-point controls and independent settlement are production-connected. Task 5–8 fault/no-Drop/process/replay evidence is separately labeled; returned errors are not counted as process death. Accounted for. |
| A7 | Task 8's **25 required outcomes** table has actual assertion owners, and its **18 mandatory cases** table names actual selected tests. Retained executed evidence is trusted, not recreated or converted from domain to wire proof. Accounted for. |
| A8 | Both authorized E3 tests call the production start/read/resolve helper then explicit `close_ticket`, asserting the original closure OID/time/author, adoption/F4/retirement. Separate newer-primary and configured-remote controls preserve explicit extra work. Accounted for. |
| A9 | `record.rs` admission, closure observe/save/adopt guards, reciprocal remote reacquisition and global identity guards preserve the matching-Closing-only exception and written-closure publication fence. Both acquisition-order proofs are in Task 4/8. Accounted for. |
| A10 | Inputs have no revealing formatting/serialization; content read alone exposes sides. Request digests and eight companions store metadata, not source. Task 8 positive privacy controls scan owned partial/result sentinels, actual progress and persisted rows. Accounted for. |
| A11 | Shape/version audits, refresh proof and artifact-free abandon prevent generic release/rebuild shortcuts. Task 8 four lost-DB cases distinguish conflict-marked discovery ItemNotFound, dirty checkout, lock evidence and clean completion; old operation reads require authority. Accounted for without a universal error-code claim. |
| A12 | Exact schema fields/nullability/enums, command/action/step/family inventories, 32 command homes, 50 data roots, three read/five mutation golden additions and local progress/effects agree. `original_request_required` names R; U has no fake resolution ID. Accounted for. |
| A13 | **Pending.** Earlier task-specific Devenv/review evidence is retained. Controller independently runs final Rust gates and obtains whole-branch SPEC/QUALITY review. Native limitations are explicit. The downstream gate remains in force until approval/closure/merge. |
| A14 | Local prepare detects incompatible closures before retained owner/Git effects; public test proves subsequent unrelated refresh. Coarse backend ambiguity preserves and blocks until exact operator proof, while proven per-member retirement automatically reconciles. Accounted for. |

### Approved-ruling self-review

| Ruling | Actual implementation/documentation scope |
| --- | --- |
| R-1 | Parsed all-side nonempty closure-pair comparison and whole-set external-only eligibility; no winner policy or weakened legacy/external validation. |
| R-2 | Existing Local envelope plus typed shape-audited companions; no fake remote row, fourth family or unjournaled durable local execution. |
| R-3 | Exact matching written Closing admission; integrating/cleanup/promotion and retained remote debt refuse; closure resumes separately. |
| R-4 | BLAKE3 scoped opaque observations/tokens, no persistent read issuance, raw-byte path ordering and presentation-only escaped paths; dual-session/Git consistency checks. |
| R-5 | Effective Git identity at commit, explicit identity-set/retry when missing, candidate identity frozen; no inline confirmation/config mutation. |
| R-6 | S owns parent, R owns attempt; intended scoped parent survives unlinked acceptance, re-entry advances the attempt fence atomically, missing source names original R. |
| R-7 | Issued/acknowledged counters preserve newer cancellation; recoverable requests stay Accepted. Artifact-free abandon atomically finishes associated S/R/U and preserves immutable finished receipts. |
| R-8 | Written versus committed versus adopted/external effects remain distinct, including partial checkout and discovery debt; Partial/Incomplete precedence retained. |
| R-9 | Exact library command identities/read DTOs exist; no CLI catalog/dispatch or desktop adaptation, human conflict artifact workflow or in-app editor is delivered. |
| R-10 | Neutral extraction retains remote orchestration/transport policy and old assertion bodies, with the explicitly approved reciprocal admission guard. Cooperative-writer and ambiguous-backend boundaries remain. |

### E1–E9 and owner extras: exact old-test/scope diff audit

Checked `origin/main...03a27cb` name-status/numstat and changed existing test hunks,
including test modules embedded in production files. New local tests and later
review fixes to those newly added tests are additions relative to main, not hidden
rewrites of the old suite.

| Edit | Actual full-branch change |
| --- | --- |
| E1 | `tests/ticket_closure/races.rs::true_race` changes eligibility and fresh-resolution result to external-only, retaining image/no-write comparisons. Exact SSH selector remains `true_race_distinct_identities_needs_external_recovery`. |
| E2 | `remote/sync_tests.rs::all_recorded_ticket_sides_constrain_retained_closure_before_writes` adds an incompatible-row flag and fresh inspection; only that row becomes ExternalResolutionRequired. Other validation/success rows and preservation checks remain. |
| E3 | Only `conflicting_advance_after_the_checkpoint_needs_a_deliberate_synchronization` and `adopted_context_is_merged_reconciled_after_f4_and_retired` replace hand synchronization with the public helper. The second now creates a real canonical conflict before F4; original adoption/retirement assertions remain, with closure-authority assertions strengthened. The manual helper and its other old consumer are unchanged. |
| E4 | `read/status_tests.rs` action inventory and companion partition grow 13 → 14; recovery step inventory adds `local_sync_prepared`. Companion edit is approved in `01M5V0RYTAKJE3F155FH4QTP53`. |
| E5 | Exactly the two command inventory tests account for local safe points, Observing progress and R's parentless durable association; all old command cases remain. |
| E6 | HOMES grows 30 → 32 with both local commands; mutation-contract data roots grow 46 → 50; new golden registrations satisfy the unchanged every-command checks. |
| E7 | Read-contract adds local action and all new DTO enum/root coverage; three existing operation schemas each add only `context_sync_local`. No existing value or global action is removed. |
| E8 | Module/re-export/test registrations and private extraction rewiring; the old comment-invariant test changes only its call target from `RepositoryService::preserves_item_invariants` to `validate::preserves_item_invariants`. New merge/reservation/shared tests are additions. |
| E9 | Three read/five mutation golden registrations, replay conformance chain, and exactly seven explicit new-variant refusal arms in `ticket_data`, `document_data`, `remote_data`, `operation_data`, `key_data`, `identity_data`, `repair_data`. Repair helper approved in `01M5V0RYTAKJE3F155FH4QTP53`; no old golden bytes changed. |
| Owner key-file addendum | `keys/{generation,deletion,registry}.rs` only add local-owned-ID refusal before insertion/effects; deletion wraps insertion/check in an immediate transaction. Approval `01M5VWDPG57KWRAJ55Y2EMXRQR`; no old key-test assertion rewrite. |
| Owner migration-fixture addendum | `tests/mutation_replay.rs::the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`, private REQUEST_TABLES/removal list and one exact marker accounting, approved in `01M5X3KKAFXVGD84SYBH3TBGFB`. Old-column, unrelated-data and privacy assertions remain. |

No unexpected old-test semantic edit, old golden rewrite, frontend/dependency/test
target change or additional owner scope decision was found in this audit. No new
application behavior or code fix was made. Historical staging comments in source
(for example `bridges/abandon.rs` saying Task 7 supplies the route) are not current
availability claims: the actual route above that guard is production-linked and
the current docs now say so.

### Checkpoint verification and outstanding gates

`git diff --check` passed (exit 0). Current diff/status/log and linked document
targets/headings were reviewed, alongside the explicit source/schema comparisons
above. `git diff --quiet 03a27cb -- src tests schemas Cargo.toml Cargo.lock
.github/workflows/build.yml` passed (exit 0), confirming no Task 9 Rust/test/schema/
Cargo/workflow delta. This worker executes **no Rust gate**, preserving the controller's concurrent final
verification tree. Earlier Task 8 evidence remains **243 selected tests/cases**
for its implementation, then **six affected tests/seven child scenarios** for its
test-only harness correction, not a freshly combined or rerun total.

All executed platform evidence retained here is **Linux x86_64**. Non-root Unix
permissions, typed seams and real bounded child processes do not establish native
Windows ACL/macOS results. The lock-snapshot ordering correction is source-level
portability work, not a native pass. Native CI remains separately authorized after
merge, on main. No native dispatch occurs at this checkpoint.

**Task 9 remains in progress.** Await controller final Devenv results and independent
whole-branch SPEC/QUALITY reviews (including adversarial ownership/partial-R/
cancellation/cache-loss review). Only then may the controller establish review-ready
status. The ticket remains implementing and its three original scope items are
preserved; no PR, closure, main merge or cleanup is performed. The docs checkpoint
is committed/pushed under existing authorization with the required co-author trailer.
