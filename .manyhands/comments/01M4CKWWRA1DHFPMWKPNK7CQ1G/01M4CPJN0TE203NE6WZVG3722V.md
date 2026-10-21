---
manyhands_managed: true
manyhands_kind: comment
id: "01M4CPJN0TE203NE6WZVG3722V"
item_id: "01M4CKWWRA1DHFPMWKPNK7CQ1G"
created_at: "2026-10-08T02:48:15Z"
---

Review checkpoint: the product owner asked for a code review of `75515d3`.
It returned eight findings. Correction to the previous comment: that commit
added seven tests, not six.

Fixed in this commit:

- Primary still reported a malformed copy of a document that has its own
  context, because the filter only knew paths of items that parsed. The
  exclusion now carries each item's effective path.
- Primary still stored source-level problems (for example a symbolic link)
  for an excluded item's files. Both problem kinds now use one rule.
- Scoping after a whole-checkout scan dropped the directory-level and
  traversal-limit problems that explain why an item's own files were not
  read, and read and parsed every file in every worktree on each refresh. An
  item worktree now collects only its own ticket directory or its document,
  and its own comment directory.
- The ownership rule existed twice in different shapes. It is now the single
  `discovery::item_owns_path`.
- A document worktree holding a second source with its item's ID is reported:
  the context is rejected with a context problem.

Left as is, for the product owner to accept or change:

- A ticket worktree no longer scans `docs/`, so a hand-made document there
  that reuses the ticket's ID is not reported until the branch merges and
  primary validates it. Scanning `docs/` in every ticket worktree to catch
  this would restore the per-worktree cost just removed.
- A stray or malformed managed file that is not the worktree's own item is
  neither listed nor reported until merge. This follows from the decision
  that a worktree contributes only its own item.
- Comments are scoped by their directory, `comments/<id>/`, not by parsing
  `item_id`. A file there naming another item is reported as a problem in
  this context; a comment for this item stored outside that directory is not
  read.

Evidence through Devenv on Linux: two tests added, nine in total for this
ticket; all 67 tests in `discovery_rebuild` pass; `cargo fmt --check` and
`cargo clippy --all-targets --all-features --locked -- -D warnings` pass;
`cargo test --all-features --locked` exits 0 with 622 passed and 0 failed
across 15 standard binaries and 15, 35, 31 and 103 SSH cases passed in the
four custom-harness suites; `cargo run --locked --bin manyhands-cli` exits 0.
`cargo check --all-features --locked` was not rerun separately after this
change; clippy and the test build compiled the same code.

Committed on this branch. The fixes themselves have not been re-reviewed.
Not pushed, merged or closed.
