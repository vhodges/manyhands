---
id: 01M8BXC9XR4NGZXGB7EZPY6T59
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-26T12:31:01.062098662Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Task 9 (the new branch), 2026-11-26.

Task 9 is done on branch manyhands/ticket/01M87683QK8E34EYY8KS4VQG9Y, which is at 84fff042 with the execution ledger. A pull-request close now goes through a new branch holding only the closure when the ticket's remote branch is gone or the ticket has no context. When an old context remains, it is retired first, under the close's own remote reservation (U-1, by generalizing finish_parent, so no owner stop was needed). The design checkpoint was written by an Opus designer, checked by an independent Opus checker and approved with corrections; it is the ledger's Appendix F.

Slices: A, prepare and consent (c1507173, fixes 249ee1ee and a2b0d224); B, a new branch with nothing to retire (2f888965, fix 38e871e6); C, the retirement and U-1 (bef571fd, fix 6e86eef9). Each had its own task review and a scoped re-review of every fix round.

Main moved: Wave 06 Cycle 02 landed. It is merged at bd7286fd (parents 38e871e6 and main 6c7937c8). An item sync meant to publish 38e871e6 merged main itself, stopped on ten code conflicts and held other items' publication for about 15 minutes. I finished that merge in place, as the CLI contract says, and the same request then published it. A read-only research agent mapped the merge (Appendix G). Four semantic conflicts in files Git had merged cleanly were fixed in the merge commit, and an Opus review of the remerge diff approved it.

Rulings 50 to 58 are in the ledger, each with its cost if wrong. The ones you may want to check: 50 and 51/51a (what counts as evidence that the remote branch was deleted: a fetch's record, or a close's own push finding; a deletion only a plain push saw needs a poll first); 52 and 54 (merging main mid-task and finishing the CLI-started merge in place); 56 (I ran the pull_request library module whole once as a guard, 83 passed); 57 (a replayed reservation goes on to step 3).

Cycle 02's note for whichever Cycle lands second is reconciled: the ten conflicts and the count of 69 are done in the merge. ticket withdraw-closure taking SetupGate::Refused and joining change.rs's race list is carried into Task 11. Closure steps already count as in the way, since in_the_way counts every unfinished ticket_closures row; Task 11 adds rows proving it for withdraw and new-branch records.

Tests run so far: each slice's new tests by exact name, red first or red by deliberate mutation; the named guards; the pull_request library module once (83 passed); the merge map's tests by exact name; static gates on every commit. The fast tier and the SSH tier have not run yet; they run once at review-ready.

Next: Task 10.