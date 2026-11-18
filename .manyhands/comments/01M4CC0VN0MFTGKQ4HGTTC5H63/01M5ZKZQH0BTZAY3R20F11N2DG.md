---
manyhands_managed: true
manyhands_kind: comment
id: "01M5ZKZQH0BTZAY3R20F11N2DG"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T21:24:14Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 9: The Editor Host

Commits: `a8e3cd8` (feat: add the edit editor's host), then the ledger
row and this comment.

`EditorHost` in `view/editor/host.rs` is the item tab's edit editor, an
editor of its own (R-62). It holds the baseline, switches Rich text and
Markdown on the same editor, and feeds each text difference as `Edited`
and `DirtyChanged` (R-12). It has `mark_saved` (R-69), `rebase` through
`set_exact` (R-18, R-79) and `reload_clean` (R-19). Escape after a
pointer press dispatches the editor's `Dismiss`, and otherwise it
focuses Done (R-59). The caret steps over math and property blocks in
rich text (R-66) and is kept in view (R-67). Its first production caller
is Task 14.

R-67's caret reveal is now one shared helper, `CaretInView`. The page's
read editor and the comment bodies use it, and their tests pass
unedited.

Tests, once each at the end (desktop-test-support):
- `editor`: 52 passed;
- `keys`: 28 passed;
- `keyboard_tests`: 23 passed;
- `view::item`: 96 passed;
- the whole desktop binary: 650 passed, 0 failed (19 new).

Both clippy runs, the CLI isolation script and `cargo fmt --check` are
clean. No pre-existing or earlier D3 test changed. No ruling was needed.
