---
manyhands_managed: true
manyhands_kind: comment
id: "01M4CNEFH3XJ32PY0X3F5D733N"
item_id: "01M4CKWWRA1DHFPMWKPNK7CQ1G"
created_at: "2026-10-08T02:28:30Z"
---

Implementation checkpoint: review-ready.

Preflight: fresh fetch observed `origin/main` at `ceb1be4`; rebase of this
branch was a no-op at `dd4a722`; ancestry verified; worktree clean.

Reproduced first. Six new tests in `tests/discovery_rebuild.rs` build two item
worktrees that both contain a third, primary document. Before the fix, refresh,
rebuild and the snapshot read failed with `IndexUnavailable`, "stored item ID
is not globally unique", and a stale copy was attributed to the wrong
worktree. Creating the second context also returned `IndexPending`, because
the save's own index handoff hit the same failure.

Fix:

- `src/repository/discovery.rs`: `scope_to_authoring_item` restricts an
  active context's sources to the item its branch identifies and that item's
  comments, and its source problems to that item's paths.
- `src/repository.rs`: when persisting a context, validation problems for an
  item excluded from it (because the item has its own context) are not
  stored.

Evidence, all through Devenv on Linux:

- The six new tests pass; all 65 tests in `discovery_rebuild` pass.
- `cargo check --all-features --locked`: pass.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo test --all-features --locked`: exit 0; 620 passed and 0 failed
  across the 15 default-harness result lines. The four custom-harness SSH
  suites ran inside the same exit-0 command; their case counts were not
  captured.
- `cargo run --locked --bin manyhands-cli`: exit 0.

No existing test expectation changed. No canonical Markdown or Git state is
changed by the fix. Not verified: Windows and macOS; and whether this also
resolves defect `01M4B4GSW8J262NA8WPBANSRN1` (nested worktree bases) for
active contexts, which was not tested.

Committed on this branch. Not independently reviewed, pushed, merged or
closed.
