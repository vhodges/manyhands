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

**Architecture:** The two new keys stay in `RepositoryConfig`'s table behind typed accessors (`src/canonical.rs`); the index stores them in new columns of `configuration_observations`; a new library module `src/forge.rs` recognises forges and builds links; `repo sync` passes a pull-only `SynchronizationSettings` in a pull-request repository, each remote operation stores the mode it began with, and a pull-only operation refuses local commits after each `graph_plan` and in `push_call`, ending its operation.

**Tech Stack:** Rust, `toml` 0.8, `rusqlite`, `git2`, `percent-encoding` 2.3 (new direct dependency), the SSH test harness for one case.

**Spec:** [Cycle 01 design](2026-11-24-wave-06-cycle-01-pull-request-setting-design.md) (revision 3), under the [Cycle document](../Cycles/wave-06-cycle-01-pull-request-setting.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **draft** (revision 3).

## Global Constraints

**Precondition.** The owner has approved the three documents, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger and ticket comments and publishes the ticket branch. Implementers never dispatch subagents and never edit ticket, comment or ledger files.

**Where.** Every task runs in `.manyhands/worktrees/01M87683441PRAYQJQBQBG96YY` on `manyhands/ticket/01M87683441PRAYQJQBQBG96YY`. No lane branches, no extra worktrees, no copies of `target/`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`, except the one `devenv shell -- cargo add percent-encoding@2.3` in Task 2. One cargo command at a time on this machine. Desktop tests run with `--features desktop-test-support --bin manyhands`, on their own.

**Registry.** The registry migration is one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Module paths follow the `#[path]` modules: `results::tests`, `forge::tests`, `repository::remote::sync::tests`, `repository::mutation::bind::sync::tests`, `human_collaboration::tests`. Task 0 confirms each, and the paths of `src/repository/discovery.rs`'s and `src/bin/manyhands-cli/human_tests.rs`'s tests.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps. Stop and narrow any single test command that runs past about 15 minutes.
- The fast tier and the SSH tier of the design's Testing section run once, in Task 10.
- No real waits.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger. A row that pins unchanged behaviour is exempt (design Testing).

**Test edits.** The design's named edits E-1 to E-14, and the mechanical edits Task 0's sweep finds, under decision 9: an edit that keeps every assertion's intent is applied and logged as E-15 onwards with its reason; an edit that removes or weakens an assertion stops for the owner.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Task 6, before Task 10, and whenever main moves. Never rebase.

**Publish.** After each task's commit, the orchestrator publishes the branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M87683441PRAYQJQBQBG96YY`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Static gates** (Task 10, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Ticket checkpoints.** The orchestrator adds a comment to vh-c2dtj after Tasks 0, 2 (with the owner's link check), 6 and 8, and at review-ready, each naming the tests that ran. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M87683441PRAYQJQBQBG96YY --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- A mode or kind spelled in another case (`"Pull_Request"`, `"GitHub"`) is invalid, not silently direct. Task 1 pins it.
- A cloud remote written with a user and port (`ssh://git@bitbucket.org:22/ws/repo.git`) is still recognised. Task 2 pins it.
- Branch names with `&`, `#`, `?`, a space or non-ASCII characters are encoded so that they cannot break a query string, in templates too. Task 2 pins it.
- A `web_url` with a path prefix (`https://example.com/bitbucket/`) keeps the prefix and loses only the trailing slash. Task 2 pins it.
- After one `primary_has_local_commits`, the repository is not held: the next `item sync` is not `busy`. Task 8 pins it.

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger `docs/plans/2026-11-24-wave-06-cycle-01-pull-request-setting-execution.md` | approval |
| 1 | `src/canonical.rs`, `tests/canonical_foundation.rs`, `tests/repository_enablement.rs`, E-1 to E-4 files | 0 |
| 2 | `src/forge.rs` (new), `src/lib.rs`, `Cargo.toml`, `Cargo.lock` | 1 |
| 3 | `src/repository/discovery.rs` (`migrate_registry`), `src/repository.rs` (`persist_configuration`), `src/repository/read/admin.rs` (`stored_configuration`), E-9 | 1 |
| 4 | `src/repository/read/dto.rs`, `src/repository/read/admin.rs`, `schemas/v1/repository_configuration.schema.json`, `src/bin/manyhands-cli/human.rs`, `tests/read_repository.rs`, E-5 to E-8 | 3 |
| 5 | `src/results.rs`, `src/results_tests.rs`, `src/repository/mutation/outcome.rs`, `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, E-10 | 0 |
| 6 | `src/repository/remote/sync.rs`, `src/repository/remote/sync_tests.rs`, the reservation code, `src/repository/remote/conflict_read.rs`, the `MERGING` literal, E-11 | 3, 5 |
| 7 | `src/repository/mutation/bind/sync.rs`, `src/repository/mutation/bind/sync_tests.rs`, the `MutationDataDto` definition (`from_stored`, `authority`), `src/bin/manyhands-cli/human_collaboration.rs` and its tests | 6 |
| 8 | `tests/mutation_remote.rs`, `schemas/v1/primary_local_commits.schema.json`, `tests/mutation_contract.rs`, `tests/support/mutation_golden.rs`, the new golden, E-12, E-14 | 7 |
| 9 | `docs/RFC/cli-contract.md`, the three RFCs of D-2, D-3 and D-6, `docs/research/wave-03-api-audit.md`, skills | 5, 8 |
| 10 | none new | all |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-24-wave-06-cycle-01-pull-request-setting-execution.md`.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2: A2.** Run `git show v0.2.1:src/canonical.rs | grep -n 'unsupported format_version'` and record the line ("unsupported format_version; expected 1") in the ledger.
- [ ] **Step 3:** Re-check every name and path the design cites (Baseline Facts 1 to 16 and Parts A to D) at the new HEAD, and the test module paths of the Global Constraints. Mark each "existing" or "new" in a ledger table with its current file. Name the `recovery` arm that gives no actions, and the predicate that selects a registration's primary context.
- [ ] **Step 4: Premise sweep.** Grep for every site of the design's sweep list (Named Edits, last paragraph) and add each hit to the ledger as a named edit E-15 onwards, under decision 9.
- [ ] **Step 5:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 6:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Configuration (Design Part A)

**Files:**
- Modify: `src/canonical.rs` (the new types and accessors, `parse_repository_config`, `serialize_repository_config`)
- Test: `tests/canonical_foundation.rs`, `tests/repository_enablement.rs`
- Named edits: E-1 to E-4

**Interfaces:**
- Produces, in `manyhands::canonical`:
  - `pub enum Integration { Direct, PullRequest }` with `pub fn as_str(self) -> &'static str` (`"direct"`, `"pull_request"`);
  - `pub enum ForgeKind { BitbucketCloud, BitbucketDataCenter, Github, Gitlab }` with `as_str` (`"bitbucket-cloud"`, `"bitbucket-data-center"`, `"github"`, `"gitlab"`);
  - `pub struct ForgeSetting { pub kind: Option<ForgeKind>, pub web_url: Option<String>, pub pull_request_url: Option<String> }`;
  - `impl RepositoryConfig { pub fn integration(&self) -> Integration; pub fn forge(&self) -> Option<ForgeSetting> }`.
- The enums derive `Clone, Copy, Debug, PartialEq, Eq`; `ForgeSetting` derives `Clone, Debug, PartialEq, Eq`.

- [ ] **Step 1: Write the failing table test** `repository_config_mode_and_forge_rules` in `tests/canonical_foundation.rs`. Each row is a source and an expected result; check it with `parse_repository_config`, and for every accepted row check that `serialize_repository_config` of the parsed value parses back equal. One row per line of the design's validation table, with the design's exact messages, including at least:
  - accepted: version 1 direct; version 2 direct; version 2 with `publication_remote = "origin"` and `integration = "pull_request"`; a `[forge]` with `kind = "bitbucket-data-center"`, `web_url = "https://bitbucket.example.com"` and `extra = true`; a `[forge]` with only `pull_request_url = "https://forge.example/new?from={source}"`; a `web_url` starting `http://`;
  - refused: version 1 with `pull_request`; version 2 `pull_request` with no remote; `integration = "Pull_Request"`; `integration = 1`; version 3; `forge = "github"`; an empty `[forge]`; `[forge]` with only `web_url`; `kind = "GitHub"`; `kind = 1`; `web_url = "bitbucket.example.com"`; `web_url = "https://bitbucket example.com"`; `pull_request_url = "/new?from={source}"`; `pull_request_url` without `{source}`.
- [ ] **Step 2: Write the failing byte test** `repository_config_written_bytes`: the parse of `"format_version = 1\nprimary_branch = \"main\"\n"` serializes to exactly that string; the pull-request row serializes with `format_version = 2`; a version 2 direct source serializes with `format_version = 1`; `extra = true` inside `[forge]` survives a round trip.
- [ ] **Step 3: Run both, expect FAIL:** `devenv shell -- cargo test --all-features --locked --test canonical_foundation repository_config_`. Record the red run.
- [ ] **Step 4: Implement** the types, the accessors, and `fn validate_mode_and_forge(format_version: i64, publication_remote: Option<&str>, table: &toml::Table) -> Result<(), ValidationProblem>`, called by `parse_repository_config` after its existing field checks and by `serialize_repository_config` before writing. `parse_repository_config` accepts integer 1 or 2; `serialize_repository_config` writes `2` when `integration()` is `PullRequest`, else `1`.
- [ ] **Step 5: Apply E-1 to E-4.**
- [ ] **Step 6: Write the failing test** `publication_remote_selection_keeps_mode_and_forge` in `tests/repository_enablement.rs`. Enable a repository, then commit `serialize_repository_config`'s output for a version 2 pull-request configuration with `publication_remote = "origin"` and a `[forge]` holding `kind = "bitbucket-cloud"` and an unknown key (K-5: the blob must be canonical). Select a second eligible SSH remote with that file's remote-selection helper. Assert that the committed configuration parses to `PullRequest`, the same `ForgeSetting`, the unknown key, and version 2 in the bytes. Then select no remote, and assert `RepositoryErrorKind::InvalidConfiguration` with the configuration unchanged. Run it red against a stub serializer that writes version 1, then green.
- [ ] **Step 7: Run** the three new tests and the edited tests of E-1 to E-4 by exact name. Expected: PASS.
- [ ] **Step 8: Commit** "feat(config): integration mode, forge setting and format version 2".

## Task 2: The Forge Module (Design Part C)

**Files:**
- Create: `src/forge.rs`, with `#[cfg(test)] mod tests`
- Modify: `src/lib.rs` (`pub mod forge;`), `Cargo.toml` and `Cargo.lock` through `devenv shell -- cargo add percent-encoding@2.3`

**Interfaces:**
- Consumes: `canonical::{Integration, ForgeKind, ForgeSetting}` (Task 1).
- Produces, in `manyhands::forge`:
  - `pub struct RemoteLocation { pub host: String, pub port: Option<u16>, pub path: String }`, `pub fn parse(url: &str) -> Option<RemoteLocation>`;
  - `pub struct Forge { pub kind: Option<ForgeKind>, pub location: Option<RemoteLocation>, pub web_url: Option<String>, pub template: Option<String> }`;
  - `pub fn recognise(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Option<Forge>`;
  - `pub fn default_integration(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Integration`;
  - `pub struct PullRequestLink { pub source: String, pub target: String, pub forge: Option<ForgeKind>, pub url: Option<String> }`, `pub fn human_line(&self) -> String`;
  - `pub fn pull_request_link(forge: Option<&Forge>, source: &str, target: &str) -> PullRequestLink`.

- [ ] **Step 1: Write the failing table tests** in `src/forge.rs`:
  - `remote_locations`: `git@bitbucket.org:ws/repo.git` gives host `bitbucket.org`, path `ws/repo`; `ssh://git@bitbucket.org:22/ws/repo.git` gives port 22 and the same path; `ssh://git@Bitbucket.Example.com:7999/key/repo.git` gives host `bitbucket.example.com`, port 7999, path `key/repo`; `git@github.com:/owner/repo/` gives `owner/repo`; `https://github.com/o/r.git`, `/srv/repo.git` and `file:///srv/repo.git` give `None`.
  - `recognition_and_default_mode`: the three cloud hosts in any case give their kinds and `PullRequest`; `git@example.com:a/b.git` gives `None` and `Direct`; the same remote with a `ForgeSetting` of kind `BitbucketDataCenter` gives that kind and `PullRequest`; a template-only setting gives a forge with no kind and `PullRequest`; no remote URL gives `Direct` whatever the setting.
  - `pull_request_urls`, with source `manyhands/ticket/01M87683441PRAYQJQBQBG96YY` and target `main` unless stated:
    - Cloud `git@bitbucket.org:ws/repo.git`: `https://bitbucket.org/ws/repo/pull-requests/new?source=manyhands/ticket/01M87683441PRAYQJQBQBG96YY&dest=main`;
    - Data Center `ssh://git@bitbucket.example.com:7999/key/repo.git` with `web_url = "https://bitbucket.example.com/bitbucket/"`: `https://bitbucket.example.com/bitbucket/projects/KEY/repos/repo/pull-requests?create&sourceBranch=refs/heads/manyhands/ticket/01M87683441PRAYQJQBQBG96YY&targetBranch=refs/heads/main`;
    - Data Center with no `web_url`, and with path `~me/repo`: no URL;
    - GitHub `git@github.com:o/r.git`: `https://github.com/o/r/compare/main...manyhands/ticket/01M87683441PRAYQJQBQBG96YY?expand=1`;
    - GitHub kind configured with `web_url = "https://ghe.example.com"` on remote `git@ghe.example.com:o/r.git`: `https://ghe.example.com/o/r/compare/main...manyhands/ticket/01M87683441PRAYQJQBQBG96YY?expand=1`; the same without `web_url`: `https://github.com/o/r/compare/…`;
    - GitLab `git@gitlab.com:group/sub/r.git`: `https://gitlab.com/group/sub/r/-/merge_requests/new?merge_request[source_branch]=manyhands/ticket/01M87683441PRAYQJQBQBG96YY&merge_request[target_branch]=main`;
    - template `https://forge.example/new?from={source}&to={target}` with kind `github`: `https://forge.example/new?from=manyhands/ticket/01M87683441PRAYQJQBQBG96YY&to=main`, and the link's `forge` is `Github`;
    - GitHub with a three-segment path, and no forge at all: no URL, with source and target kept;
    - encoding: source `a&b #c?d é` to Cloud gives `source=a%26b%20%23c%3Fd%20%C3%A9`, and the same in a template.
  - `human_lines`: with a URL, `pull request: <url>`; without, `open a pull request from manyhands/ticket/01M87683441PRAYQJQBQBG96YY into main`.
- [ ] **Step 2: Run, expect FAIL:** `devenv shell -- cargo test --all-features --locked --lib forge::tests`. Record the red run.
- [ ] **Step 3: Add the dependency** with `devenv shell -- cargo add percent-encoding@2.3`, then **implement** `src/forge.rs`: encode with `percent_encoding::utf8_percent_encode` and an `AsciiSet` that encodes every byte except `A-Z a-z 0-9 - . _ ~ /`. The parser is written here, does not call `src/repository/transport/`, and its doc comment says it is for links only.
- [ ] **Step 4: Run** the same command. Expected: PASS, more than zero tests. Then `devenv shell -- scripts/check-cli-no-gpui.sh`. Expected: clean.
- [ ] **Step 5: Commit** "feat(forge): forge recognition, default mode and pull request links", with `Cargo.toml` and `Cargo.lock`.
- [ ] **Step 6: The owner's link check (decision 7, A7).** The orchestrator builds a Bitbucket Cloud link for one of the owner's work repositories (its SSH remote URL, a branch name and its primary, all from the owner) by calling `pull_request_link` in a scratch unit test that prints it and is not committed, and gives the link to the owner. If the page is not the create-pull-request page, fix the shape and its row before Task 3. Record the answer in the ledger and the checkpoint comment.

## Task 3: The Index Columns (Design Part B, Storage)

**Files:**
- Modify: `src/repository/discovery.rs` (`migrate_registry` and a new migration function, which also adds `remote_operation_records.pull_only` for Task 6), `src/repository.rs` (`persist_configuration`), `src/repository/read/admin.rs` (`stored_configuration`)
- Test: the `src/repository/discovery.rs` tests
- Named edits: E-9

**Interfaces:** Consumes Task 1's accessors. Produces the four columns and `stored_configuration`'s extra values, which Task 4 reads.

- [ ] **Step 1: Write the failing test** `pull_request_mode_migration_adds_columns_once_and_refills`, beside `a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty` and with its helpers: build a registry without the columns or the marker, holding one refreshed repository whose configuration is a pull-request one and one finished remote operation row; migrate; assert the four configuration columns and `remote_operation_records.pull_only` exist, the old operation row reads `pull_only = 0`, the marker `wave_06_pull_request_mode_v1` is recorded once, and the primary context is dirty; refresh; assert the row's `integration` is `"pull_request"` and its forge columns hold the configured values; migrate again; assert no error and one marker.
- [ ] **Step 2: Run, expect FAIL**, by exact name. Record the red run.
- [ ] **Step 3: Implement** the migration (after `context_retirement::record::migrate`, in an immediate transaction that re-checks first; design Part B and R-5), the four columns in `persist_configuration`, and `stored_configuration`'s reading of them, with a null `integration` on a valid row reading as direct. Apply E-9.
- [ ] **Step 4: Run** the new test and E-9's tests by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(index): store the repository's mode and forge".

## Task 4: The Configuration Object (Design Part B, Reads)

**Files:**
- Modify: `src/repository/read/dto.rs` (`ConfigurationDto`, new `ForgeDto`), `src/repository/read/admin.rs` (the inspect and list builders), `schemas/v1/repository_configuration.schema.json`, `src/bin/manyhands-cli/human.rs` (`repository_inspection`)
- Test: `tests/read_repository.rs`, `src/bin/manyhands-cli/human_tests.rs`
- Named edits: E-5 to E-8

**Interfaces:** Produces `ConfigurationDto { …, pub integration: Option<String>, pub forge: Option<ForgeDto> }` and `pub struct ForgeDto { pub kind: Option<String>, pub web_url: Option<String>, pub pull_request_url: Option<String> }`.

- [ ] **Step 1: Write the failing read test** `inspect_and_list_report_mode_and_forge` in `tests/read_repository.rs`: a refreshed repository with a pull-request configuration and `[forge] kind = "bitbucket-cloud"`; `repo inspect` and `repo list` both give `integration` `"pull_request"` and `forge` `{ "kind": "bitbucket-cloud", "web_url": null, "pull_request_url": null }`; a direct repository gives `"direct"` and null; an invalid configuration gives null and null; a valid row with null columns (inserted as the file's existing row inserts do) lists as `"direct"`.
- [ ] **Step 2: Write the failing human test** `repository_inspection_names_mode_and_forge` in `human_tests.rs`: a pull-request configuration with a Cloud forge prints `Integration: pull_request` and `Forge: bitbucket-cloud`; a direct one prints `Integration: direct` and no `Forge:` line; a template-only forge prints `Forge: <pull_request_url>`.
- [ ] **Step 3: Run both, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Implement** the DTO fields, both builders, the schema (both keys required; `integration` an enum or null; `forge` an inline object with `additionalProperties: false` and a `kind` enum, or null) and the human lines. Apply E-5 to E-8. Regenerate the four goldens of E-6 with `MANYHANDS_UPDATE_GOLDEN=1` for the exact tests that own them, read the diff, and confirm that only the two new keys changed.
- [ ] **Step 5: Run** both new tests and every edited test by exact name, including `tests/read_contract.rs` `published_enumerations_list_exactly_the_registered_values` and the tests that own the E-6 goldens. Then `devenv shell -- cargo check --locked --features desktop-test-support --tests` for E-5's desktop literals. Expected: PASS.
- [ ] **Step 6: Commit** "feat(read): report the repository's mode and forge".

## Task 5: The Result Code (Design Part D, The Code)

**Files:** `src/results.rs` (`result_codes!`), `src/results_tests.rs`, `src/repository/mutation/outcome.rs` (`recovery`), `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, `tests/mutation_contract.rs`. Named edits: E-10, E-13.

**Interfaces:** Produces `ResultCode::PrimaryHasLocalCommits`: `"primary_has_local_commits"`, `FailureClass::Blocked`, message "The local primary branch has commits the remote does not have.", and no recovery action.

- [ ] **Step 1: Apply E-10**, so `results::tests` fails. Run `devenv shell -- cargo test --all-features --locked --lib results::tests`. Expected: FAIL.
- [ ] **Step 2: Implement** the code, its `recovery` arm (the no-action arm Task 0 named; design R-10), and the two schema enum lists.
- [ ] **Step 3: Apply E-13** (the code in both `CODES_` tables of `tests/mutation_contract.rs`, with the reason "golden written in Task 8"). Run `results::tests` again, the `tests/read_contract.rs` test that compares the enums with `ResultCode::ALL`, and both `every_result_code_has_a_*` tests of `tests/mutation_contract.rs`, each by exact name. Expected: PASS.
- [ ] **Step 4: Commit** "feat(results): primary_has_local_commits".

## Task 6: The Pull-Only Decision (Design Part D; Test-First With A Design Checkpoint)

**Files:** `src/repository/remote/sync.rs` (`SynchronizationSettings`, `SynchronizationError`, `local_only_commits`, the decision after each `graph_plan`, `push_call`, `ended_if_unresolved`), the reservation code in `src/repository/remote/` that inserts and takes up an operation row (writes and reads `pull_only`), `src/repository/remote/conflict_read.rs` (its exhaustive match on `SynchronizationError`), `src/repository/mutation/bind/sync.rs` (the `MERGING` literal gains `pull_only: false`), `src/repository/mutation/outcome.rs` (`synchronization_error_code`). Tests: `src/repository/remote/sync_tests.rs`, the reservation tests beside the code that inserts rows. Named edit: E-11.

**Interfaces:**
- Consumes: Task 5's code; Task 3's `pull_only` column.
- Produces: `SynchronizationSettings { refuse_divergence: bool, pull_only: bool }`; `SynchronizationError::PrimaryHasLocalCommits { commits: Vec<(git2::Oid, String)>, complete: bool }`; `fn local_only_commits(repository: &git2::Repository, local: git2::Oid, remote: git2::Oid) -> Result<(Vec<(git2::Oid, String)>, bool), git2::Error>`.

- [ ] **Step 1:** Merge `origin/main` if it moved.
- [ ] **Step 2: Write the failing tests first**, at the layers the design's Required Outcomes give, each named for its outcome:
  - `local_only_commits_lists_newest_first_and_caps_at_twenty` (beside the `graph_plan` test, with its `fixture()` and `child()` helpers): equal gives an empty list and `complete`; one local commit gives it with its summary; a divergence gives only the local side; 21 local commits give the 20 newest, newest first, and `complete` false.
  - `o3_o4_pull_only_refuses_local_commits_after_any_plan`: for each `graph_plan` result (equal, remote ahead, local ahead, diverged), the decision function gives continue, continue, `PrimaryHasLocalCommits`, `PrimaryHasLocalCommits`; for a context target it continues whatever the relation.
  - `o7_push_call_refuses_in_a_pull_only_operation`: `push_call`'s decision, given a pull-only operation and a candidate the remote lacks, gives `PrimaryHasLocalCommits` and no push.
  - `o8_o9_an_operation_keeps_the_mode_it_began_with`: a new operation reserved with `PULL_ONLY` stores `pull_only = 1` and one reserved with `MERGING` stores 0; taking up either row with the other setting gives the stored one.
- [ ] **Step 3: Run each, expect FAIL**, by exact name under `repository::remote::sync::tests::` or the reservation tests' path. Record the red runs.
- [ ] **Step 4: Design checkpoint.** Before implementing, the implementer writes in its report where each hook sits: the three `graph_plan` call sites and the decision after each, the line in `push_call`, where a new row is inserted and where a taken-up row's settings are read, and how `ended_if_unresolved` is widened with `keep` false. The orchestrator checks them against the Required Outcomes table and the source, and records the result in the ledger, before Step 5.
- [ ] **Step 5: Implement** as checked. Apply E-11 and the `MERGING` literal.
- [ ] **Step 6: Run** the tests of Step 2 by exact name. Expected: PASS.
- [ ] **Step 7: Commit** "feat(sync): pull-only primary synchronization". The orchestrator adds the library checkpoint comment.

## Task 7: The Binding, The Answer And The Human Output

**Files:** `src/repository/mutation/bind/sync.rs` (`PULL_ONLY`, `domain_call`, `stopped`, the refusal's effects and data), the `MutationDataDto` definition and its `from_stored` and `authority`, `src/bin/manyhands-cli/human_collaboration.rs`. Tests: `src/repository/mutation/bind/sync_tests.rs`, the replay tests beside `MutationDataDto::from_stored`'s, `human_collaboration::tests`.

**Interfaces:**
- Consumes: Task 6's setting and error; Task 1's `integration()`.
- Produces: `pub(super) const PULL_ONLY: SynchronizationSettings = SynchronizationSettings { refuse_divergence: false, pull_only: true };`; `MutationDataDto::PrimaryLocalCommits(PrimaryLocalCommitsDto)`, serialized with `"type": "primary_local_commits"`, with `pub struct PrimaryLocalCommitsDto { pub local_commits: Vec<LocalCommitDto>, pub complete: bool }` and `pub struct LocalCommitDto { pub oid: String, pub summary: String }`.

- [ ] **Step 1: Write the failing tests:**
  - `o10_domain_call_passes_the_setting_by_mode` in `bind/sync_tests.rs`: a primary call in a pull-request repository passes `PULL_ONLY`; in a direct repository `MERGING`; an `item sync` in a pull-request repository `MERGING`.
  - `local_commits_refusal_is_answered_as_itself`: beside `unresolved_history_is_answered_with_the_repair_and_no_retry`, the error over a journal row ended as cancelled gives `primary_has_local_commits`, class `Blocked`, no recovery, every effect `not_requested`, and `data` with `type`, `local_commits` (full OIDs and summaries) and `complete`; not `cancelled`.
  - `o5_local_commits_data_reads_back`: the stored data of that answer, through `MutationDataDto::from_stored` for `RepoSync` and for `OperationResume`, gives the same DTO.
  - `local_commits_refusal_lists_each_commit` in `human_collaboration::tests`: `error:`, `code: primary_has_local_commits`, one `local commit: <7-character oid> <summary>` line per commit, a control character in a summary escaped as the output's field escaping does, and `note: more local commits are not listed` when `complete` is false.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** `PULL_ONLY`, the choice in `domain_call` from `read_configuration_for`, the widening of `stopped`, the effects, the tagged data with its `from_stored` arm and its `authority` arm, and the human lines.
- [ ] **Step 4: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(sync): repo sync is pull-only in a pull-request repository".

## Task 8: The SSH Case, The Schema And The Golden

**Files:** `tests/mutation_remote.rs` (the case and its row in the case table, beside `c4_conformance_repo_sync`), `schemas/v1/primary_local_commits.schema.json`, `tests/mutation_contract.rs` (`DATA_ROOTS`, the two `CODES_` tables), `tests/support/mutation_golden.rs` (`CASES`), the golden `tests/fixtures/mutation_v1/repo_sync_primary_has_local_commits.json`. Named edits: E-12, E-14.

- [ ] **Step 1: Write the case** `a_pull_only_repo_sync_fast_forwards_then_refuses_local_commits`, with this comment above it: no non-SSH publication remote can be selected, and the property is that nothing reaches the server and nothing is held. The case:
  1. commits a pull-request configuration (the serializer's output) on `w.peer` and pushes it with the harness's peer push, then runs a direct-mode `repo sync` in the clone so that its primary holds the configuration (K-2);
  2. runs `repo sync` and asserts `already_applied` and publication `current` (O1); advances the server's primary by one commit from the peer, runs `repo sync`, and asserts `already_applied`, `current`, the local primary at the server's tip, and the server's primary unchanged by the clone (O2);
  3. commits locally, runs `repo sync` with a fixed request ID, and asserts `primary_has_local_commits`, every effect `not_requested`, `data.local_commits` holding that commit, no recovery, and the server's primary unchanged (O3);
  4. runs the same request again and asserts the identical envelope (O5), as `c4_merging_clean_divergence` checks a retry;
  5. runs `item sync` of an item and asserts it is not `busy` (decision 5);
  6. writes the golden with the harness's golden writer, as `repo_sync_push_rejected` is written.
- [ ] **Step 2: Run it red** against a stub `domain_call` that passes `MERGING` (step 3 then pushes): `devenv shell -- cargo test --all-features --locked --test mutation_remote a_pull_only_repo_sync_fast_forwards_then_refuses_local_commits -- --exact`. Record the red run.
- [ ] **Step 3: Add the schema**, list it in `DATA_ROOTS` (E-12), register the golden in `CASES`, generate it with `MANYHANDS_UPDATE_GOLDEN=1` on the same command, and read it. Apply E-14.
- [ ] **Step 4: Run** the case again without the update variable, then by exact name both `every_result_code_has_a_*` tests, `every_schema_is_reachable_from_a_published_root` and the data-root golden test of `tests/mutation_contract.rs`. Expected: PASS.
- [ ] **Step 5: Commit** "test(sync): pull-only repo sync end to end". The orchestrator adds the checkpoint comment.

## Task 9: Documents And Skills (Design D-1 To D-6)

- [ ] **Step 1:** Apply D-1 to D-4 and D-6. Each amendment opens with "Amended 2026-11-24 by Wave 06 Cycle 01 (vh-c2dtj):". In Manyhands-managed text, one line per paragraph.
- [ ] **Step 2:** Apply D-5 to `skills/` and `.agents/skills/`, editing each copy where the two differ, with decision 10's sentence.
- [ ] **Step 3: Run** `devenv shell -- cargo test --all-features --locked --test cli skills::` and `devenv shell -- cargo test --all-features --locked --test cli recovery_tables`. Expected: PASS, more than zero tests each.
- [ ] **Step 4: Commit** "docs: amend the contracts and skills for pull-request mode's setting".

## Task 10: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved, then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written. Expected: PASS. Record the time.
- [ ] **Step 4: The SSH tier** of the design's Testing section, each target once as `devenv shell -- cargo test --all-features --locked --test <target>`. Expected: PASS. Record the times. If `git diff --stat origin/main...HEAD` shows a change under `src/repository/transport/`, `src/repository/keys/` or `tests/support/ssh_*`, run every SSH-harness target instead.
- [ ] **Step 5:** Update the Cycle document's Acceptance statuses and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the SSH case, the named edits applied, the owner's link check) and publishes the branch.
- [ ] **Step 6: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1 | 1 |
| A2 | 0 |
| A3 | 1 (Step 6) |
| A4 | 4 |
| A5 | 3 |
| A6 | 2 |
| A7 | 2 (Step 6) |
| A8 (Required Outcomes O1 to O12) | 6, 7, 8 |
| A9 | 8 |
| A10 | 5, 8 |
| A11 | 10 |
| A12 | 9 |
| R-1 | 1 |
| R-2, R-3, R-8 | 2 |
| R-5 | 3 |
| R-7 | 6 |
| R-9 | 8 |
| R-10 | 5 |
| R-11, R-15 | 6 |
| R-14 | 7 |
| R-16 | Cycle document K-7 |
| R-12 | 10 |
| R-13 | 9 |
| Review Focus lines | 1, 2, 8 |

Checked: every design Part has a task (Part E has none, by decision 4); the types named in Interfaces match across tasks (`Integration`, `ForgeKind`, `ForgeSetting`, `Forge`, `PullRequestLink`, `PULL_ONLY`, `PrimaryHasLocalCommits`, `local_only_commits`, `PrimaryLocalCommitsDto`, `pull_only`); no placeholder steps; the plan adds no test target, so no CI list changes.
