---
id: 01M8AMBZ8AHVV8T67QDKPZ5JBA
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-26T00:34:00.114506421Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Task 7, the library checkpoint (ledger commit e23caf3b). Tasks 5, 6 and 7 are done and reviewed: c38f5adc already_closing, the preview effects and the blocker; beda6cab and b13a53c0 the primary's closure kept by a ticket context's synchronization and the two-clone settlement; and Task 7, the pull-request close as branch records of the ticket-closure lifecycle, in five slices (c7ff5cd5..726cf89e).

Task 7's design checkpoint was written before any code, checked three times by an independent reviewer against the design and the source, and approved; it is Appendix D of the ledger. Each slice was built test-first and reviewed (Opus), and each took one or two fix rounds. The pull-request close now records its closure on the ticket's branch, publishes the branch through a publication child in the record's second slot, answers from the design's answer table (stored in answer_code, so a retry or replay answers the same), never touches or synchronizes the primary, and holds its ticket against every other writer while unfinished. Direct closes are unchanged except one ruled change: a poll no longer retires a direct close's context while that close is unfinished.

Tests run, each new one red first and then green, by exact name: the closure record, observe, checkpoint and binding tests named in the ledger, and the repository::closure::run_tests::pull_request module's D1 to D21 rows and answer table, with the direct close's and promotion's run tests as guards. Static gates clean at each commit. Not run yet: the fast tier and the SSH tier (review-ready).

Rulings since Task 4 (all in the ledger): 11 to 13 (Task 6 and the slicing), 14 to 31 (Task 7). Ruling 19 lets item resolve-history pass a pull-request close's hold, so a close stopped on unresolved history can be cleared; the owner was told and did not overturn it. Ruling 21 corrects the design's D20: operation list lists a close's live child on its own row, as for a direct close today, to be recorded in the CLI contract at Task 15.

Next: Task 8, withdraw.