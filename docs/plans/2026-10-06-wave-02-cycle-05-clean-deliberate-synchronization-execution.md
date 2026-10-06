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
- Task 1: implemented and locally verified; independent review pending.
- Task 2: pending.
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
