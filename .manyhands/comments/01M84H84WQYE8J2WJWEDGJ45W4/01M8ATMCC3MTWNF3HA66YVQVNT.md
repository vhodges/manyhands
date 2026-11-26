---
id: 01M8ATMCC3MTWNF3HA66YVQVNT
item_id: 01M84H84WQYE8J2WJWEDGJ45W4
created_at: 2026-11-26T02:23:27.321177704Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Task 12, 2026-11-25.

Tasks 9 to 12 are done on top of origin/main 3df52dcb (main has not moved). The ticket branch is at 8bf16e2a; the execution ledger (docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md) has the detail.

Task 9 (0744044d, fix 573236d8): remote select takes integration and forge; a direct-to-direct change keeps today's commit (a forge change in the same commit); any change into or within pull-request mode goes through a setup branch. Review fixes: the direct path refuses a primary that turned pull-request mode under the lease; a stale first call can never record a second setup (fixed for repo enable too).

Task 10 (c5926c84, fix 029b43d8): repo setup-cancel, the new confirmed verb (show-then-wait), and repo remove finishing an accepted setup request as cancelled on both its paths (decision 16). Review fixes: a checked-out setup branch is kept; a cancel of an unregistered root is already_applied.

Task 11 (23b0fb57, fixes 8d4458f3, 6bb255d3, 60403154): the desktop polls a pending setup and its row reads Waiting for setup to merge or Setup not published yet. Review fixes: a change setup is polled without an SSH current remote, and never repolled in a tight loop.

Task 12 (b4cecf0b, fix 34e4ad2b): the one SSH case, a_setup_branch_is_published_activated_changed_and_cancelled, about 4.5 s, with five SSH goldens; the server's primary and setup ref are pinned after every setup operation.

Tests run, each red first and by exact name: the new library, desktop and integration tests of Tasks 9 to 11, the SSH case singly (mutation_remote, by exact name), the golden registration tests, and the unchanged selection, enable and desktop scheduler tests the plan names. No fast tier and no whole SSH target yet (review-ready).

Next: Task 13 (contracts, audit and skills), then Task 14 (gates and review-ready).