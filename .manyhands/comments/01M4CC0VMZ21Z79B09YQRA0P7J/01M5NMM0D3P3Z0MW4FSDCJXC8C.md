---
manyhands_managed: true
manyhands_kind: comment
id: "01M5NMM0D3P3Z0MW4FSDCJXC8C"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T00:22:54Z"
---

Task 5 (the store in the desktop) is complete: `dc22ae8` and the fix
`926c505`.

**What it adds.** `LibraryBackend::new` takes the opened store
(`DraftStore::open_default()` in `run()`, `open_in` in the fixture).
`Backend::drafts` answers `DraftStoreUnavailable(kind)` when the store
could not be opened. `Services::drafts()` holds a `DraftRegistry` that
lists the store once at launch under `ReadKind::Drafts`, on the
background executor, with a stamp so a late listing is dropped.
`stored_for` answers a readable ticket or document draft that this
process has not claimed. When the store cannot be opened the window still
opens, and the operation area shows "Recovery copies are unavailable" with
the fixed reason and no Dismiss. `ReadKind` gains `Drafts`,
`DraftWrite`, `DraftItem`; `JobKind` gains `DocumentSave`,
`TicketSave`.

**Tests.** `desktop::services::drafts` 5/5, `desktop::view::operations`
12/12. Check and clippy (`-D warnings`) are clean with `--features desktop`
alone and with `--all-targets --all-features`; fmt is clean.

**Review.** Approved, spec compliant, no Critical or Important findings.
Two minors were fixed in `926c505`: the no-Dismiss assertion now checks
every label in the session section, and `stored_for` compares typed item
IDs. Deferred: the per-variant `dead_code` allowances come off in Tasks 6
and 7; the unavailable entry sits beside "No requests".

**Rulings.** `stored_for` counts drafts held by another process (the
design's banner condition is "not claimed by this process"; Restore will
then meet `OpenElsewhere`). It never answers comment drafts. A listing
failure's kind is shown as a generic failure for now; Task 10 carries it to
the Drafts page.

Task 6 starts next.
