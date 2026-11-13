---
id: 01M7AVYT9PCCTE3R7GRM8DEK1Y
item_id: 01M7AMAPNHHTNECXBF4CA5WPKV
created_at: 2026-11-13T16:30:55.05886799Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-13.

**Base.**
- The ticket was created from origin/main `ba95e1b7` and published at `0b7a8f3d`. Main has not moved since, so nothing was merged. Every update was a commit on the ticket branch, never a rebase.
- Planning only: nothing was built and no test was run.

**Documents** on `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`, revision 4 at `8eb412d2`, pushed:

- `docs/Cycles/wave-04-cycle-04-settings-sheet.md`
- `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-design.md`
- `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-implementation.md` (10 tasks, 0–9)

**Execution waits for Cycle 03b** (`01M784Y94Q3AXT6XJX38F5VK1V`) to merge to main (decision 1). Task 0 then merges main and re-maps every citation against 03b's code.

**Owner decisions** (Cycle document):

- **1:** plan now, execute after 03b.
- **2–5:**
  - the key list in `05`'s rows with today's actions;
  - fold-in only what the move rewrites;
  - no sheet footer;
  - Show in Activity closes Settings.
- **6:** the outline and its constraints ("write the documents").
- **12:** the fold-in record corrected, and two more items taken.
- **13:** a click outside closes Settings.
- **7–11** are the planner's proposals, awaiting confirmation.

**Review rounds.**

| Round | Reviews, at | Blocking | Important | Minor | Applied in |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility; contract and consistency (`65c898a5`) | 2 | 26 | 26 | revision 2 (`2574a277`) |
| 2 | scoped re-review; adversarial on close and focus (`2574a277`) | 1 | 12 | 26 | revision 3 (`58bee94b`) |
| 3 | scoped re-review; adversarial on the focus table (`58bee94b`) | 4 | 8 | 15 | revision 4 (`8eb412d2`), **not independently reviewed** |

**Where focus goes.** Rounds 2 and 3 both found Blocking issues in how focus is kept in the sheet and returned from it. Per the planning skill:

- that contract is now the design's table A6 (F1–F19), read after a draw;
- the mechanism is a sketch;
- A7 lists what any mechanism must survive;
- Task 2 writes the table test-first, with a design checkpoint that stops for the owner if a row cannot be met.

**Open.** U-3 is checked on screen, and U-5 (03b's merged code) in Task 0.

**Awaiting the owner:**

- approval of the three documents and the named test edits E-1 to E-18;
- confirmation of decisions 7–11;
- the execution method;
- authorization to implement, once 03b has merged.

**Proposed execution.** Subagent-driven and serial, as in Cycles 02, 03a and 03b, from a handoff brief, started after 03b merges.
