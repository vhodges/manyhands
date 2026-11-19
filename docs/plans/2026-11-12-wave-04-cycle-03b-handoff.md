---
title: "Wave 04 Cycle 03b Execution Handoff"
date: 2026-11-13
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7AHWQZFEM85PGVX6PTAP76C"
---

# Wave 04 Cycle 03b Execution Handoff

This is the brief for the session that implements Wave 04 Cycle 03b,
Activity And Needs Attention.

## Where

- **Ticket:** `01M784Y94Q3AXT6XJX38F5VK1V` (`vh-dt0aq`).
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M784Y94Q3AXT6XJX38F5VK1V`.
  Work only there.
- **Branch:** `manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`, pushed to
  `origin`.
- **HEAD at handoff:** the branch's latest commit. The approval commit
  `95ba3535` adds this file. After it come the approval's ticket comment
  (`ff381f35`), the amendment allowing branch pushes (`3367b9a7`), and this
  line's correction. Confirm with `git log --oneline -6`.
- **Main at handoff:** `origin/main` at `ba95e1b7`, where Cycle 03a
  (`01M784Y8C73B39GR4JWENXE5D0`) was closed and merged.
  - The branch merged Cycle 03a's published head at `b37d2c2b` while 03a
    was in review (decision 4).
  - It then merged main with 03a at `81789331`.
  - Cycles 01 and 02 and Wave 03 D6 are on main too.

## Authority

On 2026-11-13 the product owner approved the three documents (revision 5):
the Cycle document, the design and the implementation plan. The approval
message asked for six things:

1. approve the documents;
2. approve the named test edits;
3. confirm decisions 14–20;
4. skip a further review of revision 5;
5. choose subagent-driven, serial execution in a fresh session from this
   brief;
6. authorize implementation.

The owner wrote: "All approved." That is decision 26.

The approval covers:

- **Decisions 1–26** in the Cycle document:
  - 1–3 were made in Cycle 03a's planning;
  - 4–12, 21–25 and 26 were answered by the owner;
  - 13 was the planner's outline, accepted;
  - 14–20 were proposed by the planner and confirmed.
- **The design's contract:**
  - rulings R-1 to R-59 (R-19 and R-42 withdrawn);
  - Required Outcomes P1–P32;
  - the attention table N1–N25 (N20 withdrawn) and its precedence;
  - the log's table T1–T18.
- **Named test edits E-1 to E-16**, including the removals marked
  **Removes**. **Any further edit to an existing test that execution finds
  necessary is pre-authorized** (decision 14):
  - record each in the ledger as E-17 onward, with its reason;
  - the task's reviewer checks it;
  - an edit that removes an assertion is called out in the ledger and in
    the review-ready summary.

The approval commit applied the design's Document Amendments 1–12:

- the Wave's Cycle 03b section and a Risks row;
- the handoff's `02`, `03`, `05`, `06` and `08`;
- Wave 03 D5's and D6's privacy rules, and D6's decision 13 and W-6;
- Cycle 01's decision 3 and K-19;
- Cycle 03a's K-9 and R-38.

**Execution** is subagent-driven and serial (decision 26). Follow
`.agents/skills/implementing-a-cycle/SKILL.md` and the
`superpowers:subagent-driven-development` skill.

- Each task goes to a fresh implementer. A fresh reviewer checks it before
  the next task starts.
- Implementers never dispatch subagents and never edit ticket, comment or
  ledger files; the orchestrator does.
- **Task 5 has a design checkpoint.** Its implementer writes these tests
  first, row by row, before any of Task 5's code:
  - `attention_counts_each_thing_once`, from the design's N-table with its
    precedence and latches (A3);
  - `the_log_keeps_times_and_events`, from the T-table (A4).

  If a row cannot be expressed against the code, or the code would answer
  it differently, the implementer stops and reports. The orchestrator
  brings it to the owner before anything is implemented.
  - **Why it matters:** revision 5 had no independent review. Four review
    rounds found Blocking issues in this mechanism, so these tables are its
    contract.
- **Ticket comments:** the plan's checkpoints, after Tasks 0, 5, 8, 10 and
  13, each get one.

**Authorized:**

- implementation of Tasks 0–13, including the test edits above;
- pushing this ticket branch after each task's commit (owner, 2026-11-11:
  "changes on the branch should be being pushed to origin so others who
  are not local can review");
- pushing other branches, including through `ticket save` or `comment add`
  on another ticket, which publishes that ticket's branch and checkpoints
  into its worktree (owner, 2026-11-13: "pushing is how people and agents
  will collaborate in manyhands so it's okay to push branches"). Say in
  the ledger and the next ticket comment which branches were published.

**Not authorized.** Each needs the owner's own go-ahead, asked at the time:

- a pull request, a merge into main, closing the ticket, or worktree
  cleanup;
- dispatching native CI;
- creating a ticket, for example for a library gap.

## Read First, In Order

1. `git show origin/main:AGENTS.md`. Take the current copy from main, not
   the worktree's. Its Testing Strategy (2026-11-06) governs every test run.
2. `.manyhands/tickets/01M784Y94Q3AXT6XJX38F5VK1V/ticket.md`, then every
   comment in `.manyhands/comments/01M784Y94Q3AXT6XJX38F5VK1V/`, oldest
   first.
3. `docs/Cycles/wave-04-cycle-03b-activity-and-needs-attention.md`, the Cycle
   document. Read in particular:
   - Scope 1–7;
   - Required Contract 1–10;
   - Acceptance A1–A16;
   - decisions 1–26;
   - Known Limits.
4. `docs/plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-design.md`,
   the design. In particular:
   - **Baseline Facts 1–47.** They were re-grounded at `81789331`, after
     03a merged.
   - **Part A:** the types; the sources S1–S14 and their inputs; the
     attention table A3, with its subjects, families, precedence and
     latches; the log's rules and the T-table A4; the brought-in history
     A5; `each_repository` A7.
   - **Part B:** B0 maps today's operations area to the rows; then every
     copy table and the tooltip order.
   - **Parts C–E:** the panel and its keys, the row menu with its revision
     guard, and the popover with its focus rules.
   - **Part F:** the folded-in work. Part G: privacy. Part H: keyboard.
   - **The rest:** Required Outcomes P1–P32; the Testing section, above all
     the seam (`act`, `act_refused`, `choose_activity`); Named Edits E-1 to
     E-16; Rulings R-1 to R-59; Known Limits K-1 to K-13; Unverified U-1
     to U-8.
5. `docs/plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-implementation.md`,
   the plan: Global Constraints, Review Focus, the File Map and Tasks 0–13.
6. `docs/Waves/wave-04-workspace-design.md`: the Cycle 03b section (amended
   at approval), Owner Decisions 1–10 and Testing.
7. **Cycle 03a:**
   - `docs/Cycles/wave-04-cycle-03a-repositories.md` and its design:
     `repository_state`, the footer status items, the repository menu
     layer (the popover and the row menu follow its pattern), K-9;
   - its execution ledger
     `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md`,
     for what implementers found.
8. **Cycle 01:** `docs/Cycles/wave-04-cycle-01-workspace-shell.md` and its
   design: the Activity panel, the footer, the dock menu, decisions 3, 5
   and 7, K-19.
9. **Wave 03 D5 and D6 designs, as amended at approval:**
   - `docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md`:
     Privacy;
   - `docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md`:
     holds, checks, W-6, Privacy.
10. **The design handoff:**
    - `docs/design/03-panels.md` (Activity);
    - `05-sheets-and-dialogs.md` (Needs attention popover);
    - `02-layout-and-docking.md` (Footer);
    - `06-language-and-states.md`, `07-design-tokens.md`,
      `08-gpui-kit-mapping.md`;
    - frames `docs/design/screenshots/1a` and `2f`.
11. `.agents/skills/implementing-a-cycle/SKILL.md` and its references, for
    the worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan.

## State At Handoff

- **No code yet.** Nothing was built or tested during planning.
  - Task 0 runs the static gates only.
  - The fast tier runs once, at review-ready (Task 13).
- **Unverified at handoff** (the design's U-list). Each is checked first by
  the task that relies on it:
  - **U-1 (Task 3):** the derives `ActivityMenuChoice` needs. A library
    type that lacks one stops the task.
  - **U-5 (Task 4):** `checked_now`'s in-flight branch returns a summary.
  - **U-7 (Task 3):** an open item's title is found from `(root, kind, id)`.
  - **U-2 and U-4 (Task 9):** GPUI's `uniform_list` draws and scrolls the
    rows, and keys bound in the Activity context reach the list. If U-2
    fails, the fallback is the Kit's `VirtualList`, recorded in the ledger.
  - **U-8 (Task 11):** focus comes back to the footer item after a prompt
    or menu opened from the popover. If it does not, opening such a layer
    first moves focus to the footer item.
- **Size and split line.** The plan has 14 tasks. If the owner splits the
  Cycle:
  - 03b-1 is Tasks 0–10: model, copy, panel, row menu, footer count;
  - 03b-2 is Tasks 11–13: popover, privacy, gates.

**Planning lessons the plan carries as rules:**

- **The Kit `List` presses Confirm on every click.** So Activity draws its
  own list with GPUI's `uniform_list` and its own keys.
- **The Kit `Popover` is not used.** The popover is a Workspace-owned
  layer, like Cycle 03a's repository menu: dropped under a dialog or sheet,
  one layer at a time.
- **Every Kit menu has the element id `"popup-menu"`.** Tests and sweeps
  find the row menu through the Workspace accessor, not by id.
- **`window.dispatch_action` goes only to the focused node.**
  `FocusHandle::dispatch_action` does nothing for an undrawn handle. So the
  test seam calls `choose_activity` directly.
- **GPUI deduplicates `notify` per entity within one flush.** The log's
  refresh is coalesced and runs at named refresh points, never in render.
- **`on_mouse_down_out` fires on mouse down, before the footer item's
  click.** The popover ignores presses inside the item's bounds.
- **Window tests with polling on answer every poll.** Focus is read in the
  same update that causes it. Guard tests use key presses, with a dialog
  and with a sheet.
- **Dead code staged for a later task** carries a plain
  `#[allow(dead_code)] // Used from Task N`.
- **No `chmod` fixtures, and no real waits.** The clock comes from
  `Services::with_clock`.
- **The privacy sweeps classify text by element id or container.** Renamed
  ids must keep their surfaces; the `"operation-area"` id stays on the
  list's container.

## Binding Rules

- **Cargo.** Run Cargo only through Devenv, with `--locked`.
- **Boundary.** No task changes anything under `src/` outside
  `src/desktop/`, nor `tests/`, `Cargo.toml`, `Cargo.lock` or `packaging/`.
  - After each task's commit,
    `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging`
    prints nothing.
  - A library gap stops the task. It needs a ticket, which needs the owner's
    go-ahead.
- **Desktop code** uses GPUI only through `gpui_kit::*`, never as a glob.
- **Tables first.**
  - Each model table in the plan is written as a test before its code: the
    N- and T-tables first of all, in Task 5.
  - If the code answers any row differently, stop for a ruling.
  - Never edit a table to fit.
- **Test runs** (`AGENTS.md` Testing Strategy, which overrides any broader
  run):
  - between steps, run `cargo check` or `clippy` and only the tests just
    written or changed, by exact full module path:
    `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`.
    Check that more than zero tests ran;
  - two named exceptions in the plan:
    - Task 1 runs the moved operations model's tests by module;
    - Task 9 reruns the E-5 and E-6 tests by name;
  - no whole desktop bin suite between tasks;
  - the static gates and the fast tier (with `--no-fail-fast`) run once, at
    review-ready (Task 13). No SSH target runs, since nothing under their
    paths changes;
  - no `sleep` to pass time: inject the clock or the duration;
  - if a command runs past about 15 minutes, stop and narrow it. Record
    which tests ran at each checkpoint;
  - never build while a test run is in progress.
- **No appearance tests.** Nothing asserts colours, sizes, icons or that a
  view renders.
  - An appearance test the redesign breaks is deleted under E-15, named in
    the ledger.
  - P21's bounds check is reachability, not appearance.
- **Merging.**
  - Merge `origin/main` at Task 0, before Task 9, and whenever main moves.
  - Run `git merge-tree --write-tree HEAD origin/main` before each
    checkpoint.
  - Never rebase: the branch is pushed.
- **Pushing.** After each task's commit, publish the ticket branch from the
  main checkout:
  `devenv shell -- cargo run -q --locked --bin manyhands-cli -- item sync --repo /home/vhodges/work/src/manyhands --id 01M784Y94Q3AXT6XJX38F5VK1V --non-interactive --json --request-id <ULID>`,
  and check that `HEAD` equals
  `origin/manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V`.
  - It answered `publication: published` during planning.
  - It also merges `origin/main` into the branch when main has moved: it
    did so at `81789331`. Re-read the merged hunks after such a sync.
  - Devenv prints banner lines on stdout. Take the JSON from the last line.
- **The real registry.** Run `manyhands-cli` against real data only from the
  up-to-date main checkout (`/home/vhodges/work/src/manyhands`), never a build
  of this branch. Fast-forward main before each use.
- **Commits.** One per task, with a conventional message, ending with your
  own session's `Co-Authored-By` trailer.
- **Execution ledger.** Task 0 creates
  `docs/plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-execution.md`,
  and every task commits it. Record in it:
  - each ruling, as "Ruling: decision — why — cost if wrong";
  - every named or further test edit (E-n), with its reason;
  - each converted test file's function list before and after (Tasks 6–8);
  - each U-check's answer.
- **Ticket comments.** At Tasks 0, 5, 8, 10 and review-ready (Task 13).
  - Run `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M784Y94Q3AXT6XJX38F5VK1V --non-interactive --json --request-id <ULID> --input <file>`
    from the main checkout, only while the ticket worktree is clean.
  - The input is `{"comment_id": "<ULID>", "body": "…"}`. Take ULIDs from
    `manyhands-cli id new`.
  - The comment publishes this ticket's branch.
- **Ignored files at close.** Building in the worktree creates `target/` and
  `.devenv/` there, and a later `ticket close` stops on them. Leave them for
  now: the close is not authorized.
- **The on-screen checklist** (Task 13) is the owner's to run, including the
  desktop smoke test under Wayland. Ask; never assume a result.

## Stop And Ask The Owner When

- a contract question comes up that the Cycle, design and plan do not
  settle;
- a row of the N- or T-table answers differently from the code, or cannot
  be expressed against it (Task 5's design checkpoint);
- Task 0 finds a fact changed since `81789331`, or `Cargo.lock` moved
  `gpui-kit`, `gpui-base` or `gpui-component`;
- U-1 finds a library type without a needed derive;
- a source row (design A2) cannot be built from the inputs the design
  names;
- a test outside the E-list breaks and is neither an appearance test nor a
  needed edit;
- something outside the tests breaks that no task names;
- a library gap is found, or a new ticket seems needed;
- another Cycle that touches the footer, Activity or the operations area
  reaches main while this Cycle is in flight. Wave 04 Cycle 04 (Settings
  sheet) is the likeliest.

## Handoff Prompt

Paste this into the fresh session, started in the main checkout
`/home/vhodges/work/src/manyhands`:

```text
Implement Wave 04 Cycle 03b (Activity And Needs Attention), ticket
01M784Y94Q3AXT6XJX38F5VK1V (vh-dt0aq). The owner approved its Cycle document,
design and plan on 2026-11-13 and chose subagent-driven, serial execution.

Work only in the ticket's worktree:
/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M784Y94Q3AXT6XJX38F5VK1V
on branch manyhands/ticket/01M784Y94Q3AXT6XJX38F5VK1V.

Start by reading the execution brief in that worktree,
docs/plans/2026-11-12-wave-04-cycle-03b-handoff.md, in full. It says what is
authorized and what is not, what to read and in what order, the binding
rules, and when to stop and ask me. Then use the project skill
.agents/skills/implementing-a-cycle/SKILL.md together with
superpowers:subagent-driven-development: one fresh implementer per task, then
a fresh reviewer, one task at a time, Tasks 0 to 13 of the plan. You, the
orchestrator, alone write the execution ledger and ticket comments and push
the ticket branch after each task.

Task 5 has a design checkpoint: its implementer writes the attention table
test (design A3, N1-N25 with its precedence and latches) and the log table
test (design A4, T1-T18) before any code. Revision 5 of the design had no
independent review, so if a row cannot be expressed against the code, or the
code would answer it differently, stop and bring it to me.

Pushing branches is fine, this one after each task and others when a
comment on another ticket publishes them; say which. Not authorized without
asking me at the time: a pull request, merging into main, closing the
ticket, worktree cleanup, native CI, or creating tickets.
```
