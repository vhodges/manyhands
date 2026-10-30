---
slug: vh-d78af
id: 01M67BN4F7M1TYRMTDHB4XNWCM
title: Move C4's exit-evidence cases into their own cli_remote module
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Move C4's three exit-evidence SSH cases out of the interruption module into a
module of their own.

Follow-up from Wave 03 C4 (`01M4CC0VMWEQQ9G4W0DAG7ADPY`): the final review's
CLI recommendation 2 (Task 13's review, minor 7). The owner asked for this
ticket on 2026-10-30. Start it after C4 merges.

## Today

`tests/cli_remote/interruption.rs` holds about 1,600 lines and four
concerns. Three of its cases are exit evidence for the C4 exit table, not
interruption, but they carry `c4_interruption_` names:

- `c4_interruption_transport_refusals`;
- `c4_interruption_index_hand_off_refused`;
- `c4_interruption_poll_holds_the_reservation`.

The module also redeclares constants that `tests/cli_remote/collaboration.rs`
declares (`R1`–`R6`, `C1`, `C2`). Its `FAIL_HAND_OFF` trigger copies the one
in `tests/mutation_remote/resume.rs`.

## Expected behaviour

The three cases live in `tests/cli_remote/exits.rs` as `c4_exits_*`, chained
into `tests/cli_remote.rs`'s case list. They share the constants and helpers
they need instead of copying them.

## Acceptance criteria

- `tests/cli/collaboration_exits.rs`'s `ROWS` name the new cases, and its
  reference check passes.
- The whole `cli_remote` target passes, and so does `--test cli --
  collaboration_exits`.
- No `dead_code` on a Windows build of `cli_remote`: module-level items that
  only `#[cfg(unix)]` code uses are gated.
- Test-only: no `src/` change.
