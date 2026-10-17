---
manyhands_managed: true
manyhands_kind: comment
id: "01M4CQNB7YG0PWSX6S25F1W9MR"
item_id: "01M4CKWWRA1DHFPMWKPNK7CQ1G"
created_at: "2026-10-08T03:07:12Z"
---

Second review checkpoint: the product owner asked for a re-review of the
branch including `d82e05e`. It returned ten findings.

Changed in this commit:

- The effective-copy rule is applied once, in `observe_root`, instead of when
  rows are persisted. `src/repository.rs` now differs from main only by
  removing the old exclusion set; refresh and rebuild no longer each build
  their own copy of it.
- The path heuristic from the previous commit is removed. It suppressed
  primary problems by matching the worktree copy's path, which could hide a
  problem in an unrelated primary file after a rename and miss the real stale
  copy. Primary now leaves alone only an active item's ticket and comment
  directories. An unparseable primary document stays a primary problem; the
  ticket's expected behavior says so.
- A document worktree no longer keeps problems from scanning the rest of
  `docs/`. The previous commit kept directory-level ones using a file-system
  check that followed symbolic links and could differ between two
  observations, and reported one fault once per document worktree.
- The ticket and comment directory paths are defined once and used by both
  the worktree collector and the primary filter.
- Two comments that overstated behavior are corrected.

Not changed, because the behavior predates this ticket:

- A rejected active context discards its own detailed problems and reports
  one generic context problem. That includes a document worktree whose
  document lies beyond the `docs/` traversal limits, or which holds two
  sources with its item's ID.
- The set of active items is taken from the first observation, including a
  context that then proves unstable and is not persisted that round.

Still open for the product owner, unchanged from the previous checkpoint: a
ticket worktree does not scan `docs/` for a document reusing its ID; stray or
malformed files that are not a worktree's own item are silent until merge;
comments are matched by directory.

Evidence through Devenv on Linux: nine tests for this ticket; all 67 tests in
`discovery_rebuild` pass; `cargo fmt --check` and
`cargo clippy --all-targets --all-features --locked -- -D warnings` pass;
`cargo test --all-features --locked` exits 0 with 622 passed and 0 failed
across 15 standard binaries and 15, 35, 31 and 103 SSH cases passed;
`cargo run --locked --bin manyhands-cli` exits 0.

Committed on this branch. Not pushed, merged or closed.
