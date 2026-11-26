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
