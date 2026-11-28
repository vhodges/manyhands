---
id: 01M8H438E8DD48TZHJH0A5ZME5
item_id: 01M876851FT3NPSH9KDXKXR1AJ
created_at: 2026-11-28T13:04:18.258458079Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-28: revision 4 of the Cycle's three documents is ready for the owner's approval. Not approved; nothing is implemented, and no build or test was run.

Base: origin/main 15ea8eb5 merged into this ticket branch at 0213d32d (merged, not rebased; pushed). Both dependencies, vh-gy8gz (Cycle 03) and vh-rphcs (Cycle 04), are closed and merged.

Documents (managed documents, each on its own branch, published with item sync): Cycle document 01M8G1CX05SK9MY5D9YP67B0RP (docs/Cycles/wave-06-cycle-05-after-the-merge.md, at 9a2502b5); design 01M8G1CXKNG56HZ4NFMKNAE3G7 (docs/plans/2026-11-27-wave-06-cycle-05-after-the-merge-design.md, at 481de52d); implementation plan 01M8G1CY7XE733CHN0XEF52J78 (docs/plans/2026-11-27-wave-06-cycle-05-after-the-merge-implementation.md, at 4c226e73).

Owner's decisions (Cycle document): 1, one new confirmed verb, context cleanup; 2, merged_at at every Manyhands fast-forward of the local primary; 3, a document's manual cleanup needs the primary to hold a document with that ID; 4, a surviving setup branch out of scope; 5, the outline and its names, as one Cycle; 6, managed documents; 7, context cleanup is a cleanup-only path of the close and promote lifecycles; 8, a ticket's closed needs the content proof in both modes; 9 and 11, a stuck cleanup can be abandoned when no live run holds it; 10, a closed ticket's link ends; 12, a configuration fence ends a cleanup's reservation as supersession does; 13, abandon only until the delete was observed; 14, the cleanup is refused while any setup is pending.

Review rounds: 1, on revision 1, source and feasibility 2 Blocking, 8 Important, 13 Minor, and contract and consistency 2 Blocking, 17 Important, 11 Minor; 2, on revision 2, scoped 0/9/8 and adversarial 2/11/7; 3, on revision 3, scoped 0/9/9 and adversarial 2/5/6. Every round's blockers after the first were in the same mechanism, context cleanup's stops, abandon and configuration fence. As the planning skill directs after a third such round, revision 4 states that mechanism as required outcomes (design CC1 to CC31) and leaves the rest to Task 7's test-first design checkpoint, which stops for the owner on named triggers. Revision 4 has not been independently reviewed.

Checks: every code name the documents cite was grepped in the ticket worktree, the load-bearing lines were spot-checked, and the documents' cross-references (rulings R-1 to R-44, CC1 to CC31, known limits K-1 to K-21, named edits E-1 to E-23) were checked for consistency. No cargo command was run.

Open questions: none beyond the approval asks.

Proposed execution: subagent-driven, implementers on Sonnet 5.5 at high effort, Task 7's design checkpoint and the final review on the most capable model.