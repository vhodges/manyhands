---
manyhands_managed: true
manyhands_kind: comment
id: "01M60SDCENGTWRKE6AJBEAFMGR"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-28T08:18:19Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 14: complete

Task 14, Edit, provisioning and drafts, is complete. Its commits are
0656cf3 and bbb7943, with docs a289bdc, 4d3500f, 703c80c, b8e2cff and
this commit.

- **What changed.** Edit is real.
  - It prepares the item's private copy and reads the item again.
  - It opens the edit editor with its toolbar, and moves focus into it.
  - The first change begins the item's draft, and later changes update
    it.
  - `e` and Ctrl/Cmd-Shift-E start Edit while viewing.
  - A stored draft makes Edit read Continue editing.
  - A pending index hand-off waits in Loading for Resume.
  - A refusal shows in a banner, and nothing is made.
- **Rulings.**
  - Nine of D3's own tests changed with Edit leaving the later actions.
  - A typed `e` in the read view is the Edit shortcut. It never acts in
    a text field, a dialog or the editor.
  - A pending index answer keeps the tab in Loading.
- **Review.** Approved, with no Critical or Important findings. Five of
  the seven minor findings were fixed, and two are notes for later
  tasks.
- **Tests.**
  - The whole desktop binary: 768 passed, 0 failed.
  - After the fix round: `edit` 122 and `view::item` 124 passed.
  - The 200 KiB smoke test went from 193 s to 156 s.
- **Checks.** Clippy and fmt are clean.
