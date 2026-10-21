---
manyhands_managed: true
manyhands_kind: comment
id: "01M57ANREZ3M8BV190AGS0SAKQ"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DJ"
created_at: "2026-10-18T10:59:44Z"
---

Task 9 checkpoint: the Cycle's one full verification is done, and the branch
is awaiting final review. The details are in the
[execution ledger](../../../docs/plans/2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md),
Task 9.

- **Full suite.** 3,317 passed, 0 failed, 1 ignored (the existing opt-in
  characterization):
  - 1,189 library;
  - 1,497 standard integration;
  - 620 SSH cases, including 72 in `ticket_closure`;
  - 11 doctests.

  The first run took about 90 minutes and had one failure:
  `ssh_transport` `exact_delete_scope_and_policy` still asserted the
  delete scope from before named edit 5. The owner approved named edit 6
  (2026-10-18). Fix round 1 (`7e57cce`) limits that case's refusal loop to
  the primary and adds `exact_ticket_context_delete_is_in_scope`. The
  rerun of `ssh_transport` passed all 123 cases. No production code
  changed.
- **Checks.** check, fmt, Clippy (`-D warnings`) and the CLI smoke pass.
  The desktop smoke was skipped, because the change is domain-only.
- **Main.** `origin/main` is `3765524`, already contained.
  `git merge-tree --write-tree HEAD origin/main` is clean.
- **Documents.**
  - The design is amended to match the rulings: R-T2-1/2/3, R-T4-1/3,
    R-T5-3, R-T6-1, R-T6-6, R-T8-1, R-T8-2 and R-T9-1. It also gains a Known
    Limitations section (R-T6B-1, R-T6B-2, R-T8-1).
  - The Cycle 09 "unchanged cleanup" claims carry correction notes.
  - The Wave 03 API audit records `ticket close` as delivered headless.
  - Named edits 5 and 6 are in the plan.
  - Every Cycle acceptance row maps to named passing tests.
- **Still open.**
  - The controller's whole-branch reviews.
  - Owner confirmation of the R-T6-1 and R-T8-1 amendments.
  - Push, pull request, merge, closure and cleanup each need their own
    authorization. Native CI runs on main after merge.
