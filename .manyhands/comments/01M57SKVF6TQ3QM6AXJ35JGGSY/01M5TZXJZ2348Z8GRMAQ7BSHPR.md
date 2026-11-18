---
id: 01M5TZXJZ2348Z8GRMAQ7BSHPR
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T02:16:35Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint: Task 0 baseline and inventory

Approval/planning/start commit `51a0e96da11f8bc1f2f0689e72c88ed99470c96a` is
published on the ticket branch. This checkpoint is the commit titled
`docs: record closure recovery baseline`. Durable evidence is in
`docs/plans/2026-10-25-closure-recovery-gate-execution.md`: source Facts 1–24,
function visibility, complete reference-hit file inventory, named test selectors,
sibling scope, baseline results and all 25 required recovery rows with test owners.

Fetched/merged main at start: before/after
`0cbeab084d252e3823d317a468eee1f29c92c8ce`; main
`0464c42dae19a6dde000c7e7660e82762a41fada`. Checkpoint fetch/merge again found
main unchanged; before/after `51a0e96da11f8bc1f2f0689e72c88ed99470c96a`.
Ancestry and merge-tree passed. No PR/dependencies/newer decision/implementation
was found. Cleanup sibling remains open at c5f1fb9 with only its ticket change.

Executed through Devenv: fmt/check/clippy all passed; targeted all_recorded_ticket_sides
test **1 passed**, child_fence **4 passed**, conflicting_advance_after_the_checkpoint
test **1 passed**, exact ticket_closure true_race_distinct_identities_needs_external_recovery
**1 SSH case passed**. Zero failures and no zero-selection evidence. Initial
check hit the 120s tool timeout compiling dependencies; a longer-limit retry
passed. Full exact commands/counts/durations are in the ledger. Full suite and
native CI were not run.

Baseline proves existing behavior only: E1 still asserts eligible classification,
E3 still uses manual fixture merging. New partial checkout recovery, reciprocal
admission, cancellation acknowledgement, wire read stability and local conflict
reconstruction are not executed facts; Tasks 4–7 must establish them test-first.
No production, tests or schemas changed in Task 0.

## Question for the owner

The exhaustive inventory found two exact test sites outside the named lists.
Task 0 requires stopping rather than inferring broader test-edit approval.

1. Approve adding `tests/support/repair.rs::repair_data` to E9 for an explicit
   new-local-DTO refusal arm, retaining every old success/rejection assertion?
   **Recommended: approve this seventh helper.** Its current exhaustive match
   cannot compile after the required MutationDataDto variant is added. Otherwise
   implementation remains paused for an alternate approved boundary.
2. Confirm E4 also covers (or explicitly add)
   `src/repository/read/status_tests.rs::a_local_action_is_one_the_recovery_store_writes`,
   updating its first-13-local-actions split alongside E4's named every_action
   test? **Recommended: approve this companion inventory edit.** Otherwise the
   approved new action cannot satisfy both old split assertions unchanged.

Ticket is waiting before Task 1, with green baseline and source evidence recorded.
Answer in chat or a canonical reply with parent_id
`01M5TZXJZ2348Z8GRMAQ7BSHPR`. Controller independently reviews this Task 0 checkpoint.
