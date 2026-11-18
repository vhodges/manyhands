---
manyhands_managed: true
manyhands_kind: comment
id: "01M5QRVVG8ZYQ5BTEW6N6Q50Z7"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T20:15:34Z"
---

The product owner ran the design's native checklist on Linux Wayland at
the branch head `9d34e63`. Every step, 1 to 7, passed, using a draft
seeded with `examples/seed_draft.rs`. The Cycle's evidence table now
records this. The Keyboard row is Held, and the CLI-isolation and
regression rows record the full gate at `59d7e0a`.

Still owed: the native CI run of `--test drafts` on Windows and macOS,
which runs after the merge. Merging, the Build workflow, closing the
ticket and removing the worktree each wait for the owner's approval.
