---
id: 01M7BM15DT9MZ2EJYZG7RZ3XD9
item_id: 01M784Y94Q3AXT6XJX38F5VK1V
created_at: 2026-11-13T23:31:42.11247948Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint 4 of 5: Tasks 9 and 10 done (the swap to the Activity list, the row menu, the privacy sweeps, the toolbar and the filter). Each was reviewed by fresh reviewers; Task 9 by two, then a fix round with a scoped re-review; Task 10 by one, then a fix round with a scoped re-review.

**Commits** (base `c0734a5a`, ledger commits between):
- Task 9, three milestones: `77494d11` (pane, rows, keys, tooltip, the old operations drawing removed; `view/operations.rs` 954 to 154 lines, A13 met), `19c26ca9` (the row menu, focus to the list, the focus maps deleted), `719b027d` (the privacy sweeps at the swap), `e22879bb` (the lifecycle dialog re-exports the model's offer words, a parked Task 3-8 Minor), fix `eaa34112`.
- Task 10: `e6a5285a` (toolbar, filter, the more menu with Clear finished, tests, E-10), `f36dea31` (the dock and repository menus close each other, R-13), `7a4159ad` (Show it turns the filter off for a hidden holder), fix `54976f45` (P21), `437e5cfe` (a debug print removed).

**Fix rounds.** Task 9: two Important findings, both about assertions. The T8 checks could not tell the drawn revision from the log's current one (now an A-B-A change in one update, with three mutations seen failing), and `retire_left_offer` was weaker than the `!present` it replaced. Task 10: P21 `rows_fit_a_narrow_activity` omitted the S2, state, reminder, lifecycle-Resume and S8 rows and never scrolled; it now covers every kind in a 360 px window that scrolls. Scoped re-reviews found all addressed, nothing new.

**Tests run** (each alone by exact name; no whole-module, whole-suite or SSH run; the fast tier is Task 13's): Task 9, 43 new or edited, 176 E-5 and E-6 reruns in four batches, 125 others (quit, privacy, footer, dock, keyboard shell, settings, workspace, drafts) looking for breakage, none broken; fix round, 26. Task 10, 13 new or edited plus 30 E-7, keyboard and P18 tests and 17 menu, toolbar, footer and dock tests; fix round, 3 and then 1. fmt, check and clippy `-D warnings` were clean at every commit.

**Rulings** (ledger): T9-1 (three milestone commits), T9-2 and T10-1 (the list starts its highlight on the first shown row whenever it holds focus, by any route, without stealing one; pinned by P15's Tab clause), T9-3 (`allow_parking` in P15 and P16). U-2 and U-4 both passed, so there is no `VirtualList` amendment.

**Found and fixed in Cycle 03a code.** The repository menus and the dock menu did not close each other (R-13); the Kit's focus-based dismissal hid it until an all-pairs layers test read state before any draw. Two one-line clears in `footer.rs` and `repository_menu.rs`.

**Process deviation.** For the toolbar the Task 10 implementer wrote production code before its tests. P29, P30 and P32 were green on the first run because their behaviour existed; the other tests were shown failing by mutating the line each pins. The reviewer judged from the code that each would fail.

**For the owner to look at.** (1) The disabled Retry's reason text "Not sent again after a later Edit or release" is gone with the old view and has no replacement (raised at checkpoint 3). (2) P19 sets the stopped-share and stopped-republish facts through the library calls the Workspace reads, not through a real share over SSH; the model test pins the same table.

**Deferred Minors** carried to the final review: Task 9, nine; Task 10, three. All are in the ledger.

**Boundary and branches.** `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging` prints nothing. `origin/main` is still `ba95e1b7` and `git merge-tree --write-tree HEAD origin/main` is clean. Branches published: this ticket's only.