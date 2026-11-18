---
title: "Wave 04 Cycle 05b Execution Handoff"
date: 2026-11-15
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7FNDM26BTKWQ47WBHDV8NS2"
---

# Wave 04 Cycle 05b Execution Handoff

This is the brief for the session that implements Wave 04 Cycle 05b,
Close Ticket And Resolve Conflict.

## Where

- **Ticket:** `01M7D7JK5MSY6J471JEKG3CZXR` (`vh-qjw5e`).
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M7D7JK5MSY6J471JEKG3CZXR`.
  Work only there.
- **Branch:** `manyhands/ticket/01M7D7JK5MSY6J471JEKG3CZXR`, pushed to
  `origin`.
- **HEAD at handoff:** the branch's latest commit. The approval commit
  adds this file, and the approval's ticket comment follows it. Confirm
  with `git log --oneline -4`.
- **What the branch already holds:** main, and Cycle 05a's branch as it
  stood at `2c398b03` (Cycle 04's branch and 05a's Tasks 0–3), merged in
  during planning at `ac50ba1a`; main merged again at `aaf6b475`.
- **Main at handoff:** `origin/main` at `14ca0938`, where Cycle 04
  (`01M7AMAPNHHTNECXBF4CA5WPKV`) was closed and merged.
- **Cycle 05a is not merged.** Cycle 05a (`01M7D5Z7FZTGTC1HE25KQ1MYQY`,
  the common frame and the Add repository sheet) is in execution; at
  handoff its branch was at `08c7752e`, with its Task 7 done. **This
  Cycle's Task 0 does not start until Cycle 05a is closed and merged to
  main** (the ticket's Entry Gate). Task 0 Step 1 checks for "Integrate
  closed ticket 01M7D5Z7FZTGTC1HE25KQ1MYQY" in `git log origin/main`.
  If it is not there, stop and tell the owner.

## Authority

On 2026-11-15 the product owner approved the three documents (revision
4): the Cycle document, the design and the implementation plan. The
approval message asked the owner to:

1. approve the documents, without a further independent review of
   revision 4;
2. approve the named test edits E-1 to E-15;
3. confirm decisions 7–10;
4. choose the execution method, recommended subagent-driven and serial,
   in a fresh session from a handoff brief;
5. authorize implementation, to start once Cycle 05a is on main.

The owner wrote: "Recommendations, test edits, implementation and
documents are approved. Subagent driven development in a fresh session
by a hand off brief and prompt." That is decision 13.

The approval covers:

- **Decisions 1–13** in the Cycle document:
  - 1–5, 11 and 12 were answered by the owner;
  - 6 was the planner's outline, accepted ("Write the documents");
  - 7–10 were proposed by the planner and confirmed;
  - 13 is the approval.
- **The design's contract:**
  - the stages A1, the step rows A3, the footer table A7;
  - the sheet's owner, ways in and closes, B1–B3, and its focus table,
    rows H1–H16;
  - rulings R-1 to R-28 (R-6, R-7 and R-10 withdrawn);
  - Required Outcomes P1–P13.
- **Named test edits E-1 to E-15,** including E-3's assertion removal.
  **Any further edit to an existing test that execution finds necessary
  is pre-authorized** (decision 11):
  - record each in the ledger as E-16 onward, with its reason;
  - the task's reviewer checks it;
  - an edit that removes an assertion is called out in the ledger and
    in the review-ready summary.

The approval commit applied the design's Document Amendments 2–5:
the "As built, Wave 04 Cycle 05b" notes in `docs/design/05-sheets-and-dialogs.md`
(the common frame, Close ticket, Resolve conflict, and "Review changes…"
in the popover's example), the notes in `06-language-and-states.md` and
`08-gpui-kit-mapping.md`, and the dated note in Cycle 03b's design, Part
B. **Amendment 1** (the Wave's Cycle 05b section) and **Amendment 6**
(Cycle 05a's design, B2 and B3) are Task 0 Step 4, on 05a's merged
documents.

**Execution** is subagent-driven and serial (decision 13). Follow
`.agents/skills/implementing-a-cycle/SKILL.md` and the
`superpowers:subagent-driven-development` skill.

- Each task goes to a fresh implementer. A fresh reviewer checks it
  before the next task starts.
- Implementers never dispatch subagents and never edit ticket, comment
  or ledger files; the orchestrator does.
- **Task 6 has a design checkpoint.** Its implementer writes P6 (one row
  per H1–H16) and P7 first, in
  `src/desktop/view/workspace/resolve_sheet_tests.rs`, before any of
  Task 6's code. Each must fail.

  If an H row cannot be expressed against the code, or the code would
  answer it differently, the implementer stops and reports. The
  orchestrator brings it to the owner before anything is implemented.
  Never edit a row to fit.
  - **Why it matters:** revision 4 had no independent review. Its last
    changes were to how the sheet closes under a dialog (R-26) and how
    the lifecycle dialog's ways into the sheet behave under a prompt
    (R-24), and they rest on Cycle 05a's code read at `d7b39f62`, not
    yet merged.
- **Ticket comments:** after Tasks 0 and 4, at Task 6's design
  checkpoint and at its end, and at review-ready (Task 9).

**Authorized:**

- implementation of Tasks 0–9, once Cycle 05a has merged, including the
  test edits above;
- pushing this ticket branch after each task's commit (owner,
  2026-11-11: "changes on the branch should be being pushed to origin so
  others who are not local can review");
- pushing other branches, including through `ticket save` or
  `comment add` on another ticket, which publishes that ticket's branch
  and checkpoints into its worktree (owner, 2026-11-13: "pushing is how
  people and agents will collaborate in manyhands so it's okay to push
  branches"). Say in the ledger and the next ticket comment which
  branches were published.

**Not authorized.** Each needs the owner's own go-ahead, asked at the
time:

- a pull request, a merge into main, closing the ticket, or worktree
  cleanup. Task 9's close runs only after review approval, and with a
  passphrase-protected key it runs at the owner's terminal (`AGENTS.md`);
- dispatching native CI;
- creating a ticket, for example for a library gap.

## Read First, In Order

1. `git show origin/main:AGENTS.md`. Take the current copy from main,
   not the worktree's. Its Testing Strategy (2026-11-06) governs every
   test run.
2. `.manyhands/tickets/01M7D7JK5MSY6J471JEKG3CZXR/ticket.md`, then every
   comment in `.manyhands/comments/01M7D7JK5MSY6J471JEKG3CZXR/`, oldest
   first.
3. `docs/Cycles/wave-04-cycle-05b-close-and-resolve.md`, the Cycle
   document. Read in particular:
   - the Wave's "Check first", element by element;
   - Scope 1–3 and the Exclusions;
   - Required Contract 1–6;
   - Acceptance A1–A10;
   - decisions 1–13;
   - Known Limits.
4. `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-design.md`,
   the design. In particular:
   - **Baseline Facts 1–34.** Read at `ac50ba1a`, and for Cycle 05a's
     sheet checks on its branch at `d7b39f62`; facts 2, 13–24 and 31–34
     are re-mapped in Task 0 (U-1).
   - **Part A:** the stages (A1), the header (A2), the step rows (A3),
     confirm (A4), running (A5), ended (A6), the footers (A7).
   - **Part B:** the owner (B1), opening and the ways in (B2), closing
     and focus with the table H1–H16 (B3), the header and body (B4), the
     cards (B5), the footer (B6).
   - **Part C:** the copy, the field names.
   - **The rest:** Required Outcomes P1–P13; Testing; Named Edits E-1 to
     E-15; Document Amendments 1–6; Rulings R-1 to R-28; Known Limits
     K-1 to K-21; Unverified U-1, U-6, U-7, U-9, U-10; Risks.
5. `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-implementation.md`,
   the plan: Global Constraints, Review Focus, the File Map, Tasks 0–9
   and the Plan Self-Review.
6. `docs/Waves/wave-04-workspace-design.md`: the Cycle 05b section (Task
   0 amends it), its risk row for the D5 surfaces, and Testing.
7. **Cycle 05a**, from main once it has merged:
   - `docs/Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md`:
     decisions 7, 15 and 19;
   - its design: the frame (Part A), the Add sheet's owner, ways in and
     closes (B1–B3) and its focus table (B9), which this Cycle's B1–B3
     follow;
   - its execution ledger, for its rulings and what its implementers
     found, in particular `sheet_held`, `keep_focus_in_sheet`,
     `add.waiting` and `run_waiting_add_close`.
8. **Cycle 04's design** `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-design.md`:
   the sheet owner (A1, A2), the focus table F1–F19 (A6) and its pitfalls
   (A7).
9. **Wave 03 D5's design** `docs/plans/2026-11-03-wave-03-desktop-05-recovery-and-completion-design.md`:
   rulings R-2, R-3, R-8, R-20, R-27, R-31, R-34 and R-35, which this
   Cycle keeps.
10. **The design handoff:**
    - `docs/design/05-sheets-and-dialogs.md`: the common frame, Close
      ticket (2c–2e) and Resolve conflict (2g), with the as-built notes
      the approval added;
    - `06-language-and-states.md`, `07-design-tokens.md`,
      `08-gpui-kit-mapping.md`;
    - frames `docs/design/screenshots/2c-close-ticket-confirm.png`,
      `2d-close-ticket-running.png`, `2e-close-ticket-failed.png` and
      `2g-resolve-conflict-sheet.png`.
11. `.agents/skills/implementing-a-cycle/SKILL.md` and its references,
    for the worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan.

## State At Handoff

- **No code yet.** Nothing was built or tested during planning.
  - Task 0 runs the static gates only.
  - The fast tier runs once, at review-ready (Task 9).
- **Unverified at handoff** (the design's U-list). Each is checked first
  by the task that relies on it:
  - **U-1 (Task 0):** Cycle 05a's merged names: `SheetOwner::AddRepository`,
    `sheet_held`, `keep_focus_in_sheet`, `sheet_keeps_lost_focus`,
    `add.waiting`, `run_waiting_add_close`, `sheet_footer` and
    `Tone::Accent` in use, `wrapped_kit`, `show_add_repository`. Task 0
    corrects the design first, in a commit of its own. If 05a's code
    cannot carry B1–B3, stop for the owner.
  - **U-6 (Task 0):** the named edits, against 05a's merged code.
  - **U-7 (Task 0):** each production site that checks only for a
    dialog, classified as a person's action or something that opens by
    itself; `apply_review`, `close_with_draft` and `open_created` among
    them.
  - **U-9 (Task 0):** 05a's own openers that wait for no dialog also
    wait while Resolve conflict is open.
  - **U-10 (Task 5):** Repair locally…'s waiting sheet opens at the
    lifecycle dialog's first render with focus back inside it.
- **The follow-up library ticket** `01M7FBRBN04QACSCZV6KX0WMR3`
  (`vh-nkp8c`) holds line counts, conflict authors and times, and the
  merge base. This Cycle draws none of them (decision 1).

**Lessons the plan carries as rules** (design facts and the reviews of
Cycles 04, 05a and this one):

- **The Kit's Root has one sheet slot,** and dialogs draw above it.
  `sync_sheet` closes any active sheet whose owner it does not know, so
  `show_resolve` sets the owner in the same update that opens the sheet
  (R-9).
- **A Kit sheet always closes on Escape** and always draws a close
  button in its title row, a Tab stop before the body. Its `on_close`
  runs only on the Kit's own paths (Escape, the close button, the
  scrim), not on a direct `close_sheet`.
- **The Kit cannot say which dialog is on top,** and `close_dialog`
  closes the top one (R-24).
- **Root's builders run at every render,** so a dialog's header and
  `overlay_closable` follow the entity's state (R-1, R-5).
- **`focus_lost` restores the nearest tracked ancestor.** In a sheet's
  title or footer slot that is the Kit's `sheet-host` unless the slot
  tracks a handle of its own (`footer_focus`, H8).
- **A dialog with no Tab stop lets Tab out of it,** and its Escape,
  bound inside it, stops working (R-23).
- **Nothing in GPUI or the Kit scrolls a focused control into view;**
  only the app's `ContentScroll` regions do. Resolve conflict keeps
  today's scroll, which does not (K-19).
- **Snapshots have no order.** Read drawn order from bounds (P2).
- **The harness reads ids window-wide,** and the privacy sweeps file an
  element under the first `SURFACES` entry in its path; item text is
  allowed by id only (fact 33).
- **A Kit button that an H row lands on** is asserted with
  `holds_control`, not `holds_focus` (Cycle 04's final review).
- **`lifecycle-ending` is read or waited on by about 25 tests** and four
  shared helpers; it stays (R-17).
- **`defer_close_dialog` restores focus 0.25 s later.** Tests reach it
  by advancing the test clock, never by sleeping (H10).
- **Focus is read after at least one draw.** A read inside the update
  that moved focus proves nothing.
- **Guards are tested with dispatched actions,** never pointer clicks,
  which the scrim takes, and never keys bound in another context. The
  backdrop's click (P3) is the one pointer test, by `simulate_click`
  below the title bar.
- **Window tests with polling on answer every poll.** A test that reads
  a list right after a run can meet a busy read and its retry (Cycle 04
  Ruling 52).
- **Dead code staged for a later task** carries a plain
  `#[allow(dead_code)] // Used from Task N`, on items only.
- **No lane branches and no extra worktrees.** An extra worktree once
  made `ticket create` answer `recovery_required`.

## Binding Rules

- **Cargo.** Run Cargo only through Devenv, with `--locked`.
- **Boundary.** No task changes anything under `src/` outside
  `src/desktop/`, nor `tests/`, `Cargo.toml`, `Cargo.lock` or
  `packaging/`.
  - After each task's commit,
    `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging`
    prints nothing.
  - A library gap stops the task. It needs a ticket, which needs the
    owner's go-ahead.
- **Desktop code** uses GPUI only through `gpui_kit::*`, never as a
  glob.
- **Tables first.** Each table the design gives (P1, P2, P6 with the H
  rows, P9) is written as a test before its code. If the code answers a
  row differently, stop for a ruling. Never edit a table to fit.
- **Test runs** (`AGENTS.md` Testing Strategy, which overrides any
  broader run):
  - between steps, run `cargo check` or `clippy` and only the tests just
    written or changed, by exact full module path:
    `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`.
    Check that more than zero tests ran;
  - one named exception in the plan: at the end of each of Tasks 1–7,
    the orchestrator runs once, by module, the modules the task lists
    and every file Task 0's sweep listed for it;
  - no whole desktop bin suite before Task 9;
  - the static gates and the fast tier (with `--no-fail-fast`) run once,
    at review-ready (Task 9). No SSH target runs, since nothing under
    their paths changes;
  - no `sleep` to pass time: advance the test clock;
  - if a command runs past about 15 minutes, stop and narrow it. Record
    which tests ran at each checkpoint;
  - a new window test runs alone first; never build while a test run is
    in progress.
- **No appearance tests.** Nothing asserts widths, colours, tints,
  icons, dividers, the header's layout or that a view renders. An
  appearance test the redesign breaks is deleted under decision 11 and
  named in the ledger.
- **Copy.** Every user-visible string is a `pub(crate) const` (or a
  formatting `fn`), verbatim from the design or kept from today; shared
  labels have one definition (design Part C).
- **Merging.**
  - Merge `origin/main` before Tasks 0 and 5, and whenever main moves.
  - Run `git merge-tree --write-tree HEAD origin/main` before each
    checkpoint.
  - Never rebase: the branch is pushed.
- **Pushing.** After each task's commit, publish the ticket branch from
  the main checkout:
  `devenv shell -- cargo run -q --locked --bin manyhands-cli -- item sync --repo /home/vhodges/work/src/manyhands --id 01M7D7JK5MSY6J471JEKG3CZXR --non-interactive --json --request-id <ULID>`,
  and check that `HEAD` equals
  `origin/manyhands/ticket/01M7D7JK5MSY6J471JEKG3CZXR`.
  - It also merges `origin/main` into the branch when main has moved.
    Record such a merge in the ledger, re-read the merged hunks, and run
    the task's tests again by exact name.
  - Devenv prints banner lines on stdout. Take the JSON from the first
    line that starts with `{`.
- **The real registry.** Run `manyhands-cli` against real data only from
  the up-to-date main checkout (`/home/vhodges/work/src/manyhands`),
  never a build of this branch. Fast-forward main before each use.
- **Commits.** One per task, with a conventional message, ending with
  your own session's `Co-Authored-By` trailer.
- **Execution ledger.** Task 0 creates
  `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-execution.md`,
  and every task commits it. Record in it:
  - each ruling, as "Ruling: decision — why — cost if wrong";
  - every named or further test edit (E-n), with its reason;
  - the test functions of each file a scripted edit touched, before and
    after;
  - each U-check's answer;
  - which tests ran, with counts.
- **Ticket comments.** After Tasks 0 and 4, at Task 6's design
  checkpoint and its end, and at review-ready (Task 9).
  - Run `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M7D7JK5MSY6J471JEKG3CZXR --non-interactive --json --request-id <ULID> --input <file>`
    from the main checkout, only while the ticket worktree is clean.
  - The input is `{"comment_id": "<ULID>", "body": "…"}`. Take ULIDs
    from `manyhands-cli id new`.
  - The comment publishes this ticket's branch.
- **Ignored files at close.** Building in the worktree creates `target/`
  and `.devenv/` there, and a later `ticket close` stops on them. Leave
  them for now: the close is not authorized.
- **The on-screen checklist** (Task 9) is the owner's to run, including
  the backdrop on Windows and macOS if the owner has them (K-18) and the
  desktop smoke test under Wayland. Ask; never assume a result.

## Stop And Ask The Owner When

- Cycle 05a has not merged when Task 0 starts;
- Task 0 finds that 05a's merged code cannot carry B1–B3 (an owner case
  `sync_sheet` keeps, a refusal in the Ctrl+, handler and in
  `show_add_repository`, a keep-focus gate that can take a second
  predicate, a waiting close to copy);
- `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`;
- an H row cannot be expressed against the code, or the code would
  answer it differently (Task 6's design checkpoint);
- a contract question comes up that the Cycle, design and plan do not
  settle;
- a test outside the E-list breaks and fixing it would change what it
  asserts beyond what the design states;
- something outside the tests breaks that no task names;
- a library gap is found, or a new ticket seems needed;
- another Cycle that touches `src/desktop/view/lifecycle*`,
  `src/desktop/view/conflict*` or the Workspace's sheet owner reaches
  main while this Cycle is in flight.

## Handoff Prompt

Paste this into the fresh session, started in the main checkout
`/home/vhodges/work/src/manyhands`, once Cycle 05a has merged:

```text
Implement Wave 04 Cycle 05b (Close Ticket And Resolve Conflict), ticket
01M7D7JK5MSY6J471JEKG3CZXR (vh-qjw5e). The owner approved its Cycle
document, design and plan on 2026-11-15 and chose subagent-driven, serial
execution.

Work only in the ticket's worktree:
/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M7D7JK5MSY6J471JEKG3CZXR
on branch manyhands/ticket/01M7D7JK5MSY6J471JEKG3CZXR.

Start by reading the execution brief in that worktree,
docs/plans/2026-11-15-wave-04-cycle-05b-handoff.md, in full. It says what is
authorized and what is not, what to read and in what order, the binding
rules, and when to stop and ask me. Task 0 starts only once Cycle 05a
(01M7D5Z7FZTGTC1HE25KQ1MYQY) is closed and merged to main; if it is not,
stop and tell me. Then use the project skill
.agents/skills/implementing-a-cycle/SKILL.md together with
superpowers:subagent-driven-development: one fresh implementer per task, then
a fresh reviewer, one task at a time, Tasks 0 to 9 of the plan. You, the
orchestrator, alone write the execution ledger and ticket comments and push
the ticket branch after each task.

Task 0 re-maps every name the design takes from Cycle 05a against its merged
code, and stops for me if that code cannot carry the design. Task 6 has a
design checkpoint: its implementer writes the focus table tests (design B3,
H1-H16) and P7 before any code. Revision 4 of the design had no independent
review, so if a row cannot be expressed against the code, or the code would
answer it differently, stop and bring it to me.

Pushing branches is fine, this one after each task and others when a
comment on another ticket publishes them; say which. Not authorized without
asking me at the time: a pull request, merging into main, closing the
ticket, worktree cleanup, native CI, or creating tickets.
```
