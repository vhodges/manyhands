---
id: 01M78YQEAWWJ05VM1EG7Q1H9FB
item_id: 01M784Y8C73B39GR4JWENXE5D0
created_at: 2026-11-12T22:40:50.26636681Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 5 checkpoint (the repository menu, Part D), 2026-11-12. Tasks 1-5 are done; ledger: `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md`.

**Commits** on `manyhands/ticket/01M784Y8C73B39GR4JWENXE5D0`, all pushed, base origin/main `37be101d` (not moved): Task 1 `a2abc5fa`; Task 2 `4979f441` + fix `b103c639`; Task 3 `6cb077c2` + fix `eb2bc10e`; Task 4 `02bd0015`; Task 5 `caa23a81` + fix `ba2a24bc`; ledger commits `d21b14f5`, `3dbaa472`, `64d92f09`, `94f5abbd`, `b4c6421b`. Boundary (`git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging`) prints nothing after every task. Each task had a fresh implementer and a fresh independent reviewer; Tasks 2, 3 and 5 needed one fix round each (scoped re-review approved), Tasks 1 and 4 none.

**Tests run** (exact names, `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`; no whole-suite or SSH run; the fast tier runs once at Task 10):
- Task 1: 7 ran, 7 passed (`unshared_follows_attempts_shares_and_saves` U1-U15, `to_share_counts_and_finished_shares`, `attempt_summary_counts` E-9, `status_text_table` E-9, `finished_shares_and_saves_are_heard`, `the_stored_outcome_and_shared_copy_are_kept`, `check_now_attempt_reads_the_report` E-17).
- Task 2: three tables (`each_state_and_its_precedence`, `tooltips_say_the_state_and_the_facts`, `menu_items_follow_the_state`), 1 each, passed, twice (before and after the fix).
- Task 3: `rows_and_list_states`, E-1/E-2/E-3/E-12 tests and the `select_a_repository` users: 49 ran, 48 passed, 1 known-flaky timeout under parallel load (`edit_the_mode_key_save_and_done_by_keyboard_alone`, passes alone 8.6 s, recorded flaky in the Cycle 01 and 02 ledgers); fix round: `rows_and_list_states` + `a_kept_selection_listed_again_is_resolved_again`, 2 ran, 2 passed.
- Task 4: `titles_of_what_is_not_shared` 1 passed; `rows_and_list_states` 1 passed; and, for a named doubt, `polling_privacy`, `polling_keyboard`, seven `retire_tests`, three `retire_real_tests`: 12 ran, 12 passed.
- Task 5: `the_menu_by_pointer_and_keyboard` 1 passed (and three reruns); E-13, `settings_polling_controls`, `rows_and_list_states`, `titles_of_what_is_not_shared` 4 passed; E-16's five call sites (seven tests) passed; three sidebar-Remove tests 3 passed; fix round final 3 passed. fmt, check and clippy (`-D warnings`) exit 0 after every task.

**U-checks.** U-1 holds (`shift-f10` and `menu` reach the `Repositories` context from the Kit list; the Kit `List` binds neither): no fallback. U-2 holds (a right press reaches the row's hook). U-4 holds (rows applied with `window.defer` land after one frame; no render loop): no fallback. U-6 settled by `unlock_after_cancelled_prompt` (a user-lane Check now of a locked key asks for the passphrase; the prompt in that test comes from the double): decision 9 stands. U-7 does not hold: a `LocalContextResolve` envelope names no item, so a local-context resolve's save shows in "N to share" at the next check, not at once (added to K-1 in the ledger; the design is not edited). U-3 (Task 9) and U-5 (Task 8) remain.

**Owner rulings during execution** (asked and answered 2026-11-12, Task 2): Part B rows 6 and 7's last-answer triggers hold only while checking is on and the repository is not paused, so label, sentence and Settings agree (amends Part B's table in the ledger); rows 8 and 10 are not gated on `enabled`; an unshared item with no parsed ID is named "an item".

**Rulings and notes** (full text in the ledger): the plan's "with no list read" (P5/P33) cannot hold literally because every finished request batch already reloads the repository list (`operations_changed`); the journey holds that one read and asserts the count already moved, no poll ran and the count is unchanged after the read lands. Settings… on a row also calls the Repositories page's `manage_root` (the plan's `set_selection` alone is ignored by that page). The title maps are `BTreeMap` (`ItemId` has no `Hash`). The sidebar's Remove returns focus to the list until Task 7 deletes the block. Test edits so far: E-1, E-2, E-3 (row parts), E-9, E-12 (part), E-13, E-16, plus E-17 (one added assertion) and E-18 (an unused test constant); removed assertions are only those E-2 and E-3 name (the "Showing the last list read" text and the `repo-root` path).

**Deferred Minors** are listed per task in the ledger and go to the final review; Task 6 must exercise the R-40 toolbar branch of the menu opener.

Next: Task 6 (toolbar buttons, the More menu, Check all; merge main).