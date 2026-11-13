---
manyhands_managed: true
manyhands_kind: comment
id: "01M5Z3NVQF0N3K1YC5XEJJVW2G"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T16:39:13Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 6: The Details Sheet While Viewing

Commits: `05e38a4` (feat: open the details sheet from the header's
chips), and this record.

Status: blocked. Three pre-existing tests need a ruling. None of them
is edited.

- `view/item/header_tests.rs::every_chip_emits_open_details_for_its_field`
  (from Task 5b). The first chip's click opens the sheet, and the Kit's
  backdrop then takes the next chip's click (fact 61).
- `view/item/header_tests.rs::tab_order_runs_strip_header_chips_page`
  (from Task 5b). `copy-id` is no longer the page's first control, since
  E-23 moves it into the sheet.
- `view/tickets_tests.rs::arrows_move_in_the_list_and_enter_opens`, with
  E-30 applied as written. Escape gives focus back to All details, so the
  arrows that follow no longer reach the list.

A one-line edit fixes each:
- an Escape after each chip's click;
- "the page holds focus" (`item-page`) in place of `copy-id`;
- `secondary-2` after E-30's Escape.

Each was tried on a scratch copy (12 and 9 passed) and then reverted.

Tests, desktop-test-support `--bin manyhands`, each run once at the end:
- `sheet`: 14 passed;
- `view::item`: 60 passed, 2 failed (the two above);
- `view::relationships`: 8 passed;
- `view::scroll`: 2 passed;
- `keyboard_tests`: 23 passed;
- `view::find`: 8 passed;
- `view::tickets`: 8 passed, 1 failed (the one above);
- `view::documents`: 4 passed.

The whole desktop binary, run once for the stop rule: 585 passed and 3
failed, the same three. Desktop clippy, all-features clippy and fmt are
clean.

E-23, E-26 and E-30 are applied exactly as named. No other pre-existing
test line changed.

The sheet is the Kit's, 440 px wide. It opens from each chip and from All
details, with that chip's field focused. It holds D1's details, the four
relationship groups (R-54) and the Inspect link, and everything in it is
read only. Escape, Done and the close button give focus back to the chip.
The sheet closes with its tab, and no workspace key acts behind it
(R-60).
