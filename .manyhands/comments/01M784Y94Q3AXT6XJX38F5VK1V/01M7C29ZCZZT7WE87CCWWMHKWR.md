---
id: 01M7C29ZCZZT7WE87CCWWMHKWR
item_id: 01M784Y94Q3AXT6XJX38F5VK1V
created_at: 2026-11-14T03:41:06.694182196Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint 5 of 5: Task 13 done. The branch is ready for the owner's review, with two things only the owner can do (below).

**Branch:** `manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`, pushed; HEAD and origin were equal when this was written (`623120c8`, then this comment's own commit). Base `ba95e1b7` = `origin/main`, unchanged. `git merge-tree --write-tree HEAD origin/main` is clean. Boundary: `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging` prints nothing; only `src/desktop/` and its tests changed. No other branch was pushed.

**Tests run (Task 13):**
- Static gates, rc=0: `cargo check --all-features --locked`, `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` (rerun at `2f8a1f31` after the fix wave: clean). The CLI smoke test ran, no window.
- Fast tier, once, `--no-fail-fast`, at `619ffb64`: **5,998 passed, 2 failed.** No SSH target (desktop only). The two failures are Cycle 03a tests this branch did not edit, `desktop::view::repositories::tests::the_menu_by_pointer_and_keyboard` and `check_all_checks_each_repository`. Each passed alone, and the module filter passed three times. They failed only in the 22-minute parallel run. The final review found both read or acted on the schedule before an in-flight check had ended (the same family as Cycle 03a's `595667cd`); the fix wave added the waits (no assertion removed) and I reran both by exact name. I did not rerun the whole fast tier: AGENTS.md reruns only a target whose code changed.
- After the fix wave, by exact name: both tests, the 5-test module, and the popover and Activity focus tests. All passed.

**Final whole-branch review** (most capable model, read-only, 58 commits): 0 Critical, 2 Important, 7 Minor. Important: (I-1) the two flaky tests' waits, fixed; (I-2) with the popover open, a focus loss on a row button (a new row pushes the focused row out of the scrolling list) sent focus to the footer item and left Escape and the arrows dead in the popover. Fixed in `view/workspace/focus.rs` with a test seen failing first (`the_button_scrolls_out_of_the_list`). One fix wave (`f815b793`, `e68c5516`, `2f8a1f31`), one scoped re-review: all addressed, no new breakage. Acceptance Status is filled in the Cycle document: A1 to A13, A15 and A16 by tests by exact name, A14 waits for you.

**For you (not done, and I will not assume a result):**
1. Run the on-screen checklist `docs/plans/2026-11-12-wave-04-cycle-03b-on-screen-checklist.md` (85 rows, S-1 to S-10, results blank; a "Minimum pass" list is at the top) and the desktop smoke test `devenv shell -- cargo run --locked --features desktop --bin manyhands` on Wayland, in the dark theme. A14 and the popover's focus move while the footer draws have never been seen on a real display.
2. Decide the document amendments (final-review item m-2): the rulings you approved at Task 5 (C1 to C7, C9) and three implementation rulings (T9-2/T10-1, T11-1 to T11-3) are recorded only in the execution ledger, not in the design or Cycle documents. I did not edit your documents; say if you want an amendment list added.

**Rulings I made after checkpoint 4** (all in the execution ledger, Task 13): T13-1, the two fast-tier failures are load flakes of a known family, hardened, not product defects (costs if wrong: a real race the waits only hide; both pass alone and in the module). T13-2, the new popover test puts its failed request on another repository than the pending rows, because a failed request on the same repository dropped that repository's pending rows from the log; nobody investigated whether that is the intended replacement of a pending listing by the request's own row (costs if wrong: a real dropped-rows case; worth a look).

**Deferred Minors** (for the owner or a follow-up; none blocks): duplicated helpers (`sharing`/`item_sharing`, `dto_kind`); `model/repository_state/menu.rs` imports three constants from `view::sidebar`; F4's dead drawing code kept under `cfg(test)`; P13's journey covers three counted kinds, not every kind (agreement is structural, P2 covers each kind); file sizes (`model/operations.rs` 668, `footer_tests/mod.rs` 518, `lifecycle/privacy_tests.rs` 526, `workspace/lists.rs` 502); the disabled Retry's reason text "Not sent again after a later Edit or release" is gone (told at checkpoints 3 and 4); no model test pins the order of several holds. The full list of 67 parked Minors and their triage are in the review report and the ledger.

Not done, and not authorized without you: a pull request, merge into main, closing the ticket (`ticket close` after review approval), worktree cleanup, native CI.