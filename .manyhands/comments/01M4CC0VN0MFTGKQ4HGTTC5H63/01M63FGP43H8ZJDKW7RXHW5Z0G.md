---
manyhands_managed: true
manyhands_kind: comment
id: "01M63FGP43H8ZJDKW7RXHW5Z0G"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T09:23:04Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Part D checkpoint: passed

Part D (Tasks 20 to 23: the details sheet's fields, relationships, and
creating documents and tickets) has passed its checkpoint review. The
fix commits are 6aa05ed and 4d7ef17. The docs are 269ee8d, 6eb5277 and
this commit.

- **The gate.** The whole desktop binary passed 986, with 0 failed, in
  682 s. The library's drafts tests passed 55 and 48. Both clippy runs,
  fmt, the CLI-isolation check and `check --all-features` are clean.
- **The review.** It approved, with 9 Minor issues. Two fix rounds closed
  six of them, and the re-review approved round 1. The controller checked
  round 2's diff.
  - **A stray release.** After a lost create answer and a second Save,
    leaving the tab no longer asks to release a private copy that holds
    the item's commit.
  - **A new item's tab.** Until its first read finds the item, it shows
    the item as created, in its private copy, also after Done. It shows
    no provenance and no "Not closed", and a later "not found" is shown.
  - **An occupied path that could not be checked.** The tab shows the
    library's refusal, and no longer says another file is there.
  - **A Drafts-page discard while New ticket… prepares.** It now closes
    the new tab and releases its private copy.
  - **A new ticket's fields.** An emptied Project and a `closed` status
    are tested through create.
- **Parked for the whole-branch review:** three of the review's Minor
  issues (an unreachable relationship refusal at create, one registry-only
  route, and slow tests under load), plus a few untested edges.
- **Tests.** After round 1 the whole desktop binary passed 992, with 0
  failed. Round 2 passed its filters (`create` 51, `view::item` 295).
- **Checks.** The branch merges with main cleanly, and main is unchanged
  at a372edb.
