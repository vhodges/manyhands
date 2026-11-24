---
id: 01M75C6XFPCHYTEC5P2RF9RWSZ
item_id: 01M757DHFAVPEVY4TWQTVKQAMJ
created_at: 2026-11-11T13:20:02.019716052Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint (2026-11-11). Documents drafted and reviewed once; decisions confirmed; review round 2 is next. Nothing is approved, authorized or implemented yet.

**Base.** The ticket branch was created from origin/main `48653d01` (Cycle 01 and Wave 03 D6 both merged) and still holds it. Documents at `f28e3fd6`, published with this comment to `manyhands/ticket/01M757DHFAVPEVY4TWQTVKQAMJ`. No build or test was run during planning.

**Documents.**
- `docs/Cycles/wave-04-cycle-02-tickets-and-documents.md` (`01M75C77FMKEPG4AX1HMB1CVKQ`)
- `docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-design.md` (`01M75C77FPAF6TY24045SBF4AW`)
- `docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-implementation.md` (`01M75C77FRSVW977B3HPJP4P79`)

**Approach.** No library change. Tickets: one `list_tickets` read per load, with lifecycle, readiness, a title-or-code filter and sorting done in the desktop; a Kit `List` with an app row and column header. Documents: the base Kit `Tree` with app rows, and folder expansion kept for the session. One shared 28 px row and plain list states for both. Toolbars in the panel header. Ten tasks.

**Owner decisions (2026-11-11).** The owner asked to go straight to the ticket, design and plan, then confirmed all twenty planner proposals. Among them:
- 2: Find becomes the Tickets filter input, keeping `secondary-p` and open-by-ID; matches hidden by the lifecycle choice are named with Show all.
- 4: no short code shows `—`, with the ID in the tooltip.
- 8: the pencil marks unsaved work; a private working copy is named in the tooltip.
- 10: unreadable files become rows, from the existing `index_status` read.
- 12: no Tickets More button yet.
- 18: a focused toolbar button counts as its panel's region.
- 19: a blocked ticket's reasons move from the row to Why blocked and the tooltip.
- 20: one status colour mapping, the item header's.

**Reviews.**
- Round 1 at `563763e2`, two reviewers. Source and feasibility: 4 Blocking, 15 Important, 14 Minor. Contract and consistency: 4 Blocking, 22 Important, many Minor. Each verified against the source and applied in revision 2 (`4c32325f`).
- Round 2: requested by the owner, starting now on `f28e3fd6`.