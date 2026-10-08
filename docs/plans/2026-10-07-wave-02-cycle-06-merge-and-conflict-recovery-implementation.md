---
title: "Wave 02 Cycle 06 Merge And Conflict Recovery Implementation Plan"
date: 2026-10-07
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4AZH9MXRM4MK3JX0WDPS4QJ"
---

# Merge And Conflict Recovery Implementation Plan

> **Execution gate:** The user approved the Cycle, design and this plan on
> 2026-10-07 and authorized the local planning commit. Explicit implementation
> authorization is still required. When authorized, load `implementing-a-cycle`,
> repeat worktree preflight, use the authorized execution method, and record
> each checkpoint on ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DE`.

**Goal:** Add ordered non-rebase synchronization merges, preserved real conflicts,
explicit owned-canonical two-parent resolution, and effect-aware recovery without
weakening Cycle 05 publication/discovery or prior authoring safety.

**Architecture:** Keep the existing clean virtual fast path and exact SSH
orchestration. Add a private merge/recovery component and ordered child evidence
inside the existing reservation journal. Prepare outside the lease, fence and
re-observe before each local transition, retain conflict/index/worktree state,
and reuse prepared commit OIDs on replay. Resolve only the complete eligible
canonical set; require external tools for other sets and observe their real
merge before deliberate resume.

**Stack:** Rust 2024, locked git2/libgit2/libssh2, rusqlite, existing canonical
validators and guarded file helpers, selected-key SSH adapter, disposable Rust
SSH fixture/privacy harness, Devenv, manual-only five-target native CI.
No new dependency, Cargo.lock change or Devenv input change is planned.

**Specs:** [Cycle](../Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md),
[design](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md),
[ledger](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-execution.md).
The Cycle, design and this plan are approved; the ledger records the ongoing
lifecycle. A1–A9 below refer to the Cycle acceptance IDs.

## Global Constraints And Execution Shape

- Use only `.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DE/` on
  `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DE`. Preserve other checkouts.
- Recommended execution: sequential direct tasks in this worktree, focused
  failing/passing tests, self-review at each seam and whole-change review at
  the end. User may select authorized delegation later; this request does not.
- Use one existing reservation/journal and the short common-Git lease. No
  network, full scan, user prompt or read-only merge preparation under the lease.
- Owner concurrency amendment (2026-10-08): cooperative autonomous writers use
  Manyhands API/CLI and the existing guards. Concurrent direct mutation bypassing
  coordination of affected canonical paths/ancestors, target validation state,
  Git metadata, or private staging during bounded apply/reconciliation is
  unsupported. Reads/unrelated safe work remain allowed; external repair requires
  fresh observation. Shared refs still need common-Git coordination despite
  worktree isolation. Do not claim race-free expected-inode namespace operations
  or continuous foreign-lock exclusion under arbitrary substitution. Still reject
  observed stale state and preserve ambiguous recovery/foreign locks.
- Separate owner recovery amendment (2026-10-08): ambiguous live locks created
  internally by libgit2 after a crash may require operator intervention. Preserve
  them/effects; no automatic ownership inference/deletion by age, PID, or content.
  Operator quiescence and verified stale-lock handling precede identical retry,
  exact actual-state revalidation, and recorded-candidate reuse. Other owned
  resolution effects still recover automatically. This is not authority for
  wider manual recovery or platform/dependency changes. Owner separately approved
  the [Task 4 protocol redesign](2026-10-08-wave-02-cycle-06-task-04-resolution-protocol-amendment.md)
  and sequential local implementation on 2026-10-08; retain all review/gates.
- Preserve exact fetch mappings, no `FETCH_HEAD` update/pruning, independent
  Push endpoint checks, ordinary non-force publication and authority/index replay.
- Never rebase, choose a side, force checkout/push, stash/reset/discard, abort a
  merge, materialize, delete refs/worktrees, poll, submit comments, promote/close
  items, add UI/CLI grammar or start background scheduling.
- Only the divergence path makes merge commits; clean fast-path regressions stay
  unchanged. Context integrates fetched context then primary; primary has one
  incoming stage. No empty or duplicate branch checkpoint on retry.
- Canonical resolution commits the recorded merged tree with TWO parents, not
  the existing one-parent save helper. Unrelated files remain untouched; only
  owned supported conflicts are written/staged from caller bytes.
- Owner answered the self-review question on 2026-10-07: **whole-merge external
  recovery for mixed canonical/unsupported conflicts**. No canonical writes to
  mixed sets; external tools commit the whole merge, then deliberate resume.
  Accepted cost: canonical portions of mixed sets cannot be resolved in-process.
  This scoped decision is not approval of the whole plan or implementation.
- Persist IDs, validated identities, OIDs, modes, digests and redacted categories,
  never bodies, credentials, endpoint URLs, server messages or arbitrary paths.
- Run all local Rust commands through `devenv shell -- cargo ...`. CI native
  runner Cargo is the existing approved exception; leave CI manual-only.
- The design's decision audit/ledger rulings bind these tasks. Characterization
  failure is a blocker to amend/review, not authority to weaken preservation.

## File Map

| Files | Change / ownership |
| --- | --- |
| `src/repository/remote/merge.rs` (new) | Integration step classification/preparation, safe merge application, actual conflict inspection, resolution validation/writes/checkpoint, actual-state reconciliation helpers. Split private helpers only if size requires it. |
| `src/repository/remote/merge_tests.rs` (new) | Pure graph/eligibility and real local Git/index/failure tests, wired privately from `merge.rs`. |
| `src/repository/remote/sync.rs`, `sync_tests.rs` | Divergence routing, compatible typed recovery/identity categories, same-ID resume ordering, publication/discovery handoff and regression assertions. |
| `src/repository/remote/refs.rs`, `refs_tests.rs` | Keep `plan_clean_integration`; add step classification/planning only where shared, preserving negative clean-plan tests. |
| `src/repository/remote/state.rs`, `state_tests.rs` | Transactional per-stage/attempt/path migration, strict decoders/audit, immutable intent, bounded queries and privacy. |
| `src/repository/remote/reservation.rs`, `reservation_tests.rs` | Owned stage/attempt transitions, new safe points, conflict ownership release/reacquire, fenced reconciliation and replay. |
| `src/repository/remote/mod.rs`, `src/repository.rs` | Private wiring/public exports, reuse identity/owned-path helpers, add authoring guard and test seams; do not introduce desktop coupling. |
| `src/repository/recovery.rs` | Pending-local/remote identity coexistence and authoring recovery guard as needed; keep cache-loss preservation. |
| `tests/remote_merge_recovery.rs` (new) | Public headless service and two-clone authenticated SSH acceptance/fault/privacy journeys, using existing shell-free harness. |
| `tests/remote_synchronization.rs` | Replace only public-service divergence expectations now intentionally handled by Cycle 06; preserve clean-only planner tests and all preservation/push/ignored-file cases. |
| `tests/support/{ssh_harness,ssh_remote,ssh_server,ssh_privacy}.rs` | Minimal reusable merge fixtures, effect counts and fault/privacy probes; no production helper processes. |
| `tests/local_authoring.rs`, `tests/recovery_foundation_gate.rs` | Prove ordinary save/comment checkpoint cannot complete/stage an outstanding synchronization merge; retain Wave 01 regressions. |
| `Cargo.toml`, `.github/workflows/build.yml` | Register new `harness = false` SSH test and add it to existing native headless test command. No dependency/trigger change. |
| Cycle/design/plan/ledger, ticket/comments | Planning approvals, task evidence, rulings, blockers, review-ready and separate delivery permissions. |

Dependency order: Task 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7. Test fixture support may be
prepared while its owning task needs it, but no parallel writers are proposed.
Each task checkpoint is coherent and reviewable; local commits require the
implementation phase's authorization, not this planning request.

## Task 0: Fresh Entry, Dependencies And Baseline

**Files:** ledger, ticket/comments only before implementation authority.

- [ ] Confirm all three artifacts are approved, explicit implementation authority
  and execution method, and any later review decisions. Keep ticket open.
- [ ] Inspect main/ticket status and operation-in-progress; preserve planning/user
  edits coherently before rebase. Fetch current `origin/main`, rebase ticket
  onto it inside this worktree, verify ancestry/status and record before/base/
  after SHAs and conflict rulings. Do not rebase main or other worktrees.
- [ ] Re-read changed AGENTS and reconcile Cycle 05 merged APIs. Specifically
  check exact-ref fast path, endpoint generation, monotonic checkpoint validator,
  authority/index replay, pending local/remote operation guards, guarded paths,
  effective identity and manual-only CI. Stop on behavioral incompatibility.
- [ ] Run fresh baseline and record command exits/test counts/qualifications:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

- [ ] Separate pre-existing failures and historical Cycle 05 evidence from new
  results. Do not start implementation without a understood usable baseline.

**Checkpoint:** current source/base ledger and ticket comment; native evidence
still pending. No baseline was run during planning.

## Task 1: Characterize The Locked Merge Backend And Define Contracts

**Files:** new `remote/merge.rs`, `merge_tests.rs`; modify `remote/mod.rs`,
`refs.rs`/tests and public exports in `repository.rs` as needed.
**Consumes:** real git2 commit graphs/merge index, canonical target identities,
existing `SynchronizationTarget`/`RemoteRefPlan`.
**Produces:** pure stage classifier, supported conflict inspection/token model,
typed local resolution inputs/outcomes and backend preservation evidence.

- [ ] Add failing graph tables for equality, local/incoming ancestor, divergence,
  missing objects/ancestry error and unrelated roots; primary versus ordered
  context/primary stages. Keep the Cycle 05 virtual no-partial-FF tests.
- [ ] Characterize real `merge_commits`/`merge`/`checkout_index`, conflict stage
  enumeration/modes, merge-head/state inspection and state retirement. Assert
  actual worktree markers, unchanged HEAD during conflict, tree/parent results
  for clean merges, and no automatic side favoring.
- [ ] Characterize separately opened preparation handle/ODB with
  `add_new_mempack_backend(1000)`: same-file nonconflicting content merges MUST
  create their blobs in memory without destination object/index/ref/worktree
  writes or changing another handle's ODB. Locked libgit2 merges write blobs;
  ordinary disk-ODB `merge_commits` outside the lease is not a read-only seam.
  Verify minimal result-object import/OID equality under the lease and backend
  lifetime/failure behavior. Stop for amendment if this isolation cannot hold.
- [ ] Prove safe checkout protects tracked, untracked, ignored and symlink/file-
  directory collisions with `overwrite_ignored(false)`. Include linked item
  worktree versus primary index/merge-metadata isolation. No mock substitute.
- [ ] Define `IntegrationStage`, immutable input/candidate evidence, opaque
  observations, path tokens, `ResolveSynchronizationRequest`, local result and
  redacted body/read wrappers. Define exact validation/error categories without
  returning raw backend strings. No raw OID/refspec/path/force authority in
  public mutation requests.
- [ ] Define optional caller-confirmed identity input and identity-required
  boundary through existing effective/local identity helpers. Preserve `None`
  at existing request callsites and no identity requirement for non-committing
  paths. Auxiliary confirmation carries its own stable ID, expected identity-
  config observation and bound input; adding missing identity before a candidate
  is not a changed target. Reject changed confirmation replay, reconcile partial
  config writes and reuse existing candidates as specified in the design.
- [ ] Add eligibility tables for valid document/ticket/comment conflicts,
  noncanonical/config/binary/executable/symlink, rename/delete/add-add/identity
  change and mixed sets. Marker-shaped text alone must not drive eligibility.
- [ ] Implement the smallest classification/inspection seams once tests fail;
  no public network orchestration yet.

**Commands:**

```sh
devenv shell -- cargo test --locked --lib repository::remote::merge
devenv shell -- cargo test --locked --lib repository::remote::refs
```

**Checkpoint:** A1/A2/A4 backend/API characterization and risk qualifications.
Stop if the locked backend cannot safely install/observe preserved conflicts;
request design review rather than adopting force checkout or simulated markers.

## Task 2: Migrate Ordered Recovery And Resolution Evidence

**Files:** `remote/state.rs`/tests, `reservation.rs`/tests, `recovery.rs`, minimal
exports/test seams in `repository.rs`.
**Consumes:** Task 1 stage/token contracts; existing owned reservation envelope.
**Produces:** transactional child evidence, strict identity/fencing/replay and
conflict ownership lifecycle.

- [ ] Write failing migration tests from genuine Cycle 04/05 schema/rows,
  completed authority/index-pending rows, cancelled/ambiguous push rows, empty
  schema and repeated migration. Partial schema/invalid OIDs/mismatched parents
  fail recovery without discarding rows. Extend CHECK/index/decoder/audit together.
- [ ] Add ordered `remote_integration_steps`, immutable
  `remote_resolution_attempts` and per-owned-path evidence. Preserve remote
  operation target/epoch/generation and legacy checkpoints. Keep unsupported
  path text and all caller bodies out of rows/WAL/journals/backups.
- [ ] Add child identity-confirmation evidence for missing effective Git identity,
  immutable confirmation input/config-observation digests and applied config
  progress. Confirmed values never overwrite an already valid effective identity;
  partial configuration writes are reconciled before candidate creation.
- [ ] Validate attempt/confirmation IDs across existing operation namespaces,
  parent/stage/root/target binding and digests. Different input reusing a bound
  ID is mismatch; duplicate completed input returns recorded effects.
- [ ] Prove intent is durable before local effects and completion is after actual
  observation. Transaction faults retain previous evidence and cannot manufacture
  completion, drop cancellation or authorize a stale owner.
- [ ] Add fenced APIs for prepare/apply/conflict/local-resolution/commit/retire
  and bounded stage queries. Test repeated local stages without rewinding old
  monotonic safe points. Normal operation must not scale with all retained history.
- [ ] Release only active ownership at conflict; retain pending stage. Reacquire
  explicitly and reject competing attempts/stale epoch/generation. A new action
  cannot adopt an unproved target merge. Poll/manual yield behavior remains intact.
- [ ] Add migration/startup/cache-rebuild privacy, corrupt/orphan evidence,
  cache-loss and two-service race tests using disposable real SQLite/Git.

**Commands:**

```sh
devenv shell -- cargo test --locked --lib repository::remote::state
devenv shell -- cargo test --locked --lib repository::remote::reservation
devenv shell -- cargo test --locked --test remote_reservation --test recovery_foundation_gate
```

**Checkpoint:** A5/A8 migration, persistence fault, identity, fencing, privacy
and bounded-query evidence. Record actual schema names if refined internally.

## Task 3: Integrate Clean Merges And Preserve Real Conflicts

**Files:** `remote/merge.rs`/tests, `sync.rs`/tests, `repository.rs`,
`refs.rs` only as required.
**Consumes:** prepared stage/owned journal from Tasks 1/2 and existing exact-fetch
observations/clean preflight/push path.
**Produces:** divergence-capable public synchronization and conflict inspection.

- [ ] Add local real-repo failing tests for primary clean divergence, document
  and ticket context divergence, first publication with primary divergence,
  context-then-primary two merges, clean first stage followed by second conflict,
  and already-integrated no-op/FF after an earlier merge.
- [ ] Retain the all-clean virtual fast path. Route `MergeRequired` into stage
  planning only inside a deliberate manual synchronization. Before mutation
  validate primary/publication absence/deletion/history and target worktree
  identity, actual cleanliness, configuration generation and incoming tracking OIDs.
- [ ] Prepare graph/merge index on the separate high-priority mempack ODB handle
  and perform canonical validation outside the lease. Resolve effective/
  confirmed Git identity before candidate effects. Under the lease, re-observe
  durable input intent, import only verified missing result objects into the
  actual ODB, materialize candidate and persist OID/tree/parents, lock/recheck
  expected ref and safely apply. Reuse candidate after retry; no empty checkpoint.
- [ ] Implement actual conflict index/worktree/metadata installation and record
  its observed fingerprint. Keep local HEAD unchanged on conflict, return typed
  recovery, do not push, retire nothing or hide markers. Safe-checkout failure
  retains actual partial state and reports recovery, never rollback.
- [ ] Implement actual-state inspection and explicit ephemeral side reads with
  stale token checks/redacted formatting. Show unsupported entries without
  accepting them as owned writes or leaking hostile path text.
- [ ] Verify clean candidate and target identity after mutation; generation or
  publication-config changes stop continuation. Only matching merge metadata
  may later be retired. Preserve unrelated worktrees and ignored data.
- [ ] Add tests for missing identity/caller confirmation, wrong/detached branch,
  target mismatch, concurrent ref/index/config/ignored-file change, no common
  ancestor, missing objects and ref-lock/checkout/record failure. Reject foreign
  merge/rebase/cherry-pick metadata even with a conflict-free clean-looking index;
  only this operation's exactly observed merge enters its recovery path.
- [ ] Keep ordinary Push endpoint preconditions and post-push authority; no
  Push-only merge stage. Stable results retain discovery/index-pending behavior.

**Commands:**

```sh
devenv shell -- cargo test --locked --lib repository::remote::merge
devenv shell -- cargo test --locked --lib repository::remote::sync
```

**Checkpoint:** A1/A2/A6 actual ordered parents/trees, conflict state, safe
application and preservation. Source review before adding resolution writes.

## Task 4: Implement The Owned Canonical Resolution Checkpoint

**Files:** `remote/merge.rs`/tests, state/reservation resolution transitions,
`repository.rs` owned/identity helper visibility and pending-merge authoring
guards, `tests/local_authoring.rs`, `tests/recovery_foundation_gate.rs`.
**Consumes:** actual conflict observations and immutable parent/stage evidence.
**Produces:** idempotent local-only two-parent canonical resolution.

- [ ] Add failing real-index tests for document/ticket/comment resolutions,
  multiple canonical conflicts, base/local/remote reads, exact-byte preservation,
  unknown keys, stable IDs, valid reply relations and retained closure metadata.
- [ ] Reject stale HEAD/merge parents/index/modes/file bytes, path substitution,
  malformed Markdown, duplicate IDs, invalid threads, closure reopening and
  unsupported/mixed conflict sets BEFORE writes. Include traversal/symlink and
  owned-file race seams. Invalid input must not reveal bodies in diagnostics.
- [ ] Validate a prospective result outside the lease; compare affected source
  digests under the lease. Bind exact observation/results/identity input to a
  distinct attempt ID before any write. Altered same-ID retry rejects; a new
  attempt can use a fresh conflict observation after a rejected no-effect input.
- [ ] Write only supported conflicted paths using existing guarded helpers.
  Record/reconcile each old/result digest. Inject failure after first of several
  writes; identical retry completes remaining paths, changed third value stops.
- [ ] Build the resolution tree from the recorded merged nonconflicting index
  plus caller blobs. Preserve clean tracked entries from BOTH parents; reject
  new unrelated worktree/index changes and never call blanket staging or the
  one-parent checkpoint helper. No commit with any unresolved index conflict.
- [ ] Create detached deterministic two-parent resolution candidate, durably
  record it, apply expected-old ref/index transition, observe exact resulting
  state, and retire matching merge metadata as a separately recoverable step.
  Fail after candidate/ref/record/retirement; assert one branch checkpoint OID.
- [ ] Return local stage completion and refresh-required, no network. An explicit
  original synchronization restart is the only publication continuation.
- [ ] Add guards/tests proving regular save/ticket/comment checkpoints cannot
  steal or stage an outstanding synchronization merge; unaffected read/inspection
  and later normal authoring remain usable.

**Commands:**

```sh
devenv shell -- cargo test --locked --lib repository::remote::merge
devenv shell -- cargo test --locked --test local_authoring --test recovery_foundation_gate
```

**Checkpoint:** A3/A4/A5/A8 scoped input/validation, two-parent tree, partial
write/commit replay, privacy and no hidden publication evidence.

## Task 5: Reconcile Interruptions, External Repair And Remaining Work

**Files:** `remote/sync.rs`/tests, `merge.rs`/tests, reservation/state tests,
minimal recovery/discovery integration.
**Consumes:** per-stage candidates/conflicts/attempts plus actual Git state.
**Produces:** safe restart ordering and unchanged terminal/index-only replay.

- [ ] Write table/fault tests at before/after merge preparation, durable intent,
  prepared-object import/candidate creation, checkout/index installation,
  ref commit, observation persistence, each
  resolution write, resolution candidate/checkpoint, merge-state retirement,
  between context/primary stages, push start/return/verification and refresh.
- [ ] Reconcile completed candidates/stages BEFORE the clean-target preflight or
  new network work. Accept only own exact recorded transition; preserve partial
  checkout/ref mismatches rather than discarding them or regenerating merges.
- [ ] Identical pending conflict/resolution retry is local-only; do not refetch
  before checking/writing its observed canonical checkpoint. An unfinished
  fetch with no durable completion may repeat a read-only fetch, explicitly
  distinguished from duplicating a proven effect.
- [ ] Add external repair tests: genuine clean two-parent merge of exact recorded
  parents resumes; removal of markers only, unresolved index, staged code without
  commit, wrong/unrelated parents, invalid target/canonical result, detached
  HEAD and dirty unrelated work remain recovery. Stage no arbitrary code.
- [ ] Preserve earlier context merge when primary conflicts/fails. Resume only
  pending primary stage, then ordinary publication. Fresh changed remote refs
  may append new eligible integration evidence without replaying applied stages;
  one invocation is bounded to one current-ref integration pass/one push attempt.
- [ ] Retain Push-direction ambiguous-acceptance reconciliation, remote deletion,
  endpoint generation mismatch and refusal to infer fetch/push equality. A remote
  race after local merge leaves it intact and returns recovery for later retry.
- [ ] Add cancellation/lease-observer/two-service owner tests around every new
  safe point. No total stop deadline, prompt/network/full scan under lease,
  automatic side choice, lease theft or unbounded race loop.
- [ ] After stable publication/current result, discovery failure records exact
  index-pending authority; replay with transport disabled only refreshes.
  Conflict/recovery remains directly inspectable if indexing is unavailable;
  ordinary index rebuild must not clear Git conflict or replay mutation.

**Commands:**

```sh
devenv shell -- cargo test --locked --lib repository::remote
devenv shell -- cargo test --locked --test discovery_rebuild --test recovery_foundation_gate
```

**Checkpoint:** A4/A5/A6/A7/A8 fault matrix with actual refs/index/worktree/record
and commit/effect counts; no claim based solely on durable phase names.

## Task 6: Prove Public Behavior Against Authenticated Two-Clone Remotes

**Files:** new `tests/remote_merge_recovery.rs`, reusable `tests/support/*`
fixture/privacy changes, `tests/remote_synchronization.rs`, `Cargo.toml`,
`.github/workflows/build.yml`.
**Consumes:** completed public APIs and existing authenticated fixture/harness.
**Produces:** independent real-remote acceptance evidence and native test wiring.

- [ ] Register the new shell-free `harness = false` target using the existing
  harness, raw capture isolation, canary inventory and teardown conventions.
  Tests create temporary repos/keys/local identities; never touch developer
  global Git/SSH/app data. Fail closed if fixture capability is unavailable.
- [ ] Two clones independently checkpoint the SAME shared branch, then public
  synchronize proves clean document/ticket divergence, primary divergence,
  context then primary ordered merges, retained parent histories/contents and
  ordinary verified publication. Assert exact ref scope and unchanged FETCH_HEAD.
- [ ] Create actual canonical conflict, inspect its index/markers, submit observed
  local resolution, restart explicitly and prove publication/discovery. Cover
  document/ticket/comment and multi-path sets. Conflict alone causes no receive-
  pack update or loss of worktree/branch/merge baseline.
- [ ] Cover first merge succeeded/second conflicted; code/binary/rename/delete/
  symlink/mixed conflicts externally repaired by fixture git2 commits, then
  eligible resume. False repairs and stale/different resolution retries preserve
  bytes/index/refs and return typed categories.
- [ ] Reuse receive-race/disconnect/persistence controls after a real merge to
  prove push rejection and ambiguous acceptance preserve one candidate, no blind
  repush, and endpoint-qualified observation. Test distinct Push remote, deleted
  context and changed configuration generation after conflict interaction.
- [ ] Inventory commits/parents/remote effects for each interruption seam.
  Distinguish necessary new observation from a repeated completed mutation.
  Disconnect transport after verified publication to prove index-only replay.
- [ ] Add body/credential/endpoint/hostile-path canaries; scan all recovery rows,
  live WAL/journal/backups, debug/display, captured fixture output and failure
  probes. Privacy test assertions must not print the offending private bytes.
- [ ] Update only obsolete SERVICE divergence expectations in
  `remote_synchronization`; pure clean planner still refuses divergence.
  Preserve all Cycle 05 absence/push/identity/ignored-file/collision regressions.
- [ ] Add `--test remote_merge_recovery` to the existing native headless job,
  preserve five targets and `workflow_dispatch` only. Do not enable/dispatch CI.

**Commands:**

```sh
devenv shell -- cargo test --locked --test remote_merge_recovery
devenv shell -- cargo test --locked --test remote_synchronization
devenv shell -- cargo test --locked --test ssh_fixture --test ssh_transport --test remote_observation
```

**Checkpoint:** A1–A8 real authenticated fixture and raw-privacy evidence;
list platform-only checks still pending rather than inferring native passes.

## Task 7: Final Verification, Review And Approval Handoff

**Files:** evidence/documentation/ticket/comments; code fixes go through owning
task's meaningful tests before repeating affected final gates.

- [ ] Re-read all three artifacts, decision audit and actual exports. Map A1–A9
  to concrete test names/results; reconcile internal naming without overstating
  behavior. Every remaining blocker/native gap is explicitly recorded.
- [ ] Run required final local commands and record exits/counts/source revision:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

- [ ] No desktop feature was added. Desktop startup smoke is optional if an
  active display is available; if run, label startup-only, not conflict-UI or
  Wave 03 journey evidence:
  `devenv shell -- cargo run --locked --features desktop --bin manyhands`.
- [ ] Review whole change for parent/index fidelity, exact replay/ownership,
  canonical scope, partial-state preservation, endpoint/secret boundaries,
  backend failure paths and test proof gaps. Request independent code review
  without assuming this planning request authorizes a subagent. Fix findings,
  obtain re-review and rerun meaningful gates.
- [ ] Native evidence remains pending until separate authorized publication/
  manual CI execution runs all five targets, including the new focused target.
  Capture real URLs/revisions/results and capability qualifications. Prior
  Cycle 05 owner deferral does not approve a Cycle 06 deferral; an explicit
  Cycle-specific decision is needed to deliver with this gate outstanding.
- [ ] Add review-ready ticket comment with local/native evidence, unresolved
  gaps and requested publication/review authority. Keep ticket open.
- [ ] Only after implementation code-review or PR approval and appropriate
  delivery authorization, close ticket in final pre-merge checkpoint with
  canonical `closed_at`/confirmed `closed_by`, retaining all evidence. Publishing,
  merging and worktree cleanup each require their own permission. Do not delete
  recovery resources or force-push merely because the branch was rebased.

## Planning Self-review And Owner Handoff

Self-review follows `review-cycle-docs`; its source/concern/classification/proof
matrix is in the design, rulings/costs and exact preflight in the ledger, and
summary in ticket comments. Checked: scope/API compatibility, owned versus
whole-branch effects, identity/trust, body/privacy boundaries, stale and partial
resolution, corruption/replay, cancellation/time, locked backend behavior,
fixture fidelity, native evidence and lifecycle authorization.

Self-review surfaced one material mixed-set recovery question; the owner's
whole-merge external answer is recorded in Global Constraints, the Cycle,
design decision audit, ledger and ticket comment. No material question remains.
Native execution and future implementation characterization remain EVIDENCE
gates, not completed proof or hidden approval. The user approved all three
artifacts on 2026-10-07. Recommended future execution is sequential direct
tasks in the existing ticket worktree; explicit implementation authorization
and any alternative method come later.
