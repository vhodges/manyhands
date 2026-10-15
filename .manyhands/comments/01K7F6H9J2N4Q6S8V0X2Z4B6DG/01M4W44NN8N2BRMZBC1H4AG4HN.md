---
manyhands_managed: true
manyhands_kind: comment
id: "01M4W44NN8N2BRMZBC1H4AG4HN"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-14T02:33:54Z"
---

Fresh Task 6-only implementer verified the assigned clean worktree/branch at
`ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7` and recorded main ancestry.
Task 0/prior preflight trusted. Controller/user reports corrected Task 5 spec
`ses_ec7f49995ffeVTGZBqRN4rzEoX` passed and quality
`ses_ec7c4bbc3ffensuQh7z1f1Gqul` APPROVE, no findings; ticket/ledger now reflect
those approvals, preserving historical P2 evidence. Task 2 direct-SQL poll-kind
immutability minor remains deferred final. Accepted⇒active-only, sole Task 5
final-validator same-lease exception and D1–D4 unchanged.

Task 6 **blocked before production/test edits** on the atomic-batch interface:
`reservation.rs:471–528` implements `commit_observation_batch` in its own
`state::with_transaction`; `state.rs:5002–5035` commits before returning. The
signature supplies no ordered disposition rows or transaction hook. Task 6/design
require the batch and initial ordered NotReached rows in one transaction. Writing
rows in a separate transaction creates a forbidden crash window; an outer call
cannot join the helper's separate connection. The shared advertisement writer
also has no owner/caller-observed local target payload. reservation.rs is outside
the Task 6 file list.

Requested narrow owner ruling: authorize reservation.rs/focused tests for a
full-poll transaction-composition extension, preserving the existing helper,
observation operation and synchronization behavior, with initial rows written
before the same AfterBatchCommit acknowledgment/commit. No new phase, safe point,
transport seam, scan permission or unapproved table. This is a proposal, not an
accepted amendment; approved documents remain unchanged. Detailed source evidence
is in the execution ledger's dated Task 6 entry.

No Rust command executed; 0 tests run, no RED/GREEN/gate claim. Planned RED
`devenv shell -- cargo test --locked --lib remote::poll` and
`devenv shell -- cargo test --locked --lib remote::state` remain unrun. Docs-only
checkpoint verification uses `git diff --check`,
`git diff --exit-code -- src tests Cargo.toml Cargo.lock devenv.lock` and
`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` after inspecting status/diff/log.
Local checkpoint base is the exact Task 6 base above; head returned after commit.
Task 6 unticked, ticket open; no subagents or Task 7/outward/lifecycle action.
