---
manyhands_managed: true
manyhands_kind: comment
id: "01M63KF5XNBWH4ZT0HG4TTF2PS"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-29T10:32:09Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 24: Move and New folder

Commit `d250d2f` adds Move… on a document's sheet and makes New folder
work (E-10).

Move… is offered only while the tab views the document as read, with no
session, draft, save or pending release (R-23), and otherwise says why.
Its dialog checks the destination as it is typed; Move reads the document
again, observes the destination again, and sends `document move` with
both observations. A collision and a stale source are refused and
explained, and the dialog stays; once moved, the dialog closes and the tab
follows the document by its ID. New folder… opens a dialog that explains
a folder stays on this computer until a document is saved in it, makes
it under the selected folder or `docs`, sends no refresh of its own
(R-24), and reads "Folder exists" on a retry.

Tests: the brief's 10 (E-10 replaces `folder_creation_is_visibly_unavailable`)
and 3 more, written first against stubs (0 of 10 passed). Verify runs:
`move_document` 9, `new_folder` 4, `view::documents` 4, `sheet` 31; whole
desktop binary 1006 passed, 0 failed (720 s); both clippy runs and fmt
clean. No library tests; no library file changed.

Found: a move retried after a lost answer is answered `ok`, not
`external_change`; the binding works around `01M4PB80T7` for a committed
source. The ruled `external_change` branch is tested another way. Rulings
applied: the pre-flight words and dialog rules for Move and New folder.
