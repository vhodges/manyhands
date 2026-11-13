---
title: "Wave 03 D6 Execution Handoff"
date: 2026-11-09
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M70CRVK8CXY3T3YM6QMQZ57D"
---

# Wave 03 D6 Execution Handoff

The brief for the session that implements Wave 03 Cycle D6, Desktop
Background Polling And Shutdown.

## Where

- **Ticket:** `01M4CC0VN3VPVVQ3RNWJKXWEVX`.
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VN3VPVVQ3RNWJKXWEVX`.
  Work only there.
- **Branch:** `manyhands/ticket/01M4CC0VN3VPVVQ3RNWJKXWEVX`, pushed to
  `origin`.
- **HEAD at handoff:** the commit that adds this file, on top of `38bfaf49`
  (the approval commit). Confirm it with `git log --oneline -3`.
- **Main at handoff:** `origin/main` at `f4442618`. The branch merged it as
  `9512080d` and is level with it.

## Authority

On 2026-11-09 the product owner approved the three documents at `04e1623f`
(revision 6): the Cycle document, the design and the implementation plan.
The owner wrote: "The docs are approved. Named test edits approved (if during
execution it's discovered that more tests would need to be edited, that's
approved as well). Subagent driven development. Implementation is approved as
well."

The approval covers:
- decisions 3–15 and 28 at their written proposals; 1, 2 and 16–27 are the
  owner's own;
- rulings R-1 to R-38;
- named test edits E-1 to E-6. **Any further edit to an existing test that
  execution finds necessary is pre-approved.** Record each in the ledger as
  E-7 onward, with its reason, and report it at the next checkpoint.

The approval commit, `38bfaf49`, also applied the design's Document
Amendments 1–5 and 7–10 (the PRD's 0.8 entry, four RFCs, the retirement
Cycle, Wave 03, Wave 04's Cycle 03 section, D5, D1, D2 and the ticket's exit
evidence). Amendment 6 waits for D6's merge.

- **Execution:** subagent-driven and serial. Follow
  `.agents/skills/implementing-a-cycle/SKILL.md`.
  - Each task goes to a fresh implementer, and a fresh reviewer checks it
    before the next task starts.
  - Implementers never dispatch subagents and never edit ticket, comment or
    ledger files; the orchestrator does.
  - Each Part checkpoint (Tasks 2, 5, 9, 10, 12 and 14) has its own review
    and ticket comment, as the plan says.
- **Authorized:** implementation of Tasks 0–14 (with 11a and 11b), including
  the test edits above.
- **Not authorized.** Each of these needs the owner's own go-ahead, asked at
  the time:
  - any push other than this ticket branch. Pushing the ticket branch is the
    standing habit;
  - a pull request, a merge, closing the ticket, or worktree cleanup;
  - dispatching native CI;
  - creating a ticket, for example for a library gap;
  - `ticket save` or `comment add` on any other ticket. `comment add`
    publishes the target ticket's branch and checkpoints into its worktree;
  - publishing the two follow-up tickets' branches (below).

## Read first, in order

1. `git show origin/main:AGENTS.md`. Take the current copy from main, not
   the worktree's. Its Testing Strategy (2026-11-06) governs every test run.
2. `.manyhands/tickets/01M4CC0VN3VPVVQ3RNWJKXWEVX/ticket.md` and every comment
   in `.manyhands/comments/01M4CC0VN3VPVVQ3RNWJKXWEVX/`, oldest first. The
   comment `01M6Z8A8AYDGTK4BWKZR3931ST` is Wave 04 Cycle 01's coordination
   note.
3. `docs/Cycles/wave-03-desktop-06-polling-and-shutdown.md`, the Cycle
   document: Scope 1–12, contract 1–11, acceptance A1–A23, decisions 1–28,
   the exclusions and the Wave 04 Cycle 01 coordination.
4. `docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md`,
   the design. In particular:
   - Baseline Facts 1–36 (library at `9512080d`);
   - Source Questions: what is settled, and U-2, U-9 (rest), U-11 (rest),
     U-14, U-15 and U-16, which are still open;
   - Parts A–E, especially A4's answer table, B2's P-1 to P-7, B3's in-flight
     record and L-1 to L-6, B5's provider rule, Part D, and E1's RT-1 to
     RT-11 and E3's answer table;
   - Wave 04 Coordination W-1 to W-7;
   - Rulings R-1 to R-38, Testing (E-1 to E-6), Known Limits K-1 to K-27.
5. `docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-implementation.md`,
   the plan: Global Constraints, Review Focus, the File Map and Tasks 0–14.
6. `docs/Waves/wave-03-dogfooding.md`, section D6 and its 2026-11-09 notes.
7. Wave 04 Cycle 01's approved Cycle document, on its branch:
   `git show origin/manyhands/ticket/01M6V8RK8B0SBCH0YZXFVSR9E3:docs/Cycles/wave-04-cycle-01-workspace-shell.md`
   (decisions 3, 7 and 14; K-18).
8. `.agents/skills/implementing-a-cycle/SKILL.md` and its references, for the
   worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan.

## State at handoff

- **Prerequisites.** All are merged on main:
  - D1–D5 and C5;
  - the completed-context retirement library ticket
    `01M6KHVD6D0DB8KYT6VGQP10Y0` (main `f4442618`).

  None is pending.
- **In flight beside D6:**
  - **Wave 04 Cycle 01** (`01M6V8RK8B0SBCH0YZXFVSR9E3`), approved and in
    implementation (its branch at `1cd7d817` at handoff). It is expected to
    merge first. Tasks 8, 10 and 13 have a step that applies only once it is
    on main; check at Task 0 and at every merge of main, and apply skipped
    steps when it lands (plan, Global Constraints). If D6 reached main first,
    Cycle 01 would stop for a design checkpoint with the owner.
  - The follow-ups `01M6RAS63EWVHWWVBSZ836RXG2` and `01M6H7VXJJHRYHTW9S8TJ88921`
    plan to split `D/model/operations.rs` and `D/view/operations.rs`, which
    D6 edits. Merge main at work boundaries.
- **Follow-up library tickets** (decisions 25 and 27). They are not
  prerequisites, and D6 records each as a known limit:
  - `01M70CF14M3YC6SR8T9JJ6XQBG` (`vh-ype2f`): a per-key passphrase cache
    (K-23);
  - `01M70CF1VBHJAX6MK39FY5W2RY` (`vh-xmhsj`): ending a dead poll's row
    whatever the eligibility (K-22).

  Both are filed locally and not yet published. Do not work around them.
- **Design checkpoints built into the plan.** Each is with the owner, before
  any code of its task:
  - **Task 4:** P-1 to P-7 and L-1 to L-6, against U-13, U-15 and U-16.
  - **Task 11b:** RT-1 to RT-11, against D2's draft listing and the worker's
    request states.
- **No code yet.** Nothing was built or tested during planning. Task 0 runs
  the static gates only; the fast tier runs once, at review-ready.

## Binding rules

- **Cargo.** Run Cargo only through Devenv, with `--locked`.
- **Library boundary (decision 1).** No task changes `src/repository.rs`,
  `src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
  `src/runtime.rs`, `src/lib.rs`, `Cargo.toml` or `Cargo.lock`. The only file
  under `tests/` that changes is `tests/remote_polling.rs` (Task 1's two
  cases). A library gap stops the task: it needs a ticket, which needs the
  owner's go-ahead.
- **Desktop code** uses GPUI only through `gpui_kit::*`, never as a glob.
- **Outcome tables** (A4, P-1 to P-7, L-1 to L-6, RT-1 to RT-11, E3) are
  written as tests first. If the code answers any row differently, stop for a
  ruling. Never edit a table to fit.
- **Test runs** (AGENTS.md Testing Strategy, which overrides any broader run):
  - between steps, `cargo check` or `clippy` and only the tests just written
    or changed, by exact full module path with `-- --exact`, and check that
    more than zero tests ran;
  - never a whole SSH target or the whole desktop bin suite between steps or
    at a Part checkpoint;
  - Task 1's two SSH cases run singly, by exact name;
  - the static gates, the fast tier and the SSH tier for `remote_polling` run
    once, at review-ready (Task 14);
  - no `sleep` to pass time: inject the clock or the duration;
  - stop and narrow if a command runs past about 15 minutes, and record which
    tests ran at each checkpoint.
- **Existing tests.** E-1 to E-6, plus any further edit execution finds
  necessary, recorded as E-7 onward with its reason.
- **Merging.** Merge `origin/main` at Task 0, at each Part checkpoint and
  whenever main moves. Run `git merge-tree --write-tree HEAD origin/main`
  before each checkpoint. Never rebase.
- **The real registry.** Run `manyhands-cli` against real data only from the
  up-to-date main checkout (`/home/vhodges/work/src/manyhands`), never a build
  of this branch. Fast-forward main before each use.
- **Commits.** Commit per task, conventional message, ending with your own
  session's `Co-Authored-By` trailer.
- **Execution ledger.** Task 0 creates
  `docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-execution.md`,
  and every task commits it. Record each ruling there as "Ruling: decision —
  why — cost if wrong".
- **Ticket comments.** At Task 0, at each Part checkpoint and at review-ready,
  with `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id
  01M4CC0VN3VPVVQ3RNWJKXWEVX --request-id <ULID> --input -` from the main
  checkout, only while the ticket worktree is clean. Take ULIDs from
  `manyhands-cli id new`. The comment publishes this ticket's branch.
- **The native checklist** in the design is the owner's to run, from Task
  14's written checklist. Ask; never assume a result.

## Stop and ask the owner when

- a contract question comes up that the Cycle, design and plan do not
  settle;
- a design checkpoint comes due (Tasks 4 and 11b);
- a row of an outcome table answers differently from the code;
- Task 0 finds a changed fact, or a U answer that differs from the design
  (U-2, U-9, U-11, U-13, U-15 or U-16);
- Task 1 finds no reliable connect stall (U-14), or a cancel does not return
  within the bound;
- an existing path loses editor text (Task 9) or a retired item's draft
  cannot be exported (Task 12): these are D2 or D3 defects, not D6's to patch;
- something outside the tests breaks that no task names;
- a library gap is found, or a new ticket seems needed;
- Wave 04 Cycle 01 merges with changes that alter a W item, or conflicts
  with a task in a way the W items do not settle.

## Done means

- The static gates, the fast tier and the `remote_polling` SSH tier are green
  at review-ready, with counts in the ledger.
- The ledger is complete, and the Cycle's Acceptance table and Review Record
  are updated.
- The ticket is marked review-ready (`ticket.md` committed, then `ticket
  save` from the main checkout), with a review-ready comment that lists the
  tests run, the rulings, the known limits and the native checklist.
- The ticket is closed only after review approval and the owner's go-ahead,
  with `manyhands-cli ticket close --repo /home/vhodges/work/src/manyhands
  --id 01M4CC0VN3VPVVQ3RNWJKXWEVX` at the owner's terminal. The close performs
  the merge. Native CI runs after merge and is separately gated.
