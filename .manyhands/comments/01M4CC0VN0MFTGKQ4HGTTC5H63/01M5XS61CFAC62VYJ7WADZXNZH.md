---
manyhands_managed: true
manyhands_kind: comment
id: "01M5XS61CFAC62VYJ7WADZXNZH"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-27T04:16:35Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 3: Dependencies, editor bindings, feasibility, the read-only guard and load normalization

Part A's first task, in `0889477`, with the review's fixes in
`fix: address the Task 3 review`. Both stop points passed, so nothing
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
  The caret steps over math and property blocks, inline formulas
  included, on arrow and word moves and on a click.
- **Load normalization.** The design's golden fixtures are in
  `tests/fixtures/editor/`. The hand-written expected files match the
  pinned editor's own rewrite. `normalized` also sorts a ticket's
  dependencies by ID (ruling P-1).
- **The review** found that the pinned normalization is not idempotent: a
  second load can change a text the first load made (design fact 81). The
  guard put a changed text back with `set_text`, which loads it again, so
  on such a body the revert never stopped and the app froze. The owner
  ruled exact re-entry (R-79): text an editor already holds is never
  loaded again.
  - The guard's revert is now exact. On such a body, the old revert
    spun until the test was killed; the new one settles at once.
  - `set_exact` puts an already loaded text into an editor exactly, for a
    restored draft (Task 18) and the review's result (Task 19).
  - Two new fixtures record fact 81's two shapes, and a test pins that a
    second load changes them. The idempotence test now covers the other
    fixtures only.
  - Five smaller findings are fixed, among them a bounds check on stale
    block ranges. Two are carried to Task 7.
- **Tests**, all passing: `editor` 26, `read_only` 15, `keyboard_tests`
  23. The desktop check and clippy, fmt, clippy over all targets and
  features, and the CLI check are clean.
- **For later tasks.** The read editor uses a stand-in rich-text style
  from the Kit theme until Task 7 brings the page tokens.
