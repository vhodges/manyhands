---
id: 01M89CHQHEV6NF0X0CTBC1G1AB
title: 'Wave 06 Cycle 02 Implementation Plan: Onboarding And Setting Changes On A Setup Branch'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 02 Implementation Plan: Onboarding And Setting Changes On A Setup Branch

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `repo enable` and `remote select` put a repository's Manyhands setup on a published setup branch for a pull request, hold the setup pending until the primary holds it, and let a pending setup be cancelled.

**Architecture:** A `repository_setups` registry table holds one pending setup per repository; a setup gate on `Command` refuses writes while it exists, and a complete setup is finished wherever it is seen; a setup channel in the transport (a second source for the remote's name, beside today's) fetches the setup remote's primary into a private ref and pushes or guard-deletes only `manyhands/setup/<ULID>`; setup operations in a new `src/repository/setup/` module serialize on a setup lock file and use the record as their journal; `poll once` hands a setup on another remote to `poll_setup`.

**Tech Stack:** Rust, `git2`, `rusqlite`, `fs4`, the SSH test harness for one case.

**Spec:** [Cycle 02 design](2026-11-25-wave-06-cycle-02-pull-request-onboarding-design.md) (revision 4), under the [Cycle document](../Cycles/wave-06-cycle-02-pull-request-onboarding.md) and the [Pull-Request Mode RFC](../RFC/pull-request-mode.md).

Status: **approved**, 2026-11-25, revision 4 (owner approval 2026-11-25, Cycle decisions 17 and 18). Author: Claude.

## Global Constraints

**Precondition.** The owner has approved the three documents, authorized implementation and chosen the execution method. Task 0 does not start before that.

**Roles.** The orchestrator runs the tasks, or dispatches one implementer per task and then a fresh reviewer, one at a time. It alone writes the ledger, the ticket comments and the document amendments, and publishes branches. Implementers never dispatch subagents and never edit ticket, comment, ledger or planning-document files.

**Where.** Every task runs in `.manyhands/worktrees/01M84H84WQYE8J2WJWEDGJ45W4` on `manyhands/ticket/01M84H84WQYE8J2WJWEDGJ45W4`. No lane branches, no extra worktrees, no copies of `target/`. The planning documents are Manyhands managed documents on their own branches (decision 9): amend them only with `document show` and `document save` from the main checkout, then `item sync`.

**Cargo.** Every Rust command is `devenv shell -- cargo …` with `--locked`. One cargo command at a time on this machine. Desktop tests run with `--features desktop-test-support --bin manyhands`, on their own.

**Registry.** The registry migration is one-way. Never run this branch's CLI or library against the owner's registry. Any run outside `cargo test` uses an isolated `HOME`, `MANYHANDS_DATA_DIR` and `GIT_CONFIG_GLOBAL` under a temporary root. Ticket and document operations run only from the up-to-date main checkout, as `devenv shell -- cargo run --locked --bin manyhands-cli -- …`. New library tests set a repository-local Git identity: this machine has no global one.

**Tests (AGENTS.md Testing Strategy, as decision 11 narrows it).**

- Between steps, run only the tests the step wrote or changed, by exact name: `--lib <module path>::<test> -- --exact`, `--bin manyhands-cli <module path>::<test> -- --exact`, or `--test <target> <name> -- --exact`, one test per command. Task 0 confirms each module path.
- Every test command must report more than zero tests run. A run of zero is a failed step.
- Never run a whole SSH-harness target between steps; run the SSH case singly. Stop and narrow any single test command that runs past about 15 minutes.
- Each new test sits at the cheapest layer that catches its regression; the SSH case carries a comment saying why it needs SSH (design R-15). Extra tests the plan does not list are approved as needed (decision 10).
- At review-ready only (Task 14): the fast tier, the SSH case, and the `ssh_transport` and `remote_polling` targets (decision 11). After a review fix, rerun only a target whose code or tests changed.
- No real waits.

**Every new check fails first.** Each new test or assertion is run red once, before its code or against a deliberate stub, and the red run is recorded in the ledger. A row that pins unchanged behaviour is exempt.

**Test edits** (decision 15). The design's named edits E-1 to E-15, and what Task 0's sweep finds: an edit that keeps every assertion's intent is applied and logged as E-16 onwards with its reason; an edit that removes or weakens an assertion stops for the owner.

**Main.** Merge `origin/main` into the ticket branch in Task 0, before Tasks 5, 8, 12 and 14, and whenever main moves. Never rebase. If Cycle 03 lands first with its own `pull_request` schema, merge it and keep one.

**Publish.** After each task's commit, the orchestrator publishes the ticket branch with `item sync` from the main checkout and checks that `HEAD` equals `origin/manyhands/ticket/01M84H84WQYE8J2WJWEDGJ45W4`.

**Commits.** Each ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Static gates** (Task 14, and before any commit that touches Rust):

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
```

**Goldens.** Each golden is written by the test that produces it, with `MANYHANDS_UPDATE_GOLDEN=1` on that test's exact command and its target's golden helper (Task 0 names each), then read before committing. In the same task it is registered in `tests/support/mutation_golden.rs` `CASES` (a golden is written only for a registered name), its data or preview schema is added to `tests/mutation_contract.rs` `DATA_ROOTS`, its code leaves the `CODES_` tables (E-4), and the task runs by exact name `the_fixture_directories_are_exactly_the_registered_cases`, both `every_result_code_has_a_*` tests, `every_schema_is_reachable_from_a_published_root` and `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`.

**Ticket checkpoints.** The orchestrator adds a comment to vh-c97se after Tasks 0, 6, 8 and 12, and at review-ready, each naming the tests that ran. Comments are added from the main checkout with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M84H84WQYE8J2WJWEDGJ45W4 --input <file> --non-interactive --json --request-id <ULID>`.

## Review Focus

- No setup operation can push or delete any ref but `refs/heads/manyhands/setup/<setup_id>`: the channel has no mapping for anything else. Task 5 pins it; the SSH case checks the server's primary.
- Today's `Configured` connection path is unchanged code. Task 5 keeps it, and `ssh_transport` runs at review-ready.
- A first call, an acceptance and a `--prepare` are gated; a re-entry of the setup's own request is not. Task 4 pins it.
- A setup request continues by the same request and never starts a second setup (design Part K, L1 to L14). Tasks 7, 9 and 10 pin it, test-first.
- A crash at every step of publishing, activation and cancel is finished by the same request, the next poll or completion, with no second push. Tasks 6, 8 and 10 pin it.
- The direct paths of enable and remote selection, every enable digest without the new fields, and every poll golden are unchanged. Tasks 7, 8 and 9 run today's tests unchanged.
- The record holds no remote URL, credential or identity. Task 2 pins it.

---

## File Map And Dependency Order

| Task | Files | Depends on |
| --- | --- | --- |
| 0 | the ledger `docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md` (ticket branch) | approval |
| 1 | `src/results.rs`, `src/results_tests.rs`, `src/repository/mutation/outcome.rs`, `outcome_tests.rs`, the two code-enum schemas, `tests/mutation_contract.rs`; E-2, E-3, E-11 | 0 |
| 2 | `src/repository/setup/mod.rs`, `record.rs`, `record_tests.rs` (new), `src/repository/discovery.rs` (`migrate_registry`), `src/repository.rs` (module); E-1 | 0 |
| 3 | `src/repository/read/dto.rs`, `admin.rs`, `mod.rs`, `tests.rs`, `schemas/v1/pull_request.schema.json`, `repository_setup.schema.json` (new), the repository summary and inspection schemas, `src/bin/manyhands-cli/human.rs`, `human_tests.rs`; E-5, E-6, E-7 | 2 |
| 4 | `src/repository/mutation/command.rs` (`SetupGate`), `setup_gate.rs` and `setup_gate_tests.rs` (new), `execute.rs`, `records.rs` (`check_in` at acceptance), `confirm.rs` (`prepare_call`), `src/repository/setup/complete.rs` (`is_complete`), `src/repository/read/admin.rs` (a complete setup reads as none) | 1, 2, 3 |
| 5 | `src/repository/transport/mod.rs`, `operation.rs`, `remote.rs`, `callbacks.rs`, `src/repository/mutation/bind/host.rs`, `src/repository/mutation/progress.rs` and `bind/lifecycle_tests.rs` (`verify_setup_transport`) | 2 |
| 6 | `src/repository/setup/lock.rs`, `commit.rs`, `publish.rs`, `publish_tests.rs` (new), `src/repository/coordination.rs` (the setup lock, `snapshot_file_hash`), `src/repository/remote/sync.rs` (`local_only_commits`'s visibility), `progress.rs` and the test double (`publish_setup`) | 1, 2, 5 |
| 7 | `schemas/input/v1/repo_enable.schema.json`, `src/bin/manyhands-cli/admin.rs`, `src/repository/mutation/bind/repo.rs`, `src/repository.rs` (`enable_with_setup`, `enable_direct_with`, `register_setup_pending`, `canonical_configuration`), `src/repository/recovery.rs` (`LocalStep::SetupRecorded`), `src/repository/mutation/records.rs` (`finish_accepted`), `src/repository/setup/intent.rs` and `intent_tests.rs` (new), `src/repository/mutation/dto.rs`, `schemas/v1/setup_mutation.schema.json`, `setup_preview.schema.json`, `preview_effect.schema.json`, `src/bin/manyhands-cli/human_admin.rs`, `src/desktop/view/confirm.rs`, `tests/support/mutation.rs`, `tests/support/repair.rs`, `tests/repository_enablement.rs`, `tests/cli/input_schemas.rs`, `tests/support/mutation_golden.rs`, `tests/mutation_contract.rs`; E-4, E-7, E-8 (enable), E-10, E-12, E-13, E-14 | 3, 4, 6 |
| 8 | `src/repository/mutation/bind/poll_retirement.rs`, `src/repository/setup/poll.rs`, `poll_tests.rs`, `complete.rs` (the finish), `complete_tests.rs`, `src/repository/mutation/bind/poll.rs`, `src/repository/remote/poll.rs` (`poll_probed`'s end), `src/repository/mutation/dto.rs` (`SetupPollDto`), `schemas/v1/setup_poll.schema.json`, `src/bin/manyhands-cli/polling.rs` and `polling_tests.rs`, `src/repository/remote/state.rs` (the setup poll's attempt), `progress.rs` and the test double (`poll_setup`), `tests/support/mutation.rs`, `tests/support/repair.rs`, `tests/support/mutation_golden.rs`, `tests/mutation_contract.rs`; E-7, E-10, E-13 | 6, 7 |
| 9 | `schemas/input/v1/remote_select.schema.json`, `src/bin/manyhands-cli/admin.rs` and `input.rs`, `src/repository/mutation/bind/remote.rs`, `src/repository.rs` (`set_publication_remote_with`), `src/repository/setup/change.rs` and `change_tests.rs` (new), `tests/support/mutation_golden.rs`, `tests/mutation_contract.rs`; E-4, E-8 (select), E-10 | 6, 7 |
| 10 | `src/repository/setup/cancel.rs`, `cancel_tests.rs`, `src/repository/mutation/bind/setup_cancel.rs` (new), `command.rs`, `mod.rs` (`Mutation`), `bind/mod.rs`, `dto.rs`, `progress.rs` and the test double (`cancel_setup`), `src/bin/manyhands-cli/catalog.rs`, `catalog_tests.rs`, `dispatch.rs`, `human_admin.rs`, `src/desktop/view/confirm.rs`, `schemas/v1/setup_cancel.schema.json`, `setup_cancel_preview.schema.json`, `src/repository/registration_removal.rs` (decision 16), `docs/RFC/cli-contract.md` (the taxonomy and option rows), `tests/support/conformance.rs`, `tests/mutation_confirmation/conformance.rs`, `tests/support/mutation.rs`, `tests/support/repair.rs`, `tests/support/mutation_golden.rs`, `tests/mutation_contract.rs`; E-4, E-7, E-9, E-10, E-12, E-13 | 4, 6, 7 |
| 11 | `src/desktop/services/scheduler/registration.rs`, `entity.rs`, the scheduler's backend and its test double, `src/desktop/model/repository_state.rs`, and their tests | 3, 8 |
| 12 | `tests/mutation_remote.rs` (the case), `tests/support/mutation_golden.rs`, the new goldens, `tests/mutation_contract.rs`; E-4, E-10 | 7 to 10 |
| 13 | `docs/RFC/cli-contract.md`, the RFCs of D-2 and D-3, `docs/research/wave-03-api-audit.md`, skills; E-15 | 1, 7 to 10 |
| 14 | none new | all |

## Task 0: Baseline, References, Ledger

**Files:** create the ledger `docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md` on the ticket branch.

- [ ] **Step 1:** In the worktree, `git fetch origin`, `git merge origin/main`, `git merge-base --is-ancestor origin/main HEAD`, `git status --short`. Record the base SHA and HEAD before and after in the ledger.
- [ ] **Step 2:** Re-check every name and path the design cites (Baseline Facts 1 to 20, Parts A to K) at the new HEAD, and the test module paths each task will filter on. Mark each "existing" or "new" in a ledger table with its current file and line. Read one existing `*_preview.schema.json` and its `PreviewDetailsDto` variant, and record the layout the two new previews copy. Name each target's golden helper. Record how `repo remove`'s live and registry-only paths check accepted requests today (design K5).
- [ ] **Step 3: U-1.** Check, by reading `tests/read_contract.rs` `every_published_object_is_closed_and_requires_all_its_properties`, `tests/mutation_contract.rs` `every_schema_is_reachable_from_a_published_root` and `tests/support/schema.rs`, whether a top-level `"type": ["object", "null"]` file reached by `$ref` passes; record the answer and whether Task 3 uses the shared file or inlines `setup` (design Part I).
- [ ] **Step 4: Premise sweep.** Grep for every site of the design's sweep list (Named Edits, last paragraph) and add each hit as a named edit E-16 onwards (decision 15).
- [ ] **Step 5:** Confirm that no CI test list changes: this Cycle adds no test target.
- [ ] **Step 6:** Commit the ledger. The orchestrator publishes the branch and adds the start checkpoint comment.

## Task 1: The Result Codes

**Files:** `src/results.rs` (`result_codes!`), `src/results_tests.rs`, `src/repository/mutation/outcome.rs` (`recovery`), `outcome_tests.rs`, `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, `tests/mutation_contract.rs`. Named edits: E-2, E-3, E-11.

**Interfaces:** Produces `ResultCode::SetupPending` (`"setup_pending"`, `Blocked`, "The repository's Manyhands setup has not merged yet.", recovery `repo.inspect`), `ResultCode::RemotePrimaryMissing` (`"remote_primary_missing"`, `Blocked`, "The remote has no primary branch to set up from.", no recovery action) and `ResultCode::RemoteAlreadySetUp` (`"remote_already_set_up"`, `Blocked`, "The remote's primary already holds a Manyhands configuration.", no recovery action).

- [ ] **Step 1: Apply E-2**, so the design table test fails. Run `devenv shell -- cargo test --all-features --locked --lib results::tests::result_codes_match_the_design_table_exactly -- --exact`. Expected: FAIL.
- [ ] **Step 2: Implement** the three codes in the same position of every ordered list, their `recovery` arms, and the two schema enums (E-11).
- [ ] **Step 3: Apply E-3.** Run by exact name: `results::tests::result_codes_match_the_design_table_exactly`, `results::tests::result_code_messages_are_fixed`, the `tests/read_contract.rs` tests that compare the enums with `ResultCode::ALL`, and both `every_result_code_has_a_*` tests of `tests/mutation_contract.rs`. Expected: PASS.
- [ ] **Step 4: Commit** "feat(results): setup_pending, remote_primary_missing and remote_already_set_up".

## Task 2: The Setup Record (Design Part A)

**Files:** create `src/repository/setup/mod.rs`, `record.rs`, `record_tests.rs`; modify `src/repository/discovery.rs` (`migrate_registry` and a new migration after `migrate_pull_request_mode`), `src/repository.rs` (`mod setup;`, `register_setup_pending`), `src/repository/recovery.rs` (`LocalStep::SetupRecorded` and `SetupCancelRecorded`, with `is_checkpoint` true). Named edits: E-1, E-14.

**Interfaces:** Produces `pub(crate) struct SetupRecord { setup_id: ItemId, kind: SetupKind, remote_name: String, primary_branch: String, endpoint_digest: [u8; 32], configuration: Vec<u8>, base_configuration_oid: Option<git2::Oid>, base_oid: Option<git2::Oid>, commit_oid: Option<git2::Oid>, published_oid: Option<git2::Oid>, request_id: RequestId, created_at: i64 }`, `pub(crate) enum SetupKind { Enable, Change }` (`as_str` `"enable"`, `"change"`), `pub(crate) enum SetupState { Recorded, Published }`, and `record::{read, insert, record_commit, record_published, delete}`, each taking a registry transaction; `register_setup_pending(tx, root)`; `LocalStep::SetupRecorded` (`"setup_recorded"`) and `LocalStep::SetupCancelRecorded` (`"setup_cancel_recorded"`).

- [ ] **Step 1: Write the failing tests** in `record_tests.rs`: `setup_records_round_trip_and_enforce_their_rules` (one table: insert and read back each kind; a second insert for the same repository refused; a change without `base_configuration_oid` refused; `commit_oid` without `base_oid` refused; `published_oid` other than `commit_oid` refused; a non-hex OID refused; an endpoint digest that is not 32 bytes refused); `a_setup_record_survives_a_rebuild_and_goes_with_remove`; `a_setup_record_holds_no_remote_url_credential_or_identity` (every cell of a written row is free of the remote's URL and the identity's name and email). And beside Cycle 01's migration test in `discovery.rs`, `repository_setup_migration_adds_the_table_once`.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the table (design Part A's SQL with the 40-hex `CHECK`s), the migration (immediate transaction, schema-checked, marker `wave_06_repository_setup_v1`), the store functions, `register_setup_pending` and the two steps. Apply E-1 and E-14.
- [ ] **Step 4: Run** the new tests and E-1's test by exact name, and, unchanged, `tests/mutation_replay.rs` `the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else` and `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly`. Expected: PASS.
- [ ] **Step 5: Commit** "feat(setup): the setup record".

## Task 3: The Contract Shapes And The Reads (Design Part I)

**Files:** `src/repository/read/dto.rs` (`PullRequestDto`, `RepositorySetupDto`, the `setup` field of `RepositorySummaryDto` and `RepositoryInspectionDto`), `src/repository/read/admin.rs`, `src/repository/read/mod.rs` (exports), `schemas/v1/pull_request.schema.json` and `repository_setup.schema.json` (new, or `setup` inlined per Task 0 Step 3), `schemas/v1/repository_summary.schema.json` and `repository_inspection.schema.json`, `src/bin/manyhands-cli/human.rs`. Tests: `src/repository/read/tests.rs`, `src/bin/manyhands-cli/human_tests.rs`. Named edits: E-5, E-6, E-7.

**Interfaces:** Produces `pub struct PullRequestDto { pub source: String, pub target: String, pub forge: Option<String>, pub url: Option<String> }` with `From<&forge::PullRequestLink>`, and `pub struct RepositorySetupDto { pub id: String, pub kind: String, pub state: String, pub request_id: String, pub branch: String, pub remote: String, pub integration: String, pub pull_request: PullRequestDto }`.

- [ ] **Step 1: Write the failing read test** `inspect_and_list_report_a_pending_setup` in `src/repository/read/tests.rs` (the store is crate-private): a registered repository with a record inserted through the store reads `setup` with each field, recorded and then published, from `inspect_repository` and from `list_repositories`; `repo list`'s link has a null `url` when the forge needs the remote URL; a repository without a record reads `setup: null`.
- [ ] **Step 2: Write the failing human tests** in `human_tests.rs`: `repository_inspection_names_a_pending_setup` (`Setup: recorded, not published yet`; `Setup: waiting for manyhands/setup/<id> to merge into main` and the link's human line; nothing without a setup) and `repository_list_marks_a_pending_setup` (` setup=published`).
- [ ] **Step 3: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Implement** the DTOs, both builders, the schemas (`setup` required, an object or null; `pull_request.forge`, and `kind`, `state` and `integration`, as enums) and the human lines. Apply E-5 to E-7; regenerate E-6's four goldens and confirm only `"setup": null` was added.
- [ ] **Step 5: Run** the new tests and every edited test by exact name, including `published_enumerations_list_exactly_the_registered_values`, `every_published_object_is_closed_and_requires_all_its_properties` and `every_schema_is_reachable_from_a_published_root`, then `devenv shell -- cargo check --locked --features desktop-test-support --tests`. Expected: PASS.
- [ ] **Step 6: Commit** "feat(read): report a pending setup".

## Task 4: The Setup Gate (Design Part B)

**Files:** see the File Map.

**Interfaces:** Produces `pub(crate) enum SetupGate { Carries, RefusedBeforeEnable, Refused }`, `Command::setup_gate(self) -> SetupGate`, `setup_gate::check(service: &RepositoryService, command: Command, root: &Path) -> Result<(), Answer>`, `setup_gate::check_in(tx: &rusqlite::Transaction<'_>, command: Command, root: &Path) -> Result<(), Answer>` (read-only), and `setup::complete::is_complete(repository: &git2::Repository, record: &SetupRecord) -> Result<bool, git2::Error>`, which reads `refs/heads/<record.primary_branch>`.

- [ ] **Step 1: Write the failing tests:** in `command.rs`, `every_command_has_the_setup_gate_of_the_design` (loops `Command::ALL` against design Part B's table, written out in the test); in `setup_gate_tests.rs`, `a_pending_setup_refuses_a_first_call_an_acceptance_and_a_prepare_but_not_a_reentry` (a first-time record: `ticket create` and `document promote --prepare` answer `setup_pending` with nothing written and no record; a record inserted between a first call and its acceptance refuses at acceptance; an accepted request re-entered is not refused; a change record: `ticket save` carries on, `ticket close` and `remote remove` are refused; no record: nothing refused), `a_complete_setup_refuses_nothing` (a first-time record whose local primary holds a valid configuration naming its primary; a change record whose primary's blob moved; not when only HEAD, on the setup branch, holds it), and in `src/repository/read/tests.rs`, `inspect_reads_a_complete_setup_as_none`.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Implement** the const fn, `is_complete`, `check` (before `prepare` in `first_call`, before `preview` in `prepare_call`) and `check_in` (in the acceptance and one-transaction paths). Finishing a complete setup is Task 8's; until then `check` treats a complete setup as none and finishes nothing.
- [ ] **Step 4: Run** the new tests and, unchanged, `tests/mutation_replay/not_enabled.rs` `a_registered_repository_with_no_configuration_keeps_the_answers_it_had`, by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(mutation): the setup gate".

## Task 5: The Setup Channel (Design Part C)

**Files:** see the File Map.

**Interfaces:**
- Consumes: Task 2's `record::read`.
- Produces: `pub(crate) struct SetupScope { pub setup_id: ItemId, pub remote_name: String, pub primary_branch: String, pub endpoint_digest: [u8; 32] }`; `pub(crate) fn verify_setup_transport<P>(&self, request: VerifySshTransportRequest, scope: &SetupScope, session: &mut SessionCredentials<P>) -> Result<SshTransportVerified, SshTransportError>`; `pub(crate) fn with_setup_remote<P, T>(&self, request: VerifySshTransportRequest, scope: &SetupScope, automatic: bool, session: &mut SessionCredentials<P>, use_remote: impl FnOnce(&mut AuthenticatedSshRemote<'_, '_>) -> Result<T, SshTransportError>) -> Result<T, SshTransportError>`, one connection in `request.direction`; `fetch_setup_base`, `push_setup_branch`, `delete_setup_branch`, `observe_setup_branch`, each refusing unless the connection's source is `Setup` and its direction is the method's; `endpoint_digest_of(repository, remote_name)`, refactored out of `observation.rs`.

- [ ] **Step 1: Merge `origin/main`** if it moved.
- [ ] **Step 2: Write the failing tests:** `setup_mappings_name_only_the_setup_ref` (pure validators, as `transport_delete_scope_is_exact_context_only` tests the context ones: fetch maps the primary into `refs/manyhands/setups/<id>/base` and nothing else; push maps only `refs/heads/manyhands/setup/<id>`; delete only `:refs/heads/manyhands/setup/<id>`; each only in its direction; a `Configured` source maps no setup ref and a `Setup` source no item ref); `guard_create_and_guard_delete_decide_on_one_exact_update` (the pure predicates: another ref, a non-zero old OID, another new OID, two updates, each refused); `the_setup_scope_is_rechecked_against_the_record_and_the_endpoints` (the record deleted, another `setup_id`, remote or primary, or the remote's URL changed: `EndpointChanged`); in `bind/host.rs`, `host_read_takes_the_remote_of_design_part_c` (no `--authority`: the file's publication remote, or the record's when the file names none; an `--authority` of either remote: that remote; a missing file and no record: `invalid_configuration`, and a valid file with no remote and no record: `publication_remote_required`, as today) and `host_observe_on_a_pending_setup_without_a_key_is_no_selected_key`.
- [ ] **Step 3: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Implement** as the design says, with `Session::verify_setup_transport` for `host approve` (`CallSession`, `NoSession`, the `Recorder` double). The `Configured` branches are today's code, moved behind the source without change.
- [ ] **Step 5: Run** the new tests by exact name, then `devenv shell -- cargo check --all-features --locked --tests`. Expected: PASS.
- [ ] **Step 6: Commit** "feat(transport): the setup channel".

## Task 6: Setup Operations (Design Part D; Test-First With A Design Checkpoint)

**Files:** see the File Map.

**Interfaces:**
- Consumes: Tasks 1, 2 and 5.
- Produces: `coordination::setup_lock(repository, root, operation) -> Result<SetupLock, RepositoryError>` (`busy` when held); `setup::commit::build(repository, base: Oid, configuration: &[u8], message: &str, identity: &CommitIdentity) -> Result<Oid, git2::Error>`; the trait `setup::SetupChannel` (one method per channel operation, each opening its own connection), which tests stub; `setup::publish::continue_setup(service, root, request_id, channel: &mut dyn SetupChannel) -> Result<Published, SetupError>` with `Published { setup_id, commit_oid, link: PullRequestLink }` and `SetupError` mapping to `remote_primary_missing`, `primary_has_local_commits` (with `local_only_commits`), `remote_already_set_up`, `external_change`, `recovery_required`, `cancelled`, `busy` and the transport's stops.

- [ ] **Step 1: Write the failing tests first**, named for their outcomes: `the_setup_commit_changes_only_the_configuration_of_the_base` (tree equals the base's with the file at mode `100644`, one parent, the message, the identity; the root's HEAD, index and working tree unchanged); `s1_to_s4_publish_decides_on_the_fetched_base` (one table over the stubbed channel's base: absent, behind, equal, ahead, diverged, holding a configuration (first-time), holding another configuration (change)); `s2_to_s4_a_refused_setup_is_ended` (no record, for a first-time setup no registration, and the request's own row kept completed); `s5_a_stop_before_the_push_keeps_the_record`; `s6_s7_s8_publish_finishes_across_crashes_and_never_pushes_twice` (a recorded commit and no branch, with step 1 skipped; a pushed ref not yet recorded; a ref at another OID); `s9_a_second_setup_operation_is_busy`; `s10_publish_touches_only_the_setup_ref`; `s11_a_changed_endpoint_stops_publishing`; `s12_a_setup_ended_elsewhere_answers_the_stored_envelope`; `a_lost_commit_object_is_made_again_only_while_the_remote_ref_is_absent`.
- [ ] **Step 2: Run each, expect FAIL**, by exact name under `repository::setup::`. Record the red runs.
- [ ] **Step 3: Design checkpoint.** Before implementing, the implementer writes in its report where each step of design Part D's "Publishing" sits: the lock and the record's second read, the fetch and the four refusals and when they are skipped, the repository lease around the commit and the branch, the order of record and branch writes, each connection and its direction, the observations, and how a refusal ends a setup in one transaction. The orchestrator checks them against S1 to S12 and the source, and records the result in the ledger, before Step 4.
- [ ] **Step 4: Implement** as checked.
- [ ] **Step 5: Run** the tests of Step 1 by exact name. Expected: PASS.
- [ ] **Step 6: Commit** "feat(setup): publish a setup branch". The orchestrator adds the library checkpoint comment.

## Task 7: Repo Enable In Pull-Request Mode (Design Parts E And K; Test-First With A Design Checkpoint)

**Files:** see the File Map. Named edits: E-4, E-7, E-8 (the 8 `RepoEnableInput` literals), E-10, E-12, E-13, E-14.

**Interfaces:**
- Consumes: Tasks 3, 4 and 6; Cycle 01's `forge::default_integration`.
- Produces: `RepositoryService::continue_setup` (the publish-only continuation Task 6 builds, called by `rerun`); `RepoEnableInput { …, publication_remote: Option<String>, integration: Option<Integration>, forge: Option<ForgeSetting> }`; `setup::intent::for_enable(primary_branch, publication_remote, integration, forge, fetch_url) -> Result<Intended, IntentError>` with `Intended { integration: Integration, configuration: RepositoryConfig }` and `IntentError` mapping to `invalid_input` and `invalid_remote`; `RepositoryService::enable_direct_with(request: EnableRepositoryRequest, prefix: Option<&str>, remote: Option<&str>, forge: Option<&ForgeSetting>)`; `RepositoryService::enable_with_setup(request: EnableRepositoryRequest, setup: SetupRequest, session) -> Result<SetupOutcome, RepositoryError>` with `SetupRequest { prefix, remote_name, intended: Intended }` and `SetupOutcome { Published(Published), Recorded { stop: ResultCode }, Ended(SetupError) }`; `records::finish_accepted(tx, request_id, answer: &Answer)`, guarded by command, scope, row and attempt (design K4); `MutationDataDto::Setup(SetupMutationDto)`; `PreviewDetailsDto::Setup(SetupPreviewDto)`.

- [ ] **Step 1: Write the failing tests:** `enable_intent_follows_decision_nine` in `intent_tests.rs` (one table: no remote; a cloud remote with and without `integration`; an unrecognised remote with and without `forge`; `pull_request` without a remote; an invalid forge kind; each gives the mode or the refusal; a pull-request intent serializes `format_version = 2`); in `bind/repo.rs`'s tests, `the_enable_request_lifecycle_of_design_part_k` (one table over a stubbed `Session` for L1, L2 (one row per `Owed` code of K2), L3, L5, L6, L7, L10, L11, L13 and L14: the standing, the re-entry route, the answer, and that no second registration, record or setup branch is made; and `finish_accepted` refusing a request of another command, scope, row or attempt); in `tests/repository_enablement.rs`, `a_pull_request_enable_without_a_key_leaves_a_recorded_setup` (an SSH-shaped never-contacted remote and no selected key: the prepare previews the setup effects; the confirm answers `no_selected_key` with the request accepted; `repo inspect` and `repo list` show `setup` recorded; `ticket create` answers `setup_pending`; `host observe` answers `no_selected_key`; the local primary, HEAD and working tree are unchanged; the same request again answers `no_selected_key` and registers nothing twice), `enabling_a_clone_with_other_settings_is_invalid_input`, and `a_direct_enable_with_a_remote_writes_it_in_the_initialization_commit` (and its lost-result replay); in `tests/cli/input_schemas.rs`, `repo_create_refuses_the_enable_only_fields`.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Design checkpoint.** Before implementing, the implementer writes where each Part K rule sits for `repo enable`: the recording transaction and the lease's release, `SetupRecorded.is_checkpoint()` and the route `Reentry::of` then takes (K1), the `Confirmed::row` branch and `continue_setup`, the standing of each answer (K2), the identity written again on re-entry, the effects without `commit_oid`, and `finish_accepted` and its guard. The orchestrator checks them against K1 to K7, L1 to L14 and the source, and records the result in the ledger, before Step 4.
- [ ] **Step 4: Implement** the input and its digest (new fields only when supplied), the intent, the direct path with a remote, the pull-request path (`register_setup_pending`, `LocalStep::SetupRecorded`, `Session::publish_setup`), its preview, digest, standing, re-entry and replay, the data and preview DTOs and schemas, and the human lines. Apply E-7, E-8 and E-12 to E-14. Write the `repo_enable_setup_preview`, `repo_enable_setup_recorded` (a stop carrying `setup` data, so `setup_mutation.schema.json` has a golden) and `ticket_create_setup_pending` goldens from the fast-tier tests that produce them, and register them as Global Constraints "Goldens" says (E-4, E-10).
- [ ] **Step 5: Run** the new tests and, unchanged, today's enable tests by exact name: `tests/repository_enablement.rs` `enable_born_main_writes_canonical_config_exclusion_and_single_file_commit`, `enabling_a_repository_enabled_elsewhere_registers_and_indexes_it` and `enable_wrong_checked_out_branch_preserves_repository_and_registry`, `tests/mutation_confirmation.rs` `repo_enable_on_a_wrong_branch_is_wrong_branch`, and `tests/cli/repository.rs` `enable_an_existing_repository`; then the registration tests of "Goldens", and `devenv shell -- cargo check --all-features --locked --tests`. Expected: PASS.
- [ ] **Step 6: Commit** "feat(enable): onboarding on a setup branch".

## Task 8: Poll: Activation And Completion (Design Parts B And G)

**Files:** see the File Map. Named edits: E-7, E-10, E-13.

**Interfaces:** Produces `RepositoryService::poll_setup<P>(&self, root: &Path, automatic: bool, session: &mut SessionCredentials<P>) -> Result<SetupPollReport, RepositoryError>` with `SetupPollReport { setup_id, outcome: SetupPollOutcome, integration: Option<Integration>, differs: bool, link: PullRequestLink }` and `SetupPollOutcome { NotPublished, ConfigurationInvalid, PrimaryDiffers, BranchDeleted, WaitingForMerge, WaitingForCheckout, Diverged, Activated }`; `setup::complete::finish(service, root) -> Result<bool, RepositoryError>` (false when the lock is held or the setup is not complete) and `finish_held(lock: &SetupLock, service, root)`; a `remote::state` function recording a setup poll's attempt, outcome and next eligible time; `MutationDataDto::SetupPoll(SetupPollDto)`.

- [ ] **Step 1: Merge `origin/main`** if it moved.
- [ ] **Step 2: Write the failing tests:** `g1_to_g7_the_setup_poll_decides_on_the_base` (one table over local repositories and a stubbed channel, in design Part G's order, with a merge commit, a squash and a fast-forward of the setup commit for G7); `activation_leaves_the_repository_as_a_direct_enable_would` (after `Activated`, `repo inspect`'s data, apart from `setup`, and the `repositories`, `configuration_observations` and `remote_polling_state` rows equal those of a direct enable plus remote selection of the same configuration; the record, the local setup branch and the private ref are gone; a ticket can be created; `differs` false, and true with the merged mode in effect when the merged file differs; for a change on another remote, the same on the new remote); `a_change_on_another_remote_still_polls_the_current_remote`; `a_crash_after_the_fast_forward_is_finished_by_the_next_poll_or_the_gate`; in `complete_tests.rs`, `a_setup_completes_when_the_local_primary_holds_it` (a merge commit, a squash and a fast-forward reaching the local primary: finished by the gate and at the end of `poll_probed`; not before; not with the setup branch checked out; a held lock skips it and changes no answer; the setup request, if still accepted, finished as `already_applied`, or `external_change` when the merged configuration differs: L4, L5; reconciliation stopped by an operation in the way and finished at the next look; never fencing the calling poll); in the poll binding's tests, `a_setup_poll_rerun_takes_the_setup_path` (L12) and `the_setup_poll_s_stops_are_final` (key, host, network, Ctrl-C); in `src/bin/manyhands-cli/polling_tests.rs`, `setup_poll_lines` for each outcome.
- [ ] **Step 3: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 4: Design checkpoint.** The implementer writes how the poll binding settles a setup poll's request with no remote row (finished on every answer), where the hand-off sits in `run` and `rerun` for `poll once` and `--retire-completed`, where `finish` is called at the end of `poll_probed` and `finish_held` in `poll_setup`, and where the setup poll's attempt is recorded. The orchestrator checks it against design Parts B and G and records it.
- [ ] **Step 5: Implement** as the design says, with `Session::poll_setup`, and the gate's call of `finish` (Task 4's `check`). `poll_publication_remote`'s signature and `PollMutationDto` do not change. Apply E-7 and E-13. Write the `poll_once_setup_not_published` golden from its fast-tier test and register it (E-10).
- [ ] **Step 6: Run** the new tests by exact name, and, unchanged, `poll_retirement_stored_contract_is_closed_and_cannot_complete_a_not_reached_prefix` and `task6_postfetch_advances_primary_and_replays_without_transport`; then the registration tests of "Goldens", and `devenv shell -- cargo check --all-features --locked --tests`. Expected: PASS.
- [ ] **Step 7: Commit** "feat(poll): activate and complete a setup". The orchestrator adds the checkpoint comment.

## Task 9: Setting Changes Through Remote Select (Design Parts F And K; Test-First With A Design Checkpoint)

**Files:** see the File Map. Named edits: E-4, E-8 (the 16 `RemoteSelectInput` literals), E-10.

**Interfaces:** Produces `RemoteSelectInput { root, name, integration: Option<Integration>, forge: Option<Option<ForgeSetting>> }` (outer `None`: unchanged; `Some(None)`: removed), a split `remote_select` parser with a nullable-object helper in `src/bin/manyhands-cli/input.rs`, `RepositoryService::set_publication_remote_with(request: SetPublicationRemoteRequest, forge: Option<Option<&ForgeSetting>>)` for the direct path, and `RepositoryService::change_setup(request: ChangeSetupRequest, session) -> Result<SetupOutcome, RepositoryError>` with `ChangeSetupRequest { root, operation_id, request_id, intended: Intended }`.

- [ ] **Step 1: Write the failing tests:** `remote_select_takes_the_path_of_the_design` in `change_tests.rs` (one table over design Part F's rows: an equal configuration is `already_applied`; direct to direct, with and without a forge change; direct to pull request on the same, a first and a different remote; pull request to direct; a forge change in pull-request mode; another remote in pull-request mode is `invalid_input`; clearing the remote of a pull-request configuration is `invalid_configuration`; an unfinished close, and an accepted close or other `remote select` request, are `recovery_required`, while the change's own request passes; a pending setup is `setup_pending`; each pull-request row leaves the local primary's OID unchanged); in `bind/remote.rs`'s tests, `the_select_request_lifecycle_of_design_part_k` (L1, L2, L3, L5, L6 and L14 over a stubbed `Session`: the route past `moved`, the path chosen from the intent, and no second record or setup branch); in `tests/repository_enablement.rs`, `turning_pull_request_mode_on_without_a_key_records_a_change` (no selected key: `no_selected_key`, the request accepted, `repo inspect` shows the change; the primary's configuration unchanged; `ticket close --prepare` answers `setup_pending`; `ticket save` carries on; the same request again continues and makes no second record); in `src/bin/manyhands-cli/`, the human lines of the setup path.
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Design checkpoint.** The implementer writes where each Part K rule sits for `remote select`: the recording transaction with its race checks, the route `Reentry::of` takes for a `SetupRecorded` row and how it reaches `reenter` and `rerun` instead of `moved`, the path chosen from the intent, the standing of each answer (K2), the digest of the new fields, and the direct path's forge. The orchestrator checks them against K1 to K4, L1 to L6, L14 and the source, and records the result in the ledger, before Step 4.
- [ ] **Step 4: Implement** as checked. Apply E-8. Write the `remote_select_setup_recorded` golden from its fast-tier test and register it (E-4, E-10).
- [ ] **Step 5: Run** the new tests and, unchanged, `tests/repository_enablement.rs` `publication_selection_commits_only_canonical_configuration_changes` and `publication_remote_selection_keeps_mode_and_forge`, `tests/cli/remotes.rs` `select_then_clear`, and `tests/cli/input_schemas.rs` `remote_names_are_required_and_only_selection_is_nullable`, by exact name; then the registration tests of "Goldens". Expected: PASS.
- [ ] **Step 6: Commit** "feat(remote): setting changes on a setup branch".

## Task 10: Repo Setup-Cancel (Design Parts H And K; Test-First With A Design Checkpoint)

**Files:** see the File Map. Named edits: E-4, E-7, E-9, E-10, E-12, E-13.

**Interfaces:** Produces `Command::RepoSetupCancel` (`"repo setup-cancel"`; `has_operation` true, `needs_confirmation` true, `stops_at_safe_points` false, `stage` as enable's), `Mutation::RepoSetupCancel { root }`, `RepositoryService::cancel_setup<P>(&self, root: &Path, session: &mut SessionCredentials<P>) -> Result<SetupCancelReport, RepositoryError>`, `MutationDataDto::SetupCancel(SetupCancelDto)`, `PreviewDetailsDto::SetupCancel(SetupCancelPreviewDto)`.

- [ ] **Step 1: Write the failing tests:** `cancel_outcomes` in `cancel_tests.rs` (one table over a stubbed channel: no commit, so no connection, unregistering a first-time setup; published and unmoved deletes the remote ref; `Changed` keeps it; a ref at another OID is kept with no delete sent; `StillPresent` answers `push_rejected` with nothing more changed; absent goes on; a pushed but unrecorded ref is found and deleted; already merged (the base holds the setup, whatever the checkout) is `recovery_required` with nothing changed; a change keeps the registration; no record is `already_applied`; another pending operation row in the way is `recovery_required`; the setup request finished as `cancelled` in the same transaction; the cancel's own row kept and its request finished; a publish between prepare and confirm is `external_change`); `a_cancel_interrupted_between_steps_is_finished_by_the_same_request`; in `tests/repository_enablement.rs`, `cancelling_a_recorded_setup_unregisters_it` (from Task 7's recorded setup: prepare, then confirm; the enable's request then answers `cancelled`: L4) and `repo_remove_always_removes_a_pending_setup` (L9: with the root present and gone; the setup request finished as `cancelled`; both branches left).
- [ ] **Step 2: Run each, expect FAIL**, by exact name. Record the red runs.
- [ ] **Step 3: Design checkpoint.** The implementer writes where each step of design Part H sits, each connection and its direction, the transaction that deletes the record, the registration and the other rows and finishes both requests, and `repo remove`'s finishing on both paths. The orchestrator checks them against Part H, K4, K5, L4 and L9, and records the result, before Step 4.
- [ ] **Step 4: Implement** the library, the binding with `Session::cancel_setup`, the CLI entry (`delivered_by` "C5", `MUTATION_NO_INPUT`, confirmed), `repo remove`'s finishing of the setup request on both its paths (decision 16, K5), its taxonomy and option rows in `docs/RFC/cli-contract.md`, and the registrations of E-9. Apply E-7, E-12 and E-13. Write the `repo_setup_cancel_preview`, `repo_setup_cancel_recorded` and `repo_setup_cancel_already_applied` goldens from their fast-tier tests and register them (E-4, E-10).
- [ ] **Step 5: Run** the new tests and every E-9 test by exact name, including `the_commands_whose_caller_must_first_be_shown_need_confirmation`, `tests/cli/inventory.rs`'s count and taxonomy tests, `every_mutation_sits_in_one_autonomy_column` and the help goldens; then the registration tests of "Goldens", and `devenv shell -- cargo check --all-features --locked --tests`. Expected: PASS.
- [ ] **Step 6: Commit** "feat(setup): repo setup-cancel".

## Task 11: The Desktop's Minimum (Design Part J)

**Files:** see the File Map.

- [ ] **Step 1: Write the failing tests:** `a_published_setup_on_another_remote_is_polled_on_the_setup_path` (the scheduler reads `SetupPending`, schedules it by the repository's polling policy and the setup poll's recorded attempt, calls `poll_setup` with the automatic policy, and refreshes after `activated`; for a change on another remote it also polls the current remote; a repository without a setup reads as today, with today's read counts); `a_setup_pending_repository_reads_waiting_not_needs_fixing` (published: "Waiting for setup to merge"; recorded: "Setup not published yet"; no action; a missing configuration without a setup still reads "Needs fixing").
- [ ] **Step 2: Run each, expect FAIL**, by exact name with `--features desktop-test-support --bin manyhands`. Record the red runs.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests and, unchanged, `each_state_and_its_precedence`, `menu_items_follow_the_state`, `launch_polls_due_ssh_repos` and `clock_event_reads_policies_and_remotes`, by exact name. Expected: PASS.
- [ ] **Step 5: Commit** "feat(desktop): poll and show a pending setup".

## Task 12: The SSH Case And Its Goldens

**Files:** `tests/mutation_remote.rs` (the case in its `CASES` table, after `a_pull_only_repo_sync_fast_forwards_then_refuses_local_commits`), `tests/support/mutation_golden.rs` (`CASES`), the new goldens under `tests/fixtures/mutation_v1/`, `tests/mutation_contract.rs` (`DATA_ROOTS`, the `CODES_` tables). Named edits: E-4, E-10.

- [ ] **Step 1: Merge `origin/main`** if it moved.
- [ ] **Step 2: Write the case** `a_setup_branch_is_published_activated_changed_and_cancelled` (design P15), with this comment above it: no non-SSH publication remote can be selected, and the property is a real push and guarded delete of the setup ref with the server's primary untouched. It builds a clone that is not enabled and whose host is not trusted: `clone_owned` from the fixture's server (which seeds `main`), a registered and selected key without a passphrase (as `World::plain_key` uses), a repository-local identity, and the privacy probes saved, because the target runs `run_with_output_privacy`; and a peer clone of the same server. Then:
  1. runs a pull-request `repo enable` with a fixed request ID and asserts `host_approval_required` and a recorded setup; runs `host observe` and `host approve` on the pending repository; deletes `main` on the server's bare repository; runs the same request and asserts `remote_primary_missing` and nothing registered;
  2. restores the server's `main` at the clone's primary; runs a new `repo enable` and asserts `ok`, the `setup` data with its `pull_request`, the server's primary unchanged and the server's setup ref at the setup commit; runs the same request again and asserts the identical envelope with no new push;
  3. runs `ticket create` and asserts `setup_pending`;
  4. writes on the server's primary one new commit whose tree is the setup commit's (a squash); runs `poll once` and asserts `activated`, the local primary at the server's tip, `repo inspect` reading `integration` `pull_request` and `setup` null, and the local setup branch gone; then `ticket create` succeeds;
  5. runs `remote select` changing the forge (a change on the same remote) and asserts a published setup with the server's primary unchanged; runs `ticket close --prepare` and asserts `setup_pending`; has the peer fetch `refs/heads/manyhands/setup/<id>`, commit on it and push it with `push_peer`; runs `repo setup-cancel` and asserts the remote branch kept and the local one deleted;
  6. runs a second change and `repo setup-cancel`, and asserts the server's setup ref gone and `repo inspect` reading `setup` null.
- [ ] **Step 3: Run it red** against a stub (`poll_setup` answering `waiting_for_merge`), singly by exact name: `devenv shell -- cargo test --all-features --locked --test mutation_remote a_setup_branch_is_published_activated_changed_and_cancelled -- --exact`. Record the red run.
- [ ] **Step 4:** Write the goldens with the harness's golden writer (`repo_enable_setup`, `repo_enable_remote_primary_missing`, `poll_once_setup_activated`, `remote_select_setup`, `repo_setup_cancel`) and register them as Global Constraints "Goldens" says (E-4, E-10). Read each golden. `remote_already_set_up` keeps permanent reasons in both `CODES_` tables ("a library test pins it") unless the case produces it.
- [ ] **Step 5: Run** the case again, and the registration tests of "Goldens" and `tests/read_contract.rs` `published_enumerations_list_exactly_the_registered_values`, by exact name. Expected: PASS.
- [ ] **Step 6: Commit** "test(setup): onboarding and setting changes end to end". The orchestrator adds the checkpoint comment.

## Task 13: Documents And Skills (Design D-1 To D-5)

- [ ] **Step 1:** Apply D-1 to D-4. Each amendment opens with "Amended 2026-11-25 by Wave 06 Cycle 02 (vh-c97se):". In Manyhands-managed text, one line per paragraph. Apply E-15: a recovery-table row with its golden, or a listed reason, for each recoverable code the new paths answer.
- [ ] **Step 2:** Apply D-5 to `skills/` and `.agents/skills/`, editing each copy where the two differ, keeping Cycle 01's sentence that pull-request mode is not usable end to end until close and promote support it.
- [ ] **Step 3: Run** each by its own command: `devenv shell -- cargo test --all-features --locked --test cli skills::`, `--test cli recovery_tables`, `--test cli inventory::`, `--test cli collaboration_exits` and `--test cli exits::`. Expected: PASS, more than zero tests each.
- [ ] **Step 4: Commit** "docs: amend the contracts and skills for onboarding on a setup branch".

## Task 14: Gates, Review-Ready, Closure

- [ ] **Step 1:** Merge `origin/main` if it moved, then `git merge-tree --write-tree HEAD origin/main`. Expected: no conflicts.
- [ ] **Step 2:** The static gates. Expected: clean.
- [ ] **Step 3: The fast tier**, AGENTS.md's command as written. Expected: PASS. Record the time.
- [ ] **Step 4: The SSH tier of decision 11:** the case singly by exact name, then `devenv shell -- cargo test --all-features --locked --test ssh_transport` and `--test remote_polling`. Expected: PASS. Record the times.
- [ ] **Step 5:** Update the Cycle document's Acceptance statuses through `document save`, and the ledger. The orchestrator adds the review-ready comment (gates, tiers and times, the SSH case, the named edits applied) and publishes the ticket branch and the documents with `item sync`.
- [ ] **Step 6: After review approval only:** commit any owned edit to the ticket's `ticket.md`, save the status as done with `ticket save`, publish with `item sync`, then run `ticket close --prepare` from the main checkout with the key loaded in ssh-agent, show the owner what will merge, publish and be removed, and `--confirm` after the owner's go-ahead. The close performs the merge. Then, each with `--prepare` shown to the owner and `--confirm` after the owner's go-ahead: promote the three planning documents, and save and promote the Wave 06 document with Cycle 02's boxes ticked. Native CI runs after the merge, on main, only when the owner dispatches it.

## Plan Self-Review

| Acceptance or ruling | Task |
| --- | --- |
| A1 | 7 (intent, direct path, clone, `repo create`), 9 (select's paths) |
| A2 (S1 to S12) | 6, 12 |
| A3 | 5 (host), 7, 12 (host stop and approval) |
| A4 | 4, 7 (golden) |
| A5 | 3 |
| A6 | 9 |
| A7 | 8 |
| A8 | 4 (predicate), 8 (finish) |
| A9 | 10, 12 |
| A10 | 12 |
| A11 | 14 |
| A12 | 11 |
| A13 (L1 to L14) | 6 (L8), 7 (L1 to L3, L5 to L7, L10, L11, L13, L14), 8 (L4, L5, L12), 9 (L1 to L3, L5, L6, L14), 10 (L4, L5, L9) |
| A14 | 10 (taxonomy row), 13 |
| R-1 | 2 |
| R-2 | 4 |
| R-3 | 5 |
| R-4, R-5, R-6, R-7 | 6 |
| R-8, R-9, R-10, R-11 | 7 |
| R-13, R-14 | 10 |
| Decision 16 (K5) | 10 |
| R-15 | 12 |
| R-16 | 8 |
| R-17 | 4, 8 |
| R-19, R-20 | 7, 9, 10 |
| R-21 | 8 |
| Decision 14 (autonomy) | 10 (E-9), 13 (skills) |
| D-6 | at approval, and Task 14 Step 6 |
| Review Focus lines | 2, 4, 5, 6, 7, 8, 9, 10, 12 |

Checked: every design Part has a task (A: 2; B: 4, 8; C: 5; D: 6; E: 7; F: 9; G: 8; H: 10; I: 3, 7, 8, 10; J: 11; K: 7, 9, 10); the types named in Interfaces match across tasks (`SetupRecord`, `SetupKind`, `SetupState`, `SetupScope`, `SetupChannel`, `SetupGate`, `SetupLock`, `SetupRequest`, `SetupOutcome`, `SetupError`, `Published`, `Intended`, `IntentError`, `ChangeSetupRequest`, `PullRequestDto`, `RepositorySetupDto`, `SetupPollReport`, `SetupPollOutcome`, `SetupCancelReport`, `enable_with_setup`, `enable_direct_with`, `set_publication_remote_with`, `change_setup`, `poll_setup`, `cancel_setup`, `is_complete`, `finish`, `finish_held`, `check_in`, `finish_accepted`, `continue_setup`, `register_setup_pending`, `endpoint_digest_of`); no placeholder steps; no new test target, so no CI list changes.
