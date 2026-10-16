---
title: "Wave 02 Cycle 09 Confirmed Managed-Document Promotion Implementation Plan"
date: 2026-10-15
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M500Q0YHT5PNGGCQPCBSF3Z5"
---

# Confirmed Managed-Document Promotion Implementation Plan

> Approved by the owner on 2026-10-15. Implements the
> [Cycle](../Cycles/wave-02-cycle-09-confirmed-document-promotion.md) and
> [design](2026-10-15-wave-02-cycle-09-confirmed-document-promotion-design.md).
> The owner accepted the recommended answer to D1–D7 and ruling R1; the
> passages that depend on them are marked **(D1)–(D7)**. Do not implement until
> implementation is explicitly authorized. Implementation and checkpoint
> commits have since been authorized. Execution mode: subagent-driven
> development, with independent spec then quality review at each checkpoint.
> The owner approved T0-1's ownership-first child retry amendment and targeted
> tests only until Task 8; these override the original execution/baseline text.
> On 2026-10-16 the owner approved T2-1's resulting Git mode/type digest encoding
> amendment. Task 2 below implements it; its historical quality blocker is
> preserved in the execution ledger.

**Goal:** Two headless operations:

- `preview_document_promotion`: an exact, local, read-only preflight with an
  expectation digest.
- `promote_document`: a resumable, confirmed lifecycle.
  - It synchronizes the context and merges it into a clean primary with a
    `Promote document <ULID>` commit.
  - It publishes the primary, then deletes the remote context branch, the
    worktree and the local branch.
  - A retry repeats only unfinished publication or cleanup.

**Ticket:** `01K7F6H9J2N4Q6S8V0X2Z4B6DH`

**Architecture:**

- New module `src/repository/promotion/`. It orchestrates:
  - the existing synchronization, as bound children;
  - the in-memory merge preparation and the guarded fast-forward;
  - a new guarded remote delete;
  - new generic local cleanup helpers.
- New state is two tables, `document_promotions` and `retired_contexts`
  (ruling R1).
- No change to the synchronization, poll or merge behaviour, except one: a
  retired item without a state row is eligible for first publication even when
  repository history is unknown.

**Stack:** Rust, `git2` 0.20 (locked libgit2/libssh2), `rusqlite`, `blake3`,
the in-process `russh` SSH fixture. No new dependency is expected; adding one
stops for review.

## Global Constraints

- Every local and agent Rust command uses `devenv shell -- cargo ...`.
- All work happens in the ticket worktree on
  `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH`.
- Shared domain code stays free of GPUI. Make no CLI, desktop or F2 boundary
  change. `bind/resume.rs` keeps `NotResumable`.
- No network call, prompt or discovery scan holds the repository lease. Every
  local change re-observes under the lease first.
- A promotion never force-pushes, rebases, stashes or discards. Cleanup removes
  only state verified clean at the recorded OID.
- Records hold IDs, ref names, OIDs, timestamps, fixed step names and fixed
  categories only. No `Debug` output of request data, paths or session
  material.
- Extract shared helpers; do not rewrite them. Existing synchronization, merge,
  comment-publication and polling tests stay green unchanged unless a step
  names the change.
- Tests use counters, holds and barriers, never sleeps. They never touch
  developer keys, global Git configuration or real application data.
- Write the failing test first, and record the red run before writing
  production code.
- Commit, push, pull request, CI dispatch, merge and closure each need their
  own authorization.

## File Map And Task Order

| Files | Ownership |
| --- | --- |
| `src/repository/promotion/mod.rs` (new) | Public types, `preview_document_promotion`, `promote_document`, `cancel_document_promotion` |
| `src/repository/promotion/observe.rs`, `observe_tests.rs` (new) | Observation, blockers, change set, merge-tree selection, digest |
| `src/repository/promotion/integrate.rs`, `integrate_tests.rs` (new) | Merge commit, guarded primary update, integration reconciliation |
| `src/repository/promotion/record.rs`, `record_tests.rs` (new) | `document_promotions` schema, migration, steps, reconciliation table |
| `src/repository/promotion/run.rs`, `run_tests.rs` (new) | Lifecycle orchestration, children, cancellation, refresh |
| `src/repository/cleanup.rs`, `cleanup_tests.rs` (new) | Generic pre-check (clean, ignored, locked), worktree removal, branch deletion, completion transaction pieces (reused by Cycle 10) |
| `src/repository/remote/sync.rs`, `refs.rs` | Visibility of `prepare_clean_merge`, `import_prepared_tree`, `committing_identity`, `fast_forward`, `materialized_target`, `require_clean_target`, `canonical_tree_sources`; retired-item first-publication rule in the eligibility check |
| `src/repository/remote/state.rs`, `reservation.rs` | `retired_contexts` table and migration; `publication_evidence_for` retired rule; batch deletes a retirement on `observed_published`; history-row delete helper; `promote` reservation finish/restart rules; read text at `:2898`/`:2937` |
| `src/repository/transport/remote.rs`, `callbacks.rs` | `SshDirection::Delete` mapping, `delete_exact`, `push_negotiation` guard |
| `src/repository/recovery.rs`, `src/repository.rs`, `src/repository/discovery.rs` | ID cross-check, `FailurePoint` variants, migration call, table inventory, re-exports |
| `src/repository/read/status.rs` | Promotion rows in recovery status text only (no DTO field) |
| `tests/support/ssh_server.rs`, `ssh_remote.rs` | Deletion receipt proof, `reject_context_deletion`, `race_context_update` |
| `tests/document_promotion.rs` (new), `Cargo.toml` | Real-SSH acceptance target, `harness = false` |
| `tests/repository_enablement.rs` | Table inventory |
| `.github/workflows/build.yml` | Append `--test document_promotion` |
| `docs/research/wave-03-api-audit.md` | Replace the `document promote` "missing" entries with the delivered API |

Task order:

- Task 0 gates everything.
- Task 1 (transport and fixture delete) and Task 2 (observation and preview)
  are independent.
- Task 3 (integration) depends on Task 2.
- Task 4 (local cleanup and retirement) and Task 5 (record) depend only on
  Task 0.
- Task 6 composes Tasks 1–5.
- Task 7 proves the whole over real SSH.
- Task 8 is final verification and handoff.

## Task 0: Refresh Base, Record Decisions, Characterize And Baseline

Evidence collected, including the owner-approved T0-1 amendment. Independent
spec review passed; quality review approved after the three documented
consistency fixes. Task 0 is complete for the local implementation gate.

- [x] Confirm explicit implementation authorization and the recorded answers
  to D1–D7.
- [x] `git fetch origin main`. Rebase the ticket branch onto it from inside
  the ticket worktree. Verify ancestry and a clean status. Record the base and
  the before and after SHAs. Re-read `AGENTS.md` if it changed.
- [x] Re-scan for new callers of `synchronize_remote_observed`,
  `prepare_context`, `RemoteOperationAction::Promote` and
  `remote_context_states`. Record any Wave 03 consumer as a compatibility
  item.
- [x] Characterize with throwaway, uncommitted tests, and record each finding
  in the ledger:
  - Over the fixture, a `:refs/heads/<x>` push with a `push_negotiation`
    callback:
    - the callback sees the remote's current OID;
    - returning an error sends nothing;
    - an accepted delete is reported by `push_update_reference`.
  - What `Worktree::prune` with `valid` and `working_tree` set and `locked`
    unset does:
    - on a clean worktree;
    - on a worktree holding ignored files;
    - on a locked worktree;
    - when a file in it is open (Windows-relevant; record as a native gap);
    - when interrupted after partial deletion.
  - That the tree of `prepare_clean_merge(T(P), C)` equals the tree
    synchronization's `Merge primary into document` produces for the same
    inputs. This is the basis of the D4 and D6 claim that a primary-only
    advance leaves the change set unchanged.
  - For each child synchronization end state in the design's attempt table
    (completed, conflict pending, identity required, cancelled, interrupted,
    push rejected, transport failure, busy, yielding, recovery required with
    and without a retained merge), what a replay and a `restart` under the
    same ID return. Confirm the same-or-new-attempt column before Task 6.
  - What `prepare_context(Edit)` does after the branch and worktree are gone
    and the primary contains the document: it creates from the primary head.
- [x] A finding that contradicts the design stops here for an amendment and
  owner review. Do not adapt silently.
- [x] Run and record targeted characterization-relevant baseline tests only.
  The owner's targeted-testing override defers the full check/fmt/clippy/test
  gate and CLI smoke run to Task 8.

- [x] Add a ticket comment with the base, decisions, characterization results
  and baseline.

## Task 1: Guarded Remote Context Deletion

**Files:** `src/repository/transport/remote.rs`, `callbacks.rs`,
`operation_tests.rs`, `tests/support/ssh_server.rs`, `ssh_remote.rs`.

**Consumes:** `AuthenticatedSshRemote`, `exact_mappings`,
`mappings_match_scope`, `push_update_reference` handling, the fixture receive
audit.
**Produces:**
- an `SshDirection::Delete` mapping, admitted by `mappings_match_scope` only
  for a context target;
- `delete_exact(plan, target, expected) -> Result<DeleteObservation,
  SshTransportError>`, where `DeleteObservation` is `Deleted`,
  `StillPresent(oid)` or `Changed(oid)`;
- fixture deletion proof and fault hooks.

- [ ] Failing fixture tests:
  - `receive_effect_proven` accepts a deletion only when the ref is absent
    afterwards and the old OID matched;
  - `reject_context_deletion` makes the server refuse;
  - `race_context_update` advances the context between advertisement and
    receive.
- [ ] Failing transport tests:
  - delete of a context at the expected OID → `Deleted`, and the server saw
    exactly one delete command;
  - expected OID mismatch → `Changed`, and the server received **no** command
    (negotiation refused);
  - a server rejection → `StillPresent`;
  - a delete mapping for the primary or a non-context ref is refused by
    `mappings_match_scope` before connecting;
  - a disconnect after send → re-advertisement decides;
  - nothing in errors or `Debug` contains the endpoint.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib transport
  ```

- [ ] Implement the mapping, the negotiation callback and the
  post-push advertisement check.
- [ ] Run green. Run the existing `remote_synchronization` target to confirm
  no regression in pushes. Commit (for example
  `feat: add guarded remote context deletion`) and add a ticket comment.

## Task 2: Promotion Observation, Change Set And Preview

**Completion summary — 2026-10-16:** complete at the local gate. The same quality
reviewer approved T2-1 resolved with no critical/important/minor findings; the
same spec reviewer rechecked the amended code/docs and passed, with context-path
and executable-document fixes retained. The recorded targeted selector passes
all 53 tests; no tests were rerun for this status update. Historical task
checklists/evidence remain below and in the execution ledger. The controller's
checkpoint commit is pending; Task 3 is next. Full Task 8/native gates remain
deferred.

**Files:** new `src/repository/promotion/{mod.rs, observe.rs,
observe_tests.rs}`, `src/repository/remote/sync.rs` (visibility),
`src/repository.rs` (module, re-exports).

**Consumes:**
- `observe_active_contexts` rule, `materialized_target`,
  `require_clean_target`;
- `RemoteSnapshot::publication_evidence_for`, `prepare_clean_merge`,
  `canonical_tree_sources`;
- the pending-conflict query.

**Produces:**
- `PromotionObservation`, `PromotionBlocker`, `PromotionPathChange`,
  `PromotionExpectation`, `DocumentPromotionPreview`,
  `DocumentPromotionPreflight`;
- `preview_document_promotion`.

- [ ] Failing unit tests, one per blocker, with every other condition
  satisfied:
  - not a document;
  - context missing, nonconforming, dirty (tracked, staged, untracked) or
    conflicted;
  - pending conflict;
  - primary missing, not checked out, dirty or conflicted;
  - **(D2)** primary ahead and primary diverged, with a remote;
  - `ContextBehindRemote`: a fetched `T(C)` ahead of or diverged from the
    local context;
  - **(D3)** integration conflict without a remote;
  - **(D5)** remote context deleted, and history unknown, each only when
    `T(C)` is absent. Two negative tests: `HistoryUnknown` with `T(C)`
    present does not block, and an `AlreadyContained` context with `T(C)`
    absent previews as cleanup only, with no remote branch in the cleanup
    set;
  - **(D1)** document invalid in the result tree, and a result tree with a
    `.manyhands/worktrees/` entry.

  Also: all blockers are reported together in fixed order.
- [ ] Failing tests for merge-tree selection:
  - context contained → `AlreadyContained`;
  - base ancestor of context → the context tree;
  - divergent without a remote → the prepared tree;
  - divergent with a stale `T(P)` → prepared, not blocked.
- [ ] Failing tests for the change set:
  - added, modified and deleted paths;
  - comments and a non-canonical file are listed;
  - a document moved within the context shows a delete and an add;
  - sorted output.
- [ ] Failing digest tests:
  - stable across two previews;
  - changes for each bound field (item, path, branches, remote, integration
    kind, each change entry including resulting Git mode/type, cleanup set);
  - **(T2-1, owner-approved)** retain all four regressions: remote-backed regular
    `100644` → symlink `120000` and regular → executable `100755`, each with
    unchanged Added and Modified path categories against the same primary and
    identical new blob OIDs; each must change the digest;
  - encode each Added/Modified result-tree mode (including type bits) as u32
    big endian immediately after its optional new OID. Deleted encodes absent
    OID plus zero u32, never its old mode. Verify the fixed-width byte encoding
    and explicit deletion sentinel with a protocol vector. Keep evidence internal
    and the public path-change DTO unchanged;
  - retain domain `manyhands promotion v1`, finalizing the unshipped encoding;
    there is no shipped earlier digest requiring migration;
  - **(D6)** unchanged by a new context OID that only merges the primary,
    and by a `T(P)` advance that touches other paths, with a remote;
  - changed by a new local primary OID without a remote;
  - `Debug` is redacted.
- [ ] Failing side-effect tests: the preview takes no lease (succeeds while
  the lease is held), makes zero connections, and writes no registry row and
  no ref.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib promotion::observe
  ```

- [ ] Implement. Widen the sync helpers to `pub(in crate::repository)` with
  no behaviour change.
- [ ] Run green. Commit (for example
  `feat: add document promotion preflight and preview`) and add a ticket
  comment.

## Task 3: Primary Integration

**Files:** new `src/repository/promotion/{integrate.rs, integrate_tests.rs}`.

**Consumes:** Task 2 observation, `import_prepared_tree`,
`committing_identity`, `fast_forward`, the repository lease.
**Produces:** `integrate(service, root, record, identity) ->
Result<Integrated, DocumentPromotionError>`, which recomputes the observation
and digest under the lease, and `reconcile_integration(repository, base,
context) -> IntegrationState`, which searches for a reachable merge.

- [ ] Failing tests:
  - **Integration base rule (review finding 1).** With a remote, the context
    is synchronized and then `T(P)` advances with a commit touching another
    path, as a poll would make it. Integration returns `PrimaryAdvanced`,
    writes no object or ref, and leaves the primary, context and worktrees
    byte-for-byte unchanged. No merge is ever produced whose tree omits the
    advanced commit.
  - A digest that differs under the lease → `ReconfirmationRequired`,
    nothing written.
  - With a remote-shaped base (base ancestor of context): a commit with
    subject `Promote document <ULID>`, parents `[base, context]`, the
    context's tree, and the primary moved from a local primary behind the
    base. Worktree bytes, index and status are clean afterwards.
  - Without a remote, divergent: the prepared tree, and the document valid on
    the primary afterwards.
  - Contained: no commit, and the primary unchanged.
  - Identity missing → `IdentityRequired` with nothing written. A confirmed
    identity is used.
  - The primary changed between the observation and the lease →
    `ExternalChange`, nothing moved.
  - An ignored-file collision in the safe checkout → refused, nothing moved.
  - Reconciliation, by reachability:
    - the matching merge is at the primary tip → integrated;
    - the matching merge is reachable after a later primary fast-forward or
      synchronization merge → integrated;
    - no matching merge, and the local primary is behind or at the base,
      including after a poll fast-forward → not integrated;
    - anything else → `RecoveryRequired`, reported as a pending category and
      never as an error.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib promotion::integrate
  ```

- [ ] Implement. Make one ref update from the old local primary to the
  merge.
- [ ] Run green. Commit (for example `feat: integrate a document context into
  primary`) and add a ticket comment.

## Task 4: Generic Context Cleanup And Retirement

**Files:** new `src/repository/cleanup.rs`, `cleanup_tests.rs`;
`src/repository/remote/state.rs`, `state_tests.rs`, `refs.rs`,
`refs_tests.rs`, `sync.rs` (first-publication eligibility only).

**Consumes:** `validate_context_worktree`, `materialized_target`,
`require_clean_target`, the lease, `remote_context_states`,
`publication_evidence_for`, `plan_clean_integration`, the observation batch.
**Produces:**
- `precheck_context(kind, id, oid) -> Precheck`, where `Precheck` is `Ok`,
  `Changed`, `IgnoredFiles` or `Locked`;
- `remove_context_worktree(kind, id, oid) -> WorktreeRemoval`, where
  `WorktreeRemoval` is `Removed`, `AlreadyAbsent`, `Changed` or
  `RecoveryRequired`;
- `delete_context_branch(kind, id, oid) -> BranchDeletion`;
- `retire_context(tx, kind, id, expected_tracking, operation_id, oid)`, which
  in one transaction deletes the tracking ref, deletes the state row and
  inserts `retired_contexts`;
- the `retired_contexts` table (R1) and its migration;
- the retired-item rule in `publication_evidence_for` and first-publication
  eligibility.

All take a kind and an item ID.

- [ ] Failing tests for the pre-check: clean at the OID → `Ok`. Each of the
  following → `Changed`, with a full local snapshot unchanged:
  - dirty tracked;
  - untracked;
  - staged;
  - moved to a new commit;
  - on another branch;
  - registration missing.

  **(D7)** An ignored file present → `IgnoredFiles`, and the file is kept. A
  locked worktree → `Locked`.
- [ ] Failing tests for retirement:
  - the tracking ref at the expected value, or absent → deleted, state row
    gone, retirement written in one transaction;
  - the tracking ref elsewhere → `Changed`, nothing touched;
  - an injected failure inside the transaction leaves all three unchanged.
- [ ] Failing tests for R1 evidence:
  - in a repository with `history_unknown` set, a retired item with no state
    row reads `NeverPublished`, and `plan_clean_integration` permits first
    publication;
  - a non-retired item in the same repository still reads `HistoryUnknown`;
  - an observation batch that sees the item published removes the
    retirement;
  - a batch without the item does not mark it deleted;
  - the snapshot validation invariant at `state.rs:5429` still holds;
  - the existing synchronization and poll tests are unchanged.
- [ ] Failing tests for worktree removal: directory and admin entry gone and
  other worktrees untouched; already absent → `AlreadyAbsent`; a partly
  removed directory with the registration present → `RecoveryRequired` and
  nothing further removed.
- [ ] Failing tests for branch deletion: at the OID → deleted; at another OID
  → `Changed`; checked out somewhere → `Changed`; absent → `AlreadyAbsent`.
- [ ] Failing test for a fresh context: after worktree removal, branch
  deletion and retirement, `prepare_context(Edit)` for the item creates a new
  context from the primary head.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib cleanup
  devenv shell -- cargo test --locked --lib remote::state
  devenv shell -- cargo test --locked --lib remote::refs
  ```

- [ ] Implement.
- [ ] Run green. Commit (for example `feat: add verified context cleanup
  helpers`) and add a ticket comment.

## Task 5: Promotion Record And Reservation Rules

**Files:** new `src/repository/promotion/{record.rs, record_tests.rs}`;
`src/repository/remote/reservation.rs`, `reservation_tests.rs`, `state.rs`;
`src/repository/recovery.rs`; `src/repository/discovery.rs` (migration call,
inventory); `src/repository.rs` (`FailurePoint` variants);
`src/repository/read/status.rs`; `tests/repository_enablement.rs`.

**Consumes:** `with_transaction`, the migration marker pattern, the
reservation API.
**Produces:**
- the `document_promotions` table and marker `cycle_09_document_promotions`;
- typed `PromotionStep` and step transitions, including the terminal
  `abandoned` and `superseded`;
- per-attempt child IDs (`context_attempt`/`context_sync_id`,
  `primary_attempt`/`primary_sync_id`);
- the one-unfinished-per-item rule, as a partial unique index, with the
  `superseded` exception;
- `promote` reservation finish and restart;
- ID cross-checks;
- the twelve `FailurePoint` variants F1–F12, named exactly as in the design's
  lifecycle.

- [ ] Failing migration tests:
  - a registry without the table migrates and keeps every row;
  - a second migration is a no-op;
  - a half-created table is `RecoveryRequired`;
  - the service-start audit passes;
  - the inventory test lists the table.
- [ ] Failing record tests:
  - forward-only step transitions, where an invalid transition is
    `RecoveryRequired`;
  - a new attempt stores a new child ID before the child runs, and an
    attempt's ID is never reused for another attempt;
  - the same ID with a different item or digest → `OperationMismatch`;
  - an `abandoned` row replays its stored refusal, and does not block a new ID
    for the item;
  - a second unfinished row for the item → `Busy`;
  - a row stopped on `context_changed`, `ignored_files` or
    `remote_context_changed` is marked `superseded` when a new row is
    inserted, and any other unfinished row is not;
  - column `CHECK`s on step and category values.
- [ ] Failing reservation tests:
  - a `promote` reservation with a context ref can stay active across local
    cleanup steps, and completes only in the completion transaction;
  - an automatic poll that starts while it is active is `Busy`;
  - a running poll it meets is asked to yield;
  - it is `Busy` against a running synchronization;
  - restart of an interrupted `promote` row fences and returns a new owner.

  Existing synchronization and poll rules stay frozen.
- [ ] Failing cross-check tests:
  - a local journal begin or a synchronization with a promotion's ID is
    refused;
  - a promotion with a local or foreign remote record's ID is
    `OperationMismatch`;
  - the promotion's own `promote` reservation row is accepted.
- [ ] Privacy: extend the column scan to both new tables.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib promotion::record
  devenv shell -- cargo test --locked --lib remote::reservation
  devenv shell -- cargo test --locked --lib recovery
  ```

- [ ] Implement. Update the read-status text so a promotion row is described
  by its step. Add no DTO field.
- [ ] Run green. Commit (for example `feat: persist document promotion
  records`) and add a ticket comment.

## Task 6: Orchestrate, Retry And Refresh

**Files:** new `src/repository/promotion/{run.rs, run_tests.rs}`,
`src/repository/promotion/mod.rs`, `src/repository.rs` (re-exports),
`docs/research/wave-03-api-audit.md`.

**Consumes:** Tasks 1–5, `synchronize_remote_observed`,
`resolve_synchronization`, `refresh_repository`.
**Produces:** `promote_document`, `cancel_document_promotion`,
`DocumentPromotionResult`, `DocumentPromotionOutcome` and
`DocumentPromotionError`, in the design's lifecycle order.

- [ ] Failing local-repository tests (no remote):
  - full lifecycle →
    `Merged`, `Pending(NoPublicationRemote)`, `Complete`;
  - contained → `AlreadyContained`;
  - **(D3)** a conflict surfacing only at execution →
    `ReconfirmationRequired`, nothing changed, row `abandoned`;
  - the same ID replays the stored outcome with no work;
  - a digest mismatch → `ExternalChange`, row `abandoned`; a new preview and
    ID then succeed;
  - a blocker → `Blocked`;
  - after cache loss, a contained context previews as cleanup only and a new
    ID completes it without a second merge.
- [ ] Failing tests with an injected child double (a test seam on the
  synchronization call), covering:
  - **Context child:**
    - a context `ConflictPending` returns inside `ContextSynchronization`,
      and a retry after resolution continues;
    - **(D4)** a context child that brings a new remote path →
      `ReconfirmationRequired`, nothing integrated;
    - a child that only merged the primary → continues;
    - every row of the design's ownership-first attempt table, for both
      context and primary children: authoritative completion uses the stored
      result and never reruns; `Interrupted` with retained conflict or owned
      resolution preserves the original ID with `restart=true`; retained
      `RecoveryRequired` uses same-ID restart. `ConflictPending` and
      `IdentityRequired` continue the same attempt after resolution/identity.
      Include a `ConflictPending` child whose inspection requires external
      resolution: exercise the existing inspect/read/resolve flow reporting
      `ExternalResolutionRequired`, then a valid external repair and original-ID
      restart. Do not inject `ExternalResolutionRequired` as a synchronize
      return. Only confirmed absence of retained
      ownership permits `Cancelled`, `Interrupted`, `PushRejected`, transport
      failure, `Busy`, `PollYielding` or `RecoveryRequired` to start a new
      attempt. Uncertain ownership surfaces recovery, never assumed absence.
      No end state strands the record.
  - **Primary advanced:** `T(P)` moves after the context child completes →
    `PrimaryAdvanced`; the next call re-synchronizes with a new attempt and
    integrates once.
  - **Primary child:**
    - `PushRejected` → `Ok` with `publication: Pending`,
      `cleanup: Pending(AwaitingPublication)`, and local context state kept;
    - a retry then publishes and cleans up;
    - persistent `RecoveryRequired` with retained primary-child ownership, or
      uncertain ownership → `Ok` partial outcome with
      `publication: Pending(Synchronization(recovery_required))` and
      `cleanup: Pending(AwaitingPublication)`, never a post-integration `Err`.
      Assert the fixed redacted category, retained original child ID, no
      replacement attempt on uncertainty, and refresh/IndexPending behavior;
    - a cancelled primary attempt → `publication: Pending(cancelled)`; the
      retry runs a new attempt only with confirmed absence of retained
      ownership; retained interrupted conflict/resolution uses the original
      child ID with restart, just as for the context child.
  - **Remote delete:**
    - `Changed` → `cleanup: Pending(RemoteContextChanged)`, with all local
      state kept and history still `observed_published`;
    - `StillPresent` → pending, and a retry deletes;
    - a new promotion supersedes a `RemoteContextChanged` row.
  - **Cleanup pre-check:**
    - dirty, untracked or moved context after integration →
      `Pending(ContextChanged)`, and the remote is **not** deleted;
    - **(D7)** ignored files → `Pending(IgnoredFiles)`, remote not deleted,
      files kept;
    - a new promotion then supersedes.
  - **History not reset early:** with a cleanup that stops after remote
    deletion, a following item synchronization of the context returns
    `RemoteContextDeleted` and does not republish.
- [ ] Failing interruption tests: for each of F1–F12, the exit-point
  service stops there. A reopened service with the same request reconciles
  and finishes. Counters prove:
  - at most one merge commit;
  - one context push and one primary push;
  - one remote delete;
  - one worktree removal;
  - one branch deletion.
- [ ] Failing cancellation tests:
  - cancel before integration → `Interrupted`, nothing integrated;
  - cancel after integration stops between steps, and the outcome reports
    exactly what was done with a `cancelled` category;
  - a retry finishes.
- [ ] Failing index tests:
  - refresh success → `Complete`;
  - an injected refresh failure → `IndexPending` with the same outcome, and a
    later `refresh_repository` lists the document from the primary only;
  - every partial `Ok` return after integration (publication pending, cleanup
    pending) also refreshes, so discovery shows the merged primary.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib promotion
  ```

- [ ] Implement. Re-export the public types. Update the API audit's
  `document promote` and D25 rows to name the delivered operations, their
  results, and what remains for C5/D5 (boundary binding, expiry, preview
  schema values, resume classification).
- [ ] Run green with targeted promotion tests; the owner deferred the full
  library suite to Task 8:

  ```sh
  devenv shell -- cargo test --locked --lib promotion
  ```

- [ ] Commit (for example `feat: add confirmed document promotion`) and add a
  ticket comment.

## Task 7: Prove Promotion Over Real SSH

**Files:** new `tests/document_promotion.rs`, `Cargo.toml`,
`.github/workflows/build.yml`.

**Consumes:** `tests/support/{remote_world, ssh_remote, ssh_harness,
ssh_privacy}.rs`, the two-clone `Pair`/`Side` shape and `document_target()`
from `tests/remote_merge_recovery.rs`, and the public API only.
**Produces:** the Cycle's exit evidence.

- [ ] Declare `[[test]] name = "document_promotion", harness = false`. Reuse
  the pre-thread initialization, the isolated child and the output-privacy
  architecture. Keep fixture directory names short. Every case ends with the
  privacy scan.
- [ ] Happy path: clone A edits a document and adds a comment, previews, and
  promotes. Assert:
  - the merge commit's subject and parents;
  - the server's primary equals A's primary;
  - the remote context is gone;
  - A has no worktree and no branch;
  - the document and its comment are on the primary;
  - discovery lists the document once, from the primary;
  - `git log` of the second parent shows the context history.

  Clone B polls: B's primary fast-forwards, and its context is reported
  `remotely_deleted` and preserved.
- [ ] Local-only: a repository with no remote promotes, with
  `Pending(NoPublicationRemote)` and cleanup complete. Then configure the
  remote and run primary synchronization, which publishes the merge.
- [ ] Already-contained and fresh-context cases:
  - after promotion, an Edit of the document creates a new context from the
    primary;
  - a save and a synchronization publish it as a first publication;
  - a second promotion produces a second merge.
- [ ] Blockers, each with zero connections and a byte-for-byte unchanged
  snapshot of both clones and the server:
  - dirty primary, conflicted primary, primary not checked out;
  - **(D2)** primary ahead;
  - dirty context;
  - context behind its fetched remote;
  - **(D5)** remote context deleted by B.
- [ ] Cleanup preservation after integration, each with the remote context
  still on the server and a byte-for-byte unchanged local context:
  - A edits the context file between publication and cleanup;
  - an untracked file appears;
  - **(D7)** an ignored file appears (via a repository-local exclude, never
    global configuration);
  - the worktree is locked.

  Each case is then superseded by a new preview and promotion that integrates
  the new commit, if any, and completes.
- [ ] Cache loss and unknown history:
  - A promotes, the registry is deleted and rebuilt, and an Edit then creates
    a fresh context. With `history_unknown` set, the new context publishes
    for the first time because of R1.
  - A promotion interrupted at F7 is followed by registry loss. The preview
    reports `AlreadyContained` with cleanup only, and a new ID completes it
    with no second merge and no push.
- [ ] **(D4)** B pushes a new commit to the context after A's preview. A's
  promotion synchronizes, then returns `ReconfirmationRequired`. A
  re-preview lists B's path, and promoting with the new digest succeeds. A
  primary-only advance by B between preview and promotion does **not**
  require reconfirmation.
- [ ] Context divergence with a conflict: A gets `ConflictPending`, resolves
  through `resolve_synchronization`, retries, and the promotion completes with
  one merge.
- [ ] Publication failure: with `reject_primary_updates`, the outcome is
  `Pending`, the server has no merge, A keeps its context, and the remote
  context still exists. After clearing the fault, a retry publishes and cleans
  up. Repeat with the server offline.
- [ ] Remote deletion failure: `reject_context_deletion` leaves
  `Pending(RemoteDeletionFailed)` and all local state; a retry deletes and
  cleans up. With `race_context_update`, the result is
  `Pending(RemoteContextChanged)`, and B's commit is preserved on the server
  and in A's tracking ref.
- [ ] Integration base: an automatic poll in A, run between the context
  synchronization (F2) and integration, advances `T(P)` with B's commit to
  another path. A gets `PrimaryAdvanced`. The retry produces one merge whose
  tree contains B's commit, and the server's primary keeps it.
- [ ] Child attempts: a cancelled context synchronization and a cancelled
  primary publication each leave the record resumable. The next call
  completes with a new attempt when retained ownership is confirmed absent,
  and one push of each branch. An interrupted child retaining a conflict or
  owned resolution keeps its original ID with restart for either target.
- [ ] Interruption and reopen, one case per boundary F1–F12, using the
  exit-point service. Each case asserts:
  - actual refs, worktrees, server refs and the record;
  - that the retry performs only unfinished work, through server receive
    counters (pushes and deletes) and a merge-commit count.
- [ ] Priority:
  - with an automatic poll holding its advertisement, the promotion's first
    child gets `PollYielding`, the poll interrupts, and the retry succeeds;
  - a poll run between the publication and F6 changes nothing the promotion
    relies on;
  - a poll attempted between F6 and completion is `Busy`;
  - a poll after completion neither rematerializes nor reports the context
    deleted in A.
- [ ] Configuration change: renaming the remote between steps interrupts with
  nothing further changed.
- [ ] Privacy: the document title, body, comment body, endpoint and passphrase
  sentinel are absent from:
  - every promotion and remote row;
  - the database, the WAL and a `VACUUM INTO` backup;
  - formatted results and child output.

  Include the negative output controls the other SSH targets use.
- [ ] Run and record counts:

  ```sh
  devenv shell -- cargo test --locked --test document_promotion
  devenv shell -- cargo test --locked --test remote_synchronization
  devenv shell -- cargo test --locked --test remote_merge_recovery
  devenv shell -- cargo test --locked --test remote_polling
  devenv shell -- cargo test --locked --test comment_publication
  ```

- [ ] Append `--test document_promotion` to the native test command for the
  configured targets. Leave the workflow manual-only. Do not dispatch it;
  record native execution as pending.
- [ ] Commit (for example `test: prove confirmed document promotion over
  SSH`) and add a ticket comment.

## Task 8: Full Verification, Code Review And Approval Handoff

- [ ] Run and record the full gate, with counts and lockfile hashes:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  git diff --check
  ```

- [ ] `git merge-tree` against current `origin/main` to confirm the branch
  still integrates.
- [ ] Map every row of the Cycle's acceptance table to the named passing tests
  in the ledger. A row without a test is not met.
- [ ] Request an independent whole-branch review. Resolve or record each
  finding.
- [ ] Record native results, or state them as pending authorization. Local
  Linux results and workflow wiring are not native proof.
- [ ] Update the Cycle's evidence section, the ledger and the ticket. Add the
  review-ready comment. Keep the ticket open.
- [ ] Reconcile the rebased branch with the published ticket ref before any
  authorized push. No force push without explicit authorization bound to the
  observed remote SHA.
- [ ] Closure happens only after code-review or pull-request approval, in the
  authorized final pre-merge push.

## Plan Self-Review

Every acceptance row in the Cycle names its proving task:

| Acceptance area | Tasks |
| --- | --- |
| Preflight and digest | 2, 7 |
| Consent refusal, including D6 | 2, 6, 7 |
| Integration base after a poll | 3, 6, 7 |
| Remote and local-only paths | 3, 4, 6, 7 |
| Contained, cache loss and fresh context (R1) | 2, 3, 4, 6, 7 |
| Blockers and D2–D5 | 2, 3, 6, 7 |
| Context conflict | 6, 7 |
| Publication and deletion failures | 1, 6, 7 |
| Post-integration preservation, D7, supersession | 4, 6, 7 |
| Child attempts never strand | 0, 5, 6, 7 |
| Interruption F1–F12 | 5, 6, 7 |
| Priority | 5, 7 |
| Refresh on every return | 6, 7 |
| Privacy | 5, 7 |
| Gates | 8 |

Each task starts red, names its files and commands, and ends with a commit and
a ticket comment. The seven owner decisions are confined to marked steps, so a
different answer changes a known set of tests. Task 0's real-path
characterizations and targeted baseline have been executed and recorded in
the execution ledger, including the owner-approved T0-1 amendment. Tasks 1–8
remain planned; their implementation and verification have not been run.
