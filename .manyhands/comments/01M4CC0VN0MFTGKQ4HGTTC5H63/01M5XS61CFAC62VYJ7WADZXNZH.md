---
manyhands_managed: true
manyhands_kind: comment
id: "01M5XS61CFAC62VYJ7WADZXNZH"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T04:16:35Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 3: Dependencies, editor bindings, feasibility, the read-only guard and load normalization

Part A's first task, in `0889477`. Both stop points passed, so nothing
waits on the owner.

- **Dependencies.** The `desktop` feature now brings the editor, its
  Markdown crate (no features) and the image decoder. The lockfile gained
  two lines in Manyhands' own dependency list and no new package; there is
  still one `gpui-pre`. The CLI check now also refuses the editor, Markdown
  and image crates.
- **Keys.** Startup and the test shell bind the editor's keys, without
  underline (it writes HTML) and without Escape's Dismiss, so Escape
  reaches the bindings around the editor.
- **Feasibility.** A real editor in a headless window takes typed text,
  with one change event, and Ctrl-Z undoes it.
- **The read-only guard.** All eleven of the plan's guard tests pass,
  over a real, focused, painted editor: typing, Enter, Backspace, Delete,
  paste, cut, the formatting keys, undo, an input-method commit and
  composition, a checkbox click, and right and middle presses change
  nothing. Select All, Copy, the arrows, Tab out and a link click work.
  The caret steps over math and property blocks.
- **Load normalization.** The design's 20 golden fixtures are in
  `tests/fixtures/editor/`. The hand-written expected files match the
  pinned editor's own rewrite. `normalized` also sorts a ticket's
  dependencies by ID (ruling P-1).
- **Tests**, all passing: `editor` 20, `read_only` 12, `keyboard_tests`
  23. The CLI check, the desktop check and clippy, the editor example
  check, fmt, and clippy over all targets and features are clean.
- **For later tasks.** The read editor uses a stand-in rich-text style
  from the Kit theme until Task 7 brings the page tokens.
