---
manyhands_managed: true
manyhands_kind: comment
id: "01M5YTYMWAPEXNB4J47AXAZ363"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M5YTASAVX32T0X7W4NDG3KJ3"
created_at: "2026-10-27T14:06:44Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Decision 21: C4 fixes main's red registry test

- **Decision (owner, 2026-10-27).** Named edit E-12 adds the gate's eight
  `local_context_sync_*` tables, as well as C4's `remote_resolution_owners`,
  to `registry_creates_repository_and_discovery_metadata_schema`'s exact
  table list. No separate fix on main is made.
- **Evidence.** The failed run on `a372edb` differs from the expected list
  by exactly those eight tables.
- **Effect on the plan.** Task 0 records the one failure as expected, rather
  than stopping. Task 4a applies E-12, and its run of
  `--test repository_enablement` must pass.
- **Still pending.** The owner reviews the documents later. Approval of the
  documents and of E-1 to E-12, the execution method and authorization are
  all still open.
