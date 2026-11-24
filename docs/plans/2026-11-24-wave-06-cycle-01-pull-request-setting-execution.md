# Wave 06 Cycle 01 Execution Ledger

Ticket vh-c2dtj (`01M87683441PRAYQJQBQBG96YY`). Plan: `docs/plans/2026-11-24-wave-06-cycle-01-pull-request-setting-implementation.md` (approved 2026-11-24, revision 4). Execution: subagent-driven. One line per paragraph.

## Task 0

- Base: `origin/main` `6cfe0ab0`. HEAD before and after the merge: `3b955662` (already up to date; merged, never rebased).
- A2: `git show v0.2.1:src/canonical.rs` line 1304 holds "unsupported format_version; expected 1".
- Names: every cited name exists, or is new where the design says so (report below). Corrections: `RepositoryConfig` has 14 struct literals, not 16; `served_tip` is a method of `Synced` in `tests/mutation_remote/merging.rs`, not a harness helper; `src/repository/remote/state_tests.rs` is the module `persistence_tests`.
- The primary-context predicate is `contexts.item_id IS NULL`. The no-action recovery arm is `recovery()`'s shared arm ending with `RequestNotFound`, which already holds `MergeRequired` and `PushRejected`.
- Test target flags: `--lib` for `results::tests`, `forge::tests`, `repository::remote::sync::tests`, `repository::mutation::bind::sync::tests` and `repository::discovery::tests`; `--bin manyhands-cli` for `human_collaboration::tests` and `human::tests`.
- CI: no change to `.github/workflows/build.yml` or AGENTS.md's test lists; no new test target.

### Named edits added by the sweep (decisions 9 and 12)

- **E-17** (Task 7) `tests/support/mutation.rs` (6 exhaustive `MutationDataDto` matches) and `tests/support/repair.rs` (1): add the `PrimaryLocalCommits` arm; without it nearly every integration target fails to compile.
- **E-18** (Task 6) `tests/comment_publication.rs` (an SSH-harness target, about lines 969-987): its exhaustive `SynchronizationError` match gains the `PrimaryHasLocalCommits` arm. `comment_publication` already runs at review-ready (R-12).
- **E-19** (Task 8) `tests/mutation_remote.rs` `call()`: it picks the data schema by `data.type`; add the `primary_local_commits` arm, or the SSH case panics.
- **E-20** (Task 9) `tests/cli/collaboration_exits.rs`: adding the code to the CLI contract (D-1) puts it in the audit's C4 union, so its counts go 48 to 49 and 50 to 51, plus one `REASONS` row. Task 9 Step 3 also runs this test.

### Corrections to the design's named edits

- E-3 is not needed: `tests/read_repository.rs`'s case writes `format_version = 2` without `primary_branch`, so it stays invalid. E-1, E-2 and E-4 are confirmed (5 sites).
- E-8 covers every new schema `enum`: `integration`, `forge.kind`, and (with E-15) the data's `type`.
- E-9 is 4 calls of `admin::stored_configuration`.

### Rulings

- Ruling: `remote_operation_records.pull_only` and its trigger are added by checking the schema (column and trigger present or not), not gated by the `wave_06_pull_request_mode_v1` marker, which gates only the configuration columns' dirty marks — existing tests rebuild an older `remote_operation_records` and reopen with every marker present (`persistence_tests`' cycle04 poll migration, the `poll_kind` tests, `src/repository/remote/reservation_tests.rs` about line 790) — if wrong, those tests fail at Task 3 and the column must move.
- Ruling: `insert_operation_with_kind` gains the `pull_only` parameter (2 source and 5 test call sites); `insert_operation` keeps its signature and inserts 0 — fewer edits, and the public path stays direct — none.
- Ruling: E-3 is dropped — it is not a break — none.
- Ruling: Task 2 Step 6's link check asks the owner with AskUserQuestion when Task 2 completes — the check is decision 7 and the owner is present — if the owner cannot answer then, it moves to review-ready.

## Task 0 sweep report (verbatim)

# Task 0 research report (read-only), HEAD 3b955662

Scope: Task 0 Steps 3-5 of the implementation plan. Nothing in the repository was edited and cargo was not run. "Confirmed" means read in source at this HEAD. "Inference" means reasoned from the source, not run.

All paths are relative to the worktree root. Line numbers are at HEAD 3b955662.

## 0. Summary of what matters most (details below)

1. Every name the design cites exists, except the ones the design itself marks new. Two counts differ from the design: struct literals of `RepositoryConfig` are 14, not 16 (A-1); the `read_configuration_for` etc. line numbers moved (table).
2. The design's E-list misses compile breaks and test breaks that the plan's tasks do not schedule (section D, items D-A to D-F). The three that would fail first:
   - Adding `MutationDataDto::PrimaryLocalCommits` breaks the compile of nearly every integration target: `tests/support/mutation.rs` (6 exhaustive matches) and `tests/support/repair.rs` (1) match it with no wildcard. This must be fixed in Task 7, not Task 8.
   - Adding `SynchronizationError::PrimaryHasLocalCommits` breaks `tests/comment_publication.rs` (SSH target, one exhaustive match) in Task 6.
   - The SSH case's envelope check in `tests/mutation_remote.rs` `call()` dispatches data schemas by `data.type` and falls through to `synchronization_mutation.schema.json`; the new tag needs its own arm in Task 8 or the case panics.
3. D-1/D-4 chain: a `repo sync` row for the new code in `docs/RFC/cli-contract.md` forces the code into the C4 exit-coverage union of `docs/research/wave-03-api-audit.md`, which forces an edit to `tests/cli/collaboration_exits.rs` (48 to 49, 50 to 51, one REASONS row). Task 9 Step 3 does not run that test.
4. Migration: the design's marker-gated column add is unsafe against existing tests that rebuild an older `remote_operation_records` and reopen with all markers still present (section D, D-G).
5. Section E: no CI list changes.

## A. Names and paths

"Cited as" is the design/plan's name. "Exists?" is existing, new (the Cycle adds it), or "moved/differs".

| Name | Cited as | Exists? | Current file:line | Note |
| --- | --- | --- | --- | --- |
| RepositoryConfig | struct, 3 fields, derives Clone, Debug, PartialEq | existing | src/canonical.rs:84 | Fields `primary_branch`, `publication_remote`, `unknown: toml::Table`. |
| RepositoryConfig struct literals | "16 places" (Fact 2) | existing; count differs | see A-1 | 14 literals at HEAD (13 outside the parser). |
| parse_repository_config | refuses version other than 1 | existing | src/canonical.rs:1288 (version match 1301-1313, message at 1304) | `toml::Value::Integer(1)` arm then `Integer(_)` refusal. |
| serialize_repository_config | re-inserts format_version=1 | existing | src/canonical.rs:1329 (insert at 1339) | Validates only branch and remote names today. |
| "unsupported format_version; expected 1" | message | existing | src/canonical.rs:1304 | Only source of the text; no test asserts it (grep of src, tests, skills, docs/RFC). |
| toml 0.8 without preserve_order | sorted keys | existing | Cargo.toml:36 (`toml = "0.8"`); toml 0.8.23 in registry | No `ValueAfterTable` error exists in toml 0.8.23 `src/ser`, so a `[forge]` table sorted before plain keys serializes (tables are emitted after values). Inference for the exact bytes; Task 1's byte test pins it. |
| read_configuration_for | root checkout working-tree file | existing | src/repository.rs:11857 (`read_configuration` wrapper 11853; guarded reader 11899) | `repo inspect` uses `read_configuration_guarded` (called from repository.rs:6744 and the `ConfigurationReader::Guarded` path), not `read_configuration_for`. |
| persist_configuration | writes the configuration_observations row | existing | src/repository.rs:8096 | Called only from `persist_context_index` at 7787, which runs for a context that carries `observation.configuration` (the primary). Inserts at 8121, 8126, 8129. |
| configuration_observations | table | existing | CREATE at src/repository/discovery.rs:2662; reads at src/repository.rs:8414, src/repository/read/admin.rs:229, src/repository/read/items.rs:756, src/repository/local_sync/observe.rs:68 | Columns today: repository_id, state, primary_branch, publication_remote, invalid_code, guidance. |
| admin::stored_configuration | 4-argument fn | existing | src/repository/read/admin.rs:72 | Callers: admin.rs:253 (repo list), src/repository/read/tests.rs:415, 422, 428, 434. A different private fn with the same name is at src/repository.rs:8832 (call 8430, the index snapshot); it is not touched by this Cycle. |
| ConfigurationDto | struct | existing | src/repository/read/dto.rs:131 | Derives Clone, Debug, PartialEq, Eq, Serialize (no Deserialize). |
| ConfigurationDto literals (7) | Fact 5 | existing, count confirmed | src/repository/read/admin.rs:61; src/bin/manyhands-cli/human_tests.rs:17; src/desktop/model/repository_state_tests/scene.rs:141; src/desktop/model/navigation_tests.rs:20; src/desktop/view/add_repository_tests/support.rs:74; src/desktop/view/shell/title_tests.rs:23; src/desktop/view/workspace/add_sheet_tests/share.rs:96 | 7 literals. tests/read_repository.rs:1230-1271 only names the type. |
| repository_configuration.schema.json | requires 4 keys | existing | schemas/v1/repository_configuration.schema.json:7 | `additionalProperties: false`. Nullable inline objects already exist (`"type": ["object","null"]`, e.g. poll_retirement_mutation.schema.json:28). |
| repository_inspection (human.rs) | prints the inspect lines | existing | src/bin/manyhands-cli/human.rs:64 | Format string at 66 prints 9 fixed lines. |
| human_tests line-count test | repository_and_remote_metadata_escape_controls_at_field_boundaries | existing | src/bin/manyhands-cli/human_tests.rs:931; inspect call 971; counts `11, 10` at 982-983 | 11 lines today = 9 fixed + 1 problem + 1 remote. With a valid state and an `Integration:` line it is 12; the field count 10 is unchanged unless `forge` carries CONTROL_FIELD. |
| ForgeDto, Integration, ForgeKind, ForgeSetting, Forge, RemoteLocation, PullRequestLink | types | new | none yet | New types in canonical.rs, forge.rs, read/dto.rs. `pub use read::*` at src/repository.rs:118 re-exports read types, so `ForgeDto` must be in the list at src/repository/read/mod.rs:51. |
| src/forge.rs, `pub mod forge;` | module | new | src/lib.rs has canonical, drafts, repository, results, runtime | |
| migrate_registry | runs every migration | existing | src/repository/discovery.rs:2558 | Order: main transaction (…, `remote::state::migrate` 2747, `promotion::record::migrate`, `closure::record::migrate`, commit), then `migrate_item_read_schema` (2852), `recovery::migrate_operation_records` (recovery.rs:647, adds `contexts.dirty` via marker `cycle_context_basis`), `mutation::migrate_request_tables`, and last, as the return expression, `context_retirement::record::migrate` at 2756. A new migration needs `?` on that call and goes after it. |
| registry_migrations markers | named markers | existing | CREATE in recovery.rs:67 and 678 | `require_known_markers` (context_retirement/record.rs:1247) only rejects unknown names `LIKE 'context_retirement_%'`; `wave_06_pull_request_mode_v1` is not affected. |
| migrate_item_read_schema | immediate-transaction precedent | existing | discovery.rs:2852 | Read-only precheck `missing_item_read_schema` (2887); if anything is missing, `transaction_with_behavior(Immediate)`, re-check, apply, `UPDATE repositories SET refresh_required=1`, then `UPDATE contexts SET dirty = 1` guarded by a `pragma_table_info('contexts')` check for `dirty`. |
| remote_operation_records | table | existing | CREATE in `REMOTE_OPERATION_SCHEMA` src/repository/remote/state.rs:1114; indexes/target-immutable trigger 1149-1153 | Created by `remote::state::migrate` (state.rs:105) before the new migration would run. Not validated by schema text (only `validate_poll_time_columns` 650ff reads two column definitions). |
| poll_kind ADD COLUMN | precedent | existing | state.rs:160-172 | Column-existence check, then `ALTER TABLE … ADD COLUMN poll_kind …` inside `remote::state::migrate`, independent of any marker. |
| remote_poll_kind_immutable | trigger | existing | state.rs:174 | `CREATE TRIGGER IF NOT EXISTS … BEFORE UPDATE OF poll_kind … RAISE(ABORT,'immutable poll kind')`. |
| `cycle04` table rebuild | not cited | existing | state.rs:1832-1865 | Runs only when none of the nine sync columns exist (`migrate_synchronization`, 1782); copies by the old table's column list. Not a risk once the DB is migrated. |
| synchronization_reserve_owned | fn | existing | src/repository/remote/sync.rs:4157 | Non-test callers: 6204 (inner, `existing.is_none()`), 8143 (republish, context target, new), 8446, 8472, 8484 (all in `open_synchronization`, `new=false`). cfg(test): 3958. |
| reserve_remote_operation_kind | fn | existing | src/repository/remote/reservation.rs:1453 | Callers: 1355 (`reserve_remote_poll`), 1404 (`reserve_remote_operation_with_priority`), 1435 (`reserve_promotion_child`). Existing-row path returns `Replay` at about 1578-1597 (compares target, priority, poll_kind only). Inserts at 1635 (`insert_operation_with_kind`, poll Update) and 1645 (`insert_operation`). |
| insert_operation_with_kind | fn | existing | src/repository/remote/state.rs:6606 | Non-test callers: reservation.rs:1635 and `insert_operation` (state.rs:6595). Test callers: state_tests.rs lines 344, 464, 1019, 1040, 1225 (5). |
| insert_operation | wrapper | existing | state.rs:6578ish | Non-test callers: reservation.rs:1645; context_retirement/authority.rs:360 and 484. 12 calls in state_tests.rs, plus authority_tests.rs:1755, store_tests.rs:1066, reservation_tests.rs:3206, sync_tests.rs:1421, 1480. Keep its signature (inserts 0). |
| reserve_remote_operation | public, call count | existing | reservation.rs:1379 (`_with_priority` 1395) | Non-test callers in src: sync.rs:4168 and 4170 (inside `synchronization_reserve_owned`), src/repository/promotion/run.rs:1433 (lifecycle child, via `reserving`/`restart_remote_lifecycle`); sync.rs:7672 is cfg(test). Plus about 130 calls in tests (not an edit if the signature holds). |
| Paths that take up an existing row | existing, replay race, restart, local_owner | existing | sync.rs `open_synchronization` 8420 (`existing`, replayed, cancelled, not-restart refusal); inner 6051: `existing` 6063; `local_owner` 6104-6131 (merge-holding row); replay race at the `RemoteReservationOutcome::Replay` arm 6213-6240; `restart` 6240 (`synchronization_restart_owned`, def 4170ish) | The owner exists at about 6247 (`let owner = match reservation`). The helper that reads the stored `pull_only` must run after that line and also for `local_owner`'s early reservation (6112). |
| synchronize_remote_observed_inner | fn | existing | sync.rs:6051 | Three `graph_plan` calls inside it, confirmed (below). |
| graph_plan (def) | | existing | sync.rs:4057 | Also called at 4113 in the cfg(test) helper `promotion_test_publication_plan`. |
| graph_plan call 1 | push-absence deferral | existing | sync.rs:6482 | `.is_ok_and(|planned| !planned.local_update)`; yes/no only. |
| graph_plan call 2 | divergence block | existing | sync.rs:6675 (match 6675-6694, `ended_if_unresolved` at 6686) | `Ok(_) => false`, `Err(MergeRequired) => true`, other errors wrapped. The plan result is discarded in `Ok(_)`; the pull-only check must read it. |
| graph_plan call 3 | under the repository lease | existing | sync.rs:6821 (`.map_err(ended_if_unresolved)` 6830) | |
| push_call and its decide closure | fn; closure at the call | existing | def sync.rs:8617; inner call 6907 with the `decide` closure 6907-6975; second caller 8294 (republish, context target) | Closure sets `evidence.push_oid = Some(candidate)` at about 6947 before knowing whether it pushes. |
| ended_if_unresolved | fn | existing | sync.rs:4133 | Only `HistoryUnresolved && !keep` ends the op. Existing table test: sync_tests.rs:16754 `unresolved_history_ends_the_operation_and_frees_the_reservation` (calls it at 16823 with a 4-argument signature). |
| reconcile_synchronization | fn | existing | src/repository/remote/reservation.rs:3249 | Refuses (`recovery_required`) when local head differs from the recorded candidate/expected OID. O7 premise confirmed. |
| SynchronizationError | enum | existing | sync.rs:358 (19 variants) | Derives Debug only (not Clone). Exhaustive matches: Display at sync.rs:419ish; src/repository/remote/conflict_read.rs:811-837 (`read_error`); src/repository/mutation/outcome.rs:197-234 (`synchronization_error_code`); tests/comment_publication.rs:969-987 (see D-B). Wildcard matches (no edit): promotion/run.rs:1163, poll.rs:259, comment_publication.rs:257. |
| SynchronizationOutcome | enum | existing | sync.rs:238 | No variant change in the design. `authority_outcome` at 8937. |
| SynchronizationSettings | struct | existing | sync.rs:347 | One field. Literals: src/repository/mutation/bind/sync.rs:148 (`MERGING`), tests/mutation_remote.rs:413 (`REFUSING`). Unchanged (E-11 retired). `Default` used at sync.rs:6015, 6046 and tests. |
| MutationDataDto | untagged enum | existing | src/repository/mutation/dto.rs:996-1022 (21 variants incl. Blocked) | New variant `PrimaryLocalCommits`. |
| SynchronizationConflictMutationDto and its `type` | tag pattern | existing | dto.rs:1677-1713 (`contract_enum!` `SynchronizationConflictType` with one value; `from_stored` reads the tag) | `from_stored` arm at about 2479-2490 for `ItemSync | RepoSync | OperationResume` guarded by the tag, placed before the generic `SynchronizationMutationDto` arm (2531-2537). |
| MutationDataDto::from_stored / ::authority | | existing | dto.rs:2373 (`authority`, exhaustive, no wildcard) and 2412 (`from_stored(command, stored)`) | `authority` needs an arm for the new variant. |
| binding `stopped` and HistoryUnresolved check | | existing | src/repository/mutation/bind/sync.rs:1108; `unresolved` at 1125; `cancelled` computation 1126-1142; `recovery` for `unresolved` only at 1286-1296 | `effects` come from `effects_so_far(publication)` (about 1228-1236), not `not_requested`; a refusal over a `Final(Cancelled)` row is answered `cancelled` unless widened as `unresolved` is. |
| unresolved_history_is_answered_with_the_repair_and_no_retry | test | existing | src/repository/mutation/bind/sync_tests.rs:18 | Builds a registry row, a reservation, a cancelled operation, an `Accepted`, a `SyncBinding` and `Watch`, then calls `binding.stopped(…)`. |
| result_codes! | macro | existing | src/results.rs:543 (MergeRequired 646) | New code position must be identical in five ordered lists (see section C). |
| src/results_tests.rs | table and messages | existing | `DESIGN_CODES: [..; 70]` at line 34 (`merge_required` at 86); `result_codes_match_the_design_table_exactly` 145; `result_code_messages_are_fixed` 154 (array ends 226) | Both arrays are in `ResultCode::ALL` order. |
| recovery fn | exhaustive, no-action arm | existing | src/repository/mutation/outcome.rs:773 | See section C. |
| envelope.schema.json / request.schema.json code enums | | existing | schemas/v1/envelope.schema.json:91 (`merge_required`); schemas/v1/request.schema.json:102 | Compared in order with `ResultCode::ALL` (tests/read_contract.rs:876 and 1122), so insert at the same position. |
| DATA_ROOTS | 68 roots | existing | tests/mutation_contract.rs:1603 (`[&str; 68]`) | |
| CODES_NO_MUTATION_GOLDEN_HAS | 7 | existing | tests/mutation_contract.rs:1841 (`[..; 7]`) | |
| CODES_NO_FIXTURE_CAN_PRODUCE | 3 | existing | tests/mutation_contract.rs:1789 (`[..; 3]`) | |
| contract_enumerations | fn | existing | tests/read_contract.rs:724 | `schema_enum` (667) walks `properties.<name>` per dotted path, so `forge.kind` needs `forge` to be an inline object with `properties` (not `$ref`/`oneOf`). |
| published_enumerations_list_exactly_the_registered_values | test | existing | tests/read_contract.rs:1567 | Asserts `count_enums(all published schema files) == enumerations.len() + 1`: every new schema `enum` must be registered, including the `integration` enum, `forge.kind`, and the new `type` enum. |
| tests/mutation_remote.rs case table | | existing | `const CASES` at tests/mutation_remote.rs:101; `("c4_conformance_repo_sync", …)` at 373; also chained `merging::CASES` at 76 (tests/mutation_remote/merging.rs:23) | R-9's table is the top-level one. |
| c4_conformance_repo_sync | fn | existing | tests/mutation_remote.rs:4906 | A conformance row (`CONFORMANCE` at 4785), not a golden writer. |
| assert_golden | fn | existing | tests/mutation_remote.rs:836 | Replacements today: `/data/port` to 0, authority, three fingerprints. The commit OIDs of `effects.commit_oid` are not replaced here (other cases keep none). A `/data/local_commits` array needs a loop that pushes `(oid, "<local-oid-N>")`. |
| `call()` data-schema dispatch | not cited | existing | tests/mutation_remote.rs:785-801 | See D-C. |
| tests/support/mutation_golden.rs CASES | | existing | tests/support/mutation_golden.rs:21 (`repo_sync_push_rejected` at 208) | Alphabetical; new name goes after `repo_sync_no_selected_key`/`poll_yielding`… in order. Fixture dir and CASES must match (tests/mutation_contract.rs:1511). |
| w.peer / push_peer | harness | existing | `World.peer` tests/support/remote_world.rs:140; `push_peer` remote_world.rs:57; `advance` 115; `commit` 85 | Used at tests/mutation_remote.rs:500-501, 546-547. |
| served_tip | harness | differs | tests/mutation_remote/merging.rs:212 | A method of `Synced` in merging.rs, not a World helper. The new case in tests/mutation_remote.rs reads `fixed(w.bare()?.refname_to_id("refs/heads/main"))` directly, as at 1668. |
| a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty | test | existing | src/repository/discovery.rs:3195 (helpers `registry_without_context_basis` 3138, `context_basis_rows` 3180) | Inside `mod tests {` opened at 2919. |
| unresolved_history_is_answered_with_the_repair_and_no_retry | test | existing | see above | |
| graph_plan unit test, fixture(), child() | | existing | sync_tests.rs:831 `actual_commit_graph_rejections_preserve_ref_index_and_worktree_bytes` (calls `graph_plan` at 906 and 913); `fixture()` 66 returns `(root TempDir, data TempDir, RepositoryService)`; `fixture_in` 70; `child(repo, parent, bytes)` 812 | `child` commits with summary "fixture", author "Fixture". |
| c4_merging_clean_divergence | test | existing | tests/mutation_remote/merging.rs:447 (`c4_merging_local_behind` 429) | |
| scripts/check-cli-no-gpui.sh | | existing | scripts/check-cli-no-gpui.sh | `cargo tree --locked -e normal --no-default-features`; greps `^gpui`, `^zorite…`, `^image `, `^markdown `. A `percent-encoding` dependency does not match. |
| percent-encoding | transitive 2.3.2 | existing | Cargo.lock:5915 | Not in Cargo.toml. |
| parse_ssh_endpoint, publication_remote_eligible_for | Facts 7, 8 | existing | src/repository/transport/endpoint.rs:56 (`pub(in super::super)`); src/repository.rs:12568 | |
| CleanIntegrationPlan | | existing | src/repository/remote/refs.rs:227 | `push_needed = remote_oid != Some(final_oid)`; Primary: remote ahead gives false, local ahead gives true; diverged is `Err(MergeRequired)` mapped by `graph_plan`. For a Primary target `HistoryUnknown` cannot occur. |

### A-1. RepositoryConfig struct literals (Fact 2 says 16)

14 literals at HEAD: src/canonical.rs:1322 (parser); src/repository.rs:12002; src/repository/comment_publication_tests.rs:84; tests/repository_enablement.rs:1334; tests/document_promotion.rs:173; tests/document_promotion/advanced.rs:283; tests/ticket_closure.rs:178; tests/comment_publication.rs:399; tests/canonical_foundation.rs:260, 276, 287, 300, 312. That is 14 (the parser plus 13). `config_with` (canonical_foundation.rs:1757) is a function, not a literal. R-1's argument (no literal changes) holds; only the number differs.

## B. Test module paths and cargo target flags

| Plan's path | Confirmed? | Flag and exact filter form |
| --- | --- | --- |
| results::tests | yes: src/results.rs:1126-1127 `#[path = "results_tests.rs"] mod tests;` | `--lib results::tests::<name> -- --exact` |
| forge::tests | new (to be created in src/forge.rs) | `--lib forge::tests::<name> -- --exact` |
| repository::remote::sync::tests | yes: src/repository/remote/sync.rs:4-5; nested modules `closure_conflict_classification` (sync_tests.rs:10912) and `process_death` (15777) | `--lib repository::remote::sync::tests::<name> -- --exact` |
| repository::mutation::bind::sync::tests | yes: bind/sync.rs:1568-1569; `pub(crate) mod sync;` at bind/mod.rs:61 | `--lib repository::mutation::bind::sync::tests::<name> -- --exact` |
| human_collaboration::tests | yes: src/bin/manyhands-cli/human_collaboration.rs:480-481; declared in src/bin/manyhands-cli.rs:36 | `--bin manyhands-cli human_collaboration::tests::<name> -- --exact` |
| src/repository/discovery.rs tests | inline `mod tests {` at discovery.rs:2919 | `--lib repository::discovery::tests::a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty -- --exact` (the new migration test goes beside it, same path prefix) |
| src/bin/manyhands-cli/human_tests.rs | `#[path = "human_tests.rs"] mod tests;` in human.rs:531-532; `mod human;` in manyhands-cli.rs:30 | `--bin manyhands-cli human::tests::repository_and_remote_metadata_escape_controls_at_field_boundaries -- --exact` |
| reservation tests near insert_operation_with_kind | `insert_operation_with_kind` tests live in src/repository/remote/state_tests.rs, declared at state.rs:1927 as `mod persistence_tests;` (not `tests`). Reservation-level tests live in src/repository/remote/reservation_tests.rs (reservation.rs:3-4 `mod tests;`). | `--lib repository::remote::state::persistence_tests::<name> -- --exact`; `--lib repository::remote::reservation::tests::<name> -- --exact` |
| MutationDataDto::from_stored replay tests | no unit test of the sync/resume data exists today. Closest homes: `repository::mutation::dto::` (top-level `#[test]` at dto.rs:2270 `poll_retirement_stored_contract_is_closed_and_cannot_complete_a_not_reached_prefix`, calls `MutationDataDto::from_stored(Command::PollOnce, …)`) and `repository::mutation::dto::poll_retirement_recognition_tests::` (dto.rs:2528-2529). The end-to-end replay of a stored result is exercised by tests/mutation_replay*.rs and tests/mutation_remote.rs, not by a unit test of `from_stored`. | `--lib repository::mutation::dto::<name> -- --exact` |
| remote ended_if_unresolved test | sync_tests.rs:16754 | `--lib repository::remote::sync::tests::unresolved_history_ends_the_operation_and_frees_the_reservation -- --exact` |
| graph_plan test | sync_tests.rs:831 | `--lib repository::remote::sync::tests::actual_commit_graph_rejections_preserve_ref_index_and_worktree_bytes -- --exact` |

Other target names: integration tests use `--test <target> <name> -- --exact` (`canonical_foundation`, `repository_enablement`, `read_repository`, `read_contract`, `mutation_contract`, `mutation_remote`, `cli`).

## C. Predicate and the no-action recovery arm

- **Primary context predicate:** `contexts.item_id IS NULL`. Existing uses: `UPDATE contexts SET dirty = 1 WHERE repository_id = ?1 AND item_id IS NULL` (src/repository.rs:12978, `mark_registered_contexts_dirty`, `ContextKey::Primary`); src/repository/promotion/observe.rs:299; src/repository/context_retirement/request_scope.rs:502; the `ContextKey::Item` branch uses `item_id = ?2`. `kind = 'primary'` (activity.rs:118, 196) misses a primary stored as `unverified` (invalid configuration or wrong branch, discovery.rs:984, 990), so `item_id IS NULL` is the right one for "every registration's primary context". `contexts.dirty` exists at this point of `migrate_registry` (added by `recovery::migrate_operation_records` through marker `cycle_context_basis`).
- **Recovery arm that gives no actions:** the final shared arm of `recovery()` in src/repository/mutation/outcome.rs, starting `ResultCode::Ok | ResultCode::AlreadyApplied | … ` (about line 925) and ending `| ResultCode::RequestNotFound => { if retry { retry_request() } else { Vec::new() } }` (about 1007-1013). It offers nothing when `retry` is false and the same request when `retry` is true. The sync-related block inside it (lines 976-990 area) lists `MergeRequired | RemoteBranchDeleted | PushRejected | NoSelectedKey | KeyRejected | UnlockRequired | UnlockFailed | HostApprovalRequired | HostReplacementRequired`; the new code belongs in that block. Codes that return no actions in every case: `OriginalRequestRequired | OperationNotFound` (explicit arm, `Vec::new()`), and `RecoveryRequired` when `root.is_none()` and not `retry`. All codes in the shared arm give no actions when `retry` is false: Ok, AlreadyApplied, UsageError, CommandUnavailable, Cancelled, InternalError, InvalidInput, InvalidPath, NotRepository, NotRepositoryRoot, BareRepository, RepositoryNotEnabled, RepositoryInaccessible, ItemNotFound, RelationshipCycle, InvalidRelationship, RequestMismatch, NotConfirmable, OccupiedPath, SlugAlreadyAssigned, TicketClosed, WrongBranch, WorktreeNotClean, WorktreeConflicted, RemoteNameConflict, InvalidRemote, KeyNotFound, SelectedKeyInUse, KeyNotDeletable, KeyUnavailable, ConfirmationMismatch, ConfirmationUsed, NotRepairable, IdentityAmbiguous, PathNotFound, PublicationRemoteInUse, PublicationRemoteRequired, HostMismatch, MergeRequired, RemoteBranchDeleted, PushRejected, NoSelectedKey, KeyRejected, UnlockRequired, UnlockFailed, HostApprovalRequired, HostReplacementRequired, InvalidId, PublicKeyUnavailable, AuthorityNotFound, RequestNotFound.
- For the new code, `command.is_some_and(Command::stops_at_safe_points) && retry && !offered` (closing rule of `recovery`) splices `request.retry` in when the record is left accepted. The golden's `recovery: []` (row `next` = `none`, tests/cli/recovery_tables.rs:370ff) therefore needs the binding to finish the record (Final), not leave it accepted. The existing `unresolved_history…` test shows a `Final(Cancelled)` journal settles as finished (settle.rs:55). Existing test `a_synchronization_that_stopped_with_its_reservation_held_is_offered_the_same_request` (outcome_tests.rs:1568) iterates every code for `repo sync` with `retry = true`; the new code passes it unedited (first action `request.retry`, nothing else).
- Ordered lists that must all insert the new code at the same index: `result_codes!` (results.rs:543), `DESIGN_CODES` (results_tests.rs:34), the messages array (results_tests.rs:154-226), `schemas/v1/envelope.schema.json` code enum (line 91 area), `schemas/v1/request.schema.json` (line 102 area). Tests compare these as ordered vectors.

## D. Premise sweep

### D.0 Confirmation or correction of the design's own E-1 to E-16

- E-1 confirmed: src/repository/discovery.rs:4463 (`missing_or_malformed_configuration_is_unverified_but_keeps_documents`, a loop body that asserts the config is not Valid) and :4693 (`invalid_configuration_suppresses_active_context_discovery`). Both would go green-to-red (version 2 + primary_branch main becomes Valid).
- E-2 confirmed: tests/canonical_foundation.rs:332 (`config_reports_structured_errors_for_invalid_input`, case 3, expects `InvalidField`).
- E-3 corrected: tests/read_repository.rs:681 (`inspect_repository_reports_registration_configuration_and_redacted_remotes`) writes `format_version = 2\n` with no `primary_branch`. It stays Invalid (MissingField) after the change and does not assert the code. It passes unedited; the 2-to-3 edit is optional and only keeps the intent. Not a break.
- E-4 confirmed (two cases): tests/repository_enablement.rs:1088 (`inspect_invalid_configuration_preserves_its_validation_problem_and_source_bytes`, `parse_repository_config(source).unwrap_err()` would panic) and :3431 (`enable_invalid_configuration_with_supplied_identity_preserves_all_state`, enable would succeed). tests/repository_enablement.rs:1972 (`= 9`) and the two desktop tests with `= 99` stay invalid.
- E-5 confirmed (7 literals, see table). E-6 confirmed: the only JSON fixtures that hold the configuration object are tests/fixtures/read_v1/repo_inspect.json:5, repo_list.json:9, cli_v1/cli_repo_inspect.json:5, cli_repo_list.json:9.
- E-7 confirmed (11 to 12 when the helper gives `integration`).
- E-8 confirmed and widened: `published_enumerations_list_exactly_the_registered_values` also counts every `enum` keyword of every published schema, so each of `integration`, `forge.kind` and the new `type` must be registered or the equality `published == enumerations.len() + 1` fails.
- E-9 confirmed: 4 calls at src/repository/read/tests.rs:415, 422, 428, 434, all inside `a_stored_configuration_this_build_cannot_read_is_invalid_not_valid` (400).
- E-10 confirmed (70 to 71; messages array). E-11 retired, confirmed (both literals unchanged).
- E-12 confirmed (68 to 69). E-13/E-14 confirmed: `code_differences` (mutation_contract.rs:1854ff) fails a code that has a golden and a reason, and fails a code with neither. E-15 confirmed. E-16: confirmed as needed, see D-C.

### D.1 Hits the design does not list (proposed E-17 onwards; none removes or weakens an assertion unless stated)

**D-A. Exhaustive match on `MutationDataDto`, no wildcard (compile break in nearly every integration target).**
- tests/support/mutation.rs: `ticket_data` (672), `document_data` (703), `remote_data` (838), `operation_data` (869), `key_data` (900), `identity_data` (931). Each lists every variant and panics for the wrong one. Edit: add `MutationDataDto::PrimaryLocalCommits(_) => panic!("a primary-local-commits refusal's data"),` to each (6 arms).
- tests/support/repair.rs:333-358 (`repair_data`): add `| MutationDataDto::PrimaryLocalCommits(_)` to the `Some( … )` panic group.
- src: src/repository/mutation/dto.rs:2373 (`authority`, exhaustive): add the variant to the `=> None` group (the design already names `authority`).
- Impact: `tests/support/mutation.rs` is compiled into the `cli` target and all targets that declare `mod support` with `pub mod mutation` (tests/support/mod.rs:30), so the whole fast tier would not compile after Task 7 until these are edited. They must land in Task 7 with the variant (the plan's Task 8 file list and Task 7 file list omit them).

**D-B. Exhaustive match on `SynchronizationError` in an SSH-harness target.**
- tests/comment_publication.rs:969-987 (the second match, an observation-byte table keyed by variant, no wildcard; the first match at 596-619 has `_ => 199` and is fine). Edit: add `SynchronizationError::PrimaryHasLocalCommits { .. } => 120,` (numbers 100-119 are used). Needed in Task 6 (compiles the `comment_publication` target). Task 6's file list omits it.
- src exhaustive matches already named by the plan: sync.rs Display (about 419-440), conflict_read.rs:811-837, outcome.rs:197-234. conflict_read.rs maps the new variant to `InternalError` with the other "not a read error" variants; outcome.rs maps it to `ResultCode::PrimaryHasLocalCommits`.
- Wildcard matches that need no edit: promotion/run.rs:1163 (`_ => Pub::RecoveryRequired`), poll.rs:259, comment_publication.rs:257, bind/conflict.rs:288 (calls `synchronization_error_code`).

**D-C. SSH case envelope check in tests/mutation_remote.rs `call()` (lines 785-801).**
`match value["data"]["type"].as_str()` chooses the data schema for `item sync | repo sync | operation resume`; any unknown tag falls to `_ => SYNCHRONIZATION_SCHEMA` and `schema::check_published(...).expect(...)` panics. Edit: add `const PRIMARY_LOCAL_COMMITS_SCHEMA: &str = "primary_local_commits.schema.json";` (beside 807-811) and `Some("primary_local_commits") => PRIMARY_LOCAL_COMMITS_SCHEMA,`. Without it the new SSH case fails inside the helper, before `assert_golden`.
E-16 detail: `assert_golden` only rewrites fixed pointers; the list needs a loop over `/data/local_commits/*/oid` building `replaced` pairs. The commit is `commit(...)`/`advance` with message "owned fixture child" (summary of every fixture commit), a value that will be in the golden; fine.

**D-D. Task 9 and the audit union.**
- A `repo sync | primary_has_local_commits | none | 3 | mutation_v1/repo_sync_primary_has_local_commits.json` row in docs/RFC/cli-contract.md (D-1) must satisfy `every_row_s_code_is_one_its_verbs_answer_with_by_the_audit` (tests/cli/recovery_tables.rs:535): `codes_of("repo sync")` is `union("C4")`, read from the fenced block at docs/research/wave-03-api-audit.md:744-746. So D-4 must add the code to that C4 union block (and the prose "48 library codes" at line 740).
- That changes `tests/cli/collaboration_exits.rs:323-337` (`the_c4_table_and_reasons_account_for_every_code_of_the_audit`): `assert_eq!(library.len(), 48)` becomes 49, `assert_eq!(expected.len(), 50)` becomes 51, and the code needs exactly one row in `ROWS` or `REASONS`. A `REASONS` row (reason: no CLI fixture; the library SSH case in tests/mutation_remote.rs pins it) avoids a `cli_remote` SSH case, which `How::Remote` would require (and which would add run time against AGENTS.md's budget). Edit E-17: count constants plus one `reason("primary_has_local_commits", 3, "…")`. The test is in the `cli` target (fast tier); plan Task 9 Step 3 runs only `cli skills::` and `cli recovery_tables`, so Step 3 should add `cli collaboration_exits`.
- Also `every_recoverable_code_of_an_earlier_cycle_s_union_has_a_row_or_a_reason` (recovery_tables.rs:600): with the code in the union, a `repo sync` row (or a reason) is required: D-1's row satisfies it.

**D-E. Constructor fan-out of `insert_operation_with_kind`.**
Adding a `pull_only` parameter to `insert_operation_with_kind` touches 5 test calls (state_tests.rs 344, 464, 1019, 1040, 1225) and 2 src calls (reservation.rs:1635, state.rs:6595). Keep `insert_operation` unchanged (inserts 0): it has 2 non-test callers (context_retirement/authority.rs:360, 484) and about 17 test callers. Those 5 test edits are mechanical (add `false`/0); they keep every assertion.

**D-F. Other result-code count/ordering sites.** Only src/results_tests.rs:34 (`[..; 70]`) and the message array carry counts. Tests that loop over `ResultCode::ALL` need no edit: results_tests.rs 491, 858 (class-driven), read/tests.rs 158 and 360, outcome_tests.rs 1574, 1659, 1873, 2073, 2176, 2296 (checked: the new code, class Blocked, passes each), tests/mutation_contract.rs:1817, tests/cli/recovery_tables.rs:275 (lookup), desktop operations_tests.rs:758, dto.rs 1077/2149 (typed lookups).

**D-G. Migration tests that reopen with markers present (risk for Task 3's design, not a current break).**
- The design says the migration is marker-gated with "re-checks the marker and the columns". Existing tests reconstruct older `remote_operation_records` envelopes and reopen while every registry marker, including a new `wave_06_pull_request_mode_v1`, stays committed: `cycle04_poll_migration_preserves_terminal_rows_without_inferred_publication` (src/repository/remote/state_tests.rs, `DROP TABLE remote_operation_records` at 2269, recreate old schema 2293, reopen 2316, then `read_operations`); `poll_kind_direct_sql_changes_are_denied_and_preserve_the_entire_row_after_reopen` (state_tests.rs:329-460, DROP COLUMN poll_kind and its triggers); tests at state_tests.rs 476, 559, 800-802, 966-971, 1072; reservation_tests.rs:790 (`DROP COLUMN poll_kind`); comment_publication_tests.rs:1953 (`a_legacy_registry_migrates_attribution_columns_and_old_rows_keep_null`, drops attribution columns; it passes if `pull_only` is added before them, because expected = legacy columns + attribution columns). With a marker-only guard, the first group would reopen with no `pull_only` column and no trigger; later INSERTs (which will name `pull_only`) would fail. This is why the ADD COLUMN and `CREATE TRIGGER IF NOT EXISTS` must be keyed on the actual schema (column present? trigger present?), as `migrate_item_read_schema` and the `poll_kind` code do, with the marker gating only the dirty marks. Inference: no existing test fails today; the risk is the new design's behaviour.
- SQLite caveat for new tests: dropping `pull_only` (to simulate a pre-migration registry) requires dropping its trigger first, as `remote_poll_kind_immutable` is dropped before `DROP COLUMN poll_kind` (state_tests.rs:420).
- `operation_select!` (state.rs, used by `read_operation`/`read_operation_rows`) names columns explicitly; do not add `pull_only` to it, or the cycle04 test's `read_operations` fails on its recreated table. The design's separate read helper avoids this.
- Open-time cost: the new migration must check (read-only) before it takes a write lock, or `migrated_binding_audit_does_not_promote_a_concurrent_read_snapshot_to_writer` (src/repository/comment_publication_tests.rs:4) and the other concurrent-open tests (tests/shared_key_registry.rs 861, 929) can see a lock or `SQLITE_BUSY_SNAPSHOT`.

**D-H. Tests that migrate an older registry then assert clean or current (the new dirty marks).** None currently breaks. Every existing older-registry test either starts from `create_cycle_04_registry` (tests/support/mod.rs:687, used at recovery_foundation_gate.rs 549, 746, 781, 961, 1078), whose `cycle_context_basis` migration already marks every context dirty, or re-opens a registry that already carries the new marker. `recovery_foundation_gate.rs:1075` pins the markers committed before the recovery migration (list at 1135-1143); the new marker is committed after it (last), so the list is unchanged even on that failed open. Hand-built old schemas at tests/discovery_rebuild.rs:3265ff and tests/repository_enablement.rs:481-600 only check tables exist; the table list is unchanged because the migration adds columns only (no new tables). The new Task 3 test is the only test that observes the dirty marks.

**D-I. Tests that pin columns or triggers of `remote_operation_records`.** None pins the full column list or trigger set. Column-agnostic readers: state_tests.rs:329 (`poll_kind_row`, by name), promotion/run_tests.rs:6473 (`remote_row_image`), context_retirement/authority_tests.rs:1647 (`SELECT *` zip), tests/support/privacy_rows.rs:77-87. Raw inserts name their columns (tests/support/operations.rs:86, 135; tests/discovery_rebuild.rs:3555; state_tests.rs:2310). One positional insert at recovery.rs:4396 (`INSERT INTO remote_operation_records VALUES (?1)`) is in a test that builds its own one-column table. Index/trigger name queries (context_retirement/migration_tests.rs, state_tests.rs 4586, 4682) are about other objects.

**D-J. Callers of `synchronization_reserve_owned`, `reserve_remote_operation_kind`, `insert_operation_with_kind`.** Listed in section A. Additionally the public `reserve_remote_operation` is called directly (not through `synchronization_reserve_owned`) by promotion/run.rs:1433 (a lifecycle child; stays direct, inserts 0) and by the cfg(test) helper at sync.rs:7672.

**D-K. Other.**
- `docs/RFC/cli-contract.md` is parsed by tests/cli/inventory.rs (taxonomy table, lines 25, 355-408) and recovery_tables.rs; editing the `repo sync` taxonomy row text is safe only if the verb cell stays as it is.
- `tests/cli/skills.rs` (`skill_commands_are_catalogued`, `skill_json_examples_match_their_input_schema`) parse shipped skills; D-5's edits must keep commands and JSON valid.
- tests/read_repository.rs `configuration_reads` (1226) asserts `inspected == listed` for invalid configs and `inspected == listed` for missing; both DTOs will be built by the two builders, so the new fields must be filled the same way (null for invalid and missing). Valid rows are not compared there.
- tests/mutation_replay.rs:400 compares a whole-registry dump before and after; it reopens an already migrated registry, so the new marker is in both.

## E. CI test lists

Confirmed: `.github/workflows/build.yml:117` lists the native test targets and `:129` the desktop run. The Cycle adds no test target: new tests go into `--lib` (forge::tests, remote tests, results), `--bin manyhands-cli`, and existing targets `canonical_foundation`, `repository_enablement`, `read_repository`, `read_contract`, `mutation_contract`, `mutation_remote`, `cli` (all already in the list; `ticket_closure`, `document_promotion`, `comment_publication` also present). AGENTS.md's fast-tier list and SSH table also need no change. No edit to the workflow.

## F. Confirmed facts versus inference

Confirmed (read at the cited lines): everything in the table except the rows marked "Inference"; the counts (14 literals, 7 DTO literals, 68/7/3, 70, three `graph_plan` calls in `synchronize_remote_observed_inner`); module paths; the predicate; the recovery arm; the exhaustive matches in D-A and D-B; the dispatch in D-C; the audit/union chain in D-D; the marker-independence of the `poll_kind` column add (state.rs:160-174).

Inference (not run): the byte layout of a serialized `[forge]` table (Task 1's test pins it); that a `Final(Cancelled)` journal makes the new refusal's record finish with `recovery: []` (the existing `settle.rs:55` and the `unresolved_history…` test support it, but only a run confirms); that the dirty-mark migration changes no existing assertion (D-H is by reading, not by running the fast tier); privacy and effects points below.

## G. Contradictions with, or gaps in, the design and plan

1. Design Fact 2 (16 literals) versus 14 at HEAD (A-1). No effect on R-1.
2. Design E-3: `tests/read_repository.rs:681` does not break (no `primary_branch`); the edit is optional.
3. Plan File Map, Task 7 and Task 8 lists omit `tests/support/mutation.rs`, `tests/support/repair.rs` (D-A) and `tests/mutation_remote.rs` `call()` (D-C); Task 6's list omits `tests/comment_publication.rs` (D-B). R-12's reason for running `comment_publication` ("their test files build configurations the serializer validates") is not the only reason: its source stops compiling at Task 6 without the edit.
4. Plan Task 9 Step 3 omits `tests/cli collaboration_exits`, which D-1/D-4 break (D-D). The design's Document Amendment D-4 reads as editing `repo sync`'s code list only; the test chain requires the union block to change too.
5. Design Part B says the migration "re-checks the marker and the columns first"; the existing tests in D-G need the column and trigger adds independent of the marker.
6. Design Part D, "its effects are `not_requested` throughout", versus O9 (refusal after the local fast-forward, nothing pushed): the local primary has moved, so `not_requested` write/integration effects and `local_change = false` understate it. Today's `stopped` builds effects from `effects_so_far(publication)` (bind/sync.rs about 1228) and sets `local_change` only for a merge commit. A fast-forward is neither, so the contradiction is mild; the design should say that is intended. Inference.
7. Privacy: `MutationDataDto`'s doc (dto.rs:993) says data "never holds a body or a source", and the SSH suite treats commit messages as private (`MESSAGE` probes, tests/mutation_remote.rs:2006-2037, 2750, `control_prints_the_commit_message`). The refusal publishes commit summary lines in `data` and human output. The design escapes control characters but does not rule on content. The new SSH case must not register the local commit's message as a probe, and the privacy rationale for printing summaries should be recorded. Inference; owner's call.
8. `served_tip` is `Synced`'s method (merging.rs:212), not a harness helper; the plan's reference to it is only usable inside `mutation_remote/merging.rs`. There is a second case table, `merging::CASES`, chained at mutation_remote.rs:76; the new case goes in the top-level `CASES` (line 101) as R-9 says.
9. `state_tests.rs` is module `persistence_tests`, so the plan's "reservation tests near insert_operation_with_kind" has two possible homes (persistence_tests for the insert itself; reservation::tests for `reserve_remote_operation`); `--lib` filters differ (section B).
10. `ForgeDto` must be added to the `pub use` list at src/repository/read/mod.rs:51 (not in the plan's Task 4 file list; `pub use read::*` at repository.rs:118 then exports it).
11. `push_call` has a second caller (republish, sync.rs:8294) and `ended_if_unresolved` a signature pinned by sync_tests.rs:16754; widening either must keep the 4-argument call at 16823 compiling or edit it (mechanical).

## Task 1

- Commit `745fdf6b` feat(config): integration mode, forge setting and format version 2. Review approved; 4 Minor findings, none fixed (below).
- Tests run, each by exact name, red first then green: `repository_config_mode_and_forge_rules` and `repository_config_written_bytes` (`--test canonical_foundation`), `publication_remote_selection_keeps_mode_and_forge` (`--test repository_enablement`), and the edited tests of E-1, E-2 and E-4.
- Named edits applied: E-1 (2 tests in `src/repository/discovery.rs`), E-2, E-4 (2 cases). E-3 dropped (Task 0).
- Ruling: the URL check's host rule is loose (`https://@` passes) and is kept — the design asks only for a non-empty host, and `web_url` only builds links — a malformed `web_url` yields a broken link, never a connection.
- Minor findings left as they are: the version check inside `validate_mode_and_forge` is unreachable from its callers (defensive); the P3 test stages the configuration in the index without a comment; `as_str` values are checked only through parse and write.

## Task 2

- Commit `d8abc5d7` feat(forge): forge recognition, default mode and pull request links (`src/forge.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`). Review approved; 4 Minor findings, left as they are: a drive-letter or bracketed scp form parses to an odd host (no cloud match); `.` and `..` path segments pass unencoded; no encoding row for the Data Center key or the target; the dependency's position in `Cargo.toml`.
- Tests run: `forge::tests` (4 tests: `remote_locations`, `recognition_and_default_mode`, `pull_request_urls`, `human_lines`), red against `unimplemented!()` stubs, then green; `scripts/check-cli-no-gpui.sh` clean.
- A7 (decision 7): the owner chose a placeholder repository over a real one. The builder's Bitbucket Cloud link for `git@bitbucket.org:acme/widgets.git`, primary `development`, is `https://bitbucket.org/acme/widgets/pull-requests/new?source=manyhands/ticket/01M87683441PRAYQJQBQBG96YY&dest=development`; the owner confirmed its shape by eye ("Looks right"). The link was not opened against a live repository.

## Task 3

- Commits `2cf5bb40` feat(index): store the repository's mode and forge; `3f397aca` test(index): pin the schema-checked re-add of the pull-request mode. Review approved (0 Critical, 0 Important); one fix round for three Minor findings, re-reviewed and approved.
- What it does: `migrate_pull_request_mode` runs after `context_retirement::record::migrate` in an immediate transaction with a read-only precheck; it adds the four `configuration_observations` columns, `remote_operation_records.pull_only` and the `remote_pull_only_immutable` trigger by checking the schema; the marker `wave_06_pull_request_mode_v1` gates only the primary contexts' dirty marks. `persist_configuration` writes the columns; `admin::stored_configuration` reads them through `StoredMode` (a null `integration` reads as direct), and discards them until Task 4.
- Tests run by exact name, red then green: `pull_request_mode_migration_adds_columns_once_and_refills`, `a_valid_stored_configuration_reads_its_mode_and_forge`, `a_marked_registry_that_lost_the_pull_request_mode_schema_gets_it_back_unmarked`; and, unchanged and green, 21 existing migration and reopen tests (the cycle04 poll migration, the `poll_kind` tests, the reservation test near line 790, `a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty`), `registry_creates_repository_and_discovery_metadata_schema`, `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly`, and two `shared_key_registry` and two `read_repository` tests.
- Named edit E-9 applied (4 calls pass `StoredMode::default()`).
- Ruling: a stored `integration` or forge kind this build does not know reads as null in Task 4's configuration object — a later build's value in a shared registry must not break this build's JSON, and the schema allows null — mixed builds show null until the row is rewritten (K-8).
- Ruling: a registry that has the marker but lost a configuration column gets it back as null and reads as direct until its next re-persist — the marker gates only the one-time dirty marks — a hand-damaged registry shows direct until the primary changes.
- Minor findings left as they are: the duplicated schema-check loops beside `missing_item_read_schema`; the positional `StoredForge` tuple (Task 4 may replace it); the old misleading comment in `a_registry_from_an_older_build_migrates_and_its_contexts_read_dirty`.

## Task 4

- Commit `7112900c` feat(read): report the repository's mode and forge. Review approved; 3 Minor findings left as they are: no test pins a forge with both a kind and a template (the kind prints); no `Forge:` line for an unknown kind with only a `web_url` (only from a later build's row); a doc comment could also name the unknown `forge.kind`.
- Tests run by exact name, red then green: `inspect_and_list_report_mode_and_forge` (`--test read_repository`), `repository_inspection_names_mode_and_forge` (`--bin manyhands-cli human::tests`), `a_stored_mode_or_forge_kind_this_build_does_not_know_reads_as_null` (red by mutating `known_integration`); and green: `published_enumerations_list_exactly_the_registered_values`, the four golden tests, the CLI inventory test, the escape-controls test, 18 `read_repository` neighbours and 17 `repository::read::tests`. Desktop `check --tests` clean.
- Named edits applied: E-5 (7 literals), E-6 (the four goldens gain only `"forge": null` and `"integration": "direct"`), E-7 (11 to 12 lines), E-8 (`integration` and `forge.kind` enums).

## Task 5

- Commit `afd1beff` feat(results): primary_has_local_commits. Review approved; 1 Minor finding carried to Task 7: the "no action" assertion lists in `src/repository/mutation/outcome_tests.rs` do not yet name the new code.
- Tests run: `results::tests` red after E-10 (2 failed), green after (49 passed); by exact name, green: `published_enumerations_list_exactly_the_registered_values` (`--test read_contract`), both `every_result_code_has_a_*` tests (`--test mutation_contract`), `recovery_tables` (`--test cli`, 5 tests). The CLI contract needed no row yet; Task 9 amends it.
- Named edits applied: E-10 (71 codes and the message), E-13 (both exclusion tables, reason "golden written in Task 8").

## Task 6

- Commits `c2398f7a` feat(sync): pull-only primary synchronization; `2c9189c9` test(sync): a direct-mode repo sync stores pull_only 0. Built test-first with a design checkpoint; review approved (0 Critical, 0 Important), one fix round for Minor 1, re-reviewed and approved.
- Design checkpoint: the six hooks (where a new operation's mode is derived and inserted; the read helper on every take-up path; the two plan sites; `push_call`; the wrap and `keep`; the settle paths) were checked against Required Outcomes O1 to O15 and the source.
- Rulings at the checkpoint:
  - Ruling: when the push endpoint advertises no primary, a pull-only refusal lists against nothing hidden (the candidate's 20 newest commits) — relative to the push endpoint every commit is missing, so the list is true — a long history lists 20 with `complete` false.
  - Ruling: when the push endpoint is strictly ahead of the candidate (a race after the fetch, or a separate push URL ahead), the refusal is `primary_has_local_commits` with an empty list and `complete` true, ending the operation and holding nothing; a new Known Limit, K-9 — `push_rejected` or `external_change` would hold the repository, and `AlreadyCurrent` cannot be recorded because classification requires the advertised tip to equal the candidate — the code's message is not true in this rare case, and a retry fetches again.
  - Ruling: the two defensive guards stay (a pull-only operation holding a merge answers `recovery_required` in the `local_owner` and unfinished-window paths) — fail closed if the invariant is broken — two branches no test reaches.
  - Ruling: the mode test stays in `sync_tests.rs` — it needs a real synchronization with a repository and a configuration — none.
- Tests run by exact name, red then green: `local_only_commits_lists_newest_first_and_caps_at_twenty`, `o3_o4_o8_pull_only_refuses_local_commits_after_a_plan`, `o9_o10_push_call_never_pushes_in_a_pull_only_operation`, `o11_o12_o13_the_mode_is_inserted_once_and_read_back` (with the direct-mode rows), and E-21's and E-22's tests; green: 13 persistence tests and 11 reservation tests at the edited call sites, and 4 sync tests on changed paths. `comment_publication` (E-18) compile-checked only; it runs at review-ready.
- Named edits applied: E-18, E-21 (`every_synchronization_error_has_its_code`, 19 to 20), E-22 (two rows in `unresolved_history_ends_the_operation_and_frees_the_reservation`), and the 5 `insert_operation_with_kind` test call sites.
- Not reached by any test, checked by reading (review ⚠️): the refusal at the push site after `LocalFastForwarded`, and the resume after `PushVerified`; no library test reaches `push_call`, and the SSH case covers O3.
- Minor findings left as they are: duplicate trigger assertions (plan-mandated); the `keep` line is defensive and untested; one more registry transaction per synchronization; the `ConflictPending` mapping repeated three times.
- Document amendments owed at Task 9: design O9 and Contract 9 for the two push-endpoint rulings, and K-9.
