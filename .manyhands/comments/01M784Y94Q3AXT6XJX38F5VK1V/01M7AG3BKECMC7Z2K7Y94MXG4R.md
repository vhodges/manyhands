---
id: 01M7AG3BKECMC7Z2K7Y94MXG4R
item_id: 01M784Y94Q3AXT6XJX38F5VK1V
created_at: 2026-11-13T13:03:40.997181686Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-13.

**Base.**
- **Dates.** Planning began on 2026-11-12 and ended on 2026-11-13.
- **Cycle 03a's work.** The owner asked to plan while Cycle 03a was in its final review (decision 4).
  The ticket branch merged 03a's published head `3086666f` at `b37d2c2b`.
- **Cycle 03a merged.** 03a then closed on main at `ba95e1b7`. The branch merged main at `81789331`,
  and revision 5 re-grounds its citations there.
- Every update was a merge, never a rebase.
- Planning only: nothing was built and no test was run.

**Documents** on `manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`, revision 5 at `90dca2a0`, pushed:

- `docs/Cycles/wave-04-cycle-03b-activity-and-needs-attention.md`
- `docs/plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-design.md`
- `docs/plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-implementation.md`
  (14 tasks, 0–13)

**Owner decisions** (Cycle document):

- **1–3** were made during Cycle 03a's planning.
- **4–12** were answered on 2026-11-12:
  - plan against 03a;
  - states leave, events stay;
  - the count is the popover's rows;
  - checks that changed something;
  - a row menu;
  - Dismiss and "Clear finished";
  - problem items, one row each;
  - Enter presses the button;
  - titles allowed.
- **13:** the outline's calls, accepted.
- **21–24** were answered on 2026-11-13, after round 1:
  - D6's title rule;
  - one-line rows, amending D6's decision 13;
  - focus returns to the row;
  - at most one action per row.
- **25** was answered on 2026-11-13, after round 3: a fixed failure stops counting by itself.
- **14–20** are the planner's proposals, awaiting confirmation.

**Review rounds.**

| Round | Reviews, at | Blocking | Important | Minor | Applied in |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility; contract and consistency (`74c9aabb`) | 4 | 39 | 25 | revision 2 (`b830e8e3`) |
| 2 | scoped re-review; adversarial (`b830e8e3`) | 2 | 19 | 22 | revision 3 (`4c596645`) |
| 3 | scoped re-review; adversarial on the tables (`4c596645`) | 6 | 13 | 12 | revision 4 (`05246630`) |
| 4 | scoped re-review with re-grounding; adversarial on the count (`05246630`, at `81789331`) | 6 | 11 | 11 | revision 5 (`90dca2a0`), **not independently reviewed** |

**The attention count.** Four rounds found Blocking issues in this mechanism, so its contract is now a
set of tables:

- the N-table, with its precedence and its per-attempt latches;
- the log's T-table.

Task 5 writes both test-first, with a design checkpoint. Retirements were redesigned in revision 4
(R-51).

**Open.** U-1, U-2, U-4, U-5, U-7 and U-8 are checked by the tasks that first rely on them.

**Awaiting the owner:**

- approval of the three documents and the named test edits E-1 to E-16;
- confirmation of decisions 14–20;
- the execution method;
- authorization to implement.

**Proposed execution.** Subagent-driven and serial, as in Cycles 02 and 03a: one implementer per
task, then a fresh reviewer, one at a time, following `.agents/skills/implementing-a-cycle/SKILL.md`.
It would run from a handoff brief if a fresh session executes.
