---
title: "Wave 04 Cycle 04 Execution Handoff"
date: 2026-11-13
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7AWQZFXAXX447H40X0TSC31"
---

# Wave 04 Cycle 04 Execution Handoff

This is the brief for the session that implements Wave 04 Cycle 04,
Settings Sheet.

## Where

- **Ticket:** `01M7AMAPNHHTNECXBF4CA5WPKV`.
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M7AMAPNHHTNECXBF4CA5WPKV`.
  Work only there.
- **Branch:** `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`, pushed to
  `origin`.
- **HEAD at handoff:** the branch's latest commit. The approval commit adds
  this file, and the approval's ticket comment follows it. Confirm with
  `git log --oneline -4`.
- **Main at approval:** `origin/main` at `ba95e1b7`. The branch was made
  from it and has merged nothing since.
- **Cycle 03b** (`01M784Y94Q3AXT6XJX38F5VK1V`, Activity and Needs
  attention) was in implementation at approval. **This Cycle starts only
  after 03b is closed and merged to main** (decision 1). Task 0 checks
  that first and merges main.

## Authority

On 2026-11-13 the product owner approved the three documents (revision 4):
the Cycle document, the design and the implementation plan. The approval
message asked for:

1. approval of the documents;
2. approval of the named test edits;
3. confirmation of decisions 7–11;
4. no further review of revision 4;
5. the execution method;
6. authorization to implement once 03b has merged.

The owner wrote: "All approved. Subagent driven using a brief and prompt
in fresh session that I will start after 3.3b lands on main". That is
decision 14; "3.3b" is Cycle 03b.

The approval covers:

- **Decisions 1–14** in the Cycle document:
  - 1–6, 12 and 13 were answered by the owner;
  - 7–11 were proposed by the planner and confirmed;
  - 14 is the approval itself.
- **The design's contract:**
  - **A6, where focus goes (F1–F19).** This is the contract for focus;
    the mechanism text in A2–A4 is a sketch, and A7 lists what any
    mechanism must survive;
  - rulings R-1 to R-57 (R-11 and R-37 superseded, R-49 and R-50 unused);
  - Required Outcomes P1–P25;
  - Part D's ways in, W-1 to W-35.
- **Named test edits E-1 to E-18**, including the removals marked
  **Removes**. **Any further edit to an existing test that execution finds
  necessary is pre-authorized** (decision 7):
  - record each in the ledger as E-19 onward, with its reason;
  - the task's reviewer checks it;
  - an edit that removes an assertion is called out in the ledger and in
    the review-ready summary.

The approval commit applied the design's Document Amendments 1–7:

- the Wave's Cycle 04 and Cycle 05 sections;
- the design handoff's `05` (Settings sheet, as built) and `08` (Settings
  layout);
- Wave 03 D2's design (Settings and Drafts) and D6's design (Show in
  Activity);
- Cycle 03a's decisions 15 and 20 and K-8.

**Execution** is subagent-driven and serial (decision 14). Follow
`.agents/skills/implementing-a-cycle/SKILL.md` and the
`superpowers:subagent-driven-development` skill.

- Each task goes to a fresh implementer. A fresh reviewer checks it before
  the next task starts.
- Implementers never dispatch subagents and never edit ticket, comment or
  ledger files; the orchestrator does.
- **Task 2 has a design checkpoint.** Its implementer writes the A6 rows
  Task 2 owns as tests first (P5, P22, P23), each read after a draw, before
  any focus code, and reruns P3, P6 and P14 after.
  - If a row cannot be expressed against the code, or cannot be met, the
    implementer stops and reports. The orchestrator brings it to the owner
    before anything is implemented.
  - **Why it matters:** review rounds 2 and 3 both found Blocking issues
    in the focus mechanism, and revision 4 had no independent review. The
    table is the contract; the mechanism is the implementer's to build
    against A7's pitfalls, recording in the ledger how each is met.
- **Ticket comments:** the plan's checkpoints, after Tasks 0, 2, 6 and 9,
  each get one.

**Authorized:**

- implementation of Tasks 0–9, including the test edits above, once Cycle
  03b has merged to main;
- pushing this ticket branch after each task's commit;
- pushing other branches, including through `ticket save` or `comment add`
  on another ticket, which publishes that ticket's branch (owner,
  2026-11-13: "pushing is how people and agents will collaborate in
  manyhands so it's okay to push branches"). Say in the ledger and the
  next ticket comment which branches were published.

**Not authorized.** Each needs the owner's own go-ahead, asked at the time:

- a pull request, a merge into main, closing the ticket, or worktree
  cleanup;
- dispatching native CI;
- creating a ticket, for example for a library gap.

## Read First, In Order

1. `git show origin/main:AGENTS.md`. Take the current copy from main, not
   the worktree's. Its Testing Strategy (2026-11-06) governs every test run.
2. `.manyhands/tickets/01M7AMAPNHHTNECXBF4CA5WPKV/ticket.md`, then every
   comment in `.manyhands/comments/01M7AMAPNHHTNECXBF4CA5WPKV/`, oldest
   first.
3. `docs/Cycles/wave-04-cycle-04-settings-sheet.md`, the Cycle document.
   Read in particular:
   - Scope 1–7 and the Downstream Obligations for Cycle 05;
   - Required Contract 1–9, above all 4 (focus) and 6 (the named
     behaviour changes);
   - Acceptance A1–A15;
   - decisions 1–14;
   - Known Limits.
4. `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-design.md`, the
   design. In particular:
   - **Baseline Facts 1–47**, cited at `ba95e1b7`. Task 0 re-maps them to
     03b's merged code.
   - **Part A:** the sheet's owner (A1); the one way in, its routes and
     the origin (A2); closing (A3); late landings (A4); the guard (A5);
     **A6, the focus contract, and A7, its pitfalls.**
   - **Parts B–H:** the frame and its scroll and page keys, the passphrase
     rule, the three pages and the one chosen repository, the ways in
     W-1 to W-35 with their origins, the copy table, what leaves, the
     folded-in work, keyboard and privacy.
   - **The rest:** Required Outcomes P1–P25; Testing; Named Edits E-1 to
     E-18; Rulings R-1 to R-57; Known Limits K-1 to K-7; Unverified U-3
     and U-5.
5. `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-implementation.md`,
   the plan: Global Constraints, Review Focus, the File Map and Tasks 0–9.
6. `docs/Waves/wave-04-workspace-design.md`: the Cycle 04 section (amended
   at approval), Cycle 05's common frame, Owner Decisions 6 and 10, and
   Testing.
7. **Cycle 03b, merged by then:** its Cycle document and design
   (`docs/Cycles/wave-04-cycle-03b-activity-and-needs-attention.md`,
   `docs/plans/2026-11-12-wave-04-cycle-03b-activity-and-needs-attention-design.md`)
   for `choose_activity`, the popover and its focus rules (decision 23,
   R-49, R-50, E1, E2); and its execution ledger, for what changed after
   its approval.
8. **Cycle 03a:** `docs/Cycles/wave-04-cycle-03a-repositories.md` and its
   design (the repository menu, the panel's Settings entries), and its
   execution ledger
   `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md`,
   whose R5-2 lesson shapes the one chosen repository.
9. **Cycle 01:** `docs/Cycles/wave-04-cycle-01-workspace-shell.md` and its
   design: the sheet layer, `unless_dialog`, the footer, Quit now's focus.
10. **Wave 03:** the D1 rulings in
    `.manyhands/comments/01M4CC0VMY5RBMP70QVBXWXM6A/01M5CRVEAP42DDKYBC63A66TRV.md`
    (R-B4, R-T11a to R-T17b); the D2 design's Settings and Drafts section;
    the D6 design's Checking for updates page.
11. **The design handoff:** `docs/design/05-sheets-and-dialogs.md`
    (the common frame and Settings sheet, as built), `02`, `06`, `07`,
    `08`, and frames `docs/design/screenshots/1e` and `1f`.
12. `.agents/skills/implementing-a-cycle/SKILL.md` and its references, for
    the worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan.

## State At Handoff

- **No code yet.** Nothing was built or tested during planning.
  - Task 0 runs the static gates only.
  - The fast tier runs once, at review-ready (Task 9).
- **Cycle 03b is not yet merged at handoff.** Task 0 Step 1 checks it.
- **Unverified at handoff:**
  - **U-5 (Task 0):** 03b's merged code: where `choose_activity` calls
    `attend`, `row_action` and `open_route`; that its popover closes when
    a sheet opens; that W-12 to W-35's lines and triggers still hold; and
    which 03b triggers Part D misses (they become W-36 onward).
  - **U-3 (Task 9, on screen):** the two-line sheet header.
- **Size.** Ten tasks, 0–9. There is no split line.

**Planning lessons the plan carries as rules:**

- **The Kit sheet:**
  - Root has one sheet slot; opening a sheet replaces the one there and
    keeps its recorded previous focus;
  - `close_sheet` focuses that record and ignores dialogs;
  - `on_close` runs for Escape, the close button and the scrim, but not
    for a direct `window.close_sheet`;
  - Escape's `Cancel` propagates after closing;
  - the builder runs on every frame, so state lives in entities;
  - the header's close button sits outside the body.
- **Today's `sync_sheet` closes every sheet the shown item does not own**,
  and `unless_dialog` refuses every workspace action under any sheet.
- **The Kit `Radio` cannot be focused from outside**, and a checked one has
  no click. The key radio is a wrapped div.
- **Focus reads lag a frame**, the Kit's deferred restore lands 0.25 s
  after some dialogs close, and Root's context menu lies outside the
  Workspace view (design A7).
- **Settings is reached by tests through five shared helpers** and
  `settings_shown`. `Shown::Tabs` now means Settings is closed (R-3).
- **A6 rows are read after a draw**, not inside the update that moved
  focus.
- **Window tests with polling on answer every poll.** Guard tests use
  dispatched actions, not pointer clicks, which the scrim takes.
- **Dead code staged for a later task** carries a plain
  `#[allow(dead_code)] // Used from Task N`.
- **No `chmod` fixtures, and no real waits.** Advance the test clock for
  the 0.25 s restore.

## Binding Rules

- **Cargo.** Run Cargo only through Devenv, with `--locked`.
- **Boundary.** No task changes anything under `src/` outside
  `src/desktop/`, nor `tests/`, `Cargo.toml`, `Cargo.lock` or `packaging/`.
  - After each task's commit,
    `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging`
    prints nothing.
  - A library gap stops the task. It needs a ticket, which needs the
    owner's go-ahead.
- **Desktop code** uses GPUI only through `gpui_kit::*`, never as a glob.
- **The focus table first.**
  - A6's rows are written as tests before the focus code, in Task 2.
  - If the code answers any row differently, stop for a ruling.
  - Never edit a row to fit.
- **Test runs** (`AGENTS.md` Testing Strategy, which overrides any broader
  run):
  - between steps, run `cargo check` or `clippy` and only the tests just
    written or changed, by exact full module path:
    `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`.
    Check that more than zero tests ran;
  - the plan's named module runs: Tasks 1 to 6 each run, once by module,
    the test files they touched and those Task 0's sweep listed for them;
    Task 5 also reruns every P test written in Tasks 1–4;
  - no whole desktop bin suite between tasks;
  - the static gates and the fast tier (with `--no-fail-fast`) run once,
    at review-ready (Task 9). No SSH target runs, since nothing under
    their paths changes;
  - no `sleep` to pass time: inject the clock or the duration;
  - if a command runs past about 15 minutes, stop and narrow it. Record
    which tests ran at each checkpoint;
  - never build while a test run is in progress.
- **No appearance tests.** Nothing asserts colours, sizes, icons or that a
  view renders.
- **Merging.**
  - Merge `origin/main` at Task 0, before Task 6, and whenever main moves.
  - Run `git merge-tree --write-tree HEAD origin/main` before each
    checkpoint.
  - Never rebase: the branch is pushed.
- **Pushing.** After each task's commit, publish the ticket branch from the
  main checkout:
  `devenv shell -- cargo run -q --locked --bin manyhands-cli -- item sync --repo /home/vhodges/work/src/manyhands --id 01M7AMAPNHHTNECXBF4CA5WPKV --non-interactive --json --request-id <ULID>`,
  and check that `HEAD` equals
  `origin/manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV`.
  - It merges `origin/main` into the branch first when main has moved, as
    `comment add` does. Record any such merge in the ledger and rerun the
    task's tests by name (R-41).
  - Devenv prints banner lines on stdout. Take the JSON from the last line.
- **The real registry.** Run `manyhands-cli` against real data only from
  the up-to-date main checkout (`/home/vhodges/work/src/manyhands`), never
  a build of this branch. Fast-forward main before each use.
- **Commits.** One per task, with a conventional message, ending with your
  own session's `Co-Authored-By` trailer.
- **Execution ledger.** Task 0 creates
  `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-execution.md`,
  and every task commits it. Record in it:
  - each ruling, as "Ruling: decision — why — cost if wrong";
  - every named or further test edit (E-n), with its reason;
  - Task 0's re-mapping of the design's citations and any W-36 onward;
  - how Task 2's mechanism meets each item of design A7;
  - each rewritten copy constant, old and new (Task 7);
  - each U-check's answer.
- **Ticket comments.** At Tasks 0, 2, 6 and review-ready (Task 9).
  - Run `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M7AMAPNHHTNECXBF4CA5WPKV --non-interactive --json --request-id <ULID> --input <file>`
    from the main checkout, only while the ticket worktree is clean.
  - The input is `{"comment_id": "<ULID>", "body": "…"}`. Take ULIDs from
    `manyhands-cli id new`.
  - The comment publishes this ticket's branch.
- **Ignored files at close.** Building in the worktree creates `target/`
  and `.devenv/` there, and a later `ticket close` stops on them. Leave
  them for now: the close is not authorized.
- **The on-screen checklist** (Task 9) is the owner's to run, including the
  desktop smoke test under Wayland. Ask; never assume a result.

## Stop And Ask The Owner When

- Cycle 03b has not merged to main (Task 0 Step 1);
- a contract question comes up that the Cycle, design and plan do not
  settle;
- a row of A6 answers differently from the code, or cannot be expressed
  or met (Task 2's design checkpoint);
- Task 0 finds that 03b's merged code cannot carry what the design needs
  from it (an origin through `choose_activity`, the popover closing on a
  sheet), or `Cargo.lock` moved `gpui-kit`, `gpui-base` or
  `gpui-component`;
- a test edit would change the contract rather than follow it;
- something outside the tests breaks that no task names;
- a library gap is found, or a new ticket seems needed;
- another Cycle or ticket that touches Settings, the footer or the
  workspace's sheet handling reaches main while this Cycle is in flight
  (`01M5F3GVDYTH1Q5V5FCS9YF7V3`, settings UX, is the likeliest).

## Handoff Prompt

Paste this into the fresh session, started in the main checkout
`/home/vhodges/work/src/manyhands`, after Cycle 03b has merged to main:

```text
Implement Wave 04 Cycle 04 (Settings Sheet), ticket
01M7AMAPNHHTNECXBF4CA5WPKV. The owner approved its Cycle document, design and
plan on 2026-11-13 and chose subagent-driven, serial execution, to start once
Cycle 03b (01M784Y94Q3AXT6XJX38F5VK1V) has merged to main. Check that first:
if 03b is not on origin/main, stop and tell me.

Work only in the ticket's worktree:
/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M7AMAPNHHTNECXBF4CA5WPKV
on branch manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV.

Start by reading the execution brief in that worktree,
docs/plans/2026-11-13-wave-04-cycle-04-handoff.md, in full. It says what is
authorized and what is not, what to read and in what order, the binding
rules, and when to stop and ask me. Then use the project skill
.agents/skills/implementing-a-cycle/SKILL.md together with
superpowers:subagent-driven-development: one fresh implementer per task, then
a fresh reviewer, one task at a time, Tasks 0 to 9 of the plan. You, the
orchestrator, alone write the execution ledger and ticket comments and push
the ticket branch after each task.

Task 0 merges main and re-maps the design's citations to Cycle 03b's merged
code. Task 2 has a design checkpoint: where focus goes is the design's table
A6 (F1-F19), and its implementer writes those rows as tests, each read after
a draw, before any focus code. The design's focus mechanism is a sketch and
A7 lists what any mechanism must survive. Revision 4 had no independent
review, so if a row cannot be expressed against the code, or cannot be met,
stop and bring it to me.

Pushing branches is fine, this one after each task and others when a comment
on another ticket publishes them; say which. Not authorized without asking me
at the time: a pull request, merging into main, closing the ticket, worktree
cleanup, native CI, or creating tickets.
```
