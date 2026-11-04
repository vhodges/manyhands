# Wave 03 D5 Execution Ledger

The orchestrator's record for
[the plan](2026-11-03-wave-03-desktop-05-recovery-and-completion-implementation.md):
task state, commit ranges, decisions, review findings and verification. It
is written only by the orchestrator. Implementers never edit it.

## Authorization

- **Approved 2026-11-04.** The owner wrote: "The docs and execution are
  approved, use subagent driven development." The documents were at
  `0785a807`. The approval commit is `f203e865`.
- **Implementation is authorized,** subagent-driven and serial (decision 8):
  one implementer per task, then a fresh reviewer, one at a time.
- **Still gated:** pushes beyond the ticket branch, a PR, the merge,
  closure, native CI and cleanup.

## Base

- **C5's merge.** C5 merged as `2d912d10` on local main. GitHub was
  unreachable and `origin/main` was still at `c2a34494`, so the owner
  directed proceeding on local main. It was merged here as `cd2e922e`
  (HEAD before: `6dd645c0`), with no conflicts.
- **Fetch.** `git fetch origin` failed on 2026-11-04 with "check access
  rights" (the GitHub outage).
- **Unpushed commits.** `cd2e922e` onward exist only locally until the
  push succeeds.

## Task 0: Baseline, References And Ledger

**Step 1, baseline.** The run started at `3b1443c0`; the tree differed
only in documentation.

| Check | Result |
| --- | --- |
| `cargo check --all-features --locked` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | pass |
| `scripts/check-cli-no-gpui.sh` | pass |
| `cargo test --all-features --locked --no-fail-fast` | running |

**Step 2, facts.** The (C5) facts were re-checked against
`f59aceb6..2d912d10^2`. What changed is recorded in the design's
Re-grounding section, committed at `0785a807`.

**Step 3, U-1 to U-5.** The settlements are in the design. U-1 and U-3
hold. U-2, U-4 and U-5 hold in part, and the owner ruled on each on
2026-11-04:
- **Decision 22 (U-5):** a new task, Task 3b, adds L4.
- **Decision 23 (U-2):** recorded as Known Limit K-14.
- **Decision 24 (comment posts):** recorded as Known Limit K-15.
- **U-4 correction:** `run_lifecycle` emits `connecting` when the context
  child starts.

These were committed at `a6ba36cf`.

**Step 4, sweep.** It matched E-1 to E-7 and added E-8
(`posting_tests/decisions.rs:143-149`). `LATER_NOTE` stays, because
`close_draft.rs` uses it. E-2 may be empty. The E-6 lines moved to
`outcomes_tests.rs:431-444`.

**Amendments.**
- 1–4 and 6 were applied at `f203e865`.
- 5 was a `ticket save` (request `01M6K26HZM2KJKCW2EXAVV3AXH`), checkpoint
  `3b1443c0`.
- 3's schema part lands with Task 3.

## Tasks

| Task | State | Base | Commits | Review | Verification |
| --- | --- | --- | --- | --- | --- |
| 0 | baseline running | `cd2e922e` | `f203e865`, `a6ba36cf`, this ledger | — | above |
