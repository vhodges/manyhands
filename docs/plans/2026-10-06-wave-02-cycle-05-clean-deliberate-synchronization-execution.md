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
- Task 4: complete; parent accepts full range through `ca88e002537e919899a13d61ab784d459206c953` with retained normal-gate timing qualification (below).
- Task 5: initial candidate through `907dd4204984ccb52125a6f44b990dd27ac08b7f` independently BLOCKED for two P1/three P2 proof gaps. Correction implemented at `575f657c21c16323fc64a367acef93608e528ec2`, focused23/normal772 and static/CLI gates pass; fresh independent re-review and parent acceptance pending.
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

## Task 4 independent review BLOCK and endpoint correction — 2026-10-06T18:09:22Z

Independent reviewer blocked clean completion HEAD
`46e5ad835aca3b9a624a87852bcfe35d03206f6d`, exact original-base range
`5ee6bd8e3b0b5c8b2812f177560cd7002fb52454..46e5ad835aca3b9a624a87852bcfe35d03206f6d`.
Report: task4-completion/review.md in the retained review artifact directory.
Prior acceptance rejection, review BLOCK and successful721/736-case evidence
remain retained, not relabelled as review acceptance. Parent authorized ONLY
both P1 corrections and demonstrated same-invariant proofs; Task 5 not started.
No worktree/branch/rebase/stash/reset/clean/publication/merge/closure/cleanup.

### Findings disposition and exact public-service red/green proofs

| Review finding / invariant | Correction and actual named proof |
| --- | --- |
| P1.1 same-ID restart can adopt changed pushurl under old generation | `synchronization_same_id_restart_endpoint_changed`: inject complete-Fetch batch persistence failure to leave FetchPrepared; change ONLY pushurl to a separately pretrusted independent destination with absent primary; explicit same-ID restart fails, current generation increases, immutable old row generation remains, neither endpoint receives new auth, actual local refs/index/bytes unchanged and new destination absent. |
| P1.2 BeforePush can redirect effect to endpoint lacking prior proof | `synchronization_before_push_endpoint_changed`: actual hook redirects A->B at BeforePush; service rejects, B primary remains absent, A retains original OID, bytes/refs/index preserved, durable PushPrepared/candidate retained and no authority. |
| P1.2 AfterPushReturn can fabricate publication from B equality | `synchronization_after_push_return_endpoint_changed`: real A acceptance asserted inside hook, delete A primary and redirect to independently trusted B already holding candidate; service returns RecoveryRequired, retains PushReturned/candidate with NULL authority, preserves A absence/B equality/local bytes and performs no discovery. |
| Frozen generation-establishing snapshot, not recaptured baseline | `synchronization_initial_snapshot_not_rebased`: BeforeFetch changes endpoint after snapshot/generation establishment; first call rejects without SSH; same-ID restart fences by advancing current generation rather than adopting new baseline. |
| Outer check is insufficient; actual scope preparation must be pinned | `synchronization_scope_prepare_endpoint_race`: deterministic test-only ActionSnapshotChecked hook redirects between outer comparison and driver preparation; expected endpoint rejects BEFORE new authentication/prompts, zero typed push calls, both refs/local state preserved. |
| Full snapshot check INSIDE authenticated scope | `synchronization_authenticated_action_snapshot`: Push remains explicitly pinned A but Fetch configuration changes during Push Authenticated hook; per-call Push endpoint still valid; full action expectation rejects before effect, no false authority or local change. |
| Credential prompt and cancellation boundary | `synchronization_prompt_action_snapshot_and_cancel`: selected encrypted key prompts once; provider-return hook changes Push endpoint or cancels owned action; scope rejects/cancels before transfer effects, preserving physical state and remote absence. |
| Cancelled replay must precede live config inspection | `synchronization_cancelled_replay_before_config`: cancelled exact ID then invalid live config; explicit replay stays Interrupted and SSH-free. |

All four initial service regressions failed red on their intended assertions
before implementation (`red-{same_id_restart_endpoint_changed,before_push_endpoint_changed,
after_push_return_endpoint_changed,cancelled_replay_before_config}.log`). Tests
were not weakened. Additional scope/prompt proofs cover the same invariant. The
only intermediate warnings were private-interface visibility for the narrow
expectation bridge; visibility was reduced to repository scope, not suppressed.

### Narrow implementation

- Same-ID terminal/authoritative/incomplete inspection stays ahead of configuration
  reads. For a nonauthoritative explicit restart, configure_endpoints reconciles
  actual digest/generation BEFORE restart ownership. Old action generation is
  immutable. The exact snapshot establishing generation is retained in process.
- One synchronization scope helper wraps ALL action network calls. It checks owned
  cancellation/generation and original full configuration before/after calls and
  inside the authenticated closure after prompts. A process-only SshScopeExpectation
  binds endpoint plus selected ID/path/source to actual preparation before auth;
  the adapter validates the actual anonymous remote handle against that expectation.
  No raw endpoint/path/key/passphrase appears in records/errors or Debug.
- Before/after Push effects and fresh verification cannot re-resolve a new action
  baseline. Endpoint change after possible acceptance retains intent and returns
  recovery. Original snapshot checks precede durable verification/classification.
  No rollback/reset/clear/false publication/index handoff. Existing generation-
  qualified Push-history guard and frozen local-only matcher remain intact.
- Existing ordinary transport entry points delegate with no expectation; their
  selected-key/reconnect/host-trust policy is unchanged. No schema/journal/public
  API/dependency/frontend/Task5 target/receive fixture/nativeCI expansion.

### Validation / review-ready correction

Logs outside repository: `/tmp/manyhands-cycle05-task4-endpoint-fixes/`.
- Focused service: `devenv shell -- cargo test --locked --test ssh_transport
  synchronization_`:27 passed (`focused-final.log`),8 added +19 retained cases.
- Relevant library:166 passed (`lib.log`, before final expectation refinement;
  final normal gate independently passes166 on final tree).
- Final `devenv shell -- cargo check --all-features --locked`:exit0,
  `cargo fmt --check`:exit0, `cargo clippy --all-targets --all-features --locked
  -- -D warnings`:exit0 (`{check,fmt,clippy}-final.log`).
- Final NORMAL `devenv shell -- cargo test --all-features --locked`:exit0,
  **598 standard +146 SSH =744 passed**, library166/discovery59/transport100
  (`test.log`); once after final meaningful changes, no serialized/ignored retry.
- CLI `devenv shell -- cargo run --locked --bin manyhands-cli`:exit0/no window
  (`cli.log`). Final gate ended2026-10-06T18:09:22Z.
- `git diff --check`:exit0; coherent correction commit leaves clean index/worktree.

Both P1s have source corrections and actual red/green proofs; disposition is
**corrected, pending independent re-review**, NOT self-accepted. Original-base
review diff/head artifacts regenerated at the same task4-review paths, plus
focused46e5..correction diff. Client counters/real ref observations do not claim
receive-side transaction/disconnect proof. Task5 full fixture/privacy/nativeCI
and whole-Cycle review remain pending; ticket stays open. Stop after Task4.

## Task 4 re-review BLOCK / no-remote ID correction — 2026-10-06T18:39:34Z

Independent re-review of clean HEAD
`6166c66fefce681478eeca18fcffc5ad605fe802` explicitly confirms BOTH previous P1
endpoint defects fixed. Those dispositions and prior721/736/744-case evidence
remain intact. Fresh BLOCK (task4-endpoint-fixes/review.md): live noRemote path
could adopt an existing nonauthoritative remote ID as tagged local PublishPending,
bypass typed-target validation and poison subsequent replay by forbidden ID
coexistence. Parent authorized only this boundary and atomic binding correction;
Task4 remains unaccepted, ticket open, no Task5/publication/lifecycle work.

### Finding-to-proof disposition

| Invariant | Actual proof / disposition |
| --- | --- |
| Existing same-ID remote action cannot become local-only after removal | `synchronization_existing_id_remote_removed`: actual complete-Fetch persistence fault leaves primary FetchPrepared; change only pretrusted pushurl; same-ID restart increments generation, retains old Interrupted row; remove publication_remote and COMMIT configuration; baseline AFTER fixture changes; explicit same-ID restart and ordinary replay return RecoveryRequired. No local refresh row, zero discovery observation-hook calls/authentication changes, both actual remote refs/absence and physical local HEAD/refs/index/files/status preserved. Failed red on intended RecoveryRequired assertion before implementation. |
| Different existing materialized typed target cannot reuse that ID | `synchronization_existing_id_remote_removed_changed_target`: same sequence plus actual authoring/materialization of another context; target switch to that context returns OperationMismatch on restart AND ordinary replay before fallback. Same no-row/observation/auth/ref/byte proofs cover primary AND context. Failed red on intended mismatch assertion before implementation. |
| Inspection-none does not authorize later binding; ALL remote phases reject | `local_synchronization_binding_rejects_remote_collision_after_none_inspection_in_every_phase`: fresh database per phase; observe None, insert colliding remote envelope BEFORE direct binder; reject and retain no refresh row/physical state for all15 schema-supported phases, including Completed/Interrupted/Cancelled/Failed. Structural phase fixtures prove insertion policy, NOT remote effects/authority. Failed red at Completed against delegation to old ordinary policy before specialized transactional guard. |
| Stable root identity before invalid current configuration | `existing_remote_id_rejects_other_root_before_invalid_live_configuration`: same cache contains two registered roots; wrong-root ID with invalid live configuration returns OperationMismatch, not live-config fallback. |
| Authoritative exact-ID/index-only refresh must remain compatible after config removal | `synchronization_authoritative_replay_remote_removed`: actual publication followed by injected refresh-completion failure leaves authoritative IndexPending; remove remote/commit clean configuration; replay completes ORIGINAL authority/OID via ordinary empty-target refresh, then completed replay skips observation/SSH. Physical state preserved, matcher remains empty rather than tagged. |
| Legitimate new-ID local and frozen original-OID replay | Retained unit `no_remote_refreshes_same_id_without_remote_envelope_or_git_mutation`, `no_remote_index_pending_retains_exact_outcome_and_replays_refresh_only`, `local_only_replay_freezes_oid_and_rejects_target_or_plain_refresh_reuse` all green. |
| Prior endpoint/history/snapshot/preparation/prompt/cancellation corrections | All27 existing service cases retained/green; now30 total focused SSH service cases. |

Correction: validate remote ID/root and frozen typed target BEFORE all replay,
restart and live selection; authoritative/cancelled branches remain before live
config. Existing nonauthoritative remote + noRemote returns recovery, never
binding/discovery. Specialized begin/reconcile checks ANY same-ID remote row
inside its IMMEDIATE transaction under binder's existing cache-write guard;
ordinary entry point passes the unchanged policy. No schema/second journal/public
API/dependency/frontend/Task5 fixture/nativeCI expansion. No raw endpoint/source,
credential or canonical/worktree contents persisted/output by the correction.

### Actual red / focused green evidence

Logs: `/tmp/manyhands-cycle05-task4-id-boundary/`.
- Both public-service BEFORE-implementation reds exit1:
  `red-synchronization_existing_id_remote_removed.log` and
  `red-synchronization_existing_id_remote_removed_changed_target.log`. Existing
  harness emits fixed case/assertion-line diagnostics, no raw server text.
- Atomic binder intended assertion red: `red-atomic-binder.log`, Completed accepted
  by old ordinary policy. Earlier test-authoring compile attempt used incorrect
  internal helper names/arguments; retained separately as
  `atomic-binder-compile-attempt.log`, NOT counted as red evidence. Corrected to
  existing helpers without weakening assertions.
- `devenv shell -- cargo test --locked --lib repository::remote::sync::tests`:
  15 passed (`lib-focused.log`),2 added +13 retained.
- `devenv shell -- cargo test --locked --test ssh_transport synchronization_`:
  30 passed (`focused.log`),3 added +27 retained.

### Final validation: first gate failure RETAINED, authorized second gate

Final check/fmt/clippy pass after final meaningful Rust changes:
`devenv shell -- cargo check --all-features --locked`, `cargo fmt --check`,
`cargo clippy --all-targets --all-features --locked -- -D warnings` all exit0
(`check.log`, `fmt.log`, `clippy.log`).

First NORMAL `devenv shell -- cargo test --all-features --locked` FAILED:
library168 passed; discovery_rebuild58/59 passed then
`services_sharing_a_corrupt_cache_replace_it_once` unwrapped RepositoryBusy at
`tests/discovery_rebuild.rs:2417` (`test.log`), before transport SSH target. CLI
skipped by the && chain, not a CLI failure. No retry/suppression/serialization
before explicit supervisor authorization.

Supervisor authorized exact diagnosis + NORMAL discovery target and ONE further
NORMAL full gate if both pass, no unrelated fix/timeout/sleep/ignored assertion.
Inspection: Busy originates from shared coordination::acquire timed try-lock
(250ms budget), used by repository and cache leases; diagnostic alone does not
identify which lock timed out. Test releases first service's observation barrier
concurrently with second rebuild. Rebuild acquires repository leases before begin
and after observation; begin_operation calls ordinary begin_or_reconcile_operation
with unchanged policy=false. New tagged binder is not called. Coordination,
rebuild, discovery test source UNCHANGED from correction base. This is a
**timing-sensitive validation failure**, NOT proven unrelated/pre-existing;
source and green reruns alone are insufficient to claim baseline attribution.

Authorized exact failing test (`--test discovery_rebuild
services_sharing_a_corrupt_cache_replace_it_once -- --exact`) passed1
(`discovery-exact.log`); whole `--test discovery_rebuild` with NORMAL threading
passed59 (`discovery-target.log`). No meaningful Rust changes between static
passes, failed full gate, narrow diagnosis and second full gate. Exact Rust diff
against6166 base saved as `rust-identity.diff`, SHA256
`cc22d3e7d7ef90dc6f9e76d6ac18c81042c776f4dad67bad695574fe6a57cbc4`
(`rust-identity.sha256`), rechecked after second gate; correction commit has that
same Rust content. No baseline rerun was performed or claimed.

ONE authorized second NORMAL `devenv shell -- cargo test --all-features --locked`
passed **600 standard +149 SSH =749 cases**, library168/discovery59/transport103
(`test-second.log`), ended2026-10-06T18:38:58Z. This is SECOND-attempt success,
NOT a once-only successful full gate. CLI then separately
`devenv shell -- cargo run --locked --bin manyhands-cli` exit0/no window (`cli.log`).
Both full attempts remain retained and must be visible to reviewers.

`git diff --check` passes; coherent correction commit leaves clean existing
branch/worktree/index. Original-base Task4 diff/head status regenerated at prior
paths; focused6166..final correction diff supplied. Fresh P1 corrected with actual
proofs, **pending independent re-review/parent acceptance**, not self-accepted.
Client auth observations/ref checks and scan-hook/control-flow evidence do not
claim receive-side transaction counts or full scanner entry instrumentation.
Task5/full privacy/receive-side/nativeCI/whole-Cycle review remain pending. Stop.

## Task 4 parent acceptance with notes — 2026-10-06T18:49:12Z

Accepted the complete original Task 4 range
`5ee6bd8e3b0b5c8b2812f177560cd7002fb52454..ca88e002537e919899a13d61ab784d459206c953`.
Independent retained read-only reviewer verdict: **OK with notes**, reviewed exact
HEAD `ca88e002537e919899a13d61ab784d459206c953`; no blocking findings remain.
Review artifact:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/7c0638a5-3e9b-42f4-a8a0-136b11f587cf/task4-id-boundary/review.md`.
Parent verified exact HEAD, clean worktree/index, and main ancestry at the existing
Cycle preflight base `29f3f5a25957836a8513cba8a318306e1b928063`; no reimplementation,
new worktree, branch change, or unnecessary broad rerun.

All three independently reported P1s are closed: generation reconciliation before
same-ID restart; frozen original endpoint/authenticated-scope and post-push proof;
and historical target/root validation plus atomic any-phase remote-ID exclusion
from tagged local-only binding. Earlier actual context deletion, endpoint-qualified
Push history, safe checkout/ref mismatch, cancellation/ambiguity, and exact-ID
index-only replay proofs remain intact. Coverage comprises 30 focused service SSH
cases, 15 focused private cases, and the retained full integration range.

Acceptance DOES NOT erase the first NORMAL gate failure. The initial full run
failed at discovery's shared-corrupt-cache RepositoryBusy assertion. Exact-case
and normal discovery-target diagnosis passed; one expressly authorized second
NORMAL full run on identical Rust content passed **600 standard +149 SSH =749**.
Check/fmt/clippy and separate CLI pass. The failure remains timing-sensitive,
**not proven unrelated/pre-existing**, and must be included in Task 6/whole-Cycle
review; no discovery/timeout/assertion policy was changed to obtain this result.

Proceed to approved Task 5 only: dedicated real two-clone acceptance target,
receive-side effect/rejection/disconnect controls, complete recovery/index handoff
and hostile durable-store/privacy matrix, and native-job target configuration.
Existing client counters are not server transaction evidence. Actual five-native-
target CI, desktop smoke where available, final gates/whole-Cycle review, and
publication remain pending. Push/PR, merge, closure and cleanup are unauthorized;
ticket stays open. This acceptance checkpoint changes documentation/comment only.

## Task 5 — real wire acceptance candidate — 2026-10-06T19:34:00Z

Exact task base: `0601c6bf76485fa98e9935e36266631a32efa793`.
Implementation checkpoint: `9dcd410f87ff3414e99d9d0d26ee611bc484ad59`
(`test: prove real synchronization outcomes and replay`). This evidence-only
checkpoint follows it. Final full HEAD/clean status are bound in
`/tmp/manyhands-cycle05-task5-review-head.txt`; exact **base-to-final-HEAD** diff
in `/tmp/manyhands-cycle05-task5-review.diff`. Independent read-only review and
parent acceptance are REQUIRED and pending; this is not whole-Cycle acceptance.

Read AGENTS, implementing-a-cycle skill/reference, all approved Cycle/design/
implementation/execution artifacts, fixture/custom-host seams, and accepted
Task 4 source/regressions. Reused exact branch/worktree and already-completed
fresh-main preflight; no fetch, rebase, branch/worktree change or Task 1–4 edit.
Main's unrelated files and other worktrees are untouched. No production source,
public policy, schema, dependency, Cargo.lock/devenv input, frontend, scheduler,
force/rollback/merge/reset/materialization/promotion or new journal was added.
Canonical setup/clone/commit/ref controls are git2, not shell/system Git setup.
Only existing fixture helpers launch shell-free upload-pack/receive-pack children.

### Owned seams and supervisor rulings

- Added `tests/remote_synchronization.rs`, custom `harness=false` target following
  existing real pre-thread initialization/source-inclusion conventions. All cases
  run in isolated owned children; no ignored/native-placeholder cases.
- Minimal `tests/support/ssh_remote.rs` controls plus **explicitly approved**
  `tests/support/ssh_server.rs` relay extension (the actual child stream seam).
  `tests/support/mod.rs` needed no change. Existing SSH-host cases/source remain
  untouched and pass in the full gate.
- Closed bounded pkt-line parser retains at most one <=65,520-byte packet and
  32 validated branch commands, stops at command flush, and ignores opaque pack
  bytes. Fragmentation/flush/oversize regression runs in the new host. Default
  stream bytes/order remain unchanged. One-shot CAS-race injection holds at most
  81,904 original prefix/first-read bytes, verifies the original advertised old
  OID, advances ONLY the owned bare primary, then forwards original client bytes.
- `ReceiveUpdate { reference, old_oid, new_oid, accepted }` is fixture-only safe
  metadata. Attempted commands are distinct from receive-pack advertisement-only
  sessions. Acceptance additionally requires completed helper success, a changed
  command OID, and actual owned remote ref == candidate. Each acceptance test
  pairs these counters with independent before/after bare refs. Counts do NOT
  claim process invocation equals transaction, an advertisement equals push, or
  direct fixture/peer ref controls are receive-pack transactions.
- AfterReceivePack loss now applies only to a real parsed update, waits (bounded
  five seconds) for child completion, proves the actual new ref, and withholds
  the status before it reaches the client. Fixture shutdown still owns/reaps only
  its helper children. Advertisement-only sessions cannot trigger this effect.
- Supervisor accepted complementary nonFF evidence: genuine divergent Push
  ancestry rejects BEFORE any update command; an already-sent ordinary update
  against advertised old OID is rejected by the **real receiver** after a
  competing divergent write. The latter is **EXPECTED-OLD/CAS race rejection**,
  NOT a denyNonFastForwards-policy response. Separate native
  receive.denyCurrentBranch=refuse proves receiver-policy refusal. No forced
  refspec or forged command was used to manufacture server rejection.
- CI adds `--test remote_synchronization` to the existing shared headless command,
  therefore all five existing native matrix entries retain it. No dispatch/push
  or actual native execution. Configuration is NOT platform proof.

### Coverage matrix: actual named cases and evidence

All names below are in the dedicated new target unless explicitly qualified.
Every public operation uses the real service and authenticated disposable SSH;
`World` creates TWO actual authenticated clones. Peer primary and context updates
also traverse real SSH. Bare-owned fixture writes intentionally model competing
changes/deletion/divergence, not imaginary accepted receive transactions.

| Requirement | Named case(s) | Wire/local/recovery/privacy evidence |
| --- | --- | --- |
| Primary current, FF, locally ahead | `primary_current_fast_forward_local_ahead` | Current/FF send 0 updates; peer advancement transfers over SSH; branch, required file bytes and index tree equal advanced commit; local-ahead sends exactly 1 accepted ordinary old->candidate update; FETCH_HEAD bytes and unrelated tracking ref preserved. |
| Context first/current/FF/local-ahead | `context_first_current_fast_forward_local_ahead` | First sends 1 accepted zero->context update; current sends none; second authenticated clone fetches context and publishes next commit; target bytes/ref/index reflect FF; locally ahead sends 1 accepted next->candidate update; primary ref/HEAD/index/tracked bytes remain equal. |
| Ordinary receiver-policy rejection and only unfinished effect retry | `receiver_rejection_and_exact_restart` | Real command received, candidate not accepted, remote old ref and complete local physical image unchanged; removal of receiver refusal + explicit same-ID restart adds exactly 1 accepted update, not a duplicate accepted effect. Hostile receiver text redacted. |
| Divergent receiver expected-old race | `receiver_expected_old_divergent_race` | Actual unchanged original old/new command received after one competing owned write; sibling commit ancestry proved; 0 accepted candidate updates, competing ref remains; no authority/refresh, PushPrepared retained, local image unchanged. |
| Distinct endpoints with real ancestry | `distinct_push_ancestry_and_tracking` | Separate exact host approvals; shared real commit objects seed genuine ancestor at Push; 1 accepted Push update, unchanged Fetch ref/tracking evidence; Push-only sibling object downloaded, both divergent graph relations false, 0 further sent commands; typed rejection, no local mutation. |
| Dirty/conflicted primary | `primary_dirty_conflicted_preservation` | Real staged index conflicts or dirty tracked bytes; full local refs/HEAD/index/files/status digest unchanged; 0 new authentication or update. |
| Dirty/wrong/detached context | `context_dirty_wrong_detached_preservation` | Actual deterministic linked worktree, bytes/index/refs/status unchanged in all three modes; 0 network authentication. Accepted Task 4 wrong-registered-worktree regression remains unchanged. |
| Primary and remote-context divergence | `primary_divergence_preservation`, `context_remote_divergence_preservation` | Genuine sibling commits; typed MergeRequired; complete physical images unchanged; no update command. |
| Virtual context FF then primary divergence | `context_virtual_primary_divergence_preservation` | Context could FF to remote, but divergent primary blocks BEFORE any local update; both physical images and original context OID preserved. |
| Remote changes during fetch | `remote_change_during_fetch` | Existing private post-download seam advances actual owned remote between advertisements; ExternalChange before local effect; unchanged physical state. |
| Verified first Push then immediate deletion | `verified_context_immediate_deletion` | Complete verified first publication then actual deletion without intervening Fetch-present observation; RemoteContextDeleted, 0 additional updates, absent server ref and local state preserved. |
| Incompatible-generation history | `incompatible_generation_history_unknown` | Distinct separately trusted absent Push destination after verified publication/deletion; old Push generation not borrowed; HistoryUnknown and no effect. Existing accepted ordinary HistoryUnknown regression also preserved. |
| Lost acknowledgment AFTER accepted effect | `post_accept_disconnect_exact_restart` | Actual old->candidate ref, completed helper and status-withheld flag; error is NOT authority; ordinary duplicate performs 0 calls; fresh service + explicit exact-ID restart proves candidate on same original Push; total candidate accepted effects = 1, unchanged commit inventory/local image, one refresh record. |
| Persistence fails AFTER accepted effect | `post_accept_persistence_exact_restart` | Trigger aborts PushVerified write after real receiver acceptance; same exact restart proof and one-effect/one-refresh/unchanged-commit assertions as disconnect. |
| Published/AlreadyCurrent discovery failure | `published_index_pending_refresh_only`, `already_current_index_pending_refresh_only` | Exact authoritative IndexPending; one initial discovery callback; retry only refreshes (second callback), no commands/auth/updates/ref/worktree effect; endpoint changed after authority cannot replace frozen outcome; completed replay has no scan. |
| Completed refresh, index flag write failure | `completed_refresh_index_flag_no_rescan` | Exact authority retained; repeated failing-flag and successful same-ID replay have 0 additional scans/commands/auth/updates, unchanged physical image. |
| Raw secret-bearing endpoint input | `hostile_endpoint_redaction` | Actual configured hostile URL rejected before authentication, unchanged local image; formatted Debug/Display and durable-store scan exclude raw URL. |
| Parser/control fidelity | `receiver_command_fragmentation` | Command parsed identically under 1/2/3-byte and larger fragmented reads; capability suffix bounded; pack bytes ignored; read-only flush produces no update; oversized packet fails. Other control regressions are real wire cases above. |
| Frozen noRemote/id collisions/endpoint switching | retained `tests/ssh_transport/synchronization.rs` cases | Accepted Tasks 1–4 unchanged: authoritative/existing-ID remote removal, changed target, atomic tagged binder, same-ID restart generation, BeforePush/AfterPushReturn endpoint changes, immediate-deletion/ambiguity rulings. Full gate runs all 30 original synchronization cases; new receiver counters do not rewrite their assertions. |

### Privacy inventory and counter limits

Actual original plaintext/encrypted private-key buffers and long encoded key
fragments, actual provider passphrase, raw Fetch/Push/secret-bearing URLs, hostile
server text, canonical ticket body AND full canonical Markdown bytes are nonempty
probes. Target worktree paths are additionally scanned in all remote/local
operation and observation rows and in synchronization result/error Debug/Display.
Every `World::sync` inventories nonempty durable rows and recursively inventories
all generated application-data stores, including filenames. All stronger path
checks also run after context preflight/error operations. Probe inventories are
retained for the outer custom-host raw-output scanner across failures/restarts.
**Historical claim corrected:** at the reviewed `907dd420` candidate, this did NOT
activate output scanning for new case names and later Worlds overwrote inventories.
The prior raw-output-completeness claim is invalid; see the BLOCK/correction below.
Generated-store/row scopes and the original769 normal-green results remain valid.

Original private-key files, Git config/object/worktree/canonical sources are
intentional sources and excluded from generated-store scans. Root identity,
shared-key source paths and public discovery/registry worktree path columns are
legitimate metadata; full generated-store byte scans omit ONLY the worktree-path
probe, not any secret/URL/server-text/Markdown probes. Strong target-path scanning
is applied to EVERY text/blob cell in `remote_operation_records`,
`operation_records`, and `remote_ref_observations`, where such paths are forbidden.
Those records are not exempted as discovery metadata. No empty/tautological scan
or secret-rendering failure diagnostic is used.

Both post-accept cases retain a live connection and exercise actual synchronization
writes with WAL enabled ONLY on their owned test database. A real VACUUM diagnostic
backup and active rollback journal on that diagnostic copy are generated and
scanned; explicit assertions require nonempty WAL and journal inventory. Other
cases use the service's existing database mode. No application schema/journal or
production persistence change. After replay, raw formatted results plus rows and
all generated files are re-scanned. Discovery callbacks count public scanner entry,
not every internal file read; receive counters prove only tested candidate updates
in this owned sequential fixture, not a general transaction audit under unrelated
concurrent writers. Object inventory verifies no duplicate commit objects on
accepted-effect restart; Git content-addressing and unchanged local images/ref
OIDs also prevent a manufactured checkpoint/commit claim.

### Tests-first and honest intermediate failures

Logs outside repository: `/tmp/manyhands-cycle05-task5/`.
Initial `red.log` exit101 includes missing fixture `receiver_command_fragmentation`
control plus two test-authoring errors (hook enum spelling, non-Send counter).
This is **missing-control compilation evidence**, NOT a productionbehavior red.
No production defect/fix was claimed or performed. Wire authoring failures retained:
missing local author identity, genuine initial ancestor seeding across distinct
fixtures, per-endpoint approval handling, incorrectly expecting identical resolved
pushurl to change generation, and a WAL assertion before choosing an actual WAL
fixture mode. These were corrected only in owned setup/tests without weakening
behavioral assertions. A zero-match preliminary race-filter run in
`receiver-race.log` is **invalid coverage evidence**; `receiver-race2.log` runs 1
real case and passes. All intermediate logs remain available, not relabeled green.

### Final unchanged Rust tree verification

| Exact command | Result / log |
| --- | --- |
| `devenv shell -- cargo test --locked --test remote_synchronization` | **20 passed**, no ignored cases; `focused-final.log` |
| `devenv shell -- cargo check --all-features --locked` | exit0; `check.log` |
| `devenv shell -- cargo fmt --check` | exit0; `fmt.log` |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | exit0; `clippy.log` |
| `devenv shell -- cargo test --all-features --locked` | **FIRST NORMAL Task5 gate exit0**, `test.log`; 600 standard +169 SSH = **769 passed** |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | exit0, no window; `cli.log` |
| `git diff --check` | exit0 before implementation commit and evidence checkpoint |

Full test interval: `2026-10-06T19:23:45Z`–`2026-10-06T19:33:20Z` (`test.meta`).
Normal default harness concurrency; no test-thread serialization, diagnostic
instrumentation, timeout/assertion changes, ignored cases or broad retry. Includes
168 library,59 discovery,600 total standard,15 remote_observation,20 new sync,
31 ssh_fixture,103 ssh_transport cases. All existing hosts and source regressions
passed. No redundant unchanged broad run. Static gates preceded this full run;
only documentation/comments change afterward.

This successful Task5 run DOES NOT erase Task4's first normal-gate Busy failure.
That earlier failure remains timing-sensitive, NOT proven unrelated/preexisting,
and its expressly authorized second-run qualification remains required for
whole-Cycle review. All five actual native targets remain **pending authorized
publication**, not passing from YAML/Linux. Task6 desktop/whole-branch acceptance
was not attempted. Ticket stays open; no push/PR/dispatch/merge/closure/cleanup.
Next: fresh independent read-only Task5 review of exact base-to-final HEAD and
parent acceptance; then stop for the parent-owned next gate.

## Task 5 review BLOCK and privacy-proof correction — 2026-10-06T20:10:52Z

Original task base remains `0601c6bf76485fa98e9935e36266631a32efa793`.
Correction base/reviewed candidate: `907dd4204984ccb52125a6f44b990dd27ac08b7f`.
Correction implementation: `575f657c21c16323fc64a367acef93608e528ec2`
(`test: enforce synchronization raw-output privacy`). Final evidence HEAD and
clean status are bound in `/tmp/manyhands-cycle05-task5-review-head.txt`; the
original-base-to-final-HEAD diff is regenerated at
`/tmp/manyhands-cycle05-task5-review.diff`. Correction-only diff is additionally
`/tmp/manyhands-cycle05-task5-privacy-fixes-review.diff`.

Independent review BLOCKED Task5 acceptance: two P1 privacy-proof mechanisms and
three P2 assertion gaps, **not a demonstrated production defect**. Exact review:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/54172663-df53-43c1-928c-820aba6a6c5f/task5/review.md`.
Parent authorized narrow additional `tests/support/ssh_harness.rs` and
`ssh_privacy.rs` seams; correction touches those, `remote_synchronization.rs`,
ledger and valid canonical comments only. Existing fixture/receiver source,
Tasks1–4 production/regressions, dependency/lockfiles/workflow and public/schema
policy remain unchanged. Existing branch/worktree and preflight retained.
No production spill was uncovered with the corrected raw scanner; no product fix
or suppression/filtering was introduced. Parent has NOT accepted Task5.

### Corrected claims and finding-to-test inventory

The earlier assertion that saving probes provided full new-target raw output
privacy was **invalid**: the runner scanned ONLY case `transport_privacy`; none
of the 20 new names matched. Numbered/truncating saves also lost earlier Worlds'
probes. These are actual fixture-proof defects. Preserve the original907dd/769
logs/evidence and review rather than labeling them full privacy acceptance.
Original generated-store/row/Debug/Display/WAL/journal/backup scopes remain sound.

| Finding | Correction / actual named test | Capture/assertion evidence |
| --- | --- | --- |
| P1 raw stdout/stderr never scanned | `run_with_output_privacy(CASES, OUTPUT_CONTROLS)` explicitly enables required raw scanning for EVERY dedicated-host case. `synchronization_raw_capture_privacy` | Real nested isolated children emit controlled bytes beyond20KB into stdout OR stderr, return success OR failure. Four captures must return safe typed OutputPrivacy, not Child/Fixture. A clean failing child separately returns Child. Both full captured buffers are scanned before numeric filtering/rendering and before propagating child failure. |
| P1 earlier Worlds overwritten | Content-addressed `create_new` saves preserve the case-wide union; duplicates are byte-verified without truncation. `synchronization_probe_union_privacy` | Separate nested URL/key children each create THREE real Worlds (six authenticated clones), assert actual first URL/key differ from later ones, repeat later saves, assert full first URL and first private-key buffer still exist in loaded union, then emit only that first secret into raw stdout/stderr. Actual runner capture must reject as OutputPrivacy. No raw bytes rendered. |
| P1 fail-closed inventory integrity | `synchronization_probe_inventory_fail_closed` | Actual nested missing inventory, correctly digest-named empty probe, non-file entry, and corrupt digest/content entry all return ProbeInventory before any rendering. Empty saver input/probe rejected; saver refuses existing corrupt content, rather than overwriting/guessing. Loader rejects symlinks/nonregular files and digest mismatch instead of silently ignoring malformed entries. |
| P2 context AlreadyCurrent command proof | `context_first_current_fast_forward_local_ahead` | Full receiver-update vector captured immediately before AlreadyCurrent, equality asserted immediately after it and BEFORE peer publication resets baseline. |
| P2 context FF actual index proof | `context_first_current_fast_forward_local_ahead` | Actual linked index `write_tree()` equals fetched commit tree immediately after FF and BEFORE `advance` can perform another checkout; update-count equality also asserted there. Existing ref/byte/primary image proofs retained. |
| P2 incompatible-generation NEW Push no effect | `incompatible_generation_history_unknown` | NEW destination update-command vector and accepted count captured before call, verified unchanged immediately after HistoryUnknown; actual NEW bare context-ref absence checked before AND after, alongside existing Fetch count and physical image. |

The three new top-level selftests execute **11 child-only controls** (four raw
leaks, one clean failure, four malformed/missing inventories, two first-World
leaks). Those controls are deliberately not ordinary passing cases; all are
executed inside bounded isolated runner captures on every native target running
the new target. No ignored/platform-skipped negative cases. Child-only registry
is the narrow bridge needed to test the real runner capture path, not an unrelated
`clean()` helper call. Parser-only case saves a nonempty fixed control inventory;
real synchronization cases retain their actual secret inventories.

### Raw capture order / storage boundaries

Existing `run(cases)` keeps prior default policy and `transport_privacy` scanning.
New opt-in mode calls the SAME actual process/capture/watchdog runner with required
privacy, so a missing inventory fails closed. Child status is recorded, BOTH raw
stdout/stderr are fully drained into existing bounded buffers and joined, then
inventory loaded and BOTH buffers scanned, then numeric-only diagnostics may be
rendered, then prior child status returned. Rejection/loader/capture errors return
closed safe enums/fixed messages with no raw output/probe printing. Existing
capture limit/watchdog/signal/bootstrap behavior is unchanged. Capture overflow or
I/O failure still fails closed without rendering, not falsely relabeled scan success.

Probe files are intentional fixture-only private input sources outside generated
app stores. Filenames are BLAKE3 digests using the existing dependency, never secret
bytes. Only create-new opens are used; already-existing regular contents must match.
Load requires a nonempty inventory, regular non-symlink root/files, matching digest
and nonempty bytes. Saving later snapshots/Worlds cannot truncate previous entries.
No app journal or production persistence behavior is added. Protected generated
store scopes and legitimate repository/key-source/discovery metadata exceptions
remain exactly as recorded for the original candidate. Receiver CAS remains
expected-old race rejection, never a fabricated nonFF-policy response; actual
accepted updates remain child-completion/ref proof, not advertisements/invocations.

### Meaningful controlled RED then GREEN

Logs: `/tmp/manyhands-cycle05-task5-privacy-fixes/`.

1. Added real runner bridge/control selftest against the unchanged old case-name
   predicate. `red-raw-capture.log`: compilation succeeds, command exit1,
   `synchronization_raw_capture_privacy` assertion1336 fails because actual nested
   successful stdout leak is accepted, not OutputPrivacy. Raw bytes remain in the
   nested captured buffer and are never tool output. This is a behavioral mechanism
   red, NOT a compile/authoring error.
2. Enable explicit scanner condition. `green-raw-capture.log`: same named test
   passes all success/failure stdout/stderr controls and clean-failure discriminator.
3. With raw scanner active but original truncating saver retained,
   `red-probe-union.log`: compilation succeeds, exit1, union assertion1391 fails
   because first real URL survives in captured stdout but was lost from inventory.
   Later World's saved URL is genuinely different. This is a distinct behavioral
   inventory red, not a side effect of the old scanner predicate.
4. Content-addressed create-new union makes `green-probe-controls.log` pass2
   (separate first-URL/key captures plus fail-closed inventories). Final strengthening
   includes explicit first-buffer membership and corrupt-content checks.
5. Final source `focused-final.log` passes **23** actual host cases with required
   raw-output scanning. Original20 wire scenarios/preservation/recovery/store
   privacy tests retain their assertions plus all three P2 additions.

### Source binding and final NORMAL gates

`source-identity.txt` associates implementation HEAD with the Rust content used
by focused/static gates and the subsequent full/CLI run. Stable original-base
Rust diff stored in `rust-identity.diff`; SHA256:
`7f0daff64f39ec0b1a94b496620f3d20608921cd15e45b08fd1b1fe9d2bd0abe`.
Recomputed after full/CLI, identical. Required command logs and `test.meta` are
retained; full metadata embeds actual575f full SHA and content hash, not merely
an informal clean-state assertion. No meaningful Rust change after these gates;
only ledger/comments added afterward.

| Exact command | Result / log |
| --- | --- |
| `devenv shell -- cargo test --locked --test remote_synchronization synchronization_raw_capture_privacy` | behavioral red exit1, then green1; `red-raw-capture.log`, `green-raw-capture.log` |
| `devenv shell -- cargo test --locked --test remote_synchronization synchronization_probe_union_privacy` | behavioral red exit1; `red-probe-union.log` |
| `devenv shell -- cargo test --locked --test remote_synchronization synchronization_probe_` | green2; `green-probe-controls.log` |
| `devenv shell -- cargo test --locked --test remote_synchronization` | final green23, `focused-final.log`; earlier complete23 log retained too |
| `devenv shell -- cargo check --all-features --locked` | exit0; `check.log` |
| `devenv shell -- cargo fmt --check` | exit0; `fmt.log` |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | exit0; `clippy.log` |
| `devenv shell -- cargo test --all-features --locked` | **FIRST NORMAL correction gate exit0**,600 standard +172 SSH = **772 passed**, `test.log` |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | exit0/no window; `cli.log` |
| `git diff --check` and exact committed-range diff check | exit0; final clean index/worktree bound by review-head artifact |

Full interval: `2026-10-06T20:00:39Z`–`2026-10-06T20:10:24Z`.
Default normal standard-harness concurrency, unchanged existing custom host policy;
no broad retry, serialization, timing/assertion weakening or ignored tests.
Includes existing library168/discovery59 and SSH remote_observation15/fixture31/
transport103 plus corrected sync23. Shared default runner/transport_privacy and
all other hosts pass unchanged. No redundant unchanged broad rerun.

Original Task5 normal769 success remains recorded, but did NOT prove these old
P1 claims. Task4's earlier first-normal Busy failure and expressly authorized
second-normal success also remain qualified: NOT proven unrelated/preexisting.
All five actual native executions remain pending authorized publication; Linux/
YAML do not prove them. No Task6, publication/dispatch, merge, closure or cleanup.
Ticket stays open. All five reported proof gaps corrected locally, **fresh independent
re-review and parent acceptance still required**. Stop for that gate.

## Task 5 parent acceptance with notes — 2026-10-06T20:18:42Z

Accepted the complete original Task 5 range
`0601c6bf76485fa98e9935e36266631a32efa793..0026ec993b891de7d3c595411974f635bd25ca2e`.
Independent read-only re-review verdict **OK with notes**, no issues found, all
five prior proof findings closed. Review artifact:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/c2276511-1b38-4a86-8e41-5060091009c6/task5-privacy-fixes/review.md`.
Parent verified exact clean HEAD and independently recomputed original-base Rust
content SHA256 `7f0daff64f39ec0b1a94b496620f3d20608921cd15e45b08fd1b1fe9d2bd0abe`.
All src/tests/manifests/locks/toolchain/CI content is unchanged from tested
implementation HEAD `575f657c21c16323fc64a367acef93608e528ec2`; subsequent commits
add evidence only. Reuse final gates on this identical tree rather than perform
an unnecessary unchanged broad rerun, per verification/delivery skill.

Accepted proof includes real two-clone receiver effects (advertisements separated
from ordinary update commands), rejection and honest expected-old race labels,
post-accept disconnect/persistence exact-ID no-duplicate replay, index-only handoff,
physical preservation, complete generated-store/row/WAL/journal/backup privacy,
and raw stdout/stderr scanning BEFORE filtering on success AND failure. Case-wide
probe union retains first-World keys/URLs; meaningful nested negative controls
and fail-closed inventories close the two P1s. Immediate context current/update,
FF/index-tree and new-Push-destination no-effect assertions close all three P2s.
No production/schema/public-policy/lockfile/dependency widening in Task 5.

Final focused23 and FIRST NORMAL correction full **600 standard +172 SSH =772**
pass; required check/fmt/clippy and CLI pass. Actual full metadata/source identity
are retained. Original769 pass did not prove the old privacy claim; its explicit
withdrawal remains. Task4's first NORMAL Busy failure and expressly authorized
second749 pass remain disclosed, NOT proven unrelated/preexisting. Carry these
qualifications to whole-Cycle review; no erased failures or synthetic baseline.

Proceed to parent-owned Task 6 final evidence/front-end smoke and fresh independent
whole-Cycle review. Active DISPLAY/WAYLAND variables and a present Wayland socket
were inspected; desktop launch smoke can run. Native five-target execution stays
pending publication authorization; workflow configuration is not execution proof.
No push/PR/dispatch, merge, closure or cleanup authority; ticket remains open.
This acceptance checkpoint changes documentation/comment only.

## Task 6 local verification and whole-Cycle review checkpoint — 2026-10-06T20:24:37Z

Final Rust/build/CI tree remains identical to tested implementation575f after
Task5 acceptanceafb36; parent independently recomputed the recorded SHA and
verified src/tests/manifests/locks/toolchain/CI equality. Reuse required final
check/fmt/clippy, first NORMAL772-case gate and CLI evidence rather than repeat
unchanged broad checks. Qualification remains: Task4 first NORMAL Busy failure
was retained; its authorized second-pass success does not establish causality.

Active DISPLAY and WAYLAND variables, existing socket and COSMIC Wayland session
were inspected. Parent ran exactly
`devenv shell -- cargo run --locked --features desktop --bin manyhands`, using
owned temporary XDG data/cache directories. Actual executable launched; captured
own-client protocol proves xdg_surface.get_toplevel and non-null wl_surface buffer
attach/commit. It remained alive12 seconds, with no panic/NoWaylandLib/startup
error, then parent terminated ONLY its owned smoke process group with SIGTERM.
No manual visual/feature interaction claim. Initial live regex expected @ protocol
IDs, whereas this backend uses #; false live-observer booleans were corrected by
inspection of actual captured requests, NOT an additional synthetic launch.
Protocol event payloads were removed from the owned diagnostic afterward; retained
artifact contains build lines/safe event summary and explicit postInspection.
Evidence: `/tmp/manyhands-cycle05-desktop-OecGTp/{desktop.log,result.json}`;
completed2026-10-06T20:24:37.878Z. No unrelated process or source change.

Whole-Cycle review is next, not yet approved. Parent final inspection raised an
unresolved no-discard concern: sync.rs uses CheckoutBuilder.safe() without an
explicit overwrite_ignored(false); locked git2 0.20.4 build.rs460–464 documents
ignored overwrite default TRUE, while preflight includes untracked but not ignored.
Review must examine incoming tracked-path collisions with existing ignored user
files/directories; no behavioral regression or production correction has yet
been run/made for this concern. Do not equate clean status plus safe() with a proof
that ignored user content is preserved. Route any finding to the implementer with
actual service red evidence before fixes; no broad runtime/privacy contract change.

Five native targets remain pending publication authorization. Whole-branch review,
any findings/fixes and associated verification, and native execution are separate
gates; no YAML/Linux/desktop smoke is native CI proof. Ticket stays open. No
push/PR/dispatch/merge/closure/worktree cleanup authorization exists.

## Task 6 whole-Cycle BLOCK / safety corrections — 2026-10-06T21:23:34Z

Read retained whole-review.md (task6 artifact650574f8), exact reviewed clean HEAD
`76fd280dcc2f7c300a8353fa3c7c844a3b8e9df2`, original main
`29f3f5a25957836a8513cba8a318306e1b928063`. Two source-backed/not-executed P1s:
ignored incoming checkout data loss, and atomic tagged-local LOCAL identity race.
Parent authorized actual service reds first and only demonstrated corrections.
Task4/Task5 accepted proofs are retained, not redone/weakened or self-accepted.

### P1 ignored content — actual behavioral reds and green coverage

`ignored_primary_file_collision` and `ignored_context_directory_collision`
failed BEFORE correction on the explicit ignored-private-bytes assertion AFTER
successful service integration: protected bytes were replaced/removed. Initial
red runs at rejection assertion also retained. Persistent common Git info/exclude
survives service reopen; no ephemeral add_ignore_rule. Incoming real authenticated
peer descendant tracks colliding path AND changes existing fixture.txt through
owned explicit git2 index setup (not a product force checkout/push). Both primary
and existing materialized/published context routes use current World/receiver/
append-only privacy controls, unchanged support fixtures.

Minimum production fix: existing checkout.safe().overwrite_ignored(false).
Locked expected-old ref transaction, old HEAD checkout order, LocalPrepared and
actual final proof are unchanged; no reset/rollback or blanket ignored rejection.

Twelve named actual service cases pass:
- `ignored_{primary,context}_file_collision`: ignored file vs incoming tracked file.
- `ignored_{primary,context}_directory_collision`: ignored directory/private child
  vs incoming tracked file; protected directory tree retained.
- `ignored_{primary,context}_incoming_directory_collision`: ignored file vs
  incoming tracked directory/child.
- `ignored_{primary,context}_symlink_file_collision` and
  `ignored_{primary,context}_symlink_directory_collision`: ignored symlink identity
  plus linked bytes/tree and owned target data retained. Actual Linux filesystem
  symlink creation succeeded for ALL four. Windows uses corresponding symlink
  APIs; a capability failure FAILS case, not silent native skip/passed proof. No
  native Windows/macOS execution claimed; all five native jobs still pending.
- `ignored_{primary,context}_noncolliding_control`: ignored artifact retained,
  successful real fast-forward, actual index tree equals descendant, no receive
  update from synchronization. This rules out rejecting all ignored content.

For all ten tested collisions: actual whole physical snapshot/local refs/HEAD/
index/tracked+ignored files/status identical AFTER failed call; ignored link/target
checks additionally explicit. Durable LocalPrepared, authoritative_kind NULL,
local branch remains old, actual remote descendant stays remote, receiver update
inventory unchanged (not just a private client counter). No false publication or
accepted push. Fixture setup updates are excluded by taking baselines AFTER real
peer pushes. Another incoming tracked change is included to observe partial
updates rather than assume none; these tested conflicts preserve all bytes.
Recovery remains explicit if other filesystem errors partially update checkout;
no universal atomic-filesystem or automatic rollback claim.

### P1 tagged-local LOCAL identity — actual after-None reds and disposition

Private thread-local operation-ID hook runs after absence inspection, BEFORE Git/
cache leases, removes itself before callback, and is compiled only for tests.
Allows actual competing service A to complete before losing binder B (not just
structural rows or timing sleeps).

- `local_binding_after_none_rejects_actual_other_target_service_authority`:
  actual A primary tag completes after B materialized-context None inspection;
  old binder reported unsupported B outcome. Red at intended recovery assertion.
  Green: B RecoveryRequired before discovery; physical primary/context Git images
  unchanged after A baseline, zero additional observation-hook calls, only A tag
  retained. A replays exact first outcome; B ordinary replay mismatches identity.
- `local_binding_after_none_rejects_competing_oid_then_replays_frozen_first_outcome`:
  A same target completes, then deliberate clean fixture commit advances OID
  before B binds. Red at recovery assertion. Green uses authorized REJECT competing
  OID option, not adoption/rebasing; baseline AFTER fixture commit unchanged,
  zero additional observation calls, subsequent correct exact-ID replay returns
  first frozen A outcome/OID, never B current OID.
- `local_binding_after_none_rejects_actual_plain_refresh_before_discovery`:
  actual ordinary refresh A succeeds with same ID/empty matcher after None;
  old tagged binder adopted it. Red at recovery assertion. Green B rejects before
  discovery, preserves ref/index and empty matcher; ordinary replay mismatch.
  Ordinary generic refresh was exercised successfully, not globally restricted.
- `local_binding_transaction_rejects_incompatible_existing_and_pending_rows`:
  structural transaction fixtures, red at target/completed. Green covers distinct
  tagged target, conflicting OID, malformed matcher, plain matcher, local action,
  cross-root and pending different/NULL-ID alias collisions across completed/
  created/observed/error/indexing states (completed unrelated IDs intentionally
  are not pending blockers). No row count/field or Git ref/index/byte mutation.
  These are policy fixtures, NOT remote effects or claim-owner execution proof.
- `local_binding_transaction_accepts_only_identical_complete_tag_without_rewriting`:
  identical root/action/full typed tag+full OID accepted across five states with
  same frozen OID and no row insertion/rewrite. Existing remote-ID ALL15-phase
  atomic guard, noRemote/frozen local replay and authoritative empty-target handoff
  proofs remain green.

Correction is within SAME cache-guarded IMMEDIATE begin/reconcile transaction:
strict exact LOCAL row identity and pending requested ID/root/action/full matcher
in tagged mode, including inactive completed rows; ordinary mode's lifecycle/
refresh/legacy aliases remain unchanged. Binder only returns computed OID if it
was newly stored or EXACTLY equals validated stored matcher. No new schema,
second journal, dependency/public policy, unrelated lifecycle/merge/polling code.

### Actual validation and source binding

Logs outside repository: `/tmp/manyhands-cycle05-task6-safety/`.
- Six intended behavioral red logs: ignored primary file/context directory,
  three actual local after-None cases and structural matrix. Each failure at
  intended assertion, not compile-only evidence. Initial ignored rejection reds
  also retained. First ignored green attempt failed due new-test SQL column typo
  sync_authority vs existing authoritative_kind; retained `ignored-focused.log`,
  corrected test query only, no assertion weakening or runtime workaround.
- Final focused:20 library synchronization cases (`lib-focused-final.log`),35
  dedicated service cases (`service-focused-final.log`,23 prior+12 new),30
  transport synchronization cases (`transport-focused-final.log`,all retained).
- Implementation committed BEFORE final gates as
  `d84ad8d63e1f14182e6e734239b6d5a41d89d9a2`. All command `.meta` files embed actual
  implementation HEAD, command/start/end/exit and original-base Rust diff SHA256.
  Scope: `git diff 29f3f5a25957836a8513cba8a318306e1b928063..HEAD -- src tests`.
  SHA256 `e857fdad6f0b35f7f656dfe7fb9c84ee927dfbdb427360a71cf4dd55608255f3`
  (`rust-identity.diff`, `source-identity.txt`), recomputed equal after full/CLI.
- FINAL `devenv shell -- cargo check --all-features --locked`,
  `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked
  -- -D warnings`:exit0; logs+metadata retained.
- FIRST NORMAL Task6 correction `devenv shell -- cargo test --all-features --locked`:
  **605 standard +184 SSH =789 pass**, library173/discovery59/transport103/dedicated35.
  No full retry, serialization, timeouts, ignored/assertion-suppressed cases.
  Actual interval2026-10-06T21:11:07Z–21:23:24Z (`test.log`, `test.meta`).
- CLI `devenv shell -- cargo run --locked --bin manyhands-cli`:exit0/no window,
  21:23:24Z–21:23:26Z (`cli.log`, `cli.meta`). Only docs/comments after gates.
- `git diff --check`:pass. Coherent implementation/evidence commits; clean existing
  ticket branch/worktree/index. Original-main whole-review.diff/head artifacts
  regenerated, focused76fd..final range supplied. Main .superpowers/ and devenv.nix~
  remain untouched; no stash/reset/clean/rebase/new worktree/branch.

Fresh whole-Cycle BLOCK findings now corrected with behavioral proofs, PENDING
independent whole-Cycle re-review/parent acceptance, not self-accepted. Historical
Task4 first NORMAL Busy failure remains NOT proven unrelated/pre-existing;
explicitly authorized second749 remains second-attempt success. Original Task5
raw-privacy claim remains withdrawn; corrected Task5 accepted evidence/772 gate
at575f and sourcehash7f remain historical, not inferred from old769 success.
Desktop actual startup atafb36, sanitized evidence
`/tmp/manyhands-cycle05-desktop-OecGTp`, remains startup/toplevel/buffer/12-second
survival only, NOT manual feature behavior; unchanged front end not rerun. Five
actual native jobs remain pending; CI YAML/Linux is not native execution. No
publication/dispatch/merge/closure/cleanup authority. Stop for retained fresh
whole-Cycle re-review.

## Task 6 parent local acceptance — native publication gate pending — 2026-10-06T21:32:05Z

Accepted local implementation and full original Cycle code-review range
`29f3f5a25957836a8513cba8a318306e1b928063..4cf15030d667471e10275edaa67d25cfb1d5d2d4`.
Retained independent whole-Cycle reviewer verdict **OK with notes**: both final P1s
closed with actual behavioral red/green tests, no remaining qualified code defect.
Review artifact:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/bb551116-ef66-4e37-bf23-0b9291b9bc89/task6-safety/review.md`.
Explicit SAFE plus ignored-overwrite=false protects tested file/directory/symlink
collisions while noncolliding ignored artifacts still permit synchronization.
Tagged local binding now validates exact root/ID/action/full matcher/OID in its
insertion transaction; incompatible completed/pending rows cannot fabricate
IndexPending or adopt a different first outcome. Ordinary refresh compatibility
and prior remote-ID/endpoint/authority/replay/privacy guarantees remain intact.

Parent independently verified exact HEAD/clean status, all src/tests/manifests/
locks/toolchain/CI equality to tested implementation
`d84ad8d63e1f14182e6e734239b6d5a41d89d9a2`, and recomputed original-base src/tests
SHA256 `e857fdad6f0b35f7f656dfe7fb9c84ee927dfbdb427360a71cf4dd55608255f3`.
Read actual command metadata: final check/fmt/clippy/full test/CLI all exit0 on
that implementation. Parent-derived final log totals **605 standard +184 SSH =789**,
including focused dedicated35/retained transport30 and library173. First NORMAL
Task6 corrected gate passes with no retry/serialization/assertion weakening.
Only evidence commits follow; do not duplicate broad checks on unchanged Rust.
Desktop actual startup/protocol12-second smoke remains qualified, front-end/
startup/features untouched by final corrections. No manual interaction claim.

Retained notes remain part of acceptance: Task4 initial NORMAL Busy failure not
proven unrelated/preexisting; authorized second749 success remains second-attempt.
Original Task5 raw-output completeness claim withdrawn; old769 did not prove it.
No erased failures or invalid baseline attribution. Linux symlink collision cases
actually executed; Windows capability and all five native jobs remain unverified.
Safe checkout is not universal atomic filesystem-I/O or rollback authority.

Local source/review stage is ready for publication authorization, NOT full native
Cycle acceptance or delivery closure. Existing CI configuration includes the
focused target, but actual native runs/links are still pending. No push/PR/dispatch,
merge, ticket closure or worktree cleanup is authorized. Ask owner whether to
publish normally/create PR and validate native CI, or retain this local checkpoint.
Ticket remains open; all other worktrees and unrelated main files preserved.

## Owner publication and native-CI deferral — 2026-10-06T23:56:38Z

Owner explicitly approved push and opening a PR. Owner states CI is disabled for
now and does not want time spent investigating intermittent non-Linux platform
failures; native execution is deferred until closer to release. Do NOT enable,
dispatch, monitor or repair native CI for this delivery. Existing future-focused
CI target configuration remains, without claiming native passes. This supersedes
the earlier pending-publication gate, not the honest native-evidence limits.
Authorization does NOT include PR merge, ticket closure or worktree cleanup.

Publication preflight freshly fetched origin/main at29f3f5a25957836a8513cba8a318306e1b928063;
clean ticket6807e385726fe06d8dd4554fa077656fcfe4ce20 rebase was a no-op. Main's
.superpowers/ and devenv.nix~ and all seven other worktrees remain untouched.
Remote ticket81fecfe7127b664c5b3521fd8938908d12e801d8 was the ONLY remote-only commit,
an original ticket checkpoint. git cherry marks it patch-equivalent; its exact
ticket blob8de3e5f8559a835df9bada347bd57a7648975bd7 is identical to rebased local
original checkpoint68284da5fd3648d4d91caaad865d4a4f7ee2a306. Current reviewed ticket
has later edits; initial current-file-equality guard stopped BEFORE any merge.
After proving original historical equivalence, preserve remote ancestry through
history-only ours reconciliation6556112b925d851047cdb45973ffed15755bf2f9 (not PR merge).
Expected reviewed6807 tree and resulting tree BOTH
040f7689f020c1122c56323a94f317c246210ee2; no file/source change, no ignored remote
new work and no force push. Remote ancestry now permits normal publication.
All src/tests/manifests/locks/toolchain/CI still equal testedd84ad implementation;
local789/static/CLI and qualified desktop evidence remain current. Documentation
of this owner exception is the only new file-content change. No existing open PR
was found for this exact branch. Ticket stays open; PR creation/verification next.
