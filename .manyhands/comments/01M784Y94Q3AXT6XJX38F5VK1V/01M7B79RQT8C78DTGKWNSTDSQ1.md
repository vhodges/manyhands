---
id: 01M7B79RQT8C78DTGKWNSTDSQ1
item_id: 01M784Y94Q3AXT6XJX38F5VK1V
created_at: 2026-11-13T19:49:08.308706647Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint 3 of 5: Tasks 6 to 8 done (the actions, the test seam and the conversion of the old operations tests onto it). Each was reviewed by a fresh reviewer; Task 8 by two, with one fix round.

**Commits** (base `8b960272`, ledger commits between):
- Task 6 `35bdd07f`: `ActivityMenuChoice`, `choose_activity` (re-checked through `ActivityLog::still_offered`: same revision, an enabled offer with the same action and label), `run_activity_action` (an exhaustive match over the 21 action kinds), the handlers' `origin` parameters, the test seam (`activity_support.rs`), and E-5 part 1 (six test files).
- Task 7 `f6559d70`: E-5 part 2 (seven test files).
- Task 8: part A `1b3081e4` and fix `386b179b`, part B `2eb033d7`: E-6 (about 25 test files). Run as two commits because of its size.

**Tests run** (each alone by exact path, count 1, no whole-module or whole-suite run): Task 6, 35 (the new `each_action_runs_its_handler` covers all 21 kinds, plus T11 and T12, and every E-5 part 1 test); Task 7, 31 converted tests plus 8 that share a changed helper; Task 8, 35 in part A and 23 converted plus 6 helper-sharing in part B, plus two for the fix. Every converted file's test-function list is equal before and after, in the ledger. fmt, check and clippy `-D warnings` were clean at each commit.

**Removed drawn-text assertions** (called out per file in the ledger): each is a drawn-text read replaced by a model read (`request`, `share_result`, `offered`, `row`, the pending `OperationDto`), because the copy is asserted by the model tests P4 to P7. The reviewers confirmed that none was lost or made vacuous, except the one weakened absence check found in Task 8 part A (`no_stop`), which the fix round tightened.

**Rulings so far** (ledger): P-F1 to P-F16, T1-1, T2-1, T2-2, T3-1, T3-2, T4-1 to T4-4, C1 to C9, T6-1, T7-1, T8-1 to T8-3. New since checkpoint 2: a test E-6 names that touches nothing in Activity stays unchanged (T8-2); Task 9 must keep or update the `Region::Operations.id()` guards on the kept drawn sentinel scans (T8-3); Task 9 owns replacing the seam's `origin_of`, deleting the focus maps and the kept helpers (T6-1).

**For the owner to look at.** The disabled Retry's reason text "Not sent again after a later Edit or release" lives only in the old view, and the model's `Offer` carries no reason, so Task 9's swap removes it. The behaviour stays tested. If the new Activity should explain a disabled Retry, that is a design gap.

**Deferred Minors** carried to the final review: Task 6, six; Task 7, six; Task 8, eight. All are in the ledger.

**Boundary and branches.** `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging` prints nothing. `origin/main` is still `ba95e1b7` and `git merge-tree --write-tree HEAD origin/main` is clean. Branches published: this ticket's only.
