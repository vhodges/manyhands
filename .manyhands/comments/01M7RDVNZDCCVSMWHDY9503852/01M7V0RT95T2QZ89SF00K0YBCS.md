---
id: 01M7V0RT95T2QZ89SF00K0YBCS
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-19T23:03:50.11369038Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning checkpoint 2, 2026-11-19. Design revised after review round 1 (three independent reviewers on `4e126a0b`: 8 blocking, 29 important, 9 minor findings). The revision is `47f43280`, published; a scoped re-review including an adversarial pass is running. Nothing has been built or run.

**Correction to checkpoint 1** (`01M7TZGEBQMXNGF24H4DXMXDFN`): the CLI's automatic retry on `discovery_pending` applies to `ticket close` and `document promote` (`request.retry`, `bind/lifecycle.rs:582`) and to folder create and poll; saves, creates and `comment add` offer `operation.resume` and are not retried (`command.rs:566`, pinned by `a_hand_off_no_operation_owes_is_offered_as_a_refresh`).

**Owner's answers** (asked in the planning chat, each answered with the recommended option):

1. The ticket's Acceptance is amended: no recovery behaviour changes, and the index gives today's results except that a change made outside Manyhands to another item is seen at the next refresh, poll or desktop start, or flagged stale when read. Saved to the ticket in `26e3a01c`, with the targets.

2. `discovery: current` on a write meaning "the contexts that write changed are indexed" is a clarification with no schema version change.

3. Registry open audits the schema and the unfinished rows only; a completed record is audited when a read uses it.

4. A per-item index step holds only its item (and whole-repository operations), and a whole-repository refresh completes a stale per-item row's committed index step (ruling R-3).

**Stated as defaults before those questions, no objection from the owner:** the pre-commit safety checks that read other items (duplicate ID, branch checked out elsewhere) stay as an explicit exception to decision 2, made cheap only if measured above about 100 ms; the CLI keeps retrying close and promote when their index step is pending (R-7), which replaces the earlier "don't retry" item that rested on the wrong reading above.

Next: apply the re-review's findings, record these answers as decisions in the design, then present the design for approval. The implementation plan follows approval.