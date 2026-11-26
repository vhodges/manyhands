---
id: 01M8CDBN1RPEXYR1XJZ5C1HSDB
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-26T17:09:57.945553892Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Task 14, 2026-11-26.

Tasks 13 and 14 are done on branch manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y, now at 9a31bd16 with the execution ledger. Main (6c7937c8) is merged and has not moved. Each task had its own review and a scoped re-review of its fix round.

Task 13, your lifecycles in the library (a630c572, tests only): nine tests with a local bare repository as the forge, merged and pruned with plain git2, and a poll reading a real fetch. They cover A1 to A9: each merge style ending closed and retired, the recording clone reading closing until it fetches, declined then withdrawn then recorded again, comments and saves while provisional, two clones, a dependent waiting for the primary, and both merged-first recoveries. Every test passed first and was then shown red under a deliberate stub. No lifecycle failed for real.

Task 14, the SSH case and the goldens (a8095064, d3dcfa53, fixes aa387e02 and 0da5a9ea): the one SSH case runs close, a repeat close, item sync, withdraw and a re-close against a real server and checks that only the ticket's branch ever reaches it. It also adds the two data schemas and four goldens.

The case found a real defect: a same-request replay of a finished pull-request close answered recovery_required instead of its stored answer (D19), because the replay looked for the commit on the primary. Fixed: a replay now reads the ticket's branch while it stands, otherwise the primary, never whatever is checked out, with a library table test. Two Known Limits come out of it, to be written at Task 15: after the ticket branch is gone a replay only works if the primary holds the commit (a merge commit or fast-forward does, a squash or rebase merge does not), and a replay of an operation resume that finished a branch record still answers recovery_required.

A question for you before close, no rush: a pull-request close or withdraw answer reports scope.branch as main, as every lifecycle answer does, while ticket save, comment add and item sync report the ticket's branch. Keep main for consistency with the other lifecycle verbs, or name the ticket branch, which is where the commit lands?

Tests so far: every new test by exact name, red first or by stub; the SSH case by exact name; the contract and registration tests; static gates on every commit. The fast tier and the SSH tier run once at review-ready.

Next: Task 15 (contracts, skills and the design amendments), then Task 16 (gates, tiers, the final whole-branch review).