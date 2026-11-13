---
title: "Wave 03 C3 Execution Handoff"
date: 2026-10-24
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5RC3P3MWWGWFNVCXSSNSSGT"
---

# Wave 03 C3 Execution Handoff

The brief for the session that implements Wave 03 Cycle C3, CLI Authoring And
Discovery Maintenance.

## Where

- **Ticket:** `01M4CC0VMV81DFRGJ4Z76489XM`.
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMV81DFRGJ4Z76489XM`.
  Work only there.
- **Branch:** `manyhands/ticket/01M4CC0VMV81DFRGJ4Z76489XM`, pushed to
  `origin`.
- **HEAD at handoff:** the approval commit that adds this file, on top of
  `e7b2b71`. Confirm it with `git log --oneline -3`.
- **Main at handoff:** `origin/main` at `e49f978`. The branch merged it as
  `9ed4a04` and merges cleanly with it.

## Authority

On 2026-10-24 the product owner approved three documents at `e7b2b71`: the
Cycle document, the design and the implementation plan.

- **Execution:** subagent-driven. Use `superpowers:subagent-driven-development`.
  Each task goes to a fresh implementer, and a fresh reviewer checks it
  before the next task starts. A whole-branch review follows Task 9.
- **Authorized:** implementation of Tasks 0–9, including named edit E1, the
  three help goldens regenerated one verb at a time.
- **Not authorized.** Each of these needs the owner's own go-ahead, asked at
  the time:
  - any push other than this ticket branch. Pushing the ticket branch is the
    standing habit;
  - a pull request, a merge, closing the ticket, or worktree cleanup;
  - dispatching native CI;
  - Task 9's dogfooding saves and pushes. The design's Dogfooding section
    asks for that go-ahead itself.

## Read first, in order

1. `git show origin/main:AGENTS.md`. Take the current copy from main, not
   the worktree's.
2. `.manyhands/tickets/01M4CC0VMV81DFRGJ4Z76489XM/ticket.md` and every comment
   in `.manyhands/comments/01M4CC0VMV81DFRGJ4Z76489XM/`, oldest first.
3. `docs/Cycles/wave-03-cli-03-authoring-and-discovery.md`, the Cycle
   document: its contract, decisions 1–7 and Review Record.
4. `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-design.md`,
   the design: Baseline Facts 1–37 and Rulings R-1 to R-18.
5. `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-implementation.md`,
   the plan: Global Constraints, Review Focus and Tasks 0–9.
6. `.agents/skills/implementing-a-cycle/SKILL.md` and its references, for the
   worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan.

## State at handoff

- **Prerequisites.** All are merged on main: Wave 02 Cycles 01–04, and
  Wave 03 F1, F2, C1 and C2. None is pending.
- **Known library limits** (decision 1). These tickets stay open and are not
  prerequisites:
  - `01M4PB82G07RZSQMNJZN672S3W`: partial database damage;
  - `01M4PB80T7SVB23TQ6466RDHCB`: repeating a committed move;
  - `01M4PB7YZMBFAFFM94AZY08HWS`: an interrupted save's rejected row.

  Do not work around them.
- **Other tickets.**
  - Gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` gates C4, C5 and D5, not C3.
  - Multi-item ticket `01M5NKA3MY7CW4D90M866YS946` lands after C3. C3 adds
    no `--context` to its mutation verbs.
- **Dogfooding targets.**
  - The G1 worktree was cleaned by the owner on 2026-10-24.
  - The six target branches (C4, C5, D4, D5, D6 and G1) are not on origin
    (design fact 36).
- **Open checks for Task 0.**
  - Design fact 17: whether a no-op save creates a context.
  - The codes of an invalid repair source.
  - The cache-loss codes.
  - Whether `initials_required` frees its record.
- **No code yet.** Nothing was built or tested during planning.

## Binding rules

- **Cargo.** Run Cargo only through Devenv, with `--locked`.
- **Library code is off limits.** No changes to:
  - `src/repository.rs`, `src/repository/`, `src/lib.rs`, `src/results.rs`
    or `src/runtime.rs`;
  - `Cargo.toml` or `Cargo.lock`, and no new `[[test]]` target.

  No new result code, recovery action or output schema. A library gap stops
  the task, and becomes a separate ticket under track rule 5.
- **Pre-existing tests.** Only named edit E1 may change one. Any other
  change, even a new lint on an old test, stops for the owner.
- **Test runs.**
  - Between tasks, run only the targets and filters each step names, with
    one positional filter per `cargo test`.
  - Clippy always runs with `--all-targets --all-features`.
  - At Tasks 4 and 7, run the plan's checkpoint gate, which includes
    `--all-features` on the test line.
  - The full suite runs once, in Task 9, detached, with a two-hour budget.
  - C3 changes no desktop code. The CLI binary is also checked with no
    features, and with `scripts/check-cli-no-gpui.sh`.
- **Checkpoints.** Before each checkpoint, run `git fetch origin` and
  `git merge-tree --write-tree HEAD origin/main`. If main moves, merge it;
  never rebase.
- **Task 3 starts test-first.** Step 0 writes the interrupted-saves test
  first. If the library answers any row differently, stop for a ruling.
  Do not edit the table to fit.
- **Goldens.** Update them only with `MANYHANDS_UPDATE_GOLDEN=1`, and read
  every diff before committing.
- **Implementers.** They never dispatch subagents.
- **Commits.** Every commit message ends with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Ticket comments.** Add one at Task 0, after each task, at each Part
  checkpoint, after dogfooding and at review-ready.
  - Write it as a canonical file under
    `.manyhands/comments/01M4CC0VMV81DFRGJ4Z76489XM/`.
  - Take the ULID from `manyhands-cli id new`, and `created_at` from the UTC
    clock.

## Stop and ask the owner when

- a contract question comes up that the Cycle, design and plan do not
  settle;
- something breaks outside the named edits, or Task 0 finds a changed fact
  that alters a task;
- Task 3 Step 0's checkpoint fails;
- a library gap is found;
- Task 9 reaches the dogfooding go-ahead, or one of its stop rules fires;
- a decision-1 ticket has merged since planning.

## Done means

- Tasks 0–9 are complete.
- The full gate is green, and the counts are recorded in the execution
  ledger.
- The whole-branch reviews (spec, and quality of the actual code) are
  approved.
- The dogfooding step is either done with the owner's go-ahead, or recorded
  as owed with the owner. While it is owed, the Cycle may be review-ready,
  but it is not closed or merged.
- The switch commit has had its own review.
- The review-ready ticket comment is written, and the ticket branch is
  pushed.

The ticket is closed only after the owner approves the review, in the final
push before merge. Merging needs its own authorization.
