---
title: "Wave 06 Cycle 01 Implementation Plan: Pull-Request Setting, Link Builder And Pull-Only Repo Sync"
date: 2026-11-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M878QXH8MFXHYK3RPMNMA4C8"
---

# Wave 06 Cycle 01 Implementation Plan: Pull-Request Setting, Link Builder And Pull-Only Repo Sync

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the repository's mode and forge a validated, visible setting, add forge recognition and the pull request link builder, and make `repo sync` pull-only in a pull-request repository.

**Architecture:** The two new keys stay in `RepositoryConfig`'s table behind typed accessors (`src/canonical.rs`); the index stores them in new columns of `configuration_observations`; a new library module `src/forge.rs` recognises forges and builds links; `repo sync` passes a pull-only `SynchronizationSettings` in a pull-request repository, and the synchronization refuses local commits before anything moves.

**Tech Stack:** Rust, `toml` 0.8, `rusqlite`, `git2`, `percent-encoding` 2.3 (new direct dependency), the SSH test harness for one case.

**Spec:** [Cycle 01 design](2026-11-24-wave-06-cycle-01-pull-request-setting-design.md), under the [Cycle document](../Cycles/wave-06-cycle-01-pull-request-setting.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

## Global Constraints

**Precondition.** The owner has approved the three documents, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M87683441PRAYQJQBQBG96YY` on `manyhands/ticket/01M87683441PRAYQJQBQBG96YY`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine (memory pressure has killed whole sessions). Desktop tests run with `--features desktop-test-support --bin manyhands`, on their own.

**Registry.** The registry migration is one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).** Between steps, run only the tests the step wrote or changed, by exact name: `--lib <path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Never run a whole SSH-harness target between steps. Stop and narrow any single test command that runs past about 15 minutes. The fast tier, and the SSH tier AGENTS.md selects for the changed paths, run once, in Task 9. Each new test sits at the cheapest layer that catches its regression; the one new SSH case is design P9, with its reason in the design's Testing section. No real waits.

**Test edits.** Only the design's named edits E-1 to E-9, and any edit Task 0's sweep adds to that list. Any other red test stops work for the owner.

**Every new check fails first.** Each new test or assertion is run red once (before the code, or against a deliberate stub) and the red run is recorded in the ledger.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Task 5, before Task 9, and whenever main moves. Never rebase.

**Publish.** After each task's commit, the orchestrator publishes the branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M87683441PRAYQJQBQBG96YY`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Static gates** (Task 9, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

## Review Focus

- A mode or kind spelled in another case (`"Pull_Request"`, `"GitHub"`) is invalid, not silently direct. Task 1 and Task 2 tests pin it.
- A cloud remote written with a user and port (`ssh://git@bitbucket.org:22/ws/repo.git`) is still recognised. Task 2 pins it.
- Branch names with `&`, `#`, `?`, a space or non-ASCII characters are encoded so that they cannot break the query string. Task 2 pins it.
- A `web_url` with a path prefix (`https://example.com/bitbucket/`) keeps the prefix and loses only the trailing slash. Task 2 pins it.
- A registry migrated by an older build then opened by this one marks every registration for refresh, and a repeated open adds no column twice. Task 3 pins it.

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | ledger `docs/plans/2026-11-24-wave-06-cycle-01-pull-request-setting-execution.md` | approval |
| 1 | `src/canonical.rs`, `tests/canonical_foundation.rs`, `tests/repository_enablement.rs`, E-1 to E-4 files | 0 |
| 2 | `src/forge.rs` (new), `src/lib.rs`, `Cargo.toml`, `Cargo.lock` | 1 |
| 3 | `src/repository/discovery.rs`, `src/repository.rs`, `src/repository/read/dto.rs`, `src/repository/read/admin.rs`, `schemas/v1/repository_configuration.schema.json`, `src/bin/manyhands-cli/human.rs`, goldens, E-5 to E-7 files | 1 |
| 4 | `src/results.rs`, `src/results_tests.rs`, `src/repository/mutation/outcome.rs`, `src/repository/mutation/` data DTO, `schemas/v1/`, `tests/mutation_contract.rs` | 1 |
| 5 | `src/repository/remote/sync.rs`, `src/repository/remote/sync_tests.rs` | 4 |
| 6 | `src/repository/mutation/bind/sync.rs`, `src/repository/mutation/bind/sync_tests.rs`, `src/bin/manyhands-cli/` human output, `tests/mutation_remote/` | 5 |
| 7 | `src/desktop/model/publication.rs`, `src/desktop/model/publication_tests.rs`, `src/desktop/model/activity/copy.rs`, `src/desktop/model/operations/outcomes.rs` | 6 |
| 8 | `docs/RFC/cli-contract.md`, `docs/RFC/canonical-content-and-comment-schema.md`, `docs/RFC/repository-index-persistence-and-refresh.md`, `docs/research/wave-03-api-audit.md`, skills | 4, 6 |
| 9 | none new | all |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-24-wave-06-cycle-01-pull-request-setting-execution.md`.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2:** Re-check every name and path the design cites (Baseline Facts 1 to 14, Parts A to E) with grep at the new HEAD. Mark each "existing" or "new" in a ledger table, with its current file and approximate line. Resolve U-2: the names of `graph_plan`'s relation cases, and where `refresh_required` is read.
- [ ] **Step 3: Premise sweep.** Grep for assertions on what this Cycle changes, and add each hit to the named edits in the ledger (E-10 onwards), or stop for the owner if one cannot be kept by an edit: the text "unsupported format_version"; `format_version = 2` in tests; the keys of the configuration object (`"publication_remote"` beside `"primary_branch"` in JSON assertions and goldens); `ConfigurationDto {`; `MERGING` and `SynchronizationSettings {`; exhaustive matches on `SynchronizationOutcome`, `SyncOutcome` and `MutationDataDto`; `DESIGN_CODES`; and the table and column lists pinned by `tests/repository_enablement.rs` `registry_creates_repository_and_discovery_metadata_schema`.
- [ ] **Step 4:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 5:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment to vh-c2dtj (base SHA, ledger path, Task 0's findings).

## Task 1: The Configuration (Design Part A)

**Files:**
- Modify: `src/canonical.rs` (`RepositoryConfig` accessors, the new types, `parse_repository_config`, `serialize_repository_config`)
- Test: `tests/canonical_foundation.rs`, `tests/repository_enablement.rs`
- Named edits: E-1 to E-4

**Interfaces:**
- Produces, in `manyhands::canonical`:
  - `pub enum Integration { Direct, PullRequest }` with `pub fn as_str(self) -> &'static str` (`"direct"`, `"pull_request"`);
  - `pub enum ForgeKind { BitbucketCloud, BitbucketDataCenter, Github, Gitlab }` with `as_str` (`"bitbucket-cloud"`, `"bitbucket-data-center"`, `"github"`, `"gitlab"`);
  - `pub struct ForgeSetting { pub kind: Option<ForgeKind>, pub web_url: Option<String>, pub pull_request_url: Option<String> }`;
  - `impl RepositoryConfig { pub fn integration(&self) -> Integration; pub fn forge(&self) -> Option<ForgeSetting> }`.
- All four types derive `Clone, Copy` where possible, `Debug, PartialEq, Eq`.

- [ ] **Step 1: Write the failing table test** `repository_config_mode_and_forge_rules` in `tests/canonical_foundation.rs`. Each row is a source and an expected result; check the result with `parse_repository_config`, and for every accepted row check that `serialize_repository_config` of the parsed value parses back equal. Rows (exact values from the design):

```rust
// (source, expected)
("format_version = 1\nprimary_branch = \"main\"\n", Ok(Integration::Direct, None)),
("format_version = 2\nprimary_branch = \"main\"\n", Ok(Integration::Direct, None)),
("format_version = 2\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\n", Ok(Integration::PullRequest, None)),
("format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\nintegration = \"pull_request\"\n", Err("integration = \"pull_request\" requires format_version = 2")),
("format_version = 2\nprimary_branch = \"main\"\nintegration = \"pull_request\"\n", Err("integration = \"pull_request\" requires publication_remote")),
("format_version = 2\nprimary_branch = \"main\"\nintegration = \"Pull_Request\"\n", Err("integration must be \"direct\" or \"pull_request\"")),
("format_version = 1\nprimary_branch = \"main\"\nintegration = 1\n", Err("integration must be \"direct\" or \"pull_request\"")),
("format_version = 3\nprimary_branch = \"main\"\n", Err("unsupported format_version; expected 1 or 2")),
("format_version = 1\nprimary_branch = \"main\"\nforge = \"github\"\n", Err("forge must be a table")),
("format_version = 1\nprimary_branch = \"main\"\n[forge]\nkind = \"GitHub\"\n", Err(/* names the four kinds */)),
("format_version = 1\nprimary_branch = \"main\"\n[forge]\nkind = \"bitbucket-data-center\"\nweb_url = \"bitbucket.example.com\"\n", Err(/* web_url */)),
("format_version = 1\nprimary_branch = \"main\"\n[forge]\npull_request_url = \"https://x.example/new?to={target}\"\n", Err(/* {source} */)),
("format_version = 1\nprimary_branch = \"main\"\n[forge]\nkind = \"bitbucket-data-center\"\nweb_url = \"https://bitbucket.example.com\"\nextra = true\n", Ok(Integration::Direct, Some(kind BitbucketDataCenter, web_url, None))),
```

Assert the `ValidationCode` is `InvalidField` for every `Err` row, and the message where the row gives it.

- [ ] **Step 2: Write the failing byte test** `repository_config_written_bytes` in `tests/canonical_foundation.rs`: serializing the parse of `"format_version = 1\nprimary_branch = \"main\"\n"` gives exactly that string; the pull-request row of Step 1 serializes with `format_version = 2` as its first line; a version 2 direct-mode source serializes with `format_version = 1`; the `extra = true` key inside `[forge]` survives a round trip.
- [ ] **Step 3: Run both, expect FAIL** (unknown types, then the version 2 rows): `devenv shell -- cargo test --all-features --locked --test canonical_foundation repository_config_ -- --nocapture`. Record the red run.
- [ ] **Step 4: Implement** the types and accessors, and one validation function `fn validate_mode_and_forge(format_version: i64, publication_remote: Option<&str>, table: &toml::Table) -> Result<(), ValidationProblem>` called by both `parse_repository_config` (after the existing field checks) and `serialize_repository_config` (before writing). `parse_repository_config` accepts integer 1 or 2. `serialize_repository_config` writes `2` when `integration()` is `PullRequest`, else `1`. A URL is valid when it starts with `https://` or `http://`, has a non-empty host after the scheme, and contains no whitespace or control characters.
- [ ] **Step 5: Apply E-1 to E-4** (`format_version = 2` to `3` in the six invalid-version tests; E-2's message if asserted).
- [ ] **Step 6: Write the failing test** `publication_remote_selection_keeps_mode_and_forge` in `tests/repository_enablement.rs`: enable a repository, commit a version 2 configuration with `integration = "pull_request"`, `publication_remote = "origin"` and a `[forge]` table holding an unknown key, select a different eligible SSH remote with the existing remote-selection helper of that file, and assert that the committed `.manyhands/config.toml` still parses to `PullRequest`, the same `ForgeSetting`, version 2 and the unknown key. Run it red against a stub that drops the table, then green.
- [ ] **Step 7: Run** the tests of Steps 1, 2 and 6 and the edited tests of E-1 to E-4 by exact name; then `devenv shell -- cargo test --all-features --locked --lib canonical` and `devenv shell -- cargo test --all-features --locked --test canonical_foundation`. Expected: PASS.
- [ ] **Step 8: Commit** "feat(config): integration mode, forge setting and format version 2".

## Task 2: The Forge Module (Design Part C)

**Files:**
- Create: `src/forge.rs` (with `#[cfg(test)] mod tests`)
- Modify: `src/lib.rs` (`pub mod forge;`), `Cargo.toml` (`percent-encoding = "2.3"`), `Cargo.lock`

**Interfaces:**
- Consumes: `canonical::{Integration, ForgeKind, ForgeSetting}` from Task 1.
- Produces, in `manyhands::forge`:
  - `pub struct RemoteLocation { pub host: String, pub port: Option<u16>, pub path: String }` with `pub fn parse(url: &str) -> Option<RemoteLocation>`;
  - `pub struct Forge { pub kind: Option<ForgeKind>, pub location: Option<RemoteLocation>, pub web_url: Option<String>, pub template: Option<String> }`;
  - `pub fn recognise(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Option<Forge>`;
  - `pub fn default_integration(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Integration`;
  - `pub struct PullRequestLink { pub source: String, pub target: String, pub forge: Option<ForgeKind>, pub url: Option<String> }` with `pub fn human_line(&self) -> String`;
  - `pub fn pull_request_link(forge: Option<&Forge>, source: &str, target: &str) -> PullRequestLink`.

- [ ] **Step 1: Write the failing table tests** in `src/forge.rs`:
  - `remote_locations`: `git@bitbucket.org:ws/repo.git` gives host `bitbucket.org`, path `ws/repo`; `ssh://git@bitbucket.org:22/ws/repo.git` gives port 22 and the same path; `ssh://git@Bitbucket.Example.com:7999/key/repo.git` gives host `bitbucket.example.com`, port 7999, path `key/repo`; `git@github.com:/owner/repo/` gives `owner/repo`; `https://github.com/o/r.git`, `/srv/repo.git` and `file:///srv/repo.git` give `None`.
  - `recognition_and_default_mode`: the three cloud hosts in any case give their kinds and `PullRequest`; `git@example.com:a/b.git` gives `None` and `Direct`; the same remote with a `ForgeSetting` of kind `BitbucketDataCenter` gives that kind and `PullRequest`; a setting with only `pull_request_url` gives a forge with no kind and `PullRequest`; no remote URL gives `Direct` whatever the setting.
  - `pull_request_urls`, source `manyhands/ticket/01M87683441PRAYQJQBQBG96YY`, target `main` unless stated:
    - Cloud `git@bitbucket.org:ws/repo.git`: `https://bitbucket.org/ws/repo/pull-requests/new?source=manyhands/ticket/01M87683441PRAYQJQBQBG96YY&dest=main`;
    - Data Center `ssh://git@bitbucket.example.com:7999/key/repo.git` with `web_url = "https://bitbucket.example.com/bitbucket/"`: `https://bitbucket.example.com/bitbucket/projects/KEY/repos/repo/pull-requests?create&sourceBranch=refs/heads/manyhands/ticket/01M87683441PRAYQJQBQBG96YY&targetBranch=refs/heads/main`;
    - Data Center with no `web_url`, and with path `~me/repo`: no URL;
    - GitHub `git@github.com:o/r.git`: `https://github.com/o/r/compare/main...manyhands/ticket/01M87683441PRAYQJQBQBG96YY?expand=1`;
    - GitLab `git@gitlab.com:group/sub/r.git`: `https://gitlab.com/group/sub/r/-/merge_requests/new?merge_request[source_branch]=manyhands/ticket/01M87683441PRAYQJQBQBG96YY&merge_request[target_branch]=main`;
    - template `https://forge.example/new?from={source}&to={target}`: `https://forge.example/new?from=manyhands/ticket/01M87683441PRAYQJQBQBG96YY&to=main`;
    - GitHub with a three-segment path, and no forge at all: no URL, with source and target kept;
    - encoding: source `a&b #c?d é` to Cloud gives `source=a%26b%20%23c%3Fd%20%C3%A9`.
  - `human_lines`: with a URL, `pull request: <url>`; without, `open a pull request from manyhands/ticket/01M87683441PRAYQJQBQBG96YY into main`.
- [ ] **Step 2: Run, expect FAIL:** `devenv shell -- cargo test --all-features --locked --lib forge::tests`. Record the red run.
- [ ] **Step 3: Implement** `src/forge.rs`. Encode with `percent_encoding::utf8_percent_encode` and an `AsciiSet` that encodes every byte except `A-Z a-z 0-9 - . _ ~ /` (design R-3). The parser is written here and does not call `src/repository/transport/` (design R-2); its doc comment says it is for links only.
- [ ] **Step 4: Run** the same command. Expected: PASS. Then `devenv shell -- cargo check --all-features --locked` and `scripts/check-cli-no-gpui.sh`. Expected: clean.
- [ ] **Step 5: Commit** "feat(forge): forge recognition, default mode and pull request links" with `Cargo.toml` and `Cargo.lock`.

## Task 3: The Index And The Reads (Design Part B)

**Files:**
- Modify: `src/repository/discovery.rs` (`migrate_registry`), `src/repository.rs` (`persist_configuration`, `stored_configuration`), `src/repository/read/dto.rs` (`ConfigurationDto`, new `ForgeDto`), `src/repository/read/admin.rs` (the inspect and list builders), `schemas/v1/repository_configuration.schema.json`, `src/bin/manyhands-cli/human.rs` (`repository_inspection`)
- Test: the migration test beside `src/repository/closure/record_tests.rs`'s pattern, in `src/repository/discovery.rs` tests; `tests/read_repository.rs`; `tests/read_contract.rs`; `tests/cli/admin.rs`
- Named edits: E-5 to E-7, and any added by Task 0

**Interfaces:**
- Consumes: Task 1's accessors.
- Produces: `ConfigurationDto { …, pub integration: Option<String>, pub forge: Option<ForgeDto> }` and `pub struct ForgeDto { pub kind: Option<String>, pub web_url: Option<String>, pub pull_request_url: Option<String> }`.

- [ ] **Step 1: Write the failing migration test** `configuration_mode_columns_migrate_once` in the `discovery.rs` tests: open a registry, assert the four columns exist and the marker `wave_06_configuration_mode_v1` is recorded once; build a registry without the columns or marker (the existing older-build helper, as `a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty` does), insert a registration with `refresh_required = 0`, migrate, and assert the columns exist and `refresh_required = 1`; migrate again and assert no error and still one marker.
- [ ] **Step 2: Write the failing read test** `inspect_and_list_report_mode_and_forge` in `tests/read_repository.rs`: a repository with the Task 1 pull-request configuration and `[forge] kind = "bitbucket-cloud"`, refreshed; `repo inspect` and `repo list` both give `integration` `"pull_request"` and `forge` `{ "kind": "bitbucket-cloud", "web_url": null, "pull_request_url": null }`; a direct repository gives `"direct"` and null; an invalid configuration gives null and null.
- [ ] **Step 3: Run both, expect FAIL.** Record the red runs.
- [ ] **Step 4: Implement** the migration (after the first transaction; design R-5), `persist_configuration`'s four columns, `stored_configuration`'s synthesized `integration`, `[forge]` and version, the DTO fields and both builders, the schema (both keys required; `forge` an inline object with `additionalProperties: false` or null), and the human lines `Integration: <mode>` and, when there is a forge, `Forge: <kind, or the web_url, or the template>`.
- [ ] **Step 5: Apply E-5 to E-7.** Regenerate the four goldens with `MANYHANDS_UPDATE_GOLDEN=1` for the exact tests that own them, read the diff, and confirm that only the two new keys changed.
- [ ] **Step 6: Run** Steps 1 and 2 by exact name, the edited tests by exact name, `tests/read_contract.rs` `contract_enumerations` and the read-contract tests of `repo inspect` and `repo list`, and `devenv shell -- cargo test --all-features --locked --test cli admin::`. Expected: PASS. Desktop edits of E-5: `devenv shell -- cargo check --locked --features desktop-test-support --tests`.
- [ ] **Step 7: Commit** "feat(read): report the repository's mode and forge".

## Task 4: The Result Code (Design Part D, the answer)

**Files:**
- Modify: `src/results.rs` (`result_codes!`), `src/results_tests.rs`, `src/repository/mutation/outcome.rs` (`recovery`), the `MutationDataDto` definition in `src/repository/mutation/`, `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, new `schemas/v1/primary_local_commits.schema.json`, `tests/mutation_contract.rs`
- Named edits: E-8, E-9

**Interfaces:**
- Produces: `ResultCode::PrimaryHasLocalCommits` (`"primary_has_local_commits"`, `FailureClass::Blocked`, message "The local primary branch has commits the remote does not have."); `MutationDataDto::PrimaryLocalCommits(PrimaryLocalCommitsDto)` with `pub struct PrimaryLocalCommitsDto { pub local_commits: Vec<LocalCommitDto>, pub complete: bool }` and `pub struct LocalCommitDto { pub oid: String, pub summary: String }`.

- [ ] **Step 1: Apply E-8** (the design table row and the count 71) so `src/results_tests.rs` fails. Run `devenv shell -- cargo test --all-features --locked --lib results_tests`. Expected: FAIL.
- [ ] **Step 2: Implement** the code, its `recovery` arm (`repo.inspect`, design R-10), the data DTO and its schema, and the two enum lists. Apply E-9, unless Task 6 produces a mutation golden for the code, with the reason "produced only after a real fetch; pinned by the SSH case of Task 6".
- [ ] **Step 3: Run** `src/results_tests.rs`, `tests/read_contract.rs` `contract_enumerations`, `tests/mutation_contract.rs`'s code-coverage test by exact name, and `devenv shell -- cargo test --all-features --locked --test cli recovery_tables`. Expected: PASS; if `recovery_tables` needs the contract row, add it now with Task 8's text.
- [ ] **Step 4: Commit** "feat(results): primary_has_local_commits".

## Task 5: Pull-Only Synchronization (Design Part D, the decision)

**Files:**
- Modify: `src/repository/remote/sync.rs` (`SynchronizationSettings`, `SynchronizationError`, `SynchronizationOutcome`, `synchronize_remote_observed_inner`)
- Test: `src/repository/remote/sync_tests.rs`

**Interfaces:**
- Consumes: Task 4's code, through `synchronization_error_code`.
- Produces: `SynchronizationSettings { refuse_divergence: bool, pull_only: bool }`; `SynchronizationError::PrimaryHasLocalCommits { commits: Vec<(git2::Oid, String)>, complete: bool }`; `SynchronizationOutcome::FastForwarded { target: SynchronizationTarget, oid: git2::Oid }`.

- [ ] **Step 1:** Merge `origin/main` if it moved (Global Constraints, Main).
- [ ] **Step 2: Write the failing table test** `pull_only_primary_synchronization_decisions` in `sync_tests.rs`, through the existing fetch and push stand-in (`classified_synchronization_for_testing` and its helpers), with `SynchronizationSettings { refuse_divergence: true, pull_only: true }` on the primary target. Rows:
  - equal: `AlreadyCurrent`, no push requested;
  - remote ahead by one commit: `FastForwarded` with the remote's tip, the local primary ref at that tip, no push requested;
  - local ahead by one commit: `PrimaryHasLocalCommits` with that commit and `complete: true`, local ref unchanged, no push requested, the reservation released;
  - diverged (one local, one remote): `PrimaryHasLocalCommits` with the local commit only, nothing of a merge recorded;
  - local ahead by 21 commits: 20 listed, newest first, `complete: false`.
- [ ] **Step 3: Write the failing guard test** `pull_only_is_ignored_for_item_contexts`: the same setting on a context target behaves as `refuse_divergence` alone (the binding never passes it, but the domain must not refuse an item's own commits).
- [ ] **Step 4: Run both, expect FAIL:** `devenv shell -- cargo test --all-features --locked --lib repository::remote::sync_tests::pull_only -- --nocapture`. Record the red runs.
- [ ] **Step 5: Implement.** After `graph_plan` and before the divergence check, a primary target under `pull_only` whose local primary is ahead or diverged stops with `PrimaryHasLocalCommits`, listing commits reachable from the local primary and not from the fetched remote primary (a `git2` revwalk, newest first, limited to 21 to learn `complete`), each with its summary line. A pull-only fast-forward returns `FastForwarded` in place of `AlreadyCurrent`. Map the new error in `synchronization_error_code` to `PrimaryHasLocalCommits`. Handle `FastForwarded` wherever the compiler finds a `SynchronizationOutcome` match, with the same index and journal treatment as `AlreadyCurrent`.
- [ ] **Step 6: Run** the same command, then `devenv shell -- cargo test --all-features --locked --lib repository::remote::sync_tests`. Expected: PASS. If that module run passes 15 minutes, stop and narrow it.
- [ ] **Step 7: Commit** "feat(sync): pull-only primary synchronization". The orchestrator adds the library checkpoint comment to vh-c2dtj.

## Task 6: The Binding, The CLI And The SSH Case

**Files:**
- Modify: `src/repository/mutation/bind/sync.rs` (`PULL_ONLY`, the `repo sync` setting choice, the effects of `FastForwarded`, the refusal's data), `src/bin/manyhands-cli/` (the human `local commit:` lines, beside `human_lifecycle.rs`'s own-line pattern)
- Test: `src/repository/mutation/bind/sync_tests.rs`, `src/bin/manyhands-cli/human_collaboration_tests.rs`, a new case in `tests/mutation_remote/`

**Interfaces:**
- Consumes: Task 5's setting, error and outcome; Task 1's `integration()`; Task 4's DTO.
- Produces: `pub(super) const PULL_ONLY: SynchronizationSettings = SynchronizationSettings { refuse_divergence: true, pull_only: true };`.

- [ ] **Step 1: Write the failing binding test** `repo_sync_is_pull_only_in_a_pull_request_repository` in `bind/sync_tests.rs`: a pull-request repository's `repo sync` passes `PULL_ONLY`, a direct one `MERGING`, and an `item sync` in a pull-request repository `MERGING`. Then `fast_forwarded_answers_ok_without_publication`: `FastForwarded` gives code `ok`, `effects.publication` `not_requested`, `effects.commit_oid` the new head; and `local_commits_refusal_carries_its_commits`: the error gives `primary_has_local_commits`, exit class `Blocked`, and `data.local_commits` and `data.complete`.
- [ ] **Step 2: Write the failing human test** in `human_collaboration_tests.rs`: the refusal prints `error:`, `code: primary_has_local_commits`, one `local commit: <7-character oid> <summary>` line per commit, and `note: more local commits are not listed` when `complete` is false.
- [ ] **Step 3: Run, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Implement** the setting choice in the `repo sync` binding (it already reads the configuration for the remote in `prepare`), the effects and data mapping, and the human lines.
- [ ] **Step 5: Write the SSH case** `a_pull_only_repo_sync_fast_forwards_and_pushes_nothing` in the `tests/mutation_remote/` module that holds the `repo sync` conformance cases. Reason, as a comment above it: no non-SSH publication remote can be selected, and the property is that nothing reaches the server. The case makes a pull-request repository against the harness server, advances the server's primary by one commit from a second clone, runs `repo sync`, and asserts code `ok`, publication `not_requested`, the local primary at the server's tip and the server's primary unchanged; then commits locally, runs `repo sync` again, and asserts `primary_has_local_commits` with that commit and the server's primary still unchanged.
- [ ] **Step 6: Run** the unit tests by exact name, then the SSH case alone: `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_only_repo_sync_fast_forwards_and_pushes_nothing -- --exact`. Expected: PASS. Run it red once against a stub that keeps `MERGING`, and record it.
- [ ] **Step 7: Commit** "feat(sync): repo sync is pull-only in a pull-request repository".

## Task 7: The Desktop's Classification (Design Part E)

**Files:**
- Modify: `src/desktop/model/publication.rs` (`classify_sync`, `SyncOutcome`), `src/desktop/model/activity/copy.rs`, `src/desktop/model/operations/outcomes.rs`
- Test: `src/desktop/model/publication_tests.rs`, and the activity copy table if Task 0 finds it exhaustive over `SyncOutcome`

**Interfaces:** Consumes the answer shape of Task 6. Produces `SyncOutcome::Updated`.

- [ ] **Step 1: Write the failing tests** `sync_fast_forwarded_is_updated` (code `ok`, publication `not_requested`, a `repo sync` attempt: `Updated`) and `sync_already_applied_stays_current` in `publication_tests.rs`.
- [ ] **Step 2: Run, expect FAIL:** `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands model::publication_tests::sync_fast_forwarded_is_updated -- --exact`.
- [ ] **Step 3: Implement** `SyncOutcome::Updated` and its words "Updated <primary>" in both copy sites. No label changes.
- [ ] **Step 4: Run** both tests by exact name, then `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands model::publication_tests`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(desktop): a pull-only repo sync reads as updated".

## Task 8: Documents And Skills (Design D-1 to D-5)

- [ ] **Step 1:** Apply D-1 to D-4, one line per paragraph in any Manyhands-managed text. Each amendment opens with "Amended 2026-11-24 by Wave 06 Cycle 01 (vh-c2dtj):".
- [ ] **Step 2:** Apply D-5 to `skills/` and `.agents/skills/`. Where the two copies of a skill differ today, edit each copy separately.
- [ ] **Step 3: Run** `devenv shell -- cargo test --all-features --locked --test cli skills::` and `devenv shell -- cargo test --all-features --locked --test cli recovery_tables`. Expected: PASS.
- [ ] **Step 4: Commit** "docs: amend the contracts and skills for pull-request mode's setting".

## Task 9: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved, then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written. Expected: PASS. Record the time.
- [ ] **Step 4: The SSH tier** for the changed paths: `src/repository/remote/` selects `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`; `src/repository/mutation/` selects `mutation_remote`. Each once, as `devenv shell -- cargo test --all-features --locked --test <target>`. Expected: PASS. Record the times. If Task 0 found a change under `src/repository/transport/` or `src/repository/keys/`, run every SSH-harness target instead.
- [ ] **Step 5:** The owner opens one Bitbucket link built by the link builder for a real repository and confirms it opens the create-pull-request page (design U-1). Record the answer.
- [ ] **Step 6:** Update the Cycle document's Acceptance statuses and the ledger. The orchestrator adds the review-ready comment to vh-c2dtj (gates, tiers and times, the SSH case, the named edits applied) and publishes the branch.
- [ ] **Step 7: After review approval only:** save the ticket's status as done, run `ticket close --prepare` from the main checkout, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1 | 1 |
| A2 | 0 (baseline fact at the base SHA) |
| A3 | 3 |
| A4 | 2 |
| A5 | 5 |
| A6 | 6 |
| A7 | 4 |
| A8 | 7 |
| A9 | 9 |
| A10 | 8 |
| R-1, R-4 | 1 |
| R-2, R-3, R-8 | 2 |
| R-5 | 3 |
| R-6, R-7 | 5, 6 |
| R-9 | 6 |
| R-10 | 4 |
| Review Focus lines | 1, 2, 3 |

Checked: every design Part has a task; the types named in Interfaces match across tasks (`Integration`, `ForgeKind`, `ForgeSetting`, `Forge`, `PullRequestLink`, `PULL_ONLY`, `PrimaryHasLocalCommits`, `FastForwarded`, `SyncOutcome::Updated`); no placeholder steps. The plan adds no test target, so no CI list changes.
