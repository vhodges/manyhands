---
manyhands_managed: true
manyhands_kind: comment
id: "01M64SZD98CT6ZC3QFN26PG4PN"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T21:45:07Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---
Task 27: the fix round's continuation and the document pass.

The fix round went on, in the same dispatch, with three more rulings:
- While a tab is editing, its strip mark follows the session's text, not
  the draft attached to it. Undoing back past a mode switch makes the tab
  clean in the strip and the pill alike.
- "Discard your repair?" reads "Your repair was sent and may already be
  made. Discarding does not undo it." while the repair runs or after an
  answer whose effect is unknown. This wording is for the owner to check.
- D3's dialogs whose Enter goes on to open an editor close themselves at
  once. The Kit's deferred close gives focus back to the opener 0.25 s
  later, which took focus from an editor that opened sooner.

Head `8fbc24d`. The whole desktop binary: 1073 passed, 0 failed. No
library code changed, and no main test changed.

The document pass amends the design:
- the Part E text, as Tasks 24 and 25 and the Part E review ruled;
- the avatar deviation;
- Save waiting while a relationship check runs;
- the Privacy sweep's reach;
- the native checklist's refused images;
- a fourth case in which a context is kept;
- the Known Limits from the parked index and the whole-branch reviews,
  including known defects `01M5B163PQ` and `01M4EHGE60`.

The rulings are in the execution ledger.

Next: one scoped re-review of the fix round, then the owner's on-screen
checklist and smoke test.
