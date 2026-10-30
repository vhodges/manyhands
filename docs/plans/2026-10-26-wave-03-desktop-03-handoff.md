---
title: "Wave 03 D3 Execution Handoff"
date: 2026-10-26
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5X1PGX88PDFSDBWXKTC6DJG"
---

# Wave 03 D3 Execution Handoff

The brief for the session that implements Wave 03 Cycle D3, Rich-Text And
Source Authoring.

## Where

- **Ticket:** `01M4CC0VN0MFTGKQ4HGTTC5H63`.
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VN0MFTGKQ4HGTTC5H63`.
  Work only there.
- **Branch:** `manyhands/ticket/01M4CC0VN0MFTGKQ4HGTTC5H63`, pushed to
  `origin`.
- **HEAD at handoff:** the commit that adds this file, on top of `893baf5`.
  Confirm it with `git log --oneline -3`.
- **Main at handoff:** `origin/main` at `0464c42`. The branch merged it as
  `6f1463c` and merges cleanly with it.

## Authority

On 2026-10-26 the product owner approved three documents at `c3904c7`: the
Cycle document, the design and the implementation plan. The approval
covers:
- named edits E-1 to E-29;
- the round-5 edits as they stand;
- rulings R-20 and R-43.

The approval commit, `4c8ab76`, also applied the Document Amendments.

- **Execution:** subagent-driven. Use
  `superpowers:subagent-driven-development`.
  - Each task goes to a fresh implementer, and a fresh reviewer checks it
    before the next task starts.
  - Each Part checkpoint has its own review, as the plan says.
  - The whole-branch reviews are in Task 27.
- **Authorized:** implementation of Tasks 0–27, including named edits E-1
  to E-29. The owner asked for this brief on 2026-10-26 so that a separate
  session implements D3.
- **Not authorized.** Each of these needs the owner's own go-ahead, asked at
  the time:
  - any push other than this ticket branch. Pushing the ticket branch is the
    standing habit;
  - a pull request, a merge, closing the ticket, or worktree cleanup;
  - dispatching native CI;
  - creating a ticket, for example for a library gap;
  - running `ticket save` on any ticket.

## Read first, in order

1. `git show origin/main:AGENTS.md`. Take the current copy from main, not
   the worktree's.
2. `.manyhands/tickets/01M4CC0VN0MFTGKQ4HGTTC5H63/ticket.md` and every comment
   in `.manyhands/comments/01M4CC0VN0MFTGKQ4HGTTC5H63/`, oldest first.
3. `docs/Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md`, the
   Cycle document: contract 1–16, the acceptance table, decisions 1–18 and
   the exclusions.
4. `docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md`,
   the design. In particular:
   - Baseline Facts 1–80;
   - Part 0's outcome tables;
   - Part A's Design Deviations and Not Built In D3 tables;
   - Part C's session-outcome table;
   - Rulings R-1 to R-76;
   - Named Edits.
5. `docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-implementation.md`,
   the plan: Global Constraints, Review Focus, the File Map and Tasks 0–27.
6. The owner's design handoffs, for exact sizes and copy:
   - `docs/design/item_view/`: the README, 01–06, `mockups/source/ItemView.dc.html`
     and the screenshots;
   - `docs/design/04-item-area.md`, for the tab strip;
   - `docs/design/06-language-and-states.md`, for the plain-language copy.
7. `.agents/skills/implementing-a-cycle/SKILL.md` and its references, for the
   worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan. Where the
pinned libraries differ from the item-view handoff, the design's Design
Deviations table governs.

## State at handoff

- **Prerequisites.** All are merged on main:
  - Wave 02 Cycles 01–04;
  - Wave 03 F1, F2, D1, D2 and C3.

  None is pending. C3's CLI is on main.
- **Document Amendments.** They are applied in `4c8ab76`: the desktop RFC's
  D3 section, the canonical RFC's wording, and the Wave's D3 amendment and
  D4 note. Task 0 Step 2 confirms them.
- **Known library defects** (decision 7). These tickets stay open and are not
  prerequisites:
  - `01M4PB7YZMBFAFFM94AZY08HWS`: an interrupted save closed as rejected;
  - `01M4PB80T7SVB23TQ6466RDHCB`: a repeated committed move refused;
  - `01M5B163PQTE9C1T6NMHATR3KX`: item reads hide context problems;
  - `01M4R19PC0T6ZPKM39X9DF26HX`: a held synchronization reservation;
  - `01M4EHGE60PK1WQ8DM4FRJV9YF`: committed worktree files reported as
    uncommitted.

  Do not work around them. The design says what the user sees under each.
- **Other tickets:**
  - `01M5X0YK5JQSMEZH59K38X8458` (`vh-j39te`) is decision 14's follow-up,
    for editing other front-matter fields. It depends on D3 and is not part
    of D3.
  - The D1 follow-ups `01M5CS4MH25QQ769HV61ZRKHXG` (tests and polish) and
    `01M5F3GVDYTH1Q5V5FCS9YF7V3` (settings UX) edit the same item view,
    tabs and settings files. Merge main at work boundaries.
  - The D4 ticket `01M4CC0VN1KTQFSVR99378TDHV` depends on D3.
  - The multi-item contexts ticket `01M5NKA3MY7CW4D90M866YS946` lands after
    D3.
  - Gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` gates C4, C5 and D5, not D3.
- **Stop points built into the plan:**
  - **Task 3:** the editor in a headless window, and the read-only guard.
  - **Tasks 1 and 2:** the context commands' outcome tables.
  - **Tasks 15–19:** the session-outcome table.
  - **Task 19:** the review table.
- **No code yet.** Nothing was built or tested during planning. Task 0 runs
  the baseline, the full suite included.

## Binding rules

- **Cargo.** Run Cargo only through Devenv, with `--locked`. The one
  exception is Task 3's single lockfile update.
- **Library code is Part 0's only.**
  - Tasks 1 and 2 may change only the files the plan lists for them.
  - `src/drafts/` changes only in Task 22.
  - `Cargo.toml` changes only in Task 3, and `Cargo.lock` gains no new
    package.
  - There is no new `ResultCode`.

  A library gap stops the task. It needs a ticket, which needs the owner's
  go-ahead.
- **Desktop code** uses GPUI only through `gpui_kit::*`, never as a glob.
  `zorite_editor`, `zorite_markdown` and `image` are used only under
  `src/desktop/`.
- **Pre-existing tests.** Only named edits E-1 to E-29 may change one, in
  the task the design names. Task 0's sweep proves there are no others. Any
  other change, even a new lint on an old test, stops for the owner.
- **Outcome tables** are written as tests first. If the code answers any row
  differently, stop for a ruling. Never edit a table to fit.
- **Test runs.**
  - Between tasks, run only the Verify lines each task names, with one
    positional filter per `cargo test`, and check that more than zero tests
    ran.
  - Each desktop task also runs `--features desktop` check and clippy on
    their own.
  - It ends with
    `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`.
  - Part checkpoints run the Part's targets in full.
  - The full suite runs in Task 0 and once in Task 27, detached, with a
    two-hour budget.
- **Editor tests.**
  - Type through the window-level helpers, never gpui-kit's scoped ones.
  - The test shell binds the editor's keys from Task 3 on.
  - Never send Ctrl-Cmd-Space.
- **Merging.** Merge `origin/main` at work boundaries and before each Part
  checkpoint. Run `git merge-tree --write-tree HEAD origin/main` before the
  checkpoint. Never rebase.
- **Design handoffs.** No task edits `docs/design/`.
- **Implementers.** They never dispatch subagents.
- **Commits.** Commit per task. Every commit message ends with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Execution ledger.** Task 0 creates
  `docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-execution.md`,
  and every task commits it.
- **Ticket comments.** Add one at Task 0, after each task, at each Part
  checkpoint and at review-ready.
  - Write it as a canonical file under
    `.manyhands/comments/01M4CC0VN0MFTGKQ4HGTTC5H63/`, until C4's
    `comment add` exists.
  - Take the ULID from
    `devenv shell -- cargo run --locked --bin manyhands-cli -- id new`,
    and `created_at` from the UTC clock.
  - Closure stays a canonical ticket-file edit until C5 (`AGENTS.md`).
- **The native checklist** in the design is the owner's to run, in
  Task 27 Step 2. Ask; never assume a result.

## Stop and ask the owner when

- a contract question comes up that the Cycle, design and plan do not
  settle;
- something breaks outside the named edits, or Task 0 finds a changed fact
  that alters a task;
- any stop point fires:
  - Task 3's feasibility test, or any read-only guard test;
  - a row of an outcome table in Tasks 1, 2 or 15–19;
- a library gap is found, or a new ticket seems needed;
- the item-view handoff needs a deviation that the Design Deviations table
  does not list;
- a merge of main brings item-view, tab or settings changes that conflict
  with a task;
- a known-defect ticket has merged since planning.

## Done means

- Tasks 0–27 are complete, and every Part checkpoint review has passed.
- The full gate is green, and the counts are recorded in the execution
  ledger.
- The whole-branch reviews (spec, and quality of the actual code) are
  approved.
- The owner has run the Linux Wayland checklist and the desktop smoke test,
  and the results are in the Cycle's acceptance table.
- The acceptance table is filled in. Native Windows and macOS runs are left
  to the post-merge Build workflow, which needs the owner's own go-ahead.
- The review-ready ticket comment is written, and the ticket branch is
  pushed.

The ticket is closed only after the owner approves the review, in the final
push before merge. Merging needs its own authorization.
