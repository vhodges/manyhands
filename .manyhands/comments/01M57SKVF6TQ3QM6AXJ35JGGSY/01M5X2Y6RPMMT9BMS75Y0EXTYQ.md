---
id: 01M5X2Y6RPMMT9BMS75Y0EXTYQ
item_id: 01M57SKVF6TQ3QM6AXJ35JGGSY
created_at: 2026-10-26T21:47:49Z
created_by: GPT-6 Astra <noreply@openai.com>
manyhands_managed: true
manyhands_kind: comment
---

## Question for the owner: Task 7 migration-fixture scope

Implementation base: `41bce3eb23ae4e95316f287cd832d38be00164aa`. Controller reports
Task 6 SPEC `ses_e864b2dfbffemfrKBcgnvISO55` and QUALITY
`ses_e8613a466ffe3Dlw50236N05X9` approved with zero findings over
`7283ad2..41bce3e`; latest evidence is 159 local + four shared anchors = 163
passing tests, including 21 actual no-Drop exit assertions. Recorded in the ledger.

Initial Task 7 F2 control acceptance/attempt-handover code and nine request-store
tests are committed as **WIP `ee9e485`**, `wip: stage local request acceptance and
attempt controls`. Nine tests pass; fmt check, production check and
all-target/all-feature clippy pass. The full mutation_replay target reports
**316 passed / 1 failed**.
An exact rerun selects and reproduces the same single failure.

The existing test
`tests/mutation_replay.rs::the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`
drops only the three original request tables, then at line 430 asserts that no
schema object has `request` in its name. The required new
`local_context_sync_request_control` table survives that fixture setup and violates
the assertion. The test also has a private exact three-table additive inventory.
This fixture/inventory is outside the approved E1–E9 edits. Its assertion has
not been changed.

1. **Approve changing only
   `tests/mutation_replay.rs::the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`
   and its private `REQUEST_TABLES` fixture/removal/migration-marker inventory for
   the new control table, preserving ALL existing old-column, unrelated-data and
   privacy assertions?** Recommended: approve this narrow addition so the fixture
   verifies the control table/marker as the exact additional migration effects.
   Otherwise, implementation remains on hold pending the owner's direction.

The controller verified this additional named-edit requirement and instructed that
the fixture remain unchanged without owner approval. This requests no broader
old-test or golden rewrite permission. Answer in chat or a reply comment with
`parent_id: 01M5X2Y6RPMMT9BMS75Y0EXTYQ`.

The ticket is **waiting**, using the working-on-a-ticket status-edit procedure.
The controller directed publishing the WIP code and this separate waiting record
to the ticket branch, with no further code changes or unchanged-tree gate repeats.
Task 7 is incomplete and paused at the explicit additional-old-assertion approval
gate. Executor/conflict/control/image production activation, public bindings and
reads, authoritative cancellation, domain request fences, atomic abandonment,
DTOs/schemas/goldens and all required full-protocol cases remain. No independent
Task 7 review has occurred. This is a WIP checkpoint, not the completion feature
commit; Task 8 has not started. No PR, closure, merge, native CI or cleanup occurs.
