---
id: 01M7G8EYANW8C68N76J7R84JSX
item_id: 01M5F3GVDYTH1Q5V5FCS9YF7V3
created_at: 2026-11-15T18:45:38.746997417Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Wave 04 Cycle 05a (ticket `01M7D5Z7FZTGTC1HE25KQ1MYQY`, design Amendment 5 and Part G) does five items of this ticket's "Repositories page" list. They are done on branch `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY` and reach main when that ticket closes; this ticket's other items are not touched by it and stay open.

The four Add repository items, now on the Add repository sheet (which moved out of Settings in Cycle 05a):

- **Browse-unavailable reason beside each Browse.** The reason is a note line under each Browse… (Add and Create), not once at the page bottom. Test: `browse_says_why_beside_its_button` (P21). Code: `1ff07fe3` (Add), `cb78fe3d` (Create).
- **Text while the folder chooser is open.** Browse… is disabled and the note reads "Choosing a folder…". Same test, same commits.
- **Retry for a busy folder inspection.** A busy check draws "Manyhands is busy. Try the check again in a moment." with a Retry button. Test: `folder_checks_are_rows_with_their_actions` (P9). Code: `1ff07fe3`.
- **A root that is not UTF-8 is named, not silently ignored.** The check row reads "This folder's name can't be used here. Choose another." with a Choose another action. Test: `a_folder_that_is_not_text_is_named` (P22, unix only). Code: `1ff07fe3`.

The fifth, "Forms are not cleared after success. Decide whether they should be", and its product question "Should the Repositories forms clear after a successful operation?", are answered by the owner's decision 16 (Cycle 05a, 2026-11-14, "Keep it; clear after success") with decision 12: typed text is kept when the sheet closes by Cancel, Escape, the close button or a click outside, and is cleared after a successful add, which closes the sheet. The clearing applies only to the open the add came from (ledger Ruling 19, C-5); a pending Add is always dropped, and quitting loses typed text with the app. Tests: `the_add_sheet_closes_every_way` (P7) and `late_work_moves_no_focus_after_add_closes` (P8). Code: `0d5cae5c`.

Not done by this Cycle: every item not named above. Cycle 04 took the items its own Part G folded in; this comment does not restate them.

The branch is not merged yet. The execution record is `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-execution.md` on that branch.