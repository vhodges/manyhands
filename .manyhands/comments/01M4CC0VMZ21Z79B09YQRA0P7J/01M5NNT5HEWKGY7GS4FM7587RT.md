---
manyhands_managed: true
manyhands_kind: comment
id: "01M5NNT5HEWKGY7GS4FM7587RT"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T00:43:45Z"
---

Task 6 (sessions, flush schedule and recovery state) is complete:
`3e06bf1` and the fix `dd04700`.

**What it adds.** `DraftSession` in `model/drafts.rs`, with `IDLE` 1 s
and `MAX_UNFLUSHED` 5 s. A session is due at
`min(last change + 1 s, first unflushed change + 5 s)`, and a rebase is due
at once and raises the revision (R16). While a write is in flight nothing
else is written; edits made meanwhile become one write of the newest
revision when it lands. A failed write keeps `flushed`, so the state reads
`Failed(kind)` until a later write lands. The registry gains `begin`,
`edit`, `flush_now` and `flush_all`, takes every time from the
background executor's clock, and writes behind the `DraftWrite` gate. A
failed flush shows "Recovery copy of <title> not saved: <reason>" with
Retry (`Workspace::retry_draft_flush`). The title is the base's title,
falling back to the item ID; the draft's own title never appears there
(R25). An edit redraws the window only when the recovery state changes.

**Tests.** `desktop::model::drafts` 11/11, `desktop::services::drafts`
12/12, `desktop::view::operations` 12/12. Check and clippy are clean with
`--features desktop` alone and with `--all-targets --all-features`; fmt
is clean. A sentinel string in the draft's body and title appears in no
operation entry.

**Review.** Approved, spec compliant, no Critical or Important findings.
The fix round added a test for a write whose folder sync fails after the
rename (the revision is stored, the state reads `Failed`, and a retry
rewrites the same revision), quieted per-keystroke redraws, and documented
that a dropped `begin` leaves revision 1 listed for recovery. The store's
test failure point lasts for the store's life, so the "retry then Saved"
step is pinned in the model test; this is a test-only limit, not a product
gap.

**Rulings.** In the 12-second typing test the second write lands at
+10.1 s: the first edit the +5 s write misses is at 5.1 s. `begin` waits
behind the `Drafts` gate, the design's "list and claim" read. A 4-line
registry observer in `view/workspace.rs` shows and clears the entry.

**Deferred.** `view/operations.rs` is at 596 lines (564 before D2); a split
is for the final review. Read-only-folder tests assume a non-root user.

Task 7 starts next.
