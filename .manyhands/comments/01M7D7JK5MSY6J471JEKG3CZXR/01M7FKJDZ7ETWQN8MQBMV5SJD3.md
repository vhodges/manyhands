---
id: 01M7FKJDZ7ETWQN8MQBMV5SJD3
item_id: 01M7D7JK5MSY6J471JEKG3CZXR
created_at: 2026-11-15T12:40:33.021614383Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-15.

**Base.**
- Cycle 05a's branch (`2c398b03`: Cycle 04's branch and 05a's Tasks 0–3) merged into this ticket branch at `ac50ba1a`, as the ticket's first planning step allows; main merged again at `aaf6b475` once Cycle 04 landed (`14ca0938`). Every update was a merge, never a rebase.
- Planning only: nothing was built and no test was run.

**Documents** on `manyhands/ticket/01M7D7JK5MSY6J471JEKG3CZXR`, revision 4 at `40143b35`, pushed:

- `docs/Cycles/wave-04-cycle-05b-close-and-resolve.md`
- `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-design.md`
- `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-implementation.md` (10 tasks, 0–9)

**Execution waits for Cycle 05a** (`01M7D5Z7FZTGTC1HE25KQ1MYQY`) to merge to main (the ticket's Entry Gate). Task 0 then merges main and re-maps every name taken from 05a's design.

**Owner decisions** (Cycle document):

- **1–5:** leave out the data the library lacks and file a follow-up; the ready list after the close only; Resolve conflict as a right-side sheet; the design's button order, action last; the D5 follow-ups' copy items only.
- **6:** the outline ("Write the documents").
- **11–12,** after review round 1: further test edits pre-authorized as in 05a; the ellipsis rule on Open Sharing & keys…, Open repository details… and Check host…, everywhere.
- **7–10** are the planner's proposals, awaiting confirmation: stage titles, the Waiting stage and every footer; "Closed by {name} · today"; Resolve conflict keeping today's rules where a sheet would differ; field names, kinds and states.

**Follow-up ticket** (decision 1): `01M7FBRBN04QACSCZV6KX0WMR3` (`vh-nkp8c`), library data for line counts, conflict authors and times, and the merge base; created and published at `ecb44079`.

**Review rounds.**

| Round | Reviews, at | Blocking | Important | Minor | Applied in |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility; contract and consistency (`702e569f`) | 3 | 22 | 17 | revision 2 (`874bb66b`) |
| 2 | scoped re-review (`874bb66b`); adversarial on the sheet and the dialog's stages (`aaf6b475`) | 2 | 8 | 17 | revision 3 (`3964af7c`) |
| 3 | scoped re-review (`3964af7c`) | 0 | 2 | 7 | revision 4 (`40143b35`), **not independently reviewed** |

**Where focus goes** around the Resolve sheet is the design's table B3 (H1–H16), read after a draw. Task 6 writes it test-first under a design checkpoint.

**Open.** U-1 (05a's merged names; revision 4 read 05a's branch at `d7b39f62`), U-6 (the named edits), U-7 (the dialog-only guards and focus moves) and U-9 (05a's own waiting openers) are checked in Task 0; U-10 (Repair locally's waiting sheet) in Task 5.

**Awaiting the owner:**

- approval of the three documents and the named test edits E-1 to E-15;
- confirmation of decisions 7–10;
- the execution method;
- authorization to implement, once 05a has merged.

**Proposed execution.** Subagent-driven and serial, as in Cycles 02 to 05a, from a handoff brief, started after 05a merges.
