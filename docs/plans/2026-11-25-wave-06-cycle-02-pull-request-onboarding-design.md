---
id: 01M89CHPWV2W6RY0XART0416Q3
title: 'Wave 06 Cycle 02 Design: Onboarding And Setting Changes On A Setup Branch'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 02 Design: Onboarding And Setting Changes On A Setup Branch

Cycle document: [Wave 06 Cycle 02](../Cycles/wave-06-cycle-02-pull-request-onboarding.md). Ticket: vh-c97se (`01M84H84WQYE8J2WJWEDGJ45W4`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md), section "Onboarding and setting changes", and the Cycle document's Product-Owner Decisions.

Status: **draft**, 2026-11-25, revision 2. Author: Claude.

## Intent And Approval Boundary

Let `repo enable` put a new repository's Manyhands setup on a branch for a pull request, let `remote select` do the same for a change of mode, forge or remote in an enabled repository, hold the repository in a setup-pending state until the setup is on its primary, and let a pending setup be cancelled. Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket and document branches, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `da032ffb`; Task 0 re-checks every one.
- "First-time setup" is a setup of kind `enable`: the repository had no configuration. "A change" is a setup of kind `change`: the repository is enabled and its primary holds a configuration.
- "The record" is the repository's row in the new setup table (Part A). "The setup branch" is `manyhands/setup/<setup_id>`. "The base" is the setup remote's primary tip that the setup commit is made on.
- "The setup's remote" is the record's `remote_name`. A setup is "on another remote" when its remote is not the publication remote the primary's configuration names: every first-time setup, and a change that names a first or a different remote (decision 12).
- "Pull-request path" is the path that makes a setup branch; "direct path" is today's commit on the primary.

## Baseline Facts

Each fact was read in the source at `da032ffb` by the planning research and the two round 1 reviews, and re-checked by the author where marked (✓).

1. ✓ **The configuration is a file in the root checkout.** `src/repository.rs` `read_configuration_for` reads `<root>/.manyhands/config.toml` from the working tree. `reconcile_registration` stores `config_blob_oid` from `committed_configuration_blob_oid`, which fails unless HEAD's tree holds the file.
2. ✓ **The transport takes its remote from that file.** `src/repository/transport/operation.rs` `prepare_ssh` sets the remote from `publication_remote(root)`, which answers `ConfigurationInvalid` for a missing or invalid file; `recheck_ssh` reads it again before and after each transfer. `src/repository/transport/remote.rs` `plan_matches_configuration` checks every scoped method's `RemoteRefPlan` against the file. `exact_mappings` knows the primary and the item context refs only. `transfer_inner` takes its scope as `Option<(&RemoteRefPlan, &SynchronizationTarget)>`, allows a delete only with a scope, and writes a tracking ref only for a plan scope or a poll. `delete_exact` with `callbacks.rs` `guard_delete` deletes a context ref only while it is at an expected OID; `observe_delete` reports `Deleted`, `StillPresent` (the delete was refused or lost) or `Changed`.
3. ✓ **The ref plan.** `src/repository/remote/refs.rs` `RemoteRefPlan::from_configuration` fetches the primary, `manyhands/document/*` and `manyhands/ticket/*`; `classify_advertised_ref` ignores any other ref, so polls and discovery never see `manyhands/setup/*`. No code uses `refs/manyhands/`.
4. ✓ **Remote operation rows.** `src/repository/remote/state.rs` `REMOTE_OPERATION_SCHEMA`: `action` and `kind` are `CHECK` constraints; the partial unique index `remote_active_reservation` allows one active row per repository; a row's insert needs the stored plan of `remote_polling_state`. Nine tables reference the table. Its one rebuild (`migrate_synchronization`) ran before those tables existed.
5. **About thirty sites in `src/repository/remote/` read the file** to build or check a plan: `observation.rs` `observation_plan`, `poll.rs` (the plan, materialization and fast-forward re-checks, `poll_configuration_matches`), `sync.rs` `synchronization_configuration_matches_reading` and its callers, and republish.
6. **Poll.** `src/repository/mutation/bind/poll.rs` `prepare` answers `invalid_configuration` for a missing file and `publication_remote_required` for no remote; its family is `Remote`, and `Command::PollOnce.has_operation()` is true. The domain's `poll_probed` (`poll.rs`) stops at `observation_plan` for a missing file and clears the stored plan. The desktop calls `poll_publication_remote` (`poll.rs`) directly, not through the binding, with an `automatic` policy that never prompts. `PollMutationDto` (`mutation/dto.rs`) is read back from a stored answer only with exactly five keys, and twelve goldens hold it.
7. **The primary's fast-forward.** `src/repository/integration/artifacts.rs` `fast_forward` (`pub(in super::super)`) locks the branch ref, requires the ref at the old OID and HEAD to name the branch, checks the tree out safely without overwriting ignored files, and moves the ref; the caller must prove the move is a fast-forward. `src/repository/remote/sync.rs` `require_clean_target` counts an untracked file as dirty.
8. ✓ **Host trust.** `src/repository/mutation/bind/host.rs` `Host::read` takes the remote from the file's publication remote (`invalid_configuration` when missing). `host observe` and `host approve` connect through `verify_ssh_transport` and the `Session` trait's `verify_transport` (`mutation/progress.rs`, three implementations and a test double in `bind/lifecycle_tests.rs`). `VerifySshTransportRequest` has `root`, `direction` and `approval` and about 40 struct literals. Pins are keyed by authority (`read_host_trust`).
9. **No non-SSH publication remote.** `publication_remote_eligible_for` (`src/repository.rs`) parses URLs and contacts nothing. Non-SSH tests select an SSH-shaped URL that is never contacted (`tests/fixtures/cli_input_v1/remote_add.json`, `ssh://git@example.invalid/team/repo.git`). A transport call with no selected key stops with `NoSelectedKey` in `prepare_ssh`, before any connection. libgit2 1.9.7 advertises no refs for an empty remote.
10. **Enable.** `src/repository.rs` `enable_recorded` runs under `begin_lifecycle` with a local operation row, checks the branch (`wrong_branch`), the worktree and the identity, writes the file, commits "Initialize Manyhands" on HEAD and registers. The binding (`mutation/bind/repo.rs`) is confirmed. Its execute answer carries `data: null`. On re-entry `Confirmed::row` answers a completed row from `ended`, whose evidence (`own_commit`, `initialization_commit`) walks the primary's first-parent line for "Initialize Manyhands", and a pending row by comparing the observation digest, which binds `registered`. `written()` feeds that evidence and `fields` the intent digest. `EnableRepositoryRequest` has about 80 struct literals in 44 files, 13 of them in SSH-harness files; `RepoEnableInput` has 8 literals without `..`.
11. **Remote selection.** `set_publication_remote` (`src/repository.rs`) requires a valid file, the primary checked out and the configuration path clean, and commits "Configure Manyhands publication remote" on HEAD. The verb is unconfirmed; its data is `RemoteMutationDto { name, location }`; its re-entry (`moved`) re-reads the file. `RemoteSelectInput` has 16 struct literals. The CLI parser `remote_name` (`src/bin/manyhands-cli/admin.rs`) is shared with `remote remove` and accepts only `name`.
12. ✓ **The mutation boundary.** `src/repository/mutation/execute.rs` `execute_binding` runs `settle_target`, then sends a request with no record to `first_call` (consent, `prepare`, then acceptance in `insert_request_admitted` with the binding's `admit`); `reenter` and `replay` do not call `prepare`. A confirmed verb's `--prepare` runs `confirm.rs` `prepare_call`, which does not reach `first_call`. `command.rs`'s const fns are exhaustive matches (`restores_index` is the pattern). A binding whose answer is `Standing::Owed` leaves its request accepted, and the same request re-enters (`settle.rs`), as close and promote do (`bind/lifecycle.rs` `reached`, `stopped`).
13. ✓ **Local operation rows.** `operation_records.repository_id` is `ON DELETE SET NULL` (`recovery.rs`), so unregistering leaves a root's rows; `repo remove` deletes them by `root_path` in the same transaction as the registration (`registration_removal.rs`). `completed_step` is plain text, and `LocalStep` (`recovery.rs` `local_steps!`) is code with a name-list test.
14. **A registered repository with no configuration.** Create and save answer `repository_not_registered`, sync and poll `invalid_configuration` (`tests/mutation_replay/not_enabled.rs` `a_registered_repository_with_no_configuration_keeps_the_answers_it_had`). Reads answer `ok`; discovery makes the primary context `unverified` and keeps its documents (`discovery.rs` `read_primary`).
15. **The registry.** `migrate_registry` (`discovery.rs`) ends with `migrate_pull_request_mode` (marker `wave_06_pull_request_mode_v1`). `index rebuild` upserts `repositories` and never deletes a row; `repo remove` deletes it and its dependents cascade; a corrupt registry is set aside and replaced by an empty one.
16. ✓ **The repository lease** (`src/repository/coordination.rs` `repository_lease`) is an `fs4` exclusive lock on `manyhands-operation.lock` in Git's common directory, with a 250 ms timeout, released when its process ends. `acquire_exclusive` is private to the module. `snapshot_file_hash` exempts only that file from being read, because a Windows byte lock forbids a second handle's read.
17. **The desktop.** The scheduler polls only a repository with a selected, SSH-eligible publication remote (`desktop/services/scheduler/registration.rs` `read`). The repository row shows "Needs fixing" for any configuration that is not valid, missing included (`desktop/model/repository_state.rs` `needs_fixing`). Add repository sends `RepoEnableInput` with no remote and selects the remote afterwards.
18. ✓ **Schemas.** Each preview kind has its own schema (`schemas/v1/*_preview.schema.json`) and a `PreviewDetailsDto` variant. The schema checker (`tests/support/schema.rs`) accepts `type` as a name or an array, `$ref` only on its own and only to a sibling file, and no `oneOf`; nullable objects are written inline as `"type": ["object", "null"]`. The recovery actions are a fixed set of 13 (`src/results_tests.rs` `RECOVERY_ACTIONS`), with `repo.inspect` and no poll action.
19. **Cycle 01's pieces.** `src/forge.rs` `recognise`, `default_integration`, `pull_request_link` and `PullRequestLink`; `RepositoryConfig::integration()` and `forge()`; `src/repository/remote/sync.rs` `local_only_commits` (private); the result code `primary_has_local_commits` with `PrimaryLocalCommitsDto`.

## Part A: The Setup Record

**Table (R-1).** `repository_setups`, created by a named, additive migration with marker `wave_06_repository_setup_v1`, run by `migrate_registry` after `migrate_pull_request_mode`, schema-checked and in an immediate transaction as Cycle 01's is. No schema object's name may contain "request" (`tests/mutation_replay.rs` asserts it).

```sql
CREATE TABLE IF NOT EXISTS repository_setups (
    repository_id INTEGER PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
    setup_id TEXT NOT NULL UNIQUE,              -- a ULID; the branch is manyhands/setup/<setup_id>
    kind TEXT NOT NULL CHECK(kind IN ('enable','change')),
    remote_name TEXT NOT NULL,
    primary_branch TEXT NOT NULL,
    configuration BLOB NOT NULL,                -- the intended file, the serializer's output
    base_configuration_oid TEXT,                -- change only: the primary's configuration blob when it began
    base_oid TEXT, commit_oid TEXT,             -- the base and the setup commit, once made
    published_oid TEXT,                         -- the setup commit, once the remote is seen to hold it
    request_id TEXT NOT NULL, created_at INTEGER NOT NULL,
    CHECK((kind='change')=(base_configuration_oid IS NOT NULL)),
    CHECK((base_oid IS NULL)=(commit_oid IS NULL)),
    CHECK(published_oid IS NULL OR published_oid=commit_oid)
);
```

OID columns carry the 40-hex `CHECK` the other tables use. One row per repository makes a second setup in this clone impossible (RFC). The row holds no remote URL, credential or identity; the configuration bytes hold what the configuration holds (a remote's name, a forge's web address). Its state is `recorded` until `published_oid` is set, then `published`. The link is not stored: it is built when read (Part I).

**Lifetime.** The record survives `index refresh` and `index rebuild` (Baseline Fact 15). `repo remove` drops it by cascade and leaves the branches (decision 4). A corrupt registry replaced by an empty one loses it (K-2).

**Store API.** A module `src/repository/setup/` (new) with `record.rs`: `read(tx, repository_id) -> Option<SetupRecord>`, `insert`, `record_commit`, `record_published`, `delete`. Every write is one registry transaction.

## Part B: The Setup Gate And Completion

**The table (R-2).** A new const fn on `Command`, exhaustive like `restores_index`:

```rust
pub(crate) enum SetupGate { Carries, RefusedBeforeEnable, Refused }
pub(crate) const fn setup_gate(self) -> SetupGate;
```

| Gate | Commands |
| --- | --- |
| `Refused` (any pending setup) | `ticket close`, `document promote`, `remote select`, `repo enable` |
| `RefusedBeforeEnable` (a first-time setup only) | `document create`, `document save`, `document move`, `document repair`, `ticket create`, `ticket save`, `ticket slug-assign`, `ticket repair`, `folder create`, `comment add`, `item sync`, `repo sync`, `item republish`, `item resolve-history`, `remote resolve-history`, `conflict resolve`, `context sync-local`, `context resolve-local`, `context prepare`, `context release`, `context retire`, `remote remove` |
| `Carries` | `repo create`, `repo remove`, `repo identity-set`, `repo setup-cancel`, `remote add`, every key and host verb, `operation resume`, `operation abandon`, `poll once`, `poll configure`, `poll pause`, `poll resume`, `index refresh`, `index rebuild` |

**Where it runs.** One function, `setup_gate::check(service, command, root) -> Result<(), Answer>`, reads the record (one query) and answers `setup_pending` for a refused command. It runs in `first_call` before `prepare` and again inside the acceptance transaction (`insert_request_admitted`, and the one-transaction path, as `require_unreserved` runs in both), and in `prepare_call` before `preview` (Baseline Fact 12). `reenter` and `replay` are not gated, so the setup's own request continues past its gate. `settle_target` runs first, so a command addressed by a short code it cannot resolve answers as today. The gate does nothing for a root with no registration.

**Closing the race with close and promote.** A change inserts its record in one registry transaction that also finds no unfinished close or promote parent and no accepted `ticket close` or `document promote` request for the repository, or answers `recovery_required` naming it. A close or promote is gated in its acceptance transaction. So a close or promote either is accepted before the change's record exists, and the change refuses, or meets the record and refuses.

**Completion is level-triggered (R-17).** A setup is complete when the local primary already holds it: for a first-time setup, HEAD's tree holds a valid configuration; for a change, HEAD's configuration blob is not `base_configuration_oid`. `setup::complete::finish(service, root)`, under the setup lock (Part D), deletes the local setup branch if it is at the recorded commit (keeping it otherwise), the private base ref, and then the record, and for a first-time setup reconciles the registration and stores the remote plan as remote selection does (`reconcile_registration`, `remote::state::configure`) in the same registry transaction as the record's deletion. It runs:

- in the gate, before it refuses: a complete setup refuses nothing, and is finished if the setup lock is free;
- at the end of the domain's `poll_probed`, so a CLI poll and the desktop's direct poll both finish it;
- in `poll_setup` after its fast-forward (Part G).

A held setup lock skips the finish and changes no answer. Reads treat a complete setup as none: `repo list` compares the record with `repositories.config_blob_oid` (null for a first-time setup until the primary holds a configuration; for a change, not `base_configuration_oid`), and `repo inspect` reads HEAD.

**The answer.** `setup_pending` is a new result code: class `Blocked` (exit 3), message "The repository's Manyhands setup has not merged yet.", recovery `repo.inspect`. It carries no data and nothing is written. For `ticket close --prepare` and `document promote --prepare` it is the envelope's code, not an entry of `data.blockers` (D-1).

## Part C: The Setup Channel

A narrow transport path for the setup branch, so that the poll, synchronization and observation code (Baseline Fact 5) is not taught about the record (R-3, decision 11).

**The scope source.** `PreparedSshAttempt` (`transport/operation.rs`) gains the source of its remote: `Configured`, today's, or `Setup(SetupScope { setup_id, remote_name, primary_branch })`. `prepare_ssh` and `recheck_ssh` read the remote from the file for `Configured`, exactly as today, and from the scope for `Setup`, checking the record again on each recheck (the same `setup_id`, remote and primary, or `EndpointChanged`). Key selection, unlock, host trust and endpoint checks are shared and unchanged. `SshTransportError` does not change (`bind/host.rs` destructures it).

**The entries.** Two new entries take a scope; every existing entry keeps `Configured`:

- `verify_setup_transport(request, scope, session)`: as `verify_ssh_transport`, for `host observe` and `host approve`;
- `with_setup_remote(request, scope, automatic, session, use_remote)`: a connection whose scoped methods are the four below, with today's `automatic` policy for a background poll.

**The methods**, each refusing unless the connection's source is `Setup`, with a setup arm in `transfer_inner` for each:

- `fetch_setup_base()`: fetches `refs/heads/<primary>` into the private ref `refs/manyhands/setups/<setup_id>/base` with `write_tracking_tip`'s compare-and-swap, never into the remote-tracking ref, so it cannot race a poll. It answers the advertisement (the primary's OID or none, and the setup ref's OID or none).
- `push_setup_branch(commit)`: checks its own connection's advertisement first (the setup ref absent, or already at `commit`, which pushes nothing), then pushes `refs/heads/manyhands/setup/<setup_id>` without `+`, guarded in `push_negotiation` by `guard_create`. Its outcome is read by a fresh observation (Part D), because a refused guard surfaces as `TransportUnavailable`.
- `delete_setup_branch(expected)`: `guard_delete` on the setup ref, then `observe_delete`.
- `observe_setup_branch()`: the setup ref's OID from a fresh advertisement.

`guard_create` and `guard_delete`'s decisions are pure predicates over the update count, source, destination and ref name, so a unit test can reach them (`git2::PushUpdate` cannot be built outside git2). No other ref is pushed or deleted through a setup scope, and no item method accepts one.

**Host trust (decision 2).** `Host::read` takes the setup's remote when the repository has a record on another remote, and the file's publication remote otherwise; `host observe` and `host approve` then connect through `verify_setup_transport`, by a new `Session` method `verify_setup_transport` (Part D).

## Part D: Setup Operations

Four operations use the channel: publishing (from enable or remote select), `poll_setup`, cancelling, and completion (Part B, local only). Their concurrency, crash windows and retries are stated as required outcomes and built test-first with a design checkpoint (plan Task 6).

**The setup lock (R-4).** Each setup operation holds a per-repository exclusive file lock, `manyhands-setup.lock` in Git's common directory, taken through a new `coordination.rs` function beside `repository_lease`, for its whole run, the network included. A second setup operation meeting it answers `busy`, and the same request continues later. Item writers never take it. Local Git writes inside a setup operation (the branch, the commit, the fast-forward) take the repository lease for that step only. `snapshot_file_hash` exempts the setup lock as it exempts the repository lease. The lock dies with its process, so a crash leaves nothing held; Ctrl-C ends the process and the request stays accepted.

**No remote reservation (R-5).** Setup operations reserve no `remote_operation_records` row: the setup ref is theirs alone, the base is fetched into a private ref, and the one primary update (activation) runs under the repository lease. This avoids rebuilding that table to add an action (Baseline Fact 4). `request cancel` of a setup's request marks it cancelled as for any request; the record stays, and `repo setup-cancel` removes it.

**Credentials.** The bindings reach the network only through the `Session` trait (`mutation/progress.rs`), which gains `publish_setup`, `poll_setup`, `cancel_setup` and `verify_setup_transport`, in its three implementations and the `Recorder` test double.

**Publishing.** Given a record in state `recorded`, under the setup lock:

1. Fetch the base through the channel. A remote with no primary answers `remote_primary_missing`. A local primary that is not the base or an ancestor of it answers `primary_has_local_commits`, with `local_only_commits` against the base (made visible to `setup`). A first-time setup whose base holds any `.manyhands/config.toml` answers `remote_already_set_up` (decision 13). A change whose base's configuration blob is not `base_configuration_oid` answers `external_change`: the primary moved, and a poll comes first.
2. Under the repository lease, make the setup commit with `commit(None, ..)` and no checkout: its tree is the base's tree with `.manyhands/config.toml` set to the record's `configuration` (mode `100644`), its one parent is the base, its message is "Set up Manyhands" for a first-time setup and "Change Manyhands setup" for a change (R-6), its author and committer the configured identity. Record `base_oid` and `commit_oid`, then create the local setup branch at the commit. On a retry, a recorded commit is reused, and a missing branch is created again.
3. Push the setup branch. Then observe it: at the commit, record `published_oid`; absent, answer the transport's stop and keep the record; at another OID, answer `recovery_required` and keep the record.

A retry after the push and before step 3's record sees the ref at the commit and records it without pushing again. The local primary, the root checkout and the remote's primary never change while publishing.

**Ending a refused setup (R-7).** The four refusals of step 1 end the setup, finally: the request is finished, and a new request decides afresh. A first-time setup's record, its root's operation rows and its registration are deleted in one transaction, as `repo remove` deletes a registration; a supplied identity and the worktree exclusion stay, as after a direct enable that stops. A change's record is deleted, and the repository carries on. A key, unlock, host or network stop keeps the record, in state `recorded`, and leaves the request accepted.

### Required Outcomes Of Part D

| # | Situation | Required outcome | Proof |
| --- | --- | --- | --- |
| S1 | Publish, base present, local equal to or behind the base, base as the setup starts from | setup commit on the base with only the configuration changed; local branch; remote branch at the commit; record published; local primary, checkout and remote primary unchanged | the SSH case; the commit builder's unit test |
| S2 | Remote advertises no primary | `remote_primary_missing`; ended (R-7) | the SSH case; the decision unit test |
| S3 | Local primary has commits the base lacks (ahead or diverged) | `primary_has_local_commits` with the local-only commits; ended | the decision unit test |
| S4 | First-time: the base holds a configuration. Change: the base's configuration is not the starting one | `remote_already_set_up`, or `external_change`; ended | the decision unit test |
| S5 | No selected key, unlock refused, host not trusted, network down, before the push | that stop's code; record `recorded`; the request left accepted; the same request continues | the non-SSH enable test; the SSH case's host stop; the binding's standing unit test |
| S6 | A crash after the commit is recorded and before the branch exists; then a retry | the recorded commit reused, the branch created, published once | the publish unit test with a stubbed channel |
| S7 | A crash after the push and before `published_oid`; then a retry | the ref observed at the commit, recorded, no second push | the publish unit test |
| S8 | The remote setup ref exists at another OID before the push | `recovery_required`; record kept; nothing pushed | the publish unit test |
| S9 | Two setup operations at once (enable retry and cancel, or poll and cancel) | the second answers `busy`; nothing interleaves | the lock unit test |
| S10 | Any setup operation | only `refs/heads/manyhands/setup/<setup_id>` is pushed or deleted; the primary is never pushed | the channel's predicate and mapping unit tests; the SSH case checks the server's primary |

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

**The pull-request path** (a new library entry, `enable_with_setup(request, setup: SetupRequest)`, under `begin_lifecycle` as today):

1. Today's checks: on the primary (`wrong_branch`), clean (`worktree_not_clean`, `worktree_conflicted`), identity (`identity_required`). An unborn HEAD answers `wrong_branch` (R-11).
2. Mint `setup_id`. In one registry transaction: register the repository with `config_blob_oid` null (a new `register_setup_pending`, because `reconcile_registration` needs the file in HEAD), insert the record (kind `enable`, `request_id` the request's), and record the new `LocalStep::SetupRecorded` on the local row and complete it, so the row never blocks another lifecycle. Write the supplied identity and the worktree exclusion, as today.
3. Publish (Part D) through the `Session`.
4. Answer `ok` with data `setup` (Part I), effects `write` and `checkpoint` (the setup commit) and `publication` `published`.

**Preview and consent.** The prepare of a pull-request enable answers `setup_preview.schema.json`: the root, the primary, the identity, the prefix, the mode, the remote's name and redacted location, the forge, and the effects `repository_registered`, `setup_recorded`, `setup_branch_committed`, `setup_branch_published` (new `PreviewEffect` values). The direct preview and its goldens do not change. The observation digest binds, in addition, the mode, the forge and the remote's fetch and push URLs, so a change between prepare and confirm is `external_change`. The prepare contacts no remote.

**Continuing, replay and lost results.** `Confirmed::row` gains a branch before today's: when a record's `request_id` is this request's, the row is `Unfinished` and re-entry publishes from the record, comparing no digest (as close and promote's re-entry does); a published record answers the same `setup` data. A stop after step 2 answers with `Standing::Owed`. A finished request replays its stored `setup` data through `MutationDataDto::from_stored`. A request whose setup was ended or cancelled is finished (`cancelled` for a cancel, Part H) and replays that.

## Part F: Setting Changes Through Remote Select

**Input.** `remote select` gains optional `integration` (`"direct"` or `"pull_request"`) and `forge` (an object as in Part E, or null to remove the `[forge]` table). `RemoteSelectInput` gains both; the CLI's `remote_name` parser splits from `remote remove`'s, with a nullable-object helper for `forge`; `schemas/input/v1/remote_select.schema.json` gains them inline.

**The intended configuration** is the primary's configuration with `publication_remote` set to `name`, and `integration` and `[forge]` replaced when given. Then:

| Current mode | Intended | Path |
| --- | --- | --- |
| direct | direct | today's commit on the primary, unchanged (only `forge` may also change) |
| direct | pull request, any SSH remote | setup branch on the intended remote (decision 12) |
| pull request | the same remote, either mode | setup branch |
| pull request | another remote | `invalid_input` (decision 8) |
| any | equal to the current configuration | `already_applied`, nothing written |

Validation comes first, as today: clearing the remote of a pull-request configuration is `invalid_configuration` (Cycle 01) before any other check.

**The pull-request path** (a new library entry, `change_setup(request)`):

1. Today's checks: a valid configuration, the primary checked out, the configuration path clean, the identity.
2. In one registry transaction (Part B's race rule): no pending setup, no unfinished or accepted close or promote; insert the record (kind `change`, the intended remote, `base_configuration_oid` the primary's configuration blob), and record `LocalStep::SetupRecorded` and complete the local row.
3. Publish (Part D). A refusal at publishing step 1 ends the change (R-7).
4. Answer `ok` with data `setup` (Part I). The configuration on the primary does not change.

Its re-entry continues from the record by its `request_id`, as enable's does; `moved`'s re-read of the file applies to the direct path only. While a change is pending, the gate refuses close, promote, `remote select` and `repo enable`, and every other write carries on in the current mode and on the current remote.

## Part G: Poll: Activation

**Which poll.** `poll once` on a repository whose record is on another remote (every first-time setup, and a change to a first or new remote) runs `poll_setup` instead of today's poll: the binding hands off before its configuration check, `--retire-completed` included (no item context can be complete). A change on the same remote keeps today's poll, which fast-forwards the primary, and completion (Part B) finishes it. `poll_publication_remote` keeps its signature.

**`poll_setup`**, under the setup lock, with the `automatic` policy for the desktop:

| # | What it sees | Outcome | Effect |
| --- | --- | --- | --- |
| G1 | the record is `recorded` | `not_published` | none, no connection |
| G2 | the base holds the setup (first-time: a valid configuration with the same primary branch; change: a configuration blob other than the starting one), and the root is on the primary, clean, and not ahead | `activated` | below |
| G3 | the base holds the setup, but the root is not on the primary, or not clean (untracked files count) | `waiting_for_checkout` | none |
| G4 | the base holds the setup, but the local primary has commits the base lacks | `diverged` | none |
| G5 | the base's configuration is invalid, or names another primary branch | `configuration_invalid` or `primary_differs` | none |
| G6 | the base does not hold the setup, and the remote setup branch is gone | `branch_deleted` | none; the skill says to cancel |
| G7 | otherwise | `waiting_for_merge` | none |

Activation: under the repository lease, fast-forward the local primary to the base with `artifacts::fast_forward` (a no-op when already there), and set the remote-tracking ref of the setup's remote to the base; then completion (Part B) finishes the setup and registers the repository as a direct enable plus remote selection of the merged configuration would; then refresh. The local setup branch and the private ref are deleted before the record, so a crash leaves a record the next poll finishes. `differs` is true when the merged file is not the record's `configuration`; the merged one wins. A merge commit, a squash and a fast-forward of the setup commit each activate.

Every outcome is `ok`, exit 0, and the request is finished: a stop is final and a new request polls again, as poll's are today (CLI contract, poll's R-18). The setup poll reserves no remote row; Task 8's design checkpoint names how the poll binding settles a request with no row. Its data is `setup_poll.schema.json`: `{ type: "setup_poll", setup_id, outcome, integration, differs, pull_request }`, where `integration` is the merged mode on activation and null otherwise. It is a new `MutationDataDto` variant read back by its tag, so `PollMutationDto` and its goldens do not change. Human output prints the outcome's line, and the link while waiting.

## Part H: Repo Setup-Cancel

A new confirmed verb, `repo setup-cancel --repo <root>`, with no input, registered as `remote resolve-history` was (`delivered_by` "C5", R-13). Its `Command` arms: `has_operation` true (a local row), `needs_confirmation` true, `stops_at_safe_points` false, `stage` as enable's.

- **Prepare** reads local state only. With no record it answers `already_applied`. Otherwise its preview (`setup_cancel_preview.schema.json`) names the setup, its kind, branch and state, and the effects `setup_branch_deleted_remotely` (when `commit_oid` is set), `setup_branch_deleted`, `setup_record_removed` and, for a first-time setup, `repository_unregistered`.
- **Confirm**, under the setup lock:
  1. When `commit_oid` is set, fetch the base and observe the setup ref (the S7 window may have pushed an unrecorded branch). If the setup has reached the base (Part G's G2 condition), answer `recovery_required` with recovery `repo.inspect` and change nothing; the skill and human output say to run `poll once` (R-14).
  2. When the remote setup ref is at the recorded commit, delete it with `guard_delete` at that commit: `Deleted` goes on; `Changed` (someone pushed to it) keeps it, and the answer says so; `StillPresent` (the delete was refused or lost) answers the transport's stop, retryable, and changes nothing more. Absent goes on.
  3. Delete the local setup branch (at the recorded commit; kept otherwise) and the private ref; then in one transaction the record and, for a first-time setup, the root's operation rows and the registration, as `repo remove` deletes them.
  4. Finish the setup's own accepted request as `cancelled`, so its retry answers `cancelled`.
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

- **The scheduler** reads the repository's setup before its remotes. A repository with a published setup on another remote reads as `Read::SetupPending`, is scheduled like a polled repository, and is polled through `poll_setup` with the automatic policy; an `activated` outcome refreshes the repository as a completed poll does. Any other repository reads as today, so its read counts do not change.
- **The row** gains a state before `needs_fixing`: a repository with a first-time setup reads "Waiting for setup to merge" (the RFC's words), with no action. Cycle 06 adds "Open pull request page".
- A `setup_pending` answer shows the code's message through the existing failure display (K-9).
- Add repository is unchanged and stays direct: it sends no `publication_remote`, so the default stays direct.

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | The record's table, migration once, constraints, and that it survives a rebuild and goes with `repo remove`; that it holds no remote URL or identity | `src/repository/setup/record_tests.rs`; the migration test beside Cycle 01's in `discovery.rs` |
| P2 | Every command's gate (exhaustive); a first call, a prepare and an acceptance are gated and a re-entry is not; the change's race rule; a complete setup refuses nothing | `command.rs` tests; `src/repository/mutation/setup_gate_tests.rs` |
| P3 | The channel's predicates and mappings: only the setup ref; `guard_create` and `guard_delete` refuse every other update; the scope is rechecked against the record | `src/repository/transport/` unit tests |
| P4 | Part D's S1 to S10 | `src/repository/setup/publish_tests.rs` with a stubbed channel, the lock test, and the SSH case |
| P5 | Enable's mode choice, input refusals, clone rule and direct path with a remote (Part E), as a table | `src/repository/setup/intent_tests.rs`; `tests/repository_enablement.rs` |
| P6 | A pull-request enable that stops at `no_selected_key` leaves a recorded setup: `repo inspect` and `repo list` show it, a gated write answers `setup_pending`, `host observe` answers `no_selected_key` (not `invalid_configuration`), `poll once` answers `not_published`, `repo setup-cancel` unregisters and finishes the request as `cancelled` | `tests/repository_enablement.rs` (non-SSH) and its goldens |
| P7 | Remote select's path table, the remote rules, the race rule (Part F), each pull-request row leaving the local primary unchanged | `src/repository/setup/change_tests.rs`; `tests/repository_enablement.rs` |
| P8 | G1 to G7; activation for a merge commit, a squash and a fast-forward; a crash after the fast-forward finished by the next poll; `differs`; an activated repository's `repo inspect` and registry rows equal to those of a direct enable plus remote selection of the same configuration | `src/repository/setup/poll_tests.rs` with local repositories and a stubbed channel |
| P9 | Completion: a change on the same remote completes after a poll's fast-forward, by the gate and at the end of `poll_probed`, for a merge commit, a squash and a fast-forward; not before; a held lock skips it | `src/repository/setup/complete_tests.rs` |
| P10 | Cancel's preview and outcomes: no commit, published and deleted, moved and kept, refused delete, an unrecorded push, already merged; its crash between steps finished by the same request | `src/repository/setup/cancel_tests.rs`; the SSH case |
| P11 | `host observe` and `host approve` take a setup's remote on another remote | `bind/host.rs` unit test; P6; the SSH case |
| P12 | The new codes, data, preview kinds and verb registered everywhere the named edits list | the existing registration tests |
| P13 | Desktop: a setup-pending repository is scheduled on the setup path, and its row is "Waiting for setup to merge" | desktop model and scheduler tests |
| P14 | End to end over SSH (decision 11): a pull-request enable stops at `host_approval_required`; `host approve` on the pending repository; the same request answers `remote_primary_missing` on a server whose primary was deleted; the primary restored, a new enable publishes with the server's primary unchanged and the same request replays with no new push; a gated write answers `setup_pending`; a squash on the server; `poll once` activates; a change in the activated repository publishes; close's prepare answers `setup_pending`; a peer pushes to the setup branch and cancel keeps it; a second change is cancelled and its remote branch is gone | the SSH case |

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). Decisions are library tests with local `git2` repositories and a stubbed channel; the non-SSH integration tests reach a recorded setup by stopping at `no_selected_key` against a never-contacted SSH-shaped URL (Baseline Fact 9). One SSH case (decision 11) checks that the pieces connect over a real transport, which pushing and deleting a ref need: the host stop and approval, the empty remote, the push, activation and cancel's guarded delete. It goes in `tests/mutation_remote.rs`'s case table, with a comment saying why it needs SSH (R-15). New library tests set a repository-local Git identity, since the machine has none. No real waits.

At review-ready (decision 11): the fast tier, the SSH case on its own, and the `ssh_transport` and `remote_polling` targets. This departs from AGENTS.md's table, which selects every SSH-harness target for a change under `src/repository/transport/`; native CI runs every target after the merge.

## Named Edits To Pre-Existing Tests

- **E-1** (Task 2) `tests/repository_enablement.rs` `registry_creates_repository_and_discovery_metadata_schema`: the table list gains `repository_setups` (67 to 68).
- **E-2** (Task 1) `src/results_tests.rs`: `DESIGN_CODES` gains the three codes (71 to 74) and `result_code_messages_are_fixed` their messages.
- **E-3** (Task 1) `tests/mutation_contract.rs` `CODES_NO_MUTATION_GOLDEN_HAS` and `CODES_NO_FIXTURE_CAN_PRODUCE`: the three codes, with the reason "golden written in Task 7, 9, 10 or 12"; **E-4** removes each when its golden is written.
- **E-5** (Task 3) the 6 `RepositorySummaryDto` and 6 `RepositoryInspectionDto` literals (`src/repository/read/admin.rs`, `src/bin/manyhands-cli/human_tests.rs`, 5 desktop test helpers): add `setup: None`.
- **E-6** (Task 3) the four repository goldens (`tests/fixtures/read_v1/repo_inspect.json`, `repo_list.json`, `tests/fixtures/cli_v1/cli_repo_inspect.json`, `cli_repo_list.json`): add `"setup": null`, regenerated with `MANYHANDS_UPDATE_GOLDEN=1` and read.
- **E-7** (Tasks 3, 7, 8, 10) `tests/read_contract.rs` `contract_enumerations`: every new schema `enum`.
- **E-8** (Task 7) the 8 `RepoEnableInput` literals without `..`, and (Task 9) the 16 `RemoteSelectInput` literals: add the new fields as `None`.
- **E-9** (Task 10) `src/repository/mutation/command.rs` tests that list commands, for `RepoSetupCancel`; `src/bin/manyhands-cli/catalog.rs` and `catalog_tests.rs` (81 to 82); `tests/cli/inventory.rs` (`mutating` 44 to 45, `catalogued` 81 to 82, `NO_INPUT`, `DATA`), with the taxonomy row in `docs/RFC/cli-contract.md` in the same task; `tests/support/conformance.rs` `HOMES` (46 to 47) and its row in `tests/mutation_confirmation/conformance.rs`; `tests/cli/skills.rs` `AUTONOMY` and `skills/manyhands-overview/references/autonomy.md` (decision 14); the help goldens `tests/fixtures/cli_v1/help_root.json` and `help_root_text.txt`.
- **E-10** (Tasks 7 to 12) `tests/support/mutation_golden.rs` `CASES` and `tests/mutation_contract.rs` `DATA_ROOTS` (69 onwards) for each new golden and data schema; `tests/cli/inventory.rs` `DATA` rows for `repo enable`, `remote select`, `poll once` and `repo setup-cancel`.
- **E-11** (Task 1) `schemas/v1/envelope.schema.json` and `request.schema.json` code enums; `src/repository/mutation/outcome.rs` `recovery`; the "no action" lists of `outcome_tests.rs` for `remote_primary_missing` and `remote_already_set_up`.
- **E-12** (Tasks 7 and 10) `schemas/v1/preview_effect.schema.json` and `src/bin/manyhands-cli/human_admin.rs` `effect_phrase` for the new `PreviewEffect` values; the `PreviewDetailsDto` matches in `human_admin.rs` `preview` and the desktop's `view/confirm.rs`.
- **E-13** (Tasks 7, 8 and 10) the exhaustive `MutationDataDto` matches in `tests/support/mutation.rs` (six) and `tests/support/repair.rs` (one), for each new variant, with a `cargo check --all-features --locked --tests` in the same task.
- **E-14** (Task 7) `src/repository/recovery.rs`'s `LocalStep` name-list test (about line 4137): add `setup_recorded`.
- **E-15** (Task 13) the audit unions in `docs/research/wave-03-api-audit.md` and the counts that follow from them in `tests/cli/exits.rs`, `tests/cli/collaboration_exits.rs` and `tests/cli/recovery_tables.rs`: for each recoverable code a verb newly answers (`setup_pending` on every gated verb; `remote_primary_missing`, `remote_already_set_up`, `primary_has_local_commits`, the key, host and network stops, `busy` and `recovery_required` on `repo enable`, `remote select` and `repo setup-cancel`; `setup_pending` on `poll once` is not one), a recovery-table row with its golden or a listed reason.

Task 0's sweep adds what else it finds: exhaustive matches on `Command`, `Mutation`, `MutationDataDto`, `PreviewDetailsDto`, `PreviewEffect` and `Session`; struct literals of the input, DTO and `PreparedSshAttempt` types; schema-closure and enum-count tests; desktop scheduler tests that count reads.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `repo create`, `repo enable` row and input table (the three fields, the mode, the direct path with a remote, the pull-request path, its three refusals, its Owed stops); the `remote select` row and input table (`integration`, `forge`, the setup path, the remote rules, its Owed stops); a `repo setup-cancel` row and its C2 option row; `poll once` (the setup poll, its data and its final stops); `host observe` and `host approve` (a setup's remote); `repo remove` (it drops a setup's record and leaves its branches); `repo list` and `repo inspect` (`setup`); every gated verb (`setup_pending`), including that close's and promote's prepare answer it as the envelope's code and not as a blocker; the recovery tables (E-15).
- **D-2** `docs/RFC/git-workflow-and-conflict-recovery.md`: the setup branch, its commit, its publication and its guarded deletion; that a pull-request setup never commits to or pushes the primary; that setup operations take a setup lock and no remote reservation (R-5), and what Ctrl-C and `request cancel` do to them; the direct enable's remote in its initialization commit (R-9).
- **D-3** `docs/RFC/repository-index-persistence-and-refresh.md`: the `repository_setups` table, its migration and its lifetime; that a pull-request enable registers before its Git state confirms a configuration, with the record as its evidence.
- **D-4** `docs/research/wave-03-api-audit.md`: the new codes in the verbs' unions, and `repo setup-cancel` (E-15).
- **D-5** The shipped skills, in `skills/` and `.agents/skills/`: `manyhands-repositories` (enable in pull-request mode, setting changes, cancel, host trust on a pending setup, the routes by hand of K-1 and K-2), `manyhands-recovery` (`setup_pending`, `remote_primary_missing`, `remote_already_set_up`, a lost record, a setup branch left after the merge: K-8), `manyhands-polling-and-index` (the setup poll and its outcomes), `manyhands-overview` (decision 14's autonomy rows). Each keeps Cycle 01's sentence that pull-request mode is not usable end to end until close and promote support it.
- **D-6** `docs/Waves/wave-06-pull-request-workflow.md`, a managed document: Cycle 02's checklist line "and Add repository" moves to Cycle 06 (decision 5); its "Tests with a local or `file://` remote" reads "Tests: library and non-SSH integration tests, and one SSH case" (decision 11); Cycle 03's "The record of given links and the JSON `pull_request` object (moved from Cycle 01)" becomes "The record of given links (the `pull_request` object comes from Cycle 02)" (decision 6). Applied at approval with `document save` and promoted with the owner's go-ahead.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** One record per repository in `repository_setups`, holding the intended file and no remote URL, credential or identity — one pending setup is the RFC's rule — a second setup in the same clone needs a schema change; two clones can still each open a setup (K-6).
- **R-2** The setup gate is one exhaustive table on `Command`, run at the first call, at acceptance and at prepare — a new command cannot be forgotten, and the race with a change is closed — one registry read per write.
- **R-3** A setup channel in the transport (decision 11's "another path through existing code"), not a setup-aware configuration for every remote path — about thirty remote sites read the file (Baseline Fact 5), and the setup needs one fetch, one push and one delete — today's `Configured` source is the same code, so a fault there breaks every connection, which `ssh_transport` checks.
- **R-4** Setup operations serialize on `manyhands-setup.lock`, held across the network — a file lock dies with its process, and item writers never wait on it — two setup operations at once answer `busy`.
- **R-5** No `remote_operation_records` row for a setup operation — adding an action means rebuilding a table with nine children (Baseline Fact 4), and the setup ref and the private base ref are touched by nothing else — `operation list` does not show a setup's network work; the record and `repo inspect` do.
- **R-6** The change's commit message is "Change Manyhands setup" — a reviewer tells the two kinds apart — none.
- **R-7** A setup refused at its fetch is ended and its request finished — the person must act outside Manyhands (create the primary, move commits, pull) and then try afresh — a retry of the same request replays the refusal.
- **R-8** `repo create` takes none of the new fields — a new local repository has no remote to protect (RFC) — none.
- **R-9** A direct enable writes a given `publication_remote` and `forge` in its initialization commit — one input describes the setup in both modes, and decision 9's default needs the remote — a direct enable with a remote is a shape the desktop does not use yet.
- **R-10** On a clone, supplied fields that disagree with the configuration are `invalid_input` — the shared configuration decides the mode (RFC) — a caller that sends defaults to a clone is refused.
- **R-11** An unborn HEAD in pull-request mode answers `wrong_branch` — activation fast-forwards an existing branch — the code's message ("not on the branch this needs") is loose for this case.
- **R-13** `repo setup-cancel` is registered as `delivered_by` "C5", like vh-hyksw's verbs — the inventory requires one of C1 to C5 (`tests/cli/inventory.rs` `no_verb_is_planned`) — none.
- **R-14** Cancel of a setup that has already merged answers `recovery_required` with `repo.inspect` — deleting a merged setup would leave the primary configured and this clone unregistered — there is no poll recovery action, so the next step is in the skill and human output only.
- **R-15** One SSH case, in `mutation_remote` — decision 11; no non-SSH publication remote exists (Baseline Fact 9), and pushing and deleting a ref are transport behaviour — about a minute of run time.
- **R-16** `poll once` reports a setup on another remote in its own data shape, and does not report a same-remote change's completion — `PollMutationDto` is closed and replayed with exactly five keys, and twelve goldens hold it — a caller learns of that completion from `repo inspect` (K-3).
- **R-17** Completion is level-triggered: the gate, the domain poll and `poll_setup` finish a setup the primary already holds — the desktop polls without the binding, and a primary can move through `git pull` — any move of the primary's configuration completes a change (K-7).

R-12 was retired in revision 2: decision 12 replaced it.

## Known Limits

The Cycle document's K-1 to K-12.

## Unverified

- **U-1** That a top-level nullable schema file passes the read contract's closure and reachability tests (Part I). Task 0 checks it; the fallback is named.

## Alternatives Rejected

- Teaching every remote path to read the record: R-3.
- A new `remote_operation_records` action: R-5.
- `repo remove` abandoning a setup, or a local-only cancel: decision 4.
- Requiring host trust before enable: decision 2.
- A new verb for setting changes, or `repo enable` on an enabled repository: decision 3.
- A new field in `PollMutationDto`: R-16.
- `external_change` for a remote already set up: decision 13.
- A scope field on `VerifySshTransportRequest`: about 40 struct literals; a scope source on `PreparedSshAttempt` and two new entries instead.

## Risks

| Risk | Mitigation |
| --- | --- |
| The scope source breaks today's connections | `Configured` runs today's code; `ssh_transport` runs at review-ready (decision 11) |
| The registration sweep for a verb, three codes, three data shapes and two previews misses a list | Task 0's sweep; the registration tests fail closed |
| A crash leaves a setup half made | Part D's and Part H's required outcomes, each with a test; the record is the journal; the lock dies with its process; completion is level-triggered |
| A pull-request setup pushes the primary | S10: the channel cannot map the primary for a push; the SSH case checks the server's primary |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility | 2 | 10 | 14 | Resolved in revision 2: re-entry through the record and the local row (B1; Part E "Continuing", `LocalStep::SetupRecorded`, R-7's deletion of operation rows); host trust through a scope source and `verify_setup_transport` (B2, Part C); the SSH case reordered (I1, P14); nullable schemas (I2, Part I); completion level-triggered (I3, I10, R-17); the `Session` methods and the automatic policy (I4); the direct path's interface and digest (I5); the taxonomy row in Task 10 (I6); the read test's layer (I7); pure predicates (I8); `transfer_inner`'s setup arm (I9); the minors in place (M5 `StillPresent`, M6, M10, M2, M8 as E-13, M14 in Part B) |
| 1 | contract and consistency | 3 | 13 | 8 | Resolved in revision 2: the remote rule (B1, decision 12); the test departures (B2, decision 11, D-6); merge styles (B3, P8, P9); S4 and the change's ending (I4, R-7); the refusal is final (I5, K-5); the setup's request and cancel (I6, Part H step 4, `request_id` in `setup`); the continuing proof (I7, P14, S5); crash windows (I8, Part G, Part H); completion's place (I9, R-17); limits (I10, K-6 to K-8); D-1 to D-6 (I11); recovery rows and goldens (I12, E-15, Task 12); `remote_already_set_up` (I13, decision 13); privacy wording (I14, Part A); the comparison with a direct enable (I15, P8); the unspecified answers (I16, Parts F and G); the minors in place, with the departures listed in the Cycle document |
