---
manyhands_managed: true
manyhands_kind: comment
id: "01M5PTH25RJ38F6RDG13HY49FS"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T11:25:24Z"
---

Task 10 (the Drafts page) is complete: `3154dce` and the fix `82bdb26`.

**What it adds.**

- **The page.** The Drafts page lists every kept draft. Each row shows
  its kind, title, repository, the time it was saved, its target's state
  and the actions the restore policy allows.
  - A draft restored in this window reads "Restored in this window" and
    offers Show, which focuses its tab.
  - A draft held by another process offers Inspect only.
  - An unreadable file offers Discard only. Retry appears only after a
    failed read.
  - An empty store reads "No drafts are kept."
  - If the listing fails, the page shows the store's fixed sentence for
    that failure.
  - Comment drafts are classified against their thread and labelled
    "Comment on <item title>". Outside its own row, an item is named by
    its title or ID, never by the draft's title (R25).
- **Opening it.** `secondary-shift-d` opens the page. Launch shows
  "Recovered drafts: N" with Review and Dismiss. N counts the drafts
  found at launch that are not claimed here, so it falls as drafts are
  restored or discarded. The tab banner gains "Review in Drafts", which
  opens the page with that draft's row focused.
- **Routing.** Showing and hiding the page follows the design's Routing
  rules (R23), all in `content.rs`. The page does not know its host, so
  Show asks the workspace to show the tab. A Review requested while a
  dialog is open waits until the dialog closes.
- **Not yet wired.** Restore, Inspect and Discard are drawn on the rows
  but do nothing until Task 11, which also adds Export.

**Rulings.**

- Show goes through the workspace instead of the page calling
  `Workspace::show_tab`, so that the page stays independent of its host.
- If the launch listing fails, there is no "Recovered drafts" notice for
  the session. The Drafts page still lists the drafts. Without this, a
  draft kept this session would be counted as recovered.

**Tests.**

| Filter | Passed |
| --- | --- |
| `view::drafts` | 18 |
| `model::drafts` | 30 |
| `services::drafts` | 38 |
| `view::operations` | 12 |
| `view::workspace` | 25 |
| `view::settings` | 93 |
| `view::keyboard` | 12 |

The `view::item` and `view::tabs` filters also pass. Every new test
failed once with its code missing or reverted. Check and clippy are
clean with `--features desktop` alone and with `--all-targets
--all-features`, and fmt is clean.

**Review.**

- The task review found the spec met and approved the code, with seven
  minors.
- The fix round closed six:
  - a late-read test that never reached the stamp check;
  - no view test of a row held elsewhere;
  - Review in Drafts applied under a dialog;
  - a Show with no tab to show;
  - the empty-folder fixture's cleanup;
  - a launch set taken from a later listing.
- A scoped re-review approved the fix.
- Deferred to the final review: the failed-listing test builds its error
  by hand. `view/operations.rs` (601 lines) and `model/operations.rs`
  (568) were already over 500 and grew by 5 each. The new entry lives in
  its own file.

Task 11 follows.
