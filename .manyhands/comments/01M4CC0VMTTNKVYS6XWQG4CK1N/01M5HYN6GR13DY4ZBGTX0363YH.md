---
manyhands_managed: true
manyhands_kind: comment
id: "01M5HYN6GR13DY4ZBGTX0363YH"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-22T14:01:21Z"
---

## Task 2 Completion Checkpoint

Implemented Task 2 only from `c448c249d48a9f4cfacfbb259311e43697c07dc4` in this
ticket's branch/worktree. Fetch/merge of origin/main was a clean no-op. Task 0
and Task 1 independent approvals and pushes were preserved. No subagents were
dispatched.

Added the controlling-terminal interfaces, shared bounded/zeroizing line editor,
Unix termios and Windows console implementations, fake terminal, and startup
mode validation before transport/service. Both platform readers set reading
mode before the prompt and use restoration guards. Cargo.toml adds only the
Windows Console feature; the lockfile and library boundary are unchanged.

TDD evidence through `devenv shell -- cargo test --locked --bin manyhands-cli`:

- `terminal` RED: **nine executed failures** at the unimplemented key handler.
- `startup` RED: **four failures, 14 passes**, proving the missing terminal
  ordering and mode-rule early refusal.
- `terminal` GREEN: **12 passed**, 86 filtered, zero failures/ignored.
- `startup` GREEN: **18 passed**, 80 filtered, zero failures/ignored.
- The GREEN filters cover **27 distinct tests**, including **15 new tests**;
  three startup test names also match `terminal`.
- Targeted `devenv shell -- cargo clippy --locked --bin manyhands-cli -- -D warnings`
  passed. `devenv shell -- cargo fmt --check` passed after formatting.

Only the named Task 2 checks ran; Cargo's single-filter restriction was handled
with two separate commands. The deferred Task 1 minor is covered: `--confirm`
without a request ID is accepted only in interactive human mode, across all
eight terminal/JSON/noninteractive combinations.

Self-review checked secret-buffer lifetimes, erase-before-truncate zeroization,
masked echo, mode-before-prompt ordering, guard restoration, ownership, unsafe
safety comments and startup's read-before-clap guard. Existing assertions were
preserved; FakeStartup gained only the authorized mechanical extension.
The old startup-consumed allowances are removed; two narrow Terminal method
allowances remain until Task 3 callers. No library gap was found.

Windows is **149 lines** and source-inspected against locked windows-sys API
signatures. Its compilation remains R-16's authorized post-merge native check;
this task claims no Windows runtime or live-terminal evidence.

The [execution ledger](../../../docs/plans/2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md)
records exact base, commands/counts, changed files, decisions and allowance
removal checkpoints. `git diff --check` and the merge-tree check passed.
This ID came from the existing built CLI in an isolated temporary home/data
environment with no controlling terminal.

Delivery subject: `feat: read the controlling terminal and decide the interactive mode`.
**Task 2 is ready for controller review.** The controller owns the subsequent
push. Task 3 has not started; the ticket remains open.
