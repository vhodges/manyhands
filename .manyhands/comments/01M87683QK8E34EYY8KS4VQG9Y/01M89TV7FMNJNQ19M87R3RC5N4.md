---
id: 01M89TV7FMNJNQ19M87R3RC5N4
item_id: 01M87683QK8E34EYY8KS4VQG9Y
created_at: 2026-11-25T17:08:12.481024544Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 0 checkpoint (baseline, references, ledger), commit ca350c4b. Ledger: docs/plans/2026-11-25-wave-06-cycle-03-pull-request-close-execution.md.

Base: origin/main 3df52dcb; the branch was already up to date (merged at 9a73bdbd during planning, never rebased). Cycle 02's pull_request schema, PullRequestDto and setup_pending are not on main, so Task 4 adds the schema and DTO in the shared shape { source, target, forge, url }.

Checks run: cargo clippy --all-targets --all-features --locked -- -D warnings, cargo fmt --check, and cargo test --all-features --locked --lib --no-run at 1040f41a, all clean. No test was run.

References: three read-only research agents re-checked about 450 design references and the named edits E-2 to E-23. None was wrong; two had moved and four were unclear. Each is resolved by a ruling in the ledger or carried to Task 7's design checkpoint.

Sweep: named edits E-24 to E-41 added, mechanical except two. Decision 15 (owner, 2026-11-25): the two tests that pin an open own context hiding the primary's closure are rewritten as design Part A says (E-24 tests/ticket_closure/journeys.rs, E-25 tests/cli_remote/retirement.rs); and R-9 is narrowed so ticket plan keeps each batch's number, leaving closing tickets out as a filter does, so filters_choose_which_tickets_are_returned_and_never_which_are_ready stands.

Also at approval: the Wave 06 document's Cycle 03 checklist was saved on its open branch and published at 681cbe08 (not promoted), and notes were left on vh-c97se, vh-rphcs, vh-ssepw and vh-p7st2. Next: Task 1, the closing state.