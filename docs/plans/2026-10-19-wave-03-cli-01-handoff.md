# Wave 03 C1: Fresh-Session Handoff

Completion handoff for Wave 03 Cycle C1, "CLI Shell And Inspection". The
current status below supersedes the original planning brief, retained afterward
as dated history rather than instructions to restart completed work.

## Current Completion Status — 2026-10-19

**Tasks 0–8 complete; implemented, review-ready; independent final reviews
approved.** Ticket `01M4CC0VMSZ7BRS740C8VSQ5ZK` remains **open**. Worktree:
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMSZ7BRS740C8VSQ5ZK`,
branch `manyhands/ticket/01M4CC0VMSZ7BRS740C8VSQ5ZK`.

- **Verified/reviewed code HEAD:** `309c9e071994f55eb0b1fa9f44546cc2722d0398`.
- **Verification/documentation checkpoint:**
  `7d45c5013a4eebc49ca804dbc3d847ebf825b8c4` (`docs: mark C1 review-ready`).
  Subsequent approval recording is documentation-only; code is unchanged.
- **Independent reviews, reported by the controller:** whole-branch spec
  re-review PASS at `2b84360a2fca123c37d09dd642b69b2afb368049`; whole-branch
  quality PASS on `309c9e0`, **Critical/Important/Minor NONE**. The reviewer
  separately confirmed spec PASS for decision 14's owner-approved baseline
  reload in the one pre-existing closure test: all four assertions unchanged,
  no production changes. No outstanding review findings remain.
- **Final gates:** Devenv check/all-features/locked, fmt, clippy/all-targets/
  all-features/locked with `-D warnings`, and test/all-features/locked/
  `--no-fail-fast` all exited **0**. **3,468 passed, 0 failed, 1 intentionally
  ignored**, including **11 doc tests** and **621 SSH cases**. The ignored
  case is the existing opt-in thousand-item read characterization. The
  controller confirmed these logs. No tests are rerun for approval recording.
- **Delivered coverage:** **28 available verbs / 72 cataloged**, **79 bin +
  64 CLI integration tests**, **51 JSON + 4 text = 55 CLI fixtures**. All
  11 acceptance criteria have evidence in the execution ledger.
- **Smoke checks:** bare CLI help exits 0; the normal no-default-features
  dependency graph prints 0 GPUI matches (Cargo 0, grep's expected no-hit 1).
  Active-Wayland desktop launch created/configured a toplevel and submitted
  rendered buffers; deliberately stopped after 15 seconds (124), with no
  matching process remaining. This is bounded launch evidence, **not manual
  visual or interaction approval**.
- **Logs:** `/tmp/opencode/manyhands-c1-task8-final-{check,fmt,clippy,test}.log`,
  `manyhands-c1-task8-final-cli-smoke.log`,
  `manyhands-c1-task8-final-dependency-tree.log`, and
  `manyhands-c1-task8-final-desktop-{smoke,build,launch}.log` in the same
  directory. Earlier failed attempts are preserved and resolved in the ledger.
- **Limits:** native CI is **post-merge on main**, unrun here; Windows/macOS
  and Linux-aarch64 native results are not established. R-3 leaves Windows
  default location inspection-only and macOS default redirection unrun here;
  R-4 uses fake transport-start failure; R-5 retains the child libgit2 system-
  configuration isolation limitation.
- **Delivery:** the controller handles authorized delivery. These independent
  code-review approvals do not authorize owner merge/closure actions. This
  docs-only session does not push, open a PR, merge, close or clean up.

Authoritative [final verification and acceptance mapping](2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-8-final-verification-and-review-ready-checkpoint--2026-10-19)
and [final independent approvals](2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#final-independent-review-approvals--2026-10-19).

## Historical Planning Status — 2026-10-19

- **Approved by the owner on 2026-10-19:** the Cycle doc, the design and the
  implementation plan.
- **Execution method:** subagent-driven development
  (`superpowers:subagent-driven-development`).
- **Work not yet started:** implementation, starting at Task 0.
- **Commits:** the owner approved committing at each task checkpoint.

## Where

- **Ticket:** `01M4CC0VMSZ7BRS740C8VSQ5ZK`, "Wave 03 C1: CLI Shell And
  Inspection".
- **Worktree:**
  `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMSZ7BRS740C8VSQ5ZK`.
  Work only there.
- **Branch:** `manyhands/ticket/01M4CC0VMSZ7BRS740C8VSQ5ZK`, pushed to
  `origin`.
- **Documents:**
  - Cycle (governs): `docs/Cycles/wave-03-cli-01-shell-and-inspection.md`
  - Design: `docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-design.md`
  - Plan: `docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-implementation.md`
    (Tasks 0–8)
- **Authorities behind them:**
  - `docs/Waves/wave-03-dogfooding.md` (C1 section, track rules 1–7)
  - `docs/RFC/cli-contract.md`
  - `docs/RFC/ticket-relationships-and-short-codes.md`
  - the F1 Cycle and design

## What C1 Builds

`manyhands-cli` becomes a read-only CLI over every F1 read:

- clap parsing from one command catalog;
- help and version, as text or JSON;
- JSON v1 envelopes and human output;
- exit statuses mapped from failure classes;
- two new result codes, `usage_error` and `command_unavailable`;
- the `MANYHANDS_DATA_DIR` override.

Mutation commands are cataloged as planned for C2–C5 and return
`command_unavailable`.

## Test Runs: Keep Them Targeted

**Owner instruction (2026-10-19):** in every task, run only the tests
targeted at the code under development in that task. Use the filters and
targets that task's run steps name, such as `--bin manyhands-cli`,
`--test cli`, `--lib results::tests` and `--test mutation_contract`. Do not
run the full suite between tasks, or at any task checkpoint.

Save the full run for the final review: Task 8 Step 1 runs `cargo check`,
`fmt`, `clippy` and the full `cargo test --all-features --locked
--no-fail-fast` once, on the finished branch. Tell every implementer this in
its dispatch.

## Settled Decisions And Exceptions

Recorded in the Cycle doc. Do not reopen them:

- **Decisions 1–13** (2026-10-18), including:
  - clap with the builder API;
  - `MANYHANDS_DATA_DIR`;
  - JSON help and version;
  - flags for read inputs;
  - `host inspect` without `--repo`;
  - `folder list` and `--path`;
  - `path observe` and `ticket check` moved to C3;
  - F2's `request show`, `request cancel` and `host observe` planned for
    C2, and `operation abandon` for C4;
  - minimal help for planned commands.
- **Rule 5 exception (decision 9):** the only allowed edit under
  `src/repository/` is adding the two codes to the no-action arm of the
  `match code` in `src/repository/mutation/outcome.rs`. Any other library
  gap stops the task. Raise it with the owner as a separate ticket.
- **Named edits 1–6** to pre-existing tests, listed in the plan's Global
  Constraints. Any other semantic edit to a pre-existing test stops for owner
  review.
- **Rulings R-1 to R-5:**
  - R-1: parse before the transport starts;
  - R-2: `index status` on a degraded index;
  - R-3: no platform-default test on Windows;
  - R-4: a transport failure is proven only with a fake;
  - R-5: the limits of system Git configuration isolation.

## Where Earlier Reviews Found Problems

Two independent reviews of the plan found 40 and then 10 issues. All are
resolved in the documents. Watch these areas closely in task reviews:

- **clap behavior:**
  - help and version handled as our own flags;
  - `--json` and `--help` recovered by raw scans after a planned verb's
    catch-all argument;
  - no clap-required options;
  - `args_override_self`;
  - an explicit `bin_name`.
- **The shared contract tests** in `tests/mutation_contract.rs` and
  `tests/read_contract.rs`. The new codes land together with their first CLI
  goldens in Task 3, so no commit leaves them red.
- **Test isolation.** Every CLI child gets the isolation environment from
  `tests/support/cli.rs`. Tests never touch developer keys, global Git
  configuration or real app data.

## Standing Rules

- Run every Rust command through Devenv, with `--locked`. The one exception
  is the single step in Task 1 that adds clap to `Cargo.lock`.
- If `origin/main` moves, merge it into the branch. Never rebase this branch,
  because it is pushed.
- Implementers never dispatch subagents. Commit messages end with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Record checkpoints as ticket comments, in the canonical
  `.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/` path, because the CLI
  cannot write comments yet: Task 0 baseline, each task's completion, final
  verification, review-ready.
- Pushing the ticket branch is a standing habit. Ask the owner before a pull
  request, a merge to main, closing the ticket, or cleanup.
- Native CI runs on main only after merge, and only with the owner's
  approval for that run.
- Leave the main checkout's untracked `.superpowers/` alone.

## First Steps

1. Read the Cycle doc, the design and the plan.
2. Fetch, and merge `origin/main` if it moved.
3. Start the subagent-driven-development ledger for the plan.
4. Dispatch Task 0.

## Task 8 Follow-Up Addendum (2026-10-19)

The earlier planning handoff is historical. The owner has now approved Cycle
decision 14: a test-only exception for
`src/repository/closure/checkpoint_tests.rs::existing_closure_writes_no_checkpoint`.
Reload its persisted baseline after `f.record` and before either operation,
preserving all assertions and the final whole-record timestamp comparison.
This exception does not extend to production code or shared-fixture behavior.
The implementation plan's Global Constraints and execution ledger record the
bounded approval and evidence. This follow-up starts at
`2b84360a2fca123c37d09dd642b69b2afb368049`, uses no subagents, and commits
separately with the required trailer. Only the exact test, checkpoint test
group (all features, locked, Devenv), and formatting are its verification;
the controller retains the full Task 8 rerun with sufficient time budget.
No push, PR, merge, rebase or closure is authorized by this addendum.
