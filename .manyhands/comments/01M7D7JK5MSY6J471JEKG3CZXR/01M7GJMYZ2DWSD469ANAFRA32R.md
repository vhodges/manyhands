---
id: 01M7GJMYZ2DWSD469ANAFRA32R
item_id: 01M7D7JK5MSY6J471JEKG3CZXR
created_at: 2026-11-15T21:43:41.71279675Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Tasks 1 to 4 (the lifecycle dialog), 2026-11-15. Each task: a fresh implementer, then a fresh reviewer; all approved with no Critical or Important findings. Full detail in the execution ledger `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-execution.md`.

**Commits** (on top of Task 0's `81314d73`):
- Task 1 `30aaa015` Close ticket and Resolve conflict copy (Part C constants, `field_name`, `choice_of`, `GitMerge` and `CircleX`);
- Task 2 `25335162` lifecycle stages in the frame's header (`Stage`, `stage()`, `header()`, the Git line);
- Task 3 `9a9a6e30` lifecycle step rows and outcome box (one step-row drawer, `MADE_READY_LEAD` retired for "Now ready to start");
- Task 4 `4401609c` lifecycle footers, action last; `5f729d5d` fix: keep Check now working in the lifecycle footer.

**Tests, by task.** Each implementer ran exact names only, writing P1, P4, P2, P3, P13 and P9 first and seeing them fail. At the end of each task the orchestrator ran the plan's module list plus the extra modules Task 0's sweep found, in one `cargo test` per task:
- Task 1: 256 passed;
- Task 2: 52 passed;
- Task 3: 93 passed;
- Task 4: the first run, at `4401609c`, **failed one test** (143 run, 142 passed): `check_now_tests::way_out_check_now`. After the fix `5f729d5d` the same 28 filters passed, 143 of 143.

Static gates (fmt, check, clippy -D warnings) were clean at every task. The fast tier runs once at review-ready (Task 9).

**What the Task 4 failure was.** The blocked prepare's footer holds five buttons (about 730 px) and the 580 px dialog leaves 538 px, so the frame's right-aligned row spilled past the dialog's left edge: Close was hidden and Check now's middle sat outside the dialog, so its click hit the backdrop. The implementer's own exact-name runs did not include that test. The fix wraps the lifecycle footer onto a second line, in A7's order with the action last, and changes nothing outside `lifecycle/view.rs` (`frame.rs` is unchanged for every other dialog). The owner's eye is wanted on this in the on-screen checklist: frame 2e shows one line.

**Rulings** (in the ledger):
- **O-5:** the plan's Review Focus input "a failed run offering Retry and Unlock…" cannot occur. D5 deliberately withholds Retry when Unlock… is offered (`lifecycle/actions.rs:192-198`), and A7 says "each when offered". Three P2 rows replace it: Retry alone, a locked key's Unlock… alone, and `unlock_required` with Prepare again.
- **O-6:** P13 cannot assert focus on the origin after closing a ticket, because a closed ticket no longer offers Close ticket…. D5's fallback (focus on the closed ticket's tab) is unchanged; the document's approval does return focus to its action.
- **O-7:** the footer wrap above; the hint gets the test id `frame-hint` in `frame.rs`.

**Test edits.** E-3 removes the two `lifecycle-kind` assertions (named, as the design says). E-4 to E-9, E-11 (Task 1 part) and E-13 as named. Further edits, recorded with their reasons: E-16 (the dead `Heading::branch` field), E-17 (`saved_document` visibility), E-18 (`MADE_READY_LEAD` becomes `READY_TO_START` in one comparison), E-19 (`Walk::reach_the_action` takes the tab index). None removes an assertion apart from E-3.

**What the owner will see changed by design.** Blocked, Held, Saving and no-preview stages no longer name the context branch (A2, K-15).

**Pushed.** This ticket's branch, by `comment add` from the main checkout. No other branch was published.

**Next.** Task 5 (Resolve as a sheet: owner and ways in). Nothing is waiting on the owner; Task 6's design checkpoint will stop for you if an H row cannot be expressed.