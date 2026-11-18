---
manyhands_managed: true
manyhands_kind: comment
id: "01M62SN53AY5WR4DY67BDWAJK9"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T03:01:02Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 21: complete

Task 21 is complete: a ticket's dependencies and parent can be edited in
the details sheet, with the cycle check. Its code commits are 54bddc4
and 07e8b7d. The docs are 670e64e, 1129104 and this commit.

- **What changed.** Comment 01M62QFXQS1D6J0GNR2CRGQXAZ describes it.
  The fix round added the following:
  - After the library refuses a save for its relationships, a dependency
    it cannot find still reads "not found, kept".
  - Use these choices checks the relationships again against the new
    base.
  - A ticket's name comes from the item's own reads before the picker's.
  - Closing the sheet closes the picker.
- **Rulings.**
  - The picker lists only this repository's tickets, without the ticket
    itself or its current dependencies.
  - A check still running does not block Save; a held rejection does.
  - A cycle is explained under the field whose change was checked.
  - The words, for example "This makes a cycle: {names}.", are in the
    ledger for the owner's on-screen check.
- **Review.** It found 1 Important and 13 Minor issues. One fix round
  closed the Important issue and three Minor ones. The controller checked
  the fix diff. The rest are parked for the whole-branch review or listed
  for the owner.
- **Tests.**
  - Before the fix round: the whole desktop binary, 955 passed, 0 failed.
  - After it:
    - `relations_edit`: 19 passed;
    - `sheet`: 30 passed;
    - `edit_tests::review`: 32 passed.

  No existing test changed.
- **Checks.** Clippy and fmt are clean.
