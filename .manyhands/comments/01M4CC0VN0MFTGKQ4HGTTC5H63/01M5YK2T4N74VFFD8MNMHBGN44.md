---
manyhands_managed: true
manyhands_kind: comment
id: "01M5YK2T4N74VFFD8MNMHBGN44"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T11:49:12Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 5a: The Layout And The Item's Scroll Region

Commit 1bcc366. Tests run (desktop-test-support, bin manyhands): view::item 36 passed (the five new frame tests and opening_an_item_writes_nothing among them), view::scroll 2 passed, view::tabs 29 passed, view::settings 93 passed, keyboard_tests 23 passed. check --features desktop, clippy --features desktop --bin manyhands, clippy all-targets/all-features and fmt --check are clean. Outside the gate, view::relationships (8) and view::drafts (41) also pass.

An item tab is now the tab strip, a header placeholder with D1's identity line, and the item's own scroll region holding the page and then the discussion. The page keys move that region; Settings keep D1's column. The region has the Kit's scrollbar, and the discussion's header row has Back to top. E-25 is applied to both scroll tests, which now scroll the item's region, with Back to top under a long body as the control below the fold. Every element ID from Task 0's sweep stays. D1's repository selection panel is still drawn below the item tab, because a tabs test reads it while a tab is open.
