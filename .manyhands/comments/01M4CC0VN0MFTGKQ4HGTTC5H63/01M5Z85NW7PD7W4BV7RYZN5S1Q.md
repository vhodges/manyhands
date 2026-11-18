---
manyhands_managed: true
manyhands_kind: comment
id: "01M5Z85NW7PD7W4BV7RYZN5S1Q"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T17:57:46Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 7: The Read View

Commits: `d839db7` (code and tests), then the ledger row and this comment.

An item tab now shows its body in a read editor on the white page. D1's
source textarea stays for Inspect tabs only, which the details sheet's
Inspect link opens (R-64). The editor takes its style from the page tokens
(`page_style`, all 25 fields), keeps its caret in view by itself (R-67),
and keeps its cursor across a clean reload (R-19). A click below the text
puts the caret at the end, and no click focuses the page itself.

Tests, each run once at the end with `--features desktop-test-support
--bin manyhands`: `view::item` 79 passed, `view::tabs` 29, `read_only` 16,
`keyboard_tests` 23. The whole desktop binary, run once: 605 passed, 0
failed. Both clippy runs and fmt are clean.

E-1 to E-5 are applied. There are 16 new tests in
`view/item/page_tests.rs`: the brief's 14, a real table drag (Review
Focus 5) and a Control-click on math that opens no math menu. No earlier
D3 test changed. No ruling was given.
