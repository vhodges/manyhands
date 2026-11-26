---
id: 01M89K4ZSS0ZT3PATATJGCD9KG
item_id: 01M84H84WQYE8J2WJWEDGJ45W4
created_at: 2026-11-25T14:53:28.012905964Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint, 2026-11-25 (Wave 06 Cycle 02).

Base: origin/main 3df52dcb merged into the ticket branch at da032ffb (merged, not rebased; HEAD before 4272ddc9). Nothing was built or run: planning was reading and review only.

The three planning documents are Manyhands managed documents on their own branches (owner decision 9), published with item sync:

- Cycle document 01M89CHP8JBKETDQSZGBPVRTET, docs/Cycles/wave-06-cycle-02-pull-request-onboarding.md, at 54b8e928 (revision 4).
- Design 01M89CHPWV2W6RY0XART0416Q3, docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-design.md, at 5d06880b (revision 4).
- Implementation plan 01M89CHQHEV6NF0X0CTBC1G1AB, docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-implementation.md, at b107ca96 (revision 4).

Owner decisions 1 to 16 (2026-11-25): one Cycle; register first, then resume; remote select gains integration and forge; a new confirmed repo setup-cancel; the desktop's minimum; this Cycle defines the pull_request object; primary_has_local_commits at enable; a remote move in pull-request mode refused; managed documents; extra tests approved; one SSH case, and at review-ready that case, ssh_transport and remote_polling; any SSH remote from direct mode; a new code remote_already_set_up; setup-cancel and remote select with integration or forge are show-then-wait; edits to existing tests approved; repo remove always works on a setup-pending repository, finishing its request.

Review rounds: round 1, source and feasibility 2 Blocking, 10 Important, 14 Minor, and contract and consistency 3 Blocking, 13 Important, 8 Minor (revision 2). Round 2, scoped re-review 1 Blocking, 7 Important, 8 Minor, and adversarial 3 Blocking, 8 Important, 12 Minor (revision 3; the request lifecycle restated as required outcomes, design Part K). Round 3, scoped re-review 0 Blocking, 5 Important, 11 Minor, and adversarial 2 Blocking, 8 Important, 7 Minor, all in the request lifecycle (revision 4). After a third round in one mechanism the lifecycle is not redesigned again: its rules K1 to K7 and outcomes L1 to L14 are the contract, built test-first with design checkpoints in Tasks 6 to 10. Revision 4 has not been independently reviewed.

Open for the owner: approval of the three documents; whether decision 11's review-ready SSH tier (the new case, ssh_transport, remote_polling) stands now that the gate in the mutation boundary and the poll's completion check are disclosed (the plan keeps it; mutation_remote would add about 49 minutes); the execution method; authorization to implement.

Proposed execution: subagent-driven, one implementer and a fresh reviewer per task, with the design checkpoints in Tasks 6 to 10.