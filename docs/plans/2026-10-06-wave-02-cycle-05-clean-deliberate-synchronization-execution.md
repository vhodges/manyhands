# Wave 02 Cycle 05 execution ledger

Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DD`
Branch: `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DD`
Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DD`
Plan: [approved implementation plan](2026-10-06-wave-02-cycle-05-clean-deliberate-synchronization-implementation.md)

## Authority and preflight — 2026-10-06

The user confirmed the plans are done and approved, requested starting Cycle 05,
and selected **subagent-driven tasks**. Implementation and local checkpoint
commits are authorized. Push/PR, merge, ticket closure, and cleanup are not.

The worktree held only existing approved planning edits. Preserved them in local
checkpoint `bbdfc74` before rebase; no stash/reset/clean was used. Fresh fetch
observed main `29f3f5a25957836a8513cba8a318306e1b928063`. Local main matches it.
Before checkpoint HEAD: `e6eff2654c2785250418a9df64d6710a04f0872a`.
Before rebase HEAD: `bbdfc74`.
After rebase HEAD: `7f80e8d124b2639bf2f43403693bf0f78aba5ad0`.
Rebase completed without conflicts; fetched-main ancestry and clean status
verified. Rebased `AGENTS.md` is unchanged. Main's `.superpowers/` and
`devenv.nix~`, and all other worktrees, are untouched.

Cycle 04 is now merged, including remote refs/state/reservation/observation
modules. Detailed API compatibility and baseline checks are the next gate.

## Implementation topology and ownership

Multi-seam change, partitioned by approved task contracts. All stages run
sequentially in the existing ticket worktree (no replacement worktree). Exactly
one active writer at a time; reviewer stages are fresh-context and read-only.
Each component must produce a committed handoff before integration. This serial
shape avoids shared `state.rs`/module wiring conflicts while honoring the user's
existing-worktree requirement.

| Lane | Exclusive contract / files while active | Next gate / handoff |
| --- | --- | --- |
| Baseline | API reconciliation; ledger, ticket comment only | Required Devenv baseline; stop on material incompatibility |
| Task 1 | Exact ref mappings, typed targets, pure graph evaluator; refs/state/mod/exports, private tests | Focused red/green tests, checkpoint commit, independent review |
| Task 2 | Durable envelope migration/replay/observation persistence; state/reservation/observation/recovery | Migration/recovery/privacy tests, checkpoint commit, independent review |
| Task 3 | Scoped exact authenticated transfers; transport operation/remote and focused tests | Selected-key/trust/ref-scope tests, checkpoint commit, independent review |
| Task 4 | Integration-only orchestration consuming Tasks 1–3; sync.rs and minimal wiring | Preflight/preservation/replay service tests, commit, independent review |
| Task 5 | Real SSH fixture evidence and CI target; synchronization tests/support/build.yml | Acceptance/privacy/index-only replay tests, commit, independent review |
| Task 6 | Full validation/evidence only, fixes routed to owning component | Required local gates, fresh whole-branch review; native CI pending publication authority |

## Task state

- Baseline: passed; API prerequisites reconciled (checkpoint below).
- Task 1: complete; independent review accepted at `cc53b842373daeec07a8cfec712ba2f68555868f`.
- Task 2: implemented candidate at `f4f97118cf452004bbaf2063a2a55fc794b580c1`; independent review pending, unqualified full-test gate blocked by the recorded discovery race.
- Task 3: pending.
- Task 4: pending.
- Task 5: pending.
- Task 6: pending.

## Verification and review

Baseline and per-task Rust validation evidence is recorded below. Native
five-target CI requires later authorized publication. Prior Cycle 04 results are
not substituted for fresh baseline or final Cycle 05 checks.

## Baseline and dependency checkpoint — 2026-10-06T14:29:13Z

Inspected checkpoint `c12eb964748c29e9fb3079bc54f7bc0e170db8e8` in the
existing ticket worktree; status was clean before and after all baseline commands.
Fetched main `29f3f5a25957836a8513cba8a318306e1b928063` remains an ancestor
(`git merge-base --is-ancestor … HEAD`, exit 0). Parent preflight is reused;
no additional worktree, rebase, stash, reset, or clean was performed.
Read approved Cycle/design/implementation artifacts, AGENTS.md, ticket/comments,
Wave 02, and Git workflow, repository/index, authentication, and test strategy
RFCs. Explicit implementation/local-commit authority remains as recorded above.

### Merged API reconciliation

- `remote/refs.rs`: `RemoteRefPlan::from_configuration`, `primary`, `context`,
  validated `RemoteRefTarget`, and classification/tracking derivation match the
  plan. Existing `fetch_refspecs()` is the three-mapping **poll** policy including
  context wildcards, not deliberate sync; Task 1 adds the approved exact helpers.
- `remote/state.rs`: `RemoteOperationTarget::for_primary_synchronization` and
  `for_context(..., SynchronizeContext, ...)` already derive validated targets.
  `RemotePublicationEvidence::{NeverPublished,ObservedPublished,HistoryUnknown}`
  and `RemoteSnapshot::publication_evidence_for` provide the intended boundary;
  missing evidence defaults to unknown after history loss, otherwise never
  published. `RemoteRefObservation` retains advertised/tracking OIDs and validated
  names; malformed names are redacted before persistence. `complete_batch`
  atomically publishes a complete current advertisement, retaining prior OIDs
  on absence and distinguishing deletion from unknown history. Therefore Task 4
  must provide the complete final advertisement, not merely fetched target refs.
- `remote/reservation.rs`: one durable `remote_operation_records` envelope,
  manual priority and `PollYielding`, exact target/ID replay, owner generation/
  epoch fencing, cancellation and ordered safe points are present. Tokens hold
  no Git lease or SQLite guard. Current `commit_observation_batch`,
  `finish_remote_operation`, and `restart_remote_observation` intentionally permit
  only Poll actions. Generic sync completion/restart and new durable phases/OIDs
  are the approved Task 2 extension, not pre-existing synchronization support.
  SQL CHECKs/active index and row decoder must be migrated together while
  retaining Cycle 04 rows; no second journal/reservation is needed.
- `remote/observation.rs`: `observe_publication_remote` uses the scoped Fetch
  adapter, reads tracking metadata, rechecks configuration/key/endpoints after
  network, and commits via the owned batch path. It performs no transfer or
  branch update. Configuration equality is process-only; persisted endpoint
  identity is a digest, not a URL. Reuse/extract complete-batch mechanics without
  importing poll-only behavior into sync.
- Cycle 03 `transport/{operation,remote}.rs`: crate-private
  `with_authenticated_remote[_policy]` and `AuthenticatedSshRemote` supply
  advertisement, download and ordinary push; every reconnect rechecks selected
  key/source/endpoint/trust and one selected-key submission. Push direction is
  separately resolved. Fetch options disable FETCH_HEAD. Existing `download`
  transfers objects only; it does not update tracking tips, and raw mappings are
  currently internal primitives. Task 3 must add the planned validated exact
  tracking-update/composed capability, not assume download alone is a fetch.
  Push rejection is already mapped to fixed `PushRejected`; advertisement after
  push remains caller-owned verification.
- Existing `repository_lease` in `repository/coordination.rs` is the bounded
  common-Git local lock; remote `state::with_transaction` uses only the short
  cache guard/SQLite transaction. Existing discovery/lifecycle code releases
  local leases before full scans. No domain dependency on GPUI is introduced.

All prerequisite names are present; **no name-only substitutions or material
contract incompatibilities were found**. Missing synchronization phases,
tracking-tip updates, generic restart/completion and authoritative-result replay
are explicitly planned Tasks 1–4, not baseline regressions. No Rust, lockfile,
transport policy, or CI file was edited.

### Sequential local verification

Run interval: `2026-10-06T14:17:18Z` to `2026-10-06T14:28:47Z`.
Large raw logs are outside the repository at `/tmp/manyhands-cycle05-baseline/`.

| Exact command | Exit | Evidence |
| --- | ---: | --- |
| `devenv shell -- cargo check --all-features --locked` | 0 | `check.log`, dev check succeeded |
| `devenv shell -- cargo fmt --check` | 0 | `fmt.log`, no formatting differences |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | 0 | `clippy.log`, no warning failures |
| `devenv shell -- cargo test --all-features --locked` | 0 | `test.log`, 559 standard tests plus 112 custom-host SSH cases passed |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | 0 | `cli.log`, skeleton exited successfully, no window |

Test breakdown: library 127; desktop/CLI binary targets 0/0;
canonical_foundation 29; discovery_rebuild 59; key_material 39; key_storage 2;
local_authoring 112; recovery_foundation_gate 50; remote_reservation 9;
repository_enablement 73; session_credentials 11; shared_key_registry 39;
doc tests 9. Standard harness reports 0 failed/ignored/filtered. Custom SSH
hosts report remote_observation 15, ssh_fixture 31, ssh_transport 66 cases passed.
Combined: **671 passed cases**, no baseline test failures. All five commands
were executed in the listed order; no failed gate was skipped or relabelled.
No pre-existing Rust failure was observed. Desktop launch was not requested or
performed. These are local Linux baseline results, not Cycle 05 acceptance.

CI inspection: `.github/workflows/build.yml` is manual `workflow_dispatch` only;
the five native targets are Linux x86_64/aarch64, Windows x86_64/aarch64, macOS
aarch64. Existing headless job runs library plus shared_key_registry,
key_storage, session_credentials, key_material, ssh_fixture, ssh_transport,
remote_observation. Task 5 must add remote_synchronization. Workflow feasibility
is confirmed only by source inspection; **no actual native CI was run or claimed**.
Native evidence remains pending authorized publication. Baseline is ready for
Task 1; ticket stays open and publication/closure/cleanup remain unauthorized.

## Task 1 review gate — 2026-10-06T14:46:58Z

Independent fresh reviewer `6e77d809-5d0e-475e-9c06-c4607c6dadd3` inspected
`03d7d1fd42eb51584bf8aa3180f885660ced43d0..cc53b842373daeec07a8cfec712ba2f68555868f`
using an exact-range diff artifact and current source/tests. Confirmed current
full HEAD and clean status. Verdict: **OK**, no issues. Parent accepts Task 1.
The review confirmed exact ordinary mappings, restricted typed requests, virtual
context graph evaluation, distinct failure boundaries and test fidelity. It
reviewed the local validation logs; no new commands or mutations by reviewer.
Review artifact: `/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/2fdcd2dc-9e86-44fc-a640-ea52d3bc7538/task1/review.md`.

Task 2 may proceed. Durable replay, transport, actual mutation, privacy and
index-only replay remain downstream gates; this is not whole-Cycle acceptance.
Native CI remains pending publication authority. Ruling: reviewers without shell
access receive an exact committed-range diff plus parent full-head confirmation,
rather than treating a clean working tree as proof of a committed change.

## Task 1 — exact refs, typed targets, pure graph planning — 2026-10-06T14:41:28Z

Task base: `03d7d1fd42eb51584bf8aa3180f885660ced43d0` (committed baseline
evidence read before implementation). Reused the existing ticket worktree/branch
and parent preflight; no fetch/rebase/worktree/stash/reset/clean was performed.
Checkpoint message: `feat: define clean synchronization contracts`.
**Implementation and local checks complete; independent review is pending.**
Task 2 must wait for that review gate; no review approval is implied here.

### Interface and scope

- `RemoteRefPlan::{primary_fetch_refspec,context_fetch_refspec}` derive only
  `+P:T(P)` and `[+P:T(P), +C:T(C)]`. The leading `+` updates tracking refs only.
  `{primary_push_refspec,context_push_refspec}` derive exact ordinary `P:P` or
  `C:C`; no force, wildcard, empty source, or arbitrary ref input. Existing poll
  wildcard policy and configuration validation are unchanged.
- Added approved `SynchronizationTarget::{Primary,Context { kind,item_id }}`
  and `SynchronizeRemoteRequest { root,operation_id,target,approval,restart }`
  in `state.rs`, with minimal module/repository exports. Target construction
  uses canonical `ItemId` and `AuthoringKind`; `operation_target(&RemoteRefPlan)`
  reuses the existing validated `RemoteOperationTarget` constructors. There are
  no URL/refspec/OID/key/worktree/credential/force request fields.
- `remote::refs::plan_clean_integration` is visible only inside `remote` and
  consumes OIDs and an ancestry closure `FnMut(ancestor,descendant) ->
  Result<bool,E>`, never repository/network handles. It returns internal
  `CleanIntegrationPlan { final_oid,local_update,push_needed }` or
  `CleanIntegrationError::{PrimaryMissing,RemoteContextDeleted,HistoryUnknown,
  MergeRequired,Ancestry(E)}`. An ancestry query failure is not divergence.
  Equal OIDs bypass ancestry queries. There is no public graph API.
- Context planning evaluates the remote relation and then primary relation
  virtually before returning one plan. Missing context permits first publication
  only with `NeverPublished`; observed deletion and unknown history are typed
  boundaries. Primary presence is required before any successful plan. Current
  advertised context presence uses the actual graph despite unknown old history.
- `push_needed` describes Fetch-side graph planning, **not** publication proof
  or authority to skip independent Push-direction checks (Task 4). Callers must
  supply tracking OIDs matching the complete current Fetch advertisement; an
  absent context must be `None` even if a stale tracking ref survives.
- Narrow dead-code allowances on the internal graph seam keep this intermediate
  checkpoint warning-free until Task 4 consumes it; remove them when wired.
  No persistence schema, reservation, transport, orchestration, integration
  fixture, CI, dependency, lockfile, desktop, or CLI grammar change was made.

### Tests-first and local verification

Added nine private source-named tests in `remote/refs_tests.rs` before production
implementation and attached them to `refs.rs`. Tests cover exact strings for
both context kinds and slash-containing valid names; invalid/reserved names and
canonical-ID injection; target derivation for primary/document/ticket; equal,
behind, ahead and divergent graph pairs; missing primary; first publication,
deleted and history-unknown absence; primary divergence after an otherwise valid
context fast-forward; equal-OID query bypass and ancestry-error propagation.
These are pure planning tests, not real worktree/ref preservation evidence.

Raw logs: `/tmp/manyhands-cycle05-task1/` (outside repository).

| Exact command | Exit | Evidence |
| --- | ---: | --- |
| `devenv shell -- cargo test --locked --lib repository::remote::refs::tests` (red, before production) | 101 (expected) | `red.log`: missing target/request imports, four exact ref methods, graph types/function; 32 compile errors demonstrate absent contract |
| `devenv shell -- cargo fmt` | 0 | `format.log`: formatted only owned Rust files |
| `devenv shell -- cargo test --locked --lib repository::remote::refs::tests` (green) | 0 | `green.log`: 9 passed, 0 failed, 127 filtered |
| `devenv shell -- cargo test --locked --lib` | 0 | `lib.log`: 136 passed, 0 failed/ignored/filtered |
| `devenv shell -- cargo check --all-features --locked` | 0 | `check.log`: successful dev check |
| `devenv shell -- cargo fmt --check` | 0 | `fmt.log`: no formatting differences |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | 0 | `clippy.log`: no warning failures |
| `devenv shell -- cargo test --all-features --locked` | 0 | `test.log`: 568 standard tests and 112 SSH cases passed (680 total) |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | 0 | `cli.log`: CLI skeleton exited without a window |
| `git diff --check` | 0 | no whitespace errors |

All Cargo commands ran through Devenv. An optional `python3` text-edit helper
was unavailable (exit 127, no edits by that helper); native editing tools and
`sed` completed the same mechanical changes. No Rust/Devenv infrastructure or
validation failure occurred. No secrets/endpoints or backend text were introduced
into durable records/errors. Full privacy/real transfer/replay acceptance remains
Tasks 2–5; all five native CI targets remain pending authorized publication.

Changed source files: `src/repository/remote/{refs.rs,refs_tests.rs,state.rs,mod.rs}`
and `src/repository.rs`. Durable evidence: this ledger and canonical ticket
comment `01M5100000J5K6M7N8P9Q0R1S2`. Ticket remains open; no push/PR/merge/closure/
cleanup occurred. Next gate: independent Task 1 review, then Task 2 durable state.

## Task 2 — durable synchronization candidate — 2026-10-06T15:21:14Z

Task base: `9035d6b23628a8b7c7ddadddf1d86d6750b0ef73`.
Implementation head: `f4f97118cf452004bbaf2063a2a55fc794b580c1`
(`feat: persist synchronization recovery boundaries`). This evidence checkpoint
follows that implementation commit; final evidence HEAD/status are captured in
`/tmp/manyhands-cycle05-task2-review-head.txt` and the exact base-to-final-HEAD
range in `/tmp/manyhands-cycle05-task2-review.diff` for a read-only reviewer.
**Candidate only: independent review pending; required normal full-test gate
is NOT passing. Task 3 remains gated.** Ticket remains open.

Reused parent preflight, approved artifacts, accepted Task 1, and the existing
branch/worktree. No rebase/new worktree, dependency, lockfile, transport,
synchronization orchestrator, frontend, scheduler, fixture or CI change.
Production scope: `remote/{state,reservation}.rs`; private tests:
`remote/{state_tests,reservation_tests}.rs`. The sole recovery-adapter edit
extends its existing active-phase SQL predicate, keeping local lifecycle work
fenced by the same remote reservation. `observation.rs` is unchanged: its poll
path consumes the shared owned batch commit without duplicating its policy.

### Durable interfaces for Tasks 3–4

- The single `remote_operation_records` envelope gains nullable typed OID
  evidence and action checkpoints. No second journal exists. Migration rebuilds
  the envelope's SQL CHECKs and active unique index in the same transaction;
  immutable target enforcement is restored. Existing completed/interrupted/
  yielded/cancelled/failed Poll rows keep their phase, safe point, timestamps,
  OIDs, requests, epoch and fixed outcome. They acquire no inferred publication
  or authoritative outcome. Partial reservation/sync schemas require recovery.
- `RemoteOperationPhase` adds `FetchPrepared`, `FetchObserved`, `LocalPrepared`,
  `LocalFastForwarded`, `PushPrepared`, `PushReturned`, `PushVerified`,
  `Reconciling`. Terminal phase changes do not erase `sync_checkpoint` or OIDs.
  `RemoteOperationSafePoint` adds before/after Fetch and local update, before
  push, after push return/verification, and before discovery. Discovery's point
  is persisted atomically with classification, not obtained as a network token.
- Crate-private `SynchronizationCheckpoint` names those action boundaries plus
  `DiscoveryPending`. `SynchronizationEvidence` holds only optional
  `expected_oid`, `local_oid` (planned/final local candidate), `tracking_oid`
  (target Fetch tracking), `primary_tracking_oid`, `push_oid`,
  `push_advertised_oid` (independent Push evidence). Existing advertised OIDs
  remain in the complete Fetch batch. `SynchronizationAuthority` is
  `Published(Oid)` or `AlreadyCurrent(Oid)`. Inspection exposes checkpoint,
  evidence, authority and index-pending getters to Task 4. These and the new
  transition methods remain crate-private; temporary dead-code allowances are
  confined to the Task 4 seam and should be removed when consumed.
- `checkpoint_synchronization(root, owner, checkpoint, evidence)` enforces
  owner/service/epoch/generation, forward action order, immutable expected/push
  candidate, required OIDs and exact verification. `LocalPrepared` makes the
  candidate durable before mutation; `PushPrepared` does so before the call;
  `PushReturned` is not publication proof. Completed Fetch observation can only
  be claimed by the owned complete-batch transaction, not this generic method.
- Shared `state::persist_complete_advertisement` preserves Cycle 04's complete
  current-batch/deletion/history model without changing polling retry policy.
  `reservation::commit_observation_batch` permits an owned synchronization
  Fetch boundary and commits evidence/checkpoint atomically. Poll still uses
  `complete_batch` and its existing outcome/backoff wrapper. A cancelled or
  failed/partial batch never classifies an absent ref as deleted. Fresh Fetch
  during reconciliation does not erase the old local/push checkpoint.
- `restart_remote_synchronization(root, id, target)` explicitly fences the old
  owner and returns `Reconciling`, retaining candidate/evidence. Ordinary
  duplicates only inspect. Crate-private `reconcile_synchronization(root,
  owner, actual_local_oid, actual_worktree_oid, actual_push_advertised_oid,
  push_is_strict_ancestor)` requires freshly persisted Fetch, ref/worktree
  equality, recorded candidate compatibility and non-contradictory Push proof.
  Equal Push candidate advances to verified; divergent/mismatched evidence is
  recovery-required; a proven ancestor can retain the recorded ordinary push
  intent only after explicit restart. A previously verified push cannot be
  reclassified into another push. Cycle 04's legacy manual
  `BeforeLocalMutation` boundary remains accepted but cannot be assumed retry-safe.
- `classify_synchronization(root, owner, authority)` durably stores exact
  authority + `index_pending` + discovery checkpoint and releases the remote
  slot. `finish_synchronization_index(root, id, target)` clears only that exact
  authoritative handoff. Authoritative replay never yields a transport token,
  including after refresh failure or cancellation. Publication authority survives
  cancellation/yield once verified and classified.

### Supervisor rulings and downstream obligations

1. **Reconciliation split accepted:** Task 2 owns durable fencing/replay
   selection, NOT actual network/local-effect observation. Task 4 must re-open
   actual refs and the exact target worktree, prove symbolic identity,
   cleanliness/conflicts and branch/worktree consistency under the short Git
   lease, and independently observe the Push endpoint before passing OIDs and
   ancestry to reconciliation. A fresh Fetch alone cannot authorize another
   local update or push. OID/ancestor inputs are internal observations, never
   public request authority. Task 5 owns real accepted-before-disconnect proof.
2. **Same-ID discovery accepted:** remote-path discovery must use the same
   operation ID too. The narrow local-ID coexistence exception requires an
   exact Completed authoritative sync record and checks EVERY same-ID local row
   for this repository/root's `refresh` action with empty refresh target.
   Failed/in-progress/completed legitimate refresh records replay index-only;
   another root/action/target or an incomplete sync remains OperationMismatch.
   Task 4 must use existing `RefreshRepositoryRequest`/index-owner retry logic;
   this state checkpoint neither refreshes nor bypasses local owner validation.
3. **Terminal-slot release/cancellation preserved:** do not add a cross-ID
   terminal lock or cancellation-unblock policy. Same-ID cancelled actions stay
   cancelled with ambiguity retained. New IDs can acquire ownership but receive
   no old verification/checkpoint authority. EVERY new Task 4 action must prove
   actual local/worktree/Push state: equal candidate means no push, mismatch or
   divergence means recovery, only freshly proved ordinary ancestry permits an
   effect. Cross-ID non-blind safety is a REQUIRED Task 4/5 service test, not
   claimed as state-layer proof here.
4. **Gate failure handling:** serial full tests are diagnostic coverage only,
   not a replacement for the required normal gate. Preserve failures and allow
   independent review to audit cause; no retry-loop, lease-policy weakening,
   ignoring test or out-of-scope test correction was authorized or performed.

### Tests-first, regression, privacy and final-tree evidence

Added **16** private tests: 13 synchronization state/reservation cases and three
migration/integrity/bounded-history cases. Initial four sync tests were written
before production; the focused red command exited 101 with 62 missing-contract
compile errors. Later tests cover exact legacy SQL migration; before/after
Fetch/local preparation/update/push preparation/return/verification/discovery
SQL failures; preserved candidates; explicit restart; stale owner, generation,
root and target fencing; SQL active uniqueness; immutable cancellation;
reconciliation mismatch/ancestor/equality; exact primary/context authority;
legitimate same-ID refresh replay and unrelated-ID rejection; polling-policy
preservation; hostile sentinels across formatted replay, rows, live WAL and
backup. Existing journal/side-file scans remain in the state regression suite.
The idempotent-migration VM-step test compares before/after 4,000 completed
remote rows and passes its bounded-work assertion. None proves real Git/network
side effects; those remain Tasks 4–5.

Raw logs (all outside the repository): `/tmp/manyhands-cycle05-task2/`.
Repeated intermediate focused runs are retained as applicable; final green
counts below are not substituted for the failed whole-suite gate.

| Command (each Cargo invocation through Devenv) | Result / log |
| --- | --- |
| `devenv shell -- cargo test --locked --lib repository::remote::reservation::tests::sync_` | Expected red exit 101, absent contract (62 compile errors), `red.log` |
| `devenv shell -- cargo test --locked --lib repository::remote::reservation::tests` | Green 15 tests at initial implementation, `green-attempt.log` |
| `devenv shell -- cargo fmt` | Exit 0; owned-file formatting, `format.log` |
| `devenv shell -- cargo test --locked --lib repository::remote` | Final tree green **66 passed**, 86 filtered, `remote-green.log`; intermediate 56/61/63/65 green during added coverage |
| `devenv shell -- cargo test --locked --lib` | 149 passed at that checkpoint, `lib.log`; final normal full gate independently passed **152 library tests** |
| `devenv shell -- cargo test --locked --test remote_reservation` | Initial 8 pass/1 fail exposed legacy BeforeLocalMutation incompatibility; fixed within seam, then **9 passed**, `reservation-regression{,-attempt}.log` |
| `devenv shell -- cargo test --locked --test recovery_foundation_gate --test remote_reservation --test remote_observation` | **50 + 9 + 15 SSH** passed, `regressions.log` |
| `devenv shell -- cargo check --all-features --locked` | Final tree exit 0, `check.log` |
| `devenv shell -- cargo fmt --check` | Final tree exit 0, `fmt.log` |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Final tree exit 0, `clippy.log` |
| `devenv shell -- cargo test --all-features --locked` | Default attempts exited 101 at the same discovery race, `test.log` and `test-rerun.log` (151 lib; discovery 58/59 each) |
| `devenv shell -- cargo test --all-features --locked --test discovery_rebuild corrupt_cache_replacement_rechecks_after_the_exclusive_guard` | Focused race **1 passed**, `discovery-rerun.log` |
| `devenv shell -- cargo test --all-features --locked --test discovery_rebuild` | Default whole target **59 passed**, `discovery-default.log` |
| `RUST_TEST_THREADS=1 devenv shell -- cargo test --all-features --locked` | Diagnostic exit 0: **583 standard + 112 SSH = 695 passed**, `test-serial.log`; before the final added migration test |
| `devenv shell -- cargo test --all-features --locked` | `test-final.log`: interrupted by agent's 30-minute runtime timeout during SSH fixture cases; completed library152/discovery59 and other preceding targets, but **NOT a completed gate** |
| `devenv shell -- cargo test --all-features --locked` | Resumed once with 1,200-second command allowance: exit **101**, library152/canonical29 passed, discovery **58 pass/1 fail**, `test-resumed.log` and timestamp/load `test-resumed.meta` |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | Exit 0, no window, `cli.log` |
| `git diff --check` | Exit 0 before implementation commit and after timeout recovery |

### Timeout recovery and unresolved normal-gate assessment

Original worker timed out after final-tree focused/check/fmt/clippy succeeded,
while another normal full run was inside SSH fixture cases. Parent confirmed
no remaining Cargo/fixture process, HEAD still task base, five unstaged owned
files intact, clean diff check; preserved `/tmp/manyhands-cycle05-task2-timeout.diff`.
Revived worker confirmed that exact state at `2026-10-06T15:20:14Z`, reused the
completed logs, added NO source/test change, and ran the required normal gate
once as instructed. Run interval `15:20:22Z`–`15:20:29Z`; load averages were
0.99/1.33/3.06 at start and 1.21/1.37/3.05 at finish. It again failed the same
`corrupt_cache_replacement_rechecks_after_the_exclusive_guard` test at
`tests/discovery_rebuild.rs:2471` with `RepositoryBusy`. No further retry.

Source assessment: that unchanged test races two rebuilds and unwraps both as
success, while existing cache/Git leases return retryable Busy after 250ms.
The corrupt-cache replacement path runs migration under an exclusive cache
guard; the new schema adds fixed column/CHECK/query work there, but no new
long-lived guard or network/discovery scope. New-table creation happens once;
legacy CHECK/index rebuild happens only when sync columns are absent;
idempotent migration does not recreate its index or scan retained operation
history (new VM-step regression passes). Rebuild's replacement migration is
before its Git-lease section. The failure could be scheduling sensitivity or
new fixed startup cost interacting with the test's bounded wait; **causation is
NOT proven, and this is NOT labelled pre-existing** because baseline/Task 1
normal gates passed. Focused/default discovery and serial success do not
prove absence of a delta-caused liveness regression. Independent review must
examine this residual; no lease or test policy was changed.

Candidate committed for review per supervisor direction despite that honest
gate blocker. No push/PR/merge/closure/cleanup or Task 3 work occurred. Next:
independent Task 2 code and verification review, resolve/route the normal-gate
blocker before parent accepts Task 2. Native CI remains pending publication
permission, not claimed passing.
