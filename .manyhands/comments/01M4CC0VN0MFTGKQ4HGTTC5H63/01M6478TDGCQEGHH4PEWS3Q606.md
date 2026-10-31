---
manyhands_managed: true
manyhands_kind: comment
id: "01M6478TDGCQEGHH4PEWS3Q606"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T16:18:13Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---
Task 26, the keyboard and privacy sweeps, is complete.

- **Commits:**
  - the sweeps: `7e623a5`;
  - three D3 fixes the sweep found: `f18d8a2` (the sheet's Updated row had no name), `e3ddf06` (a focused review control is now scrolled into view) and `1a346c3` (focus stays in a dialog whose button disables while it works);
  - fix round 1: `c64be8a`, `3462077` and `f1a5599`.
- **Tests:**
  - the whole desktop binary passed, 1044 run and 0 failed, at `7e623a5`;
  - after the fixes: `keyboard_tests` 37, `privacy` 4, `create` 55, `move_document` 14 and `new_folder` 6;
  - both clippy runs and fmt are clean;
  - no library tests were run.
- **Review:**
  - The review found 1 Important and 9 Minor issues. The Important one: the privacy sweep's exemption for the draft section also hid the whole editing sheet and the review's own words.
  - Round 1 narrowed it to the editors themselves, and it found no leak. The same round walked the editing sheet and "Replace your edits?" by key, lifted the shared test helpers and split the toolbar walk.
  - It also added a fix: after a refused keyed press, focus moves to the field with the note, or else to the enabled primary button.
  - The re-review found every finding addressed and nothing broken.
- **For the owner:** in the review, the fields after the body's result editor are reached by Shift-Tab, because that editor keeps Tab. It is a design question, on the on-screen checklist.
- The desktop has no logging, so there was none to capture.
- The rulings are in the execution ledger.
