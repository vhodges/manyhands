---
title: "Wave 02 Cycle 08 One-Shot Polling And Materialization Implementation Plan"
date: 2026-10-11
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4NM26R0Q7ZXR0TDSPQB6H59"
---

# One-Shot Polling And Materialization Implementation Plan

> Approved by the owner on 2026-10-11. Implements the
> [Cycle](../Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md)
> and [design](2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-design.md).
> The owner also approved the recommended answer to all four decisions; the
> passages that depend on them are marked **(D1)–(D4)**. Do not implement
> until implementation is separately authorized.
> Execution mode: direct sequential tasks, each with a ticket comment and an
> independent review before the next.

**Goal:** A headless `poll_publication_remote` that fetches once, fast-forwards
clean strictly-behind state, materializes each conforming new context once,
refreshes discovery, persists attempt and eligibility state, and leaves every
exceptional context unchanged and visible.

**Ticket:** `01K7F6H9J2N4Q6S8V0X2Z4B6DG`

**Architecture:** New `src/repository/remote/poll.rs` orchestrates existing
reservation, transport, observation-batch, fast-forward and refresh code. New
state is two columns, one kind column and two small tables. No new phase or
safe-point name.

**Stack:** Rust, `git2` (locked libgit2/libssh2), `rusqlite`, the in-process
`russh` SSH fixture. No new dependency is expected; adding one stops for
review.

**Status:** Approved plan; implementation not yet authorized. See the
[execution ledger](2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md).

## Global Constraints

- Every local and agent Rust command uses `devenv shell -- cargo ...`.
- All work happens in the ticket worktree on `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DG`.
- Shared domain code stays free of GPUI. No CLI or desktop change.
- A poll never pushes, checkpoints, merges, rebases, stashes, forces a local
  ref, or deletes a branch, worktree or tracking ref. Tests assert the server
  received no update.
- No network call, prompt or scan holds the repository lease. Every local
  change re-observes under the lease first.
- Recovery rows hold IDs, ref names, OIDs, timestamps and fixed categories
  only. No `Debug` output of requests or session material.
- Extract shared helpers; do not rewrite synchronization behaviour. Existing
  synchronization, merge and comment-publication tests stay green unchanged
  unless a step names the change.
- Tests use counters, holds and barriers, never sleeps, developer keys, global
  Git configuration or real application data.
- Write the failing test first and record the red run before production code.
- Commit `Cargo.lock` only if dependencies change (not expected).
- Commit, push, pull request, CI dispatch, merge and closure each need their
  own authorization.

## File Map And Task Order

| Files | Ownership |
| --- | --- |
| `src/repository/remote/poll.rs`, `poll_tests.rs` (new) | Request, result, report and error types; eligibility; orchestration; classification; materialization |
| `src/repository/remote/state.rs`, `state_tests.rs` | Policy columns, poll kind, disposition and materialization tables, migration, snapshot accessors |
| `src/repository/remote/reservation.rs`, `reservation_tests.rs` | Kind-specific safe points, restart, finish and timestamp rules |
| `src/repository/remote/sync.rs`, `refs.rs` | Visibility of the fast-forward, cleanliness, target and tree-source helpers only |
| `src/repository/remote/mod.rs` | Module and public re-exports |
| `src/repository/transport/remote.rs`, `operation.rs`, `callbacks.rs` | `fetch_poll_set`, progress callback, combined automatic-plus-expectation scope |
| `src/repository/recovery.rs` | Journal-begin guard **(D4)** |
| `src/repository.rs`, `src/repository/discovery.rs` | Visibility of worktree/path validators and the active-context item rule; migration call; table inventory |
| `src/repository/read/status.rs`, `status_tests.rs` | Fill `next_eligible_at` |
| `tests/remote_polling.rs` (new), `Cargo.toml` | Real-SSH acceptance target, `harness = false` |
| `tests/remote_reservation.rs`, `tests/repository_enablement.rs`, `tests/local_authoring.rs` | Kind rules; table inventory; local-yield behaviour **(D4)** |
| `.github/workflows/build.yml` | Append `--test remote_polling` |
| `docs/research/wave-03-api-audit.md` | Replace the "missing one-shot poll" entries with the delivered API |

Order: Task 0 gates everything. Task 1 (policy state) and Task 2 (reservation
rules) are prerequisites for Task 6. Task 3 (fetch), Task 4 (classification
and fast-forward) and Task 5 (materialization) each depend only on Tasks 1–2
and are done in that order. Task 6 composes them. Task 7 proves the whole over
real SSH. Task 8 is final verification and handoff.

## Task 0: Refresh Base, Record Decisions, Characterize And Baseline

- [ ] Confirm explicit implementation authorization. Document approval and
  the D1–D4 answers (all as recommended) are recorded for 2026-10-11.
- [ ] `git fetch origin main`; rebase the ticket branch onto it from inside the
  ticket worktree; verify ancestry and a clean status. Record base and
  before/after SHAs. Re-read `AGENTS.md` if it changed.
- [ ] Re-scan for new callers of `observe_publication_remote`,
  `reserve_remote_operation*`, `remote_safe_point`, `finish_remote_operation`
  and `RemoteSnapshot`; record any Wave 03 consumer as a compatibility item.
- [ ] Characterize, with throwaway tests that are not committed, and record
  each finding in the ledger:
  - the meaning of the stored observation `tracking_oid` and the snapshot
    reader's equality check when tracking refs are written *before* the batch;
  - that restarting an interrupted or failed synchronization after its
    tracking refs moved re-fetches and does not report a false result;
  - **(D3)** whether a parked conflict's resolution reads tracking refs;
  - what `git2` worktree creation leaves on disk and in the admin directory
    when checkout fails part-way;
  - that returning `false` from `transfer_progress` stops a download with no
    ref written, and how quickly on the fixture's paced transfer.
- [ ] A finding that contradicts the design stops here for an amendment and
  owner review; do not adapt silently.
- [ ] Run and record the baseline:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

- [ ] Add a ticket comment with the base, decisions, characterization results
  and baseline.

## Task 1: Persist Attempt State And Decide Automatic Eligibility

**Files:** `src/repository/remote/state.rs`, `state_tests.rs`, new
`src/repository/remote/poll.rs` and `poll_tests.rs`, `src/repository/remote/mod.rs`,
`src/repository/read/status.rs`, `status_tests.rs`.

**Consumes:** `remote_polling_state`, `RemotePollingConfiguration`,
`PollingInterval`, `AutomaticBackoff`, `with_transaction`, the probe-and-add
migration, `PollingStatusDto`.
**Produces:** `latest_attempt_at` and `next_eligible_at` columns and snapshot
accessors; `PollIneligibility`, `AutomaticPollEligibility`,
`automatic_poll_eligibility`; a pure `eligibility(policy, next, now)` and a
pure `next_eligible(category, invocation, completed_at, policy)`.

- [ ] Failing state tests: a registry without the columns migrates with both
  empty and every existing row intact; a second migration is a no-op; a
  half-migrated table is `RecoveryRequired`; the audit at service start still
  passes.
- [ ] Failing pure tests for eligibility: disabled, paused and
  recovery-suspended each win over a due time; empty or past time is eligible;
  future time is not due; a time beyond `now + max(interval, 900 s)` is
  rewritten to that bound once and a second call does not move it again.
- [ ] Failing pure tests for the result table in the design: every outcome
  category crossed with automatic and explicit, including backoff 60 → 120 →
  … → 900 and no change for explicit failures.
- [ ] Failing read test: `polling_status` returns the stored
  `next_eligible_at` as RFC 3339 UTC and stays empty when none is stored.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib remote::poll
  devenv shell -- cargo test --locked --lib remote::state
  devenv shell -- cargo test --locked --lib read::status
  ```

- [ ] Add the columns with range checks, the migration probe, readers and one
  writer used only inside a finishing transaction. Implement the two pure
  functions and `automatic_poll_eligibility` (a short transaction; the only
  write is the clamp).
- [ ] Fill `next_eligible_at` in `polling_status`. Change no schema file and
  no other DTO field.
- [ ] Privacy: extend the existing column scan so the new columns are covered.
- [ ] Run the focused tests green. Review the diff, commit a coherent
  checkpoint (for example `feat: persist poll attempt and eligibility state`),
  and comment on the ticket.

## Task 2: Extend The Poll Reservation For A Local Stage

**Files:** `src/repository/remote/state.rs`, `reservation.rs`,
`reservation_tests.rs`, `src/repository/recovery.rs`,
`tests/remote_reservation.rs`, `tests/local_authoring.rs`.

**Consumes:** `reserve_remote_operation_with_priority`, `remote_safe_point`,
`acknowledge`, `restart_remote_observation`, `finish`,
`commit_observation_batch`, `has_pending_conflict`, the journal-begin guard.
**Produces:** stored `poll_kind`; kind-specific safe-point, restart and finish
rules; timestamps written at finish; a crate-private
`restart_remote_poll`; the conflict-pending gate **(D3)**; the local-yield
guard **(D4)**.

- [ ] Failing tests that existing behaviour is frozen: an `observe` poll still
  rejects `BeforeDiscovery`, completes only at `AfterBatchCommit`, and
  restarts as before. Existing poll rows read as `observe` after migration.
- [ ] Failing tests for `update`: `BeforeLocalMutation` accepted repeatedly
  and only after `AfterBatchCommit`; `BeforeDiscovery` accepted once after it;
  `Completed` refused before `BeforeDiscovery`; backwards order is
  `RecoveryRequired`; phase reads `local_prepared` during the local stage and
  the record still counts as the one active reservation.
- [ ] Failing tests for yield and cancel at every `update` safe point,
  including between two `BeforeLocalMutation` calls: phase becomes
  `interrupted` or `cancelled`, the reservation is released, and backoff and
  next eligible time are unchanged.
- [ ] Failing tests for restart: an interrupted or abandoned `update` record
  at any step, including `BeforeLocalMutation`, returns a fresh owner at
  `reserved` with a bumped epoch; the fenced old owner's next transition is
  refused; a completed or cancelled record replays; replaying an ID with the
  other kind is `OperationMismatch`.
- [ ] Failing tests that finish writes `latest_attempt_at` and the Task 1
  next-eligible value in the same transaction as the phase.
- [ ] **(D3)** Failing test: with a pending synchronization conflict, an
  `update` reservation attempt reports conflict pending, creates no record and
  changes no polling state. An `observe` poll is unaffected.
- [ ] **(D4)** Failing tests in `recovery` and `tests/local_authoring.rs`: a
  local save, context preparation and comment begun while a poll-priority
  record is active set `yield_requested` and return `RepositoryBusy`; against
  a manual-priority record they return today's `RecoveryRequired`; the same
  local action succeeds after the poll acknowledges. No journal row is left.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib remote::reservation
  devenv shell -- cargo test --locked --test remote_reservation
  devenv shell -- cargo test --locked --test local_authoring
  ```

- [ ] Add `poll_kind` by probe-and-add (empty for non-poll rows, constrained
  to `observe`/`update` for poll rows). Fork the three rule sites on kind;
  leave the synchronization paths untouched.
- [ ] Implement the gates and the guard change. Keep the guard's SQL the
  single source of the active-phase list.
- [ ] Run green, plus the untouched suites that share this code:

  ```sh
  devenv shell -- cargo test --locked --lib remote::
  devenv shell -- cargo test --locked --test remote_observation
  ```

- [ ] Review the diff, commit (for example
  `feat: give poll reservations a local stage`), and comment on the ticket.

## Task 3: Fetch The Poll Ref Set With Guarded Tracking Writes

**Files:** `src/repository/transport/remote.rs`, `operation.rs`,
`callbacks.rs`, `operation_tests.rs`, `tests.rs`; `tests/ssh_transport/`
(new `poll.rs` case file registered in `tests/ssh_transport.rs`).

**Consumes:** `AuthenticatedSshRemote`, the private `transfer` path and
`TransferAction::FetchTracking`, `RemoteRefPlan::classify_advertised_ref`,
`with_authenticated_remote_policy_expected`, the repository lease.
**Produces:** `fetch_poll_set`; a transfer-progress stop hook; a
remote-module-visible scope that takes both the automatic flag and the frozen
expectation.

- [ ] Failing transport tests against the real fixture: primary plus a
  document and a ticket context are fetched and their tracking refs equal the
  advertisement; a malformed family name and an out-of-family branch gain no
  tracking ref and their objects are absent; a remote rewind moves the
  tracking ref and no local branch; an absent primary writes nothing for it;
  an empty set transfers nothing and succeeds.
- [ ] Failing guard tests: a tracking ref changed between capture and write
  fails the fetch and is not overwritten; a fetched tip that is not a commit
  fails; FETCH_HEAD, tags and every unrelated ref are unchanged; no tracking
  ref is ever deleted **(D1)**.
- [ ] Failing stop tests: with the fixture pacing the transfer, a stop request
  ends the download, no tracking ref is written, and the helper count shows no
  further connection. The hook is not called under the lease.
- [ ] Failing policy tests: an automatic scope with a blocked unlock returns
  without prompting; an endpoint or key that differs from the frozen
  expectation is refused before download.
- [ ] Failing negative test: the server records zero receive-pack commands and
  zero ref updates across every case.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib transport
  devenv shell -- cargo test --locked --test ssh_transport poll_
  ```

- [ ] Implement `fetch_poll_set` on the shared transfer path: build explicit
  forced mappings from the advertisement, extend the scope check to accept
  exactly that set, reuse the capture/verify/guarded-write sequence, and
  return the advertisement fetched against.
- [ ] Register `transfer_progress` only for this action. The caller supplies a
  throttled closure; the transport knows nothing about reservations.
- [ ] Run green including the whole `ssh_transport` target, which must be
  unchanged for existing cases.
- [ ] Review the diff, commit (for example
  `feat: fetch the poll ref set with guarded tracking refs`), and comment on
  the ticket.

## Task 4: Classify Local State And Fast-Forward Clean Targets

**Files:** `src/repository/remote/poll.rs`, `poll_tests.rs`,
`src/repository/remote/sync.rs`, `refs.rs`, `src/repository.rs`,
`src/repository/discovery.rs`.

**Consumes:** `materialized_target`, `require_clean_target`, `local_oid`,
`fast_forward`, `clean_descendant`, `canonical_tree_sources`,
`canonical::validate_context`, the active-context item rule.
**Produces:** `PollDisposition` with fixed guidance; a pure
`classify_relation`; `classify_local_target`; `poll_fast_forward`;
`conforming_context_tree`.

- [ ] Failing pure tests for `classify_relation`: equal, strictly behind,
  ahead, diverged, and an ancestry error surfaced rather than guessed.
- [ ] Failing repository tests for `classify_local_target`, each asserting the
  disposition and an unchanged before/after snapshot of worktree bytes, index,
  status, HEAD and refs: modified tracked file; staged change; untracked file;
  untracked directory; ignored file only (clean); in-progress merge; index
  conflict; detached HEAD; other branch checked out; worktree directory
  missing; worktree unregistered; deterministic path a symlink; primary not
  checked out at the root; index tree different from HEAD.
- [ ] Failing fast-forward tests: a clean strictly-behind primary and context
  advance branch, index and worktree to the tracking commit; a second call is
  `current`; a ref moved between classification and update is refused with
  nothing changed; an ignored file colliding with an incoming path leaves
  everything unchanged and reports `dirty`.
- [ ] Failing tree tests for `conforming_context_tree`, built from in-memory
  trees: valid ticket; valid document; document ID present twice; ticket ID
  not matching the branch; kind not matching the branch; item absent;
  malformed front matter; non-UTF-8 blob; executable or symlink mode at the
  item path; gitlink; an entry under `.manyhands/worktrees/`; valid item with
  an invalid comment (conforming).
- [ ] Failing agreement test: every tree accepted by
  `conforming_context_tree`, once checked out as a worktree, is reported as an
  active context by the existing discovery. For rejected trees assert poll
  rejection and no automatic creation, not discovery inactivity. **Owner-approved
  amendment, 2026-10-13:** materialization validation may be stricter than local
  discovery; retain executable rejection and existing discovery behavior. The
  agreement is accepted ⇒ active only, with no rejected ⇒ inactive implication.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib remote::poll
  ```

- [ ] Widen visibility of the four synchronization helpers and the tree
  reader to the remote module; widen the active-context item rule to the
  crate. Change no body except what a moved test hook needs.
- [ ] Implement the classification order from the design and the guidance
  table. One disposition per target, no free-text reason.
- [ ] Run green, then the suites that share the moved helpers:

  ```sh
  devenv shell -- cargo test --locked --lib remote::
  devenv shell -- cargo test --locked --test remote_synchronization
  devenv shell -- cargo test --locked --test remote_merge_recovery
  ```

- [ ] Review the diff, commit (for example
  `feat: classify and fast-forward clean poll targets`), and comment on the
  ticket.

## Task 5: Materialize A Conforming New Context Exactly Once

**Files:** `src/repository/remote/poll.rs`, `poll_tests.rs`,
`src/repository/remote/state.rs`, `state_tests.rs`, `src/repository.rs`,
`src/repository/discovery.rs`, `tests/repository_enablement.rs`.

**Consumes:** `validate_deterministic_worktree_path_chain`,
`ensure_authoring_worktree_base`, `validate_context_worktree`,
`branch_checked_out_in_another_worktree`, the repository lease, Task 4's
`conforming_context_tree`.
**Produces:** `remote_poll_materializations`; `materialize_context`;
`reconcile_materialization`.

- [ ] Failing schema tests: the table is created all-or-nothing with the
  other remote tables, validated at startup, and listed in the registry
  inventory in `tests/repository_enablement.rs`; an existing registry migrates
  with no row.
- [ ] Failing success tests for a document and a ticket: exactly one local
  branch at the fetched commit, one registered worktree at the deterministic
  path with HEAD on the branch, a clean status, no upstream configuration, the
  intent `complete`, and `validate_context_worktree` passing.
- [ ] Failing once-only tests: a second call creates nothing and reports the
  context present; two services racing on one data directory produce one
  branch and one worktree.
- [ ] Failing refusal tests, each with an unchanged repository snapshot: local
  branch already present at the same commit, at another commit, or checked out
  elsewhere; path present as a file, an empty directory, a non-empty directory
  or a symlink; worktree name registered at another path; `.manyhands` or the
  worktrees base redirected. Each reports `blocked_by_local_state`.
- [ ] Failing interruption tests, one per boundary, using new poll variants
  added to the existing public `FailurePoint` seam
  (`open_at_with_failure_point_for_testing`): after the intent row; after branch creation; after a failed
  worktree creation; after worktree creation and before `complete`. For each,
  assert the observed Git state, then that the next call takes exactly the
  design's reconciliation action and repeats nothing **(D2)**.
- [ ] Failing test that a partly present worktree is reported
  `materialization_incomplete` and that no file, directory, branch or
  registration is removed **(D2)**.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib remote::poll
  devenv shell -- cargo test --locked --lib remote::state
  devenv shell -- cargo test --locked --test repository_enablement
  ```

- [ ] Implement the five ordered steps under one lease acquisition, with the
  intent row written through the short cache transaction. Use the worktree
  options local authoring uses. Write no branch configuration.
- [ ] Implement reconciliation from Git state first, the intent second.
- [ ] Run green. Review the diff, commit (for example
  `feat: materialize conforming remote contexts once`), and comment on the
  ticket.

## Task 6: Compose The One-Shot Poll

**Files:** `src/repository/remote/poll.rs`, `poll_tests.rs`,
`src/repository/remote/state.rs`, `state_tests.rs`,
`src/repository/remote/mod.rs`, `src/repository.rs`,
`docs/research/wave-03-api-audit.md`.

**Consumes:** Tasks 1–5; `observation_plan`, `observation_configuration`,
`configure_endpoints`, `commit_observation_batch`, `transport_category`,
`refresh_repository`, the refresh-required flag.
**Produces:** public `poll_publication_remote` and its request, result,
report and error types; `remote_poll_dispositions`; snapshot accessors for the
latest dispositions and times.

- [ ] Failing type tests: the request carries no URL, refspec, worktree, key
  or passphrase; formatted results and errors show only IDs, ref names, OIDs
  and fixed text.
- [ ] Failing gate tests on a repository whose remote is unreachable and with
  a credential provider that panics if asked, so any transport is a failure: no
  publication remote; each ineligibility reason for automatic; explicit
  bypasses each and leaves stored policy identical; conflict pending **(D3)**;
  local operation pending; busy; poll yielding.
- [ ] Structure the orchestration so everything after the fetch is one
  crate-private function taking the frozen plan, configuration and fetched
  advertisement. Library tests drive that function on a plain local
  repository whose tracking refs the test installs; the fetch itself is
  proved only over real SSH in Tasks 3 and 7. No fake transport is added.
- [ ] Failing flow tests on that post-fetch function: the safe points occur
  in the design's order; the batch and the initial `not_reached`
  dispositions commit together; targets are processed primary, existing
  contexts, new contexts, each by ref name; each processed target replaces its
  row; completion writes outcome, times and backoff once.
- [ ] Failing stop tests: yield and cancel at each safe point and between
  targets return `Polled` with the right completion and the targets already
  changed, set refresh-required when anything changed, and run no scan.
- [ ] Failing change-detection tests: configuration, endpoint or key changed
  after the fetch; a tracking ref changed before a target; a local branch
  changed before a target. Each interrupts with nothing further changed.
- [ ] Failing replay tests: a completed ID returns the stored report with no
  transport; an older completed ID returns its outcome with an empty target
  list; an interrupted ID without `restart` is `Interrupted`; with `restart`
  it re-fetches and reports already-done targets as `current`.
- [ ] Failing index tests: refresh success is `Complete`; an injected refresh
  failure is `IndexPending` with the same report; a later
  `refresh_repository` repairs discovery with no transport; a crash between
  completion and refresh leaves refresh-required set.
- [ ] Failing snapshot tests: `RemoteSnapshot` exposes the latest attempt
  time, next eligible time and per-target dispositions; a materialized context
  reads `observed` after the refresh; the audit at service start accepts the
  new tables.
- [ ] Record red:

  ```sh
  devenv shell -- cargo test --locked --lib remote::poll
  devenv shell -- cargo test --locked --lib remote::state
  ```

- [ ] Add `remote_poll_dispositions` (all-or-nothing migration, inventory
  entry, privacy scan). Implement the orchestration in the design's data-flow
  order. Use a fresh internal operation ID for the refresh.
- [ ] Re-export the public types. Update the API audit: the `poll once`,
  R09, R13 and R16 rows name the delivered operation, its results and what
  remains for Wave 03. Claim nothing about scheduling.
- [ ] Run green, then the full library suite:

  ```sh
  devenv shell -- cargo test --locked --lib
  ```

- [ ] Review the diff, commit (for example `feat: add one-shot remote poll`),
  and comment on the ticket.

## Task 7: Prove The Poll Over Real SSH

**Files:** new `tests/remote_polling.rs`, `Cargo.toml`,
`.github/workflows/build.yml`.

**Consumes:** `tests/support/{ssh_remote,ssh_harness,ssh_privacy}.rs`, the
two-clone `Pair`/`Side` shape of `tests/remote_merge_recovery.rs`, the public
API only.
**Produces:** the Cycle's exit evidence.

- [ ] Declare `[[test]] name = "remote_polling", harness = false` and reuse the
  pre-thread initialization, isolated child and output-privacy architecture.
  Keep fixture directory names short for the Windows path limit. Every case
  ends with the privacy scan and a check that the server recorded no
  receive-pack command.
- [ ] Clean updates, each asserting branch, index, worktree bytes and
  tracking ref, then a second poll that changes nothing and transfers no new
  object: primary fast-forward; document context fast-forward; ticket context
  fast-forward; all three in one poll; already current.
- [ ] Materialization: a collaborator's new document context and new ticket
  context each appear as one branch and one worktree and are listed by
  discovery with their comments; a second poll, a reopened service and a
  second service on the same data directory create nothing more.
- [ ] Exceptional-state matrix, each asserting the disposition, fixed
  guidance and a byte-for-byte unchanged local snapshot: dirty tracked;
  staged; untracked; conflicted worktree left by a synchronization; local
  ahead; diverged; context on the wrong branch; worktree directory removed;
  primary not checked out; remote context deleted with local work present;
  remote context renamed; nonconforming tree; malformed family name;
  out-of-family branch; pre-existing local branch or path blocking
  materialization; history unknown after registry loss.
- [ ] Policy: automatic poll while disabled, paused, recovery-suspended and
  not due makes zero connections and zero prompts; explicit poll runs in each
  and leaves policy unchanged; success sets the next eligible time from the
  interval; an unreachable server doubles backoff to the cap across automatic
  attempts and an explicit attempt leaves it alone; a rejected key and an
  unapproved host record their category without backoff; a locked key with a
  blocked session does not prompt on an automatic attempt.
- [ ] Priority: with the advertisement held, a manual synchronization from a
  second service gets `PollYielding`, the poll returns interrupted having
  changed nothing, and the manual retry succeeds; the same with the transfer
  paced, proving the in-transfer stop; a cancel at each boundary; an explicit
  poll in progress makes a second manual operation `Busy`; **(D4)** a local
  save during an automatic poll gets `RepositoryBusy` and succeeds on retry.
- [ ] **(D3)** With a retained conflict from a real divergence, both
  invocations report conflict pending and make zero connections.
- [ ] Interruption and reopen, using registry triggers and fixture faults, one
  case per boundary: disconnect during transfer; failure after tracking
  writes; failure committing the batch; failure after a fast-forward and
  before its row; failure inside materialization at each step; failure at
  completion; refresh failure. Each asserts actual refs, worktrees and
  records, then that `restart` or the next poll finishes only what is
  unfinished.
- [ ] Configuration change mid-poll: publication remote renamed, endpoint
  changed, and selected key changed while the advertisement is held. The poll
  stops and changes nothing local.
- [ ] Privacy: a comment body, item title, endpoint and passphrase sentinel
  are absent from every recovery row, the database, WAL and a `VACUUM INTO`
  backup, formatted results and child output. Include the negative output
  controls the other SSH targets use.
- [ ] Run and record counts:

  ```sh
  devenv shell -- cargo test --locked --test remote_polling
  devenv shell -- cargo test --locked --test remote_observation
  devenv shell -- cargo test --locked --test remote_synchronization
  devenv shell -- cargo test --locked --test remote_merge_recovery
  devenv shell -- cargo test --locked --test comment_publication
  ```

- [ ] Append `--test remote_polling` to the native test command for all five
  targets. Leave the workflow manual-only. Do not dispatch it; record native
  execution as pending.
- [ ] Review the diff, commit (for example
  `test: prove one-shot polling and materialization`), and comment on the
  ticket.

## Task 8: Full Verification, Code Review And Approval Handoff

- [ ] Run and record the full gate with counts and lockfile hashes:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  git diff --check
  ```

- [ ] Map every row of the Cycle's acceptance table to the named passing
  tests in the ledger. A row without a test is not met.
- [ ] Request an independent whole-branch review. Resolve or record each
  finding.
- [ ] Record native results for the five targets, or state them as pending
  authorization, unavailable or owner-deferred. Local Linux results and
  workflow wiring are not native proof.
- [ ] Update the Cycle's evidence section, the ledger and the ticket; add the
  review-ready comment. Keep the ticket open.
- [ ] Reconcile the rebased branch with the published ticket ref before any
  authorized push; no force push without explicit authorization bound to the
  observed remote SHA.
- [ ] Closure happens only after code-review or pull-request approval, in the
  authorized final pre-merge push. Merge and worktree cleanup are separately
  authorized.

## Plan Self-Review

Every acceptance row in the Cycle names its proving task: policy and attempt
state in Tasks 1, 2 and 7; fetch and no-push in Tasks 3 and 7; priority in
Tasks 2 and 7; fast-forward and preservation in Tasks 4 and 7; materialization
in Tasks 5–7; exceptional remote state in Tasks 4–7; interruption in Tasks 2,
5, 6 and 7; index separation in Tasks 6 and 7; privacy in Tasks 1 and 7; gates
in Task 8. Each task starts red, names its files and commands, and ends with a
commit and a ticket comment. The four owner decisions are confined to marked
steps so a different answer changes a known set of tests. Nothing here has
been run; all verification is planned.
