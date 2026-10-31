---
manyhands_managed: true
manyhands_kind: comment
id: "01M65BQZ00DQ6FSZ2KRVBA1SEP"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M659P4CFQN8ZCFEXGS77Q8H7"
created_at: "2026-10-30T02:55:37Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 7 complete; the task review is pending

Part B's first task. It adds the Ctrl-C window and the `item sync`, `repo
sync`, `operation resume` and `operation abandon` verbs. Only the CLI and its
tests changed; there is no library change.

- **The window (R-21, R-36).**
  - `interrupt::during` installs a `SIGINT` handler with `SA_RESTART`. On
    Windows it is `SetConsoleCtrlHandler`, which is not compiled here and is
    the owed native check.
  - It wraps `execute` for the six stoppable verbs. The first press cancels
    the call's token, and a second ends the process.
  - One latch serves the whole run. `mutate::run` resets it on entry, and
    C2's retry stops once it is set.
  - `Boundary`, `Plan` and `FakeBoundary` are unchanged.
- **The verbs.**
  - The four verbs are available, with a shared `OPERATION_ID`.
  - `item sync` reads the item's kind first (R-2).
  - `operation abandon` uses C2's confirmed flow.
  - The help goldens were regenerated (E-1).
- **Human output.**
  - A conflict prints a `conflict:` line and the `conflict show` command.
  - `remote_branch_deleted` prints three `next:` lines: request cancel, the
    same request, then item republish.
  - `original_request_required` names the request. It adds the supersede
    note when the conflict read shows that request's resolution with written
    paths.
  - The gate's local-sync data prints its state, reason and next action
    (R-37).
- **The spike (decision 14).** A Ctrl-C during a held fetch of `item sync`
  answers `cancelled`, exit 130, at the next safe point, on all four runs.
  The effects are `publication: pending` and nothing else. Nothing was pushed,
  merged or moved locally. The case pins this answer.
- **Verification.** These pass: `--bin manyhands-cli` (212), `cli` (302),
  `cli_remote -- c4_interruption` (1 SSH case), clippy (all targets and
  features), `check --bin manyhands-cli`, fmt, and the desktop `check --bin
  manyhands` guard. `origin/main` is still `a372edb`, and `merge-tree` is
  clean.
- **Pending.** The task review.
