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
- **E-25** (Task 1) `tests/cli_remote/retirement.rs` about 131-153 (SSH target `cli_remote`): "default poll retains own-worktree read precedence" flips: `ticket show` reads `closed` and `OTHER_ITEM` is ready. **Reverses an intent; approved by the owner in decision 15.**
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
- Note for vh-p7st2 (not this Cycle's, R-22): `desktop/.../lifecycle_lock.rs:109` leaves `confirmed` unlocked, but a new-branch record holds the ticket from step 1 while at `confirmed`, so the desktop would not show it locked.


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
