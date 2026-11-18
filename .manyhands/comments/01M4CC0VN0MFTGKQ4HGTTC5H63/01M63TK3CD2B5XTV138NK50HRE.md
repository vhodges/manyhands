---
manyhands_managed: true
manyhands_kind: comment
id: "01M63TK3CD2B5XTV138NK50HRE"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T12:36:38Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 25: Repair

Repair works. The commits are 862c82e and 44c8c0a; the docs are this
commit.

- **Entry.** "Needs fixing" offers Repair…. An unmanaged document's
  banner offers Adopt…. An Inspect tab of a file that needs no fixing
  offers Repair… on its page. A file that is not UTF-8 says it cannot be
  repaired here.
- **The view.** A plain text area holds the whole file exactly as read.
  No markdown editor or draft store touches it.
- **Preview** goes through the one confirmation dialog. It names Root,
  Path and ID, and says "A new ID will be assigned" for an adoption.
- **Answers.** A stale preview offers Prepare again. An ambiguous
  identity shows the library's message and code. A repair made, the tab
  follows the item by its ID. A retry after a lost answer is answered
  `ok`, with one commit.
- **Leaving.** Closing, Cancel and quitting with edited text ask first.
- **Decided.** A repair never writes primary, so the tab follows the
  item rather than reading its old path again. The observation is the
  tab's own read's token.
- **Tests.** The whole desktop binary passed 1023, with 0 failed, at
  862c82e. Then `repair` 13 passed, `model::item` 9, `view::item` 305,
  `confirm` 24 and `model::tabs` 10. Both clippy runs and fmt are clean.
  No library tests were run.
