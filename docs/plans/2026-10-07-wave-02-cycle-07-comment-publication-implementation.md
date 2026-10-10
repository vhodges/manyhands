---
title: "Wave 02 Cycle 07 Comment Publication Implementation Plan"
date: 2026-10-07
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4B2JBSQSBKQB0GJHMKK6W05"
---

# Comment Publication Implementation Plan

> The user approved the [Cycle](../Cycles/wave-02-cycle-07-comment-publication.md),
> [design](2026-10-07-wave-02-cycle-07-comment-publication-design.md) and this plan
> on 2026-10-07. Do not implement until explicit implementation authorization
> exists and the dependency gates pass. Amended 2026-10-09 after Cycle 06 merged
> (`5e4fad6`): that gate is met; ticket `01M4GD0KKXW684QBA49F6EX3WE` (save must
> leave the Git index current) is now a prerequisite. Amendments are marked.
> Then use `implementing-a-cycle`, repeat ticket preflight and work task by task
> in this ticket's existing worktree. Proposed execution is direct, sequential
> implementation with task checkpoints and code-review handoff; delegation is
> not assumed or authorized by this plan.

**Goal:** One authoritative comment checkpoint followed immediately by safe
context synchronization, with saved-local/publication/indexing evidence and
body-free, non-duplicating publication retry.

**Ticket:** `01K7F6H9J2N4Q6S8V0X2Z4B6DF`.

**Architecture:** Extract the existing local submit into a private checkpoint
helper. Bind one persistent synchronization child to the caller's original
submit ID, prove the original commit, finish local discovery, then delegate all
remote behavior to Cycles 05/06. Keep publication and index state separate;
retry only unproved effects. No second reservation, new merge engine or journal.

**Stack:** Rust 2024, locked git2/libgit2 SSH transport, rusqlite registry,
existing session/host trust and recovery APIs, Devenv, custom pre-thread
SSH/privacy harness, five-target manual native workflow.

**Status:** Approved by the user on 2026-10-07; approval-state updates and the
planning commit are authorized. Implementation, publication, merge, closure
and cleanup are not authorized. No Rust validation/implementation or native
evidence is claimed. Planning evidence and decision audit live in the
[execution ledger](2026-10-07-wave-02-cycle-07-comment-publication-execution.md).

## Global Constraints

- Use the existing ticket worktree/branch; preserve main and other worktrees.
  Fresh fetch, rebase ticket onto current main, and ancestry verification precede
  implementation. Approval does not authorize push/PR/merge/closure/cleanup.
- Domain logic stays in the library. No GPUI dependency, UI, CLI grammar,
  scheduler, system-Git runtime dependency or new transport/merge backend.
- Preserve canonical comment paths, parent semantics, immutable creation time,
  scoped checkpoint and expected-path checks. Persist no body/hash/draft,
  credential/key content, endpoint URL or arbitrary backend error text.
- Bind the child before writes and reject operation/root/target/ID collisions
  transactionally. Do not weaken reservation or pending-local recovery guards.
- Known checkpoint means saved-local, even if transport, receipt persistence
  or discovery fails. Unknown effects mean explicit recovery, not a fresh post.
- No remote means no prompt/network/reservation and no started terminal child;
  the same pending receipt can start its child after a remote is configured.
- With remote, immediately call only `SynchronizationTarget::Context`; entire
  committed context history can publish, but unsaved buffers are not checkpointed.
- Reuse selected-key session/host approval, exact refs, ordinary push,
  expected-OID checks, explicit restart and Cycle 06 conflict handling.
- Never hold the short lease across SSH, prompts or discovery. Inherit per-call
  transport budgets and safe-point cancellation; no total deadline or retry loop.
- Amended 2026-10-09: add no Git-index refresh in production or fixtures; the
  index ticket is a prerequisite and the acceptance tests prove save →
  synchronize unaided. New recovery tables follow the Cycle 06 registry rules
  (all-or-nothing migration, startup validation, table inventory test).
- Every local/agent Rust command uses `devenv shell -- cargo ...`. A new test
  declaration does not require Cargo dependencies; no lockfile churn expected.

## File Map And Task Order

| Files | Ownership |
| --- | --- |
| `src/repository/comment_publication.rs` (new) | Compound entry points, receipt/outcome mapping, body-free retry and bound cancellation adapter |
| `src/repository.rs` | Private local checkpoint extraction, accurate checkpoint evidence, module/exports, test-only identity seam |
| `src/repository/recovery.rs` | Binding migration, identity uniqueness, original receipt observation/reconciliation; reuse local operation/index recovery |
| `src/repository/comment_publication_tests.rs` (new, private unit module) | Binding/mapping/replay/fault tests with narrow test hooks |
| `tests/local_authoring.rs`, `tests/recovery_foundation_gate.rs` | Migrate compound signature; retain Wave 01 local preservation and crash regressions |
| `tests/comment_publication.rs` (new) | Public headless local-only and real authenticated-SSH acceptance |
| `tests/support/{ssh_harness,ssh_remote,ssh_privacy}.rs` | Reuse existing helpers; add only deterministic owned fault/transport-counter controls missing from them |
| `Cargo.toml` | Declare `comment_publication` custom-host integration target with `harness = false` |
| `.github/workflows/build.yml` | Include focused target on all existing native jobs; keep manual-only triggers |
| `docs/research/wave-03-api-audit.md` | Update audited submission/receipt/retry API and integration dependency |
| Cycle/design/plan/execution ledger and canonical ticket/comments | Record progress, rulings, checks, blockers, review and approval states |

Change remote modules only if Task 0 proves a narrow missing adapter is needed
within the approved contract; document it, test it, and do not reimplement
Cycles 05/06. Task order is baseline → API/local extraction → binding/receipt →
compound orchestration → replay/recovery → real integration → verification/review.

## Task 0: Refresh Base, Verify Dependencies And Baseline

- [ ] Confirm explicit implementation authorization and artifact approval.
- [ ] Inspect status in main and this worktree; preserve unrelated edits. Fetch
  `origin main`, record fetched base and old ticket HEAD, rebase **this ticket**
  onto that base, verify ancestry/status and record new HEAD/conflicts.
- [ ] Read rebased `AGENTS.md`, approved Cycle 06 Cycle/design/plan, review and
  execution evidence. Inspect actual merged `synchronize_remote` and resolution
  contract. Record exact names, restart/cancel semantics and conflict fixture
  cases. If absent or contract-changing, stop before Rust edits.
- [ ] Amended 2026-10-09: confirm ticket `01M4GD0KKXW684QBA49F6EX3WE` is merged
  to the base and that a public save followed by `synchronize_remote` succeeds
  with no index refresh; if not, stop before Rust edits. Record the merged
  Cycle 06 contract from the design's "Update at base `5e4fad6`" section and
  correct it against the code if it has moved again.
- [ ] Re-scan all `submit_comment` callers, tests, remote-state migrations and
  pending-local guards. Known callers at `5e4fad6`: `tests/local_authoring.rs`,
  `tests/recovery_foundation_gate.rs`, `tests/remote_merge_recovery.rs`,
  `src/repository/remote/sync_tests.rs`; the Wave 03 read boundary and
  `schemas/v1/operation.schema.json` name the `submit_comment` action. Resolve name-only differences in the ledger; behavioral
  changes require approval of affected artifacts.
- [ ] Run baseline and record existing failures separately:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

- [ ] Add ticket checkpoint with approval/base/dependency evidence and baseline
  logs/results. Establish task progress in the existing execution ledger.

## Task 1: Define Compound API And Preserve Local Checkpoint Behavior

**Files:** `src/repository.rs`, new `src/repository/comment_publication.rs`
and `comment_publication_tests.rs`, `tests/local_authoring.rs`,
`tests/recovery_foundation_gate.rs`, `docs/research/wave-03-api-audit.md`.

**Consumes:** Existing `SubmitCommentRequest`, `AuthoringTarget`, local recovery,
`LocalCheckpoint`, context/parent/schema/identity and owned-path logic.
**Produces:** Proposed public compound request/receipt/result/error types,
private local checkpoint helper and in-repository caller migration.

- [ ] Add failing type/outcome tests for identity-required, known saved checkpoint,
  uncommitted retained file, separate publication/indexing, stable root/item/
  comment/parent/action identity, and redacted formatting. The request has no
  URL/refspec/worktree/key/passphrase/force control.
- [ ] Retain/add local regression tests for document and ticket roots and nested
  replies; absent/cross-item parent; invalid canonical content; create-intent
  target; missing identity; occupied/unsafe destination; exact-ID replay with
  unchanged bytes/time; unrelated live index/staged/unstaged/untracked/deleted/
  conflicted paths; no empty checkpoint.
- [ ] Establish red evidence using focused tests before production changes:

  ```sh
  devenv shell -- cargo test --locked --lib comment_publication
  devenv shell -- cargo test --locked --test local_authoring comment_
  devenv shell -- cargo test --locked --test recovery_foundation_gate
  ```

- [ ] Extract, not rewrite, current `submit_comment_with_effective_config`
  behavior. The helper carries explicit canonical comment identity and original
  OID evidence; public normal submit can no longer return `SyncDeferred`.
  Preserve the identity-config test seam without exposing a public production
  local-only submission bypass. Existing configured-remote local tests now
  expect saved/pending recovery or use crate-private unit coverage of the helper.
- [ ] Update API-audit description and consumers atomically. Keep generic saves
  and canonical schema unchanged. Do not claim remote support before Task 3.
- [ ] Run focused regressions green; record intermediate limitations honestly.
  Review diff, commit a coherent checkpoint (for example
  `refactor: isolate authoritative comment checkpoint`), and comment on ticket.

## Task 2: Persist One Child Binding And Original Checkpoint Receipt

**Files:** `src/repository/recovery.rs`, `src/repository.rs`,
`src/repository/comment_publication{,_tests}.rs`.

**Consumes:** Task 1 local identity/OID contract and existing registry migration.
**Produces:** Additive binding migration, atomic parent/child identity, original
receipt persistence and safe commit-before-receipt reconciliation.

- [ ] Write failing migration tests using current databases with completed and
  pending comments, refresh records, interrupted remote records and no new
  bindings. Assert old rows decode unchanged and migration contacts no remote,
  changes no Git/file state and infers no publication/checkpoint.
- [ ] Write transactional collision tests: same parent changed root/item/kind/
  comment/parent; same comment different action; child collision with local or
  remote ID; same-ID concurrent services. Exactly one binding/child can win;
  the loser receives mismatch/busy without another canonical write.
- [ ] Add fault/reopen tests before binding commit, after binding/before write,
  after file/before checkpoint, after Git commit/before receipt persistence,
  after receipt/before discovery and after local discovery/before child start.
  Assert one timestamp, exact bytes, one original checkpoint and one child ID.
- [ ] Run `devenv shell -- cargo test --locked --lib comment_publication` red.
- [ ] Implement `comment_publication_bindings` as correlation/receipt metadata
  in the existing registry. Amended 2026-10-09: create it all-or-nothing
  in the existing migration with startup validation, add it to the table
  inventory in `tests/repository_enablement.rs`, and check child-ID uniqueness
  against remote operation records as well as local ones. Keep the original submit ID for local recovery and
  a distinct generated child ID; atomically enforce uniqueness against existing
  identity tables. Do not store an independent remote phase machine.
- [ ] Prove checkpoint from actual commit/tree/path/diff and ancestry relative
  to the recorded pre-checkpoint OID; do not infer from subject/file alone.
  Store original checkpoint OID/time only after proof. Preserve receipt across
  refresh failure and do not replace it with merge/push OIDs.
- [ ] Add negative tests for altered/deleted file, branch moved externally,
  unreachable/missing commit, competing plausible checkpoint and lost/corrupt
  binding. Unknown authority stops without overwrite/checkpoint. Legacy binding
  absence reports recovery; ordinary deliberate sync remains available.
- [ ] Scan binding/recovery rows, database/WAL/journal/backups and public errors
  using body/URL/key/passphrase/server sentinels. Only canonical Markdown/Git
  blobs may contain comment text. Add no body-derived persistent fingerprint.
- [ ] Run focused tests green and existing recovery regressions. Commit/comment
  the reviewed checkpoint (for example `feat: bind comment publication retries`).

## Task 3: Compose Checkpoint, Discovery And Immediate Context Sync

**Files:** `src/repository/comment_publication{,_tests}.rs`,
`src/repository.rs`; existing remote adapter only if narrowly required.

**Consumes:** Tasks 1–2, current Cycles 05/06 APIs and caller-owned session.
**Produces:** Working compound submit with known-local outcomes and no deferred
publication handoff for normal callers.

- [ ] Write failing orchestration tests proving ordering: validation/identity
  before write, checkpoint before prompt/transport, local discovery before
  remote child and no short lease held at prompt/transport/scan boundaries.
- [ ] No-remote tests assert zero credential prompts, network contacts and
  remote reservations, including unrelated dirty state. Binding allocation is
  allowed but the child has not started/frozen a local-only sync result.
- [ ] Remote tests assert exact derived context target and persisted child ID,
  no primary action, fixed error categories and receipt on busy/poll-yield,
  dirty/conflicted context, missing key, cancelled unlock, trust failure,
  remote deletion and changed endpoint. No implicit item save or auto-republish.
- [ ] Run the focused library tests red, implement orchestration by delegation,
  then run them green. Finish local pending handoff before invoking remote;
  do not exempt it from `require_no_pending_local`.
- [ ] Map `SynchronizationResult::{Complete, IndexPending}` independently from
  local indexing. Prove original checkpoint ancestry and original comment blob
  at the context before delegation and at the verified authority OID afterward.
  Add negative tests for externally moved branch and removed/replaced comment:
  successful branch sync alone cannot claim this comment was published.
  Preserve `Published`/`AlreadyCurrent` authority and original checkpoint OID;
  catch post-checkpoint failures as saved/pending or saved/index-pending rather
  than resubmission errors.
- [ ] Amended 2026-10-09: map each Cycle 06 typed recovery by name to
  saved/pending — `Busy` (including another context's pending conflict),
  `ConflictPending`, `ExternalResolutionRequired`, `IdentityRequired` with its
  opaque `ExpectedConfiguration`, `PushRejected`, `RemoteContextDeleted`,
  `ExternalChange`, `RecoveryRequired` — with one test per category. Forward
  `confirmed_identity` to the child only; test that an identity removed after
  the checkpoint yields saved/pending `IdentityRequired` and that a retry
  carrying the returned value completes. Relay `Published` versus
  `AlreadyCurrent` as the child reports it.
- [ ] Use the same session/HostApproval as existing transport. Honor per-call
  budgets and existing child safe points; no retries inside the compound call.
- [ ] Run library plus local-authoring/recovery regressions. Commit/comment
  reviewed checkpoint (for example `feat: publish checkpointed comments`).

## Task 4: Implement Body-Free Retry, Cancellation And Recovery Mapping

**Files:** `src/repository/comment_publication{,_tests}.rs`,
`src/repository/recovery.rs`, `src/repository.rs`.

**Consumes:** Bound original receipt, local handoff state, delegated child state.
**Produces:** `retry_comment_publication`, binding-based cancellation lookup,
terminal/index-only replay and non-duplicating pending publication recovery.

- [ ] Add failing tests for same-submit replay (including changed body/metadata
  rejection against original committed blob) and body-free publication retry.
  Receiptless retry must request original submission/reconciliation, not a body
  substitute or new checkpoint. Unknown binding returns typed recovery.
- [ ] Test no-remote submit followed by configured remote: same unused child
  starts once. Completed publication replay returns historical evidence; later
  endpoint/ref changes do not silently republish under that action.
- [ ] Test interrupted child needs explicit restart; duplicate/retry cannot steal
  a reservation. Map parent cancellation to the existing child safe-point API;
  cancellation before child start has no remote effect and never undoes the
  checkpoint. Follow Cycle 06's exact cancelled/conflict restart contract.
  Amended 2026-10-09, that contract is: a cancel with a pending conflict or
  owned resolution leaves the child `interrupted` and restartable; a cancel
  during a resolve does not outlive it; `cancel_remote_operation` has no effect
  on a child parked after a released conflict; a terminally cancelled child
  cannot be reused, so the comment stays saved-local and later publishes through
   an ordinary context synchronization. Test each.
   **Scope exception approved 2026-10-10:** repair the shared recoverable-cancel
   predicate for an applied but unreleased owned resolution using existing
   attempt/release evidence. Add applied-clean-merge and released-resolution
   terminal negatives, plus the public SSH regression. Other delegation and
   ownership boundaries remain as approved.
- [ ] Test local index failure repair then sync, remote index failure then
  discovery-only retry, saved-local cache/receipt failure reporting, and retained
  conflict recovery. Assert original OID/comment/time remain fixed.
- [ ] Run focused library tests red; implement lookup/reconciliation/mapping
  without bypassing remote ownership or implementing resolution/merge policy.
- [ ] Run focused tests green, full library/local-authoring/foundation regression
  targets, and review negative states. Commit/comment the reviewed checkpoint
  (for example `feat: resume comment publication without resubmission`).

## Task 5: Prove Public Acceptance With Real SSH And Fault Injection

**Files:** new `tests/comment_publication.rs`, `Cargo.toml`,
`tests/support/{ssh_harness,ssh_remote,ssh_privacy}.rs` only as needed,
`.github/workflows/build.yml`.

**Consumes:** Completed compound/retry APIs and Cycle 06 real recovery fixture.
**Produces:** One focused custom-host acceptance target and native workflow wiring.

- [ ] Declare `[[test]] name = "comment_publication", harness = false` and reuse
  existing pre-thread initialization/output privacy architecture. Test fixture
  readiness/teardown and fault controls deterministically; use test-only counters
  and barriers, not sleeps or developer keys/config/data.
- [ ] Add root/reply tests across document and ticket contexts: initial remote
  publication, already-current evidence when another collaborator published,
  local-only discovery and later configured-remote retry. Assert original path,
  timestamp, local commit count, child count, discovery thread membership,
  exact remote OID ancestry and original canonical blob at the published tree.
  Include external branch-move/comment-removal/ID-collision negative cases;
  do not equate transport success with publication of this receipt.
- [ ] Add tests that prior checkpointed context work publishes too while unsaved
  item text, unrelated worktree state, primary ref/worktree, unrelated tracking
  refs and `FETCH_HEAD` remain unchanged. Dirty sync reports saved/pending.
- [ ] Amended 2026-10-09: model the target on `tests/remote_merge_recovery.rs`
  (two clones, one-round bcrypt fixture key, `race_update`, output-control
  children). Use no fixture index refresh after any save or submit. Keep fixture
  directory names short and build no context worktree inside a nested
  output-control child: Windows has about 40 characters of path headroom and
  comment files are the longest canonical paths. Add one case where another
  context holds a pending conflict and the submission is saved/pending `Busy`.
- [ ] Add real two-clone nonconflicting divergence and conflicting history tests. Resolve through `resolve_synchronization`
  keyed by the receipt's bound child ID, and separately by an exact external
  two-parent repair.
  Invoke **Cycle 06's** explicit resolution/checkpoint boundary; retry publishes
  the same comment with permitted merge/resolution commits only, no additional
  `Checkpoint comment` commit or ID. Preserve both histories/context markers.
- [ ] Add selected/wrong key, locked/cancelled session, unapproved/changed host,
  offline disconnect, normal push rejection, distinct Push endpoint and remote
  deletion tests. Exercise representative real failures; pure cases alone are
  not selected-key or backend proof.
- [ ] Add close/reopen failure injection at every local/remote handoff, accepted
  push before acknowledgement/persistence, conflict checkpoint completion and
  local/remote discovery failure. Record transport/effect counters. Terminal
  index retry must perform no fetch/push/merge/checkpoint; ambiguous push retry
  must observe before deciding any new push is safe.
- [ ] Scan all approved persistence surfaces and redacted fixture/error output.
  Production timeout claims rely on inherited transport tests; watchdog expiry
  fails the test, not a successful timeout proof.
- [ ] Run red-to-green acceptance:

  ```sh
  devenv shell -- cargo test --locked --test comment_publication
  devenv shell -- cargo test --locked --test remote_synchronization
  devenv shell -- cargo test --locked --test ssh_transport
  devenv shell -- cargo test --locked --test local_authoring
  devenv shell -- cargo test --locked --test recovery_foundation_gate
  ```

- [ ] Add `--test comment_publication` to the existing native headless-test
  command on all five targets. Leave `workflow_dispatch` and automatic CI
  disabled; no dispatch/enable/investigation without separate authorization.
  Record native execution as pending, not passed. No lockfile update unless
  actual dependency inputs changed.
- [ ] Commit/comment reviewed test checkpoint (for example
  `test: prove comment publication and recovery`).

## Task 6: Full Verification, Code Review And Approval Handoff

- [ ] Run all required local checks from this ticket worktree:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  git diff --check
  ```

- [ ] Record focused/full test counts, base/HEAD/lockfile hashes, checkpoint and
  child invariants, preservation snapshots, privacy scan and timing limitations.
  No desktop GUI or future CLI grammar is required for headless acceptance;
  desktop smoke needs an active display if startup paths changed.
- [ ] Record actual authorized native results on Linux x86_64/aarch64, Windows
  x86_64/aarch64 and macOS aarch64, or explicitly pending/unavailable/owner-deferred
  evidence. Test wiring and Linux fixtures do not prove native ABI behavior.
- [ ] Obtain code review, resolve findings, rerun affected/final checks. Add
  review-ready ticket comment and update ledger/artifact evidence honestly.
- [ ] Keep the ticket open until code-review or PR approval. At separately
  authorized delivery, inspect fetched remote ticket history before push; never
  force-push without explicit authority. Close only after approval, normally in
  the final authorized pre-merge checkpoint. Merge and cleanup are separate gates.

## Plan Self-Review

Acceptance-to-task coverage is explicit in the Cycle table. API/local preservation
belongs to Task 1; durable identity and original checkpoint proof to Task 2;
immediate ordered delegation to Task 3; index/cancel/retry semantics to Task 4;
real SSH, merge/conflict integration, crash/privacy/native wiring to Task 5; and
final evidence/review/lifecycle to Task 6. Task 0 gates the absent Cycle 06
contract instead of inventing future APIs. The skill-based decision matrix and
rulings are in the execution ledger. No unresolved product question was found;
Cycle 06 and native execution were explicit evidence/dependency gaps; amended
2026-10-09, Cycle 06 is merged and the index ticket is the remaining dependency.
