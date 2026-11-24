---
title: "Wave 06 Cycle 01 Design: Pull-Request Setting, Link Builder And Pull-Only Repo Sync"
date: 2026-11-24
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M878QWXT8RVAKS7FB68PH3D7"
---

# Wave 06 Cycle 01 Design: Pull-Request Setting, Link Builder And Pull-Only Repo Sync

Cycle document: [Wave 06 Cycle 01](../Cycles/wave-06-cycle-01-pull-request-setting.md). Ticket: vh-c2dtj (`01M87683441PRAYQJQBQBG96YY`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md) and the Cycle document's Product-Owner Decisions.

Status: **approved 2026-11-24** (revision 4).

## Intent And Approval Boundary

Make the mode and the forge a validated part of the repository's configuration, show them, give later Cycles the functions that recognise a forge and build a pull request link, and make `repo sync` pull-only in a pull-request repository. Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket branch, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `21b811d5`; Task 0 re-checks every one.
- "The table" means `RepositoryConfig::unknown`, the `toml::Table` holding every configuration key other than `format_version`, `primary_branch` and `publication_remote`.
- "Pull-request repository" means a valid configuration with `integration = "pull_request"`.

## Baseline Facts

Each fact was read in the source at `754a42a1`, and checked by the round 1 and round 2 reviews.

1. **The configuration type.** `src/canonical.rs` `RepositoryConfig { primary_branch, publication_remote, unknown: toml::Table }` derives `Clone, Debug, PartialEq`. `parse_repository_config` refuses any `format_version` other than integer 1 ("unsupported format_version; expected 1") and moves every other key into the table. `serialize_repository_config` re-inserts `format_version = 1`, `primary_branch` and `publication_remote` into a copy of the table. `toml` 0.8.23 is built without `preserve_order`, so keys are written sorted. The `v0.2.1` tag's parser holds the same refusal.
2. **Construction sites.** `RepositoryConfig { … }` is built by struct literal in 16 places, including `tests/ticket_closure.rs`, `tests/document_promotion.rs`, `tests/document_promotion/advanced.rs` and `tests/comment_publication.rs`, which are SSH-harness targets.
3. **Readers and the index.** `src/repository.rs` `read_configuration_for` reads the root checkout's working-tree file. The index's `configuration_observations` table stores `state`, `primary_branch`, `publication_remote`, `invalid_code` and `guidance`. `persist_configuration` writes it, and runs only when the primary context is re-persisted; a refresh re-observes only changed or dirty contexts (`whole_refresh_tests.rs` `a_refresh_observes_only_changed_contexts`). `repo list` reads the rows through `src/repository/read/admin.rs` `stored_configuration`. The index snapshot's `SnapshotConfiguration::Valid` holds only the primary branch and the remote, and is not changed by this Cycle.
4. **An invalid configuration.** Mutating verbs answer `invalid_configuration` (blocked, exit 3, recovery `repo.inspect`). `repo inspect`, `repo list` and `index status` still succeed and report it. Remote selection maps a serializer refusal to `RepositoryErrorKind::InvalidConfiguration`.
5. **The configuration object.** `src/repository/read/dto.rs` `ConfigurationDto { state, primary_branch, publication_remote, problems }` is built by literal in 7 places: `src/repository/read/admin.rs`, `src/bin/manyhands-cli/human_tests.rs` and 5 desktop test files. `schemas/v1/repository_configuration.schema.json` has `additionalProperties: false` and requires all four keys. The shapes are in `tests/fixtures/read_v1/repo_inspect.json`, `repo_list.json` and `tests/fixtures/cli_v1/cli_repo_inspect.json`, `cli_repo_list.json`. `human_tests.rs` `repository_and_remote_metadata_escape_controls_at_field_boundaries` counts the lines of human `repo inspect`.
6. **Version 2 as an invalid example.** `src/repository/discovery.rs` (two tests), `tests/canonical_foundation.rs`, `tests/read_repository.rs` and `tests/repository_enablement.rs` (two cases) write `format_version = 2` as an unsupported version. None asserts the message. Two desktop tests use `99`, which stays invalid.
7. **The SSH URL parser.** `src/repository/transport/endpoint.rs` `parse_ssh_endpoint` keeps the host and port, not the path, and is visible only inside `crate::repository`. A change under `src/repository/transport/` makes AGENTS.md select every SSH-harness target.
8. **No non-SSH publication remote.** `publication_remote_eligible_for` requires an SSH endpoint for fetch and push. A test that really fetches or pushes needs the SSH harness. This departs from the RFC's Testing line "CLI contract tests against a local remote", which cannot be met.
9. **`repo sync`.** `src/repository/mutation/bind/sync.rs` binds it to `SynchronizationTarget::Primary`, and `domain_call` passes `MERGING` for every call, including `operation resume` and the `conflict resolve` continuation, which do not run `prepare`. `src/repository/remote/sync.rs` `SynchronizationSettings` has one field, `refuse_divergence`. After the fetch, `graph_plan` returns a `CleanIntegrationPlan { final_oid, local_update, push_needed }` or `MergeRequired`. A remote that is only ahead is fast-forwarded, `push_call` pushes nothing, and the answer is `AlreadyCurrent`: `already_applied`, publication `current` (pinned by `c4_merging_local_behind`). A local primary that is ahead is pushed.
10. **The divergence refusal keeps its reservation.** With `refuse_divergence`, a divergence returns `MergeRequired` before anything of a merge is recorded, and the operation keeps its remote reservation until `request cancel` (`tests/mutation_remote.rs`, the refused-divergence case). `ended_if_unresolved` is the existing way a refusal cancels its operation and lets the reservation go, used for `HistoryUnresolved`.
11. **The journal's outcomes.** The remote journal stores an `authoritative_kind` of `published` or `already_current` only (`src/repository/remote/state.rs`), and `authority_outcome` maps it back on a replay.
12. **A new result code** is registered in: `src/results.rs` `result_codes!`; `src/results_tests.rs`, both the design table with its count (70) and `result_code_messages_are_fixed`; `src/repository/mutation/outcome.rs` `synchronization_error_code` and the exhaustive `recovery`; `schemas/v1/envelope.schema.json` and `schemas/v1/request.schema.json`, checked through `tests/read_contract.rs` `contract_enumerations`; `tests/mutation_contract.rs`, where every code has a mutation golden or a listed reason (`CODES_NO_MUTATION_GOLDEN_HAS`, `CODES_NO_FIXTURE_CAN_PRODUCE`); the recovery tables of `docs/RFC/cli-contract.md`, checked by `tests/cli/recovery_tables.rs`; and `docs/research/wave-03-api-audit.md`.
13. **A new data schema** must be reachable from `DATA_ROOTS` (`tests/mutation_contract.rs`, 68 roots), and every data root must describe a golden's data. Mutation refusals can carry `MutationDataDto` data (an untagged enum): `repo sync` already carries host and conflict data. Each schema `enum` is counted by `tests/read_contract.rs` `published_enumerations_list_exactly_the_registered_values`.
14. **Registry migrations.** `src/repository/discovery.rs` `migrate_registry` runs every migration, idempotently and one way, with named markers in `registry_migrations`. `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` pins the markers committed before the recovery migration, so a new marker belongs after `context_retirement::record::migrate`. `migrate_item_read_schema` is the precedent for marking contexts dirty in a migration.
15. **Percent-encoding.** `percent-encoding` 2.3.2 is in `Cargo.lock` through `url` (a `git2` dependency) but is not a direct dependency.
16. **The desktop** reads `already_applied` with publication `current` from `repo sync` as `SyncOutcome::Current` ("… is the same as the shared copy"). A code it does not know becomes `Failed { code }`, showing the code's message.
17. **The operation row.** A remote operation is a row of `remote_operation_records`, whose target columns are immutable by trigger; the table is not validated by its schema text. It records no mode. Columns have been added to it with `ALTER TABLE … ADD COLUMN`, for example `poll_kind` in `remote::state::migrate`, made immutable by `remote_poll_kind_immutable`.
18. **The synchronization's seams.** `synchronize_remote_observed_inner` calls `graph_plan` three times: for a new operation's push-absence deferral (used only as a yes or no), in the divergence block, and under the repository lease before the local update; the last two are wrapped in `ended_if_unresolved`. `push_call` decides whether to push, records the candidate as `push_oid` even when it pushes nothing, and answers a moved local head with `ExternalChange`. A resumed operation whose local head moved is refused by `reconcile_synchronization` with `RecoveryRequired` before any plan. A lifecycle child's synchronization passes `SynchronizationSettings::default()`. The binding's `stopped` (`bind/sync.rs`) answers a journal row ended as cancelled as the caller's own cancellation, except for `HistoryUnresolved`. A replay rebuilds stored data through `MutationDataDto::from_stored`, which for `repo sync` knows only the synchronization conflict's tagged shape and the host data, and answers `internal_error` for any other.

## Part A: The Configuration

**Representation (R-1).** `RepositoryConfig` keeps its three fields. `integration` and `forge` stay in the table, and two accessors read them:

```rust
pub enum Integration { Direct, PullRequest }          // "direct", "pull_request"
pub enum ForgeKind { BitbucketCloud, BitbucketDataCenter, Github, Gitlab }
pub struct ForgeSetting {                             // from [forge]
    pub kind: Option<ForgeKind>,
    pub web_url: Option<String>,
    pub pull_request_url: Option<String>,
}
impl RepositoryConfig {
    pub fn integration(&self) -> Integration;         // Direct when absent
    pub fn forge(&self) -> Option<ForgeSetting>;      // None when no [forge]
}
```

The types live in `src/canonical.rs`, each with `as_str`. The accessors are infallible: parse and serialize both validate the table. A hand-built configuration with an invalid value reads as `Direct` or `None`, and fails at serialize.

**Validation**, in one function called by both `parse_repository_config` and `serialize_repository_config`. Every refusal is `ValidationCode::InvalidField`:

| Input | Result |
| --- | --- |
| `format_version` 1 or 2 | accepted; any other integer: "unsupported format_version; expected 1 or 2" |
| `integration` absent | direct |
| `integration` `"direct"` or `"pull_request"`, exactly | that mode |
| `integration` any other string, or not a string | "integration must be \"direct\" or \"pull_request\"" |
| `pull_request` with `format_version = 1` | "integration = \"pull_request\" requires format_version = 2" |
| `pull_request` without `publication_remote` | "integration = \"pull_request\" requires publication_remote" |
| `forge` not a table | "forge must be a table" |
| `[forge]` with neither `kind` nor `pull_request_url` | "forge needs a kind or a pull_request_url" |
| `forge.kind` not a string, or not one of the four exactly | "forge.kind must be one of bitbucket-cloud, bitbucket-data-center, github, gitlab" |
| `forge.web_url` or `forge.pull_request_url` not a string, not starting `https://` or `http://`, with an empty host, or containing whitespace or a control character | "forge.<key> must be an absolute http or https URL" |
| `forge.pull_request_url` without `{source}` | "forge.pull_request_url must contain {source}" |
| other keys in `[forge]` | kept |

`{target}` in a template is optional.

**Writing.** `serialize_repository_config` writes `format_version = 2` when `integration()` is `PullRequest`, and `1` otherwise (decision 8). The table keeps its other keys, so a direct-mode file without `[forge]` is written byte for byte as today.

**Unchanged writers.** Enable and create write a direct-mode file as today. Remote selection rewrites the whole file through `serialize_repository_config`, so it keeps `integration`, `[forge]` and the version; this Cycle adds the test that pins that. Clearing the remote of a pull-request repository fails the serializer's validation and answers `invalid_configuration` (Baseline Fact 4).

## Part B: The Index And The Reads

**Migration (R-5).** One named, additive migration, marker `wave_06_pull_request_mode_v1`, run by `migrate_registry` after `context_retirement::record::migrate`, in an immediate transaction that re-checks the marker and the columns first, as `migrate_item_read_schema` does, so that two processes opening at once do not both add a column:

- `ALTER TABLE configuration_observations ADD COLUMN integration TEXT`, and `forge_kind`, `forge_web_url`, `forge_pull_request_url`, all `TEXT` and nullable;
- `ALTER TABLE remote_operation_records ADD COLUMN pull_only INTEGER NOT NULL DEFAULT 0 CHECK(pull_only IN (0,1))`, and its `BEFORE UPDATE OF pull_only` abort trigger (Part D);
- mark every registration's primary context (`contexts.item_id IS NULL`) dirty, as `migrate_item_read_schema` marks contexts, so that the next refresh re-persists the configuration row.

**Persist and read.** `persist_configuration` writes the four columns for a valid configuration and leaves them null otherwise. `stored_configuration` in `read/admin.rs` reads them. A valid row whose `integration` is null (written before the migration and not yet refreshed) reads as `"direct"`, with `forge` null: no build before this one ever wrote these keys. The index snapshot is unchanged (Baseline Fact 3).

**The configuration object.** `ConfigurationDto` gains:

- `integration`: `"direct"` or `"pull_request"` when `state` is `valid`, otherwise null;
- `forge`: `{ "kind", "web_url", "pull_request_url" }`, each a string or null, when the valid configuration has a `[forge]` table, otherwise null. It is the configured setting.

`schemas/v1/repository_configuration.schema.json` requires both. `integration`'s values and `forge.kind`'s values are schema `enum`s, registered in `contract_enumerations`. `repo inspect` builds the fields from its live read, and `repo list` from the index. Human `repo inspect` adds `Integration: <mode>` for a valid configuration and, when there is a forge, `Forge: <kind>`, or `Forge: <pull_request_url>` for a template-only setting. Human `repo list` is unchanged.

## Part C: The Forge Module

A new library module, `src/forge.rs`, declared in `src/lib.rs`, with no GPUI dependency.

**Remote location (R-2).** `RemoteLocation::parse(url: &str) -> Option<RemoteLocation>`, with `host` (lowercased), `port` (`Option<u16>`) and `path` (no leading `/`, no trailing `/` or `.git`). It accepts `ssh://[user@]host[:port]/path` and scp-like `[user@]host:path`, and answers `None` for anything else. It is used only to build links: never to connect, and never for host trust.

**Recognition.** `recognise(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Option<Forge>`, where `Forge { kind: Option<ForgeKind>, location: Option<RemoteLocation>, web_url: Option<String>, template: Option<String> }`:

- a `ForgeSetting` gives a forge with its `kind` (possibly none), `web_url` and template, and the remote's location;
- otherwise a remote location whose host is `bitbucket.org`, `github.com` or `gitlab.com` gives that kind;
- otherwise `None`.

**Default mode.** `default_integration(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Integration` is `PullRequest` when there is a remote URL and `recognise` gives a forge, and `Direct` otherwise.

**The link.** `pull_request_link(forge: Option<&Forge>, source: &str, target: &str) -> PullRequestLink`, with `PullRequestLink { source, target, forge: Option<ForgeKind>, url: Option<String> }`. `forge` is the forge's kind, or none. Branch names are percent-encoded wherever they appear, templates included, with every byte outside `A-Z a-z 0-9 - . _ ~ /` encoded (R-3).

The web base is `web_url` with any trailing `/` dropped when it is set, and otherwise the kind's cloud address (`https://bitbucket.org`, `https://github.com`, `https://gitlab.com`). The remote's own host is never used as a web address.

| Forge | URL, or why none |
| --- | --- |
| a template | the template with `{source}` and `{target}` replaced; it wins over `kind` |
| Bitbucket Cloud | `<base>/<workspace>/<repo>/pull-requests/new?source=<source>&dest=<target>`; the path must have two segments |
| Bitbucket Data Center | `<web_url>/projects/<KEY>/repos/<repo>/pull-requests?create&sourceBranch=refs/heads/<source>&targetBranch=refs/heads/<target>`, with the key upper-cased; no URL without `web_url`, or for a path that is not two segments, or whose first segment starts with `~` (R-8) |
| GitHub | `<base>/<owner>/<repo>/compare/<target>...<source>?expand=1`; two segments |
| GitLab | `<base>/<path>/-/merge_requests/new?merge_request[source_branch]=<source>&merge_request[target_branch]=<target>`; two or more segments |
| none, or an unusable path | no URL |

**The human line.** `PullRequestLink::human_line(&self) -> String` gives `pull request: <url>`, or `open a pull request from <source> into <target>`.

## Part D: Pull-Only Repo Sync

Three review rounds found faults in this mechanism, so it is stated as required outcomes (below) and built test-first with a design checkpoint (plan Task 6).

**The mode belongs to the operation (decision 11, R-11).** `remote_operation_records` gains `pull_only INTEGER NOT NULL DEFAULT 0 CHECK(pull_only IN (0,1))`, added by `ALTER TABLE … ADD COLUMN` as `poll_kind` was, in the same migration as Part B's columns, with a `BEFORE UPDATE OF pull_only` trigger that aborts, as `remote_poll_kind_immutable` does for `poll_kind`. Rows from before the migration are 0, direct.

**Who decides (R-14).** The synchronization decides, not the binding. When `synchronize_remote_observed_inner` reserves a new operation on the primary target that is not a lifecycle child (`execution` is none), it derives `pull_only` from the configuration it has just read and checked (the same read its preflight checks against HEAD and a clean root): 1 for a pull-request repository, 0 otherwise. The value goes into the row's `INSERT`, passed down through `synchronization_reserve_owned`, `reserve_remote_operation_kind` and `insert_operation_with_kind`; the public `reserve_remote_operation` keeps its signature and inserts 0. A context target, a poll and a lifecycle child (close and promote, which pass `SynchronizationSettings::default()`) always insert 0, so close and promote stay direct (decision 1). After reservation, one helper reads the owned row's `pull_only`, and every pull-only check of that call uses its answer: a first call, `operation resume`, the `conflict resolve` continuation, the call `request cancel` makes, a rerun, and a replay race alike. The bindings and `SynchronizationSettings` do not change.

**The decision.** A pull-only operation never merges and never pushes:

- right after the `graph_plan` call of the divergence block and the one under the repository lease (Baseline Fact 18), a plan with `push_needed`, or `MergeRequired`, stops the operation with `SynchronizationError::PrimaryHasLocalCommits { commits, complete }`, before any merge window or local update;
- in `push_call`, a pull-only operation never pushes. It uses `Authority::AlreadyCurrent` when the push endpoint holds the candidate; when the push endpoint is behind the candidate (a separate push URL behind the fetch URL, or a rewind of the push endpoint after the fetch), it stops with the same error, after the local fast-forward and with nothing pushed (R-15). Its list is taken against the push endpoint's advertised tip, or against the fetched remote primary when the push endpoint advertises none.

The list comes from a pure function:

```rust
fn local_only_commits(repository: &git2::Repository, local: git2::Oid, remote: git2::Oid)
    -> Result<(Vec<(git2::Oid, String)>, bool), git2::Error>
```

It walks from `local`, hiding `remote`, in `git2::Sort::TOPOLOGICAL | git2::Sort::TIME` order (newest first), takes up to 20 commits with their summary lines, and gives `complete` false when a 21st exists (R-7).

**Ending the operation (decision 5).** Every pull-only refusal ends its operation through `ended_if_unresolved`, widened to end the operation for `PrimaryHasLocalCommits` as it does for `HistoryUnresolved`: the operation is cancelled and the reservation let go. For a pull-only operation `keep` is false whatever `push_intent` says, because the `decide` closure records the candidate as `push_oid` even when it pushes nothing. The `push_call` site is wrapped as the two `graph_plan` sites already are. The binding's `stopped`, which answers a journal row ended as cancelled as the caller's own cancellation except for `HistoryUnresolved`, is widened the same way, without `HistoryUnresolved`'s repair action.

**A finished pull-only operation's authority.** A pull-only operation always settles with `Authority::AlreadyCurrent`, on a first call and on a resume after `PushVerified`, so its answer is never `published` (decision 4).

**The answer.** `primary_has_local_commits` is a new result code: class `Blocked` (exit 3), message "The local primary branch has commits the remote does not have.", and no recovery action (R-10). Its effects are `not_requested` throughout, set explicitly, `publication` included: nothing was or will be pushed. Its data is a new `MutationDataDto` variant, `PrimaryLocalCommitsDto { type, local_commits: [{ oid, summary }], complete }`, whose `type` is a contract enum with the one value `primary_local_commits`, as `SynchronizationConflictMutationDto`'s is, because `MutationDataDto` is untagged. The `oid` is the full 40 characters. A `PrimaryLocalCommitsDto::from_stored` arm, guarded by the tag and placed before the generic arm, reads it back on a replay. Its schema `schemas/v1/primary_local_commits.schema.json` is listed in `DATA_ROOTS` with the SSH case's golden, and its `type` enum in `contract_enumerations`. Human output prints `error:` and `code:` as for every refusal, then one `local commit: <7-character oid> <summary>` line per commit through the human output's field escaping, and `note: more local commits are not listed` when `complete` is false.

### Required Outcomes Of Part D

| # | Situation | Required outcome | Proof |
| --- | --- | --- | --- |
| O1 | New operation, pull-request repository, local equal to remote | `already_applied`, publication `current`; nothing held | SSH case |
| O2 | New operation, remote ahead | the local primary fast-forwarded; `already_applied`, `current`; nothing pushed | SSH case |
| O3 | New operation, local ahead | `primary_has_local_commits`, effects `not_requested`, no recovery, the commits in `data`; nothing pushed; local primary unchanged; operation ended and nothing held | SSH case; `local_only_commits` and the decision function |
| O4 | New operation, diverged | as O3, with only the local side listed and no merge window recorded | decision function |
| O5 | The same request retried after O3 | the identical envelope, read back through `from_stored` | SSH case; a `from_stored` round-trip unit test |
| O6 | A crash after O3's ending and before its request record is final; then a retry | `cancelled`; a new `repo sync` answers O3 again | Known Limit K-7; not tested |
| O7 | A pull-only operation resumed after the local head moved | `recovery_required` from `reconcile_synchronization`, as for any synchronization today; nothing pushed; held until `request cancel` | existing reconciliation tests, unchanged |
| O8 | A crash after the fetch of a pull-only operation, with local commits already there and the head unchanged; then a resume | the plan's check refuses with `primary_has_local_commits`; operation ended | the decision function on the stored value |
| O9 | A pull-only operation whose push endpoint is behind the candidate | `primary_has_local_commits` after the fast-forward, listed against the push endpoint's tip; nothing pushed; operation ended | `push_call`'s refusal unit test |
| O10 | A resume after `PushVerified` of a pull-only operation | `already_applied`, `current`, never `published` | the authority unit test |
| O11 | A pull-only operation resumed in a repository switched back to direct mode | stays pull-only | the stored-value helper's unit test |
| O12 | A direct-mode operation (merge pending or push prepared) resumed after the repository switched to pull-request mode | finishes in direct mode, as today | the stored-value helper's unit test |
| O13 | `item sync`, a poll, or close's or promote's primary child in a pull-request repository | direct, unchanged | the insert unit test (`pull_only` 0 for each) |
| O14 | Root not on the primary, or not clean | today's refusal, nothing reserved | existing tests, unchanged |
| O15 | `request cancel` of a pull-only operation | as for any synchronization | existing cancellation path |

## Part E: The Desktop

No change (decision 4), beyond the named edits for the configuration object's literals. A pull-only fast-forward reads as current, as a direct one does today, and `primary_has_local_commits` shows its message through `Failed { code }`. Its wording is Cycle 06's.

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | Every validation row of Part A | `tests/canonical_foundation.rs`, one table-driven test |
| P2 | A direct-mode file is written byte for byte as today; a pull-request file as version 2; a version 2 direct file as version 1 | the same target, one test |
| P3 | Remote selection keeps `integration`, `[forge]`, the version and unknown keys of a pull-request configuration | `tests/repository_enablement.rs` |
| P4 | `repo inspect` and `repo list` report `integration` and `forge`, from the file and from the index, with a pre-refresh row reading `"direct"` | `tests/read_repository.rs`, the read contract and the CLI goldens |
| P5 | The migration adds the columns once, and the first refresh after it fills them | a migration test in `src/repository/discovery.rs` |
| P6 | Recognition, default mode and every URL row of Part C | `src/forge.rs` unit tests, table-driven |
| P7 | `local_only_commits`, the pull-only decision after the two plan sites, `push_call`'s refusal and the pull-only authority (O3, O4, O8, O9, O10) | `src/repository/remote/sync_tests.rs`, beside the `graph_plan` test |
| P8 | A new non-child primary operation in a pull-request repository inserts `pull_only` 1, and every other insert 0 (O13); the stored-value helper gives the stored value whatever the configuration says now (O11, O12); the trigger refuses an update; `stopped` answers the refusal as itself, not as `cancelled`; `from_stored` reads the refusal's data back (O5) | the reservation tests in `src/repository/remote/`, `src/repository/mutation/bind/sync_tests.rs`, and `src/repository/mutation/` replay tests |
| P9 | End to end (O1 to O3, O5): a pull-only fast-forward leaves the server's primary unchanged; a local commit is refused, the server's primary is unchanged, the same request retried gives the same envelope, and a following `item sync` is not `busy`; the golden is written | one SSH case in `mutation_remote` |
| P10 | `primary_has_local_commits` is registered everywhere Baseline Facts 12 and 13 list | the existing registration tests |

## Testing

Each test sits at the cheapest layer that catches its regression. P9 is the only SSH case: no non-SSH publication remote can be selected (Baseline Fact 8), and the property it pins (nothing reaches the server, and the repository is not held) needs a real fetch and push. Its edge cases are in P7 and P8. No real waits. A row that pins unchanged behaviour (the equal and remote-ahead rows, the unchanged byte output) cannot fail first, and is exempt from the plan's fail-first rule.

At review-ready the SSH tier is AGENTS.md's `src/repository/remote/` row (`ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`) and `mutation_remote` for `src/repository/mutation/`, plus `ticket_closure`, `document_promotion` and `comment_publication` (R-12).

## Named Edits To Pre-Existing Tests

- **E-1** (Task 1) `src/repository/discovery.rs`, the two tests that write `format_version = 2` as an invalid configuration: `2` to `3`.
- **E-2** (Task 1) `tests/canonical_foundation.rs`, the unsupported-version case: `2` to `3`.
- **E-3** (Task 1) `tests/read_repository.rs`, the invalid-configuration case: `2` to `3`.
- **E-4** (Task 1) `tests/repository_enablement.rs`, the two invalid-version cases: `2` to `3`.
- **E-5** (Task 4) the 7 `ConfigurationDto { … }` literals: add `integration` and `forge`.
- **E-6** (Task 4) the four configuration goldens: add the two fields, regenerated with `MANYHANDS_UPDATE_GOLDEN=1` and read before committing.
- **E-7** (Task 4) `src/bin/manyhands-cli/human_tests.rs` `repository_and_remote_metadata_escape_controls_at_field_boundaries`: the line count grows by the `Integration:` line.
- **E-8** (Task 4) `tests/read_contract.rs` `contract_enumerations`: add the `integration` and `forge.kind` enums.
- **E-9** (Task 3) `src/repository/read/tests.rs`, the calls of `admin::stored_configuration`: pass the new columns.
- **E-10** (Task 5) `src/results_tests.rs`: the design table gains `primary_has_local_commits`, the count becomes 71, and `result_code_messages_are_fixed` gains its message.
- **E-11** retired in revision 4: `SynchronizationSettings` no longer changes.
- **E-12** (Task 8) `tests/mutation_contract.rs` `DATA_ROOTS`: add `primary_local_commits.schema.json` (68 to 69).
- **E-13** (Task 5) `tests/mutation_contract.rs` `CODES_NO_MUTATION_GOLDEN_HAS` (7 to 8) and `CODES_NO_FIXTURE_CAN_PRODUCE` (3 to 4): add `primary_has_local_commits` with the reason "golden written in Task 8", so that no commit is left red.
- **E-14** (Task 8) the same two tables: remove the entries E-13 added (back to 7 and 3).
- **E-15** (Task 8) `tests/read_contract.rs` `contract_enumerations`: add the new schema's `type` enum (`primary_local_commits`).
- **E-16** (Task 8) the golden comparison of `tests/mutation_remote.rs` (`assert_golden`) and its support: replace `data.local_commits[].oid` with a placeholder, as `commit_oid` is replaced.

Task 0's sweep adds what else it finds, under decision 9: the version message; `format_version = 2` in tests; the configuration object's keys in JSON assertions; `stored_configuration` callers; `SynchronizationSettings {` literals; exhaustive matches on `SynchronizationError` and `MutationDataDto`; result-code counts; tests that migrate an older registry and then assert that contexts are clean or the index current, which the new dirty marks change; tests that pin the columns or triggers of `remote_operation_records`; and callers of `synchronization_reserve_owned`, `reserve_remote_operation_kind` and `insert_operation_with_kind`.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `repo sync` row (pull-only in a pull-request repository); the sentence that "a later `repo sync` publishes" (not in pull-request mode); the configuration object's two fields; `primary_has_local_commits` in the recovery tables, with its golden and next step `none`; its data shape.
- **D-2** `docs/RFC/canonical-content-and-comment-schema.md`: `format_version = 2`, `integration` and `[forge]`, and the compatibility decision the RFC's Migration section asks for (decision 8 and the Pull-Request Mode RFC's decision 21). This departs from the Pull-Request Mode RFC's note that it amends the schema RFC "for closure immutability only": decision 21 cannot be delivered without it (R-13).
- **D-3** `docs/RFC/repository-index-persistence-and-refresh.md`: the configuration rows' new columns and the migration's dirty marks.
- **D-4** `docs/research/wave-03-api-audit.md`: `primary_has_local_commits` in `repo sync`'s codes.
- **D-5** The shipped skills, in `skills/` and `.agents/skills/` (each copy edited where they differ): `manyhands-repositories` (the two keys, version 2, how a repository reaches pull-request mode: K-2), `manyhands-sharing-and-sync` (pull-only `repo sync`) and `manyhands-recovery` (`primary_has_local_commits`). Each says that pull-request mode is not usable end to end until close and promote support it (decision 10).
- **D-6** `docs/RFC/git-workflow-and-conflict-recovery.md`, Deliberate Primary Synchronization: it pushes `P:P` in direct mode only; in a pull-request repository it fetches and fast-forwards.
- **D-7** `docs/Waves/wave-06-pull-request-workflow.md`, Cycle 01's checklist: the link record and the guard move out (decisions 1 and 2). Applied at approval.

## Rulings

Each: decision — why — cost if wrong. Revision 1's R-4 and R-6 became the owner's decisions 8 and 4, and their numbers are not reused.

- **R-1** `integration` and `forge` stay in the table behind typed accessors, not new struct fields — no struct literal changes, including in four SSH-harness test files, and validation is in one place — a later Cycle that wants fields refactors 16 sites.
- **R-2** A separate remote-URL parser for links, not a change to `transport/endpoint.rs` — the transport parser drops the path and is crate-private, and a change there forces every SSH target — two parsers can disagree on an odd URL, which costs a missing or wrong link, never a connection or trust decision.
- **R-3** `percent-encoding` 2.3 as a direct dependency, with `/` left unencoded — it is already compiled through `git2`, and forges accept slashes in branch names in these URLs — a forge that needs `%2F` gets a broken link until the set changes.
- **R-5** The migration marks each primary context dirty, so the next refresh re-persists the configuration row — a refresh re-observes only dirty or changed contexts — one primary re-observation per repository after the upgrade.
- **R-7** The refusal names at most 20 commits, newest first, with `complete` — enough to act on, and bounded — a person with more than 20 local commits sees a partial list.
- **R-8** No Data Center URL for a personal repository or an unexpected path — their web paths differ and are not in the RFC — such a repository gets the branch names without a link.
- **R-9** The SSH case is registered in `tests/mutation_remote.rs`'s case table, beside `c4_conformance_repo_sync` — the target already pins `repo sync`, and AGENTS.md selects it for `src/repository/mutation/` — none beyond its run time.
- **R-10** `primary_has_local_commits` has no recovery action — the way out (moving the commits to a branch) is Git work outside Manyhands, which the contract's recovery tables write as `none`, and the refusal holds nothing — an agent gets no automatic next step.
- **R-11** The mode is stored on the operation in its `INSERT`, made immutable by trigger, and every later call reads the stored value (decision 11) — a mode read afresh per call could strand an operation begun in the other mode (round 2), and an `UPDATE` after the insert would leave a crash window reading direct (round 3) — a column, a trigger and a threaded parameter.
- **R-12** The SSH tier at review-ready adds `ticket_closure`, `document_promotion` and `comment_publication` — their test files build configurations the serializer now validates — about 15 more minutes.
- **R-14** The synchronization derives the mode of a new non-child primary operation from its own configuration read, and the bindings and `SynchronizationSettings` do not change — the binding's read comes earlier and could disagree with the synchronization's (round 3), and lifecycle children must stay direct — none; the binding's module documentation gains one sentence that the domain refuses before merging in a pull-request repository.
- **R-15** `push_call` refuses any push in a pull-only operation — a separate push endpoint, or one rewound after the fetch, can be behind the candidate after the local fast-forward, and pull-request mode must never push (round 3 corrected round 2's reason: a moved local head is already `ExternalChange`) — the refusal comes after a fast-forward, which O9 states.
- **R-16** A crash between ending the operation and finishing its request record leaves a retry answering `cancelled` (K-7) — recording the cause on the row adds a column and a state for a window in which nothing changed, and a new `repo sync` answers the refusal again — a caller that retries once sees `cancelled` instead of the refusal.
- **R-13** The canonical schema RFC is amended for the configuration, a departure from the Pull-Request Mode RFC's "closure immutability only" — decision 21's version 2 needs it — none.

## Known Limits

The Cycle document's K-1 to K-8.

## Unverified

- **U-1** The Data Center, GitHub and GitLab URL shapes are the forges' documented forms, not checked against a live forge (decision 7 checks Bitbucket Cloud).

## Alternatives Rejected

- New fields on `RepositoryConfig`: R-1.
- Extending the transport's parser: R-2.
- An interim guard: decision 1.
- A new "updated" answer for a pull-only fast-forward: decision 4.
- Keeping the reservation after the refusal, as the divergence path does: decision 5.
- Reusing `merge_required`: it names a merge as the way forward, which pull-request mode forbids, and it does not cover a primary that is only ahead.

## Risks

| Risk | Mitigation |
| --- | --- |
| A hand-written version 2 file is not byte-canonical, and remote selection refuses to rewrite it | K-5; noted on vh-c97se at approval |
| The live read and the index disagree on the new fields | P4 reads both from one fixture |
| `mutation_remote`'s run time (about 49 minutes) | One case, run singly between steps; the whole target once at review-ready |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility | 3 | 8 | 7 | All resolved in revision 2: the pure decision function and its tests (B1); the refusal ends its operation (B2, decision 5); the golden, `DATA_ROOTS` and schema move with the SSH case (B3); dirty marks (I4); the migration's place (I5); the setting per call (I6, R-11); today's fast-forward answer kept (I7, decision 4); E-7 to E-12 (I8); test paths (I9); the switch reaches the remote first (I10, K-2); pre-refresh rows read `"direct"` (I11); the minors in place |
| 1 | contract and consistency | 2 | 16 | 15 | All resolved in revision 2: the push evidence (B1, decision 6); the 0.2.1 check (B2, A2); test paths (I3); recovery `none` (I4, R-10); the golden (I5); D-1, D-6, R-13 (I6); notes to other tickets (I7); the conflict-resolution exclusion (I8); replay and resume (I9); recognition (I10); the link rules (I11); R-12 (I12); P3 and K-5 (I13); decision 9 and the wider sweep (I14); decision 7 and A7 (I15); the checkout row (I16); Baseline Fact 8's departure (I17); decision 10 (I18); the minors in place |
| 2 | scoped re-review with adversarial round | 3 | 4 | 6 | Resolved in revision 3: the refusal's data is tagged and read back (B1, O5); the mode is stored per operation (B2, B3, decision 11, R-11); `PULL_ONLY` without `refuse_divergence` (B3, R-14); `stopped` widened (I1); the crash window documented (I2, R-16, K-7); `push_call` refuses (I3, R-15); effects `not_requested` (I4); E-13 and E-14 keep every commit green (M1); file lists (M2); the migration's transaction (M3); K-8 (M4); the invalid-configuration path (M5); wording (M6). Part D restated as required outcomes, built test-first with a design checkpoint |
| 3 | scoped re-review with adversarial round | 0 | 5 | 5 | Resolved in revision 4, which has not been independently reviewed: `push_call`'s refusal ends its operation and its real case is stated (1, R-15, O9); the pull-only authority and `keep` (2, O10); O7 restated as today's `recovery_required`, with O8 added (3); the mode inserted, not updated, with a trigger and one read helper (4, R-11); the tagged data's contract enum, E-15 (5); Baseline Facts 17 and 18 (6); the synchronization decides the mode (7, R-14); lifecycle children stay direct (8, O13); E-16 for the golden's commit IDs (9); the binding's documentation and K-8 (10) |
