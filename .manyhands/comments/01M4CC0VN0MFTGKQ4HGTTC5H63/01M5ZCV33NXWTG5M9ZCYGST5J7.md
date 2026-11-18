---
manyhands_managed: true
manyhands_kind: comment
id: "01M5ZCV33NXWTG5M9ZCYGST5J7"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T19:19:22Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 8: The Discussion

Commits: `53824f5` (code and tests), then the ledger row and this comment.

The comments below the page are now the design's discussion. Each root
comment has its own card, in the library's order. Every reply sits at
the one reply indent, and a reply deeper than one level says "Reply to
{name}". Each row shows an avatar coloured from the author, the name
without its email, and the time in local time. Each body is in a read
editor of its own, kept by comment ID across reloads. A file that is no
comment shows its path and reason. The header's Comments {n},
Ctrl Shift C and a floating button all jump to the discussion and focus
its heading. The floating button hides once the discussion shows, and
when there are no comments.

Tests, each run once at the end with `--features desktop-test-support
--bin manyhands`: `discussion` 16 passed, `view::item` 88 passed and 2
failed, `view::scroll` 2, `keyboard_tests` 23. The whole desktop binary,
run once: 620 passed, 2 failed. Both clippy runs and fmt are clean.

Blocked on a ruling, as in Task 6: two earlier D3 tests fail, and I have
edited neither.
- `header_tests::tab_order_runs_strip_header_chips_page`: Comments {n}
  is a new tab stop between the strip and the chips.
- `frame_tests::back_to_top_returns_and_focuses_the_page`: in a 700 px
  window the discussion is taller than the region, so End no longer
  shows Back to top.

A proposed edit for each passed on a scratch copy and was then reverted.
E-24 is applied. No ruling was given.
