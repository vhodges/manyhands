---
manyhands_managed: true
manyhands_kind: comment
id: "01M5YTASAVX32T0X7W4NDG3KJ3"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M5XHWVN84RM75AR5MEP1NA0P"
created_at: "2026-10-27T13:55:53Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Re-grounded on the merged gate, ready for owner review again

- **Base.** The gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` merged and closed
  at `a372edb`. The ticket branch merged that main as `e920043`, without
  rebasing, and pushed it.
- **Re-check.** Three read-only reviewers re-checked every design fact and
  citation at `e920043`. They found 26 findings: 3 blocking, 11 important
  and 12 minor. They overlap on two findings, E-6 and `incompatible_closures`.
  - Facts 1, 3, 8–11, 19, 25, 30, 33, 49, 53, 64, 67, 72, 75 and 76 changed.
  - Facts 80–86 are new.
  - Every other citation was corrected.
- **Owner decisions.** Two, taken on 2026-10-27:
  - 19: the local verbs stay with C5;
  - 20: decision 18's supersede stays for remote, and the gate's RFC
    sentences are scoped to the local operation.
- **New rulings and edits.** Rulings R-37 to R-41, and named edit E-12.
  - The association table is renamed `remote_resolution_owners`, so it
    passes `tests/mutation_replay.rs:400`.
  - Rows for an unfinished local sync are added.
  - `incompatible_closures` is reported as external-only.
- **Main is red on one test.**
  `registry_creates_repository_and_discovery_metadata_schema` fails on
  `a372edb`, because its table list lacks the gate's tables. I confirmed it
  by running that one test, the only Cargo run in planning. The fix belongs
  on main, outside C4. Task 0 and Task 4a stop on it if it is still red.
- **Unreviewed.** The re-grounding edits had no independent review, and
  neither had the third round's edits.
- **Implementation.** Not authorized. Approval, the named edits E-1 to E-12,
  the execution method and authorization are asked for together.
