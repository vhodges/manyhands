---
title: "Wave 06 Cycle 01 Design: Pull-Request Setting, Link Builder And Pull-Only Repo Sync"
date: 2026-11-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M878QWXT8RVAKS7FB68PH3D7"
---

# Wave 06 Cycle 01 Design: Pull-Request Setting, Link Builder And Pull-Only Repo Sync

Cycle document: [Wave 06 Cycle 01](../Cycles/wave-06-cycle-01-pull-request-setting.md). Ticket: vh-c2dtj (`01M87683441PRAYQJQBQBG96YY`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md) and the Cycle document's Product-Owner Decisions.

Status: **draft** (revision 1).

## Intent And Approval Boundary

Make the mode and the forge a validated part of the repository's configuration, show them, give later Cycles the functions that recognise a forge and build a pull request link, and make `repo sync` pull-only in a pull-request repository. Approving this design approves the Required Contract of the Cycle document and the rulings below. It does not authorize implementation, pushes other than the ticket branch, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at ticket head `adeae732`; Task 0 re-checks every one.
- "The table" means `RepositoryConfig::unknown`, the `toml::Table` that holds every configuration key other than `format_version`, `primary_branch` and `publication_remote`.
- "Pull-request repository" means a valid configuration with `integration = "pull_request"`.

## Baseline Facts

Each fact was read in the source at `adeae732`.

1. **The configuration type.** `src/canonical.rs` `RepositoryConfig { primary_branch, publication_remote, unknown: toml::Table }` derives `Clone, Debug, PartialEq`. `parse_repository_config` refuses any `format_version` other than integer 1 with `ValidationCode::InvalidField` and "unsupported format_version; expected 1", and moves every other key into the table. `serialize_repository_config` re-inserts `format_version = 1`, `primary_branch` and `publication_remote` into a copy of the table and writes it with `toml::to_string`. `toml` 0.8 is built without `preserve_order`, so the table is sorted and the written keys come out alphabetically.
2. **Construction sites.** `RepositoryConfig { … }` is built by struct literal in 16 places, including `tests/ticket_closure.rs`, `tests/document_promotion.rs`, `tests/document_promotion/advanced.rs` and `tests/comment_publication.rs`, which are SSH-harness targets.
3. **Readers.** `src/repository.rs` `read_configuration_for` reads the root checkout's working-tree file. `src/repository/mutation/bind/ticket.rs` and `src/repository/promotion/run.rs` read the committed blob. The index's `configuration_observations` table (`src/repository/discovery.rs` `migrate_registry`) stores `state`, `primary_branch`, `publication_remote`, `invalid_code` and `guidance`; `persist_configuration` writes it, and `stored_configuration` rebuilds a configuration for the index snapshot by synthesizing `format_version = 1` source from those columns.
4. **An invalid configuration.** Mutating verbs answer `invalid_configuration` (blocked, exit 3, recovery `repo.inspect`). `repo inspect`, `repo list` and `index status` still succeed and report the configuration as invalid.
5. **The configuration object.** `src/repository/read/dto.rs` `ConfigurationDto { state, primary_branch, publication_remote, problems }` is built in 12 places, 6 of them in desktop tests. `schemas/v1/repository_configuration.schema.json` has `additionalProperties: false` and requires all four keys. The shapes are pinned by `tests/fixtures/read_v1/repo_inspect.json`, `repo_list.json` and `tests/fixtures/cli_v1/cli_repo_inspect.json`, `cli_repo_list.json`. Human `repo inspect` is `src/bin/manyhands-cli/human.rs` `repository_inspection`, with no golden.
6. **Version 2 as an invalid example.** These tests use `format_version = 2` as an unsupported version: `src/repository/discovery.rs` (two tests, at about 4463 and 4693), `tests/canonical_foundation.rs` (about 332), `tests/read_repository.rs` (about 681) and `tests/repository_enablement.rs` (about 1088 and 3431). Two desktop tests use `format_version = 99`, which stays invalid.
7. **The SSH URL parser.** `src/repository/transport/endpoint.rs` `parse_ssh_endpoint` accepts `ssh://[user@]host[:port]/path` and scp-like `[user@]host:path`, keeps the host and port as `SshAuthority`, and does not keep the path separately. The module is visible only inside `crate::repository`. A change under `src/repository/transport/` makes AGENTS.md select every SSH-harness target.
8. **No non-SSH publication remote.** `publication_remote_eligible_for` requires `configured_remote_endpoint` to succeed for fetch and push, so `file://` and local paths cannot be selected. A test that really fetches or pushes needs the SSH harness.
9. **`repo sync`.** `src/repository/mutation/bind/sync.rs` binds `repo sync` to `SynchronizationTarget::Primary` and calls `synchronize` with `MERGING`. `src/repository/remote/sync.rs` `SynchronizationSettings` has one field, `refuse_divergence`; when it is set, a divergence is refused with `SynchronizationError::MergeRequired` "before anything of a merge is recorded or prepared". After the fetch, `graph_plan` classifies the relation. A remote that is only ahead is fast-forwarded locally, and `push_call` pushes nothing when the remote already holds the candidate, so the answer is `SynchronizationOutcome::AlreadyCurrent`: code `already_applied`, `effects.publication` `current`. A local primary that is ahead is pushed.
10. **Primary pushes.** Every push of the primary goes through `src/repository/remote/refs.rs` `RemoteRefPlan::primary_push_refspec` and `push_exact`. No guard is added there (decision 1).
11. **The desktop's classification.** `src/desktop/model/publication.rs` `classify_sync` keys first on `effects.publication`: `Published` gives `SyncOutcome::Published` ("Shared …"), `Current` gives `SyncOutcome::Current` ("… is the same as the shared copy"), and `NotRequested` or `Pending` falls through to the code.
12. **A new result code** is registered in: `src/results.rs` `result_codes!`; `src/results_tests.rs` (the design table and its count, 70 today); `src/repository/mutation/outcome.rs` `synchronization_error_code` and the exhaustive `recovery`; `schemas/v1/envelope.schema.json` and `schemas/v1/request.schema.json`, checked by `tests/read_contract.rs` `contract_enumerations`; `tests/mutation_contract.rs` `CODES_NO_MUTATION_GOLDEN_HAS` or a `tests/fixtures/mutation_v1` golden listed in `tests/support/mutation_golden.rs`; the recovery tables of `docs/RFC/cli-contract.md`, checked by `tests/cli/recovery_tables.rs`; and `docs/research/wave-03-api-audit.md`, which that test reads.
13. **Registry migrations.** `src/repository/discovery.rs` `migrate_registry` runs every migration, idempotently and one way. Named markers are rows of `registry_migrations`; `src/repository/closure/record.rs` (`MARKER`, `migrate`) is the template. `tests/repository_enablement.rs` `registry_creates_repository_and_discovery_metadata_schema` pins the table list, and `tests/recovery_foundation_gate.rs` pins the markers written in the first migration transaction.
14. **Percent-encoding.** `percent-encoding` 2.3.2 is in `Cargo.lock` through `url` (a `git2` dependency) but is not a direct dependency.

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

The types live in `src/canonical.rs` beside `RepositoryConfig`, each with `as_str`. The accessors are infallible: parse and serialize both validate the table, so a configuration that reached a caller holds valid values. A hand-built configuration with an invalid value reads as `Direct` or `None`, and fails at serialize.

**Validation**, in `parse_repository_config` and `serialize_repository_config`, both through one function:

| Input | Result |
| --- | --- |
| `format_version` 1 or 2 | accepted; any other integer is "unsupported format_version; expected 1 or 2" |
| `integration` absent | direct |
| `integration` `"direct"` or `"pull_request"` | that mode |
| `integration` any other value or type | `InvalidField`, "integration must be \"direct\" or \"pull_request\"" |
| `pull_request` with `format_version = 1` | `InvalidField`, "integration = \"pull_request\" requires format_version = 2" |
| `pull_request` with no `publication_remote` | `InvalidField`, "integration = \"pull_request\" requires publication_remote" |
| `forge` not a table | `InvalidField`, "forge must be a table" |
| `forge.kind` not one of the four | `InvalidField`, naming the four |
| `forge.web_url`, `forge.pull_request_url` not an absolute `https://` or `http://` URL, or containing whitespace | `InvalidField` |
| `forge.pull_request_url` without `{source}` | `InvalidField` |
| other keys in `[forge]` | kept |

**Writing.** `serialize_repository_config` writes `format_version = 2` when `integration()` is `PullRequest`, and `1` otherwise (R-4). The table keeps its other keys, so a direct-mode file without `[forge]` is written byte for byte as today and the strings pinned in `src/repository.rs` tests and `tests/mutation_confirmation.rs` do not change.

**Unchanged writers.** Enable and create write a direct-mode file as today. Remote selection rewrites the whole file through `serialize_repository_config`, so it keeps `integration`, `[forge]` and the version; this Cycle adds the test that pins it (nothing pins remote selection's preservation of the table today).

## Part B: The Index And The Reads

**Migration (R-5).** One named, additive migration, marker `wave_06_configuration_mode_v1`, run by `migrate_registry` outside the first transaction:

- `ALTER TABLE configuration_observations ADD COLUMN integration TEXT`, and `forge_kind`, `forge_web_url`, `forge_pull_request_url`, all `TEXT` and nullable;
- `UPDATE repositories SET refresh_required = 1`, so the next refresh rewrites every configuration row.

**Persist and rebuild.** `persist_configuration` writes the four columns for a valid configuration and leaves them null otherwise. `stored_configuration` synthesizes `integration` and `[forge]` from them, and `format_version = 2` for pull-request mode, so the snapshot's configuration parses to the same value as the file.

**The configuration object.** `ConfigurationDto` gains:

- `integration`: `"direct"` or `"pull_request"` when `state` is `valid`, otherwise null;
- `forge`: `{ "kind", "web_url", "pull_request_url" }` (each a string or null) when the valid configuration has a `[forge]` table, otherwise null.

`schemas/v1/repository_configuration.schema.json` requires both, with `forge`'s object inline and `additionalProperties: false`. `repo inspect` builds them from its live read; `repo list` from the index columns. Human `repo inspect` adds `Integration: <mode>` and, when set, `Forge: <kind or web_url>`.

## Part C: The Forge Module

A new library module, `src/forge.rs`, declared in `src/lib.rs`, with no GPUI dependency.

**Remote location (R-2).** `RemoteLocation::parse(url: &str) -> Option<RemoteLocation>`, with `host` (lowercased), `port` (`Option<u16>`) and `path` (no leading `/`, no trailing `/` or `.git`). It accepts `ssh://[user@]host[:port]/path` and scp-like `[user@]host:path`, and answers `None` for anything else. It is used only to build links: never to connect, and never for host trust.

**Recognition.** `recognise(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Option<Forge>`, where `Forge` holds the kind (or none, for a template-only setting), the location, the web address and the template:

- a `ForgeSetting` with a `kind` or a `pull_request_url` wins;
- otherwise the location's host `bitbucket.org`, `github.com` or `gitlab.com` gives that kind;
- otherwise `None`.

**Default mode.** `default_integration(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Integration` is `PullRequest` when there is a remote URL and `recognise` gives a forge, and `Direct` otherwise (RFC decision 9).

**The link.** `pull_request_link(forge: Option<&Forge>, source: &str, target: &str) -> PullRequestLink`, with `PullRequestLink { source, target, forge: Option<ForgeKind>, url: Option<String> }`. Branch names are percent-encoded with every byte outside `A-Z a-z 0-9 - . _ ~ /` encoded (R-3). A trailing `/` on `web_url` is dropped.

| Forge | URL, or why none |
| --- | --- |
| template | the template with `{source}` and `{target}` replaced |
| Bitbucket Cloud | `https://bitbucket.org/<workspace>/<repo>/pull-requests/new?source=<source>&dest=<target>`; the path must have two segments |
| Bitbucket Data Center | `<web_url>/projects/<KEY>/repos/<repo>/pull-requests?create&sourceBranch=refs/heads/<source>&targetBranch=refs/heads/<target>`, with the key upper-cased; no URL without `web_url`, or for a path that is not two segments or whose first segment starts with `~` (a personal repository) (R-8) |
| GitHub | `https://github.com/<owner>/<repo>/compare/<target>...<source>?expand=1`; two segments |
| GitLab | `https://gitlab.com/<path>/-/merge_requests/new?merge_request[source_branch]=<source>&merge_request[target_branch]=<target>`; two or more segments |
| none, or an unusable path | no URL |

With `web_url` set, a cloud kind uses it in place of the cloud host.

**The human line.** `PullRequestLink::human_line(&self) -> String` gives `pull request: <url>`, or `open a pull request from <source> into <target>`.

## Part D: Pull-Only Repo Sync

**Settings.** `SynchronizationSettings` gains `pull_only: bool`: "Never merge and never push. A local target with commits the remote lacks is `PrimaryHasLocalCommits`, before anything of a merge, a local update or a push is prepared." A pull-only setting also sets `refuse_divergence`, so a synchronization that already holds a merge of its own, begun without it, stays `RecoveryRequired` as the field's comment says.

**The binding.** `SyncBinding` for `repo sync` reads the repository's configuration, which its `prepare` already needs for the remote, and calls `synchronize` with a new `PULL_ONLY` setting in a pull-request repository and `MERGING` otherwise. `item sync` always uses `MERGING`.

**The decision.** In `synchronize_remote_observed_inner`, after `graph_plan` and before the divergence check, a pull-only primary synchronization whose local primary holds commits that the fetched remote primary does not (local ahead, or diverged) stops with `SynchronizationError::PrimaryHasLocalCommits { commits, complete }`. `commits` lists up to 20 of those commits, newest first, each with its ID and first summary line (R-7). The reservation is released as for every refusal; only the fetch's tracking ref has moved.

**The outcomes.**

| Relation after the fetch | Today (direct mode) | Pull-only |
| --- | --- | --- |
| equal | `already_applied`, publication `current` | the same |
| remote ahead | fast-forward; `already_applied`, publication `current` | fast-forward; `ok`, publication `not_requested`, `commit_oid` the new head (R-6) |
| local ahead | push; `ok`, publication `published` | `primary_has_local_commits`, nothing moved |
| diverged | merge and push | `primary_has_local_commits`, nothing moved |

The pull-only fast-forward is a new `SynchronizationOutcome::FastForwarded { target, oid }`, made only by a pull-only synchronization whose local ref moved. The index step runs as for every synchronization that moves its target.

**The answer.** `primary_has_local_commits` is a new result code: class `Blocked` (exit 3), message "The local primary branch has commits the remote does not have.", recovery `repo.inspect` (R-10). Its data is a new `MutationDataDto` variant, `{ "local_commits": [{ "oid", "summary" }], "complete" }`, with `schemas/v1/primary_local_commits.schema.json`. Human output prints `error:` and `code:` as for every refusal, then one `local commit: <short oid> <summary>` line per commit, and a `note:` line when the list is not complete.

**Replay and resume.** A retried request returns its stored answer. `operation resume` of a pull-only synchronization interrupted after its fast-forward finishes it like any synchronization. A pending primary merge begun in direct mode before the switch is `RecoveryRequired` under the pull-only setting (Baseline Fact 9).

## Part E: The Desktop

`classify_sync` maps a `repo sync` answer with code `ok` and publication `not_requested` to a new `SyncOutcome::Updated`, worded "Updated <primary>" in `src/desktop/model/activity/copy.rs` and `src/desktop/model/operations/outcomes.rs`. `already_applied` with publication `current` stays `SyncOutcome::Current`. `primary_has_local_commits` falls through to `Failed { code }`, which shows its message; its own wording is Cycle 06's. No label changes.

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | Every validation row of Part A | `tests/canonical_foundation.rs`, one table-driven test |
| P2 | A direct-mode file is written byte for byte as today; a pull-request file is written as version 2 | the same test, and the existing pinned strings unchanged |
| P3 | Remote selection keeps `integration`, `[forge]` and the version | a new case in `tests/repository_enablement.rs` |
| P4 | `repo inspect` and `repo list` report `integration` and `forge`, from the file and from the index | `tests/read_repository.rs`, the read contract and the CLI goldens |
| P5 | The migration adds the columns once and marks every registration for refresh | a migration test beside the template's |
| P6 | Recognition, default mode and every URL row of Part C | `src/forge.rs` unit tests, table-driven |
| P7 | The pull-only decisions of Part D | `src/repository/remote/sync_tests.rs`, through the existing fetch and push stand-in |
| P8 | The binding picks the setting by mode | `src/repository/mutation/bind/sync_tests.rs` |
| P9 | End to end: a pull-only fast-forward leaves the server's primary unchanged, and a local commit is refused with nothing pushed | one SSH case in `mutation_remote` |
| P10 | `primary_has_local_commits` is registered everywhere Baseline Fact 12 lists | the existing registration tests |
| P11 | The desktop classifies a pull-only result as updated or current | `src/desktop/model/publication_tests.rs` |

## Testing

As AGENTS.md's Testing Strategy requires, each test sits at the cheapest layer that catches its regression. P9 is the only SSH case: no non-SSH publication remote can be selected (Baseline Fact 8), and the property it pins is that nothing reaches the server, which only a real fetch and push can show. Its edge cases are in P7. No real waits.

## Named Edits To Pre-Existing Tests

- **E-1** (Task 1) `src/repository/discovery.rs`, the two tests that write `format_version = 2` as an invalid configuration: change it to `format_version = 3`.
- **E-2** (Task 1) `tests/canonical_foundation.rs`, the unsupported-version case: `2` to `3`, and its expected message to "unsupported format_version; expected 1 or 2" if the case asserts it.
- **E-3** (Task 1) `tests/read_repository.rs`, the invalid-configuration case that writes `format_version = 2`: `2` to `3`.
- **E-4** (Task 1) `tests/repository_enablement.rs`, the two cases that write `format_version = 2`: `2` to `3`.
- **E-5** (Task 3) the 12 `ConfigurationDto { … }` literals: add `integration` and `forge`.
- **E-6** (Task 3) `tests/fixtures/read_v1/repo_inspect.json`, `repo_list.json`, `tests/fixtures/cli_v1/cli_repo_inspect.json`, `cli_repo_list.json`: add the two fields, regenerated with `MANYHANDS_UPDATE_GOLDEN=1` and read before committing.
- **E-7** (Task 3) `tests/repository_enablement.rs` `registry_creates_repository_and_discovery_metadata_schema`, only if it pins columns: add the four columns.
- **E-8** (Task 4) `src/results_tests.rs`: the design table gains `primary_has_local_commits` and the count becomes 71.
- **E-9** (Task 4) `tests/mutation_contract.rs` `CODES_NO_MUTATION_GOLDEN_HAS`, if the code has no mutation golden: add it with its reason.

Task 0 greps for every other assertion on the behaviour this Cycle changes (the version message, the configuration object's keys, `MERGING` in `repo sync`, `SyncOutcome` matches) and adds what it finds here before Task 1. Any red test outside this list stops work for the owner.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `repo sync` row (pull-only in a pull-request repository), the configuration object's two fields, `primary_has_local_commits` in the recovery tables, and its data shape.
- **D-2** `docs/RFC/canonical-content-and-comment-schema.md`: `format_version = 2`, `integration` and `[forge]`. That RFC requires "an explicit compatibility and migration decision" for a later version; the Pull-Request Mode RFC's decision 21 is that decision, cited here.
- **D-3** `docs/RFC/repository-index-persistence-and-refresh.md`: the configuration rows' new columns.
- **D-4** `docs/research/wave-03-api-audit.md`: `primary_has_local_commits` in `repo sync`'s codes.
- **D-5** The shipped skills, in `skills/` and in `.agents/skills/` where the copies are the same file: `manyhands-repositories` (the two keys and version 2), `manyhands-sharing-and-sync` (pull-only `repo sync`) and `manyhands-recovery` (`primary_has_local_commits`).

## Rulings

Each: decision — why — cost if wrong.

- **R-1** `integration` and `forge` stay in the table behind typed accessors, not new struct fields — no struct literal changes, including in four SSH-harness test files, and validation is in one place — a later Cycle that wants fields refactors 16 sites.
- **R-2** A separate remote-URL parser for links, not a change to `transport/endpoint.rs` — the transport parser drops the path and is crate-private, and a change there forces every SSH target (about 95 minutes) — two parsers can disagree on an odd URL, which costs a missing or wrong link, never a connection or trust decision.
- **R-3** `percent-encoding` 2.3 as a direct dependency, with `/` left unencoded — it is already compiled through `git2`, and forges accept slashes in branch names in these URLs — a forge that needs `%2F` gets a broken link until the set changes.
- **R-4** A version 2 direct-mode file is accepted and written back as version 1 — the RFC keeps direct mode at version 1, and a hand edit to version 2 should not be fatal — a reader who expects the version to be sticky sees it change.
- **R-5** The migration marks every registration `refresh_required` rather than back-filling the columns — no build ever wrote these keys, so a refresh is the cheapest way to make the rows true — one extra refresh per repository after the upgrade.
- **R-6** A pull-only fast-forward answers `ok` with publication `not_requested`, not today's `already_applied` and `current` — the owner's decision 4 needs the answer to say the primary moved — a caller that treats only `already_applied` as "nothing to do" sees `ok`.
- **R-7** The refusal names at most 20 commits, newest first, with `complete` — enough to act on, and bounded — a person with more than 20 local commits sees a partial list.
- **R-8** No Data Center URL for a personal repository or an unexpected path — their web paths differ (`/users/<name>/repos/…`) and are not in the RFC — such a repository gets the branch names without a link.
- **R-9** The SSH case goes in `mutation_remote` — it already pins `repo sync`'s conformance, and AGENTS.md selects it for `src/repository/mutation/` anyway — none beyond its run time.
- **R-10** `primary_has_local_commits` offers `repo.inspect` — the way out (moving the commits to a branch) is a person's Git work, not a Manyhands verb — an agent gets no automatic next step.

## Known Limits

The Cycle document's K-1 to K-4.

## Unverified

- **U-1** The URL shapes are the forges' documented "create pull request" forms. The RFC asks for a check against each forge's own message after a push; with no forge available to this Cycle, the owner checks one Bitbucket link by hand at review-ready.
- **U-2** The names of `graph_plan`'s relation cases and the place `refresh_required` is read are confirmed by Task 0.

## Alternatives Rejected

- New fields on `RepositoryConfig`: R-1.
- Extending the transport's parser: R-2.
- An interim guard: the owner's decision 1.
- Reusing `merge_required` for a diverged primary: it names a merge as the way forward, which pull-request mode forbids, and it does not cover a local primary that is only ahead.

## Risks

| Risk | Mitigation |
| --- | --- |
| A hand-written version 2 file is not byte-canonical, and remote selection's staged-entry check (`canonical_configuration_blob`) refuses it | Remote selection in a pull-request repository is Cycle 02's; the Known Limit is recorded in Cycle 02's ticket at approval |
| The index snapshot and the live read disagree on the new fields | P4 reads both from one fixture |
| `mutation_remote`'s run time (about 49 minutes) | One case added, run singly between steps; the whole target once at review-ready |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| — | — | — | — | — | not yet reviewed |
