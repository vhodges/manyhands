# Wave 06 Cycle 02 Execution Ledger

Ticket vh-c97se (`01M84H84WQYE8J2WJWEDGJ45W4`). Plan: managed document `01M89CHQHEV6NF0X0CTBC1G1AB`, `docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-implementation.md` (approved 2026-11-25, revision 4, at `40fe01fe` on its document branch). Design `01M89CHPWV2W6RY0XART0416Q3` (`40436b2e`), Cycle document `01M89CHP8JBKETDQSZGBPVRTET` (`7cda8aa1`). Execution: subagent-driven, implementers on Sonnet 5.5 at high effort (Cycle decision 18). One line per paragraph.

## Approval

- 2026-11-25: the owner approved the three documents at revision 4, confirmed the tests (decision 11's tier stands; E-1 to E-15 under decision 15), chose subagent-driven execution with Sonnet 5.5 high implementers, and authorized implementation (Cycle decisions 17 and 18). Not authorized: merge, close, promotion, native CI.
- D-6 saved on the Wave 06 document `01M86QA59F7N2DFWTDP5ST2QAW` at `b1bccae7` (its document branch, published, not promoted).
- Notes left on vh-gy8gz, vh-rphcs, vh-p7st2 and vh-ssepw. Cycle 03 (vh-gy8gz) is being implemented at the same time; whichever Cycle lands second reconciles the `pull_request` schema and DTO (one copy), the result-code lists, the golden registrations and the gate row for `ticket withdraw-closure` (`Refused`, like close).

## Task 0

- Base: `origin/main` `3df52dcb`. HEAD before and after the merge: `f0de4f24` (already up to date; merged, never rebased). Main has not moved since planning (`git log da032ffb..HEAD -- src tests schemas Cargo.toml .github` is empty).
- Names: every cited name exists, or is new where the design says so; the reference table is in the report below. Discrepancies DISC-1 to DISC-9 are below with their rulings.
- U-1: yes. A top-level `"type": ["object", "null"]` file reached by `$ref` passes `tests/support/schema.rs`, `closed_object_problems` and `every_schema_is_reachable_from_a_published_root`, so Task 3 uses the shared `repository_setup.schema.json`. It must not be a `DATA_ROOT`.
- Test module paths per task: the report's section 4. New test files are declared at the end of their module as `#[cfg(test)] #[path = "X_tests.rs"] mod tests;`; `bind/host.rs` and `bind/remote.rs` gain test modules.
- Preview layout: the report's section 5 (required keys `confirmation_id`, `expires_at`, `command`, `scope`, `effects`, `details`; `details` closed, told apart by its key set; `PreviewDetailsDto` is untagged). Golden helpers: section 6. K5: section 7.
- CI: no change to `.github/workflows/build.yml` or AGENTS.md's test lists; no new test target.
- The controller's pre-flight scan found one ordering conflict: Task 6's R-7 ending needs `records::finish_accepted`, which Task 7's Interfaces produce. Ruling R-B below.

### Discrepancies and rulings

- DISC-1: design S11 and K2 answer a changed setup endpoint as `recovery_required`, but `outcome.rs` `ssh_transport_error_code` maps `EndpointChanged` to `external_change`, pinned by `outcome_tests.rs` `ssh_kinds()`. Ruling: the setup paths answer it as the design says, `recovery_required`, by mapping it in the setup error conversion; the shared mapping and its pinned table are unchanged — `external_change` would offer `request.prepare` for an accepted setup request — if wrong, one mapping arm.
- DISC-2: Part H names no `RepositoryOperation` for `repo setup-cancel`'s local row. Ruling: a new `RepositoryOperation::CancelSetup`, action `cancel_setup`, handled at every exhaustive site (`src/repository.rs` enum, `read/dto.rs` `OperationAction`, `read/status.rs`, `recovery.rs` name maps, `bridges/abandon.rs`, the desktop's `activity/copy/states.rs`) and added to the action enums of `blocker`, `operation`, `operation_mutation` and `operation_abandon_preview` schemas (additive; E-7 covers `contract_enumerations`; E-19 below) — reusing `RemoveRegistration` collides with `require_registry_only_removal`'s identity test, and `Enable` would mislabel the row — if wrong, a rename.
- DISC-3: `guard_delete` is an inline `push_negotiation` closure with no pure predicate, and `guard_create` does not exist. Ruling: Task 5 extracts the update decision of `guard_delete` into a pure function the closure calls, with no behaviour change, and adds `guard_create`'s predicate beside it. Because this touches the shared delete guard, at review-ready the existing SSH cases that exercise `guard_delete` also run singly by exact name (Task 5 names them) — if wrong, a context deletion changes behaviour; those cases catch it.
- DISC-4: the free fn `reconcile_registration` stores no remote plan; the method `RepositoryService::reconcile_registration` stores it only for `RepositoryOperation::SetPublicationRemote`. Ruling: completion (Part B step 2) and activation of a setup on another remote reconcile through the method with `SetPublicationRemote`, so the remote plan is stored as remote selection stores it; Task 8's checkpoint checks it — if wrong, `activation_leaves_the_repository_as_a_direct_enable_would` fails on `remote_polling_state`.
- DISC-5: `repo remove`'s live path still refuses for an unrelated pending local row or an active remote reservation (`in_the_way`). Ruling: decision 16's "always works" means a pending setup and its request never block `repo remove`; an unrelated unfinished operation refuses as today, unchanged. L9's test uses no unrelated row — if the owner meant more, that is a contract change for them.
- DISC-6 to DISC-9 (trivial, no ruling needed): `EnableRepositoryRequest` has 86 literals in 44 files, 14 in SSH-harness files; the remote module has 19 configuration read sites in 5 files, not about thirty; `migrate_synchronization` guards the existing empty retirement children; Task 3 creates `pull_request.schema.json` (new), and the File Map omits `transport/tests.rs` (E-16), `tests/cli/recovery_tables.rs` (E-17) and `tests/cli/authoring_exits.rs` (E-18).
- Ruling R-B: Task 6 creates `records::finish_accepted(tx, request_id, answer)` with its K4 guard (command, scope, row, attempt) and a unit test of the guard, because design Part D's R-7 ending finishes the request in its transaction; Task 7 consumes it and keeps its L-table rows — if wrong, the function lands one task early.
- Ruling R-C: implementer commits carry `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`, the orchestrator's the Opus 5.5 trailer — attribution names the model that wrote the commit — if wrong, trailer text only.
- Ruling R-J: Task 5 keeps `prepare_ssh`'s signature and adds a sibling for the setup source, so `operation_registry_tests.rs`'s calls need no edit — if wrong, a mechanical extra argument.
- Ruling R-K: E-15's C5 recovery rows for `setup_pending` are goldens (for example a `ticket close --prepare` golden), not reasons: `tests/cli/recovery_tables.rs`'s `c5_reasons.len() <= 2` has one free slot, and raising it would weaken the assertion. If Task 13 needs a second C5 reason, it stops for the owner.
- Ruling R-L: the desktop `Backend` gains the setup poll with a default body and keeps it off the recorded call kinds, so the four implementations and the scheduler's read-count tests need no edit; if Task 11 cannot, it names the edits then.

### Named edits added by the sweep (decision 15)

- **E-16** (Task 5) `src/repository/transport/tests.rs` `prepared()`: its `PreparedSshAttempt` literal gains the source field as `Configured`.
- **E-17** (Tasks 10 and 13) `tests/cli/recovery_tables.rs` const `C2` (and `FAMILIES`): add `repo setup-cancel` (the "Repository and credentials" family), so `codes_of` finds the verb's audit row.
- **E-18** (Task 13) `tests/cli/authoring_exits.rs` `the_c3_table_and_reasons_account_for_every_code_of_the_audit`: if D-4 adds `setup_pending` to the audit's C3 union, the counts go 40 to 41 and 42 to 43, with one `ROWS` entry (preferred) or `REASONS` entry.
- **E-19** (Task 10, Ruling DISC-2) `tests/read_contract.rs` `LOCAL_ACTIONS` (`[OperationAction; 16]` to 17) and its `contract_enumerations` rows for the four action enums: add `cancel_setup`.

## Task 0 sweep report (verbatim)

# Task 0 report (Steps 2 to 5): references, U-1, sweep, CI

Checked against the ticket worktree `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M84H84WQYE8J2WJWEDGJ45W4` at HEAD `f0de4f24` (clean tree). Read-only: nothing was edited, no cargo was run. Paths below are relative to that worktree unless they start with `tests/`, `src/` and so on (same root).

Main moved: nothing. Main has not moved since planning. `git log da032ffb..HEAD -- src tests schemas Cargo.toml .github` is empty, so every line number below is also the line number at the design's `da032ffb`; the design's "approximate" numbers differ from these only by being approximate.

Status: DONE_WITH_CONCERNS. 9 discrepancies (2 substantive, 3 moderate, 4 trivial). 6 E-16+ candidates (3 firm edits, 2 conditional, 1 owner-stop hazard). U-1: yes. No CI list change.

## 1. Discrepancies

Each: exact design text, exact source evidence, severity.

**DISC-1 (substantive). Part D S11 and Part K K2: `EndpointChanged` is "answered as `recovery_required`".**
- Design: S11 "The setup remote's URL changed after recording | `EndpointChanged`, answered as `recovery_required`; nothing sent"; K2 lists `recovery_required` (S8, S11, ...) among the `Owed` answers.
- Source: `src/repository/mutation/outcome.rs:109-124` `ssh_transport_error_code` maps `SshTransportErrorKind::EndpointChanged` (with `KeySourceChanged`, `SelectionChanged`, `HostTrustChanged`) to `ResultCode::ExternalChange`. `src/repository/mutation/outcome_tests.rs` `ssh_kinds()` (line ~360, `(Kind::EndpointChanged, Code::ExternalChange)`) pins it, and `code_of_every_ssh_kind` fails to compile for an unlisted kind.
- Consequence: the setup bindings must map `EndpointChanged` themselves (they cannot change the shared mapping without breaking the pinned table), or the design must say `external_change`. `external_change` on a confirmed command offers `request.prepare` (`outcome_tests.rs:1545-1565`), which is wrong for an accepted setup request. Task 6 must decide before implementing S11; the design checkpoint should name it.

**DISC-2 (substantive, a gap). Part H and plan Task 10 name no `RepositoryOperation` / `OperationAction` for `repo setup-cancel`'s local row.**
- Design: "Like a setup request, it opens its local row with `begin_lifecycle`, records `LocalStep::SetupCancelRecorded` ...". `begin_lifecycle` (`src/repository.rs:4060`) requires a `RepositoryOperation`; its `action_name` is the row's `action` text. Enable and `remote select` reuse `Enable` and `SetPublicationRemote`; the cancel has no named one.
- If a new variant is added, these sites change and none is in the File Map or E-1 to E-15: `src/repository.rs:1284-1288` (the enum), `src/repository/read/dto.rs:784-787` (`OperationAction` `contract_enum!`), `src/repository/read/status.rs:242-245`, `src/repository/recovery.rs:3687-3690` and `3727-3730` (name maps), `src/repository/bridges/abandon.rs:232-237` (exhaustive `Writes` match), `src/desktop/model/activity/copy/states.rs:264-268`, the action enums of `schemas/v1/blocker.schema.json:39-42`, `operation.schema.json:46-49`, `operation_mutation.schema.json:30-33`, `operation_abandon_preview.schema.json:84-87`, `tests/read_contract.rs` `LOCAL_ACTIONS: [OperationAction; 16]` (line 690) and its `contract_enumerations` rows (1077-1172). Reusing `RemoveRegistration` would collide with `require_registry_only_removal`'s `action='remove_registration' AND target=''` identity test (`registration_removal.rs:172, 216`). The owner or orchestrator should pick one before Task 10's design checkpoint.

**DISC-3 (moderate). Part C: "`guard_create` and `guard_delete`'s decisions are pure predicates over the update count, source, destination and ref name, so a unit test can reach them".**
- Source: `src/repository/transport/callbacks.rs:222-238` `guard_delete` takes `&mut RemoteCallbacks` and holds the whole decision inline in a `callbacks.push_negotiation(move |updates| ...)` closure over `git2::PushUpdate`. There is no free predicate. `guard_create` does not exist. The `callbacks.rs` test module (line 241) has one test (`a_second_key_submission_on_one_connection_fails_closed`).
- Consequence: Task 5 must extract a pure predicate from the existing `guard_delete` (a change to the `Configured` path, which the design says is "today's code ... without change") so that plan test `guard_create_and_guard_delete_decide_on_one_exact_update` can reach it.

**DISC-4 (moderate). Baseline Fact 1: "`reconcile_registration` stores `config_blob_oid` ... and stores the remote plan (`remote::state::configure`)".**
- Source: the free fn `reconcile_registration` (`src/repository.rs:13440-13500`) only upserts the `repositories` row (`config_blob_oid` from `committed_configuration_blob_oid`, `refresh_required = 1`). The remote plan is stored by the method `RepositoryService::reconcile_registration` (`src/repository.rs:4015-4056`) and only when `operation == RepositoryOperation::SetPublicationRemote` (lines 4038-4054). For `Enable` no plan is stored. Part B step 2 ("reconciles the registration and stores the remote plan as remote selection does") therefore must call the method as `SetPublicationRemote`, not as `Enable`.

**DISC-5 (moderate). Baseline Fact 14 and decision 16 / R-18 ("`repo remove` always works"): refusal "only on its registry-only path".**
- Source: the accepted-request check is only in `require_registry_only_removal` (`src/repository/registration_removal.rs:141-238`), called only when `repository.is_none()` (lines 342-344). That part is as stated (section 6). But the live path then calls `begin_or_reconcile_operation` (`registration_removal.rs:345`), whose `begin_or_reconcile` runs `in_the_way` (`src/repository/recovery.rs:2839`, called at 1668) and refuses `recovery_required` for any other pending local row or active remote reservation, and yields an automatic poll with `poll_yielding`. A setup request's own row is completed at `SetupRecorded`, so it is not in the way, but "always works" does not hold against an unrelated pending row (a pending `ticket save` row, an active remote operation). L9's proof (`tests/repository_enablement.rs`) should not assume otherwise.

**DISC-6 (trivial). Baseline Fact 10 counts.** Design: "`EnableRepositoryRequest` has about 80 struct literals in 44 files, 13 of them in SSH-harness files". Source: 86 `EnableRepositoryRequest {` occurrences in 44 files; 14 occurrences in the 9 files that name `ssh_harness` (`remote_polling.rs` 3, `comment_publication.rs` 1, `remote_merge_recovery.rs` 2, `document_promotion.rs` 2, `document_promotion/quality.rs` 1, `ssh_transport/synchronization.rs` 1, `ticket_closure.rs` 2, `support/lifecycle_world.rs` 1, `mutation_remote/c5_poll.rs` 1). `RepoEnableInput`: 8 plain literals (admin.rs:228, desktop `fixtures.rs:645`, `add_repository_tests.rs:443`, `build.rs:71`, `tests/mutation_confirmation.rs:2421, 2578`, `tests/support/confirmation.rs:292, 797`), as stated; the other 13 use `..`; `bind/repo.rs:145` is a destructure.

**DISC-7 (trivial). Baseline Fact 5: "About thirty sites in `src/repository/remote/` read the file".** Source: 19 non-test `read_configuration` / `ConfigurationInspection::` read sites in 5 files (`sync.rs` 10, `poll.rs` 3, `conflict_read.rs` 2, `observation.rs` 2, `resolve.rs` 2); 53 lines in those files mention the inspection or the plan builder. The design's conclusion (do not teach them about the record) is unaffected.

**DISC-8 (trivial). Baseline Fact 4: "Its one rebuild (`migrate_synchronization`) ran before those tables existed."** Source: `src/repository/remote/state.rs:1782-1880` rebuilds `remote_operation_records` and explicitly handles the already-existing, empty context-retirement children (lines 1805-1823: refuses if any hold rows; the comment says SQLite would rewrite the FKs of "the empty additive retirement envelope"). True of the 7 `remote_*` children in `state.rs`; for the 2 children in `context_retirement/record.rs:324, 347` the code guards against their presence. The count of nine is right (7 + 2; the third retirement table named in `historical_catalogs.json` is not created by current code). Conclusion (R-5: no new action, no rebuild) unaffected.

**DISC-9 (trivial). Plan File Map omissions.** Task 3 lists `schemas/v1/pull_request.schema.json` without "(new)"; it does not exist at HEAD (only `repository_setup.schema.json` is marked new). Task 5's file list omits `src/repository/transport/tests.rs` and `operation_registry_tests.rs` (E-16, E-17 below). Task 10 omits `tests/cli/recovery_tables.rs` (E-18). Task 13 omits `tests/cli/authoring_exits.rs` (E-19).

## 2. E-16 onwards: premise-sweep candidates (not already E-1 to E-15)

"Keeps intent" = whether the needed edit keeps every assertion's intent. Anything "no" stops for the owner (decision 15).

| # | Task | Site | What breaks | Needed edit | Keeps intent |
| --- | --- | --- | --- | --- | --- |
| E-16 | 5 | `src/repository/transport/tests.rs:21-45` `prepared()` builds a `PreparedSshAttempt { registration, source, endpoint, trust, approval, registry }` literal with no `..`. Callers: `tests.rs:71, 138, 268, 305, 816`, `callbacks.rs:252`. | Compile error once `PreparedSshAttempt` gains its source field (Part C). | Add the field with the `Configured` value. Not in the File Map. | yes |
| E-17 (conditional) | 5 | `src/repository/transport/operation_registry_tests.rs:67` calls `service.prepare_ssh(VerifySshTransportRequest {..}, &mut context)`; `:173, 174, 252, 352, 374` call `recheck_ssh(&prepared, &context)`. | Only if `prepare_ssh` gains a scope parameter. Avoidable by a second function for the setup source. | Mechanical extra argument. | yes |
| E-18 | 10, 13 | `tests/cli/recovery_tables.rs:41-60` const `C2` (and `FAMILIES`, line 83). `codes_of(verb)` (line ~468) panics "no audit row" for a verb in no list; a recovery-table row for `repo setup-cancel` (E-15) needs it. If D-4 instead puts the verb in the audit's C5 table, `assert_eq!(verbs.len(), 10)` (line 519) breaks. | Row for the new verb panics, or the C5 count. | Add `"repo setup-cancel"` to `C2` (it joins the "Repository and credentials" family, RFC heading at `docs/RFC/cli-contract.md:1452`), or change 10 to 11. E-15 covers rows, not these consts. | yes |
| E-19 | 13 | `tests/cli/authoring_exits.rs:295-312` `the_c3_table_and_reasons_account_for_every_code_of_the_audit` asserts 40 library codes and 42 with `CLI_ONLY`. `setup_pending` is answered by every C3 write verb (`document create/save/move/repair`, `ticket create/save/repair/slug-assign`, `folder create`). | If D-4 adds `setup_pending` to the audit's "C3 exit-coverage union" (`docs/research/wave-03-api-audit.md:1008`), counts become 41/43 and the code needs a `ROWS` entry (`assert_case_exists` needs an existing `#[test] fn` in `tests/cli/*.rs`, `authoring_exits.rs:283-290`) or a `REASONS` entry. E-15 names `exits.rs`, `collaboration_exits.rs` and `recovery_tables.rs` but not this file. | Counts plus one row or reason. | yes |
| E-20 (hazard, owner stop if breached) | 13 | `tests/cli/recovery_tables.rs:591-596`: `assert!(c5_reasons.len() <= 2)`. The RFC's "Codes with no golden" table holds exactly one C5 reason (`poll once`, `remote_unavailable`, `cli-contract.md:1675`). | One free slot. E-15's C5 rows (`ticket close`, `document promote`, ... `setup_pending`) must be goldens (for example a `ticket_close_setup_pending` prepare golden), not reasons. | Raising the cap would weaken the assertion: no. | no, if reasons are used |
| E-21 (conditional) | 11 | Desktop `Backend` implementations: `LibraryBackend` (`src/desktop/services/mod.rs:409`), `PollingBackend` (`src/desktop/test_support/polling.rs:352`), `DelayingBackend` (`test_support/delaying.rs:291`), `Recorder` (`src/desktop/services/scheduler_consent_tests.rs:150`). Read-count assertions: `scheduler_entity_tests.rs:93` (`remote_reads()` 6, then 12), `scheduler_extra_tests.rs:178` (2, 4) and its `PollCall::Resolve` counting (244-275), `poll_tests.rs:185-245` exact `PollCall` lists. | A required new trait method breaks the three doubles; a setup read that calls `resolve` or `publication_remotes` again changes the counts. | Give the setup read a default body on the trait and keep it off the recorded call kinds; then none of these tests is edited. The File Map says "its test double" (singular); there are four implementations. | yes |

Checked and expected to need no edit (each by reading the assertion): `command.rs` test `the_commands_whose_caller_must_first_be_shown_need_confirmation` (E-9 covers); `only_a_command_whose_work_no_journal_records_has_no_operation` (a confirmed local-row command is already `has_operation`); the `Command::ALL` loops in `outcome_tests.rs` (1384, 1416, 1552, 1668, 1871, 2311) derive expectations from the predicates, so they hold for `RepoSetupCancel` (needs_confirmation true, stops_at_safe_points false) if `recovery()` stays generic; `tests/cli/inventory.rs` `every_input_taking_verb_has_one_schema_and_one_example_and_every_other_takes_none` (the new verb is in `NO_INPUT`, E-9); `tests/mutation_contract.rs` `every_bound_command_has_a_golden` (line ~1586: the new command needs a mutation golden whose `"command"` is `repo setup-cancel`; Task 10 writes `repo_setup_cancel_preview`, so this passes at the end of Task 10, red inside it); the snapshot tests that hash `.git` (`cleanup_tests.rs:59`, `closure/observe_tests.rs:1138`, `closure/checkpoint_tests.rs:66`, `promotion/integrate_tests.rs:72`, `promotion/observe_tests.rs:1147`) build no setup record, so `manyhands-setup.lock` is never created (design Part D: every function reads the record first); the `#[cfg(test)]` `snapshot_file_hash` (`coordination.rs:253-268`, exempts only `.git/manyhands-operation.lock`) needs the second exemption in Task 6 as designed; `recovery_foundation_gate.rs:1075` marker list (the failed open stops before the pull-request-mode and later markers, so the new marker is not in it); `tests/mutation_replay.rs:400` (its `before` dump already holds `repository_setups` because `world()` opens the service; the "request" substring test is on object names only, `repository_setups` has none and no index may be named `*request*`); `recovery_scope_tests.rs:1042` and `context_retirement/execution_tests.rs:1244` (iterate `LocalStep::ALL`; the new steps are foreign to retirement rows and `is_authoritative` is derived from `is_checkpoint`; run them by name in Task 2 as a cheap check: `repository::recovery::scope_tests::` (`recovery.rs:4773-4775`) and `retirement_quality_q2_selected_rejects_other_step_authority`, `retirement_quality_q2_no_effect_refusal_rejects_other_step_authority`); `human_admin_tests.rs:111` (`every_preview_effect_has_a_distinct_phrase`: new phrases must be distinct and contain no `_`); `tests/cli/input_schemas.rs` `every_input_schema_uses_only_the_checked_keywords_and_is_closed` and `inventory.rs` (a shared `forge` input schema file would break `inventory.rs:491-496`, which allows only `commit_identity.schema.json` as a shared file: the design's inline `forge` is therefore required, not just chosen); `tests/cli/input_schemas.rs:165` `remote_names_are_required_and_only_selection_is_nullable` (unchanged if `name` stays required and `integration`/`forge` stay optional).

Source sites with an exhaustive match that are not named in the design (implementation edits, not test edits; each is a compile error until handled): `src/repository.rs:4296-4328` `follows_an_owned_write` (exhaustive `LocalStep` match: add `SetupRecorded | SetupCancelRecorded` to the `false` arm); `src/repository/recovery.rs:1926` `is_checkpoint` and `:1982` `continued` (exhaustive); `src/repository/mutation/bind/mod.rs:1052-1203` `recorded_effects` (match on `Some(Command::..)`, plus `RepoSetupCancel`); `src/repository/mutation/dto.rs:2540-2600` `MutationDataDto::from_stored` (match on `Command`, today `RepoEnable => None`, `RemoteSelect` falls to the remote arm, `PollOnce` three-way); `src/repository/mutation/mod.rs:638, 694, 752` (`Mutation::kind`, `confirmed`, `into_binding`); `src/bin/manyhands-cli/human_admin.rs:99-120` (`preview`) and `:165-204` (`effect_phrase`); `src/desktop/view/confirm.rs:649-780`.

## 3. U-1 (Step 3)

**Yes: a top-level `"type": ["object", "null"]` schema file reached by `$ref` passes all three checks, so Task 3 uses the shared `repository_setup.schema.json` file; no fallback to inlining `setup` is needed.**

Evidence:
- `tests/support/schema.rs`: `validate_schema` accepts `type` as a name or a non-empty array of known names (lines 105-117); `properties`, `required` and `additionalProperties` are allowed when `allows("object")` is true, which an array containing `"object"` satisfies (lines 98-103, 118-120); a property whose value is `{"$ref": ...}` must have no other keyword and the target must be a sibling file name (lines 82-92, 188-196). `check_node` (lines 198-262) follows `$ref` to the file, then tests the instance against the type array (`null` is allowed, `has_type` line 283-300), skips `properties`/`required` for a non-object instance (line 227 `if let Some(object) = instance.as_object()`), and applies them to an object.
- `tests/read_contract.rs:491-554` `closed_object_problems` (used by `every_published_object_is_closed_and_requires_all_its_properties`, line 557): `object_type` is true for an array containing `"object"` (503-507); such a node is then required to have `properties`, `additionalProperties: false` and `required` equal to the property names (513-539). A file that is closed that way passes. A property that is only `{"$ref": ...}` returns early (499-501), so `setup` in `repository_summary.schema.json` and `repository_inspection.schema.json` and `pull_request` inside `repository_setup.schema.json` pass. The same function already accepts an inline nullable object: `schemas/v1/repository_configuration.schema.json` `forge` is `"type": ["object", "null"]` with `properties`, closed and fully required (lines 33-57).
- `tests/mutation_contract.rs:1700-1743` `schema_differences` / `every_schema_is_reachable_from_a_published_root`: it only collects every `$ref` string (the `references` fn, line 1680) and walks files from `DATA_ROOTS` plus `OTHER_ROOTS`; it never reads a file's `type`. `repository_setup.schema.json` is reached from `repository_summary` and `repository_inspection` (both are `DATA_ROOTS`, lines 1663-1668), and `pull_request.schema.json` from `repository_setup.schema.json`.
- Other constraints a new file must meet (`tests/read_contract.rs:455-470`): `.schema.json` suffix, `$schema` equal to `https://json-schema.org/draft/2020-12/schema`, a string `title`. A nullable enum lists `null` in its `enum` (the checker applies `enum` regardless of `type`; `schema_enum` at `read_contract.rs:667-684` filters `null`), as `repository_configuration.schema.json` `integration` and `forge.kind` do. Do not make `repository_setup.schema.json` a `DATA_ROOT`: `every_data_root_describes_the_data_of_a_golden...` (line 1781) would demand a golden whose data it alone describes.

## 4. Test module paths per task (Step 2, second half)

Conventions confirmed: library unit tests are `--lib <module path>::<test>`; the CLI binary is `--bin manyhands-cli <module>::<test>` (module names from `src/bin/manyhands-cli.rs`, each `#[path = "manyhands-cli/X.rs"] mod X;`); the desktop is `--features desktop-test-support --bin manyhands desktop::...`; integration targets are `--test <target> [<module>::]<test> -- --exact`, where `tests/cli.rs` and `tests/mutation_replay.rs` declare each file as a `mod` (so `cli` tests are `admin::repo_list`, `repository::enable_an_existing_repository`, `remotes::select_then_clear`, `input_schemas::...`, `skills::...`, `inventory::...`, `recovery_tables::...`, `exits::...`; `mutation_replay` tests are `not_enabled::...`). A new test file for a module `X.rs` is declared at the end of `X.rs` as the codebase does everywhere (`results.rs:1126-1129`, `remote/poll.rs:2530-2533`):

```rust
#[cfg(test)]
#[path = "X_tests.rs"]
mod tests;
```

`#[path]` resolves next to `X.rs`, so `src/repository/setup/record.rs` with `record_tests.rs` beside it works (precedent: `remote/poll.rs` with `remote/poll_tests.rs`). `src/repository.rs` declares modules at lines 40-45 (`mod remote;`, `mod ticket_guard;`, ...): add `mod setup;` there; `src/repository/mutation/mod.rs` declares at lines 43-62: add `mod setup_gate;`.

| Task | Exact filters (existing: confirmed from the `mod` declaration) | New modules: where the declaration must go |
| --- | --- | --- |
| 1 | `--lib results::tests::result_codes_match_the_design_table_exactly`, `--lib results::tests::result_code_messages_are_fixed` (`results.rs:1126-1129`); `--lib repository::mutation::outcome::tests::` (`outcome.rs:1596-1598`, E-11's two no-action lists); `--test read_contract published_enumerations_list_exactly_the_registered_values` (the test that compares `envelope.code` and `request.result.code` with `ResultCode::ALL`, `read_contract.rs:896, 1142`); `--test mutation_contract every_result_code_has_a_mutation_golden_or_a_reason_none_has_it` and `every_result_code_has_a_golden_or_a_reason_no_fixture_can_produce_it` (all with `-- --exact`) | none |
| 2 | `--lib repository::discovery::tests::repository_setup_migration_adds_the_table_once` (inline `mod tests` at `discovery.rs:3050`, put the test beside `pull_request_mode_markers`, ~3597); E-1 `--test repository_enablement registry_creates_repository_and_discovery_metadata_schema`; E-14 `--lib repository::recovery::abandoned_row_tests::every_step_is_stored_under_its_own_name_and_read_back` (it is inside `mod abandoned_row_tests`, `recovery.rs:3897`, test at 4110, name list 4117-4144); unchanged: `--test mutation_replay the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`, `--test recovery_foundation_gate failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` | `mod setup;` in `src/repository.rs`; `src/repository/setup/mod.rs` with `pub(crate) mod record;`; `record.rs` ends with the `#[path = "record_tests.rs"] mod tests;` block, giving `--lib repository::setup::record::tests::setup_records_round_trip_and_enforce_their_rules`, `...::a_setup_record_survives_a_rebuild_and_goes_with_remove`, `...::a_setup_record_holds_no_remote_url_credential_or_identity` |
| 3 | `--lib repository::read::tests::inspect_and_list_report_a_pending_setup` (`read/mod.rs:519-520` `#[cfg(test)] mod tests;`, file `read/tests.rs`); `--bin manyhands-cli human::tests::repository_inspection_names_a_pending_setup` and `human::tests::repository_list_marks_a_pending_setup` (`human.rs:541-543`); E-6 `--test read_contract repo_list_matches_its_schema_and_golden`, `repo_inspect_matches_its_schema_and_golden` (lines 2488, 2519), `--test cli admin::repo_list`, `admin::repo_inspect` (macro `repository_case!`, `admin.rs:117-125`); `--test read_contract published_enumerations_list_exactly_the_registered_values`, `every_published_object_is_closed_and_requires_all_its_properties`; `--test mutation_contract every_schema_is_reachable_from_a_published_root` | none |
| 4 | `--lib repository::mutation::command::tests::every_command_has_the_setup_gate_of_the_design` (inline `mod tests`, `command.rs:703-704`, E-9's `the_commands_whose_caller_must_first_be_shown_need_confirmation` is `command.rs:870`); `--lib repository::read::tests::inspect_reads_a_complete_setup_as_none`; unchanged: `--test mutation_replay not_enabled::a_registered_repository_with_no_configuration_keeps_the_answers_it_had` (declared at `tests/mutation_replay.rs:2297-2298`) | `mod setup_gate;` in `mutation/mod.rs`; `setup_gate.rs` ends with `#[path = "setup_gate_tests.rs"] mod tests;` giving `--lib repository::mutation::setup_gate::tests::a_pending_setup_refuses_a_first_call_an_acceptance_and_a_prepare_but_not_a_reentry` and `...::a_complete_setup_refuses_nothing`; `setup/complete.rs` declared in `setup/mod.rs` |
| 5 | `--lib repository::transport::remote::tests::setup_mappings_name_only_the_setup_ref` (inline `mod tests`, `remote.rs:967-968`, next to `transport_delete_scope_is_exact_context_only` at 972); `--lib repository::transport::callbacks::tests::guard_create_and_guard_delete_decide_on_one_exact_update` (inline `mod tests`, `callbacks.rs:240-241`); `--lib repository::transport::operation::registry_tests::the_setup_scope_is_rechecked_against_the_record_and_the_endpoints` (`operation.rs:462-464`, file `operation_registry_tests.rs`, the file whose `prepare` helper calls `prepare_ssh`); `--lib repository::mutation::bind::lifecycle::tests::` for the `Recorder` double (`lifecycle.rs:1401-1403`) | `bind/host.rs` has no test module: add `#[cfg(test)] #[path = "host_tests.rs"] mod tests;` at its end, giving `--lib repository::mutation::bind::host::tests::host_read_takes_the_remote_of_design_part_c` and `...::host_observe_on_a_pending_setup_without_a_key_is_no_selected_key` |
| 6 | `--lib repository::setup::publish::tests::<name>` for all of `s1_to_s4_...`, `s2_to_s4_...`, `s5_...`, `s6_s7_s8_...`, `s9_...`, `s10_...`, `s11_...`, `s12_...`, `a_lost_commit_object_...`, and `the_setup_commit_changes_only_the_configuration_of_the_base` (File Map lists only `publish_tests.rs`; if the commit builder test is wanted under `commit.rs`, it is `repository::setup::commit::tests::`) | `setup/lock.rs`, `commit.rs`, `publish.rs` in `setup/mod.rs`; `publish.rs` ends with `#[path = "publish_tests.rs"] mod tests;`; `coordination.rs` already has an inline `mod tests` (347) if the lock test is placed there instead (`repository::coordination::tests::`) |
| 7 | `--lib repository::setup::intent::tests::enable_intent_follows_decision_nine`; `--lib repository::mutation::bind::repo::tests::the_enable_request_lifecycle_of_design_part_k`; `--test repository_enablement a_pull_request_enable_without_a_key_leaves_a_recorded_setup`, `enabling_a_clone_with_other_settings_is_invalid_input`, `a_direct_enable_with_a_remote_writes_it_in_the_initialization_commit`; `--test cli input_schemas::repo_create_refuses_the_enable_only_fields`; unchanged: `--test repository_enablement enable_born_main_writes_canonical_config_exclusion_and_single_file_commit` (1317), `enabling_a_repository_enabled_elsewhere_registers_and_indexes_it` (1412), `enable_wrong_checked_out_branch_preserves_repository_and_registry` (3245), `--test mutation_confirmation repo_enable_on_a_wrong_branch_is_wrong_branch` (895), `--test cli repository::enable_an_existing_repository` (345); registration tests of "Goldens" as in section 7 | `setup/intent.rs` with `#[path = "intent_tests.rs"] mod tests;`; `bind/repo.rs` has no test module: add `#[cfg(test)] #[path = "repo_tests.rs"] mod tests;` at its end |
| 8 | `--lib repository::setup::poll::tests::g1_to_g7_the_setup_poll_decides_on_the_base`, `...::activation_leaves_the_repository_as_a_direct_enable_would`, `...::a_change_on_another_remote_still_polls_the_current_remote`, `...::a_crash_after_the_fast_forward_is_finished_by_the_next_poll_or_the_gate`; `--lib repository::setup::complete::tests::a_setup_completes_when_the_local_primary_holds_it`; `--lib repository::mutation::bind::poll::tests::a_setup_poll_rerun_takes_the_setup_path` and `...::the_setup_poll_s_stops_are_final`; `--bin manyhands-cli polling::tests::setup_poll_lines` (`polling.rs:372`); unchanged: `--lib repository::mutation::dto::poll_retirement_stored_contract_is_closed_and_cannot_complete_a_not_reached_prefix` (a top-level `#[test]` in `dto.rs:2339`, no test module), `--lib repository::remote::poll::tests::task6_postfetch_advances_primary_and_replays_without_transport` (`poll.rs:2530-2533`, test at `poll_tests.rs:1357`) | `setup/poll.rs` and `complete.rs` each end with their `#[path = "poll_tests.rs"]` / `"complete_tests.rs"` `mod tests;`; `bind/poll.rs` has no test module: add `#[cfg(test)] #[path = "poll_tests.rs"] mod tests;` (a second `poll_tests.rs` name already exists under `remote/` and `desktop/`, which is fine, they are different directories) |
| 9 | `--lib repository::setup::change::tests::remote_select_takes_the_path_of_the_design`; `--lib repository::mutation::bind::remote::tests::the_select_request_lifecycle_of_design_part_k`; `--test repository_enablement turning_pull_request_mode_on_without_a_key_records_a_change`; unchanged: `--test repository_enablement publication_selection_commits_only_canonical_configuration_changes` (1829), `publication_remote_selection_keeps_mode_and_forge` (1881), `--test cli remotes::select_then_clear` (80), `--test cli input_schemas::remote_names_are_required_and_only_selection_is_nullable` (165) | `setup/change.rs` with `change_tests.rs`; `bind/remote.rs` has no test module: add `#[cfg(test)] #[path = "remote_tests.rs"] mod tests;` at its end |
| 10 | `--lib repository::setup::cancel::tests::cancel_outcomes` and `...::a_cancel_interrupted_between_steps_is_finished_by_the_same_request`; `--test repository_enablement cancelling_a_recorded_setup_unregisters_it`, `repo_remove_always_removes_a_pending_setup`; `--lib repository::registration_removal::tests::` (`registration_removal.rs:378`, file `registration_removal_tests.rs`, for the removal unit test); E-9: `--lib repository::mutation::command::tests::the_commands_whose_caller_must_first_be_shown_need_confirmation`, `--bin manyhands-cli catalog::tests::catalog_matches_every_command_and_delivering_cycle_without_duplicates` (`catalog.rs:1228`), `--test cli inventory::every_bound_mutation_is_a_catalogued_mutating_verb_less_the_two_exempt`, `inventory::the_catalog_and_the_rfc_taxonomies_name_the_same_verbs`, `inventory::every_input_taking_verb_has_one_schema_and_one_example_and_every_other_takes_none`, `inventory::every_verb_s_data_has_an_output_schema_and_its_goldens_match_it`, `skills::every_mutation_sits_in_one_autonomy_column`, the help goldens in `--test cli help::`; the conformance row: the conformance entry runs inside the `mutation_confirmation` target (`tests/mutation_confirmation/conformance.rs:268-274`, test `every_command_of_this_target_keeps_the_rules_of_the_boundary`) and the other three targets also call `run_part` with `bound_commands()` (`tests/mutation_replay/conformance.rs:21`, `tests/mutation_bridges/conformance.rs:253`, `tests/mutation_remote.rs:4903`), so a missing `HOMES` entry fails all four | `setup/cancel.rs` with `cancel_tests.rs`; `bind/setup_cancel.rs` declared `pub(crate) mod setup_cancel;` in `bind/mod.rs` (lines 37-62) |
| 11 | `--features desktop-test-support --bin manyhands desktop::services::scheduler::entity_tests::a_published_setup_on_another_remote_is_polled_on_the_setup_path` (`scheduler.rs:282-283` `mod entity_tests`, the existing `launch_polls_due_ssh_repos` is there, line 93); `desktop::model::repository_state::tests::a_setup_pending_repository_reads_waiting_not_needs_fixing` (`repository_state.rs:442-443`, file `repository_state_tests.rs`, which has its own submodules `causes`, `menus`, `pairs`, `scene`, `states`, `tips`); unchanged: `desktop::model::repository_state::tests::each_state_and_its_precedence` (107) and `...::menu_items_follow_the_state` (188), `desktop::services::scheduler::entity_tests::launch_polls_due_ssh_repos`, `desktop::services::scheduler::extra_tests::clock_event_reads_policies_and_remotes` (`scheduler.rs:286-287`, test 178) | none |
| 12 | `--test mutation_remote a_setup_branch_is_published_activated_changed_and_cancelled -- --exact` (harness=false target; the case is registered in `tests/mutation_remote.rs` `CASES`, after the entry at line 375 `a_pull_only_repo_sync_fast_forwards_then_refuses_local_commits`); then `--test mutation_contract the_fixture_directories_are_exactly_the_registered_cases`, both `every_result_code_has_a_*`, `every_schema_is_reachable_from_a_published_root`, `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`, `--test read_contract published_enumerations_list_exactly_the_registered_values` | none: the case is a `fn` plus a `("name", fn)` row in `CASES` (`tests/support/ssh_harness.rs:48` `type Case`) |

Existing helpers the Task 12 text names, all present: `clone_owned` (`tests/support/remote_world.rs:47`), `push_peer` (57), `World::plain_key` (153), the probe save/scan (`ssh_privacy.rs:43, 74`), and the harness child inherits `MANYHANDS_UPDATE_GOLDEN` (it removes only `GIT_*`, `SSH_*`, `GCM_*`, `MANYHANDS_SSH_TEST_HOME`: `ssh_harness.rs:197-207`).

## 5. Preview layout (Step 2)

Existing previews read: `schemas/v1/remote_resolve_history_preview.schema.json` and `repository_removal_preview.schema.json` (and the older `preview.schema.json`, which is `repo create` / `repo enable`'s direct preview).

Layout both new previews (`setup_preview.schema.json`, `setup_cancel_preview.schema.json`) must copy:
- Top level: `$schema` (draft 2020-12), `title` ("Manyhands JSON v1 ... preview"), `description`, `"type": "object"`, `"additionalProperties": false`, `required` = exactly `["confirmation_id", "expires_at", "command", "scope", "effects", "details"]`, in that order.
- `properties`: `confirmation_id` (string), `expires_at` (string), `command` (string), `scope` (`{"$ref": "scope.schema.json"}`), `effects` (`"type": "array"`, `items: {"$ref": "preview_effect.schema.json"}`), `details` (an inline closed object: `additionalProperties: false`, `required` equal to all its property names, each property typed and described; `["string","null"]` for optional values; a nested object such as `identity` is inline and closed too).
- No `oneOf`. Previews are told apart by `details` alone: the Rust enum `PreviewDetailsDto` is `#[serde(untagged)]` (`src/repository/mutation/dto.rs:246-261`, variants `Repository`, `RemoteRemoval`, `RepositoryRemoval`, `KeyDeletion`, `OperationAbandon`, `Identity`, `Host`, `Repair`, `Republish`, `ResolveHistory`, `ResolveCacheLoss(RemoteResolveHistoryPreviewDto)`, `Lifecycle(Box<..>)`), wrapped by `PreviewDto` (`dto.rs:833`, fields `confirmation_id`, `expires_at`, `command`, `scope`, `effects: Vec<PreviewEffectDto>`, `details`). Each variant is a struct (for example `RemoteResolveHistoryPreviewDto { root, marker_present }`, `dto.rs:431`). A golden's data must match exactly one `DATA_ROOTS` schema, which holds as long as each `details` is closed with a distinct required key set (the new setup preview's details is a closed superset of the direct preview's, so the direct schema rejects it and vice versa).
- The effect values: `preview_effect.schema.json` has `"enum"` of 37 names and a long `description` that names each effect; the Rust side is `PreviewEffect` in `dto.rs:160-198`. New values (setup preview: `setup_recorded`, `setup_branch_committed`, `setup_branch_published`; cancel preview: `setup_branch_deleted_remotely`, `setup_branch_deleted`, `setup_record_removed`, `repository_unregistered`; `repository_registered` exists) need the enum, the description (E-12), `human_admin.rs:165-204` `effect_phrase` (exhaustive; phrases distinct, no `_`), and `tests/read_contract.rs` `contract_enumerations` (E-7). Seven new values: note `branch_deleted` and `registration_removed` already exist with different meanings.
- Registration of a preview golden: (1) file in `schemas/v1/`; (2) its name in `DATA_ROOTS` (`tests/mutation_contract.rs:1603`, `[&str; 69]`, alphabetical-ish groups; `remote_resolve_history_preview.schema.json` is at 1663); (3) the golden name in `tests/support/mutation_golden.rs` `CASES` (line 21; the `remote_resolve_history_preview` entry is at 168); (4) the golden written by `assert_prepare_contract(name, &world, Some(SCHEMA), &prepare_envelope)` (`mutation_contract.rs:1912`, replaces `<confirmation>` and `<expires-at>`; `mutation_contract.rs:2036-2043` is the template, with the schema named by a const like `CLEAR_PREVIEW_SCHEMA` at line 1907); (5) a `DATA` row in `tests/cli/inventory.rs` listing the preview schema name without the suffix (`inventory.rs:213-215` for `remote resolve-history`); (6) the match arms: `human_admin.rs:99-120` and `src/desktop/view/confirm.rs:649-780` (E-12).
- The prepare envelope's `data` is the preview and its `effects` (the envelope's, not the preview's) are all `not_requested`, `commit_oid` null (see `tests/fixtures/mutation_v1/remote_resolve_history_preview.json`).

## 6. Golden helpers (Step 2)

All writers honour `MANYHANDS_UPDATE_GOLDEN=1` the same way: `tests/support/golden.rs:20` `UPDATE_VARIABLE`; `golden::update_requested(update, ci)` (lines 257-264) returns true only when the value is exactly `1` and panics if `CI` is set (so run with `CI` unset); on true the helper writes the rendered text (`golden::render`, sorted keys, two-space indent, final newline) to the fixture path instead of comparing. A name must already be registered or the helper refuses.

| Golden kind | Helper (file:line) | Registered in | Fixture directory | Where it is used this Cycle |
| --- | --- | --- | --- | --- |
| mutation_v1 (enable, select, poll, cancel, `setup_pending`, previews) | `mutation_golden::assert_contract(&ContractCase, &envelope)` (`tests/support/mutation_golden.rs:289`; update branch 315-322) and `mutation_golden::assert_contract_at(name, root, data_schema, replaced, &envelope)` (365; adds `<root>`, `<commit>`, `<operation>`, local-commit placeholders and ends in `assert_contract`). Per-target wrappers: `tests/mutation_contract.rs` `assert_contract_excluding` (156), `assert_prepare_contract` (1912), `World::assert_golden` / `assert_golden_with_data` (`tests/support/mutation.rs:454, 459`); `tests/mutation_remote.rs` `assert_golden(w, name, data_schema, &envelope)` (844, adds `<authority>` and fingerprint placeholders, checks port); `tests/support/lifecycle_world.rs:371` `golden_at`; `tests/mutation_bridges/folder.rs:171`, `host.rs:89` | `mutation_golden::CASES` (line 21) | `tests/fixtures/mutation_v1` | Tasks 7-10 write from `tests/repository_enablement.rs` (which today uses no golden helper: it already has `mod support;`, line 22, so call `support::mutation_golden::assert_contract_at` directly) and `tests/mutation_contract.rs`; Task 12 from `tests/mutation_remote.rs` `assert_golden` |
| cli_v1 (JSON and text) | `cli::assert_cli_golden(name, &value, &placeholders)` and `cli::assert_cli_text_golden` (`tests/support/cli.rs:~586, ~595`, both through `assert_fixture`, 602-625, update branch at 614-622); per-file wrappers: `tests/cli/admin.rs:52` `golden`, `tests/cli/mutation.rs:107` `golden`, `tests/cli/lifecycle.rs:212`, `c4_support.rs:49`, ... | `CLI_CASES`, `CLI_TEXT_CASES` (`tests/support/cli.rs`; `cli_repo_list`, `cli_repo_inspect` at 290-291) | `tests/fixtures/cli_v1` | E-6's `cli_repo_list.json` and `cli_repo_inspect.json` are written by `--test cli admin::repo_list` and `admin::repo_inspect` (macro at `admin.rs:117`, name `concat!("cli_", stringify!($test))`) |
| read_v1 | `golden::assert_contract` (`tests/support/golden.rs:118`) / `assert_mutation_contract` (132) → `assert_case` → `assert_golden(name, value, placeholders)` (230; update branch 232-240) | `golden::CASES` (line 26; `repo_inspect` 74, `repo_list` 75) | `tests/fixtures/read_v1` | E-6's `repo_inspect.json` and `repo_list.json`, from `tests/read_contract.rs` `repo_list_matches_its_schema_and_golden` (2488) and `repo_inspect_matches_its_schema_and_golden` (2519) |

Registration checks that read the fixture directories (not produced by the writing test): `tests/mutation_contract.rs` `the_fixture_directories_are_exactly_the_registered_cases` (1509; also checks each fixture is canonical text and matches the published schema), `every_result_code_has_a_mutation_golden_or_a_reason_none_has_it` (1876), `every_result_code_has_a_golden_or_a_reason_no_fixture_can_produce_it` (1890; `code_differences` 1822-1840 reports a code that is both in a `CODES_` table and in a golden, which is why E-4 removes each entry in the task that writes its golden), `every_bound_command_has_a_golden` (1586), `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root` (1781), `every_schema_is_reachable_from_a_published_root` (1737); `tests/read_contract.rs` `golden_fixtures_are_exactly_the_registered_cases` (634). `CODES_NO_FIXTURE_CAN_PRODUCE` is `[(ResultCode, &str); 3]` at line 1790 and `CODES_NO_MUTATION_GOLDEN_HAS` is `[...; 7]` at line 1842: both array lengths change with E-3 and E-4.

## 7. K5: how `repo remove` checks accepted requests today (Step 2)

`src/repository/registration_removal.rs`:
- Registry-only path (`repository.is_none()`, the root is gone): line 342-344 calls `require_registry_only_removal(&db, &root, operation_id)` (defined 141-238). It first runs `local_sync::record::audit_unfinished` (146); returns early if an automatic poll is active so that the begin path yields it (148-158); rejects an `operation_id` that belongs to another operation (`OperationMismatch`, 160-175); rejects an unrecognised local journal step or malformed row (`RecoveryRequired`, 177-212, using `LocalStep::ALL`, so the two new step names must be in `ALL`, which Task 2 does); then one `pending` query (213-229) that is true for: an operation row of the root that is not `completed` (or is `comment_registration_pending`) other than its own `remove_registration` row (216-218), a non-final `remote_operation_records` row, an unfinished `document_promotions` or `ticket_closures` row (221-222 region), **any `request_records` row with `scope_key = ?1 AND state = 'accepted'` except its own `repo remove` request** (223-226: `command='repo remove' AND target='' AND` an own `request_operations` row of family `local` with this operation ID), and more than one accepted request on its own operation (227-228). `pending` true returns `RepositoryErrorKind::RecoveryRequired` "unfinished repository work must be recovered before removing this registration" (230-237).
- Live path (the root exists): no `request_records` check at all. After `check_registration` (339-341) it calls `begin_or_reconcile_operation` (345-352), whose `in_the_way` (see DISC-5) refuses for another pending local row or an active remote reservation and yields an active automatic poll; then, in one `Immediate` transaction (360), it deletes `operation_records WHERE root_path = ?1` (363-364) and `repositories WHERE root_path = ?1` (365-366) and commits (368); dependents cascade (`foreign_keys` is ON, `discovery.rs:2525, 4083`; `operation_records.repository_id` is `ON DELETE SET NULL`, `recovery.rs:654`). `request_records` rows are untouched (they key on `scope_key` text), which is why design K5 adds `finish_accepted` inside this transaction on both paths.
- `bind/repo_remove.rs` adds nothing: `call` (139) calls `remove_registration_checked` (152); its `accepted` values are the request's own.

## 8. Reference table (Step 2): every cited name, by design part

"E" = existing, "N" = new in this Cycle. Line numbers are at HEAD. "yes" means the design's claim holds at HEAD; differences are in section 1 (DISC-n).

### Conventions and Baseline Facts 1 to 20

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| Fact 1 `read_configuration_for` | E | `src/repository.rs:11869` (reads `<root>/.manyhands/config.toml` from the working tree, 11873-11885) | yes |
| Fact 1 `reconcile_registration` (free fn) | E | `src/repository.rs:13440`; method `:4015` | DISC-4 |
| Fact 1 `committed_configuration_blob_oid` | E | `src/repository.rs:13502` (reads HEAD's tree, errors if the file is absent) | yes |
| Fact 1 `remote::state::configure` | E | `src/repository/remote/state.rs:2697` | yes (called only from the method, for `SetPublicationRemote`) |
| Fact 2 `prepare_ssh` | E | `src/repository/transport/operation.rs:218` (remote from `publication_remote(root)` 229, key read 231-236; `ConfigurationInvalid` for a missing/invalid file via `publication_remote` 384-392) | yes |
| Fact 2 `recheck_ssh` | E | `operation.rs:260` (re-reads the file 280-281; called before and after each transfer, `remote.rs:~512, ~717`) | yes |
| Fact 2 `plan_matches_configuration` | E | `src/repository/transport/remote.rs:870` | yes |
| Fact 2 `exact_mappings` | E | `remote.rs:847` (primary and context refs only) | yes |
| Fact 2 `transfer_inner` | E | `remote.rs:467` (scope `Option<(&RemoteRefPlan, &SynchronizationTarget)>`; refuses push on a fetch connection and delete on any other, 505-518; tracking written only for `FetchTracking` with a scope or poll, 620-690) | yes |
| Fact 2 `delete_exact`, `observe_delete`, `DeleteObservation` | E | `remote.rs:269`, `:308`, `:34-38` (`Deleted`, `StillPresent(Oid)`, `Changed(Oid)`) | yes |
| Fact 2 `guard_delete` (`callbacks.rs`) | E | `src/repository/transport/callbacks.rs:222` | yes (inline closure: DISC-3) |
| Fact 2 `promotion/run.rs` second connection for `observe_delete` | E | `src/repository/promotion/run.rs:1537` | yes |
| Fact 3 `RemoteRefPlan::from_configuration` | E | `src/repository/remote/refs.rs:37` (primary, `manyhands/document/*`, `manyhands/ticket/*`, 58-60) | yes |
| Fact 3 `classify_advertised_ref` | E | `refs.rs:119` | yes |
| Fact 3 "No code uses `refs/manyhands/`" | E | grep over `src`: no hit | yes |
| Fact 4 `REMOTE_OPERATION_SCHEMA` | E | `remote/state.rs:1114` (`action` CHECK 1122, `remote_active_reservation` 1153) | yes |
| Fact 4 `remote_polling_state` `endpoint_digest` | E | `state.rs:50-59` | yes |
| Fact 4 nine child tables | E | 7 in `state.rs` (451, 928, 1160, 1175, 1201, 1214, 1291) + 2 in `context_retirement/record.rs:324, 347` | yes (DISC-8) |
| Fact 4 `migrate_synchronization` | E | `state.rs:1782` | yes (DISC-8) |
| Fact 5 remote sites reading the file | E | `observation.rs:466` `observation_plan`, `poll.rs`, `sync.rs`, `resolve.rs`, `conflict_read.rs` | DISC-7 |
| Fact 5 `poll_configuration_matches` | E | `remote/poll.rs:2344` | yes |
| Fact 5 `synchronization_configuration_matches_reading` | E | `remote/sync.rs:4388` | yes |
| Fact 6 poll binding `prepare` / `run` / `rerun` / `call` | E | `src/repository/mutation/bind/poll.rs:586`, `:611`, `:642`, `:375` (`invalid_configuration` for missing/invalid 595-598, `publication_remote_required` 600-602, family `Remote` 606; `rerun` calls with `restart` true) | yes |
| Fact 6 `poll_probed` | E | `remote/poll.rs:1535` | yes |
| Fact 6 `poll_publication_remote` | E | `remote/poll.rs:1493` (desktop `Backend::poll_automatic`, `desktop/services/mod.rs:333-339`, calls it) | yes |
| Fact 6 `PollMutationDto` (exactly five keys) | E | `src/repository/mutation/dto.rs:2095`, `from_stored` 2105 (`len() != 5`) | yes |
| Fact 6 "twelve goldens hold it" | E | 6 in `tests/fixtures/mutation_v1` (`poll_once.json`, `_cancelled`, `_discovery_pending`, `_interrupted`, `_key_rejected_reentered`, `_repository_inaccessible`) + 6 in `cli_v1` (`poll_once_closed_ticket`, `_dirty`, `_diverged`, `_fast_forward_and_materialize`, `_malformed`, `_remotely_deleted`) | yes |
| Fact 7 `artifacts::fast_forward` | E | `src/repository/integration/artifacts.rs:178` (`pub(in super::super)`; locks ref, old OID and HEAD check, safe checkout with `overwrite_ignored(false)`, moves ref) | yes |
| Fact 7 `require_clean_target` | E | `remote/sync.rs:685` (untracked counts: `include_untracked(true)` at 774) | yes |
| Fact 8 `Host::read` | E | `bind/host.rs:214` (remote from the file; `InvalidConfiguration` 233; `PublicationRemoteRequired` 229-230; authority must be a remote's host via `serves`, 252-263) | yes |
| Fact 8 `observe_host`, `verify_ssh_transport` | E | `bind/host.rs:904`, `:934`; `transport/operation.rs:15` | yes |
| Fact 8 `Session::verify_transport`, `CallSession`, `NoSession`, `Recorder` | E | `progress.rs:311`, `:331` (impl 464), `:499` (impl 501, 568); `bind/lifecycle_tests.rs:416, 516` | yes |
| Fact 8 `VerifySshTransportRequest` and its literals | E | `transport/mod.rs:99`; 39 literals | yes (about 40) |
| Fact 8 `read_host_trust` | E | `transport/trust.rs:106` | yes |
| Fact 9 `publication_remote_eligible_for` | E | `src/repository.rs:12580` (parses endpoints; no connection) | yes |
| Fact 9 fixture `ssh://git@example.invalid/team/repo.git` | E | `tests/fixtures/cli_input_v1/remote_add.json` | yes |
| Fact 9 `NoSelectedKey` before any connection | E | `operation.rs:231-236` | yes |
| Fact 9 libgit2 1.9.7 | E | `Cargo.lock:4426-4427` (`libgit2-sys 0.18.8+1.9.7`); "advertises no refs for an empty remote" cannot be verified by reading | yes (the version) |
| Fact 10 `enable_recorded` | E | `src/repository.rs:6310` (`wrong_branch` 6340-6375, worktree 6377-6400, config checks 6402+) | yes |
| Fact 10 `begin_lifecycle` | E | `src/repository.rs:4060` (lease + local row) | yes |
| Fact 10 `RepoBinding` pieces: `ended_without_checkpoint` `SaysNothing`; `written()`; `fields`; `reenter`; `rerun`; `Confirmed::row`; `own_commit`; `initialization_commit` | E | `bind/repo.rs:1167-1168`, `:453`, `:1183`, `:1268`, `:1281`, `:1287`, `:327`, `:256` | yes |
| Fact 10 `Row::Unfinished` | E | `src/repository/mutation/confirm.rs:167` (handled 243-256) | yes |
| Fact 10 `evidence.rs still_reachable` | E | `src/repository/mutation/evidence.rs:548` | yes |
| Fact 10 `EnableRepositoryRequest` / `RepoEnableInput` | E | `src/repository.rs:672`; `bind/repo.rs:91` | DISC-6 |
| Fact 11 `set_publication_remote` | E | `src/repository.rs:5647` (commit "Configure Manyhands publication remote" in `set_publication_remote_recorded` 5680) | yes |
| Fact 11 `RemoteSelectBinding` `ended_without_checkpoint` `DidNothingToASetting`; `Reentry::of`; `moved`; `RemoteSelectInput` | E | `bind/remote.rs:592-593`; `reenter.rs:36-56`; `bind/remote.rs:606`; `bind/remote.rs:412` (16 literals) | yes |
| Fact 11 `LocalStep::is_checkpoint` (exhaustive) | E | `src/repository/recovery.rs:1926` | yes |
| Fact 11 CLI parser `remote_name` (accepts only `name`) | E | `src/bin/manyhands-cli/admin.rs:381-420` (`object.only(&["name"])` 393); callers `remote_remove` 344, `remote_select` 363 | yes |
| Fact 12 `execute_binding`, `settle_target`, `first_call`, `insert_request_admitted`, `write_records`, `prepare_call` | E | `mutation/execute.rs:73, 80`; `execute.rs:265`; `records.rs:942, 909`; `confirm.rs:318` | yes (`consent` runs before `prepare` in `first_call`, `execute.rs:266-272`) |
| Fact 12 `command.rs` const fns exhaustive | E | `command.rs:101, 198, 251, 305, 365, 441, 497, 566, 626` | yes |
| Fact 12 `Standing::Owed` | E | `bind/mod.rs:643-665`; `settle.rs` | yes |
| Fact 12 `request cancel` asks nothing of a command that does not stop at safe points | E | `command.rs:365-440` (`RepoEnable`, `RemoteSelect` false) | yes |
| Fact 13 `operation_records.repository_id ON DELETE SET NULL`; `completed_step` text; `local_steps!` | E | `recovery.rs:654, 662`, `:1822-1852` | yes |
| Fact 14 `registration_removal.rs` | E | see section 7 | yes (DISC-5) |
| Fact 15 `a_registered_repository_with_no_configuration_keeps_the_answers_it_had` | E | `tests/mutation_replay/not_enabled.rs:272` | yes |
| Fact 15 `read_primary` | E | `src/repository/discovery.rs:959` (`Missing`/`Invalid` gives `Unverified`, 989-991) | yes |
| Fact 15 `never_enabled` | E | `bind/mod.rs:152` (false when HEAD or any local branch tip holds the file) | yes |
| Fact 16 `migrate_registry`, `migrate_pull_request_mode`, marker | E | `discovery.rs:2558`, `:2809`, `:2761` (`migrate_pull_request_mode(connection)` is the last statement, 2759) | yes |
| Fact 16 `index rebuild` never deletes; corrupt registry set aside | E | only `DELETE FROM repositories` is `registration_removal.rs:366`; `repository.rs:2252` `replace_corrupt_cache` | yes |
| Fact 17 `repository_lease`, `acquire_exclusive`, `snapshot_file_hash` | E | `coordination.rs:135`, `:284` (private), `:253` (`#[cfg(test)]`); `LEASE_TIMEOUT` 250 ms `:14`; lock file `manyhands-operation.lock` `:216`; `fs4` `Cargo.toml:19` | yes |
| Fact 18 scheduler `registration.rs` `read` | E | `src/desktop/services/scheduler/registration.rs:127` (`Read::{Polled, NotPolled, Failed}` 28-46; `ssh_publication` 54) | yes |
| Fact 18 `needs_fixing` | E | `src/desktop/model/repository_state.rs:251` (any non-`Valid` state, missing included) | yes |
| Fact 18 Add repository sends `RepoEnableInput` with no remote | E | `desktop/services/requests/build.rs:66-76` | yes |
| Fact 19 `preview` schemas, `PreviewDetailsDto` | E | section 5 | yes |
| Fact 19 schema checker rules | E | `tests/support/schema.rs` (type name or array 105-117; `$ref` alone to a sibling 82-92, 188-196; no `oneOf`: any other keyword is "unsupported keyword" 164) | yes |
| Fact 19 `RECOVERY_ACTIONS` (13, `repo.inspect`, no poll action) | E | `src/results_tests.rs:1567`; `results.rs:~400-420` | yes |
| Fact 19 `CASES`, `DATA_ROOTS`, `CODES_*` | E | `tests/support/mutation_golden.rs:21`; `mutation_contract.rs:1603, 1790, 1842` | yes |
| Fact 20 `forge::recognise`, `default_integration`, `pull_request_link`, `PullRequestLink` | E | `src/forge.rs:78`, `:102`, `:146`, `:114` (`forge: Option<ForgeKind>`, `url: Option<String>`) | yes |
| Fact 20 `RepositoryConfig::integration()` / `forge()` | E | `src/canonical.rs:134`, `:139` (struct 76; `Integration` 91, `ForgeKind` 106, `ForgeSetting` 125) | yes |
| Fact 20 `local_only_commits` (private) | E | `remote/sync.rs:4097` (also private `commits_missing_from` 4105) | yes |
| Fact 20 `primary_has_local_commits` with `PrimaryLocalCommitsDto` | E | `results.rs:652`; `mutation/dto.rs:1751` | yes |

### Part A

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `repository_setups` table, marker `wave_06_repository_setup_v1`, migration | N | after `migrate_pull_request_mode` at `discovery.rs:2759-2809` | n/a |
| `tests/mutation_replay.rs` "no schema object name contains request" | E | `tests/mutation_replay.rs:442` (object names of `dump`, line 355; columns are not names) | yes |
| 40-hex OID `CHECK` "the other tables use" | E | e.g. `remote/state.rs` OID columns | yes (not re-read) |
| `src/repository/setup/` (`record.rs` and `read`, `insert`, `record_commit`, `record_published`, `delete`) | N | none exists at HEAD | n/a |
| `repo remove` drops the record by cascade | E | `registration_removal.rs:366`; FK `ON DELETE CASCADE` as `discovery.rs:2620, 2655, 2663, 2674` | yes |

### Part B

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `Command::restores_index` ("exhaustive like") | E | `command.rs:251` | yes |
| `SetupGate`, `Command::setup_gate` | N | `command.rs` (new) | n/a (the table partitions all 46 commands plus `repo setup-cancel`: 5 + 21 + 21 = 47) |
| `setup_gate::check`, `check_in`, `is_complete`, `finish`, `finish_held` | N | `mutation/setup_gate.rs`, `setup/complete.rs` | n/a |
| `first_call`, `prepare_call`, `insert_request_admitted`, one-transaction path | E | `execute.rs:265`; `confirm.rs:318`; `records.rs:942`; `execute.rs:386` `written_with_its_record` | yes |
| `require_unreserved` in both acceptance paths | E | `records.rs:935, 977` | yes |
| `begin_lifecycle`, `in_the_way` | E | `repository.rs:4060`; `recovery.rs:2839` | yes |
| `reconcile_registration` under remote selection's lifecycle check | E | `repository.rs:4015` | DISC-4 |
| `poll_probed` end (`finish`) | E | `remote/poll.rs:1535` | yes |
| `ResultCode` and `setup_pending` (`Blocked`, exit 3, `repo.inspect`) | N | `results.rs:543` (`result_codes!`) | n/a |
| `read` admin `inspect_repository`, `list_repositories`; `repositories.config_blob_oid` | E | `read/admin.rs:397`, `:308`; column `repositories.config_blob_oid TEXT` nullable (`discovery.rs:2566-2579`) | yes |

### Part C

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `PreparedSshAttempt` | E | `transport/mod.rs:60` (literals: `operation.rs:250`, `transport/tests.rs:23`) | yes (E-16) |
| `SetupScope`, `Configured` / `Setup` source | N | `transport/mod.rs` | n/a |
| `endpoint_digest` (today a method of `ObservationConfiguration`) | E | `remote/observation.rs:115` (blake3 over fetch and push `connection_url`) | yes; the free fn `endpoint_digest_of` is N |
| `verify_setup_transport`, `with_setup_remote` | N | `transport/operation.rs` | n/a |
| `fetch_setup_base`, `push_setup_branch`, `delete_setup_branch`, `observe_setup_branch` | N | `transport/remote.rs` | n/a |
| `write_tracking_tip` (compare-and-swap) | E | `remote.rs:913` (`reference_matching`) | yes |
| `guard_create` | N | none | DISC-3 |
| `push_negotiation` | E | libgit2 callback used at `callbacks.rs:228` | yes |
| `SshTransportError`, `SshTransportErrorKind::EndpointChanged` | E | `transport/error.rs:27` (kind), struct with `root, remote_name, direction, selected_key_id, authority, kind` (`operation.rs:76-83`); `bind/host.rs:~955-985` matches `error.kind` | yes |
| `Host::read` new behaviour, `Session::verify_setup_transport` | N | `bind/host.rs`, `progress.rs` | n/a |

### Part D

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `manyhands-setup.lock`, `coordination::setup_lock`, `SetupLock` | N | beside `repository_lease` (`coordination.rs:135`) | n/a |
| `snapshot_file_hash` exemption | E | `coordination.rs:253-268` | yes |
| `enable_with_setup`, `change_setup`, `continue_setup`, `SetupChannel`, `setup::commit::build`, `setup::publish` | N | `repository.rs`, `setup/` | n/a |
| `local_only_commits` made visible | E | `remote/sync.rs:4097` | yes |
| Session methods `publish_setup`, `poll_setup`, `cancel_setup`, `verify_setup_transport` | N | `progress.rs:215` trait | n/a |
| `remote_primary_missing`, `primary_has_local_commits`, `remote_already_set_up`, `external_change`, `recovery_required`, `busy` | N, E, N, E, E, E | `results.rs` | S11 mapping: DISC-1 |

### Part E

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `schemas/input/v1/repo_enable.schema.json` (`primary_branch`, `identity` `$ref`, `ticket_slug_prefix`) | E | `schemas/input/v1/repo_enable.schema.json` | yes |
| CLI `repository_input` (shared with `repo create`) | E | `bin/manyhands-cli/admin.rs:185-203` (`only(&["primary_branch","identity","ticket_slug_prefix"])`) | yes |
| `RepoEnableInput`: new fields `publication_remote`, `integration`, `forge` | E (struct) / N (fields) | `bind/repo.rs:91` | yes |
| `fields` digest ("an absent field still writes a tag") | E | `bind/repo.rs:1183-1210` | yes |
| `canonical_configuration` | E | `repository.rs:12001` (builds `RepositoryConfig { primary_branch, publication_remote: None, unknown }`) | yes |
| `serialize_repository_config` | E | `canonical.rs:1384` | yes |
| `register_setup_pending`, `enable_direct_with`, `SetupRecorded`, `setup_preview.schema.json`, `setup_mutation.schema.json` | N | various | n/a |
| `invalid_remote`, `invalid_input`, `wrong_branch`, `worktree_not_clean`, `worktree_conflicted`, `identity_required` | E | `results.rs` | yes |
| `repo create` still refuses new fields | E | `repository_input` split | n/a |

### Part F

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `schemas/input/v1/remote_select.schema.json` (`name` required, nullable) | E | file read | yes |
| `RemoteSelectInput` (16 literals) | E | `bind/remote.rs:412` | yes |
| `set_publication_remote_with`, `change_setup`, nullable-object helper | N | `repository.rs`, `input.rs` | n/a |
| `InputObject::nullable_string`, `optional_object` | E | `bin/manyhands-cli/input.rs:142, 207` | yes |
| `invalid_configuration` for clearing a pull-request remote (Cycle 01) | E | serializer, `canonical.rs:1384` | yes |
| `ticket close`, `document promote`, `remote select`, `remote remove`, `repo enable` gated | E | `command.rs` names | yes |

### Part G

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `bind/poll_retirement.rs` `prepare` | E | `bind/poll_retirement.rs:196, 231` (the `#[cfg(test)]` lines; `prepare` follows) | yes |
| `poll_setup`, `SetupPollDto`, `setup_poll.schema.json` | N | various | n/a |
| `record_poll_attempt` ("beside") | E | `remote/state.rs:3133` (`pub(super)`) | yes |
| `RemoteOutcomeCategory` (`completed` and the transport stops) | E | `remote/state.rs:2287-2297` | yes |
| `artifacts::fast_forward` | E | `artifacts.rs:178` | yes |
| `remote_polling_state` | E | `state.rs:50` | yes |
| `PollMutationDto` unchanged | E | `dto.rs:2095` | yes |

### Part H

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `remote resolve-history` registered "as ... was" (`delivered_by` "C5") | E | `tests/cli/inventory.rs:85, 213`; `catalog_tests.rs:111`; `tests/support/conformance.rs:105`; `skills.rs:453`; `mutation_confirmation/conformance.rs:162` | yes |
| `Command::RepoSetupCancel` arms (`has_operation`, `needs_confirmation`, `stops_at_safe_points`, `stage`) | N | `command.rs` | n/a |
| `LocalStep::SetupCancelRecorded` | N | `recovery.rs:1852` list | n/a |
| `push_rejected` | E | `results.rs` | yes |
| `DeleteObservation::{Deleted, Changed, StillPresent}` | E | `remote.rs:34-38` | yes |
| `in_the_way` (cancel step 4) | E | `recovery.rs:2839` | yes |
| `RepositoryOperation` for the cancel's row | N, unspecified | | DISC-2 |
| `setup_cancel.schema.json`, `setup_cancel_preview.schema.json` | N | `schemas/v1/` | n/a (layout, section 5) |

### Part I

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `pull_request.schema.json`, `PullRequestDto` | N | `schemas/v1/` (absent), `read/dto.rs` | n/a (DISC-9) |
| `repository_setup.schema.json` | N | `schemas/v1/` | n/a (U-1: shared file OK) |
| `every_published_object_is_closed_and_requires_all_its_properties` | E | `tests/read_contract.rs:557` | yes |
| `every_schema_is_reachable_from_a_published_root` | E | `tests/mutation_contract.rs:1737` | yes |
| `repository_summary.schema.json`, `repository_inspection.schema.json`, `repository_list.schema.json` (closed, all properties required) | E | `schemas/v1/` | yes |
| `RepositorySummaryDto`, `RepositoryInspectionDto` (6 literals each) | E | `read/dto.rs:119, 177`; literals: `read/admin.rs:344, 448`, `human_tests.rs:576, 608, 639, 983, 1020`, `desktop/model/navigation_tests.rs:16`, `repository_state_tests/scene.rs:137`, `view/shell/title_tests.rs:19`, `view/workspace/add_sheet_tests/share.rs:90`, `view/add_repository_tests/support.rs:68` | yes (summary: 6 = admin + human ×2 + 3 desktop; inspection: 6 = admin + human ×3 + 2 desktop) |
| `forge::pull_request_link(forge::recognise(..))` | E | `forge.rs:146, 78` | yes |
| `repo inspect` / `repo list` human output | E | `bin/manyhands-cli/human.rs:64, 36` | yes |
| `MutationDataDto` (tagged `from_stored` arm) | E | `dto.rs:996-1022` enum; `from_stored` 2540-2600; `primary_local_commits` precedent tag at 1714-1751 | yes |
| `setup_mutation.schema.json`, `setup_poll`, `setup_cancel` data | N | | n/a |
| `setup_pending`, `remote_primary_missing`, `remote_already_set_up` | N | `results.rs`, E-2, E-3, E-11 | n/a |

### Part J

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| scheduler `registration.rs` `read` / `Read` | E | `registration.rs:127` / `28-46` | yes |
| `Read::SetupPending` | N | | n/a |
| `needs_fixing` | E | `repository_state.rs:251` | yes |
| `entity.rs` | E | `src/desktop/services/scheduler/entity.rs` | yes |
| Add repository unchanged | E | `build.rs:66-76` | yes |

### Part K and Rulings

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `LocalStep::SetupRecorded` `is_checkpoint` true | N | `recovery.rs:1926` | n/a; also `continued` 1982, `repository.rs:4300-4328` exhaustive matches |
| `Reentry::of`, `AsRecorded`, `NewUnlessMoved` | E | `reenter.rs:23-56`, handled 142-143 | yes |
| `Confirmed::row` / `reenter` / `rerun` of `RepoBinding` | E | `bind/repo.rs:1287, 1268, 1281` | yes |
| `RemoteSelectBinding` `reenter` / `rerun` / `moved` | E | `bind/remote.rs:708, 723, 606` | yes |
| `records::finish_accepted`, `stored_result(&Answer)` | N / E | `records.rs` / `settle.rs:80` (`pub(super)`) | yes |
| `settle.rs`, `local_control::effects` (stored envelope answered) | E | `settle.rs`, `local_control.rs:220` | yes |
| `operation resume` → `original_request_required` | E | `bind/resume.rs` | yes |
| `request cancel` asks nothing | E | `command.rs:365-440` | yes |
| `repo remove` finishing on both paths | E | `registration_removal.rs:141-238`, `:345-368` | DISC-5 |
| `tests/repository_enablement.rs` (L9, P6, P7) | E | exists; no golden helper today | yes |

### Required Outcomes, Testing, Named Edits, Document Amendments

| Name | E/N | file:line | Agrees |
| --- | --- | --- | --- |
| `src/repository/setup/record_tests.rs`, `publish_tests.rs`, `intent_tests.rs`, `poll_tests.rs`, `complete_tests.rs`, `cancel_tests.rs`, `change_tests.rs`; `mutation/setup_gate_tests.rs` | N | none exists | n/a (declaration points: section 4) |
| `src/repository/remote/poll_tests.rs` pattern | E | exists; `#[path] mod tests` at `poll.rs:2530-2533` | yes |
| `bind/host.rs` unit test | E (file) / N (test module) | no `mod tests` in `host.rs`, `repo.rs`, `remote.rs`, `poll.rs` | DISC-9 area: add the test modules |
| `tests/mutation_remote.rs` case table | E | `CASES` at `mutation_remote.rs:114-`, entry `a_pull_only_repo_sync_...` at 375 | yes |
| E-1 `registry_creates_repository_and_discovery_metadata_schema` | E | `tests/repository_enablement.rs:475` (sorted table list from 595) | yes |
| E-2 `DESIGN_CODES` (71), `result_code_messages_are_fixed` | E | `src/results_tests.rs:34` (`[...; 71]`), `:155` | yes |
| E-3 / E-4 `CODES_NO_MUTATION_GOLDEN_HAS`, `CODES_NO_FIXTURE_CAN_PRODUCE` | E | `mutation_contract.rs:1842, 1790` | yes |
| E-5 six + six DTO literals, five desktop helpers | E | above | yes |
| E-6 four goldens | E | `tests/fixtures/read_v1/repo_inspect.json`, `repo_list.json`; `tests/fixtures/cli_v1/cli_repo_inspect.json`, `cli_repo_list.json` | yes |
| E-7 `contract_enumerations` | E | `tests/read_contract.rs:724` | yes |
| E-8 eight `RepoEnableInput` + sixteen `RemoteSelectInput` literals | E | DISC-6 and `bind/remote.rs:412` literal list (16: `cli/admin.rs:377`, `reenter_tests.rs:229`, desktop `fixtures.rs:1060`, `build.rs:100`, `mutation_contract.rs:638`, `mutation_replay/{publication_wait.rs:125, remotes.rs:437, 1465, index.rs:642, not_enabled.rs:121, live_calls.rs:158}`, `mutation_confirmation/{conformance.rs:224, abandon.rs:2007}`, `support/confirmation.rs:606`, `repository_enablement/enablement_index.rs:443`, `mutation_remote/c5_poll.rs:673`) | yes |
| E-9 `command.rs` tests, `catalog.rs`, `catalog_tests.rs` (81), `tests/cli/inventory.rs` (`mutating` 44 at 394, `catalogued` 81 at 412, `NO_INPUT` 29, `DATA` 133), `tests/support/conformance.rs` `HOMES: [..; 46]` (59), `tests/mutation_confirmation/conformance.rs` (162), `tests/cli/skills.rs` `AUTONOMY` (459), `skills/manyhands-overview/references/autonomy.md`, `help_root.json`, `help_root_text.txt` | E | all exist | yes |
| E-10 `CASES` (mutation_golden.rs:21), `DATA_ROOTS` (69, 1603), `inventory.rs` `DATA` | E | | yes |
| E-11 envelope and request code enums, `outcome.rs` `recovery`, `outcome_tests.rs` | E | `schemas/v1/envelope.schema.json`, `request.schema.json`; `outcome.rs:778`; `outcome_tests.rs:358, 1064, 1632, 2373` (lists with `PublicationRemoteRequired`) | yes |
| E-12 `preview_effect.schema.json`, `human_admin.rs` `effect_phrase` / `preview`, desktop `confirm.rs` | E | `human_admin.rs:165, 98`; `confirm.rs:649-780` | yes |
| E-13 exhaustive `MutationDataDto` matches: six in `tests/support/mutation.rs` (674, 725, 861, 893, 925, 958), one in `tests/support/repair.rs` (335-359) | E | | yes |
| E-14 `LocalStep` name-list test | E | `recovery.rs:4110-4144` (in `mod abandoned_row_tests`, 3897) | yes (design says "about line 4137") |
| E-15 audit unions, `exits.rs` (49/52 at 600-602), `collaboration_exits.rs` (C4 49/51 at 330-332; C5 46/49 at 1289-1291), `recovery_tables.rs` | E | | yes; also E-18, E-19, E-20 |
| D-1 `docs/RFC/cli-contract.md`, D-2 `git-workflow-and-conflict-recovery.md`, D-3 `repository-index-persistence-and-refresh.md`, D-4 `docs/research/wave-03-api-audit.md`, D-5 `skills/` and `.agents/skills/`, D-6 `docs/Waves/wave-06-pull-request-workflow.md` | E | all exist | yes |
| `tests/cli/inventory.rs` `no_verb_is_planned` | E | `inventory.rs:413-423` (`C1`..`C5`) | yes |
| `tests/read_contract.rs` `contract_enumerations`, `published_enumerations_list_exactly_the_registered_values` | E | 724, 1594 | yes |
| `human_tests.rs`, `human_admin_tests.rs` | E | `bin/manyhands-cli/human_tests.rs`, `human_admin_tests.rs` | yes |
| File Map Task 11 `src/desktop/services/scheduler/entity.rs` and the scheduler's backend / test double | E | `scheduler/entity.rs`; `Backend` trait `desktop/services/mod.rs:213`; doubles E-21 | yes |

## 9. Step 5: CI test lists

No CI test list changes.
- `.github/workflows/build.yml:117` runs `cargo test --locked --no-fail-fast --target ... --lib --test shared_key_registry ... --bin manyhands-cli --test cli --test cli_remote` and `:129` the desktop (`--features desktop-test-support --bin manyhands`). Every target the plan touches is in that list: `repository_enablement`, `mutation_contract`, `mutation_confirmation`, `mutation_replay`, `mutation_remote`, `read_contract`, `cli`, `recovery_foundation_gate`, `ssh_transport`, `remote_polling`, plus `--lib` and both bins.
- AGENTS.md's fast tier (27 targets plus `--lib --bins`) holds `repository_enablement`, `mutation_contract`, `mutation_confirmation`, `mutation_replay`, `read_contract`, `cli`, `recovery_foundation_gate`; the SSH tier holds `mutation_remote`, `ssh_transport`, `remote_polling`. `tests/*.rs` has 39 files = 27 fast + 11 SSH-harness + `editor_feasibility`.
- The plan's files: every `tests/` path it names already exists (`tests/mutation_contract.rs`, `tests/support/*.rs`, `tests/repository_enablement.rs`, `tests/read_contract.rs`, `tests/cli/input_schemas.rs`, `tests/mutation_remote.rs`, `tests/recovery_foundation_gate.rs`, `tests/mutation_replay.rs`, `tests/mutation_replay/not_enabled.rs`, `tests/mutation_confirmation.rs`, `tests/mutation_confirmation/conformance.rs`, `tests/cli/repository.rs`, `tests/cli/remotes.rs`, `tests/cli/inventory.rs`, `tests/support/conformance.rs`). The only new test files are `src/` unit-test files (`*_tests.rs` under `src/repository/setup/` and `src/repository/mutation/`), which belong to the `--lib` target. No file would create a new `tests/<name>.rs` target, and `Cargo.toml` needs no `[[test]]` entry.
- Incidental, pre-existing and not this Cycle's: `read_characterization` is in AGENTS.md's fast tier but not in `build.yml:117`'s list.

## 10. What the sweep covered

Per the design's last Named Edits paragraph, with the result:
- Exhaustive matches on `Command`: `command.rs` const fns (9, all in the File Map), `bind/mod.rs` `recorded_effects`, `dto.rs` `from_stored`; tests: only the `command.rs` test lists and `outcome_tests.rs` loops (section 2).
- `Mutation`: `mutation/mod.rs:638, 694, 752` only; no exhaustive match in tests or desktop.
- `MutationDataDto`: tests/support (E-13) only; `src/bin/manyhands-cli/{human_collaboration,history,polling}.rs` use `if let`/`matches!`.
- `PreviewDetailsDto`: `human_admin.rs:99-120`, `confirm.rs:649-780` (E-12); tests use constructors only.
- `PreviewEffect`: `human_admin.rs:165-204` (E-12); `human_admin_tests.rs:111` iterates `ALL`.
- `LocalStep`: `recovery.rs:1926, 1982`, `repository.rs:4300-4328` (source), E-14 (test); iterating tests listed in section 2.
- `Session`: three impls (`progress.rs:501` `NoSession`, `CallSession` at 331, `lifecycle_tests.rs:416` `Recorder`), as the File Map says.
- Struct literals: `RepoEnableInput`, `RemoteSelectInput` (E-8), `RepositorySummaryDto`/`RepositoryInspectionDto` (E-5), `PreparedSshAttempt` (E-16), `VerifySshTransportRequest` (39, unchanged by design), `EnableRepositoryRequest` (86, unchanged by design).
- Schema-closure and enum-count tests: E-7; `inventory.rs` input-schema file set (inline `forge` required); `DATA_ROOTS` length (E-10); `CODES_` array lengths (E-3, E-4).
- Desktop scheduler read counts: E-21.
- Snapshot tests of `.git`: no break expected (section 2).
