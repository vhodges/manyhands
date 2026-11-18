---
manyhands_managed: true
manyhands_kind: comment
id: "01M66CYGCCJJKJ63D7MZ630KQX"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M66AAXFRS1Y9WQKZ4QTGQX81"
created_at: "2026-10-30T12:35:55Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 12b complete; the task review is pending

The conflict and republish journeys now run through the binary between the
two clones of `CliPair`. Only tests and goldens changed: there is no `src/`
change, so no `--lib` target ran.

- **The cases** (10 `c4_conflicts_` SSH cases, two of them Unix-only):
  - a canonical conflict with two paths, shown in JSON and text, resolved
    with the shown tokens, and then replayed;
  - `request_mismatch`, and a second resolution refused;
  - the stale resolutions: a stale observation, an incomplete set and a
    foreign token, none of which writes;
  - the outside repair of a code file: the resolve refused for outside
    repair, the five repairs that are not accepted, then the person's
    two-parent merge, which `operation resume` publishes;
  - a resolve whose continuation meets a deleted branch: three `next:` lines
    naming the `item sync` that holds the reservation, and the way out
    followed;
  - a hand edit after a resolve stopped with written paths:
    `original_request_required` for a new request, `external_change` with the
    restore note for the original, and the original finishing once the edit
    is undone. A read-only directory stops the resolve, with no test hook;
  - republish: `item sync`'s refusal, the preview, the confirm, `busy` with
    Task 10's note before the retry line, a moved local branch, a transport
    stop continued by the same request, a replay, `request_mismatch`, and a
    Ctrl-C at the interactive consent prompt.
- **The peer.** Clone B deletes the item's branch with its own Git push.
- **New goldens:** `conflict_show_ok`, `conflict_show_text` (the owner's
  label check), `conflict_resolve_ok`, `item_republish_preview` and
  `item_republish_ok`.
- **Verification.**
  - `cli_remote -- c4_conflicts`: 10 SSH cases passed.
  - The whole `cli_remote` target: 50 SSH cases passed.
  - Clippy (all targets and features), `fmt --check` and the golden
    registration test pass.
- **For the owner.**
  - A text code file in a conflict reads `structure: binary`, and an add/add
    reads `delete`. The library's own table pins the first.
  - The resolve stopped by the file system answers `internal_error`, with no
    recovery action.
- **Review.** The task review is pending.
