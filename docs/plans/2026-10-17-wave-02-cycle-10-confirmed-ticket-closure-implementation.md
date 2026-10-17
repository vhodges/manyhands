---
title: "Wave 02 Cycle 10 Confirmed Ticket Closure Implementation Plan"
date: 2026-10-17
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M55D122HHHKQERDE71WEMPNF"
---

# Confirmed Ticket Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task by task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> **Approved by the owner on 2026-10-17**, together with the Cycle, the design
> and named test edit 1. Execution mode: **subagent-driven development**, with
> independent spec then quality review at each task. Implementation authority
> and task checkpoint commit authority still need to be granted. Native CI runs
> after merge (owner decision 2026-10-17); it is not a gate in this plan.

**Goal:** Two headless operations: `preview_ticket_closure`, an exact local
read-only preflight with a digest that binds the closure identity, and
`close_ticket`, a resumable confirmed lifecycle that records
`closed_at`/`closed_by` in a `Close ticket <ULID>` checkpoint, merges with
`Integrate closed ticket <ULID>`, publishes, cleans up and retries only
unfinished work. Plus a domain guard against writes to closed tickets.

**Architecture:** Make Cycle 09's promotion runner generic over a
crate-private `Lifecycle` trait with associated types, keeping every promotion
signature as a wrapper (Task 1, behavior-preserving). Add a `closure` module
with its own record table, observation, digest and checkpoint step driven by
that runner, and a separate ticket guard.

**Tech Stack:** Rust, `git2` 0.20 (locked libgit2/libssh2), `rusqlite`,
`blake3`, `time`, the in-process `russh` SSH fixture. No new dependency;
adding one stops for review.

**Spec:** [Cycle 10](../Cycles/wave-02-cycle-10-confirmed-ticket-closure.md)
and its
[design](2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-design.md).
Executors read both, and the
[Cycle 09 design](2026-10-15-wave-02-cycle-09-confirmed-document-promotion-design.md)
whose rules the design inherits. Owner decisions are marked **(C1)–(C12)**;
inherited Cycle 09 decisions **(D1)–(D7)**.

**Ticket:** `01K7F6H9J2N4Q6S8V0X2Z4B6DJ`. Worktree
`.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DJ`, branch
`manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DJ`.

## Global Constraints

- Every Rust command: `devenv shell -- cargo ...`. Never on Sprites.
- All work in the ticket worktree. **Bring the branch up to date by merging
  `origin/main` into it, never by rebasing** (owner ruling 2026-10-17: the
  branch is published). Never rebase main onto the branch.
- Shared domain code stays free of GPUI. No CLI, desktop or F2 boundary change;
  `bind/resume.rs` keeps closure `NotResumable`.
- No network call, prompt or discovery scan holds the repository lease; every
  local change re-observes under the lease first.
- Never force-push, rebase, stash or discard. Cleanup removes only state
  verified clean at the recorded OID.
- Closure records hold IDs, ref names, OIDs, timestamps (`closed_at`
  included), fixed step names and fixed categories only. **Never `closed_by`**,
  a title, body, path content, endpoint or secret; no `Debug` of request data.
- The promotion digest domain (`manyhands promotion v1`), table, migration
  marker, step names, subjects and failure-point names do not change.
- **Existing tests stay green and unedited**, except compile-only import
  changes and exactly these named edits:
  1. `tests/local_authoring.rs` `ticket_edit_preserves_unknown_closure_and_exact_body`
     becomes a refusal test (Task 7). **Owner-approved 2026-10-17.**
  2. The `RepositoryErrorKind` completeness lists gain `TicketClosed`:
     `src/repository/mutation/outcome_tests.rs`
     (`every_repository_error_kind_has_its_code`) and the `KINDS` list in
     `src/repository/read/tests.rs` (Task 7).
  3. The exact migration-marker list in `tests/recovery_foundation_gate.rs`
     gains `cycle_10_ticket_closures` (Task 2).
  4. The column-scoped privacy scanner moves from `tests/document_promotion.rs`
     `scan_rows` to `tests/support/` unchanged in behavior (Task 8).
  Any other needed semantic edit to an existing test stops for review.
- Tests use counters, holds and barriers, never sleeps; never touch developer
  keys, global Git configuration or real application data.
- New crate-internal items use `pub(in super::super)`-style paths that compile
  inside the source-included `production` test wrappers (Cycle 09 T7-8: 44
  E0433 errors). Check with the full test build, not only `--lib`.
- Fixtures that hash or snapshot files under `.git` must tolerate the Windows
  mandatory lock on `.git/manyhands-operation.lock` and compare paths through
  one canonical root (Cycle 09 native fixes `35d4ef7`, `213c04b`).
- Write the failing test first and record the red run before production code.
- **Each task ends green on** its targeted tests, every affected target the
  task names, and Clippy with warnings denied
  (`devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`).
  Lint debt is never carried forward (Cycle 09 carried 28 lints to Task 8).
- Tool calls cap at 10 minutes: run longer Cargo commands in the background
  and wait for completion. Budget at least 3 hours for the Task 9 full suite.
  SSH cases have a fixed 100-second watchdog each; split large matrices.
- A characterization or review finding that contradicts the design stops work
  for an owner amendment. Do not adapt silently.
- Commit, push, pull request, native dispatch, merge, closure and cleanup each
  need their own authorization. Task checkpoint commits only if the owner's
  implementation authority includes them.

## Review Focus

Inputs the spec implies but no acceptance row names. Each has a test in its
owning task.

1. **A hand-written ticket** (`manyhands_*` keys first, quoted scalars, unknown
   keys such as `wave`/`cycle`, CRLF line endings): canonical re-serialization
   reorders keys but every parsed field, unknown key and the body must survive,
   and preview and execution must agree. Task 4
   (`closed_ticket_bytes_preserves_hand_written_fields`,
   `closed_ticket_bytes_crlf`), Task 3 (`preview_and_execution_share_closed_ticket_check`).
2. **An executable `ticket.md` (`100755`)**: allowed as a canonical source; the
   checkpoint keeps the mode. Task 4 (`checkpoint_preserves_executable_mode`).
3. **A host clock in a non-UTC zone or with sub-second precision**:
   `closed_at` is whole seconds with `Z`. Task 4
   (`closed_at_is_whole_utc_seconds`).
4. **A comment checkpointed on the ticket but still publish-pending**:
   closure refuses with `CommentPublicationPending` until the comment's own
   publication finishes or is abandoned (C12). Task 3
   (`pending_comment_publication_blocks`) and Task 8 journey.
5. **A ticket context that changes other items or code** (whole-branch effect):
   the preview lists every path, and the digest changes when any changes. Task
   3 (`preview_lists_whole_branch_changes`).

---

## File Map And Task Order

| Files | Ownership |
| --- | --- |
| `src/repository/promotion/lifecycle.rs` (new) | `Lifecycle` trait, `LifecycleKind` (Task 1) |
| `src/repository/promotion/{run,integrate,observe,record,mod}.rs` | Generic over `Lifecycle`; promotion signatures kept as wrappers (Task 1) |
| `src/repository/remote/reservation.rs`, `remote/sync.rs` | Kind-parameterized arms (Task 1); `Close` arms (Task 5) |
| `src/repository/cleanup.rs` | `RetirementEvidence::matches(kind, ..)` (Task 5) |
| `src/repository/recovery.rs`, `src/repository/read/status.rs` | Cross-checks and listing over both parent tables (Task 2) |
| `src/repository/closure/mod.rs` (new) | Public closure types and operations (Tasks 2, 3, 6) |
| `src/repository/closure/record.rs`, `record_tests.rs` (new) | `ticket_closures` (Task 2) |
| `src/repository/closure/observe.rs`, `observe_tests.rs` (new) | Observation, blockers, digest (Task 3) |
| `src/repository/closure/checkpoint.rs`, `checkpoint_tests.rs` (new) | Closed ticket bytes, checkpoint, C9 path, reconciliation (Task 4) |
| `src/repository/closure/run_tests.rs` (new) | Lifecycle tests (Task 6) |
| `src/repository/ticket_guard.rs`, `ticket_guard_tests.rs` (new) | `require_ticket_open` (Task 7) |
| `src/repository.rs` | Modules, re-exports, three `FailurePoint`s (Task 4), `RepositoryErrorKind::TicketClosed` and guard calls (Task 7), migration call (Task 2) |
| `src/repository/discovery.rs`, `remote/state.rs` | Migration hook and startup audit (Task 2) |
| `src/repository/mutation/outcome.rs`, `src/repository/read/mod.rs` | `TicketClosed` code mapping (Task 7) |
| `src/repository/mutation/bind/ticket.rs` | Comment on `refuse_closed` updated (Task 7) |
| `tests/ticket_closure.rs`, `tests/ticket_closure/*.rs` (new), `Cargo.toml` | Real-SSH target, `harness = false` (Task 8) |
| `tests/support/` | Extracted column-scoped privacy scanner (Task 8) |
| `tests/repository_enablement.rs`, `tests/recovery_foundation_gate.rs` | Table inventory, marker list (Task 2) |
| `.github/workflows/build.yml` | Append `--test ticket_closure` (Task 8) |
| `docs/research/wave-03-api-audit.md`, Cycle 09 design/handoff | Delivered API; correction note (Task 9) |
| `docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md` (new) | Execution ledger (Task 0 onward) |

Dependencies:

| Task | Depends on |
| --- | --- |
| 1 Seam | 0 |
| 2 Record | 1 |
| 3 Observation | 1, 2 (blocker mask) |
| 4 Checkpoint and integration | 2, 3 |
| 5 Reservation and completion | 1, 2 |
| 6 Orchestration | 1–5 |
| 7 Ticket guard | 0 (independent; may run in parallel after Task 1 review) |
| 8 SSH proof | 6, 7 |
| 9 Verification | all |

## Task 0: Base, Decisions, Characterization, Baseline

**Files:** Create the execution ledger. No source changes are kept.

- [ ] Confirm the owner's approval of Cycle, design and plan, C1–C12, the
  named existing-test edit in Global Constraints item 1, and the
  implementation authority (execution mode, commit authority, test scope).
  Record them in the ledger.
- [ ] `git fetch origin`. If `origin/main` moved past `3765524`, **merge** it
  into the ticket branch (`git merge --no-edit origin/main`) and record the
  before and after SHAs. Verify `git merge-base --is-ancestor origin/main HEAD`
  and a clean status. Re-read `AGENTS.md` if it changed.
- [ ] Re-scan and record callers of: every `record::`, `observe::` and
  `integrate::` function `promotion/run.rs` calls; `finish_promotion`;
  `restart_remote_promotion`; `RetirementEvidence`; `ChildExecution`; every
  SQL literal naming `document_promotions`; `RemoteOperationAction::Close`;
  `save_ticket_with_effective_config`; `prepare_context`;
  `checkpoint_comment_locally`; `repair_item`. Compare with the design's
  source boundary; a new document-only site joins Task 1's list.
- [ ] Characterize with throwaway, uncommitted tests and record each finding:
  1. `canonical::serialize_item` on every `.manyhands/tickets/*/ticket.md` in
     this repository, a CRLF ticket and a ticket with unknown keys, with
     closure fields added: re-parse equals the original plus the two fields.
     A ticket that fails is a design stop.
  2. A commit object written unreferenced to the repository ODB is visible to
     `prepare_clean_merge(base, that_commit)`.
  3. `sync::fast_forward` (checkout first, then guarded ref update) on a
     repository opened at the ticket context worktree moves the context branch
     to a child commit and leaves the worktree clean; record the state after a
     simulated stop between checkout and ref update.
  4. `exact_mappings`/`mappings_match_scope` admit a `SshDirection::Delete`
     mapping for a ticket context target, and the fixture's deletion receipt
     proof accepts a ticket ref.
  5. How a `harness = false` target runs a single named case (Cycle 09 found
     `--skip` rejected). Record the exact command.
  6. The columns that identify a pending `operation_records` row's target item
     and an unfinished `comment_publication_bindings` row's ticket, for C12.
  7. F2 `ticket save` replay of an already-applied request after the ticket is
     closed: does the binding answer already-applied before calling the
     domain? Task 7 must not change binding answers.
- [ ] Run and record the targeted baseline Task 1 must keep: `--lib promotion`,
  `--lib cleanup`, `--lib remote::reservation`, `--lib read::status`,
  `--lib recovery`, and the targets `document_promotion`,
  `remote_synchronization`, `remote_merge_recovery`, `remote_polling`,
  `comment_publication`, `remote_reservation`, `read_status`,
  `mutation_remote` (each `devenv shell -- cargo test --locked --test <name>`,
  in the background). Record counts and wall time. Run Clippy; record that it
  is clean at the base.
- [ ] Ticket comment (canonical file under
  `.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DJ/<ULID>.md`, front matter as
  existing comments; ULIDs valid Crockford base32, no `I L O U`) with base,
  decisions, characterizations and baseline.

## Task 1: Lifecycle Seam (Behavior-Preserving)

**Files:** Create `src/repository/promotion/lifecycle.rs`,
`lifecycle_tests.rs`. Modify `promotion/{mod,run,integrate,observe,record}.rs`,
`remote/reservation.rs`, `remote/sync.rs` (child fence call), `cleanup.rs`
(call sites).

**Interfaces:**
- Produces:

```rust
// src/repository/promotion/lifecycle.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum LifecycleKind { DocumentPromotion, TicketClosure }
impl LifecycleKind {
    pub(in super::super) fn authoring(self) -> AuthoringKind;      // Document | Ticket
    pub(in super::super) fn action(self) -> RemoteOperationAction; // Promote | Close
    pub(in super::super) fn merge_subject(self, item: &ItemId) -> String;
    //   "Promote document <ULID>" | "Integrate closed ticket <ULID>"
    pub(in super::super) fn context_ref(self, item: &ItemId) -> String;
    //   "refs/heads/manyhands/{document|ticket}/<ULID>"
}

pub(in super::super) trait Lifecycle {
    type Record;
    type Error;
    type Preview;
    type Expectation;
    const KIND: LifecycleKind;
    // One method per record::/observe::/integrate:: function the runner calls
    // at the base, with today's argument and return types translated to the
    // associated types. Step 1 writes the exact list into the ledger.
    fn pre_integration(&self, stage: &mut LeasedIntegration<'_>)
        -> Result<PreIntegration, Self::Error>;            // promotion: Ok(PreIntegration::None)
}
pub(in super::super) struct DocumentPromotionLifecycle;   // implements Lifecycle
```

  Every existing `pub`/`pub(..)` promotion function keeps its signature and
  delegates to the generic code. `LeasedIntegration` and `PreIntegration` are
  defined here as the hook's input (the lease-held repository, observation,
  record handle) and output (`None`, or `Checkpoint { merged_context: Oid }`).

- [ ] **Step 1: List.** In the ledger: every document-only site (design
  source boundary plus Task 0 re-scan) with its replacement, and every
  `record::`/`observe::`/`integrate::` call in `run.rs` with its trait method.
- [ ] **Step 2: Failing tests** in `promotion/lifecycle_tests.rs`:
  `document_kind_matches_legacy_constants` (values equal the literals at the
  base) and `ticket_kind_constants`. Run
  `devenv shell -- cargo test --locked --lib promotion::lifecycle`; record the
  compile failure.
- [ ] **Step 3: Implement** `lifecycle.rs` and `DocumentPromotionLifecycle` by
  moving code, not rewriting it.
- [ ] **Step 4: Thread the kind** through `run.rs`, `integrate.rs`,
  `observe.rs` (including `observation_guard`'s hashed directory), reservation
  and sync call sites. `reconcile_integration` searches for
  `KIND.merge_subject(item)`. `finish_promotion` becomes `finish_parent(kind, ..)`
  with today's SQL in the `Promote` arm. A `TicketClosure` kind reaching
  reservation code returns `state::recovery_required()` until Task 5.
- [ ] **Step 5: Gate.** Every Task 0 baseline count unchanged; no existing test
  edited beyond imports; Clippy clean.
- [ ] **Step 6: Independent review**, spec then quality, focused on F10/F11
  lock and lost-commit paths, `KnownPublication`, ownership-first child
  selection, the child fence transaction and subject-keyed reconciliation. No
  Task 2 before both pass.
- [ ] **Step 7: Checkpoint** (if authorized) `refactor: make the promotion
  lifecycle generic`, ticket comment.

## Task 2: `ticket_closures` Record And Cross-Checks

**Files:** Create `src/repository/closure/{mod.rs, record.rs, record_tests.rs}`.
Modify `promotion/record.rs` (ID collision and identity queries),
`recovery.rs`, `read/status.rs`, `discovery.rs` (migration call),
`remote/state.rs` (audit), `src/repository.rs`, `tests/repository_enablement.rs`,
`tests/recovery_foundation_gate.rs` (named edit 3), `tests/read_status.rs`
(new case).

**Interfaces:**
- Consumes: `Lifecycle`, `LifecycleKind` (Task 1).
- Produces:

```rust
pub(in super::super) enum ClosureStep {
    Confirmed, ContextSynchronized, Closing, Integrating, Integrated, Published,
    PublicationPending, RemoteDeleted, WorktreeRemoving, WorktreeRemoved,
    BranchDeleted, Complete, Abandoned, Superseded,
}
pub(in super::super) enum ClosureKind { Record, Existing }
/// Same fields and field types as PromotionRecord, plus the four closure fields.
pub(in super::super) struct ClosureRecord {
    /* every PromotionRecord field, same types (refusal, updated_at included) */
    pub closure_kind: ClosureKind,
    pub closed_at: Option<OffsetDateTime>,       // whole UTC seconds
    pub closure_oid: Option<Oid>,
}
impl ClosureRecord { pub fn merged_context(&self) -> Option<Oid>; } // closure_oid.or(context_oid)
pub(in super::super) fn migrate(tx: &Transaction<'_>) -> Result<(), RepositoryError>;
pub(in super::super) fn audit(tx: &Transaction<'_>) -> Result<(), RepositoryError>;
pub(in super::super) fn begin_closing(tx: &Transaction<'_>, repository_id: i64,
    operation: OperationId, base: Oid, closed_at: OffsetDateTime, closure: Oid)
    -> Result<(), RepositoryError>;              // ContextSynchronized → Closing
pub(in super::super) fn replace_unmoved_closure(tx: &Transaction<'_>, repository_id: i64,
    operation: OperationId, closure: Oid) -> Result<(), RepositoryError>;
// Plus one function per promotion record function the Lifecycle trait needs,
// same shape, own SQL.
pub(in super::super) const CLOSURE_BLOCKERS: [ClosureBlocker; 21]; // design order
```

  `ClosureBlocker` is declared in `closure/mod.rs` here with the design's 21
  variants in order, so the mask is fixed before Task 3.

- [ ] **Step 1: Failing tests** in `closure/record_tests.rs`:
  `migration_is_all_or_nothing_and_idempotent`;
  `steps_permit_only_designed_transitions` (`ContextSynchronized→Closing→Integrating`,
  `ContextSynchronized→Integrating` for `Existing`, `Closing→Abandoned` only
  while the checkpoint ref has not moved as the runner reports it, never
  `Closing→Superseded`);
  `step_invariants_enforced` (each row of the design's invariant table);
  `context_oid_immutable_once_set`; `merged_context_prefers_closure_oid`;
  `one_unfinished_row_per_item`; `closing_row_makes_new_id_busy`;
  `supersession_only_from_context_changed_ignored_remote_changed`;
  `abandoned_refusal_replays`; `blocker_mask_round_trips_all_21`;
  `closed_at_stored_as_whole_utc_seconds`;
  `no_closed_by_column_or_value`;
  `operation_ids_collide_across_both_parent_tables_and_journals`.
  Add the inventory entry in `tests/repository_enablement.rs` and the marker
  in `tests/recovery_foundation_gate.rs`. Add
  `status_lists_closure_parent_and_hides_its_reservation` to
  `tests/read_status.rs` (action `close`, state `closing`). Record red.
- [ ] **Step 2: Implement** schema, marker `cycle_10_ticket_closures`, audit,
  the record functions, and extend every cross-journal query from Task 0
  (`promotion/record.rs`, `recovery.rs`, `read/status.rs`) to name both tables.
- [ ] **Step 3: Gate.** `--lib closure::record`, `--lib promotion`,
  `--lib read::status`, `--lib recovery`, `--test read_status`,
  `--test repository_enablement`, `--test recovery_foundation_gate`,
  `--test document_promotion`, Clippy.
- [ ] **Step 4: Review, checkpoint** `feat: persist ticket closure records`,
  ticket comment.

## Task 3: Closure Observation, Blockers, Preview And Digest

**Files:** Create `closure/observe.rs`, `observe_tests.rs`. Modify
`closure/mod.rs` (public types, `preview_ticket_closure`),
`promotion/observe.rs` (extract shared change-set, D2/D5/D6 and
`PreparedObservation` helpers; no behavior change), `src/repository.rs`
(re-exports).

**Interfaces:**
- Consumes: `LifecycleKind`, `select_committing_identity` (`remote/sync.rs`),
  `closed_ticket_bytes` (declared here, implemented and fully tested in Task
  4: Task 3 adds it with its final body because preview and execution must
  share it).
- Produces: the design's public types exactly as named, and:

```rust
pub(in super::super) struct ClosureObservation {
    pub item_id: ItemId, pub ticket_path: PathBuf,
    pub context_oid: Oid, pub base: Oid, pub remote: Option<String>,
    pub changes: Vec<ChangeEvidence>,           // pre-closure, with mode (T2-1)
    pub closure: PlannedClosure,                // Record { closed_by } | Existing { closed_at, closed_by }
    pub integration: PlannedIntegration,
    pub needs_context_child: bool,              // false when contained
    pub cleanup: PlannedCleanup,
    pub identity: Option<CommittingIdentity>,   // never Debug-printed
    pub expectation: ClosureExpectation,
}
pub(in super::super) fn prepare_observation(..) -> Result<PreparedClosureObservation, RepositoryError>; // outside lease
pub(in super::super) fn revalidate_under_lease(..)
    -> Result<Result<ClosureObservation, Vec<ClosureBlocker>>, RepositoryError>;
pub(in super::super) fn closed_by_text(identity: &CommitIdentity) -> String; // "<name> <<email>>"
pub(in super::super) fn closed_ticket_bytes(original: &[u8], closed_at: OffsetDateTime,
    closed_by: &str) -> Result<Vec<u8>, ClosureContentError>;
pub(in super::super) const CLOSURE_DIGEST_DOMAIN: &[u8] = b"manyhands closure v1";
```

- [ ] **Step 1: Failing tests** in `closure/observe_tests.rs`:
  - each of the 21 blockers alone, and all together in fixed order;
    `AlreadyClosed` replaces `ContextMissing`; `IdentityRequired` only for
    `Record`, only when nothing else blocks, with `identity_configuration`;
  - each row of the design's integration table (seven tests named by row);
  - `pending_local_operation_blocks`, `pending_comment_publication_blocks`
    (Review Focus 4);
  - `context_with_closure_fields_previews_existing`;
  - `preview_lists_whole_branch_changes` (Review Focus 5);
  - `preview_and_execution_share_closed_ticket_check` (Review Focus 1: a
    ticket `closed_ticket_bytes` refuses is `TicketInvalidAfterClosure` in
    preview);
  - digest stable; changes with each bound fact (item, path, branches, remote,
    integration kind, each change entry, mode only `100644→100755` and
    `→120000`, closure tag, `closed_by`, existing `closed_at`, cleanup set,
    and without a remote each OID); unchanged by a primary-only advance with a
    remote (D6) and by the time of day;
  - `effective_identity_wins_over_confirmation`,
    `confirmation_must_match_configuration`;
  - `preview_is_read_only`: no lease file touched, no SQL write, no network
    (transport counters), no ref or worktree change.
  Record red.
- [ ] **Step 2: Implement** using the extracted promotion helpers with
  `LifecycleKind::TicketClosure`, the pending-work queries from Task 0 item 6,
  the integration table, and the digest encoding exactly as the design lists.
- [ ] **Step 3: Gate.** `--lib closure`, `--lib promotion`,
  `--test document_promotion`, Clippy.
- [ ] **Step 4: Review, checkpoint** `feat: add ticket closure preflight and
  preview`, ticket comment.

## Task 4: Closure Checkpoint, Integration And The Post-Checkpoint Path

**Files:** Create `closure/checkpoint.rs`, `checkpoint_tests.rs`. Modify
`promotion/integrate.rs` (accept a merged-context OID and prepared result tree
from the hook; no promotion behavior change), `src/repository.rs`
(`FailurePoint::{ClosureAfterClosingRecorded, ClosureDuringCheckpointCheckout,
AfterClosureCheckpoint}`).

**Interfaces:**
- Consumes: `ClosureObservation`, `closed_ticket_bytes` (Task 3);
  `begin_closing`, `replace_unmoved_closure` (Task 2); `sync::fast_forward`,
  `prepare_clean_merge`, `LocalConfigEdit::begin`, `would_persist_identity`.
- Produces:

```rust
pub(in super::super) struct PreparedClosure {   // objects in the ODB, unreferenced
    pub closed_at: OffsetDateTime, pub ticket_blob: Oid,
    pub checkpoint_commit: Oid, pub result_tree: Oid,
}
pub(in super::super) fn prepare_closure(repo: &git2::Repository, observation: &ClosureObservation,
    closed_at: OffsetDateTime, identity: &CommitIdentity) -> Result<PreparedClosure, ClosureStop>;
pub(in super::super) fn move_context_to_checkpoint(context_repo: &git2::Repository,
    item: &ItemId, from: Oid, to: Oid) -> Result<(), RepositoryError>; // fast_forward order
pub(in super::super) enum ClosingState {
    NothingWritten, CheckoutDone, CheckpointWritten { tip: Oid }, Unrecognized,
}
pub(in super::super) fn reconcile_closing(context_repo: &git2::Repository,
    record: &ClosureRecord) -> Result<ClosingState, RepositoryError>;
pub(in super::super) struct CheckpointAuthority { pub closed_by: String, pub signature: CommitIdentity }
pub(in super::super) fn checkpoint_authority(repo: &git2::Repository, closure: Oid)
    -> Result<CheckpointAuthority, RepositoryError>;                 // from the commit, never config
pub(in super::super) fn post_checkpoint_result(repo: &git2::Repository, base: Oid, tip: Oid)
    -> Result<Result<Oid, ()>, RepositoryError>;                    // tree, or Err(()) on conflict
pub(in super::super) fn now_whole_utc_seconds() -> OffsetDateTime;
```

- [ ] **Step 1: Failing tests** in `closure/checkpoint_tests.rs`:
  - `closed_ticket_bytes_preserves_hand_written_fields` (this repository's
    own ticket shape: every field, unknown key and body equal after re-parse;
    only the closure fields added) and `closed_ticket_bytes_crlf`
    (Review Focus 1);
  - `checkpoint_preserves_executable_mode` (Review Focus 2);
  - `closed_at_is_whole_utc_seconds` (Review Focus 3: non-UTC input with
    nanoseconds, `...Z` whole seconds out);
  - `checkpoint_commit_has_exact_subject_parent_and_single_path`;
  - `prepare_builds_nothing_referenced` (no ref, worktree or index change);
  - `no_remote_conflicting_merge_refuses_before_any_ref_moves`;
  - `move_context_leaves_worktree_clean_at_checkpoint`;
  - `reconcile_closing` rows, one test each: ref at `context_oid` and clean →
    `NothingWritten`; ref at `context_oid` and only the ticket path equals the
    stored Blob → `CheckoutDone`; ref at `closure_oid` →
    `CheckpointWritten`; adoptable descendant (a synchronization merge over
    the checkpoint with the same closure) → `CheckpointWritten`; a descendant
    with different closure fields, or any other state → `Unrecognized`;
  - `checkpoint_authority_reads_commit_not_config` (change `user.email` after
    the checkpoint; authority unchanged);
  - `post_checkpoint_merges_advanced_base_in_memory` and
    `post_checkpoint_conflict_reports_conflict`;
  - `contained_open_closes_with_in_memory_merge`;
  - `existing_closure_writes_no_checkpoint`;
  - `confirmed_identity_persisted_before_checkpoint`, including the Q3-1
    include case, as new functions calling the shared promotion fixtures
    (originals unedited).
  Record red.
- [ ] **Step 2: Implement** the design's "Before a checkpoint exists" steps
  1–8 and "After a checkpoint exists" steps 1–6 as the `pre_integration` hook
  and its helpers, with the three failure points at the design's positions and
  refresh-required marked when the context ref moves.
- [ ] **Step 3: Gate.** `--lib closure`, `--lib promotion`,
  `--test document_promotion`, `--test remote_reservation`, Clippy.
- [ ] **Step 4: Review, checkpoint** `feat: write the ticket closure checkpoint
  and integrate it`, ticket comment.

## Task 5: Reservation `Close` Arms, Retirement And Completion

**Files:** Modify `remote/reservation.rs`, `cleanup.rs`,
`promotion/record.rs` (`ChildExecution::validate_request` by parent kind),
`closure/record.rs` (completion). New cases in `remote/reservation_tests.rs`
and `cleanup_tests.rs`.

**Interfaces:**
- Consumes: `LifecycleKind`, `ClosureRecord::merged_context`.
- Produces: `RetirementEvidence::matches(&self, kind: AuthoringKind,
  operation: OperationId, item: &ItemId, oid: Oid) -> bool` (replaces
  `matches_promotion`; promotion passes `Document`); `finish_parent` `Close`
  arm joining `ticket_closures` with `retired_contexts` on `kind='ticket'` and
  `retired_oid` = merged context; `restart_remote_lifecycle(kind, ..)`
  (`restart_remote_promotion` stays as a wrapper); `Close` arms in
  `acknowledge` and cancel; `ChildExecution` accepting a ticket context target
  only for a closure parent.

- [ ] **Step 1: Failing tests:** `close_reservation_holds_slot_like_promote`,
  `close_restart_fences_owner_epoch`,
  `close_cancel_flags_parent_and_reservation`,
  `finish_close_requires_complete_closure_and_ticket_retirement_at_merged_context`,
  `finish_close_refuses_document_retirement_row`,
  `retirement_evidence_checks_kind`,
  `child_fence_accepts_ticket_target_only_for_closure_parent`,
  `child_fence_refuses_document_target_for_closure_parent`. Record red.
- [ ] **Step 2: Implement.**
- [ ] **Step 3: Gate.** `--lib remote::reservation`, `--lib cleanup`,
  `--lib promotion`, `--test remote_reservation`, `--test document_promotion`,
  `--test remote_polling`, Clippy.
- [ ] **Step 4: Review, checkpoint** `feat: add ticket closure reservation and
  completion`, ticket comment.

## Task 6: Orchestrate, Retry And Refresh

**Files:** Modify `closure/mod.rs` (`close_ticket`, `cancel_ticket_closure`,
`TicketClosureLifecycle`). Create `closure/run_tests.rs`.

**Interfaces:**
- Consumes: everything above.
- Produces: public `close_ticket` and `cancel_ticket_closure` with the design's
  signatures; `TicketClosureLifecycle: Lifecycle`.

- [ ] **Step 1: Failing tests** in `closure/run_tests.rs` (remote-less and
  bare-remote fixtures as `promotion/run_tests.rs` uses; call its helpers,
  do not copy them):
  - happy path without a remote: `closure: Recorded`, `integration: Merged`,
    `publication: Pending(NoPublicationRemote)`, `cleanup: Complete`; primary
    ticket closed with the original `status`;
  - completed ID replays the stored outcome with no Git effect;
    `OperationMismatch` for a different item or digest;
  - each pre-checkpoint refusal ends `abandoned` and replays; a new ID works;
  - **checkpoint seams**: exit at `ClosureAfterClosingRecorded`,
    `ClosureDuringCheckpointCheckout` and `AfterClosureCheckpoint`, reopen,
    same-ID retry: one `Close ticket` commit reachable, one merge, one
    `closed_at`; a new ID meanwhile is `Busy`;
  - after `ClosureAfterClosingRecorded` with collaborator content arriving
    (no remote: change the context by hand), retry abandons with nothing
    written;
  - after `AfterClosureCheckpoint` with a changed `user.email`, retry
    finishes with the checkpoint's `closed_by` and signature;
  - after `AfterClosureCheckpoint` with an advanced primary, retry merges in
    memory; with a conflicting advance, `RecoveryRequired`, record unchanged,
    then a deliberate context synchronization and same-ID retry finish;
  - exits at F1–F12 and the F10/F11 seams for the closure kind: no duplicate
    checkpoint, merge, deletion or removal (counters as promotion's tests);
  - cancellation before integration → `Interrupted`; after → `pending(cancelled)`;
    completed child authority consumed before cancellation;
  - every `Ok` after integration refreshes or returns `IndexPending`;
  - after completion the read API returns the ticket from the primary only:
    `ClosureDto { state: Closed, closed_at, closed_by }`, `status` unchanged.
  Record red.
- [ ] **Step 2: Implement** `TicketClosureLifecycle` and the public operations
  over the generic runner.
- [ ] **Step 3: Gate.** `--lib closure`, `--lib promotion`,
  `--test document_promotion`, `--test remote_reservation`, Clippy.
- [ ] **Step 4: Review, checkpoint** `feat: add confirmed ticket closure`,
  ticket comment.

## Task 7: Closed-Ticket Domain Guard (C5)

**Files:** Create `src/repository/ticket_guard.rs`, `ticket_guard_tests.rs`.
Modify `src/repository.rs` (`RepositoryErrorKind::TicketClosed`; calls in
`save_ticket_with_effective_config`, `prepare_context`,
`checkpoint_comment_locally`), `mutation/outcome.rs` and `read/mod.rs`
(mapping), `mutation/bind/ticket.rs` (comment on `refuse_closed`), and the
named test edits 1 and 2.

**Interfaces:**
- Produces: `RepositoryErrorKind::TicketClosed`;
  `pub(in super) fn require_ticket_open(repo: &git2::Repository, primary_branch: &str,
  item: &ItemId) -> Result<(), RepositoryError>` reading the local primary
  branch tip and, when present, the context branch tip; never the filesystem.

- [ ] **Step 1: Failing tests** in `ticket_guard_tests.rs`:
  `save_edit_of_closed_ticket_is_refused_before_provisioning` (no branch,
  worktree or file created; the new journal row ends rejected);
  `save_refused_when_context_copy_closed`;
  `relationship_and_slug_saves_refused` (through `save_ticket_with`);
  `prepare_context_edit_of_closed_ticket_refused`;
  `new_comment_on_closed_ticket_refused`; `create_unaffected`;
  `repair_of_closed_ticket_still_allowed`;
  `resumed_row_not_refused`;
  `writes_to_other_tickets_that_depend_on_closed_ticket_allowed`;
  `dirty_draft_cannot_hide_closure` (closure on the branch tip, the worktree
  file edited to remove it);
  `refused_error_maps_to_ticket_closed_result_code`.
  Apply named edit 1 (refusal, closure fields and body unchanged, nothing
  provisioned) and named edit 2. Record red.
- [ ] **Step 2: Implement** the guard after `begin_lifecycle`, new rows only.
- [ ] **Step 3: Gate.** `--lib ticket_guard`, `--lib mutation::outcome`,
  `--lib read`, `--test local_authoring`, `--test mutation_relationships`,
  `--test mutation_contract`, `--test mutation_replay`,
  `--test comment_publication`, `--test read_items`, `--test mutation_bridges`,
  Clippy. Any other existing test that now fails stops for review.
- [ ] **Step 4: Review, checkpoint** `feat: refuse writes to closed tickets`,
  ticket comment.

## Task 8: Prove Closure Over Real SSH

**Files:** Create `tests/ticket_closure.rs` and `tests/ticket_closure/{journeys,
checkpoint, exits, races, privacy}.rs`, mirroring `tests/document_promotion.rs`
(own `main`, `ssh_harness::initialize`, `maybe_exit_child`,
`run_with_short_output_privacy`, `#[path]` support includes). Modify
`Cargo.toml` (`[[test]] name = "ticket_closure"`, `harness = false`),
`tests/support/` (named edit 4), `.github/workflows/build.yml` (append
`--test ticket_closure`).

- [ ] **Step 1: Write the cases:**
  - journeys over two clones: happy path (B polls, sees the closed primary,
    reports nothing for the context; the remote context branch never held the
    checkpoint); local-only then later remote plus primary synchronization
    publishes; existing closure fields; contained-and-closed cleanup-only,
    including after cache loss with the remote branch gone; contained-and-open
    closes without a context child; context conflict, resolve, same-ID retry
    closes once; poll advancing `T(P)` mid-closure; primary publication
    rejected then retry publishes and cleans up only; publication success then
    remote deletion failure then retry deletes and cleans up only; changed
    remote context; offline before and after integration with recovery under
    the identical request; dirty, untracked, ignored and moved context after
    integration; every blocker with full preservation snapshots; poll priority;
    pending comment publication blocks, then finishes, then closure succeeds;
    the C5 guard does not refuse closure's own checkpoint, including a
    same-ID retry from `closing`, and refuses a new save afterwards;
  - checkpoint: the three checkpoint seams by child-process exit, with an
    advanced and a conflicting primary before the retry;
  - races: sequential second closure in clone B (B's context child merges A's
    closure into B's context; B's leased re-check stops before writing; next
    preview `AlreadyClosed`); true race with **distinct identities** in each
    clone, both held by a barrier after their context children: both closures
    in history, the later clone publication pending conflict, external-recovery
    guidance, and its later promotion or closure blocked
    `PrimaryAheadOfRemote`;
  - exits: F1–F12 and both F10/F11 seams, reopen, identical retry, effect
    counters; the T6-1 lock case with simulated operator repair in the fixture
    only;
  - privacy: `closed_by` only in canonical Markdown, Git objects and
    `discovered_items.closed_by` (live, WAL, backup; positive controls), absent
    from every workflow row (negative controls updating an existing
    `operation_records.target` transactionally, then rollback); no endpoint,
    body, credential or key material;
  - discoverability: the closed ticket lists under closure filter `closed`, not
    `open`, with `status` unchanged, from both clones after a poll.
  Keep each case under the 100-second watchdog.
- [ ] **Step 2: Run** `devenv shell -- cargo test --locked --test ticket_closure`
  in the background; fix code, never weaken a case. Rerun affected targets:
  `document_promotion`, `remote_synchronization`, `remote_merge_recovery`,
  `remote_polling`, `comment_publication`, `remote_reservation`,
  `ssh_transport`, `mutation_remote`, `ssh_fixture`, `remote_observation`.
  Clippy.
- [ ] **Step 3: Review, checkpoint** `test: prove ticket closure over SSH`,
  ticket comment with counts and the acceptance mapping.

## Task 9: Full Verification, Documents And Handoff

- [ ] `git fetch origin`; if main moved, merge it in (no rebase) and rerun
  affected targets.
- [ ] Run in the background with at least a 3-hour allowance, and record
  counts and wall time:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

- [ ] Desktop smoke only if a display is available; the change is domain-only.
- [ ] Map every Cycle acceptance row to named passing tests in the ledger.
- [ ] Update `docs/research/wave-03-api-audit.md`: `ticket close` delivered as
  `preview_ticket_closure`/`close_ticket`/`cancel_ticket_closure`; the closed
  write guard is in the domain; F2 binding and the ready-ticket report remain
  C5/D5.
- [ ] Add a correction note to the Cycle 09 design and handoff where they say
  Cycle 10 reuses cleanup "unchanged", pointing at Tasks 1 and 5.
- [ ] `git merge-tree --write-tree HEAD origin/main` succeeds; record the tree.
- [ ] Independent whole-branch spec and quality reviews over
  `origin/main..HEAD` plus any uncommitted change.
- [ ] Ticket comment: review-ready, with counts and hashes. Push, PR, merge,
  closure and cleanup wait for separate authorization. Native CI is **not**
  dispatched before merge: after merge, ask the owner to dispatch the
  four-target workflow on main; a failure there is follow-on work in a
  separate session.

## Plan Self-Review

- **Spec coverage.** Contract item → task: preflight and digest (3);
  confirmation and records (2, 6); context child, D4 (6, 8); checkpoint and
  integration (4); post-checkpoint path (4, 6, 8); publication and cleanup
  (5, 6, 8); retry (6, 8); refresh (4, 6); guard (7); bounds and privacy (2,
  8); concurrency (8); seam preservation (1); verification (9).
- **Placeholders.** None. Signatures that wrap existing functions say so
  instead of inventing argument lists; Task 1 Step 1 writes the exact list.
- **Type consistency.** `Lifecycle`, `LifecycleKind`, `ClosureRecord`,
  `ClosureStep`, `ClosureKind`, `ClosureBlocker` (21), `ClosureExpectation`,
  `PlannedClosure`, `ClosureRecording`, `PreparedClosure`, `ClosingState`,
  `CheckpointAuthority`, the three `FailurePoint`s and
  `RepositoryErrorKind::TicketClosed` match the design.
- **Review Focus.** Five items, each with a named test in its owning task.
- **Known risk.** Task 1 changes the code that took Cycle 09 the most review
  rounds; it has its own gate and review before any closure code.
