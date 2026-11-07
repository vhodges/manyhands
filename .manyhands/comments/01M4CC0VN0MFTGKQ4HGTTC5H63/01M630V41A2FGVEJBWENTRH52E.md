---
manyhands_managed: true
manyhands_kind: comment
id: "01M630V41A2FGVEJBWENTRH52E"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T05:06:38Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 22: complete

Task 22 is complete: a new item's draft is created by Save. Its code
commits are 53d8b93 and 0aea947. The docs are 4af076a, 5df44a6 and this
commit.

- **What changed.** Comment 01M62YY8PKY0JG92VPWJHDNPC0 describes it. The
  fix round added the following:
  - tests for a restored not-created draft's Save, and its close dialog's
    Save: each creates the item once;
  - tests for a create sent again after its answer was lost. It makes no
    second item: the draft retires, or, with changed text, goes to review;
  - small cleanups. The save record names how it was sent, and the
    context key comes from the Create prepare.
- **Rulings.**
  - R-20's order is one entry point, `DraftRegistry::new_item`, which
    Task 23's dialog calls.
  - R-46's words: "Crash recovery is off, so a new item cannot be
    created."
  - The four tests that faked a never-created item now run over a real
    one, with the same names and results.
  - A kept create record lets a new Save and a Retry both be in flight.
    This is documented, not changed: the library answers the second one
    `already_applied` or `occupied_path`.
- **Review.** It found 0 Critical, 0 Important and 9 Minor issues, and
  approved the task. One fix round closed five of the Minor issues. The
  controller checked the fix diff. The rest are parked for the
  whole-branch review or listed for the owner.
- **Tests.**
  - Before the fix round: the whole desktop binary, 970 passed, 0 failed;
    library `drafts::`, 55; `--test drafts`, 48.
  - After it:
    - `services::drafts::create`: 19 passed;
    - `drafts`: 161 passed;
    - `view::item`: 295 passed.

  No main test changed. The only earlier D3 tests that changed are the
  four that faked a never-created item.
- **Checks.** Clippy and fmt are clean.
