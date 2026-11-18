---
manyhands_managed: true
manyhands_kind: comment
id: "01M61EAGHWPBGPS42D2DZGD6PP"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T14:23:45Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 17: Release on leave

The commits are 0a78378 for the code and tests, and this commit for the
ledger row and this comment.

- **What changed.**
  - The desktop now sends `context release` for an item's private copy
    when the design's four conditions hold:
    - this run made the copy, saved nothing into it, and has not
      released it already;
    - the tab leaves clean, or a draft of the item is discarded, from
      the tab, its banners or the Drafts page, with or without a tab;
    - no save of the item is in flight;
    - a fresh listing of the draft store shows no draft of the item.
      With crash recovery off, or a failed listing, nothing is released.
  - A tab closed while its Edit prepares releases the copy once the
    answer is ok with created: true.
  - Quit releases nothing, and refusals show in the operation area.
  - A refusal with recovery_required, while the read shows no copy,
    offers "Release stranded work context" (R-40).
  - R-78 holds: once a later prepare of the item was sent, a release is
    never sent again, and once a later release was sent, a prepare is
    never sent again. In the operation area that Retry is shown
    unavailable, with the reason. Resume leaves the tab viewing.
- **Tests.** The four session-outcome rows were written first, and each
  answers as the design's table says, so there was no stop. The brief's
  other tests and three more followed. The restored-draft test is
  written in Task 18 (P-4). The release path already covers it.
  - RED: 11 of 17 failed against a stub.
  - Each of 13 mutations was caught.
- **Runs.** All ran through Devenv with the normal profile:
  - `edit`: 199 passed;
  - `drafts`: 137 passed;
  - `keyboard_tests`: 23 passed;
  - the whole desktop binary: 846 passed, 0 failed;
  - both clippy runs and fmt: clean.
- No main or Task 0 to 16 test changed, and no library file changed.
