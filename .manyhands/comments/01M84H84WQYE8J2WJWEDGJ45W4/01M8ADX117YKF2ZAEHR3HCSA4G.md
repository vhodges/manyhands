---
id: 01M8ADX117YKF2ZAEHR3HCSA4G
item_id: 01M84H84WQYE8J2WJWEDGJ45W4
created_at: 2026-11-25T22:40:59.043530154Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Task 8, 2026-11-25.

Tasks 7 and 8 are done on top of origin/main 3df52dcb (main has not moved). The ticket branch is at 1c78dd60; the execution ledger (docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md) has the detail.

Task 7 (12dffa95, fix a42d9f03): repo enable takes publication_remote, integration and forge; a direct enable writes a given remote and forge in its initialization commit; a pull-request enable registers the repository as setup pending, records the setup and publishes it, with the request lifecycle of design Part K (re-entry through the record, every stop after recording owed, a domain-finished request answered from its stored envelope). Design checkpoint passed. Review fix: a racing call of the same request can no longer delete a recorded request.

Task 8 (59704ab6, fixes 4d1d4e23 and a93f71b7): poll once hands a setup on another remote to the setup poll, which activates the repository when the setup has merged; completion is level-triggered (the gate, the end of every poll, the setup poll). Design checkpoint passed. Review fixes: no double setup poll on a re-entered retiring request; a crashed or abandoned finish no longer strands a setup or blocks writes; activation leaves the polling state as a direct enable plus remote selection would.

Decisions taken without stopping (all in the ledger): two enables of one root at once give the second busy, then setup_pending on retry (owner: "that's fine, keep going"); a setup's host stop offers host approve with the authority from the transport error; pull_request.schema.json is nullable at the top level, identical to Cycle 03's file.

Tests run, each red first and by exact name: the new library tests of Tasks 7 and 8 (about 30), tests/cli/repository.rs a_pull_request_enable_without_a_key_stops_with_no_selected_key, three tests/repository_enablement.rs tests, tests/cli/input_schemas.rs repo_create_refuses_the_enable_only_fields, the unchanged enable, poll and retirement tests the plan names, and the golden registration tests. No fast tier and no SSH target yet (review-ready).

Next: Task 9, setting changes through remote select (test-first, with a design checkpoint).