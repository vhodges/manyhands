---
manyhands_managed: true
manyhands_kind: comment
id: "01M5ZHGRE69APG552HWF0H60S3"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T20:41:06Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Part A checkpoint: complete

Part A, the item frame (Tasks 3 to 8), passed its checkpoint.

- **Merge.** Main had not moved (a372edb), and `git merge-tree` was
  clean, so there was nothing to merge.
- **Runs.** The first run was at 12ed527; HEAD then only differed in
  docs.
  - The desktop binary: 624 passed, 0 failed.
  - Desktop-only clippy, all-features clippy and fmt were clean.
  - The CLI isolation check exited 0, also when re-run through Devenv.
  - `--test drafts` was left out by ruling: it tests the library's
    drafts store, which Part A does not change.
- **Review.** A fresh reviewer, with the adversarial brief on the
  read-only guard, found that the guard holds against every listed input
  and that viewing writes nothing. It raised one Important finding: a
  full tab strip did not bring the shown tab into view. It also raised
  eight Minor ones.
- **Fixes**, in d097857 and 1a50165:
  - The tab strip scrolls the shown or focused tab into view.
  - The sheet names the private copy.
  - The write tests also check the request queue.
  - A new test proves the Undo stop.
  - The pill never hides a kept draft.
- **Owner decisions** (2026-10-27):
  - The pill combines a kept draft with Closed and Needs attention.
  - The read view's hover handles become a Design Deviations row.
  - The viewing Waiting on rows become a Design Deviations row.
- **After the fixes.** The whole desktop binary: 631 passed, 0 failed.
