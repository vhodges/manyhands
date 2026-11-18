---
manyhands_managed: true
manyhands_kind: comment
id: "01M5HZGZ0FMBRT77B4AT0WFXQY"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-22T14:16:31Z"
---

Implementation started: the baseline and dependency checkpoint passed.

**Execution method.** Subagent-driven, as in D1. Each task gets a fresh
implementer that does not dispatch subagents. An independent task review
follows each task, covering spec compliance, quality and the plan's five
Review Focus failure modes. Findings go back to the same implementer, and
a scoped re-review checks the fix. Part A and Part B end with their plan
checkpoints, an independent Part review and a stop for the product owner.
A final review covers the whole branch. The CLI cannot add comments yet,
so these comments are written as files.

**Base.** `git fetch origin main` observed `origin/main` at `d7962c3`.
The branch head was `a9d7b51`. `d7962c3` was already merged (as
`74fea8d`), so `git merge origin/main` changed nothing and
`git merge-base --is-ancestor origin/main HEAD` holds. No rebase.

**Baseline**, all through Devenv (cargo 1.98.0), at `a9d7b51`:

| Check | Result |
| --- | --- |
| `cargo check --all-features --locked` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | pass |
| `cargo test --all-features --locked` | pass: 3271 passed, 0 failed, 1 ignored |
| `cargo run --locked --bin manyhands-cli` | pass: prints the command catalog and opens no window |

**Audit re-check.**
`git diff --stat d7962c3 origin/main -- src/repository.rs src/repository src/results.rs src/lib.rs src/desktop`
is empty. No cited library signature moved, and no desktop path moved.

**Pre-flight rulings** (small implementation decisions):

- `DraftSession::flush_due` takes no clock argument, as the plan's
  signature has it. It returns the absolute due time,
  `min(last change + 1 s, first unflushed change + 5 s)`, which is the
  design's formula.
- The flush-failure entry, "Recovery copy of <title> not saved", names
  the item by its title as read when the draft began (the base's title).
  It is rendered as `TicketName { slug: None, title }.title_or(id)` and
  falls back to the item ID when there is no base. It never uses the
  draft's title (ruling R25).

Task 1 starts next.
