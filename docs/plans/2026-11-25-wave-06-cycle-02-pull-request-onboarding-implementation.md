---
id: 01M89CHQHEV6NF0X0CTBC1G1AB
title: 'Wave 06 Cycle 02 Implementation Plan: Onboarding And Setting Changes On A Setup Branch'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 02 Implementation Plan: Onboarding And Setting Changes On A Setup Branch

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `repo enable` and `remote select` put a repository's Manyhands setup on a published setup branch for a pull request, hold the repository setup pending until a poll sees the setup on the remote's primary, and let a pending setup be cancelled.

**Architecture:** A `repository_setups` registry table holds one pending setup per repository; a setup gate on `Command` refuses writes while it exists; a narrow setup channel in the transport fetches the remote's primary into a private ref and pushes or guard-deletes only `manyhands/setup/<ULID>`; setup operations in a new `src/repository/setup/` module serialize on a setup lock file and use the record as their journal; `poll once` hands a first-time setup to `poll_setup`, and a change completes when the primary's configuration moves.

**Tech Stack:** Rust, `git2`, `rusqlite`, `fs4`, the SSH test harness for two cases.

**Spec:** [Cycle 02 design](2026-11-25-wave-06-cycle-02-pull-request-onboarding-design.md) (revision 1), under the [Cycle document](../Cycles/wave-06-cycle-02-pull-request-onboarding.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **draft**, 2026-11-25, revision 1. Author: Claude.

## Global Constraints

**Precondition.** The owner has approved the three documents, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger, the ticket comments and the document amendments, and publishes branches. Implementers never dispatch subagents and never edit ticket, comment, ledger or planning-document files.

**Where.** Every task runs in `.manyhands/worktrees/01M84H84WQYE8J2WJWEDGJ45W4` on `manyhands/ticket/01M84H84WQYE8J2WJWEDGJ45W4`. No lane branches, no extra worktrees, no copies of `target/`. The planning documents are Manyhands managed documents on their own branches (decision 9): amend them only with `document show` and `document save` from the main checkout, then `item sync`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine. Desktop tests run with `--features desktop-test-support --bin manyhands`, on their own.

**Registry.** The registry migration is one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket and document operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`.

**Tests (AGENTS.md Testing Strategy).**

- Between steps, run only the tests the step wrote or changed, by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`. Task 0 confirms each module path.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps; run the two new cases singly. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression; the two SSH cases carry a comment saying why they need SSH (design R-15).
- The fast tier and every SSH-harness target (this Cycle changes `src/repository/transport/`) run once, in Task 14.
- No real waits.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger. A row that pins unchanged behaviour is exempt.

**Test edits.** The design's named edits E-1 to E-13, and what Task 0's sweep finds. A mechanical edit that keeps every assertion's intent is applied and logged as E-14 onwards with its reason (proposed decision 10); an edit that removes or weakens an assertion stops for the owner.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Task 5, before Task 12, before Task 14, and whenever main moves. Never rebase. If Cycle 03 lands first with its own `pull_request` schema, merge it and keep one.

**Publish.** After each task's commit, the orchestrator publishes the ticket branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M84H84WQYE8J2WJWEDGJ45W4`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Static gates** (Task 14, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Ticket checkpoints.** The orchestrator adds a comment to vh-c97se after Tasks 0, 6, 8 and 12, and at review-ready, each naming the tests that ran. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M84H84WQYE8J2WJWEDGJ45W4 --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- No setup operation can push or delete any ref but `refs/heads/manyhands/setup/<setup_id>`: the channel has no mapping for anything else. Task 5 pins it; both SSH cases check the server's primary.
- A first call and a `--prepare` are gated; a re-entry of the setup's own request is not. Task 4 pins it.
- A crash at every step of publishing, activation and cancel is finished by the same request or the next poll, with no second push. Tasks 6, 8 and 10 pin it.
- The direct paths of enable and remote selection, and every poll golden, are unchanged. Tasks 7, 8 and 9 run today's tests unchanged.
- The record holds no URL and no identity. Task 2 pins it.

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger `docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md` (ticket branch) | approval |
| 1 | `src/results.rs`, `src/results_tests.rs`, `src/repository/mutation/outcome.rs`, the two code-enum schemas, `tests/mutation_contract.rs`; E-2, E-3, E-11 | 0 |
| 2 | `src/repository/setup/mod.rs`, `record.rs`, `record_tests.rs` (new), `src/repository/discovery.rs` (`migrate_registry`), `src/repository.rs` (module); E-1 | 0 |
| 3 | `src/repository/read/dto.rs`, `admin.rs`, `mod.rs`, `schemas/v1/pull_request.schema.json`, `repository_setup.schema.json` (new), the repository summary and inspection schemas, `src/bin/manyhands-cli/human.rs`; E-5, E-6, E-7 | 2 |
| 4 | `src/repository/mutation/command.rs` (`SetupGate`), `setup_gate.rs` and its tests (new), `execute.rs` (`first_call`), `confirm.rs` (`prepare_call`) | 1, 2 |
| 5 | `src/repository/transport/operation.rs`, `remote.rs`, `callbacks.rs`, `mod.rs` (the setup scope and methods), `src/repository/mutation/bind/host.rs` | 2 |
| 6 | `src/repository/setup/lock.rs`, `commit.rs`, `publish.rs` and tests (new), `src/repository/coordination.rs` | 2, 5 |
| 7 | `schemas/input/v1/repo_enable.schema.json`, `src/bin/manyhands-cli/admin.rs`, `src/repository/mutation/bind/repo.rs`, `src/repository.rs` (`enable_with_setup`), `src/repository/mutation/dto.rs`, `schemas/v1/setup_mutation.schema.json`, `setup_preview.schema.json`, `preview_effect.schema.json`, `human_admin.rs`, the desktop confirm view, `tests/repository_enablement.rs`; E-8 (enable), E-12 | 3, 4, 6 |
| 8 | `src/repository/setup/poll.rs` and tests (new), `src/repository/mutation/bind/poll.rs`, `src/repository/remote/poll.rs` and `sync.rs` (the change's completion call), `schemas/v1/setup_poll.schema.json`, `src/bin/manyhands-cli/polling.rs` | 6, 7 |
| 9 | `schemas/input/v1/remote_select.schema.json`, `src/bin/manyhands-cli/admin.rs`, `src/repository/mutation/bind/remote.rs`, `src/repository/setup/change.rs` and tests (new); E-8 (select) | 6, 7 |
| 10 | `src/repository/setup/cancel.rs` and tests (new), `src/repository/mutation/bind/setup_cancel.rs` (new), `command.rs`, `mod.rs` (`Mutation`), `bind/mod.rs`, `src/bin/manyhands-cli/catalog.rs`, `dispatch.rs`, `schemas/v1/setup_cancel*.schema.json`, `tests/support/conformance.rs`; E-9 | 4, 6 |
| 11 | `src/desktop/services/scheduler/registration.rs`, `entity.rs`, `src/desktop/model/repository_state.rs` and their tests | 3, 8 |
| 12 | `tests/mutation_remote.rs` (two cases), `tests/support/mutation_golden.rs`, the new goldens, `tests/mutation_contract.rs`; E-4, E-10 | 7 to 10 |
| 13 | `docs/RFC/cli-contract.md`, the RFCs of D-2 and D-3, `docs/research/wave-03-api-audit.md`, skills; E-13 | 1, 7 to 10 |
| 14 | none new | all |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md` on the ticket branch.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2:** Re-check every name and path the design cites (Baseline Facts 1 to 18, Parts A to J) at the new HEAD, and the test module paths each task will filter on. Mark each "existing" or "new" in a ledger table with its current file and line. Read one existing `*_preview.schema.json` and its `PreviewDetailsDto` variant, and record the layout the two new previews copy (design U-1, U-2).
- [ ] **Step 3: Premise sweep.** Grep for every site of the design's sweep list (Named Edits, last paragraph) and add each hit as a named edit E-14 onwards, under proposed decision 10.
- [ ] **Step 4:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 5:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Result Codes

**Files:** `src/results.rs` (`result_codes!`), `src/results_tests.rs`, `src/repository/mutation/outcome.rs` (`recovery`), `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, `tests/mutation_contract.rs`. Named edits: E-2, E-3, E-11.

**Interfaces:** Produces `ResultCode::SetupPending` (`"setup_pending"`, `Blocked`, "The repository's Manyhands setup is waiting for its pull request to merge.", recovery `repo.inspect`) and `ResultCode::RemotePrimaryMissing` (`"remote_primary_missing"`, `Blocked`, "The remote has no primary branch to set up from.", no recovery action).

- [ ] **Step 1: Apply E-2**, so `results::tests` fails. Run `devenv shell -- cargo test --all-features --locked --lib results::tests`. Expected: FAIL.
- [ ] **Step 2: Implement** both codes in the same position of every ordered list, their `recovery` arms (`repo.inspect` for `setup_pending`; the no-action arm for `remote_primary_missing`), and the two schema enums (E-11).
- [ ] **Step 3: Apply E-3.** Run `results::tests`, the `tests/read_contract.rs` tests that compare the enums with `ResultCode::ALL`, and both `every_result_code_has_a_*` tests of `tests/mutation_contract.rs`, each by exact name. Expected: PASS.
- [ ] **Step 4: Commit** "feat(results): setup_pending and remote_primary_missing".

## Task 2: The Setup Record (Design Part A)

**Files:** create `src/repository/setup/mod.rs`, `record.rs`, `record_tests.rs`; modify `src/repository/discovery.rs` (`migrate_registry` and a new migration after `migrate_pull_request_mode`), `src/repository.rs` (`mod setup;`). Named edit: E-1.

**Interfaces:** Produces `pub(crate) struct SetupRecord { setup_id: ItemId, kind: SetupKind, remote_name: String, primary_branch: String, configuration: Vec<u8>, base_configuration_oid: Option<git2::Oid>, base_oid: Option<git2::Oid>, commit_oid: Option<git2::Oid>, published_oid: Option<git2::Oid>, link: Option<StoredLink>, request_id: RequestId, created_at: i64 }`, `pub enum SetupKind { Enable, Change }` (`as_str` `"enable"`, `"change"`), `pub enum SetupState { Recorded, Published }`, and `record::{read, insert, record_commit, record_published, delete}`, each taking a registry transaction.

- [ ] **Step 1: Write the failing tests** in `record_tests.rs`: `setup_records_round_trip_and_enforce_their_rules` (one table: insert and read back each kind; a second insert for the same repository refused; a change without `base_configuration_oid` refused; `commit_oid` without `base_oid` refused; `published_oid` other than `commit_oid` refused; a non-hex OID refused); `a_setup_record_survives_a_rebuild_and_goes_with_remove` (a rebuild keeps it; `repo remove`'s deletion cascades it); `a_setup_record_holds_no_url_or_identity` (every cell of a written row is free of the remote's URL and the identity's name and email). And beside Cycle 01's migration test in `discovery.rs`, `repository_setup_migration_adds_the_table_once`.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the table (design Part A's SQL with the 40-hex `CHECK`s), the migration (immediate transaction, schema-checked, marker `wave_06_repository_setup_v1`), and the store functions. Apply E-1.
- [ ] **Step 4: Run** the new tests and E-1's test by exact name, and `tests/mutation_replay.rs` `the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else` and `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly`, unchanged. Expected: PASS.
- [ ] **Step 5: Commit** "feat(setup): the setup record".

## Task 3: The Contract Shapes And The Reads (Design Part I)

**Files:** `src/repository/read/dto.rs` (`PullRequestDto`, `RepositorySetupDto`, the `setup` field of `RepositorySummaryDto` and `RepositoryInspectionDto`), `src/repository/read/admin.rs`, `src/repository/read/mod.rs` (exports), `schemas/v1/pull_request.schema.json` and `repository_setup.schema.json` (new), `schemas/v1/repository_summary.schema.json` and `repository_inspection.schema.json`, `src/bin/manyhands-cli/human.rs`. Tests: `tests/read_repository.rs`, `src/bin/manyhands-cli/human_tests.rs`. Named edits: E-5, E-6, E-7.

**Interfaces:** Produces `pub struct PullRequestDto { pub source: String, pub target: String, pub forge: Option<String>, pub url: Option<String> }` with `From<&forge::PullRequestLink>`, and `pub struct RepositorySetupDto { pub id: String, pub kind: String, pub state: String, pub branch: String, pub remote: String, pub integration: String, pub pull_request: Option<PullRequestDto> }`.

- [ ] **Step 1: Write the failing read test** `inspect_and_list_report_a_pending_setup` in `tests/read_repository.rs`: a registered repository with a record inserted through the store (recorded, then published with a link) reads `setup` with each field, from `repo inspect` and from `repo list`; a repository without a record reads `setup: null`.
- [ ] **Step 2: Write the failing human test** `repository_inspection_names_a_pending_setup` in `human_tests.rs`: `Setup: waiting for manyhands/setup/<id> to merge into main` and the link's human line; nothing when there is no setup.
- [ ] **Step 3: Run both, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Implement** the DTOs, both builders, the schemas (`setup` required, an object or null; `pull_request.forge` and `repository_setup` `kind`, `state` and `integration` as enums) and the human lines. Apply E-5 to E-7; regenerate E-6's four goldens with `MANYHANDS_UPDATE_GOLDEN=1` for the exact tests that own them, read the diff, and confirm only `"setup": null` was added.
- [ ] **Step 5: Run** both new tests and every edited test by exact name, including `published_enumerations_list_exactly_the_registered_values` and `every_published_object_is_closed_and_requires_all_its_properties`, then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 6: Commit** "feat(read): report a pending setup".

## Task 4: The Setup Gate (Design Part B)

**Files:** `src/repository/mutation/command.rs` (`SetupGate`, `setup_gate`), `src/repository/mutation/setup_gate.rs` and `setup_gate_tests.rs` (new), `src/repository/mutation/execute.rs` (`first_call`), `src/repository/mutation/confirm.rs` (`prepare_call`).

**Interfaces:** Produces `pub(crate) enum SetupGate { Carries, RefusedBeforeEnable, Refused }`, `Command::setup_gate(self) -> SetupGate`, and `setup_gate::check(service: &RepositoryService, command: Command, root: &Path) -> Result<(), Answer>`.

- [ ] **Step 1: Write the failing tests:** in `command.rs`, `every_command_has_the_setup_gate_of_the_design` (loops `Command::ALL` against design Part B's table, written out in the test); in `setup_gate_tests.rs`, `a_pending_setup_refuses_a_first_call_and_a_prepare_but_not_a_reentry` (with a first-time record: `ticket create` and `document promote --prepare` answer `setup_pending` with nothing written and no record; an accepted request re-entered is not refused; with a change record, `ticket save` carries on and `ticket close` is refused; with no record nothing is refused).
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the const fn, the check, and its two calls (before `prepare` in `first_call`, before `preview` in `prepare_call`).
- [ ] **Step 4: Run** the new tests and `tests/mutation_replay/not_enabled.rs` `a_registered_repository_with_no_configuration_keeps_the_answers_it_had`, unchanged, by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(mutation): the setup gate".

## Task 5: The Setup Channel (Design Part C)

**Files:** `src/repository/transport/mod.rs` (`SetupScope`), `operation.rs` (`prepare_ssh` and `recheck_ssh` take a scope source; `with_setup_remote`), `remote.rs` (`fetch_setup_base`, `push_setup_branch`, `delete_setup_branch`, `observe_setup_branch`), `callbacks.rs` (`guard_create`), `src/repository/mutation/bind/host.rs` (`Host::read` for a first-time setup). Tests: beside the transport's unit tests, and `bind/host.rs`'s.

**Interfaces:**
- Consumes: Task 2's `record::read`.
- Produces: `pub(crate) struct SetupScope { pub setup_id: ItemId, pub remote_name: String, pub primary_branch: String }`; `pub(crate) fn with_setup_remote<P, T>(&self, root: &Path, scope: &SetupScope, direction: SshDirection, session: &mut SessionCredentials<P>, use_remote: impl FnOnce(&mut AuthenticatedSshRemote<'_, '_>) -> Result<T, SshTransportError>) -> Result<T, SshTransportError>`; the four methods, each refusing unless the connection was made with a setup scope.

- [ ] **Step 1: Merge `origin/main`** if it moved.
- [ ] **Step 2: Write the failing tests:** `setup_mappings_name_only_the_setup_ref` (fetch maps the primary into `refs/manyhands/setups/<id>/base` and nothing else; push maps only `refs/heads/manyhands/setup/<id>`; delete only `:refs/heads/manyhands/setup/<id>`; no setup method accepts a configured scope, and no item method a setup scope); `guard_create_refuses_any_update_but_creating_the_ref` (another ref, a non-zero old OID, another new OID, two updates: each refused); `the_setup_scope_is_checked_against_the_record` (a recheck with the record deleted, or with another `setup_id`, remote or primary, answers `EndpointChanged`); in `bind/host.rs`, `host_read_takes_a_first_time_setup_s_remote` (file missing and a first-time record: the record's remote; a change record: the file's; neither: `invalid_configuration` as today).
- [ ] **Step 3: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Implement** as the design says. Today's scope source is the default, so no existing caller changes.
- [ ] **Step 5: Run** the new tests by exact name, then `devenv shell -- cargo check --all-features --locked --tests`. Expected: PASS.
- [ ] **Step 6: Commit** "feat(transport): the setup channel".

## Task 6: Setup Operations (Design Part D; Test-First With A Design Checkpoint)

**Files:** create `src/repository/setup/lock.rs`, `commit.rs`, `publish.rs` and `publish_tests.rs`; modify `src/repository/coordination.rs` (the setup lock's file name).

**Interfaces:**
- Consumes: Tasks 1, 2 and 5.
- Produces: `setup::lock::acquire(repository, root) -> Result<SetupLock, RepositoryError>` (`busy` when held); `setup::commit::build(repository, base: Oid, configuration: &[u8], message: &str, identity: &CommitIdentity) -> Result<Oid, git2::Error>`; `setup::publish::run(service, root, setup_id, channel: &mut dyn SetupChannel) -> Result<Published, SetupError>`, where `SetupChannel` is the trait over Task 5's four methods that the tests stub; `SetupError` maps to `remote_primary_missing`, `primary_has_local_commits` (with `local_only_commits`), `external_change`, `recovery_required`, `busy` and the transport's stops.

- [ ] **Step 1: Write the failing tests first**, named for their outcomes: `the_setup_commit_adds_only_the_configuration_to_the_base` (tree equals the base's plus the file at mode `100644`, one parent, the message, the identity; the root's HEAD, index and working tree unchanged); `s1_s2_s3_s4_publish_decides_on_the_fetched_base` (one table over the stubbed channel's base: absent, behind, equal, ahead, diverged, holding a configuration, holding another configuration); `s5_a_stop_before_the_push_keeps_the_record`; `s6_s7_s8_publish_finishes_across_crashes_and_never_pushes_twice` (a recorded commit and no branch; a pushed ref not yet recorded; a ref at another OID); `s9_a_second_setup_operation_is_busy`; `s10_publish_touches_only_the_setup_ref`.
- [ ] **Step 2: Run each, expect FAIL**, by exact name under `repository::setup::`. Record the red runs.
- [ ] **Step 3: Design checkpoint.** Before implementing, the implementer writes in its report where each step of design Part D's "Publishing" sits: the lock, the fetch and the three refusals, the repository lease around the commit and the branch, the order of record and branch writes, the push and the observation, the link, and how a first-time refusal ends the setup and the registration in one transaction. The orchestrator checks them against S1 to S10 and the source, and records the result in the ledger, before Step 4.
- [ ] **Step 4: Implement** as checked.
- [ ] **Step 5: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 6: Commit** "feat(setup): publish a setup branch". The orchestrator adds the library checkpoint comment.

## Task 7: Repo Enable In Pull-Request Mode (Design Part E)

**Files:** `schemas/input/v1/repo_enable.schema.json`, `src/bin/manyhands-cli/admin.rs` (split `repository_input`), `src/repository/mutation/bind/repo.rs` (`RepoEnableInput`'s fields, the pull-request path's preview, digest, run, standing, re-entry and replay), `src/repository.rs` (`enable_with_setup`, the setup-pending registration), `src/repository/setup/intent.rs` and tests (new: the intended configuration and mode), `src/repository/mutation/dto.rs` (`SetupMutationDto`, `PreviewDetailsDto::Setup`, the new `PreviewEffect` values, `from_stored`, `authority`), `schemas/v1/setup_mutation.schema.json`, `setup_preview.schema.json`, `preview_effect.schema.json`, `src/bin/manyhands-cli/human_admin.rs`, the desktop confirm view. Tests: `src/repository/setup/intent_tests.rs`, `tests/repository_enablement.rs`. Named edits: E-8 (the 8 `RepoEnableInput` literals), E-12.

**Interfaces:**
- Consumes: Tasks 3, 4 and 6; Cycle 01's `forge::default_integration`.
- Produces: `RepoEnableInput { …, publication_remote: Option<String>, integration: Option<Integration>, forge: Option<ForgeSetting> }`; `setup::intent::for_enable(primary_branch, publication_remote, integration, forge, fetch_url) -> Result<Intended, IntentError>` with `Intended { integration: Integration, configuration: RepositoryConfig }`; `RepositoryService::enable_with_setup(request: EnableRepositoryRequest, setup: SetupRequest) -> Result<SetupOutcome, RepositoryError>`.

- [ ] **Step 1: Write the failing tests:** `enable_intent_follows_decision_nine` in `intent_tests.rs` (one table: no remote; a cloud remote with and without `integration`; an unrecognised remote with and without `forge`; `pull_request` without a remote; an invalid forge kind; each gives the mode or the refusal); in `tests/repository_enablement.rs`, `a_pull_request_enable_without_a_key_leaves_a_recorded_setup` (an SSH-shaped never-contacted remote and no selected key: the prepare previews the setup effects; the confirm answers `no_selected_key` with the request accepted; `repo inspect` and `repo list` show `setup` recorded; `ticket create` answers `setup_pending`; the local primary, HEAD and working tree are unchanged; the same request after a key is selected again answers `no_selected_key` until it can connect, and registers nothing twice), `enabling_a_clone_with_other_settings_is_invalid_input`, and `a_direct_enable_with_a_remote_writes_it_in_the_initialization_commit`.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the input, the intent, the pull-request path and its preview, digest, standing, re-entry and replay, the data and preview DTOs and schemas, and the human lines. Apply E-8 and E-12.
- [ ] **Step 4: Run** the new tests and, unchanged, today's enable tests by exact name: `enable_born_main_writes_canonical_config_exclusion_and_single_file_commit`, `enabling_a_repository_enabled_elsewhere_registers_and_indexes_it`, `enable_wrong_checked_out_branch_preserves_repository_and_registry`, `repo_enable_on_a_wrong_branch_is_wrong_branch`, and `tests/cli/repository.rs` `enable_an_existing_repository`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(enable): onboarding on a setup branch".

## Task 8: Poll: Activation And A Change's Completion (Design Part G)

**Files:** create `src/repository/setup/poll.rs` and `poll_tests.rs`, `src/repository/setup/complete.rs`; modify `src/repository/mutation/bind/poll.rs` (the hand-off before its configuration check), `src/repository/remote/poll.rs` and `src/repository/remote/sync.rs` (call `complete_change` at the end of `poll once` and `repo sync`), `src/repository/mutation/dto.rs` (`SetupPollDto`), `schemas/v1/setup_poll.schema.json`, `src/bin/manyhands-cli/polling.rs`.

**Interfaces:** Produces `RepositoryService::poll_setup<P>(&self, root: &Path, session: &mut SessionCredentials<P>) -> Result<SetupPollReport, RepositoryError>` with `SetupPollReport { setup_id, outcome: SetupPollOutcome, integration: Option<Integration>, differs: bool }` and `SetupPollOutcome { NotPublished, WaitingForMerge, ConfigurationInvalid, PrimaryDiffers, WaitingForCheckout, Diverged, Activated }`; `setup::complete::complete_change(service, root) -> Result<bool, RepositoryError>`.

- [ ] **Step 1: Merge `origin/main`** if it moved.
- [ ] **Step 2: Write the failing tests:** `g1_to_g7_the_setup_poll_decides_on_the_base` (one table over local repositories and a stubbed channel); `activation_registers_the_repository_as_a_direct_enable_would` (after `Activated`: `config_blob_oid` set, the remote plan stored, the record and the local setup branch gone, a ticket can be created, `differs` false; with a merged file that differs, `differs` true and the merged mode in effect); `a_crash_after_the_fast_forward_is_finished_by_the_next_poll`; `a_change_completes_when_the_primary_s_configuration_moves` (by `poll once` and by `repo sync`; not when it has not moved); and in `src/bin/manyhands-cli/polling_tests.rs`, `setup_poll_lines` for each outcome.
- [ ] **Step 3: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Implement** as the design says. `poll_publication_remote` and `PollMutationDto` do not change.
- [ ] **Step 5: Run** the new tests by exact name, and, unchanged, `poll_retirement_stored_contract_is_closed_and_cannot_complete_a_not_reached_prefix` and `task6_postfetch_advances_primary_and_replays_without_transport`. Expected: PASS.
- [ ] **Step 6: Commit** "feat(poll): activate a setup and complete a change". The orchestrator adds the checkpoint comment.

## Task 9: Setting Changes Through Remote Select (Design Part F)

**Files:** `schemas/input/v1/remote_select.schema.json`, `src/bin/manyhands-cli/admin.rs` (`remote_select`), `src/repository/mutation/bind/remote.rs` (`RemoteSelectInput`'s fields; the setup path's run, re-entry and replay), `src/repository/setup/change.rs` and `change_tests.rs` (new). Named edit: E-8 (the 16 `RemoteSelectInput` literals).

**Interfaces:** Produces `RemoteSelectInput { root, name, integration: Option<Integration>, forge: Option<Option<ForgeSetting>> }` (outer `None`: unchanged; `Some(None)`: removed) and `RepositoryService::change_setup(request: ChangeSetupRequest) -> Result<SetupOutcome, RepositoryError>`.

- [ ] **Step 1: Write the failing tests:** `remote_select_takes_the_path_of_the_design` in `change_tests.rs` (one table: direct to direct, direct to pull request, pull request to direct, a forge change in each mode; another remote in pull-request mode is `invalid_input`; an unfinished close is `recovery_required`; a pending setup is `setup_pending`); and in `tests/repository_enablement.rs`, `turning_pull_request_mode_on_without_a_key_records_a_change` (no selected key: `no_selected_key`, the request accepted, `repo inspect` shows the change; the primary's configuration unchanged; `ticket close --prepare` answers `setup_pending`; `ticket save` carries on).
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** as the design says. Apply E-8.
- [ ] **Step 4: Run** the new tests and, unchanged, `publication_selection_commits_only_canonical_configuration_changes`, `publication_remote_selection_keeps_mode_and_forge`, `tests/cli/remotes.rs` `select_then_clear` and `tests/cli/input_schemas.rs` `remote_names_are_required_and_only_selection_is_nullable`, by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(remote): setting changes on a setup branch".

## Task 10: Repo Setup-Cancel (Design Part H)

**Files:** create `src/repository/setup/cancel.rs` and `cancel_tests.rs`, `src/repository/mutation/bind/setup_cancel.rs`; modify `src/repository/mutation/command.rs`, `src/repository/mutation/mod.rs` (`Mutation`), `src/repository/mutation/bind/mod.rs`, `src/repository/mutation/dto.rs` (`SetupCancelDto`, `PreviewDetailsDto::SetupCancel`), `src/bin/manyhands-cli/catalog.rs`, `dispatch.rs`, `schemas/v1/setup_cancel.schema.json` and `setup_cancel_preview.schema.json`, `tests/support/conformance.rs`, `tests/mutation_confirmation/conformance.rs`. Named edit: E-9.

**Interfaces:** Produces `Command::RepoSetupCancel` (`"repo setup-cancel"`), `Mutation::RepoSetupCancel { root }`, `RepositoryService::cancel_setup<P>(&self, root: &Path, session: &mut SessionCredentials<P>) -> Result<SetupCancelReport, RepositoryError>`.

- [ ] **Step 1: Write the failing tests:** `cancel_outcomes` in `cancel_tests.rs` (one table over a stubbed channel: unpublished first-time setup unregisters with no connection; published and unmoved deletes the remote ref; moved keeps it; absent goes on; already merged is `recovery_required` with nothing changed; a change keeps the registration; no record is `already_applied`); `a_cancel_stopped_by_the_network_changes_nothing_and_continues`; in `tests/repository_enablement.rs`, `cancelling_a_recorded_setup_unregisters_it` (from Task 7's recorded setup, prepare then confirm).
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the library, the binding, the CLI entry (`delivered_by` "C5", `MUTATION_NO_INPUT`, confirmed), and the registrations of E-9.
- [ ] **Step 4: Run** the new tests and every E-9 test by exact name, including `the_commands_whose_caller_must_first_be_shown_need_confirmation`, `tests/cli/inventory.rs`'s count tests, `every_mutation_sits_in_one_autonomy_column` and the help goldens. Expected: PASS.
- [ ] **Step 5: Commit** "feat(setup): repo setup-cancel".

## Task 11: The Desktop's Minimum (Design Part J)

**Files:** `src/desktop/services/scheduler/registration.rs` (`Read::SetupPending`), `entity.rs` (schedule it; call `poll_setup`), the scheduler's backend trait and its test double, `src/desktop/model/repository_state.rs` (the waiting state before `needs_fixing`). Tests: the scheduler's and the repository state's test modules.

- [ ] **Step 1: Write the failing tests:** `a_published_first_time_setup_is_polled_on_the_setup_path` (the scheduler reads `SetupPending`, schedules it, calls `poll_setup`, and refreshes after `activated`); `a_setup_pending_repository_reads_waiting_not_needs_fixing` (the row's state and sentence; no action; a missing configuration without a setup still reads "Needs fixing").
- [ ] **Step 2: Run each, expect FAIL**, by exact name with `--features desktop-test-support --bin manyhands`. Record the red runs.
- [ ] **Step 3: Implement.** Keep the scheduler's read count unchanged for repositories without a setup, or log the count edits as named edits.
- [ ] **Step 4: Run** the new tests and, unchanged, `each_state_and_its_precedence`, `menu_items_follow_the_state` and `launch_polls_due_ssh_repos`, by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(desktop): poll and show a pending setup".

## Task 12: The SSH Cases And The Goldens

**Files:** `tests/mutation_remote.rs` (two cases in its `CASES` table, after `a_pull_only_repo_sync_fast_forwards_then_refuses_local_commits`), `tests/support/mutation_golden.rs` (`CASES`), the new goldens under `tests/fixtures/mutation_v1/`, `tests/mutation_contract.rs` (`DATA_ROOTS`, the `CODES_` tables). Named edits: E-4, E-10.

- [ ] **Step 1: Merge `origin/main`** if it moved.
- [ ] **Step 2: Write case 1** `a_pull_request_enable_publishes_a_setup_branch_and_a_poll_activates_it` (design P14), with this comment above it: no non-SSH publication remote can be selected, and the property is a real push of the setup ref with the server's primary untouched. A clone that is not enabled is built from `clone_owned`, the server and the key setup (as `tests/cli_remote/lifecycle.rs` `LocalOnly::new` builds one). The case:
  1. against a server with no primary, runs a pull-request `repo enable` and asserts `remote_primary_missing` and nothing registered;
  2. pushes a primary without configuration from the peer; runs `repo enable` with a fixed request ID and asserts `ok`, the `setup` data with its `pull_request`, the server's primary unchanged and the server's setup ref at the setup commit; runs the same request again and asserts the identical envelope with no new push;
  3. runs `host observe` and asserts it reaches the server with the pending remote; runs `ticket create` and asserts `setup_pending`;
  4. writes on the server's primary one new commit whose tree is the setup commit's (a squash); runs `poll once` and asserts `activated`, the local primary at the server's tip, `repo inspect` reading `integration` `pull_request` and `setup` null, and the local setup branch gone; then `ticket create` succeeds.
- [ ] **Step 3: Write case 2** `a_setting_change_goes_through_a_setup_branch_and_cancel_deletes_it_guarded` (design P15): from the harness's enabled direct `World`, `remote select` with `integration` `pull_request` publishes a setup branch with the server's primary unchanged; `ticket close --prepare` answers `setup_pending`; the peer pushes one commit to the setup branch; `repo setup-cancel` keeps the remote branch (moved) and deletes the local one; a second change is published and cancelled, and the server's setup ref is gone; `repo inspect` reads `direct` and `setup` null.
- [ ] **Step 4: Run each red** against a stub (case 1: `poll_setup` answering `waiting_for_merge`; case 2: cancel skipping the remote delete), singly by exact name: `devenv shell -- cargo test --all-features --locked --test mutation_remote <name> -- --exact`. Record the red runs.
- [ ] **Step 5:** Write the goldens with the harness's golden writer (`repo_enable_setup`, `repo_enable_remote_primary_missing`, `ticket_create_setup_pending`, `poll_once_setup_activated`, `remote_select_setup`, `repo_setup_cancel`), register them in `CASES`, list the data schemas in `DATA_ROOTS`, and apply E-4 and E-10. Read each golden.
- [ ] **Step 6: Run** both cases again, and by exact name the `tests/mutation_contract.rs` tests `every_result_code_has_a_*`, `every_schema_is_reachable_from_a_published_root`, `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`, `every_bound_command_has_a_golden`, and `tests/read_contract.rs` `published_enumerations_list_exactly_the_registered_values`. Expected: PASS.
- [ ] **Step 7: Commit** "test(setup): onboarding and setting changes end to end". The orchestrator adds the checkpoint comment.

## Task 13: Documents And Skills (Design D-1 To D-5)

- [ ] **Step 1:** Apply D-1 to D-4. Each amendment opens with "Amended 2026-11-25 by Wave 06 Cycle 02 (vh-c97se):". In Manyhands-managed text, one line per paragraph. Apply E-13.
- [ ] **Step 2:** Apply D-5 to `skills/` and `.agents/skills/`, editing each copy where the two differ, keeping Cycle 01's sentence that pull-request mode is not usable end to end until close and promote support it.
- [ ] **Step 3: Run** `devenv shell -- cargo test --all-features --locked --test cli skills::`, `--test cli recovery_tables`, `--test cli inventory::` and `--test cli collaboration_exits`. Expected: PASS, more than zero tests each.
- [ ] **Step 4: Commit** "docs: amend the contracts and skills for onboarding on a setup branch".

## Task 14: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved, then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written. Expected: PASS. Record the time.
- [ ] **Step 4: The SSH tier:** every SSH-harness target (this Cycle changes `src/repository/transport/`), each once as `devenv shell -- cargo test --all-features --locked --test <target>`, plus `--lib repository::local_sync::` only if `src/repository/local_sync/` changed. Expected: PASS. Record the times.
- [ ] **Step 5:** Update the Cycle document's Acceptance statuses through `document save`, and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the two SSH cases, the named edits applied) and publishes the ticket branch and the documents with `item sync`.
- [ ] **Step 6: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Then promote each of the three planning documents with `document promote --prepare`, shown to the owner, and `--confirm` after the owner's go-ahead. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1 | 7 (intent), 9 (select's paths) |
| A2 (S1 to S10) | 6, 12 |
| A3 | 5 (host), 7 |
| A4 | 4, 12 (golden) |
| A5 | 3 |
| A6 | 9 |
| A7 | 8 |
| A8 | 8 |
| A9 | 10, 12 |
| A10 | 12 (case 1) |
| A11 | 12 (case 2) |
| A12 | 14 |
| A13 | 11 |
| A14 | 13 |
| R-1 | 2 |
| R-2 | 4 |
| R-3 | 5 |
| R-4, R-5, R-6, R-7 | 6 |
| R-8, R-9, R-10, R-11 | 7 |
| R-12 | 9 |
| R-13 | 10 |
| R-14 | 10 |
| R-15 | 12 |
| R-16 | 8 |
| Review Focus lines | 2, 4, 5, 6, 7, 8, 9, 10, 12 |

Checked: every design Part has a task (A: 2; B: 4; C: 5; D: 6; E: 7; F: 9; G: 8; H: 10; I: 3, 7, 8, 10; J: 11); the types named in Interfaces match across tasks (`SetupRecord`, `SetupKind`, `SetupScope`, `SetupChannel`, `SetupGate`, `PullRequestDto`, `RepositorySetupDto`, `SetupPollReport`, `SetupPollOutcome`, `enable_with_setup`, `change_setup`, `poll_setup`, `cancel_setup`, `complete_change`); no placeholder steps; no new test target, so no CI list changes.
