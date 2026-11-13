---
id: 01M4VMJE8B5XZAF6KRT7YYRSBY
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-13T22:01:48Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Controller conveys spec review `ses_ec8bd9b3effeS3OD0g7achFQNy`: BLOCKED P1
before quality. Old tracked directories skipped by the incoming-path preflight
can contain ignored descendants that obstruct a directory-to-file/symlink
checkout after unrelated paths were already changed. Poll-only correction
authorized, shared helper bodies and approved agreement unchanged.

Clean fix base `e08d42d8cfdbc1aa5798c30a94d554d4b7f175e9`, assigned worktree/
branch verified. Meaningful local-repo regression first: tracked dir/child and
afile, clean status with ignored dir/local.ignored, incoming descendant replaces
dir and changes afile. Full snapshot comparison precedes the Dirty assertion.

RED: `devenv shell -- cargo test --locked --lib remote::poll::tests::poll_retiring_directory_ignored_descendants`,
exit 101, **0 passed, 1 failed, 547 filtered**. Checkout changed afile and deleted
dir/child while HEAD/index/refs stayed old and the ignored sentinel survived;
returned RecoveryRequired. This proves the preservation violation before any
production correction. Initial shadowed-TreeBuilder borrow compile trial was
corrected; no fake transport/sleep/global Git mutation. Nested, symlink and clean
control variants will execute after the fix.

Ticket open, spec correction in progress, quality not started. Cumulative review
base `961b3c21db11f797e5bd1cdac0bf85d748cd1dec`; deferred Task 2 SQL kind-
immutability minor retained for final review. No subagents or later-task work.
