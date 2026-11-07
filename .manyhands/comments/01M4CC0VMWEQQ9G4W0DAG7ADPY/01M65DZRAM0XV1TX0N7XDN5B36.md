---
manyhands_managed: true
manyhands_kind: comment
id: "01M65DZRAM0XV1TX0N7XDN5B36"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M65BQZ00DQ6FSZ2KRVBA1SEP"
created_at: "2026-10-30T03:34:50Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 8 complete; the task review is pending

Part B's second task. It adds the `comment add` verb. Only the CLI, the input
schema and fixture, and tests changed; there is no library change, so no
`--lib` target ran.

- **The verb.**
  - `comment add` is available, with `--repo`, `--id` and a required
    `--input` of `comment_id`, `body`, `parent_id` and `created_by`.
  - It reads the item's kind with C3's shared read (R-2), and parses the two
    IDs in the input with one reader, `authoring::ulid`. A malformed one is
    `invalid_input`. C3's `item_id` parses now call the same reader.
  - It runs through Task 7's driver, with the bounded retry. The Ctrl-C window
    was already wired for it.
  - `--prepare` is `not_confirmable`. The help goldens were regenerated (E-1).
- **Human output.** The publication note is a `note:` on stderr when the
  result carries a checkpointed comment. A deleted remote branch prints
  Task 7's three `next:` lines: cancel the request, send it again, then
  `item republish --prepare`. The second line is the generic "send request
  again with its original command and input", because the retry needs the
  caller's input. It is unit-tested from a constructed envelope. Task 12a
  covers it end to end.
- **Verification.** The tests were written first and failed. Then: the
  `manyhands-cli` bin 220 passed, `--test cli` 317 passed, and the
  `input_schemas` filter 7 passed. Clippy (all targets and features),
  `fmt --check`, `check --bin manyhands-cli` and the desktop `check` pass.
- **Review.** The task review is pending. The remote rows of the comment table
  (publication, the refused push, the conflict, the Ctrl-C rows) are Task 12a's.
