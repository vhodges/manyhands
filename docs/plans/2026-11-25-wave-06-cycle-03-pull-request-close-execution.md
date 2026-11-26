# Wave 06 Cycle 03 Execution Ledger

Ticket vh-gy8gz (`01M87683QK8E34EYY8KS4VQG9Y`). Plan: `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-implementation.md` (approved 2026-11-25, revision 5, amended by decision 15). Execution: subagent-driven, implementers on Sonnet 5.5 at high effort, one fresh reviewer per task. One line per paragraph.

## Approval

- The owner approved the three documents at revision 5, the named test edits E-2 to E-23 and Task 0's mechanical edits, subagent-driven execution, and implementation (Cycle document decision 14). Commit `2e9ffa8a`; approval comment `01M89S6P4DPY474F28E5T3GSTN`, published at `1040f41a`.
- Notes left at approval: vh-c97se (`01M89S94FTQEZP1T765HR5G6E4`), vh-rphcs (`01M89S95WXZS17TKVF3TTSP6TV`; its first answer was `partial` / `discovery_pending` with the comment published, and the same request sent again finished it), vh-ssepw (`01M89S977WXZ67PRYFR574RSCZ`), vh-p7st2 (`01M89S98K2RR1XMH7A2VZKM3WK`).
- D-9: with the owner's go-ahead, the Wave 06 document's Cycle 03 checklist gained three "Decided" lines and a Tests line, saved on its open branch (on top of Cycle 02's D-6) and published with `item sync` at `681cbe08`. Not promoted.
- Cycle 02's session left comment `01M89SA16Q63FHHNVVTE4VSSGQ` on this ticket (commit `72296bd0`): its `setup_pending` is one gate, an exhaustive `Command::setup_gate`, and both Cycles append result codes after `already_applied`, so the second to land merges the code lists.

## Task 0

- Base: `origin/main` `3df52dcb`. HEAD before and after the merge: `1040f41a` (already up to date; merged at `9a73bdbd` during planning, never rebased). Main has not moved since planning.
- Cycle 02 on `origin/main`: absent. No `schemas/v1/pull_request.schema.json`, no `PullRequestDto`, no `setup_pending`. Task 4 creates the schema and DTO in the shape `{ source, target, forge, url }`, which Cycle 02's design (its Part I) also uses.
- Baseline at `1040f41a` (09:43 to 09:47): `cargo clippy --all-targets --all-features --locked -- -D warnings` clean, `cargo fmt --check` clean, `cargo test --all-features --locked --lib --no-run` built. No test was run.
- References (Step 3), by three read-only research agents, appendices A to C: Baseline Facts and Parts A to C, 207 rows (188 existing, 17 new and absent, 1 moved, 1 unclear, none wrong); Parts D to H, the obligations and rulings, 223 rows (180 existing, 39 new and absent, 1 moved, 3 unclear, none wrong); named edits E-2 to E-23, 22 checked, none wrong.
- Moved: Fact 15's `WorktreeConflicted` refusal is `remote/sync.rs` `require_clean_target_state` (741-770), not `refs.rs`. U-1's `finish_remote_operation` is declared at `remote/reservation.rs:2112` (2182 is inside `finish_remote_operation_capturing`).
- Test module paths (Step 3): all exist, every one under `--lib` (appendix C, section 3). The `pull_request` module of `closure/run_tests.rs` is new. Targets `read_items`, `read_relationships`, `read_contract`, `mutation_contract`, `mutation_bridges`, `mutation_confirmation`, `cli` exist.
- CI (Step 5): no new test target; `.github/workflows/build.yml` needs no change. (Its list already omits `editor_feasibility` and `read_characterization`, unrelated to this Cycle.)

### Named edits added by the sweep (Step 4)

- **E-24** (Task 1) `tests/ticket_closure/journeys.rs` `contained_closed(false)` (about 355-405, SSH target `ticket_closure`): a kept open context beside a primary that holds the closure reads `closed` with the primary's fields (Part A's first row), not `open`; its observation match gains `Closing` (with E-2). **Reverses an intent; approved by the owner in decision 15.**
- **E-25** (Task 1) `tests/cli_remote/retirement.rs` about 131-153 (SSH target `cli_remote`): "default poll retains own-worktree read precedence" flips: `ticket show` of `ITEM` reads `closed`. Corrected at Task 1: `OTHER_ITEM` is itself closed on the primary, so it reads `closed` and is never ready; `old_ready`'s `!any(OTHER_ITEM)` (145-152) holds for that reason, and `ready_after` (186-195, "retirement exposes primary closure and unblocks the still-open peer dependent") flips to `!any(OTHER_ITEM)`. **Reverses an intent; approved by the owner in decision 15** (ruling 9).
- **E-26** (Task 4) `ClosureDto` literals E-18 misses: `src/repository/closure/run_tests.rs:1532`, `tests/ticket_closure/journeys.rs:399`; production `read/items.rs:1355`.
- **E-27** (Task 4) `ItemDto` production literals in `read/items.rs` (about 1328, 1441, 1497, 2284, 2449).
- **E-28** (Task 7) if `TicketClosureOutcome::integration()` and `cleanup()` become optional: 48 test sites in `closure/run_tests.rs` and `tests/ticket_closure/` (appendix C). See ruling 3.
- **E-29** (Task 7) if `closure::begin_record` gains the mode and action: about 30 test callers (appendix C). See ruling 4.
- **E-30** (Task 7) exhaustive `ChildRole` matches in `remote/reservation_tests.rs` (4542-4660) and `promotion/record_tests.rs` (222-236), files in no File Map row.
- **E-31** (Task 7) step lists and fence rows in `remote/reservation_tests.rs` (3749-3753, 4413-4425): `Retired` and the branch-record rows.
- **E-32** (Task 7) the 16 `PromotionIo` implementations in tests, if the trait gains a required method. See ruling 5.
- **E-33** (Task 3) the validators' local-sync callers, `local_sync/resolve.rs` (446, 615, 636, 993) and `local_sync/run.rs` (714, 794): no primary side.
- **E-34** (Task 2) `require_ticket_open`'s production callers (`repository.rs:316`, `context_release.rs:231`, `bind/republish.rs:202`, `bind/resolve_history.rs:226`).
- **E-35** (Task 1) `src/bin/manyhands-cli/dispatch.rs:59-61` maps any other `--closure` value to `All`: `closing` needs its explicit arm.
- **E-36** (Task 1) production exhaustive matches on `ClosureState`, `DependencyState`, `ReadinessReasonCode` and `ClosureFilter` (appendix C), desktop arms compile-level only.
- **E-37** (Tasks 5, 10, 11) `human_admin_tests.rs:107-111` (a distinct phrase per effect) and the production `PreviewDetailsDto`, `LifecycleBlocker`, `Command` and `MutationDataDto` matches (appendix C).
- **E-38** (Task 10) none expected: the direct close's made-ready read runs after the index refresh, so `c5_made_ready.rs`'s cases keep their answers.
- **E-39** (Tasks 11, 15) tests that parse documents (`tests/cli/skills.rs`, `inventory.rs`, `recovery_tables.rs`, `collaboration_exits.rs`, `exits.rs`, `authoring_exits.rs`): the amendments keep them true.
- **E-40**, **E-41**: none expected (tests over `::ALL`; the desktop's intents).

### Corrections to the design's named edits

- E-12 also raises `tests/mutation_confirmation/conformance.rs` `ROWS: [Row; 8]` to 9.
- E-15: `tests/cli/recovery_tables.rs` (586-587) allows at most two C5 reasons, so the new verb's recoverable codes need rows with goldens, not reasons.
- E-16 (lines in appendix C, 1a): with decision 15, line 422's plan becomes `[(2, [B, C]), (3, [D])]` (A is batch 1 in the plan of every ticket and is left out); line 425 compares with the blocked list. A's own `readiness` reads `ready` (Part A: computed as an open ticket's) while the queries leave A out. The second test's SQL (951-957) is limited to A's primary row.
- E-18 misses two `ClosureDto` literals (E-26).

### Rulings

- Ruling 1: if the stored-data dispatch in `mutation/dto.rs` is keyed by a `Command` value that does not exist until Task 11, Task 10 writes and passes the `ticket close` rows and Task 11 adds the `ticket withdraw-closure` rows to the same test — the plan's order (Task 10 before 11) and "every commit compiles" both hold that way — none, the rows move one task.
- Ruling 2: if Cycle 02 lands first, `setup_pending` for the new verb is its classification as Refused in Cycle 02's `Command::setup_gate`, in place of Task 11 Step 1's "add `setup_pending` to the close and withdraw prepares, after the ticket blockers" — the gate and its order (the envelope's code, before the ticket's own blockers) are Cycle 02's (decision 5, Cycle 02's comment `01M89SA16Q63FHHNVVTE4VSSGQ`) — if wrong, one row moves between the gate and the prepare.
- Ruling 3: Task 7 keeps the direct `TicketClosureOutcome` accessors' types where it can (a branch outcome of its own, or accessors the direct path still reads unchanged), so E-28's 48 sites stay untouched; if the design checkpoint shows the shared type must change, E-28 applies as a mechanical edit — Review Focus: "every direct record behaves as before" — 48 mechanical edits.
- Ruling 4: likewise, the direct `begin_record` keeps its signature and branch records get their own entry point, so E-29's callers stay; otherwise E-29 applies — the same reason — about 30 mechanical edits.
- Ruling 5: the publication child's new `PromotionIo` method has a default that the direct path never reaches, so only `FakeIo` implements it and E-32's 15 other impls stay — no direct test changes — if the default hides a missing implementation, a test of the branch path fails on it.
- Ruling 6: the design's Part D prepare names `WorktreeConflicted` as a blocker, but it is a result code; the pull-request prepare reports the same condition with an existing blocker (the direct close's `PrimaryConflicted` where it is the root, the context's existing refusal where it is the worktree), since Obligation 10 forbids a new closure blocker — the Task 7 design checkpoint records which — if wrong, a blocker name in one preview.
- Ruling 7: R-30's "push decision" names no symbol; Task 6 finds where a ticket context's synchronization chooses the head it pushes (the plan path and the merging path both reach it), next to `pull_only_push_decision` (`remote/sync.rs` about 4178), and the report names it — the design names the behaviour, not the place — none.
- Ruling 8: the other unclear points of appendix B (Part D 3.3's `repo sync` way out for a tip only the local primary holds; `closed_at` "frozen in the record before the commit" against today's single `begin_closing` UPDATE; `permits`' abandon list, which keeps today's `Closing -> Abandoned`; the frozen DDL for the rebuild, Obligation 3) go to Task 7's design checkpoint, which the orchestrator checks against the design before implementation — they are rules inside the lifecycle's mode, the checkpoint's purpose — none.
- Ruling 9 (Task 1): E-25 covers `ready_after` (186-195) as well as the `ticket show` read; both assert the own-worktree precedence decision 15 reverses, in the same case, and the ledger had mis-described the effect — the owner chose "apply as designed" for that precedence — if wrong, one assertion of one SSH case.
- Note for vh-p7st2 (not this Cycle's, R-22): `desktop/.../lifecycle_lock.rs:109` leaves `confirmed` unlocked, but a new-branch record holds the ticket from step 1 while at `confirmed`, so the desktop would not show it locked.


## Task 1: The Closing State

- Commit `940b42be` (base `e21ba7d5`). New tests: `closure_state_follows_both_copies`, `closure_filter_closing_and_open` (`read_items`); `a_closing_dependency_blocks_until_the_primary_holds_it` (`read_relationships`); `a_closing_ticket_is_open_to_its_dependents_and_left_out_of_the_queries`, `critical_path_gives_closing_tickets_no_length` (`--lib repository::read::graph::tests`). Each was run red, then green by exact name, with the edited tests of E-2, E-3, E-16, E-17, E-24 (`ticket_closure` case `contained_closed_collaborator_cleanup_only`), E-25 (`cli_remote` case `retirement_cli_authenticated_live_pass`), `presentation_across_contexts` and `published_enumerations_list_exactly_the_registered_values`; the three help goldens regenerated. Static gates and `cargo check --features desktop-test-support --tests` clean.
- E-16's second test also rewrites A's primary file after the SQL closure, so the refresh observes the primary again.
- Review: spec compliant, approved, no Critical or Important. Five Minor findings deferred to the final review (documentation of `ReadinessReasonDto`, `ReadinessState`, `ReadinessDto` and `ticket_critical_path`; a stale test name; first row wins for two primary-copy rows of one ID).

## Task 2: The Guard And The Save Check

- Commit `925b5dd8` (base `ba899c17`). New tests: `the_guard_reads_the_primary_only_in_pull_request_mode`, `a_save_cannot_change_the_closure` (`--lib repository::ticket_guard::tests`); `context::a_closing_ticket_takes_comments_and_saves` (`mutation_bridges`); E-14's `guard_reads_branch_tips_only` edited. Red first, then green by exact name; static gates clean.
- In direct mode a head that holds a closure is refused as `ticket_closed` before the save check, so the save check's "changed" and "removed" rows are pull-request rows only (documented in the test).
- Review: approved, no Critical or Important. Three Minor deferred (the guard table's 48 fixtures, about 11 s; an unexplained `ne TicketClosed`; the head is parsed again on every save).

## Task 3: Immutability In A Ticket Context

- Commits `5c6e4538` and `1d581a9c` (base `925b5dd8`). New tests: `provisional_closures_may_change_primary_ones_may_not` (`--lib repository::integration::tests`, 33 rows over both validators); in `--lib repository::remote::sync::tests`, `ticket_context_closure_rule::a_pinned_ticket_context_window_gives_the_fetched_primary_to_both_validators`, `…::a_primary_target_gives_no_primary_side_and_keeps_every_side_closure`, `…::a_primary_target_pinned_window_gives_no_primary_side` and `ticket_context_resolution_call_site::resolve_synchronization_keeps_the_closure_the_pinned_primary_holds`. Unchanged and green: `all_recorded_ticket_sides_constrain_retained_closure_before_writes`, `retained_candidate_cannot_bypass_all_side_closure_validation`; E-14 and E-33 applied.
- Review round 1 found the primary-target row could not fail (no pinned window) and no test drove a call site; fix round 1 added a pinned window 1 (red with the target-kind guard removed) and the `resolve_synchronization` row (red with the preflight passing none). Re-review: both addressed. `reconcile_pending_candidate` (two sites), `reconcile_resolution_candidate` and `conflict_read::repair_accepted` are compile-checked only; the SSH tier's `remote/` targets run at review-ready.
- Five Minor deferred (an absent pinned window row reads as no primary side; `conflict_read` reads the window on every lookup; the three-side rule applies to every changed ticket path; two table rows want a comment; a stray `closed_by` without `closed_at`).

## Task 4: The Link Record And The Item's Fields

- Commit `7cf3f9c2` (base `1d581a9c`). New tests: `pull_request_links_migration_creates_its_table_once` (`--lib repository::discovery::tests`), `retire_context_deletes_the_link` (`--lib repository::cleanup::tests`), `a_finalized_retirement_deletes_the_link` (`--lib repository::context_retirement::tests`), `ticket_show_gives_the_link_and_the_publication` (`read_items`), `show_names_the_publication_and_the_pull_request_link_when_it_has_them` (`--bin manyhands-cli`). Red first, then green by exact name, with the read_contract and cli golden tests; 25 goldens regenerated, adding only 44 `"publication": null` and 53 `"pull_request": null` lines.
- `schemas/v1/pull_request.schema.json` and `PullRequestDto` added in Cycle 02's shape (Cycle 02 is not on main).
- Ruling 10: `pull_request.schema.json`'s type is `["object", "null"]` — the item's field must be null in lists, for documents and without a link (R-15), the schema checker accepts only a bare `$ref`, and the repository already uses nullable referenced files (Cycle 02's `repository_setup` among them) — if Cycle 02 keeps an always-object file, the second Cycle to land adds a nullable variant for the item. Cycle 02 was told (comment `01M89ZZQPRRTY40V192ABXY76A` on vh-c97se, saved on its branch; its publication is pending while that session's worktree has uncommitted work).
- **E-42** (Task 4, mechanical) `tests/mutation_replay.rs` about 442: `the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`'s `name.contains("request")` matched the new table; it excludes `pull_request_links` by name, intent unchanged.
- Review: approved, no Critical or Important. Five Minor deferred (the preserving retirement branch has no test of its own; `tracked_closure` reads any failure as `pending`; `ForgeKind::parse` repeats a match in `canonical.rs`; `item_show` rebuilds a link from the DTO; E-42's predicate).

## Task 5: The Code, The Effects And The Blocker

- Commit `c38f5adc` (base `1e22fdc7`). E-5 run red first (`results::tests::result_codes_match_the_design_table_exactly`, `results::tests::result_code_messages_are_fixed`), then green with a new `way_out` row for `remote_context_unmaterialized` (red first), the `read_contract` enumeration test, both `every_result_code_has_a_*` tests (E-6, reason "golden written in Task 14"), the `human_admin` effect-phrase test and eight E-40 tests over `ResultCode::ALL`; `cargo check --features desktop-test-support --tests` and the static gates clean.
- Review: approved, no Critical or Important. Two Minor deferred (the blocked schema's description names a `repo sync` way out the code does not route; no combination row in the way-out table).

## Task 6: The Primary's Closure And Two Clones

- Commits `beda6cab` and `b13a53c0` (base `c38f5adc`). New tests: `closure_replaced_bytes` helper test and `closure_only_conflicts_settle_with_the_remote_closure` (G1 to G7, `--lib repository::integration::tests`); `a_context_stage_settles_a_closure_only_conflict`, `a_primary_stage_keeps_the_primary_closure`, `a_fast_forward_that_drops_the_primary_closure_is_not_published` (G8) and `a_proven_merge_merges_the_primary_in_cleanly` (`--lib repository::remote::sync::tests`); `a_ticket_closed_refusal_is_answered_as_itself` (`--lib repository::mutation::bind::sync::tests`). G4 to G6 and the proven merge pin unchanged behaviour (exempt); the rest red first. `tests/comment_publication.rs` gained a mechanical `TicketClosed` arm (compiled, not run); `outcome_tests.rs` a row and a count (20 to 21).
- The push decision is in `remote/sync.rs` `synchronize_remote_observed`, just before the push, on the one `candidate` every path reaches (the plan path, the merging path and both resumes): `push_unless_unclosed` runs `primary_closure_push_refusal` and only then the push it is given (ruling 7, ruling 12).
- Ruling 11: the refusal also answers `ticket_closed` for a head already on the remote, which would otherwise answer `current` — a ticket whose fetched primary holds its closure is closed; the guard answers the same once the local primary moves (R-10), and K-11 expects `ticket_closed` under a fast-forward forge merge — an item sync of a closed ticket answers `ticket_closed`, not `current`.
- Ruling 12: the refusal and the push are one function that takes the push as a parameter, tested with a recording push (never called when refused, called once otherwise) — no non-SSH driver of `synchronize_remote_observed_inner` exists, and the SSH budget is one case (Task 14) — a small refactor of the tail.
- Review (Opus): behaviour correct by inspection; two Important test gaps (the ended item sync's answer, the production call site), fixed in round 1 and confirmed by a scoped re-review. Deferred Minor: `republish_reserved` (`remote/sync.rs` about 8653) pushes a ticket context's head without R-30's check; a kept `ticket_closed` item sync offers no way out (as `HistoryUnresolved`); a head with no canonical ticket at the path bypasses R-30; `NotPreserved` answers `recovery_required`; small duplications.

## Task 7: Branch Records, The Pull-Request Close

- Ruling 13: the design checkpoint (Step 4) is written before the tests (Step 2), by an Opus designer, and splits the task into commit-sized slices that Sonnet implementers then build test-first in order — the checkpoint precedes implementation either way, a changed approach would otherwise waste the D-row tests, and one implementer cannot hold the task's twenty-odd files — one hand-off per slice.
- Design checkpoint (Step 4): written by an Opus designer, checked three times by an independent Opus checker against the design and the source (revision 1: sound with fixes; revision 2: six new problems; revision 3: two one-line corrections, applied by the controller), and approved on 2026-11-25. The approved checkpoint is Appendix D. It splits the task into slices A (the record's path: rebuild, types, child roles, fence range), B (the branch observation and consent), C (the branch runner and the publication child), D (the stop rows and the ends) and E (the hold and the binding views), each compiling on its own with the direct tests green. Rulings 3, 4 and 5 are met without E-28, E-29 and E-32: `TicketClosureOutcome`, the direct `begin_record` and `PromotionIo` are unchanged.
- Ruling 14 (designer's Q1): a nullable `consented_context_oid` column, so a retry can prove the consented closure against the head the digest bound — the digest keeps the head, as approved — one more internal column.
- Ruling 15 (Q2): prepare reports `remote_context_unmaterialized` through a preflight-only `ClosureBlocker` value left out of `CLOSURE_BLOCKERS`, so the stored mask is unchanged (Obligation 10 read as "no new stored blocker") — one enum value.
- Ruling 16 (Q3): the step table is read through R-3: `closing` → `publication_pending` (the publication child minted) → `published` or `complete`; attempt numbering is internal — none.
- Ruling 17 (Q4): `answer_code` has a CHECK on its shape only, its values listed in Rust, so Tasks 8 and 9 add values without another rebuild — a value error is caught in Rust, not SQL.
- Ruling 18 (Q5): `new_branch` may be abandoned from `branch_deleted`, for Part F's re-check (F8) — one transition.
- Ruling 19 (Q6, flagged to the owner, who may overturn it before slice E): `item resolve-history` passes a branch record's hold, so a publication child stopped by unresolved history can be cleared and the same request continue; otherwise the record would hold the ticket for good — D16's resolve-history row changes for branch records.
- Ruling 20 (Q7): answer row 5 ("the publication remote changed or is gone") is the runner's configuration check plus the transport kinds `PublicationRemoteMissing`, `EndpointChanged` and `ConfigurationInvalid` — R-35's scope.
- Ruling 21 (D20): `operation list` lists a branch record's live child on its own row, as it lists a direct close's today (`read/status.rs` about 520-540, 705-737); D20's "children not listed on their own" and Baseline Fact 12 were wrong, a design correction for Task 15's D-1 — none for behaviour.
- Ruling 22: the poll's retirement check covers every record the fence holds, direct included, as R-17 says; ruling 26 supersedes ruling 24 and gives the direct case its reachable setup (a direct record at `published` with cleanup `cancelled` after `CancelAfterPrimary`, holding no reservation) — a direct poll can no longer retire a context under an unfinished direct close.
- Ruling 23: prepare answers `busy` for an unfinished synchronization of the ticket's context (a read of its unfinished remote rows, pull-request mode only), leaving out the unfinished record's own children so decision 33's `recovery_required` still names the holder — none.
- Ruling 25: a new request observes in the live mode; only a same-request continuation uses its record's mode; a supersedable direct record met by a pull-request prepare answers `ticket_closed` (the primary holds the closure), the old record untouched — none.
- A confirm's or step 3's `busy` stays `Repository(RepositoryBusy)` in the domain (mapping it would change the direct close's errors and drop the blocker) and is `busy` in the binding.
- Slice A, the record's path: commits `c7ff5cd5` and `b2f1b378` (base `350e1bb3`). Tests: `branch_records_rebuild_and_keep_direct_rows` (P7, with `new_branch` rows for `merged_context`), `written_closure_unfinished_covers_branch_records_by_step`, `child_fence_admits_the_publication_child_only_while_a_branch_record_publishes`; E-22 and E-30 applied; the §0.1 guards green. Review (Opus): `merged_context` gave the closure commit through a new branch's retirement; fixed by step (round 1). Ruling 27: a branch record's `validate` does not require `remote_name` at `abandoned`, defensively, with an accepted twin row.
- Slice B, observation and consent: commits `56a5fc1b`, `9163bc01`, `21ef048f`. Tests: `pull_request_prepare_decides_the_situation_and_its_blockers`, `pending_work_leaves_out_a_record_s_own_children_and_finds_the_record`, `pull_request_digest_wire_format`, `a_branch_record_begins_replays_and_is_refused_on_its_own_path`, and in `repository::closure::run_tests::pull_request` (file `closure/run_pull_request_tests.rs`): `d17_…`, `d18_…`, `a_new_request_observes_live_and_a_continuation_its_record_s_mode`, `a_foreign_context_synchronization_is_busy_at_prepare_and_confirm`; the 65 `observe::tests` guards green, `closure_digest_golden_vector`'s value unchanged. Review: situation 1 did not decide; fixed in two rounds. Ruling 28: the foreign-synchronization `busy` is read only after situation 1 and only with no unfinished non-supersedable record of the ticket. Ruling 29: a closed primary alone decides the pull-request prepare, exactly `[AlreadyClosed]`.
- Slice C, the runner and the publication child: commits `87393908`, `bcc8c6a0`. Tests in the `pull_request` module: `d1_…` to `d5_…`, `d7_…` to `d10_…`, `d12_…` to `d15_…`, `d20_…`, `d21_…`, `a_branch_record_s_own_publication_child_passes_the_fence` (E-23), `a_context_branch_gone_…`, `a_publication_that_published_nothing_…`, `an_index_pending_branch_answer_is_owed_and_offers_its_own_request`, `a_branch_answer_gives_its_code`, `a_branch_parent_view_is_never_integrated`, the hold unit test; 185 guards green. Review: seven run tests lacked the primary assertion, and `branch_reached` gave an owed answer with no way out; fixed (round 1). Ruling 30: an index-pending branch answer is `discovery_pending`, owed, with the same request as its way out, as the direct close's.
- Slice D, the stops and ends: commits `99355d18`, `e1e91e05`. Tests: `d6_…`, `d11_…`, `answer_table_chooses_each_row_in_order` (rows 1 to 9, each shown red by a mutation), `a_completed_publication_answers_by_its_child_under_a_changed_remote`, `a_retained_conflict_stands_as_the_stop_when_the_tracking_primary_holds_the_closure`, `a_retained_child_s_stop_stands_and_the_record_stays_unfinished`, `a_retained_context_child_s_end_stands_and_the_record_stays_confirmed`, `a_failure_after_a_stop_s_completion_answers_from_the_record`. A completed child is answered by its own row before the remote check (closer to Part D's "it stopped because the publication remote changed" than the checkpoint). Ruling 31: a retained child's stop stands at an end (its category saved, its conflict data answered, the record unfinished), since R-36 forbids discarding it.
- Slice E, the hold: commits `cbf62eab`, `726cf89e`. Tests: `d16_the_hold_refuses_every_writer_but_its_own_publication_child`, `item_resolve_history_passes_a_branch_record_s_hold` (ruling 19), `a_poll_does_not_retire_a_context_a_branch_record_holds`, `a_poll_does_not_retire_a_context_a_direct_closure_holds` (rulings 22, 26), `a_repair_checks_the_hold_again_under_the_lease`, `a_candidate_captured_before_the_record_held_is_refused_at_its_claim`. **E-43** (mechanical) `ticket_guard_tests.rs` `guard_for` gains the service parameter. Review: `context retire` did not name its holder, and the repair checked the hold outside the lease (a race); fixed (round 1). A test seam `LifecycleLeasePhase::RepairBegun` was added on the existing lease-hook pattern.
- Direct-mode change, as ruled (22, 26): a poll no longer retires a direct close's context while the close is unfinished.
- Notes for the final review: a retained clean candidate meeting `RemoteContextDeleted` repeatedly may have no way out (unverified); the R-35 arm covers only a record with its publication child; a cancelled request whose remote changed completes as row 5; `republish_reserved` has no R-30 check (Task 6).
- Not run: the fast tier and the SSH tier (review-ready, Task 16).
- Deferred to the final review: the `reentry` branch of `bind/retire.rs` (`lost_initial` returning `Err`) answers `refused(e)` without `naming`; `branch_hold_of` repeats a registry lookup; D16 omits the domain republish `Busy` assertion; a comment at the `let_go` inside a match guard in `promote_branch`.

## Task 8: Withdraw

- Design checkpoint (Obligation 5), written by an Opus designer before any code, checked by an independent Opus checker (sound with fixes), approved with the controller's corrections on 2026-11-25; it is Appendix E. Slices: A, the withdraw observation and consent; B, the withdraw lifecycle, hook and answers; C, reconciliation on `withdraw_oid`.
- Ruling 32: after the publication child, "the published head holds a closure" answers `ticket_closed` as Part E says (it is R-30's restored merge, which the tracking-primary row catches first), stored as its own `BranchAnswer` `closed_since` so its mapping can change cheaply — another clone's closure merged there would read `closing`, a rare case.
- Ruling 33: "the tracking primary holds the withdrawn closure" reads as any closure there — withdraw never reopens (R-27) — none.
- Ruling 34: the withdraw digest binds no mode, as Part E lists it — withdraw behaves the same in both modes (decision 11) — a mode switch between a withdraw's prepare and confirm is not `external_change`.
- Ruling 35: with an unfinished non-supersedable record of the ticket, the withdraw observation answers neither "nothing to withdraw" nor its foreign-synchronization `busy`, so the binding's hold answers `recovery_required` naming the holder (decision 33), as ruling 28 does for the close — none.
- Slice A, observation and consent: commit `0da47cba`. The `Verb` threaded through `observe_git`; `withdraw_observation` in Part E's order (no remote, then a closed primary alone, then the foreign-synchronization `busy` filtered by ruling 35, Part D's blockers, `context_missing`, `remote_context_deleted`, the closure from the context or else the tracking context, nothing to withdraw); `ClosureWithdrawalPreflight`; `withdrawn_ticket_bytes` with a round-trip test; the digest dispatched on the path (`withdraw_digest_wire_format`); W6's conflicting rows in `provisional_closures_may_change_primary_ones_may_not`. Review (Opus): approved; the ruling-35 placeholder closure cannot reach a preview a user consents to, a stored record or an unrefused confirm.
- Slice B, lifecycle, hook and answers: commit `9fbacee8`. `TicketWithdrawLifecycle`; `Lifecycle::continues` in both `begin_record`s, both `refuse_record`s and `run_observed`; `withdraw_step` (tip check, tracking primary closed, nothing to withdraw, malformed head, consent, then the commit with `withdraw_oid` stored before the ref moves); `withdrawal_answer`; four `BranchAnswer` values (`withdrawn`, `withdraw_closed`, `closed_since`, `already_applied`). Tests `w1_…` to `w5_…`, `w7_…` to `w12_…`, `w14_…`, the withdraw answer table, the cross-verb test (including a completed direct close) and the malformed head; 131 guards green. W9 holds in all three merge styles. Review (Opus): approved.
- Slice C, reconciliation: commit `dbdcf019`. A withdraw at `closing` reconciles on `withdraw_oid` and reuses the stored commit (it stores no time to rebuild with); the descendant rule for a withdraw requires an open ticket. Tests `w13_…` (crash points 2 to 4) and `reconcile_closing_reads_a_withdraw_by_its_commit`; a mutation that rebuilds the commit fails `w13`. Review (Opus): approved; checking the stored commit's parent and subject is enough (one writer, content addressing, the tip and consent proven again).
- Deferred to the final review: an optional open-ticket check in `stored_withdraw`; `w13` could assert one child; the abandon now reachable at `closing` with nothing written has no test; two of the cross-verb runs lacked primary checks (fixed in slice C); answer-table row 1 is not shown ahead of rows 2 and 5.
- For Task 11 (carried): the binding must apply decision 33's hold before showing a ready withdrawal; binding rows for W4 (`recovery_required` naming the holder) and W12 (`publication_remote_required`); `publication_remote_required` pre-empts the hold unless Task 11's configuration check runs first.

## Task 9: The New Branch

- Design checkpoint (Step 3), written by an Opus designer, checked by an independent Opus checker (sound with fixes), approved with the controller's corrections on 2026-11-25; it is Appendix F. U-1 does not stop for the owner: `finish_parent` (`remote/reservation.rs` about 1217-1249) is generalized by widening its SQL (about 1236) to a new branch's `retired` end, its checks unchanged (`finish_remote_operation` refuses non-poll, non-synchronization targets). Slices: A, prepare and consent (`unheld_entries`, situations 2 and 3, the digest's new fields, the preview, an interim guard); B, a new branch with nothing to retire (steps 1 and 3, the context-at-commit function); C, the retirement and U-1.
- **E-44** (controller-approved): replaces the two "until Task 9" rows Task 7 wrote in `closure/observe_tests.rs` (about 1797-1809) with the Task 9 situations.
- Rulings 36 to 49, in Appendix F's corrections and the designer's open questions: situation 2 gets 3.1's ancestor check (36), counting every push `push_absence` counts (45); F8's wording (37); refusals under the reservation release it as `cancelled` through `finish_parent` (38); the cleanup's code is `context_changed`, `recovery_required` (39); a kept closure's record stores the kept `closed_at`, and one not in whole-second UTC form blocks prepare with `TicketInvalidAfterClosure` (40); with no context, ambiguous or unknown push evidence blocks prepare with `history_unknown` or `history_unresolved` (41); a local branch with no worktree keeps `context_missing` (42); a dirty worktree at confirm answers `Blocked([ContextDirty])` (43); the digest binds `retired_oid`, 3.1 falls back to the deletion record's `last_published`, "occupied" includes a worktree registration, a cancel under the reservation stops at `retired` (44); a retry after this record's own tracking deletion accepts the bound evidence (46); a stale tracking-ref lock after a crash is a known limit with the operator's removal as its way out, as for the direct close (47, a Known Limit at Task 15); a configuration change during the retirement answers `recovery_required` until restored (48); a pull-request close that meets a deletion recorded between its context child and step 3 abandons with `external_change` before writing, and a new prepare offers the new branch (49, replacing row 3's answer for that window).

## Appendix A: References, Baseline Facts and Parts A to C (research agent, verbatim)

### Task 0 reference check, design sections Conventions, Baseline Facts 1-23, Parts A-C

Checked against the worktree at HEAD 72296bd0 (origin/main 3df52dcb plus documentation commits). Paths are relative to the worktree root. "existing" includes line drift under about 30 lines.

| Design place | Reference | Status | Current location | Note |
| --- | --- | --- | --- | --- |
| Conventions | `integration = "pull_request"`, `Integration::PullRequest` | existing | src/canonical.rs:91 | `RepositoryConfig::integration()` at :134 |
| Conventions | `row_rank`, `PRIMARY_COPY` | existing | src/repository/read/items.rs:1984, 1974 | non-active context gives PRIMARY_COPY |
| Conventions | `promotion/run.rs` `run_lifecycle`, `promote_inner`, `finish_steps`, `child()` | existing | src/repository/promotion/run.rs:93, 434, 1268, 1001 | |
| Conventions | `closure/` module, `ticket_closures` record | existing | src/repository/closure/record.rs:228 | |
| Fact 1 | `ClosureState { Open, Closed }` (`contract_enum!`) | existing | src/repository/read/dto.rs:338 | |
| Fact 1 | `ClosureDto { state, closed_at, closed_by }` and its doc | existing | src/repository/read/dto.rs:505-512 | doc text matches |
| Fact 1 | `items.rs` `closure()` derives state from `closed_at` alone | existing | src/repository/read/items.rs:1354 | |
| Fact 1 | `stored_item_dto` (lists), `parsed_item_dto` (show), `read_path` | existing | items.rs:1435, 1471, 2373 | `parsed_item_dto` called at :1940 |
| Fact 2 | `effective_rows`, `effective_copy` | existing | items.rs:2035, 2085 | |
| Fact 2 | `OWN_WORKTREE`, `PRIMARY_COPY` ranks | existing | items.rs:1973-1974 | `Unverified` is a non-active context so ranks PRIMARY_COPY |
| Fact 2 | `stored_items` and its callers | existing | items.rs:869; callers :2177, :2257, :2396; graph.rs:949; also comments.rs:268, status.rs:925 | design lists the items/graph ones only; accurate |
| Fact 2 | `graph.rs` moved-context substitution | existing | graph.rs:950-953 (`substitute_observed`) | |
| Fact 2 | `Targets::of`, `Targets::stored_node`, `Related::of` | existing | items.rs:301, 351, 370 | |
| Fact 2 | `TicketFilter::matches` incl. `--ready/--blocked` | existing | items.rs:81 | |
| Fact 2 | `docs/plans/2026-11-19-fast-writes-design.md` Table A | existing | docs/plans/2026-11-19-fast-writes-design.md:189 | |
| Fact 3 | `TicketNode { id, closed: bool, parent, deps }` | existing | graph.rs:54-61 | |
| Fact 3 | `blockers`, `is_ready`, `readiness_state`, `unplannable`, `batches`, `critical_path` | existing | graph.rs:239, 272, 282, 522, 555, 665 | |
| Fact 3 | tree line state / deps root state from `closed` | existing | graph.rs:493-497, 1103 | |
| Fact 3 | `ticket_readiness` keeps `closed_at.is_none()` rows | existing | graph.rs:1023-1029 | |
| Fact 3 | private `Target`, exhaustive match in `Related::check` | existing | items.rs:285, 411 (match ~:419-423) | |
| Fact 3 | batch numbering docs (`plan.schema.json`, graph.rs ~1167, dto.rs ~1063) | existing | schemas/v1/plan.schema.json; graph.rs:1170-1186; read/dto.rs:1063 | note: today a batch keeps its number under a filter (graph.rs:1183); R-9 renumbers, so it is a behaviour change, not a doc-only edit |
| Fact 4 | `ClosureFilter { Open, Closed, All }` | existing | items.rs:45 | |
| Fact 4 | parse.rs value parser `open\|closed\|all` | existing | src/bin/manyhands-cli/parse.rs:86-88 | |
| Fact 4 | dispatch.rs wildcard to `All` | existing | src/bin/manyhands-cli/dispatch.rs:58-62 | |
| Fact 4 | tests/cli/content.rs test of the filter mapping | existing | tests/cli/content.rs:352 (`every_ticket_filter_matches_the_same_f1_filter`) | |
| Fact 4 | `help_ticket_list.json`, `help_resource_ticket.json`, `help_root.json` pin `open\|closed\|all` | existing | tests/fixtures/cli_v1/help_ticket_list.json:39, help_resource_ticket.json:39, help_root.json:216 | the string also lives in src/bin/manyhands-cli/catalog.rs:175 (not cited by design) |
| Fact 4 | `catalog_tests.rs` (about 230) | existing | src/bin/manyhands-cli/catalog_tests.rs:230 | |
| Fact 4 | `parse_tests.rs` (about 408 and 433) | existing | src/bin/manyhands-cli/parse_tests.rs:408, 433 | |
| Fact 5 | `ClosureState` exhaustive match relationships.rs | existing | src/desktop/model/relationships.rs:336 | |
| Fact 5 | `ClosureState` exhaustive match tickets.rs | existing | src/desktop/model/tickets.rs:240 | |
| Fact 5 | `ClosureState` exhaustive match relations_edit.rs | existing | src/desktop/model/relations_edit.rs:327 | |
| Fact 5 | `ClosureState` exhaustive match journeys.rs | existing | tests/ticket_closure/journeys.rs:390 | |
| Fact 5 | `ClosureState` exhaustive match tickets_tests.rs | existing | src/desktop/model/tickets_tests.rs:136 | |
| Fact 5 | `DependencyState` matches relationships.rs | existing | src/desktop/model/relationships.rs:101, 298 | |
| Fact 5 | `DependencyState` match graph_tests.rs | existing | src/repository/read/graph_tests.rs:88 | |
| Fact 5 | `ReadinessReasonCode` match tickets.rs | existing | src/desktop/model/tickets.rs:159 | |
| Fact 5 | `ClosureFilter` matches filter_bar.rs, list_view.rs, ticket_view.rs | existing | src/desktop/view/tickets/filter_bar.rs:47-94; src/desktop/model/list_view.rs:162-167; src/desktop/model/ticket_view.rs:104, 231 | filter_bar's `SEGMENTS: [_; 3]` array also needs care |
| Fact 5 | `PreviewDetailsDto` matches human_admin.rs, confirm.rs | existing | src/bin/manyhands-cli/human_admin.rs:98; src/desktop/view/confirm.rs:~649-772 | |
| Fact 5 | `PreviewEffect` `effect_phrase` | existing | human_admin.rs:165 | |
| Fact 5 | `LifecycleBlocker` `blocker_words` | existing | src/desktop/view/lifecycle/prepare.rs:47 | |
| Fact 5 | `MutationDataDto` `authority`, tests/support/mutation.rs (six), repair.rs (one) | existing | src/repository/mutation/dto.rs:2441; tests/support/mutation.rs:674, 706, 842 and three more; tests/support/repair.rs:335 | six and one confirmed by count |
| Fact 5 | "Seven checks compare `== ClosureState::Closed`" | existing | bind/ticket.rs:241, drafts/compare.rs:234, desktop/model/edit.rs:77, desktop/view/item/header.rs:142, item/pill.rs:104, item/sheet/facts.rs:72, human_local_sync.rs:69 | exactly seven in non-test code |
| Fact 5 | `bind/ticket.rs` `refuse_closed` | existing | src/repository/mutation/bind/ticket.rs:237 | |
| Fact 5 | `drafts/compare.rs` `lifecycle_closed` | existing | src/drafts/compare.rs:231 | |
| Fact 5 | `desktop/model/edit.rs` `edit_offered` | existing | src/desktop/model/edit.rs:70 | |
| Fact 6 | `ticket_guard.rs` `require_ticket_open(repo, primary_branch, item)` reads both tips | existing | src/repository/ticket_guard.rs:18 | refuses `TicketClosed` on a closure key in either tip |
| Fact 6 | `refuse_closed_ticket` and its three users | existing | src/repository.rs:299; callers :2392 (`prepare_context`), :3097 (`save_ticket_with_effective_config`), :3481 (`checkpoint_comment_locally_with`) | |
| Fact 6 | `context_release.rs` caller | existing | src/repository/context_release.rs:231 | after the unfinished-closure refusal at :221 |
| Fact 6 | `bind/republish.rs`, `bind/resolve_history.rs` callers | existing | republish.rs:202; resolve_history.rs:226 | `configuration` is `canonical::RepositoryConfig`, so `integration()` is readable |
| Fact 6 | `context retire`, `ticket repair` read neither guard nor `ticket_closures` | existing | no hit in context_retirement/ or bind/repair.rs | |
| Fact 6 | `draft_of`, `holds` carry `closed_at/closed_by` through | existing | bind/ticket.rs:1008 (`draft_of`), :296 (`holds`) | comment "The save carries these through from the file" |
| Fact 7 | `preserves_item_invariants` | existing | src/repository/integration/validate.rs:199 | |
| Fact 7 | `validates_resolution_sides` | existing | validate.rs:224, 251 | |
| Fact 7 | `validates_external_integration` | existing | validate.rs:505, 577 | |
| Fact 7 | remote callers sync.rs (3317, 3434, 4898, 5304), conflict_read.rs 768 | existing | sync.rs:3317, 3434, 4898, 5304; remote/conflict_read.rs:768 | local_sync/run.rs:714, 794 and local_sync/resolve.rs:446, 615, 636, 993 also call them (not cited; relevant to "local synchronization gives none") |
| Fact 7 | `Recorded` with step window number | existing | conflict_read.rs:324; `step.window_number` used at :503 | |
| Fact 7 | `state::integration_window` | existing | src/repository/remote/state.rs:4148 | |
| Fact 7 | window 0 has no `primary_oid` | existing | remote/state.rs:3264-3269, 4173 | |
| Fact 7 | tests sync_tests.rs about 11008 and 11622 | existing | remote/sync_tests.rs:11008, 11622 | tests pin closure sides in resolutions; names do not say "primary-target" |
| Fact 7 | tests local_sync/conflict_tests.rs about 716 and 780 | existing | local_sync/conflict_tests.rs:716, 780 | |
| Fact 7 | `IncompatibleClosures`, `resolve_synchronization` refusal | existing | integration/validate.rs:32; sync.rs:5258, 5280-5284 | refusal is on `eligibility != EligibleCanonical` |
| Fact 7 | `PreparedMerge` private to `integration/prepare.rs` | existing | integration/prepare.rs:50 | `pub(in super::super)` struct with private field |
| Fact 7 | `integrate_divergence`, `DivergenceInputs` incl. primary | existing | sync.rs:3616, 3574 (`primary: git2::Oid` field) | |
| Fact 7 | sync.rs 3712-3718, 3786-3860 clean merge; 3721-3745 no merge | existing | sync.rs:3705-3760 | |
| Fact 7 | `integration/tests.rs` | existing | src/repository/integration/tests.rs | |
| Fact 8 | `LifecycleMutationDto` doc (no `closed_by`) | existing | src/repository/mutation/dto.rs:1189-1194 | cited as "dto.rs about 1190"; read/dto.rs is a different file |
| Fact 8 | `tests/cli/lifecycle.rs` `application_data_holds_no_closed_by_or_title_beyond_the_index` | existing | tests/cli/lifecycle.rs:923 | |
| Fact 8 | `closure/record.rs` `supersedable` | existing | record.rs:460 | |
| Fact 8 | `begin_record` busy unless supersedable | existing | record.rs:474-491 | |
| Fact 8 | `run.rs` `abandon`, uses at 700, 734, 750, 787 | existing | run.rs:259; calls :700, :734, :750, :787 | |
| Fact 8 | `lifecycle_record.rs` `request_cancellation` | existing | lifecycle_record.rs:1027 | |
| Fact 8 | cancel flag cleared by same request (run.rs 584-590) | existing | run.rs:582-590 | |
| Fact 9 | `Lifecycle` trait with `prepare_integration`, `write_commit`, `apply`, `pre_integration_with`, `resume_written` | existing | promotion/lifecycle.rs:351, 494, 502, 509, 522, 536 | |
| Fact 9 | `promote_inner` (about 434-870) | existing | promotion/run.rs:434 | |
| Fact 9 | `finish_steps` (about 1268-1728) | existing | run.rs:1268 | |
| Fact 9 | `PreparedIntegration` concrete struct (`integrate.rs` 69-87) | existing | promotion/integrate.rs:69 | |
| Fact 9 | closure hook `closure/checkpoint.rs` `pre_integration`, needs lease and `context_synchronization_unfinished` false | existing | checkpoint.rs:601, 649, 681 | |
| Fact 9 | `resume_written` / `reconcile_closing` / `NothingWritten`, `CheckoutDone`, `CheckpointWritten` | existing | checkpoint.rs:728, 390, 376, 380, 383 | |
| Fact 9 | `TicketClosureOutcome.integration/.cleanup` not optional | existing | closure/mod.rs:245-251 | |
| Fact 9 | `make_outcome` expects integration or base OID | existing | run.rs:1898-1905 | |
| Fact 9 | `run_observed` fails without `integration_oid` | existing | run.rs:342, 370 | returns the original error after `index_children_owed` |
| Fact 9 | binding `reached_effects`, `reached_code`, `reached_data`, `parent_integrated`, `parent_effects`, `made_ready` | existing | bind/lifecycle.rs:345, 291, 501, 1165, 1206, 456 | |
| Fact 10 | `ChildRole { Context, Primary }` | existing | promotion/lifecycle_record.rs:451 | |
| Fact 10 | record child slots `context_attempt`/`context_sync_id`, `primary_attempt`/`primary_sync_id` | existing | lifecycle_record.rs:266-267, 468-469; closure/record.rs schema | |
| Fact 10 | `child_allowed` | existing | lifecycle_record.rs:457-465 | matches the Confirmed / Integrated-or-PublicationPending rule |
| Fact 10 | `ChildExecution`, `validate_request`, `validate` | existing | lifecycle_record.rs:520, 554, 573 | |
| Fact 10 | `child()`, `release_child`, `next_attempt` | existing | run.rs:1001, 988; lifecycle_record.rs:648 | |
| Fact 10 | `synchronization_target` | existing | run.rs:1130 | |
| Fact 10 | `remote_identity_of` | existing | lifecycle_record.rs:761 (not run.rs) | design lists it in a run.rs-adjacent sentence; the file is lifecycle_record.rs, "else `SynchronizePrimary`" confirmed at :771-773 |
| Fact 10 | `children_contexts`, `primary_authority`, `remember_publication` | existing | run.rs:1877, 1209, 1190 | |
| Fact 10 | `lifecycle_context_child`, `lifecycle_parent_of` and callers | existing | remote/state.rs:3828, 3860; conflict_read.rs:445; bind/conflict.rs:196; mutation/records.rs:845 | |
| Fact 11 | `written_closure_unfinished` true for `closing` through `branch_deleted` | existing | closure/record.rs:774 | SQL step list matches |
| Fact 11 | `closure_holds_context`, callers (about 1618, 3211) | existing | remote/reservation.rs:427, 1618, 3211 | |
| Fact 11 | poll.rs about 389 and 668 | existing | remote/poll.rs:389, 668 | |
| Fact 11 | `context_synchronization_unfinished` | existing | closure/record.rs:788 | |
| Fact 11 | `local_sync/record.rs` `admit` | existing | local_sync/record.rs:1144 | refuses all except own ticket at `Closing` with `Record` kind |
| Fact 12 | `read/status.rs` `closure_operations`, hiding of parent's own remote row | existing | read/status.rs:639, 712-729 | `resumable` at :673 |
| Fact 12 | `recovery.rs` `attribute_remote_row` | existing | src/repository/recovery.rs:1154 | |
| Fact 12 | `reservation.rs` `abandonable` | existing | remote/reservation.rs:782 | design cites 801-812; body is within |
| Fact 12 | `local_sync/record.rs` `FOREIGN_IDENTITIES` | existing | local_sync/record.rs:1058 | |
| Fact 12 | `registration_removal.rs` (210-225) | existing | registration_removal.rs:210-226 | |
| Fact 12 | `lifecycle_record.rs` `mint_attempt`, `bindings`, `child_repository`, `audit`, `request_cancellation` | existing | lifecycle_record.rs:671, 312, 840, 880, 1027 | |
| Fact 12 | `bind/resume.rs` `found_lifecycle`, `original_request_required` | existing | bind/resume.rs:291, 36 | |
| Fact 12 | `progress.rs` `asks_a_parent`, `cancel_parent` | existing | src/repository/mutation/progress.rs:631, 641 | file is under repository/mutation/, not the CLI |
| Fact 12 | desktop `lifecycle_lock.rs` (about 109) | existing | src/desktop/model/lifecycle_lock.rs:104-111 (`step_locks`) | locks every step but confirmed and context_synchronized |
| Fact 13 | `ticket_closures` schema, `step` CHECK, `closure_kind`, unique index `ticket_closures_unfinished` | existing | closure/record.rs:229-267 | step list matches exactly |
| Fact 13 | `ClosureStep::permits` | existing | record.rs:64 | |
| Fact 13 | `save_record`, `validate`, `complete_record` | existing | record.rs:545, 365, 746 | |
| Fact 13 | `finish_parent` | existing | remote/reservation.rs:1204 | |
| Fact 13 | `validate_schema_object` | existing | remote/state.rs:1632 | |
| Fact 13 | marker `closure_refusal_blockers_v2` rebuild with `INSERT ... SELECT *` | existing | record.rs:269, 272, 304 | |
| Fact 13 | record_tests.rs tests (six names) | existing | record_tests.rs:140, 250, 324, 91, 1324, 830 | all six exist |
| Fact 13 | `tests/recovery_foundation_gate.rs` marker list | existing | recovery_foundation_gate.rs:1134, 1138 | |
| Fact 13 | `tests/ticket_closure.rs` privacy scan (about 446) | existing | tests/ticket_closure.rs:446 | |
| Fact 14 | `closure/observe.rs` `expectation` domain `manyhands closure v1` | existing | observe.rs:1004, 38 | |
| Fact 14 | `observe_git` | existing | observe.rs:567 | |
| Fact 14 | `ClosureExpectation` redacted `Debug` | existing | closure/mod.rs:65-77 | |
| Fact 14 | `TicketCloseInput` holds root and ID; resume rebuild | existing | bind/lifecycle_verb.rs:44; bind/resume.rs:1314 | |
| Fact 14 | `bind/lifecycle.rs` `digest_of` | existing | bind/lifecycle.rs:192 | |
| Fact 15 | `synchronize_remote_observed`, `synchronize_promotion_child_observed` | existing | remote/sync.rs:6171, 6182 | |
| Fact 15 | `refs.rs` `context_fetch_refspec` (fetches primary too) | existing | remote/refs.rs:91 | fetch refspecs are `+` forced; push refspecs (:99, :103) are unforced |
| Fact 15 | `never_gets_past` (`RemoteBranchDeleted`, `MergeRequired`) | existing | mutation/outcome.rs:1083 | |
| Fact 15 | `RemoteContextDeleted` with no tracking ref and `ObservedPublished` | existing | remote/refs.rs:262-268 | |
| Fact 15 | `WorktreeConflicted` on merge/rebase/cherry-pick in progress (about 745-757) | MOVED | remote/sync.rs:741-770 (`require_clean_target_state`) | not in refs.rs; the sentence reads as if the line range continued refs.rs |
| Fact 15 | `promotion_test_drive_local_bound` | existing | remote/sync.rs:3928 (`cfg(test)`) | |
| Fact 16 | `transport/remote.rs` `delete_exact`, used only by cleanup | existing | transport/remote.rs:269; sole caller promotion/run.rs:165 | reservation under the parent is at run.rs:1392-1449 |
| Fact 16 | poll records `remotely_deleted` | existing | remote/state.rs:3217 | |
| Fact 16 | `poll.rs` `primary_holds` | existing | remote/poll.rs:789 | |
| Fact 17 | `ContextCleanup` and methods (`acquire`, `full_check`, `remove_context_worktree`, `remove_fresh_context_worktree`, `delete_context_branch`, `require_local_cleanup_absent`, `delete_tracking_ref`) | existing | cleanup.rs:102, 138, 369, 334, 352, 477, 540, 572 | |
| Fact 17 | `retire_context` (about 953) | existing | cleanup.rs:953 | upserts `retired_contexts` at :1017; sole caller promotion/lifecycle_record.rs:972 |
| Fact 17 | `context_retirement/record.rs` `finalize_index` | existing | record.rs:1966; callers request_scope.rs:482, repository.rs:7662 | it does not call `cleanup::retire_context` today (see Part C row) |
| Fact 17 | `retired_contexts` fresh-publication logic (state.rs 2610-2632, sync.rs 7963-8052) | existing | remote/state.rs:2605-2635; sync.rs:7962-8054 | |
| Fact 18 | `prepare_context_unlocked` | existing | src/repository.rs:2432 | |
| Fact 18 | `mark_contexts_dirty` | existing | repository.rs:12915 | |
| Fact 19 | `context_retirement/proof.rs` `preservation` | existing | proof.rs:24 | `TicketOpen` comes from `completed_item` (:156, :206) after the shallow check and tree reads, so "first" is loose |
| Fact 19 | Folded loop exception for ticket's own open file | existing | proof.rs:63-91, `older_open_ticket` :212 | |
| Fact 19 | `context_retirement/tests.rs` | existing | context_retirement/tests.rs | |
| Fact 20 | `Outcome::of_mutation` | existing | src/results.rs:718 | |
| Fact 20 | `results_tests.rs` `DESIGN_CODES` (71, `already_applied` last) | existing | src/results_tests.rs:34, 105 | |
| Fact 20 | `envelope.schema.json`, `request.schema.json` code lists | existing | schemas/v1/envelope.schema.json, request.schema.json | each lists `already_applied` once |
| Fact 20 | `recovery` matches every code | existing | mutation/outcome.rs:778 | |
| Fact 20 | `from_stored` lifecycle arm before untagged host arm | existing | mutation/dto.rs:2564, ~2584-2590 | |
| Fact 20 | `PreviewEffect` 37 values incl. `context_created` | existing | mutation/dto.rs:160 | 37 counted |
| Fact 20 | `PreviewDetailsDto` untagged | existing | mutation/dto.rs:246-248 | |
| Fact 20 | `LifecycleBlocker` 24 values | existing | mutation/dto.rs:352 | 24 counted |
| Fact 20 | `published_enumerations_list_exactly_the_registered_values` | existing | tests/read_contract.rs:1594 | |
| Fact 20 | `tests/cli/inventory.rs` 44 mutating, 81 catalogued, schema-file check, labels C1-C5 | existing | inventory.rs:394, 412, 524-537, 421 | |
| Fact 20 | `catalog_tests.rs` `DESIGN` table | existing | catalog_tests.rs:6 | |
| Fact 20 | `conformance.rs` `HOMES` (46), `run_part`, `Expect::Exempt` | existing | tests/support/conformance.rs:59, 370, 163 | |
| Fact 20 | `mutation_contract.rs` `every_bound_command_has_a_golden` | existing | tests/mutation_contract.rs:1587 | |
| Fact 20 | `mutation_golden.rs` `CASES` | existing | tests/support/mutation_golden.rs:21 | re-exported via tests/support/mutation.rs:39 |
| Fact 20 | `every_schema_is_reachable_from_a_published_root` | existing | mutation_contract.rs:1737 | |
| Fact 20 | `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root`, `DATA_ROOTS` (69) | existing | mutation_contract.rs:1781, 1603 | `lifecycle_preview.schema.json` at :1643 |
| Fact 20 | `tests/cli/skills.rs` `every_mutation_sits_in_one_autonomy_column` | existing | skills.rs:459 | |
| Fact 20 | `tests/support/cli.rs` `CLI_CASES` | existing | support/cli.rs:217 | |
| Fact 20 | `tests/repository_enablement.rs` registry table list | existing | repository_enablement.rs:581-650 | `ticket_closures` is last entry |
| Fact 20 | `tests/cli/recovery_tables.rs` `LIFECYCLE`, `FAMILIES`, `C5_REQUIRED`, `c5_codes()` (`verbs.len() == 10`) | existing | recovery_tables.rs:73, 83, 96, 466, 519 | |
| Fact 21 | `forge.rs` `recognise`, `pull_request_link`, `human_line` | existing | src/forge.rs:78, 146, 122 | |
| Fact 21 | `PullRequestLink` not `Serialize`, unused outside forge.rs | existing | forge.rs:113 (`derive(Clone, Debug, PartialEq, Eq)`) | no `forge::` or `PullRequestLink` use elsewhere in src/ or tests/ |
| Fact 21 | SSH harness URL gives no URL without `[forge]` template | existing | tests/support/ssh_remote.rs:539 (`ssh://fixture@...`); forge.rs `recognise` hosts only the three cloud hosts | |
| Fact 22 | `PromotionIo`, `run_with_io` | existing | promotion/run.rs:21, 323 | `PromotionIo` is a trait |
| Fact 22 | `run_tests.rs` `FakeIo`, `reserve_promotion_child` | existing | promotion/run_tests.rs:357, 391; remote/reservation.rs:1429 | |
| Fact 22 | `context_retirement/acceptance_tests.rs` (313-360), `poll::synthetic_fetch` | existing | acceptance_tests.rs:362, 399; remote/poll.rs:1435 | |
| Fact 22 | `closure/run_tests.rs` `exit_at` | existing | run_tests.rs:1151 | |
| Fact 22 | sync_tests.rs about 4994 window pinning | existing | remote/sync_tests.rs:4980 (`append_window`) | |
| Fact 23 | `discovery.rs` `migrate_registry`, `migrate_pull_request_mode`, `wave_06_pull_request_mode_v1` | existing | discovery.rs:2558, 2809, 2761 | |
| Fact 23 | `closure/record.rs` `migrate` called in registry-schema transaction | existing | record.rs:272; discovery.rs:2750 | |
| Part A | `ClosureState::Closing` | new | absent | |
| Part A | `DependencyState::Closing` | new | absent | `DependencyState` exists at read/dto.rs:~346 with Open, Closed, Unresolved |
| Part A | `ReadinessReasonCode::ClosingDependency` / `closing_dependency` | new | absent | `ReadinessReasonCode` at read/dto.rs:365 holds three values |
| Part A | `ClosureFilter::Closing` | new | absent | |
| Part A | schema enums `item.schema.json` `closure.state`, `parent.state`; `dependency.schema.json`; `dependency_tree_node.schema.json`; `readiness_reason.schema.json` | existing | schemas/v1/item.schema.json:64, 88; dependency.schema.json:13; dependency_tree_node.schema.json:14; readiness_reason.schema.json:11 | |
| Part A | doc sites (`ClosureDto`, `ReadinessState`, `DependencyDto`, `ReadinessDto`, items.rs ~42, graph.rs ~56 and ~293, item.schema.json ~99) | existing | read/dto.rs:505, 356, 519, 530; items.rs:42; graph.rs:56, 280-282; item.schema.json:99 | |
| Part A | `TicketNode.closing` | new | absent | `closed` exists at graph.rs:57 |
| Part A | `ticket ready`, `ticket blocked`, `ticket list --ready/--blocked`, `ticket plan`, `ticket critical-path` | existing | cli catalog.rs:348, 351, 379, 386; read/graph.rs:1180 (`ticket_plan`), :1231 (`ticket_critical_path`) | |
| Part A | `tickets_made_ready` unchanged | existing | graph.rs:981 | |
| Part A | CLI `open\|closed\|closing\|all` and explicit `closing` mapping | new | parse.rs:86-88, dispatch.rs:58-62 are the edit points | |
| Part A | `item_line`, `item_show`, `dependency_line` print `as_str()` | existing | cli/human.rs:300, 330, 457 | |
| Part A | `ClosureDto.publication`, `Publication: pending/published` line | new | absent | read/dto.rs has no `publication` field; mutation/dto.rs:1205 has an unrelated `LifecycleMutationDto.publication` |
| Part A | desktop arms (`Lifecycle::Open` mapping) | existing | desktop/model/tickets.rs:241, relationships.rs:337, relations_edit.rs:328 | `Lifecycle` enum exists |
| Part A | `already_closing` | new | absent | `AlreadyClosed` blocker exists (mutation/dto.rs:~372) |
| Part B | `require_ticket_open` gains integration parameter | new | ticket_guard.rs:18 has no mode parameter today | |
| Part B | `refuse_closed_ticket`, `bind/republish.rs`, `bind/resolve_history.rs`, `context_release.rs` pass mode | existing | repository.rs:299; republish.rs:202; resolve_history.rs:226; context_release.rs:231 | |
| Part B | `ticket save`, slug assignment, context prepare, `comment add`, `item republish`, `item resolve-history` writes | existing | repository.rs:2392, 3097, 3481; bind/republish.rs; bind/resolve_history.rs | slug assign (`TicketSlugAssignInput`) goes through the save path |
| Part B | `bind/ticket.rs` `refuse_closed` | existing | bind/ticket.rs:237 | |
| Part B | desktop "Close ticket" action compares `Closed` | existing | desktop/view/item/header.rs:142 | |
| Part B | save check (R-11) in the domain save | new | absent | `ticket_guard` and `draft_of` carry closure fields through |
| Part B | `validates_resolution_sides`, `validates_external_integration` optional primary-side commit | new | validate.rs:224, 505 have no such parameter | |
| Part B | `classify_conflict_entry` | existing | integration/validate.rs:59; remote/sync.rs:4544 wrapper | |
| Part B | `remote/conflict_read.rs` reads window's `primary_oid` from `Recorded` | new | conflict_read.rs:324 (`Recorded`), state.rs:4148 (`integration_window`) | |
| Part B | integration `Context`/`Primary` stage; step's incoming commit | existing | remote/merge.rs:15 | |
| Part B | `integration/prepare.rs` clean merge given the closure's fields | new | prepare.rs:60 (`prepare_clean_merge`) | |
| Part B | `evidence.primary_tracking_oid` | existing | remote/state.rs:2103; sync.rs:6469, 6486, 6594 | |
| Part B | "push decision, reached from both the merging path and the plan path" | UNCLEAR | sync.rs:4178 (`pull_only_push_decision`), :7114 | no single symbol named "push decision"; Task 0 / plan must pick the exact function(s) |
| Part B | `ticket_closed` result code | existing | results.rs (`TicketClosed`); repository.rs:1356 `RepositoryErrorKind::TicketClosed` | |
| Part C | `PullRequestDto`, `schemas/v1/pull_request.schema.json` ("Cycle 02's") | new | absent at this HEAD | design says Task 0 checks origin/main; none present, so this Cycle adds them (R-26) |
| Part C | `forge` kind values `bitbucket-cloud`, `bitbucket-data-center`, `github`, `gitlab` | existing | src/canonical.rs:106-120 (`ForgeKind::as_str`) | |
| Part C | `contract_enumerations` | existing | tests/read_contract.rs:724 | |
| Part C | `forge::recognise`, `forge::pull_request_link`, configuration `forge()` | existing | forge.rs:78, 146; canonical.rs:139 | |
| Part C | `pull_request_links` table | new | absent | |
| Part C | marker `wave_06_pull_request_links_v1` migration after `migrate_pull_request_mode` | new | marker absent; hook point discovery.rs:2757 | |
| Part C | `cleanup.rs` `retire_context` as helper call site | existing | cleanup.rs:953 | called only by promotion/lifecycle_record.rs:972 today |
| Part C | `context_retirement/record.rs` `finalize_index` retiring/preserving branches | existing | record.rs:1966-1972 (preserving: `is_preserving` -> `preserve::finalize`) | retiring branch is the remainder of the function |
| Part C | "a poll's retirement, `context retire` and the refresh path" finish in `finalize_index` | existing | request_scope.rs:482; repository.rs:7662 | |
| Part C | `ItemDto.pull_request` after `readiness` | new | ItemDto at read/dto.rs:451; `readiness` :480, next field :486 | |
| Part C | `ticket show`, `ticket show --path` | existing | items.rs:2373 (`read_path`) | |
| Part C | `item.schema.json` required list | existing | schemas/v1/item.schema.json:7-20 | |

## Findings

No row is WRONG. Names the design says are new (`ClosureState::Closing`, `DependencyState::Closing`, `ClosingDependency`, `ClosureFilter::Closing`, `already_closing`, `PullRequestDto`, `pull_request.schema.json`, `pull_request_links`, `wave_06_pull_request_links_v1`, `ClosureDto.publication`, `ItemDto.pull_request`, `TicketNode.closing`) are all absent, so Cycle 02's shapes are not on this HEAD and this Cycle adds them (R-26).

- MOVED, Fact 15: the `WorktreeConflicted` refusal while a merge, rebase or cherry-pick is in progress ("about 745-757") is in `src/repository/remote/sync.rs:741-770` (`require_clean_target_state`), not `refs.rs`. The `RemoteContextDeleted` half of the same sentence is correctly `remote/refs.rs:262-268`.
- UNCLEAR, Part B R-30: "in the push decision, reached from both the merging path and the plan path" names no symbol. The nearest is `pull_only_push_decision` (`remote/sync.rs:4178`, called at :7114), which is the pull-only guard. The implementer must locate the actual place a context head is chosen for push (the `evidence.primary_tracking_oid` uses at sync.rs:6469-6594 and 6954 feed the merge path).

Notes that are not discrepancies but affect the plan:

- Part A R-9 renumbering is a behaviour change: today `ticket_plan` keeps each batch's number under a filter and the docs say so (`src/repository/read/graph.rs:1183-1186`, `src/repository/read/dto.rs:1061-1063`).
- Fact 10 `remote_identity_of` is in `promotion/lifecycle_record.rs:761`, and Fact 12 `asks_a_parent`/`cancel_parent` are in `src/repository/mutation/progress.rs:631,641`, not under the CLI; both are consistent with the design's file labels read in context.
- Fact 7 lists only remote callers of `validates_resolution_sides`/`validates_external_integration`; `local_sync/run.rs:714,794` and `local_sync/resolve.rs:446,615,636,993` also call them. Making the new primary-side argument optional keeps them unchanged, as the design intends.
- Fact 17: `cleanup::retire_context` has one caller today (`promotion/lifecycle_record.rs:972`); `context_retirement` paths do not call it, so the Part C "both retirement paths call the delete helper" is new wiring in `finalize_index`.
- Fact 19: `TicketOpen` is raised from `completed_item` after the shallow-history check and the tree reads, so it is "first" among content refusals, not literally first.
- The word `closing` already names a `ClosureStep` (`closure/record.rs:31`) and the `closure_hold`/`written_closure_unfinished` SQL step list; the new `ClosureState::Closing` is a different enum with the same string.

## Appendix B: References, Parts D to H, obligations and rulings (research agent, verbatim)

### Task 0 reference check B: design Parts D to H, Obligations, Rulings

Design: `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-design.md`. Checked against HEAD 1040f41a (working tree). Also covered: the Required Outcomes, Testing and Named Edits sections, where they cite code (they sit between Part H and the Rulings). "bind/" in the design means `src/repository/mutation/bind/` (there is no `src/repository/bind/`). Statuses: existing, new (absent now, as the design says), WRONG, MOVED, UNCLEAR.

| Design place | Reference | Status | Current location | Note |
| --- | --- | --- | --- | --- |
| D R-1 record | `ticket_closures` table | existing | src/repository/closure/record.rs:229 (SCHEMA), TABLE const | Unique unfinished index `ticket_closures_unfinished` at :267 matches "one unfinished record per ticket". |
| D R-1 record | `closure_refusal_blockers_v2` marker/rebuild | existing | src/repository/closure/record.rs:269, rebuild :300-304 | Today's rebuild interpolates the live `SCHEMA` const and copies with `INSERT ... SELECT *`. "Frozen DDL" and "explicit column list" are therefore new work (Obligation 3), not current state. |
| D R-1 record | `wave_06_closure_modes_v1` marker | new | none | No hit in src/tests. Sibling `wave_06_pull_request_mode_v1` is at src/repository/discovery.rs:2761 (separate migration run after the registry-schema transaction). |
| D R-1 record | marker "inserted on the path that creates the table" | existing | src/repository/closure/record.rs:272-312 | `migrate` already inserts MARKER then BLOCKERS_MARKER on the `created` path. |
| D R-1 record | column `integration_mode` | new | none | Values `direct`/`pull_request` match `canonical::Integration::as_str` (src/canonical.rs:93-100, Obligation 11). |
| D R-1 record | column `action` | new | none | Not in SCHEMA. |
| D R-1 record | column `withdraw_oid` | new | none | |
| D R-1 record | column `retired_oid` (on `ticket_closures`) | new | none | The name already exists on `retired_contexts` (src/repository/cleanup.rs:116, :630); no collision with `ticket_closures`. |
| D R-1 record | column `pending_ref` | new | none | |
| D R-1 record | column `answer_code` | new | none | |
| D R-1 record | ref namespace `refs/manyhands/closures/<operation ULID>` | new | none | No `refs/manyhands` use anywhere in src. |
| D R-1 record | per-action CHECKs on `closing` | existing | src/repository/closure/record.rs:229-265 | Today's CHECKs: `closed_at IS NULL OR closure_kind IS 'record'`, `(closed_at IS NULL)=(closure_oid IS NULL)`, `step!='closing' OR (kind record AND closed_at AND base_oid AND context_oid)`. A kind `existing` record can never be at `closing` now, so the design's `existing` at `closing` is a change. |
| D R-1 record | `closure_kind` / `ClosureKind` (record, existing) | existing | src/repository/closure/record.rs:37 | |
| D R-1 record | `base_oid` field | existing | src/repository/closure/record.rs:167 (struct) | "Tracking primary the hook read" is new semantics (today the direct hook stores the integration base). |
| D R-1 record | `closed_at` "frozen in the record before the commit (C6)" | UNCLEAR | src/repository/closure/record.rs:595 (`begin_closing`), checkpoint.rs:670 | Today the commit object is built first, then one UPDATE stores `closed_at` and `closure_oid` together (CHECK forbids one without the other). A `closed_at` stored strictly before the commit would need that CHECK pair relaxed. Step 3.3 of "Confirm" says "stored ... closure_oid stored" as if two writes. |
| D R-1 record | `closure_oid` | existing | src/repository/closure/record.rs:167 | |
| D R-2 steps | `ClosureStep` names (confirmed, context_synchronized, closing, published, publication_pending, complete, abandoned, worktree_removing, worktree_removed, branch_deleted) | existing | src/repository/closure/record.rs:30-36 | All exist. `Closing` maps to `ParentStep::ContextSynchronized` (record.rs:~95). |
| D R-2 steps | step `retired` | new | none | |
| D R-2 steps | `ClosureStep::permits` | existing | src/repository/closure/record.rs:64 | Signature `(self, next, remote, kind)`. Today it already allows `Closing -> Abandoned`; the design's "abandoned reachable from confirmed, context_synchronized ... (as today, plus that one)" omits `closing`. No `Confirmed -> WorktreeRemoving` or `Closing -> Published` edge exists yet (new). |
| D R-2 steps | `validate` | existing | src/repository/closure/record.rs:365 | A free function `validate(&ClosureRecord)`, not a `ClosureStep` method. |
| D R-2 steps | completion function for branch record | new | existing `complete_record` at src/repository/closure/record.rs:746 -> promotion/lifecycle_record.rs:931 | Existing one requires `step == BranchDeleted` and retirement evidence, so a new one is needed as the design says. |
| D R-2 steps | `supersedable()` | existing | src/repository/closure/record.rs:460 (fn), :814 (impl), promotion/lifecycle_record.rs:271 (trait) | Today false for `Closing`/`Integrating`, else the cleanup category decides. |
| D R-2 steps | `PublicationCategory` shared with `document_promotions` | existing | src/repository/promotion/record.rs (imported at closure/record.rs:19) | Both records use `promotion::record::PublicationCategory`. |
| D R-2 steps | `document_promotions` table | existing | src/repository/promotion/record.rs | |
| D R-3 children | `ChildRole::Publication` | new | `ChildRole` at src/repository/promotion/lifecycle_record.rs:451 has `Context`, `Primary` only | |
| D R-3 children | `ParentFacts` | existing | src/repository/promotion/lifecycle_record.rs:475 | Holds `ParentStep`, no closure step and no path yet (new). |
| D R-3 children | `child_allowed` | existing | src/repository/promotion/lifecycle_record.rs:457 | Context only from `Confirmed`; Primary from `Integrated`/`PublicationPending`. |
| D R-3 children | `primary_attempt`, `primary_sync_id` | existing | closure/record.rs:167, SCHEMA :229 | |
| D R-3 children | `validate_request` | existing | src/repository/promotion/lifecycle_record.rs:554 | A method of `ChildExecution`; matches `(Primary, Primary)` and `(Context, Context)` only. |
| D R-3 children | `synchronization_target` | existing | src/repository/promotion/run.rs:1130 | Role-based (kind, role, item). A second, unrelated `synchronization_target(&RemoteOperationTarget)` is at remote/state.rs:2177; the design means the run.rs one. |
| D R-3 children | `remote_identity_of` | existing | src/repository/promotion/lifecycle_record.rs:761 | Maps the second slot to `SynchronizePrimary`. |
| D R-3 children | `children_contexts` | existing | src/repository/promotion/run.rs:1877 | Maps `primary_sync_id` to `ContextKey::Primary`. |
| D R-3 children | `primary_authority` | existing | src/repository/promotion/run.rs:1209 | Reads `ChildRole::Primary`. |
| D R-3 children | `remember_publication` | existing | src/repository/promotion/run.rs:1190 | Reads `r.primary_sync_id()`. |
| D R-3 children | `child()` | existing | src/repository/promotion/run.rs:1001 | |
| D R-3 children | `ChildExecution` | existing | src/repository/promotion/lifecycle_record.rs (struct near :500) | `validate` checks `child_allowed(parent.step, role)`. |
| D R-3 children | Baseline Fact 12 lists naming `primary_sync_id` | existing | remote/reservation.rs:801; recovery.rs:1174; local_sync/record.rs:1063-1067; remote/state.rs:3835-3871; promotion/mod.rs:437 | All read the column by name, so a second-slot child is covered. |
| D R-3 children | conflict verbs' two lookups (Baseline Fact 10) | existing | remote/state.rs:3828 `lifecycle_context_child`, :3860 `lifecycle_parent_of` | Both read only `context_sync_id`; callers conflict_read.rs:445, mutation/bind/conflict.rs:196, mutation/records.rs:845. Consistent with "gain the second slot". |
| D R-1 runner | `promote_inner` | existing | src/repository/promotion/run.rs:434 | |
| D R-1 runner | `finish_steps` | existing | src/repository/promotion/run.rs:1268 | Returns `Result<(), Clean>`; publication errors are folded by `publication_error`. Context child error is returned via `L::Error::context_synchronization`. Matches the design's contrast. |
| D R-1 runner | `TicketClosureOutcome` integration/cleanup optional | existing | src/repository/closure/mod.rs:245 | Both fields are non-optional today (as the design implies). |
| D R-1 runner | binding `reached_*` | existing | mutation/bind/lifecycle.rs:291 `reached_code`, :345 `reached_effects`, :501 `reached_data`, :576 `reached` | |
| D R-1 runner | `parent_integrated` | existing | mutation/bind/lifecycle.rs:1165 | |
| D R-1 runner | `parent_effects` | existing | mutation/bind/lifecycle.rs:1206 | |
| D R-1 runner | `made_ready` | existing | mutation/bind/lifecycle.rs:456 | The design's "about 456-499" matches. |
| D R-1 runner | `LifecycleBinding` | existing | mutation/bind/lifecycle.rs:99 | |
| D R-1 runner | hook part: `context_synchronization_unfinished` | existing | closure/record.rs:788 | Used at checkpoint.rs:~650 and in the `closing` transaction; returns bare `E::busy()` (naming the blocker is done by the binding's `BlockedDto`). |
| D R-1 runner | hook part: identity against consented `closed_by` | existing | closure/checkpoint.rs:~641-657 (`pre_integration`, step 4) | |
| D R-1 runner | hook part: `move_context_to_checkpoint_with` | existing | closure/checkpoint.rs:314 | Signature `(context_repo, item, from, to, between)`. |
| D R-1 runner | hook part: reconciliation `ClosingState` (`CheckoutDone`, `CheckpointWritten`) | existing | closure/checkpoint.rs:373 (enum), :380, :383; `reconcile_closing` :390 | Keyed on `closure_oid`; generalizing to `withdraw_oid` is new. |
| D R-1 runner | `resume_written` | existing | closure/checkpoint.rs:728; trait method promotion/lifecycle.rs:536, closure/lifecycle.rs:413 | |
| D R-1 runner | `current_base` | existing | closure/checkpoint.rs:811 | Reads tracking primary else local primary. |
| D R-1 runner | `primary_advanced` | existing | closure/lifecycle.rs:97 (impl), promotion/lifecycle.rs:232 (trait) | A `LifecycleError` constructor; `is_primary_advanced` at :114. |
| D R-1 runner | `post_checkpoint_result` | existing | closure/checkpoint.rs:253 | |
| D R-1 runner | `adopt_written_closure` | existing | closure/record.rs:640 | |
| D R-1 runner | `begin_closing` | existing | closure/record.rs:595 | Requires step `ContextSynchronized` and kind `Record`; "variant per action" is new. |
| D R-1 runner | `closed_ticket_bytes` | existing | closure/observe.rs:75 | `(original: &[u8], closed_at, closed_by)`; refuses an already-closed ticket (`AlreadyClosed`), fine for an open head. Withdraw/replace helpers are new. |
| D R-1 runner | "checkpoint-only commit builder (no merge into the base)" | new | existing builder `prepare_closure` closure/checkpoint.rs:85 | Existing builder is tied to `ClosureObservation` (direct). |
| D R-1 runner | `LeasedIntegration` | existing | promotion/lifecycle.rs:275 | R-34's "without a `LeasedIntegration`" refers to this. |
| D R-1 runner | injected clock (R-23) | new | `now_whole_utc_seconds` closure/checkpoint.rs:70, used :670 | No clock parameter anywhere in promotion/run.rs. |
| D R-17 hold | `written_closure_unfinished` | existing | closure/record.rs:774 | Step list today: closing, integrating, integrated, published, publication_pending, remote_deleted, worktree_removing, worktree_removed, branch_deleted. Extending to branch records and `new_branch` step 1 is new. |
| D R-17 hold | `closure_holds_context` | existing | remote/reservation.rs:427; callers :1618, :3211 | Today it takes no execution, but `execution` is in scope at both call sites, so the "except a validated `ChildExecution`" exemption is implementable. |
| D R-17 hold | poll does not materialize | existing | remote/poll.rs:389 | |
| D R-17 hold | poll does not fast-forward | existing | remote/poll.rs:668 | |
| D R-17 hold | "poll's retirement gains the check" | new | none in src/repository/context_retirement/ | No `written_closure_unfinished` use there today. |
| D R-17 hold | `refuse_closed_ticket` | existing | src/repository.rs:299; callers :2392 (`prepare_context`), :3097 (`save_ticket_with_effective_config`), :3481 (`checkpoint_comment_locally_with`) | "slug" reaches it through the save path. |
| D R-17 hold | `bind/republish.rs` | existing | mutation/bind/republish.rs:202 | Calls `require_ticket_open`. |
| D R-17 hold | `bind/resolve_history.rs` | existing | mutation/bind/resolve_history.rs:226 | Calls `require_ticket_open`. |
| D R-17 hold | `ticket repair` / `context retire` / `context sync-local` | existing | mutation/bind/repair.rs; bind/retire.rs; bind/local_sync.rs | No `require_ticket_open` in these today; the hook refusals are new. |
| D R-17 hold | `context release` refuses for an unfinished closure | existing | src/repository/context_release.rs:221 | |
| D R-17 hold | `admit` | existing | mutation/bind/retire.rs:240 (also poll_retirement.rs:302, bind/mod.rs:879, context_retirement/handoff.rs:322, local_sync/record.rs:1144) | Several `admit` fns; the design does not say which. |
| D R-17 hold | K-10: `context sync-local` while any closure unfinished | existing | local_sync/record.rs:1162-1172 | Query over unfinished `ticket_closures`. |
| D R-2 consent | digest domain `manyhands closure v1` | existing | closure/observe.rs:38 (`CLOSURE_DIGEST_DOMAIN`), digest `expectation` :1004 | |
| D R-2 consent | domain `manyhands pull request closure v1` | new | none | |
| D R-2 consent | RFC quote of the digest binding | existing | docs/RFC/pull-request-mode.md:180 | Sentence matches the design's quote. |
| D prepare 2/3.1 | `republish_evidence` | existing | remote/sync.rs:8159 | Returns `RepublishEvidence { deletion, last_published }`; `deleted()` true for Fetched/Pushed. Fits "tracking tip, or else newest proven push". |
| D prepare 3.3 | blocker `remote_context_unmaterialized` | new | none | `LifecycleBlocker` (dto.rs:352) ends at `identity_required`, so appending works. |
| D prepare 3.3 | `PollOnce` way out | existing | mutation/way_out.rs:46 | |
| D prepare 3.3 | way out `repo sync` for "a tip only the local primary holds" | UNCLEAR | mutation/way_out.rs:301-335 | `repo sync` is `LifecycleWayOut::SharePrimary` (:42), offered only when `PrimaryAheadOfRemote` is among the blockers. Adding the new blocker to `tracking` yields `PollOnce` alone; the `repo sync` variant of the blocker needs extra logic the design does not name. |
| D prepare blockers | `history_unknown`, `history_unresolved` | existing | closure/mod.rs:36 (`ClosureBlocker`), dto.rs:352 | |
| D prepare blockers | `identity_required` | existing | closure/mod.rs:36, results.rs:624 | |
| D prepare blockers | "(`WorktreeConflicted`)" for merge/rebase/cherry-pick at the root | UNCLEAR | `SynchronizationError::WorktreeConflicted` (sync.rs `require_clean_target_state` :740-772), `ResultCode::WorktreeConflicted` results.rs:638 | The direct close reports this condition as blocker `PrimaryConflicted` (closure/observe.rs:640). `WorktreeConflicted` is not a `ClosureBlocker`/`LifecycleBlocker`, and Obligation 10 says no new closure blocker. The design should say which blocker or code a pull-request prepare answers. |
| D prepare blockers | `ClosureBlocker` mask unchanged (Obligation 10) | existing | closure/mod.rs:36 (22 variants), SCHEMA CHECK `BETWEEN 1 AND 4194303` | 2^22-1, consistent. |
| D prepare | `already_closed` blocker | existing | dto.rs:372 | |
| D prepare | `ticket_closed` | existing | results.rs:632 | |
| D preview | `PreviewDetailsDto` | existing | mutation/dto.rs:248 | `#[serde(untagged)]` enum; existing variants carry no `type` field, so the new variant's `"type"` tag is a field of its own struct. |
| D preview | `"type": "pull_request_closure_preview"` | new | none | |
| D preview | effect `context_synchronized` | existing | dto.rs `PreviewEffect` (:160) | |
| D preview | effect `closure_recorded` | existing | dto.rs:160 | |
| D preview | effect `worktree_removed`, `branch_deleted`, `context_created` | existing | dto.rs:160 | `ContextCreated` at dto.rs:184. |
| D preview | effects `context_published`, `pull_request_link`, `closure_withdrawn` | new | none | `forge::pull_request_link` fn (src/forge.rs:146) is unrelated. |
| D confirm | `begin_record` with mode/action | existing | closure/record.rs:474 | Signature `(tx, id, op, item, expectation, remote, kind)`; lifecycle trait `begin_record` at closure/lifecycle.rs:193. |
| D confirm | `complete_record` with link upsert | existing | closure/record.rs:746 | Link upsert is new (`pull_request_links` table absent). |
| D confirm | `request.retry` | existing | results.rs:415 | `RecoveryActionKind::RequestRetry`. |
| D answers | codes `ticket_closed`, `remote_branch_deleted`, `already_applied`, `recovery_required`, `external_change`, `push_rejected`, `poll_yielding`, `transport_unavailable`, `cancelled`, `original_request_required`, `occupied_path`, `publication_remote_required`, `invalid_input`, `identity_required`, `unlock_required` | existing | results.rs:584-684 | All are `ResultCode`s. `busy`, `context_missing`, `already_closed`, `remote_context_deleted`, `history_*` are blockers or `Busy` code as used. |
| D answers | `answer_code` replay path | new | none | |
| D answers | `withdrawn_since` | new | none | |
| D answers | code `already_closing` | new | none | |
| D R-5 | "as `item sync` answers that stop" | existing | mutation/bind/lifecycle_verb.rs `Stop::ContextSynchronization`; bind/lifecycle.rs:615 `stopped` | |
| D R-5 | `request cancel`, Ctrl-C | existing | mutation/progress.rs:631 `asks_a_parent`, :641 `cancel_parent` | Both match `DocumentPromote`/`TicketClose` only. |
| D outcomes | D20 `operation list` lists as `close` | existing | read/status.rs:636-690 (`closure_operations`, `OperationAction::Close`) | |
| D outcomes | D20 `original_request_required` for resume | existing | mutation/bind/resume.rs:971 | |
| D outcomes | D20 `operation abandon` of a child refused | existing | mutation/records.rs:845 (`lifecycle_parent_of`) | |
| D outcomes | `ticket close --prepare` | existing | catalog.rs:1057 | |
| D outcomes | `PromotionIo` | existing | promotion/run.rs:21 | |
| E withdraw | `ticket withdraw-closure --id` | new | none | |
| E withdraw | `context_missing`, `remote_context_deleted`, `already_applied` | existing | dto.rs:352 blockers; results.rs:684 | |
| E withdraw | `item republish` as the way out | existing | way_out.rs `Republish` (:36), Command `ItemRepublish` | |
| E withdraw | `Withdraw closure of ticket <ULID>` commit subject | new | none | Direct checkpoint subject is `Close ticket <ULID>` at closure/checkpoint.rs:39. |
| E withdraw | "new byte helper beside `closed_ticket_bytes`" | new | observe.rs:75 | |
| F new branch | `.manyhands/worktrees/<ULID>` | existing | closure/checkpoint.rs `context_repository`; repository.rs:2432 | |
| F new branch | `manyhands/ticket/<ULID>` | existing | promotion/lifecycle.rs `context_branch` | |
| F new branch | `occupied_path` | existing | results.rs:600 | |
| F new branch | unheld entries via `proof.rs` `entries` | existing | context_retirement/proof.rs:98 (private `fn entries`) | New function there is feasible; the algorithm mirrors the loop in `preservation` (:24-84). |
| F new branch | `remove_fresh_context_worktree` / `remove_context_worktree` | existing | cleanup.rs:352 / :334 | Run.rs:1593-1595 uses fresh on first attempt, plain after. |
| F new branch | `delete_tracking_ref` | existing | cleanup.rs:572 | Its comment says the caller must hold the real reservation; the fn itself takes no reservation. |
| F new branch | `retire_context` | existing | cleanup.rs:953 | `pub(super)`; takes `RetirementEvidence`. |
| F new branch | `delete_context_branch`, `BranchDeletion` | existing | cleanup.rs; run.rs:1620 | |
| F new branch | `prepare_context_unlocked` | existing | src/repository.rs:2432 | Has the "root on primary" and "item on local primary" checks the design says to omit. |
| F new branch | new fn creating branch at a commit | new | none | |
| F new branch | `retire_context` + link deletion in one transaction | new | cleanup.rs:953 | Link deletion helper (R-14) is new. |
| F new branch | `PollOnce` way out for F5 | existing | way_out.rs:319, :334 | |
| G two clones | `integration/prepare.rs` | existing | src/repository/integration/prepare.rs | |
| G two clones | `MergePreparation::Clean` | existing | integration/prepare.rs:54 | |
| G two clones | `prepare_clean_merge` | existing | integration/prepare.rs:60; wrapper remote/sync.rs:828 | |
| G two clones | `integrate_divergence` | existing | remote/sync.rs:3616 | Calls `prepare_clean_merge` at ~:3715 for `MergeRequired`. |
| G two clones | `DivergenceInputs` | existing | remote/sync.rs:3574 | Field `primary` is the fetched primary tracking OID. |
| G two clones | Context stage | existing | remote/merge.rs:15 (`IntegrationStage::Context`), :20 (`integration_stages`) | Context stage exists only with context tracking. |
| G two clones | `IncompatibleClosures` | existing | integration/validate.rs:32, :182 | |
| G two clones | new byte helper replacing closed-ticket fields | new | none | |
| H codes | `already_applied` class `None` | existing | results.rs:684 | `Outcome::of_mutation` makes it `noop` (results.rs:737). |
| H codes | `already_closing` | new | none | Class `None` yields `success` for any code other than `AlreadyApplied` (results.rs:738). |
| H codes | `recovery`'s shared arm | existing | mutation/outcome.rs:778 (`fn recovery`), shared arm :929 | Lists `Ok \| AlreadyApplied \| ...`. |
| H data | `MutationDataDto` | existing | mutation/dto.rs:999 | |
| H data | `PullRequestClosureMutationDto` and `"type": "pull_request_closure"` | new | none | |
| H data | `from_stored`, `TicketClose` lifecycle arm | existing | mutation/dto.rs:2482; arm `DocumentPromote \| TicketClose` ~:2542 | Exhaustive match on `Command` (no wildcard), so the new command needs an arm. |
| H data | synchronization conflict arm | existing | mutation/dto.rs ~:2527 (`ItemSync \| RepoSync \| OperationResume` guarded on tag) | |
| H data | host-data arm | existing | mutation/dto.rs ~:2570 (`ItemSync \| RepoSync \| OperationResume \| ItemRepublish`) | |
| H data | `mutation/way_out.rs` | existing | src/repository/mutation/way_out.rs:114 (`lifecycle_way_out`) | |
| H data | CLI retry guard reads blockers | existing | src/bin/manyhands-cli/mutate.rs:101 (`should_retry`) | Reads `MutationDataDto::Lifecycle` only. |
| H data | schema `pull_request_closure_mutation.schema.json` | new | schemas/v1/ | |
| H link | `pull_request_link` data tag / schema | new | schemas/v1/ | |
| H link | `human_collaboration.rs` | existing | src/bin/manyhands-cli/human_collaboration.rs | |
| H link | `human_line()` | existing | src/forge.rs:122 (`PullRequestLink::human_line`) | |
| H blockers | `LifecycleBlocker` | existing | mutation/dto.rs:352 | |
| H blockers | `way_out.rs` `tracking` set | existing | mutation/way_out.rs:301 | It is a closure, `PrimaryAheadOfRemote \| PrimaryMissing \| HistoryUnknown`. |
| H blockers | desktop `blocker_words` | existing | src/desktop/view/lifecycle/prepare.rs:47 | |
| H verb | `Command::TicketWithdrawClosure` | new | `Command::TicketClose` at mutation/command.rs:77 | |
| H verb | `Mutation::TicketWithdrawClosure(TicketWithdrawClosureInput { root, id })` | new | `Mutation::TicketClose(TicketCloseInput)` mutation/mod.rs:615; input struct bind/lifecycle_verb.rs:44 (`root`, `id`) | |
| H verb | `LifecycleBinding`, own verb type | existing | bind/lifecycle.rs:99; `LifecycleVerb` trait bind/lifecycle_verb.rs:101; `Close` :145 | |
| H verb | catalog entry label `C5`, options `--repo --id --request-id --prepare --confirm`, no input | existing | src/bin/manyhands-cli/catalog.rs:1057 | `ticket close` entry has exactly those options. |
| H verb | `progress.rs` `asks_a_parent`, `cancel_parent` | existing | src/repository/mutation/progress.rs:631, :641 | |
| H verb | `bind/lifecycle.rs` `held_by_its_child` | existing | mutation/bind/lifecycle.rs:1359 | |
| H verb | resume's rebuild of a lifecycle request | existing | mutation/bind/resume.rs:1314 (`TicketCloseInput`), :310 (`lifecycle_standing` maps to `Command::TicketClose`) | |
| H verb | autonomy column of `ticket close` | existing | skills/manyhands-overview/references/autonomy.md:7 | Middle column. |
| H verb | `Confirmation` home conformance row | existing | tests/support/conformance.rs:43-54 (`Home`), :59 (`HOMES`, 46 rows) | `ticket close` is `Home::Remote` (:99). |
| H operations | `operation list` lists branch record as `close` | existing | read/status.rs:636-690 | |
| H preview | `PreviewEffect` | existing | mutation/dto.rs:160 | `context_created` doc to widen: doc block above :160. |
| H preview | `schemas/v1/pull_request_closure_preview.schema.json` | new | schemas/v1/ | |
| H preview | `DATA_ROOTS` entry like `lifecycle_preview.schema.json` | existing | tests/mutation_contract.rs:1603 (69 entries), :1643 | |
| H human | preview prints closure with `closed_by` as direct does | existing | bin/manyhands-cli/human_admin.rs:97-160 (`lifecycle_details`, field dump) | |
| H desktop | `confirm.rs` arms | existing | src/desktop/view/confirm.rs:649-772 | Exhaustive `PreviewDetailsDto` match. |
| H desktop | `lifecycle_lock.rs` | existing | src/desktop/model/lifecycle_lock.rs:109 | `UNLOCKED_STEPS = [confirmed, context_synchronized]`; every other step (including unknown ones such as `retired`) locks. A `new_branch` record at `confirmed` after step 1 would not lock the tab although the repository hold applies. |
| Obl. 1 | `ParentRecord`/`LifecycleRecord` path accessors | existing | promotion/lifecycle_record.rs:262, promotion/lifecycle.rs:171 | Path is new. |
| Obl. 1 | `effect_written` | existing | closure/record.rs (LifecycleRecord impl, `closure_oid.is_some()`) | |
| Obl. 1 | `written_checkpoint` | existing | `ParentRecord::written_checkpoint` lifecycle_record.rs:~275; closure/record.rs impl | |
| Obl. 1 | `merged_context` | existing | closure/record.rs `ClosureRecord::merged_context` | |
| Obl. 1 | resume dispatch by `integration_oid` | existing | promotion/run.rs:~580 (`if r.integration_oid().is_some() && r.step() != Step::Integrating`) | Confirms the claim. |
| Obl. 3 | frozen v2 DDL, explicit column list | new | closure/record.rs:300-304 | See "Part D R-1 record". |
| Obl. 4 | `make_outcome` | existing | promotion/run.rs:1898 | Uses `.expect("post-integration evidence")` on `integration_oid`/`base_oid`, so a branch record needs a different path. |
| Obl. 4 | `run_observed` | existing | promotion/run.rs:342 | Its `integration_oid().is_none()` exit is the "before integration" path. |
| Obl. 4 | `Reached` | existing | mutation/bind/lifecycle_verb.rs:72 | |
| Obl. 4 | `closure_reached` | existing | mutation/bind/lifecycle_verb.rs:260 | |
| Obl. 5 | `Lifecycle` trait, `preview_request`, `prepare`, `KIND` | existing | promotion/lifecycle.rs:351; closure/lifecycle.rs:297, :316 | |
| Obl. 5 | `LifecycleKind::TicketClosure` | existing | promotion/lifecycle.rs:~37 | |
| Obl. 6 | `promotion_test_drive_local_bound` runs only `integrate_divergence` | existing | remote/sync.rs:3928 | `cfg(test)`; ends in `integrate_divergence` at :4024. |
| Obl. 8 | `Stages` exhaustive `ChildRole` match | existing | mutation/bind/lifecycle.rs:151, :161-179 | |
| Obl. 8 | `ParentView` | existing | promotion/mod.rs:365 | Fields `step`, `checkpoint`, `children: [Option<OperationId>; 2]`; no path or `withdraw_oid`. |
| Obl. 8 | `bind/mod.rs` recorded effects | existing | mutation/bind/mod.rs:1053 (`recorded_effects`), :1074-1080 | |
| Obl. 8 | `mutate.rs` retry guard | existing | bin/manyhands-cli/mutate.rs:101 | |
| Obl. 11 | `Integration` | existing | src/canonical.rs:93 | Variants Direct, PullRequest. |
| R-6/R-7 | `PRIMARY_COPY` | existing | read/items.rs:1974 | Rank 1; set for non-Active context rows. |
| R-8 | `closing_dependency` | new | none | Exact `ReadinessReasonCode` at read/dto.rs:365. |
| R-14 | `finalize_index` retiring branch | existing | context_retirement/record.rs:1966 | |
| R-14 | one helper deleting the link | new | none | |
| R-10 | `context_release` direct guard | existing | context_release.rs:231 | |
| R-22 | `blocker_words` | existing | desktop/view/lifecycle/prepare.rs:47 | |
| R-25 | `[forge] pull_request_url` | existing | src/canonical.rs:128 (field), forge.rs:85 (template) | |
| R-26 | Cycle 02 `pull_request` schema/DTO | new | none on this HEAD | No `pull_request` DTO or schema file; only `repository_configuration.schema.json` has a `pull_request_url`. The design covers this ("or added identically"). |
| R-28 | label `C5` row | existing | bin/manyhands-cli/catalog_tests.rs:6 (`DESIGN`), row at :85-110 | |
| R-31 | `delete_tracking_ref` needs a held reservation | existing | cleanup.rs:572 | By comment only. |
| U-1 | `finish_parent` | existing | remote/reservation.rs:1204 | Requires the parent `complete` and retired; so no existing release-without-complete transition. |
| U-1 | `finish_remote_operation` "about 2182" | MOVED | remote/reservation.rs:2112 (fn), :2136 (`_capturing`), `finish_at` call ~2182 | Cited line falls inside `finish_remote_operation_capturing`; declaration is 70 lines earlier. Harmless. |
| Testing | `FakeIo` | existing | promotion/run_tests.rs:357 | Hard-coded remote `origin`; writes a tracking ref only for the primary (`refs/remotes/origin/main`); context-target tracking/bare-repo push is the new extension. |
| Testing | `synthetic_fetch` | existing | remote/poll.rs:1435 | |
| Testing | `closure/run_tests.rs` `pull_request` module | new | src/repository/closure/run_tests.rs | File exists; module new. |
| Testing | ssh targets (`ticket_closure`, `document_promotion`, `mutation_remote`, `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`, `comment_publication`, `ssh_fixture`) | existing | tests/*.rs | All present. |
| Testing | `checkpoint_comment_locally_with` | existing | src/repository.rs:3419 | |
| Named E-1 | `tests/read_contract.rs` lists from `::ALL` | existing | tests/read_contract.rs:758 | |
| Named E-2 | exhaustive `ClosureState` matches | existing | tests/ticket_closure/journeys.rs:391; src/desktop/model/tickets_tests.rs:137; `DependencyState` read/graph_tests.rs:89 | |
| Named E-3 | `open\|closed\|all` strings | existing | tests/fixtures/cli_v1/help_root.json:216, help_resource_ticket.json:39, help_ticket_list.json:39; catalog_tests.rs:230; also src/bin/manyhands-cli/catalog.rs:175 (source) | Source file also needs the change (not a test). `parse_tests.rs:408` and `tests/cli/content.rs:360-374` exist. |
| Named E-4 | fixtures dirs, `MANYHANDS_UPDATE_GOLDEN` | existing | tests/fixtures/read_v1/, cli_v1/ | |
| Named E-5 | `DESIGN_CODES` 71 | existing | src/results_tests.rs:34 | |
| Named E-6/E-7 | `CODES_` tables | existing | tests/mutation_contract.rs:1790, :1842 | |
| Named E-8 | six + one `Lifecycle` arms | existing | tests/support/mutation.rs:699, 731, 867, 899, 931, 963; tests/support/repair.rs:359 | Counts match. |
| Named E-9 | `contract_enumerations` | existing | tests/read_contract.rs:724 | |
| Named E-10 | command.rs hard-coded sets "about 770, 870, 1018" | existing | mutation/command.rs:761, 785-789, 889, 1019 | Drift under 20. |
| Named E-11 | inventory 44 / 81, `NO_INPUT`, `DATA` | existing | tests/cli/inventory.rs:394, :412, :35, :105 | |
| Named E-12 | `HOMES` 46 | existing | tests/support/conformance.rs:59 | |
| Named E-13 | help fixtures, `CLI_CASES`, `CASES` | existing | tests/fixtures/cli_v1/help_root_text.txt; tests/support/cli.rs:217; tests/support/mutation_golden.rs:21; tests/cli/help.rs:144-149 | `help_ticket_withdraw_closure.json` new. |
| Named E-14 | five `require_ticket_open` calls | existing | src/repository/ticket_guard_tests.rs:523-545 | |
| Named E-14 | validator call "about 4015" | existing | remote/sync_tests.rs:4017 (`validates_external_integration`) | |
| Named E-14 | `preserves_item_invariants` "about 11480" | existing | remote/sync_tests.rs:11480 | |
| Named E-15 | recovery tables, exits | existing | tests/cli/recovery_tables.rs:73, 83, 96, 466; tests/cli/collaboration_exits.rs:1003-1010 | |
| Named E-16 | two read_relationships tests | existing | tests/read_relationships.rs:290-438, :919-983 | Ranges match. |
| Named E-17 | items_tests test | existing | src/repository/read/items_tests.rs:334 | |
| Named E-18 | registry table list "about 581-652" | existing | tests/repository_enablement.rs:581-652 | |
| Named E-19 | `AUTONOMY` | existing | tests/cli/skills.rs:410 | |
| Named E-20 | lifecycle test | existing | tests/cli/lifecycle.rs:923 | |
| Named E-21 | `DATA_ROOTS` 69 | existing | tests/mutation_contract.rs:1603 | |
| Named E-22 | record_tests names; marker list; recovery.rs tables; read_status INSERTs | existing | closure/record_tests.rs:91, 140, 250, 324, 830, 1324; tests/recovery_foundation_gate.rs:1124-1140; src/repository/recovery.rs:4529-4549; tests/read_status.rs:1405, 1821 | |
| Named E-23 | C1 pair "about 1858-1932" | existing | closure/run_tests.rs:1863, :1893 | |
| Required P-rows | files named in P1-P17 | existing | all files exist; `pull_request_links`, new schemas, new help fixture absent | |

## Findings

- **UNCLEAR, Part D prepare blockers, "`WorktreeConflicted`"**: `WorktreeConflicted` is a `SynchronizationError`/`ResultCode` (results.rs:638, remote/sync.rs:769), not a blocker. The direct close reports the same condition as `ClosureBlocker::PrimaryConflicted` (closure/observe.rs:640, closure/mod.rs:36). With Obligation 10 forbidding a new closure blocker, the pull-request prepare's blocker or code for a merge/rebase/cherry-pick at the root is not determined.
- **UNCLEAR, Part D prepare 3.3, way out `repo sync`**: `repo sync` is `LifecycleWayOut::SharePrimary` (way_out.rs:42), offered only when `PrimaryAheadOfRemote` is a blocker (way_out.rs:319-335). Adding `remote_context_unmaterialized` to `tracking` yields `PollOnce` only; the "tip only the local primary holds" variant needs its own rule.
- **UNCLEAR, Part D hook, `closed_at` "frozen in the record before the commit"**: today `begin_closing` writes `closed_at` and `closure_oid` in one UPDATE after the commit is built (closure/record.rs:595, checkpoint.rs:670), and the table CHECK (record.rs:229-265) ties the two together. The design's step 3.3 wording implies a separate earlier write.
- **MOVED (minor), U-1 `finish_remote_operation` "about 2182"**: the function is declared at remote/reservation.rs:2112; line 2182 is the `finish_at` call inside `finish_remote_operation_capturing` (declared :2136). `finish_parent` (:1204) is as cited.
- **Note, `ClosureStep::permits` "as today plus that one"**: today `Closing -> Abandoned` is also permitted (closure/record.rs:64-85); the design's list of abandon sources omits `closing`. Not wrong for a branch record, but the list is not "as today".
- **Note, rebuild "frozen DDL"**: the existing `closure_refusal_blockers_v2` rebuild uses the live `SCHEMA` constant and `SELECT *` (closure/record.rs:300-304); the new migration must freeze that copy (Obligation 3) or the old-table copy breaks.
- **Note, desktop lock gap**: `lifecycle_lock.rs:109` leaves `confirmed` unlocked, but a `new_branch` record holds the ticket from step 1 while still at `confirmed` (R-17); the tab would not read locked there. Compile-level arms only per R-22, so informational.
- No reference is WRONG: every named function, type, constant, test, file and line number exists at or near the cited place, and every item the design calls new is absent.

## Appendix C: Named edits, sweep, module paths and CI (research agent, verbatim)

### Task 0 research: named edits, premise sweep, test paths, CI list

Read-only research. Worktree `.manyhands/worktrees/01M87683QK8E34EYY8KS4VQG9Y`. HEAD is `72296bd0`, not `1040f41a`: the one extra commit adds a comment file under `.manyhands/`, so no source or test file differs from `1040f41a` (`git diff --stat 1040f41a..HEAD`). Line numbers are for this HEAD. Nothing was run (no cargo).

## 1. Named edits E-2 to E-23

Status key: ok = every named item exists as described; WRONG = it does not; MOVED = exists elsewhere; UNCLEAR = exists but the design text does not settle what the edit is.

| Edit | Item | Status | Current location | Note |
| --- | --- | --- | --- | --- |
| E-2 | `ClosureState` match, journeys | ok | tests/ticket_closure/journeys.rs:390-393 (`Some(Open) => 1`, `Some(Closed) => 2`) | the arm also feeds `ssh_harness::observation`, which only prints |
| E-2 | `ClosureState` match, desktop | ok | src/desktop/model/tickets_tests.rs:136-139 | |
| E-2 | `DependencyState` match | ok | src/repository/read/graph_tests.rs:88-91 (`Unresolved => "?"`, `Open \| Closed => ""`) | a string-building match, so a `Closing` arm that fails the test is a new arm, not an edit of an assertion |
| E-3 | `open\|closed\|all` in three goldens | ok | tests/fixtures/cli_v1/help_ticket_list.json:39, help_resource_ticket.json:39, help_root.json:216 | `help_root_text.txt` does not contain the value list |
| E-3 | `catalog_tests.rs` | ok | src/bin/manyhands-cli/catalog_tests.rs:230 | `catalog.rs:175` holds the source string (Task 1 file) |
| E-3 | `parse_tests.rs` valid list | ok | src/bin/manyhands-cli/parse_tests.rs:408 (`["open","closed","all"]`); invalid-value check at 433-435 | |
| E-3 | `tests/cli/content.rs` row | ok | tests/cli/content.rs:360-377 (`every_ticket_filter_matches_the_same_f1_filter`, rows open/closed/all) | `dispatch.rs:59-61` maps unknown values to `All` by `_`: see E-35 |
| E-4 | item goldens | ok | 25 item goldens (those holding `"changed_at"`): tests/fixtures/read_v1 (14) and cli_v1 (11). None holds `pull_request`; `closure` is `{...}` for tickets and `null` for documents and nonconforming entries | the 4 preview goldens (`mutation_v1/` and `cli_v1/` `ticket_close_preview.json`, `document_promote_preview.json`) have a `closure` key of another shape (recording/closed_by) and are not item goldens. `MANYHANDS_UPDATE_GOLDEN` exists (tests/support/golden.rs:20) |
| E-5 | `DESIGN_CODES` 71 | ok | src/results_tests.rs:34 (`[(&str, Option<FailureClass>); 71]`); messages test `result_code_messages_are_fixed` at 229 | |
| E-6 | both `CODES_` tables | ok | tests/mutation_contract.rs:1790 (`CODES_NO_FIXTURE_CAN_PRODUCE`, 3 rows) and 1842 (`CODES_NO_MUTATION_GOLDEN_HAS`, 7 rows); checked by `code_differences` at 1815 | |
| E-7 | removal in Task 14 | ok | same two tables | |
| E-8 | `tests/support/mutation.rs` six arms | ok | exhaustive `match envelope.data` over `MutationDataDto` at 673 (`ticket_data`), 705 (document), 841 (remote), 873 (operation), 905 (key), 937 (identity) | each lists every variant, `Lifecycle(_) => panic!` included |
| E-8 | `tests/support/repair.rs` one arm | ok | tests/support/repair.rs:333-360 (`repair_data`) | |
| E-9 | `contract_enumerations` | ok | tests/read_contract.rs:724; `closure.state` at 978, `DependencyState` at 984/1009/1040, `ReadinessReasonCode` at 1014 | there is no `forge` entry for a `pull_request` object yet (only `forge.kind` of the repository configuration, at 743-754), so the "unless Cycle 02 registered it" clause applies. The test that runs it is `published_enumerations_list_exactly_the_registered_values` (1594) |
| E-10 | `command.rs` tests | ok | src/repository/mutation/command.rs:769 (`the_stopping_commands_and_their_stages_and_confirmation`, lifecycle list at 785/789), 869 (`the_commands_whose_caller_must_first_be_shown_need_confirmation`, list at 889), 1017 (`a_promotion_and_a_closure_are_confirmed_operations_with_safe_points`, list at 1019) | |
| E-11 | `mutating.len() == 44`, `catalogued.len() == 81` | ok | tests/cli/inventory.rs:394 and 412 (current values 44 and 81) | |
| E-11 | `NO_INPUT`, `DATA` | ok | tests/cli/inventory.rs:35 (`NO_INPUT`, `ticket close` at 91), 105 (`DATA`, `ticket close` at 246: `lifecycle_preview`, `lifecycle_blocked`, `lifecycle_mutation`) | |
| E-12 | `HOMES` 46 | ok | tests/support/conformance.rs:59 (`[(&str, Home); 46]`; `ticket close` is `Home::Remote` at 99) | the new row goes in `tests/mutation_confirmation/conformance.rs:29` `ROWS: [Row; 8]`, which becomes 9 (the design does not name that count). `Expect::Exempt` exists (conformance.rs:163) |
| E-13 | help listings | ok | fixtures `help_root.json`, `help_root_text.txt`, `help_resource_ticket.json`; list in tests/cli/help.rs:138-160 (`help_ticket_close` at 154) | |
| E-13 | `DESIGN` table, `C5` row | ok | src/bin/manyhands-cli/catalog_tests.rs:6 (`DESIGN`), `"C5"` at 95 | |
| E-13 | `CLI_CASES`, `CASES` | ok | tests/support/cli.rs:217; tests/support/mutation_golden.rs:21 (`ticket_create_already_applied` at 248 shows the naming) | |
| E-14 | 5 `require_ticket_open` calls | ok | src/repository/ticket_guard_tests.rs:523, 526, 529, 535, 545 | |
| E-14 | validator call "about 4015" | ok | src/repository/remote/sync_tests.rs:4015 (`RepositoryService::validates_external_integration(&repository, &step, candidate)`) | |
| E-14 | `preserves_item_invariants` "about 11480" | ok | src/repository/remote/sync_tests.rs:11480 | |
| E-15 | `LIFECYCLE`, `FAMILIES`, `C5_REQUIRED`, `c5_codes()` | ok | tests/cli/recovery_tables.rs:73, 83, 96, 466 | the loop at 577-587 needs a row or a reason for every `(verb, code)` of `C5_REQUIRED` for each verb of `LIFECYCLE`, and the cap `c5_reasons.len() <= 2` (586-587) leaves room for at most two reasons for the new verb |
| E-15 | `collaboration_exits.rs`, audit counts | ok | tests/cli/collaboration_exits.rs:1003-1079 (`C5_CLI_ONLY`, `c5_union`), 1287-1291 (`library.len() == 46`, `expected.len() == 49`); audit at docs/research/wave-03-api-audit.md | the counts change when D-6 adds `already_closing` to the C5 union |
| E-16 | see 1a below | ok, with two notes | tests/read_relationships.rs:289-438 and 918-983 | |
| E-17 | `ticket_filters_compare_whole_values_and_closure_reads_only_closed_at` | ok | src/repository/read/items_tests.rs:334 (rows `says_closed`, `is_closed` at 361-366; `context("/r")` is `Primary`, so each row is its own primary copy) | `StoredItem` literals: items_tests.rs:243, 335, 393, 426, 440, 522, 541, 588. `Related::of` calls: 219, 435, 487, 607, 642, 669, 734. `TicketNode` literals: graph_tests.rs:20, 29, 36, 400, 404, 654, 679, 721, 756 |
| E-18 | registry table list | ok | tests/repository_enablement.rs:581-652; the list is alphabetical, so `pull_request_links` goes between `problems` and `registry_migrations` | |
| E-18 | `ItemDto` literals | ok | src/desktop/model/drafts/create.rs:130, src/drafts/compare_tests.rs:100, src/desktop/model/drafts/save_tests.rs:61, ticket_view_tests/fixtures.rs:55, edit_tests.rs:41, editor_text_tests.rs:177, documents_tests.rs:46, src/bin/manyhands-cli/fill_tests.rs:6 and 125, human_tests.rs:44, 71, 86, 510 | |
| E-18 | `ClosureDto` literals | ok, list incomplete | compare_tests.rs:109 and 149, edit_tests.rs:91, create.rs:139, human_tests.rs:89, 316, 346, fixtures.rs:64 | two literals are in no named file: src/repository/closure/run_tests.rs:1532 and tests/ticket_closure/journeys.rs:399 (E-26, E-24) |
| E-19 | `AUTONOMY`, `autonomy.md` | ok | tests/cli/skills.rs:410; skills/manyhands-overview/references/autonomy.md:7 (`ticket close` in the confirm column) | |
| E-20 | `application_data_holds_no_closed_by_or_title_beyond_the_index` | ok | tests/cli/lifecycle.rs:923 | |
| E-21 | `DATA_ROOTS` 69 | ok | tests/mutation_contract.rs:1603 (`[&str; 69]`) | |
| E-22 | the four named record tests | ok | src/repository/closure/record_tests.rs:140 (`steps_permit_only_designed_transitions`), 250 (`closure_step_round_trips_through_parent_view`), 324 (`step_invariants_enforced`), 830 (`no_closed_by_column_or_value`, pins the 25 column names at 840-866) | |
| E-22 | rebuild tests | ok | record_tests.rs:91 (`migration_is_all_or_nothing_and_idempotent`), 1324 (`closure_refusal_blockers_v2_rebuilds_an_old_bound_table_once`; its `INSERT ... SELECT * FROM before` at 1350 is positional) | |
| E-22 | marker list | ok | tests/recovery_foundation_gate.rs:1132-1142 (8 markers; `closure::record::migrate` runs in the registry-schema transaction, discovery.rs:2749, so the new marker belongs there; Cycle 01's `wave_06_pull_request_mode_v1` runs later at discovery.rs:2757 and is not listed) | |
| E-22 | `recovery.rs` minimal tables | ok | src/repository/recovery.rs:4529-4549 (`ticket_closures` with 4 columns, positional `VALUES` at 4549) | |
| E-22 | `read_status.rs` INSERTs | ok | tests/read_status.rs:1405 and 1821 (explicit columns, no NOT NULL addition expected) | |
| E-23 | C1 pair | ok | src/repository/closure/run_tests.rs:1863 (`a_written_checkpoint_refuses_a_manual_context_synchronization`) and 1893 (`an_active_manual_context_synchronization_holds_off_the_checkpoint`) | |

### 1a. E-16 exact lines

`ready_and_blocked_are_decided_across_primary_and_two_item_worktrees` (tests/read_relationships.rs:289-438). A has its closure only in its own worktree (line 309), B has `deps: [A]` there (310), C depends on A on the primary (297), D depends on B (298).

Setup, unchanged: 295-311. Comments to reword: 305-306, 318-319, 348.

Assertions the edit changes:
- 320-324: the primary copy of A stays open. Unchanged text.
- 325: `sorted_ids(&ready) == [B, C]` becomes `[]`.
- 326: `sorted_ids(&blocked) == [D]` becomes `[B, C, D]`.
- 327-328: unchanged.
- 329-332: the loop over `ready.items` asserts `Ready` and no reasons; it becomes vacuous. It moves to `blocked` items B, C with one `closing_dependency` reason naming A (`reasons(...)` at 116 gives `(code, ids, complete)`).
- 333-340: the context kinds of B (`Active`) and C (`Primary`) are read from `ready`; they are read from `blocked`.
- 341-346: D, `OpenDependency` on B. Unchanged.
- 349-352: `ticket_readiness(default)` is `[B, C, D]`. The ids stay equal (A is left out as closing); the comment "a closed one never" is wrong for A.
- 353-362: `ClosureFilter::Closed` is empty. Still holds (A is `closing`).
- 366: `listed.items.len() == 4`. Still holds (lists show closing tickets).
- 367-370: `state(a) == Closed` and no reasons. A reads `closing`; its own readiness is "computed as an open ticket's", so `Ready` with no reasons.
- 371-373: list/readiness agreement. Holds once both sides are recomputed.
- 374-380: `list --ready`/`--blocked` agree with `ready`/`blocked`. Holds (both now `[]` and `[B, C, D]`).
- 382-387: expected states `(A, Closed)`, `(B, Ready)`, `(C, Ready)`, `(D, Blocked)` become A `Ready`, B `Blocked`, C `Blocked`, D `Blocked`.
- 390: `shown.readiness == listed.readiness`. Holds.
- 399-405: down tree of D, states `[Open, Closed]` become `[Open, Closing]`.
- 406-415: up tree of A, lines unchanged; 415 `up.ticket.state == Closed` becomes `Closing`.
- 416-418: both-way tree of B. Unchanged.
- 419-425: plan. With A as an open node the batches are A=1, B and C=2, D=3. Removing A and renumbering gives `[(1,[B,C]),(2,[D])]`, the same as line 422. Line 424 (`unplannable` empty) holds. Line 425 (`batches(&plan)[0].1 == sorted_ids(&ready)`) breaks, because `ready` is empty.
- 426-428: plan item contexts. Unchanged.
- 429-430: critical path `[B, D]`. A has no length and drops out, and B beats C by the lowest ID, so unchanged.
- 431: cycles empty. Unchanged.
- 432-436: `show_path` of the primary copy of A is `Ready`. The copy has no `closed_at` and the primary's copy is open, so it stays `open`, and the file read decides its own readiness. Unchanged.
- New part: commit A's closure on the primary as well, refresh, and assert B and C ready, A closed. Lines 437-438 get the new assertions before `assert_git_transport_uninitialized`.

`closing_a_dependency_makes_its_dependent_ready_without_the_dependent_being_observed_again` (tests/read_relationships.rs:918-983). The design says "about 919" and that is the right test. Currently A exists only in its worktree (`save_ticket(..., Create)` at 927).
- 925-928: setup. A must be written and committed on the primary first (as B is at 926) and its context made with an edit intent (`edit_context`, line 232, or `save_ticket` with `ContextIntent::Edit`).
- 931-946: edges before and the dependent bytes. Unchanged.
- 951-957: the SQL update `WHERE item_id = ?1` matches every row of A; with two rows `assert_eq!(closed, 1)` at 957 fails, so the update must be limited to A's primary row.
- 959-965: B ready, `deps[0].state == Closed`, plan `[(1,[B])]`. A's primary row is closed and the context copy has no `closed_at`, so the copy reads `closed` (Part A row 1): unchanged.
- 967-970: write A's context again, refresh, B blocked. Unchanged text (the reason is `open_dependency`).
- 971-976: closure in A's worktree only. 976 `ids(&ready) == [B]` becomes B blocked with one `closing_dependency` reason and `ready` empty. The comment at 971-972 is reversed.
- 977-981: edges and dependent bytes unchanged. Unchanged.

Notes for E-16: (1) the design says "the plan's batches change"; by the table above the numbers do not change, only 425 does. (2) The design does not say what readiness state a closing ticket's own item shows in `list`/`show` (lines 369, 383); "computed as an open ticket's" gives `Ready` for A while `ticket ready` omits A. (3) The test `a_status_that_says_closed_closes_nothing` (877-916) writes the closure on the primary and is unaffected.

## 2. Premise sweep (E-24 onwards)

Intent: M = mechanical, R = REVERSES. Rows with no edit expected are marked "none".

| Edit | Where | What the Cycle changes | Task | Intent |
| --- | --- | --- | --- | --- |
| E-24 | tests/ticket_closure/journeys.rs:355-405, case `contained_closed(false)` (journeys.rs:21, fn at 311) | B's kept context is open while B's primary is closed (B's local primary holds the closure after the poll; asserted at 405). The listing is asserted to read the context's open copy: `Some(ClosureState::Open) => 1` (391) and `ClosureDto { Open, None, None }` (399-403), comment "reads the kept context's open copy (R-T6-6)" (358-359). Part A row 1 (copy has no `closed_at`, primary's copy holds a closure) gives `closed` with the primary's `closed_at`/`closed_by`; the listed copy stays `Active`. Also the `ClosureDto` literal gains `publication`. | 1 (and 4) | REVERSES. SSH target `ticket_closure` |
| E-25 | tests/cli_remote/retirement.rs:131-153, case `retirement_cli_authenticated_live_pass` (retirement.rs:9, fn at 42) | After the default poll B keeps its old context ("default preserves old local copies", 131) and its primary holds the closure. Line 141 asserts `ticket show` reads `closure.state == "open"` ("default poll retains own-worktree read precedence") and 145-152 asserts `ticket ready` does not list `OTHER_ITEM`. Under Part A the show reads `closed` and `OTHER_ITEM` (which depends on `ITEM`) becomes ready, so both assertions flip. The later `ready_after` (186-195) and 253-284 are unchanged. | 1 | REVERSES. SSH target `cli_remote` |
| E-26 | `ClosureDto` literals outside E-18: src/repository/closure/run_tests.rs:1532 (`a_completed_closure_reads_closed_from_the_primary_only`, primary closed, meaning unchanged); tests/ticket_closure/journeys.rs:399 (see E-24); production src/repository/read/items.rs:1355 (`closure()`) | gain `publication` (and `items.rs` gains the derivation) | 4 (items.rs also 1) | M |
| E-27 | `ItemDto` production literals: src/repository/read/items.rs:1328, 1441, 1497, 2284, 2449 | gain `pull_request` | 4 | M |
| E-28 | `TicketClosureOutcome::integration()` and `cleanup()` used as non-optional by tests: src/repository/closure/run_tests.rs:426, 435, 731, 1232, 1325, 1417, 1423, 1487, 1766, 2114, 2174, 2283, 2590, 2620; tests/ticket_closure/checkpoint.rs:139, 146, 226, 227; identity.rs:148, 151; exits.rs:262, 287; races.rs:67, 206; journeys.rs:104, 169, 235, 294, 335, 339, 415, 419, 462, 505, 567, 601, 648, 700, 740, 795, 824, 868, 898, 901, 963, 1057, 1100, 1456 (48 sites) | Part D makes both optional ("absent for a branch record"); each site becomes `Some(..)` or an unwrap, unless the accessors keep their types and the branch shape is a separate type. No named edit and no File Map row covers `tests/ticket_closure/*` | 7 | M (if the accessors become `Option`) |
| E-29 | `closure::begin_record` callers in tests (7-argument form, src/repository/closure/record.rs:474): record_tests.rs:39 (helper), 203, 306, 392, 481, 513, 535, 548, 576, 637, 688, 919, 939, 1064, 1160, 1189, 1228, 1288, 1307; checkpoint_tests.rs:141, 1038; remote/reservation_tests.rs:3702; remote/sync_tests.rs:17782, 17930, 18288; local_sync/admission_tests.rs:81; local_sync/review_tests.rs:403; context_release_tests.rs:696. Trait wrapper at closure/lifecycle.rs:193-212 | Part D step 1 says `begin_record` takes `integration_mode` and `action`; if the signature changes every caller gains arguments (direct, close). Only `record_tests.rs` is in Task 7's file list | 7 | M |
| E-30 | Exhaustive `match` on `ChildRole`: src/repository/remote/reservation_tests.rs:4542, 4546, 4577, 4633, 4660; src/repository/promotion/record_tests.rs:222-236; production src/repository/mutation/bind/lifecycle.rs:161-179, promotion/run.rs, lifecycle.rs, lifecycle_record.rs | `ChildRole::Publication` needs an arm in each; `reservation_tests.rs` and `promotion/record_tests.rs` are in no File Map row | 7 | M |
| E-31 | Step lists and fence tests: remote/reservation_tests.rs:3749-3753 (steps set while building a parent), 4413 and 4421-4425 (`close_primary_fence_at`: steps that allow or refuse the primary child, defined at 4355); closure/record_tests.rs:250-270 (E-22) | the fence's range and the new `retired` step and the branch-record rows (publication child from `closing`); the lists need `Retired` and new rows. `closure_holds_context`'s exemption has existing tests only for the direct holds (`close_reservation_holds_slot_like_promote`, 3773) | 7 | M |
| E-32 | `PromotionIo` implementations in tests: promotion/run_tests.rs:363 (`FakeIo`), 720, 895, 1654, 2300, 2519, 2949, 3020, 3983, 4463, 4788, 5394, 5554, 7123; closure/run_tests.rs:1439, 1564 (16 impls) | the design extends `FakeIo` for the publication child; every other impl needs the new method too, unless the trait gives it a default | 7 | M |
| E-33 | Callers of the two validators that Task 3's File Map omits: src/repository/local_sync/resolve.rs:446, 615, 636, 993; local_sync/run.rs:714, 794. (Named: remote/sync.rs:3317, 3434, 4551-4594, 4898, 5304 and remote/conflict_read.rs:768.) | gain no primary side (`None`) or the new argument; Testing lists `--lib repository::local_sync::` for them, the File Map does not list the files | 3 | M |
| E-34 | `require_ticket_open` production callers: src/repository.rs:316, context_release.rs:231, mutation/bind/republish.rs:202, mutation/bind/resolve_history.rs:226 | gain the integration (`Direct` for `context_release.rs`) | 2 | M |
| E-35 | src/bin/manyhands-cli/dispatch.rs:59-61 (`_ => ClosureFilter::All`) | a new `closing` value would silently map to `All`; needs the explicit arm (Part A) | 1 | M |
| E-36 | Production exhaustive matches (compile-level): `ClosureState`: desktop/model/relationships.rs:337-338, tickets.rs:241-242, relations_edit.rs:328-329, read/items.rs:1357-1358; `DependencyState`: relationships.rs:102-104 and 299-301, read/items.rs:323-325 (`Target`), read/graph.rs:494-496 and 1104-1105; `ReadinessReasonCode`: desktop/model/tickets.rs:160-167; `ClosureFilter`: read/items.rs:91-93, desktop/view/tickets/filter_bar.rs:92-94, desktop/model/list_view.rs:165-167 and 255, desktop/model/ticket_view.rs:104-106 and 231-233 | `Closing` arms. `desktop/view/item/details.rs:201` already has a `_` arm (not in the design's Baseline Fact 5) | 1 | M |
| E-37 | `PreviewEffect::ALL` phrase test src/bin/manyhands-cli/human_admin_tests.rs:107-111 (every effect has a distinct phrase); production exhaustive matches: `PreviewDetailsDto` human_admin.rs:99-120 and desktop/view/confirm.rs:649-772; `LifecycleBlocker` desktop/view/lifecycle/prepare.rs:58+; `Command` command.rs (157, 208, 261, 315, 380, 455, 509, 578, 660), mutation/mod.rs:672, bind/mod.rs:1074-1079 and 1192, progress.rs:634-654, bind/resume.rs:306-317 and 1308-1313, bind/lifecycle.rs:1367-1368; `MutationDataDto`: bind/mod.rs:319. All tests that unpack a `PreviewDetailsDto` use a wildcard or let-else | new arms and three new phrases | 5, 10, 11 | M |
| E-38 | The direct close's made-ready read, src/repository/mutation/bind/lifecycle.rs:456-499 (`read.target_closed || !stop`), with graph.rs:297-307 (`made_ready` returns `closed[ticket]`) | The read now says `target_closed` only when the primary's copy holds the closure. The binding reads only after the domain's index refresh (`MadeReadyFrom::IndexPending` otherwise), and `c5_made_ready.rs:497-499` states the index reads the ticket as closed past integration, so the cases `one`, `stop_past_integration`, `stopped_in_cleanup` and `peer_reopens_the_ticket` (704-731) keep their answers. No test pins a made-ready read taken while only the context held the closure | 10 (read), none (tests) | M (none expected) |
| E-39 | Tests pinned to documents: tests/cli/skills.rs:215 (`skill_commands_are_catalogued`), 258, 335, 459; tests/cli/inventory.rs:25-27 and 357-380 (taxonomy of cli-contract.md and the relationships RFC); tests/cli/recovery_tables.rs:21-22; collaboration_exits.rs:21; exits.rs:604; authoring_exits.rs:18 | D-1, D-4, D-6, D-7 edits must keep these parsers and counts true; Task 11 covers the taxonomy row, Task 15 covers E-15 only | 11, 15 | M |
| E-40 | Tests that iterate `Command::ALL`, `ResultCode::ALL` or `PreviewEffect::ALL` and gain the new value on their own: src/repository/mutation/outcome_tests.rs:1384, 1416 (needs `OperationResume` offered for every command but `FolderCreate` and `PollOnce`), 1552, 1668, 1871, 2311; src/results_tests.rs:237, 493, 860 (`already_closing` has class `None`, so 860 expects `Success`, which the design gives); src/repository/read/tests.rs:158, 360; src/desktop/view/operations_tests.rs:758; src/bin/manyhands-cli/human_collaboration_tests.rs:35; tests/cli/recovery_tables.rs:275; tests/cli/documents.rs:81; tests/cli_remote/interruption.rs:1077 | none expected | 5, 11 | M (none) |
| E-41 | src/desktop/model/activity_tests.rs:448 (`subjects()` has 44 desktop intents) | unchanged if the desktop gets no `Intent` for the new verb (R-22: compile-level arms only; the `Mutation` to `Intent` direction needs none) | 11 | M (none) |

### Checked, unaffected (no edit expected)

- Closure only on the primary: tests/read_relationships.rs:571, 729, 909, 1548; tests/read_items.rs:638, 898, 3283, 3700 and every `CLOSURE` of tests/support/items.rs; tests/read_characterization.rs:147; tests/read_boundary.rs:499; tests/mutation_relationships.rs:710; tests/mutation_bridges/context.rs:592, 773 (uncommitted edits and a committed closure before a context exists); the item goldens of E-4.
- Closed ticket refused in direct mode: ticket_guard_tests.rs (all `Fixture::new(CLOSURE)` rows, once E-14 passes `Direct`), tests/local_authoring.rs:3951-4009, tests/mutation_replay/slug_assign.rs:171, execute.rs:728, tests/mutation_bridges/context.rs:301, tests/mutation_remote/republish.rs:633-668 (closure committed on the item's branch; the guard reads both tips in direct mode), tests/mutation_remote/c5_lifecycle.rs:661-675 and 3203-3222, tests/ticket_closure/races.rs:61, journeys.rs:368 and 1064. No existing test runs `ticket close`, a ticket write or a guard row in a pull-request repository.
- After a finished direct close the primary holds the closure, so reads say `closed`: closure/run_tests.rs:1516-1545 and 2269-2310, tests/cli_remote/polling.rs:672-695, journeys_c5.rs:820-837, retirement.rs:253-284, context_retirement/acceptance_tests.rs:262-267, the desktop tests (after_tests.rs:159, confirm_tests.rs:340, interruption_tests.rs:417).
- Validators: `all_recorded_ticket_sides_constrain_retained_closure_before_writes` (sync_tests.rs:11008) and `retained_candidate_cannot_bypass_all_side_closure_validation` (11622) run resolution fixtures that pin no window (`resolution_fixture`, 2307; `primary_oid` appears only at 5001, 6036, 7484, 13818), so no primary side is given and the rows hold; local_sync/conflict_tests.rs:716, 780 and 870 are local synchronizations and unchanged; `c4_conflict_read_incompatible_closures` (tests/mutation_remote/conflict_read.rs:681) is a primary-target `repo sync`, so Part G's settlement does not reach it.
- `ticket_closures` readers by named column: tests/ticket_closure.rs:446, 613, 624, 830, 837; ticket_closure/identity.rs:19, privacy.rs:133, 191; tests/cli/lifecycle.rs:869; mutation_remote/c5_lifecycle.rs:743, 3114; c5_made_ready.rs:1016; remote/reservation_tests.rs:3919-4096; local_sync/admission_tests.rs:121-123, 401, 411; registration_removal_tests.rs:186-190; local_sync/record_tests.rs:391-398. They keep working if the new columns have defaults. `closure/record_tests.rs:559` and `1055` insert with explicit columns.
- `command_index_tests.rs:1133` (`rows: [Row; 27]`): its `ticket close` row is direct mode; a row for the new verb is optional.
- The `refuse_closed` precheck (bind/ticket.rs:241) compares with `Closed`; a direct close still unfinished (context-only closure) now reaches the domain guard, which reads both tips in direct mode and refuses `ticket_closed`. No test pins the precheck alone.

## 3. Test module paths (plan Task 0 Step 3)

All exist. Form is `--lib <path>`; the module is declared as shown.

| Plan name | Path to use | Defined in | An existing test |
| --- | --- | --- | --- |
| `repository::closure::run_tests` | `repository::closure::run_tests::` | src/repository/closure/mod.rs:27-29 (file run_tests.rs) | `no_remote_closure_records_merges_and_cleans_with_the_status_kept` (run_tests.rs:412) |
| its new `pull_request` module | new | no `mod` exists inside run_tests.rs | n/a |
| `repository::closure::record` tests | `repository::closure::record::tests::` | closure/record.rs:954-956 (file record_tests.rs) | `migration_is_all_or_nothing_and_idempotent` (record_tests.rs:91) |
| `repository::ticket_guard::tests` | `repository::ticket_guard::tests::` | ticket_guard.rs:60-62 (file ticket_guard_tests.rs) | `save_edit_of_closed_ticket_is_refused_before_provisioning` (232) |
| `repository::integration::tests` | `repository::integration::tests::` | integration/mod.rs:58-59 (file integration/tests.rs) | `memory_preparation_has_no_persistent_objects_refs_or_index_effect` (54) |
| `repository::remote::sync::tests` | `repository::remote::sync::tests::` | remote/sync.rs:3-5 (file sync_tests.rs) | `comment_publication_identity_mapping_keeps_the_opaque_confirmation` (4) |
| `repository::remote::reservation::tests` | `repository::remote::reservation::tests::` | remote/reservation.rs:2-4 (file reservation_tests.rs) | `promotion_manual_reservation_holds_across_cleanup_and_only_final_sql_finishes` (13) |
| `repository::context_retirement::tests` | `repository::context_retirement::tests::` | context_retirement/mod.rs:20-21 (file tests.rs) | `retirement_proof_ancestor_requires_positive_completion` (70) |
| `repository::cleanup::tests` | `repository::cleanup::tests::` | cleanup.rs:1021-1023 (file cleanup_tests.rs) | `cleanup_retirement_branch_guard_runs_under_ref_lock_before_delete` (70) |
| `repository::discovery::tests` | `repository::discovery::tests::` | inline `mod tests` at discovery.rs:3050 | `a_migration_waits_for_one_in_progress_and_adds_only_what_is_still_missing` (3150) |
| `repository::read::items::tests` | `repository::read::items::tests::` | read/items.rs:2784-2786 (file items_tests.rs) | `observation_token_is_the_digest_of_the_documented_bytes` (13) |
| `repository::read::graph::tests` | `repository::read::graph::tests::` | read/graph.rs:1263-1265 (file graph_tests.rs) | `a_ticket_is_ready_when_every_dependency_is_a_closed_ticket` (113) |
| `results::tests` | `results::tests::` | results.rs:1127-1129 (file results_tests.rs) | `result_codes_match_the_design_table_exactly` (146) |

Targets (`tests/<name>.rs` exists for each; the name is the `--test` argument):

| Target | Existing test |
| --- | --- |
| `read_items` | `refresh_stores_closed_by_and_unknown_metadata_with_each_item` (135); `presentation_across_contexts` (5703) |
| `read_relationships` | `ready_and_blocked_are_decided_across_primary_and_two_item_worktrees` (290) |
| `read_contract` | `published_enumerations_list_exactly_the_registered_values` (1594) |
| `mutation_contract` | `an_opted_poll_matches_its_distinct_closed_schema_and_golden` (42); `every_bound_command_has_a_golden` (1587); `every_result_code_has_a_mutation_golden_or_a_reason_none_has_it` (1876); `every_result_code_has_a_golden_or_a_reason_no_fixture_can_produce_it` (1890) |
| `mutation_bridges` | modules `context`, `repair`, ...: `context::prepare_edit_provisions_and_a_second_request_reuses_it` (tests/mutation_bridges/context.rs:193) |
| `mutation_confirmation` | `a_preview_of_repo_enable_shows_what_will_happen_and_writes_one_confirmation` (137) |
| `cli` | modules: `fixture_directory_is_exactly_the_registered_cases` (tests/cli.rs:103); `inventory::every_bound_mutation_is_a_catalogued_mutating_verb_less_the_two_exempt` (tests/cli/inventory.rs:380) |

Also confirmed by name for later tasks: `all_recorded_ticket_sides_constrain_retained_closure_before_writes` (remote/sync_tests.rs:11008) and `retained_candidate_cannot_bypass_all_side_closure_validation` (11622); `every_ticket_filter_matches_the_same_f1_filter` (tests/cli/content.rs:353).

## 4. CI list

The plan adds no new test target. It names these targets only: `read_items`, `read_relationships`, `read_contract`, `mutation_contract`, `mutation_bridges`, `mutation_confirmation`, `mutation_remote` (new cases in `tests/mutation_remote/c5_lifecycle.rs` and the case table in `tests/mutation_remote.rs`), `cli`, `ticket_closure`, `document_promotion`, `cli_remote`, plus the SSH tier of AGENTS.md. Its only `Create:` lines (implementation plan lines 161, 163, 263) are `src/repository/pull_request_links.rs`, two schema files, and a ledger document; the new integration tests are modules under existing targets.

`tests/*.rs` has 39 files. The `cargo test` line of `.github/workflows/build.yml` (line 117) names 37 of them. The two missing, `editor_feasibility` and `read_characterization`, are pre-existing and unrelated to this plan (`read_characterization` is in the AGENTS.md fast tier).

## Findings

1. **E-24 REVERSES (not in the design).** tests/ticket_closure/journeys.rs:391-405 (`contained_closed(false)`) asserts that a kept open context beside a closed primary lists as `ClosureState::Open` with no closure fields; Part A row 1 makes it `closed` with the primary's fields. Needs the owner's listing with E-16.
2. **E-25 REVERSES (not in the design).** tests/cli_remote/retirement.rs:139-152 asserts `ticket show` reads `open` and `ticket ready` omits `OTHER_ITEM` while B keeps an open context beside a closed primary ("own-worktree read precedence"); both flip under Part A (`closed`, dependent ready). SSH target `cli_remote`.
3. **E-16 (declared reversal) scope.** First test: lines 325, 326, 329-332, 333-340, 367-370, 382-387, 404, 415, 425 change, plus the new second phase; 422, 430 and 436 do not. The design's "the plan's batches change" overstates it: after removing A and renumbering, `batches(&plan)` is still `[(1,[B,C]),(2,[D])]` (line 422); only line 425 breaks. The state of A's own readiness (369, 383) is not decided by the design text (open-ticket rule gives `Ready`). Second test: 927 (setup), 951-957 (SQL must be limited to A's primary row, else `assert_eq!(closed, 1)` at 957 fails), 971-976.
4. **E-18 incomplete.** The `ClosureDto` literals at src/repository/closure/run_tests.rs:1532 and tests/ticket_closure/journeys.rs:399 are in no named file.
5. **E-28 unnamed edit, 48 sites.** If `TicketClosureOutcome::integration()` and `cleanup()` become `Option` (Part D), 48 test call sites in closure/run_tests.rs and tests/ticket_closure/* change; no E-number and no Task 7 file-map row covers `tests/ticket_closure/`.
6. **File Map gaps (E-29 to E-33).** Files that need edits but sit in no row: remote/reservation_tests.rs (ChildRole matches 4542-4660, step lists 3749 and 4421, `begin_record` 3702), promotion/record_tests.rs:222-236, checkpoint_tests.rs, local_sync/admission_tests.rs and review_tests.rs, context_release_tests.rs:696, remote/sync_tests.rs (17782, 17930, 18288), local_sync/resolve.rs and run.rs (validator callers, Task 3), and the 15 other `PromotionIo` impls if the trait gains a required method.
7. **E-35.** dispatch.rs:59-61 maps any unknown `--closure` value to `All`, so `closing` would pass as `All` without the explicit arm.
8. **E-15 cap.** tests/cli/recovery_tables.rs:586-587 allows at most two C5 reasons, so the new verb's recoverable codes need rows (goldens), not reasons, in nearly every case.
9. **E-12.** The new conformance row raises `ROWS: [Row; 8]` in tests/mutation_confirmation/conformance.rs:29 to 9; the design names only `HOMES` 46 to 47.
10. **E-38.** The made-ready read does not reverse any existing assertion because the binding reads after the domain's index refresh (c5_made_ready.rs:497-499); it would only differ if the primary row were stale, which the code answers as `index_pending`.
11. **CI.** No new test target; nothing to add to build.yml.

## Appendix D: Task 7 design checkpoint, approved (verbatim)

# Task 7 Design Checkpoint: Branch Records, The Pull-Request Close

## Controller approval and final corrections (binding; they override the text below where they differ)

Approved by the controller on 2026-11-25 after three independent checks (verdict on revision 3: sound with two fixes, applied here).

1. **Slice B's run tests.** D17 and D18 (and any other slice B run test) use `lifecycle_request(req)` and `FakeIo::default()` directly, and assert directly that no child ran and the primary is unchanged. `PullRequest::run` with `clock`, `PullRequestIo`, `assert_primary_untouched(&io)` and `complete_child_row` are introduced in slice C, where `clock`, `ChildExecution::role()` and `complete_child_row` land.
2. **R26's direct poll-retirement row (slice E).** Keep `CancelAfterPrimary` and assert what it produces: the direct record at step `published` with cleanup `cancelled` (as `completed_child_authority_is_consumed_before_the_cancellation`, closure/run_tests.rs about 1473-1491, pins), no reservation held, the context remotely deleted and the local primary holding the closure, so the poll's retirement meets a `Closed` candidate and must leave the context alone while the fence holds; then the same request completes. This row is in slice E's tests (§11.3's poll row and §12E's "branch only, R24" and open question 2's "not in the slices" are superseded).
3. **R25's supersedable-direct test.** Make the cleanup stop with a clean context worktree (for example a moved head), or restore it before the prepare, so the blockers are exactly `[AlreadyClosed]` and the answer is `ticket_closed`.
4. **A confirm or step-3 `busy`** is `Repository(RepositoryBusy)` in the domain and `busy` in the binding (not mapped, revision 3 point 3); the binding not naming the blocker on a confirm or step-3 stop is Task 10's stop data.


Designer's checkpoint for Task 7, written before any Task 7 test or code. Anchors are `file:line` in today's source. HEAD is `c1ddae12`, a ledger-only commit on `b13a53c0`, so every `src/` and `tests/` anchor is the same at `b13a53c0`. "The design" is `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-design.md`. Paths without a prefix are under `src/repository/`. This is revision 3.

## Revision 3 changes

The checker re-checked revision 2 and found six new problems; the controller ruled on them. Each is fixed here.

1. **E0392.** In slice C, `BranchEnd` has no lifetime: `pub(in super::super) enum BranchEnd { ContextGone }`, and the trait takes `end: BranchEnd`. Slice D adds `<'a>` together with `ContextChild(&'a SynchronizationError)` (section 4.5).
2. **Decision 33 before `busy`.** The prepare's read of a live `synchronize_context` row leaves out the unfinished record's own `context_sync_id` and `primary_sync_id`. So the binding's `held` answers for the holder's own stopped child, and a foreign synchronization alone answers `busy` (section 8.2). d14 gains the row the controller asked for: a stopped record's own kept child, then a new prepare through `f.service.prepare`, which answers `recovery_required` naming the holder.
3. **What each layer answers for a foreign live synchronization** (section 8.2). Not mapped, and the citation is corrected:
   - **The domain.** A new request's confirm runs `L::prepare(…)?` at run.rs:499-506, and `prepare_observation_at` propagates `observe_git`'s error (observe.rs:347-358). The `?` makes it `TicketClosureError::Repository(e)` with `e.kind == RepositoryBusy` (closure/mod.rs:344-348), never `mapped` (531). The hook's step-1 prepare answers the same.
   - **The binding** answers `busy`, through `stop_answer` (bind/lifecycle.rs:1302) and `repository_error_code` (read/mod.rs:299).
   - **Why not map.** The `?` at run.rs:506 is the direct path's, so mapping it would change a direct close's variant for every `RepositoryBusy` from its prepare. `Busy` also carries no blocker, so Task 10 would lose the row.
   - **The tests.** d5 and d14 assert `Err(Busy)` for the holder's own kept child: with fix 2, `begin_record` answers that through `mapped` at 571. A new d5 row asserts `Repository(RepositoryBusy)`, with the blocker naming the row, for a foreign synchronization at step 3.
4. **R24, the direct poll row.** The `BeforeRemoteDeletion` row is dropped: the close holds its parent reservation there, which refuses the poll (run.rs:1368-1371, 1428-1449). The check covers direct records by construction: it is the same `written_closure_unfinished` range. I dispute "a direct record cannot produce a `Closed` candidate while the fence holds it". Open question 2 gives the evidence and a reachable row at `publication_pending` after a cancelled primary child. That row is not in the slices until the controller rules.
5. **The interim R-35 path.** In slice C it answers `Err(L::Error::recovery_required())`, not `recovery::<L>` (section 4.1).
6. **R25, the observation's mode.**
   - A new request always observes in the live mode.
   - A same-request continuation observes in its record's mode. `Lifecycle::preview_request` gains `record: Option<&Self::Record>`, and closure's `PreviewRequest` carries that record's mode (sections 0.1 and 8.1).
   - A non-supersedable unfinished record answers decision 33's `recovery_required`.
   - A supersedable direct record (a cleanup stop) that a pull-request prepare meets: the prepare answers `ticket_closed`, blocker `already_closed`, under situation 1. The old record is untouched (section 8.1, with the evidence).
   - §2.4's "since no record exists at insertion" is corrected.

The checker's minors:
- d3 no longer asserts `ended_answer`; that stays in `a_branch_parent_view_is_never_integrated`.
- d11's fourth row has the full `local_prepared` evidence (remote/state.rs:3544-3580).
- d13's in-process row uses a wrapper of its own that does not complete its row.
- `pending_work`'s new read runs only in pull-request mode, so a direct prepare reads nothing new. This is listed in §0.1 with its guard.
- §6's refusal comes from `remote_identity_of` (`OperationMismatch`).
- The `stopped` anchor is bind/lifecycle.rs:679-697.
- `revalidate_under_lease` keeps the prepared mode.
- `written_commit()` is slice A's.

## Revision 2 changes

The checker's findings are in `task-7-checkpoint-findings.md`. Every one is fixed below, and the controller's rulings are applied.

- **Rulings on the open questions** (section "Rulings applied"):
  - Q1: option B, the `consented_context_oid` column.
  - Q2: a preflight-only `ClosureBlocker::RemoteContextUnmaterialized` (Task 9).
  - Q3: steps `Closing → PublicationPending → Published | Complete`; attempt numbering is internal.
  - Q4: `answer_code` has a CHECK on its shape only.
  - Q5: `BranchDeleted → Abandoned` is permitted for `new_branch`.
  - Q6: `item resolve-history` passes a branch record's hold. This is kept isolated in slice E.
  - Q7: row 5's error set.
- **`reconcile_completion` is now path-aware.** `ClosureRecord::remote_deletion_required()` is `false` for a branch record (sections 2.3 and 4.4). The revision 1 claim that it "stays generic" is withdrawn.
- **"Byte for byte" is gone.** Section 0.1 lists the behaviour-preserving refactors, each with the direct tests that guard it.
- **The binding's `ended_answer` and `stopped` are in this task.** `ParentView` gains `path` and `answered`. An answered branch parent is re-answered by the domain from `answer_code` (D19). This moved to slice C.
- **D20 is a design correction.** Today `operation list` lists a direct close's live child on its own (`read/status.rs` 705-737 skips only a parent's own row; 520-540 lists any row that holds the reservation). Branch records do the same, and d20 asserts that. The design's "its children not listed on their own" rests on a wrong Baseline Fact 12.
- **The poll's retirement check applies to every record the fence holds**, a direct one included. This is a direct behaviour change, with a direct test row of its own (sections 7 and 11).
- **Prepare's "unfinished synchronization of the ticket's context (busy, naming it)" has a mechanism.** The direct prepare has none to reuse; only the hook catches it at confirm (checkpoint.rs:648, 680-697). `pending_work` now reads the context's unfinished remote row, the branch observation answers `busy` with that row as its blocker, and the binding names it (section 8.2).
- **D5 and D7 order.** A context child's error is classified first, and the child is let go only on a D6 end (section 4.2).
- **The observation is chosen by the record's path.** A ticket's unfinished record decides the observation's mode, so a record continues in its own mode, direct ones included (R-2). A refusal stored with `observed: None` takes the mode the revalidation observed in (sections 8.1 and 2.4).
- **`checkpoint_oid`** is `None` for an abandoned record (section 1, Obligation 4).
- **Slice C compiles.** `BranchEnd::ContextGone` and a minimal `end_branch` are in C; `BranchEnd::ContextChild`, the D6 classification and `let_go` are in D. In C, `finish_branch` takes no `remote_current` parameter.
- **Mechanical edits added:** `closure/observe_tests.rs:1380-1382` `closure_digest_golden_vector` (slice B), and `mutation/bind/lifecycle_tests.rs:174-199` `impl LifecycleVerb for Fixed` (slice C).
- **Slice A has no dead code.** `ChildExecution::role()` and `BranchAnswer::name()` move to slice C, where they are read.
- **Visibility, to avoid error E0446.** `ClosureOutcome`, `BranchFinish`, `BranchStep` and `BranchEnd` are `pub(in super::super)`. The associated type is renamed `BranchFinish`, so it no longer shares a name with the `BranchAnswer` enum.
- **Test I/O fixes** (section 11.2):
  - each fetch is scoped to its role and applied before any stop;
  - `drive` serves either role, runs once, and completes its row;
  - stops can leave a reserved, kept child;
  - an acknowledgement path is added;
  - D3 and D21 use a real local update;
  - D16's poll row gets a real `Closed` candidate;
  - the sync-local row and the reservation unit test assert the right shapes;
  - D2 asserts the link;
  - `d6_` is in the table.
- **Smaller fixes:** anchor drift; the hook gets the root; `branch_link`'s configuration on the R-35 path; the `write` effect of a stop is stated as Task 10's.

## Rulings applied

The controller's rulings, as revision 2 applies them:

| Question | Ruling | Where it lands |
| --- | --- | --- |
| Q1 | Option B: the nullable `consented_context_oid` column | sections 2.2 and 4.3 |
| Q2 | A preflight-only `ClosureBlocker::RemoteContextUnmaterialized`, appended after `HistoryUnresolved` and left out of `CLOSURE_BLOCKERS` (record.rs:133-156), so the stored mask and its CHECK are unchanged | Task 9; noted in Obligation 10 |
| Q3 | `Closing → PublicationPending → Published \| Complete` | section 3 |
| Q4 | A shape-only CHECK; the value list is in Rust (`BranchAnswer::parse`) | section 2.2 |
| Q5 | `BranchDeleted → Abandoned` is permitted for `new_branch` | section 3 |
| Q6 | `item resolve-history` passes a branch record's hold, and the same request then continues | slice E only (section 7); if the owner overturns it, only that row of section 7 and its D16 row change |
| Q7 | Row 5 is the runner's configuration check plus the `Transport` kinds `PublicationRemoteMissing`, `EndpointChanged` and `ConfigurationInvalid` | section 9.1 |

Tests never assert an exact attempt number: `child()` replaces a minted, unrun attempt (run.rs:1012, 1020, 1087-1092).

The controller's rulings on revision 2's checker points:

| Point | Ruling | Where it lands |
| --- | --- | --- |
| R21 | D20 asserts that a child is listed on its own | §11.3 |
| R22 | The poll's retirement check covers every hold | §7 |
| R23 | Prepare's `busy` has a mechanism | §8.2 |
| R24 | The direct poll row is dropped | §7; open question 2 |
| R25 | A new request observes in the live mode | §8.1 |

## 0. Decisions in one place

- **The path.** A record's path is `(integration_mode, action)`. `(direct, close)` is a direct record and keeps every rule it has today. The one exception is the poll's retirement check, which R-17 extends to every hold (section 7). Every other pair is a branch record. The shared runner sees only `RecordPath { Direct, Branch }`.
- **The runner.** Branch records run a separate tail in `promotion/run.rs`: `promote_branch` and `finish_branch`. `promote_inner` dispatches to it right after the shared prefix (lookup, consent, `begin_record`, cancellation). The direct tail's behaviour is unchanged; the refactors it goes through are listed in section 0.1.
- **Ruling 3: "a branch outcome of its own".** `TicketClosureOutcome` (closure/mod.rs:245-268) is unchanged, so E-28 is not applied. `TicketClosureResult` gains `Branch(BranchClosureOutcome)` and `BranchIndexPending(IndexPending<BranchClosureOutcome>)`. That costs 3 mechanical arms (section 11.4).
- **Ruling 4.** The direct `record::begin_record` (record.rs:474-493) keeps its 7-argument signature. A new `record::begin_branch_record` serves branch records, so E-29 is not applied.
- **Ruling 5.** `PromotionIo` gets no new method: the publication child is a `synchronize` call with a context target (run.rs:21-40). E-32 is not applied, and `FakeIo`'s fields are unchanged (section 11.2).
- **Finished branch records answer from their record.** Every finished branch record, complete or abandoned by the runner with an answer, answers `Ok(TicketClosureResult::Branch…)` rebuilt from its stored `answer_code`. That holds on the first call, a same-request re-entry and a replay alike, in the domain and in the binding (D19). Stops stay `Err(…)`. A refusal recorded at insertion keeps today's `Err(refusal)`.
- **The clock.** `LifecycleRequest` gains a field `clock: fn() -> OffsetDateTime`. The builders default it to the wall clock, and the one test literal uses `..` (promotion/run_tests.rs:5432). The direct hook keeps its wall clock (checkpoint.rs:670, unchanged).
- **The observation's mode (R25).** A new request observes in the live configuration's mode. A same-request continuation observes in its record's mode (section 8.1).

### 0.1 Behaviour-preserving refactors of the direct path, and what guards each

The direct tests named here run in the slice that makes the refactor, by exact name. The `ticket_closure` and `document_promotion` SSH targets run at review-ready (Task 16).

| Refactor | Slice | Guarding direct tests |
| --- | --- | --- |
| `promote_inner`'s context-child block (run.rs:665-712) extracted into `context_step`. The abandon at 700 becomes a returned `ContextStep::Gone`, which the direct caller abandons exactly as today | C | closure run tests `pre_checkpoint_refusals_abandon_with_nothing_written_replay_and_a_new_id_works` (500), `cancellation_before_integration_is_interrupted_and_the_same_id_continues` (1378), `a_minted_context_attempt_that_never_reserved_has_no_delete_guard` (1750), `a_reset_that_leaves_the_context_contained_without_tracking_reconfirms` (1789); promotion run tests `every_interruption_reopens_with_one_candidate_and_only_unfinished_children` (491), `post_child_new_path_reconfirms_but_primary_only_merge_preserves_consent` (687), `remote_children_publication_cleanup_and_replay_are_once_only` (5189) |
| `build_closure` (checkpoint.rs:138-199) split into `build_closure_commit` plus the merge it already does | C | every `closure::checkpoint::tests` test; `process_exit_at_every_boundary_reopens_without_repeated_effects` (run_tests.rs:1181) |
| `reconcile_closing` (390-500) and `finish_context_move` (837-845) keyed on `written_commit()`, which equals `closure_oid` for every direct record | C | the ten `reconcile_*` tests (checkpoint_tests.rs:509-690); `never_abandons_after_the_checkout` / `_after_the_checkpoint` / `_with_an_adopted_descendant` (run_tests.rs:932-943) |
| `child()`'s cancel-and-acknowledge (run.rs:1045-1061) extracted into `acknowledge_cancellation`, called by `child()` and `let_go` | D | promotion `ownership_first_attempt_matrix_both_children_has_no_permanent_strand` (1008), `uncertain_child_ownership_fails_closed_without_replacement_and_primary_returns_pending` (1100), `cancel_at_pre_ref_and_each_local_boundary_stops_before_the_whole_step` (2203), `cancellation_before_and_after_integration_preserves_and_retry_finishes` (5318) |
| `observe_git` (604-1037) takes the mode as a parameter and guards the direct-only inserts with `if !branch` | B | every `closure::observe::tests` test, `closure_digest_golden_vector` (observe_tests.rs:1380) included: its golden must not change |
| `child_allowed(step, role)` becomes `child_allowed(&ParentFacts, role)` | A | `child_fence_admits_closure_primary_only_while_integrated_or_publication_pending` (reservation_tests.rs:4386), the child-fence wording tests (4716-4746), promotion `record_tests` 201 |
| `Lifecycle::refuse_record`'s `remote: Option<&str>` becomes `config: &RepositoryConfig` (both impls read `config.publication_remote`) | B | `pre_checkpoint_refusals_abandon_…` (500), `a_changed_or_missing_identity_since_the_preview_is_external_change` (565), promotion `new_blocker_is_stored_and_replayed_before_any_child` (5133) |
| `ClosureRecord::remote_deletion_required` made path-aware (direct unchanged) | C | `a_child_publication_guards_the_delete_and_the_checkpoint_is_retired` (1615), `without_a_child_or_tracking_context_nothing_is_reserved_or_deleted` (1727) |
| `Lifecycle::preview_request` gains `record: Option<&Self::Record>` (R25). Its three call sites: run.rs:256's `preview` helper, used at 672 and 726 with `Some(&r)`; run.rs:501 with `None`; integrate.rs:122 with `parent`. Closure's `type PreviewRequest` becomes `ClosurePreviewRequest { request: PreviewTicketClosureRequest, mode: Option<Integration> }`, and promotion's impl ignores the argument. A direct record's continuation therefore keeps today's direct observation after a switch to pull-request mode | B | `a_resumed_attempt_rechecks_under_the_lease_without_a_consent_preview` (2496), `a_confirmed_close_takes_at_most_two_previews_and_no_whole_repository_observation` (2464), every `promotion::run::tests` test |
| `pending_work` (observe.rs:404-448) gains the live `synchronize_context` read (section 8.2), run only when the observation's mode is pull request. A direct prepare reads nothing new | B | every `closure::observe::tests` test, `closure_digest_golden_vector` (1380) included |

## 1. Design Checkpoint Obligations

**Obligation 1, path accessors.**
- `promotion/lifecycle.rs:171` `LifecycleRecord` gains `fn path(&self) -> RecordPath { RecordPath::Direct }` (slice A) and `fn answered(&self) -> bool { false }` (slice C).
- `promotion/lifecycle_record.rs:262` `ParentRecord` gains `fn closure_step(&self) -> Option<ClosureStep> { None }`. `ClosureRecord` overrides it with `Some(self.step)`.
- `ClosureRecord` (record.rs:166-197) gains:
  - `path()`: `Direct` iff `(integration_mode, action) == (Integration::Direct, ClosureAction::Close)`;
  - `written_commit()`: `withdraw_oid` for `withdraw`, else `closure_oid`. It lands in slice A with the three readers below;
  - `answered()`: `answer_code.is_some()`.
- `ParentFacts` (lifecycle_record.rs:475-516) gains `path: RecordPath` and `closure_step: Option<ClosureStep>`, filled in `ParentFacts::of` (484). `child_allowed` reads them (section 5).
- `effect_written()` (record.rs:891-893) becomes `written_commit().is_some()`. `written_checkpoint()` (817-819) becomes `written_commit()`.
- `merged_context()` (198-207): unchanged for direct. For branch it is `written_commit().or(context_oid)`. A `new_branch` record before `closing` gives `retired_oid`, which Task 9 uses.
- Resume routes by path and step, not by `integration_oid`:
  - the branch dispatch after run.rs:604;
  - the answered checks before run.rs:461 and before run.rs:573;
  - inside `promote_branch`, `r.primary_sync_id().is_some()` goes straight to `finish_branch` (design R-3). Attempt numbering is internal.

**Obligation 2, the branch hook (R-34).** `closure/checkpoint.rs` gains `branch_step` (section 4.3), built from the direct hook's parts:
- the lease, as `integrate.rs:136`;
- `context_synchronization_unfinished` (record.rs:788), checked before the write and inside the `closing` transaction (as checkpoint.rs:648, 680-697);
- the identity against the consented `closed_by` (as checkpoint.rs:659-668);
- `closed_at` from `request.clock`, stored in the `closing` UPDATE;
- `build_closure_commit`, the checkpoint-only builder;
- `begin_branch_closing`, the per-action variant of `begin_closing`;
- `move_context_to_checkpoint_with` (checkpoint.rs:314);
- the reconciliation generalized to `written_commit()`.

These do not apply to a branch record: `primary_advanced`, "the context holds the base" (625-644), `current_base` (811), `post_checkpoint_result` (253), `adopt_written_closure` (record.rs:640) and `resume_written` (checkpoint.rs:728). `base_oid` is stored as the tracking primary the hook read. The preconditions hold without a `LeasedIntegration`: the hook takes its own lease, and the parts take the root repository, the registry, the confirmation and the context worktree's repository (`context_repository`, checkpoint.rs:292).

**Obligation 3, the rebuild.** Section 2.

**Obligation 4, completion and answers.**
- **Completion.** `record::complete_branch_record` completes a branch record with no retirement. `record::abandon_branch_record` abandons one with its refusal and `answer_code` (section 2.4). `reconcile_completion` (lifecycle_record.rs:1001-1025) is reached after an uncertain completion commit. It works for branch records because `ClosureRecord::remote_deletion_required()` answers `false` for them (section 2.3): a branch `close` has no parent remote row, so 1019-1021 no longer refuses.
- **Domain re-entry and replay** read `answer_code` through `answered()`:
  - before run.rs:461 and 573: `if r.answered() { return Ok(refresh::<L>(…, true)); }`;
  - in `run_observed` (370), before the `integration_oid` test: `if r.path() == RecordPath::Branch { if r.answered() { return Ok(refresh::<L>(service, &root, &r, true)); } index_children_owed(service, &root, &r); return Err(error); }`.
- **Binding re-entry (slice C).**
  - `ParentView` (promotion/mod.rs:365-386) gains `path: RecordPath` and `answered: bool`, filled in `lifecycle_parent` (427-438) and carried by `of_blockers` (392-408).
  - `ended_answer` (bind/lifecycle.rs:853-879) returns `None` for `parent.answered`. `row` (1089-1110) then answers `Row::Unfinished`, and the rerun reaches the domain, which answers from `answer_code`. Today `ended_answer` would answer an abandoned parent from its refusal alone (`ExternalChange`/`ReconfirmationRequired` → `external_change`).
  - `stopped` (615-716) treats an answered parent as it treats a `Complete` one (the arm at 679-697): the same-request recovery and `Standing::Owed`.
- **Outcome types.** `TicketClosureOutcome` is unchanged. The new public `BranchClosureOutcome` (closure/mod.rs) holds, each with an accessor:
  - `operation_id`, `action: ClosureAction`, `answer: BranchAnswer`;
  - `checkpoint_oid: Option<Oid>`: `written_commit()` for a `complete` record, `None` for an abandoned one, since an abandon from `closing` in the `NothingWritten` case holds an unreferenced `closure_oid`;
  - `publication: Option<PromotionPublication>`, `Some` only when the publication child completed.
- **`make_outcome`** (run.rs:1898-2007) returns `L::branch_outcome(r, publication)` for a branch record, before the `expect` at 1904-1907.
- **The closure's lifecycle.** `TicketClosureLifecycle::Outcome` becomes `pub(in super::super) enum ClosureOutcome { Direct(TicketClosureOutcome), Branch(BranchClosureOutcome) }`. It is not module-private, because it is a `pub(in super::super)` trait's associated type (error E0446). `complete` and `index_pending` (closure/lifecycle.rs:438-443) map it.
- **The binding's verb.**
  - `Reached` (bind/lifecycle_verb.rs:72-78) is unchanged.
  - `LifecycleVerb::call` (113-123) returns `Option<Result<Called, Stop>>`, with `pub(in super::super::super) enum Called { Reached(Reached), Branch { outcome: BranchClosureOutcome, index_pending: bool } }`. `Promote` wraps in `Called::Reached`. `Close::call` (430-434) maps the new variants to `Called::Branch`. `closure_reached` (260-268) is unchanged.
  - `LifecycleBinding::call` (bind/lifecycle.rs:776) sends `Called::Branch` to a new `branch_reached`: the code from 9.3, effects (`checkpoint: Committed` when `checkpoint_oid` is some; `publication` from the outcome; `discovery` from `index_pending`), no data, standing `Final`.
  - Task 10 replaces the data and effects with `PullRequestClosureMutationDto`, and adds a stop's `write` and `checkpoint` `committed` through `parent_effects`, which is in Task 10's file list (R-5). Task 7's stop effects are the before-integration shape (Obligation 8).

**Obligation 5 (Task 8's; room left).** `ClosureAction::Withdraw`, `withdraw_oid`, its `closing` CHECK, `written_commit()` and the generalized reconciliation exist from this task. A withdraw `Lifecycle` over `ClosureRecord` needs only its observation and digest, `BranchWrite::Withdraw { commit }` and its answer rows. `new_branch` (Task 9) has its action, `retired_oid`, `pending_ref`, step `retired`, permits and CHECKs here. The close observation keeps today's blockers for Part D's situations 2 and 3 until Task 9.

**Obligation 6, R-30.** Met by Task 6:
- `remote/sync.rs` `push_unless_unclosed`, just before the push;
- `ended_if_unresolved` (sync.rs:4336-4357) keeps a lifecycle child stopped by `TicketClosed`;
- the seam is ruling 12's recording-push test.

Task 7 consumes it: answer row 4, and D6's `context_ticket_closed`.

**Obligation 7, the publication child's error.** `finish_branch` returns a stopped publication child's error as `Err(L::Error::context_synchronization(Box::new(error)))`, the context child's constructor (run.rs:691). It stores `publication_error`'s category (1162-1180). The binding maps the error to `Stop::ContextSynchronization` (lifecycle_verb.rs:446-448), which answers as `item sync` does (R-5). There is no new error variant.

**Obligation 8, the binding's sites.**
- `Stages::event` (bind/lifecycle.rs:160-183, slice A): `ChildStarting(Publication) => Publishing`, and `ChildSafePoint(Primary | Publication, _) => return`. The design is silent on the publication child's stages; this mirrors the primary child's.
- `held_by_its_child` (1359-1374): unchanged. It reads both slots (1371). The withdraw arm comes with the verb (Task 11).
- `ParentView` and `ended_answer`/`stopped`: Obligation 4 (slice C).
- `parent_integrated` (1165-1186, slice C) answers `false` for `path == Branch`. So `parent_effects` (1206-1298) and `bind/mod.rs:1079-1083`'s recorded effects give a branch record the before-integration shape. Task 10 adds the publication child's push and `write`.
- `way_out.rs` and `mutate.rs` belong to Tasks 10 and 11.

**Obligation 9, the conflict lookups.** These are Task 12's (`remote/state.rs` 3828 and 3860; callers `conflict_read.rs:449`, `bind/conflict.rs:196`, `records.rs:845`). Task 7 makes them decidable in SQL:
- current child: `CASE WHEN NOT (integration_mode='direct' AND action='close') AND primary_attempt>0 THEN primary_sync_id ELSE context_sync_id END`;
- reverse map: `context_sync_id=?2 OR (NOT (integration_mode='direct' AND action='close') AND primary_sync_id=?2)`.

**Obligation 10, no new stored closure blocker.**
- `ClosureBlocker` (closure/mod.rs:36-60), `CLOSURE_BLOCKERS` (record.rs:133-156) and the mask CHECK are unchanged in Task 7.
- A confirm whose situation changed differs in its digest, so it answers `external_change` through run.rs:518-527.
- The root's `WorktreeConflicted` condition is `PrimaryConflicted` (ruling 6).
- Task 9 adds the preflight-only `RemoteContextUnmaterialized` under Q2's ruling.
- The prepare's "busy, naming it" is an error code with a blocker, not a `ClosureBlocker` (section 8.2).

**Obligation 11, `Integration`.** `integration_mode` decodes into `crate::canonical::Integration` (canonical.rs:90-102). There is no new mode enum.

## 2. The rebuilt record

### 2.1 The frozen v2 DDL and the migration

- record.rs:229-266: today's `SCHEMA` text is kept verbatim as `const SCHEMA_V2: &str`, with a doc line saying it is the `closure_refusal_blockers_v2` rebuild's table, frozen there. `SCHEMA` becomes the v3 text in 2.2. `INDEX` (267) is unchanged.
- New constants:
  - `MODES_MARKER: &str = "wave_06_closure_modes_v1"`;
  - `V2_COLUMNS: &str`, the 25 v2 column names in table order (`repository_id` through `adopted_context_oid`).
- `migrate` (272-313):
  - the created path (286) runs the v3 `SCHEMA` and inserts `MARKER`;
  - the v2 rebuild at 304 interpolates `{SCHEMA_V2}`, not `{SCHEMA}`; its `SELECT *` stays, because v1 and v2 share the 25 columns;
  - after 311 comes a third block:
    ```rust
    let modes: bool = /* EXISTS registry_migrations name=MODES_MARKER */;
    if !modes {
        // The path columns: rebuild a table that predates them. Existing rows read
        // `direct` and `close` (their column defaults); the index goes first.
        if !created {
            tx.execute_batch(&format!("DROP INDEX ticket_closures_unfinished; ALTER TABLE ticket_closures RENAME TO ticket_closures_before_modes; {SCHEMA}; {INDEX}; INSERT INTO ticket_closures({V2_COLUMNS}) SELECT {V2_COLUMNS} FROM ticket_closures_before_modes; DROP TABLE ticket_closures_before_modes;")).map_err(|_| state::recovery_required())?;
        }
        tx.execute("INSERT INTO registry_migrations(name) VALUES(?1)", [MODES_MARKER]).map_err(|_| state::recovery_required())?;
    }
    validate_schema(tx)
    ```
- What a registry gets:

  | Registry | Result |
  | --- | --- |
  | fresh | v3 and three markers, no rebuild |
  | v1 | two rebuilds in the one registry-schema transaction (discovery.rs:2749) |
  | v2 | one rebuild |

### 2.2 The v3 `SCHEMA`

- Columns 230-254 are unchanged except `step`, which gains `'retired'`:
  `step TEXT NOT NULL CHECK(step IN ('confirmed','context_synchronized','closing','integrating','integrated','published','publication_pending','remote_deleted','worktree_removing','worktree_removed','branch_deleted','retired','complete','abandoned','superseded')),`
- Appended after `adopted_context_oid`. `<oid x>` is today's OID shape:
  ```sql
  integration_mode TEXT NOT NULL DEFAULT 'direct' CHECK(integration_mode IN ('direct','pull_request')),
  action TEXT NOT NULL DEFAULT 'close' CHECK(action IN ('close','withdraw','new_branch')),
  withdraw_oid TEXT CHECK(<oid withdraw_oid>),
  retired_oid TEXT CHECK(<oid retired_oid>),
  pending_ref TEXT CHECK(pending_ref IS NULL OR pending_ref='refs/manyhands/closures/'||operation_id),
  answer_code TEXT CHECK(answer_code IS NULL OR (typeof(answer_code)='text' AND length(answer_code) BETWEEN 1 AND 64 AND answer_code NOT GLOB '*[^a-z_]*')),
  consented_context_oid TEXT CHECK(<oid consented_context_oid>),
  ```
- The stored DDL carries no SQL comments, because `validate_schema_object` compares the text exactly.
- `consented_context_oid` is Q1's column: the head the branch digest bound at confirm, written by `begin_branch_record`.
- `answer_code`'s values are checked in Rust (`BranchAnswer::parse` in `decode`); that is Q4's ruling.
- The table CHECKs at 255-262 are kept verbatim.
- Three changed CHECKs replace 263, 264 and 265:
  ```sql
  CHECK(closed_at IS NULL OR closure_kind IS 'record' OR action='new_branch'),
  CHECK(adopted_context_oid IS NULL OR (closure_oid IS NOT NULL AND adopted_context_oid!=closure_oid AND closure_kind IS 'record' AND integration_mode='direct' AND action='close')),
  CHECK(step!='closing' OR (base_oid IS NOT NULL AND context_oid IS NOT NULL AND (
      (action='close' AND closure_kind IS 'record' AND closed_at IS NOT NULL)
   OR (action='close' AND integration_mode='pull_request' AND closure_kind IS 'existing' AND closed_at IS NULL)
   OR (action='withdraw' AND closure_kind IS 'existing' AND closed_at IS NULL AND withdraw_oid IS NOT NULL)
   OR (action='new_branch' AND closed_at IS NOT NULL)))),
  ```
- New CHECKs:
  ```sql
  CHECK(action!='new_branch' OR integration_mode='pull_request'),
  CHECK((integration_mode='direct' AND action='close') OR (integration_oid IS NULL AND integration_kind IS NULL AND step NOT IN ('integrating','integrated','remote_deleted'))),
  CHECK(NOT (integration_mode='direct' AND action='close') OR (withdraw_oid IS NULL AND retired_oid IS NULL AND pending_ref IS NULL AND answer_code IS NULL AND consented_context_oid IS NULL AND step!='retired')),
  CHECK(step NOT IN ('worktree_removing','worktree_removed','branch_deleted','retired') OR (integration_mode='direct' AND action='close' AND step!='retired') OR action='new_branch'),
  CHECK(step!='context_synchronized' OR action!='new_branch'),
  CHECK(withdraw_oid IS NULL OR (action='withdraw' AND step NOT IN ('confirmed','context_synchronized'))),
  CHECK(retired_oid IS NULL OR action='new_branch'),
  CHECK(pending_ref IS NULL OR (action='new_branch' AND closure_oid IS NOT NULL AND step NOT IN ('complete','abandoned','superseded'))),
  CHECK(answer_code IS NULL OR step IN ('complete','abandoned')),
  CHECK(step!='complete' OR (integration_mode='direct' AND action='close') OR answer_code IS NOT NULL)
  ```
- A direct row's `closing` CHECK is today's. The unique unfinished index keeps one unfinished record per ticket, of any action. The record never holds `closed_by`, a title, a body or a URL.

### 2.3 Rust types (record.rs, closure/mod.rs)

- `ClosureStep` (record.rs:30-36) gains `Retired=>"retired"` between `BranchDeleted` and `Complete`.
  - `From<ClosureStep> for ParentStep` (92-110) maps `Retired => BranchDeleted`.
  - `set_step` (862-867) keeps `Retired` when it is given `BranchDeleted`, as it keeps `Closing`.
- `closure/mod.rs` gains `pub enum ClosureAction { Close, Withdraw, NewBranch }` and `pub enum BranchAnswer { … }` (values in 9.2; `Copy`; a categories-only `Debug`).
  - `ClosureAction`'s `name`/`parse` land in slice A, where `insert_record` and `decode` read them.
  - `BranchAnswer::parse` lands in A (read by `decode`); `name` lands in C (read by the writers). No slice leaves dead code.
- `RecordPath { Direct, Branch }` is `pub(in super::super)` in `promotion/lifecycle.rs`.
- `ClosureRecord` gains these fields, decoded in `decode` (333-363) and immutable in `save_record`: `integration_mode: Integration`, `action: ClosureAction`, `withdraw_oid`, `retired_oid`, `pending_ref: Option<String>`, `answer_code: Option<BranchAnswer>`, `consented_context_oid`. The `Debug` (208-225) adds `integration_mode`, `action`, `answer_code`, `withdraw_oid` and `retired_oid`.
- `validate` (365-442) is per path. Direct rows run today's body, plus a refusal of any branch-only field or of `Retired`. Branch rows replace 395-402 and 407-423 with these rules:
  - `integration_oid`, `integration_kind` and `adopted_context_oid` are none, and the step is not `Integrating`, `Integrated` or `RemoteDeleted`;
  - `context_oid` from `context_synchronized` (from `closing` for `new_branch`);
  - `base_oid` from `closing`;
  - the per-action rules of 2.2's `closing` CHECK;
  - `closed_at` before `closing` only for `new_branch`;
  - `withdraw_oid` only for `withdraw` from `closing`;
  - `retired_oid` and `pending_ref` only for `new_branch`;
  - `cleanup` only for `new_branch` at `worktree_removing` through `branch_deleted`;
  - `complete` needs `answer_code`, and `answer_code` only at `complete` or `abandoned`;
  - at `publication_pending` the publication is none or not published/current;
  - no publication rule at `published` or `complete`;
  - `remote_name` required.
- `save_record` (545-591):
  - the seven new fields join the unchanged-field checks at 556-579;
  - the permits call at 568-570 dispatches by path: `old.step.permits(…)` for direct, `old.step.branch_permits(r.step, old.action)` for branch.
- `supersedable` (460-463) becomes `old.path() == RecordPath::Direct && <today's>`.
- `remote_deletion_required` (`deletion_reservation_required`, 768-770; impl 925-927) becomes `r.path() == RecordPath::Direct && <today's>` (slice C). A branch record reserves no parent row for a remote delete.
- `insert_record` (519-541) gains `path: (Integration, ClosureAction)` and `consented: Option<Oid>`. `begin_record` passes `(Direct, Close)` and `None`.

### 2.4 Record writers (record.rs; each re-reads the record through `read_record`, which validates it)

```rust
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn begin_branch_record(tx: &Transaction<'_>, id: i64, op: OperationId, item: &ItemId,
    expectation: ClosureExpectation, remote: Option<&str>, kind: ClosureKind,
    path: (Integration, ClosureAction), consented: Option<Oid>) -> Result<ClosureRecord, RepositoryError>;
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn refuse_record_on(/* refuse_record's arguments */, path: (Integration, ClosureAction)) -> Result<ClosureRecord, RepositoryError>;

pub(in super::super) enum BranchWrite { Closure { closed_at: OffsetDateTime, commit: Oid }, Existing } // Task 8: Withdraw
pub(in super::super) fn begin_branch_closing(tx: &Transaction<'_>, repository_id: i64,
    operation: OperationId, base: Oid, write: BranchWrite) -> Result<(), RepositoryError>;
pub(in super::super) fn begin_publication(tx: &Transaction<'_>, id: i64, op: OperationId) -> Result<ClosureRecord, RepositoryError>;
pub(in super::super) fn complete_branch_record(tx: &Transaction<'_>, id: i64, op: OperationId,
    answer: BranchAnswer, link: Option<&PullRequestLink>, given_at: i64) -> Result<(), RepositoryError>;
pub(in super::super) fn abandon_branch_record(tx: &Transaction<'_>, id: i64, op: OperationId,
    refusal: &Refusal, answer: BranchAnswer, link: Option<&PullRequestLink>, given_at: i64) -> Result<(), RepositoryError>;
```

- `begin_branch_record` follows `begin_record` (474-493): the existing record is returned; a supersedable direct record is superseded; anything else is `busy`. It inserts the path and `consented`.
- **Refusals at insertion.** `Lifecycle::refuse_record` (promotion/lifecycle.rs:391-400) replaces `remote: Option<&str>` with `config: &RepositoryConfig`, the configuration the revalidation observed with (run.rs:507). Both impls read `config.publication_remote`. The closure impl (closure/lifecycle.rs:223-243) stores:
  - the observed path when `observed` is some;
  - otherwise `(config.integration(), Close)`. A refusal at insertion belongs to a new request, which always observes in the live mode (R25), so the mode the revalidation observed in is the configuration's, whatever unfinished record the ticket has.

  It calls `record::refuse_record_on`. The direct `record::refuse_record` keeps its signature and passes `(Direct, Close)`.
- `begin_branch_closing` requires step `ContextSynchronized`, a branch path, action `Close`, a kind matching the write (`Closure`↔`Record`, `Existing`↔`Existing`), and a non-zero `base`. It runs `UPDATE step='closing', base_oid, closed_at, closure_oid`. The caller re-checks `context_synchronization_unfinished` in the same transaction.
- `begin_publication` requires a branch path, `Closing` and `primary_attempt == 0`. It runs `parent::next_attempt::<ClosureRecord>(tx, id, op, ChildRole::Publication)`, then `UPDATE step='publication_pending'`, in one transaction.
- `complete_branch_record` requires:
  - a branch path, step `PublicationPending` or `Published`, and a post-child answer (9.2);
  - the second slot's row, when present, not active without authority (R-36).

  It runs `UPDATE step='complete', answer_code, cleanup_category=NULL, cancel_requested=0`, and `pull_request_links::upsert_link` when `link` is some. It does no retirement and no `finish_parent`.
- `abandon_branch_record` requires:
  - a branch path;
  - step `Confirmed`, `ContextSynchronized`, or `Closing` with nothing written (caller-proved);
  - the context slot's row, when present, not active without authority.

  It runs `UPDATE step='abandoned', refusal_category, refusal_blockers, answer_code`, plus `upsert_link` for `AlreadyClosing`.
- `pull_request_links.rs` gains `pub(super) fn upsert_link(connection: &Connection, repository_id: i64, kind: &str, item_id: &str, link: &PullRequestLink, given_at: i64) -> rusqlite::Result<()>`. It runs `INSERT … ON CONFLICT(repository_id,kind,item_ulid) DO UPDATE SET source, target, forge, url, given_at`, with the forge written by the stored name `link_of` (pull_request_links.rs:11-33) parses.
- **The link.** It is built outside the transaction by `checkpoint::branch_link(repo, primary: &str, forge: Option<&ForgeSetting>, remote: &str, item) -> PullRequestLink`: `forge::pull_request_link(forge::recognise(<git2 URL of remote, if it exists>, forge).as_ref(), &context_branch, primary)` (R-25).
  - With a valid configuration, `primary` and `forge` are that configuration's.
  - On the R-35 path (no valid configuration) they come from the configuration in `base_oid`'s tree, as `recorded_primary` (run.rs:417-433) reads it, and the URL is none when the remote is gone.
  - `given_at` is `request.clock().unix_timestamp()`.

## 3. `permits` per path

**Direct.** `ClosureStep::permits` (record.rs:64-87) is unchanged, and `Retired` is never a source or target. Its transitions, as today:
- `Confirmed→ContextSynchronized`
- `ContextSynchronized→Closing` (record) or `ContextSynchronized→Integrating` (existing)
- `Closing→Integrating`
- `Integrating→Integrated`
- `Integrated→Published|PublicationPending`
- `PublicationPending→Published`
- `PublicationPending→WorktreeRemoving` (no remote)
- `Published→RemoteDeleted`
- `RemoteDeleted→WorktreeRemoving`
- `WorktreeRemoving→WorktreeRemoved`
- `WorktreeRemoved→BranchDeleted`
- `Confirmed|ContextSynchronized|Closing→Abandoned`

`Complete` and `Superseded` are written by `complete_record` and `supersede`.

**Branch.** `fn branch_permits(self, next: Self, action: ClosureAction) -> bool`. Terminal steps permit nothing; self loops are allowed.

| Action | Transitions |
| --- | --- |
| `close`, `withdraw` | `Confirmed→ContextSynchronized`, `ContextSynchronized→Closing`, `Closing→PublicationPending`, `PublicationPending→Published\|Complete`, `Published→Complete`, `Confirmed\|ContextSynchronized\|Closing→Abandoned` |
| `new_branch` | `Confirmed→WorktreeRemoving\|Closing`, `WorktreeRemoving→WorktreeRemoved`, `WorktreeRemoved→BranchDeleted`, `BranchDeleted→Retired`, `Retired→Closing`, `Closing→PublicationPending`, `PublicationPending→Published\|Complete`, `Published→Complete`, `Confirmed\|BranchDeleted\|Retired→Abandoned` |

How branch differs:
- no integration steps;
- `Closing→PublicationPending` with the mint (Q3's ruling);
- `PublicationPending→Complete` (rows 1 to 5);
- no remote delete;
- never superseded;
- `BranchDeleted→Abandoned` for `new_branch`, for F8's re-check (Q5's ruling).

A branch record leaves `Closing` for `Abandoned` only while reconciliation proves nothing written, with the direct rule's meaning (ruling 8, section 10).

## 4. The path in the runner

### 4.1 `promote_inner` (run.rs:434-870)

1. **Prior-record block (448-480).**
   - Before 461: `if validated.answered() { return Ok(refresh::<L>(…, true)); }`.
   - At 473-479 add `|| validated.path() == RecordPath::Branch && validated.primary_sync_id().is_some()`. Such a record, under a configuration that is not valid, goes to `finish_branch` with `config: None` (R-35, slice D), with `recorded_primary(&root, &validated)`.
     - In slice C it answers `Err(L::Error::recovery_required())`, with the record unchanged.
     - It must not answer `recovery::<L>`: that sets `cleanup = RecoveryRequired`, which a branch record's `validate` refuses, so the save would be dropped. It would also build an outcome for an unanswered record.
2. **Prepare, revalidation and consent (481-535)** are unchanged in shape. The new request's prepare (501) passes `record: None`, so it observes in the live mode (R25). A foreign live synchronization answers `Repository(RepositoryBusy)` here, through the `?` at 506 (section 8.2). D17 and D18 answer `external_change` through the compare at 521, with the refused row stored through `L::refuse_record(…, &config, …)` at 547-557.
3. **Begin (536-571).** `TicketClosureLifecycle::begin_record` (closure/lifecycle.rs:193-213) calls `record::begin_branch_record` for a branch observation, with `consented = Some(observation.context_oid)`.
4. **After begin.** Before 573: `if r.answered() { return Ok(refresh::<L>(…, true)); }`. Cancellation clearing (584-590) is shared.
5. **Branch dispatch, after 604.** `if r.path() == RecordPath::Branch { return promote_branch::<L>(lifecycle, service, &request, &config, &bound_primary, &mut r, existing, io, progress); }`.
6. **Lines 606-870 serve direct records.** The context block (665-712) is extracted into `context_step` (section 0.1).

### 4.2 `promote_branch` (new, run.rs)

```rust
fn promote_branch<L: Lifecycle>(lifecycle: &L, service: &RepositoryService, request: &LifecycleRequest<L::Expectation>,
    config: &RepositoryConfig, primary: &str, r: &mut L::Record, existing: bool,
    io: &mut impl PromotionIo, progress: &mut dyn RunObserver) -> Result<L::Output, L::Error>
```

1. **The publication child exists.** If `r.primary_sync_id().is_some()`, return `finish_branch(…, Some(config), …)`.
2. **Before `closing`.**
   - `r.remote_name() != config.publication_remote` answers `Err(interrupted)` (as 653-655). At `closing` this is skipped: `finish_branch` answers it as row 5.
   - `AfterConfirmed` when `!existing`.
   - The cancellation check, with `release_child(Context)` at `Confirmed` (as 659-664).
3. **At `Confirmed`, `context_step`.** On a child error, classify first:
   - **Cancelled:** `release_child(Context)`, then `Err(interrupted)` (as 687-690).
   - **A D6 end (slice D).** `fn ends_context_child(e: &SynchronizationError) -> bool` is `matches!(e, RemoteContextDeleted | HistoryUnresolved | TicketClosed)`: the stops `ended_if_unresolved` keeps for a lifecycle child, or that only `request cancel` lets go (Fact 15). On a D6 end: `let_go(Context)` (4.4), then `L::end_branch(service, root, r, BranchEnd::ContextChild(&e))`, then `Ok(refresh::<L>(…, false))` on the reloaded record.
   - **Any other stop (D5, D7):** `Err(L::Error::context_synchronization(Box::new(e)))`. The child is left as it stands, retained or not, for `child()` on the same request, and the record stays `confirmed`.
   - **In slice C** there is no D6 branch: every non-cancelled error is the D5/D7 return.
   - **`ContextStep::Gone`** (the context branch is gone after the child): `L::end_branch(…, BranchEnd::ContextGone)`, then `Ok(refresh)`. This is in slice C.
   - **On success:** step `context_synchronized`, saved, then `AfterContextSync` (as 707-711).
4. **Cancellation.** The check at 713-715.
5. **Recording.** `progress.event(RunEvent::Recording)` under 766-767's condition.
6. **Step 3.** `lifecycle.branch_step(service, request, r)?`:
   - `BranchStep::Closing`: reload, then `finish_branch(…, Some(config), …)`.
   - `BranchStep::Finished`: reload, then `Ok(refresh)`. The hook abandoned the record with its answer.
   - Errors (`busy`, repository) return with the record unchanged.

### 4.3 The hook: `Lifecycle::branch_step`, implemented in checkpoint.rs

`TicketClosureLifecycle::branch_step(&self, service, request, r)` calls `checkpoint::branch_step(service, &request.root, request.confirmed_identity.as_ref(), (request.clock)(), r)`. In order:

1. **Prepare and lease.**
   - `observe::prepare_observation_in(service, &request, Some(r.integration_mode))` for the item, outside the lease. Its mode is the record's (R25), so the record continues in its own mode after a switch.
   - A foreign live synchronization of the context (not the record's own child) answers `Err(TicketClosureError::Repository(e))` here, with `e.kind == RepositoryBusy` and its blocker naming the row. The binding answers `busy` (8.2). In the direct close the same case is the hook's `E::busy()` at checkpoint.rs:648.
   - Then `repository_lease(&repository, root, RepositoryOperation::Read)`.
2. **At `closing`.**
   - Kind `existing` returns `Closing`.
   - Kind `record` runs `reconcile_closing(context_repo, r)` on `written_commit()`:
     - `CheckoutDone`: `context_moving`, `finish_context_move`, `context_moved_item`, `AfterClosureCheckpoint`, then `Closing`.
     - `CheckpointWritten`: `context_moved_item`, then `Closing`.
     - `Unrecognized`: `Err(recovery_required)`, with the record kept.
     - `NothingWritten`: continue below as `closing`.

   `context_moved_item` marks only the item context, as `context_moving` (567-578) does.
3. **Tip check.** A tip of the context ref other than `r.context_oid` abandons with `ExternalChange`/`external_change` (as 622-624).
4. **3.1.** The tracking primary (`refs/remotes/<remote>/<primary>`) names a closure (`commit_names_closure`, 239-248): abandon with `Blocked([AlreadyClosed])`/`AlreadyClosed`.
5. **3.2.** The tip names a closure:
   - kind `record`: abandon with `ExternalChange`/`AlreadyClosing` and `link`;
   - kind `existing`: continue to step 6.
6. **Revalidate.** `observe::revalidate_under_lease(&prepared, service, &repository)`. It keeps the mode the prepare was made in (8.1).
   - Blocked: abandon with `ReconfirmationRequired`/`reconfirmation_required` (R-18; as run.rs:786-793).
   - Ready: `observe::branch_expectation_with_head(&obs, r.consented_context_oid) == r.expectation`, else abandon with `ExternalChange`/`external_change` (D21; an identity changed since consent).
   - Kind `existing` with the proof holding: `begin_branch_closing(tracking, BranchWrite::Existing)`, then `Closing`.
7. **3.3, kind `record`, open tip.**
   - `context_synchronization_unfinished` gives `Err(busy)`.
   - The live identity against `obs`'s `Record { closed_by }` (as 659-668); a mismatch abandons with `ExternalChange`.
   - `closed_at = r.closed_at.unwrap_or_else(|| whole_utc_seconds(now))`.
   - `build_closure_commit(repo, item, path, tip, closed_at, &identity)`; `Invalid` abandons with `ReconfirmationRequired`.
   - One transaction: `context_synchronization_unfinished`, then `begin_branch_closing(tracking, Closure { … })`, or at `closing` `replace_unmoved_closure` (619-631).
   - `ClosureAfterClosingRecorded`, `context_moving`, `move_context_to_checkpoint_with(…, between: ClosureDuringCheckpointCheckout)`, `context_moved_item`, `AfterClosureCheckpoint`, then `Closing`.
8. **Abandons.** Every abandon is one `abandon_branch_record` transaction under the lease. An abandon from `closing` happens only after `NothingWritten`.

### 4.4 `finish_branch` (new, run.rs)

`fn finish_branch<L: Lifecycle>(lifecycle, service, request, config: Option<&RepositoryConfig>, primary: &str, r: &mut L::Record, io, progress) -> Result<L::Output, L::Error>`. `finish_integrated` and `finish_steps` (1242-1728) are unchanged.

1. **Mint.** If `r.primary_sync_id().is_none()`: `*r = with_transaction(|tx, id| L::begin_publication(tx, id, op))`.
2. **The remote is current.** `remote_current = config.is_some_and(|c| c.primary_branch == primary && c.publication_remote.as_deref() == r.remote_name())`. In slice C, `!remote_current` answers `Err(L::Error::recovery_required())` with the record at `publication_pending`. Slice D replaces this with row 5.
3. **A completed child comes first.** If the second slot is `ChildState::Complete`, `child()` replays it (1014-1016), whatever cancellation says.
4. **Otherwise, when cancelled:** `release_child(Publication)`, `set_publication(Cancelled)`, `save`, then `Err(interrupted)` (D15).
5. **Run the child.** `result = child::<L>(…, ChildRole::Publication, …)`.
   - On `Ok`: `remember_publication` (1190-1206), `save`, `AfterPublication`.
   - On `Err` with cancellation: as step 4.
6. **The answer.** `answer = L::branch_answer(service, request, r, &result)?`. Slice D adds the `remote_current: bool` argument.
   - `None`: `set_publication(publication_error(&e))`, keep `publication_pending`, `save`, then `Err(L::Error::context_synchronization(Box::new(e)))` (Obligation 7). In slice C every child error ends here.
   - `Some(finish)`:
     - for a stopped child (slice D, rows 1 to 5), `let_go(Publication)` first;
     - then `BeforeCompletion`, the transaction `L::complete_branch(tx, id, op, &finish)`, `reconcile_completion` on an uncertain commit (as 1714-1721; path-aware through 2.3), `PromotionAfterCompletionCommit`, reload, `AfterCompletion`, and `Ok(refresh::<L>(…, false))`.

`let_go::<L>(service, request, r, role, io)` is new in slice D. It is called only on a D6 end or a row 1 to 5 completion:
- `release_child` (988-1000);
- then, if the row is still active, `acknowledge_cancellation` (extracted from 1045-1061);
- then `L::child_state` must be `Absent`, `Complete`, or `Unowned` in a terminal phase, or the call answers `Err(recovery_required)` with the record unfinished.

### 4.5 Other runner places and the trait

- `children_contexts` (1877-1886): for a branch record, either slot gives `ContextKey::Item(item)` once.
- `refresh_with_publication` (1829-1862): for a branch record the contexts are `[ContextKey::Item(item)]`, and a replay checks `service.owed_of(root, &[Item]).is_empty()`.
- `child_id` (975-979): `Primary | Publication` → `primary_sync_id()`. `synchronization_target` (1130-1142): `Publication` → `Context { kind: kind.authoring(), item_id }`. Both in slice A.
- `primary_authority` (1209-1240) passes `second_role(r.path())`.
- `remember_publication` (1190-1206) is unchanged.
- `Lifecycle` trait additions (promotion/lifecycle.rs:351-554). None has a default body. Promotion's impl (759-1004) answers `Err(…recovery_required())` or `unreachable!("a promotion record is always direct")`, and sets `type BranchFinish = std::convert::Infallible;`.
  ```rust
  type BranchFinish: Clone;                                                       // slice C
  fn branch_step(&self, service: &RepositoryService, request: &LifecycleRequest<Self::Expectation>, r: &Self::Record) -> Result<BranchStep, Self::Error>;   // C
  fn end_branch(service: &RepositoryService, root: &Path, r: &Self::Record, end: BranchEnd) -> Result<(), RepositoryError>;  // C; D makes it `end: BranchEnd<'_>`
  fn begin_publication(tx: &Transaction<'_>, id: i64, op: OperationId) -> Result<Self::Record, RepositoryError>;               // C
  fn branch_answer(service: &RepositoryService, request: &LifecycleRequest<Self::Expectation>, r: &Self::Record,
      child: &Result<SynchronizationOutcome, SynchronizationError>) -> Result<Option<Self::BranchFinish>, RepositoryError>;   // C; D adds `remote_current: bool`
  fn complete_branch(tx: &Transaction<'_>, id: i64, op: OperationId, finish: &Self::BranchFinish) -> Result<(), RepositoryError>;  // C
  fn branch_outcome(r: &Self::Record, publication: Option<PromotionPublication>) -> Self::Outcome;                            // C
  ```
- In `promotion/lifecycle.rs`:
  - `pub(in super::super) enum BranchStep { Closing, Finished }`;
  - In slice C, `pub(in super::super) enum BranchEnd { ContextGone }`, with no lifetime: a lifetime that no variant uses is error E0392. Slice D makes it `pub(in super::super) enum BranchEnd<'a> { ContextGone, ContextChild(&'a SynchronizationError) }` and the trait's parameter `end: BranchEnd<'_>`, in one change.
- The closure's `BranchFinish` is `pub(in super::super) struct BranchFinish { answer: BranchAnswer, link: Option<PullRequestLink>, given_at: i64 }`.
- `end_branch(ContextGone)` abandons with `ReconfirmationRequired`/`reconfirmation_required`.
- `LifecycleRequest` (116-124) gains `clock: fn() -> OffsetDateTime`, defaulted in both builders (promotion/run.rs:287-307; closure/lifecycle.rs:54-74). `close_ticket_observed` (closure/mod.rs:374-382) gains `clock`, which its one caller, `mutation/progress.rs:450`, passes as `OffsetDateTime::now_utc`.

## 5. The second-slot readers

| Reader | Change | Slice |
| --- | --- | --- |
| `ChildRole` (lifecycle_record.rs:450-454) | gains `Publication` | A |
| `child_allowed` (457-465) | becomes `fn(&ParentFacts, role)`, shown below | A |
| `attempt_of` (466-471), `mint_attempt` (676-679), `ChildExecution::validate` (584-587) | `Primary \| Publication` → the second slot | A |
| `next_attempt` (648) | gates on `child_allowed(&ParentFacts::of_ref(&r), role)` | A |
| `ChildExecution::validate_request` (554-571) | gains `(Publication, Context { kind, item_id }) => *kind == self.kind.authoring() && item_id == &self.item` | A |
| `remote_identity_of` (761-781) | a branch record's second slot expects `SynchronizeContext`, so 780-781's item check applies | A |
| `audit` (880-919) | loops `[Context, second_role(r.path())]`; `second_role` (Direct → Primary, Branch → Publication) is the production constructor of `Publication` in slice A | A |
| `ChildExecution::role()` and `is_publication_of(kind, parent) -> bool` | read by the fence exemption and the test IO | C |
| run.rs readers and `Stages` | sections 4.5 and 1 | A and C |
| By column name, unchanged | `bindings` (lifecycle_record.rs:312), `existing_record`'s collision (357), `child_repository` (840), `reservation.rs` `abandonable` (801-812), `recovery.rs` `attribute_remote_row` (1154-1199), `local_sync/record.rs` `FOREIGN_IDENTITIES` (1058-1072), `read/status.rs` (705-737, 639), `registration_removal.rs` (210-225), `ParentView.children` | — |
| Conflict lookups | Obligation 9 | Task 12 |

The new `child_allowed`:

```rust
fn child_allowed(facts: &ParentFacts, role: ChildRole) -> bool {
    match (facts.path, role) {
        (_, ChildRole::Context) => facts.step == ParentStep::Confirmed,
        (RecordPath::Direct, ChildRole::Primary) => matches!(facts.step, ParentStep::Integrated | ParentStep::PublicationPending),
        (RecordPath::Branch, ChildRole::Publication) => matches!(facts.closure_step, Some(ClosureStep::Closing | ClosureStep::PublicationPending)),
        (RecordPath::Direct, ChildRole::Publication) | (RecordPath::Branch, ChildRole::Primary) => false,
    }
}
```

## 6. The fence: its range and its exemption

- **The range** (record.rs:774-785).
  - `written_closure_holder(db, id, item) -> Result<Option<(OperationId, RecordPath)>, _>` is added in slice C. Slice A changes only `written_closure_unfinished`'s SQL, which is `holder.is_some()` from C on.
  - The SQL: `(integration_mode='direct' AND action='close' AND step IN (<today's nine>)) OR (NOT (integration_mode='direct' AND action='close') AND step NOT IN ('complete','abandoned','superseded') AND (step NOT IN ('confirmed','context_synchronized') OR (action='new_branch' AND closure_oid IS NOT NULL)))`.
  - Slice E adds `branch_hold(db, id, item) -> Option<OperationId>` and `hold_refusal(op) -> RepositoryError`, which is `recovery_required().with_blocker(Blocker::remote(&op.to_string()))`.
- **The exemption** (slice C). `closure_holds_context` (reservation.rs:427-440) gains `execution: Option<&ChildExecution>` and answers `false` iff the holder is a branch record and `execution.is_some_and(|e| e.is_publication_of(TicketClosure, holder))`.
  - Both callers already pass a validated execution: 1618-1622 (validated at 1503-1505) and 3211-3213 (validated at 3181-3183).
  - Everything else of the ticket's `SynchronizeContext` target stays `Busy`.
  - An execution of another record never reaches the fence. The reservation's identity check refuses it first: `validate_poll_identity` → `remote_identity_of` (lifecycle_record.rs:733, 761-781) answers `OperationMismatch`, since the target's item or slot is not that record's. Its own `validate` (573-605) may refuse it earlier with the same kind.
- **The poll's materialize and fast-forward checks** (poll.rs:387-397, 663-672) call `written_closure_unfinished` and cover branch records through the new range. The poll's retirement check is in section 7.

## 7. The writers the hold reaches, per mode (slice E)

"Named" means the domain error carries `with_blocker(Blocker::remote(op))` (repository.rs:1518), which `read/status.rs` `read_blocking_operation` resolves through `closure_operations` and `Answer::naming` (bind/mod.rs:361-407) names. Slice E adds `naming` to any of these bindings that does not already pass its domain error's blocker, as `bind/save.rs:250-253` does.

| Writer | Where the hold is checked | Branch record | Direct record |
| --- | --- | --- | --- |
| save, slug, context prepare, comment checkpoint | `repository.rs` `refuse_closed_ticket` (300-324) gains `service` and checks `branch_hold` (a registry read, as 4470-4509) before the guard at 317. Callers: 2398-2400, 3101-3106 (save, relationship and slug; `bind/ticket.rs` 1248→1252→3044→3065→3103), 3496-3502 | `recovery_required`, named | the guard's `ticket_closed`. The new read runs in both modes, because a withdraw is a branch record in either mode, so a registry read error now surfaces here as `recovery_required`. Guarded by `ticket_guard_tests` (Task 2), `tests/mutation_bridges` and `comment_publication` at review-ready |
| `item sync`, a comment's publication | `closure_holds_context` (6) | `busy` | `busy` |
| `item republish` | `bind/republish.rs` `observe` 201-209, after `require_ticket_open`: `branch_hold`. The domain's `SynchronizeContext` reservation is the backstop (`busy`) | `recovery_required`, named | unchanged |
| `item resolve-history` | no hold check (Q6's ruling; this row is the only Q6 site) | passes; the same request then continues its record | unchanged |
| `ticket repair` | `bridges/repair.rs` `repair_item`, after 482-488: `branch_hold` | `recovery_required`, named | unchanged |
| `context retire` | `context_retirement/authority.rs` `claim_scoped`, in its transaction after 324: `branch_hold` | `recovery_required`, named | unchanged |
| the poll's retirement | `context_retirement/source.rs` `capture_target`, in the transaction at 228-235: a ticket item for which `written_closure_unfinished` holds gets `c.refusal = Some("recovery_required")`. `authority.rs` `claim_initial` (in its transaction after 423) is the backstop | candidate refused; the context left alone | **a change in direct behaviour, by construction:** the check is `written_closure_unfinished`, whose direct range (`closing` through `branch_deleted`) is unchanged, so a direct record's candidate is refused the same way (R-17's "while it holds"). The direct test row is dropped (R24); open question 2 disputes its premise and proposes a reachable row |
| the ticket's own `context sync-local` | `local_sync/record.rs` `admit` (1144-1220): the exception at 1169-1176 gains `\|\| r.path() != RecordPath::Direct`. A branch record on the same item answers `ReadError::new(RecoveryRequired)` with `recovery = vec![RecoveryAction::for_operation(RecoveryActionKind::OperationShow, op.to_string(), [])]` | `recovery_required`, naming the record by its recovery action | unchanged (K-10) |
| poll materialize, fast-forward | `poll.rs` 387-397, 663-672 | the context left alone | unchanged |
| `context release` | `context_release.rs` 218-241, unchanged | refused | refused |

Before `closing`, a branch record's prepare is held by `held` (bind/lifecycle.rs:386-426), and its step 3 waits through `context_synchronization_unfinished`.

## 8. The branch observation

### 8.1 Mode, dispatch and situations (slice B)

- **The mode (R25).**
  - `observe::prepare_observation_in(service, request, mode: Option<Integration>)` is the one entry point. `None` means the live `config.integration()` (canonical.rs:146-148). `prepare_observation` (observe.rs:313-318) is `prepare_observation_in(…, None)`.
  - `observe_git` takes the resolved `mode`. `PreparedClosureObservation` (291-298) stores it, and `revalidate_under_lease` (371-402) keeps the stored mode without reading it again.
  - **A new request always observes in the live mode:** the binding's prepare (`V::observe` → `preview_ticket_closure`), the confirm's prepare (run.rs:501, `record: None`), and a refusal at insertion.
  - **A same-request continuation passes its record's mode:**
    - `Lifecycle::preview_request(root, item, identity, record: Option<&Self::Record>)`, from run.rs:256 at 672 and 726, and integrate.rs:122 with `parent`;
    - the branch hook (4.3, step 1).

    So a record continues in its own mode, direct ones included (R-2). After a switch to pull-request mode, a direct record's retry continues the direct close and pushes the primary, as R-2 has it.
  - **A non-supersedable unfinished record** answers decision 33's `recovery_required` at a new prepare, through `held` (bind/lifecycle.rs:386-426).
  - **A supersedable direct record that a pull-request prepare meets.** `supersedable` requires a cleanup category of `ContextChanged`, `IgnoredFiles` or `RemoteContextChanged` (promotion/record.rs:69-74) and a step other than `closing` or `integrating` (record.rs:460-463). Those categories are set only by `finish_integrated` (run.rs:1263) after integration, so the local primary holds the record's merged closure.
    - The pull-request observation's situation 1 then gives `[AlreadyClosed]`.
    - `held` finds no holder, since `unfinished_holder` (promotion/mod.rs:449-460) drops supersedable records.
    - `blocked_code([AlreadyClosed])` is `TicketClosed` (bind/lifecycle.rs:213-219). The prepare answers `ticket_closed`, blocker `already_closed`, and writes nothing.
    - The old direct record is untouched: no confirm can follow a blocked prepare, and a confirm whose revalidation is blocked only refuses at insertion (`refuse_record` never supersedes, record.rs:511-515). It stays at its cleanup stop, to be finished by its own request (in its direct mode) or superseded by a later direct-mode request.
- **Dispatch.** `let branch = mode == Integration::PullRequest;`. These inserts are guarded with `if !branch`: `PrimaryNotCheckedOut`, `PrimaryDirty`, the primary-presence `PrimaryConflicted`, `PrimaryAheadOfRemote` and `ContextBehindRemote` (669-701). After the remote facts, `if branch { return branch_observation(…) }`.
- **Paths.** `ClosureObservation` (204-231) gains `path: (Integration, ClosureAction)`, and `TicketClosurePreview` (closure/mod.rs:118-185) gains `path` and an accessor.
- **A branch observation's fields.**
  - `base` is the tracking primary.
  - `needs_context_child` is true.
  - `changes` is empty, and the planned cleanup is all false.
  - `integration` is `MergeCommit`, documented as unused by a branch record and read only as "the context child runs".
- **Situations,** in Part D's order:
  1. The local or tracking primary's ticket is not `Open` (`Malformed` included): `AlreadyClosed`.
  2. Today's `RemoteContextDeleted`, until Task 9.
  3. Today's `ContextMissing` or `AlreadyClosed` (744-748), until Task 9.
  4. The context's ticket holds a closure: `Existing`.
  5. Otherwise: `Record`.

### 8.2 Blockers and the prepare's `busy`

- **Kept:** `NotATicket`, `ContextMissing`, `ContextNonconforming`, `ContextDirty`, `ContextConflicted`, `ConflictPending`, `LocalOperationPending`, `CommentPublicationPending`, `PrimaryMissing` (local primary, or tracking primary), `RemoteContextDeleted`, `HistoryUnknown`, `HistoryUnresolved`, `AlreadyClosed`, `IdentityRequired` (`record`), `TicketInvalidAfterClosure` (a `Malformed` context closure, or `closed_ticket_bytes` refusing the tip's ticket).
- **Ruling 6:** the root repository's `state() != RepositoryState::Clean` (the condition `require_clean_target_state`, sync.rs:747-780, refuses through the common directory) is `PrimaryConflicted`, whatever branch the root is on.
- **Not reported:** `PrimaryNotCheckedOut`, `PrimaryDirty`, `PrimaryAheadOfRemote` (except Task 9's 3.3), `ContextBehindRemote`, `IntegrationConflict`, `PrimaryReopened`.
- **"An unfinished synchronization of the ticket's context (busy, naming it)."** The direct prepare has no mechanism to reuse: `pending_work` reads only local rows and comment bindings, and the direct close meets this only at confirm (checkpoint.rs:648, 680-697).
- **The new read.** In pull-request mode only, `pending_work` also reads:
  ```sql
  SELECT operation_ulid FROM remote_operation_records
   WHERE repository_id=?1 AND action='synchronize_context' AND kind='ticket' AND item_id=?2
     AND phase NOT IN ('completed','interrupted','cancelled','failed')
     AND operation_ulid NOT IN (SELECT context_sync_id FROM ticket_closures WHERE repository_id=?1 AND item_id=?2 AND step NOT IN ('complete','abandoned','superseded') AND context_sync_id IS NOT NULL
                                UNION SELECT primary_sync_id FROM ticket_closures WHERE repository_id=?1 AND item_id=?2 AND step NOT IN ('complete','abandoned','superseded') AND primary_sync_id IS NOT NULL)
   LIMIT 1
  ```
  The phase predicate is record.rs:796's.
  - The unfinished record's own children are left out, so the binding's `held` answers for them (decision 33): a kept context stop (D5), a retained context conflict (D7), a kept publication stop (D8), a publication conflict (D10).
  - A foreign live synchronization, such as a manual `item sync`, is what remains.
- **The answer.** `branch_observation` answers `Err(RepositoryError)` of kind `RepositoryBusy` with `.with_blocker(Blocker::remote(&op))` for such a row.
- **What each layer answers for a foreign live row:**

  | Where | Domain | Binding |
  | --- | --- | --- |
  | prepare | the error | `LifecycleBinding::observe` (bind/lifecycle.rs:379-384) maps it with `refused(error).naming(service, &root, error.blocker())`: `busy`, naming the row. The direct observation sets no blocker and runs no such read, so the direct answers are unchanged |
  | a new request's confirm | `L::prepare(…)?` (run.rs:499-506) propagates the error, which `prepare_observation_at` passes on (observe.rs:347-358), as `TicketClosureError::Repository(e)` with `e.kind == RepositoryBusy` (closure/mod.rs:344-348). It does not go through `mapped` at 531. Nothing is inserted | `Stop::Repository(e)` → `repository_error_code(RepositoryBusy)` = `busy` (bind/lifecycle.rs:1302; read/mod.rs:299) |
  | step 3's prepare (4.3, step 1) | the same `Repository(RepositoryBusy)`, with the record unchanged | `busy` |

  The error is not mapped, for two reasons. The `?` at run.rs:506 is the direct path's, and mapping it would change a direct close's variant for every `RepositoryBusy` its prepare can meet. `TicketClosureError::Busy` carries no blocker, which Task 10's stop data needs.
- **The holder's own kept child, at a new request's confirm:** with the exclusion, the observation passes and `begin_record` answers `busy` (record.rs:486-489) through `mapped` at run.rs:571. That is `TicketClosureError::Busy`.

### 8.3 The digest

- `PULL_REQUEST_CLOSURE_DIGEST_DOMAIN = b"manyhands pull request closure v1"`.
- `branch_expectation(o)` is `branch_expectation_with_head(o, Some(o.context_oid))`.
- `branch_expectation_with_head(o, head)` hashes `blake3(domain ‖ fields)` with `digest::field` framing:
  - the action tag (`close 0`, `withdraw 1`, `new_branch 2`);
  - the item;
  - the context branch and the primary branch;
  - `digest::remote`;
  - the head (`[0]` none, or `[1]` followed by 20 bytes);
  - the closure effect, framed as `expectation` (observe.rs:1041-1073) frames it: `0‖closed_by` for `record`, `1‖closed_by‖closed_at` for `existing`;
  - for `new_branch`, Task 9's fields.
- A `record` closure's `closed_at` is not bound (C6).
- The domain makes D17; `closed_by` makes D18. A changed action, effect or head is `external_change` at confirm. At step 3, `consented_context_oid` stands in for the head the child may have moved (Q1).

## 9. The answer table

### 9.1 After the publication child: `checkpoint::publication_answer` (rows 6 to 9 in slice C, rows 1 to 5 in slice D)

- "This record's closure" is `commit_closure_text(closure_oid)` for `record`, `commit_closure_text(context_oid)` for `existing`, compared on both fields.
- "The tracking primary" is `refs/remotes/<remote>/<primary>` as the child left it.
- "The head" is the completed child's authority OID.

| # | Condition | `BranchAnswer` | Code | Record |
| --- | --- | --- | --- | --- |
| 1 | the tracking primary holds this closure | `Merged` | `ok` | `complete` |
| 2 | the tracking primary names another closure | `ClosedElsewhere` | `ticket_closed` | `complete` |
| 3 | child `Err(RemoteContextDeleted)` | `RemoteBranchDeleted` | `remote_branch_deleted` | `complete` |
| 4 | child `Err(TicketClosed)` | `TicketClosed` | `ticket_closed` | `complete` |
| 5 | `!remote_current`, or child `Err(Transport)` of kind `PublicationRemoteMissing \| EndpointChanged \| ConfigurationInvalid` (Q7) | `RemoteChanged` | `recovery_required` | `complete` |
| — | child `Err(other)` | `None`: the stop stands | the child's | `publication_pending` |
| 6 | child `Ok`, kind `existing` | `Existing` | `already_closing` | `complete` |
| 7 | child `Ok`, the head holds this closure | `Published` | `ok` | `complete` |
| 8 | child `Ok`, the head names another closure | `KeptPublished` | `already_closing` | `complete` |
| 9 | child `Ok`, the head names no closure | `WithdrawnSince` | `ok`, recording `withdrawn_since` | `complete` |

- A child `Ok` outcome with no OID is `recovery_required`.
- Rows 1 to 5 after a stopped child let it go first.
- Every completion upserts the link (2.4).

### 9.2 Stored `answer_code` values (`BranchAnswer`)

- **After the child:** `merged`, `closed_elsewhere`, `remote_branch_deleted`, `ticket_closed`, `remote_changed`, `existing`, `published`, `kept_published`, `withdrawn_since`.
- **Abandons,** each with the refusal it stores:

  | Answer | Cause | Refusal |
  | --- | --- | --- |
  | `already_closed` | 3.1 | `Blocked([AlreadyClosed])` |
  | `already_closing` | 3.2, `record`; the link upserted | `ExternalChange` |
  | `external_change` | D21, a moved tip, a failed digest proof, an identity mismatch | `ExternalChange` |
  | `reconfirmation_required` | a blocked revalidation, an invalid closed ticket, `ContextGone` | `ReconfirmationRequired` |
  | `context_remote_branch_deleted`, `context_history_unresolved`, `context_ticket_closed` | D6 | `ReconfirmationRequired` |

  The refusal keeps `abandoned ⇔ refusal` true; `answer_code` decides the answer, in the domain and, through `ParentView.answered`, in the binding.

### 9.3 Codes (binding, `fn branch_code(BranchAnswer) -> ResultCode`, slice C)

| Code | Answers |
| --- | --- |
| `Ok` | `merged`, `published`, `withdrawn_since` |
| `AlreadyClosing` | `existing`, `kept_published`, `already_closing` |
| `TicketClosed` | `closed_elsewhere`, `ticket_closed`, `already_closed`, `context_ticket_closed` |
| `RemoteBranchDeleted` | `remote_branch_deleted`, `context_remote_branch_deleted` |
| `RecoveryRequired` | `remote_changed`, `context_history_unresolved` (as outcome.rs:233-240) |
| `ExternalChange` | `external_change` |
| `ReconfirmationRequired` | `reconfirmation_required` |

The answer and the record's final step are written in one transaction, and every later call answers from them.

## 10. Ruling 8's open points, settled

1. **`repo sync` for a tip only the local primary holds (3.3).** This is Task 9's. In that case the local primary is ahead of the remote, so the refusal also carries the existing `PrimaryAheadOfRemote`, which `way_out.rs` (319-335) routes to `SharePrimary`. The blocker itself is Q2's preflight-only variant.
2. **`closed_at` frozen before the commit (C6).** The commit object is built unreferenced, then one UPDATE (`begin_branch_closing`) stores `closed_at` and `closure_oid` together, before the ref moves. A `NothingWritten` rebuild reuses `closed_at` (4.3, step 7). No CHECK is relaxed.
3. **`permits`' abandon list.** Section 3: direct unchanged; branch `close` and `withdraw` from `Confirmed`, `ContextSynchronized`, and `Closing` with nothing written; `new_branch` from `Confirmed`, `BranchDeleted` (Q5) and `Retired`.
4. **The frozen DDL.** `SCHEMA_V2` (2.1).

## 11. The test plan

### 11.1 Module and fixture

The run tests are the `pull_request` module: a new file `src/repository/closure/run_pull_request_tests.rs`, declared in `closure/run_tests.rs` as `#[path = "run_pull_request_tests.rs"] mod pull_request;`. Its module path is `repository::closure::run_tests::pull_request::<test>`, and each test runs by exact name.

`struct PullRequest { f: Fixture, primary_before: Oid }` builds on `Fixture::remote()` (run_tests.rs:75-87):
- `new()` commits `.manyhands/config.toml` with `format_version = 2`, `integration = "pull_request"` and `publication_remote = "origin"` on `main` at the root, and moves `refs/remotes/origin/main` to it. It reuses Task 2's or Task 6's helper through `test_support.rs` if one exists.
- `request()` returns the request for `preview_ticket_closure`'s expectation.
- `run(req, io)` is `run::run_with_io(&TicketClosureLifecycle, &service, LifecycleRequest { clock, ..lifecycle_request(req) }, io)`, with `fn clock() -> OffsetDateTime { OffsetDateTime::from_unix_timestamp(1_795_000_000).unwrap() }`.
- `branch(result)` returns the branch outcome.
- `closed_commit_on(parent, closed_at, closed_by)`, `open_commit_on(parent)` and `remote_clone_commit(...)` build the commits a remote clone, a forge merge or a withdraw would make.
- `assert_primary_untouched(&io)` checks `refs/heads/main == primary_before` and `io.primary_calls == 0`. Every run test ends with it.

### 11.2 `PullRequestIo` (`FakeIo` is wrapped, and its fields are unchanged)

```rust
struct PullRequestIo {
    inner: FakeIo,
    /// `refs/remotes/origin/main` as each role's fetch leaves it, written first, before any stop.
    fetched: Vec<(ChildRole, Oid)>,
    /// Scripted answers by role, first in first out; `kept: true` reserves the child and leaves its row active.
    stops: VecDeque<(ChildRole, SynchronizationError, bool /* kept */)>,
    /// One real merge, taken once: the role and the incoming commit for `promotion_test_drive_local_bound`.
    drive: Option<(ChildRole, Oid)>,
    primary_calls: usize,
}
```

`synchronize`, in order:
1. A `Primary` target increments `primary_calls` and returns `Err(RecoveryRequired)`.
2. **An acknowledgement.** A `restart: true` request for a row with `cancel_requested=1` (the cancel-and-acknowledge `child()` and `let_go` make) sets that row `phase='cancelled', outcome='cancelled'` and returns `Err(Interrupted)`, as a real child does at its first safe point.
3. **The fetch.** Write `refs/remotes/origin/main` from `fetched` for `execution.role()`, before step 4, so a stop still leaves its fetch.
4. **A scripted stop.** Pop the front of `stops` if its role matches. When `kept`, the IO first reserves the child as `FakeIo` does (`reserve_promotion_child`, promotion/run_tests.rs:385-397) and leaves the row reserved and active, the shape a real `RemoteContextDeleted`, `TicketClosed` or `HistoryUnresolved` stop keeps (sync.rs:4336-4357; Fact 15). Return the error.
5. **A real merge.** If `drive` matches the role, `take` it and call `sync::promotion_test_drive_local_bound(service, &req, incoming, execution)` (sync.rs:3983-4091).
   - On `Err` (a conflict, D7 and D10), return it, with the row retained.
   - On `Ok(merged)`, complete the row with `merged` as its authority, through a test helper `complete_child_row(service, root, op, oid, kind)` extracted unchanged from `FakeIo`'s completion UPDATE (promotion/run_tests.rs:417-421), and return `Published { oid: merged }`.

   The local update moves the context ref and its worktree as a real child does.
6. **Otherwise,** `inner.synchronize`, a `FakeIo` completion with the local head.
7. **The tracking context.** Write `refs/remotes/origin/manyhands/ticket/<ID>` to the published OID.

`delete` delegates. `FakeIo` and its 16 impls (E-32) are unchanged apart from the extracted `complete_child_row`, which leaves `FakeIo`'s behaviour as it is.

`DriveThenStop { inner: PullRequestIo, incoming: Oid }` is a second wrapper, for d13's in-process row. Its publication child calls `promotion_test_drive_local_bound` for `incoming` and returns `Err(SynchronizationError::Interrupted)` without completing the row. The row is left reserved after the local update and before any push, as a lost process leaves it. Every other call it delegates to `inner`, whose `drive` is `None`.

`stops` with `kept: true` and a "something to lose" shape (d11's fourth row) also writes the evidence the decode rule requires for `local_prepared` (remote/state.rs:3544-3580):
- `phase='local_prepared'` and `sync_checkpoint='local_prepared'` together;
- `expected_oid`, `local_oid` and `primary_tracking_oid` set: the fetched remote tip, the local head and the fetched primary.

`release_promotion_child` then reports something to lose (reservation.rs:1812-1825), and `let_go` takes the cancel-and-acknowledge path.

### 11.3 Tests, each run red first, by slice

| Slice | Test | Setup | Asserts |
| --- | --- | --- | --- |
| A | `branch_records_rebuild_and_keep_direct_rows` (P7, `closure/record_tests.rs`) | `db()` (20), plus a table from `SCHEMA_V2` with direct rows at `confirmed`, `closing`, `integrating` and `complete`, without the modes marker | one rebuild; each row reads `Direct`/`Close` with the other columns equal; the marker once; a second migrate changes nothing; a branch row at `publication_pending` inserts and reads back; the direct rows still validate |
| A | `child_fence_admits_the_publication_child_only_while_a_branch_record_publishes` (`remote/reservation_tests.rs`, beside 4386) | `close_parent` (3696), made a branch record by SQL `UPDATE` | `Publication` validates at `closing` and `publication_pending`, and is refused at `confirmed`, `context_synchronized`, `published` and `complete`; `Primary` refused on a branch record; `Publication` refused on a direct one |
| B | `pull_request_prepare_decides_the_situation_and_its_blockers` (`closure/observe_tests.rs`, table) | rows: situation 1 by the local primary and by the tracking primary; 4; 5; `PrimaryConflicted` with a merge in progress at a root on another branch; each dropped blocker absent; a foreign live `synchronize_context` row; an unfinished record's own live child row | the situation or blocker of each row; the foreign row answers kind `RepositoryBusy` with `blocker().operation_id == row`; the record's own child row does not make it busy; the domain digest differs from the direct one for the same state; `closed_at` of `record` not bound |
| B | `a_new_request_observes_live_and_a_continuation_its_record_s_mode` (`closure/observe_tests.rs`, R25) | a direct record at a supersedable cleanup stop (`ContextChanged`) after integration, then the configuration switched to pull request | `prepare_observation_in(…, None)` observes pull request and is blocked `[AlreadyClosed]` (situation 1), and the old record is unchanged; `prepare_observation_in(…, Some(Direct))` observes direct (the direct digest) |
| B | `d17_a_mode_switch_between_prepare_and_confirm_is_external_change` | table, both directions | `Err(ExternalChange)`; the refused row's path is the confirm's mode and its `answer_code` none; no commit; no child ran |
| B | `d18_an_identity_change_between_prepare_and_confirm_is_external_change` | `user.email` changed after prepare | `Err(ExternalChange)`; nothing written |
| C | `d1_records_publishes_and_completes_without_touching_the_primary` | `PullRequestIo` default | `Published`; one `Close ticket <ULID>` commit over the synchronized head, with `closed_at == clock()` and `closed_by`; the tracking context at it; `complete` with `answer_code == published`; the link row (`linked_items_for_test`); the hold released after; the same request again gives the same answer |
| C | `d2_an_existing_closure_publishes_pending_work_and_is_already_closing` | the head holds a closure before prepare | preview `Existing`; no new commit; `Existing`; `complete`, `closure_oid` none; the link row present |
| C | `d3_a_closure_the_context_child_brings_in_abandons_as_already_closing` | `drive = (Context, closed_commit_on(head))` (a real fast-forward of the ref and the worktree) | `AlreadyClosing`; no closure commit; `abandoned` with `answer_code`; the link upserted; the same request again (domain) gives the same. The binding's `ended_answer` is asserted in `a_branch_parent_view_is_never_integrated` |
| C | `d4_a_closure_the_tracking_primary_holds_after_the_context_child_abandons_as_ticket_closed` | `fetched = [(Context, closed_commit_on(primary))]` | `AlreadyClosed`, refusal `Blocked([AlreadyClosed])`; nothing written |
| C | `d5_a_stopped_context_child_keeps_the_record_confirmed_and_the_same_request_continues` | table of context stops: transport, key, host (`Transport` kinds), `PushRejected`, `Busy`, `PollYielding`, one of them `kept`; plus a row where a manual `item sync` of the context (`reserve_remote_operation`, as run_tests.rs:1864) is live when step 3 runs | stop rows: `Err(ContextSynchronization(e))`; `confirmed`; the kept row's child still reserved (not let go); a new request's confirm `Err(Busy)` (its own child is excluded, so `begin_record` answers); the same request then gives `Published`. The manual row: `Err(Repository(e))` with `e.kind == RepositoryBusy` and `e.blocker()` naming the manual row; `context_synchronized`; nothing written; once the manual row is cancelled the same request gives `Published` |
| C | `d7_a_context_conflict_is_retained_and_the_same_request_continues` | `drive = (Context, conflicting remote commit)` | `Err(ContextSynchronization(ConflictPending))`; child retained; resolved through the conflict verbs' library call; the same request then gives `Published` |
| C | `d8_a_stopped_publication_child_is_held_and_the_same_request_continues` | table of publication stops: transport, key, host, `Busy`, `PollYielding`; one row `kept` | `Err(ContextSynchronization(e))`; `publication_pending` with that category; held; reads `closing`; the same request then gives `Published` |
| C | `d9_a_rejected_publication_is_replaced_and_keeps_the_published_closure` | first call `stops = [(Publication, PushRejected, true)]`; second call `drive = (Publication, remote_clone_commit with another closure)` | the first call is D8; the second's child merges (Part G) and completes: `KeptPublished` |
| C | `d10_a_publication_conflict_is_retained_and_the_same_request_continues` | `drive = (Publication, conflicting commit)` | as D8, with `ConflictPending`; resolved; the same request continues to the answer |
| C | `d12_a_crash_before_the_ref_moved_rebuilds_the_same_commit` | `exit_at` (run_tests.rs:1151) at `EXITS[2]` (`ClosureAfterClosingRecorded`) and `EXITS[3]` (`ClosureDuringCheckpointCheckout`); `closure_exit_child` (1101-1149) runs `FakeIo`, which completes a context-target publication child | the same `closed_at`; one `Close ticket` commit (`commits_in_odb`, 156); `Published` |
| C | `d13_a_crash_after_the_ref_moved_never_runs_the_hook_again` | `exit_at` at `EXITS[4]` (`AfterClosureCheckpoint`), `EXITS[7]` (`AfterPublication`), `EXITS[12]` (`BeforeCompletion`); plus an in-process row through `DriveThenStop` (11.2), whose drive leaves its row reserved and uncompleted | no second commit; `RunEvent::Recording` not told once `primary_attempt > 0` (the `Told` observer, as 2321); `child()` replays or restarts; the answer's row (`Published`, or `KeptPublished` for the in-process row) |
| C | `d14_a_new_request_while_unfinished_is_held_and_busy` | table: a record at `publication_pending` from D8's `kept` row (its own publication child holds a live row), and a record at `confirmed` from D5's `kept` row (its own context child holds one) | `promotion::unfinished_holder::<ClosureRecord>` names the record; a new prepare through the binding, `f.service.prepare(&Mutation::TicketClose(…))` (as run_tests.rs:2402-2407), answers `recovery_required` (not `busy`) with the holder's recovery action: `request.retry` naming its request, or `operation.resume` naming its operation when no request record holds it (bind/lifecycle.rs:413-420); a new request's confirm `Err(Busy)`; nothing written |
| C | `d15_a_cancelled_publication_child_is_replaced_by_the_same_request` | a wrapper that calls `cancel_ticket_closure` at the publication child and returns `Err(Interrupted)` (as `CancelAfterPrimary`, run_tests.rs:1435-1460) | `Err(Interrupted)`; `publication_pending`/`cancelled`; the same request: a new attempt; `Published` |
| C | `d20_operations_list_a_branch_record_as_a_close_and_its_child_as_its_own_row` (library half; design correction) | a record at `publication_pending` with its publication child's row reserved (D10's conflict) | the operations read (`read/status.rs` `repository_operations`, 845-858) lists the record once as `close` with its step and resume offered; it lists the publication child as its own `synchronize_context` row with no next action while it holds the reservation (520-540), attributed to the record's request (`recovery.rs` 1154-1199); abandon of the child is refused (`reservation.rs` `abandonable`, 801-812). This is what a direct close's child gets today |
| C | `d21_a_changed_existing_closure_before_step_three_abandons_as_external_change` | `existing` preview; table: `drive = (Context, closed_commit_on(head, another closure))`, and `drive = (Context, open_commit_on(head))` (a withdraw) | `ExternalChange` abandon (not `reconfirmation_required`: the worktree is clean after the real update); no closure commit |
| C | `a_branch_record_s_own_publication_child_passes_the_fence` (E-23, `closure/run_tests.rs` after 1894) | a `PullRequest` record at `publication_pending`, built in the child module and used through `pub(super)` | its validated execution reserves (`Reserved`); a manual reservation of the same target `Busy`; a stale attempt's execution refused with the lifecycle mismatch |
| C | `closure_hold_lets_through_only_its_own_publication_child` (`reservation_tests.rs`) | — | through the reserve (1618) and restart (3211) callers: exempt for the holder's own `Publication` execution; `Busy` with no execution; `Busy` for a direct holder; another record's execution refused by `validate` with the lifecycle mismatch (`OperationMismatch`) before the fence |
| C | `a_branch_parent_view_is_never_integrated` (`mutation/bind/lifecycle_tests.rs`) | a `ParentView` literal as at 264 with `path: Branch` | `parent_integrated == false` at `publication_pending` and `complete`; `ended_answer` returns `None` when `answered` |
| D | `d6_a_deleted_or_ended_context_child_abandons_with_its_code` | table: context stops `RemoteContextDeleted`, `HistoryUnresolved`, `TicketClosed`, each `kept` | `abandoned` with `context_remote_branch_deleted` / `context_history_unresolved` / `context_ticket_closed`; the child's row no longer active (let go through `release_promotion_child`, nothing to lose); nothing written; the same request again gives the same |
| D | `d11_a_deleted_remote_branch_after_the_closure_answers_by_the_tracking_primary` | table of 3: `stops = [(Publication, RemoteContextDeleted, true)]`, `fetched = [(Publication, this closure / another / open)]`; a fourth row whose kept row has the full `local_prepared` evidence (11.2), so `release` reports something to lose and the cancel-and-acknowledge path runs | `Merged`, `ClosedElsewhere`, `RemoteBranchDeleted`, all `complete`; in every row the child's row ends terminal before completion |
| D | `answer_table_chooses_each_row_in_order` | 9 rows (`kept` stops for 3 and 4; `fetched` for 1 and 2; `publication_remote` changed in the configuration for 5; an `existing` record for 6; plain for 7; `drive` with another clone's closure for 8 and its withdraw for 9) | each row's `BranchAnswer` and `answer_code`; `complete`; the link; no active child row left |
| E | `d16_the_hold_refuses_every_writer_but_its_own_publication_child` | table; a record at `closing` (`failing(AfterClosureCheckpoint)`, run_tests.rs:143) and at `publication_pending` (D8) | `save_ticket_with`, the slug (`TicketWriteOptions { slug }`), `prepare_context`, `checkpoint_comment_locally_with`, `repair_item` and `execute_retirement_request` answer `RecoveryRequired` with `blocker().operation_id == record op`; the ticket's `begin_local_context_sync_with_request` answers a `ReadError` `RecoveryRequired` whose recovery action names the record's operation; a manual context reservation and the domain republish answer `Busy`; `resolve_history` is not refused by the hold (Q6); the poll row (below) refuses its candidate; then the same request's publication child completes |
| E | D16's poll row (branch only, R24) | a record at `publication_pending` whose publication child was let go (D15's cancelled row, so no child holds the reservation and `poll`'s reservation is `Reserved`, run_tests.rs:1948-1959); the tracking primary and the local primary moved by hand to a commit holding this closure; the remote context reported absent | the poll runs its retirement pass through the entry that reaches `capture_retirement_target` (poll.rs:2177), with `poll::synthetic_fetch` as `context_retirement/acceptance_tests.rs` 313-360 does, if `poll` (run_tests.rs:1938) does not reach it; `PollDisposition::Closed` is observed and its candidate refused; the context ref and worktree are unchanged; then the same request completes (row 1) |

The direct close's existing tests run unchanged and are exempt from fail-first. The refactors' guards (section 0.1) run in their slices.

### 11.4 E-22, E-23 and the mechanical edits

**E-22 (slice A)**, in `closure/record_tests.rs`:
- `steps_permit_only_designed_transitions` (140): the direct assertions are unchanged. It gains `branch_permits` rows: each transition of section 3, and the refusals `Closing→Integrating`, `→Superseded`, `Closing→Published` and `Retired` for `close`.
- `closure_step_round_trips_through_parent_view` (250): `Retired` views as `BranchDeleted`, and writing that back keeps `Retired`.
- `step_invariants_enforced` (324): one `rejected(…)` row and its accepted twin per CHECK of 2.2.
- `no_closed_by_column_or_value` (830): the column list (838-866) gains `integration_mode, action, withdraw_oid, retired_oid, pending_ref, answer_code, consented_context_oid`.
- `migration_is_all_or_nothing_and_idempotent` (91): three markers on the fresh path, none after the fault.
- `closure_refusal_blockers_v2_rebuilds_an_old_bound_table_once` (1324): the old table is built from `SCHEMA_V2` with the mask narrowed, the v2 and modes markers are deleted, and both rebuilds run once.

Two more files:
- `tests/recovery_foundation_gate.rs:1131-1143`: `"wave_06_closure_modes_v1"`, last.
- `recovery.rs:4529-4549` and `tests/read_status.rs:1405, 1821`: unaffected. The implementer confirms each by running it.

**E-23 (slice C):** the new test above. The C1 pair (run_tests.rs:1864, 1894) is unchanged.

**Mechanical edits** (a new arm, field or return type; no assertion's meaning changes):

| Slice | Edit |
| --- | --- |
| A | E-30: `ChildRole::Publication => unreachable!(…)` arms in `remote/reservation_tests.rs` 4542, 4546, 4577, 4633, 4660 and `promotion/record_tests.rs` 222-236 |
| B | `closure/observe_tests.rs:1380-1382` `closure_digest_golden_vector`: the `ClosureObservation` literal gains `path: (Integration::Direct, ClosureAction::Close)`; its golden value must not change |
| C | `TicketClosureResult` arms in `closure/run_tests.rs:250-255` (`authoritative`), `tests/ticket_closure.rs:364-367` and `572-575` (ruling 3; the SSH target is compiled now and run at review-ready) |
| C | `mutation/bind/lifecycle_tests.rs:174-199` `impl LifecycleVerb for Fixed`: `call` returns `Option<Result<Called, Stop>>`; the body stays `None` |
| C | `ParentView`'s literal at `mutation/bind/lifecycle_tests.rs:264` gains `path: RecordPath::Direct, answered: false` |
| C | `promotion/run_tests.rs`: `complete_child_row` extracted from `FakeIo` (417-421), with behaviour unchanged |

E-28, E-29 and E-32 are not applied. E-31's direct lists (reservation_tests.rs 3748-3754, 4413-4425) are unchanged; the branch rows go in the new fence test.

## 12. Slices

Each slice compiles and passes the static gates. The direct tests stay green.

**A. The record's path.**
- `closure/record.rs`: 2.1 to 2.3 except `remote_deletion_required`; `written_commit()` with its readers `effect_written`, `written_checkpoint` and `merged_context`; `branch_permits`; the save dispatch; `supersedable`; the fence SQL.
- `closure/mod.rs`: `ClosureAction`, and `BranchAnswer` with `parse`.
- `promotion/lifecycle.rs`: `RecordPath`, `LifecycleRecord::path`.
- `promotion/lifecycle_record.rs`: `ChildRole::Publication`, `ParentFacts`, `child_allowed`, the slot readers, `remote_identity_of`, `audit`, `second_role`.
- `promotion/run.rs`: the `child_id` and `synchronization_target` arms.
- `mutation/bind/lifecycle.rs`: the `Stages` arms.
- `tests/recovery_foundation_gate.rs`.
- Tests: P7, the fence test. Edits: E-22, E-30.

**B. The branch observation and consent.**
- `closure/observe.rs`: 8.1 to 8.3: `prepare_observation_in` and the stored mode (R25); the pull-request-only `pending_work` read, which leaves out the record's own children; the prepare's `busy`.
- `closure/mod.rs`: `TicketClosurePreview.path`.
- `closure/record.rs`: `begin_branch_record`, `refuse_record_on`.
- `closure/lifecycle.rs`: the begin and refuse dispatch, plus a `require_identity` override that answers `Err(RecoveryRequired)` for a branch observation until slice C. No record or commit is written.
- `promotion/lifecycle.rs`, `promotion/run.rs` (256, 501, 547-557, 672, 726) and `promotion/integrate.rs:122`: `refuse_record` takes `config`, and `preview_request` takes `record`.
- `closure/lifecycle.rs`: `ClosurePreviewRequest`.
- `mutation/bind/lifecycle.rs:379-384`: `observe` names its error's blocker.
- Tests: the two `observe_tests` tests (`pull_request_prepare_decides_the_situation_and_its_blockers`, `a_new_request_observes_live_and_a_continuation_its_record_s_mode`), D17, D18. Edit: `closure_digest_golden_vector`'s literal.

**C. The branch runner, the publication child and the binding's re-entry.**
- `closure/checkpoint.rs`: `build_closure_commit`, the generalized reconciliation, `branch_step`, `publication_answer` rows 6 to 9, `branch_link`.
- `closure/record.rs`: the writers of 2.4, `written_closure_holder`, path-aware `remote_deletion_required`.
- `closure/lifecycle.rs`: the trait methods, `ClosureOutcome`, `BranchFinish`, `end_branch(ContextGone)`; the slice B override removed.
- `closure/mod.rs`: `BranchClosureOutcome`, the result variants, `BranchAnswer::name`, the clock parameter.
- `promotion/lifecycle.rs`: the trait additions, `BranchStep`, `BranchEnd { ContextGone }` with no lifetime, `LifecycleRequest.clock`, `answered`.
- `promotion/lifecycle_record.rs`: `ChildExecution::role`, `is_publication_of`.
- `promotion/run.rs`: the dispatch, `context_step`, `promote_branch` (D5/D7 return), `finish_branch` (rows 6 to 9 and the stop's return), `make_outcome`, the answered checks, `children_contexts`, the refresh contexts, and the interim R-35 arm at 473-479 answering `Err(recovery_required)`.
- `promotion/mod.rs`: `ParentView.path` and `answered`.
- `remote/reservation.rs`: the exemption.
- `pull_request_links.rs`: `upsert_link`.
- `mutation/progress.rs:450`.
- `mutation/bind/lifecycle_verb.rs`: `Called`, `branch_code`.
- `mutation/bind/lifecycle.rs`: `branch_reached`, `parent_integrated`, `ended_answer`, `stopped`.
- Tests: D1 to D5, D7 to D10, D12 to D15, D20, D21, E-23, the hold unit test, the `ParentView` test.
- Edits: the `TicketClosureResult` arms, `Fixed::call`, the `ParentView` literal, `complete_child_row`.

**D. The stop rows and the ends.**
- `closure/checkpoint.rs`: rows 1 to 5.
- `closure/lifecycle.rs`: `end_branch(ContextChild)`.
- `promotion/lifecycle.rs`: `BranchEnd<'a>` with `ContextChild(&'a SynchronizationError)`, and the trait's `end: BranchEnd<'_>`; `branch_answer` gains `remote_current`.
- `promotion/run.rs`: `ends_context_child`, `let_go`, `acknowledge_cancellation`, rows 1 to 5 in `finish_branch`, the R-35 prior-record path (473-479, replacing C's interim arm), `run_observed`'s branch arm.
- Tests: D6, D11, the answer table.

**E. The hold.**
- `closure/record.rs`: `branch_hold`, `hold_refusal`.
- `src/repository.rs`: `refuse_closed_ticket` and its callers.
- `bridges/repair.rs`.
- `context_retirement/authority.rs` and `source.rs`.
- `local_sync/record.rs`.
- `mutation/bind/republish.rs`.
- The writers' bindings' `naming`.
- The Q6 ruling (resolve-history gets no check) lives only here.
- Tests: D16, with its poll row (branch only, R24).

**Review-ready SSH tier for these paths** (Task 16): `ticket_closure`, `document_promotion`, `mutation_remote`, the `remote/` targets, `cli_remote`, `comment_publication`, and `--lib repository::local_sync::`.

## Open questions for the controller

All seven questions of revision 1 are ruled, and so are the checker's points R21 to R25; the rulings are applied above. Two questions remain:

1. **D20 is a correction to the design.** Today the operation list lists a direct close's live child row on its own: `read/status.rs` 705-737 skips only a parent's own row, and 520-540 lists any row that holds the reservation, with no next action. Under the controller's ruling, branch records do the same and d20 asserts it. Task 15's D-1 amendment of `docs/RFC/cli-contract.md` ("`operation list` and `operation show` of a branch record") should say so, and the design's D20 row and Baseline Fact 12 want the same correction. This is for the owner or the documents task to record. It does not block Task 7.

2. **R24: I dispute "a direct record cannot produce a `Closed` candidate through a poll while the fence holds it".**
   - **Dropped as ruled.** The `BeforeRemoteDeletion` row is unreachable: the close holds its parent reservation there (run.rs:1428-1449), which refuses the poll (1368-1371), and the `poll` helper panics unless it gets `Reserved` (closure/run_tests.rs:1948-1959).
   - **Reachable while the fence holds: `publication_pending` after a cancelled primary child.**
     - `finish_steps` lets the child go (`release_child`, 1342-1344) and returns `AwaitingPublication` at 1351-1353, before the cleanup's reservation (1395-1441). So no row holds the reservation, and a poll can reserve.
     - The context child published the context, so an outside deletion is observed as deleted.
     - The local primary holds the integrated closure, which `completed_deleted_context` (poll.rs:279-294) needs for `Closed`.
     - Without the new check, `capture_target` would retire the context under the unfinished direct record.
   - **The same holds at `published`** after the cleanup's full check stops it (1381-1385), before it reserves.
   - **Proposed fix:** a direct row, `a_poll_does_not_retire_a_context_a_direct_closure_holds` in `closure/run_tests.rs` beside 1983-2120. Its setup is `Fixture::remote()` and `CancelAfterPrimary` (1435-1460), a record at `publication_pending` with publication `cancelled`, and the remote context reported deleted. It asserts `Closed` is observed and its candidate refused, the context unchanged, and that the same request then completes.
   - **Status:** not in the slices until the controller rules. Slice E would carry it.

## Appendix E: Task 8 design checkpoint, approved (verbatim)

# Task 8 Design Checkpoint: Withdraw

## Controller approval and corrections (binding; they override the text below where they differ)

Approved by the controller on 2026-11-25 after an independent check (verdict: sound with fixes). Rulings R32 (the published head's closure answers `ticket_closed`, stored as `closed_since`), R33 (any closure on the tracking primary means closed) and R34 (no mode in the withdraw digest) stand.

1. **Slice A's static gates.** `prepare_observation_at` would be dead in the non-test build once `prepare_observation_in` moves to `prepare_observation_for`; mark it `#[cfg(test)]` or delete it and have observe_tests.rs:51 call `observe_for_at(…, Verb::Close, when)`.
2. **`continues` at insertion.** A refused request reaches `refuse_record_on`, whose `existing_record` returns any same-ID record without a `continues` check (record.rs:858-860; run.rs:591, 616). Apply `continues` in both closure `refuse_record`s (or once after the insertion transaction), so a withdraw can never answer, or run `withdraw_step` on, a close record of the same ID.
3. **Ruling 35 (W4 and decision 33).** The binding observes before it checks `held` (bind/lifecycle.rs:1056-1057), so a withdraw observation answering `NothingToWithdraw` would become `already_applied` ahead of the hold. As ruling 28 does for `busy`: when an unfinished non-supersedable record of the ticket exists, the withdraw observation does not answer `NothingToWithdraw` (nor its foreign-synchronization `busy`); it is ready, and the binding's `held` answers `recovery_required` naming the holder. `w4` gains domain rows for the two reachable holders the checker named (a withdraw lost at `BeforeCompletion`, EXITS[12], with W pushed and both contexts open; a close stopped at `confirmed`, D5, on an open ticket), asserting the observation does not answer `NothingToWithdraw` and the holder is found; Task 11 adds the binding row (`recovery_required` naming the holder).
4. **W5.** D7, D9 and D10 are runner-generic and covered by `d7_…`, `d9_…` and `d10_…` (a retained child held by `let_go`, run.rs:1052-1066); additionally script the row-order test's row-6 stop as `PushRejected`, so D9's path through `withdrawal_answer` is exercised.
5. **W12.** The domain variant is tested here; Task 11 names a binding test for `publication_remote_required`.
6. **Test fixes.** W1's direct row: hold first, or refresh after the hold, so the "before" reads are `Closing` and `[ClosingDependency]` (`list_tickets` reads the stored index); the tracking-only row asserts no `dependent_reasons`. `withdraw_answers_choose_each_row_in_order`: run the withdraws at `later`, so `link_given_at() == CLOCK` can fail. W6 clean: assert the head descends from both N and W′ (the head is the Primary-stage merge). W6 conflicting: extend `provisional_closures_may_change_primary_ones_may_not` (integration/tests.rs:220) with the `(A, A, N, …)` rows instead of a new run test. `a_request_continues_only_its_own_verb_s_record`: add a row with a completed direct close's ID and digest, so the run.rs:387-389 arm is shown red too. Add a test for 3.1 step 5's malformed head (`ExternalChange`). W9's fast-forward row: say in the test that `decide_push` mirrors `push_unless_unclosed`, which Task 6 unit-tests.
7. **Docs and anchors.** Update `ClosingState::CheckpointWritten`'s doc (checkpoint.rs:430-433) with slice C's rule. The `[IdentityRequired]` → `ExternalChange` mapping inherited from the close is intended. Anchor drift (current lines): `closure_digest_golden_vector` observe_tests.rs:1455; `pre_checkpoint_refusals_…` run_tests.rs:506; promotion `every_interruption_…` run_tests.rs:514; `remote_children_publication_…` 5212; `never_abandons_*` 938-949; `process_exit_…` 1187.


Designer's checkpoint for Task 8, written before any Task 8 test or code. HEAD is `a2e1a1b3`, so every anchor is today's source at that commit. Paths without a prefix are under `src/repository/`. "The design" is `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-design.md`; "Task 7's checkpoint" is `task-7-checkpoint.md` (approved). Rulings 14 to 31 are the execution ledger's.

## 0. Decisions in one place

- **The lifecycle (Obligation 5).** A new unit struct `TicketWithdrawLifecycle` in `closure/lifecycle.rs` implements `Lifecycle` with `type Record = ClosureRecord` and `KIND = LifecycleKind::TicketClosure`. It is chosen by the verb, statically: its `preview_request`, `preview`, `prepare`, `revalidate` and `ready` observe a withdraw, so the runner's confirm (`promotion/run.rs:541`) and its resumed preview (`run.rs:1154`) observe a withdraw with no change at those call sites. The runner's one change is a guard in `run_observed` (1.3). Everything about the record, the children, the fence, the hold, cancellation, listing and the branch tail is the close's, forwarded (section 1).
- **The action column.** A request continues only a record of its own verb: a defaulted `Lifecycle::continues` says which records a lifecycle continues; both closure lifecycles' `begin_record` and `run_observed`'s guard refuse a record of the other verb with `OperationMismatch` (section 1.3). Inside the shared code the record's `action` selects the written commit (`written_commit`, already built), the `closing` write (`begin_branch_closing`) and the reconciliation's descendant rule (section 1.4).
- **The observation.** One new verb parameter, `observe::Verb { Close, Withdraw }`, reaches `observe_git`. A withdraw takes the branch observation's road in either mode and ends in a new `withdraw_observation` (section 2). A withdraw that has nothing to confirm (no publication remote, or no closure) is not a blocker: it is an internal `Unoffered` reason on the refusal, and a new public `ClosureWithdrawalPreflight` carries it to the binding. `ClosureBlocker` and its stored mask are unchanged (Obligation 10, ruling 15's reading).
- **The digest.** `observe::expectation` dispatches on the path, not the mode, so a direct-mode withdraw has the pull-request domain. The withdraw reuses `branch_expectation_with_head` with action tag 1: it binds the item, the context branch, the primary branch, the remote, the head and the closure being withdrawn (both fields). It binds no mode (Part E's field list; open question 3).
- **The hook.** A new `checkpoint::withdraw_step`, sibling of `branch_step`, writes the commit `Withdraw closure of ticket <ULID>` from a new byte helper `observe::withdrawn_ticket_bytes` (beside `closed_ticket_bytes`, which is in `closure/observe.rs:81`, not `checkpoint.rs` as the brief says). `withdraw_oid` is stored with `closing` before the context moves. A `closing` record with nothing written moves to its stored commit rather than rebuilding it (section 3.4): a withdraw stores no time, so a rebuild would make a second commit.
- **The answers.** A new `checkpoint::withdrawal_answer` gives Part E's rows. Four new `BranchAnswer` values (`withdrawn`, `withdraw_closed`, `closed_since`, `already_applied`), added in Rust only (ruling 17); the action-neutral ones are reused. A withdraw never gives a link: the link record is kept as it is (W1).
- **The hold.** Nothing changes: Task 7's range and writers already cover a withdraw from `closing` (section 5).
- **Entry points.** `RepositoryService::preview_closure_withdrawal` and `RepositoryService::withdraw_ticket_closure` (both `pub`), and `closure::lifecycle::withdraw` for Task 11's observed entry. Nothing of the CLI, the catalog, `Command`, `Mutation`, the DTOs, the preview variant or `way_out.rs` (section 9).

## 1. Obligation 5: the withdraw lifecycle

### 1.1 The type, and what it shares with the close's

`closure/lifecycle.rs`, after `TicketClosureLifecycle` (178):

```rust
/// A withdraw of a provisional closure (Part E) over the closure lifecycle's
/// record: a branch record with the action `withdraw`, in either mode. Its
/// observation, its step 3 and its answers are its own; everything else is
/// the close's.
pub(in super::super) struct TicketWithdrawLifecycle;
impl Lifecycle for TicketWithdrawLifecycle { … }
```

| Trait item (`promotion/lifecycle.rs:395-648`) | Withdraw | Close today |
| --- | --- | --- |
| `Record`, `Error`, `Preview`, `Expectation`, `PreviewRequest`, `Observation`, `Refusal`, `ReplacementGuard`, `Outcome`, `Output`, `BranchFinish`, `KIND` | the close's (`closure/lifecycle.rs:181-192`) | — |
| `type Preflight` | `Result<ClosureObservation, ClosureRefusal>`: the whole refusal, so `ready` can read `unoffered` (2.3) | `Result<ClosureObservation, Vec<ClosureBlocker>>` (185) |
| `read_record`, `save_record`, `replacement_guard`, `clear_cancellation`, `reset_context_after_primary_advance`, `child_state`, `child_execution`, `next_attempt`, `complete_record`, `reconcile_completion` | call the same `record::` functions as 194-320 | — |
| `begin_record`, `continues` | `begin_closure_record` then `continues` (`r.action == Withdraw`) (1.3) | `begin_record` (212-244) becomes the same with `r.action != Withdraw` |
| `refuse_record` | `record::refuse_record_on(…, observed.map_or((config.integration(), ClosureAction::Withdraw), \|o\| o.path))` with `kind = observed.map(kind_of)` and `remote = config.publication_remote` — never the direct `refuse_record`, so a withdraw refused at insertion in direct mode is stored `(direct, withdraw)`, a branch record | 255-272, default path `(config.integration(), Close)`, unchanged |
| `preview_request` | `ClosurePreviewRequest { request, mode: record.map(\|r\| r.integration_mode), verb: Verb::Withdraw }` | 329-342 gains `verb: Verb::Close` |
| `prepare`, `configuration` | the close's (`prepare` now passes `request.verb`) | 354-362 call `observe::prepare_observation_for(service, &request.request, request.mode, request.verb)` |
| `preview` | `Ok(observe::prepare_observation_for(…)?.observed)` | 344-353, the same call, `.map_err(\|r\| r.blockers)` |
| `revalidate` | `observe::revalidate_observation_under_lease(…)` (the whole refusal) | 363-369 unchanged (`revalidate_under_lease`) |
| `ready` | `Ok(o) => Ok(o)`; `Err(r) if r.unoffered.is_some() => Err(Refusal::ExternalChange)`; `Err(r) => <TicketClosureLifecycle as Lifecycle>::ready(Err(r.blockers))` | 373-383 unchanged |
| `refusal_error`, `integration_request`, `write_commit`, `resume_candidate`, `apply` | forwarded to the close's | — |
| `prepare_integration`, `pre_integration` | `Err(TicketClosureError::RecoveryRequired)`, documented "a withdraw is a branch record, which never integrates (Part D)"; `pre_integration_with` and `resume_written` keep the trait defaults | 408-457 |
| `branch_step` | `checkpoint::withdraw_step(service, &request.root, request.confirmed_identity.as_ref(), (request.clock)(), r)` (section 3) | 459-474 |
| `end_branch`, `begin_publication`, `complete_branch`, `branch_outcome`, `outcome`, `complete`, `index_pending` | forwarded: the D6 ends, the mint, the completion writer and the outcome are action-neutral | 475-614 |
| `branch_answer` | own: `checkpoint::withdrawal_answer(&repo, r, primary, child)?` mapped to `BranchFinish { answer, link: None, given_at: (request.clock)().unix_timestamp() }` (section 4) | 517-548 |

`closure/lifecycle.rs` also gains the production entry, beside `close` (39-57):

```rust
pub(in super::super) fn withdraw<P: SessionCredentialProvider>(service: &RepositoryService,
    request: CloseTicketRequest, session: &mut SessionCredentials<P>, probe: Option<CancellationProbe>,
    progress: &mut dyn run::RunObserver, clock: fn() -> OffsetDateTime)
    -> Result<TicketClosureResult, TicketClosureError>
```

It is `close`'s body with `&TicketWithdrawLifecycle`. A withdraw's request is a close's: the same ticket, operation and consent, so `CloseTicketRequest` and `lifecycle_request` (59-81) are reused unchanged.

### 1.2 How `preview_request` and `prepare` observe a withdraw at confirm and on resume

The runner never names a lifecycle; it calls `L::…`. With `L = TicketWithdrawLifecycle`:

- **A new request's confirm.** `run.rs:541-548`, `L::prepare(service, L::preview_request(root, item, identity, None))`: `mode: None` (the live configuration, R25) and `verb: Withdraw`. The revalidation under the preflight lease (`run.rs:561`) is `L::revalidate`, the withdraw observation again with the stored verb (2.1). The refused or begun record has the observed path `(mode, Withdraw)`.
- **A resumed attempt at `confirmed`.** `context_step` (`run.rs:1154`), `preview::<L>(…, Some(&*r))` → `L::preview_request(…, Some(r))`: the record's mode (R25) and `verb: Withdraw`. A branch observation always has `needs_context_child` true, so the child runs whatever the resumed preview says.
- **Step 3.** `withdraw_step` prepares `observe::prepare_observation_for(service, &request, Some(r.integration_mode), Verb::Withdraw)` itself, as `branch_step` does at `checkpoint.rs:943`.
- `promotion/integrate.rs:122` (the integration's preview) is never reached for a branch record (`run.rs:652` dispatches before it).

### 1.3 A same-request continuation keeps the action

- `promotion/lifecycle.rs`, in the trait beside `require_identity` (515-525), a defaulted method (so promotion's impl and every test impl are unchanged):
  ```rust
  /// Whether a request of this lifecycle continues `r`. A closure verb
  /// continues only a record of its own action; every other record is its
  /// lifecycle's own.
  fn continues(_r: &Self::Record) -> bool { true }
  ```
  `TicketClosureLifecycle` overrides it as `r.action != ClosureAction::Withdraw` (a close's verb begins `close` records and, from Task 9, `new_branch`), `TicketWithdrawLifecycle` as `r.action == ClosureAction::Withdraw`.
- `closure/lifecycle.rs`: today's body of `begin_record` (221-243) moves to a private `fn begin_closure_record(tx, id, op, item, expectation, remote, observed) -> Result<ClosureRecord, RepositoryError>`; each impl's `begin_record` is
  ```rust
  let r = begin_closure_record(tx, id, op, item, expectation, remote, observed)?;
  if !Self::continues(&r) { return Err(record::action_mismatch()); }
  Ok(r)
  ```
  and `record.rs` gains `pub(in super::super) fn action_mismatch() -> RepositoryError { parent::mismatch(KIND) }` (beside `begin_record`, 762). Every runner path reaches the record through `L::begin_record` first: the prior-record block (`run.rs:467-478`, `observed: None`) and the insertion (`run.rs:602-610`); `mapped` makes the error `L::Error::operation_mismatch()` (`run.rs:207-213`).
- `promotion/run.rs:368`, `run_observed`'s guard after an error, becomes `if r.item_id() != &request.item_id || r.expectation() != request.expected || !L::continues(&r) { return Err(error); }`. Without it a request that reuses another verb's operation ID and digest would be answered from that record's stored answer by the branch arm at 372-377, after `begin_record` refused it.

The digest also differs by its action tag (`observe.rs:1471-1475`), so `existing_record`'s expectation compare (`promotion/lifecycle_record.rs:370`) refuses a request that sends its own verb's digest with the other verb's operation ID; `continues` makes the column, not a hash, the rule, and catches a request that reuses both.

For the direct path the check never bites: `continues` is `true` for every `close` record and for promotion. Guarded by the direct closure run tests that re-enter a record (`process_exit_at_every_boundary_reopens_without_repeated_effects`, `closure/run_tests.rs:1186`; `pre_checkpoint_refusals_abandon_with_nothing_written_replay_and_a_new_id_works`, 500) and promotion's `every_interruption_reopens_with_one_candidate_and_only_unfinished_children` (`promotion/run_tests.rs:491`).

### 1.4 Where the record's `action` drives dispatch

| Place | Rule | Built |
| --- | --- | --- |
| `TicketWithdrawLifecycle` vs `TicketClosureLifecycle` | chosen by the verb; `begin_record` and `run_observed` refuse the other verb's record through `Lifecycle::continues` | Task 8 |
| `ClosureRecord::written_commit` (`record.rs:264-270`) | `withdraw_oid` for `withdraw`, else `closure_oid`; read by `effect_written` (1501), `merged_context` (279-292), `written_checkpoint` (1421), `reconcile_closing` (439-441) and `finish_context_move` (886-898) | Task 7 |
| `ClosureRecord::path` (`record.rs:251-257`) | `(direct, withdraw)` is a branch record, so it takes `promote_branch` (`run.rs:652`), the branch `validate` (656-719), `branch_permits` (95-126, `withdraw` shares `close`'s row) and the branch fence range (1350-1376) | Task 7 |
| `record::begin_branch_closing` (`record.rs:1024-1061`) | the write must match the action (3.3) | Task 8 |
| `checkpoint::reconcile_closing` (`checkpoint.rs:535-550`) | a descendant of the written commit "holding its closure state": the same closure for a close, an open ticket for a withdraw (3.4) | Task 8, slice C |
| The binding (Task 11) | resume's rebuild of a lifecycle request reads the record's action to rebuild the right input (design Part H), and `held_by_its_child`, `asks_a_parent` and `cancel_parent` reach the new verb | not Task 8 |

## 2. Prepare: the withdraw observation

### 2.1 Entry points and the verb (`closure/observe.rs`)

- New: `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub(in super::super) enum Verb { Close, Withdraw }`, documented "the verb a closure observation observes for: a close (from Task 9 its situation may make it a new branch) or a withdraw".
- New: `fn branch_observed(mode: Integration, verb: Verb) -> bool { mode == Integration::PullRequest || verb == Verb::Withdraw }` — "a pull-request close, and every withdraw, takes the branch record's observation (Part D, Consent and dispatch)".
- `prepare_observation_in` (345-351) keeps its signature and calls the new `pub(in super::super) fn prepare_observation_for(service, request, mode: Option<Integration>, verb: Verb) -> Result<PreparedClosureObservation, RepositoryError>`, which calls `observe_for_at(service, request, mode, verb, now())`.
- `prepare_observation_at` (359-408) keeps its signature for its one test caller (`observe_tests.rs:51`) and becomes `observe_for_at(service, request, mode, Verb::Close, at)`. Its body moves into `fn observe_for_at(…, verb: Verb, at)`, with the `pending_work` argument at 371 changed to `branch_observed(mode, verb)` and `verb` passed to `observe_git`.
- `PreparedClosureObservation` (309-318) gains `verb: Verb` beside `mode`, so a revalidation keeps the verb as it keeps the mode.
- New `pub(in super::super) fn revalidate_observation_under_lease(prepared, service, repository) -> Result<Result<ClosureObservation, ClosureRefusal>, RepositoryError>`: today's body of `revalidate_under_lease` (410-441) without its last `.map_err`, with 419's argument `branch_observed(prepared.mode, prepared.verb)` and `prepared.verb` passed to `observe_git`. `revalidate_under_lease` becomes `Ok(revalidate_observation_under_lease(…)?.map_err(|refusal| refusal.blockers))`; its callers (`closure/lifecycle.rs:368`, `checkpoint.rs:1029`, `observe_tests.rs:732, 771`) are unchanged.
- `observe_git` (696-1079) gains `verb: Verb` after `mode`. Line 710 becomes `let branch = branch_observed(mode, verb);`, so the primary's checkout blockers (765), the root's state (779) and the remote facts (798-803) take the branch reading for a withdraw in direct mode too. The dispatch at 805 becomes `if branch { return match verb { Verb::Close => branch_observation(…), Verb::Withdraw => withdraw_observation(…, mode, …) }; }` with the same `Gathered`.
- `ClosureObservation.identity`'s doc (226-230) widens: "the selected identity of a `Record` closure or a withdraw (its commit's author); `None` for `Existing`".

### 2.2 `withdraw_observation`, in order

`#[allow(clippy::too_many_arguments)] fn withdraw_observation(repository, root, config, item, snapshot, facts, confirmation, mode: Integration, pending: PendingWork, gathered: Gathered) -> Result<Result<ClosureObservation, ClosureRefusal>, RepositoryError>`, after `branch_observation` (1262-1397). Its doc lists the order:

1. **No publication remote** (`config.publication_remote.is_none()`): `Ok(Err(ClosureRefusal::unoffered(Unoffered::PublicationRemoteRequired)))` (W12, R-33). First, as `item republish` checks the remote before the ticket (`mutation/bind/republish.rs:198-210`). Only direct mode reaches it: a pull-request configuration requires a publication remote (`canonical.rs:1467`).
2. **Situation 1, deciding alone** (ruling 29): the local or the tracking primary's ticket is not `FileClosure::Open` (`Malformed` included), read as `branch_observation` reads it (1291-1303): `[AlreadyClosed]` alone. Part E's prepare text lists Part D's blockers and not the situation; it is included because Part E's step 3 ("withdrawing it would be a reopen"), the Cycle document's item 7 ("`ticket_closed` when the primary holds it") and K-7 ("withdraw refuses it as closed") all ask for it, and the binding then answers `ticket_closed` (`bind/lifecycle.rs:218-224`).
3. **Busy** (ruling 28): `pending.synchronization` filtered by `!pending.record_unfinished` → `Err(foreign_synchronization_busy(root, op))`, as 1304-1310. `pending_work` reads it because `branch_observed` is true for a withdraw in either mode.
4. **The blockers** (Part D's, as `branch_observation` keeps them): those `observe_git` gathered (`NotATicket`, `ContextNonconforming`, `ContextDirty`, `ContextConflicted`, `ConflictPending`, `LocalOperationPending`, `CommentPublicationPending`, `PrimaryMissing`, the root's `PrimaryConflicted`); `PrimaryMissing` when the tracking primary is absent (1286-1288); `ContextMissing` when `context_missing` (W14); and, for a context that exists (`!context_missing`) with no tracking context, `remote_history_blocker` (`RemoteContextDeleted`, whose way out is `item republish`; `HistoryUnknown`; `HistoryUnresolved`), as 1311-1314. A ticket with no context is `ContextMissing` and nothing of its remote history (Part E: "a context whose remote branch's deletion is evidenced").
5. **The closure being withdrawn.** The context's ticket through `context_ticket_at` (1098-1116, which inserts `ContextNonconforming` for a non-canonical entry) and the tracking context's ticket through `tree_ticket` on `tracking_context`'s tree; each read with `file_closure` (600-620):
   - the context's holds `Closed { closed_at, closed_by }`: that closure;
   - the context's is `Open` and the tracking context's holds `Closed`: the tracking context's (the withdraw is offered and its context child brings the closure in);
   - either one read is `Malformed`, or `withdrawn_ticket_bytes` refuses the ticket holding the chosen closure: `TicketInvalidAfterClosure`;
   - neither holds one: nothing to withdraw.
6. **Blocked** if any blocker was inserted: `Ok(Err(refusal_of(blockers, None)))`. Blockers come before "nothing to withdraw" because Part E lists them first: a dirty context with no closure answers `context_dirty`.
7. **Nothing to withdraw:** `Ok(Err(ClosureRefusal::unoffered(Unoffered::NothingToWithdraw)))` (W2, decision 11). No identity is asked for it.
8. **The identity** of the commit's author: `select_identity(repository, root, confirmation, true)` (1142-1194); none is `[IdentityRequired]` with its configuration (`refusal_of` gives the configuration only when it is alone, 1228-1238). The identity is not consented: the digest binds no `closed_by` of the withdrawer.
9. **Ready:** a `ClosureObservation` as 1369-1393 with `closure: PlannedClosure::Existing { closed_at, closed_by }` (the chosen closure, exactly as the file has it), `identity: Some(..)`, `base` the tracking primary, `needs_context_child: true`, `changes` empty, `cleanup` all false, `integration: MergeCommit` (unused), `path: (mode, ClosureAction::Withdraw)`, and `expectation = expectation(&observation)`.

The observation's `context_oid` is the head, which `begin_branch_record` stores as `consented_context_oid` (`closure/lifecycle.rs:240`, ruling 14).

### 2.3 Nothing to confirm: the internal reason and the public preflight

- `ClosureRefusal` (272-275) gains `pub(in super::super) unoffered: Option<Unoffered>`, `None` in `refusal_of`. New:
  ```rust
  /// Why a withdraw offers nothing to confirm; `blockers` is then empty.
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub(in super::super) enum Unoffered {
      /// Neither the context's ticket nor its tracking ref's holds a closure (decision 11).
      NothingToWithdraw,
      /// No publication remote: a withdraw's children publish (R-33).
      PublicationRemoteRequired,
  }
  impl ClosureRefusal { fn unoffered(reason: Unoffered) -> Self { … } }
  ```
  A close's observation never sets it, so `into_preflight` (277-286) and the close's `preview` are unchanged.
- At a confirm the reason is a changed situation (Obligation 10): the withdraw's `ready` maps it to `Refusal::ExternalChange`, which the runner stores as today's refusal at insertion, with nothing written. The unoffered reason is never stored.
- `closure/mod.rs`, after `TicketClosurePreflight` (306-317), and re-exported with it at `repository.rs:33-37`:
  ```rust
  /// A withdraw's preflight: a close's two answers, and the two with nothing to confirm.
  #[derive(Clone, Debug, PartialEq, Eq)]
  #[allow(clippy::large_enum_variant)]
  pub enum ClosureWithdrawalPreflight {
      Ready(TicketClosurePreview),
      Blocked { blockers: Vec<ClosureBlocker>, identity_configuration: Option<ExpectedConfiguration> },
      /// `already_applied`, with no data and nothing to confirm (decision 11).
      NothingToWithdraw,
      /// `publication_remote_required` (R-33).
      PublicationRemoteRequired,
  }
  ```
- `PreparedClosureObservation::withdrawal_preflight(&self) -> ClosureWithdrawalPreflight` (beside `preflight`, 319-323) maps the four; `observe::withdrawal_preview(service, request)` is `Ok(prepare_observation_for(service, request, None, Verb::Withdraw)?.withdrawal_preflight())`.
- `RepositoryService::preview_closure_withdrawal(&self, request: PreviewTicketClosureRequest) -> Result<ClosureWithdrawalPreflight, RepositoryError>` (pub), beside `preview_ticket_closure` (`closure/mod.rs:528-533`).

### 2.4 The digest

- `observe::expectation` (1399-1404) becomes `match o.path { (Integration::Direct, ClosureAction::Close) => direct_expectation(o), _ => branch_expectation(o) }`. Today it dispatches on the mode only, which would give a direct-mode withdraw the direct digest (`manyhands closure v1`), against Part D's "for every withdraw … `manyhands pull request closure v1`". For every record that exists today the result is unchanged (a close's path is `(mode, Close)`), guarded by `closure_digest_golden_vector` (`observe_tests.rs:1380`) and `pull_request_digest_wire_format` (1952).
- The withdraw's fields are `branch_expectation_with_head`'s (1464-1491): the domain, the tag `1`, the item, the context branch, the primary branch, the remote, the head, and the closure effect `1 ‖ closed_by ‖ closed_at` (`closure_effect`, 1435-1452). Part E's list (action, item, branch, remote, both fields) is a subset; the primary branch and the head are what Part D binds for every branch record. At step 3 the head is `consented_context_oid` (ruling 14), so a closure the context child brought in from the tracking ref still proves.
- No mode is bound (Part E's list): a withdraw prepared in direct mode and confirmed after a switch begins in the live mode (open question 3).

### 2.5 Direct mode

A withdraw in direct mode is the same observation: `branch_observed` is true, so it reports no `PrimaryNotCheckedOut`, `PrimaryDirty`, `PrimaryAheadOfRemote` or `ContextBehindRemote`, reads pending work with the busy read, and its record is `(direct, withdraw)`, a branch record (`record.rs:251-257`). It needs a publication remote (2.2, step 1).

### 2.6 The byte helper (`closure/observe.rs`, beside `closed_ticket_bytes` at 81)

- `ClosureContentError` (56-73) gains `NotClosed` ("the ticket holds no closure to remove"), and its doc names both helpers. Nothing outside `observe.rs` matches it exhaustively.
- `with_closure` (132-162) takes `closure: Option<(OffsetDateTime, &str)>` and sets both fields from it (`None` clears them); `closed_ticket_bytes` (95) and `closure_replaced_bytes` (113) pass `Some((closed_at, closed_by))`. Behaviour preserving, guarded by `closed_ticket_bytes_round_trips_every_field_and_body`, `closed_ticket_bytes_normalizes_relationships_without_refusing`, `closed_ticket_bytes_refusals` (`observe_tests.rs:212-290`), `closure_replaced_bytes_gives_the_closure_and_keeps_everything_else` (298) and `repository::integration::tests::closure_only_conflicts_settle_with_the_remote_closure` (`integration/tests.rs:381`, through `prepare.rs:228, 311`).
- New:
  ```rust
  /// The ticket's bytes with its closure removed (Part E): parsed, both
  /// fields cleared, serialized canonically, and every other parsed field,
  /// unknown key and the body as they were. `NotClosed` for an open ticket.
  pub(in super::super) fn withdrawn_ticket_bytes(original: &[u8]) -> Result<Vec<u8>, ClosureContentError>
  ```
  `parsed_ticket`, then `NotClosed` when both fields are absent, then `with_closure(&path, &ticket, None)`.

## 3. Step 3: the withdraw hook

### 3.1 `checkpoint::withdraw_step`, in order

`pub(in super::super) fn withdraw_step<E: LifecycleError>(service, root: &Path, confirmation: Option<&ConfirmedCommitIdentity>, now: OffsetDateTime, r: &ClosureRecord) -> Result<BranchStep, E>`, after `branch_step` (`checkpoint.rs:927-1139`). It shares `branch_step`'s parts and none of the direct hook's base checks (R-34):

1. **Prepare and lease**, as 937-955: `observe::prepare_observation_for(service, &request, Some(r.integration_mode), observe::Verb::Withdraw)?` outside the lease; a foreign synchronization of the context answers `foreign_synchronization_busy` (`Repository(RepositoryBusy)`, named; Task 7 correction 4); then the lease. `remote`, `context` (`r.context_oid`), `path`, `context_ref` and `given_at` as 956-962. The abandon closure is 968-982's with `link: None` always: a withdraw gives no link.
2. **At `closing`**: slice B answers `Err(E::recovery_required())` with the record kept ("reconciled in slice C"). Slice C replaces it with 3.4.
3. **The tip:** the context ref is not `r.context_oid` → abandon `Refusal::ExternalChange` / `BranchAnswer::ExternalChange` (as 1005-1008).
4. **The tracking primary's ticket names a closure** (`commit_names_closure`, 288-299, on `refs/remotes/<remote>/<primary>`) → abandon `Refusal::Blocked(vec![ClosureBlocker::AlreadyClosed])` / `BranchAnswer::AlreadyClosed` (W3; withdrawing it would be a reopen).
5. **The head's ticket names no closure** (`!commit_names_closure(repo, context, path)`) → abandon `Refusal::ExternalChange` / `BranchAnswer::AlreadyApplied` (W2: another clone's withdraw arrived). A head that names a closure that does not parse (`commit_closure_text` is `None`) → abandon `ExternalChange` / `ExternalChange`.
6. **Revalidate** `observe::revalidate_under_lease(&prepared, service, &repo)`: `Err` of kind `ExternalChange`, or `Ok(Err(_))`, → abandon `ReconfirmationRequired` / `ReconfirmationRequired` (as 1029-1047).
7. **The consent:** `observe::branch_expectation_with_head(&observation, r.consented_context_oid) != r.expectation` → abandon `ExternalChange` / `ExternalChange`. This is W11: another closure on the head is another `PlannedClosure::Existing` and another digest. The observation's closure must be `PlannedClosure::Existing` (a withdraw observation's always is), else `Err(E::recovery_required())`.
8. **A synchronization still running** (`record::context_synchronization_unfinished`, `record.rs:1392-1400`) → `Err(E::busy())`, before anything is written (as 1079-1085).
9. **The commit:** `integrate::promotion_identity::<E>(service, &repo, root, confirmation)?` (`promotion/integrate.rs:640`), then `build_withdraw_commit(&repo, item, &path, context, whole_utc_seconds(now), &identity)?`; `None` → abandon `ReconfirmationRequired` / `ReconfirmationRequired` (as 1097-1105).
10. **`closing`:** one transaction: `context_synchronization_unfinished` → held off, `Err(E::busy())`; else `record::begin_branch_closing(tx, id, r.operation_id, observation.base, BranchWrite::Withdraw { commit })`. `base_oid` is the tracking primary the hook read (Obligation 2).
11. **The move**, as 1129-1137: `boundary(ClosureAfterClosingRecorded)`, `context_repository`, `context_moving`, `move_context_to_checkpoint_with(&context_repo, item, context, commit, || boundary(service, root, FailurePoint::ClosureDuringCheckpointCheckout))`, `context_moving` (the move marks the ticket's context alone), `boundary(AfterClosureCheckpoint)`, `Ok(BranchStep::Closing)`.

Every abandon is one `abandon_branch_record` transaction under the lease (`record.rs:1148-1174`); one from `closing` happens only after `NothingWritten` (3.4).

### 3.2 The commit

`checkpoint.rs`, beside `checkpoint_subject` (45-47) and `build_closure_commit` (158-211):

```rust
/// A withdraw's commit subject.
fn withdraw_subject(item: &ItemId) -> String { format!("Withdraw closure of ticket {item}") }
/// The commit `Withdraw closure of ticket <ULID>`: the context's ticket with
/// both closure fields removed (`withdrawn_ticket_bytes`, mode kept), parent
/// the context, the identity as author and committer at `at`. Unreferenced.
/// `None` when the ticket is not a canonical regular file or the helper refuses it.
pub(in super::super) fn build_withdraw_commit(repo: &git2::Repository, item: &ItemId, path: &Path,
    context: Oid, at: OffsetDateTime, identity: &CommitIdentity) -> Result<Option<Oid>, RepositoryError>
```

The body is `build_closure_commit`'s (entry and mode at 170-178, blob, tree update, signature, `repo.commit(None, …)`) with `withdrawn_ticket_bytes(original.content())` and `withdraw_subject`.

### 3.3 The record writer (`record.rs`)

- `BranchWrite` (1011-1022) gains `/// A withdraw's commit, built and not yet referenced. Withdraw { commit: Oid }`.
- `begin_branch_closing` (1024-1061): `matching` becomes
  - `Closure { commit, .. }` → `r.action == ClosureAction::Close && r.closure_kind == Some(Record) && !commit.is_zero()`;
  - `Existing` → `r.action == ClosureAction::Close && r.closure_kind == Some(Existing)`;
  - `Withdraw { commit }` → `r.action == ClosureAction::Withdraw && r.closure_kind == Some(Existing) && !commit.is_zero()`.

  The separate `r.action != ClosureAction::Close` test at 1039 is folded into `matching`. The new arm writes `UPDATE ticket_closures SET step='closing',base_oid=?2,withdraw_oid=?3,updated_at=max(updated_at,?4) WHERE operation_id=?1`, and the re-read validates it against the `closing` CHECK's `withdraw` row (`record.rs:406`) and `validate_branch` (682-684). Its doc names the withdraw.

### 3.4 `closing` reconciled on `withdraw_oid` (slice C; W8's later points need none, W13)

In `withdraw_step`, step 2 becomes:

- `reconcile_closing(&context_repo, r)?` (`checkpoint.rs:439`), which already reads `written_commit()` (= `withdraw_oid`):
  - `CheckoutDone` → `context_moving`, `finish_context_move` (886-898, already on `written_commit`), `context_moving`, `boundary(AfterClosureCheckpoint)`, `Ok(Closing)` (as 984-1004);
  - `CheckpointWritten { .. }` → `context_moving`, `Ok(Closing)`;
  - `Unrecognized` → `Err(E::recovery_required())`;
  - `NothingWritten` → steps 3 to 8 again, then step 9 does not build: `r.withdraw_oid` is moved to as it is, when `repo.find_commit(oid)` has one parent equal to `context` and the subject `withdraw_subject(item)`; otherwise `Err(E::recovery_required())`. Step 10's transaction keeps only its `context_synchronization_unfinished` check (no write: the record already holds `closing`). A withdraw stores no time to rebuild with, so a rebuild would be a second commit (W13 asserts one); the stored commit is the one the record was written for (C6's intent for a withdraw).
- `reconcile_closing`'s descendant rule (535-550) is made the record's closure state:
  ```rust
  let same_state = match record.action {
      ClosureAction::Withdraw => commit_holds_open_ticket(context_repo, tip, &path),
      _ => closure_fields.is_some() && commit_closure_text(context_repo, tip, &path) == closure_fields,
  };
  ```
  with a new `fn commit_holds_open_ticket(repo, commit, path) -> bool` (the path is a blob whose front matter names neither key) beside `commit_names_closure`. Direct and close rows are unchanged, guarded by the ten `reconcile_*` tests (`checkpoint_tests.rs:509-700`) and `reconcile_checkout_done_under_line_ending_filters` (1493).
- After `CheckpointWritten` with a descendant tip, `r.context_oid` still names the pre-withdraw head and the record holds `withdraw_oid`; the publication child publishes the tip, which `withdrawal_answer` reads.

Once the publication child is minted (`primary_attempt > 0`) the runner never calls the hook again (`run.rs:929-931`): W8's crashes after it need no reconciliation.

## 4. The publication child and the answers

### 4.1 `checkpoint::withdrawal_answer`

`pub(in super::super) fn withdrawal_answer(repo: &git2::Repository, r: &ClosureRecord, primary: &str, child: Option<&Result<SynchronizationOutcome, SynchronizationError>>) -> Result<Option<BranchAnswer>, RepositoryError>`, after `publication_answer` (1152-1215), whose skeleton it keeps. "The tracking primary" is the record's remote's `refs/remotes/<remote>/<primary>` as the child left it (R-27).

| # | Condition, in order | `BranchAnswer` | Code |
| --- | --- | --- | --- |
| 1 | the tracking primary's ticket names a closure (the withdrawn one or another, a malformed one included; open question 2) | `WithdrawClosed` | `ticket_closed` |
| 2 | no child ran (`child` is `None`: the remote is not the record's, R-35) | `RemoteChanged` | `recovery_required` |
| 3 | child `Err(TicketClosed)` (R-30's refusal, W9's fast-forward) | `WithdrawClosed` | `ticket_closed` |
| 4 | child `Err(RemoteContextDeleted)` | `RemoteBranchDeleted` | `remote_branch_deleted`, never `ok` |
| 5 | child `Err(Transport)` of kind `PublicationRemoteMissing`, `EndpointChanged` or `ConfigurationInvalid` (ruling 20) | `RemoteChanged` | `recovery_required` |
| — | any other child error, or `Ok(PublishPending)` | `None`: the stop stands | the child's |
| 6 | child `Ok`, the published head's ticket names a closure | `ClosedSince` | `ticket_closed` (open question 1) |
| 7 | child `Ok`, the published head's ticket names none | `Withdrawn` | `ok` |

Part E's three groups are these rows: rows 1, 3 and 6 are its `ticket_closed` (row 6 is "the published head holds a closure"), row 4 its `remote_branch_deleted`, row 7 its `ok`. Rows 2 and 5 are R-35's, which Part D gives every branch record. Row 6 cannot be R-30's merge: R-30 gives a merge the closure the fetched primary holds, so row 1 has already answered (open question 1).

### 4.2 Which of the close's rows apply, and how they read `withdraw_oid`

- The close's rows 1, 2 and 6 to 9 compare against "this record's closure" (`checkpoint.rs:1159-1163`); a withdraw has none to compare, so they are replaced by rows 1, 6 and 7.
- The close's rows 3, 4 and 5 keep their conditions, with row 4's answer `WithdrawClosed`.
- `withdraw_oid` is read through the outcome, not the table: `branch_outcome` (`closure/lifecycle.rs:564-580`) gives `checkpoint_oid = written_commit()` (= `withdraw_oid`) for a `complete` record, and `None` for an abandoned one. The published head is the child's authority OID, `withdraw_oid` or a descendant of it (a merge the child made).
- Every row after a stopped child lets it go first (`finish_branch`, `run.rs:1056-1066`); a retained child's stop stands (ruling 31). Both are the runner's, unchanged.

### 4.3 W5's `remote_branch_deleted` rule

A publication child that stops with `RemoteContextDeleted` answers row 1 when the tracking primary (as its fetch left it) names a closure, `WithdrawClosed` → `ticket_closed`; otherwise row 4, `RemoteBranchDeleted`. The record completes either way, and the withdraw commit stays on the local branch (the context branch at `withdraw_oid`, the tracking context where it was; nothing pushed). Task 10 gives row 4 its way out from the action in the data.

### 4.4 Stored `answer_code` values (ruling 17: Rust only, no rebuild)

`closure/mod.rs:96-113` `BranchAnswer` gains, appended, each with `name`/`parse` arms (116-157):

| Value | Stored | After the child (`record.rs:1101-1115` `after_child`) |
| --- | --- | --- |
| `Withdrawn` | `withdrawn` | yes |
| `WithdrawClosed` | `withdraw_closed` | yes |
| `ClosedSince` | `closed_since` | yes |
| `AlreadyApplied` | `already_applied` | no (an abandon, with `Refusal::ExternalChange`) |

Reused unchanged, with the same meaning for a withdraw: `RemoteBranchDeleted`, `RemoteChanged` (after the child); `AlreadyClosed`, `ExternalChange`, `ReconfirmationRequired`, `ContextRemoteBranchDeleted`, `ContextHistoryUnresolved`, `ContextTicketClosed` (abandons; the last three through the forwarded `end_branch`, `closure/lifecycle.rs:475-516`). Each value satisfies the shape CHECK (`record.rs:391`). The record's `action` tells a withdraw's `RemoteBranchDeleted` from a close's.

`mutation/bind/lifecycle_verb.rs:505-527` `branch_code` is an exhaustive match, so it gains the arms in this task: `Withdrawn => Ok`, `WithdrawClosed | ClosedSince => TicketClosed`, `AlreadyApplied => AlreadyApplied`. Withdraw's `already_applied` at step 3 keeps `noop` (`Outcome::of_mutation`, R-32); its context child's publication is in that child's completed row, which Task 10's effects show.

### 4.5 The link record is kept (W1)

`TicketWithdrawLifecycle::branch_answer` gives `link: None`, so `complete_branch_record`'s `give_link` (`record.rs:1176-1197`) writes nothing; the abandons pass `None` too. Neither retirement path runs for a branch record, so the row an earlier close gave stays as it is, `given_at` included. "`ok` with the link record's link, or null" is the answer's data, read from that row by Task 10.

## 5. The writers and the hold

Nothing changes in code. A withdraw record holds its ticket from `closing` until it is finished, as a close does:

- `written_closure_holder`'s SQL (`record.rs:1350-1376`) holds every non-direct record past `context_synchronized` until `complete`/`abandoned`/`superseded`; `withdraw` rows at `closing` and `publication_pending` are pinned by `written_closure_unfinished_covers_branch_records_by_step` (`record_tests.rs:2190-2199`).
- `branch_hold` (1377-1385) gives every branch holder, so slice E's writers (`refuse_closed_ticket`, republish, `ticket repair`, `context retire`, the ticket's `context sync-local`) answer `recovery_required` naming a withdraw's request, in both modes (`refuse_closed_ticket` reads `branch_hold` whatever the mode). Ruling 19 lets `item resolve-history` pass it.
- `closure_holds_context` exempts the holder's own publication child by role (`is_publication_of`), whatever the action; the poll's materialize, fast-forward and retirement checks read the same range (rulings 22, 26).
- Before `closing` the prepare is held by the binding's `held` and step 3 waits on `context_synchronization_unfinished`, as for a close.

The one new check is end to end, in W5: a withdraw at `publication_pending` refuses a ticket save naming its request, in pull-request and in direct mode, and its own publication child then completes. While it holds, the ticket reads `open` (the context's copy is the withdrawn one), not `closing` as a close's does.

## 6. W6 and W7

**W6** is Tasks 3's and 6's behaviour. What proves it today is at the prepare layer only, and in the mirrored direction: G5 (`integration/tests.rs`, "G5 ours withdrawn, theirs unchanged") proves the clean merge for the withdrawing clone, and no row of `provisional_closures_may_change_primary_ones_may_not` (`integration/tests.rs:220`) is W6's conflicting shape (base `A`, local `A`, incoming `N`, primary open, result `N`). `w6_another_clone_s_synchronization_keeps_the_withdraw` proves both for the clone that holds the unchanged closure, which is W6's direction:

- **Clean:** after a close (`C`, published), the local context gets an unrelated commit over `C` (`notes.txt`), the remote branch the other clone's withdraw of `C` (`withdraw_commit_on(C)`); after `p.f.configure_endpoints()` (as `d5_…`'s manual reservation), `sync::promotion_test_drive_local` (`remote/sync.rs:3975`, unbound: no record holds the context) with `SynchronizeRemoteRequest { root, operation_id: OperationId::new(), target: SynchronizationTarget::Context { kind: AuthoringKind::Ticket, item_id: id() }, approval: None, confirmed_identity: None, restart: false }` merges it through `integrate_divergence`'s two stages. The merged head's ticket holds no closure, `notes.txt` is kept, the head has both parents, and the context ref is at it.
- **Conflicting:** ours is `C` with the body changed, theirs the withdraw of `C` with the body changed another way (the closures differ, so the conflict is external repair only, `IncompatibleClosures`). A repair commit with both parents, the merged body and no closure: `integration::validate::validates_external_integration(&repo, ours, theirs, Some(tracking primary), repair)` is `true` (R-12: none is allowed when the primary is open), and `false` with no primary side (today's rule keeps the local closure); a repair keeping ours' closure is `true`; with a primary that holds `C`'s closure, the withdraw-keeping repair is `false`.

W6 pins unchanged behaviour and is exempt from fail-first; the "no primary side" assertion is its contrast. It ends with the local primary unchanged.

**W7** is Part D's close on the withdrawn head: the close observation reads the head open, so its closure is `record` and step 3 writes a second `Close ticket <ULID>` commit on the withdraw commit with `closed_at` from the request's clock (R-23). `w7_…` runs the close, the withdraw and the close again with a later clock, and asserts a new `closed_at`.

## 7. The test plan

### 7.1 Module, fixture and helpers

All run tests are in `repository::closure::run_tests::pull_request` (`closure/run_pull_request_tests.rs`), each by exact name. New helpers there (no existing helper changes meaning):

- `const DEPENDENT: &str` (a valid ULID other than `ID`), `fn later() -> OffsetDateTime` (`CLOCK + 86_400`).
- `PullRequest::direct() -> Self`: `Fixture::remote()`, `switch(&f, Integration::Direct)`, `primary_before = f.primary()`. `PullRequest::local() -> Self`: `Fixture::new()` (direct, no remote).
- `PullRequest::close_first(&self) -> Oid`: a close request run to `Published` with its own `PullRequestIo`, asserting its primary untouched; returns the closure commit `C` (the context's tip, published, the link given at `CLOCK`).
- `PullRequest::withdrawal(&self) -> ClosureWithdrawalPreflight` through `preview_closure_withdrawal`; `withdraw_request(&self) -> CloseTicketRequest` from its `Ready` preview (panics otherwise).
- `PullRequest::withdraw(&self, req, io)`, `withdraw_in(&self, service, req, io)` and `withdraw_at(&self, req, io, clock)`: `run::run_with_io(&TicketWithdrawLifecycle, service, LifecycleRequest { clock, ..lifecycle_request(req) }, io)`. `run_at(&self, req, io, clock)` for the close.
- `PullRequest::withdraw_commit_on(&self, parent) -> Oid`: `parent` with its ticket through `observe::withdrawn_ticket_bytes` (another clone's withdraw).
- `PullRequest::add_dependent(&mut self)`: commits `.manyhands/tickets/<DEPENDENT>/ticket.md` (a canonical ticket with `deps: ["<ID>"]`) on the primary, moves `refs/remotes/origin/main` to it, refreshes the index (`refresh_repository`, as `closure/run_tests.rs:1279-1288`) and resets `primary_before`. `dependent_reasons(&self) -> Vec<ReadinessReasonCode>` from `list_tickets`.
- `fn squash_merge(p: &PullRequest, tip: Oid) -> Oid`: a commit on the primary whose ticket is `tip`'s, one parent.
- `fn withdraw_subject() -> String` in `closure/run_tests.rs`, beside `close_subject` (46-48).
- `fn link_given_at(p: &PullRequest) -> Option<i64>`: `pull_request_links.given_at` for the ticket.

`close_first` and `withdraw_commit_on` land in slice A with `w6_…`; the others in slice B with their first test.

### 7.2 Mechanical edits (slice B)

- `PullRequestIo` (`run_pull_request_tests.rs:428-445`) gains `decide_push: bool` (default `false`). When true, the drive's `Ok(merged)` (531-547) is first put to R-30's push decision as production's `push_unless_unclosed` (`remote/sync.rs:4370-4384`) puts it: `integration::prepare::lacks_primary_closure(&repo, merged, <refs/remotes/origin/main>, &observe::ticket_path(&id()))` true → `Err(SynchronizationError::TicketClosed)` with the row left as the drive left it (kept for its parent, `ended_if_unresolved`'s `keep`), nothing completed or published. Existing tests leave it false.
- `closure_exit_child` (`closure/run_tests.rs:1106-1155`) reads `MANYHANDS_CLOSURE_EXIT_VERB` (absent or `close` → `TicketClosureLifecycle`, `withdraw` → `TicketWithdrawLifecycle`); `exit_at` (1157-1184) becomes `exit_at_with(f, req, point, "close")`, and `exit_at_with` sets the variable. Guarded by `process_exit_at_every_boundary_reopens_without_repeated_effects` (1186), `d12_…` and `d13_…`.
- `a_branch_answer_gives_its_code` (`mutation/bind/lifecycle_tests.rs:350-413`) gains four rows: `(Withdrawn, Ok, "withdrawn")`, `(WithdrawClosed, TicketClosed, "withdraw_closed")`, `(ClosedSince, TicketClosed, "closed_since")`, `(AlreadyApplied, AlreadyApplied, "already_applied")`. Run red first against a `branch_code` arm stub.

### 7.3 Tests, by slice, each red first unless marked exempt

Every run test ends with `p.assert_primary_untouched(&io)` for each io it ran (a test that runs a close first asserts the close's io too).

**Slice A** (observe and the module):

| Test | Setup | Asserts | Red by |
| --- | --- | --- | --- |
| `withdrawn_ticket_bytes_removes_both_fields_and_keeps_everything_else` (`closure::observe::tests`, beside 298) | `hand_written` and `repository_ticket`, both line ends, closed with `closed_ticket_bytes` | the result parses with neither field and every other field, unknown key and the body as the closed ticket's; `closed_ticket_bytes(&withdrawn, at, CLOSER) == closed` (a round trip); `NotClosed` for an open ticket, `NotText`, `NotATicket` for a document | a stub returning `Err(NotPreserved)` |
| `withdraw_prepare_decides_its_answers_and_blockers` (`closure::observe::tests`, after the pull-request table) | `Fixture::new(true)` (direct, remote) and `pull_request_fixture()`; `close_context`, `close_primary`, `tracking`, `closed_base`, `history`, `reserve_context`, `missing_identity`, `write` as the pull-request table uses them; a tracking context ref at a closed commit over the head | rows: no remote (`Fixture::new(false)`, context closed, primary closed too) → `PublicationRemoteRequired`; local or tracking primary closed (malformed too), with a dirty context → `Blocked([AlreadyClosed])`, both modes; a foreign `synchronize_context` row → `RepositoryBusy` naming it; no context (worktree and branch removed, with `history("observed_published")`) → `[ContextMissing]`; `history("observed_published")` with the context closed → `[RemoteContextDeleted]`; context closed → `Ready`, `Existing` with the context's two fields, path `(mode, Withdraw)`, `base` the tracking primary, the child runs, no changes, no cleanup, both modes; only the tracking context closed → `Ready` with its fields; neither → `NothingToWithdraw`, both modes; dirty context, nothing to withdraw → `[ContextDirty]`; no identity with a closure → `[IdentityRequired]` and its configuration; no identity, nothing to withdraw → `NothingToWithdraw`; direct mode with the primary dirty, or not checked out → `Ready` | the dispatch sending `Withdraw` to `branch_observation`; the direct rows against `branch = mode == PullRequest` |
| `withdraw_digest_binds_the_action_and_both_fields` (same module) | as above | the withdraw digest differs from the pull-request close's `existing` digest of the same state (tag); two tracking-only closures differing in `closed_at` alone, then `closed_by` alone, give different digests; a direct-mode withdraw's digest equals the pull-request-mode withdraw's for the same state (no mode bound) and differs from the direct close's | the stub's tag |
| `withdraw_digest_wire_format` (beside 1952) | `ClosureObservation` literal, path `(Integration::Direct, ClosureAction::Withdraw)`, `Existing { closed_at, closed_by }` | `expectation(&o) == blake3("manyhands pull request closure v1" ‖ 1 ‖ item ‖ branch ‖ main ‖ remote ‖ head ‖ 1 ‖ closed_by ‖ closed_at)` | today's mode dispatch (direct digest) |
| `w6_another_clone_s_synchronization_keeps_the_withdraw` | section 6 | section 6 | exempt (Tasks 3 and 6) |

**Slice B** (the lifecycle, the first run of the hook, the answers):

| Test | Setup | Asserts |
| --- | --- | --- |
| `w1_a_withdraw_publishes_and_reopens_in_either_mode` | table: (pull request) `add_dependent`, `close_first`, withdraw at `later`; (direct) `PullRequest::direct()`, `add_dependent`, `hold(closed_commit_on(context, 1_700_000_000, CLOSED_BY))`; (pull request, tracking only) the tracking context at `closed_commit_on(head, …)`, `drive = (Context, it)` | before: `listed_state() == Closing` (not for the tracking-only row), `dependent_reasons() == [ClosingDependency]`; `Withdrawn`, `action() == Withdraw`, `checkpoint_oid() == Some(W)`, `publication() == Published { W }`; one `withdraw_subject()` commit whose parent is `r.context_oid`, time the run's clock, ticket the head's with both fields removed (`withdrawn_ticket_bytes` of the parent's); record `complete`, `answer_code == Withdrawn`, `withdraw_oid == W`, `closure_oid`/`closed_at` none, path `(mode, Withdraw)`; tracking context and context tip at `W`; after: `listed_state() == Open`, `dependent_reasons() == [OpenDependency]`; link: pull request row's `link()` and `link_given_at() == CLOCK` unchanged, direct and tracking-only rows `link() == None`; `!held()`; the same request again gives the same outcome, runs no child and makes no commit |
| `w2_nothing_to_withdraw_is_already_applied_with_nothing_written` | rows: (prepare) the fixture's open context; (confirm) a held closure, `withdraw_request()`, then `hold(open_commit_on(head))` and the tracking context left open; (step 3) `close_first`, `drive = (Context, withdraw_commit_on(C))`; (R-32) `close_first`, `hold(withdraw_commit_on(C))` (an unpublished local withdraw) | prepare → `NothingToWithdraw`; confirm → `Err(ExternalChange)`, through `assert_refused_untouched(…, (PullRequest, Withdraw), …)` and no `withdraw_subject()` commit; step 3 → `AlreadyApplied`, record `abandoned`, refusal `ExternalChange`, `withdraw_oid` none, no withdraw commit, the context as the child left it, `!held()`, the same request answers the same; R-32 → prepare `Ready` (the tracking ref holds the closure), then `AlreadyApplied` with the context child's publication: the tracking context at the local withdraw, `outcome.publication() == None` |
| `w3_a_closure_the_tracking_primary_holds_at_step_three_is_ticket_closed` | `close_first`; table `fetched = [(Context, own_closure_on_primary)]`, `[(Context, another_closure_on_primary)]` | `AlreadyClosed`; refusal `Blocked([AlreadyClosed])`; no withdraw commit; `io.children() == 1` |
| `w4_an_unfinished_closure_record_holds_a_withdraw_and_a_withdraw_holds_a_close` | (i) a close stopped at `publication_pending` (kept transport stop); (ii) a withdraw stopped at `publication_pending` (kept transport stop) | (i) `holder()` names the close (what the withdraw verb's `held` will read, Task 11); `withdrawal()` is `Ready`, not busy (the holder's own child is left out, ruling 28); a new withdraw confirm `Err(Busy)`, no child, one record; (ii) `f.service.prepare(&Mutation::TicketClose(…))` answers `recovery_required` naming the withdraw's request (as `d14_…`, 1244-1258); a new close confirm `Err(Busy)`; one record |
| `w5_a_stopped_child_answers_as_a_close_s_and_a_deleted_branch_by_the_tracking_primary` | `close_first`; table: context transport stop, kept; context `RemoteContextDeleted`, kept; publication transport stop, kept (pull request, and `PullRequest::direct()` with a held closure); publication `RemoteContextDeleted`, kept, with `fetched = [(Publication, own_closure_on_primary)]`; the same with the tracking primary open | context stop → `Err(ContextSynchronization)`, `confirmed`, child kept, then the same request → `Withdrawn`; D6 → `ContextRemoteBranchDeleted` abandon, child let go, no withdraw commit; publication stop → `Err(ContextSynchronization)`, `publication_pending`, `held()`, `listed_state() == Open`, a ticket save refused through `assert_held` naming the withdraw's request (both modes), then the same request → `Withdrawn`; deleted branch → `WithdrawClosed`, resp. `RemoteBranchDeleted`, `complete`, the child ended (`assert_no_active_child`), the context at `withdraw_oid`, the tracking context still `C` |
| `w7_a_close_after_a_withdraw_records_a_new_closed_at` | `close_first` (`CLOCK`), withdraw, then a close request run at `later` | `Published`; two `close_subject()` commits; the new one's parent is `W`, its ticket's `closed_at == later()`; the record's `closed_at == later()`; `listed_state() == Closing`; `link_given_at() == later` |
| `w8_a_crash_after_the_publication_child_never_runs_the_hook_again` | `close_first`; `exit_at_with(…, point, "withdraw")` at `EXITS[7]` (`AfterPublication`) and `EXITS[12]` (`BeforeCompletion`); `run_with_io_observed(&TicketWithdrawLifecycle, &p.f.reopened(), …, &mut Told(..))` | `Withdrawn`; one withdraw commit; `RunEvent::Recording` not told (as `d13_…`, 1150-1173) |
| `w9_a_forge_merge_between_the_children_answers_ticket_closed_in_every_style` | `close_first`; forged computed before the withdraw: fast-forward (`C`), merge commit (`forge_merge(&p)`), squash (`squash_merge(&p, C)`); `PullRequestIo { fetched: [(Publication, forged)], drive: Some((Publication, C)), decide_push: true }` | all `WithdrawClosed` (row 1), `complete`, no active child; fast-forward: the context at `W`, the tracking context `C`, `publication() == None` (K-11); merge and squash: the head's ticket holds `C`'s two fields, `W` is its ancestor, the tracking context at it, `publication() == Published { head }` |
| `w10_a_cancelled_withdraw_is_continued_by_the_same_request` | `close_first`; `CancelAtPublication { parent: Some(req.operation_id) }` (1274-1313) | `Err(Interrupted)`; `publication_pending`, publication `Cancelled`; the same request: a new attempt, `Withdrawn` |
| `w11_another_closure_on_the_head_at_step_three_is_external_change` | `close_first`; `drive = (Context, closed_commit_on(C, 1_750_000_000, "Another <…>"))` | `ExternalChange` abandon; no withdraw commit; `!held()` |
| `w12_without_a_publication_remote_a_withdraw_is_refused_in_direct_mode` | `PullRequest::local()`, a held closure | prepare → `PublicationRemoteRequired`; a confirm with `ClosureExpectation::from_bytes([7; 32])` → `Err(ExternalChange)`, the row `(Direct, Withdraw)` with no remote (ruling 27), `assert_refused_untouched` |
| `w14_a_ticket_with_no_context_is_context_missing` | a held closure, `withdraw_request()`, then the worktree and branch removed (as `observe_tests.rs:1697-1704`) | prepare → `Blocked([ContextMissing])`; confirm → `Err(Blocked([ContextMissing]))`, the refused row `(PullRequest, Withdraw)`, no child, no commit |
| `withdraw_answers_choose_each_row_in_order` | `close_first`; rows: own closure fetched with a completed child; another closure fetched; kept `TicketClosed` with the tracking primary open; a stop, then the remote removed (a new `withdraw_stopped_at_publication`, `stopped_at_publication`'s twin (1645-1657) through `p.withdraw`, then `configure(p, None, "main")` as `remote_removed`, 1876-1879, so the runner's R-35 arm runs with no child); kept `Transport(PublicationRemoteMissing)`; a stop, then `drive = (Publication, closed_commit_on(W, …))` (`W` is known only once the first run wrote it); plain | `WithdrawClosed`, `WithdrawClosed`, `WithdrawClosed`, `RemoteChanged`, `RemoteChanged`, `ClosedSince`, `Withdrawn`; each `complete`, no active child, `link_given_at() == CLOCK` |
| `a_request_continues_only_its_own_verb_s_record` | (i) `close_first`'s request sent to `TicketWithdrawLifecycle`; (ii) a withdraw stopped at `publication_pending` sent to `TicketClosureLifecycle` | both `Err(OperationMismatch)`, the record unchanged, no child; (ii)'s withdraw request then gives `Withdrawn`. Red: without `continues`, (i) answers the close's stored `Published`; with `begin_record`'s check but not `run_observed`'s, (i) still answers it through 372-377 |
| `begin_branch_closing_writes_what_the_action_writes` (`closure::record::tests`) | `db()`, `branch_row` at `context_synchronized` with `closure_kind` `existing` and `context_oid` | withdraw + `Withdraw { commit }` → `closing`, `withdraw_oid`, `base_oid`; withdraw + `Closure`/`Existing` refused; close + `Withdraw` refused; a zero commit refused |

**Slice C** (`closing` reconciled):

| Test | Setup | Asserts | Red by |
| --- | --- | --- | --- |
| `w13_a_withdraw_lost_at_closing_is_reconciled_on_its_commit` | `close_first`; `exit_at_with(…, point, "withdraw")` at `EXITS[2]` (`ClosureAfterClosingRecorded`), `EXITS[3]` (`ClosureDuringCheckpointCheckout`), `EXITS[4]` (`AfterClosureCheckpoint`); then `withdraw_in(&p.f.reopened(), …)` | each: the record was `closing` with `withdraw_oid`; `Withdrawn`; the same `withdraw_oid`; one withdraw commit in the ODB (the exit child ran at the wall clock and the retry at `CLOCK`, so a rebuild would make two); the context at it | slice B's interim `recovery_required` |
| `reconcile_closing_reads_a_withdraw_by_its_commit` (`closure::checkpoint::tests`, after 700) | `Fixture::new(true)`, the context closed, a withdraw record at `context_synchronized` (`begin_branch_record` with `(PullRequest, Withdraw)`, `save_record`), `build_withdraw_commit`, `begin_branch_closing(Withdraw)` | ref at the context, ticket unchanged → `NothingWritten`; checkout without the ref update → `CheckoutDone`; ref at `withdraw_oid` → `CheckpointWritten`; a descendant with an open ticket → `CheckpointWritten { tip }`; a descendant whose ticket holds a closure → `Unrecognized` | the descendant row against today's rule |

### 7.4 Guards run in each slice (unchanged behaviour)

- **A:** every `closure::observe::tests` test (with `closure_digest_golden_vector` and `pull_request_digest_wire_format`), `closure_only_conflicts_settle_with_the_remote_closure`, and the module's `d17_…`, `d18_…`, `a_new_request_observes_live_and_a_continuation_its_record_s_mode`, `a_foreign_context_synchronization_is_busy_at_prepare_and_confirm`.
- **B:** every test in `repository::closure::run_tests::pull_request` (the close's `d…` and answer-table tests go through `begin_record`'s `continues`, `begin_branch_closing` and `TicketClosureLifecycle::preview_request`), `closure::record::tests`, `a_branch_parent_view_is_never_integrated`, `process_exit_at_every_boundary_reopens_without_repeated_effects` and the direct closure run tests that re-enter a record (`pre_checkpoint_refusals_abandon_with_nothing_written_replay_and_a_new_id_works`, 500), and, for `run_observed`'s guard, promotion's `every_interruption_reopens_with_one_candidate_and_only_unfinished_children` (`promotion/run_tests.rs:491`) and `remote_children_publication_cleanup_and_replay_are_once_only` (5189).
- **C:** the ten `reconcile_*` tests and `reconcile_checkout_done_under_line_ending_filters`, `d12_…`, `d13_…`, and `never_abandons_after_the_checkout` / `_after_the_checkpoint` / `_with_an_adopted_descendant` (`closure/run_tests.rs:932-943`).

No SSH target runs between steps; `ticket_closure` and `document_promotion` compile through these files and run at review-ready (Task 16).

## 8. Slices

Each compiles, passes the static gates, adds no function without a production caller, and leaves the direct and close tests green.

**A. The withdraw observation and consent.**
- `closure/observe.rs`: `Verb`, `branch_observed`, `prepare_observation_for`, `observe_for_at` (with `prepare_observation_at` delegating), `PreparedClosureObservation.verb`, `revalidate_observation_under_lease`, `observe_git`'s verb, `withdraw_observation`, `Unoffered` and `ClosureRefusal.unoffered`, `withdrawal_preflight`, `withdrawal_preview`, `expectation`'s path dispatch, `withdrawn_ticket_bytes`, `ClosureContentError::NotClosed`, `with_closure`'s option.
- `closure/mod.rs`: `ClosureWithdrawalPreflight`, `RepositoryService::preview_closure_withdrawal`. `repository.rs:33-37`: the re-export.
- Tests: the four observe tests and `w6_…` (exempt). No mechanical edits.

**B. The withdraw lifecycle, its hook and its answers.**
- `promotion/lifecycle.rs`: `Lifecycle::continues` (defaulted). `promotion/run.rs:368`: `run_observed`'s guard.
- `closure/lifecycle.rs`: `TicketWithdrawLifecycle`, `withdraw`, `ClosurePreviewRequest.verb`, `begin_closure_record` and both `continues`.
- `closure/checkpoint.rs`: `withdraw_subject`, `build_withdraw_commit`, `withdraw_step` (with the interim `closing` arm), `withdrawal_answer`.
- `closure/record.rs`: `BranchWrite::Withdraw`, `begin_branch_closing`'s per-action match, `after_child`'s three values, `action_mismatch`.
- `closure/mod.rs`: the four `BranchAnswer` values; `RepositoryService::withdraw_ticket_closure` (pub, as `close_ticket` at 538-552).
- `mutation/bind/lifecycle_verb.rs`: `branch_code`'s arms.
- Tests: `w1_…` to `w5_…`, `w7_…` to `w12_…`, `w14_…`, `withdraw_answers_choose_each_row_in_order`, `a_request_continues_only_its_own_verb_s_record`, `begin_branch_closing_writes_what_the_action_writes`.
- Mechanical edits: 7.2's three, and `withdraw_subject()`.

**C. `closing` reconciled on `withdraw_oid`.**
- `closure/checkpoint.rs`: `withdraw_step`'s `closing` arm (3.4), the stored-commit move, `reconcile_closing`'s descendant rule, `commit_holds_open_ticket`.
- Tests: `w13_…`, `reconcile_closing_reads_a_withdraw_by_its_commit`.

## 9. Where Task 8 stops, and what Task 11 binds

Task 8 adds no CLI verb, catalog entry, `Command::TicketWithdrawClosure`, `Mutation::TicketWithdrawClosure`, `TicketWithdrawClosureInput`, `LifecycleVerb` impl, Session method, DTO, preview variant, schema, golden, `way_out.rs` row or desktop arm. Its only binding-file edit is `branch_code`'s four arms (an exhaustive match the new values break) and their test rows.

Task 11 binds:
- `RepositoryService::preview_closure_withdrawal(PreviewTicketClosureRequest) -> Result<ClosureWithdrawalPreflight, RepositoryError>` for the verb's `observe`. It maps `NothingToWithdraw` to `already_applied` with no data and nothing to confirm (P15's golden), and `PublicationRemoteRequired` to `publication_remote_required` (the binding should also answer that from the configuration before the domain, as `bind/sync.rs:1539-1543` does). `NothingToWithdraw` must not pre-empt decision 33: the binding observes before `held` (`bind/lifecycle.rs:384-426`), so `held` must still answer `recovery_required` for an unfinished record (W4).
- `closure::lifecycle::withdraw(service, CloseTicketRequest, session, probe, progress, clock)`, through a new `RepositoryService::withdraw_ticket_closure_observed` (pub(super), as `close_ticket_observed`, `closure/mod.rs:554-563`) and a Session method beside `close_ticket` (`mutation/progress.rs:441-457`). Its answers are `TicketClosureResult::Branch` / `BranchIndexPending` with `action() == Withdraw`, and its errors `TicketClosureError`.
- `RepositoryService::cancel_ticket_closure` (unchanged) for `request cancel`; `promotion::lifecycle_parent::<ClosureRecord>` and `unfinished_holder::<ClosureRecord>` for the verb's `parent` and `holder` (`lifecycle_verb.rs:481-500`, as the close's).

Task 10 gives the withdraw's data (`recording: withdrawn` or `none`, `checkpoint_oid` from `withdraw_oid`, the link row as it stands), the preview variant from a `TicketClosurePreview` whose path is `(mode, Withdraw)`, R-32's effects (the context child's publication), and the way out of `RemoteBranchDeleted` for a withdraw and of `RemoteContextDeleted` (`item republish`).

## Open questions for the controller

1. **Row 6, "the published head holds a closure (R-30 restored it in a merge)".** R-30 gives a merge the closure the fetched primary holds, so the tracking primary names it and row 1 answers first; row 6 is reachable only when no primary holds a closure and the published head does, that is another clone's closure merged in (for example a re-recorded closure kept by a conflict repair, G6's shape). Part E answers `ticket_closed`, which says the ticket is closed when it is closing. Built as the design says (`ticket_closed`), stored as its own value `closed_since`, so a ruling of `already_closing` changes only `branch_code`'s arm and its test row.
2. **Row 1's "the withdrawn closure".** Built as "the tracking primary names any closure": another closure there is still a closed ticket, and the alternative ("otherwise `ok`") would report a withdraw of a ticket the primary has closed. Evidence: R-27, the Cycle document's item 7 ("`ticket_closed` when the primary holds it"). If ruled otherwise, row 1 compares both fields and another closure falls to a new row.
3. **No mode in the withdraw's digest.** R-2 says a mode switch between prepare and confirm is `external_change`, by the digest's domain; a withdraw's observation and domain are the same in both modes and Part E's field list binds no mode, so a switch is not refused and the record begins in the live mode. Nothing of a withdraw's run depends on the mode (the hold and the guard read the configuration at each call). Built per Part E. If R-2 must hold, the narrowest fix is one mode byte after the action tag in `branch_expectation_with_head` for `Withdraw` only (the close's wire format is unchanged).

## Appendix F: Task 9 design checkpoint, approved (verbatim)

# Task 9 Design Checkpoint: The New Branch

## Controller approval and corrections (binding; they override the text below where they differ)

Approved by the controller on 2026-11-25 after an independent check (verdict: sound with fixes; U-1 holds by generalizing `finish_parent`). Rulings R36 to R44 on the designer's open questions stand, and E-44 is approved (it replaces two rows this Cycle wrote in Task 7). The check's fixes, ruled:

1. **R45, the ancestor check counts every push `push_absence` counts.** The third value of situations 2 and 3.1 (R36) is every row `push_absence` counts, proven or intent, compatible or not, with its published OID; an earlier push that is not the retired tip's ancestor blocks prepare with `history_unresolved` (the design says "an earlier push", not only proven ones), so the boundary after the retirement can never read `Ambiguous` or `Unknown` for a record prepare offered.
2. **R46, a retry after this record's own tracking deletion.** When the tracking ref is absent and the record bound `remote_context_oid` (Fact 16: an absent ref was deleted by this record), §2.2.5 accepts the bound evidence instead of re-reading it (lost-history evidence is read from the tracking ref, sync.rs:8355-8404), so the retry continues the retirement rather than abandoning with the old worktree and branch already removed.
3. **R47, a stale tracking-ref lock is a known limit.** A process death at the TRACKING or COMMIT seam leaves the tracking ref's `.lock` (held in `RetirementEvidence`, cleanup.rs:600-631), as for the direct close today (closure/run_tests.rs:1216-1230): the record stops with `recovery_required` until an operator removes the lock, and the same request then continues. Record it as a Known Limit (Task 15). F10's tracking-deletion test asserts the sequence (first retry restarts the row, epoch +1, and stops at `lock_ref` with the row reserved; the simulated operator removes the lock; the second retry, epoch +2, continues), and F17 removes the lock as the simulated operator and asserts the epoch then.
4. **R48, R-35 during the retirement path.** A remote or configuration change after step 1 answers `recovery_required` at §2.1.1, and the way out is restoring the configuration; the same request then continues. `require_configuration` is re-validated before each step, as `finish_steps` does.
5. **Step 3** re-checks `context_synchronization_unfinished` (busy), for symmetry with `branch_step` and Part D's "its step 3 waits".
6. **3.1's fallback** to `last_advertised_oid` requires `find_commit` at prepare; a commit not in the local object store gives `history_unresolved`.
7. **R40's knock-ons:** a kept closure's `closed_at` is T's (not the clock's) in §1.6 and §8 row 3; `build_kept_closure_commit`'s commit time is the injected clock's; F9 asserts the record's `closed_at` equals T's.
8. **R49, a pull-request close that meets a recorded deletion.** `branch_step` re-observes (checkpoint.rs:1144): if a poll records the remote deletion between the context child and step 3, the observation becomes `new_branch`, the digest differs, and the close abandons with `external_change` before writing; a new `ticket close --prepare` then offers the new branch. This replaces Task 7's behaviour (closure written, then `remote_branch_deleted` by row 3) for that window. Add a slice A test row for it.
9. **Slice A static gates.** Defer `TicketClosurePreview::new_branch()` to Task 10, its first caller, so no dead accessor lands. `build_kept_closure_commit` and `begin_new_branch_record` take `#[allow(clippy::too_many_arguments)]`, as record.rs:790 and 885 do.
10. **The interim guard.** A changed-digest confirm in slice A stores a refused `(PullRequest, NewBranch)` row through `refuse_record_on`: terminal and valid, accepted. "No new-branch record before slice B" means none that runs.
11. **Test fixes.**
    - F5's confirm row expects the refused row on path `(PullRequest, Close)`, since a blocked revalidation leaves `observed` empty (run.rs:541, 563-564; lifecycle.rs:318).
    - E-44's no-context rows prune the worktree instead of `remove_dir_all`, as `already_closed` does (observe_tests.rs:706-719).
    - F11 and F16 capture the digest before the run, because `p.request()` panics on a blocked preview.
    - F1 and F4 assert the tracking ref's deletion from a hook at `BeforeContextBranchCreation`, or else assert the ref is at C, because the fake publication force-sets it.
    - F16 removes its boundary hooks before the retry.
12. **Wording and minors.**
    - §0's "nothing stops while holding" is reworded: `validate_cleanup_owner`, `RecoveryRequired` from `require_local_cleanup_absent` or `delete_tracking_ref`, and SQL errors stop with the row reserved, as the direct close does.
    - §8's "ref at C (an abandon whose transaction failed)": the abandon deletes the ref first, so the ref is absent and the retry recreates it.
    - §2.2.7 includes `boundary(PromotionAfterTrackingDeletion)` (run.rs:2097).
    - A call entering with `closure_oid` already stored still takes the repository lease.
    - `withdraw_observation`'s destructuring of `Gathered` (observe.rs:1561-1569) is a mechanical edit.
    - Every pull-request close prepare now runs `push_absence`, which can answer `recovery_required` on an inconsistent sync row, as `item sync` and `item republish` already can.


Designer's checkpoint for Task 9 (plan Step 3), written before any Task 9 test or code. HEAD is `4c285865`; every anchor is `file:line` in today's source at that commit. Paths without a prefix are under `src/repository/`. "The design" is `docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-design.md`; "Task 7's" and "Task 8's checkpoint" are the approved `task-7-checkpoint.md` and `task-8-checkpoint.md`, whose controller corrections apply here unchanged. Rulings 14 to 35 are the execution ledger's.

## 0. Decisions in one place

- **Where it lives.** A new module `closure/new_branch.rs` (`pub(super) mod new_branch;` beside `closure/mod.rs:26-29`) runs steps 1 to 3 of Part F under the closure's own leases. It calls the lifecycle's cleanup functions (`cleanup.rs` `ContextCleanup`) directly, in `finish_steps`'s order (`promotion/run.rs:1953-2122`), as the plan's file list says ("step 2's retirement through the lifecycle's cleanup functions").
- **Dispatch.** `TicketClosureLifecycle::branch_step` (`closure/lifecycle.rs:520-533`) sends a `new_branch` record to `new_branch::step` and every other record to `checkpoint::branch_step`, as today. `promote_branch` (`promotion/run.rs:917-997`) skips the context child for a record that has none, through a new defaulted `LifecycleRecord::synchronizes_context`. `finish_branch` (`run.rs:999-1107`) serves the publication child and the answer table unchanged.
- **U-1: yes, by generalizing `finish_parent`** (`remote/reservation.rs:1217-1249`). Its checks (owned row, action `close`, manual priority, `completed_step == AfterAdvertisement`) all hold for the new branch's reservation. Only its one SQL ties it to `step='complete'` and to `retired_oid = coalesce(adopted, closure, context)`. Section 3 widens that SQL to a new branch's `retired` end, and to its abandon under the reservation. Every direct and promotion condition stays as it is. `finish_remote_operation` (`reservation.rs:2125-2228`) cannot be generalized: it refuses any target that is not a poll or a synchronization (`2182-2186`).
- **The reservation is held only for the remote part** (R-31). Between `Reserved` and the retirement's transaction nothing stops while holding it. A cancellation is honoured at the next boundary after `retired` (section 2.3).
- **Prepare.** Part D's situations 2 and 3 become real in `branch_observation` (`closure/observe.rs:1390-1523`). The situation is read from a new remote-history read in the observation's own registry transaction: `republish_evidence`'s body, and every proven push. `unheld_entries` (`context_retirement/proof.rs`) lists what the old tip holds that the tracking primary lacks. The digest gains the new branch's fields. The preview carries them.
- **Rust-only values (ruling 17).** One new `BranchAnswer`, `ContextChanged` (`context_changed`, an abandon). One preflight-only `ClosureBlocker`, `RemoteContextUnmaterialized`, appended after `HistoryUnresolved` and left out of `CLOSURE_BLOCKERS` (ruling 15). Two new `FailurePoint`s, `NewBranchCommitRecorded` and `NewBranchCommitHeld`. No schema change: Task 7's rebuild already holds `retired_oid`, `pending_ref`, step `retired`, `new_branch`'s permits and CHECKs (`closure/record.rs:95-126`, `360-418`).
- **The hold from step 1 is built** (Task 7). `written_closure_holder`'s SQL (`record.rs:1375-1395`) holds a `new_branch` at `confirmed` once `closure_oid` is stored, and at every later unfinished step. Task 9 adds no fence code; F14 pins it.

## 1. Step 1: the commit, its pending ref, the hold

`new_branch::step` (section 0) first runs, when `r.step == Confirmed` and `r.closure_oid.is_none()`:

1. **Observe outside the lease**, as `branch_step` does (`checkpoint.rs:1038-1046`):
   - `observe::prepare_observation_in(service, &request, Some(r.integration_mode))`.
   - A foreign synchronization of the context (`observe::foreign_synchronization`) gives `Err(foreign_synchronization_busy(..))`, named; nothing is written.
   - Then `repository_lease(&repo, root, RepositoryOperation::RepositorySnapshot)`. This one lease covers steps 1 and 2's local part (`ContextCleanup::holding`, `cleanup.rs:154-165`).
2. **Revalidate under the lease:** `observe::revalidate_under_lease(&prepared, service, &repo)`.
   - `Err` of kind `ExternalChange`, or `Ok(Err(blockers))`, abandons with `ReconfirmationRequired` / `BranchAnswer::ReconfirmationRequired`, as `branch_step` does (`checkpoint.rs:1124-1141`). The one exception: blockers holding `RemoteContextUnmaterialized` are a situation that changed, so they abandon `ExternalChange` / `ExternalChange` (Part D confirm step 1).
   - The observation's path must be `(PullRequest, NewBranch)`, and `observation.expectation == r.expectation`. Otherwise abandon `ExternalChange` / `ExternalChange` (F8: `P`, `T`, a tracking tip or the unheld entries changed). No context child ran, so the whole digest compares, its head included.
3. **`record::context_synchronization_unfinished`** (`record.rs:1417-1427`) gives `Err(TicketClosureError::Busy)`, with nothing written.
4. **The identity**, as `branch_step` does (`checkpoint.rs:1178-1191`): `integrate::promotion_identity(service, &repo, root, confirmation)?`.
   - For a `record` closure, `closed_by_text(&identity)` must be the consented `closed_by`, and the live selection must agree. Otherwise abandon `ExternalChange`.
   - For a kept closure the identity is only the commit's author, so it is not compared.
5. **Build `C` on `P = observation.base`** (the tracking primary). The ticket is `P`'s, given the closure's fields. `C` is unreferenced.
   - Record: `checkpoint::build_closure_commit(&repo, item, &path, P, whole_utc_seconds(now), &identity)` (`checkpoint.rs:161-191`). Its `context` parameter is the parent, here `P`. It edits only the ticket path, through `commit_edited_ticket` (`248-291`).
   - Kept (closure kind `existing`, R-21): a new `checkpoint::build_kept_closure_commit(repo, item, path, P, at, identity, kept_at, kept_by)`, built on the same `commit_edited_ticket` with `|original| observe::closure_replaced_bytes(original, kept_at, &kept_by)` (`observe.rs:109-115`) and the subject `checkpoint_subject(item)`.
     - `kept_at` and `kept_by` are read now from `T`'s tree: the canonical `Ticket` at the ticket path of `r.consented_context_oid`.
     - Item 2's digest compare has just proven they are the consented `PlannedClosure::Existing`.
   - `None` (not a canonical regular file, or the edit refused) abandons `ReconfirmationRequired`.
6. **One transaction.** `context_synchronization_unfinished` gives `Busy` with nothing stored. Otherwise the new `record::begin_new_branch(tx, id, op, base: P, closed_at, commit: C, tracking: observation.tracking_context)`:
   - It requires a `new_branch` record at `confirmed` with no `closure_oid`, and non-zero OIDs.
   - It writes, in one UPDATE: `closed_at`, `closure_oid=C`, `base_oid=P`, `remote_context_oid=<tracking>` (the tracking context tip the consent bound; none when there is no tracking ref), and `pending_ref='refs/manyhands/closures/'||operation_id`. The step stays `confirmed`. The re-read validates it (`record.rs:656-750`; the pending-ref and `closed_at` rules already allow it).
   - `closed_at` is the clock's whole seconds, the same for both kinds (open question 5). C6 holds for a `record` closure: the time is stored with the commit before anything refers to it.
   - **From this commit on, the record holds the ticket** (`record.rs:1385`, `(action='new_branch' AND closure_oid IS NOT NULL)`).
7. `run::boundary(.., FailurePoint::NewBranchCommitRecorded)` (new: "a new branch's commit is recorded, unreferenced; its pending ref is not made").
8. **The pending ref** `refs/manyhands/closures/<operation ULID>` is pointed at `C`:
   - absent: create it;
   - at `C`: keep it;
   - anywhere else: `Err(RecoveryRequired)`. The name is the record's own, and SQL is written before the ref, so a ref elsewhere is not Manyhands'.

   This order means a retry never meets a stray ref of its own with no `closure_oid` stored.
9. `run::boundary(.., FailurePoint::NewBranchCommitHeld)` (new: "a new branch's commit is held by its pending ref; the old context is untouched").

On a later call that finds `closure_oid` stored, step 1 only ensures the pending ref:
- absent: recreate it at `closure_oid`, which must exist (`find_commit`), else `RecoveryRequired`;
- at `C`: go on;
- elsewhere: `RecoveryRequired`.

No re-observation, rebuild or second commit follows (F10).

Every abandon in sections 1 and 2 is `new_branch::abandon` (section 2.4).

## 2. Step 2: the retirement (situations 2 and 3.1: `r.retired_oid.is_some()`)

The oid every cleanup function is given is `r.merged_context()`. It is `retired_oid` from `confirmed` through `retired` (`record.rs:279-292`, Task 7 slice A's fix): `T` in situation 2, the 3.1 target otherwise. "An old context" means `r.consented_context_oid.is_some()` (`T`, section 7.2).

### 2.1 The local part: under the lease, no reservation

The order and the checks are `finish_steps`' (`run.rs:1953-2031`):

1. **Checks before the removals.**
   - `run::cancelled::<TicketClosureLifecycle>(service, request, r)` gives `Err(Interrupted)`, at the stored step.
   - `run::require_configuration(root, primary, r.remote_name())` (`run.rs:2164-2174`) gives `Err(RecoveryRequired)` with nothing reserved. `primary` is `run::recorded_config(root, r)?.primary_branch` (`run.rs:437-454`), read from `P`'s tree.
2. **`confirmed`.**
   - With an old context: `cleanup.full_check(service, Ticket, item, oid)` (`cleanup.rs:369-377`). Anything but `Precheck::Ok` abandons with `Refusal::ReconfirmationRequired` / `BranchAnswer::ContextChanged` and deletes the pending ref (Part F step 2: "a refusal abandons the record with the cleanup's code").
   - In 3.1 there is no context to check.
   - Then `checkpoint::context_moving(service, root, item)` marks the ticket's context owed an indexing before Git is touched, as `checkpoint.rs:711-724` does (R-SDD-49; made `pub(super)`).
   - Then step `worktree_removing` (`r.step = WorktreeRemoving; r.cleanup = None`), through `record::save_record` (`branch_permits`: `Confirmed → WorktreeRemoving`). `fresh = true` for this call only.
3. **`worktree_removing`.**
   - `run::boundary(.., DuringWorktreeRemoval)`, then the cancellation check.
   - The removal:
     - an old context: `remove_fresh_context_worktree` when `fresh`, else `remove_context_worktree` (`cleanup.rs:334-362`);
     - 3.1: always `remove_context_worktree`, which reads a worktree that is not there as `AlreadyAbsent` before any check (`cleanup.rs:427-436`).
   - `Removed` (with `#[cfg(test)] promotion::run_tests::local_effect(.., "worktree_remove")`) or `AlreadyAbsent` goes on. `Changed` stops with `ContextChanged`; `RecoveryRequired` stops with `RecoveryRequired` (2.3).
   - Then step `worktree_removed`, with `cleanup = None`.
4. **`worktree_removed`.**
   - The cancellation check, `run::boundary(.., BeforeBranchDeletion)`, the cancellation check again.
   - `cleanup.delete_context_branch(Ticket, item, oid)` (`cleanup.rs:477-484`). `Deleted` (with `local_effect(.., "branch_delete")`) or `AlreadyAbsent` goes on. `Changed` stops with `ContextChanged`.
   - Then step `branch_deleted`.
5. **`branch_deleted`.** The cancellation check, then `run::boundary(.., BeforeCompletion)`. This point is before the reservation; finish_steps' own is under its owner (`run.rs:2049`). Then the cleanup capability, and so the lease, is dropped before reserving, as `run.rs:1777-1782` drops it.

### 2.2 The remote part: the parent's own `close` reservation (R-31, U-1)

As `finish_steps` takes it (`run.rs:1788-1849`):

1. **The plan and the endpoints.** `plan = RemoteRefPlan::from_configuration(remote, primary)`, `frozen = service.observation_configuration(root, &plan)`, then `state::configure_endpoints` in a transaction.
2. **The reservation.** `target = RemoteOperationTarget::for_context(&plan, KIND.action() /* close */, Ticket, item)`.
   - If the parent's row exists: `service.restart_remote_lifecycle(KIND, root, op, &target)`. Otherwise `service.reserving(root, op, || service.reserve_remote_operation(root, op, &target))`.
   - `Reserved(owner)` goes on.
   - `Busy` gives `Err(Busy)` and `PollYielding` gives `Err(PollYielding)`. Either way nothing is held: the row is unreserved, and the record stays at `branch_deleted` for the same request.
   - `Replay` means the row is completed. Reload the record: at `retired` go to Part F step 3 (section 5); otherwise `Err(RecoveryRequired)`.
3. **`service.remote_safe_point(root, &owner, BeforeTransport)`, then `service.remote_safe_point(root, &owner, AfterAdvertisement)`.** Nothing is fetched or pushed between them, as finish_steps' restart from a local step does (`run.rs:1941-1948`): "Nothing on the remote changes". `finish_parent` requires `AfterAdvertisement` (`reservation.rs:1224-1230`), so every release below, the abandon's included, comes after it.
   - `Continue`, `Cancelled` or `Interrupted` all go on. A `close` row acknowledged for a cancellation stays `reserved` and owned, with `completed_step` set (`reservation.rs:634-647`). The record's own `cancel_requested` is kept for the next boundary (2.3).
   - `Err` gives `Err(RecoveryRequired)` with the row reserved, for `restart_remote_lifecycle`.
4. **`ContextCleanup::acquire(root)`** (the lease), then `run::validate_cleanup_owner(service, root, Some(&owner), Some(&plan), Some(&frozen), primary)` (`run.rs:2139-2162`). A refusal is `Err(RecoveryRequired)` with the row reserved, as a direct cleanup's. This is the one stop that holds it, and only for a configuration changed inside this part.
5. **The re-check under the reservation** (Part F step 2). In one transaction, `sync::republish_evidence_in(tx, id, root, &plan, Ticket, item)?.0.deleted()` (section 7.1) must hold. The tracking context ref `plan.context(Ticket, item).tracking_ref()` must be `r.remote_context_oid` or absent. Absent is accepted: fetches never prune a tracking ref (Fact 16), so only this record's earlier attempt deleted it under the hold. Otherwise abandon under the reservation: `ExternalChange` / `ExternalChange`, releasing it (2.4).
6. **Local absence.** `cleanup.require_local_cleanup_absent(Ticket, item)` (`cleanup.rs:540-565`). `Changed` abandons under the reservation with `ExternalChange`. `RecoveryRequired` is `Err(RecoveryRequired)` with the row reserved.
7. **The tracking ref.** `evidence = cleanup.delete_tracking_ref(Some(&plan), Ticket, item, r.remote_context_oid, op, oid)` (`cleanup.rs:572-640`). `Changed` abandons under the reservation with `ExternalChange`. `RecoveryRequired` is `Err(RecoveryRequired)` with the row reserved. Then `#[cfg(test)] promotion::run_tests::tracking_deleted(service, root)` (the TRACKING seam, F10).
8. **The retirement's transaction.** `cleanup.require_local_cleanup_absent(..)` again, inside it (as `run.rs:2098-2101`), then `record::retire_new_branch(tx, service, id, op, &evidence, &owner)` (section 3). Then `#[cfg(test)] promotion::run_tests::completion_returned(..)` (the COMMIT seam, F10/F17) and `run::boundary(.., PromotionAfterCompletionCommit)`.
   - On an error: `record::reconcile_retirement(tx, id, op)`. `true` goes on; `false` is `Err(RecoveryRequired)` with the row reserved (as `run.rs:2108-2117`).
9. **After it.** Drop `evidence`; keep the lease (`cleanup.into_lease()`) for Part F step 3. The reservation is gone: `finish_parent` completed it in the transaction.

### 2.3 Stops, categories and re-checks

| Where | Cause | Answer | Record |
| --- | --- | --- | --- |
| 2.1 before `worktree_removing` | the full check refused: dirty, moved, locked, nested repository | `Ok(Branch)` with `ContextChanged` | abandoned; pending ref deleted; old context intact |
| `worktree_removing` | `WorktreeRemoval::Changed` / `::RecoveryRequired` | `Err(RecoveryRequired)` | unfinished; `cleanup = context_changed` / `recovery_required`; nothing reserved (F16) |
| `worktree_removed` | `BranchDeletion::Changed` | `Err(RecoveryRequired)` | unfinished; `cleanup = context_changed`; nothing reserved |
| any 2.1 boundary | cancellation | `Err(Interrupted)` | unfinished at its step (F13) |
| 2.1 | live configuration not the record's | `Err(RecoveryRequired)` | unfinished; nothing reserved |
| 2.2 step 2 | `Busy` / `PollYielding` | `Err(Busy)` / `Err(PollYielding)` | `branch_deleted`; nothing reserved |
| 2.2 steps 5, 6, 7 | the deletion no longer evidenced, the tracking tip moved, a local branch or worktree back, the tracking ref changed | `Ok(Branch)` with `ExternalChange` | abandoned from `branch_deleted` (ruling 18); pending ref deleted; reservation released `cancelled` (F8) |
| 2.2 | SQL or Git error, owner lost | `Err(RecoveryRequired)` | `branch_deleted`; row reserved; the retry restarts it |

- A stop saves `r.cleanup` through `save_record`, which `validate_branch` allows only at a `new_branch`'s `worktree_removing`, `worktree_removed` and `branch_deleted` (`record.rs:703-705`).
- The next successful step clears it (`cleanup = None` in the save). The abandon writer and the retirement writer clear it too (carried note): `abandon_branch_record` already sets `cleanup_category=NULL` (`record.rs:1194`), and `retire_new_branch` writes it so.
- **Never superseded** (F16): `supersedable` is false for a branch record (`record.rs:737-741`), so a new request's prepare is held (decision 33) and its confirm is `busy` (`record.rs:805-807`).
- **Cancellation under the reservation.** Steps 2 to 8 of 2.2 have no cancellation check. A `request cancel` arriving then is acknowledged by the safe point without releasing the row. The part runs on, bounded and with no transport, to its release. The record's `cancel_requested` stays set (`retire_new_branch` does not clear it), so Part F step 3's first check (5.1) stops at `retired`, holding nothing (F13's last row). This is how "a reservation held across … any stop" (R-31) is avoided.

### 2.4 `new_branch::abandon(service, root, r, refusal, answer, owner: Option<&RemoteReservation>)`

1. **Git first.** The pending ref, when it is at `r.closure_oid`, is deleted. Absent is fine; elsewhere is `Err(RecoveryRequired)`. An abandoned record holds no ref: the CHECK `pending_ref IS NULL OR … step NOT IN ('complete','abandoned','superseded')` (`record.rs:415`).
2. **One transaction.**
   - Without an owner: `record::abandon_branch_record(tx, id, op, &refusal, answer, None, given_at)`.
   - With one: the new `record::abandon_reserved_new_branch(tx, service, id, op, &refusal, answer, &owner)`, which calls `abandon_branch_record` and then `reservation::finish_parent(KIND, service, tx, id, &owner)`. That ends the row `cancelled` (section 3).
3. `Ok(BranchStep::Finished)`. The runner reloads and answers from `answer_code` (`run.rs:983-996`).

`abandon_branch_record` (`record.rs:1173-1199`) changes:
- The allowed steps become per action. `NewBranch`: `Confirmed | BranchDeleted | Retired`, as `branch_permits` has them (`record.rs:121`). The others keep `Confirmed | ContextSynchronized | Closing`.
- Its UPDATE gains `pending_ref=NULL`. That is a no-op for the other actions, whose rows never hold one (`record.rs:415`).
- No link is given for a new branch's abandon (`link: None`).
- No Task 9 path abandons from `retired`: step 3's refusals are `recovery_required` (Part F step 3). The permit stays, as Task 7 built it.

## 3. U-1: the reservation and the transition that releases it

**Taken only for the remote part:** section 2.2, after `branch_deleted` is stored. There is no reservation for situations 3.2 to 3.4, nor for any local step.

**The transition: `finish_parent`, generalized** (`remote/reservation.rs:1217-1249`):
- The checks at `1224-1230` are unchanged: owned, action `close`, manual priority, `completed_step == AfterAdvertisement`.
- The `TicketClosure` arm of the `complete` query (`1236`) becomes a query of the row's end:
  ```sql
  SELECT CASE
    WHEN EXISTS(<today's query at 1236, verbatim>) THEN 'completed'
    WHEN EXISTS(SELECT 1 FROM ticket_closures c JOIN retired_contexts r
                  ON r.repository_id=c.repository_id AND r.item_id=c.item_id AND r.kind='ticket'
                 AND r.operation_id=c.operation_id AND r.retired_oid=c.retired_oid
                 WHERE c.repository_id=?1 AND c.operation_id=?2
                   AND c.action='new_branch' AND c.step='retired') THEN 'completed'
    WHEN EXISTS(SELECT 1 FROM ticket_closures c
                 WHERE c.repository_id=?1 AND c.operation_id=?2 AND c.action='new_branch'
                   AND c.step='abandoned' AND c.retired_oid IS NOT NULL AND c.pending_ref IS NULL) THEN 'cancelled'
  END
  ```
  - `DocumentPromotion`'s arm (`1235`) gives `'completed'` exactly when it does today.
  - No end (`NULL`) is `recovery_required`, as `!complete` is today (`1239-1241`).
- The UPDATE at `1242` writes `phase=?3, outcome=?3` from the end instead of the literal `'completed'`. The rest of it is unchanged.
- `cancelled` is the end `supersede` gives a parent row it ends without completion (`promotion/lifecycle_record.rs:442`).
- The doc comment says the row ends with its record's end: complete and retired, a new branch's retirement, or a new branch abandoned under it.

**Why this is a generalization and not a new transition.**
- The function already is "the parent's own reservation ends in the caller's transaction, given that the parent's table records its end".
- The direct callers are unchanged: `parent::complete_record` (`lifecycle_record.rs:1037-1039`) reaches the first `WHEN`, which is today's query verbatim.
- A completed row replays on a restart (`reservation.rs:1327-1328`), which U-1 notes. The new branch never restarts its row after `retired` (F17).
- The final completion's `reconcile_completion` (`lifecycle_record.rs:1043-1068`) reads the parent's own row as `Completed`. For a new branch that retired, it is.

**The retirement's transaction: `record::retire_new_branch(tx, service, id, op, evidence: &RetirementEvidence<'_>, owner: &RemoteReservation)`**, new in `closure/record.rs`. It is modelled on `parent::complete_record` (`lifecycle_record.rs:974-1041`), whose end it does not take.

1. **Checks:**
   - the record is a `new_branch` at `branch_deleted`, with `retired_oid` set;
   - `evidence.matches(Ticket, op, item, retired_oid)` (`cleanup.rs:124-136`);
   - `remote_polling_state.remote_name` is the record's (as `999-1004`);
   - the parent's row exists, and `reservation::owned` and `validate_remote_identity` pass (as `1005-1014`).
2. `cleanup::retire_context(tx, id, evidence)` (`cleanup.rs:953-1022`). This also deletes the link (R-14, `1018-1020`).
3. `recovery::invalidate_index_observations(tx, id)`, then `mark_registered_lifecycle_contexts_dirty(tx, id, root, &lifecycle_contexts(item))`, as `1016-1035`.
4. `UPDATE ticket_closures SET step='retired', cleanup_category=NULL, updated_at=max(updated_at,?2) WHERE operation_id=?1`. This is `BranchDeleted → Retired` (`record.rs:119`), and `cancel_requested` is kept (2.3).
5. `reservation::finish_parent(KIND, service, tx, id, owner)` (the second `WHEN`).
6. The re-read validates it.

**`record::reconcile_retirement(tx, id, op) -> Result<bool, RepositoryError>`**, after an uncertain commit.
- `true`: the record is a `new_branch` at `retired` or later, unfinished, and the parent's row is `Completed`.
- `false`: at `branch_deleted`.
- Otherwise `recovery_required`. As `reconcile_completion`, absence in Git is not evidence.

## 4. `retire_context`'s evidence, with and without a context

- **Where it comes from.** The evidence is the `RetirementEvidence` that `delete_tracking_ref` returns. It is the only constructor (`cleanup.rs:106-121, 572-640`): `delete_tracking_ref(Some(&plan), Ticket, item, expected = r.remote_context_oid, op, oid = r.retired_oid)`. `retire_context` then writes `retired_contexts(…, operation_id=op, retired_oid=oid)` (`1013`). It also deletes `remote_context_states`, rewrites the current observation batch without the item (`986-1011`), and deletes the link.
- **With a context (situation 2).** `retired_oid` is `T`, the old context's tip, and the consent's head. `expected` is the stale tracking ref's tip the digest bound, or none. The deletion is "evidenced, by a fetch or a push", as `republish_evidence` reads it (`remote/sync.rs:8316-8431`, `deleted()` at `8286-8291`).
- **Without a context (situation 3.1).** `retired_oid` is chosen at prepare (section 7.2), from the history read in the observation's registry transaction:
  1. the tracking context ref's tip, when there is one (F4's stale ref);
  2. else the newest proven push: the second value `push_absence` returns (`sync.rs:8103-8247`), as `RepublishEvidence::last_published` holds it for `Pushed` (`8406-8409`);
  3. else the deletion record's `last_published` (`Fetched`, `8383-8393`); with none at all, `HistoryUnresolved` (open question 9).
- **`history_unresolved` for an earlier push that is not its ancestor.** In 3.1, every proven push's published OID must be `retired_oid` or an ancestor of it (`graph_descendant_of(retired, published)`). Otherwise the prepare is blocked with `HistoryUnresolved`.
  - The reason: after the retirement, `push_absence` skips a proven push only when it is the retired tip or its ancestor (`8207-8214`).
  - Any other push stays verified, so the boundary reads `Deleted`, and the publication child would be refused as a deleted branch (answer row 3) instead of publishing fresh.
- **The proven pushes.** `push_absence` returns them as a third value: every publication it counts `verified` (`8216-8222`), in row order. Its two callers take `.0` (`7491-7500`) and `(boundary, newest, _)` (`8338`).
- **Not in situation 2.** The design states the ancestor check only for 3.1 (open question 1).

## 5. Step 3: the branch, and the new context-at-commit function

### 5.1 Step 3 in `new_branch::step`

It runs at `confirmed` with `closure_oid` and no `retired_oid` (situations 3.2 to 3.4), at `retired`, and, reconciling, at `closing`. It runs under the lease: step 2's, or a new `repository_lease` when step 3 starts the call.

1. **Not yet `closing`.** The cancellation check gives `Err(Interrupted)`.
2. **Not yet `closing`.** The pending ref is ensured as in section 1, then `record::begin_new_branch_closing(tx, id, op)` (new) stores step `closing` and `context_oid = closure_oid`.
   - It requires a `new_branch` with `closure_oid` and `pending_ref`, and either `step == Retired` or `(step == Confirmed && retired_oid.is_none())` (`branch_permits` `Confirmed → Closing`, `Retired → Closing`; `record.rs:116, 120`).
   - `validate_branch` requires `context_oid` and `base_oid` from `closing` (`record.rs:674-675`).
   - From here the answer table reads "this record's closure" at `closure_oid` for a `record` closure and at `context_oid` (also `C`) for a kept one (`checkpoint.rs:1426-1430`). It is the same commit.
3. **`checkpoint::context_moving`**, which marks the ticket's context before Git is touched.
4. **The branch and its worktree:** `service.context_at_commit(root, AuthoringKind::Ticket, item, closure_oid)` (5.2).
   - The branch: absent, create it; at `C`, keep it; anything else, `RecoveryRequired` (Part F step 3).
   - The worktree: add it if missing.
5. **The pending ref.** Delete it if it is at `C`. Absent is fine; elsewhere is `RecoveryRequired`. Then `record::clear_pending_ref(tx, id, op)` (new): a `new_branch` at `closing` writes `pending_ref=NULL`, and one that has none is left as it is.
6. `checkpoint::context_moving` again, then `Ok(BranchStep::Closing)`.
   - The index is refreshed outside the lease by the runner's exit (`finish_branch` → `refresh`, `run.rs:1104`; `refresh_with_publication` for `[Item]`).
   - `finish_branch` mints the publication child. `record::begin_publication` (`record.rs:1085-1097`) gains `|| r.action == NewBranch && r.pending_ref.is_some()` among its refusals: no child mints before step 3 is done.

**At `closing` on a retry**, the same 4 to 6 run. Each is idempotent: the branch at `C` is kept, an existing worktree is checked as 5.2 says, an absent ref and a cleared column are left. A branch moved past `C` before the mint is `RecoveryRequired`. Once `primary_attempt > 0` the runner never calls the hook (`run.rs:931-933`).

### 5.2 `RepositoryService::context_at_commit` (new, private, in `src/repository.rs` beside `prepare_context_unlocked` at `2455`)

```rust
/// A ticket's editing context made at `commit`: its branch
/// `manyhands/<kind>/<ID>` at `commit` and its worktree at
/// `.manyhands/worktrees/<ID>` (Part F step 3). Unlike
/// `prepare_context_unlocked` it does not require the root to be on the
/// primary, nor the item to be on the local primary: the commit, built on
/// the tracking primary, holds it. The caller holds the repository lease
/// and the item's closure hold, so no other writer makes the context.
fn context_at_commit(&self, root: &Path, kind: AuthoringKind, item_id: &canonical::ItemId,
    commit: git2::Oid) -> Result<ItemContext, RepositoryError>
```

**Preconditions,** each an `Err` with nothing changed. The operation is `RepositoryOperation::PrepareContext` and the kinds are `prepare_context_unlocked`'s, so the binding maps them as it does today: `MismatchedAuthoringContext` gives `recovery_required` (`mutation/outcome.rs:63-65`).
- `canonical_repository_root(root, ..)`. The configuration is valid (`read_configuration_for`), and the branch name is not the primary's (as `2510-2517`).
- `commit` exists (`find_commit`).
- `validate_deterministic_worktree_path_chain(&worktree, &root, ..)` (`11593`) and `ensure_authoring_worktree_base(&root, ..)` (`11069`).
- The branch is not checked out in another worktree (`branch_checked_out_in_another_worktree`, `11621`), as `2542-2555`.
- **The branch.** Absent: `check_failure(BeforeContextBranchCreation)`, then `repository.branch(&branch, &commit, false)`. At `commit`: kept. Anywhere else: `MismatchedAuthoringContext` ("the deterministic authoring branch exists at another commit").
- **The worktree.** Directory and registration both absent: `check_failure(BeforeWorktreeCreation)`, then `repository.worktree(&id, &worktree, Some(WorktreeAddOptions::new().reference(Some(&reference))))`, as `2602-2626`. Both present: `validate_context_worktree_identity` (`11522`), and its HEAD must be `commit`. Exactly one present, or a HEAD elsewhere: `MismatchedAuthoringContext`. There is no `canonical_context_at` walk under the lease: `commit`'s tree is `P`'s with the ticket edited.

**Not done**, unlike `prepare_context_unlocked`:
- `require_index_available` (`2462`);
- the root's checked-out branch (`2484-2491`) and `ensure_configuration_path_clean` (`2493`);
- `canonical_context_at(&root)` and `primary_has_target` (`2525-2538`, `2568-2575`);
- the Create intent's occupancy check;
- the Edit intent's "branch at the primary's head" rule (`2575-2588`);
- any lifecycle row's step (`advance_lifecycle`).

## 6. `unheld_entries` (`context_retirement/proof.rs`)

```rust
/// One path the branch holds that the primary does not (Part F, R-19).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct UnheldEntry {
    pub(in super::super) path: Vec<u8>,
    pub(in super::super) change: PathChange, // crate::repository::PathChange, promotion/mod.rs:239
}
/// Each path whose entry in `local` differs from its entry in the one merge
/// base of `local` and `primary`, and also from its entry in `primary`, with
/// its change against `primary`; compared per path as `preservation`
/// compares, without rename detection, unbounded, sorted by path.
pub(in super::super) fn unheld_entries(repository: &git2::Repository, local: git2::Oid,
    primary: git2::Oid) -> Result<Vec<UnheldEntry>, ProofRefusal>
```

Rules, in order:
1. `repository.is_shallow()` gives `ShallowHistory`.
2. `local == primary`, or `primary` descends from `local`, gives `Ok(vec![])`. The one merge base is `local` then, and every entry is the base's.
3. `merge_bases(local, primary)`: `NotFound` gives `UnrelatedHistory`, another error `MissingObject` (as `proof.rs:45-51`). `[]` gives `UnrelatedHistory` and more than one `MultipleMergeBases` (`52-56`).
4. Read `entries(repository, x, false)` (`proof.rs:103-142`) for `local`, `primary` and the base. Objects are not verified, because only entries are compared. Empty trees are leaves, as `preservation` reads them.
5. For each path in the sorted union of the base's and `local`'s keys (a `BTreeSet<&Vec<u8>>`), with `l = local.get`, `b = base.get`, `p = primary.get`:
   - skip when `b == l` or `p == l`;
   - otherwise `(Some, Some)` is `Modified`, `(Some, None)` is `Added`, and `(None, Some)` is `Deleted`.
   - A rename on the branch is its old path `Deleted` and its new path `Added`.
   - There is no exception for the ticket's own file: the function takes no kind or ID.

`preservation` (`proof.rs:24-82`) is unchanged.

## 7. Prepare: situations 2 and 3 made real, the digest, the preview

### 7.1 The history read

- **`remote/sync.rs`.** `republish_evidence`'s transaction body (`8336-8412`) moves into `pub(crate) fn republish_evidence_in(tx: &rusqlite::Connection, id: i64, root: &Path, plan: &RemoteRefPlan, kind: AuthoringKind, item_id: &ItemId) -> Result<(RepublishEvidence, Vec<git2::Oid>), RepositoryError>`.
  - The second value is `push_absence`'s proven list (section 4).
  - `republish_evidence` keeps its signature and returns `.0`. `RepublishEvidence` is unchanged.
- **`closure/observe.rs`.**
  - `PendingWork` (`349-361`) gains `history: Option<NewBranchHistory>`, with `struct NewBranchHistory { evidence: sync::RepublishEvidence, proven: Vec<Oid> }`.
  - It is filled in the same read-only registry transaction as `pending_work`, at `464-466` and `527-530`. The read runs only for a pull-request close (`mode == PullRequest && verb == Close`) with a publication remote. It calls `sync::republish_evidence_in` with the plan from the observation's own `config`, which is not read again.
  - A direct prepare and a withdraw read nothing new.

### 7.2 `branch_observation` (`observe.rs:1390-1523`)

`Gathered` (`1368-1376`) gains two fields, filled in `observe_git` where the presence is read (`830-856`):
- `context_absent`: `ContextPresence::Missing` and no local branch `refs/heads/manyhands/ticket/<ID>`;
- `worktree_occupied`: `ContextPresence::Missing`, and either `.manyhands/worktrees/<ID>` exists (`symlink_metadata`) or a worktree registration of that name does.

Order:
1. **Situation 1 and the foreign synchronization's `busy`:** unchanged (`1420-1437`; rulings 28 and 29).
2. **The situation.** This replaces `1439-1446`, "until Task 9". `deleted` is `history.evidence.deleted()`.
   - **2:** a context with its tip `T` (`context_oid` with `!context_missing`) and `deleted`. A new branch; `old_tip = Some(T)`, `retired_oid = Some(T)`.
   - **3:** `context_absent`, then the first that applies:
     - **3.1** `deleted`: a new branch with `old_tip = None` and `retired_oid` as section 4 chooses it. A proven push that is neither `retired_oid` nor its ancestor, or no `retired_oid`, inserts `HistoryUnresolved`.
     - **3.2** the tracking context tip `S` is `P` or an ancestor of it: a new branch with no retirement.
     - **3.3** a tracking ref otherwise: `RemoteContextUnmaterialized`, plus `PrimaryAheadOfRemote` when `S` is an ancestor of the local primary (Task 7 checkpoint §10.1; `way_out.rs:296-336` routes them to `PollOnce` then `SharePrimary`).
     - **3.4** no tracking ref: a new branch with no retirement.
   - **Otherwise** (a context whose deletion is not evidenced, or a local branch with no worktree): situations 4 and 5 as today, and today's `ContextMissing` for `context_missing` (open question 7).
   - **The history blocker:** when there is no tracking context, `remote_history_blocker` (`1248-1255`) is extended into the blockers as today. Its `RemoteContextDeleted` is dropped when `deleted` chose situation 2 or 3.1. Its `HistoryUnknown` and `HistoryUnresolved` stay ("as the direct close reports them").
3. **For a new branch:**
   - **F7.** `P`'s ticket (`tree_ticket(repository, commit_tree(P), path, root)`) must be there, as a canonical regular entry. Missing: `RemoteContextDeleted` (situation 2) or `ContextMissing` (situation 3). Not canonical: `TicketInvalidAfterClosure`.
   - **The effect.** Situation 2: `T`'s ticket (`context_ticket_at`). `Open` is `Record`, `Closed` is `Existing` (the kept closure, R-21), and `Malformed` is `TicketInvalidAfterClosure`. Situation 3: `Record`.
   - **The closure check on `P`'s bytes.** `Record`: `closed_ticket_bytes(P_bytes, at, closed_by)`. Kept: the kept fields parsed from `T`'s canonical ticket, then `closure_replaced_bytes(P_bytes, kept_at, &kept_by)`. Either refusal is `TicketInvalidAfterClosure`.
   - **The identity** is selected as `needed` for both kinds, `select_identity(.., true)`: the commit's author. For a kept closure, `IdentityRequired` is inserted when nothing else blocks, as `withdraw_observation` does (`observe.rs:1640-1655`).
   - **The unheld entries** (situation 2 only): `proof::unheld_entries(repository, T, P)`. Any `ProofRefusal` is `Err(state::recovery_required())` ("no merge base or more than one is `recovery_required`").
   - **Occupied** (situation 3): with no blocker, `worktree_occupied` is `Err(RepositoryError::new(.., RepositoryErrorKind::OccupiedItemPath, ..))`. The binding answers `occupied_path` (`mutation/outcome.rs:69`), F15.
   - **With an unfinished record that holds new requests** (`pending.record_unfinished`), neither error is raised, so the binding's `held` answers (decision 33; ruling 35's rule applied to these two errors). The occupied check is skipped, and a `ProofRefusal` gives an empty list.
   - The entries are always computed otherwise. This matters at step 1: its re-observation runs while its own record is unfinished, which `pending_work` counts (`observe.rs:610`), and it must bind the same entries the consent did. An empty placeholder there would make every step 1 an `external_change`.
   - **The observation:**
     - `path: (PullRequest, NewBranch)`, `base: P`;
     - `context_oid: T.unwrap_or(P)`, documented: for a new branch, `new_branch.old_tip` says whether there is a context;
     - `needs_context_child: false`, `changes: []`;
     - `cleanup: PlannedCleanup { worktree: old_tip.is_some(), local_branch: old_tip.is_some(), remote_branch: false }`;
     - `integration: MergeCommit` (unused, as today);
     - `new_branch: Some(NewBranchFacts { old_tip, retired_oid, unheld })`.
4. **Situations 4 and 5** are as today, with `new_branch: None`.

**`ClosureObservation`** (`observe.rs:240-274`) gains `new_branch: Option<NewBranchFacts>`. Its three constructions (`1184`, `1497`, `1679`) set it. `NewBranchFacts` is defined in `closure/mod.rs`: `#[derive(Clone, Debug, PartialEq, Eq)] pub(in super::super) struct NewBranchFacts { pub(in super::super) old_tip: Option<Oid>, pub(in super::super) retired_oid: Option<Oid>, pub(in super::super) unheld: Vec<UnheldEntry> }`. A new method gives the head the consent binds:

```rust
pub(in super::super) fn consented_head(&self) -> Option<Oid> {
    match &self.new_branch { Some(n) => n.old_tip, None => Some(self.context_oid) }
}
```

`begin_closure_record` (`closure/lifecycle.rs:223-235`) passes `observation.consented_head()`. For a `NewBranch` path it calls the new `record::begin_new_branch_record(tx, id, op, item, expectation, remote, kind, consented, retired: observation.new_branch.retired_oid)`. That is `begin_branch_record`'s body (`record.rs:791-821`), with `insert_record` (`885-911`) gaining `retired: Option<Oid>` for its INSERT's `retired_oid`. Its two other callers pass `None`, so `begin_branch_record`'s signature is unchanged.

**`ready`** (`closure/lifecycle.rs:434-444`) gains `Err(blockers) if blockers.contains(&ClosureBlocker::RemoteContextUnmaterialized) => Err(Refusal::ExternalChange)`. The preflight-only value is never encoded: `Refusal::encoded` would panic on it (`promotion/lifecycle_record.rs:122-130`). A confirm whose situation changed is `external_change` (Part D confirm step 1; Obligation 10).

### 7.3 The digest's new fields

`branch_expectation` (`observe.rs:1765-1767`) becomes `branch_expectation_with_head(o, o.consented_head())`. In `branch_expectation_with_head` (`1775-1801`), after `closure_effect` and only when `o.new_branch` is `Some(n)`:
- `hash.update(o.base.as_bytes())`: the tracking primary `P`;
- `n.retired_oid` framed as the head is (`[0]`, or `[1]` and 20 bytes): what step 2 retires;
- `o.tracking_context` framed the same: a remote tracking ref's tip;
- `(n.unheld.len() as u64).to_be_bytes()`, then for each entry `digest::field(path)` and a change byte (`Added` 0, `Modified` 1, `Deleted` 2).

The old context's tip `T` is the head (`[1]‖T`, or `[0]` with no context), under the action tag 2 (`1784-1788`). The kept closure is bound by `closure_effect`'s `existing` arm with both fields (`1746-1761`). A record closure's `closed_at` is not bound (C6). The close and withdraw digests are byte-identical to today's, because `new_branch` is `None` and `consented_head()` is `Some(context_oid)`.

### 7.4 The preview

- `TicketClosurePreview` (`closure/mod.rs:237-254`) gains `new_branch: Option<NewBranchFacts>`, set in `ClosureObservation::preview` (`observe.rs:276-301`), with `pub(in super::super) fn new_branch(&self) -> Option<&NewBranchFacts>`. It names `P` (`tracking_primary()`), `T` (`old_tip`), the retired OID and the unheld entries.
- Its `Debug` (`310`) adds whether it is a new branch and the number of entries.
- Task 10 maps it to the DTO. Until then the binding renders it through `ready_of_closure`, direct-shaped (carried note).

## 8. The retry from each step

| The record as it stands | What the same request does |
| --- | --- |
| no record | a new confirm: the digest is compared again |
| `confirmed`, no `closure_oid` (a crash at `AfterConfirmed`, or `busy`/`interrupted` before step 1's write) | step 1 from the start: observe, compare, build with the clock (nothing was stored) |
| `confirmed`, `closure_oid`, ref absent (`NewBranchCommitRecorded`) | the ref is made at `closure_oid`, with no rebuild: the same `closed_at` and one `C`; then step 2 or 3 |
| `confirmed`, `closure_oid`, ref at `C` (`NewBranchCommitHeld`; an abandon whose transaction failed) | step 2 (with `retired_oid`) or step 3 |
| `worktree_removing`, stopped or crashed | `remove_context_worktree` (no `fresh`); the category is cleared on success |
| `worktree_removed` | branch deletion |
| `branch_deleted`, no parent row (a crash at `BeforeCompletion`, a `busy` reservation) | `reserving` |
| `branch_deleted`, parent row reserved (a crash under it; TRACKING seam) | `restart_remote_lifecycle`, the re-checks with an absent tracking ref accepted, `delete_tracking_ref` with nothing to delete, the transaction |
| `retired` (COMMIT seam; a cancellation under the reservation) | Part F step 3, with no reservation (F17) |
| `closing` (`BeforeContextBranchCreation`, `BeforeWorktreeCreation`, or a crash before the mint) | 5.1's reconciliation, then the mint |
| `publication_pending` | `finish_branch`, `child()` (D8, D13) |
| `complete` / `abandoned` | the answer from `answer_code` (D19) |

The hold covers every row from `NewBranchCommitRecorded` on, until `complete` or `abandoned`.

## 9. The answers, F1 to F17

The one new `BranchAnswer` is `ContextChanged`, stored `context_changed`. It is an abandon's (the full check refused), not `after_child`'s (`record.rs:1123-1140`). `name` and `parse` gain arms (`closure/mod.rs:128-177`). `branch_code` (`mutation/bind/lifecycle_verb.rs:505-531`) gains `ContextChanged => ResultCode::RecoveryRequired`, the code a direct close's `context_changed` cleanup stop answers (`mutation/bind/lifecycle.rs:322-331`; open question 4). `closure_blocker` (`lifecycle_verb.rs:196-221`) gains `RemoteContextUnmaterialized => LifecycleBlocker::RemoteContextUnmaterialized` (Task 5's value, `mutation/dto.rs:390`).

| F | Ends at | Domain answer | Binding code |
| --- | --- | --- | --- |
| F1, F2, F3, F4, F6 | answer row 7 (the head holds this closure) | `Branch(Published)` | `ok` |
| F9 | answer row 6 (closure kind `existing`) | `Branch(Existing)` | `already_closing` |
| rows 1 to 5 after the child | Part D's table, unchanged (`checkpoint.rs:1420-1493`) | `Merged`, `ClosedElsewhere`, `RemoteBranchDeleted`, `TicketClosed`, `RemoteChanged` | as Task 7 |
| F5 | prepare | `Blocked([RemoteContextUnmaterialized])`, or with `PrimaryAheadOfRemote` | the blocked code; way out `PollOnce` (then `SharePrimary`) |
| F5, confirm | refused at insertion | `Err(ExternalChange)` (`ready`) | `external_change` |
| F7 | prepare and confirm | `Blocked([RemoteContextDeleted])` / `Blocked([ContextMissing])` | as today |
| F8, confirm (`P`, `T`, tracking tip) | refused at insertion | `Err(ExternalChange)` | `external_change` |
| F8, confirm (dirty worktree) | refused at insertion | `Err(Blocked([ContextDirty]))` (open question 8) | as today |
| F8, before step 1's write | abandoned | `Branch(ExternalChange)` / `Branch(ReconfirmationRequired)` | `external_change` |
| F8, the full check | abandoned | `Branch(ContextChanged)` | `recovery_required` |
| F8, under the reservation | abandoned, released `cancelled` | `Branch(ExternalChange)` | `external_change` |
| F10, F17 | as F1 after the retry | `Branch(Published)` | `ok` |
| F11 | prepare: `recovery_required` naming the holder (binding `held`); confirm: `Err(Busy)` | | `recovery_required` / `busy` |
| F12 | `publication_pending` | `Err(ContextSynchronization(stop))` | as `item sync`'s stop |
| F13 | the stored step | `Err(Interrupted)` | `cancelled` |
| F14 | — | writers' `Err(RecoveryRequired)` naming the record (`hold_refusal`, `record.rs:1411-1414`) | `recovery_required` |
| F15 | prepare | `Err(Repository(OccupiedItemPath))` | `occupied_path` |
| F16 | `worktree_removing` / `worktree_removed`, with its category | `Err(RecoveryRequired)` | `recovery_required` |

## 10. The test plan

Every new check is run red once before its code, and the red run is recorded. Run tests are in `repository::closure::run_tests::pull_request` (`closure/run_pull_request_tests.rs`), each by exact name, and each ends with `p.assert_primary_untouched(&io)` for every io it ran (`run_pull_request_tests.rs:412-416`).

### 10.1 New helpers (pull_request module; no existing helper changes meaning)

- `PullRequest::deleted_on_remote(&self)` inserts a `remote_context_states` row with `state='remotely_deleted'`, `publication_evidence='observed_published'` and `last_advertised_oid` at the tracking context's tip, as `observe_tests.rs:151-161` does.
- `remove_context(&self)` prunes the worktree and deletes the local branch (as `observe_tests.rs:706-719`). `drop_tracking_context(&self)` deletes `tracking_context_ref()`.
- `merged_on_tracking(&self) -> Oid`: `squash_merge(self, T)` (`run_pull_request_tests.rs:3024-3031`), with `refs/remotes/origin/main` moved to it. The local primary does not move.
- `pending_ref(&self, op) -> Option<Oid>`; `parent_row(&self, op) -> Option<(String, i64)>` (phase, owner epoch); `retired_row(&self) -> Option<(OperationId, Oid)>`.
- `assert_new_branch(&self, r, p_oid)`:
  - the context ref is `r.closure_oid`, whose only parent is `P`;
  - the diff of `P`'s tree and `C`'s tree is exactly one delta, at `ticket()`;
  - the worktree's HEAD is `C`; no pending ref; `!held()`.
- `assert_nothing_deleted_remotely(io)`: `io.inner.calls.borrow().deletes == 0`.
- `NEW_BRANCH_EXITS: [FailurePoint; 4] = [NewBranchCommitRecorded, NewBranchCommitHeld, BeforeContextBranchCreation, BeforeWorktreeCreation]` and `NEW_BRANCH_EXIT_BASE = 100`, in `closure/run_tests.rs`. `closure_exit_child` (`1109-1162`) gains the arm `point >= NEW_BRANCH_EXIT_BASE => open_at_with_exit_point_for_testing(.., NEW_BRANCH_EXITS[point - BASE])` before its seams arm. `EXITS`, `F10_SEAM` and `F11_SEAM` are unchanged.

### 10.2 Tests, by slice

**Slice A** (observation and consent; nothing runs past insertion):

| Test | Setup | Asserts | Red by |
| --- | --- | --- | --- |
| `unheld_entries_against_the_primary` (`repository::context_retirement::tests`, after `retirement_proof_history_refusals` at 306) | `commit(&repo, &files, &parents)` (`tests.rs:20-54`). Base `B = {ticket, code}`. Rows: held exactly (`T = B+code'`, `P` its squash on `B`); a squash after which `P` changed `code` again; `T` adds `new.txt`; `T` deletes `code` and `P` keeps it; `T` renames `code` to `moved`; `T` equal to `P`, and `P` a descendant of `T`; unrelated roots; a criss-cross (`T = M(A1,A2)`, `P = M(A2,A1)`) | `[]`; `[code Modified]`; `[new.txt Added]`; `[code Deleted]`; `[code Deleted, moved Added]` in path order; `[]`, `[]`; `Err(UnrelatedHistory)`; `Err(MultipleMergeBases)` | a stub returning `Ok(vec![])` |
| E-44's rows in `pull_request_prepare_decides_the_situation_and_its_blockers` (`observe_tests.rs:1797-1809`) | as 12.1 says | as 12.1 says | today's situation-2 and situation-3 code |
| `new_branch_digest_wire_format` (`closure::observe::tests`, beside `pull_request_digest_wire_format`) | a `ClosureObservation` literal, path `(PullRequest, NewBranch)`, `Record`, `new_branch: Some(..)` with one entry | `expectation(&o) == blake3(domain ‖ 2 ‖ item ‖ branch ‖ main ‖ remote ‖ [1]T ‖ 0 ‖ closed_by ‖ P ‖ retired ‖ tracking ‖ 1 ‖ path ‖ change)`; a change to the entry's change, `P`, the retired OID or the tracking tip changes it | the head and fields not bound |
| `f5_a_remote_branch_neither_primary_holds_is_unmaterialized` | `remove_context`; the tracking context at a commit off `base` that neither primary holds; a second row with that commit on the local primary (`primary_before` reset) | prepare `Blocked([RemoteContextUnmaterialized])`, then `[PrimaryAheadOfRemote, RemoteContextUnmaterialized]`; a confirm with the digest of a ready preview taken before the ref moved is `Err(ExternalChange)`, the refused row `(PullRequest, NewBranch)`, nothing written | today's `ContextMissing` |
| `f7_a_tracking_primary_without_the_ticket_keeps_today_s_blocker` | `P` a commit removing `ticket()`; rows: an old context with `deleted_on_remote`; `remove_context` | `Blocked([RemoteContextDeleted])`; `Blocked([ContextMissing])` | exempt for the second row (today's answer); the first row red: today it is a ready `close` |
| `f8_a_change_before_the_retirement_is_external_change_or_abandons` (slice A rows) | F1's setup; between preview and confirm, rows: the tracking primary moved; a commit on the context; the tracking context ref moved; a dirty worktree | `Err(ExternalChange)` with the refused row `(PullRequest, NewBranch)`, no pending ref and no `close_subject()` commit; the dirty row `Err(Blocked([ContextDirty]))` with the refused row `(PullRequest, Close)` (a blocked revalidation has no observation, so `refuse_record` takes the configuration's path, `closure/lifecycle.rs:318`) | today there is no ready new-branch preview to confirm (the tracking ref hides the deletion and the close is offered instead); the `P` and tracking rows are red again against a digest without those fields |
| `f15_an_occupied_worktree_path_is_occupied_path_at_prepare` | `remove_context`, `drop_tracking_context`; a plain directory at `.manyhands/worktrees/<ID>` | `preview_ticket_closure` is `Err(e)` with `e.kind == OccupiedItemPath`; `f.service.prepare(&Mutation::TicketClose(..))` answers `occupied_path`; removing the directory makes it ready | today's `ContextMissing` |

**Slice B** (step 1 and step 3 with no retirement; the runner's dispatch):

| Test | Setup | Asserts |
| --- | --- | --- |
| `f3_a_ticket_with_no_context_never_published_gets_a_new_branch` | `remove_context`, `drop_tracking_context` | preview `new_branch` with `old_tip == None`, `retired_oid == None`, no entries; `Published`; `assert_new_branch`; `assert_nothing_deleted_remotely`; no parent row and no `retired_contexts` row (no retirement); `closed_at == clock()`; the tracking context at `C` |
| `f6_a_surviving_branch_the_tracking_primary_holds_is_fast_forwarded` | `remove_context`; the tracking context left at `base`, which `P` descends from | no retirement; `Published`; `C` descends from the old tracking tip; the tracking context at `C` |
| `f8_…` (slice B row) | F3's setup; a hook at `AfterConfirmed` moves the tracking primary | `Branch(ExternalChange)`, abandoned; `closure_oid` none; no pending ref |
| `f10_a_crash_after_the_commit_is_recorded_reuses_it` | F3's setup; `exit_at` at `NEW_BRANCH_EXIT_BASE + 0`; the stored `closed_at` read after the exit | the record held `closure_oid` with no ref; the retry: `Published`, one `close_subject()` commit in the ODB, the same `closed_at` and `closure_oid` |
| `f10_a_crash_after_the_commit_is_held_continues` | `+1` | as above, with the ref present before the retry |
| `f10_a_crash_before_the_branch_is_made_continues_at_closing` | `+2` | the record at `closing` with no branch; the retry makes it at `C`; one commit |
| `f10_a_crash_after_the_branch_before_the_worktree_adds_it` | `+3` | the branch at `C`, no worktree; the retry adds it; `assert_new_branch` |
| `f11_a_new_request_while_unfinished_is_held` | a record stopped at `closing` (`p.f.failing(BeforeWorktreeCreation)`) | `holder()` names it; the binding's prepare answers `recovery_required` with the holder's recovery action (as `d14_…`); a new confirm `Err(Busy)`; nothing written |
| `f12_a_stopped_publication_child_leaves_the_branch_local_and_closing` | F3's setup; `stops = [(Publication, transport, true)]` | `Err(ContextSynchronization(_))`; `publication_pending`; the branch at `C`; `listed_state() == Closing`; `held()`; the same request gives `Published` |
| `f13_a_cancelled_new_branch_is_continued_by_the_same_request` (slice B row) | F3's setup; `CancelAtPublication` (`run_pull_request_tests.rs:1481-1522`) | `Err(Interrupted)`; `publication_pending`, `Cancelled`; the same request gives `Published` |
| `f14_the_hold_refuses_a_save_and_a_comment_and_makes_no_branch` (slice B row) | F11's record at `closing` | `assert_save_held` and a comment checkpoint (`checkpoint_comment_locally_with`) through `assert_held`, both naming the record; no new commit on the branch |

**Slice C** (step 2 and U-1):

| Test | Setup | Asserts |
| --- | --- | --- |
| `finish_close_ends_a_new_branch_s_reservation_by_its_end` (`remote/reservation_tests.rs`, after `finish_close_requires_complete_closure_and_ticket_retirement_at_merged_context`, about 4054) | a new-branch parent built with `begin_new_branch_record`, `begin_new_branch` and `save_record` through `branch_deleted`; its close reservation reserved and advanced as `finish_close_case` (4016-4041) | `retired` and retired at `retired_oid`: `completed`; `retired` and retired at another OID: refused; `branch_deleted`: refused; `abandoned` with no pending ref: `cancelled`; the direct rows stay in the existing test (exempt, its guard) |
| `f1_a_merged_old_context_is_retired_and_its_closure_is_the_new_branch_s_only_commit` | `merged_on_tracking`, `deleted_on_remote` | preview `new_branch`, `old_tip == Some(T)`, no entries; `Published`; `assert_new_branch`; `assert_nothing_deleted_remotely`; `effects("worktree_remove") == 1`, `effects("branch_delete") == 1`; the stale tracking ref deleted by step 2, and the tracking context at `C` after the publication; `retired_contexts` at `T` with the record's operation; no `remote_context_states` row; the parent row `completed`; the link given at completion |
| `f2_unheld_entries_are_listed_and_the_old_commits_are_unreferenced` | rows: a squash after which the tracking primary changed the ticket again; a declined request (`P` unchanged); both with `notes.txt` committed on the context; `deleted_on_remote` | the preview's entries `[ticket Modified, notes.txt Added]` in path order; after confirm, as F1; no ref of `refs(&repo)` is `T` or a descendant of it (K-6) |
| `f4_a_deleted_history_and_its_stale_tracking_ref_are_retired` | `remove_context`, `deleted_on_remote`; the tracking ref left | `retired_oid` is the tracking tip; no removal effects; the tracking ref deleted; `retired_contexts` at it; `Published`; nothing deleted remotely |
| `f8_…` (slice C rows) | F1's setup. Rows: (e) a hook at `NewBranchCommitHeld` writes `draft.txt` into the worktree; (f) a hook at `BeforeCompletion` sets the history row `state='observed'`; (g) a hook at `BeforeCompletion` moves the tracking context ref | (e) `Branch(ContextChanged)`, abandoned, no pending ref, the old worktree and branch at `T`, no parent row. (f) and (g) `Branch(ExternalChange)`, abandoned from `branch_deleted`, no pending ref, the parent row `cancelled`, no `retired_contexts` row |
| `f9_a_kept_provisional_closure_is_the_new_branch_s` | `p.hold(p.closed_commit_on(T, 1_700_000_000, CLOSED_BY))`, `deleted_on_remote` | preview `Existing` with both fields; `Branch(Existing)`; the record's kind `existing`; `C`'s ticket holds `closed_at` and `closed_by` as `T` has them; `C`'s parent is `P` |
| `f10_a_crash_while_removing_the_worktree_continues` | F1's setup; `exit_at` `EXITS[10]` (`DuringWorktreeRemoval`) | the record at `worktree_removing`; the retry `Published`; one removal effect; one commit |
| `f10_a_crash_before_the_branch_deletion_continues` | `EXITS[11]` | at `worktree_removed`; then as above |
| `f10_a_crash_before_the_reservation_reserves_again` | `EXITS[12]` (`BeforeCompletion`) | at `branch_deleted`, no parent row; the retry reserves and retires |
| `f10_a_crash_after_the_tracking_deletion_restarts_the_reservation` | `F10_SEAM` | at `branch_deleted`, the parent row reserved, the tracking ref gone; the retry: owner epoch +1, `completed`, `Published` |
| `f13_…` (slice C rows) | a hook at `DuringWorktreeRemoval` calls `cancel_ticket_closure`; a TRACKING hook calls it under the reservation | `Err(Interrupted)` at `worktree_removing`, nothing reserved; `Err(Interrupted)` at `retired` with the parent row `completed`; each time the same request then gives `Published` |
| `f14_…` (slice C rows) | records at `worktree_removing` (F16) and at `retired` (F13's second row) | the save and the comment refused naming the record; at `retired`, no branch `refs/heads/manyhands/ticket/<ID>` is made |
| `f16_a_refused_removal_stops_unfinished_holding_nothing_remote` | rows: a hook at `DuringWorktreeRemoval` writes `draft.txt`; a hook at `BeforeBranchDeletion` moves the branch ref to `base` | `Err(RecoveryRequired)`; `worktree_removing` / `worktree_removed` with `cleanup == ContextChanged`; no parent row; `holds_new_requests`; a new confirm `Err(Busy)`; the cause removed (draft deleted, ref put back), the same request gives `Published` with the category cleared |
| `f17_a_crash_after_the_retirement_goes_to_step_three_without_the_reservation` | `F11_SEAM` | after the exit: `retired`, the parent row `completed` at epoch `e`, `retired_contexts` at `T`; the retry: `Published`, the parent row unchanged (epoch `e`, no restart) |
| `a_branch_answer_gives_its_code` row | (`mutation/bind/lifecycle_tests.rs:350`) `(ContextChanged, RecoveryRequired, "context_changed")` | red against a stub arm |

### 10.3 Guards run in each slice (unchanged behaviour)

- **A:**
  - every `closure::observe::tests` test, `closure_digest_golden_vector`, `pull_request_digest_wire_format` and `withdraw_digest_wire_format` included;
  - `withdraw_prepare_decides_its_answers_and_blockers`;
  - the pull_request module's `d17_…`, `d18_…`, `a_new_request_observes_live_and_a_continuation_its_record_s_mode`, `a_foreign_context_synchronization_is_busy_at_prepare_and_confirm`;
  - every `context_retirement::tests::retirement_proof_*`;
  - `remote::resolve_tests::republish_reads_lost_history_with_a_tracking_ref_and_a_current_batch_as_a_deletion`;
  - in `remote::sync::tests`: `released_checkpoint_descendant_after_old_push_intent_opens_a_continuation_attempt`, `an_unreturned_first_push_with_no_publication_evidence_is_retryable_not_ambiguous`, `a_confirmed_abandon_answers_the_push_so_a_new_synchronization_is_not_ambiguous` (the `push_absence` callers).
- **B:**
  - every test in `repository::closure::run_tests::pull_request` (the `d…` and `w…` tests take `promote_branch` and `begin_publication`);
  - `closure::record::tests`;
  - `process_exit_at_every_boundary_reopens_without_repeated_effects` (`closure_exit_child` edited), `d12_…`, `d13_…`, `w8_…`, `w13_…`;
  - `a_branch_parent_view_is_never_integrated`;
  - promotion's `every_interruption_reopens_with_one_candidate_and_only_unfinished_children` and `remote_children_publication_cleanup_and_replay_are_once_only`.
- **C:**
  - `finish_close_requires_complete_closure_and_ticket_retirement_at_merged_context`;
  - the closure run tests `a_child_publication_guards_the_delete_and_the_checkpoint_is_retired` and `without_a_child_or_tracking_context_nothing_is_reserved_or_deleted`;
  - promotion's `cancellation_before_and_after_integration_preserves_and_retry_finishes`;
  - `process_exit_at_every_boundary_reopens_without_repeated_effects` (the seams' visibility);
  - every test of the pull_request module.

No SSH target runs between steps. `ticket_closure`, `document_promotion` and `mutation_remote` compile through these files and run at review-ready (Task 16).

## 11. Slices

Each compiles, passes the static gates, adds no function without a production caller, and leaves the direct, close and withdraw tests green.

**A. The observation and consent.**
- `context_retirement/proof.rs`: `UnheldEntry`, `unheld_entries`.
- `remote/sync.rs`: `push_absence`'s third value, `republish_evidence_in`.
- `closure/observe.rs`: `NewBranchHistory` and its read, the `Gathered` fields, 7.2 and 7.3, `consented_head`, the preview's field.
- `closure/mod.rs`: `ClosureBlocker::RemoteContextUnmaterialized` (appended, not in `CLOSURE_BLOCKERS`), `NewBranchFacts`, `TicketClosurePreview.new_branch`.
- `closure/lifecycle.rs`: `ready`'s arm, `consented_head` in `begin_closure_record`, and an interim `require_identity` override (`promotion/lifecycle.rs:518-524`). The override answers `Err(TicketClosureError::RecoveryRequired)` for an unchanged confirm of a `(PullRequest, NewBranch)` preview, so no record is written until slice B. Refusals at insertion are unaffected: `require_identity` runs only when the digest is unchanged (`run.rs:566-569`).
- `mutation/bind/lifecycle_verb.rs`: `closure_blocker`'s arm.
- Tests: `unheld_entries_against_the_primary`, E-44, `new_branch_digest_wire_format`, `f5_…`, `f7_…`, `f8_…`'s confirm rows, `f15_…`.
- Mechanical: `new_branch: None` in the three literals (`observe_tests.rs:1562, 2060, 2457`).

**B. A new branch with nothing to retire (situations 3.2 to 3.4).**
- `closure/new_branch.rs` (new): `step`, step 1, step 3 with its reconciliation, `abandon` with no owner. A `retired_oid` record answers `Err(RecoveryRequired)` there; slice A's override now refuses only previews with `retired_oid.is_some()`, so it cannot be reached.
- `closure/record.rs`: `begin_new_branch_record` and `insert_record`'s `retired`, `begin_new_branch`, `begin_new_branch_closing`, `clear_pending_ref`, `abandon_branch_record`'s per-action steps and `pending_ref=NULL`, `begin_publication`'s pending-ref guard.
- `closure/checkpoint.rs`: `build_kept_closure_commit`; `context_moving` made `pub(super)`.
- `closure/lifecycle.rs`: `begin_closure_record` → `begin_new_branch_record`; `branch_step`'s dispatch.
- `promotion/lifecycle.rs`: `LifecycleRecord::synchronizes_context` (default `true`; `ClosureRecord`: `action != NewBranch`).
- `promotion/run.rs`: `promote_branch` runs `context_step` only when `r.synchronizes_context()` (`948`); `boundary` (`241`) and `cancelled` (`276`) made `pub(in super::super)`.
- `src/repository.rs`: `context_at_commit`; `FailurePoint::NewBranchCommitRecorded` and `NewBranchCommitHeld`, after `AfterClosureCheckpoint` (`1158`), each documented.
- `closure/run_tests.rs`: `NEW_BRANCH_EXITS` and `closure_exit_child`'s arm.
- Tests: `f3_…`, `f6_…`, `f8_…`'s step-1 row, the four step-1 and step-3 `f10_…`, `f11_…`, `f12_…`, `f13_…`'s publication row, `f14_…`'s `closing` row.

**C. The retirement and U-1.**
- `closure/new_branch.rs`: section 2 (2.1, 2.2, the reserved abandon). The interim `require_identity` override is removed.
- `closure/record.rs`: `retire_new_branch`, `reconcile_retirement`, `abandon_reserved_new_branch`.
- `remote/reservation.rs`: `finish_parent` (section 3).
- `closure/mod.rs`: `BranchAnswer::ContextChanged`. `mutation/bind/lifecycle_verb.rs`: `branch_code`'s arm.
- `promotion/run.rs`: `validate_cleanup_owner` (`2139`) and `require_configuration` (`2164`) made `pub(in super::super)`.
- `promotion/run_tests.rs`: `completion_returned` (34), `tracking_deleted` (46) and `local_effect` (483) made `pub(in super::super)` (mechanical).
- Tests: `finish_close_ends_a_new_branch_s_reservation_by_its_end`, `f1_…`, `f2_…`, `f4_…`, `f8_…`'s full-check and reservation rows, `f9_…`, the four step-2 `f10_…`, `f13_…`'s step-2 rows, `f14_…`'s other rows, `f16_…`, `f17_…`, and `a_branch_answer_gives_its_code`'s row.

The step-2 `f10_…` and `f17_…` need slice C. The plan's single commit subject, "feat(closure): close through a new branch", goes on slice C; A and B take "feat(closure): a new branch's prepare" and "feat(closure): a new branch with nothing to retire", unless the controller prefers one subject with three commits.

## 12. Named and mechanical edits

### 12.1 E-44 (named; for the controller's approval)

`closure/observe_tests.rs:1797-1809`, in `pull_request_prepare_decides_the_situation_and_its_blockers`, has two rows commented "Situations 2 and 3, as the direct close has them until Task 9". They assert `[RemoteContextDeleted]` (a context with `history("observed_published")`) and `[ContextMissing]` (no context).

Their intent was temporary, and Task 9 reverses them as Part D says. They are replaced by the Task 9 situations:
- **2:** ready; path `(PullRequest, NewBranch)`; `base` the tracking primary; `old_tip` and `retired_oid` the context's tip; entries `[ticket Modified]` (the context's body against the base's); `needs_context_child == false`.
- **3.4:** no context and no tracking ref: ready, `old_tip == None`, `retired_oid == None`.
- **3.1 by a stale tracking ref:** no context, `history("observed_published")`, the tracking ref at `base`: `retired_oid == Some(base)`.
- **3.1 by pushes only:** no context and no tracking ref; two proven pushes (a new observe-fixture helper `proven_push(oid)`, beside `reserve_context` at `observe_tests.rs:207`: a `synchronize_context` row of the ticket, reserved as `reserve_context` reserves one, then completed with its authority at `oid` as `promotion::run_tests::complete_child_row`'s UPDATE completes one), the newer a descendant of the older: `retired_oid` = the newer. With the older not its ancestor: `[HistoryUnresolved]`.
- **3.2:** no context, the tracking ref at `base` (which the tracking primary holds), no history: ready, no retirement.
- **3.3:** no context, the tracking ref at a commit off `base` that neither primary holds: `[RemoteContextUnmaterialized]`.

No other existing assertion changes intent.

### 12.2 Mechanical

- **A:** `new_branch: None` in `observe_tests.rs` at 1562, 2060 and 2457.
- **B:** `closure_exit_child`'s arm (`run_tests.rs:1125-1144`); `boundary` and `cancelled` visibility.
- **C:** visibility of `validate_cleanup_owner`, `require_configuration`, `completion_returned`, `tracking_deleted` and `local_effect`; the `a_branch_answer_gives_its_code` row.
- `push_absence`'s callers take the new third value (`sync.rs:7491-7500, 8338`). That is production code, with no assertion changed.

Task 10's DTO, Task 11's CLI and Task 12's conflict lookups are not touched.

## Open questions for the controller

1. **Situation 2 has no ancestor check on its pushes.** A proven push of the old branch that is not an ancestor of `T` is not skipped after the retirement (`sync.rs:8207-8214`). The publication child then meets a `Deleted` boundary and answers `remote_branch_deleted` (row 3) instead of publishing.
   - Built as the design says: the check is 3.1's only.
   - Narrowest fix: the same check in situation 2 against `T`, blocking `history_unresolved`.
2. **F8's "nothing else changed" cannot hold for an abandon under the reservation.** R-31's order removes the old worktree and branch before the re-check. An abandon from `branch_deleted` leaves them removed and `T` unreferenced (K-6), with nothing retired.
   - Step 1's re-observation re-checks the evidence before the removals, which narrows the window.
   - Fix: the F8 row's wording; no code.
3. **Every refusal under the reservation abandons and releases** (the re-check, local absence, the tracking ref `Changed`), so that no stop holds the reservation (R-31). The design names only the re-check's abandon.
   - The release on abandon ends the parent's row `cancelled`, as `supersede` ends one, through the same generalized `finish_parent` (section 3).
   - Confirm both.
4. **"The cleanup's code" for a full-check refusal** is built as `BranchAnswer::ContextChanged` → `recovery_required`, the code a direct close's `context_changed` cleanup stop answers (`bind/lifecycle.rs:322-331`). The alternative is `reconfirmation_required`, a one-arm change.
5. **The stored `closed_at` of a kept closure** is the clock's time when `C` is built, not the kept value. The column and `validate` require whole UTC seconds (`record.rs:494-499, 583`), and a kept closure need not have them (`canonical.rs:1256-1272`). `C`'s ticket holds the kept fields as `T` has them. Confirm.
6. **Push evidence `Ambiguous` or `Unknown` with no context.** It is not 3.1, and the direct close reports no blocker for it, since it reads fetch history only (`observe.rs:1248-1255`). So 3.4 offers a new branch whose publication child the push-absence boundary would stop.
   - Built as written.
   - Narrowest fix: `history_unknown` for `Ambiguous`, `history_unresolved` for `Unknown`.
7. **A local ticket branch with no worktree** is neither "a context" nor "no context", and step 3 could not make the branch at `C`. Built as today's `context_missing`.
8. **F8's "the old worktree changed between prepare and confirm"** answers as today at the confirm: `Blocked([ContextDirty])` (`promotion/run.rs:564`, `closure/lifecycle.rs:442`), not `external_change`, because the digest binds no cleanliness. Only a change after step 1 is the cleanup's refusal. The test asserts that.
9. **Readings, for confirmation:**
   - the digest also binds `retired_oid`, reading "the old context's tip" as what step 2 retires (`T`, or the 3.1 target);
   - 3.1 falls back to the deletion record's `last_published` when there is no tracking tip or proven push, and blocks `history_unresolved` with none;
   - "occupied" includes a worktree registration named `<ID>`, which `git2` cannot add a worktree over;
   - a cancellation that arrives under the reservation is honoured at `retired`.
