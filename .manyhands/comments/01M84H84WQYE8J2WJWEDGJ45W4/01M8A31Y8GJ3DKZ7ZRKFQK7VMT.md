---
id: 01M8A31Y8GJ3DKZ7ZRKFQK7VMT
item_id: 01M84H84WQYE8J2WJWEDGJ45W4
created_at: 2026-11-25T19:31:25.74290299Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Library checkpoint after Task 6, 2026-11-25.

Tasks 1 to 6 are done, each implemented by Sonnet 5.5 and reviewed by a fresh reviewer (Opus for the gate, the channel and publishing), on top of origin/main 3df52dcb (main has not moved). The ticket branch is at 252c7401; the execution ledger (docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md) has the detail.

Done: the three result codes (c6a03c3b); the setup record and its migration (7e8931a6); the read shapes, with a new problem code setup_unreadable so an unreadable setup record degrades repo inspect and repo list instead of refusing them (ffa2c1dc, 15e42060); the setup gate, with repo list reading a complete setup from the registration (6fe9f6ec, 7fa26b07); the setup channel in the transport, its Configured path reviewed as unchanged (ccd0035d); publishing a setup branch under the setup lock, with the refusal ending and finish_accepted (f72c306b, 88aa36d0). Task 6's design checkpoint was checked against design Part D, S1 to S12 and K4 before implementation, and passed.

Review fix rounds: Task 3 (unreadable record), Task 4 (repo list's completeness check), Task 6 (a crash window on the lost-commit path, and an ending that must roll back if K4's guard refuses a still-accepted request). Each fix had a scoped re-review; all findings addressed.

Tests run, all by exact name, each red first: the new library tests of each task (about 35), the named edits' tests (E-1 to E-3, E-5 to E-7, E-11, E-14, E-16), the four repository goldens regenerated (only "setup": null added), and unchanged mutation_replay, recovery_foundation_gate and not_enabled tests. No fast tier and no SSH target yet; they run once at review-ready, with the four document_promotion cases that reach the shared guard_delete run singly (it was refactored with no behaviour change).

Next: Task 7, repo enable in pull-request mode (test-first, with a design checkpoint).