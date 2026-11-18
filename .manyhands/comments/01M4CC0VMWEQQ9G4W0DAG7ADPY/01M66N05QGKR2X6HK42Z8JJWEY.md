---
manyhands_managed: true
manyhands_kind: comment
id: "01M66N05QGKR2X6HK42Z8JJWEY"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M66CYGCCJJKJ63D7MZ630KQX"
created_at: "2026-10-30T14:56:38Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 13 complete; the task review is pending

Interruption, lost output and the exit table are now proven through the
binary. Only tests changed, so there is no `src/` change and no `--lib` run.

- **Ctrl-C** (15 new `c4_interruption_` SSH cases beside Task 7's spike).
  The server holds each run's fetch, and the press lands while it waits.
  - `item sync`, `comment add`, `conflict resolve`, `item republish` and
    `operation resume` each answer `cancelled`, exit 130, with their effects
    intact. `request show` agrees, and a new request completes the work:
    an `item sync` or `repo sync`, or a new prepare for a republish, whose
    old confirmation is `confirmation_used`.
  - A second press ends the process by `SIGINT`, read with the new
    `PtyCli::finish_any`. The request stays accepted and holds the
    reservation, so a new request is `busy` until the same request is sent
    again; that request then completes the work.
  - A press at the unlock prompt stays `unlock_required`, exit 3, with the
    window installed. The same request then answers `unlock_failed` under
    the same operation.
  - A press after the last safe point of an `item sync` whose push is
    refused (R-36) answers `push_rejected`, exit 4, with one refused push.
    The latched press stops C2's retry. Without a press, the same request is
    retried: four refused pushes.
- **`request cancel`** from a second invocation stops `comment add`,
  `item sync`, `repo sync` and `item republish` at their next safe point.
- **Lost stdout.** `comment add` and `item sync` exit 1. `request show`
  returns the stored result, and a replay makes no second push, file or
  checkpoint.
- **The exit table (R-27).** `tests/cli/collaboration_exits.rs` matches the
  audit's 48-code union, plus the CLI's own codes (50), by set equality.
  - On Unix, all but three codes are reached by a C4 verb through the
    binary, exits 0 to 5 and 130 included.
  - `poll_yielding`, `remote_unavailable` and `confirmation_expired` carry
    reasons. Off Unix, so do `unlock_failed` and
    `original_request_required`, which only a terminal or a read-only
    directory stages.
  - The `busy` row records that its envelope carries an `operation_id`
    with no request record.
- **Verification.**
  - `cli_remote -- c4_interruption`: 16 SSH cases passed.
  - `cli -- collaboration_exits`: 11 passed.
  - Clippy (all targets and features) and `fmt --check` pass.
  - The whole `cli_remote` and `cli` targets each ran once.
- **For the owner.**
  - After a second press, a new request is `busy` until the interrupted
    request is sent again.
  - The latch's answer is the push's own refusal, since no safe point saw
    the press.
- **Review.** The task review is pending.
