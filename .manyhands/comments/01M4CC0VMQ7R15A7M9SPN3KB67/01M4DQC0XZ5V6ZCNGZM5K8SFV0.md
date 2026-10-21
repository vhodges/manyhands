---
manyhands_managed: true
manyhands_kind: comment
id: "01M4DQC0XZ5V6ZCNGZM5K8SFV0"
item_id: "01M4CC0VMQ7R15A7M9SPN3KB67"
created_at: "2026-10-08T12:21:21Z"
---

Task 5 checkpoint (item lists and complete reads): complete at `bcbe2e6`,
range `dffa110..bcbe2e6`.

Two independent reviews, neither with a blocker and neither able to read
outside the repository. The first found three contract-level issues: a
migrated index reported `never_refreshed` instead of `stale`; an item the
index held twice failed the read; a listed nonconforming path could not be
read by path. The second found that a real error from an item's worktree copy
was hidden behind the primary copy. All are fixed. The last two fix commits,
`f7c3146` and `bcbe2e6`, were not independently re-reviewed.

Evidence through Devenv on Linux at `bcbe2e6`, each command exit 0:
`cargo check --all-features --locked`; `cargo fmt --check`;
`cargo clippy --all-targets --all-features --locked -- -D warnings`;
`cargo test --all-features --locked --no-fail-fast` with 815 passed and 0
failed, plus 15, 35, 31 and 103 SSH cases passed;
`cargo run --locked --bin manyhands-cli`. This is the first full-suite run
since the baseline.

Authority change: on 2026-10-08 the product owner authorized pushing this
ticket branch to origin; it is pushed and kept current. A same-day trial of
running cargo on remote sprites was abandoned and nothing remote remains.
Pull request, merge, closure and worktree cleanup remain unauthorized.

Carried to the Task 10 handoff as separate defect tickets: nothing writes an
`accessibility` other than `accessible`; refresh persists the root, contexts
and cleanup in separate transactions, so the index can hold an item twice
(reads now tolerate it); a committed file in an item worktree is stored as
`uncommitted`.

Accepted limits and rulings are in the execution ledger under "Task 5
rulings". Next: Task 6, comment reads.
