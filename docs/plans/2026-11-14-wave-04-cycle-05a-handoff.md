---
title: "Wave 04 Cycle 05a Execution Handoff"
date: 2026-11-14
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M7DWN6NVW9YPFNX3GE4K2ZXF"
---

# Wave 04 Cycle 05a Execution Handoff

This is the brief for the session that implements Wave 04 Cycle 05a,
Common Frame, Add Repository And Unlock Key.

## Where

- **Ticket:** `01M7D5Z7FZTGTC1HE25KQ1MYQY` (`vh-kebv6`).
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M7D5Z7FZTGTC1HE25KQ1MYQY`.
  Work only there.
- **Branch:** `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY`, pushed to
  `origin`.
- **HEAD at handoff:** the branch's latest commit. The approval commit
  adds this file, and the approval's ticket comment follows it. Confirm
  with `git log --oneline -4`.
- **Main at handoff:** `origin/main` at `ea6d899b`, where Cycle 03b
  (`01M784Y94Q3AXT6XJX38F5VK1V`) was closed and merged. Cycles 01, 02,
  03a and 03b are on main. The ticket branch was created from
  `ea6d899b`, and main has not moved since.
- **Cycle 04 is not merged.** Cycle 04 (`01M7AMAPNHHTNECXBF4CA5WPKV`,
  the Settings sheet) is in execution; at handoff its branch was at
  `16f7746b`, with its Task 5 reviewed. **This Cycle's Task 0 does not
  start until Cycle 04 is closed and merged to main** (decision 1).
  Task 0 Step 1 checks for "Integrate closed ticket
  01M7AMAPNHHTNECXBF4CA5WPKV" in `git log origin/main`. If it is not
  there, stop and tell the owner.

## Authority

On 2026-11-14 the product owner approved the three documents (revision
3): the Cycle document, the design and the implementation plan. The
approval message asked the owner to:

1. approve the documents, without a further independent review of
   revision 3;
2. approve the named test edits E-1 to E-16;
3. confirm decisions 6–14, 19 and 20;
4. choose the execution method, recommended subagent-driven and serial,
   in a fresh session from this brief;
5. authorize implementation, to start once Cycle 04 is on main.

The owner wrote: "The docs are approved. Subagent driven development
using a handoff brief in a fresh session. Implementation is approved".
That is decision 21. The owner did not answer the named edits
separately; they are part of the approved design, and the authorized
tasks make them, so decision 21 counts them as approved with the
documents.

The approval covers:

- **Decisions 1–21** in the Cycle document:
  - 1–4 and 15–18 were answered by the owner;
  - 5 was the planner's outline, accepted ("Write the documents");
  - 6–14, 19 and 20 were proposed by the planner and confirmed;
  - 21 is the approval.
- **The design's contract:**
  - rulings R-1 to R-31;
  - Required Outcomes P1–P23;
  - the focus table B9, rows G1–G23;
  - the ways into the Add sheet AW-1 to AW-5 (B2) and the footer's
    enablement table (B10).
- **Named test edits E-1 to E-16** (E-3 unused), including the
  assertion removals they mark. **Any further edit to an existing test
  that execution finds necessary is pre-authorized** (decision 8):
  - record each in the ledger as E-17 onward, with its reason;
  - the task's reviewer checks it;
  - an edit that removes an assertion is called out in the ledger and
    in the review-ready summary.

The approval commit applied the design's Document Amendments 2 and 3:
the "As built, Wave 04 Cycle 05a" notes in `docs/design/05-sheets-and-dialogs.md`
(common frame, Add repository, Unlock key) and the mode control's note
in `08-gpui-kit-mapping.md`. **Amendment 1** (the Wave's Cycle 05
section) is Task 0 Step 4, after Cycle 04's own amendment of that
section has merged. **Amendments 4 and 5** (Cycle 04's documents, and a
comment on `01M5F3GVDYTH1Q5V5FCS9YF7V3`) are Task 10's.

**Execution** is subagent-driven and serial (decision 21). Follow
`.agents/skills/implementing-a-cycle/SKILL.md` and the
`superpowers:subagent-driven-development` skill.

- Each task goes to a fresh implementer. A fresh reviewer checks it
  before the next task starts.
- Implementers never dispatch subagents and never edit ticket, comment
  or ledger files; the orchestrator does.
- **Task 4 has a design checkpoint.** Its implementer writes P3, P4, P5,
  P6, P7, P8, P15 and P23 first, in
  `src/desktop/view/workspace/add_sheet_tests.rs`, table-driven over the
  design's B9 rows except G16 and G22 (Task 5) and G20 (Task 6), before
  any of Task 4's code. Each must fail.

  If a G row cannot be expressed against Cycle 04's merged code, or that
  code would answer it differently, the implementer stops and reports.
  The orchestrator brings it to the owner before anything is
  implemented. Never edit a row to fit.
  - **Why it matters:** revision 3 had no independent review. Cycle 04
    needed three review rounds on its own focus table, and this Cycle's
    rows rest on Cycle 04's approved design, not yet on its merged code.
- **Ticket comments:** the plan's checkpoints, after Tasks 0, 4, 7 and
  10, each get one.

**Authorized:**

- implementation of Tasks 0–10, once Cycle 04 has merged, including the
  test edits above;
- pushing this ticket branch after each task's commit (owner,
  2026-11-11: "changes on the branch should be being pushed to origin so
  others who are not local can review");
- pushing other branches, including through `ticket save` or
  `comment add` on another ticket, which publishes that ticket's branch
  and checkpoints into its worktree (owner, 2026-11-13: "pushing is how
  people and agents will collaborate in manyhands so it's okay to push
  branches"). Task 10's comment on `01M5F3GVDYTH1Q5V5FCS9YF7V3` is one.
  Say in the ledger and the next ticket comment which branches were
  published.

**Not authorized.** Each needs the owner's own go-ahead, asked at the
time:

- a pull request, a merge into main, closing the ticket, or worktree
  cleanup. Task 10's close runs only after review approval, and with a
  passphrase-protected key it runs at the owner's terminal (`AGENTS.md`);
- dispatching native CI;
- creating a ticket, for example for a library gap.

## Read First, In Order

1. `git show origin/main:AGENTS.md`. Take the current copy from main,
   not the worktree's. Its Testing Strategy (2026-11-06) governs every
   test run.
2. `.manyhands/tickets/01M7D5Z7FZTGTC1HE25KQ1MYQY/ticket.md`, then every
   comment in `.manyhands/comments/01M7D5Z7FZTGTC1HE25KQ1MYQY/`, oldest
   first.
3. `docs/Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md`,
   the Cycle document. Read in particular:
   - Scope 1–7 and the Exclusions;
   - Required Contract 1–9;
   - Acceptance A1–A13;
   - decisions 1–21;
   - Known Limits.
4. `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-design.md`,
   the design. In particular:
   - **Baseline Facts 1–41** (with 11a, 13a and 23a). They were read at
     `ea6d899b` and, for Cycle 04, on its approved design; facts 9, 16
     and 18–24 are re-mapped in Task 0 (U-1).
   - **Part A:** the frame (A1), the theme slots (A2), each dialog's
     footer (A3), Escape and focus (A4), the sheet headers (A5), inputs
     (A6).
   - **Part B:** the owner (B1), the ways in (B2), closing and coming
     back (B3), the sheet's state (B4), its sections (B5–B8), the focus
     table B9, the footer and adding (B10), Settings after the move
     (B11), keyboard (B12).
   - **Parts C–G:** sharing, Unlock, the empty pane, the copy table, the
     folded-in items.
   - **The rest:** Required Outcomes P1–P23; Testing; Named Edits E-1 to
     E-16; Document Amendments 1–5; Rulings R-1 to R-31; Known Limits
     K-1 to K-15; Unverified U-1 to U-8; Risks.
5. `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-implementation.md`,
   the plan: Global Constraints, Review Focus, the File Map, Tasks 0–10
   and the Plan Self-Review.
6. `docs/Waves/wave-04-workspace-design.md`: the Cycle 05 section (Task
   0 amends it), Owner Decisions 1–10 and Testing.
7. **Cycle 04**, from main once it has merged:
   - `docs/Cycles/wave-04-cycle-04-settings-sheet.md`: its decisions and
     its Downstream Obligations, "For Cycle 05";
   - its design `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-design.md`:
     the sheet owner and routes (A2), the focus table F1–F19 (A6), its
     pitfalls (A7), and the ways in (Part D, W-5, W-6, W-22, W-27);
   - its execution ledger `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-execution.md`,
     for its rulings, W-36 and what its implementers found.
8. **Cycle 02's segmented choice:** `src/desktop/view/tickets/filter_bar.rs`,
   which the Add sheet's mode control copies (fact 39, R-10).
9. **The design handoff:**
   - `docs/design/05-sheets-and-dialogs.md`: the common frame, Add
     repository (1d) and Unlock key (2h), with the as-built notes the
     approval added;
   - `04-item-area.md` (the empty pane);
   - `06-language-and-states.md`, `07-design-tokens.md`,
     `08-gpui-kit-mapping.md`;
   - frames `docs/design/screenshots/1d-add-repository-sheet.png` and
     `2h-unlock-key-dialog.png`.
10. `.agents/skills/implementing-a-cycle/SKILL.md` and its references,
    for the worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan.

## State At Handoff

- **No code yet.** Nothing was built or tested during planning.
  - Task 0 runs the static gates only.
  - The fast tier runs once, at review-ready (Task 10).
- **Unverified at handoff** (the design's U-list). Each is checked first
  by the task that relies on it:
  - **U-1 (Task 0):** Cycle 04's merged names, signatures and lines.
    Task 0 corrects the design first, in a commit of its own. If Cycle
    04's code cannot carry B2's or B3's steps, stop for the owner.
  - **U-3 and U-4 (Task 0):** A3's footer table against each dialog's
    code, and the module paths of the Unlock evidence tests.
  - **U-8 (Task 0):** a read double in `src/desktop/test_support/` that
    P18 can fail; else a `#[cfg(test)]` hook on the dialog.
  - **U-6 and U-7 (Task 1):** the five icons load (E-14's test proves
    it); the `ThemeConfig` keys for the secondary button, input
    background and sheet border. A slot with no key stays the Kit's
    (K-13).
  - **U-2 (Task 10):** which theme mode the app shows. The owner's
    on-screen check settles it.

**Lessons the plan carries as rules** (design facts 32–41 and Cycle 04's
A7):

- **The Kit's Root has one sheet slot,** and dialogs draw above it. The
  Add sheet replaces Settings or the details sheet, and its confirmation
  opens above it.
- **A Kit sheet always closes on Escape;** it cannot refuse. So the Add
  sheet closes on Escape as Cancel, also while a request runs, and
  nothing is cancelled (decision 7).
- **`on_close` runs only on the Kit's own Cancel, Confirm and backdrop
  paths,** not on a direct `window.close_dialog`.
- **`defer_close_dialog` restores focus 0.25 s later.** Tests reach it
  by advancing the test clock, never by sleeping; G18 keeps a late
  restore from landing behind the sheet.
- **A dialog's footer is drawn in its own body,** as the body's last
  child, never in the Kit's `footer` slot (R-19). Focus-gated closes
  and the privacy sweeps depend on it.
- **`theme.apply_config` also sets the theme's mode** to the config's.
  Apply the other mode first and the current mode last, then
  `sync_base` (R-2). Colours must be `#hex`; `rgba(…)` does not parse.
- **The Kit's `ToggleGroup` has no keyboard path,** so the mode control
  copies Cycle 02's segments (R-10).
- **A Kit `Select`'s disabled items are only drawn disabled.**
  `on_will_change` can veto a change, but `Confirm` still fires, so Add
  refuses a branch other than the checked-out one on `Confirm` too
  (R-12, P10).
- **Kit buttons take Enter and Space themselves.**
- **`ConfirmDialog` closes itself on any finish that is not stale.** An
  add's success is handled after it, with `defer_in`, and the sheet's
  close and the return to Settings wait until no dialog is open (R-25).
  Each open of the sheet has a number, and a success closes only its
  own open (R-26).
- **`Followed::Confirmed` carries no attempt,** and a finished request
  drops its envelope's scope. R-28 adds the attempt, and R-29 keeps
  `registered_root` on the desktop's `Request`.
- **Focus is read after at least one draw** (B9; Cycle 04's A6). A read
  inside the update that moved focus proves nothing.
- **Guards are tested with dispatched actions,** never pointer clicks,
  which the scrim takes, and never keys bound in another context.
- **Window tests with polling on answer every poll.**
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
- **Tables first.** Each table the design gives (P1, P2, P4, P5, P9,
  P11, P16, and B9's G rows) is written as a test before its code. If
  the code answers a row differently, stop for a ruling. Never edit a
  table to fit.
- **Test runs** (`AGENTS.md` Testing Strategy, which overrides any
  broader run):
  - between steps, run `cargo check` or `clippy` and only the tests just
    written or changed, by exact full module path:
    `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`.
    Check that more than zero tests ran;
  - one named exception in the plan: at the end of each of Tasks 1–5,
    the orchestrator runs once, by module, every test file the task
    touched and every file Task 0's sweep listed for it;
  - no whole desktop bin suite before Task 10;
  - the static gates and the fast tier (with `--no-fail-fast`) run once,
    at review-ready (Task 10). No SSH target runs, since nothing under
    their paths changes;
  - no `sleep` to pass time: advance the test clock;
  - if a command runs past about 15 minutes, stop and narrow it. Record
    which tests ran at each checkpoint;
  - a new window test runs alone first; never build while a test run is
    in progress.
- **No appearance tests.** Nothing asserts widths, colours, icons,
  dividers, the header's layout, the theme slots or that a view renders.
  An appearance test the redesign breaks is deleted under decision 8 and
  named in the ledger.
- **Copy.** Every user-visible string is a `pub(crate) const` (or a
  formatting `fn`), verbatim from design Part F or kept from today.
- **Merging.**
  - Merge `origin/main` before Tasks 0 and 7, and whenever main moves.
  - Run `git merge-tree --write-tree HEAD origin/main` before each
    checkpoint.
  - Never rebase: the branch is pushed.
- **Pushing.** After each task's commit, publish the ticket branch from
  the main checkout:
  `devenv shell -- cargo run -q --locked --bin manyhands-cli -- item sync --repo /home/vhodges/work/src/manyhands --id 01M7D5Z7FZTGTC1HE25KQ1MYQY --non-interactive --json --request-id <ULID>`,
  and check that `HEAD` equals
  `origin/manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY`.
  - It answered `publication: published` during planning.
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
  `docs/plans/2026-11-14-wave-04-cycle-05a-frame-add-repository-and-unlock-execution.md`,
  and every task commits it. Record in it:
  - each ruling, as "Ruling: decision — why — cost if wrong";
  - every named or further test edit (E-n), with its reason;
  - the test functions of each file a scripted edit touched, before and
    after;
  - each U-check's answer;
  - which tests ran, with counts.
- **Ticket comments.** At Tasks 0, 4, 7 and review-ready (Task 10).
  - Run `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id 01M7D5Z7FZTGTC1HE25KQ1MYQY --non-interactive --json --request-id <ULID> --input <file>`
    from the main checkout, only while the ticket worktree is clean.
  - The input is `{"comment_id": "<ULID>", "body": "…"}`. Take ULIDs
    from `manyhands-cli id new`.
  - The comment publishes this ticket's branch.
- **Ignored files at close.** Building in the worktree creates `target/`
  and `.devenv/` there, and a later `ticket close` stops on them. Leave
  them for now: the close is not authorized.
- **The on-screen checklist** (Task 10) is the owner's to run, including
  the theme mode (U-2) and the desktop smoke test under Wayland. Ask;
  never assume a result.

## Stop And Ask The Owner When

- Cycle 04 has not merged when Task 0 starts;
- Task 0 finds that Cycle 04's merged code cannot carry B2's or B3's
  steps (an origin for AW-3, a quiet close that empties the passphrase,
  a `Manage(root)` route, a refusal in its Ctrl+, handler);
- `Cargo.lock` moved `gpui-kit`, `gpui-base` or `gpui-component`;
- a G row of B9 cannot be expressed against the code, or the code would
  answer it differently (Task 4's design checkpoint);
- a contract question comes up that the Cycle, design and plan do not
  settle;
- a test outside the E-list breaks and fixing it would change what it
  asserts beyond what the design states;
- something outside the tests breaks that no task names;
- a library gap is found, or a new ticket seems needed;
- another Cycle that touches `src/desktop/view/settings/`, the
  Workspace's sheet owner or the dialogs reaches main while this Cycle
  is in flight.

## Handoff Prompt

Paste this into the fresh session, started in the main checkout
`/home/vhodges/work/src/manyhands`, once Cycle 04 has merged:

```text
Implement Wave 04 Cycle 05a (Common Frame, Add Repository And Unlock Key),
ticket 01M7D5Z7FZTGTC1HE25KQ1MYQY (vh-kebv6). The owner approved its Cycle
document, design and plan on 2026-11-14 and chose subagent-driven, serial
execution.

Work only in the ticket's worktree:
/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M7D5Z7FZTGTC1HE25KQ1MYQY
on branch manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY.

Start by reading the execution brief in that worktree,
docs/plans/2026-11-14-wave-04-cycle-05a-handoff.md, in full. It says what is
authorized and what is not, what to read and in what order, the binding
rules, and when to stop and ask me. Task 0 starts only once Cycle 04
(01M7AMAPNHHTNECXBF4CA5WPKV) is closed and merged to main; if it is not,
stop and tell me. Then use the project skill
.agents/skills/implementing-a-cycle/SKILL.md together with
superpowers:subagent-driven-development: one fresh implementer per task, then
a fresh reviewer, one task at a time, Tasks 0 to 10 of the plan. You, the
orchestrator, alone write the execution ledger and ticket comments and push
the ticket branch after each task.

Task 0 re-maps every name the design takes from Cycle 04 against its merged
code, and stops for me if that code cannot carry the design. Task 4 has a
design checkpoint: its implementer writes the focus table tests (design B9,
G1-G23) before any code. Revision 3 of the design had no independent review,
so if a row cannot be expressed against the code, or the code would answer
it differently, stop and bring it to me.

Pushing branches is fine, this one after each task and others when a
comment on another ticket publishes them; say which. Not authorized without
asking me at the time: a pull request, merging into main, closing the
ticket, worktree cleanup, native CI, or creating tickets.
```
