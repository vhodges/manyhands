---
manyhands_managed: true
manyhands_kind: comment
id: "01M5ZXXFZM32PE9J2WQH2YTKK6"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T00:17:46Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 11: complete

Task 11, links, is complete. Its commits are f660313 and 5bfe75c, with
docs ecbf58c and fa97f6e.

- **What changed.** Links in the read view, in comment bodies and in the
  edit editor open as decision 10 says:
  - `http`, `https` and `mailto` open in the browser;
  - a path or a title opens the item's tab, found in the item's own
    context first, then on primary, then by the nearest name;
  - anything else shows a banner that never names the link's text.

  Backslashes, drive letters, a leading `//` and a `..` past the root
  never reach the filesystem.
- **Rulings.**
  - A link to an existing file with no ID opens its Inspect tab.
  - A `#tag` click is followed as a title link; this is a Known Limit.
- **Review.** Approved. There were no Critical or Important findings,
  and three minor findings were fixed.
- **Tests.**
  - The whole desktop binary, before the fix round: 694 passed, 0
    failed.
  - After the fix round: `links` 24 and `view::item` 96 passed.
- **Checks.** Clippy and fmt are clean.
