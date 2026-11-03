---
manyhands_managed: true
manyhands_kind: comment
id: "01M5YK2T4N74VFFD8MNMHBGN44"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T11:49:12Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 5a: The Layout And The Item's Scroll Region

Commits 1bcc366, 9fe4c0b and the review fixes in "fix: address the Task 5a review". Tests run after the fixes (desktop-test-support, bin manyhands): view::item 36 passed (the five frame tests and opening_an_item_writes_nothing among them), view::scroll 2, view::tabs 29, view::settings 93, view::workspace 25, view::drafts 41, keyboard_tests 23, and view::relationships 8 outside the gate. check --features desktop, clippy --features desktop --bin manyhands, clippy all-targets/all-features and fmt --check are clean.

An item tab fills the content column. It holds the tab strip, then a header placeholder with D1's identity line, then the item's own scroll region. The region holds the page, then the discussion, then D1's details of the selected repository. The page keys move that region, and Settings, the Drafts page and the empty pane keep D1's column. The region has the Kit's scrollbar, with a light thumb at rest, hovered and dragged, and the discussion's header row has Back to top. In a short window the item tab keeps room for the strip and the header, and the column scrolls.

Ruling (owner, 2026-10-27, revised after the review): the repository details end the item's own scroll region, after the discussion, "for now", and the owner checks it on screen (native checklist step 1). A fixed panel below the tab had left the region about 58 px tall in a 700 px window; it is now 270.5 px. Also by the owner: layout needs no tests, so the details' place, the region's height, the extra frame after a resize and the short window are not tested.

E-25 is applied to both scroll tests, which now scroll the item's region, with Back to top under a long body as the control below the fold. Every element ID from Task 0's sweep stays.
