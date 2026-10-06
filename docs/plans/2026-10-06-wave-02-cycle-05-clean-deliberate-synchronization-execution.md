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
- Task 2: complete; source review and targeted correction review accepted at `77018dc16b806c0a380bf9b02a5b926d59acf781`; normal required local gates pass.
- Task 3: complete; independent review accepted at `f4ecbd50504a77a70c0528a892b135e46db155ce`; required normal local gates pass.
- Task 4: implementation and assigned integration evidence complete; independent review pending.
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

## Task 2 review acceptance — 2026-10-06T15:56:22Z

Parent accepts Task 2 at `77018dc16b806c0a380bf9b02a5b926d59acf781`, after
original independent source review (no source defects, verification blocker)
and fresh targeted review `99cde9a2-9fb4-4302-831d-95754d1b2c37`.
Targeted range: `9596bf40a32d08dd3a08386aa8427d1d8eff3ea2..77018dc16b806c0a380bf9b02a5b926d59acf781`.
Verdict: **OK with notes**, no findings; prior local-gate blocker resolved.
Reviewer confirmed retry still exercises the exclusive-guard recheck because
availability is service-local; hooks are consumed safely, original ID/owner
semantics remain, and backup/snapshot assertions are retained. Final normal
Devenv tests: 584 standard + 112 SSH = **696 passed**; check/fmt/clippy/CLI pass.
Parent confirmed exact clean HEAD, correction-only diff and final logs/counts.

Performance notes remain: valid base/HEAD timing changed near the bounded cache
wait; instrumentation perturbs scheduling, and no production latency distribution
is claimed. This was source/artifact review, not a separate reviewer runtime run.
Review artifact: `/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/46574008-7e86-4177-a9c3-7cd370167cba/task2-correction/review.md`.
Task 3 may proceed. Actual refs/worktree/Push reconciliation, real effect and
cross-ID recovery tests remain Tasks 4–5; native CI and whole-Cycle review remain
pending. No publication, closure, cleanup or overall merge authority implied.

## Task 2 verification-blocker diagnosis and correction — 2026-10-06T15:47:39Z

Reviewer `42a6675a-11bb-401c-ad87-403ccb42b44b` found **no source issues** in
`9035d6b23628a8b7c7ddadddf1d86d6750b0ef73..9596bf40a32d08dd3a08386aa8427d1d8eff3ea2`,
but verdict **BLOCK** for the normal full gate. This follow-up addresses that
blocker only. Original implementation/replay rulings and downstream actual-state
proof obligations remain unchanged; no Task 3 work or state implementation edit.
Diagnosis/correction base: `9596bf40a32d08dd3a08386aa8427d1d8eff3ea2`.
Source comparison base: `9035d6b23628a8b7c7ddadddf1d86d6750b0ef73`.
Final correction HEAD/status are captured in
`/tmp/manyhands-cycle05-task2-diagnosis-review-head.txt`; exact correction-range
diff is `/tmp/manyhands-cycle05-task2-diagnosis-review.diff`.

### Controlled comparison and artifact identity

Parent authorized an immutable `git archive` snapshot of the exact source base
into owned scratch, not a new branch/worktree. All Cargo invocations ran through
Devenv FROM the existing ticket worktree. No concurrent Cargo/fixture process;
resolved target is this ticket's actual directory, not a shared symlink.
Snapshot manifest recorded in
`/tmp/manyhands-cycle05-task2-diagnosis/base-snapshot.txt`.

Initial purported base repetitions reused HEAD artifacts without recompilation,
despite selecting the snapshot manifest; these **base1–3 logs are INVALID
baseline evidence**. Preserved, not counted as comparative results. Parent then
approved narrowly invalidating `manyhands` package artifacts only, sequentially
with explicit ticket target directory (`cargo clean -p manyhands`, not git clean,
not a general Cargo/dependency cleanup). The first valid base and HEAD builds
explicitly recompiled manyhands from their respective manifest locations.
SHA-256 identities retained in `base-identity.txt`/`head-identity.txt`:

| Identity | Exact base | Pre-correction HEAD |
| --- | --- | --- |
| state.rs SHA-256 | `5314eba672c71e05051a461677f62e8f6042ec15cafc5169f7f295130f81fdf2` | `9c636ffe9e70f72eca00998906a6df498983b69e3f669d42e5656b0ec72e70a2` |
| coordination.rs SHA-256 | `0372f54299a6afea209b28b9eda97262c9b935221fb35cbc02af54eae7625914` | identical |
| discovery executable SHA-256 | `c10861131408303a489f07090ed395e92799280684991e47feacd9c1ea903977` | `5b4d75e25436db8af68568ef5cd62ad7fda6dcd52122c61adea56cdab585271a` |

Executable filename is the same in both builds; SHA, source and recompilation
logs distinguish them. Valid default whole discovery-target comparison:

- Base: **59/59 passed in all three runs**, harness durations
  5.33/5.24/5.27s; command durations 14/6/6s (first includes package rebuild).
- HEAD: **58 passed/1 failed in all three runs**, same corrupt-cache race;
  harness durations 5.47/5.40/5.45s; command durations 14/6/6s.

This implicates changed startup work/timing, not an unrelated broad SSH failure.
It does not quantify production performance distribution from three samples.
Earlier failures are NOT relabelled pre-existing.

### Wait/holder attribution

Temporary fixed-category/duration-only instrumentation was applied to the
existing worktree, with one diagnostic source backup tar and retained temporary
diff outside the repository. No path, endpoint, credential or backend payload
was logged by instrumentation. An initial diagnostic compile error (moved path)
was corrected solely in temporary instrumentation, not production code.

- Full wait/hold tracing perturbed scheduling: instrumented whole targets passed
  3/3, so those passes are NOT comparative safety evidence.
- Minimal timeout-only tracing reproduced a **cache** acquisition timeout at
  **253413us** and **251130us**, not Git.
- Holder-phase tracing reproduced specifically **cache exclusive** wait timeout
  **250590us**. The replacement holder returned after **243993us**, with its
  critical test hook consuming only **14us**, and migration **128540us**.
  Remaining hold time covers replacement/marker/open/other work; no finer causal
  partition is asserted. Close-boundary wait vs measured hold may include time
  before/after the instrumented span and scheduling; do not infer exact additive
  decomposition or an exact production latency from this one sample.

High confidence: the reproduced failure is the bounded exclusive CACHE wait,
and the measured hook release is prompt rather than a main-thread channel stall.
Valid base-vs-HEAD evidence supports changed cache replacement/schema work
interacting with the existing 250ms policy/test success assumption. Precise
performance delta and failure probability remain unmeasured; instrumentation
changes scheduling. No longer-lived guard scope, Git-lease policy change, or
unbounded history scan was identified, and no speculative production refactor
was made. The original bounded-history VM test alone is not wall-clock proof.

### Approved smallest correction

Parent accepted a deterministic **test caller correction**, not a production
lease/Busy/retry change. Only
`tests/discovery_rebuild.rs::corrupt_cache_replacement_rechecks_after_the_exclusive_guard`
changes:

1. Keep both original concurrent same-ID rebuilds, decision barrier and
   exclusive-guard hook overlap.
2. Join BOTH callers before any retry. Check exactly one corrupt diagnostic
   backup immediately after the initial pair.
3. Accept initial error ONLY if exactly `RepositoryBusy`; unexpected errors
   fail. Retry ONLY each Busy caller ONCE with its original service/root and
   SAME operation ID. No sleeps, retry loops, new IDs or serialized initial calls.
4. Check exactly one backup again after retries, and retain BOTH service
   snapshot assertions. Hooks are already one-shot/consumed; a remaining
   critical closure shares the entered flag and cannot resend/wait on retry.

The test now respects the service's existing retryable-Busy boundary while
retaining authoritative recovery/recheck/one-replacement assertions, rather
than requiring every concurrent attempt to finish within 250ms. Targeted fresh
review must verify this correction still proves the invariant, not just success.
Temporary timing instrumentation was restored byte-for-byte before formatting
and all final checks; final production source equals pre-correction HEAD.

### Commands and final uninstrumented validation

Logs and exact comparative identities live outside the repository in
`/tmp/manyhands-cycle05-task2-diagnosis/`; prior failure/timeout logs stay intact.
`runs.txt` labels invalid initial baseline evidence; `temporary-timing.diff`
and `diagnostic-source-backup.tar` retain instrumentation provenance.

| Command | Outcome |
| --- | --- |
| `git archive 9035d6b23628a8b7c7ddadddf1d86d6750b0ef73` into immutable owned scratch | Selected exact baseline source; no branch/worktree/source reset |
| `devenv shell -- cargo metadata --manifest-path <snapshot>/Cargo.toml --no-deps --format-version 1` | Confirms snapshot manifest selection; insufficient alone to prove artifact binding |
| `devenv shell -- cargo clean --manifest-path <selected>/Cargo.toml --target-dir <ticket-target> -p manyhands` | Base and HEAD package-only invalidations succeeded; explicit source recompilation follows each |
| `devenv shell -- cargo test --manifest-path <snapshot>/Cargo.toml --target-dir <ticket-target> --all-features --locked --test discovery_rebuild` | Three valid base runs59/59; first recompiled snapshot package |
| `devenv shell -- cargo test --target-dir <ticket-target> --all-features --locked --test discovery_rebuild` | Three valid pre-correction HEAD runs58/59; first recompiled ticket package |
| `devenv shell -- cargo test --all-features --locked --test discovery_rebuild` with temporary diagnostic tracing | Perturbation noted; timeout-only2/3 and holder-timing1/3 reproduce exact cache categories/durations above |
| `devenv shell -- cargo fmt` | Exit0, corrected test formatting |
| `devenv shell -- cargo test --all-features --locked --test discovery_rebuild` on corrected uninstrumented tree | Exit0, **59/59**, 5.37s |
| `devenv shell -- cargo check --all-features --locked` | Exit0, command2s |
| `devenv shell -- cargo fmt --check` | Exit0, command1s |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Exit0, command6s |
| `devenv shell -- cargo test --all-features --locked` | **NORMAL default gate exit0**, command303s; **584 standard +112 SSH =696 passed**, including library152 and discovery59 |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | Exit0, command6s; no window |
| `git diff --check` | Exit0 |

Final gate interval: `2026-10-06T15:41:54Z`–`15:47:12Z`; no test-thread
serialization or diagnostic timing environment. Shared target now contains
rebuilt current package artifacts. No production state/coordination edits,
dependencies, locks, transport/Task3, scheduler, frontend or CI changes.

Normal verification blocker is now resolved locally **by the approved test
contract correction**, not by declaring old failures harmless. Task 2 still
requires fresh targeted independent review and parent acceptance. Ticket open;
publication/native CI/merge/closure/cleanup remain unauthorized.

## Task 3 review acceptance — 2026-10-06T16:27:13Z

Parent accepts scoped transport at `f4ecbd50504a77a70c0528a892b135e46db155ce`.
Independent fresh reviewer `6aeef3d8-1d92-42a6-9553-a0a030474724` inspected
`1903f3aaaa6f334c2ea9b54b4433223021bc9d75..f4ecbd50504a77a70c0528a892b135e46db155ce`:
**OK with notes**, no findings. Review confirmed typed exact/direction/config
validation, reconnect key/source/endpoint/trust checks, ordinary push rejection,
leased expected-old/create-only tracking writes and wire-test scope/privacy.
It independently confirmed the locked libgit2 FETCH_HEAD truncation motivating
the approved workaround. Parent confirmed clean full HEAD and normal test counts:
585 standard + 119 SSH = **704 passed**; check/fmt/clippy/CLI logs pass.
Review artifact: `/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/0f0c5c17-2a95-4a11-83db-392a6f99fb2c/task3/review.md`.

Task 4 may consume the accepted contracts. It must compose safe points outside
libgit2, compare complete Fetch pre/post advertisements and tracking OIDs, check
actual clean symbolic branch/worktree identity under the short lease, and verify
the independent Push endpoint candidate after push. Partial tracking updates and
push return are never authoritative completion. Remove temporary consumption-only
dead-code allowances when wired. Actual durable effect/replay/privacy proof and
native CI/whole-Cycle review remain downstream. No publication authority implied.

## Task 3 — scoped exact authenticated transport — 2026-10-06T16:20:00Z

Task base: `1903f3aaaa6f334c2ea9b54b4433223021bc9d75`.
Implementation head: `6aa81fc99fa4bb6c6f7e31df997962ec2aacc516`
(`feat: support exact synchronization transfers`). This evidence-only checkpoint
follows it; full final base/head/status and exact committed-range diff are in
`/tmp/manyhands-cycle05-task3-review-head.txt` and
`/tmp/manyhands-cycle05-task3-review.diff`. **Independent review pending**;
Task 4 must wait for parent acceptance. Ticket remains open.

Read AGENTS, approved Cycle/design/implementation/execution, canonical ticket
and comments, accepted Tasks 1–2 and existing transport/fixture seams. Parent
preflight/main ancestry remains reused; branch/worktree were clean at task base.
No new worktree/rebase/stash/reset/clean, dependency/lockfile, public export,
frontend, sync orchestrator, state/recovery, scheduler, poll-policy or CI edit.
No publication/PR/merge/closure/cleanup. Only production `transport/remote.rs`
changed; `operation_tests.rs` adds private test checkpoints, not runtime policy.

### API / supervisor ruling / Task 4 handoff

Three crate-private `AuthenticatedSshRemote` methods, each confined to the
existing `with_authenticated_remote[_policy]` secret borrow:

- `fresh_advertisement(&mut self) -> Result<Vec<(String, Oid)>, SshTransportError>`
  genuinely disconnects/reconnects and lists the request's direction. Existing
  `advertisement` remains unchanged for poll/Cycle 03 policy.
- `fetch_exact(&mut self, &RemoteRefPlan, &SynchronizationTarget) -> Result<(), …>`
  derives only exact primary or primary+one context mappings, lists successfully
  to select present sources, then downloads via a newly authenticated connection.
  Its own download connection's advertisement supplies OIDs; absence on that
  connection fails closed rather than using stale earlier OIDs. Empty selected
  scope skips download (an empty libgit2 refspec list would select defaults).
  `update_fetchhead(false)`, tags `None` and prune `Off` are explicit.
- `push_exact(&mut self, &RemoteRefPlan, &SynchronizationTarget) -> Result<(), …>`
  derives one ordinary mapping. It cannot accept force/wildcard/deletion/raw
  caller mappings or another target. Both local non-fast-forward and per-ref
  server status rejection map fixed `PushRejected`, never backend text.

All typed transfers bind the plan to current publication remote and primary,
reject mismatched direction/scope, and recheck configuration/key selection/source,
endpoint and host trust before/after reconnect/transfer. Every connection must
submit exactly the selected key once; unexpected extra reconnects fail closed.
The existing legacy object-only download/raw fixture push remain unchanged in
policy and are NOT synchronization APIs. New methods have narrow reasoned
`dead_code` allowances until Task 4 consumes them; remove when wired.

Supervisor approved **typed primitives, not another composed wrapper**. Task 4
owns complete list / durable safe point / exact transfer / safe point / fresh list
composition in the existing scoped closure. Checks must run OUTSIDE libgit2
calls and with no Git lease held over network/prompt/discovery. Task 4 must
compare its complete pre/post Fetch advertisements and exact tracking OIDs before
local integration; internal listing is not a substitute. Failed/partial transfers
are not complete observation or remote deletion evidence. Push-direction OIDs
are operation proof only; distinct `pushurl` never changes Fetch tracking evidence.
A successful push return is NOT Published; Task 4 must independently list again
and observe the exact candidate OID, recording the Task 2 durable boundaries.

### Locked libgit2 preservation defect and approved workaround

First wire run failed the existing-FETCH_HEAD byte assertion. Inspection of
locked **libgit2-sys 0.18.8+1.9.7** `remote.c` found
`git_remote_update_tips` unconditionally calling `truncate_fetch_head` at line2139,
before testing update flags. `RemoteUpdateFlags::empty()` therefore still truncates
the file. Parent independently verified the source and approved the following
narrow workaround; no dependency change or save/restore of FETCH_HEAD:

Download objects only, then validate ALL exact tracking destinations, downloaded
commit objects, and expected old direct-reference OIDs before the first write.
Acquire the existing short common-Git lease only AFTER download returns;
revalidate plan/key/endpoint/trust under it. Use Git expected-old ref writes
(`reference_matching`) for existing tracking refs, create-only writes for absent
ones. Only plan-derived `refs/remotes/…` can change; tracking rewind is allowed,
local branches never are. Release the lease before a later advertisement.
An external tracking change/write lock maps fixed redacted `ProtocolFailure`;
a partial write remains inspectable tracking metadata, not completed observation
or branch integration. No absent tracking ref is pruned, no tags/local branch or
FETCH_HEAD is modified. Task 4 must preserve partial state/reconcile normally.

### Tests-first / preservation / privacy evidence

Five real SSH contract cases were written before production: initial command
failed compilation with missing exact methods (15 missing-method errors plus
one fixture key-version mismatch, corrected via existing byte API). Added later
coverage plus one private mapping unit test yields **7 new wire cases +1 unit**.
The pre-thread custom host now runs **73** SSH transport cases (previous66).
No mock substitutes for transfer/tracking/push claims; no support-fixture change.

Coverage: primary/context exact scope, hostile default prune/tag settings,
existing FETCH_HEAD bytes and absent FETCH_HEAD, unrelated tracking refs/tags/
local branches, genuine tracking rewind, context/primary absence without prune,
plan/remote/primary/direction mismatch, force/wildcard/delete/cross-target mapping
rejection, encrypted selected-key/session reuse and exact per-connection key
counts, endpoint/selection/source/pin/host changes on reconnect, distinct Fetch
and Push destinations with distinct primary OIDs and post-push exact context OID,
changed/missing transfer advertisement, external tracking changes before/after
validation, second-ref lock failure leaving only inspectable partial tracking,
ordinary non-fast-forward and actual reference-status rejection. Hostile endpoint
and server-message sentinels remain absent from formatted diagnostics. Existing
transport/privacy/poll/state regression cases pass; this is not Task 5's full
synchronization durable-store privacy or accepted-before-disconnect proof.

### Commands and honest failure record

All Rust commands used Devenv; logs outside repository:
`/tmp/manyhands-cycle05-task3/`.

| Command | Result / retained log |
| --- | --- |
| `devenv shell -- cargo test --locked --test ssh_transport exact_` (before production) | Expected exit101 absent API, `red.log` |
| Same focused target, initial production | Failed existing FETCH_HEAD preservation at assertion63, `green-attempt.log`; led to approved libgit2 workaround, not weakened assertion |
| Intermediate focused/rejection targets | Failures from 17-byte hostile marker against required16-byte packet (`green2.log`, `rejection-attempt.log`), then local NFF category (`rejection2.log`); fixed fixture marker and scoped fixed rejection mapping |
| `devenv shell -- cargo test --locked --lib repository::transport::remote::tests` | 1 passed, `unit-green.log`; also passes in final full gate |
| `devenv shell -- cargo test --locked --test ssh_transport` initial regression | Stopped at distinct-endpoint setup: root commit onto existing ref without parent rejected; corrected setup to create owned ref then set destination main, `transport.log` |
| `devenv shell -- cargo test --locked --test ssh_transport exact_` | Final7 passed, `green-final.log` |
| `devenv shell -- cargo test --locked --test ssh_transport` | Final73 passed, `transport-final.log` |
| `devenv shell -- cargo fmt` | Exit0; formatting logs retained |
| `devenv shell -- cargo check --all-features --locked` | Final exit0, `check-final2.log` |
| `devenv shell -- cargo fmt --check` | Final exit0, `fmt-final2.log` |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Initial3 collapsible-if warnings, then12 large typed error closure warnings; fixed let-chains and narrow test-module allowance consistent with transport contract; final exit0 `clippy-final2.log` |
| `devenv shell -- cargo test --all-features --locked` | Normal default gate exit0, **585 standard+119 custom SSH =704 passed**, `test.log`; library153/discovery59/transport73 |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | Exit0, no window, `cli.log` |
| `git diff --check` | Exit0 before source checkpoint; final evidence checkpoint checked too |

No infrastructure failure, full-test retry, serialization, discovery correction
change or failed-gate relabelling. Intermediate failures retained and diagnosed
before rerun; final broad test gate run only once. Native CI remains pending
publication authority; whole-Cycle acceptance/service-effect proof remains
Tasks 4–6. Next gate: independent Task 3 review and parent acceptance.

## Task 4 candidate — 2026-10-06T16:59:29Z

Base `5ee6bd8e3b0b5c8b2812f177560cd7002fb52454`. Implemented the library-only
`RepositoryService::synchronize_remote` integration consuming accepted Tasks 1–3.
Candidate checkpoint is **review pending**, not Task 4/whole-Cycle acceptance.
No new branch/worktree/rebase/stash/reset/clean/publication/CI/closure/cleanup.
No dependencies, locks, front-end grammar, scheduler, merge, poll, checkpoint,
materialization, staging, promotion or lifecycle implementation changed.

The service derives targets, checks existing clean symbolic worktrees under the
short lease, owns one durable remote envelope, performs complete Fetch list /
exact transfer / fresh list, checks selected pre/post OIDs and current tracking,
atomically commits complete metadata, computes the actual virtual commit graph,
then performs at most one locked expected-old transition and safe checkout.
Independent Push advertisements and actual commit ancestry precede ordinary
exact push; post-push exact equality supplies durable authority. Classification
releases the reservation before same-ID refresh. Authoritative replay is
network/mutation-free; explicit restart reconciles actual clean refs/worktree and
independent Push evidence before resuming intent. Cancellation stays terminal.

Supervisor-approved bridges/rulings are documented in the design: (1) typed
Push-target object-only download uses an anonymous remote bound explicitly to
the resolved Push endpoint, never Fetch URL/tracking writes/FETCH_HEAD; (2)
repeated owned request checks preserve action checkpoint/owner fencing; (3)
local-only original OID+typed target are bound in the existing refresh matcher
BEFORE handoff, no remote record or second journal; completed matching refresh
is proof for index-only completion; (4) locked git2 lacks a baseline setter:
LocalPrepared -> lock/verify old ref -> safe checkout against OLD HEAD ->
recheck symbolic identity -> single ref commit -> actual final proof. An
interrupted checkout/ref mismatch remains recovery, never rollback or force.

Tests-first: initial four private service tests failed with 16 absent-contract
compile errors (`red.log`). Intermediate failures exposed fixture data-directory
lifetime, completed-refresh replay behavior, and real safe-checkout byte
preservation; assertions were retained and the approved rulings resolve them.
The added private suite now has **13 tests** (local-only exact identity/frozen
OID/recreation, index failure and bind SQL rollback, dirty/conflicted/missing
primary/unmaterialized/wrong branch, existing authored context isolation, real
Git divergent/deleted/unknown graph boundaries, actual FF bytes, ref lock and
post-checkout mismatch preservation). Existing-host extension adds **4 real SSH
smoke cases**: primary publish/current/actual FF/index-only replay; cancellation
before transfer; distinct Push object acquisition with ref/FETCH_HEAD
preservation; post-acceptance verification-write failure requiring explicit
restart and equal-candidate reconciliation. No Task 5 acceptance target, fixture
controls or CI expansion. Consumption-specific allowances removed.

Validation logs outside repository: `/tmp/manyhands-cycle05-task4/`.
- `devenv shell -- cargo test --locked --lib repository::remote::sync::tests`:
  red exit101, subsequent green (11 at last focused run; final suite includes13).
- `devenv shell -- cargo test --locked --test ssh_transport synchronization_`:
  final4 passed; initial fixture outcome assertion corrected to AlreadyEnabled.
- `devenv shell -- cargo test --locked --lib`: 165 passed before final bind test.
- Final `devenv shell -- cargo check --all-features --locked`, `cargo fmt --check`,
  `cargo clippy --all-targets --all-features --locked -- -D warnings`: exit0.
  Initial clippy failed large typed transport-error closures in the new fixture
  module; applied the same narrow result_large_err convention as existing tests.
- Final NORMAL `devenv shell -- cargo test --all-features --locked`: exit0,
  **598 standard +123 SSH =721 passed**, including166 library,59 discovery and
  77 transport SSH cases. No serialization/ignored failure/repeated broad gate.
- `devenv shell -- cargo run --locked --bin manyhands-cli`: exit0/no window.
- `git diff --check`: exit0; no staged files after coherent checkpoint.

Residual review/test obligations: Task 5 still owns full two-clone/context
publication matrix, actual receive-side push-effect counters/disconnect controls,
complete hostile durable-store privacy scans, native five-target CI and full
ordered service fault matrix. Current graph/helper tests are NOT advertised as
remote service deletion/divergence evidence. Task 4 review must assess missing
service-level wrong registered-worktree and missing/deleted/history-unknown
context cases, cancellation after fetch, LocalPrepared mismatch restart,
index-flag failure after completed refresh and cancelled-old/new-ID safety;
existing state/transport unit proofs and source checks are not substituted for
those actual integration effects. Runtime boundary requires this coherent
candidate checkpoint rather than silent expansion into Task 5 or lost edits.
Exact base..HEAD diff and full HEAD/status are saved for read-only review at
`/tmp/manyhands-cycle05-task4-review.diff` and
`/tmp/manyhands-cycle05-task4-review-head.txt`. Ticket remains open.

## Task 4 completion follow-up — 2026-10-06T17:30:54Z

Resumed clean candidate `13d717dade60331671a9c1f5eeb80c8c4051d090` after parent
**rejected acceptance for incomplete assigned Task 4 evidence**, not a Rust gate
failure. Prior rejection/history and721-case command evidence are retained above.
Parent required completing these proofs before independent review; none was
silently reassigned to Task 5. Original task base remains
`5ee6bd8e3b0b5c8b2812f177560cd7002fb52454`. No new worktree/branch/rebase/stash/
reset/clean or publication/CI/merge/closure/cleanup. Ticket stays open.

### Missing requirement -> actual public-service proof

All named cases are registered in the EXISTING authenticated SSH transport host;
source is `tests/ssh_transport/synchronization.rs`. Requests use the public
synchronization service and real fixture advertisements/owned git2 commit graphs.
TargetState compares actual local branch refs, HEAD, index, tracked worktree bytes
and status via digests, never formatted canonical Markdown or worktree paths.

| Assigned proof | Named service case / observation |
| --- | --- |
| Wrong branch and registered/deterministic context-worktree identity | `synchronization_context_identity_preservation`: existing authored context on wrong symbolic branch, then real deterministic worktree with its registered stable name pointing elsewhere; typed rejection, zero SSH, exact refs/index/bytes/status preservation. |
| Absent context with stale tracking / first publication | `synchronization_context_absence_boundaries`: complete actual absence ignores surviving stale tracking; first ordinary publication succeeds with actual remote candidate OID. |
| Observed Fetch context deleted | `synchronization_fetch_observed_context_deleted`: real service complete Fetch-present observation then actual ref deletion and complete absence; RemoteContextDeleted, no recreation or local changes. |
| Missing primary through service | `synchronization_missing_remote_primary`: actual remote primary deletion for both primary/context requests; complete fetch then PrimaryMissing; refs/index/bytes unchanged. Existing private service case also covers missing local primary. |
| Unknown context history | `synchronization_context_absence_boundaries`: explicit cache-history-loss flag, real complete absence, HistoryUnknown with actual remote absence/local preservation. |
| Context divergence and virtual-primary divergence | `synchronization_service_divergence_preservation`: actual Git siblings for context divergence; context could FF to real remote descendant but remote primary diverges; both MergeRequired before ANY local change. Also covers divergent primary synchronization. |
| Cancellation after completed fetch before local effect | `synchronization_cancel_after_fetch`: actual advertised/downloaded new primary equals tracking and durable complete snapshot; AfterFetch cancellation leaves local refs/index/bytes untouched; cancelled restart terminal and network-free. |
| LocalPrepared physical mismatch on explicit restart | `synchronization_local_prepared_mismatch_restart`: actual safe checkout reaches candidate index/bytes, injected symbolic change aborts locked ref commit; fixture restores symbolic identity ONLY, old ref/candidate index/worktree mismatch persists. LocalPrepared old/candidate OIDs inspected; repeated explicit restart returns RecoveryRequired without SSH/reset/ref/index/content repair. |
| Discovery completed, index-flag write fails | `synchronization_completed_refresh_index_flag_replay`: real FF/discovery completes, SQL trigger rejects only index_pending clear; authoritative IndexPending frozen. Replay with trigger retained and then removed performs no discovery scan (panic hook), SSH or local change; eventually Complete exact outcome. |
| Cancelled-old ambiguous push / NEW ID real proof | `synchronization_cancelled_push_new_id_proof`: actual server accepts candidate before AfterPushReturn cancellation. Same-ID restart terminal; NEW ID independently lists Push, returns AlreadyCurrent; typed push-call counter zero and actual remote candidate/local state unchanged. Context equivalent: `synchronization_cancelled_context_equal_candidate`. |

### Demonstrated defect, approved ruling and additional proof

Initial expanded tests compiled red for two absent test-only counter boundaries
(`counter-red.log`; earlier `red.log` also records a private-module test reference
error corrected without production change). After hooks, immediate-delete service
case failed at its typed RemoteContextDeleted assertion (`green-attempt.log`).
Diagnosis: successful first Push publication had an absent pre-push Fetch batch;
blind new-ID first-publication policy could recreate the now-deleted Push branch.
Supervisor forbade masking the failure with an added Fetch observation or marking
Fetch published from Push proof. Approved smallest bridge consumes existing
validated root/context remote-operation rows and monotonic generation/digest
fencing. It performs fresh independent Push proof before local FF/reconciliation
and before push: compatible exact old proof + absent => RemoteContextDeleted;
incompatible-generation proof => HistoryUnknown; older/inherited unverified
PushPrepared/PushReturned intent + absent => RecoveryRequired. Current first-call
unsent intent is excluded, so legitimate first publication remains possible.
No schema/public API/second journal/Fetch-history mutation or repair was added.

Additional cases retain the original red behavior and prove the bridge:
- `synchronization_context_absence_boundaries`: verified first publication ->
  immediate deletion with a newly advertised primary descendant that COULD FF
  context; deletion boundary BEFORE local effects; Fetch history remains never.
- `synchronization_distinct_push_context_deleted`: Fetch always absent, actual
  verified publication at distinct Push endpoint -> immediate deletion; recreated
  service/new ID returns deletion, no phantom Fetch history/local changes.
- `synchronization_push_history_generation_fencing`: real Push endpoint change
  increments generation; old proof cannot assert endpoint equivalence; absent
  new endpoint produces HistoryUnknown without recreation.
- `synchronization_ambiguous_cancelled_context_absent`: actual accepted push then
  cancellation/deletion/recreated-service/new-ID => RecoveryRequired; fresh Push
  advertisement occurs but zero typed push calls; old cancelled ID stays terminal.
- `synchronization_context_inherited_absent_push`: actual accepted context push,
  verification persistence fault, deletion, explicit same-ID restart => recovery.
- `synchronization_context_deleted_before_push`: disappearance between initial
  guard advertisement and final pre-push advertisement is not recreated.

Test-only `ExactPushStarted` counts client typed push attempts;
`PushAdvertisementObserved` follows successful fresh receive-pack listing on the
resolved Push endpoint. These are NOT server transaction/disconnect counters.
Actual remote refs and branch/index/worktree observations underpin preservation
and acceptance assertions, not inferences from stored operation rows. Two
independently pinned endpoints use approval=None during synchronization; an early
fixture failure passing the Fetch approval to distinct Push was corrected without
weakening trust policy (`green3.log`). No Task 5 target/CI/server fixture expansion.

### Final validation and review-ready checkpoint

Logs outside repository: `/tmp/manyhands-cycle05-task4-completion/`.
- `devenv shell -- cargo test --locked --test ssh_transport synchronization_`:
  final **19 real SSH cases passed** (`focused-final.log`);15 newly added cases
  plus the4 retained successful candidate cases, not replayed/reimplemented tasks.
- `devenv shell -- cargo test --locked --lib`:166 passed (`lib.log`).
- `devenv shell -- cargo check --all-features --locked`:exit0 (`check.log`).
- `devenv shell -- cargo fmt --check`:exit0 (`fmt.log`).
- `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`:
  exit0 (`clippy.log`).
- `devenv shell -- cargo test --all-features --locked`:NORMAL final gate exit0
  **598 standard +138 SSH =736 passed**, library166/discovery59/transport92
  (`test.log`); once after final meaningful tree changes, no serialization/retry.
- `devenv shell -- cargo run --locked --bin manyhands-cli`:exit0/no window
  (`cli.log`). Final normal gate ended2026-10-06T17:30:54Z.
- `git diff --check`:exit0; checkpoint leaves clean worktree/index.

Task 4 assigned integration evidence is now complete and **review ready**, not
independently accepted. Original-base..final-HEAD review diff/head/status artifacts
regenerated at the same `/tmp/manyhands-cycle05-task4-review{.diff,-head.txt}`
paths. Remaining Task 5 obligations ONLY: full two-clone publication matrix,
receive-side effect/disconnect controls, full hostile durable-store/privacy
matrix, focused new acceptance target and actual five-native-target CI after
publication permission. Native CI/whole-Cycle review remain pending, never claimed
from local Linux evidence. Stop after Task 4; independent reviewer is next gate.
