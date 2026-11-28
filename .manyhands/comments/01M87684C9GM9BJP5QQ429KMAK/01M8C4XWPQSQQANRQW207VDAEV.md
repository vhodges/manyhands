---
id: 01M8C4XWPQSQQANRQW207VDAEV
item_id: 01M87684C9GM9BJP5QQ429KMAK
created_at: 2026-11-26T14:42:39.012888019Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-26: Wave 06 Cycle 04 documents at revision 4, drafted and reviewed, awaiting the owner's approval. Nothing was built or run; every claim was checked by reading the source at Cycle 03's branch tip bef571fd (which holds main 6c7937c8).

Base: origin/main 6c7937c8 merged into this ticket branch at 00c043da (merged, not rebased; the branch was at 61e12865 before), pushed. vh-gy8gz (Cycle 03) added as a dependency at 83297a99 (decision 5): Task 0 starts only after Cycle 03 merges, and re-grounds every Cycle 03 name then.

Documents (managed, each on its own branch, published with item sync): Cycle document 01M8BXXFPDGS3A5MJBM26JEEX3 (docs/Cycles/wave-06-cycle-04-pull-request-promote.md, revision 4 at c2b5d5db); design 01M8BXXGDN9HPD8BM9SJTHHVB2 (docs/plans/2026-11-26-wave-06-cycle-04-pull-request-promote-design.md, at 6ca7aa60); implementation plan 01M8BXXH54RS9M6NX5XW9ARX9A (docs/plans/2026-11-26-wave-06-cycle-04-pull-request-promote-implementation.md, at 0f8f8c03).

Owner decisions: 1 a mode of the promotion lifecycle; 2 promoting again synchronizes, then already_applied with the same link; 3 the link ends by the content proof; 4 item sync gives a document's outstanding link; 5 Cycle 03 is a dependency; 6 no new SSH case, Cycle 03's case extended; 7 the outline; 8 managed documents; 9 a cheaper check in document show; 10 history_unknown does not block the pull-request prepare; 11 remote_branch_deleted's way out is item republish; 12 repo inspect's link listing goes to Cycle 06; 13 retirement is fenced while a pull-request promotion is unfinished; 14 a stopped promote that changed since consent ends as external_change before its child publishes, and finishes once it has.

Reviews: round 1 on revision 1 (source and feasibility 1 Blocking, 11 Important, 7 Minor; contract and consistency 2 Blocking, 15 Important, 16 Minor); round 2 on revision 2 (scoped 0/6/10; adversarial 2/7/9); round 3 on revision 3 (scoped 2/3/7; adversarial 3/4/6). Round 3's blockers were again in decisions 13 and 14's mechanisms, so revision 4 states them as required-outcome tables (RR1 to RR8, F1 to F4) and makes Task 4 test-first with a design checkpoint that chooses the mechanism. Revision 4 has not been independently reviewed.

Open for the owner: approval of the three documents; the named test edits E-1 to E-12; the execution method (proposed: subagent-driven, Sonnet 5.5 at high effort as the implementer); authorization to implement once Cycle 03 has merged.