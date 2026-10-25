---
manyhands_managed: true
manyhands_kind: comment
id: "01M5J349989WE6SFASNNZ7CDE7"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-22T15:19:30Z"
---

Task 3 is complete: atomic writes, claims, listing, retirement and
discard, in commit `8a36495`.

- **Built.** `src/drafts/store.rs` holds `DraftStore`, `DraftClaim`,
  `Listing` and `Held`, the failure points and `CRASH_EXIT_CODE`. Tests
  are `store_tests.rs` and `tests/drafts/{write,crash,claims}.rs`, and
  `access.rs` is extended. `Folder` gains `open_lock` (opened or
  created, 0600, no-follow). `open_file` now tells a file that vanished
  after listing apart from one it cannot open.
- **Tests.**
  - `--test drafts` passes 32 and `--lib drafts::` passes 24. Clippy and
    fmt are clean.
  - The crash test runs each of the five failure points in a real child
    process. The parent asserts exit code 86 and one test run, then finds
    a whole revision: 1 before the rename, 2 after it.
  - The claims test holds a lock in a live child process.
- **Small decisions:**
  - A file that vanishes between reading the folder and opening it is
    skipped. A file that exists but cannot be opened is still listed as
    unreadable.
  - The next claim of a draft removes its stray temporary files.
  - A write at the same revision is allowed, so the session can write
    again after a failed folder sync.
- **Review.** An independent review approved the task with no Critical or
  Important finding. It traced create, claim, retire, discard and list
  across two processes and found no sequence that gives two writers.
  Seven minor points are deferred to the final review. One is handled in
  Task 6: if `create` fails at the folder sync, revision 1 is left on
  disk, so the desktop must not simply create again.
- **Known gap.** The Windows changes are uncompiled here. The native run
  after merge covers them.
