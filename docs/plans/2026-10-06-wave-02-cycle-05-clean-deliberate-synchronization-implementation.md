---
title: "Wave 02 Cycle 05 Clean Deliberate Synchronization Implementation Plan"
date: 2026-10-06
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M5100000D9E3F4G5H6J7K8M9"
---

# Clean Deliberate Synchronization Implementation Plan

> **For agentic workers:** Do not begin until the user approves the Cycle,
> design, and this plan and explicitly authorizes implementation. Then use
> `implementing-a-cycle` and the user-selected execution method task by task,
> re-read the design, repeat ticket preflight, and record every checkpoint on
> ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DD`.

**Goal:** Deliver one selected-key, clean-only synchronization operation for a
configured primary or materialized shared context, including verified ordinary
publication and non-repeating index handoff.

**Architecture:** Build pure exact-ref and ancestry planning on Cycle 04's
remote state, then use its durable reservation across short authenticated fetch
and push phases. Reacquire the existing repository lease for each local
re-observation/fast-forward only, record every durable boundary, verify remote
publication by OID, and let the existing discovery refresh run only after a
stable Git outcome.

**Tech Stack:** Rust 2024, locked `git2`/libgit2, Cycle 03 selected-key SSH
adapter, Cycle 04 remote-state/reservation model, rusqlite, disposable Rust SSH
fixture, Devenv, and the existing five-target native GitHub Actions matrix.

**Spec:** [Cycle](../Cycles/wave-02-cycle-05-clean-deliberate-synchronization.md)
and [design](2026-10-06-wave-02-cycle-05-clean-deliberate-synchronization-design.md).

**Status:** Approved by the user on 2026-10-06. This planning approval does not
authorize Rust changes, publication, merge, ticket closure, or worktree
cleanup; explicit implementation authorization remains required.

## Global Constraints

- Work only in ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DD`'s existing worktree and
  branch. Preserve unrelated main and other-worktree edits.
- Before implementation, fetch `origin/main`, rebase this ticket branch onto
  it, verify ancestry/cleanliness, and record the base and before/after heads.
  Confirm Cycle 04 is merged and its reviewed API is available; do not copy its
  unmerged source or weaken its recovery model.
- Keep all domain behavior in the library. Add no GPUI dependency, desktop/CLI
  grammar, scheduler, direct system-Git dependency, new network backend, or
  Cargo dependency unless separately approved.
- Reuse the Cycle 03 scoped selected-key/host-trust/session boundary. Do not
  accept or persist URL, raw refspec, raw worktree path, credential, key,
  passphrase, server text, response body, canonical Markdown, or force flag.
- Use Cycle 04's one remote-operation record/reservation and the existing short
  common-Git lease. Network, prompting, and discovery never hold that lease.
- Fetch only exact primary/context mappings with `update_fetchhead(false)`;
  push only exact normal mappings. Never force-push, prune, rebase, merge,
  checkpoint, stash, reset, discard, materialize, delete, or clean up.
- A complete final Fetch advertisement, not a failed/partial transfer, is the
  only source of remote deletion classification. Preserve `never_published`,
  `observed_published`, and `history_unknown` semantics from Cycle 04.
- Compute context integration virtually before modifying its branch; use
  expected-old-OID ref writes and safe checkout after a fresh clean/status
  check. Preserve a ref/worktree mismatch for recovery rather than rollback.
- Treat configured `pushurl` as a separate Push-direction destination. Verify
  its target OID before and after push; never persist/display either endpoint.
- Use `devenv shell -- cargo ...` for every local Rust command. Run the full
  required final checks and CLI smoke test; preserve lockfiles unless their
  inputs change.

## Review Focus

1. A second ancestry check must not leave an item context partially
   fast-forwarded when it discovers a primary merge is required; Task 1 owns
   this virtual-plan test.
2. A push accepted before disconnect or database failure must not be repeated
   blindly; Tasks 3 and 5 own post-push observation and exact-ID replay tests.
3. Dirty, conflicted, wrong-branch, and wrong-worktree targets must remain
   byte-for-byte untouched; Task 4 owns these preflight and safe-checkout tests.
4. A different Push endpoint must be checked independently without URL/server
   leakage or tracking-ref confusion; Tasks 3 and 5 own fixture/privacy proof.
5. A discovery failure after stable Git state must refresh only on replay and
   never cause another fetch, fast-forward, or push; Task 5 owns this test.

## File Map And Dependency Order

| Files | Responsibility |
| --- | --- |
| `src/repository/remote/refs.rs` | Pure exact primary/context fetch and ordinary push mappings plus graph-planning helpers. |
| `src/repository/remote/state.rs` | Backward-compatible remote-operation fields/checkpoints, current complete-advertisement persistence, and index-pending durable query/update. |
| `src/repository/remote/reservation.rs` | Generic synchronization reservation ownership, safe points, explicit restart/reconciliation, and terminal replay rules. |
| `src/repository/remote/observation.rs` | Extract/reuse complete Fetch advertisement persistence without duplicating Cycle 04 poll policy. |
| `src/repository/remote/sync.rs` | Public request/outcome/error types, target preflight, exact fetch/observation, one fast-forward, push verification, and discovery handoff. |
| `src/repository/remote/mod.rs`, `src/repository.rs` | Module wiring and only the approved public exports. |
| `src/repository/transport/{operation,remote}.rs` | Narrow scoped-adapter capability for exact transfer/progress and Push-direction advertisement, retaining every Cycle 03 callback/recheck rule. |
| `tests/remote_synchronization.rs` | Public-service, two-clone authenticated SSH, recovery, privacy, and index-replay evidence. |
| `tests/support/{mod,ssh_remote}.rs` | Minimal shell-free owned-ref, endpoint, disconnect, and preservation fixture controls. |
| `.github/workflows/build.yml` | Add the focused synchronization target to every existing native headless contract job. |
| Cycle/design/plan and ticket comments | Keep decisions, checkpoints, evidence gaps, review, and handoff current. |

Dependency order: baseline/Cycle 04 reconciliation; pure ref and graph
contracts; durable replay/checkpoints; scoped transfer support; orchestration
and local mutation; real fixture evidence; full verification and review. Each
task is a coherent reviewable checkpoint with a ticket comment; do not fold
Cycle 06+ behavior into an earlier task.

## Baseline And Dependency Checkpoint

- [ ] Repeat the fetch/rebase/ancestry preflight in this ticket worktree. Record
  current main SHA, before/after ticket heads, conflict decisions, and clean
  result. Inspect the remote ticket branch before any later publication; never
  force-push without separate explicit authorization.
- [ ] Verify that the merged Cycle 04 types support `RemoteRefPlan`, complete
  persisted observations, publication evidence, durable manual reservation,
  stable operation ID replay, and lease-safe local re-observation. Record any
  name-only reconciliation in a ticket comment; stop for a changed behavioral
  contract.
- [ ] Run and record the required baseline:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

- [ ] Record pre-existing failures separately and confirm approved artifacts
  plus explicit implementation authority before editing Rust.

## Task 1: Define Exact Refs, Targets, And Clean Graph Plans

**Files:** modify `src/repository/remote/refs.rs`, `state.rs`, `mod.rs`, and
`src/repository.rs`; add/extend private remote unit tests.

**Consumes:** Cycle 04 `RemoteRefPlan`, `RemoteOperationTarget`,
`RemotePublicationEvidence`, canonical `ItemId`, and `AuthoringKind`.

**Produces:** `SynchronizationTarget`, validated `SynchronizeRemoteRequest`
target construction, `primary_fetch_refspec`, `context_fetch_refspec`,
`primary_push_refspec`, `context_push_refspec`, and a pure
`CleanIntegrationPlan { final_oid, local_update, push_needed }` or one typed
non-mutating boundary.

- [ ] Write failing table tests for the exact primary/context fetch strings,
  exact ordinary push strings, invalid/reserved names, no wildcard/force/delete
  push, and no caller-controlled ref text.
- [ ] Run the focused library test and confirm the new contract is absent or
  failing before implementation.
- [ ] Write failing graph tables for equal, local-behind, local-ahead, context
  remote-behind/ahead, primary-behind/ahead, missing primary, first publication,
  remote-deleted, history-unknown, and every divergent pair. Assert virtual
  evaluation returns no intermediate update when the primary relation diverges.
- [ ] Implement the pure constructors and graph evaluator with commit ancestry
  supplied by a narrow trait/closure, not a repository or network handle.
- [ ] Run focused ref/graph tests to green and then the relevant library suite.
- [ ] Commit the coherent checkpoint, for example:

  ```sh
  git add src/repository/remote/{refs,state,mod}.rs src/repository.rs
  git commit -m "feat: define clean synchronization contracts"
  ```

- [ ] Add a ticket comment with the exact interface/graph evidence and an
  independent-review result before starting Task 2.

## Task 2: Extend Durable Synchronization Replay Safely

**Files:** modify `src/repository/remote/{state,reservation,observation}.rs`
and tests; modify `src/repository/recovery.rs` only when Cycle 04's recovery
adapter needs the new action checkpoints.

**Consumes:** Task 1 target/ref/graph types and Cycle 04 migration,
reservations, safe points, current-batch persistence, and publication evidence.

**Produces:** Backward-compatible synchronization action phases for fetch
observed, local fast-forwarded, push prepared, push verified, and discovery
pending; generic ownership checks; exact-ID terminal/restart behavior; and one
shared complete-advertisement persistence path.

- [ ] Write migration tests opening a Cycle 04 database with completed,
  interrupted, yielded, cancelled, and failed observation records. Assert every
  old row decodes unchanged and no migration infers a push or publication.
- [ ] Write failure-injection tests before/after complete Fetch observation,
  before/after local update, before push, after push return, after post-push
  observation, and during discovery state persistence. Assert terminal replay,
  explicit restart fencing, operation/target mismatch rejection, and no second
  active reservation.
- [ ] Run the focused tests red before changing production state code.
- [ ] Extend the existing record rather than adding a journal. Persist only
  validated names, IDs, OIDs, fixed categories, safe point, and timestamps.
  Refactor Cycle 04 batch commit only enough that a synchronization reservation
  can atomically record a complete final Fetch observation without pretending it
  was a poll.
- [ ] Make recovery inspect real refs/worktrees before deciding a recorded
  `push_prepared` effect is incomplete. A post-push ambiguity first requires
  observation; it cannot schedule an automatic repeat push.
- [ ] Run focused migration/reservation/recovery tests to green and a privacy
  scan over rows, WAL/journal/backups, `Debug`, and `Display` sentinels.
- [ ] Commit and ticket-comment the reviewed checkpoint, for example
  `feat: persist synchronization recovery boundaries`.

## Task 3: Make Scoped Transport Support Exact Fetch And Verified Push

**Files:** modify `src/repository/transport/{operation,remote}.rs`; extend
transport tests and only the narrow remote-facing test helpers needed for
callback/progress coverage.

**Consumes:** Task 1 exact mappings; Cycle 03 prepared endpoint, selected key,
host-pin, session credential, and callback contract; Task 2 safe-point hook.

**Produces:** Crate-private composed operations that (a) Fetch-direction list,
download only supplied exact validated mappings with `FETCH_HEAD` disabled, and
list again; (b) Push-direction list, ordinary push, and list again; and (c)
invoke caller-controlled safe-point checks only outside libgit2 calls.

- [ ] Add failing tests that reject a transfer mapping with force/wildcard/delete
  syntax or mismatched direction, retain one selected-key submission and host
  trust on every reconnect, and leave `FETCH_HEAD` unchanged.
- [ ] Add tests for an explicit Push-direction endpoint distinct from Fetch:
  its OID is evidence for publication but cannot overwrite Fetch tracking
  evidence. Add hostile endpoint/server-message sentinels to output scans.
- [ ] Run focused transport tests red.
- [ ] Implement the smallest scoped adapter extension. It may reconnect only
  through existing Cycle 03 policy and must recheck selected key/source,
  configured endpoint, and host trust before each transfer. Do not expose raw
  `git2::Remote`, generic transfer, URL, callbacks, or secret material.
- [ ] Ensure push reference-status rejection maps to the existing fixed
  `PushRejected`, and a completed library call is followed by caller-owned
  advertisement rather than treated as verified publication.
- [ ] Run focused transport tests green; run `devenv shell -- cargo test --locked
  --test ssh_transport` when that target is available after rebasing Cycle 04.
- [ ] Commit and ticket-comment the independently reviewed transport checkpoint,
  for example `feat: support exact synchronization transfers`.

## Task 4: Orchestrate Preflight, Fetch, One Fast-Forward, And Publication

**Files:** create `src/repository/remote/sync.rs`; modify remote module/export
wiring, `state.rs`, `reservation.rs`, and `repository.rs` only for the Task 1
public contract; add targeted unit/service tests.

**Consumes:** Tasks 1–3 and existing repository/common-Git lease, worktree
inspection, safe checkout, discovery, key/session, and recovery APIs.

**Produces:** `RepositoryService::synchronize_remote<P: SessionCredentialProvider>(
request: SynchronizeRemoteRequest, session: &mut SessionCredentials<P>
) -> Result<SynchronizationResult, SynchronizationError>`, with no front-end
behavior.

- [ ] Add failing service tests for no configured remote (`PublishPending` and
  no transport/reservation); primary/context wrong worktree, wrong branch,
  dirty, and conflicted preflight; missing primary; missing/deleted/unknown
  context; and a merge-required graph. Assert no status/index/ref/worktree
  bytes change in every rejection case.
- [ ] Add a failure-injection test where the context could fast-forward to its
  remote but then primary diverges. Assert virtual planning returns
  `MergeRequired` and leaves the context at its original OID.
- [ ] Run the focused service tests red.
- [ ] Implement the operation in ordered helpers: configuration/target
  preflight; reserve; Fetch-direction list/download/list; atomically persist
  final observation; lease-protected fresh graph/status check; at most one
  expected-old-OID fast-forward plus safe checkout; Push-direction
  advertisement/push/advertisement; durable classification; released-lease
  discovery handoff. For no configured remote, skip reservation/transport and
  invoke the existing `RefreshRepositoryRequest` with the same operation ID.
- [ ] At every durable boundary honor Cycle 04 cancellation/yield without
  interrupting a libgit2 call or a transaction. Re-open and re-observe after
  each network phase; never use a stale ref/cleanliness decision.
- [ ] Record `push_prepared` before the network call. On post-push ambiguity,
  reconcile observed target OID on explicit restart; never convert an error into
  a blind duplicate push or an unqualified success.
- [ ] Run the focused service tests green and `devenv shell -- cargo test
  --locked --lib`.
- [ ] Commit and ticket-comment the reviewed checkpoint, for example
  `feat: synchronize clean remote branches`.

## Task 5: Prove Real Remote Outcomes, Replay, And Index Handoff

**Files:** create `tests/remote_synchronization.rs`; extend
`tests/support/{mod,ssh_remote}.rs` only for owned fixture controls; update
`.github/workflows/build.yml`.

**Consumes:** Completed Task 4 public service and the existing authenticated
SSH fixture/custom transport test initialization.

**Produces:** Real two-clone acceptance evidence and a focused native CI target.

- [ ] Add shell-free fixture setup for a fetch source, optional distinct push
  destination, two authenticated local clones, controlled receive-pack
  disconnect/rejection, and captured refs/`FETCH_HEAD`/worktree snapshots.
- [ ] Write red real-SSH cases for primary fast-forward/current/local-ahead
  publish; context first publication/fast-forward/current/local-ahead publish;
  ordinary non-fast-forward rejection; and fetch/push endpoints with distinct
  OID evidence.
- [ ] Add red preservation/recovery cases for dirty/conflicted primary,
  dirty context, remote deleted/history unknown, both divergence classes,
  remote change during fetch, post-push disconnect, persistence failure after
  server acceptance, and exact-ID resume. Assert every retry performs only its
  unproved effect and produces no duplicate remote ref/commit/checkpoint.
- [ ] Add red index-handoff cases: inject refresh failure after a verified
  `Published`/`AlreadyCurrent`, then replay the same ID and assert it refreshes
  without a fetch, fast-forward, or push. Add full secret/URL/server-text
  sentinel scans for records, diagnostics, WAL/journal/backups, and assertions.
- [ ] Implement only fixture capabilities required by those tests, then run the
  focused target red-to-green through the custom pre-thread test host where the
  SSH fixture requires it.
- [ ] Add `--test remote_synchronization` to every existing native headless
  contract job. Do not call the workflow change proof; actual native runs remain
  pending until authorized publication.
- [ ] Commit and ticket-comment the reviewed checkpoint, for example
  `test: prove clean synchronization recovery`.

## Task 6: Verify, Review, And Handoff

- [ ] Run the final local gate from this ticket worktree:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  git diff --check
  ```

- [ ] Record focused test counts/results; exact transfer/ref/worktree
  preservation; replay and index-pending evidence; privacy-scan result; source
  and lockfile changes; and native CI links. Label unavailable native evidence
  as pending, never passing.
- [ ] Request independent task/whole-branch review, resolve findings, rerun
  affected and final checks, and add a review-ready ticket comment.
- [ ] Keep the ticket open until code review or PR approval. Push/PR, merge,
  ticket closure, and worktree cleanup each require later authorization.

## Plan Self-Review

The plan maps all Cycle/design requirements to tasks: pure exact mappings and
virtual graph decision (Task 1); migration, durable state, and replay (Task 2);
selected-key exact transfer plus endpoint distinction (Task 3); clean mutation
and public outcomes (Task 4); real fixture, privacy, index-only replay, and
native feasibility (Task 5); and full verification/review/lifecycle (Task 6).
Names introduced in later tasks are defined by Tasks 1–4, and every review
focus item has a named owning test task. The plan deliberately contains no
merge, polling, materialization, comment, promotion, closure, UI, or scheduler
task, preserving downstream Cycle boundaries.

Task 4 compatibility clarification: the approved locked-ref expected-old
transition runs safe checkout against old HEAD before the single ref commit,
with LocalPrepared durable before either effect and fresh proof afterward.
See the design's Task 4 rulings for mismatch recovery, typed Push object download,
and frozen local-only refresh identity. These do not authorize Task 5 scope.
