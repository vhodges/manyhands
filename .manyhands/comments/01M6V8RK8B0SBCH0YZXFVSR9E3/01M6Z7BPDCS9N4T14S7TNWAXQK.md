---
id: 01M6Z7BPDCS9N4T14S7TNWAXQK
item_id: 01M6V8RK8B0SBCH0YZXFVSR9E3
created_at: 2026-11-09T04:00:41.765208469Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint (2026-11-09T03:59Z). Documents ready for owner approval; nothing is approved, authorized or implemented yet.

**Base.** origin/main `8da9afd4` merged into the ticket branch as `cc2af8d0` (merge, not rebase). Documents at `558bf082`, with the planning boxes ticked at `3df88b06`, all pushed to `manyhands/ticket/01M6V8RK8B0SBCH0YZXFVSR9E3`. No build or test was run during planning.

**Documents.**
- `docs/Cycles/wave-04-cycle-01-workspace-shell.md` (`01M6YCHWDYY8ZCFZ62VX0ES1JM`)
- `docs/plans/2026-11-08-wave-04-cycle-01-workspace-shell-design.md` (`01M6YVNFD5D8C8P15EC464R5PM`)
- `docs/plans/2026-11-08-wave-04-cycle-01-workspace-shell-implementation.md` (`01M6YVNFE6AB0RBGQK72CFJJ4X`)

**Owner decisions (2026-11-08).**
- 1–4: one Cycle; Ctrl B stays Bold in an editor; Needs attention counts everything awaiting the user; Find at the top of the Tickets panel.
- 5–11: proposed and confirmed.
- 12: test runs per AGENTS.md as written.
- 13: a failed layout save is shown.
- 14: a failed pending read counts in Needs attention.
- 15: Activity at 150 px, contents scroll.
- 16: docks close through the Kit, keeping its 29 px bottom strip for now.
- 7, revised: a waiting quit changes no layout.

Later work recorded:
- a footer status line;
- one blocking dialog for quitting with work at risk, replacing Quit now and Quit anyway;
- a watchdog for pending operations;
- hiding the bottom dock completely;
- saving open tabs, the focused pane and the scope.

**Reviews.**
- Round 1 at `bb9b3cc9`. Source review: 3 Blocking, 14 Important, 16 Minor. Contract review: 2 Blocking, 17 Important, 11 Minor.
- Round 2 at `fbca53f4`. Re-review: 1 Blocking, 9 Important, 10 Minor. Adversarial: 4 Blocking, 9 Important, 5 Minor. The take-out-and-rebuild dock mechanism was replaced by decision 16.
- Round 3 at `32b82b9c`. Re-review: 7 Important, 8 Minor. Adversarial: 1 Blocking, 6 Important, 8 Minor. The quit override was removed by decision 7, revised.
- Round 4 at `d035d90b` (re-review): 4 Important, 10 Minor, none Blocking. Its fixes are in `558bf082`, which had no independent review.

**Open.**
- No owner questions remain.
- U-3: the context-menu key's name under Wayland, settled on the native checklist.
- U-4 and U-6 are proved by Task 4b's first tests.

**Proposed execution.** Subagent-driven and serial: one implementer per task, then a fresh reviewer. Tasks depend on each other's interfaces, and review kept finding first-contact traps. Awaiting the owner's choice.