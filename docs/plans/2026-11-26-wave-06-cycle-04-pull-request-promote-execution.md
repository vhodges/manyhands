# Wave 06 Cycle 04 Execution Ledger

Ticket vh-rphcs (`01M87684C9GM9BJP5QQ429KMAK`). Plan: `docs/plans/2026-11-26-wave-06-cycle-04-pull-request-promote-implementation.md` (managed document `01M8BXXH54RS9M6NX5XW9ARX9A`, approved 2026-11-26 at revision 7, `333aeac5`). Design: managed document `01M8BXXGDN9HPD8BM9SJTHHVB2` (`fc405d54`). Cycle document: managed document `01M8BXXFPDGS3A5MJBM26JEEX3` (`c073ebe8`). The three documents live on their own branches until they are promoted at closure (design D-6); this ledger stays on the ticket branch (Cycle document decision 8). Execution: subagent-driven, implementers on Sonnet 5.5 at high effort, one fresh reviewer per task. One line per paragraph.

## Approval

- The owner approved the three documents at revision 7 without a further round, the named test edits E-1 to E-14, subagent-driven execution with Sonnet 5.5 at high effort, and implementation (Cycle document decision 16), on 2026-11-26. The documents were saved with status approved and published (Cycle document `c073ebe8`, design `fc405d54`, plan `333aeac5`). Approval comment `01M8DB478S2PJ7EGBHH3HPFRAC`, published at `cc3be53a`.
- Notes left at approval: vh-ssepw (`01M8DAX2D6CGW2GYJY04QJRB3P`, published; the comment merged main into its branch at `13103042`), vh-p7st2 (`01M8DAX3584DNP5NW9KY9P8SNA`, published). vh-gy8gz is closed and takes no comment; its note stays in the Cycle document.

## Task 0

- Step 1: vh-gy8gz closed at 2026-11-26T22:29:45Z and on `origin/main` (`6bc4b69c` is an ancestor). Base: `origin/main` `8e5d9832`. HEAD before and after the merge: `aee99255` (already up to date; merged at `0c5027e0` during planning, never rebased). The approval comment then moved the branch to `cc3be53a`.

## Pre-flight scan (before Task 1)

Pairs of tasks that share a file or an interface:

| Tasks | Shared | Produced against consumed | Found |
| --- | --- | --- | --- |
| 1, 4 | `promotion/record.rs` | 1 builds the rebuild and its columns; 4 adds the fence's query and may amend the migration | Agrees: Task 1 Step 0 lets Task 4's checkpoint amend the migration on the branch |
| 1, 2, 3 | `promotion/lifecycle.rs` | 1 `path()`, `answered()`; 2 `Preview`, `runs_context_child`, `require_identity`; 3 `branch_step`, `end_branch`, `Outcome` | Agrees; different items |
| 2, 3, 4, 5 | `bind/lifecycle_verb.rs`, `bind/lifecycle.rs` | 2 dispatch and preview effects; 3 `Called::Branch` and the answer; 5 the data | Agrees: "until Task 5 a pull-request promote may answer without data" |
| 2, 6 | `promotion/observe.rs` | 2 `primary_holds_document`; 6 `document_link_outstanding` against it | Agrees |
| 3, 4, 7 | `promotion/run_tests.rs` `pull_request` module, the shared fake publisher | 3 creates both; 4 and 7 add rows | Agrees |
| 5, 8 | the mutation schema, `DATA_ROOTS`, inventory | 5 the preview schema; 8 the mutation schema with its first golden | Agrees (revision 7's move) |
| 5, 6 | `mutation/dto.rs`, `bind/sync.rs` | 5 the new DTOs; 6 reuses Cycle 03's `PullRequestLinkDataDto` | Agrees |
| 3, 5 | `closure/checkpoint.rs` `give_link`/`branch_link`, the way out | 3 the kind parameter; 5 the route | Agrees |
| 9, 5 | `tests/cli/recovery_tables.rs`, the contract | 9 writes no row (E-10); 5 adds no row | Agrees |

Each task against itself:

| Task | Found |
| --- | --- |
| 0 | Agrees |
| 1 | Agrees: the tests name the columns and steps the Files list implements |
| 2 | Agrees |
| 3 | Agrees; it adds `DocumentPromotionResult::Branch`, whose exhaustive matches (CLI, desktop) get arms in the same task (Intermediate commits) |
| 4 | Agrees; its mechanism is chosen at Step 3, after the red tests |
| 5 | Agrees; its goldens carry no `pull_request_promotion` data (the prepare's `already_applied` has none, R-4), so the mutation schema waits for Task 8 |
| 6 to 10 | Agree |

- Ruling: "Commits ... trailer `Co-Authored-By: Claude Opus 5.5`" applies to implementer commits too, although implementers run on Sonnet 5.5 — the plan states one trailer and the session's attribution rule names it — a wrong trailer is cosmetic.
- Ruling: the static gates before a commit that touches Rust are `cargo check --all-features`, `fmt --check` and `clippy --all-targets --all-features -D warnings`, which also compile the desktop; the `--features desktop-test-support --tests` check runs only where the plan names it (Task 5) and in any task that touches `src/desktop/` — the plan names it once — a missed desktop test compile shows at Task 10's gates.

## Task 0 baseline

- Baseline at `cc3be53a` (18:50 to 18:55): `cargo clippy --all-targets --all-features --locked -- -D warnings` clean (1 min 38 s), `cargo fmt --check` clean, `cargo test --all-features --locked --lib --no-run` built (2 min 4 s). No test was run.

## Task 0 Steps 2 and 3: the references

By a read-only research agent at `aee99255` (Appendix A, verbatim), 220 rows: 201 found, 4 moved, 4 changed (D-4, D-5 and the two parsed-document rows), none renamed, none missing.

- Step 2 (U-1): all 23 of Cycle 03's names exist with the cited shape; nothing stops for the owner. Shapes the tasks need: `BranchEnd<'a>`, whose `ContextChild` carries `&'a SynchronizationError`; `give_link` (`closure/record.rs:1461`) hard-codes `"ticket"`, `branch_link` (`closure/checkpoint.rs:1036`) takes its kind from the file's `KIND`, and `branch_data` and `fill_complete_read` hard-code the ticket kind; `upsert_link` rewrites every column, so no helper updates `given_at` alone yet; the withdraw prepare's `already_applied` is built at `bind/lifecycle_verb.rs:623` and answered at `bind/lifecycle.rs:1545-1547`, with `data: null`, `outcome: noop`, `recovery: []`, and the CLI boundary (`execute.rs:307-311`) answers `external_change` for it at confirm; `PullRequestIo` and `Forge` are private to `closure::run_tests::pull_request` and its child `lifecycles`, and `PullRequestIo` does not use `Forge`.
- Moved: the binding's decision-33 hold `fn held` is `mutation/bind/lifecycle.rs:656` (not 396-435); `finalize_index` is `context_retirement/record.rs:1966` (not about 2044); `retire_context_for_request_cancellable` is `context_retirement/request_scope.rs:531` (`execute_retirement_request` is 598); `FetchTracking` is `TransferAction::FetchTracking`, `transport/remote.rs:19`, not in `remote/sync.rs`.
- Not located: the design's `sync.rs` about 6812 ("an ended child stores none"); 6812 is `synchronization_push_absence_check`. The behaviour relied on, `RemoteContextDeleted` from the evidence, is at `sync.rs:500` and `:4137`.
- Minor: `DocumentPromotionPreflight` appears in 13 files and has two variants; `tests/support/cli.rs` has no `golden()`, the name checks are in `assert_cli_golden` and `assert_cli_text_golden`; retirement records its digest through `ObservationConfiguration::endpoint_digest()` (`source.rs:66, 156`), while `endpoint_digest_of` is `remote/observation.rs:125`; `SyncBinding::link_given` stamps `given_at` with `OffsetDateTime::now_utc()` (`bind/sync.rs:1172`), not an injected clock.
- The design's new names (`wave_06_promotion_modes_v1`, `PromotionAnswer`, `BranchPromotionOutcome`, `PullRequestPromotion*`, `primary_holds_document`, `document_link_outstanding`, the new digest domain) are absent from `src/`, `tests/` and `schemas/`.
- Step 3, test module paths: `repository::promotion::record::tests::`, `repository::promotion::observe::tests::`, `repository::mutation::bind::sync::tests::`, `--bin manyhands-cli lifecycle::tests::`, `results::tests::` (library). `promotion/run_tests.rs` has no `pull_request` submodule yet: Task 3 adds it, and its filter is then `repository::promotion::run_tests::pull_request::<test>`. `mutation_remote` is `harness = false`: the case name is a positional filter with `--exact`. The targets `read_items`, `read_contract`, `mutation_contract` and `cli` exist (`tests/cli.rs`; `fixture_directory_is_exactly_the_registered_cases` has no module prefix).
- Step 3, the parsed documents: `tests/cli/recovery_tables.rs` reads `docs/RFC/cli-contract.md` `## Recovery tables` (its family subsections and `### Codes with no golden`) and the audit's tables; `tests/cli/collaboration_exits.rs` reads only `docs/research/wave-03-api-audit.md` (the fenced block under `#### C4 exit-coverage union`, the groups after `### Wave 03 CLI Cycle C4 refresh`, and the table under `#### C5 bindings and result-set key`). `document promote`'s audit row is `M + R + C + I + T + Y + setup_pending`, 42 codes (`audit.md:549`); group Y already holds `already_applied`, `remote_branch_deleted` and `recovery_required`.
- E-14: no comment in `bind/sync_tests.rs` (`:508-514`, `:566-571`) says documents never get a link; the document case at `:587-594` asserts no data and no row for a document with no link row, which Part F keeps. E-14 needs no edit unless Task 6 finds otherwise.
- D-4: `skills/manyhands-overview/references/concepts.md` about 25 has no "until Cycle 04" sentence, so there is nothing to remove there; the other sentences are at the cited lines, `.agents/skills` mirrors them (its `manyhands-recovery` at 179 and 183), and six files of `.agents/skills` already differ from `skills/`.

### Rulings

- Ruling: E-10 covers both parsed documents, the contract and the audit; D-5 changes `document promote`'s audit row (its codes, or its count of 42) only for a code the pull-request promote can answer that the row's groups lack, and otherwise adds its note to the row's design-anchor cell; any change of codes or count is checked by both `recovery_tables` and `collaboration_exits` in Task 9 — the design's "none expected" named only the contract — if wrong, Task 9's two tests fail and the row is fixed then.
- Ruling: the design's line numbers that moved are read at the lines above; none changes a design claim — they are locations only — none.
- Ruling: E-14 stands as "read the comment; no edit expected"; Task 6 records whether it edits anything — the comment the design expected is not there — a missed comment is a stale doc line.
- Ruling: Part F's `link_given` keeps `OffsetDateTime::now_utc()` for `given_at`, as Cycle 03's ticket arm does, since Part F writes no row and so stamps nothing for a document — a document arm that needed a clock would be found by Task 6's `given_at` assertion — none.

## Task 0 Step 4 and Step 5: the premise sweep and CI

By a read-only research agent (report in the controller's workspace, `task-0-sweep.md`), its three new rows checked by the controller against the source. None removes, weakens or reverses an assertion's intent: each changes only a struct literal's field list.

- **E-15** (Task 1) `src/repository/promotion/lifecycle_tests.rs:90` `promotion_steps_map_to_the_shared_step_view`: the full `PromotionRecord { .. }` literal gains Part A's new fields with direct values; the 13 steps and the shared view are unchanged. It is the only `PromotionRecord` literal outside `record.rs` `decode` (`run_tests.rs` 328 and 610 and `record_tests.rs` 13 and 1368 are helpers that read or begin a record).
- **E-16** (Task 2) `src/repository/mutation/bind/lifecycle_tests.rs:1117` (`a_branch_record_s_answers_name_the_ticket_s_branch_and_a_direct_one_s_name_primary`, 1082): the `Ready { .. }` literal gains Part G item 12's pull-request facts, and follows `primary_oid` if it becomes optional. Drops out if Task 2's checkpoint leaves `Ready` unchanged.
- **E-17** (Task 2) the same file, 1260 (`a_branch_record_previews_its_own_effects_and_details`, 1250): the same change as E-16.
- Pointer for Task 1's checkpoint: the free function `promotion::record::begin_record` has about 32 test callers (21 in `record_tests.rs`, the rest in `reservation_tests.rs`, `sync_tests.rs`, `lifecycle_tests.rs`, `admission_tests.rs`, `review_tests.rs`, `context_release_tests.rs`, `recovery.rs`); Cycle 03 kept `closure::begin_record` and added `begin_branch_record` beside it. A signature change there would be a mechanical edit to each, logged as E-18 onwards.
- Pointer for Task 3's checkpoint (U-2): lifting Cycle 03's `PullRequestIo` and `Forge` into shared test support would move code out of two closure test files, with no change of intent; a promotion twin needs no edit.
- E-1 to E-14: each matches at its stated line. E-12's one exhaustive `DocumentPromotionPreflight` match in tests is `tests/document_promotion.rs:301`.
- The column scans `tests/ticket_closure.rs:449` and `tests/document_promotion.rs:886` are table-name lists for a `SELECT *` privacy scan, and `tests/read_status.rs:1798` inserts with an explicit column list into which the new columns default: no edit.
- No document golden carrying `"pull_request": null` changes; the only test that inserts a link row is `tests/read_items.rs:6159` (E-1).
- Skills: D-4's list is complete, seven "until Cycle 04" sentences in each of `skills/` and `.agents/skills/` and the "not usable end to end" wording; no test pins them.
- Step 5: the design adds no `[[test]]` and no top-level `tests/<name>.rs`; `.github/workflows/build.yml`'s test list (line 117) already holds every target the design extends. No CI change.

## Task 0 status

- Task 0: complete. Steps 1 to 5 above; Step 6 commits this ledger, publishes the branch and adds the start comment (recorded under Task 1).

## Task 1: The Record's Mode

- Base `679b2661`. Implementer: Sonnet 5.5 at high effort.
- Design checkpoint (Step 0), written by the implementer against the source (Appendix C, verbatim), approved by the orchestrator on 2026-11-26 as written. All six of Part A's columns stay, each tied to the RR, F or C row that reads it. Nothing reaches `begin_record` in this task: its signature and its 32 callers stay, so no E-18 onwards; Task 3 adds the pull-request begin with its caller and tests that it writes the four begin-time columns. Task 1 tests pull-request rows through a test-only insert helper read back by `read_record` and saved by `save_record`.
- Ruling: `answer_code` is a closed list in SQL, not Cycle 03's GLOB form — this table's other categories are closed lists — a seventh answer needs the migration amended on the branch, which Task 4's checkpoint may do.
- Ruling: a pull-request row moves `confirmed` → `context_synchronized` → `complete` and `confirmed` → `abandoned` only; `context_synchronized` → `abandoned` is refused, unlike Cycle 03's graph — Part A names only those moves, RR2 finishes a completed child whatever else changed, and the second table starts at `confirmed` — if a Task 4 row needs it, Task 4's checkpoint adds the arm.
- Ruling: the SQL and `validate` also refuse, for a pull-request row, a primary child, a cleanup category, a publication category before `complete`, and an answer outside its step's set (`complete`: `promoted`, `already_promoted`, `primary_holds`; `abandoned`: `primary_holds`, `remote_branch_deleted`, `history_unresolved`, `context_gone`, with the refusal `reconfirmation_required`) — each follows a Part C table — Task 4 may relax one with a row that needs it.
- Ruling: `PromotionSituation { First, Again }` is defined here, stored as `first` and `again`, for Task 2's observation to reuse — the brief gives the column no type — none.
- Ruling: the four begin-time columns are all set or all null, and `save_record` holds them, the mode and the answer equal to what was inserted — a refused record written at confirm has none, one that RR4 or RR5 end keeps those it began with (Part A) — none.

- Commits: `fb666745` feat(promotion): the record's mode and stored answer; `a4586e67` test(promotion): pin the record's column checks (fix round 1).
- Tests (implementer, each by exact name, 1 test each): `promotion_modes_migration_rebuilds_a_v2_table_once` and `pull_request_promotion_steps_and_checks` (new); the E-4 tests `promotion_refusal_blockers_v2_rebuilds_an_old_bound_table_once`, `privacy_column_inventory_for_both_new_tables`, `schema_rejects_free_text_categories_steps_and_refusal_payloads` (edited), `migration_preserves_legacy_rows_is_idempotent_and_audited`, `half_created_migration_and_missing_marked_table_fail_closed`, `record_forward_transitions_and_invalid_backwards_recover` (unchanged); E-15's `promotion_steps_map_to_the_shared_step_view`; E-3's `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` (`--test recovery_foundation_gate`). Static gates clean. Red: a compile failure (133 errors), then four deliberate stubs (the pull-request `permits` on the direct graph, the no-integration-columns CHECK removed, the answer-implies-`reconfirmation_required` CHECK removed, the rebuild skipped), each failing a named assertion; in fix round 1, six column-CHECK stubs and Cycle 03's GLOB form for `answer_code`, each failing.
- Named edits applied: E-3, E-4 (three of its six tests edited, additively; three unchanged), E-15. No E-18 onwards.
- Review (Opus): spec compliant; one Important: five new free-text schema assertions were never red and hit table CHECKs rather than their column CHECKs, so the closed lists, the digest's length and the consented OID's format were untested. Fix round 1/5 (1 addressed, 0 open; commits `fb666745..a4586e67`): each bad value now runs on a row where only its column CHECK can refuse it, beside a fitting value that must be accepted; `answer_code='merged'` pins the closed list against the GLOB form. Re-review (Sonnet): addressed, no new Critical or Important.
- Deferred Minor findings (for the final review): an answered `abandoned` pull-request row with no begin-time columns is accepted by the SQL and by `validate` (a CHECK `answer_code IS NULL OR consented_context_oid IS NOT NULL` would refuse it; Task 3 or 4 may amend the migration on the branch); `primary_branch`'s bound counts characters in SQL and bytes in Rust; the `save_record` rows for the mode's and the answer's immutability pass through `validate` anyway, the refused-record nulls assertion passes because `abandoned` is terminal, and no row saves `context_synchronized` → `complete` with an answer; the comment at `record.rs:90` says the first three answers go with `complete` and the rest with `abandoned`, though `primary_holds` is allowed at `abandoned`; the row "abandoned with no refusal" (`record_tests.rs:2074`) also clears the answer; `parse_mode` copies the closure module's private parser (an `Integration::parse` in `canonical.rs` could serve both); the fix's `typeof` sub-clauses of `endpoint_digest` and `primary_branch` have no case that only they refuse.
- Pointers for Task 3 (from the implementer): replace the `unreachable!("a promotion record is always direct")` hooks; guard `parent::complete_record` and `supersede` against pull-request rows; the finish function checks `permits`.
- Task 1: complete (commits `679b2661..a4586e67`, review clean after fix round 1).

## Task 2: The Pull-Request Observation

- Base `034f4346`. Implementer: Sonnet 5.5 at high effort.
- Design checkpoint (Step 1), written by the implementer against the source (Appendix D, verbatim), approved by the orchestrator on 2026-11-26 with one correction (below). The shape: `DocumentPromotionPreview` gains `pull_request: Option<PullRequestPromotion>` (situation, link, and `R` only when it is ahead of or beside `L`); `DocumentPromotionPreflight` gains one unit variant `PrimaryHolds`; the binding's `Ready` gains `pull_request` and `Observed` gains `PrimaryHoldsDocument`, the twin of `NothingToWithdraw`, which digests as its own shape. Three exhaustive `DocumentPromotionPreflight` matches change (`lifecycle.rs` `ready`, `Promote::observe`, and E-12's `tests/document_promotion.rs:301`); E-16 and E-17 each add `pull_request: None`. The mode dispatch sits in `observe_git`, which reads the configuration on every prepare and revalidation, so `preview_document_promotion` does not change. The prepare's `already_applied` is answered as Cycle 03's withdraw answers `NothingToWithdraw`, after the binding's hold.
- Correction: no document twin of `branch_link`. `closure/checkpoint.rs` gains one helper that takes the context branch's name (the address, `forge::recognise` and `forge::pull_request_link`), `branch_link` keeps its signature and calls it with the ticket's branch, and the pull-request observation calls it with the document's branch — a twin would duplicate the function verbatim — Task 3's item 9 then only adds the link row's kind.
- Ruling: `primary_oid` keeps its type; in a pull-request preview it holds the tracking primary's tip, and nothing in the pull-request digest or (from Task 5) the pull-request preview DTO reads it — making it optional would edit `observe_tests.rs:82` and `:899`, outside the named edits — if a reader needs "absent", Task 5's DTO says so.
- Ruling: the endpoint digest is carried from Task 3, which reads it at begin — Task 2 has no reader for it — none.
- Ruling: an interim guard in `DocumentPromotionLifecycle::begin_record` answers `recovery_required` before any record for a pull-request preview, so no confirm between Tasks 2 and 3 runs a direct promotion in a pull-request repository or reaches the runner's `unreachable!`; it is pinned by `a_pull_request_confirm_stops_before_any_direct_record`, a test of this Cycle that Task 3 replaces with its own when it removes the guard — the plan has the runner in Task 3 — none.
- Ruling: until Task 5 the pull-request preview's details are the direct `Lifecycle` DTO with empty changes (Global Constraints) — a reader of the interim preview sees the wrong shape only on this branch — none.

- Commits: `bcf29349` feat(promotion): the pull-request observation and the content-proof rule; `177be9ec` test(promotion): pin the pull-request preview's effects and the held answer (fix round 1).
- Tests (implementer, each by exact name): new `primary_holds_document_by_the_content_proof` (P3: 2 documents × 4 merges × 9 scenarios × 6 states of `R`, plus a shallow repository, two merge bases and an unrelated tip), `pull_request_promotion_observation` (P2), `pull_request_promotion_digest_binds_each_field`, `a_pull_request_promotion_previews_its_three_effects_always` (`bind/lifecycle_tests.rs`) and the interim guard's `a_pull_request_confirm_stops_before_any_direct_record` (`run_tests.rs`); unchanged `digest_binds_each_field_change_blob_and_cleanup_member` and the direct observation tests that pin the code moved into the shared helper (`not_a_document`, `context_missing`, `context_branch_missing_is_blocked_without_panicking`, `context_nonconforming`, `context_dirty_tracked`, `context_conflicted`, `remote_context_deleted`, `remote_history_unknown`, `present_tracking_context_overrides_deleted_history`, `remote_digest_excludes_moving_oids`, `contained_absent_tracking_context_is_cleanup_only`, `t2_1_deletion_encoding_has_zero_big_endian_u32_mode`, `t2_1_added_executable_bit_changes_digest`); E-16's and E-17's edited tests; `--test cli lifecycle::a_context_primary_already_holds_is_only_cleaned_up` (the moved direct-effects code). E-12 is in the SSH target `document_promotion`: compiled through clippy, not run. Static gates clean. Red: stubs first, and where a stub went red only at a first row, eight mutations of the real code, each failing the intended test; in fix round 1, the effects selection removed (fails with `[ContextSynchronized, PrimaryPublished]`) and the held arm mapped to `recovery_required`.
- Departures from the checkpoint, accepted: the new struct and accessors are `pub(super)` (clippy `private-interfaces`); a merge in progress at the root also reads `context_conflicted`, because `sync::require_clean_target_state` (`remote/sync.rs:754-766`) checks the foreign-state files of both the linked worktree and its `commondir()` — the same refusal the context child's synchronization gives.
- Named edits applied: E-12 (`tests/document_promotion.rs:301` gains the `PrimaryHolds` arm), E-16 and E-17 (`pull_request: None`). No other test edit.
- Review (Opus): the observation, the content proof (R-3), the digest (R-7), the interim guard and the four named risks (the direct observation unchanged by the refactor; the root-merge blocker; R-3's reading of the local primary and `R`; no production path in a pull-request repository that moves or synchronizes the primary) check out; spec ❌ on one Important: the effects test compared a constant with itself, so the preview's pull-request effects selection and the prepare's held `already_applied` were untested (removing the selection would preview `primary_published`). Fix round 1/5 (1 addressed, 0 open; commits `bcf29349..177be9ec`): the selection is `ready_effects(&Ready)` and the held answer `unoffered_answer(&Observed)`, both called by `Confirmed::preview` and asserted for `First`, `Again`, a direct `Ready` and the held shape. Re-review (Sonnet): addressed, no new Critical or Important.
- Deferred Minor findings (for the final review): the digest test's "domain" row passes vacuously (it compares with the direct digest over other fields, so dropping the pull-request domain still passes; `observe_tests.rs:1802-1806`); the digest test lacks the remote-`None` row the checkpoint listed (`:1821`); the "no identity" dropped row never removes the identity (`:1632`); `runs_context_child`'s override copies the trait default verbatim (`promotion/lifecycle.rs:871-875` against `:316-319`); comments: `observe_tests.rs:1748` attributes the root-merge blocker to Git, `observe.rs:1548` says the root's state does not matter, `bind/lifecycle.rs:640`'s `note_observed` doc is stale; the report's mutation table skips M6; `Confirmed::preview` calls `unoffered_answer(..).expect(..)` on states its arm already narrows, and `unoffered_answer`'s `PublicationRemoteRequired` arm duplicates the early return.
- Task 2: complete (commits `034f4346..177be9ec`, review clean after fix round 1).

## Task 3: Confirm, The Answer Tables

- Base `6c46b35c`. Implementer: Sonnet 5.5 at high effort. Step 1: `origin/main` `8e5d9832` is an ancestor of HEAD; no merge.
- Design checkpoint (Step 2), written by the implementer against the source (Appendix E, verbatim), approved by the orchestrator on 2026-11-26 with one correction (below). `branch_step` and `end_branch` call a new `promotion/answer.rs` (`finish`: the first table, judged on the child's authority OID read through `child_state`, with the tracking primary as it is and the record's own `primary_branch` and remote; `end`: the second table, judged on the consented tip, `R` read as absent after `remote_branch_deleted`); the record layer's `begin_pull_request_record`, `refuse_record_on` (the confirm's mode; `refuse_record` delegates with `Direct`, so its callers do not change), `complete_branch_record` (holds the answer and link-write pairing) and `abandon_branch_record`; `pull_request_links::touch_link` renews `given_at` alone; `Outcome` is `PromotionOutcome { Direct, Branch }`, `DocumentPromotionResult` gains `Branch` and `BranchIndexPending`; the binding's `Called::Branch` holds `Branched::{Closure, Promotion}`, and `promotion_branch_code` maps `promoted` → `ok`, `already_promoted` and `primary_holds` → `already_applied`, `remote_branch_deleted` → itself, `history_unresolved` → `recovery_required`, `context_gone` → `external_change` (as Cycle 03's `ContextGone`); `RunEvent::Recording` is gated to the ticket closure on the branch path; the interim guard and its test go.
- Correction: C9's squash and C10's merge are scripted as moves of the tracking primary made by the child's own fetch (the fake's `fetched`), not before the confirm — a tracking primary that already holds the document at the runner's revalidation is C19's refused record, not C9 or C10 — the checkpoint's wording ("then a squash on the tracking primary") left it open.
- Ruling: U-2 is a promotion twin of `PullRequestIo` in `promotion/run_pull_request_tests.rs` (module `run_tests::pull_request`), not a lift — Cycle 03's is built on the ticket fixture with about 100 call sites, and a lift would change two Cycle 03 test files — Task 7 may lift the bare-repository `Forge` if both users need it.
- Ruling: Part G item 9 is met by a document twin (the design allows one): the record layer writes the document's row through `pull_request_links::upsert_link` with kind `document` and renews it through `touch_link`; `closure/record.rs` and `closure/checkpoint.rs` do not change — `give_link` is a four-line wrapper that writes kind `ticket` — none.
- Ruling: `BranchPromotionOutcome` carries `operation_id` beside `answer` and `publication`, whose OID is the record's `context_oid` — both direct and branch closure outcomes carry one — none.
- Ruling: an `again` record whose link row is gone when it finishes answers `already_promoted` and renews nothing — the first table keys the answer on the consented situation, and the row can only go by a retirement, which Task 4's fence holds — a null link in that answer.
- Ruling: no extra guards on `parent::complete_record` and `supersede` — the first refuses unless `branch_deleted`, which a pull-request row can never reach, and the second is reached only for a supersedable cleanup, which a pull-request row never has — a guard no test could show red.
- Ruling: resume's `DocumentPromote` scope arm, `settled`, `branch_record_of`, `replay.rs` `result_commit_held`, the CLI retry guard and `continued_a_lifecycle` are Task 5's, as the plan's Self-Review maps Part G item 4's replay, resume and retry-guard parts — they key on Task 5's data — none.
- Ruling: Part G item 7 needs no code (the second slot's readers already answer for an empty second slot); its runner tests assert `record::audit` after each finish or stop, assertions that pin unchanged behaviour and so are exempt from the red run (Global Constraints) — none.
- Ruling: the answered-`abandoned` invariant (Task 1's deferred Minor) is added here: `CHECK(answer_code IS NULL OR consented_context_oid IS NOT NULL)` and the same rule in `validate`, one row in `pull_request_promotion_steps_and_checks`, run red once — the migration has never run on a real registry — none.
- Pointer for Task 4: after the child, `promote_branch` still runs the cancellation check (`run.rs:977`) and, before it, the check that the record's remote is the configured one while `base_oid` is unset (`run.rs:936`), which a pull-request record (never with a base) fails on every call once the configured remote changes; RR2, RR4, RR5, RR8 and the crash boundaries are Task 4's.

- Commits: `faced22a` feat(promotion): promote in a pull-request repository publishes and gives the link; `390b54bb` fix(promotion): the record holds a completed child, and the answers share their helpers (fix round 1); `95c37cf0` test(promotion): a retained child holds the record (fix round 2).
- Tests (implementer, each by exact name, `repository::promotion::run_tests::pull_request::`): `first_promote_publishes_and_gives_the_link` (C1 to C3), `promoting_again_synchronizes_and_gives_the_same_link` (C4 to C7), `a_held_document_gives_no_link` (C8's prepare half, C9, C10), `a_declined_pull_request_answers_remote_branch_deleted` (C11, with the republish), `a_pull_request_promotion_whose_index_step_failed_is_branch_index_pending`; `repository::promotion::record::tests::pull_request_begin_and_finish_write_their_columns` (service-backed, real child rows: complete over a completed child only; abandon refused over a live or retained child) and the added invariant row of `pull_request_promotion_steps_and_checks`; `repository::mutation::bind::lifecycle::tests::a_pull_request_promotion_answers_by_its_answer` (the binding's codes and effects; the checkpoint named it `promotion_branch_answers_map_to_codes_and_effects`) and the five `branch_reached` tests, unchanged; the unchanged `cache_loss_contained_context_cleans_without_another_merge_or_child`, `already_current_publication_is_stored_and_replayed_without_children_or_deletion`, `contained_cleanup_needs_no_identity_or_new_merge`, `d1_records_publishes_and_completes_without_touching_the_primary`, `d2_an_existing_closure_publishes_pending_work_and_is_already_closing`, `promotion_modes_migration_rebuilds_a_v2_table_once`; E-5's `local_full_lifecycle_and_exact_replay`. The three E-5 arms in the SSH target `document_promotion` were compiled through clippy, not run. Static gates clean. Red: the compile failure, deliberate stubs (a) to (i), the invariant's two halves, and in the fix rounds the record's completed-child check, `child_holds` (both arms) and the primary assertion.
- Named edits applied: E-5 (`run_tests.rs:5069-5074`, `tests/document_promotion.rs:314-320`, `tests/document_promotion/journeys.rs:257-261`, `tests/document_promotion/quality.rs:130-134`, arms only). Removed this Cycle's own interim-guard test `a_pull_request_confirm_stops_before_any_direct_record`. A test-only wrapper `remote/state.rs` `prepare_step_for_test` builds a retained child through the real `prepare_integration_step`.
- Review (Opus): the tables in row order, the link writes, no path in a pull-request repository that reaches the primary or `finish_branch`, the direct path unchanged; spec ❌ and two Important: the record layer had dropped the approved completed-child check (an implementer departure after approval), and `promotion_branch_reached` copied three blocks of `branch_reached`; plus test rows the checkpoint named (C6's other parent, C7's prepared link, C11's republish) and a vacuous primary assertion. Ruling on the copy (partly plan-mandated, "twins"): factored into `branch_published_effects` and `branch_ran`, which both call — the design's twin need not be a copy, and the R-32 and owed-index rules must not drift — the closure's tests guard the refactor. Fix round 1/5 (3 addressed, 1 open: no retained-child row; `faced22a..390b54bb`); fix round 2/5 (1 addressed, 0 open; `390b54bb..95c37cf0`). Re-reviews (Sonnet): the `branch_reached` refactor is behaviour-preserving; addressed, no new Critical or Important.
- Deferred Minor findings (for the final review): R-12's choice of the child's authority OID (`answer.rs:82`) and `remote_gone` (`:161`) are not pinned by a row where they differ (a row whose child brings another clone's later save over a squash of `L` would); red runs were per test, not per row (C2, C3, C5 to C7, most record mismatch rows); a redundant remote check (`lifecycle.rs:924` and `record.rs:654`); `pub(super) mod answer` is wider than needed; no runner test for the `history_unresolved` and `context_gone` ends (record and binding layers only); the `PullRequestIo` twin repeats Cycle 03's cancel-acknowledgement and reservation code; the `Child::Ended` fixture sets `phase='cancelled'` by SQL.
- Task 3: complete (commits `6c46b35c..95c37cf0`, review clean after fix round 2).

## Appendix A: References, Task 0 Steps 2 and 3 (research agent, verbatim)

### Task 0: names check of the Cycle 04 design against HEAD aee99255 (main 8e5d9832)

Code: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M87684C9GM9BJP5QQ429KMAK`. Design: `docs/plans/2026-11-26-wave-06-cycle-04-pull-request-promote-design.md` (revision 7). Paths below are relative to the code worktree. Read-only check; no cargo run, nothing edited. "Cited" line numbers are the design's; a name within about 40 lines counts as found. Status: found, moved, renamed, changed, missing.

### Task A: Cycle 03's names (design U-1)

| Name | Design cites | Found at (file:line) | Status | Note |
| --- | --- | --- | --- | --- |
| `RecordPath` | `promotion/lifecycle.rs` about 178; `{ Direct, Branch }` | `src/repository/promotion/lifecycle.rs:178` | found | `pub(in super::super) enum RecordPath { Direct, Branch }`; `LifecycleRecord::path()` defaults to `Direct` at :217 |
| `BranchStep` | about 186; `{ Closing, Finished }` | `src/repository/promotion/lifecycle.rs:186` | found | variants `Closing`, `Finished` as cited |
| `BranchEnd` | about 197; `{ ContextGone, ContextChild }` | `src/repository/promotion/lifecycle.rs:197` | found | shape detail: it is `BranchEnd<'a>` and `ContextChild(&'a SynchronizationError)` carries a reference to the child's error, `ContextGone` is unit |
| `promote_branch` | `run.rs` about 917 | `src/repository/promotion/run.rs:917` | found | `fn promote_branch<L: Lifecycle>`; interrupted check at :936-938, `AfterConfirmed` at :940, `end_branch(ContextGone)` at :950-956 |
| `ends_context_child` | `run.rs` about 1306 | `src/repository/promotion/run.rs:1306` | found | names exactly `RemoteContextDeleted`, `HistoryUnresolved`, `TicketClosed`; `let_go` at :1320 |
| `give_link` | `closure/record.rs` about 1461; kind `"ticket"` written in about 1470 | `src/repository/closure/record.rs:1461` | found | private `fn give_link(tx, id, item, Option<&PullRequestLink>, given_at)`; `"ticket"` hard-coded at :1470; callers :1316 (`complete_branch_record`) and :1346 (`abandon_branch_record`). A second `give_link` in `tests/read_items.rs:6155` is an unrelated test helper |
| `branch_link` | `closure/checkpoint.rs` about 1036 | `src/repository/closure/checkpoint.rs:1036` | found | `pub(in super::super) fn branch_link(repo, primary, forge_setting: Option<&ForgeSetting>, remote, item: &ItemId)`; kind comes from the file const `KIND = LifecycleKind::TicketClosure` (:44) via `KIND.context_branch(item)`, so it is ticket-only today |
| `abandon_branch_record` | `closure/record.rs` about 1327 | `src/repository/closure/record.rs:1327` | found | `(tx, id, op, &Refusal, BranchAnswer, Option<&PullRequestLink>, given_at)`; the closure's `end_branch` calls it with `Refusal::ReconfirmationRequired` (`closure/lifecycle.rs:575-588`, cited 580-589) |
| `complete_branch_record` | about 1296 | `src/repository/closure/record.rs:1296` | found | stores `answer_code`, calls `give_link` |
| `Called::Branch` | `bind/lifecycle_verb.rs` about 117-125 | `src/repository/mutation/bind/lifecycle_verb.rs:117-125` | found | `Called::Branch { outcome: BranchClosureOutcome, index_pending: bool }`; closure-typed as the design says; built by `called_of` at :362 (arms :371, :375) |
| `branch_code` | about 676 | `src/repository/mutation/bind/lifecycle_verb.rs:676` | found | `pub(super) fn branch_code(answer: BranchAnswer) -> ResultCode` (closure's `BranchAnswer`) |
| `ClosureOutcome` | `closure/mod.rs` about 565; `closure/lifecycle.rs` about 269 | `src/repository/closure/mod.rs:565`; `src/repository/closure/lifecycle.rs:269` | found | `enum ClosureOutcome { Direct(TicketClosureOutcome), Branch(BranchClosureOutcome) }`; `type Outcome = ClosureOutcome` at lifecycle.rs:269 (also :756) |
| `TicketClosureResult::Branch` | `closure/mod.rs` about 471-560 | `src/repository/closure/mod.rs:554-559` | found | `Branch(BranchClosureOutcome)` and `BranchIndexPending(IndexPending<BranchClosureOutcome>)`; `BranchClosureOutcome` struct at :471, `BranchAnswer` enum at :104 |
| `pull_request_links` table | `discovery.rs` about 2944, kinds `ticket` and `document`, marker `wave_06_pull_request_links_v1`, migrated last about 2760 | `src/repository/discovery.rs:2940-2954`, call at :2760 | found | `kind TEXT NOT NULL CHECK(kind IN ('ticket','document'))`, PK `(repository_id, kind, item_ulid)`, columns `source, target, forge, url, given_at` |
| `pull_request_links` module | `src/repository/pull_request_links.rs`: `link_of`, `upsert_link`, `delete_link` (`pub(super)`) | `src/repository/pull_request_links.rs:11, 38, 69` | found | `mod pull_request_links;` at `src/repository.rs:76`. `upsert_link` rewrites every column including `given_at` (ON CONFLICT DO UPDATE), so there is no update-`given_at`-alone helper yet; `link_of` reads an unknown forge as none (:27) |
| `PullRequestLinkDataDto` | `dto.rs` about 1820, `of` takes any link | `src/repository/mutation/dto.rs:1820` | found | `of(&forge::PullRequestLink)` at :1827; `from_stored` at :1842; `MutationDataDto::PullRequestLink(Box<..>)` at :1096 |
| `schemas/v1/pull_request_link.schema.json` | description says "of a ticket" and that the link is recorded | `schemas/v1/pull_request_link.schema.json` | found | description: "...result of item sync of a ticket in a pull-request repository... The link is also recorded for the item, and ticket show reads it from there." |
| golden `item_sync_pull_request_link` | `mutation_v1/item_sync_pull_request_link.json` | `tests/fixtures/mutation_v1/item_sync_pull_request_link.json` | found | registered `tests/support/mutation_golden.rs:104`; written by the SSH case at `tests/mutation_remote/c5_lifecycle.rs:3850` |
| test `item_sync_gives_the_link_of_a_closing_ticket` | `bind/sync_tests.rs` about 519; document case about 587-594 | `src/repository/mutation/bind/sync_tests.rs:519` | found | document target (`AuthoringKind::Document`) at :587-594 with `anyone(...)` asserting `data.is_none()` and no row; the fixture seeds no document row. The doc comment (:508-514) and the lead-in comment (:566-571) do not say "documents never get a link" (see Discrepancies) |
| withdraw prepare's `already_applied` shape | `Observed::NothingToWithdraw` about 100, 623; answered `Confirmed::preview` `bind/lifecycle.rs` about 1545-1547; digest `unoffered` about 439-441; `Binding::prepare` about 1495-1506; CLI boundary `execute.rs` about 307-311; golden `ticket_withdraw_closure_already_applied` | variant `lifecycle_verb.rs:100`, built `lifecycle_verb.rs:623` (from `ClosureWithdrawalPreflight::NothingToWithdraw`); answered `bind/lifecycle.rs:1545-1547` `Err(Answer::stopped(ResultCode::AlreadyApplied))`; digest `bind/lifecycle.rs:439-440` `text("unoffered", "nothing_to_withdraw")`; `Binding::prepare` expectation `None` at `bind/lifecycle.rs:1495-1500`; confirm-side `Seen::State { own: None }` at :1706-1708; boundary `src/repository/mutation/execute.rs:307-311` | found | carries no data: golden `tests/fixtures/mutation_v1/ticket_withdraw_closure_already_applied.json` has `data: null`, `outcome: noop`, `recovery: []`, `operation_id: null`, all effects `not_requested`, scope with `item_id` only; digest differs from every ready digest so the CLI boundary answers `external_change`. Written by `tests/mutation_confirmation/conformance.rs:349` `withdraw_with_no_closure_is_already_applied`, registered `tests/support/mutation_golden.rs:298` |
| `PullRequestIo` | `closure/run_pull_request_tests.rs` about 618-775, private; wraps `FakeIo`; drives `promotion_test_drive_local_bound`; refuses a primary target about 664-667 | `src/repository/closure/run_pull_request_tests.rs:618` | found | private `struct PullRequestIo { inner: FakeIo, primary_calls, .. }` (`impl PromotionIo` at :651); `promotion_test_drive_local_bound` at :732 and :1472; counts `primary_calls` (:665). Visible only inside module `repository::closure::run_tests::pull_request` and its children |
| test-local `Forge` | `closure/run_pull_request_lifecycle_tests.rs` about 17-250, child module of `run_pull_request_tests.rs` | `src/repository/closure/run_pull_request_lifecycle_tests.rs:19` (`impl` :23, `Merge` :238) | found | module is `mod lifecycles` declared at `run_pull_request_tests.rs:6151`; methods `push, publish, merge_commit, squash, fast_forward, delete_branch, fetch, poll`. Note `PullRequestIo` does not itself use `Forge`; the lifecycle tests use both |
| SSH test `a_pull_request_close_publishes_only_the_ticket_branch` | `tests/mutation_remote/c5_lifecycle.rs` about 3669-3934, registered about 248 | `tests/mutation_remote/c5_lifecycle.rs:3669-3934`, registered :248-249 | found | writes goldens `ticket_close_pull_request_ok` (:3800), `ticket_close_already_closing` (:3825), `item_sync_pull_request_link` (:3850), `ticket_withdraw_closure_ok` (:3890); mentioned also in `tests/cli/collaboration_exits.rs:1288` text |

### Task B: Baseline Facts 1 to 25 and Parts A to G

| Name | Design cites | Found at (file:line) | Status | Note |
| --- | --- | --- | --- | --- |
| BF1 `LifecycleBinding::prepare` | `mutation/bind/lifecycle.rs` | `src/repository/mutation/bind/lifecycle.rs:1481` | found | `fn prepare(&mut self, call)`; the observe step is :640-648 |
| BF1 `Promote::observe` | `bind/lifecycle_verb.rs` about 409 | `src/repository/mutation/bind/lifecycle_verb.rs:409` | found | in `impl LifecycleVerb for Promote` (:403) |
| BF1 `RepositoryService::preview_document_promotion` | `promotion/mod.rs` about 501 | `src/repository/promotion/mod.rs:501` | found | |
| BF1 `observe::preview`, `prepare_for` | `promotion/observe.rs` about 42, 346 | `src/repository/promotion/observe.rs:42, 346` | found | |
| BF1 `observe_git` | about 948 | `src/repository/promotion/observe.rs:948` | found | |
| BF1 `expectation` | about 1367 | `src/repository/promotion/observe.rs:1367` | found | domain bytes `b"manyhands promotion v1"` at :1372 (also `closure/observe.rs:42` doc comment, `observe_tests.rs:145`) |
| BF1 `digest_of` | `bind/lifecycle.rs` about 422 | `src/repository/mutation/bind/lifecycle.rs:422` | found | |
| BF1 `Observed` four shapes | `lifecycle_verb.rs` about 95-104 | `src/repository/mutation/bind/lifecycle_verb.rs:95-101` | found | `Ready(Box<Ready>)`, `Blocked`, `NothingToWithdraw`, `PublicationRemoteRequired` |
| BF1 `Ready`, `primary_oid`, `record: Option<BranchReady>` | about 77-93, 83, 64-74 | `lifecycle_verb.rs:77, 83, 92`, `BranchReady` at :64 | found | `primary_oid: git2::Oid` not optional (:83); `record: Option<BranchReady>` at :92 |
| BF2 `planned_result` | `observe.rs` about 918 | `src/repository/promotion/observe.rs:918` | found | `PlannedResult::Contained` at :912 |
| BF2 `a_context_primary_already_holds_is_only_cleaned_up` | `tests/cli/lifecycle.rs` about 626 | `tests/cli/lifecycle.rs:639` | found | 13 lines off |
| BF2 resume's `already_applied` for a complete record | `bind/resume.rs` about 263, 990 | `src/repository/mutation/bind/resume.rs:263-268 (Named::Complete), 990` | found | also `AlreadyApplied` at :1185, :1799 |
| BF3 `PromotionBlocker` `BLOCKERS` (17) | `promotion/record.rs` | `src/repository/promotion/record.rs:78` | found | `[PromotionBlocker; 17]` |
| BF3 `promotion_blocker` | `lifecycle_verb.rs` about 210 | `src/repository/mutation/bind/lifecycle_verb.rs:210` | found | |
| BF3 `primary_presence` | `observe.rs` about 798 | `src/repository/promotion/observe.rs:798` | found | |
| BF3 `primary_missing`/`primary_ahead_of_remote` logic | about 1023-1043 | `src/repository/promotion/observe.rs:1015-1040` | found | `primary_ahead` flag at ~:1037 |
| BF3 `identity_required` at confirm | `bind/lifecycle.rs` about 1483-1488 | `src/repository/mutation/bind/lifecycle.rs:1484-1488` | found | also :453 |
| BF3 `require_identity` | `promotion/lifecycle.rs` about 1004-1027; `run.rs` about 579-581 | `src/repository/promotion/lifecycle.rs:1017` (trait :524); `src/repository/promotion/run.rs:568` | found | |
| BF3 `DocumentPromotionPreview.primary_oid` | `promotion/mod.rs` about 288 | `src/repository/promotion/mod.rs:288` | found | struct at :282; not optional |
| BF3 `DocumentPromotionPreflight` matched in about twelve files | | `src/repository/promotion/mod.rs:357` (enum `{ Ready(DocumentPromotionPreview), Blocked { blockers } }`) | found | 13 files mention it: `bind/lifecycle_verb.rs`, `promotion/{integrate_tests,lifecycle,mod,observe,observe_tests,run_tests}.rs`, `tests/{cli_remote/interruption,cli_remote/lifecycle,document_promotion/journeys,document_promotion,drafts/targets,mutation_remote/c5_lifecycle}.rs` |
| BF3 `begin_record` from observation | `lifecycle.rs` about 895-903; `run.rs` about 597-605 | `promotion/lifecycle.rs:895`; `promotion/run.rs:604` | found | |
| BF3 `ClosureObservation.path` | `closure/lifecycle.rs` about 291-330 | struct `src/repository/closure/observe.rs:244`; use `src/repository/closure/lifecycle.rs:~291-335` (`o.path` at :335) | found | the struct is defined in observe.rs, used in lifecycle.rs |
| BF3 `runs_context_child()` | `lifecycle.rs` about 344-348; `context_step` `run.rs` about 1153-1167 | `src/repository/promotion/lifecycle.rs:316` (default method of `LifecyclePreview`); `run.rs:1153-1167` | found | 28 lines off; default is `!(AlreadyContained && tracking_context.is_none())` |
| BF3 `foreign_synchronization` | `closure/observe.rs` about 744-779 | `src/repository/closure/observe.rs:744, 771` | found | |
| BF4 `remote_context_history` | `observe.rs` about 891 | `src/repository/promotion/observe.rs:891` | found | Deleted/Unknown/Unresolved blockers at ~:1080-1086 |
| BF4 `advance_tracking_to_authority` | `remote/sync.rs` about 9289 | `src/repository/remote/sync.rs:9289` | found | |
| BF4 child's fetch and evidence | `sync.rs` about 6719-6748, 8921-8981; `refs.rs` about 264-268 | `sync.rs:6746` (`HistoryUnknown`), `:8921` (fetch), `:8975-8982`; `remote/refs.rs:262-270` | found | `NeverPublished`/`ObservedPublished`/`HistoryUnknown` arms in `refs.rs`; `RemoteContextDeleted` at `sync.rs:500, 4137`; `HistoryUnresolved` at `sync.rs:415, 4140` |
| BF4 `FetchTracking` | "(`FetchTracking`)" | `src/repository/transport/remote.rs:19` (`TransferAction::FetchTracking`), used :257, :286 | moved | it is a `TransferAction` variant in `transport/remote.rs`, not a name in `remote/sync.rs` |
| BF5 `document_promotions` `SCHEMA` | `promotion/record.rs` about 141-169 | `src/repository/promotion/record.rs:141-169` | found | `step` CHECK list exactly as cited; `CHECK((step='abandoned')=(refusal_category IS NOT NULL))` at :167 |
| BF5 `INDEX` | | `src/repository/promotion/record.rs:170` | found | `document_promotions_unfinished`, `WHERE step NOT IN ('complete','abandoned','superseded')` |
| BF5 `migrate`, markers | about 172, 197-208 | `src/repository/promotion/record.rs:172-213`; markers `cycle_09_document_promotions` (:174), `promotion_refusal_blockers_v2` (`BLOCKERS_MARKER` :171) | found | rebuild by rename and `INSERT ... SELECT *` from current `SCHEMA` at :197-199 |
| BF5 called inside the registry-schema transaction before closure's | `discovery.rs` about 2749-2750 | `src/repository/discovery.rs:2749-2750` | found | `closure::record::migrate` right after; `migrate_pull_request_links` last at :2760 |
| BF5 `PromotionStep::permits`, `validate` | about 35, 242 | `promotion/record.rs:35, 242` | found | `(r.step == Abandoned) != r.refusal.is_some()` at :252 |
| BF5 `validate_schema_object` | (stored DDL compared exactly) | `src/repository/remote/state.rs:1632` | found | called from `record.rs` `validate_schema` as `state::validate_schema_object` |
| BF5 `promotion_refusal_blockers_v2_rebuilds_an_old_bound_table_once` | `record_tests.rs` about 1417-1458 | `src/repository/promotion/record_tests.rs:1417` | found | builds the old table from the current DDL (:1429) |
| BF6 `RecordPath`/`BranchStep`/`BranchEnd`, `path()`, `answered()`, `synchronizes_context()` | `lifecycle.rs` about 178, 186, 197, 217, 222, 228 | `lifecycle.rs:178, 186, 197, 217, 222, 228` | found | |
| BF6 `promote_inner` | `run.rs` | `src/repository/promotion/run.rs:455` | found | |
| BF6 `context_step` | about 1139 | `src/repository/promotion/run.rs:1139` | found | sets `context_oid` at ~:1181 |
| BF6 pre-child `interrupted` while `base_oid` unset | about 934-938 | `src/repository/promotion/run.rs:935-938` | found | |
| BF6 `RunEvent::Recording` when `!effect_written()` | about 980-982 | `src/repository/promotion/run.rs:980-982` | found | the condition is `!r.effect_written() \|\| r.step() == ContextSynchronized`; also :810 |
| BF6 desktop wording "Marking it closed" | `progress.rs` about 122 | `src/desktop/model/operations/progress.rs:122` | found | |
| BF6 `Lifecycle::branch_step`, `Finished` returns `refresh`, never `finish_branch` | about 984-990 | `src/repository/promotion/run.rs:983-990` | found | `finish_branch` at :999 |
| BF6 `let_go` | about 1320 | `src/repository/promotion/run.rs:1320` | found | |
| BF6 `ContextGone` ends via `end_branch`; closure answers `ReconfirmationRequired` | `closure/lifecycle.rs` about 567 | `src/repository/closure/lifecycle.rs:567` | found | |
| BF7 `DocumentPromotionLifecycle` and stubs | `lifecycle.rs` about 864; stubs about 1102-1140 | `src/repository/promotion/lifecycle.rs:864` (`type BranchFinish = Infallible` :878); `branch_step :1103`, `end_branch :1111`, `begin_publication :1119`, `branch_answer :1126`, `branch_outcome :1143-1149` | found | all answer `recovery_required`; `branch_outcome` is `unreachable!("a promotion record is always direct")` (:1147); `complete_branch` also a stub |
| BF7 `PromotionRecord` never overrides `path()` | | no `fn path` in `promotion/record.rs`; only the trait default `lifecycle.rs:217` | found | |
| BF7 `DocumentPromotionResult` `{ Complete, IndexPending }` | `promotion/mod.rs` about 188 | `src/repository/promotion/mod.rs:188` | found | |
| BF7 exhaustive matches on `DocumentPromotionResult` | `lifecycle_verb.rs` about 458-464; `run_tests.rs` about 5069-5074; `tests/document_promotion.rs` about 314-320; `journeys.rs` about 257-261; `quality.rs` about 130-134 | `lifecycle_verb.rs:459-462`; `promotion/run_tests.rs:5065-5071`; `tests/document_promotion.rs:316-317`; `tests/document_promotion/journeys.rs:259-260`; `tests/document_promotion/quality.rs:132-134` | found | `quality.rs` also :232; `run_tests.rs` has more uses at :1365, 1445, 1586, 2760 |
| BF7 `make_outcome` | `run.rs` about 2326-2337 | `src/repository/promotion/run.rs:2320` | found | a branch record reads `primary_authority::<L>` (the second slot) at :2327-2336 |
| BF8 `child_allowed` | `lifecycle_record.rs` about 474 | `src/repository/promotion/lifecycle_record.rs:474` | found | `(_, Context)` at `Confirmed`; `(Direct, Primary)`; `(Branch, Publication)` as cited |
| BF9 `ticket_closures` rebuilt under `wave_06_closure_modes_v1` with `integration_mode`, `action`, `answer_code`; `SCHEMA_V2`, `V2_COLUMNS` | `closure/record.rs` about 428, 327-486; `record_tests.rs` about 1782 | `closure/record.rs:326-330, 427, 428, 486`; `closure/record_tests.rs:1782, 1978` | found | |
| BF9 `ClosureRecord::path()` | about 256 | `src/repository/closure/record.rs:256` | found | `Direct` only for `(Direct, Close)` |
| BF9 answer CHECKs of the closure | `closure/record.rs` about 421-422 | `src/repository/closure/record.rs:421-422` | found | `answer_code IS NULL OR step IN ('complete','abandoned')`; `step!='complete' OR direct-close OR answer_code IS NOT NULL` |
| BF9 `branch_reached`, `branch_data` closure-typed | `bind/lifecycle.rs` about 886-1038 | `bind/lifecycle.rs:886` (`outcome: &BranchClosureOutcome`), `:987`; `branch_data` reads `link_of` with `AuthoringKind::Ticket` hard-coded (:996-1000) | found | |
| BF9 `branch_stopped`, `parent_effects`, `parent_integrated`, `branch_scope` | about 1159-1229, 1798-1918, 1756, 186-212 | `bind/lifecycle.rs:1159, 1798, 1749, 186` | found | `branch_scope(kind: &AuthoringKind, id, repository, root)` takes a kind already |
| BF9 resume's branch scope, `DocumentPromote` arm, `settled`, `branch_record_of` | `bind/resume.rs` about 1649, 1335; `bind/lifecycle.rs` about 216-226 | `resume.rs:1647-1649` (scope uses `AuthoringKind::Ticket`), `:1335-1336` (`DocumentPromote => continued(...)`), `:1659-1660` (`settled`); `bind/lifecycle.rs:216-226` | found | `branch_record_of` returns `ItemId` via `Close::parent` only |
| BF9 `note_observed` | about 627-631 | `src/repository/mutation/bind/lifecycle.rs:627-628` | found | checks `ready.record.is_some()` |
| BF9 `result_commit_held` | `replay.rs` | `src/repository/mutation/replay.rs:190` | found | tags at :198-201 (`TicketClose \| TicketWithdrawClosure`, `ticket/`, `pull_request_closure`) |
| BF9 CLI retry guard, `continued_a_lifecycle` | `mutate.rs` about 104-111; `collaboration.rs` about 217-224 | `src/bin/manyhands-cli/mutate.rs:105-108`; `src/bin/manyhands-cli/collaboration.rs:219-222` | found | match `Lifecycle` and `PullRequestClosure` data only |
| BF10 retirement deletes the link | `cleanup.rs` `retire_context` about 1018; `context_retirement/record.rs` `finalize_index` about 2044 | `cleanup.rs:953` (`retire_context`), delete at `:1019`; `context_retirement/record.rs:1966` (`finalize_index`) | moved | `finalize_index` is at 1966, 78 lines before the cited 2044 |
| BF10 `retire_context_deletes_the_link` | `cleanup_tests.rs` about 1495 | `src/repository/cleanup_tests.rs:1495` | found | |
| BF10 `a_finalized_retirement_deletes_the_link` | `context_retirement/execution_tests.rs` about 228 | `src/repository/context_retirement/execution_tests.rs:228` | found | |
| BF11 `preservation` | `context_retirement/proof.rs` about 27 | `src/repository/context_retirement/proof.rs:27` | found | refuses shallow (:35); `entries(.., true)` at :38-39; `completed_item` at :223; `Ancestor` when `local == primary` or primary descends from local; one merge base else `Folded`/`ContentMismatch` |
| BF11 `completed_deleted_context` | `remote/poll.rs` about 279-307 | `src/repository/remote/poll.rs:279-307` | found | `PollDisposition::Promoted` at :304 |
| BF11 `retirement_proof_document_presence_is_not_preservation` | `context_retirement/tests.rs` about 268 | `src/repository/context_retirement/tests.rs:268` | found | |
| BF12 retirement captures `Closed` and `Promoted` | `source.rs` about 176 | `src/repository/context_retirement/source.rs:176` | found | |
| BF12 `require_fetch_absence` | `record.rs` about 1680 | `src/repository/context_retirement/record.rs:1680` | found | |
| BF12 `compare_local_proof` | `run.rs` about 927 | `src/repository/context_retirement/run.rs:927` | found | |
| BF12 `context release` refuses published | `context_release.rs` header | `src/repository/context_release.rs:4-30` (step 4c) | found | |
| BF13 `synchronize_remote_observed` | `remote/sync.rs` about 6316 | `src/repository/remote/sync.rs:6316` | found | |
| BF13 stages `[Context, Primary]` or `[Primary]` | `remote/merge.rs` about 20-31 | `src/repository/remote/merge.rs:25-30` | found | |
| BF13 "Merge primary into document <ULID>" | `sync.rs` about 3872 | `src/repository/remote/sync.rs:3872` | found | formatted `"Merge primary into {} {item_id}"`; "Merge remote context" at :3862 |
| BF13 outcome codes | `mutation/outcome.rs` about 207, 358-368 | `src/repository/mutation/outcome.rs:207, 364` | found | `HistoryUnresolved` at :240 |
| BF13 context target never pull-only | `sync.rs` about 6468-6474 | `src/repository/remote/sync.rs:6466-6474` | found | `execution.is_none()` at :6472 |
| BF13 `SyncBinding::link_given` | `bind/sync.rs` about 1099, 1113-1182 | `src/repository/mutation/bind/sync.rs:1113` | found | ticket-only at :1118-1124; `holds_provisional_closure` at :1152; `branch_link` :1160; `upsert_link` with `now_utc()` (not an injected clock) :1168-1176 |
| BF13 `holds_provisional_closure` | `closure/observe.rs` | `src/repository/closure/observe.rs:865` | found | |
| BF14 `PullRequestClosureMutationDto`, `PullRequestClosurePreviewDto` | `dto.rs` about 1712, 1871 | `src/repository/mutation/dto.rs:1712, 1871` | found | `blockers: Vec<LifecycleBlocker>` at ~:1726 |
| BF14 `withdraw_with_no_closure_is_already_applied` | `conformance.rs` about 348-370 | `tests/mutation_confirmation/conformance.rs:349` | found | |
| BF14 `FakeIo` | `promotion/run_tests.rs` about 357 | `src/repository/promotion/run_tests.rs:357` | found | |
| BF15 `ItemDto.pull_request` | `read/dto.rs` about 571, doc 565-570 | `src/repository/read/dto.rs:571` (doc :565-570) | found | doc says null "in every list, for a document and for a ticket that has none" |
| BF15 `item.schema.json` | | `schemas/v1/item.schema.json:122-124` (and :21) | found | |
| BF15 `fill_complete_read` | `read/items.rs` about 1533 | `src/repository/read/items.rs:1533` | found | early return for non-ticket at :1538-1540 via `(Some(id), ItemDtoKind::Ticket)`; link read with `AuthoringKind::Ticket` hard-coded; called by `show_item` (:2518) and `show_path` (:2629) |
| BF15 `ticket_show_gives_the_link_and_the_publication` | `tests/read_items.rs` about 6220, 6244-6257 | `tests/read_items.rs:6220`; comment :6245-6246; assertion :6256-6257 | found | document `docs/note.md` committed on the primary, no context |
| BF16 `PreviewEffect` | `dto.rs` about 212, 226-228 | `src/repository/mutation/dto.rs:212, 226-228` | found | `ContextSynchronized`, `ContextPublished`, `PullRequestLink` (:227), `ClosureWithdrawn` |
| BF16 `branch_effects` | `bind/lifecycle.rs` about 313-360 | `src/repository/mutation/bind/lifecycle.rs:313` | found | |
| BF16 preview early return for a closure | about 1565-1581; direct block 1582-1616 | `bind/lifecycle.rs:1565-1580`, direct block from :1582 | found | `if let (Some(record), Some(closure)) = (&ready.record, &ready.closure)` |
| BF16 `MutationDataDto` with `PullRequestClosure`, `PullRequestLink` | about 1071, 1093-1096 | `dto.rs:1071, 1093, 1096` | found | |
| BF16 `PreviewDetailsDto`, `PullRequestClosure` | about 316, 331 | `dto.rs:316, 331` | found | |
| BF16 `PrepareDataDto` | about 962 | `dto.rs:962` | found | |
| BF16 `MutationDataDto::from_stored` arms | about 3064-3256; promote arm about 3173; close/withdraw 3177-3191; conflict arm 3150-3160; host-data arm 3224-3231; resume arms 3086-3101 | `dto.rs:3064`; `Command::DocumentPromote` :3173; `TicketClose \| TicketWithdrawClosure` :3153-3154, :3177, :3185, :3228-3229; `ItemRepublish` :3227; `PullRequestClosure` :3100, :3190; `PullRequestLink` :3214-3217 | found | |
| BF16 `lifecycle_way_out`, `route_of`, `branch_route`, `child_conflict` | `way_out.rs` about 121-139, 242, 150-189, 294; `PrepareAgain`, `SyncItem` about 46-50 | `src/repository/mutation/way_out.rs:121, 242, 150, 294`; `PrepareAgain :46`, `SyncItem :50` | found | `child_conflict` only for `TicketClose \| TicketWithdrawClosure` (:128-134) |
| BF16 contract rows | `cli-contract.md` about 1577, 1592 | `docs/RFC/cli-contract.md:1577` (`document promote`, `ticket close` / `remote_branch_deleted` / `request.cancel`), `:1592` | found | |
| BF16 `DATA_ROOTS` (77) | `tests/mutation_contract.rs` about 1605; `the_fixture_directories_are_exactly_the_registered_cases` about 1511 | `tests/mutation_contract.rs:1605` (`[&str; 77]`), `:1511` | found | `data_root_differences` at :1757; the three lifecycle roots are listed at :1643-1661 |
| BF16 `CLI_CASES`, `CLI_TEXT_CASES` | `tests/support/cli.rs`, `golden()` about 587 | `tests/support/cli.rs:217, 435`; asserts at :591, :597-598 (`assert_cli_golden`, `assert_cli_text_golden`) | found | there is no function named `golden`; the checks are in those two functions |
| BF16 `tests/cli/inventory.rs` `DATA` row | about 267-274, `ticket close` about 280-292, test about 552-591 | `tests/cli/inventory.rs:267-274`; `every_verb_s_data_has_an_output_schema_and_its_goldens_match_it` at :576 | found | |
| BF16 exhaustive `MutationDataDto` matches | `tests/support/mutation.rs` (six), `tests/support/repair.rs` about 346-367 | `tests/support/mutation.rs:674-679` (six consecutive); `tests/support/repair.rs:~335-365` | found | |
| BF16 `tests/cli/recovery_tables.rs` | `LIFECYCLE`, `PROMOTE_AND_CLOSE` about 74-106 | `tests/cli/recovery_tables.rs:74, 81, 95, 106` | found | `LIFECYCLE` now holds `document promote`, `ticket close`, `ticket withdraw-closure` |
| BF16 `tests/read_contract.rs` enumerations | | `tests/read_contract.rs` (target root file) | found | not further inspected |
| BF17 `Command::setup_gate` | `mutation/command.rs` about 324 | `src/repository/mutation/command.rs:324` | found | `DocumentPromote` at :328 |
| BF17 gate sites | `execute.rs` about 276, 359, 415; `confirm.rs` about 346 | `src/repository/mutation/execute.rs:275-276, 359, 415`; `confirm.rs:345-346` | found | |
| BF18 `RepositoryConfig::integration()`, `Integration` | `canonical.rs` about 146, 91, 1457-1470 | `src/canonical.rs:146, 91, 1457-1470` | found | requires `format_version = 2` and `publication_remote` |
| BF18 promotion primary child exempt from pull-only | `sync.rs` about 6474 | `src/repository/remote/sync.rs:6472` | found | |
| BF19 `PromotionIo`, `run_with_io` | `promotion/run.rs` about 23, 327 | `src/repository/promotion/run.rs:23, 327, 337` | found | |
| BF19 `promotion_test_drive_local_bound` | `sync.rs` about 3983 | `src/repository/remote/sync.rs:3983` | found | |
| BF19 `synthetic_fetch` | `poll.rs` about 1435 | `src/repository/remote/poll.rs:1435` | found | |
| BF19 `execute(Mutation::PollOnceRetiring)` with `synthetic_fetch` | `context_retirement/mutation_tests.rs` about 207-214 | `src/repository/context_retirement/mutation_tests.rs:209-213` | found | |
| BF19 objects fetched with plain `git2` | `acceptance_tests.rs` about 357-363 | `src/repository/context_retirement/acceptance_tests.rs:355-362` | found | |
| BF19 `synthetic_advertisement` `pub(super)` | `context_retirement::run` | `src/repository/context_retirement/run.rs:101` | found | |
| BF19 `parse_ssh_endpoint` refuses `file` | `transport/endpoint.rs` | `src/repository/transport/endpoint.rs:56` | found | |
| BF20 goldens | `tests/fixtures/cli_v1/document_promote_*`, `mutation_v1/document_promote_*`; `mutation_golden.rs` about 56-65 | `tests/fixtures/cli_v1/` (14 files), `tests/fixtures/mutation_v1/document_promote_*.json` (9), `tests/support/mutation_golden.rs:56-61+` | found | |
| BF21 `capture_target` fences | `source.rs` about 201-208 | `src/repository/context_retirement/source.rs:166` (fn), fence at :205 | found | |
| BF21 `claim_initial` | `authority.rs` about 404-435 | `src/repository/context_retirement/authority.rs:404`, fence :433 | found | |
| BF21 `written_closure_unfinished` | `closure/record.rs` about 1625 | `src/repository/closure/record.rs:1625` | found | holder SQL at :1646 excludes a branch record at `confirmed`/`context_synchronized` (as cited about 1645) |
| BF21 `retire_context_for_request_cancellable` | `request_scope.rs` about 598-641 | `src/repository/context_retirement/request_scope.rs:531` (`execute_retirement_request` at :598) | moved | the file is `context_retirement/request_scope.rs`; the named function is at 531, 67 lines before the cited range |
| BF21 `claim_scoped` | `authority.rs` about 302-335 | `src/repository/context_retirement/authority.rs:302` | found | `branch_hold` :328, `hold_refusal` :330 |
| BF21 `branch_hold`, `hold_refusal` | `closure/record.rs` | `src/repository/closure/record.rs:1679, 1688` | found | |
| BF21 `retire_context_scoped` | `context_retirement/run.rs` about 359-364 | defined `src/repository/context_retirement/execution.rs:252`; called `run.rs:236, 350, 365` | found | the absent-context call is `run.rs:365` |
| BF21 `refuse_fresh_scoped` | `authority.rs` about 530-556 | `src/repository/context_retirement/authority.rs:530` | found | |
| BF21 `TargetNotMaterialized` | `remote/sync.rs` about 650-687 | `src/repository/remote/sync.rs:630-664` | found | |
| BF21 poll consults `written_closure_unfinished` | `poll.rs` about 389, 668 | `src/repository/remote/poll.rs:389, 668` | found | |
| BF22 `refuse_record` | `promotion/run.rs` about 563-600; `promotion/record.rs` about 399-411, 382-384 | `promotion/run.rs:593`; `promotion/record.rs:373-398` (`refuse_record`), `:399` (`insert_record`), existing row returned at :380-382 | found | |
| BF22 runner's `abandon` | `run.rs` about 262 | `src/repository/promotion/run.rs:262` | found | |
| BF22 `save_record` refuses a `context_oid` change | `record.rs` about 434 | `src/repository/promotion/record.rs:414` (fn), check at :435 | found | |
| BF22 `consented_context_oid` | `closure/record.rs` about 397 | `src/repository/closure/record.rs:397` | found | |
| BF22 `recorded_config` needs a base | `run.rs` about 437-447, 525 | `src/repository/promotion/run.rs:437-447` | found | `r.base_oid().ok_or_else(recovery_required)` at :443-444 |
| BF22 `child_state` | `run.rs` about 1166-1176, 1419 | `ChildState`/`child_state` in `promotion/lifecycle_record.rs:650-659`; users `run.rs:1022, 1349, 1419` | found | `child()` at `run.rs:1406` |
| BF22 retained child cannot be let go | `lifecycle_record.rs` about 650 | `promotion/lifecycle_record.rs:650-657` (`ChildState` with `Retained`) | found | |
| BF22 `release_promotion_child` | `remote/reservation.rs` about 1832-1869 | `src/repository/remote/reservation.rs:1832` | found | |
| BF23 `in_the_way` | `setup/change.rs` about 457-481 | `src/repository/setup/change.rs:417, 457` | found | |
| BF23 `context sync-local` hold | `local_sync/record.rs` about 1174 | `src/repository/local_sync/record.rs:1174` | found | |
| BF23 child abandon refusal | `remote/reservation.rs` about 807-844 | `src/repository/remote/reservation.rs:807` (`abandonable`), `:827` loop over `document_promotions`, `ticket_closures` | found | |
| BF23 `context release` refuses its context | `context_release.rs` about 216-227 | `src/repository/context_release.rs:~212-227` (`unowned`) | found | |
| BF24 context child's moves | `sync.rs` about 3825, 3911-3916, 7205-7236, 6703-6708, 6738, 6812 | `sync.rs:3825` (fast_forward), `:3862` (Merge remote context), `:3872` (Merge primary), `:3918` (fast_forward), `:6703-6707`, `:6739`, `:7205` | found | the "ended child stores none" at about 6812 is not located: :6812 is `synchronization_push_absence_check`; `RemoteContextDeleted` arises at `sync.rs:500` and `:4137` |
| BF24 conflict resolution adopts the child | `bind/conflict.rs` about 45-52 | `src/repository/mutation/bind/conflict.rs:44-52` | found | module doc, "decision 34" |
| BF24 fetch never prunes | `transport/remote.rs` about 228-261 | `src/repository/transport/remote.rs:228-262` (`fetch_exact_advertised`) | found | |
| BF25 `item sync` `history_unresolved` code | `outcome.rs` about 240 | `src/repository/mutation/outcome.rs:240` | found | |
| BF25 boundary compare | `execute.rs` about 305-311 | `src/repository/mutation/execute.rs:307-311` | found | |
| BF25 `check_golden` | `recovery_tables.rs` about 318-354 | `tests/cli/recovery_tables.rs:338` | found | `covered` at :582 |
| BF25 `endpoint_digest_of` | `remote/observation.rs` about 125-160 | `src/repository/remote/observation.rs:125` | found | `pub(in super::super)`; used by `setup/change.rs`, `setup/enable.rs`, `transport/operation.rs`; retirement records its digest from `ObservationConfiguration::endpoint_digest()` (`source.rs:66, 156`), not from this function |
| BF25 `forge::recognise` | `forge.rs` about 78-100 | `src/forge.rs:78` | found | `pull_request_link` at :146, `human_line` at :122 |
| BF25 `link_of` reads an unknown forge as none | `pull_request_links.rs` about 24 | `src/repository/pull_request_links.rs:27` | found | |
| BF25 `branch_link` reads URL and setting live | `closure/checkpoint.rs` about 1036-1052 | `src/repository/closure/checkpoint.rs:1036-1052` | found | |
| BF25 `unheld_entries` | `proof.rs` about 102-147 | `src/repository/context_retirement/proof.rs:100` | found | |
| BF25 CLI human-output tests | `src/bin/manyhands-cli/lifecycle_tests.rs` | `src/bin/manyhands-cli/lifecycle_tests.rs` | found | declared `src/bin/manyhands-cli/lifecycle.rs:122-123` |
| BF25 `fixture_directory_is_exactly_the_registered_cases` | `tests/cli` | `tests/cli.rs:103` | found | at the target root, no module prefix |
| BF25 `Fixture::with_remote` | `tests/cli/collaboration_exits.rs` about 446-458 | `tests/cli/collaboration_exits.rs:446` | found | writes no tracking primary |
| PartA `record_tests.rs` E-4 tests | `migration_preserves_legacy_rows_is_idempotent_and_audited` about 366, `half_created_migration_and_missing_marked_table_fail_closed` about 403, `record_forward_transitions_and_invalid_backwards_recover` about 417, `schema_rejects_free_text_categories_steps_and_refusal_payloads` about 560, `privacy_column_inventory_for_both_new_tables` about 581 | `src/repository/promotion/record_tests.rs:366, 403, 417, 560, 581` | found | exact lines |
| PartA `migrate` and `SCHEMA` today | marker `wave_06_promotion_modes_v1`, `SCHEMA_V2` | not present in `src/` or `tests/` | found | new names, no collision: `wave_06_promotion_modes_v1`, `SCHEMA_V2` (only the closure's), `PromotionAnswer`, `BranchPromotionOutcome`, `PullRequestPromotion*`, `primary_holds_document`, `document_link_outstanding`, `manyhands pull request promotion v1`, `pull_request_promotion` all absent |
| PartA `BranchAnswer`-style answer, `answered()` | | `promotion/lifecycle.rs:222` default false | found | |
| PartB `closure/observe.rs` `PrimaryConflicted` mapping | about 1037-1041 | `src/repository/closure/observe.rs:1061, 1067` | found | the in-progress-merge check at :1065-1067 |
| PartB binding's hold (decision 33) | `bind/lifecycle.rs` about 396-435 | `src/repository/mutation/bind/lifecycle.rs:656` (`fn held`, comment :653-655) | moved | lines 396-435 now hold `branch_preview` data and `digest_of` (:422); the hold moved to :656 |
| PartD new schemas / DATA_ROOTS | `lifecycle_preview.schema.json` and the closure's | `schemas/v1/lifecycle_preview.schema.json`, `lifecycle_blocked`, `lifecycle_mutation`, `pull_request_closure_mutation`, `pull_request_closure_preview`, `pull_request_link`, `pull_request` | found | `pull_request_promotion_mutation` and `_preview` schemas do not exist yet (new) |
| PartE `fill_complete_read` and `ItemDto.pull_request` | | see BF15 | found | |
| PartF `item_sync_pull_request_link` golden | | see Task A | found | |
| PartG item 3 `ClosureOutcome` | | see Task A | found | |
| PartG item 4 `replay.rs` `result_commit_held` | | `src/repository/mutation/replay.rs:190` | found | tag `pull_request_closure` at :201 |
| PartG item 9 `give_link`, `branch_link` take the kind | | `closure/record.rs:1461`, `closure/checkpoint.rs:1036` | found | both ticket-only today (see Task A) |
| PartG item 13 library fakes | | `closure/run_pull_request_tests.rs:618` (private), `closure/run_pull_request_lifecycle_tests.rs:19` (child module) | found | no promotion twin exists; `promotion/run_tests.rs` has no submodule |
| E-1 `tests/read_items.rs` comment "nor does a document's read, whatever row it has" | about 6244-6256 | `tests/read_items.rs:6245-6246` | found | text: "and nor does a document's read, whatever row it has." |
| E-3 `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` | `tests/recovery_foundation_gate.rs` about 1075; `ORDER BY name` about 1125 | `tests/recovery_foundation_gate.rs:1075, 1125` | found | |
| E-4 / E-5 / E-6 / E-7 / E-8 / E-9 locations | see rows above | see rows above | found | |
| E-13 SSH case | see Task A | `tests/mutation_remote/c5_lifecycle.rs:3669` | found | |
| E-14 `sync_tests.rs` comment | about 587-594 | `bind/sync_tests.rs:587-594` | found | the comment there does not say documents never get a link (see Discrepancies) |
| Task 0 sweep: column scans and row inserts | `tests/ticket_closure.rs` about 449; `tests/document_promotion.rs` about 886; `tests/read_status.rs` about 1798 | `tests/ticket_closure.rs:448-449`; `tests/document_promotion.rs:886, 891`; `tests/read_status.rs:1798, 1882` | found | |
| D-1 contract rows | `cli-contract.md` rows about 166, 181, 183, 341, 1592; known limit decision 35 | `docs/RFC/cli-contract.md:166` (`document list`, `document show`), `:181` (`item sync`), `:183` (`document promote`, `ticket close`), `:341` (the `--closure` amendment paragraph holding "null in every list, for a document and for a ticket with none"), `:1592`, `:1929-1936` (decision 35 text) | found | the `item sync` row at :181 still says "close and promote are unchanged until Wave 06's later Cycles" |
| D-4 skill sentences | `sharing-and-sync` about 47; `recovery` about 183, 187 (`.agents` about 179, 183); `tickets` about 236; `finding-work` about 82; `repositories` about 154; `overview/references/concepts.md` about 25 | `skills/manyhands-sharing-and-sync/SKILL.md:47`; `skills/manyhands-recovery/SKILL.md:183, 187`, `.agents/skills/manyhands-recovery/SKILL.md:179, 183`; `skills/manyhands-tickets/SKILL.md:236`; `skills/manyhands-finding-work/SKILL.md:82`; `skills/manyhands-repositories/SKILL.md:154` | changed | the first five are present as cited (each mirrored in `.agents/skills/` at the same line except recovery). `skills/manyhands-overview/references/concepts.md` line 25 has no sentence that promote still pushes the primary (it describes `ticket close` in a pull-request repository; line 26 describes promote without mention of mode): nothing to remove there |
| D-4 "not usable end to end" wording | `polling-and-index` about 62; `overview/references/errors-and-exit-codes.md` about 42; `repositories` about 154, 164 | `skills/manyhands-polling-and-index/SKILL.md:62`; `skills/manyhands-overview/references/errors-and-exit-codes.md:42`; `skills/manyhands-repositories/SKILL.md:154, 164` | found | `.agents/skills` mirrors have the same lines |
| D-4 `manyhands-documents` promote section | about 147; "never mentions pull-request mode" | `skills/manyhands-documents/SKILL.md:~146` (`## Promote: show the person first`) | found | no "pull-request" or "pull_request" text in the file |
| D-5 audit's `document promote` rows | `docs/research/wave-03-api-audit.md` | `docs/research/wave-03-api-audit.md:549` (codes row `M + R + C + I + T + Y + setup_pending`, count 42), `:530`, `:2000` | changed | the code set already contains `already_applied`, `remote_branch_deleted` and `recovery_required` (group Y, `:728-735`); see Discrepancies |

### Task C: test module paths and filters

Filters are for `devenv shell -- cargo test ... -- --exact`. `<t>` is a test name.

| Name | Design cites | Found at (file:line) | Status | Note |
| --- | --- | --- | --- | --- |
| tests of `repository::promotion::record` | `record_tests.rs` | `src/repository/promotion/record_tests.rs`, declared `src/repository/promotion/record.rs:546-548` (`#[path = "record_tests.rs"] mod tests;`) | found | `cargo test --lib repository::promotion::record::tests::<t> -- --exact` |
| tests of `repository::promotion::observe` | `observe_tests.rs` | `src/repository/promotion/observe_tests.rs`, declared `observe.rs:1393-1395` | found | `--lib repository::promotion::observe::tests::<t>` |
| `repository::promotion::run_tests` | `run_tests.rs`; a `pull_request` submodule | `src/repository/promotion/run_tests.rs`, declared `src/repository/promotion/mod.rs:23-25` (`pub(super) mod run_tests`) | found | `--lib repository::promotion::run_tests::<t>`. No `pull_request` submodule exists in the promotion file (it has no `mod` lines): the design's `pull_request` module is new; declared with `#[path]` it would be `repository::promotion::run_tests::pull_request::<t>`. The closure analogue is `repository::closure::run_tests::pull_request::<t>` (`closure/run_tests.rs:35-36`) with a child `...::pull_request::lifecycles::<t>` (`run_pull_request_tests.rs:6151-6152`). Other promotion test modules: `repository::promotion::lifecycle::tests::`, `repository::promotion::integrate::tests::` |
| tests of `repository::mutation::bind::sync` | `sync_tests.rs` | `src/repository/mutation/bind/sync_tests.rs`, declared `bind/sync.rs:1683-1685` | found | `--lib repository::mutation::bind::sync::tests::<t>`, e.g. `...::tests::item_sync_gives_the_link_of_a_closing_ticket`. The module chain is `repository::mutation` (`repository.rs:69`) then `bind` (`mutation/mod.rs:45`) then `sync` (`bind/mod.rs:62`, `pub(crate)`) |
| `bind::lifecycle` tests | | `src/repository/mutation/bind/lifecycle_tests.rs`, declared `bind/lifecycle.rs:2022-2024` | found | `--lib repository::mutation::bind::lifecycle::tests::<t>` |
| CLI binary human-output tests | `src/bin/manyhands-cli/lifecycle_tests.rs` | `src/bin/manyhands-cli/lifecycle_tests.rs`, declared `src/bin/manyhands-cli/lifecycle.rs:121-123`; `mod lifecycle` at `src/bin/manyhands-cli.rs:46-47` | found | `cargo test --bin manyhands-cli lifecycle::tests::<t> -- --exact`, e.g. `lifecycle::tests::a_branch_record_answer_naming_blockers_is_not_retried_automatically`. It imports `human_lifecycle::{self, output}`; `human_lifecycle.rs` (the code under test) is `manyhands-cli.rs:38` |
| `results::tests` | | library crate `manyhands`: `src/results.rs:1138-1140` (`#[path = "results_tests.rs"] mod tests;`), file `src/results_tests.rs` | found | `--lib results::tests::<t>`, e.g. `results::tests::result_codes_match_the_design_table_exactly` |
| `context_retirement` tests (P13) | `tests.rs` or `mutation_tests.rs` | `src/repository/context_retirement/tests.rs` (`mod.rs:20-21`); `execution_tests.rs` (`mod.rs:26-27`) declares `mutation_tests` (:5-6) and `index_claim_tests`; `mutation_tests.rs` declares `acceptance_tests` (:6-7) | found | `--lib repository::context_retirement::tests::<t>`; `repository::context_retirement::execution_tests::<t>` (holds `a_finalized_retirement_deletes_the_link`); `repository::context_retirement::execution_tests::mutation_tests::<t>`; `...::mutation_tests::acceptance_tests::<t>`. `store_tests` is `mod.rs:23-24` |
| `repository::cleanup` and `remote::poll` tests | | `cleanup.rs:1024-1026` (`cleanup_tests.rs`); `remote/poll.rs:2552-2554` (`poll_tests.rs`) | found | `repository::cleanup::tests::<t>`; `repository::remote::poll::tests::<t>` |
| integration target `read_items` | `tests/read_items.rs` | `tests/read_items.rs` (+ `tests/read_items/local_sync.rs`, `tests/support/`) | found | `--test read_items ticket_show_gives_the_link_and_the_publication -- --exact` (top-level fn at :6220, no module prefix) |
| integration target `read_contract` | `tests/read_contract.rs` | `tests/read_contract.rs` (`mod support` :36) | found | `--test read_contract <t>` |
| integration target `mutation_contract` | `tests/mutation_contract.rs` | `tests/mutation_contract.rs` | found | `--test mutation_contract the_fixture_directories_are_exactly_the_registered_cases -- --exact` (top-level fn :1511); `DATA_ROOTS` at :1605 |
| integration target `cli` | `tests/cli.rs` | `tests/cli.rs` (mods :13-100, e.g. `mod lifecycle`, `mod inventory`, `mod recovery_tables`, `mod collaboration_exits`) | found | `--test cli lifecycle::<t>`, `inventory::every_verb_s_data_has_an_output_schema_and_its_goldens_match_it`, `recovery_tables::<t>`, and `fixture_directory_is_exactly_the_registered_cases` (:103, no prefix) |
| integration target `mutation_confirmation` (prepare's golden) | `tests/mutation_confirmation/conformance.rs` | `tests/mutation_confirmation.rs:5799-5800` (`#[path] mod conformance`) | found | `--test mutation_confirmation conformance::withdraw_with_no_closure_is_already_applied -- --exact` |
| integration target `mutation_remote` | entry file under tests/ | `tests/mutation_remote.rs` (`harness = false`, `Cargo.toml:83-84`; custom `main` :67, `ssh_harness::run_with_output_privacy`); case table `tests/mutation_remote/c5_lifecycle.rs` (`CASES`, :248) | found | not a libtest target: the case name is a positional filter, `--test mutation_remote a_pull_request_close_publishes_only_the_ticket_branch -- --exact` (`parse_options`, `tests/support/ssh_harness.rs:316-343`); a name has no module prefix |
| Markdown tables parsed by `tests/cli/recovery_tables.rs` | "the CLI contract's tables" | `docs/RFC/cli-contract.md`, section `## Recovery tables` (`tests/cli/recovery_tables.rs:210-222`), its `###` subsections `Authoring`, `Repository and credentials`, `Collaboration`, `Lifecycle`, `Local sync`, `Polling`, `Codes with no golden` (:90-98, :246-253); row tables with header `\| Verb \| Code \| Next \| Exit \| Golden \|` and, under `### Codes with no golden`, `\| Scope \| Code \| Reason \|` (:22-23). It also parses `docs/research/wave-03-api-audit.md`: the fenced block under `#### <cycle> exit-coverage union` (:468-480) and, from `### Wave 03 CLI Cycle C4 refresh` ("The groups are:" definitions) and `#### C5 bindings and result-set key`, the table `\| Verb \| Codes \| Count \| Design anchor \|` (:482-535) | changed | the design says both tests "parse the CLI contract's tables"; recovery_tables parses the contract and the audit |
| Markdown tables parsed by `tests/cli/collaboration_exits.rs` | "the CLI contract's tables" | `docs/research/wave-03-api-audit.md` only (`include_str!` at `collaboration_exits.rs:21`): the fenced `text` block under `#### C4 exit-coverage union` (:322-329); the groups after `### Wave 03 CLI Cycle C4 refresh` ("The groups are:", :1021-1030, 10 groups asserted at :1048); and the table `\| Verb \| Codes \| Count \| Design anchor \|` under `#### C5 bindings and result-set key` (:1051-1058) | changed | it does not read `cli-contract.md` at all. The `document promote` row of that audit table (`audit.md:549`) is read by both tests |

### Discrepancies

- **U-1 / Task A:** no Cycle 03 name is renamed, missing or changed in a way that stops the design. Shape notes only: `BranchEnd<'a>::ContextChild` carries `&SynchronizationError`; `give_link`, `branch_link`, `branch_data` and `fill_complete_read` hard-code the ticket kind (`"ticket"` at `closure/record.rs:1470`, `KIND` at `closure/checkpoint.rs:44`, `AuthoringKind::Ticket` at `bind/lifecycle.rs:998-1000` and `read/items.rs:1544`), as Part G item 9 expects; `upsert_link` has no update-`given_at`-alone sibling; `SyncBinding::link_given` stamps `given_at` with `OffsetDateTime::now_utc()` (`bind/sync.rs:1172`), not an injected clock; `PullRequestIo` and `Forge` are private to the closure's test modules (`Forge` is in the child module `lifecycles`, and `PullRequestIo` does not use it).
- **PartB blockers, the binding's hold (decision 33):** design "bind/lifecycle.rs about 396-435"; code: `fn held` at `src/repository/mutation/bind/lifecycle.rs:656` (moved about 260 lines).
- **BF10 `finalize_index`:** design "context_retirement/record.rs about 2044"; code `src/repository/context_retirement/record.rs:1966`.
- **BF21 `retire_context_for_request_cancellable`:** design "`request_scope.rs` about 598-641"; code `src/repository/context_retirement/request_scope.rs:531` (the file is under `context_retirement/`; `execute_retirement_request` is at :598).
- **BF4 `FetchTracking`:** design reads it as a `sync.rs` name; code: `TransferAction::FetchTracking` at `src/repository/transport/remote.rs:19`.
- **BF24 "ended child stores none (sync.rs about 6812)":** not located; `sync.rs:6812` is `synchronization_push_absence_check`. The behaviour the design relies on (`RemoteContextDeleted` from the evidence, `sync.rs:500`, `:4137`) exists.
- **BF25 / Part A `endpoint_digest_of` "as retirement records it":** the function exists (`remote/observation.rs:125`, `pub(in super::super)`) and is used by setup and transport; retirement records `ObservationConfiguration::endpoint_digest()` instead (`context_retirement/source.rs:66, 156`).
- **Task C promotion `run_tests` `pull_request` module:** none exists yet in `src/repository/promotion/run_tests.rs`; the filter would be `repository::promotion::run_tests::pull_request::<t>` once added. The closure's `pull_request` module is `repository::closure::run_tests::pull_request`.
- **E-10 / recovery tables / D-5:** `tests/cli/collaboration_exits.rs` does not parse the CLI contract; it parses `docs/research/wave-03-api-audit.md` (`#### C4 exit-coverage union`, C4 refresh groups, `#### C5 bindings and result-set key`). `tests/cli/recovery_tables.rs` parses both `docs/RFC/cli-contract.md` (`## Recovery tables`) and the same audit tables. The audit row `document promote` is `M + R + C + I + T + Y + setup_pending`, count 42 (`audit.md:549`), and group Y (`audit.md:728-735`) already contains `already_applied`, `remote_branch_deleted` and `recovery_required`. So D-5's "document promote's codes gain `already_applied`" is already true of the set, and any edit to that row that changes the codes or the count 42 would be read by both tests: E-10's "none expected" should name the audit as well as the contract.
- **D-4 `manyhands-overview/references/concepts.md` about 25:** no sentence there says promote still pushes the primary until Cycle 04; line 25 is about `ticket close`, line 26 about promote with no mode note. The other D-4 sentences are present at the cited lines (`.agents/skills/` copies at the same lines, except `manyhands-recovery` at 179 and 183); six `.agents/skills` files already differ from `skills/` (`finding-work`, `overview`, `cli-conventions.md`, `polling-and-index`, `recovery`, `tickets`).
- **E-14 `bind/sync_tests.rs` comment:** the comments at `sync_tests.rs:508-514` and :566-571 say nothing like "documents never get a link"; the document case (:587-594) asserts only that a document target with no row gets no data and no row. Nothing to reword unless the design means the whole-test doc comment ("`item sync` of a ticket ... Every other answer is unchanged: no data and no row").
- **BF3 `DocumentPromotionPreflight`:** 13 files mention it (design says about twelve): `lifecycle_verb.rs`, `promotion/{integrate_tests,lifecycle,mod,observe,observe_tests,run_tests}.rs`, `tests/cli_remote/{interruption,lifecycle}.rs`, `tests/document_promotion{,/journeys}.rs`, `tests/drafts/targets.rs`, `tests/mutation_remote/c5_lifecycle.rs`. The enum has two variants (`Ready`, `Blocked`).
- **BF16 `CLI_CASES` "its `golden()` asserts the name about 587":** there is no `golden` function; the name checks are `assert_cli_golden` and `assert_cli_text_golden` (`tests/support/cli.rs:591, 597-598`).

## Appendix B: Premise sweep and CI, Task 0 Steps 4 and 5 (research agent, verbatim)

### Task 0 sweep: tests and fixtures the pull-request promote design breaks

Read-only sweep of worktree 01M87684C9GM9BJP5QQ429KMAK at HEAD aee99255 (main 8e5d9832) against the design `docs/plans/2026-11-26-wave-06-cycle-04-pull-request-promote-design.md` (revision 7, Parts A, D, E, F, G, Named Edits, Required Outcomes, Rulings). Nothing was edited, built or run. Line numbers are the ones at this HEAD.

### Result in one paragraph

The sweep finds 3 new edits (E-15 to E-17), all in `src/` unit tests, all mechanical (a struct literal gains the fields the design adds), none removing, weakening or reversing an assertion's intent. Every other hit is covered by E-1 to E-14, or is unaffected. Two contingencies are not edits but decisions the Task 1 and Task 2 checkpoints must take so that the count stays at 3: the free function `promotion::record::begin_record` must keep its signature (about 32 test call sites otherwise), and the U-2 lift of `PullRequestIo` and `Forge` is optional.

### (c) New edits the design forces (E-15 onwards)

| E-n | File:line | Test/fixture | Now asserts | Must change | Design part | Reverses intent? |
| --- | --- | --- | --- | --- | --- | --- |
| E-15 | `src/repository/promotion/lifecycle_tests.rs:88-127` (the struct literal at line 90) | `promotion_steps_map_to_the_shared_step_view` | Builds a `record::PromotionRecord { ... }` with every field spelled out, then walks all 13 `PromotionStep`s and asserts `step.name()`, `r.step()` and `set_step` map to the shared `ParentStep` view; `shared_view` (line 31) is an exhaustive match on `PromotionStep` | The literal gains the new record fields (`integration_mode`, `answer_code`, `consented_context_oid`, `situation`, `primary_branch`, `endpoint_digest`, or whatever names Task 1's checkpoint settles) with the direct values (`Direct`, `None`). The 13 steps and `shared_view` do not change: Part A adds no step and no name | Part A (the record's new columns) and Part G item 1 | No. The assertions (step names and the shared step view) stand unchanged; only the literal's field list grows. No other `PromotionRecord { ... }` literal exists outside `record.rs` `decode` (checked by grep) |
| E-16 | `src/repository/mutation/bind/lifecycle_tests.rs:1117-1135` (the `ready` closure) | `a_branch_record_s_answers_name_the_ticket_s_branch_and_a_direct_one_s_name_primary` (starts line 1082) | The binding's `Ready { expected, branch, primary_branch, remote, context_oid, primary_oid, context_child, changes, integration, publication, cleanup, closure, record }` is spelled out in full; the test asserts a branch record's answers name the ticket's branch and worktree and a direct one's, or a binding that learned nothing, name primary | The literal gains whatever Part G item 12 adds to `Ready` (a promotion's pull-request facts `L`, `R`, situation, link, endpoint, beside `record`) and follows `primary_oid` if it becomes optional or absent (`Option<Oid>` or removed). Any field added or retyped on `Ready` forces this; the only way to avoid it is not to touch `Ready`, which item 12 excludes | Part G item 12 (and Baseline Fact 1: `Ready` holds `primary_oid`, line 83 of `lifecycle_verb.rs`) | No. The test's assertions (scope names the ticket branch, the primary otherwise) are about the ticket verb and stay as they are; the literal is built with the new field empty |
| E-17 | `src/repository/mutation/bind/lifecycle_tests.rs:1260-1290` (the `ready` closure) | `a_branch_record_previews_its_own_effects_and_details` (starts line 1250) | Same `Ready` literal, with `primary_oid: oid("2")` and `record: Some(BranchReady {..})`; asserts each close action's effects in order and the details a branch record shows (action, commits, closure, unheld paths, link) | Same as E-16: the new field, and `primary_oid` if retyped. The `primary_oid` value `oid("2")` is also read back by the test's expected details (`tracking_primary_oid` is a different field, line 1377), so if Task 2 makes `primary_oid` optional the literal becomes `Some(oid("2"))` and no expectation moves | Part G item 12 | No. Only the literal changes; every expectation stands |

Both `Ready` literals are the only places outside production code that construct `Ready` (grep `Ready {` in `src/repository/mutation/bind/`). E-12 names the `DocumentPromotionPreflight` matches that item 12's shape changes; it does not name the `Ready` literals, which is why they are listed here. If Task 2's checkpoint finds a shape that leaves `Ready` untouched, E-16 and E-17 drop out; E-15 stays in any case.

#### Contingent, not edits (decide at the checkpoints)

- `promotion::record::begin_record(tx, id, op, item, expectation, remote)` (record.rs:352) has about 32 test call sites: `record_tests.rs` 21, `remote/reservation_tests.rs` 3, `remote/sync_tests.rs` 2 (lines 18207 and 19760), `promotion/lifecycle_tests.rs` 2, `local_sync/admission_tests.rs` 1 to 2, `local_sync/review_tests.rs` 1, `context_release_tests.rs` 1, `recovery.rs:4013` 1 (a `#[cfg(test)]` helper). Part G item 1 says "`begin_record` takes the mode and Part A's columns from the observation". The trait method `Lifecycle::begin_record` already takes `observed: Option<&DocumentPromotionPreview>` (promotion/lifecycle.rs:895-904), so the mode can reach the record there. Cycle 03 kept `closure::record::begin_record` and added `begin_branch_record` and `begin_new_branch_record` (closure/record.rs:774, 803, 832), so no closure test call site changed. Recommend the same twin here (`begin_branch_record`), which keeps these 32 sites untouched; changing the free function's signature would add about 32 mechanical edits (E-18 and on) with no change of intent. This is a Task 1 checkpoint decision.
- Part G item 13 (U-2): lifting `PullRequestIo` (`closure/run_pull_request_tests.rs` about 618-775) and the test-local `Forge` (`closure/run_pull_request_lifecycle_tests.rs` about 17-250) into a shared test-support module would edit those two closure test files (a move, intent unchanged). The design allows promotion twins instead, so it is not forced.
- `Called::Branch` (bind/lifecycle_verb.rs:120-125), `branch_reached` (lifecycle.rs:886) and `branch_code` (lifecycle_verb.rs:676) are called directly by about 10 closure tests in `bind/lifecycle_tests.rs` (lines 331-1492) with `BranchClosureOutcome` and `BranchAnswer`. Part G item 4 gives promotion *twins* of `branch_reached`, `branch_data` and `branch_code`, so these tests are unaffected; if the implementation instead widens the closure-typed signatures to an enum, those tests change (not forced by the design as written).

### (a) Hits already covered by a named edit

| Named edit | Hits in the sweep that it covers (file:line) | Check |
| --- | --- | --- |
| E-1 | `tests/read_items.rs:6246-6257` (comment at 6246-6247, `give_link(data, "document", DOCUMENT_A)` at 6248, `note.pull_request == None` at 6257) | The only test that inserts a link row (`give_link`, line 6159) and the only one that reads a document's `pull_request`. `docs/note.md` is committed on the primary with no context, so the assertion stands under Part E, as E-1 says |
| E-2 | `tests/mutation_contract.rs:1605` (`DATA_ROOTS: [&str; 77]`), 1748, 1793 | 77 to 79 and two list entries; `data_root_differences` (1755-1784) reports a root no golden carries, hence "with the golden that carries its data" |
| E-3 | `tests/recovery_foundation_gate.rs:1125-1143` (marker list, `ORDER BY name`) and the `promotion_refusal_blockers_v2` entry at 1139 | `wave_06_promotion_modes_v1` sorts after `wave_06_closure_modes_v1` (1142). This is the only place a marker list is asserted outside the two record_tests files |
| E-4 | `src/repository/promotion/record_tests.rs:366` `migration_preserves_legacy_rows_is_idempotent_and_audited`, 403 `half_created_migration_and_missing_marked_table_fail_closed`, 417 `record_forward_transitions_and_invalid_backwards_recover`, 560 `schema_rejects_free_text_categories_steps_and_refusal_payloads`, 581 `privacy_column_inventory_for_both_new_tables` (the `document_promotions` allowed-column list at 584-600), 1417-1458 `promotion_refusal_blockers_v2_rebuilds_an_old_bound_table_once` (its `INSERT INTO document_promotions SELECT * FROM before` at 1429, a column-order copy that breaks on the new DDL) | All five named tests exist at the named lines. The 1417 test is the one `SELECT *` copy from the current DDL in the tree |
| E-5 | `src/repository/promotion/run_tests.rs:5069-5074` (`authoritative`, exhaustive on `Complete`, `IndexPending`), `tests/document_promotion.rs:314-320` (`promote`), `tests/document_promotion/journeys.rs:257-261` (`authoritative`), `tests/document_promotion/quality.rs:130-134` (`promotion_category`) | These four are the only exhaustive matches on `DocumentPromotionResult` outside production (`lifecycle_verb.rs:459-464`). Every other test hit (run_tests.rs 1365, 1445, 1586, 2760, 5065, 5285, 5393, 5708, 5709, 6018, 6038, 6279, 6409, 6465, 6561, 6620, 6658, 7080, 7264; `advanced.rs:131`; `identity.rs:144, 153`; `quality.rs:232`) is a `matches!`, a `let ... else`, a `match` with a wildcard or `other` arm, or a tuple match with a catch-all, so unaffected. `run_tests.rs:5063` `outcome` panics on `other` |
| E-6 | `tests/support/mutation.rs:674-704, 711-741, 852-882, 889-919, 926-956, 963-993` (six exhaustive matches), `tests/support/repair.rs:335-346` | These are the only exhaustive matches on `MutationDataDto` in tests. The other 400 or so `MutationDataDto::` hits in tests are constructions, `Some(MutationDataDto::X(d)) => d` with a fallback arm, `let ... else`, or `matches!` (checked per file: `bind/lifecycle_tests.rs:225, 454`, `made_ready_tests.rs:137`, `human_collaboration_tests.rs:94`, `tests/mutation_remote/*`, `tests/mutation_bridges/*`) |
| E-7 | `tests/support/cli.rs` `CLI_CASES` and `CLI_TEXT_CASES` | Fixture directory is checked against the registered cases (`fixture_directory_is_exactly_the_registered_cases`) |
| E-8 | `tests/cli/inventory.rs:267-274` (`document promote` DATA row: `lifecycle_preview`, `lifecycle_blocked`, `lifecycle_mutation`) | `ticket close`'s row (280-292) is the pattern |
| E-9 | `tests/support/mutation_golden.rs:56-65` (`document_promote_*` cases) and `tests/mutation_confirmation/conformance.rs` (beside `withdraw_with_no_closure_is_already_applied`) | |
| E-10 | `tests/cli/recovery_tables.rs`, `tests/cli/collaboration_exits.rs` | They parse the CLI contract's tables; D-1 edits the `document promote, ticket close` row. None expected, Task 9 confirms |
| E-11 | `tests/read_contract.rs:916-972` (the enumeration table, shaped on `pull_request_closure_*`) | An enum the two new schemas declare adds rows; existing rows are unchanged |
| E-12 | The one exhaustive `DocumentPromotionPreflight` match in tests: `tests/document_promotion.rs:301-308` (`Ready`, `Blocked`) | See the (b) list for the non-exhaustive ones. E-12 covers this one if item 12 adds a `Preflight` variant; it does not if the shape leaves `DocumentPromotionPreflight` alone |
| E-13 | `tests/mutation_remote/c5_lifecycle.rs:3669-3934` `a_pull_request_close_publishes_only_the_ticket_branch` | Also the P10 case. Its `w.privacy()` call (3648) scans durable state with push URLs and paths as probes; a pull-request promote row holds `primary_branch` (`main`) and an `endpoint_digest` (a digest, not a URL), neither a probe |
| E-14 | `src/repository/mutation/bind/sync_tests.rs:587-594` (the document case of `item_sync_gives_the_link_of_a_closing_ticket`, the `document` binding with `AuthoringKind::Document`) | The `Closing` fixture writes no link row for a document, so the assertion (no data, no row) stands under Part F |

### (b) Hits not affected, and why

Counts of the pattern in `src/` and `tests/` (production code outside any `#[cfg(test)]`, test files, and inline `#[cfg(test)]` modules). Production hits are not rows: the implementation extends them.

| Pattern | Production | Test files | Inline test modules | Verdict |
| --- | --- | --- | --- | --- |
| `DocumentPromotionResult::` | 4 (`lifecycle_verb.rs:459, 462`, `promotion/lifecycle.rs:1163, 1166`) | 33 | 0 | E-5 covers the four exhaustive test matches; the rest have a wildcard, `else` or `matches!` |
| `DocumentPromotionPreflight::` | 7 (`lifecycle_verb.rs` 3, `promotion/lifecycle.rs` 2, `observe.rs` 2) | 14 | 0 | One exhaustive test match (E-12). The others take `Ready(p) => p, other => panic!` (`observe_tests.rs:404`, `integrate_tests.rs:185`, `run_tests.rs:233, 693`, `drafts/targets.rs:501`), or `if let (Ready, Ready)` (`run_tests.rs:815`), or `let ... else` (`cli_remote/interruption.rs:2355`, `cli_remote/lifecycle.rs:639`, `c5_lifecycle.rs:2770`), or `matches!` (`journeys.rs:676`), or an `assert_eq!` on `Blocked { .. }` (`observe_tests.rs:411`); all have a catch-all |
| `MutationDataDto::` | 115 | 305 | 23 | E-6 covers the seven exhaustive test matches. Production exhaustive matches (`desktop/model/operations/review.rs:52-74`, `human_admin.rs`, `human_collaboration.rs:250-300`, `dto.rs`) are R-14 compile-level arms, not test edits |
| `PromotionStep::` | 17 | 77 | 0 | The design adds no step and renames none. `permits` is private to `record.rs` (record.rs:35, 431) and no test calls it. `record_tests.rs`, `run_tests.rs`, `remote/reservation_tests.rs` (6) and `remote/sync_tests.rs` (6) set `r.step = PromotionStep::...` on direct records begun with `begin_record`; the default mode stays `direct`. `lifecycle_tests.rs:31` `shared_view` is exhaustive on the 13 steps and stays (E-15 notes it) |
| `written_closure_unfinished` | 7 | 5 | 0 | `closure/record_tests.rs` (4) and `closure/run_pull_request_tests.rs` (1) test the closure's fence. The design reads a promotion fence *beside* it (Part G item 14) and leaves the closure's query alone |
| `document_promotions` | 28 | 43 | 2 | See the table below |
| `promotion_refusal_blockers_v2` | 1 | 4 | 0 | E-3 (1) and E-4 (3) |
| `manyhands promotion v1` | 2 | 1 | 0 | `observe_tests.rs:145` is an independent byte-vector of the direct digest (`t2_1_deletion_encoding_has_zero_big_endian_u32_mode`). The design gives the pull-request observation "its own domain" (R-2, Part G item 5), so the direct wire format does not change and this test is the guard P12 relies on. `closure/observe.rs:42` is a comment |

The `document_promotions` hits that are neither named nor edited:

| File:line | Test | Why unaffected |
| --- | --- | --- |
| `tests/ticket_closure.rs:449` | `row_policy().workflow` | A list of table names handed to `privacy_rows::scan_rows`, which does `SELECT *` over each table and checks every text or blob cell against probes (`tests/support/privacy_rows.rs:74-142`). No column list. The probes are names, emails, titles, bodies and URLs; the new columns hold `direct`/`pull_request`, answer codes, OIDs, `main` and a digest, so a clean scan stays clean |
| `tests/document_promotion.rs:886-893` | `ROW_POLICY` | Same scanner, same reasoning; `required: "document_promotions"` only proves the table exists |
| `tests/read_status.rs:1798` | `insert_promotion` | An `INSERT INTO document_promotions (repository_id, operation_id, item_id, expectation, remote_name, step, base_oid, updated_at, refusal_category)` with an explicit column list; the new columns are nullable or `DEFAULT 'direct'` (Part A), so the insert stays valid. The `UPDATE ... SET step, context_oid, integration_kind, integration_oid, publication_category` at 1882 sets a direct `complete` row, valid under the per-mode CHECKs |
| `tests/read_status.rs:1792-1900` | `an_unfinished_promotion_lists_resume` | Reads `operation list`/`show` of a direct row; unchanged |
| `tests/document_promotion/identity.rs:41`, `journeys.rs:294, 1011, 1013, 1190, 1236`, `advanced.rs:18, 20`, `quality.rs:326`, `exits.rs:316`, `document_promotion.rs:522` | privacy image, step and child-ID reads, a trigger on `step` | Name specific old columns or `SELECT *` for an image; the old columns keep their names and meanings |
| `tests/cli/lifecycle.rs:883` | `cells_holding` | Asserts the table exists, then scans every cell of every table for sentinels |
| `tests/repository_enablement.rs:600` | table-name list | Names only, no new table |
| `tests/support/lifecycle_world.rs:295` | `table(verb)` | Returns the table name; users run `COUNT(*)`, `SELECT operation_id` and `ALTER TABLE ... RENAME` (`c5_lifecycle.rs:1117, 2469`, `c5_cancel.rs`, `c5_resume.rs`) |
| `tests/recovery_foundation_gate.rs:1136` | marker `cycle_09_document_promotions` | Stays in the list; E-3 adds the new marker beside it |
| `tests/mutation_replay.rs:415-470`, `tests/read_repository.rs:626`, `tests/discovery_rebuild.rs:3302` | registry dump and marker deletions | They compare a registry opened by this build with itself or delete other markers; the new migration is idempotent and runs on open, so a dump taken after the first open is stable |
| `src/repository/recovery.rs:4630, 4649` | `a_remote_row_is_the_request_s_that_began_it_or_its_parent_s` | Creates its own four-column `document_promotions` table; never runs the registry migration |
| `src/repository/local_sync/record.rs:1062`, `record_tests.rs:389`, `registration_removal_tests.rs:186`, `setup/change_tests.rs:549-570, 796, 848` | foreign-identity table lists and `w.parent("document_promotions", step)` | Table names and an `INSERT` with an explicit column list (`repository_id, operation_id, item_id, expectation, step, updated_at`); direct rows. A pull-request promotion holds a setting change like any unfinished promotion (Part G item 8) with no change to these tests |
| `src/repository/promotion/run_tests.rs:570, 2309, 2528, 3259, 3763, 4910` | triggers and an image helper | Triggers on `step`, `cancel_requested`, `cleanup_category`; `operation_image` does `SELECT *` into a before/after comparison |
| `src/repository/promotion/record_tests.rs:374, 406, 408, 676, 847, 1163, 1207, 1391` | other record tests | Drop and recreate the table or its index, set old columns, or read `step` and `refusal_blockers`; E-4 names the tests that need new rows. `service_start_audits_promotion_rows_and_schema` (659-681) drops the unfinished index and expects `recovery_required`; the index is kept (Part A) |

Other (b) hits worth knowing:

- `src/repository/remote/sync_tests.rs:19738-19800` (inside `o11_o12_o13_the_mode_is_inserted_once_and_read_back`) begins a *direct* promotion record in a pull-request repository, drives its primary child and asserts `pull_only = 0` ("A promotion's primary child is direct, as close's is"). The design leaves the pull-only exemption (`sync.rs` about 6474) alone and a pull-request promote has no primary child, so the assertion stands. It depends on `begin_record` keeping its signature (see the contingent note).
- `src/repository/mutation/way_out_tests.rs:810-845` asserts the `SynchronizationConflict` route for `ticket close` and `ticket withdraw-closure` and that `item sync` gets none. It names no `document promote` conflict, so adding the `document promote` route (Part D) breaks nothing; the last assertion is about `item sync`.
- `tests/cli/lifecycle.rs`: every pull-request case uses `Case::pull_request(CLOSE or WITHDRAW)` (lines 973-1686); no existing case previews or confirms a `document promote` in a pull-request repository, so there is no old behaviour pinned.
- `src/repository/mutation/dto.rs` cfg(test) modules (3259-3500) have no `DocumentPromote` row; Part D's `from_stored` guard adds rows, and the stored promote data keeps its `"type": "lifecycle"` tag (`LifecycleMutationDto.r#type`), so replays of existing stored answers still read.
- `tests/cli/skills.rs` (494 lines): checks front matter, links, that fenced commands are catalogued verbs and flags, that `json schema=` blocks validate, the README's minimum version, and the autonomy table. It pins none of the "until Cycle 04" sentences (grep of `tests/` and `src/` for "still pushes", "until Wave 06", "Cycle 04", "end to end" finds no skill text). D-4's new text must keep those checks green (new fenced commands must be real verbs and flags, links must resolve).

#### Golden fixtures: `"pull_request": null` in document goldens

The goldens that carry a document's `pull_request: null` are `tests/fixtures/read_v1/document_show.json` (27), `document_show_nonconforming.json` (34), `document_list.json` (34, 69), and their `tests/fixtures/cli_v1/cli_document_show.json` (27), `cli_document_show_nonconforming.json` (34), `cli_document_list.json` (34, 69) twins. None changes: Part E fills `pull_request` only for a document read from its active context that has a link row, and the only test that inserts a link row at all is `tests/read_items.rs:6159` (E-1); no read golden's fixture has one, and lists stay null. The ticket goldens with `null` (`ticket_*.json`, `cli_ticket_*.json`) are about tickets and are not touched. The mutation goldens `item_sync_pull_request_link.json` and `ticket_close_*` are reused or unchanged.

#### Skills: the sentences saying promote still pushes the primary until Cycle 04

All in `skills/` and `.agents/skills/` (they differ elsewhere, so each is a separate edit in D-4); no test pins them.

| File | skills/ line | .agents/skills/ line | Sentence (shortened) |
| --- | --- | --- | --- |
| `manyhands-sharing-and-sync/SKILL.md` | 47 | 47 | "`document promote` is the one verb still direct: in a pull-request repository it still merges into the primary and pushes it, until Wave 06's Cycle 04 changes it" |
| `manyhands-recovery/SKILL.md` | 183 | 179 | "`document promote` still merges into the primary and pushes it there until Wave 06's Cycle 04 ... tell the person before you run one" |
| `manyhands-recovery/SKILL.md` | 187 | 183 | "`document promote` in a pull-request repository still pushes the main branch until Wave 06's Cycle 04" |
| `manyhands-tickets/SKILL.md` | 236 | 236 | "This is for tickets only: `document promote` ... still merges into the primary and pushes it until Wave 06's Cycle 04" |
| `manyhands-finding-work/SKILL.md` | 82 | 82 | "(This is for tickets: `document promote` ... still pushes the main branch until Wave 06's Cycle 04." |
| `manyhands-repositories/SKILL.md` | 154 | 154 | "`document promote` does not yet: ... still merges into the primary and pushes it ... until Wave 06's Cycle 04, and the cleanup and reporting after a merge arrive in later Cycles"; the same line begins "Not usable end to end yet." |
| `manyhands-overview/references/concepts.md` | 25 | 25 | "`document promote` in a pull-request repository still pushes main until a later Cycle of Wave 06 changes it" |

"Not usable end to end" wording (D-4's last clause): `manyhands-polling-and-index/SKILL.md:62`, `manyhands-overview/references/errors-and-exit-codes.md:42`, `manyhands-repositories/SKILL.md:154` and `:164` (both trees, same line numbers). This list matches D-4's; the grep (`Cycle 04`, `until a later Cycle`, `end to end`) finds no other sentence in either tree. `manyhands-documents/SKILL.md` has no pull-request wording today (D-4 says so), so it needs new text, not a removal.

### Step 5: no new integration test target

The design names no new target: its tests all sit in existing files (`src/repository/promotion/{record,observe,run}_tests.rs`, `src/bin/manyhands-cli/lifecycle_tests.rs`, `src/repository/context_retirement/tests.rs` or `mutation_tests.rs`, `tests/read_items.rs`, `tests/cli/lifecycle.rs`, `tests/mutation_confirmation/conformance.rs`, `tests/mutation_remote/c5_lifecycle.rs`, `tests/cli/inventory.rs`, `tests/support/*`). A grep of the design for `[[test]]`, "new test target", "build.yml" and "test list" finds nothing; every `tests/` path it cites is an existing file or a module under an existing target. `Cargo.toml` has 12 `[[test]]` entries (`ssh_fixture`, `cli_remote`, `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `mutation_remote`, `comment_publication`, `remote_polling`, `document_promotion`, `ticket_closure`, `editor_feasibility`); the design adds none, and adds no top-level `tests/<name>.rs` (39 exist now). So `.github/workflows/build.yml` does not change.

The workflow's test list is line 117 (step "Test headless credential and platform contracts"):

```
run: cargo test --locked --no-fail-fast --target ${{ matrix.target }} --lib --test shared_key_registry --test key_storage --test session_credentials --test key_material --test ssh_fixture --test ssh_transport --test remote_observation --test remote_synchronization --test remote_merge_recovery --test comment_publication --test remote_polling --test remote_reservation --test recovery_foundation_gate --test repository_enablement --test local_authoring --test discovery_rebuild --test read_boundary --test read_contract --test read_relationships --test read_repository --test read_credentials --test read_items --test read_comments --test read_status --test canonical_foundation --test mutation_relationships --test mutation_replay --test mutation_contract --test mutation_confirmation --test mutation_remote --test journal_rejection --test mutation_bridges --test document_promotion --test ticket_closure --test drafts --bin manyhands-cli --test cli --test cli_remote
```

and line 129 runs the desktop's `--features desktop-test-support --bin manyhands`. Every target the design extends (`document_promotion`, `ticket_closure`, `mutation_remote`, `mutation_confirmation`, `mutation_contract`, `read_items`, `read_contract`, `recovery_foundation_gate`, `cli`, `cli_remote`, `--lib`, `--bin manyhands-cli`) is in that list already. The AGENTS.md fast-tier command also needs no change.

## Appendix C: Task 1 design checkpoint, approved (verbatim)

### Task 1 design checkpoint (Step 0): the record's mode

Phase 1 only: nothing below is written yet. Files read: `promotion/record.rs`, `promotion/lifecycle.rs` (`LifecycleRecord` impl), `closure/record.rs` (the pattern), `record_tests.rs`, `lifecycle_tests.rs`, `tests/recovery_foundation_gate.rs`, design Parts A to C and the named edits.

### 1. Each of Part A's six columns against the RR and F rows

All six stay. None is droppable; each is read by a row that has no other source.

| Column | Read by | Why no other source serves |
| --- | --- | --- |
| `integration_mode` | Every row below applies to a pull-request record only. `path()` (Branch vs Direct) routes the runner (`run.rs` 375, 510, 654, 2239, 2293, 2326); `permits`/`validate`/CHECKs switch on it; F1 to F3 fence "a pull-request promotion record", so they must tell it from a direct one (the direct record is held by its own existing rules); C18 (a refused record "of the confirm's mode") | Cycle 01 decision 11: fixed for the record's life; the live configuration may have switched since (C18) |
| `answer_code` | RR2 (a finished record answers from it), C14, C15 (the envelope was lost: "rebuilt from `answer_code`"), both Part C tables' "Record" column (`promoted`, `already_promoted`, `primary_holds`, `remote_branch_deleted`, `history_unresolved`, `context_gone`), `answered()` (the runner's re-entry at `run.rs` 376, 484, 618) | The envelope can be lost; the link row can be gone or renewed; the tracking primary has moved. Nothing else says which of the six answers was given |
| `consented_context_oid` | RR4 ("the branch holds a commit its child did not make", before any attempt pushed: the check is the branch against the consented tip, the child's own moves being known by `child_state`); RR7 and the second table ("R-3 ... on the consented tip" after a child that ended or a branch gone); C25 | `context_oid` is unset at `confirmed` and, once set at `context_synchronized`, is the branch **after** the child (`context_step`, `run.rs` about 1176-1187), so it cannot be the pre-child baseline. The expectation is a digest and cannot be compared with a head. Cycle 03's column of the same name and immutability |
| `situation` | The first table's second row ("the record's situation is `again`": `already_promoted`, `given_at` renewed) versus third row (`promoted`, link written); C4 to C7, C1 to C3: the answer follows the situation as consented, not the link row as it is when the record finishes (a link given between consent and finish changes the row, C19 makes that an `external_change` only before the record begins) | The row can change after begin (C11 deletes it on an ended record; a re-run after a crash finds it renewed). The consented situation is the contract |
| `primary_branch` | RR2 ("without needing the live configuration", C26, C27): after a child that completed, R-3 and the link's pull-request base are judged on the primary branch the record began with, even if the configuration became unreadable or the setting changed | The live `RepositoryConfig` is the thing that may be unreadable (Baseline Fact 22: `recorded_config` needs a base, which a pull-request promotion never has) |
| `endpoint_digest` | RR5 ("the publication remote's name or its endpoint (`endpoint_digest_of`) is not what the record began with"); C20; `remote_name` already holds the name, only the digest sees a changed URL under an unchanged name | `remote_name` alone cannot see a changed endpoint. Stored as the 32-byte digest (a BLOB, as retirement stores it, `context_retirement/record.rs` 269), never a URL |

Task 4's checkpoint may still amend the rebuild on this branch (no real registry has run it).

### 2. DDL

`SCHEMA_V2` is today's `SCHEMA`, byte for byte (renamed; the `promotion_refusal_blockers_v2` step rebuilds from it). The new `SCHEMA` is `SCHEMA_V2` with these six columns appended after `refusal_blockers` and six table CHECKs appended after the existing six. `INDEX` is unchanged and still unique per `(repository_id,item_id)` for unfinished steps, of either mode. No SQL comments.

```sql
    integration_mode TEXT NOT NULL DEFAULT 'direct' CHECK(integration_mode IN ('direct','pull_request')),
    answer_code TEXT CHECK(answer_code IN ('promoted','already_promoted','primary_holds','remote_branch_deleted','history_unresolved','context_gone')),
    consented_context_oid TEXT CHECK(length(consented_context_oid)=40 AND consented_context_oid NOT GLOB '*[^0-9a-f]*' AND consented_context_oid!='0000000000000000000000000000000000000000'),
    situation TEXT CHECK(situation IN ('first','again')),
    primary_branch TEXT CHECK(primary_branch IS NULL OR (typeof(primary_branch)='text' AND length(primary_branch) BETWEEN 1 AND 255)),
    endpoint_digest BLOB CHECK(endpoint_digest IS NULL OR (typeof(endpoint_digest)='blob' AND length(endpoint_digest)=32)),
    ... the six existing CHECKs, then:
    CHECK((consented_context_oid IS NULL)=(situation IS NULL) AND (consented_context_oid IS NULL)=(primary_branch IS NULL) AND (consented_context_oid IS NULL)=(endpoint_digest IS NULL)),
    CHECK(integration_mode='pull_request' OR (consented_context_oid IS NULL AND answer_code IS NULL)),
    CHECK(integration_mode='direct' OR step='abandoned' OR consented_context_oid IS NOT NULL),
    CHECK(integration_mode='direct' OR (base_oid IS NULL AND integration_oid IS NULL AND integration_kind IS NULL AND cleanup_category IS NULL AND primary_attempt=0 AND step NOT IN ('integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted','superseded'))),
    CHECK(answer_code IS NULL OR step IN ('complete','abandoned')),
    CHECK(answer_code IS NULL OR step='complete' OR refusal_category='reconfirmation_required'),
    CHECK(step!='complete' OR integration_mode='direct' OR (answer_code IS NOT NULL AND publication_category IS NOT NULL AND publication_category IN ('published','already_current')))
```

Decisions inside it, for the approver:

- **A closed answer list, not Cycle 03's `typeof/length/GLOB`.** This table's other categories are closed lists (`integration_kind`, `publication_category`), and Task 1's test of free-text rejection reads best against one. The cost: a seventh answer needs the migration amended on the branch, which the brief already allows. Say if you want the GLOB form instead.
- **The four begin-time columns are all set or all null** (CHECK 1). A pull-request row holds all four from `confirmed` (CHECK 3); the only rows with none are refused records written at confirm (`abandoned`, no begin). A record that RR4 or RR5 end keeps the four it began with, as the design says.
- **A pull-request row never holds `primary_attempt`/`primary_sync_id`** (CHECK 4, via the existing `(primary_attempt=0)=(primary_sync_id IS NULL)`), nor `cleanup_category`. The design says it never reaches a publication child or a cleanup (R-1, "never reaches `integrating` ... `superseded`"); I made the SQL say so.
- **An answer implies the `reconfirmation_required` refusal** at `abandoned` (CHECK 6), as Cycle 03's `abandon_branch_record` stores them; a refused record has a refusal and no answer.
- The `complete` CHECK also requires the child's category in `publication_category` (`published` or `already_current`), per Part A ("at `complete` holds its context child's category").
- `refusal_blockers`'s range and the `(step='abandoned')=(refusal_category IS NOT NULL)` CHECK are unchanged and serve both modes.

### 3. The migration

Marker `wave_06_promotion_modes_v1`, run in `migrate` after `promotion_refusal_blockers_v2`:

- the `promotion_refusal_blockers_v2` step rebuilds from `SCHEMA_V2` (not `SCHEMA`); its copy stays `INSERT ... SELECT *` (the pre-v2 table has the 21 V2 columns);
- the modes step, if unmarked and not just created: `DROP INDEX document_promotions_unfinished; ALTER TABLE document_promotions RENAME TO document_promotions_before_modes; {SCHEMA}; {INDEX}; INSERT INTO document_promotions({V2_COLUMNS}) SELECT {V2_COLUMNS} FROM document_promotions_before_modes ORDER BY rowid; DROP TABLE document_promotions_before_modes;` then the marker insert. Existing rows read `direct` by the column default; the six new columns are null.
- the create path (marker `cycle_09_document_promotions` absent) writes `SCHEMA` and, as today, falls through with `created = true`, so neither rebuild runs; both later markers are inserted.
- `validate_schema` compares the stored DDL to `SCHEMA` exactly.

`V2_COLUMNS` (21, in `SCHEMA_V2` order): `repository_id,operation_id,item_id,expectation,remote_name,step,context_attempt,context_sync_id,primary_attempt,primary_sync_id,base_oid,context_oid,integration_oid,integration_kind,remote_context_oid,publication_category,cleanup_category,cancel_requested,updated_at,refusal_category,refusal_blockers`.

New table order after `refusal_blockers`: `integration_mode, answer_code, consented_context_oid, situation, primary_branch, endpoint_digest` (this is the order the privacy inventory test lists).

### 4. `PromotionStep::permits` and `validate`, per mode

`permits(self, next, remote, mode: Integration)`; its one caller is `save_record`, which passes `old.integration_mode`.

- `Direct`: exactly today's arms. No change.
- `PullRequest`: not terminal, and `self == next` or `(Confirmed, ContextSynchronized)` or `(ContextSynchronized, Complete)` or `(Confirmed, Abandoned)`. Everything else is refused, so no integration, publication or cleanup step. `ContextSynchronized → Abandoned` is **not** permitted (design: "`confirmed` → `abandoned`" only; RR2 finishes a child that completed). Cycle 03's graph does permit it; if you want it for parity, it is one arm, say so. `ContextSynchronized → Complete` is the finish function's guard (Task 3), as `abandon_branch_record` calls `branch_permits`; `save_record` itself can never reach `complete` for a pull-request row, because `answer_code` is immutable in it and `complete` needs one (as a direct row's `complete` needs the retirement transaction, `record_forward_transitions...` pins that).

`validate` is split. The common part (ids, attempts, refusal presence, zero OIDs, `context_oid` required from `context_synchronized` on, `remote_context_oid`/`remote_deleted` need a remote) stays inline as today. The four clauses that read the integration columns (base and kind from `integrating`, integration OID from `integrated`, the publication category at `published` onward, `publication_pending`) move to `validate_direct`, **unchanged in behaviour** and extended to demand the six new columns null (mode direct, no answer, no begin-time column). They cannot stay common: a pull-request `complete` has no base or integration. `validate_pull_request` demands:

- no `base_oid`, `integration_oid`, `integration_kind`, `cleanup`, `primary_sync_id`; no step among the nine excluded ones;
- the four begin-time columns all set or all none; all set at every step but `abandoned`; `primary_branch` set implies a remote name and passes `RemoteRefPlan::from_configuration(remote, primary_branch)`; a remote name present on every non-abandoned row (a pull-request record always has the publication remote);
- `complete`: `context_oid`, `publication` in {`published`, `already_current`}, `answer_code` in {`promoted`, `already_promoted`, `primary_holds`}, no refusal;
- `abandoned`: `publication` none; `answer_code` none (a refused record, any refusal) or in {`primary_holds`, `remote_branch_deleted`, `history_unresolved`, `context_gone`} **with** the refusal `reconfirmation_required`;
- `confirmed`, `context_synchronized`: no answer, no publication.

(The answer-set-per-step and the nones for `publication` are Rust-only, beyond the SQL above, and come straight from the two tables of Part C.)

### 5. `save_record`

The immutability list gains, as flat equality (not "once set": the begin-time columns are written at insert only, and a null stays null): `old.integration_mode != r.integration_mode`, `old.answer_code != r.answer_code`, and the four `consented_context_oid`, `situation`, `primary_branch`, `endpoint_digest`. All answer `recovery_required`, as the neighbouring rules do. The existing `context_oid` rule stands: unset at `confirmed`, set once at `context_synchronized`, then immutable. `answer_code` is written only by Task 3's finish function (like Cycle 03's `complete_branch_record`). The UPDATE statement writes none of the six.

### 6. `PromotionAnswer` and the other new types (in `promotion/record.rs`, built with `categories!` as the neighbouring enums)

- `PromotionAnswer { Promoted=>"promoted", AlreadyPromoted=>"already_promoted", PrimaryHolds=>"primary_holds", RemoteBranchDeleted=>"remote_branch_deleted", HistoryUnresolved=>"history_unresolved", ContextGone=>"context_gone" }`, with `name()`/`parse()` from the macro (any other text reads as recovery).
- `PromotionSituation { First=>"first", Again=>"again" }`: the brief gives `situation` no type; I need one now so that decode and the CHECK agree. Task 2's observation may reuse it.
- `PromotionRecord` gains `integration_mode: Integration`, `answer_code: Option<PromotionAnswer>`, `consented_context_oid: Option<Oid>`, `situation: Option<PromotionSituation>`, `primary_branch: Option<String>`, `endpoint_digest: Option<[u8; 32]>`. `decode` reads them (a private `parse_mode` of seven lines, copied rather than loosening Cycle 03's private one: no edit to `closure/record.rs`; the BLOB through `row.0.get::<_, Option<Vec<u8>>>` and `try_into`). `Debug` shows the mode, the answer and the situation, never the digest.

### 7. `path()` and `answered()`

Inherent methods on `PromotionRecord` as on `ClosureRecord`: `path()` is `RecordPath::Branch` for `PullRequest`, `Direct` for `Direct`; `answered()` is `answer_code.is_some()`. The `LifecycleRecord` impl for `PromotionRecord` in `lifecycle.rs` forwards both. No other change in `lifecycle.rs`. The `unreachable!("a promotion record is always direct")` branch hooks stay; no pull-request row can exist in a production run before Task 3 (nothing inserts one), and Task 3 replaces them.

### 8. How the mode and the begin-time columns reach `begin_record`: the signature and dead-code question

**Choice: nothing reaches `begin_record` in Task 1. Its signature and all about 32 callers stay as they are; Task 1 adds no begin-time function; the pull-request rows are created in the tests by a test-only insert helper in `record_tests.rs`.**

Why:
- A new `begin_pull_request_record` (or a mode argument on `begin_record`) has no production caller until Task 3 (nothing begins a pull-request promotion before it). The only way to compile it today is under this file's blanket `#![allow(dead_code)]`, which is exactly the "function with no production caller yet" the plan forbids. A mode argument that production passes as `Direct` would change 32 callers (each an E-18 onwards entry), for a variant production never builds.
- What Task 1 owns is the schema, `validate`, `permits`, `save_record`, `decode`, `PromotionAnswer`, `path`/`answered`. All of it is exercised in production by direct rows (`decode`, `validate`, `save_record`, the migration, `path()`, `answered()` are called by the runner for every record) and tested for pull-request rows by inserting a row with SQL and reading it back through `read_record` (so `decode` and `validate` run) and saving through `save_record`.
- `begin_pull_request_record` and `refuse_record`'s mode arrive in Task 3 with their caller, where Task 3 also tests that they write the four columns (a begin-time test row belongs there).

Cost: Task 1 does not test that begin writes the four columns; Task 3's tests must. If you prefer the begin twin now behind the file's existing allow, say so; I recommend against.

### 9. Tests (all in `src/repository/promotion/record_tests.rs`, library layer, in-memory registry, no SSH)

New, names exact (the brief's):
1. `promotion_modes_migration_rebuilds_a_v2_table_once`. Builds a table with `SCHEMA_V2` + `INDEX` and the old markers from rows made through `begin_record`/`save_record` at several steps (confirmed; context_synchronized; integrated; a refused `abandoned`; a `complete` through `complete_record` if cheap, else `publication_pending`) with distinct items, copied by `V2_COLUMNS`, the modes marker deleted. Migrates; asserts every row reads back equal to its pre-migration record with `integration_mode == Direct` and the six new fields none, rowid order kept, the stored DDL equals `SCHEMA`, the marker count is 1; a second `migrate` changes nothing (`total_changes`); a fresh registry has `SCHEMA`, `INDEX` and the three markers (`cycle_09_document_promotions`, `promotion_refusal_blockers_v2`, `wave_06_promotion_modes_v1`).
2. `pull_request_promotion_steps_and_checks`. One table of cases `(label, step, mutate columns, rust_ok, sql_ok)`: a pull-request base row per step, inserted by the helper (an `insert_row(&PromotionRecord)` that writes all 27 columns), asserting the SQL accepts or refuses, and `validate` plus `decode` accept or refuse, for: the good rows at `confirmed`, `context_synchronized`, `complete` (`published` and `already_current`; each of the three answers), `abandoned` with `reconfirmation_required` and each of the four ended answers, `abandoned` as a refused record (each of `blocked`, `external_change`, `reconfirmation_required`; no answer; four columns null, and four columns set as RR4/RR5 leave them); the bad ones: any integration, cleanup, base, primary-attempt column; each excluded step; a begin-time column missing at `confirmed`/`context_synchronized`/`complete`; one of four set; answer at `confirmed`/`context_synchronized`; answer missing or `remote_branch_deleted` at `complete`; answer at `abandoned` with `blocked`/`external_change`; `primary_holds` at `abandoned` with no refusal; a direct row with any of the six; a bad `primary_branch`; a `complete` row with a missing/other publication category. Also `permits`: all 13 x 13 step pairs in pull-request mode against the expected set, and a few direct pairs unchanged. Also `path()` and `answered()` for both modes. Also `save_record`: from a `confirmed` pull-request row, each of the four begin-time columns, the mode and the answer changed refuses (`recovery_required`), `context_oid` set once at `context_synchronized` succeeds and then refuses a change; every non-listed step move refuses.

One test per brief, a table inside each; the rows of the second carry the "pins unchanged" and "new" distinction in their labels.

Extension of existing tests (E-4, additive only):
- `privacy_column_inventory_for_both_new_tables`: the allowed `document_promotions` list gains the six names in the order above. Required: it fails otherwise.
- `schema_rejects_free_text_categories_steps_and_refusal_payloads`: its assignment list gains `integration_mode='body sentinel'`, `answer_code='path sentinel'`, `situation='title sentinel'`, `endpoint_digest=x'01'`, `primary_branch=''`. Rows added, none changed.
- `promotion_refusal_blockers_v2_rebuilds_an_old_bound_table_once`: rewritten to build the old table from `SCHEMA_V2.replace("131071","65535")` with `INSERT INTO document_promotions({V2_COLUMNS}) SELECT {V2_COLUMNS} FROM before`, deleting the two markers `promotion_refusal_blockers_v2` and `wave_06_promotion_modes_v1`, as `closure/record_tests.rs` (about 1782) does. Its assertions are unchanged, including the marker-count one (the modes marker is re-inserted by the same migration). Required: the current `SELECT *` into the 21-column V2 would fail.
- `migration_preserves_legacy_rows_is_idempotent_and_audited`, `half_created_migration_and_missing_marked_table_fail_closed`, `record_forward_transitions_and_invalid_backwards_recover`: named in E-4 but I expect **no edit**: they do not enumerate columns (the first drops the table and one marker and re-creates with `SCHEMA`; the second drops the table; the third saves direct rows). I will run each by exact name in Phase 2 and edit only one that fails, and report it. The mode's rows live in my two new tests.

Existing tests changed, beyond those:
- **E-3** `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` (about 1135-1143): the marker list gains `"wave_06_promotion_modes_v1"` after `"wave_06_closure_modes_v1"` (sorted `ORDER BY name`).
- **E-15** `promotion/lifecycle_tests.rs:90` `promotion_steps_map_to_the_shared_step_view`: the `PromotionRecord { .. }` literal gains `integration_mode: Integration::Direct` and `answer_code`, `consented_context_oid`, `situation`, `primary_branch`, `endpoint_digest` all `None`. The 13 steps and the shared view are untouched. It is the only literal of the struct in `src/` and `tests/` (grep).
- No caller of `begin_record` or `refuse_record` changes, so there is no E-18 onwards.

### 10. Verification plan (Phase 2)

Red: tests written first against the unchanged record (a compile failure on the new fields and `SCHEMA_V2`/`V2_COLUMNS`/`PromotionAnswer`, recorded). Then, since a compile failure shows no row biting, after implementing, one deliberate-stub run: with the pull-request arm of `permits` stubbed to the direct graph and CHECK 4 removed, the second test fails on named rows (recorded), then the stub is reverted. Green: each new test and each edited test by exact name; `recovery_foundation_gate` E-3 test by exact name; then check, fmt, clippy (`-D warnings`). No whole module, no whole target, no CLI.

### 11. Points for the approver

1. Closed answer list in SQL (vs the GLOB form). Recommended: closed.
2. `ContextSynchronized → Abandoned` refused for pull-request rows (design-faithful); parity with Cycle 03 would permit it.
3. The extra constraints beyond the design's words: no primary child and no cleanup category; `publication` only at `complete`; the answer set per step. Each follows a Part C table; Task 4 can relax one.
4. No begin-time function in Task 1 (section 8).
5. `PromotionSituation` is defined here.

## Appendix D: Task 2 design checkpoint, approved with a correction (verbatim; the correction is in Task 2's section)

### Task 2 design checkpoint (Step 1): the pull-request observation

Phase 1 only: nothing below is written yet. HEAD 034f4346. Read: `promotion/observe.rs`, `promotion/mod.rs`, `promotion/lifecycle.rs`, `promotion/run.rs` (the begin and revalidation path, 480-640), `promotion/record.rs` (Task 1's mode and answers), `bind/lifecycle_verb.rs`, `bind/lifecycle.rs` (digest, hold, prepare, preview, observe_again), `closure/observe.rs` (the pull-request branch and the withdraw's `unoffered`), `closure/checkpoint.rs` `branch_link`, `context_retirement/proof.rs`, `remote/poll.rs` `completed_deleted_context`, `pull_request_links.rs`, `canonical.rs` (a pull-request configuration always has a remote), the design's Baseline Facts, Part B, Part C step 1, Part G, P2/P3, R-2 to R-7, R-18, R-24, R-29, E-12.

### 1. `primary_holds_document` (R-3)

In `promotion/observe.rs`, beside `ancestor` and `reference_oid`:

```rust
pub(in super::super) fn primary_holds_document(
    repository: &git2::Repository,
    tip: Oid,                      // L, the branch tip to judge
    tracking_primary: Option<Oid>, // given: after a child, the caller passes the tip its fetch moved it to
    primary_branch: &str,
    remote: &str,                  // the publication remote's name
    item: &ItemId,
    remote_gone: bool,             // read R as absent (after remote_branch_deleted: the fetch does not prune it)
) -> bool
```

How it reads, with no fetch, lease or write (refs and objects only; every error reads not held):

- `R`: unless `remote_gone`, `reference_oid(refs/remotes/<remote>/manyhands/document/<item>)` through `RemoteRefPlan::from_configuration(remote, primary_branch)` (`plan.context(Document, item).tracking_ref()`, the ref `remote_facts` reads). `R` is acceptable when absent (or `remote_gone`), equal to `tip`, or an ancestor of `tip` (`graph_descendant_of(tip, r)`). `R` ahead of or beside `tip` returns false before any proof.
- The tracking primary is the argument; `None` returns false (a document is never held against no remote primary).
- The local primary is `refs/heads/<primary_branch>`, read here (peeled to its commit). It is a candidate only when it equals the tracking primary or is an ancestor of it; ahead or beside is never used (in this mode Manyhands never commits there).
- The proof: `context_retirement::proof::preservation(repository, tip, candidate, AuthoringKind::Document, item).is_ok()`, tried against the tracking primary, then against the local candidate when it differs. Held when either passes. `Ok(Ancestor | Folded)` is held, every `ProofRefusal` (`InvalidPrimaryItem`, `TicketOpen`, `MissingObject`, `ShallowHistory`, `UnrelatedHistory`, `MultipleMergeBases`, `ContentMismatch`) is `Err`, so not held. `proof.rs` is not touched. The second candidate matters: a tracking primary that later deleted the document fails `completed_item`, while the merge still stands on the local primary behind it.

### 2. Part G items, against the source

**Item 1 (`path()`, `answered()`, `begin_record` takes the mode from the observation).** `path()` and `answered()` are Task 1's, done. The mode reaches `begin_record` only through the preview, as the closure's reaches it through `ClosureObservation.path` (`closure/lifecycle.rs` `begin_closure_record` matches `observed.map(|o| o.path)`): `Lifecycle::begin_record(tx, id, op, item, expectation, remote, observed: Option<&Preview>)` already receives `Option<&DocumentPromotionPreview>` (today `_observed`), and `refuse_record` receives `config: &RepositoryConfig` and `observed` (so a refused record takes `config.integration()`). Task 3 (not built now) matches `observed.and_then(|p| p.pull_request())`: `Some(facts)` gives `integration_mode = pull_request`, the situation from `facts`, the consented tip from `p.context_oid()`, `primary_branch` from `p.primary_branch()`, and the endpoint digest. The endpoint digest (`endpoint_digest_of(repository, remote)`) is not carried by this task's observation: Task 2 has no reader for it, and a field with no production reader would need an `allow(dead_code)`. Task 3 adds it to the facts (a pure read of the remote's configured endpoints, so it costs nothing to compute at observation).

**Item 5 (the digest's domain constant beside `manyhands promotion v1`).** `const PULL_REQUEST_PROMOTION_DIGEST_DOMAIN: &[u8] = b"manyhands pull request promotion v1";` goes immediately above `expectation()` in `observe.rs`. Direct's literal stays as it is (the t2_1 wire-vector test builds it independently).

**Item 10 (`runs_context_child` true, `require_identity` passing in pull-request mode).** `impl LifecyclePreview for DocumentPromotionPreview` (`lifecycle.rs:842`) overrides `runs_context_child` as `self.pull_request.is_some() || default`. `DocumentPromotionLifecycle::require_identity` (`lifecycle.rs:~1008`) returns `Ok(())` first when `preview.pull_request().is_some()` (the child's own identity stop answers as `item sync`'s does). `Promote::observe`'s `identity_required` guard (`lifecycle_verb.rs:419`) gains `&& preview.pull_request().is_none()`. A pull-request preview also plans `integration = AlreadyContained` (nothing integrates), so even the unguarded forms would pass; the explicit checks are the contract and the plan value is the belt.

**Item 12 (the shape: `Ready`, `Observed`, the preview).** Chosen shape, the one that changes the fewest exhaustive matches:

- `DocumentPromotionPreview` (`promotion/mod.rs`) gains a private field `pull_request: Option<PullRequestPromotion>` and a `pub(in super::super)` accessor `pull_request()`. `PullRequestPromotion` is a new `pub(in super::super)` struct `{ situation: PromotionSituation, link: PullRequestLink, unmerged_remote: Option<Oid> }` (`unmerged_remote` is `R` only when `R` is ahead of or beside `L`, which is how the digest binds it). `L` is the preview's `context_oid`, `R` is its `tracking_context`, the remote is `publication_remote`. Every field is read by the digest, so no `allow(dead_code)`. A pull-request preview plans: `integration = AlreadyContained`, `publication = Publish`, an empty `changes`, `cleanup` all false, and `tracking_primary` set (a missing one is the `primary_missing` blocker).
- `primary_oid`: the field and the accessor `primary_oid() -> Oid` keep their type. Making it `Option` would break `observe_tests.rs:82` and `:899` (`assert_eq!(p.primary_oid(), f.base)`), which are existing tests beyond E-12. In a pull-request preview it is documented and set as the tracking primary's tip (the primary the observation judged against), and the local primary is only an input to `primary_holds_document`. Nothing in the digest or the pull-request preview DTO (Task 5) reads it.
- `DocumentPromotionPreflight` gains ONE variant, the unit variant `PrimaryHolds`. A ready pull-request observation is `Ready(preview)` with `pull_request` set, so no second variant and no third arm.
- The binding's `Ready` (`lifecycle_verb.rs:77`) gains `pull_request: Option<PullRequestPromotion>` beside `record`, `Observed` gains `PrimaryHoldsDocument` (the twin of `NothingToWithdraw`), and `Ready.primary_oid` keeps its type.

**Item 15 (the held shape's own digest).** In `digest_of` (`bind/lifecycle.rs:422`): `Observed::PrimaryHoldsDocument => Observation::of(name).text("unoffered", "primary_holds_document")`, as `unoffered`/`nothing_to_withdraw` is built (the name and the reason text differ from every other shape; `document promote`'s own name is in the digest, so it cannot equal a ready digest, which is `expectation`). At confirm the CLI boundary (`execute.rs:307-311`) re-observes through `Binding::prepare`, gets this digest, finds it differs from the consent's, and answers `external_change` with nothing recorded. `Binding::prepare` (`bind/lifecycle.rs:1495-1500`) keeps no expectation for it (`self.expected = None`, in the `Blocked | NothingToWithdraw | PublicationRemoteRequired` arm). `observe_again` (`:1706`) treats it as `NothingToWithdraw` is treated: `expected = None`, `Seen::State { found, own: None }`. In the library, `Lifecycle::ready` maps `PrimaryHolds` to `Err(Refusal::ExternalChange)`, so the runner's revalidation refuses with a refused record (C19; Task 3 gives that row the confirm's mode).

### 3. The prepare's `already_applied` before any record

`Promote::observe` maps `DocumentPromotionPreflight::PrimaryHolds` to `Observed::PrimaryHoldsDocument`. `Confirmed::preview` (`bind/lifecycle.rs:1545`) answers it exactly as it answers `NothingToWithdraw`: `Err(Answer::stopped(ResultCode::AlreadyApplied))`, which is class `None`, outcome `noop`, no data, `recovery` `[]`, with no record written (the answer is the prepare's; `begin_record` is only reached by a confirm). The same arm list in `stop_blockers` (`:1251`) adds the variant. Order: `PublicationRemoteRequired`, then the hold (`held()`, which is the unfinished promotion record's `recovery_required` naming its request, kept as is), then the observed shape, so an unfinished record of the document still wins over `already_applied` (ruling 35's order). The library puts the held test after the blockers (Part B's order): a blocked state (dirty context, `history_unresolved`, no tracking primary) answers blocked and never `already_applied`.

### 4. The observation (Part B), in order

`observe_git` (private, `observe.rs:~948`) dispatches on `config.integration()`, which `open_observation` already reads on every prepare and revalidate (so `preview_document_promotion` in `mod.rs` needs no change; the brief's file list over-states it). Direct goes through today's function unchanged. The code both modes share (context presence, cleanliness, the document path, `pending`) moves verbatim into one helper; the direct tests pin it.

`read_view` gains the registry's link row, read in the same read-only transaction, for a pull-request configuration only: `link_of(tx, id, "document", &item)`. `snapshot, pending, link` travel as one `StoredView` so `observe_git` stays within clippy's argument limit.

The pull-request observation decides in this order (Part B):

1. Step 1, `not_document` or context missing: `NotADocument`, `ContextMissing`, as today.
2. The blockers kept (a `BTreeSet`, so the order is the enum's): `ContextNonconforming`, `ContextDirty`, `ContextConflicted` (all from `context_presence`/`facts.document_path`), `ConflictPending` (the `pending` flag), `HistoryUnresolved` (the history of an absent tracking context is `Unresolved`), `PrimaryMissing` (`remote_facts` finds no tracking primary; the local primary is only read as an optional ref), `PrimaryConflicted` (`repository.state() != Clean` at the root, as `closure/observe.rs:1037-1041`). The unfinished promotion record's `recovery_required` is not in the library: it stays the binding's `held()`. Any blocker returns `Blocked`.
3. The primary holds the document: `primary_holds_document(L, tracking_primary, primary_branch, remote, item, false)`. True returns `PrimaryHolds`.
4. The tracking context is absent and the history is `Deleted`: `Blocked([RemoteContextDeleted])`. This is after step 3 because a merged and deleted branch is the usual case that must answer `already_applied`.
5. A link row exists: situation `Again`, link the row's.
6. Otherwise `First`, link built now by a document twin of `branch_link` (`document_link(repo, primary, config.forge(), remote, item)`: `forge::recognise` and `forge::pull_request_link` with `LifecycleKind::DocumentPromotion.context_branch(item)`; `branch_link` hard-codes the ticket kind, and Task 3's item 9 may merge the two).

`HistoryUnknown` never blocks (R-5): `Unknown` and `NeverPublished` are both no blocker. `unmerged_remote` is `R` when `R` is neither equal to nor an ancestor of `L`.

Dropped (nothing is computed, so nothing can block): `PrimaryNotCheckedOut`, `PrimaryDirty` (`primary_presence` is not called), `PrimaryAheadOfRemote`, `ContextBehindRemote` (the two flags of `remote_facts` are ignored), `IntegrationConflict`, `DocumentInvalidAfterIntegration` (no `planned_result`), `HistoryUnknown`, and `identity_required` (item 10).

The digest (`pull_request_expectation`, `observe.rs`, dispatched from `expectation()` when `pull_request` is set): `blake3(domain ‖ field(item) ‖ field(document_path) ‖ field(context_branch) ‖ field(primary_branch) ‖ remote(Some) ‖ L(20 bytes) ‖ optional(unmerged_remote) ‖ situation tag (first 0, again 1) ‖ link: field(source), field(target), forge (tag 0, or 1 then field(kind spelling)), url (tag 0, or 1 then field))`, with the existing `digest::field`/`remote` encodings. Not bound: the tracking or local primary, `primary_oid`, `R` when absent, equal or behind. So a poll that first creates `R` at this clone's own push changes nothing.

### 5. The preview's effects and details (bind/lifecycle.rs)

`Confirmed::preview` returns early for `ready.pull_request.is_some()` with `effects = [ContextSynchronized, ContextPublished, PullRequestLink]`, always (R-24). `details` until Task 5 is the existing `PreviewDetailsDto::Lifecycle` the direct path builds from `ready`, whose fields for a pull-request ready are already the right empty ones (no changes, `already_contained`, `publish`, cleanup false, `primary_oid` the tracking primary's). Task 5 replaces it with `PullRequestPromotionPreviewDto`. `note_observed` (`:628`) recognises `ready.pull_request.is_some()` as it does `ready.record` (the preview's scope then names the document's branch), per item 4's prepare scope.

### 6. Interim confirm behaviour (Tasks 2 to 3)

After this commit a pull-request prepare answers; its confirm reaches the library with a ready pull-request preview. Task 3 owns the runner, so the interim guard is in `DocumentPromotionLifecycle::begin_record` (`lifecycle.rs`, the choke point that creates a record): when `observed` has `pull_request()` it returns `Err(state::recovery_required())` before `record::begin_record`. The call is a transaction, so nothing is written; the runner maps it to `DocumentPromotionError::RecoveryRequired`, the binding to `Stop::RecoveryRequired`. So a confirm in a pull-request repository answers `recovery_required` with no record, no child, no lease taken for an integration, and the primary never moved. It never reaches the runner's `unreachable!("a promotion record is always direct")` (that hook is called only for a branch record, and no promotion record is a branch record yet), and never runs a direct promotion. `refuse_record` is left alone: a confirm that finds a changed consent still writes a refused record (an `abandoned` row, no effect), which is harmless; Task 3 gives it the confirm's mode. Task 3 deletes the guard when it adds the pull-request `begin_record`. At the CLI the boundary answers `external_change` first when the consent differs. Test (run_tests.rs, one new test): `a_pull_request_confirm_stops_before_any_direct_record` (section 8).

### 7. E-12, E-16, E-17 and the matches changed

`DocumentPromotionPreflight` matches changed (each gains the `PrimaryHolds` arm):

1. `promotion/lifecycle.rs` `ready` (`:1049`): `PrimaryHolds => Err(Refusal::ExternalChange)`.
2. `bind/lifecycle_verb.rs` `Promote::observe` (`:417`): `PrimaryHolds => Observed::PrimaryHoldsDocument`.
3. **E-12** `tests/document_promotion.rs:301` `preview()`: `DocumentPromotionPreflight::PrimaryHolds => Err(FixtureError)`, beside `Blocked` (a direct fixture never holds; same failure as a blocked preview). `document_promotion` is an SSH target, so after the edit I rely on compile through clippy and do not run it, except by exact name if a case needs it.

Every other `DocumentPromotionPreflight` use in tests (`let ... else`, `Ready(p) => p, other => panic!`, `matches!`, the `if let (Ready, Ready)`) is not exhaustive and compiles unchanged: `integrate_tests.rs:185`, `observe_tests.rs:404/411`, `run_tests.rs:233/693/815`, `tests/drafts/targets.rs:501`, `tests/cli_remote/*`, `tests/mutation_remote/c5_lifecycle.rs`, `tests/document_promotion/journeys.rs:676`.

`Observed` matches changed (the binding's own, each gains `PrimaryHoldsDocument` next to `NothingToWithdraw`): `digest_of` (`:422`), `stop_blockers` (`:1251`), `Binding::prepare` (`:1496`), `Confirmed::preview` (`:1545`), `observe_again` (`:1706`).

`Ready` changes (`pull_request: Option<PullRequestPromotion>` is added), so **E-16** `bind/lifecycle_tests.rs:1117` (`a_branch_record_s_answers_name_the_ticket_s_branch_and_a_direct_one_s_name_primary`) and **E-17** `:1260` (`a_branch_record_previews_its_own_effects_and_details`) each gain `pull_request: None,` and nothing else. `primary_oid` keeps its type, so neither literal changes otherwise. The alternative (a `Ready`-less shape with a second `Preflight` variant and a new `Observed` ready variant) adds more arms to the same matches and leaves E-16 and E-17 in place anyway.

### 8. Tests (exact names, rows)

All in `src/repository/promotion/observe_tests.rs` unless noted. New fixture helper `pull_request()` writes `PULL_REQUEST_CONFIG` to the root's `.manyhands/config.toml`, as `closure/observe_tests.rs:203`. To run each red by assertion, not by compile error, I first add stubs (the `PrimaryHolds` variant, `primary_holds_document` returning false, the dispatch falling through to direct), commit nothing, run red, then implement.

**`primary_holds_document_by_the_content_proof`** (P3, A6). Plain `git2` repository in a tempdir (no service). A table of `(document, merge, scenario, R)`:
- document: a new one (absent from the primary before the merge) and an edit of a merged one.
- merge: before the merge (none), a merge commit, a squash, a fast-forward.
- scenario (where the merge sits, all expectations written as literals): (1) on the tracking primary, local primary at the pre-merge base (behind): held; (2) on both, local equal to tracking: held; (3) on tracking, local ahead of it by a commit: held; (4) on tracking, local beside it: held; (5) on a local primary ahead of the tracking primary, tracking still at the base: NOT held; (6) on local, tracking beside it: NOT held; (7) on local, tracking a later commit ahead of it that keeps the document: held; (8) as 7 but the later tracking commit deletes the document: held (by the local candidate); (9) no tracking primary: NOT held.
- `R`: absent, equal to `L`, behind `L`, ahead of `L`, beside `L`, and present but read with `remote_gone` true. `R` ahead or beside: not held in every scenario; the other four: the scenario's result.
- The merge "before" is never held in any cell. An edit before its merge is not held although the document is on the primary with older content.
- Plus: a shallow repository (`.git/shallow` naming a commit) is not held; two merge bases (a criss-cross: M1 = merge(B1, C1), M2 = merge(C1, B1)) is not held; a tip unrelated to the primary is not held.
Expected values come from per-scenario and per-`R` literal flags, not from a re-statement of the algorithm.

**`pull_request_promotion_observation`** (P2, A7). `Fixture::new(true)` plus `pull_request()`, sequential rows on one fixture where the order allows, a fresh one where not:
- dispatch: the same state in a direct repository is `Ready` with `pull_request()` none, `MergeCommit` and the changes; in a pull-request repository `Ready` with `pull_request()` some, `AlreadyContained`, no changes, no cleanup, and a different digest.
- Part B's order: not a document -> `[NotADocument]`; no context -> `[ContextMissing]`; each blocker kept alone: nonconforming, dirty, conflicted, `conflict_pending` (the fixture's `pending_conflict`), `history_unresolved` (evidence `history_unknown`, no tracking context), `primary_missing` (the tracking primary deleted), the root in a merge (`MERGE_HEAD`) -> `primary_conflicted`; a blocker with the document held answers `Blocked`, never `PrimaryHolds`; held (merged document on the tracking primary) -> `PrimaryHolds`; held with a deleted-branch history -> `PrimaryHolds`; not held with the deleted-branch history and no tracking context -> `[RemoteContextDeleted]`; `history_unknown` evidence with the branch published and no tracking context -> `Ready` (not blocking); no link row -> `First` with the link built from the configured remote; a link row -> `Again` with the row's link.
- dropped, each leaving `Ready`: the root dirty (a tracked file modified), the root on another branch, the local primary ahead of its remote (a commit on `main`), the local primary absent, an unreadable identity (the preview plans `AlreadyContained`, and `LifecycleKind`'s `require_identity` returns `Ok` for it).
- the effects: a binding test, since effects are the binding's (`bind/lifecycle_tests.rs`, new `a_pull_request_promotion_previews_its_three_effects_always`: `Confirmed::preview` effects for a `Ready` with `pull_request` some, for `Again` and `First`, equal `[ContextSynchronized, ContextPublished, PullRequestLink]`; and `Observed::PrimaryHoldsDocument` answers `already_applied` with no data; its digest differs from a ready digest and from `NothingToWithdraw`'s). That test is the only addition outside `observe_tests.rs`, and it is why `bind/lifecycle_tests.rs` is touched (E-16 and E-17 are in the same file).

**`pull_request_promotion_digest_binds_each_field`**, beside `digest_binds_each_field_change_blob_and_cleanup_member` (unchanged): from one pull-request `Ready`, clones with the item, the path, the branch, the primary branch, the remote (changed, and none), `L`, `unmerged_remote` set, the situation and the link (source, target, forge, url each) each give `pull_request_expectation` different from the original; `R` absent, equal to `L` and behind `L` (the fixture's tracking context moved to each) give the same digest; moving the tracking primary and the local primary give the same digest; `R` ahead of `L` and beside `L` give two other digests; the digest differs from the direct digest of the same state.

**`a_pull_request_confirm_stops_before_any_direct_record`** (interim guard, `promotion/run_tests.rs`): `Fixture::new()`, `remote()`, a pull-request configuration, a request from the preview, `run_io` with `FakeIo`: `Err(DocumentPromotionError::RecoveryRequired)`, no `document_promotions` row, no child recorded, the local primary unmoved. Red against a stub with the guard removed (the run then begins a direct record and calls the child).

### 9. Existing tests I expect to edit

Only E-12 (`tests/document_promotion.rs:301`), E-16 (`bind/lifecycle_tests.rs:1117`) and E-17 (`:1260`). No existing assertion changes. The compile-checked non-exhaustive uses above need nothing. If the compile finds another exhaustive match, I stop and report it.

### 10. Concerns for the approver

- The interim preview `details` for a pull-request promote is the direct `Lifecycle` DTO (empty changes, `already_contained`, cleanup false, the tracking primary as `primary_oid`) until Task 5; correct enough for the guarded interval, a wrong shape for a reader.
- `primary_oid` for a pull-request preview is the tracking primary's, not "optional or absent" as Part G item 12 words it; I kept the type to avoid editing `observe_tests.rs:82/899`. Say if you prefer `Option` and the two edits.
- The endpoint digest is Task 3's (no reader here).
- The document twin of `branch_link` lives in `observe.rs`; Task 3's item 9 can fold it into `branch_link(kind)`.

## Appendix E: Task 3 design checkpoint, approved with a correction (verbatim; the correction and rulings are in Task 3's section)

### Task 3 design checkpoint (Step 2): confirm of a pull-request promote, the two answer tables

Written against the source at HEAD `6c46b35c` (Task 3 base). No code or test has been written.

Step 1: `git fetch origin` ran; `git merge-base --is-ancestor origin/main HEAD` holds (`origin/main` is `8e5d9832`, an ancestor of HEAD `6c46b35c`), so the merge is a no-op.

Two things before the detail. The task is large because the runner, the record, the outcome types and the binding all change, but nothing in it needs a new mechanism: Cycle 03's branch path (`promote_branch`, `context_step`, `ends_context_child`, `let_go`, `refresh`) already runs a promotion record up to the two hooks that are stubs today (`branch_step`, `end_branch`). The departures from the brief are listed in section 16; the decisions that need the approver are marked **(approve)** where they arise.

### 1. What changes, by file

| File | Change |
| --- | --- |
| `promotion/record.rs` | the answered-`abandoned` invariant (SQL CHECK and `validate_pull_request`); `PullRequestBegin`, `begin_pull_request_record`; `refuse_record_on(..., mode)` (`refuse_record` keeps its signature and calls it with `Direct`); `BranchFinish`, `LinkWrite`, `complete_branch_record`, `abandon_branch_record` |
| `promotion/answer.rs` (new) | the two answer tables: `finish` (first table, called by `branch_step`) and `end` (second table, called by `end_branch`) |
| `promotion/lifecycle.rs` | the interim guard removed; `begin_record` dispatches on the observed preview; `refuse_record` passes the confirm's mode; `type Outcome = PromotionOutcome`; `branch_step`, `end_branch`, `branch_outcome`, `outcome`, `complete`, `index_pending` implemented; `begin_publication`, `branch_answer`, `complete_branch` stay as they are (`BranchFinish = Infallible`) |
| `promotion/mod.rs` | `BranchPromotionOutcome`, `PromotionOutcome`, `DocumentPromotionResult::Branch` and `::BranchIndexPending`; `mod answer` |
| `promotion/run.rs` | `RunEvent::Recording` only for a ticket closure (line 980) |
| `pull_request_links.rs` | `touch_link`: updates `given_at` alone (item 9) |
| `mutation/bind/lifecycle_verb.rs` | `Called::Branch` holds `Branched::{Closure, Promotion}`; `Promote::call` builds it; `called_of` wraps the closure's; `promotion_branch_code` |
| `mutation/bind/lifecycle.rs` | `call` matches the two; `promotion_branch_reached` (code, effects; no data until Task 5) |

Not touched: `closure/record.rs` and `closure/checkpoint.rs` (see item 9), `lifecycle_record.rs`, `read/*`, the CLI and the desktop (no match over the changed types exists there; I grepped `DocumentPromotionResult` over `src/` and `tests/`).

### 2. Part G item 2: `branch_step` and `end_branch`

`DocumentPromotionLifecycle::branch_step(&self, service, request, r)` calls `answer::finish(service, request, r)` and returns `BranchStep::Finished`; the runner then reloads the record and calls `refresh` (`promote_branch`, `run.rs:983-989`), which marks the item's context dirty, indexes it and builds the outcome. `BranchStep::Closing` is never returned, so `finish_branch` is never reached for a promotion (R-1), and `begin_publication`, `branch_answer` and `complete_branch` stay the `recovery_required` stubs.

`DocumentPromotionLifecycle::end_branch(service, root, r, end)` calls `answer::end`.

#### `answer::finish`: the first table, after a child that completed (R-12)

1. Read the context child through `record::child_state(tx, r, ChildRole::Context)`. Anything but `ChildState::Complete(stored)` is `recovery_required`; `stored.authority` is `Published(oid)` or `AlreadyCurrent(oid)` (`None` is `recovery_required`). `oid` is the child's authority OID, and the category (`published` or `already_current`) is what the record stores.
2. Judge R-3 on that OID with the tracking primary as it is now: the tracking primary is read from the plan's tracking ref (`RemoteRefPlan::from_configuration(remote, primary_branch).primary().tracking_ref()`, through `observe::reference_oid`) after the child's fetch moved it; then `observe::primary_holds_document(&repo, oid, tracking, r.primary_branch, r.remote_name, item, false)`. The record's own `primary_branch` and remote are used, not the live configuration (R-27).
3. The first row that matches:

   | Row | `answer_code` | `LinkWrite` |
   | --- | --- | --- |
   | R-3 holds | `primary_holds` | `Untouched` |
   | `situation == again` | `already_promoted` | `Renew { given_at }` |
   | otherwise | `promoted` | `Give { link, given_at }` |

   `given_at` is `(request.clock)().unix_timestamp()` (R-16; no real wait).
4. One `state::with_transaction` calls `record::complete_branch_record(tx, id, op, &BranchFinish { answer, category, link })`, which stores `answer_code` and `publication_category`, sets `step='complete'`, clears `cancel_requested`, writes the link as `link` says, and reads the row back through `validate`.

`complete_branch_record` checks, before the UPDATE: `path() == Branch`; `step.permits(Complete, ..)` (only `context_synchronized` passes: `Complete` is terminal, `Confirmed → Complete` is refused); the category is `Published` or `AlreadyCurrent`; the context child is `ChildState::Complete`; and the answer and the link write are paired (`promoted`↔`Give`, `already_promoted`↔`Renew`, `primary_holds`↔`Untouched`). Any other combination is `recovery_required`, so the table's pairing is held by the record layer and not only by `answer::finish`.

#### `answer::end`: the second table, after a child that ended

`BranchEnd::ContextGone` and `BranchEnd::ContextChild(SynchronizationError::RemoteContextDeleted | HistoryUnresolved)` are what the runner calls it with (`ends_context_child` also names `TicketClosed`, which only a ticket's context answers; for it `end` answers `Err(recovery_required)` as the closure does for an unnamed stop, and the record stays `confirmed`).

1. R-3 on the consented tip (`r.consented_context_oid`) with the tracking primary as it is: `primary_holds_document(.., consented, tracking, r.primary_branch, remote, item, remote_gone)`, where `remote_gone` is true only for `RemoteContextDeleted` (the fetch does not prune `R`).
2. The first row that matches: held → `primary_holds` (link untouched); `RemoteContextDeleted` → `remote_branch_deleted` (link row deleted); `HistoryUnresolved` → `history_unresolved` (untouched); `ContextGone` → `context_gone` (untouched).
3. `record::abandon_branch_record(tx, id, op, answer)`: `path() == Branch`, `permits(Abandoned)` (only from `confirmed`), the answer is one of the four allowed at `abandoned`, and the child is not retained or live without authority (the same test `let_go` uses: `Retained`, or `Unowned` whose phase is not `Cancelled | Interrupted | Failed | Completed`, refuses). It stores `step='abandoned'`, the refusal `reconfirmation_required` (through `Refusal::encoded_columns`), the answer; `remote_branch_deleted` also deletes the link row (kind `document`).

Departures from Cycle 03's shape, deliberately: `abandon_branch_record` takes no refusal (a promotion's ended record is always `reconfirmation_required`, R-13) and no link/`given_at` (an ended record never writes a link).

### 3. Part G item 3: the outcome types and every exhaustive match

```rust
// promotion/mod.rs
pub struct BranchPromotionOutcome { operation_id: OperationId, answer: PromotionAnswer, publication: Option<PromotionPublication> }
pub(in super::super) enum PromotionOutcome { Direct(DocumentPromotionOutcome), Branch(BranchPromotionOutcome) }
pub enum DocumentPromotionResult { Complete(..), IndexPending(..), Branch(BranchPromotionOutcome), BranchIndexPending(IndexPending<BranchPromotionOutcome>) }
```

`BranchPromotionOutcome` derives `Clone, Debug, PartialEq, Eq`; `operation_id()` and `publication()` are `pub`, `answer()` is `pub(in super::super)` (`PromotionAnswer` is a `categories!` type, visible to `crate::repository` only; the binding is inside it). **Addition to the brief's `{answer, publication}`: `operation_id`**, because `DocumentPromotionOutcome` and `BranchClosureOutcome` both carry one.

`type Outcome = PromotionOutcome`; `outcome()` returns `Direct`; `branch_outcome(r, _)` returns `Branch` built from the record: `answer = r.answer_code.expect(..)` (a finished branch record has one, as the closure's does), and `publication` from the record's `publication_category` and `context_oid` (design item 3 reads the category from the record): `published` → `Published { oid: context_oid }`, `already_current` → `AlreadyCurrent { oid: context_oid }`, `None` otherwise (an abandoned record has none). The runner's `make_outcome` passes the second slot's authority, which a promotion has none of, so it is ignored. **(approve)** The OID is the context tip `context_step` stored right after the child completed, which is the tip the child published or found current (nothing else may move the branch in between; Task 4's RR rows judge the cases that do). The binding reads only the category, so no data reads it.

`complete` and `index_pending` map `Direct` to `Complete`/`IndexPending` and `Branch` to `Branch`/`BranchIndexPending`.

Exhaustive matches the new variants change (found by reading every `DocumentPromotionResult` use in `src/` and `tests/`):

| Where | What |
| --- | --- |
| **production** `mutation/bind/lifecycle_verb.rs:468-474` (`Promote::call`) | gains `Branch` and `BranchIndexPending` arms producing `Called::Branch` |
| `promotion/lifecycle.rs` `complete`, `index_pending` (production) | as above |
| E-5, test: `src/repository/promotion/run_tests.rs:5069-5074` (`authoritative`) | gains `Branch(_) \| BranchIndexPending(_) => unreachable!("a direct fixture never answers as a pull-request promotion")`; `outcome()` just above it (5063-5067) has a wildcard and needs nothing |
| E-5, test: `tests/document_promotion.rs:314-320` (`promote`) | gains `Branch(_) \| BranchIndexPending(_) => Err(FixtureError)` |
| E-5, test: `tests/document_promotion/journeys.rs:257-261` (`authoritative`) | gains the same `unreachable!` arm |
| E-5, test: `tests/document_promotion/quality.rs:130-134` (`promotion_category`) | gains `Branch(_) => 3` and `BranchIndexPending(_) => 4` (the values 3 to 9 are unused) |

The last three are in the SSH target `document_promotion`: compiled through clippy only, not run. Every other use (`matches!`, `let ... else`, `match` with a wildcard: `run_tests.rs:1365, 1445, 1586, 2760, 5285, 5393, 5708-5709, 6018, 6038, 6279, 6409-6410, 6465, 6561, 6620, 6658, 7080, 7264`; `tests/document_promotion/advanced.rs:131`, `identity.rs:144, 153`) compiles unchanged, and each is a direct-mode assertion whose meaning does not change. The three `bind/*_tests.rs` hits (`lifecycle_tests.rs:1745`, `poll_tests.rs:225`, `repo_tests.rs:205`) only name the type in a fake's return.

### 4. Part G item 4: the binding

- `Called::Branch { outcome: Branched, index_pending }` with `enum Branched { Closure(BranchClosureOutcome), Promotion(BranchPromotionOutcome) }` (both in `lifecycle_verb.rs`). `called_of` wraps the closure's outcome in `Closure`; `Promote::call` wraps the promotion's in `Promotion`. No test constructs `Called::Branch` (grep), so no test edit; the closure's `branch_reached(&service, &accepted, &BranchClosureOutcome, bool)` keeps its signature, so its tests (`lifecycle_tests.rs:331-1188`) are untouched.
- `LifecycleBinding::call` (`lifecycle.rs:1380-1383`) matches `Branched::Closure(o) => self.branch_reached(..)` and `Branched::Promotion(o) => self.promotion_branch_reached(..)`.
- `promotion_branch_code(answer) -> ResultCode`, beside `branch_code`:

  | `PromotionAnswer` | Code |
  | --- | --- |
  | `promoted` | `ok` |
  | `already_promoted`, `primary_holds` | `already_applied` |
  | `remote_branch_deleted` | `remote_branch_deleted` |
  | `history_unresolved` | `recovery_required` (the blocker rides on Task 5's data) |
  | `context_gone` | `external_change` (as Cycle 03: `ContextGone` ends as `ReconfirmationRequired`, which `branch_code` maps to `ExternalChange`) |

- `promotion_branch_reached` is the twin of `branch_reached` (`lifecycle.rs:949-1032`): it stores `branch_record = true` (so the answer names `manyhands/document/<ULID>` and its worktree, decision 17), maps the code, and `index_pending && code == Ok` answers `DiscoveryPending` with the same-request recovery, as the closure does. Its `Effects`: `discovery` current or pending; `publication` from `outcome.publication()` (`Published` / `Current`), and when it is `None` from `context_child_push` (what the context child's row says landed, as the closure's R-32 does); `commit_oid` the pushed OID; `write`, `checkpoint`, `integration` and `cleanup` `NotRequested` (nothing reached the primary). **No data**: the answer's `MutationDataDto::PullRequestPromotion` and the link read with the document kind are Task 5's, as the task statement says ("until Task 5 a pull-request promote may answer without data"). `reached_code`, `reached_effects` and `Reached` are the direct path's and are not touched.
- The generic pieces serve as they are (Part G item 4): `parent_effects` (the context child's landed push and merge, nothing of a checkpoint), `parent_integrated` (false for a branch record), `branch_scope`, `branch_stopped`, `ended_answer` (an answered record returns `None`). Resume's `DocumentPromote` scope arm, its `settled` and `branch_record_of` (which reads `Close::parent` only), `replay.rs` `result_commit_held`, the CLI retry guard and `continued_a_lifecycle` need the new data tag, so they are Task 5's or Task 4's; **(approve)** I leave them, and say so in the report.

### 5. The pull-request begin (signature, caller, what it writes, `refuse_record`'s mode)

Caller: `DocumentPromotionLifecycle::begin_record(tx, id, op, item, expectation, remote, observed)`, which `promote_inner` calls with the revalidated preview inside the preflight lease (`run.rs:604-612`). The guard goes; the method becomes

```rust
match observed.and_then(|p| p.pull_request().map(|facts| (p, facts))) {
    Some((preview, facts)) => {
        let remote = remote.ok_or_else(state::recovery_required)?;
        record::begin_pull_request_record(tx, id, op, item, expectation, Some(remote), &record::PullRequestBegin {
            consented: preview.context_oid(),
            situation: facts.situation,
            primary_branch: preview.primary_branch().to_owned(),
            endpoint_digest: endpoint_digest_at_begin(tx, id, remote)?,
        })
    }
    None => record::begin_record(tx, id, op, item, expectation, remote),   // direct, or an existing record of either mode
}
```

`endpoint_digest_at_begin` reads the registration's `root_path` in the same transaction, opens the Git repository and returns `observation::endpoint_digest_of(&repository, remote)` ("read at begin", under the lease the revalidation held). `begin_pull_request_record` takes the digest as a plain argument, so the record layer is testable without Git.

`record::begin_pull_request_record(tx, id, op, item, expectation, remote, begin)`: returns the existing record of the operation first (`parent::existing_record`, as `begin_record` does, so a same-request re-entry continues the record in its own mode); any other unfinished record of the document is `busy` (a pull-request record supersedes nothing: its `cleanup` is never set, so no row is supersedable); otherwise it inserts `step='confirmed'`, `integration_mode='pull_request'` and the four begin-time columns (`consented_context_oid`, `situation`, `primary_branch`, `endpoint_digest`) in one INSERT, and reads the row back through `validate`. `insert_record` gains the mode and an optional `PullRequestBegin`.

`refuse_record`: `Lifecycle::refuse_record` passes `config.integration()` (the live configuration the confirm's `prepare` read, the same one that chose its observation) to the new `record::refuse_record_on(tx, id, op, item, expectation, remote, refusal, mode)`; the existing `record::refuse_record` delegates with `Integration::Direct`, so its seven callers (`record_tests.rs:1464, 1627, 1640`, `lifecycle.rs:943`) are unchanged. A pull-request refusal leaves the four columns null (`validate_pull_request` allows that only at `abandoned`, and the SQL CHECK already ties the four together).

### 6. `branch_step` reads the child's category; `end_branch` judges on the consented tip (R-3, R-12)

Covered in section 2. The two rules that are easy to get wrong, stated once: after a child that **completed**, R-3 is judged on the child's **authority OID** (the OID `child_state` returns), not on `context_oid` and not on the consented tip; after one that **ended**, on the **consented tip**, and the tracking primary is read at the moment the record answers (the child's fetch moved it before it stopped, `FetchTracking`, Baseline Fact 24). Neither call fetches.

### 7. The link: `first` built at finish (R-30), stored; `again`'s `given_at` renewal (item 9)

- **Built when the record finishes**, in `answer::finish`, outside the transaction: `canonical_repository_root(&request.root)` gives the repository; `read_configuration(&root)`: `Valid(config)` → `config.forge()`, otherwise `None`; `closure::checkpoint::context_branch_link(&repo, r.primary_branch, forge.as_ref(), remote, &LifecycleKind::DocumentPromotion.context_branch(item))`. That function (Task 2's correction: one helper taking the branch name) reads the remote's URL as `git2` reports it and builds the link through `forge::recognise` and `forge::pull_request_link`. A missing URL or unreadable configuration builds a link with a null URL and never fails the promote. The record stores no URL (R-30).
- **Stored** by `complete_branch_record`: `pull_request_links::upsert_link(tx, id, "document", item, &link, given_at)`. It writes the document's row with kind `document`.
- **`again`**: the row's link is returned (the binding reads it in Task 5); `pull_request_links::touch_link(tx, id, "document", item, given_at) -> rusqlite::Result<usize>` is `UPDATE pull_request_links SET given_at = ?4 WHERE repository_id = ?1 AND kind = ?2 AND item_ulid = ?3`, so the row's other columns are unchanged. If the row is gone (the update touches 0 rows) the record still finishes `already_promoted` and the answer's link reads null, as C15 says for a rebuilt answer. **(approve)** I do not fall back to `promoted`: the table keys the answer on the record's situation, not on what the row holds later.
- **Item 9 and the files the brief names.** `give_link` (`closure/record.rs:1461`) is private and writes the kind `"ticket"`; `branch_link` already calls `context_branch_link`. A promotion writes through `pull_request_links` directly with `"document"`, so there is no change to `closure/record.rs` or `closure/checkpoint.rs`: the design allows "a document twin beside them", and the twin here is three lines in the record layer plus `touch_link`, which the closure does not need. This also leaves Cycle 03's code and tests byte-for-byte as they are.
- RR8 (no link row written when the context does not exist at finish) is a re-run row for Task 4; in this task the branch exists whenever `branch_step` runs (`context_step` ends the record `ContextGone` when it does not).

### 8. Part G item 7: the second slot's readers

`remote_identity_of` (`lifecycle_record.rs:803-830`), `children_contexts` (`run.rs:2291-2308`) and `audit` (`lifecycle_record.rs:923-969`) already answer for a pull-request record with an empty second slot: `remote_identity_of` handles an `op` equal to `context_sync_id` (the record has no `primary_sync_id`, `validate_pull_request` refuses one) and a branch record's second slot; `children_contexts` returns the item's context for a branch record with a context child; `audit` reads `second_role(Branch)` = `Publication`, whose `attempt_of` is the empty primary slot (`ChildState::Absent`). So **item 7 needs no code**. It is pinned by running the real runner over a pull-request record: every runner test below runs `record::audit(&db)` after the record finishes or stops (it fails on any non-empty or mismatched second slot), and the stop-path through `index_children_owed` is run by the index-pending test. These are assertions that pass without new code, so I do not claim they were run red.

### 9. Part G item 11: no `RunEvent::Recording` for a promotion

`promote_branch` emits `Recording` when `!r.effect_written() || step == ContextSynchronized` (`run.rs:980-982`); `PromotionRecord::effect_written()` is always false, so a promotion would announce "Marking it closed…". The direct path already gates the same event on `L::KIND == TicketClosure` (`run.rs:807`); the branch path gets the same gate. Pinned by the first test (the observer is told `[ChildStarting(Context)]` and nothing else), run red by removing the gate.

### 10. Part G item 13 and U-2: the fake publisher

**Choice: a promotion twin, not a lift.** Reasons, from the source:

- Cycle 03's `PullRequestIo` (`closure/run_pull_request_tests.rs:618-788`) is built on the closure fixture: its `tracking_context` writes `tracking_context_ref()` (a ticket ref), its `decide_push` calls `lacks_primary_closure` with the ticket path, and it delegates to the closure module's re-export of `FakeIo`, whose publish path needs the tracking context ref to exist already (`oid = refname_to_id(tracking ref)`), which is false for the first publication of a new document (C1).
- It has about 100 `PullRequestIo::default()` / struct-update call sites in two closure test files; lifting it means changing every one or inventing a constructor that carries the ref names. That is a change of two Cycle 03 test files with a risk to `ticket_closure` and no gain for the four tests of this task.
- The test-local `Forge` (a bare repository) is needed only by P5 (Task 7: the poll and the retirement). The four tests here script a merge as closure's D4/D11 do (plain local commits and tracking refs: `forge_merge`, `fetched`). If Task 7 finds the lift worthwhile it can do it then, with both users in view.

The twin lives in a new `src/repository/promotion/run_pull_request_tests.rs`, included from `run_tests.rs` as `#[path = "run_pull_request_tests.rs"] mod pull_request;` (the closure's own arrangement), so the test path is `repository::promotion::run_tests::pull_request::<test>` and Tasks 4 and 7 extend the same module. It holds:

- `PullRequestIo` (implements `PromotionIo`): records `children: Vec<(SynchronizationTarget, OperationId, bool)>`; a **primary target is refused** (`primary_calls += 1`, `Err(RecoveryRequired)`) and `delete` is counted and asserted zero; for the context role it reserves the child as `FakeIo` does (`reserve_promotion_child`, `restart_promotion_child` on replay), and then, in order: a scripted `fetched` tracking-primary move (written to `refs/remotes/origin/main` before any stop, as a child's fetch is), a scripted stop (`kept` reserves the child and leaves its row active, as closure's; a `restart` acknowledgement of a cancelled row ends it), a scripted `drive` (a real local update through `sync::promotion_test_drive_local_bound`, the "Merge primary into document" stage), else a plain publication of the branch's tip. A publication completes the child row with `complete_child_row` (`published`, or `already_current` when the tracking context already equals the tip), writes the tracking context ref, and returns the matching `SynchronizationOutcome`.
- The fixture additions on `Fixture`: `pull_request()` (the PR configuration written uncommitted in the primary's worktree, as `a_pull_request_confirm_stops_before_any_direct_record` does today, so the local primary never moves), `unpublish()` (deletes the tracking context ref: a new document), `merge_into_primary()` (a forge squash or merge: a commit on the primary with the document's content, tracking primary moved to it), `edit()` (a save: a commit in the context), `link()` and `refs()`.
- Every test asserts that the local primary did not move, that `io.primary_calls == 0` and `io.deletes == 0`, that the table has no `integrating`-or-later row and no `Promote document <ULID>` commit exists (`promotion_commits == 0`), and that `record::audit` accepts the registry.

### 11. The guard's removal

The interim guard in `DocumentPromotionLifecycle::begin_record` (`lifecycle.rs:918-923`) is removed. Its test `a_pull_request_confirm_stops_before_any_direct_record` (`run_tests.rs:7305-7332`, this Cycle's own) is **deleted**, not replaced: its two claims (no direct record is begun, nothing merges into or publishes the primary) are now asserted by every test of the new module, and its third (`recovery_required`) is no longer true.

### 12. The answered-`abandoned` invariant (deferred review point)

Amend `SCHEMA` in `promotion/record.rs` with `CHECK(answer_code IS NULL OR consented_context_oid IS NOT NULL)`, next to the other mode checks (`record.rs:262-267`), and the same rule in `validate_pull_request`: `r.answer_code.is_some() && !begun` refuses. The migration has never run on a real registry, so the amended DDL is the one the rebuild and the create path both write (`SCHEMA` is the single source; `SCHEMA_V2` and `V2_COLUMNS` do not change). Test: one row in `pull_request_promotion_steps_and_checks`, "an answer with no begin-time columns" (`Abandoned`, `bundle(r, false)` keeping the default `ContextGone` answer and its `reconfirmation_required` refusal), `Expect::Both`, run red once by removing the SQL CHECK and the Rust rule (today both accept it, so the row fails on `validate` and on `inserted.is_err()`). It passes through Task 1's table with its existing rows unchanged; `promotion_modes_migration_rebuilds_a_v2_table_once` compares to `SCHEMA` and needs no edit (I will confirm by running it).

### 13. Guards on `parent::complete_record` and `supersede`

Task 1's implementer asked for them. **(approve) I add none**: `parent::complete_record` already refuses unless `r.step() == BranchDeleted`, which `validate_pull_request` and the SQL CHECK forbid for a pull-request row (Task 1's table has `branch_deleted` as `Expect::Both`); `parent::supersede` is reached only from `record::begin_record` after `old.cleanup.is_some_and(supersedable)`, and a pull-request row never has a `cleanup`; `begin_pull_request_record` refuses any unfinished record as `busy`. An explicit mode check would be a second refusal of what two checks already refuse, which no test could show red. If the approver wants it, it is two lines and one row.

### 14. Tests, with exact names and rows

All run by `--lib repository::promotion::run_tests::pull_request::<name> -- --exact` unless a path is given. Each starts from the same pull-request fixture and asserts the common invariants of section 10.

**`first_promote_publishes_and_gives_the_link`** (C1, C2, C3; the answer is `Branch(promoted)` and `outcome.publication()` is `Published{oid: tip}` or `AlreadyCurrent{oid: tip}`; the record is `complete`, mode `pull_request`, `answer_code promoted`, `situation first`, `consented_context_oid == L`, `primary_branch == "main"`, `endpoint_digest == endpoint_digest_of(repo, "origin")`, `context_oid == tip`, no `base_oid`/`integration_*`/`cleanup`/primary child; the link row has source `manyhands/document/<ULID>`, target `main`, kind `document`, `given_at == CLOCK` and is the only row; the observer is told exactly `[ChildStarting(Context)]`, so no `Recording`; the same request again answers equal and runs no child):

| Row | Setup | Publication | `publication_category` |
| --- | --- | --- | --- |
| a new document (C1) | `unpublish()`: no tracking context | `Published` | `published` |
| an edit of a merged document (C2) | `merge_into_primary()`, then `edit()` | `Published` | `published` |
| a branch already published by `item sync` (C3) | the fixture's tracking context equals the tip | `AlreadyCurrent` | `already_current` |

**`promoting_again_synchronizes_and_gives_the_same_link`** (C4 to C7, A4): each row first runs a first promote, reads the link row `A`, then a second request on a later clock (`CLOCK + 86_400`). Common assertions: answer `already_promoted`; `situation again`; the row has `A`'s source, target, forge and URL and `given_at == later`; still one row; the first record's answer is unchanged.

| Row | Change before the second prepare | Child | Publication | Also asserts |
| --- | --- | --- | --- | --- |
| nothing new (C4) | none | `already_current` | `AlreadyCurrent` | |
| after a save (C5) | `edit()` | publishes | `Published{tip}` | tip is the edited commit |
| after the remote's primary moved (C6) | tracking primary moved to a newer commit; `drive` merges it | publishes the merge | `Published{merge}` | the branch tip is a commit `Merge primary into document` with the old tip as a parent; the primary's commit is its other parent |
| after the forge setting changed (C7) | `[forge] pull_request_url` written into the configuration | `already_current` | `AlreadyCurrent` | the row's URL is still `A`'s (null), not the template's; the prepared link equals the row's |

**`a_held_document_gives_no_link`** (C8 to C10):

| Row | Situation | Result | Record | Link row |
| --- | --- | --- | --- | --- |
| prepare, held (C8) | forge squash fetched before the prepare | `preview_document_promotion` is `PrimaryHolds`; a confirm with the earlier consent is `ExternalChange`; no row in `document_promotions` | none | none |
| squash between prepare and confirm, branch kept (C9) | prepared (`first`), then a squash on the tracking primary; `drive` merges it into the branch | `Branch(primary_holds)`, `Published{merge}` | `complete`, `primary_holds`, category `published` | none (`first`); in a second row the same with a row from an earlier promote: untouched, `given_at` not renewed |
| merge with the branch deleted (C10) | prepared, then a merge commit on the tracking primary moved by the child's fetch; the child stops `RemoteContextDeleted` | `Branch(primary_holds)`, `publication None` | `abandoned`, refusal `reconfirmation_required`, `primary_holds`, no category | untouched |

C10's row also asserts the child's row ended (`cancelled`) and nothing holds the context.

**`a_declined_pull_request_answers_remote_branch_deleted`** (C11): a first promote, then a second request (`again`); the branch is deleted without a merge (the child stops `RemoteContextDeleted`, no fetch): answer `Branch(remote_branch_deleted)`, record `abandoned` with `reconfirmation_required` and `remote_branch_deleted`, the link row deleted (no row), child ended. Then a "republish" (the fixture's tracking context restored to the tip) and a new request: the prepare is `first` (row gone), the answer is `Branch(promoted)`, the row exists again with a fresh `given_at`.

Additional tests (each is a layer a named test cannot reach):

- **`a_pull_request_promotion_whose_index_step_failed_is_branch_index_pending`** (the twin): a service opened with `FailurePoint::RefreshFailure` finishes the record and answers `BranchIndexPending` whose `authoritative` outcome equals the later `Branch` answer of the same request; the record is `complete`.
- **`pull_request_begin_and_finish_write_their_columns`** (`promotion::record_tests`, the layer that has no runner): `begin_pull_request_record` writes the four columns and the mode and returns the same record for the same operation; a second operation for the document is `busy`; `refuse_record_on(PullRequest)` writes the mode and no column; `complete_branch_record` and `abandon_branch_record` rows: every valid pair (`promoted`+`Give` stores the link row with kind `document`; `already_promoted`+`Renew` changes `given_at` alone; `primary_holds`+`Untouched` writes nothing), each mismatch refused (`promoted`+`Renew`, `primary_holds`+`Give`), a category other than `published`/`already_current`, a record at `confirmed` for `complete`, a retained child for `abandon`, an answer outside `abandoned`'s four, and `remote_branch_deleted` deleting the row while the others keep it.
- **`pull_request_promotion_steps_and_checks`** (existing, Task 1's): the one added row of section 12.
- **`promotion_branch_answers_map_to_codes_and_effects`** (`mutation::bind::lifecycle_tests`): `promotion_branch_code` for the six answers, `promotion_branch_reached` for `promoted` (`Ok`, effects `Published`), the same with `index_pending` (`DiscoveryPending`, `Standing::Owed`), `already_promoted` over `AlreadyCurrent` (`Current`), `primary_holds` (`AlreadyApplied`), `remote_branch_deleted`, `history_unresolved` (`RecoveryRequired`), `context_gone` (`ExternalChange`); the binding names the branch afterwards (`names_branch()`).

Red runs (Step 4): the new runner tests fail first for the missing types (compile), then, for each row that would pass at once, against a deliberate stub: (a) the pull-request begin stubbed to a direct begin (the columns test and `first_…` fail), (b) the tracking primary not read (C10's row fails), (c) `touch_link` stubbed to a full upsert (the `again` rows fail on the columns), (d) `Recording` gate removed (the observer assertion fails), (e) R-3 stubbed to false (C8 and C9 fail), (f) the link delete removed (C11 fails). The invariant row of section 12 runs red by removing the CHECK and the rule.

Unchanged tests Step 6 names, run after: `cache_loss_contained_context_cleans_without_another_merge_or_child`, `already_current_publication_is_stored_and_replayed_without_children_or_deletion`, `contained_cleanup_needs_no_identity_or_new_merge` (promotion `run_tests`), `d1_records_publishes_and_completes_without_touching_the_primary` and `d2_an_existing_closure_publishes_pending_work_and_is_already_closing` (`closure::run_tests::pull_request`), `promotion_modes_migration_rebuilds_a_v2_table_once`, and the E-5 test in `run_tests.rs` (`local_full_lifecycle_and_exact_replay` uses `authoritative`).

### 15. Existing tests I expect to edit

1. E-5, four sites (section 3), additive arms only; three are in the SSH target `document_promotion` (compiled through clippy only).
2. `a_pull_request_confirm_stops_before_any_direct_record` (`run_tests.rs:7305-7332`): deleted (section 11).
3. `pull_request_promotion_steps_and_checks` (`record_tests.rs:1833`): one added row (section 12).

No other. In particular none of Cycle 03's closure tests, none of `bind/lifecycle_tests.rs` (the new binding test is added, the old ones keep their signatures), and none of Task 1's or Task 2's other tests.

### 16. Departures from the brief, and points for the approver

1. **No edit to `closure/record.rs` or `closure/checkpoint.rs`** (the brief lists them for item 9): the link row is written and renewed from `promotion/record.rs` and `pull_request_links.rs` (section 7).
2. **`BranchPromotionOutcome` carries `operation_id`** besides `answer` and `publication`; the publication's OID is the record's `context_oid` (section 3) **(approve)**.
3. **`again` with its row gone** finishes `already_promoted` and renews nothing (section 7) **(approve)**.
4. **No guards on `parent::complete_record` and `supersede`** (section 13) **(approve)**.
5. **Resume's `DocumentPromote` scope arm, `settled`, `branch_record_of`, `replay.rs` and the CLI retry guard** are not in this task: they key on the new data tag (Task 5/8) or on the re-run (Task 4) **(approve)**.
6. **The re-run rows are Task 4's and stay as they are**: after the child, `promote_branch` still runs the cancellation check (`run.rs:977`) and, before it, the "remote is the one configured while `base_oid` is unset" check (`run.rs:936`), which a promotion's record (it never has a base) fails on every call after the configured remote changes; RR2 (no checks after the child), RR4, RR5 and the crash boundaries around the completion are not added here.
7. **`TicketClosed` as an ended child** answers `recovery_required` from `end` (a document's context cannot answer it).
8. **The twin fake, not a lift** (section 10), with the reasons.
