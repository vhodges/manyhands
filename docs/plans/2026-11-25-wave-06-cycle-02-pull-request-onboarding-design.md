---
id: 01M89CHPWV2W6RY0XART0416Q3
title: 'Wave 06 Cycle 02 Design: Onboarding And Setting Changes On A Setup Branch'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 02 Design: Onboarding And Setting Changes On A Setup Branch

Cycle document: [Wave 06 Cycle 02](../Cycles/wave-06-cycle-02-pull-request-onboarding.md). Ticket: vh-c97se (`01M84H84WQYE8J2WJWEDGJ45W4`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md), section "Onboarding and setting changes", and the Cycle document's Product-Owner Decisions.

Status: **draft**, 2026-11-25, revision 1. Author: Claude.

## Intent And Approval Boundary

Let `repo enable` put a new repository's Manyhands setup on a branch for a pull request, let `remote select` do the same for a change of mode, forge or remote selection in an enabled repository, hold the repository in a setup-pending state until a poll sees the setup on the remote's primary, and let a pending setup be cancelled. Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket and document branches, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `da032ffb`; Task 0 re-checks every one.
- "First-time setup" is a setup of kind `enable`: the repository had no configuration. "A change" is a setup of kind `change`: the repository is enabled and its primary holds a configuration.
- "The record" is the repository's row in the new setup table (Part A). "The setup branch" is `manyhands/setup/<setup_id>`. "The base" is the remote's primary tip the setup commit is made on.
- "Pull-request path" is the path that makes a setup branch; "direct path" is today's commit on the primary.

## Baseline Facts

Each fact was read in the source at `da032ffb` by the planning research and re-checked by the author where marked (✓).

1. ✓ **The configuration is a file in the root checkout.** `src/repository.rs` `read_configuration_for` reads `<root>/.manyhands/config.toml` from the working tree. `reconcile_registration` stores `config_blob_oid` from `committed_configuration_blob_oid`, which fails unless HEAD's tree holds the file.
2. ✓ **The transport takes its remote from that file.** `src/repository/transport/operation.rs` `prepare_ssh` sets the remote from `publication_remote(root)`, which answers `ConfigurationInvalid` for a missing or invalid file; `recheck_ssh` reads it again before and after each transfer. `src/repository/transport/remote.rs` `plan_matches_configuration` checks every scoped method's `RemoteRefPlan` against the file. `exact_mappings` knows the primary and the item context refs only. `delete_exact` with `callbacks.rs` `guard_delete` deletes a context ref only while it is at an expected OID; `observe_delete` reports `Deleted`, `StillPresent` or `Changed`.
3. ✓ **The ref plan.** `src/repository/remote/refs.rs` `RemoteRefPlan::from_configuration` fetches the primary, `manyhands/document/*` and `manyhands/ticket/*`; `classify_advertised_ref` ignores any other ref, so polls never see `manyhands/setup/*`.
4. ✓ **Remote operation rows.** `src/repository/remote/state.rs` `REMOTE_OPERATION_SCHEMA`: `action` and `kind` are `CHECK` constraints; the partial unique index `remote_active_reservation` allows one active row per repository; a row's insert needs the stored plan of `remote_polling_state`. Nine tables reference the table. Its one rebuild (`migrate_synchronization`) ran before those tables existed.
5. **About thirty sites in `src/repository/remote/` read the file** to build or check a plan: `observation.rs` `observation_plan`, `poll.rs` (the plan, materialization and fast-forward re-checks, `poll_configuration_matches`), `sync.rs` `synchronization_configuration_matches_reading` and its callers, and republish.
6. **Poll.** `src/repository/mutation/bind/poll.rs` `prepare` answers `invalid_configuration` for a missing file and `publication_remote_required` for no remote. The domain's `poll_probed` (`poll.rs`) stops at `observation_plan` for a missing file and clears the stored plan. The desktop calls `poll_publication_remote` (`poll.rs`) directly, not through the binding. `PollMutationDto` (`mutation/dto.rs`) is read back from a stored answer only with exactly five keys, and twelve goldens hold it.
7. **The primary's fast-forward.** `src/repository/integration/artifacts.rs` `fast_forward` locks the branch ref, requires HEAD to name the branch, checks the tree out safely without overwriting ignored files, and moves the ref. `src/repository/remote/sync.rs` `require_clean_target` counts an untracked file as dirty.
8. ✓ **Host trust.** `src/repository/mutation/bind/host.rs` `Host::read` resolves the registration and takes the publication remote from the file (`invalid_configuration` when missing). Pins are keyed by authority (`read_host_trust`), not by repository.
9. **No non-SSH publication remote.** `publication_remote_eligible_for` (`src/repository.rs`) parses URLs and contacts nothing. Non-SSH tests select an SSH-shaped URL that is never contacted (`tests/fixtures/cli_input_v1/remote_add.json`, `ssh://git@example.invalid/...`). A transport call with no selected key stops with `NoSelectedKey` in `prepare_ssh`, before any connection.
10. **Enable.** `src/repository.rs` `enable_recorded` checks the branch (`wrong_branch`), the worktree and the identity, writes the file, commits "Initialize Manyhands" on HEAD and registers. The binding (`mutation/bind/repo.rs`) is confirmed; its execute answer carries `data: null`; its lost-result evidence (`initialization_commit`) walks the primary's first-parent line for that message. `EnableRepositoryRequest` has about 80 struct literals in 44 files, 13 of them in SSH-harness files; `RepoEnableInput` has 8 literals without `..`.
11. **Remote selection.** `set_publication_remote` (`src/repository.rs`) requires a valid file, the primary checked out and the configuration path clean, and commits "Configure Manyhands publication remote" on HEAD. The verb is unconfirmed; its data is `RemoteMutationDto { name, location }`. `RemoteSelectInput` has 16 struct literals. The CLI parser `remote_name` (`src/bin/manyhands-cli/admin.rs`) accepts only `name`.
12. ✓ **The mutation boundary.** `src/repository/mutation/execute.rs` `execute_binding` sends a request with no record to `first_call`; `reenter` and `replay` do not call `prepare`. A confirmed verb's `--prepare` runs `confirm.rs` `prepare_call`, which does not reach `first_call`. `command.rs`'s const fns are exhaustive matches (`restores_index` is the pattern for a per-command gate). A binding whose answer is `Standing::Owed` leaves its request accepted, and the same request continues (`settle.rs`), as close and promote do (`bind/lifecycle.rs` `reached`, `stopped`).
13. **A registered repository with no configuration.** Create and save answer `repository_not_registered`, sync and poll `invalid_configuration` (`tests/mutation_replay/not_enabled.rs` `a_registered_repository_with_no_configuration_keeps_the_answers_it_had`). Reads answer `ok`; discovery makes the primary context `unverified` and keeps its documents (`discovery.rs` `read_primary`).
14. **The registry.** `migrate_registry` (`discovery.rs`) ends with `migrate_pull_request_mode` (marker `wave_06_pull_request_mode_v1`). `index rebuild` upserts `repositories` and never deletes a row; `repo remove` deletes it and its dependents cascade; a corrupt registry is set aside and replaced by an empty one.
15. ✓ **The repository lease** (`src/repository/coordination.rs` `repository_lease`) is an `fs4` exclusive lock on `manyhands-operation.lock` in Git's common directory, with a 250 ms timeout, released when its process ends. `acquire_exclusive` takes any lock path.
16. **The desktop.** The scheduler polls only a repository with a selected, SSH-eligible publication remote (`desktop/services/scheduler/registration.rs` `read`). The repository row shows "Needs fixing" for any configuration that is not valid, missing included (`desktop/model/repository_state.rs` `needs_fixing`). Add repository sends `RepoEnableInput` with no remote and selects the remote afterwards (`desktop/view/workspace/add_sheet/share.rs`).
17. ✓ **Each preview kind has its own schema** (`schemas/v1/*_preview.schema.json`) and a `PreviewDetailsDto` variant (`mutation/dto.rs`). The recovery actions are a fixed set of 13 (`src/results_tests.rs` `RECOVERY_ACTIONS`), including `repo.inspect` and no poll action.
18. **Cycle 01's pieces.** `src/forge.rs` `recognise`, `default_integration`, `pull_request_link` and `PullRequestLink`; `RepositoryConfig::integration()` and `forge()`; `src/repository/remote/sync.rs` `local_only_commits`; the result code `primary_has_local_commits` with `PrimaryLocalCommitsDto`.

## Part A: The Setup Record

**Table (R-1).** `repository_setups`, created by a named, additive migration with marker `wave_06_repository_setup_v1`, run by `migrate_registry` after `migrate_pull_request_mode`, schema-checked and in an immediate transaction as Cycle 01's is. The name contains neither "request" (`tests/mutation_replay.rs` asserts no schema object does) nor the `remote_` prefix (the privacy scans read every `remote_%` table).

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
    forge_kind TEXT, link_url TEXT,             -- the link, once published
    request_id TEXT NOT NULL, created_at INTEGER NOT NULL,
    CHECK((kind='change')=(base_configuration_oid IS NOT NULL)),
    CHECK((base_oid IS NULL)=(commit_oid IS NULL)),
    CHECK(published_oid IS NULL OR published_oid=commit_oid)
);
```

OID columns carry the 40-hex `CHECK` the other tables use. One row per repository is the rule that a second setup is refused (RFC). The record holds no URL and no identity. Its state is `recorded` until `published_oid` is set, then `published`.

**Lifetime.** The record survives `index refresh` and `index rebuild` (Baseline Fact 14). `repo remove` drops it by cascade and leaves the branches (decision 4). A corrupt registry replaced by an empty one loses it (K-2).

**Store API.** A module `src/repository/setup/` (new) with `record.rs`: `read(tx, repository_id) -> Option<SetupRecord>`, `insert`, `record_commit`, `record_published`, `delete`. Every write is one registry transaction.

## Part B: The Setup Gate

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

**Where it runs.** One function, `setup_gate::check(service, command, root) -> Result<(), Answer>`, reads the record (one query) and answers `setup_pending` for a refused command. It runs in `first_call` before `prepare`, and in `prepare_call` before `preview` (Baseline Fact 12), so a first call and a `--prepare` are both gated. `reenter` and `replay` are not gated: a request accepted before the setup began is continued as before, and the setup's own request continues past its gate. A change refuses to begin while an unfinished close or promote holds an item (Part F), so no close or promote accepted in the old mode can continue after a change began. The gate does nothing for a root with no registration; resolution answers as today.

**The answer.** `setup_pending` is a new result code: class `Blocked` (exit 3), message "The repository's Manyhands setup is waiting for its pull request to merge.", recovery `repo.inspect`. It carries no data and nothing is written.

## Part C: The Setup Channel

A narrow transport path for the setup branch, so that the poll, synchronization and observation code (Baseline Fact 5) is not taught about the record (R-3).

**The scope.** A `SetupScope { setup_id, remote_name, primary_branch }` replaces the configuration file as the source of the remote for one connection. `prepare_ssh` and `recheck_ssh` take a scope source: the file, as today, or a setup scope, which they check by reading the record again (the same `setup_id`, remote and primary, or `EndpointChanged`). Key selection, unlock, host trust and endpoint checks are unchanged.

**The methods.** One entry, `with_setup_remote(root, scope, direction, session, use_remote)`, gives an `AuthenticatedSshRemote` whose scoped methods for a setup scope are:

- `fetch_setup_base()`: fetches `refs/heads/<primary>` into the private ref `refs/manyhands/setups/<setup_id>/base`, never into the remote-tracking ref, so it cannot race a poll's tracking update. It answers the advertised primary OID, or `None` when the remote advertises no primary.
- `push_setup_branch(commit)`: pushes `refs/heads/manyhands/setup/<setup_id>` without `+`, guarded in `push_negotiation` by a new `guard_create(reference, new)` that refuses unless the update creates the ref (old OID zero) at `commit`.
- `delete_setup_branch(expected)`: `guard_delete` on the setup ref.
- `observe_setup_branch()`: the advertised OID of the setup ref, from a fresh advertisement.

No other ref is pushed or deleted through a setup scope, and no item method accepts one. Changing `src/repository/transport/` selects every SSH-harness target at review-ready (AGENTS.md).

**Host trust for a first-time setup.** `Host::read` (`bind/host.rs`) takes the remote from the record when the file is missing and a first-time record exists, and `host observe` and `host approve` connect through a setup scope. A change keeps the file's remote, which is the record's (Part F).

## Part D: Setup Operations

Four operations use the channel: publishing a setup (from enable or remote select), the first-time setup's poll, cancelling, and completing a change. Three review risks are concurrency, crash windows and retries, so the mechanism is stated as required outcomes (below) and built test-first with a design checkpoint (plan Task 6).

**The setup lock (R-4).** Each setup operation holds a per-repository exclusive file lock, `manyhands-setup.lock` in Git's common directory, taken with `coordination.rs` `acquire_exclusive` for its whole run, the network included. A second setup operation meeting it answers `busy`, and the same request continues later. Item writers never take it. Local Git writes inside a setup operation (the branch, the commit, the fast-forward) take the repository lease for that step only, as other writers do. The lock dies with its process, so a crash leaves nothing held.

**No remote reservation (R-5).** Setup operations reserve no `remote_operation_records` row: the setup ref is theirs alone, the base is fetched into a private ref, and the one primary update (activation) runs under the repository lease with the poll's own checks. This avoids rebuilding that table to add an action (Baseline Fact 4).

**Publishing.** Given a record in state `recorded`:

1. Fetch the base through the channel. A remote with no primary answers `remote_primary_missing`. A local primary that is not the base or an ancestor of it answers `primary_has_local_commits`, with `local_only_commits` against the base. A base whose tree's configuration is not the one the setup starts from answers `external_change`: for a first-time setup, the base holds any `.manyhands/config.toml`; for a change, the base's configuration blob is not `base_configuration_oid`.
2. Under the repository lease, make the setup commit with `commit(None, ..)` and no checkout: its tree is the base's tree with `.manyhands/config.toml` set to the record's `configuration` (mode `100644`), its one parent is the base, its message is "Set up Manyhands" for a first-time setup and "Change Manyhands setup" for a change (R-6), its author and committer the configured identity. Record `base_oid` and `commit_oid`, then create the local setup branch at the commit. On a retry, a recorded commit is reused, and a missing branch is created again.
3. Push the setup branch with `guard_create`. Then observe it: at the commit, record `published_oid` and the link; absent, answer the stop and keep the record; at another OID, answer `recovery_required` and keep the record.
4. The link is `forge::pull_request_link(forge::recognise(remote_url, setting), branch, primary)`, from the record's configuration and the remote's fetch URL read from Git's configuration at that moment; its forge kind and URL are stored, never the remote URL.

A retry after the push and before step 3's record sees the ref at the commit and records it without pushing again. The local primary and the root checkout never change while publishing.

**Ending a refused first-time setup.** `remote_primary_missing`, `primary_has_local_commits` and `external_change` at step 1 of a first-time setup end it: the record and the registration are deleted in one transaction, and nothing is left (R-7). A key, unlock, host or network stop keeps the record, in state `recorded`.

### Required Outcomes Of Part D

| # | Situation | Required outcome | Proof |
| --- | --- | --- | --- |
| S1 | First-time publish, base present, local equal to or behind the base, base without configuration | setup commit on the base with only the configuration added; local branch; remote branch at the commit; record published with the link; local primary, checkout and remote primary unchanged | SSH case 1; the commit builder's unit test |
| S2 | Remote advertises no primary | `remote_primary_missing`; nothing registered, no record, no branch | SSH case 1; the decision unit test |
| S3 | Local primary has commits the base lacks (ahead or diverged) | `primary_has_local_commits` with the local-only commits; as S2 | the decision unit test |
| S4 | The base already holds a configuration (first-time), or not the starting one (change) | `external_change`; first-time as S2; a change keeps its record unchanged and ends | the decision unit test |
| S5 | No selected key, unlock refused, host not trusted, network down, before the push | that stop's code; record `recorded`; the request left accepted; the same request continues | non-SSH enable test (no selected key); the binding's standing unit test |
| S6 | A crash after the commit is recorded and before the branch exists; then a retry | the recorded commit reused, the branch created, published once | the publish unit test with a stubbed channel |
| S7 | A crash after the push and before `published_oid`; then a retry | the ref observed at the commit, recorded, no second push | the publish unit test with a stubbed channel |
| S8 | The remote setup ref exists at another OID before the push | `recovery_required`; record kept; nothing pushed | the publish unit test |
| S9 | Two setup operations at once (enable retry and cancel, or poll and cancel) | the second answers `busy`; nothing interleaves | the lock unit test |
| S10 | Any setup operation | only `refs/heads/manyhands/setup/<setup_id>` is pushed or deleted; the primary is never pushed | the channel's mapping unit test; SSH cases 1 and 2 check the server's primary |

## Part E: Repo Enable In Pull-Request Mode

**Input.** `repo enable` gains three optional fields; `repo create` does not (R-8):

| Field | Type |
| --- | --- |
| `publication_remote` | string |
| `integration` | `"direct"` or `"pull_request"` |
| `forge` | object with optional strings `kind`, `web_url`, `pull_request_url` |

`schemas/input/v1/repo_enable.schema.json` gains them, with `forge` an inline closed object (the input schema checker resolves `$ref` only to a sibling file and has no `oneOf`). The CLI's shared `repository_input` splits, so `repo create` still refuses the new fields. `RepoEnableInput` gains `publication_remote`, `integration` and `forge`; `EnableRepositoryRequest` does not (Baseline Fact 10): the pull-request path is a new library entry, `enable_with_setup(request, setup: SetupRequest)`, and the direct path keeps `enable_with_slug_prefix`.

**The mode, for a repository without configuration.** `integration` when given; otherwise `forge::default_integration(fetch URL of publication_remote, forge)` when `publication_remote` is given (decision 9: a recognised cloud host, or any `forge`, gives pull-request mode); otherwise direct. The intended configuration is built with `primary_branch`, `publication_remote`, `integration` and `[forge]`, and validated by Cycle 01's serializer: a pull-request configuration without a remote, or an invalid forge, answers `invalid_input` before any write. A named remote that is missing or not SSH-eligible answers `invalid_remote`, as remote selection does. In direct mode a given `publication_remote` and `forge` are written in the "Initialize Manyhands" commit, so direct onboarding can be one step (R-9).

**A repository with a valid configuration** (a clone): today's register-only path. A supplied `publication_remote`, `integration` or `forge` must equal the configuration's, or the answer is `invalid_input` and nothing is written, as a different `ticket_slug_prefix` is today (R-10).

**The pull-request path:**

1. Today's checks: on the primary (`wrong_branch`), clean (`worktree_not_clean`, `worktree_conflicted`), identity (`identity_required`). An unborn HEAD answers `wrong_branch` (R-11): there is no primary commit to set up from, and activation must fast-forward an existing branch.
2. Mint `setup_id`. In one registry transaction, register the repository with `config_blob_oid` null (a new `register_setup_pending`, because `reconcile_registration` needs the file in HEAD: Baseline Fact 1) and insert the record (kind `enable`, state `recorded`, `request_id` the request's). Write the supplied identity and the worktree exclusion, as today.
3. Publish (Part D).
4. Answer `ok` with data `setup` (Part I), effects `write` and `checkpoint` (the setup commit), `publication` `published`.

**Preview and consent.** The prepare of a pull-request enable answers `setup_preview.schema.json`: the root, the primary, the identity, the prefix, the mode, the remote's name and redacted location, the forge, and the effects `repository_registered`, `setup_recorded`, `setup_branch_committed`, `setup_branch_published` (new `PreviewEffect` values). The direct preview and its goldens do not change. The observation digest binds, in addition, the mode, the forge and the remote's fetch and push URLs, so a change between prepare and confirm is `external_change`. The prepare contacts no remote.

**Continuing and replay.** A stop after registration (S5) answers with `Standing::Owed`, so the request stays accepted, and a re-entry continues from the record (found by `request_id`) without registering again. A finished request replays its stored `setup` data through `MutationDataDto::from_stored`. A lost result is reconciled from the record and the setup branch, not from "Initialize Manyhands" on the primary.

## Part F: Setting Changes Through Remote Select

**Input.** `remote select` gains optional `integration` (`"direct"` or `"pull_request"`) and `forge` (an object as in Part E, or null to remove the `[forge]` table). `RemoteSelectInput` gains both; the CLI's `remote_name` accepts them; `schemas/input/v1/remote_select.schema.json` gains them inline.

**The intended configuration** is the primary's configuration with `publication_remote` set to `name`, and `integration` and `[forge]` replaced when given. Then:

| Current mode | Intended mode | Path |
| --- | --- | --- |
| direct | direct | today's commit on the primary, unchanged (only `forge` may also change) |
| direct | pull request | setup branch |
| pull request | either | setup branch |

**The pull-request path** (a new library entry, `change_setup(request)`):

1. Today's checks: a valid configuration, the primary checked out, the configuration path clean, the identity.
2. The intended remote must be the current publication remote: otherwise `invalid_input` (decision 8 and R-12). Clearing the remote of a pull-request configuration stays `invalid_configuration` (Cycle 01).
3. No pending setup (the gate answers `setup_pending` first). No unfinished close or promote: otherwise `recovery_required` naming its operation, because it would finish in the old mode.
4. Insert the record (kind `change`, `base_configuration_oid` the primary's configuration blob), then publish (Part D). A refusal at publishing step 1 deletes the record.
5. Answer `ok` with data `setup` (Part I). The configuration on the primary does not change.

While a change is pending, the gate refuses close, promote, `remote select` and `repo enable`, and every other write carries on in the current mode.

## Part G: Poll: Activation And Completion

**A first-time setup's poll.** `poll once` on a repository with a first-time record runs `poll_setup` instead of today's poll: the CLI binding hands off before its configuration check, and the desktop's scheduler calls `poll_setup` for such a repository (Part J). Today's `poll_publication_remote` is unchanged. `poll_setup`, under the setup lock:

| # | What it sees | Outcome | Effect |
| --- | --- | --- | --- |
| G1 | the record is `recorded` | `not_published` | none |
| G2 | the base has no `.manyhands/config.toml` | `waiting_for_merge` | none |
| G3 | the base's configuration is invalid | `configuration_invalid` | none |
| G4 | the base's configuration names another primary branch | `primary_differs` | none |
| G5 | the root is not on the primary, or not clean (untracked files count) | `waiting_for_checkout` | none |
| G6 | the local primary has commits the base lacks | `diverged` | none |
| G7 | otherwise | `activated` | below |

Activation, level-triggered so a crash anywhere is finished by the next poll: under the repository lease, fast-forward the local primary to the base with `artifacts::fast_forward` (a no-op when already there); set the remote-tracking ref to the base; then in one registry transaction reconcile the registration (`config_blob_oid` from HEAD), store the remote plan as remote selection does (`remote::state::configure`), and delete the record; then delete the local setup branch if it is at the recorded commit (kept otherwise, and said so), and the private base ref; then refresh. The repository is now active in the merged configuration's mode, as a direct enable plus remote selection would leave it. `differs` is true when the merged file is not the record's `configuration`; the merged one wins.

Its data is `setup_poll.schema.json`: `{ type: "setup_poll", setup_id, outcome, integration, differs }`, where `integration` is the merged mode on activation and null otherwise. It is a new `MutationDataDto` variant read back by its tag, so `PollMutationDto` and its goldens do not change. Human output prints one line per outcome, and the link again while waiting.

**A change's completion.** A change is complete when the local primary's configuration blob is not `base_configuration_oid`. `complete_change(root)` checks that, under the setup lock, and deletes the local setup branch (at the recorded commit), the private ref and the record. It runs at the end of `poll once` (after its refresh) and of `repo sync`, the two commands that move the primary. The merged configuration is already in effect: today's poll re-reads the file and refits its plan. `poll once`'s data does not report the completion (K-3); `repo inspect` shows `setup` null.

## Part H: Repo Setup-Cancel

A new confirmed verb, `repo setup-cancel --repo <root>`, with no input, registered as `remote resolve-history` was (`delivered_by` "C5", R-13).

- **Prepare** reads local state only. With no record it answers `already_applied`. Otherwise its preview (`setup_cancel_preview.schema.json`) names the setup, its kind, branch and state, and the effects `setup_branch_deleted_remotely` (published only), `setup_branch_deleted`, `setup_record_removed` and, for a first-time setup, `repository_unregistered`.
- **Confirm**, under the setup lock: for a published setup, fetch the base first; if the setup has already reached it (G7's condition for a first-time setup, the base's configuration not the starting one for a change), answer `recovery_required` with recovery `repo.inspect`, and the skill says to run `poll once` (R-14). Otherwise delete the remote setup branch with `guard_delete` at `published_oid`: `Deleted` or absent goes on; `StillPresent` or `Changed` (someone pushed to it) keeps the remote branch and says so. Then delete the local setup branch (at the recorded commit; kept otherwise) and the private ref, and in one transaction the record and, for a first-time setup, the registration.
- A key, unlock, host or network stop changes nothing and leaves the request accepted. An unpublished setup needs no connection.
- **Data** `setup_cancel.schema.json`: `{ type: "setup_cancel", setup_id, kind, remote_branch: "deleted" | "absent" | "kept" | "not_published", local_branch: "deleted" | "absent" | "kept", unregistered }`.

## Part I: Reads And Contract Shapes

- **`pull_request.schema.json`** (decision 6): `{ source, target, forge, url }`, `forge` one of the four kinds or null, `url` a string or null; `PullRequestDto` from `PullRequestLink`. Cycle 03 reuses it.
- **`repository_setup.schema.json`**: `{ id, kind, state, branch, remote, integration, pull_request }`, with `pull_request` null until published.
- **`repo list` and `repo inspect`** gain `setup`: that object or null. `repo list` builds it from the record; `repo inspect` too, as it reads registration today. Human `repo inspect` adds `Setup: waiting for <branch> to merge into <primary>` and the link's human line when there is a record; human `repo list` is unchanged.
- **Mutation data.** `setup_mutation.schema.json` `{ type: "setup", setup }` for enable's and remote select's pull-request path; `setup_poll` and `setup_cancel` above. Each is a tagged `MutationDataDto` variant with a `from_stored` arm guarded by its tag, as Cycle 01's `primary_local_commits` is.
- **Codes.** `setup_pending` (Part B) and `remote_primary_missing`: class `Blocked`, message "The remote has no primary branch to set up from.", no recovery action.

## Part J: The Desktop

The minimum of decision 5:

- **The scheduler** reads a repository whose `repo list` entry has a published first-time `setup` as `Read::SetupPending`, schedules it like a polled repository, and calls `poll_setup` for it; an `activated` outcome refreshes the repository as a completed poll does.
- **The row** gains a state before `needs_fixing`: a repository with a first-time setup reads "Waiting for its setup to merge", with no action. Cycle 06 adds "Open pull request page".
- Add repository is unchanged and stays direct: it sends no `publication_remote`, so the default stays direct.

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | The record's table, migration once, constraints, and that it survives a rebuild and goes with `repo remove` | `src/repository/setup/record_tests.rs`, and the migration test beside Cycle 01's in `discovery.rs` |
| P2 | Every command's gate (exhaustive), and that a first call and a prepare are gated and a re-entry is not | `command.rs` tests; `src/repository/mutation/setup_gate_tests.rs` |
| P3 | The channel maps only the setup ref; `guard_create` and `guard_delete` refuse every other update | `src/repository/transport/` unit tests |
| P4 | Part D's S1 to S10 | `src/repository/setup/publish_tests.rs` with a stubbed channel, the lock test, and the SSH cases |
| P5 | Enable's mode choice, input refusals and clone rule (Part E), as a table | `src/repository/setup/` unit test of the intended configuration; `tests/repository_enablement.rs` |
| P6 | A pull-request enable that stops at `no_selected_key` leaves a recorded setup: `repo inspect` and `repo list` show it, a gated write answers `setup_pending`, `poll once` answers `not_published`, `repo setup-cancel` unregisters | `tests/repository_enablement.rs` (non-SSH, `ssh://git@example.invalid/...`) and the CLI goldens |
| P7 | Remote select's path table, the remote rule, the unfinished-lifecycle rule (Part F) | `src/repository/setup/` unit test; `tests/repository_enablement.rs` |
| P8 | G1 to G7 and activation's effects; a crash after the fast-forward finished by the next poll; `differs` | `src/repository/setup/poll_tests.rs` with local repositories and a stubbed channel |
| P9 | A change completes when the primary's configuration moves, by poll and by `repo sync`, and not otherwise | `src/repository/setup/` unit test |
| P10 | Cancel's preview and outcomes: unpublished, published and deleted, moved and kept, already merged | `src/repository/setup/cancel_tests.rs`; SSH case 2 |
| P11 | `host observe` and `host approve` take a first-time setup's remote | `bind/host.rs` unit test; SSH case 1 observes |
| P12 | The new codes, data and verb registered everywhere Baseline Fact 17 and the named edits list | the existing registration tests |
| P13 | Desktop: a setup-pending repository is scheduled on the setup path and its row is "Waiting for its setup to merge" | desktop model and scheduler tests |
| P14 | End to end, first-time: `remote_primary_missing` on an empty server; then enable publishes with the server's primary unchanged; a gated write answers `setup_pending`; a squash written on the server's primary; `poll once` activates; the repository reads pull-request mode with no setup; a ticket can be created | SSH case 1 |
| P15 | End to end, change: `remote select` turns pull-request mode on through a setup branch; close's prepare answers `setup_pending`; a peer pushes to the setup branch and cancel keeps it; a second change is cancelled and its remote branch deleted; the repository stays direct | SSH case 2 |

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). Decisions are library tests with local `git2` repositories and a stubbed channel; the non-SSH integration tests reach a recorded setup by stopping at `no_selected_key` against a never-contacted SSH-shaped URL (Baseline Fact 9). Two SSH cases, one per feature, check that the pieces connect over a real transport, which is what pushing and deleting a ref needs: P14 for first-time enable (the push, the empty remote, the server's primary unchanged), and P15 for a change (the push, and cancel's guarded delete, a transport behaviour). Both go in `tests/mutation_remote.rs`'s case table, with a comment saying why they need SSH (R-15). No real waits.

At review-ready the SSH tier is every SSH-harness target, because the change touches `src/repository/transport/` (AGENTS.md's table).

## Named Edits To Pre-Existing Tests

- **E-1** (Task 2) `tests/repository_enablement.rs` `registry_creates_repository_and_discovery_metadata_schema`: the table list gains `repository_setups` (67 to 68).
- **E-2** (Task 1) `src/results_tests.rs`: `DESIGN_CODES` gains both codes (71 to 73) and `result_code_messages_are_fixed` their messages.
- **E-3** (Task 1) `tests/mutation_contract.rs` `CODES_NO_MUTATION_GOLDEN_HAS` and `CODES_NO_FIXTURE_CAN_PRODUCE`: both codes, with the reason "golden written in Task 7" or "in Task 12"; **E-4** (Tasks 7 and 12) removes them.
- **E-5** (Task 3) the 6 `RepositorySummaryDto` and 6 `RepositoryInspectionDto` literals (`src/repository/read/admin.rs`, `src/bin/manyhands-cli/human_tests.rs`, 5 desktop test helpers): add `setup: None`.
- **E-6** (Task 3) the four repository goldens (`tests/fixtures/read_v1/repo_inspect.json`, `repo_list.json`, `tests/fixtures/cli_v1/cli_repo_inspect.json`, `cli_repo_list.json`): add `"setup": null`, regenerated with `MANYHANDS_UPDATE_GOLDEN=1` and read.
- **E-7** (Tasks 3, 7, 8, 10) `tests/read_contract.rs` `contract_enumerations`: every new schema `enum`.
- **E-8** (Task 7) the 8 `RepoEnableInput` literals without `..`, and (Task 9) the 16 `RemoteSelectInput` literals: add the new fields as `None`.
- **E-9** (Task 10) `src/repository/mutation/command.rs` tests that list commands, for `RepoSetupCancel`; `src/bin/manyhands-cli/catalog.rs` and `catalog_tests.rs` (81 to 82); `tests/cli/inventory.rs` (`mutating` 44 to 45, `catalogued` 81 to 82, `NO_INPUT`, `DATA`); `tests/support/conformance.rs` `HOMES` (46 to 47); `tests/cli/skills.rs` `AUTONOMY` and `skills/manyhands-overview/references/autonomy.md`; the help goldens `tests/fixtures/cli_v1/help_root.json` and `help_root_text.txt`.
- **E-10** (Tasks 7 to 12) `tests/support/mutation_golden.rs` `CASES` and `tests/mutation_contract.rs` `DATA_ROOTS` (69 onwards) for each new golden and data schema; `tests/cli/inventory.rs` `DATA` rows for `repo enable`, `remote select`, `poll once` and `repo setup-cancel`.
- **E-11** (Task 1) `schemas/v1/envelope.schema.json` and `request.schema.json` code enums; `src/repository/mutation/outcome.rs` `recovery`; the "no action" lists of `outcome_tests.rs` for `remote_primary_missing`.
- **E-12** (Task 7) `schemas/v1/preview_effect.schema.json`, `src/bin/manyhands-cli/human_admin.rs` `effect_phrase` and the desktop's confirm view for the new `PreviewEffect` values.
- **E-13** (Task 13) the audit unions in `docs/research/wave-03-api-audit.md` and the counts that follow from them in `tests/cli/exits.rs`, `tests/cli/collaboration_exits.rs` and `tests/cli/recovery_tables.rs`, for `setup_pending` on the verbs that answer it.

Task 0's sweep adds what else it finds: exhaustive matches on `Command`, `Mutation`, `MutationDataDto`, `PreviewDetailsDto` and `PreviewEffect`; struct literals of the input and DTO types above; schema-closure and enum-count tests; desktop scheduler tests that count reads (`launch_polls_due_ssh_repos`, `clock_event_reads_policies_and_remotes`) if the scheduler's read adds one.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `repo create`, `repo enable` row and input table (the three fields, the mode, the pull-request path, `remote_primary_missing`); the `remote select` row and input table (`integration`, `forge`, the setup path, the remote rule); a `repo setup-cancel` row; `poll once` (the setup poll and its data); `repo list` and `repo inspect` (`setup`); the recovery tables' rows for `setup_pending` and `remote_primary_missing`; the C2 option table for `repo setup-cancel`.
- **D-2** `docs/RFC/git-workflow-and-conflict-recovery.md`: the setup branch, its commit and its deletion; that a pull-request setup never commits to or pushes the primary.
- **D-3** `docs/RFC/repository-index-persistence-and-refresh.md`: the `repository_setups` table, its migration and its lifetime.
- **D-4** `docs/research/wave-03-api-audit.md`: the new codes in the verbs' unions, and `repo setup-cancel`.
- **D-5** The shipped skills, in `skills/` and `.agents/skills/`: `manyhands-repositories` (enable in pull-request mode, setting changes, cancel, host trust on a pending setup, moving a pull-request repository to another remote by hand: K-1), `manyhands-recovery` (`setup_pending`, `remote_primary_missing`, a lost setup record: K-2), `manyhands-polling-and-index` (the setup poll), `manyhands-overview` (the autonomy row of `repo setup-cancel`). Each keeps Cycle 01's sentence that pull-request mode is not usable end to end until close and promote support it.
- **D-6** `docs/Waves/wave-06-pull-request-workflow.md`, Cycle 02's checklist: "and Add repository" moves to Cycle 06 (decision 5). Applied at approval.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** One record per repository in `repository_setups`, holding the intended file and no URL or identity — one pending setup is the RFC's rule, and the privacy scans read every table — a second concurrent setup needs a schema change.
- **R-2** The setup gate is one exhaustive table on `Command`, run in `first_call` and `prepare_call` — a new command cannot be forgotten, and both entry points are covered (Baseline Fact 12) — none beyond one registry read per write.
- **R-3** A setup channel in the transport, not a setup-aware configuration for every remote path — about thirty remote sites read the file (Baseline Fact 5), and the setup needs one fetch, one push and one delete — a fourth setup ref operation later extends the channel.
- **R-4** Setup operations serialize on `manyhands-setup.lock`, held across the network — a file lock dies with its process, so a crash holds nothing, and item writers never wait on it — two setup operations at once answer `busy`.
- **R-5** No `remote_operation_records` row for a setup operation — adding an action means rebuilding a table with nine children (Baseline Fact 4), and the setup ref and the private base ref are touched by nothing else — a setup push and an item sync can run at once, which is safe because their refs differ.
- **R-6** The change's commit message is "Change Manyhands setup" — a reviewer tells the two kinds apart — none.
- **R-7** A first-time setup refused at its fetch is ended, its registration and record removed — the person must act outside Manyhands (create the primary, move commits, pull) and then enable afresh — a retry of the same request answers the same refusal after a new fetch.
- **R-8** `repo create` takes none of the new fields — a new local repository has no remote to protect (RFC) — none.
- **R-9** A direct enable writes a given `publication_remote` and `forge` in its initialization commit — one input describes the setup in both modes — a direct enable with a remote is a new shape the desktop does not use yet.
- **R-10** On a clone, supplied fields that disagree with the configuration are `invalid_input` — the shared configuration decides the mode (RFC) — a caller that sends defaults to a clone is refused.
- **R-11** An unborn HEAD in pull-request mode answers `wrong_branch` — activation fast-forwards an existing branch — the code's message ("not on the branch this needs") is loose for this case.
- **R-12** A setting change keeps the current publication remote — decision 8, and it keeps host trust and the poll on one remote — a direct repository with no remote, or with an unshared one, cannot switch to pull-request mode through Manyhands (K-1).
- **R-13** `repo setup-cancel` is registered as `delivered_by` "C5", like vh-hyksw's verbs — the inventory requires one of C1 to C5 (`tests/cli/inventory.rs` `no_verb_is_planned`) — none.
- **R-14** Cancel of a setup that has already merged answers `recovery_required` with `repo.inspect` — deleting a merged setup would leave the remote's primary configured and this clone unregistered — there is no poll recovery action, so the next step is in the skill and human output only.
- **R-15** Two SSH cases, one per feature, in `mutation_remote` — no non-SSH publication remote exists (Baseline Fact 9), and pushing and deleting a ref are transport behaviour — about two minutes of run time.
- **R-16** `poll once` reports a first-time setup in its own data shape, and does not report a change's completion — `PollMutationDto` is closed and replayed with exactly five keys, and twelve goldens hold it — a caller learns of a change's completion from `repo inspect`.

## Known Limits

The Cycle document's K-1 to K-8.

## Unverified

- **U-1** That libgit2 advertises no refs for an empty remote, so that S2 cannot be told from a remote without that one branch. Both answer `remote_primary_missing`; SSH case 1 checks the empty case.
- **U-2** The exact preview schema layout for the two new previews is read at Task 0 from an existing `*_preview.schema.json`.

## Alternatives Rejected

- Teaching every remote path to read the record: R-3.
- A new `remote_operation_records` action: R-5.
- `repo remove` abandoning a setup, or a local-only cancel: decision 4.
- Requiring host trust before enable: decision 2.
- A new verb for setting changes, or `repo enable` on an enabled repository: decision 3.
- A new field in `PollMutationDto`: R-16.

## Risks

| Risk | Mitigation |
| --- | --- |
| The transport change breaks an item path | The setup scope is a separate source and separate methods; every SSH target runs at review-ready |
| The registration sweep for a verb, two codes and five data shapes misses a list | Task 0's sweep; the registration tests fail closed |
| A crash leaves a setup half made | Part D's required outcomes, each with a test; the record is the journal; the lock dies with its process |
| A pull-request setup pushes the primary | S10: the channel cannot map the primary for a push; both SSH cases check the server's primary |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
