---
id: 01M89NJNAGHPGC0RF3E8EW384C
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-25T15:35:53.202558031Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-25 (planning-a-cycle).

Base: origin/main 3df52dcb merged into the ticket branch at 9a73bdbd (merged, not rebased). Drafts, all status draft, revision 5 at ad0d5fca: docs/Cycles/wave-06-cycle-03-pull-request-close.md, docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-design.md and docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-implementation.md. No build or test was run; the drafts were checked by reading the source.

Owner decisions (the Cycle document's Product-Owner Decisions): 1 merged first: a new branch, as the RFC; 2 two clones: keep the published closure; 3 a new data type for the pull-request close; 4 already_closing is success, exit 0; 5 the pull_request schema and DTO are Cycle 02's, reused; 6 a surviving remote branch closes in place; 7 a ticket with no context closes through the new branch; 8 a stopped close is handled as the direct close's; 9 closing tickets left out of ticket ready, blocked, plan and critical-path; 10 closed_by stays out of the answer; 11 departures accepted (withdraw of nothing is already_applied, one SSH case proves the primary is never pushed, closing derived in both modes with two relationship tests rewritten); 12 item sync gives the link; 13 the pull-request close, withdraw and new branch are modes of the existing ticket-closure lifecycle.

Review rounds: 1 on revision 1 (source 5/14/15, contract 5/15/19 Blocking/Important/Minor); 2 on revision 2 (scoped 2/15/26, adversarial 6/10/6); 3 on revision 3 (scoped 1/6/13, adversarial 5/7/7), whose blockers were again in the separate runner and led to decision 13; 4 on revision 4 (source 1/11/15, adversarial 3/9/7), resolved in revision 5, which has not been independently reviewed. Round 4's remaining adaptations of the lifecycle are listed as the design's Design Checkpoint Obligations for plan Tasks 7 to 9.

Open questions for approval: the named test edits E-2 to E-23 (E-16 approved in decision 11) and Task 0's sweep; the new verb's delivery label C5 and its autonomy column; whether revision 5 needs a fifth review round; the execution method.

Proposed execution method: subagent-driven, one implementer and one reviewer per task, with design checkpoints at Tasks 7, 8, 9 and 11. The plan has 17 tasks (0 to 16), one SSH case, and runs every SSH-harness target once at review-ready.