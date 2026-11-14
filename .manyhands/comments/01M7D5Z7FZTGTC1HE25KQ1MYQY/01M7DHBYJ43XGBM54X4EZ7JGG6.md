---
id: 01M7DHBYJ43XGBM54X4EZ7JGG6
item_id: 01M7D5Z7FZTGTC1HE25KQ1MYQY
created_at: 2026-11-14T17:23:34.467913137Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-14.

**Base.**
- The ticket was created from origin/main `ea6d899b` and published at `0698f556`. Main has not moved since, so nothing was merged. Every update was a commit on the ticket branch, never a rebase.
- Planning only: nothing was built and no test was run.
- Cycle 05 was split on the Wave's split line (decision 2). 05b is `01M7D7JK5MSY6J471JEKG3CZXR`, which records the Wave's "Check first" of D5's two surfaces.

**Documents** on `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY`, revision 3 at `cab2aac4` (plus `ce3a758f`, two stale numbers), pushed:

- `docs/Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md`
- `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-design.md`
- `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-implementation.md` (11 tasks, 0–10)

**Execution waits for Cycle 04** (`01M7AMAPNHHTNECXBF4CA5WPKV`) to merge to main (decision 1). Task 0 then merges main and re-maps every name taken from Cycle 04's design.

**Owner decisions** (Cycle document):

- **1–5:** plan now; split into 05a and 05b; the remote select sends today's selection after the add; no inline error for a wrong passphrase; the outline ("Write the documents").
- **15–18,** after review round 1: button order kept, except the Confirmation dialog and Unlock key; typed text kept on close, cleared after a successful add; the configured remote as default, else Keep on this computer; Open its settings for a registered folder.
- **6–14, 19, 20** are the planner's proposals, awaiting confirmation.

**Review rounds.**

| Round | Reviews, at | Blocking | Important | Minor | Applied in |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility; contract and consistency (`9e22c765`) | 3 | 28 | 34 | revision 2 (`67fb6fee`) |
| 2 | scoped re-review; adversarial on the add, share and focus mechanisms (`67fb6fee`) | 0 | 12 | 25 | revision 3 (`cab2aac4`), **not independently reviewed** |

**Where focus goes** around the Add sheet is the design's table B9 (G1–G23), read after a draw. Task 4 writes it test-first under a design checkpoint, as Cycle 04 does for its A6.

**Open.** U-1 (Cycle 04's merged names) and U-3, U-4, U-8 are checked in Task 0; U-2 on screen; U-7 in Task 1.

**Awaiting the owner:**

- approval of the three documents and the named test edits E-1 to E-16;
- confirmation of decisions 6–14, 19 and 20;
- the execution method;
- authorization to implement, once Cycle 04 has merged.

**Proposed execution.** Subagent-driven and serial, as in Cycles 02 to 04, from a handoff brief, started after Cycle 04 merges.
