---
id: 01M89CHPWV2W6RY0XART0416Q3
title: 'Wave 06 Cycle 02 Design: Onboarding And Setting Changes On A Setup Branch'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 02 Design: Onboarding And Setting Changes On A Setup Branch

Cycle document: [Wave 06 Cycle 02](../Cycles/wave-06-cycle-02-pull-request-onboarding.md). Ticket: vh-c97se (`01M84H84WQYE8J2WJWEDGJ45W4`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md), section "Onboarding and setting changes", and the Cycle document's Product-Owner Decisions.

Status: **draft**, 2026-11-25, revision 4. Author: Claude.

## Intent And Approval Boundary

Let `repo enable` put a new repository's Manyhands setup on a branch for a pull request, let `remote select` do the same for a change of mode, forge or remote in an enabled repository, hold the repository in a setup-pending state until the setup is on its primary, and let a pending setup be cancelled. Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket and document branches, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `da032ffb`; Task 0 re-checks every one.
- "First-time setup" is a setup of kind `enable`: the repository had no configuration. "A change" is a setup of kind `change`: the repository is enabled and its primary holds a configuration.
- "The record" is the repository's row in the new setup table (Part A). "The setup branch" is `manyhands/setup/<setup_id>`. "The base" is the setup remote's primary tip that the setup commit is made on.
- "The setup's remote" is the record's `remote_name`. A setup is "on another remote" when its remote is not the publication remote the primary's configuration names: every first-time setup, and a change that names a first or a different remote (decision 12).
- "The local primary" is `refs/heads/<primary_branch>`, not HEAD.
- "Setup request" is the request that made a record: a pull-request `repo enable`, or a `remote select` on the setup path.

## Baseline Facts

Each fact was read in the source at `da032ffb` by the planning research and the reviews of rounds 1 and 2, and re-checked by the author where marked (✓).

1. ✓ **The configuration is a file in the root checkout.** `src/repository.rs` `read_configuration_for` reads `<root>/.manyhands/config.toml` from the working tree. `reconcile_registration` stores `config_blob_oid` from `committed_configuration_blob_oid`, which reads HEAD's tree, and stores the remote plan (`remote::state::configure`).
2. ✓ **The transport takes its remote from that file.** `src/repository/transport/operation.rs` `prepare_ssh` sets the remote from `publication_remote(root)` before it reads the key, and answers `ConfigurationInvalid` for a missing or invalid file; `recheck_ssh` reads it again before and after each transfer. A connection has one direction. `src/repository/transport/remote.rs` `plan_matches_configuration` checks every scoped method's `RemoteRefPlan` against the file. `exact_mappings` knows the primary and the item context refs only. `transfer_inner` takes its scope as `Option<(&RemoteRefPlan, &SynchronizationTarget)>`, refuses a push on a fetch connection and a delete on any other, and writes a tracking ref only for a plan scope or a poll. `delete_exact` with `callbacks.rs` `guard_delete` deletes a context ref only while it is at an expected OID; the push closes its connection, so `observe_delete` (`Deleted`, `StillPresent` for a refused or lost delete, `Changed`) runs on a second one, as `promotion/run.rs` does.
3. ✓ **The ref plan.** `src/repository/remote/refs.rs` `RemoteRefPlan::from_configuration` fetches the primary, `manyhands/document/*` and `manyhands/ticket/*`; `classify_advertised_ref` ignores any other ref, so polls and discovery never see `manyhands/setup/*`. No code uses `refs/manyhands/`.
4. ✓ **Remote operation rows.** `src/repository/remote/state.rs` `REMOTE_OPERATION_SCHEMA`: `action` and `kind` are `CHECK` constraints; the partial unique index `remote_active_reservation` allows one active row per repository; a row's insert needs the stored plan of `remote_polling_state`, whose `endpoint_digest` fences an endpoint change. Nine tables reference the table. Its one rebuild (`migrate_synchronization`) ran before those tables existed.
5. **About thirty sites in `src/repository/remote/` read the file** to build or check a plan: `observation.rs` `observation_plan`, `poll.rs` (the plan, materialization and fast-forward re-checks, `poll_configuration_matches`), `sync.rs` `synchronization_configuration_matches_reading` and its callers, and republish.
6. **Poll.** `src/repository/mutation/bind/poll.rs` `prepare` answers `invalid_configuration` for a missing file and `publication_remote_required` for no remote; its family is `Remote`; its `rerun` calls the domain poll with `restart`. The domain's `poll_probed` (`poll.rs`) stops at `observation_plan` for a missing file, clears the stored plan and records `ConfigurationRequired`. The desktop calls `poll_publication_remote` (`poll.rs`) directly, not through the binding, with an `automatic` policy that never prompts. `PollMutationDto` (`mutation/dto.rs`) is read back from a stored answer only with exactly five keys, and twelve goldens hold it.
7. **The primary's fast-forward.** `src/repository/integration/artifacts.rs` `fast_forward` (`pub(in super::super)`) locks the branch ref, requires the ref at the old OID and HEAD to name the branch, checks the tree out safely without overwriting ignored files, and moves the ref; the caller must prove the move is a fast-forward. `src/repository/remote/sync.rs` `require_clean_target` counts an untracked file as dirty.
8. ✓ **Host trust.** `src/repository/mutation/bind/host.rs` `Host::read` takes the remote from the file's publication remote (`invalid_configuration` when missing), and a given `--authority` must be that remote's. `host observe` calls `verify_ssh_transport` directly (`observe_host`); `host approve` connects through the `Session` trait's `verify_transport` (`mutation/progress.rs`: two implementations, `CallSession` and `NoSession`, and the `Recorder` double in `bind/lifecycle_tests.rs`). `VerifySshTransportRequest` has `root`, `direction` and `approval` and about 40 struct literals. Pins are keyed by authority (`read_host_trust`).
9. **No non-SSH publication remote.** `publication_remote_eligible_for` (`src/repository.rs`) parses URLs and contacts nothing. Non-SSH tests select an SSH-shaped URL that is never contacted (`tests/fixtures/cli_input_v1/remote_add.json`, `ssh://git@example.invalid/team/repo.git`). A transport call with no selected key stops with `NoSelectedKey` in `prepare_ssh`, before any connection. libgit2 1.9.7 advertises no refs for an empty remote.
10. **Enable.** `src/repository.rs` `enable_recorded` runs under `begin_lifecycle`, which takes and holds the repository lease and a local operation row; it checks the branch (`wrong_branch`), the worktree and the identity, writes the identity, the file and the commit "Initialize Manyhands" on HEAD, and registers. The binding (`mutation/bind/repo.rs`) is confirmed and answers `SaysNothing` for a row without a checkpoint. Its execute answer carries `data: null`. On re-entry `Confirmed::row` answers a completed row from `ended`, whose evidence (`own_commit`, `initialization_commit`) walks the primary's first-parent line, and a pending row by comparing the observation digest, which binds `registered`; `Row::Unfinished` (`confirm.rs`) re-enters without that comparison. `written()` feeds the evidence and `fields` the intent digest, in which an absent field still writes a tag. A replay gives a commit result again only while the commit is reachable from the binding's base ref (`replay.rs`, `evidence.rs` `still_reachable`), which for enable is the primary. `EnableRepositoryRequest` has about 80 struct literals in 44 files, 13 of them in SSH-harness files; `RepoEnableInput` has 8 literals without `..`.
11. **Remote selection.** `set_publication_remote` (`src/repository.rs`) requires a valid file, the primary checked out and the configuration path clean, and commits "Configure Manyhands publication remote" on HEAD. The verb is unconfirmed and does not stop at safe points; its data is `RemoteMutationDto { name, location }`. Its binding answers `DidNothingToASetting` for a row without a checkpoint, so `Reentry::of` (`reenter.rs`) gives `NewUnlessMoved`, and `moved` (`bind/remote.rs`) re-reads the file: unchanged, the record is released and the request runs as new. `LocalStep::is_checkpoint` (`recovery.rs`) is an exhaustive match. `RemoteSelectInput` has 16 struct literals. The CLI parser `remote_name` (`src/bin/manyhands-cli/admin.rs`) is shared with `remote remove` and accepts only `name`.
12. ✓ **The mutation boundary.** `src/repository/mutation/execute.rs` `execute_binding` runs `settle_target`, then sends a request with no record to `first_call` (consent, `prepare`, then acceptance in `records.rs` `insert_request_admitted`, an immediate `write_records` transaction, with the binding's `admit`); `reenter` and `replay` do not call `prepare`. A confirmed verb's `--prepare` runs `confirm.rs` `prepare_call`, which does not reach `first_call`. `command.rs`'s const fns are exhaustive matches. A binding whose answer is `Standing::Owed` leaves its request accepted, and the same request re-enters (`settle.rs`). `request cancel` asks nothing of a command that does not stop at safe points (`progress.rs`), which `repo enable` and `remote select` do not.
13. ✓ **Local operation rows.** `operation_records.repository_id` is `ON DELETE SET NULL` (`recovery.rs`). A completed row blocks no later lifecycle. `completed_step` is plain text; `LocalStep` (`local_steps!`) is code with a name-list test.
14. ✓ **Removing a registration.** `repo remove` (`registration_removal.rs`) deletes the root's operation rows and its `repositories` row in one transaction, and dependents cascade. It refuses `recovery_required` for an accepted request or unfinished work only on its registry-only path (`require_registry_only_removal`, when the root is gone).
15. **A registered repository with no configuration.** Create and save answer `repository_not_registered`, sync and poll `invalid_configuration` (`tests/mutation_replay/not_enabled.rs` `a_registered_repository_with_no_configuration_keeps_the_answers_it_had`). Reads answer `ok`; discovery makes the primary context `unverified` and keeps its documents (`discovery.rs` `read_primary`). `bind/mod.rs` `never_enabled` is false when any local branch tip holds a configuration.
16. **The registry.** `migrate_registry` (`discovery.rs`) ends with `migrate_pull_request_mode` (marker `wave_06_pull_request_mode_v1`). `index rebuild` upserts `repositories` and never deletes a row; a corrupt registry is set aside and replaced by an empty one.
17. ✓ **Locks.** The repository lease (`src/repository/coordination.rs` `repository_lease`) is an `fs4` exclusive lock on `manyhands-operation.lock` in Git's common directory, with a 250 ms timeout, released when its process ends; a second handle in the same process conflicts with it. `acquire_exclusive` is private to the module. `snapshot_file_hash` exempts only that file from being read, because a Windows byte lock forbids a second handle's read.
18. **The desktop.** The scheduler polls only a repository with a selected, SSH-eligible publication remote (`desktop/services/scheduler/registration.rs` `read`). The repository row shows "Needs fixing" for any configuration that is not valid, missing included (`desktop/model/repository_state.rs` `needs_fixing`). Add repository sends `RepoEnableInput` with no remote and selects the remote afterwards.
19. ✓ **Schemas and goldens.** Each preview kind has its own schema (`schemas/v1/*_preview.schema.json`) and a `PreviewDetailsDto` variant. The schema checker (`tests/support/schema.rs`) accepts `type` as a name or an array, `$ref` only on its own and only to a sibling file, and no `oneOf`. The recovery actions are a fixed set of 13 (`src/results_tests.rs` `RECOVERY_ACTIONS`), with `repo.inspect` and no poll action. A golden is written only for a name already in `tests/support/mutation_golden.rs` `CASES`; `tests/mutation_contract.rs` checks the fixture directory against `CASES`, each code against its goldens or a listed reason, and each data schema against `DATA_ROOTS` and a golden.
20. **Cycle 01's pieces.** `src/forge.rs` `recognise`, `default_integration`, `pull_request_link` and `PullRequestLink`; `RepositoryConfig::integration()` and `forge()`; `src/repository/remote/sync.rs` `local_only_commits` (private); the result code `primary_has_local_commits` with `PrimaryLocalCommitsDto`.

## Part A: The Setup Record

**Table (R-1).** `repository_setups`, created by a named, additive migration with marker `wave_06_repository_setup_v1`, run by `migrate_registry` after `migrate_pull_request_mode`, schema-checked and in an immediate transaction as Cycle 01's is. No schema object's name may contain "request" (`tests/mutation_replay.rs` asserts it).

```sql
CREATE TABLE IF NOT EXISTS repository_setups (
    repository_id INTEGER PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
    setup_id TEXT NOT NULL UNIQUE,              -- a ULID; the branch is manyhands/setup/<setup_id>
    kind TEXT NOT NULL CHECK(kind IN ('enable','change')),
    remote_name TEXT NOT NULL,
    primary_branch TEXT NOT NULL,
    endpoint_digest BLOB NOT NULL,              -- the setup remote's fetch and push endpoints, as remote_polling_state digests them
    configuration BLOB NOT NULL,                -- the intended file, the serializer's output
    base_configuration_oid TEXT,                -- change only: the primary's configuration blob when it began
    base_oid TEXT, commit_oid TEXT,             -- the base and the setup commit, once made
    published_oid TEXT,                         -- the setup commit, once the remote is seen to hold it
    request_id TEXT NOT NULL, created_at INTEGER NOT NULL,
    CHECK((kind='change')=(base_configuration_oid IS NOT NULL)),
    CHECK((base_oid IS NULL)=(commit_oid IS NULL)),
    CHECK(published_oid IS NULL OR published_oid=commit_oid),
    CHECK(length(endpoint_digest)=32)
);
```

OID columns carry the 40-hex `CHECK` the other tables use. One row per repository makes a second setup in this clone impossible (RFC). The row holds no remote URL, credential or identity: the endpoints are a digest, and the configuration bytes hold what the configuration holds (a remote's name, a forge's web address). Its state is `recorded` until `published_oid` is set, then `published`. The link is built when read (Part I).

**Lifetime.** The record survives `index refresh` and `index rebuild` (Baseline Fact 16). `repo remove` drops it by cascade and leaves the branches (decision 4), and finishes the setup request as `cancelled` if it is still accepted (decision 16, Part K). A corrupt registry replaced by an empty one loses it (K-2).

**Store API.** A module `src/repository/setup/` (new) with `record.rs`: `read(tx, repository_id) -> Option<SetupRecord>`, `insert`, `record_commit`, `record_published`, `delete`. Every write is one registry transaction. Every function that might lock reads the record first, so a repository with no record never creates the setup lock file (Part D).

## Part B: The Setup Gate And Completion

**The table (R-2).** A new const fn on `Command`, exhaustive like `restores_index`:

```rust
pub(crate) enum SetupGate { Carries, RefusedBeforeEnable, Refused }
pub(crate) const fn setup_gate(self) -> SetupGate;
```

| Gate | Commands |
| --- | --- |
| `Refused` (any pending setup) | `ticket close`, `document promote`, `remote select`, `remote remove`, `repo enable` |
| `RefusedBeforeEnable` (a first-time setup only) | `document create`, `document save`, `document move`, `document repair`, `ticket create`, `ticket save`, `ticket slug-assign`, `ticket repair`, `folder create`, `comment add`, `item sync`, `repo sync`, `item republish`, `item resolve-history`, `remote resolve-history`, `conflict resolve`, `context sync-local`, `context resolve-local`, `context prepare`, `context release`, `context retire` |
| `Carries` | `repo create`, `repo remove`, `repo identity-set`, `repo setup-cancel`, `remote add`, every key and host verb, `operation resume`, `operation abandon`, `poll once`, `poll configure`, `poll pause`, `poll resume`, `index refresh`, `index rebuild` |

**Where it runs.** `setup_gate::check(service, command, root) -> Result<(), Answer>` reads the record (one query) and answers `setup_pending` for a refused command; first it finishes a complete setup (below), when its lock is free. It runs in `first_call` before `prepare`, and in `prepare_call` before `preview`. Inside the acceptance transaction (`insert_request_admitted`, and the one-transaction path, as `require_unreserved` runs in both) a read-only `setup_gate::check_in(tx, command, root)` refuses on the record and `is_complete`, and never finishes. `reenter` and `replay` are not gated, so a setup request continues past its gate. `settle_target` runs first, so a command addressed by a short code it cannot resolve answers as today. The gate does nothing for a root with no registration.

**Closing the races.** A change inserts its record in one registry transaction that also finds no unfinished close or promote parent and no accepted `ticket close`, `document promote` or `remote select` request for the repository other than the setup request itself, or answers `recovery_required` naming it (a final answer: the change's request is finished). A close, promote or `remote select` is gated in its acceptance transaction. So such a request is either accepted before the change's record exists, and the change refuses, or meets the record and refuses.

**Completion is level-triggered (R-17).** A setup is complete when the local primary already holds it. `setup::complete::is_complete(repository, record)` reads `refs/heads/<record.primary_branch>`: its tree must hold a valid configuration that names that primary branch, and for a change its configuration blob must not be `base_configuration_oid`. `finish` then, under the setup lock, and only while HEAD names the primary:

1. deletes the local setup branch if it is at the recorded commit (keeping it otherwise, K-15), and the private base ref;
2. for a setup on another remote, reconciles the registration and stores the remote plan as remote selection does, through `reconcile_registration` under the same lifecycle check remote selection takes (`begin_lifecycle` and `in_the_way`); in the way, it stops and the next look finishes it;
3. in one registry transaction: deletes the record, and finishes the setup request if it is still accepted (Part K): `already_applied` when the merged configuration is the record's `configuration`, `external_change` otherwise.

These are ordered transactions; a crash between them leaves a record the next look finishes, because completion is level-triggered. `finish` runs at the end of `poll_probed` only after the poll's own remote row is final, so the plan store never fences the poll that calls it. A `finish` that fails (the setup branch checked out in another worktree, a vanished root) changes no answer and is tried again at the next look.

There are two entries: `finish(service, root)`, which takes the setup lock and answers false when it is held, and `finish_held(&SetupLock, service, root)` for a caller that holds it. `finish` runs in the gate before it refuses (a complete setup refuses nothing), and at the end of the domain's `poll_probed`, so a CLI poll and the desktop's direct poll both finish it. `poll_setup` uses `finish_held` (Part G). A held setup lock skips the finish and changes no answer. Reads treat a complete setup as none: `repo inspect` reads the local primary as `is_complete` does; `repo list`, which reads the index only, compares the record with `repositories.config_blob_oid` and can lag until the next refresh (K-16).

**The answer.** `setup_pending` is a new result code: class `Blocked` (exit 3), message "The repository's Manyhands setup has not merged yet.", recovery `repo.inspect`. It carries no data and nothing is written. For `ticket close --prepare` and `document promote --prepare` it is the envelope's code, not an entry of `data.blockers` (D-1).

## Part C: The Setup Channel

A narrow transport path for the setup branch, so that the poll, synchronization and observation code (Baseline Fact 5) is not taught about the record (R-3, decision 11).

**The scope source.** `PreparedSshAttempt` (`transport/operation.rs`) gains the source of its remote: `Configured`, today's, or `Setup(SetupScope { setup_id, remote_name, primary_branch, endpoint_digest })`. `prepare_ssh` and `recheck_ssh` read the remote from the file for `Configured`, exactly as today, and from the scope for `Setup`, checking on each recheck that the record still holds the same `setup_id`, remote and primary and that the remote's endpoints still digest to `endpoint_digest`, or `EndpointChanged`. The digest is today's (`observation.rs`), refactored into a free function over a named remote's fetch and push endpoints, `endpoint_digest_of(repository, remote_name)`, which the observation configuration then calls. Key selection, unlock, host trust and endpoint checks are shared and unchanged. `SshTransportError` does not change (`bind/host.rs` destructures it).

**The entries.** Two new entries take a scope; every existing entry keeps `Configured`:

- `verify_setup_transport(request, scope, session)`: as `verify_ssh_transport`, for `host observe` and `host approve`;
- `with_setup_remote(request, scope, automatic, session, use_remote)`: one connection in `request.direction`, with today's `automatic` policy for a background poll.

**The methods**, each refusing unless the connection's source is `Setup` and its direction is the method's, with a setup arm in `transfer_inner` for each:

- `fetch_setup_base()` on a fetch connection: fetches `refs/heads/<primary>` into the private ref `refs/manyhands/setups/<setup_id>/base` with `write_tracking_tip`'s compare-and-swap, never into the remote-tracking ref, so it cannot race a poll. It answers the advertisement: the primary's OID or none, and the setup ref's OID or none.
- `push_setup_branch(commit)` on a push connection: checks the connection's own advertisement (the setup ref absent, or already at `commit`, which pushes nothing), then pushes `refs/heads/manyhands/setup/<setup_id>` without `+`, guarded in `push_negotiation` by `guard_create`. A refused guard surfaces as `TransportUnavailable`, so its outcome is read by a fresh observation.
- `delete_setup_branch(expected)` on a delete connection: `guard_delete` on the setup ref.
- `observe_setup_branch()` on its own fetch connection: the setup ref's OID from a fresh advertisement, used after a push and after a delete.

The `SetupChannel` trait (Part D) opens one connection per method. Publishing takes at least three connections (fetch, push, observe), and a cancel of a published setup at least three (fetch, delete, observe). `guard_create` and `guard_delete`'s decisions are pure predicates over the update count, source, destination and ref name, so a unit test can reach them (`git2::PushUpdate` cannot be built outside git2). No other ref is pushed or deleted through a setup scope, and no item method accepts one.

**Host trust (decision 2).** `Host::read` takes, without `--authority`, the file's publication remote when there is one and the setup's remote otherwise; with `--authority`, the matching authority of either remote. `host observe` and `host approve` then connect through `verify_setup_transport` for the setup's remote (`host approve` by a new `Session` method `verify_setup_transport`), and as today for the file's.

## Part D: Setup Operations

Four operations use the channel: publishing (from enable or remote select), `poll_setup`, cancelling, and completion (Part B, local only). Their concurrency, crash windows and retries are stated as required outcomes and built test-first with a design checkpoint (plan Task 6).

**The setup lock (R-4).** Each setup operation holds a per-repository exclusive file lock, `manyhands-setup.lock` in Git's common directory, taken through a new `coordination.rs` function beside `repository_lease`, for its whole run, the network included. A second setup operation meeting it answers `busy`, and the same request continues later. Item writers never take it. `snapshot_file_hash` exempts it as it exempts the repository lease. The lock dies with its process, so a crash leaves nothing held.

**The repository lease.** Local Git writes inside a setup operation (the branch, the commit, the fast-forward) take the repository lease for that step only. `enable_with_setup` and `change_setup` release the lease `begin_lifecycle` holds once their recording transaction commits (Part E step 2), before any network call, so the lease is never held across a connection or a passphrase prompt.

**No remote reservation (R-5).** Setup operations reserve no `remote_operation_records` row: the setup ref is theirs alone, the base is fetched into a private ref, and the one primary update (activation) runs under the repository lease. This avoids rebuilding that table to add an action (Baseline Fact 4). `request cancel` asks nothing of a setup request, because `repo enable` and `remote select` do not stop at safe points (Baseline Fact 12); Ctrl-C ends the process, the lock dies, and the request stays accepted; `repo setup-cancel` is the way to stop a setup.

**Credentials.** The bindings reach the network through the `Session` trait (`mutation/progress.rs`), which gains `publish_setup`, `poll_setup`, `cancel_setup` and `verify_setup_transport`, in `CallSession`, `NoSession` and the `Recorder` double.

**Publishing** is `continue_setup(service, root, request_id, session)`, under the setup lock, reading the record again first: gone, the setup ended elsewhere and the request was finished in that transaction, so it answers the stored envelope (Part K):

1. While `commit_oid` is null: fetch the base through the channel. A remote with no primary answers `remote_primary_missing`. A local primary that is not the base or an ancestor of it answers `primary_has_local_commits`, with `local_only_commits` against the base (made visible to `setup`). A first-time setup whose base holds any `.manyhands/config.toml` answers `remote_already_set_up` (decision 13). A change whose base's configuration blob is not `base_configuration_oid` answers `external_change`: for a change on the same remote the primary moved and a poll comes first; for a change on another remote, that remote's primary does not hold this repository's configuration (K-13).
2. Under the repository lease, make the setup commit with `commit(None, ..)` and no checkout: its tree is the base's tree with `.manyhands/config.toml` set to the record's `configuration` (mode `100644`), its one parent is the base, its message is "Set up Manyhands" for a first-time setup and "Change Manyhands setup" for a change (R-6), its author and committer the configured identity. Record `base_oid` and `commit_oid`, then create the local setup branch at the commit. On a retry, a recorded commit is reused, step 1 is skipped, and a missing branch is created again.
3. Observe the setup ref: at the commit, go to step 5. Absent, push it. At another OID, answer `recovery_required` and keep the record.
4. After the push, observe again: at the commit, go on; absent, answer the transport's stop and keep the record; at another OID, `recovery_required`.
5. Record `published_oid`.

A retry after the push and before step 5's record sees the ref at the commit and records it without pushing again. The local primary, the root checkout and the remote's primary never change while publishing.

**Ending a refused setup (R-7).** The four refusals of step 1 end the setup, finally: the private base ref is deleted, then in one registry transaction the record is deleted, for a first-time setup the registration too, and the setup request is finished with the refusal (Part K's `finish_accepted`, so no crash can leave it accepted); the request's own operation row is kept, completed. A supplied identity and the worktree exclusion stay, as after a direct enable that stops. A change's repository carries on. Every other stop keeps the record and leaves the request accepted (Part K).

**A lost commit object.** A recorded `commit_oid` whose object is missing (power loss before the object reached disk) with the remote setup ref absent is made again from the record and recorded; with the remote ref present, the answer is `recovery_required`.

### Required Outcomes Of Part D

| # | Situation | Required outcome | Proof |
| --- | --- | --- | --- |
| S1 | Publish, base present, local equal to or behind the base, base as the setup starts from | setup commit on the base with only the configuration changed; local branch; remote branch at the commit; record published; local primary, checkout and remote primary unchanged | the SSH case; the commit builder's unit test |
| S2 | Remote advertises no primary | `remote_primary_missing`; ended (R-7) | the SSH case; the decision unit test |
| S3 | Local primary has commits the base lacks (ahead or diverged) | `primary_has_local_commits` with the local-only commits; ended | the decision unit test |
| S4 | First-time: the base holds a configuration. Change: the base's configuration is not the starting one | `remote_already_set_up`, or `external_change`; ended | the decision unit test |
| S5 | No selected key, unlock refused, host not trusted, network down, before the push | that stop's code; record kept; the request left accepted; the same request continues | the non-SSH tests; the SSH case's host stop |
| S6 | A crash after the commit is recorded and before the branch exists; then a retry | step 1 skipped; the recorded commit reused; the branch created; published once | the publish unit test with a stubbed channel |
| S7 | A crash after the push and before `published_oid`; then a retry | the ref observed at the commit, recorded, no second push | the publish unit test |
| S8 | The remote setup ref exists at another OID before the push | `recovery_required`; record kept; nothing pushed | the publish unit test |
| S9 | Two setup operations at once (enable retry and cancel, or poll and cancel) | the second answers `busy`; nothing interleaves | the lock unit test |
| S10 | Any setup operation | only `refs/heads/manyhands/setup/<setup_id>` is pushed or deleted; the primary is never pushed | the channel's predicate and mapping unit tests; the SSH case checks the server's primary |
| S11 | The setup remote's URL changed after recording | `EndpointChanged`, answered as `recovery_required`; nothing sent | the scope recheck unit test |
| S12 | A cancel, or completion, between recording and the lock | publishing finds no record and answers the request's stored envelope (`cancelled`, `already_applied` or `external_change`) | the publish unit test |

## Part E: Repo Enable In Pull-Request Mode

**Input.** `repo enable` gains three optional fields; `repo create` does not (R-8):

| Field | Type |
| --- | --- |
| `publication_remote` | string |
| `integration` | `"direct"` or `"pull_request"` |
| `forge` | object with optional strings `kind`, `web_url`, `pull_request_url` |

`schemas/input/v1/repo_enable.schema.json` gains them, with `forge` an inline closed object. The CLI's shared `repository_input` splits, so `repo create` still refuses the new fields. `RepoEnableInput` gains `publication_remote`, `integration` and `forge`; `EnableRepositoryRequest` does not (Baseline Fact 10). The intent digest (`fields`) adds each new field only when it is supplied, so the digest of every existing enable request is unchanged.

**The mode, for a repository without configuration.** `integration` when given; otherwise `forge::default_integration(fetch URL of publication_remote, forge)` when `publication_remote` is given (decision 9); otherwise direct. The intended configuration is built with `primary_branch`, `publication_remote`, `integration` and `[forge]`, and validated by Cycle 01's serializer: a pull-request configuration without a remote, or an invalid forge, answers `invalid_input` before any write. A named remote that is missing or not SSH-eligible answers `invalid_remote`, as remote selection does.

**The direct path with a remote (R-9).** A direct enable with `publication_remote` or `forge` writes them in its "Initialize Manyhands" commit. A new library entry, `enable_direct_with(request, prefix, remote, forge)`, takes them; `canonical_configuration` gains them; `written()` includes them, so the lost-result evidence matches. Without them the direct path is today's, byte for byte.

**A repository with a valid configuration** (a clone): today's register-only path. A supplied `publication_remote`, `integration` or `forge` must equal the configuration's, or the answer is `invalid_input` and nothing is written (R-10).

**The pull-request path** (a new library entry, `enable_with_setup(request, setup: SetupRequest, session)`, under `begin_lifecycle` as today, so a second enable of the same root at the same time answers `busy`):

1. Today's checks: on the primary (`wrong_branch`), clean (`worktree_not_clean`, `worktree_conflicted`), identity (`identity_required`). An unborn HEAD answers `wrong_branch` (R-11).
2. Mint `setup_id`. In one registry transaction: find no record for the repository (one with this request's ID: continue at step 3; another: `setup_pending`, final), register the repository with `config_blob_oid` null (a new `register_setup_pending`, because `reconcile_registration` needs the file in HEAD), insert the record (kind `enable`, `request_id` the request's, the endpoint digest), and record `LocalStep::SetupRecorded` on the local row and complete it. Release the repository lease. Then write the supplied identity and the worktree exclusion, as today; a re-entry writes a missing identity again from its input, under the repository lease.
3. Publish: `continue_setup` (Part D) through the `Session`.
4. Answer `ok` with data `setup` (Part I), effects `write` `written`, `checkpoint` `not_requested` with no `commit_oid` (the setup commit is in `data`, and is never on the local primary, so a replay's reachability check does not apply to it), and `publication` `published`. Every answer after step 2 carries the `setup` data with its state, a stop included.

**Preview and consent.** The prepare of a pull-request enable answers `setup_preview.schema.json`: the root, the primary, the identity, the prefix, the mode, the remote's name and redacted location, the forge, and the effects `repository_registered`, `setup_recorded`, `setup_branch_committed`, `setup_branch_published` (new `PreviewEffect` values). The direct preview and its goldens do not change. The observation digest binds, in addition, the mode, the forge and the remote's fetch and push URLs, so a change between prepare and confirm is `external_change`. The prepare contacts no remote.

Re-entry, replay and lost results are Part K's.

## Part F: Setting Changes Through Remote Select

**Input.** `remote select` gains optional `integration` (`"direct"` or `"pull_request"`) and `forge` (an object as in Part E, or null to remove the `[forge]` table). `RemoteSelectInput` gains both; the CLI's `remote_name` parser splits from `remote remove`'s, with a nullable-object helper for `forge`; `schemas/input/v1/remote_select.schema.json` gains them inline. The intent digest adds `integration` and `forge` only when supplied, and tells a null `forge` (removed) from an absent one (unchanged).

**The intended configuration** is the primary's configuration with `publication_remote` set to `name`, and `integration` and `[forge]` replaced when given. Validation comes first, as today: clearing the remote of a pull-request configuration is `invalid_configuration` (Cycle 01) before any other check. Then:

| Current mode | Intended | Path |
| --- | --- | --- |
| any | equal to the current configuration | `already_applied`, nothing written |
| direct | direct | today's commit on the primary; a forge change goes in the same commit, through `set_publication_remote_with(request, forge)`, and `moved` compares the whole intended file |
| direct | pull request, any SSH remote | setup branch on the intended remote (decision 12) |
| pull request | the same remote, either mode | setup branch |
| pull request | another remote | `invalid_input` (decision 8) |

**The pull-request path** (a new library entry, `change_setup(request, session)`):

1. Today's checks: a valid configuration, the primary checked out, the configuration path clean, the identity.
2. In one registry transaction (Part B's race rule): no record (one with this request's ID: continue at step 3; another: `setup_pending`, final), no unfinished close or promote, no accepted close, promote or `remote select` request other than this one; insert the record (kind `change`, the intended remote, its endpoint digest, `base_configuration_oid` the primary's configuration blob), and record `LocalStep::SetupRecorded` and complete the local row. Release the repository lease.
3. Publish: `continue_setup` (Part D). A refusal at publishing step 1 ends the change (R-7).
4. Answer `ok` with data `setup` (Part I) and effects as Part E's. The configuration on the primary does not change.

While a change is pending, the gate refuses close, promote, `remote select`, `remote remove` and `repo enable`, and every other write carries on in the current mode and on the current remote. Re-entry is Part K's.

## Part G: Poll: Activation

**Which poll.** For a repository with a record on another remote, `poll once` runs `poll_setup`: for a first-time setup instead of today's poll, and for a change on another remote after today's poll of the current remote (with `--retire-completed` passed to that poll). A change on the same remote keeps today's poll alone, which fast-forwards the primary, and completion (Part B) finishes it. The binding chooses in `run` and in `rerun`, not only in `prepare`, so a re-entered poll request takes the same path; `--retire-completed`'s binding (`bind/poll_retirement.rs`) accepts a first-time setup in its `prepare` the same way. `poll_publication_remote` keeps its signature.

**`poll_setup`**, under the setup lock, with the `automatic` policy for the desktop, checking in this order:

| # | What it sees | Outcome | Effect |
| --- | --- | --- | --- |
| G1 | the record is `recorded` | `not_published` | none, no connection |
| G2 | the base's configuration is invalid, or names another primary branch | `configuration_invalid` or `primary_differs` | none |
| G3 | the base does not hold the setup (first-time: no configuration; change: the starting blob), and the remote setup branch is gone | `branch_deleted` | none; the skill says to cancel |
| G4 | the base does not hold the setup | `waiting_for_merge` | none |
| G5 | the root is not on the primary, or not clean (untracked files count) | `waiting_for_checkout` | none |
| G6 | the local primary has commits the base lacks | `diverged` | none |
| G7 | otherwise | `activated` | below |

Activation: under the repository lease, fast-forward the local primary to the base with `artifacts::fast_forward` (a no-op when already there), and set the remote-tracking ref of the setup's remote to the base; then `finish_held` (Part B) finishes the setup and registers the repository as a direct enable plus remote selection of the merged configuration would; then refresh. The local setup branch and the private ref are deleted before the record, so a crash leaves a record the next poll or gate finishes. `differs` is true when the merged file is not the record's `configuration`; the merged one wins. A merge commit, a squash and a fast-forward of the setup commit each activate.

Every outcome is `ok`, exit 0, and the request is finished: a stop is final and a new request polls again, as poll's are today (CLI contract, poll's R-18). `poll_setup` reserves no remote row. For a first-time setup it records its attempt and next eligible time in `remote_polling_state` (a new function beside `record_poll_attempt`), with its outcome mapped onto the existing categories (`completed` for any answered advertisement, the transport's stops to theirs), so the desktop schedules it by the repository's polling policy; for a change on another remote it records nothing, and today's poll's record schedules both. A key, unlock, host or network stop answers that code, finally, as today's poll's stops do; Ctrl-C answers `cancelled`, finally; `request cancel` reaches it at its safe points, as `poll once`'s. Task 8's design checkpoint names how the poll binding settles a request with no remote row. Its data is `setup_poll.schema.json`: `{ type: "setup_poll", setup_id, outcome, integration, differs, pull_request }`, where `integration` is the merged mode on activation and null otherwise. It is a new `MutationDataDto` variant read back by its tag, so `PollMutationDto` and its goldens do not change; for a change on another remote the answer is today's poll's, and the setup's outcome is visible in `repo inspect` (R-16). Human output prints the outcome's line, and the link while waiting.

## Part H: Repo Setup-Cancel

A new confirmed verb, `repo setup-cancel --repo <root>`, with no input, registered as `remote resolve-history` was (`delivered_by` "C5", R-13). Its `Command` arms: `has_operation` true (a local row), `needs_confirmation` true, `stops_at_safe_points` false, `stage` as enable's.

- **Prepare** reads local state only. With no record it answers `already_applied`. Otherwise its preview (`setup_cancel_preview.schema.json`) names the setup, its kind, branch and state, and the effects `setup_branch_deleted_remotely` (when `commit_oid` is set), `setup_branch_deleted`, `setup_record_removed` and, for a first-time setup, `repository_unregistered`.
- **Its own request.** Like a setup request, it opens its local row with `begin_lifecycle`, records `LocalStep::SetupCancelRecorded` and completes the row, and releases the lease before any connection; a re-entry repeats the steps, which are idempotent; its observation digest binds the record's state, so a publish between prepare and confirm is `external_change`.
- **Confirm**, under the setup lock:
  1. When `commit_oid` is set, fetch the base and observe the setup ref (the S7 window may have pushed an unrecorded branch). If the base holds the setup (Part G's condition, without the checkout checks), answer `recovery_required` with recovery `repo.inspect` and change nothing; the skill and human output say to run `poll once` (R-14).
  2. The remote setup ref: at the recorded commit, delete it with `guard_delete` at that commit, then observe it on a new connection: `Deleted` goes on; `Changed` keeps it, and the answer says so; `StillPresent` (the delete was refused or lost) answers `push_rejected`, retryable, and changes nothing more. At another OID before any delete: kept, no delete sent, and said so. Absent: goes on.
  3. Delete the local setup branch (at the recorded commit; kept otherwise) and the private ref.
  4. In one registry transaction: refuse `recovery_required` if another pending operation row is in the way (`in_the_way`), as `repo remove`'s reconciliation would; otherwise delete the record; for a first-time setup, the root's other operation rows and the registration, as `repo remove` deletes them, keeping the cancel's own row; finish the setup request as `cancelled` if it is still accepted, and the cancel's own request with its answer (Part K).
- A key, unlock, host or network stop changes nothing and leaves the cancel's request accepted. A setup with no commit needs no connection.
- **Data** `setup_cancel.schema.json`: `{ type: "setup_cancel", setup_id, kind, remote_branch: "deleted" | "absent" | "kept" | "not_published", local_branch: "deleted" | "absent" | "kept", unregistered }`.

## Part I: Reads And Contract Shapes

- **`pull_request.schema.json`** (decision 6): `{ source, target, forge, url }`, `forge` one of the four kinds or null, `url` a string or null; `PullRequestDto` from `PullRequestLink`. Cycle 03 reuses it. It is always an object.
- **`repository_setup.schema.json`**: a nullable object (`"type": ["object", "null"]`) `{ id, kind, state, request_id, branch, remote, integration, pull_request }`, `pull_request` a `$ref` to the link (known before publishing: its source, target and forge; its `url` may be null). `repo list` and `repo inspect` gain `setup`, a `$ref` to it. Task 0 confirms that a top-level nullable file passes `every_published_object_is_closed_and_requires_all_its_properties` and the reachability test; if not, `setup` is inlined in both read schemas.
- **The link** is built when read: `forge::pull_request_link(forge::recognise(fetch URL, configuration's forge), branch, primary)`, from the record's configuration and the remote's fetch URL in Git's configuration. `repo list`, which reads the index only, builds it from the configuration's `[forge]` alone and gives a null `url` when the forge needs the remote URL; `repo inspect` gives the full link.
- **Human output.** `repo inspect` adds `Setup: recorded, not published yet` or `Setup: waiting for <branch> to merge into <primary>`, and the link's human line. `repo list` adds ` setup=<state>` to a repository's line when it has a setup.
- **Mutation data.** `setup_mutation.schema.json` `{ type: "setup", id, kind, state, request_id, branch, remote, integration, pull_request }` for enable's and remote select's pull-request path (the setup fields inline, non-null); `setup_poll` and `setup_cancel` above. Each is a tagged `MutationDataDto` variant with a `from_stored` arm guarded by its tag, as Cycle 01's `primary_local_commits` is.
- **Codes.** `setup_pending` (Part B); `remote_primary_missing`: `Blocked`, "The remote has no primary branch to set up from.", no recovery action; `remote_already_set_up` (decision 13): `Blocked`, "The remote's primary already holds a Manyhands configuration.", no recovery action.

## Part J: The Desktop

The minimum of decision 5:

- **The scheduler** reads the repository's setup before its remotes. A repository with a published setup on another remote reads as `Read::SetupPending` (with, for a change, today's `Polled` for its current remote too), is scheduled by its polling policy (Part G records the setup poll's attempts there), and is polled through `poll_setup` with the automatic policy; an `activated` outcome refreshes the repository as a completed poll does. Any other repository reads as today, so its read counts do not change.
- **The row** gains a state before `needs_fixing` for a first-time setup: published, "Waiting for setup to merge" (the RFC's words); recorded, "Setup not published yet". Neither has an action; Cycle 06 adds "Open pull request page".
- A `setup_pending` answer shows the code's message through the existing failure display (K-9).
- Add repository is unchanged and stays direct: it sends no `publication_remote`, so the default stays direct.

## Part K: The Request Lifecycle Of A Setup

Three review rounds found faults in how a setup request continues, replays and ends, so it is stated as rules and required outcomes, and plan Tasks 7, 9 and 10 build it test-first with a design checkpoint. Where the rules and the table disagree with a sentence elsewhere, the table wins.

**The rules.**

- **K1 Recording.** A setup request records its setup in Part E or F's step 2 transaction, with `LocalStep::SetupRecorded` on its own completed operation row. That transaction finds the repository's record first: none, it records; one with this request's ID, it continues; another, `setup_pending`, finally. `SetupRecorded.is_checkpoint()` is true, so `Reentry::of` takes a re-entered setup request up as recorded (`AsRecorded`), and `remote select`'s re-entry reaches its binding's `reenter` and `rerun`, never `moved` (verified in round 3; `own_commit` and the pending-row readers do not read it as a commit).
- **K2 Standing.** After recording, every answer of a setup request is `Standing::Owed`, so the request stays accepted, unless the setup ended: the four refusals of publishing step 1 (R-7) and an answer the request was already finished with (K4) are final. `busy` from the setup lock or the repository lease, `recovery_required` (S8, S11, a lost commit object with the remote ref present), the transport's stops, `identity_required`, and registry or Git errors are all `Owed`. Before recording, a setup request answers and settles as `repo enable` and `remote select` do today.
- **K3 Re-entry.** Enable's `Confirmed::row`, and `remote select`'s `reenter` and `rerun`, choose from the request's intent, not from the file: for a setup request they look first for the record with the request's ID. Present, the request is `Unfinished` and `rerun` calls `continue_setup`, comparing no digest. Absent with its own row completed at `SetupRecorded`, the request was finished by whoever removed the record (K4), and re-entry answers that stored envelope. A registry read that fails answers `Owed` and never falls through to the direct path or `save::reenter`. With a pending row and no step, the request runs afresh as a setup request (nothing was recorded).
- **K4 Ending.** Every removal of a record finishes the setup request, if it is still accepted, in the same registry transaction, through `records::finish_accepted(tx, request_id, answer)`: R-7's ending with its refusal; `repo setup-cancel` with `cancelled`; `repo remove` with `cancelled` (decision 16); completion with `already_applied` when the merged configuration is the record's `configuration`, `external_change` otherwise. `finish_accepted` finishes only a request whose command is `repo enable` or `remote select`, whose scope is the repository's, whose local row completed at `SetupRecorded`, and at its current attempt; it stores the result through `stored_result(&Answer)` with every effect `not_requested` and the `setup` data, so a replay passes `from_stored`. A binding that then settles a request the domain already finished answers the stored envelope, as local sync's does (`settle.rs`, `local_control::effects`).
- **K5 Removal.** `repo remove` finishes, in its removal transaction, every accepted request in the scope that the record names or whose local row completed at `SetupRecorded` (decision 16), on its live path and its registry-only path, so it works with the root gone. It drops the record by cascade and leaves both setup branches.
- **K6 Other verbs.** `request cancel` asks nothing of a setup request (they do not stop at safe points). `operation resume` of a setup request's operation answers `original_request_required`, naming the request.
- **K7 Replay.** A published setup's answer carries no `commit_oid` in its effects (Part E step 4), so a replay answers the stored envelope without a reachability check.

| # | Situation | Required outcome | Proof |
| --- | --- | --- | --- |
| L1 | First call, publishes | `ok`, data `setup` published; the request finished | the SSH case; the binding unit tests over a stubbed `Session` |
| L2 | First call stops after recording, for each `Owed` answer of K2 | that code, with `setup` data recorded; the request accepted; the record kept | the binding unit tests, one row per code; the non-SSH tests |
| L3 | The same request again, record present | `continue_setup`; no second registration, record or setup branch; published once | the binding unit tests (enable and select); the non-SSH tests; the SSH case's host stop |
| L4 | The same request again after the setup was cancelled, removed, completed or refused | the stored envelope: `cancelled`, `cancelled`, `already_applied` or `external_change`, or the refusal | the cancel, removal, completion and publish unit tests |
| L5 | A crash between any removal of the record and the request's settle | the request was finished in the removal's transaction: its envelope replays | the same tests, with the settle skipped |
| L6 | A crash before step 2's transaction; then the same request | the pending row has no step: the request runs afresh as a setup request, not the direct path | the binding unit tests |
| L7 | The same request after it finished, or after a lost result | the stored envelope, through `from_stored`, with no reachability check | the replay unit test; the SSH case |
| L8 | `repo setup-cancel` or completion between recording and publishing's lock | publishing answers the request's stored envelope (S12) | the publish unit test |
| L9 | `repo remove` while a setup request is accepted, with the root present or gone | removed; the request finished `cancelled`; both branches left | `tests/repository_enablement.rs` and a registration-removal unit test |
| L10 | Two first-time enables of one root at once | the second waits for the lease, then finds the first's record and answers `setup_pending`, finally; the first's supplied identity is not overwritten | the binding unit test |
| L11 | The same request sent twice at once | the second waits, then finds its own record and continues; both answers agree; one setup branch | the binding unit test |
| L12 | A setup poll request killed mid-run, then sent again | `rerun` takes the setup path again; finished on its answer | the poll binding unit test |
| L13 | `operation resume` of a setup request's operation | `original_request_required`, naming the request | the resume unit test |
| L14 | A registry read that fails on re-entry | `Owed`; no direct path, no second setup | the binding unit test |

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | The record's table, migration once, constraints, and that it survives a rebuild and goes with `repo remove`; that it holds no remote URL or identity | `src/repository/setup/record_tests.rs`; the migration test beside Cycle 01's in `discovery.rs` |
| P2 | Every command's gate (exhaustive); a first call, an acceptance and a prepare are gated and a re-entry is not; the races with a change; a complete setup refuses nothing and is finished | `command.rs` tests; `src/repository/mutation/setup_gate_tests.rs` |
| P3 | The channel's predicates and mappings: only the setup ref; one direction per connection; `guard_create` and `guard_delete` refuse every other update; the scope is rechecked against the record and the endpoint digest | `src/repository/transport/` unit tests |
| P4 | Part D's S1 to S12 | `src/repository/setup/publish_tests.rs` with a stubbed channel, the lock test, and the SSH case |
| P5 | Enable's mode choice, input refusals, clone rule and direct path with a remote (Part E), as a table | `src/repository/setup/intent_tests.rs`; `tests/repository_enablement.rs` |
| P6 | A pull-request enable that stops at `no_selected_key` leaves a recorded setup: `repo inspect` and `repo list` show it, a gated write answers `setup_pending`, `host observe` answers `no_selected_key`, `poll once` answers `not_published`, the same request continues (L3), `repo setup-cancel` unregisters and the enable's request then answers `cancelled` (L4) | `tests/repository_enablement.rs` (non-SSH) and its goldens |
| P7 | Remote select's path table, the remote rules, the race rule (Part F), each pull-request row leaving the local primary unchanged, and its re-entry (L3) | `src/repository/setup/change_tests.rs`; `tests/repository_enablement.rs` |
| P8 | G1 to G7; activation for a merge commit, a squash and a fast-forward; a crash after the fast-forward finished by the next poll; `differs`; an activated repository's `repo inspect` and registry rows equal to those of a direct enable plus remote selection of the same configuration; for a change on another remote, today's poll still runs on the current remote | `src/repository/setup/poll_tests.rs` with local repositories and a stubbed channel |
| P9 | Completion for a merge commit, a squash and a fast-forward, by the gate and at the end of `poll_probed`; not before; not with the setup branch checked out; a held lock skips it | `src/repository/setup/complete_tests.rs` |
| P10 | Cancel's preview and outcomes: no commit, published and deleted, moved and kept, at another OID and kept, refused delete, an unrecorded push, already merged, another pending row in the way; its crash between steps finished by the same request | `src/repository/setup/cancel_tests.rs`; the SSH case |
| P11 | `host observe` and `host approve` take a setup's remote as Part C says | `bind/host.rs` unit test; P6; the SSH case |
| P12 | Part K's L1 to L14 | as the table says |
| P13 | The new codes, data, preview kinds and verb registered everywhere the named edits list | the existing registration tests |
| P14 | Desktop: a setup-pending repository is scheduled on the setup path by its policy, and its row reads as Part J says | desktop model and scheduler tests |
| P15 | End to end over SSH (decision 11), in one case: a pull-request enable stops at `host_approval_required`; `host approve` on the pending repository; the same request answers `remote_primary_missing` on a server whose primary was deleted; the primary restored, a new enable publishes with the server's primary unchanged and the same request replays with no new push; a gated write answers `setup_pending`; a squash on the server; `poll once` activates; a change in the activated repository publishes; close's prepare answers `setup_pending`; a peer pushes to the setup branch and cancel keeps it; a second change is cancelled and its remote branch is gone | the SSH case |

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). Decisions are library tests with local `git2` repositories, a stubbed channel and a stubbed `Session`; the non-SSH integration tests reach a recorded setup by stopping at `no_selected_key` against a never-contacted SSH-shaped URL (Baseline Fact 9). One SSH case (decision 11) checks that the pieces connect over a real transport, which pushing and deleting a ref need: the host stop and approval, the empty remote, the push, activation and cancel's guarded delete. It goes in `tests/mutation_remote.rs`'s case table, with a comment saying why it needs SSH (R-15). New library tests set a repository-local Git identity, since the machine has none. No real waits.

At review-ready (decision 11): the fast tier, the SSH case on its own, and the `ssh_transport` and `remote_polling` targets. This departs from AGENTS.md's table, which selects every SSH-harness target for a change under `src/repository/transport/`, `mutation_remote` for `src/repository/mutation/` and six targets for `src/repository/remote/`; native CI runs every target after the merge.

## Named Edits To Pre-Existing Tests

Decision 15 approves edits to existing tests; each is still logged with its reason.

- **E-1** (Task 2) `tests/repository_enablement.rs` `registry_creates_repository_and_discovery_metadata_schema`: the table list gains `repository_setups` (67 to 68).
- **E-2** (Task 1) `src/results_tests.rs`: `DESIGN_CODES` gains the three codes (71 to 74) and `result_code_messages_are_fixed` their messages.
- **E-3** (Task 1) `tests/mutation_contract.rs` `CODES_NO_MUTATION_GOLDEN_HAS` and `CODES_NO_FIXTURE_CAN_PRODUCE`: the three codes, with the reason "golden written in a later task of this Cycle"; **E-4** removes each in the task that writes its golden.
- **E-5** (Task 3) the 6 `RepositorySummaryDto` and 6 `RepositoryInspectionDto` literals (`src/repository/read/admin.rs`, `src/bin/manyhands-cli/human_tests.rs`, 5 desktop test helpers): add `setup: None`.
- **E-6** (Task 3) the four repository goldens (`tests/fixtures/read_v1/repo_inspect.json`, `repo_list.json`, `tests/fixtures/cli_v1/cli_repo_inspect.json`, `cli_repo_list.json`): add `"setup": null`, regenerated with `MANYHANDS_UPDATE_GOLDEN=1` and read.
- **E-7** (Tasks 3, 7, 8, 10) `tests/read_contract.rs` `contract_enumerations`: every new schema `enum`.
- **E-8** (Task 7) the 8 `RepoEnableInput` literals without `..`, and (Task 9) the 16 `RemoteSelectInput` literals: add the new fields as `None`.
- **E-9** (Task 10) `src/repository/mutation/command.rs` tests that list commands, for `RepoSetupCancel`; `src/bin/manyhands-cli/catalog.rs` and `catalog_tests.rs` (81 to 82); `tests/cli/inventory.rs` (`mutating` 44 to 45, `catalogued` 81 to 82, `NO_INPUT`, `DATA`), with the taxonomy row in `docs/RFC/cli-contract.md` in the same task; `tests/support/conformance.rs` `HOMES` (46 to 47) and its row in `tests/mutation_confirmation/conformance.rs`; `tests/cli/skills.rs` `AUTONOMY` and `skills/manyhands-overview/references/autonomy.md` (decision 14); the help goldens `tests/fixtures/cli_v1/help_root.json` and `help_root_text.txt`.
- **E-10** (Tasks 7 to 12, each in the task that writes a golden) `tests/support/mutation_golden.rs` `CASES` and `tests/mutation_contract.rs` `DATA_ROOTS` (69 onwards) for each new golden and data schema; `tests/cli/inventory.rs` `DATA` rows for `repo enable`, `remote select`, `poll once` and `repo setup-cancel`.
- **E-11** (Task 1) `schemas/v1/envelope.schema.json` and `request.schema.json` code enums; `src/repository/mutation/outcome.rs` `recovery`; the "no action" lists of `outcome_tests.rs` for `remote_primary_missing` and `remote_already_set_up`.
- **E-12** (Tasks 7 and 10) `schemas/v1/preview_effect.schema.json` and `src/bin/manyhands-cli/human_admin.rs` `effect_phrase` for the new `PreviewEffect` values; the `PreviewDetailsDto` matches in `human_admin.rs` `preview` and the desktop's `view/confirm.rs`.
- **E-13** (Tasks 7, 8 and 10) the exhaustive `MutationDataDto` matches in `tests/support/mutation.rs` (six) and `tests/support/repair.rs` (one), for each new variant, with a `cargo check --all-features --locked --tests` in the same task.
- **E-14** (Task 2) `src/repository/recovery.rs`'s `LocalStep` name-list test (about line 4137): add `setup_recorded` and `setup_cancel_recorded`.
- **E-15** (Task 13) the audit unions in `docs/research/wave-03-api-audit.md` and the counts that follow from them in `tests/cli/exits.rs`, `tests/cli/collaboration_exits.rs` and `tests/cli/recovery_tables.rs`: for each recoverable code a verb newly answers (`setup_pending` on every gated verb; `remote_primary_missing`, `remote_already_set_up`, `primary_has_local_commits`, the key, host and network stops, `busy` and `recovery_required` on `repo enable`, `remote select` and `repo setup-cancel`; `push_rejected` on `repo setup-cancel`), a recovery-table row with its golden or a listed reason.

Task 0's sweep adds what else it finds: exhaustive matches on `Command`, `Mutation`, `MutationDataDto`, `PreviewDetailsDto`, `PreviewEffect`, `LocalStep` and `Session`; struct literals of the input, DTO and `PreparedSshAttempt` types; schema-closure and enum-count tests; desktop scheduler tests that count reads; snapshot tests that list the files of `.git` (`cleanup_tests.rs`, `closure/observe_tests.rs`, the promotion tests).

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `repo create`, `repo enable` row and input table (the three fields, the mode, the direct path with a remote, the pull-request path, its three refusals, its Owed stops, Part K); the `remote select` row and input table (`integration`, `forge`, the setup path, the remote rules, its Owed stops); a `repo setup-cancel` row and its C2 option row; `poll once` (the setup poll, its data and its final stops); `host observe` and `host approve` (a setup's remote); `repo remove` (it drops a setup's record, finishes its request as `cancelled`, and leaves its branches, with the root present or gone: decision 16); `repo list` and `repo inspect` (`setup`); every gated verb (`setup_pending`), including that close's and promote's prepare answer it as the envelope's code and not as a blocker; `request cancel` asks nothing of a setup request; `operation resume` of one answers `original_request_required`; the recovery tables (E-15).
- **D-2** `docs/RFC/git-workflow-and-conflict-recovery.md`: the setup branch, its commit, its publication and its guarded deletion; that a pull-request setup never commits to or pushes the primary; that setup operations take a setup lock and no remote reservation (R-5), that Ctrl-C ends one with its request accepted, and that `request cancel` does not reach it; the direct enable's remote in its initialization commit (R-9).
- **D-3** `docs/RFC/repository-index-persistence-and-refresh.md`: the `repository_setups` table, its migration and its lifetime; that a pull-request enable registers before its Git state confirms a configuration, with the record as its evidence.
- **D-4** `docs/research/wave-03-api-audit.md`: the new codes in the verbs' unions, and `repo setup-cancel` (E-15).
- **D-5** The shipped skills, in `skills/` and `.agents/skills/`: `manyhands-repositories` (enable in pull-request mode, setting changes, cancel, host trust on a pending setup, the routes by hand of K-1, K-2 and K-13), `manyhands-recovery` (`setup_pending`, `remote_primary_missing`, `remote_already_set_up`, a lost record, a setup branch left after the merge: K-8, a kept local setup branch: K-15), `manyhands-polling-and-index` (the setup poll and its outcomes), `manyhands-overview` (decision 14's autonomy rows). Each keeps Cycle 01's sentence that pull-request mode is not usable end to end until close and promote support it, and says not to run an older build on a repository with a pending setup (K-14).
- **D-6** `docs/Waves/wave-06-pull-request-workflow.md`, a managed document: Cycle 02's checklist line "and Add repository" moves to Cycle 06 (decision 5); its "Tests with a local or `file://` remote" reads "Tests: library and non-SSH integration tests, and one SSH case" (decision 11); Cycle 03's "The record of given links and the JSON `pull_request` object (moved from Cycle 01)" becomes "The record of given links (the `pull_request` object comes from Cycle 02)" (decision 6). Applied at approval with `document save` and promoted with the owner's go-ahead.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** One record per repository in `repository_setups`, holding the intended file, an endpoint digest and no remote URL, credential or identity — one pending setup is the RFC's rule — a second setup in the same clone needs a schema change; two clones can still each open a setup (K-6).
- **R-2** The setup gate is one exhaustive table on `Command`, run at the first call and at prepare (finishing a complete setup) and read-only at acceptance — a new command cannot be forgotten, and the races with a change are closed — one registry read per write.
- **R-3** A setup channel in the transport (decision 11's "another path through existing code"), not a setup-aware configuration for every remote path — about thirty remote sites read the file (Baseline Fact 5), and the setup needs one fetch, one push and one delete — today's `Configured` source is the same code, so a fault there breaks every connection, which `ssh_transport` checks.
- **R-4** Setup operations serialize on `manyhands-setup.lock`, held across the network — a file lock dies with its process, and item writers never wait on it — two setup operations at once answer `busy`.
- **R-5** No `remote_operation_records` row for a setup operation — adding an action means rebuilding a table with nine children (Baseline Fact 4), and the setup ref and the private base ref are touched by nothing else — `operation list` does not show a setup's network work, and `request cancel` does not reach it; the record, `repo inspect` and `repo setup-cancel` do.
- **R-6** The change's commit message is "Change Manyhands setup" — a reviewer tells the two kinds apart — none.
- **R-7** A setup refused at its fetch is ended and its request finished — the person must act outside Manyhands (create the primary, move commits, pull) and then try afresh — a retry of the same request replays the refusal, or answers `cancelled` after a crash in the window (K-5).
- **R-8** `repo create` takes none of the new fields — a new local repository has no remote to protect (RFC) — none.
- **R-9** A direct enable writes a given `publication_remote` and `forge` in its initialization commit — one input describes the setup in both modes, and decision 9's default needs the remote — a direct enable with a remote is a shape the desktop does not use yet.
- **R-10** On a clone, supplied fields that disagree with the configuration are `invalid_input` — the shared configuration decides the mode (RFC) — a caller that sends defaults to a clone is refused.
- **R-11** An unborn HEAD in pull-request mode answers `wrong_branch` — activation fast-forwards an existing branch — the code's message ("not on the branch this needs") is loose for this case.
- **R-13** `repo setup-cancel` is registered as `delivered_by` "C5", like vh-hyksw's verbs — the inventory requires one of C1 to C5 (`tests/cli/inventory.rs` `no_verb_is_planned`) — none.
- **R-14** Cancel of a setup that has already merged answers `recovery_required` with `repo.inspect` — deleting a merged setup would leave the primary configured and this clone unregistered — there is no poll recovery action, so the next step is in the skill and human output only.
- **R-15** One SSH case, in `mutation_remote` — decision 11; no non-SSH publication remote exists (Baseline Fact 9), and pushing and deleting a ref are transport behaviour — about a minute of run time.
- **R-16** `poll once` reports a first-time setup in its own data shape, and does not report a change's completion or a change on another remote's outcome — `PollMutationDto` is closed and replayed with exactly five keys, and twelve goldens hold it — a caller learns of those from `repo inspect` (K-3).
- **R-17** Completion is level-triggered on the local primary: the gate, the domain poll and `poll_setup` finish a setup the primary already holds — the desktop polls without the binding, and a primary can move through `git pull` — any move of the primary's configuration completes a change (K-7).
- **R-18** Replaced in revision 4 by decision 16: `repo remove` always works, finishing the setup request (K5).
- **R-19** A setup request's re-entry is taken from its record, and every removal of a record finishes the request in the same transaction (Part K, K3 and K4) — the framework's digest, checkpoint and `moved` paths cannot see a setup, and a request left accepted without its record could start a second setup — one cross-request write, `records::finish_accepted`, guarded by command, scope, row and attempt.
- **R-20** After recording, every non-final answer of a setup request is `Owed` (K2) — the framework's default for a completed row deletes the request, which frees its ID for a second setup — a request stopped by a persistent fault stays accepted until it is cancelled or removed.
- **R-21** Completion finishes a setup request as `already_applied` only when the merged configuration is the intended one, and `external_change` otherwise — the request did not apply what it asked for — a caller reads `external_change` for a setup that someone edited in review.

R-12 was retired in revision 2: decision 12 replaced it.

## Known Limits

The Cycle document's K-1 to K-17.

## Unverified

- **U-1** That a top-level nullable schema file passes the read contract's closure and reachability tests (Part I). Task 0 checks it; the fallback is named.
- **U-2** Resolved in round 3: `SetupRecorded.is_checkpoint()` true routes both bindings as K1 says, and nothing else reads it as a commit.

## Alternatives Rejected

- Teaching every remote path to read the record: R-3.
- A new `remote_operation_records` action: R-5.
- `repo remove` abandoning a setup, or a local-only cancel: decision 4.
- Requiring host trust before enable: decision 2.
- A new verb for setting changes, or `repo enable` on an enabled repository: decision 3.
- A new field in `PollMutationDto`: R-16.
- `external_change` for a remote already set up: decision 13.
- A scope field on `VerifySshTransportRequest`: about 40 struct literals; a scope source on `PreparedSshAttempt` and two new entries instead.
- Making `repo enable` and `remote select` stop at safe points so `request cancel` reaches them: it changes both verbs for every caller; `repo setup-cancel` stops a setup.

## Risks

| Risk | Mitigation |
| --- | --- |
| The scope source breaks today's connections | `Configured` runs today's code; `ssh_transport` runs at review-ready (decision 11) |
| The gate or completion breaks an existing command | no record means one registry read and nothing else; the fast tier runs every command through the boundary |
| The registration sweep for a verb, three codes, three data shapes and two previews misses a list | Task 0's sweep; each golden registered in the task that writes it; the registration tests fail closed |
| A crash leaves a setup half made, or a request that starts a second setup | Parts D, H and K's required outcomes, each with a test; the record is the journal; the lock dies with its process; completion is level-triggered |
| A pull-request setup pushes the primary | S10: the channel cannot map the primary for a push; the SSH case checks the server's primary |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility | 2 | 10 | 14 | Resolved in revision 2: re-entry through the record and the local row (B1); host trust through a scope source and `verify_setup_transport` (B2, Part C); the SSH case reordered (I1); nullable schemas (I2); completion level-triggered (I3, I10, R-17); the `Session` methods and the automatic policy (I4); the direct path's interface and digest (I5); the taxonomy row in Task 10 (I6); the read test's layer (I7); pure predicates (I8); `transfer_inner`'s setup arm (I9); the minors in place |
| 1 | contract and consistency | 3 | 13 | 8 | Resolved in revision 2: the remote rule (B1, decision 12); the test departures (B2, decision 11, D-6); merge styles (B3, P8, P9); S4 and the change's ending (I4, R-7); the refusal is final (I5, K-5); the setup's request and cancel (I6); the continuing proof (I7); crash windows (I8); completion's place (I9); limits (I10); D-1 to D-6 (I11); recovery rows and goldens (I12, E-15); `remote_already_set_up` (I13, decision 13); privacy wording (I14); the comparison with a direct enable (I15); the unspecified answers (I16); the minors in place, with the departures listed in the Cycle document |
| 2 | scoped re-review | 1 | 7 | 8 | Resolved in revision 3, which has not been independently reviewed: `remote select`'s re-entry (1, Part K, R-19); `finish_held` (2); the read-only acceptance check (3, Part B); goldens registered in the task that writes them (4, E-10); one direction per connection (5, Part C); a change on another remote reconciled, its current remote still polled, the mirror precondition and `remote remove` gated (6, Parts B, D, G, K-13); decision 11's disclosure (7, Cycle document); completion on the local primary (8, Part B); validity before G2 (9); cancel's merged check, an unrecorded OID, `push_rejected` (10, Part H); Baseline Fact 8 (11); the identity before recording (12, Part E); the lock file only with a record (13, Part A); plan bookkeeping (14); the digest and the direct forge path (15, Part F); the peer clone and a plain key (16, plan Task 12) |
| 2 | adversarial | 3 | 8 | 12 | Resolved in revision 3, not independently reviewed: `remote select`'s re-entry (B1, Part K); replay with no `commit_oid` (B2, Part E step 4); `repo remove` refuses and cancel keeps its row and finishes the request (B3, R-18, Parts H and K); step 1 skipped once a commit is recorded (I1, Part D); completion on the local primary (I2); the current remote still polled and host verbs naming either remote (I3, Parts C and G); `request cancel` described as it is (I4, R-5, D-2); cancel's merged check (I5); the lease released before the network and the acceptance check read-only (I6, Parts B and D); the endpoint digest (I7, Part A, S11); the poll's `rerun` and its schedule (I8, Part G, L12); the minors in place: L5 (M1), the identity again on re-entry (M2), L11 (M3), the race rule with `remote select` (M4), K-15 (M5), Part H step 2 (M6), K-16 (M7), the desktop's recorded row (M8), L4 and L8 (M9), S12 (M10), `--retire-completed` (M11), K-14 (M12) |
| 3 | scoped re-review | 0 | 5 | 11 | Resolved in revision 4, which has not been independently reviewed: L10 restated and enable's step 2 checks the record (1); the race rule excludes the setup request (2); every non-final answer `Owed` (3, K2, R-20); the setup poll's attempt mapped onto today's categories, and nothing recorded for a change on another remote (4, Part G); a stopped setup request carries `setup` data, so its goldens carry the schema (5, Part E step 4); task order (6, plan); ordered transactions for completion (7, Part B); `continue_setup` named and the select path chosen from the intent (8, K3); `finish_accepted` specified (9, K4); the private ref deleted at the ending (10, R-7); `poll_retirement.rs` (11, Part G); E-15 and the `CODES_` reasons (12); no recovery action names `repo setup-cancel`, so its naming is in the message, human output and skill (13); `publication_remote_required` kept for a valid configuration with no remote (14, plan Task 5); `endpoint_digest_of` (15, Part C); stale text (16) |
| 3 | adversarial | 2 | 8 | 7 | Resolved in revision 4, not independently reviewed: the way out is `repo remove`, which always works (1, decision 16, K5); the standing of every answer (2, K2); the second call continues its own record (3, L11, `continue_setup`); the record check and the identity after recording (4, Part E, L10); the race rule (5); every removal finishes the request in its transaction (6, K4, L5); `finish_accepted`'s guard (7, K4); the stored envelope, and `already_applied` only for the intended configuration (8, K4, R-21); cancel's own row, lease and `in_the_way` (9, Part H); the change on another remote that diverges (10, K-13); completion under remote selection's lifecycle check, after the poll's row is final (11, Part B); an index rebuild re-registering (12, K-17); the minors: the setup poll's stops, Ctrl-C and `request cancel` (Part G), `finish` failures (Part B), a published retry (K4), `setup_pending` and the record's `request_id` in `repo inspect`, `operation resume` (K6, L13), older builds (K-14), cancel's digest (Part H), a lost commit object (Part D) |
