---
title: "Wave 02 Cycle 04 Remote Observation And Recovery Implementation Plan"
date: 2026-10-05
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M5000000ACD5888AA91D6181"
---

# Remote Observation And Recovery Implementation Plan

> **For agentic workers:** Do not begin until the user approves the Cycle,
> design, and this plan and explicitly authorizes implementation. Then use the
> implementation workflow selected by the user, re-read the design, repeat the
> ticket worktree preflight, and record every checkpoint on the Cycle ticket.

**Goal:** Persist a complete, authenticated remote-ref observation and the
reservation/cancellation state required for safe later synchronization, without
transferring refs or changing any branch/worktree/canonical file.

**Architecture:** A pure ref-plan/classifier feeds a dedicated application-local
remote-state schema. A short cache transaction reserves an operation; Cycle
03's scoped SSH adapter lists advertised refs outside all short locks; a second
transaction atomically persists a batch and derived visible states. Future
Cycles reuse the ref plan and reservation, acquire the Git lease only before a
local mutation, and re-observe actual Git state.

**Tech stack:** Rust 2024, locked `git2`/libgit2 transport from Cycle 03,
rusqlite, existing `RepositoryService`/recovery/discovery layers, disposable
Rust SSH fixture, Devenv for local Cargo commands, and the existing five-target
native GitHub Actions matrix.

**Spec:** [Design](2026-10-05-wave-02-cycle-04-remote-observation-and-recovery-design.md)
and [Cycle](../Cycles/wave-02-cycle-04-remote-observation-and-recovery.md).

**Status:** Approved 2026-10-05. The user authorized implementation through
subagent-driven development. Publication, merge, ticket closure, and cleanup
remain separate later authorizations.

## Global Constraints

- Work only in ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DC`'s existing worktree and
  branch. Preserve unrelated main/worktree edits.
- Keep shared domain logic in `src/lib.rs`-reachable library modules; add no
  GPUI dependency or desktop/CLI grammar.
- Reuse Cycle 03's selected-key, host-trust, session-only credential boundary.
  No agent/default-key/helper/password/anonymous fallback and no raw backend or
  server diagnostics may enter this Cycle's records or errors.
- Accept a shared process `SessionCredentials` instance rather than creating one
  per observation. Desktop startup may use that provider once to unlock an
  eligible protected key; later manual/automatic observations reuse only the
  successful in-memory session cache. Cancellation/provider failure becomes a
  session-only unlock-required result with no repeated background prompt and no
  change to durable `paused`; host approval remains separately explicit.
- The only live remote action is authenticated advertisement. Do not call
  `download` or `push`, create/update any tracking ref, write `FETCH_HEAD`,
  alter a local/remote branch, materialize a worktree, checkpoint, merge,
  rebase, fast-forward, prune, or refresh canonical discovery.
- Persist only non-secret repository/remote names, validated ref names, OIDs,
  action/phase/priority, timestamps, and fixed redacted categories. Never
  persist URLs, drafts, response bodies, key paths/material, passphrases, or
  secret-derived values.
- Preserve the user-approved cache-loss rule: publish a non-secret
  remote-history-recovery marker before corrupt-registry replacement or fail
  replacement. Marker presence suspends automatic polling until explicit resume
  without changing explicit pause, and an absent local context remains
  `history_unknown` until a later explicit recovery/republish confirmation.
- Network, credential prompting, and scans remain outside the repository lease
  and SQLite/cache guard. Later local mutation code must reacquire the existing
  repository lease and re-observe Git state; do not pre-acquire it here.
- Preserve the RFC polling defaults: enabled/unpaused, five minutes, interval
  bounds one–sixty minutes, automatic backoff one–fifteen minutes, and no
  automatic delay of an explicit one-shot action. Do not start a scheduler.
- Use `devenv shell -- cargo ...` for every local Rust command. Preserve
  `Cargo.lock` only if dependencies change and `devenv.lock` only if Devenv
  inputs change.

## Review Focus

1. A partial, failed, cancelled, or stale advertisement must never turn a prior
   context into `remotely_deleted`.
2. A manual operation must request poll yield without racing the poll's current
   SQLite transition, taking its reservation, or holding a Git lease during
   network activity.
3. Adding remote state must not reinterpret/block legacy local recovery
   records, leak raw endpoint/remote error data, or weaken the selected-key and
   host-pin policy.
4. Observation must prove no hidden ref update (`FETCH_HEAD`, local refs,
   remote-tracking refs, index/worktree, and canonical bytes remain unchanged).
5. A recognized branch name must stay distinct from a validated/materialized
   item context; malformed and remote-deleted state must remain visible and
   non-destructive on every native target.
6. Cache loss must not make a formerly published or unknown context eligible
   for implicit first publication, nor allow an automatic poll to resume merely
   because a fresh database was initialized.
7. A protected-key unlock is one shared desktop-process interaction, not one
   prompt per poll. Unlock cancellation/failure and missing host approval must
   suspend the affected session without mutating durable pause or silently
   authorizing trust.

## File Map And Dependencies

| Files | Responsibility |
| --- | --- |
| `src/repository/remote/{mod,refs,state,reservation,observation,tests}.rs` | New remote domain types, pure ref plan/classifier, snapshot state, durable reservation transitions, authenticated advertisement orchestration, and private tests. |
| `src/repository.rs` | Export the remote module/public requests/outcomes; extend operation/error inspection only where needed to present typed recovery without raw diagnostics. |
| `src/repository/discovery.rs` | Migrate new application-local tables, join remote state into snapshots/problems, and preserve old schema/rebuild behavior. |
| `src/repository/recovery.rs` | Reconcile dedicated remote records alongside legacy local operation records; preserve action/target mismatch and restart semantics. |
| `src/repository/transport/{operation,remote}.rs` | Narrow crate-private adapter support only if required to invoke the existing authenticated advertisement under the proposed remote operation context; do not expose raw Git handles or transfers. |
| `tests/remote_observation.rs` | Public service, real authenticated fixture, migration, no-mutation, deletion/malformed/unmaterialized, reservation, interruption, and privacy evidence. |
| `tests/support/{mod,ssh_remote}.rs` | Minimal fixture helpers to add/remove owned test refs and capture ref/FETCH_HEAD/worktree preservation; keep helpers test-only and shell-free. |
| `.github/workflows/build.yml` | Include the focused remote-observation test target in every native headless contract job. |
| `docs/Cycles/...`, `docs/plans/...`, `.manyhands/comments/...` | Keep decisions, task evidence, platform gaps, review result, and final handoff current. |

Order: baseline, pure contracts, persistence/recovery, reservations, authenticated
observation, discovery/fixture proof, then complete verification and review.
Each task must land as a coherent reviewable checkpoint with a ticket comment;
do not combine unfinished downstream synchronization behavior into a task.

## Baseline Checkpoint

- [x] Repeat the fetch/rebase/ancestry preflight in this ticket worktree and
  record fetched base plus before/after ticket heads. Inspect the remote ticket
  ref before any authorized publication; never force push without new explicit
  authorization.
- [x] Run and record the required local baseline:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

- [x] Record pre-existing failures separately. Confirm the implementation
  begins with approved Cycle/design/plan and explicit implementation authority.

## Task 1: Define Remote Ref And State Contracts

**Files:** create `src/repository/remote/mod.rs`, `refs.rs`, `state.rs`, and
unit tests; modify `src/repository.rs` only for module/type exports.

**Interfaces:** Define validated `RemoteRefPlan`, recognized/malformed ref
classification, remote/tracking observation values, polling configuration,
operation action/priority/phase, snapshot state, and redacted outcomes. Keep
the exact branch/ref formulas in one pure module. A caller supplies no URL,
refspec, OID, or credential.

- [x] Add failing tests for primary names, valid document/ticket ULIDs,
  unsupported family-shaped refs, nested/suffixed IDs, ref separators/control
  characters, remote-name changes, tracking-name derivation, and all three
  exact `+` refspecs.
- [x] Add failing boundary tests for five-minute defaults, one/sixty-minute
  intervals, one/fifteen-minute backoff, explicit-call backoff exemption, and
  fixed/redacted status rendering.
- [x] Implement pure contracts with no repository, SQLite, or network access.
  Assert no valid classification is mistaken for canonical-tree validation.
- [x] Run focused library tests red then green; record the contract checkpoint
  and commit `feat: define remote observation contracts`.

## Task 2: Migrate And Query Non-Secret Remote State

**Files:** create `remote/state.rs`; modify `discovery.rs`, `recovery.rs`, and
the relevant snapshot/public types in `repository.rs`; add focused migration
and snapshot tests in `tests/remote_observation.rs`.

**Interfaces:** Create the design's polling, batch/ref/context-state, and
remote-operation tables with foreign keys, bounds, uniqueness, and indexes.
Expose query/update helpers only through the remote module. Extend snapshot
presentation with remote states/problems while retaining existing local
discovery semantics.

The polling-policy row is repository-scoped: retain explicit pause and interval
across publication remote/key removal or replacement, while invalidating active
reservations and remote-specific observations. The current configuration alone
determines whether automatic polling is eligible.

- [x] Add failing tests that start from the current Cycle 03 schema and legacy
  operation rows, run migration repeatedly, preserve all old rows, and create
  safe defaults exactly once.
- [x] Test completed batch replacement, last-successful OID retention,
  unmaterialized/malformed/deleted/history-unknown query rows, configuration
  bounds, and corrupt/invalid row rejection with recovery-required rather than
  repair.
- [x] Add corrupt-registry cases that prove marker publication precedes
  replacement, marker-publication failure preserves the old database, fresh
  state is automatically recovery-suspended without changing `paused`, and an
  absent local context cannot become first-publication eligible after restart.
- [x] Scan schema, rows, WAL/journal/backups, `Display`/`Debug`, and snapshots
  for URL, passphrase, key, remote-response, and Markdown sentinel leakage.
- [x] Implement one short transaction per durable transition. Do not treat
  SQLite state as a Git lock or canonical source. Record/commit
  `feat: persist remote observations and polling policy`.

## Task 3: Reserve Remote Work And Reconcile Safe Interruption

**Files:** create `remote/reservation.rs`; modify `remote/state.rs`,
`recovery.rs`, and `repository.rs`; add private and integration concurrency/
failure tests.

**Interfaces:** Implement begin/replay/mismatch checks, active reservation
inspection, manual `yield_requested`, cancellation request, safe-point
acknowledgement, completion/interruption transitions, and restart inspection.
Remote actions retain their stable `OperationId`; the existing local recovery
journal still blocks incompatible operations.

- [x] Add failing two-service tests: a poll reserves; a manual action records
  yield and receives `PollYielding`; poll acknowledgement releases only after
  the safe transition; manual retry reserves; competing manual actions and
  mismatched duplicate IDs fail deterministically.
- [x] Add named hooks/failure cases before transport, after transport, before
  batch commit, after batch commit, and during per-ref persistence. Assert
  prior batch/local Git state preservation and retry of only unfinished
  read-only work.
- [x] Represent those hooks as durable named safe points. Cycle 04 invokes no
  transfer-progress callback because it does not transfer; retain the same
  mechanism for Cycle 08 to call from its later fetch-progress path.
- [x] Test cancellation at the same safe points, restart/reconciliation,
  legacy-local-record coexistence, and no repository lease across waits.
- [x] Implement SQL atomics and fixed recovery guidance; record/commit
  `feat: coordinate remote observation reservations`.

## Task 4: Persist An Authenticated Complete Advertisement

**Files:** create `remote/observation.rs`; modify `remote/mod.rs`, narrowly
extend `transport/{operation,remote}.rs` if access is insufficient, and add
focused service tests.

**Interfaces:** `observe_publication_remote` creates/replays a reservation,
uses the existing selected-key Fetch-direction scoped adapter to call only
`advertisement()`, observes existing tracking refs locally, applies safe-point
checks, atomically persists one complete batch, and returns a typed snapshot.
It accepts existing host approval/session-provider values but no raw transport
handle or endpoint input.

- [x] Add failing tests for selected-key success, absent selection/remote,
  host approval/rejection, cancellation/yield before and after advertisement,
  protocol failure, and database failure. Assert typed Cycle 03 errors and
  only redacted persisted result categories.
- [x] Add shared-session tests: a protected key prompts once for initial
  startup/first observation and later observations reuse the cache; selection,
  source, clear, and process-session replacement invalidate it; cancellation or
  provider failure leaves an unlock-required result and does not call the
  provider again from automatic retry. Keep host approval as a separate exact
  interaction and assert neither block overwrites durable `paused`.
- [x] Capture refs, `FETCH_HEAD`, index, worktree status/canonical bytes, and
  tracking refs before successful and failed calls. Assert all stay byte-for-
  byte or OID-for-OID unchanged, apart from approved host-pin and SQLite
  observation changes.
- [x] Ensure completion after a durable batch is idempotent; an interrupted
  pre-commit call re-advertises, while retry after commit reads/reconciles
  rather than inferring a new deletion.
- [x] Implement without `download`, `push`, direct raw `Remote` exposure, or
  a Git lease around prompt/network work. Record/commit
  `feat: observe authenticated remote refs`.

## Task 5: Prove Visible Exceptional States With The Real Fixture

**Files:** extend `tests/support/ssh_remote.rs` and `tests/support/mod.rs` only
as needed; create/extend `tests/remote_observation.rs`; update
`.github/workflows/build.yml`.

- [x] Add shell-free fixture controls for owned remote ref creation/deletion
  and complete advertisement capture. Keep arbitrary path/command handling and
  sensitive output forbidden.
- [x] Prove a recognized remote-only context is `unmaterialized`; a malformed
  family ref is visible but cannot create a local branch/worktree; a ref present
  in one complete batch and absent in the next is `remotely_deleted` while its
  locally observed-published context remains untouched. Prove a never-published
  local context remains first-publication eligible, while a cache-loss marker
  makes an absent local context `history_unknown` and blocks that inference.
- [x] Prove a failed advertisement after a prior good batch does not mark any
  state deleted. Exercise poll/manual yield and cancellation around real
  authenticated advertisements, not only a mock recorder.
- [x] Extend each native CI target's headless test command with
  `--test remote_observation`. Do not describe workflow editing as native
  evidence; actual all-target runs remain a final acceptance gate.
- [x] Run focused tests red then green, execute an intentional temporary
  redaction/atomicity probe where safe, remove it, and record/commit
  `test: prove remote observation recovery states`.

## Task 6: Verify, Review, And Handoff

- [x] Run the final local gate from the ticket worktree:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  git diff --check
  ```

- [x] Record focused test counts and results, fixture preservation evidence,
  privacy scan result, exact source/lockfile changes, and native CI run links.
  Label unavailable native evidence as pending rather than passing.
- [x] Request independent implementation/whole-branch review; resolve findings,
  rerun affected and final checks, and add a review-ready ticket comment.
- [ ] Keep the ticket open until code review or PR approval. Push/PR, merge,
  close, and worktree cleanup each require their own later authorization and
  lifecycle step.
