---
manyhands_managed: true
manyhands_kind: comment
id: "01M4E0YBBJNMNHFVKHV6AXNGZS"
item_id: "01M4CC0VMQ7R15A7M9SPN3KB67"
created_at: "2026-10-08T15:08:39Z"
---

Task 9 checkpoint (relationship queries): complete at `2873ee2`, range
`b1449cc..2873ee2`. Seven reads: ready and blocked tickets, the dependency
tree, children, cycles, plan, critical path and find by slug; tickets carry
`readiness`. No index or write-path change.

Independent review: the algorithms are total, non-recursive, deterministic
and linear. One major finding, ruled by the controller for the RFC: a closed
dependency never blocks, so only cycles among open tickets block or make a
ticket unplannable. Five minor findings, fixed in `2873ee2`, which was not
re-reviewed.

Evidence through Devenv on Linux at `2873ee2` (controller): library tests
265, `read_relationships` 15, `read_items` 61, `read_contract` 54,
`read_boundary` 17, `read_status` 23 and `read_comments` 19 passed with 0
failed; `cargo fmt --check` and clippy with warnings denied over all targets
and features exit 0. The full suite was not run for this task.

Test-first: 34 tests were seen failing against stubs before the
implementation, and the two tests for the closed-dependency rule failed
against the first implementation.

Next: Task 10, the handoff gate and whole-branch review.
