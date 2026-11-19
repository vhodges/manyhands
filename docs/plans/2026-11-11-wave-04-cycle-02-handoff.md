---
title: "Wave 04 Cycle 02 Execution Handoff"
date: 2026-11-11
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M75J41PKWM0QFT9RFNA6M4CF"
---

# Wave 04 Cycle 02 Execution Handoff

The brief for the session that implements Wave 04 Cycle 02, Tickets And
Documents Panels.

## Where

- **Ticket:** `01M757DHFAVPEVY4TWQTVKQAMJ` (`vh-r2n89`).
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M757DHFAVPEVY4TWQTVKQAMJ`.
  Work only there.
- **Branch:** `manyhands/ticket/01M757DHFAVPEVY4TWQTVKQAMJ`, pushed to
  `origin`.
- **HEAD at handoff:** the commit that adds this file, on top of `ceaeb9d7`
  (the approval commit). Confirm it with `git log --oneline -3`.
- **Main at handoff:** `origin/main` at `48653d01`. The branch was created
  from it and holds it. Cycle 01 (`eb0798a2`) and Wave 03 D6 (`48653d01`)
  are both merged there.

## Authority

On 2026-11-11 the product owner approved the three documents (revision 3
with review round 3's fixes): the Cycle document, the design and the
implementation plan. The owner wrote: "Accept Decision 10. Documents are
approved. Implementation is approved. Subagent driven development but
prepare a brief to hand to a fresh session."

The approval covers:

- decisions 1–20, proposed by the planner and confirmed by the owner,
  with decision 10 as narrowed in revision 3 (unreadable files show as
  rows but do not open; design R-38), and decision 21 (implementation);
- rulings R-1 to R-38 and the Required Outcomes P1–P32;
- named test edits E-1 to E-14. **Any further edit to an existing test that
  execution finds necessary is pre-authorized** (decision 16): record each
  in the ledger as E-15 onward, with its reason; the task's reviewer checks
  it; an edit that removes an assertion is called out in the ledger and in
  the review-ready summary.

The approval commit, `ceaeb9d7`, applied the design's Document Amendments
1–6: the Wave's Cycle 02 section and Deferred Work, Cycle 01 decision 4,
handoff `03` and `08`, and the ticket relationships RFC.

- **Execution:** subagent-driven and serial. Follow
  `.agents/skills/implementing-a-cycle/SKILL.md` (and the
  `superpowers:subagent-driven-development` skill, if your harness has it).
  - Each task goes to a fresh implementer, and a fresh reviewer checks it
    before the next task starts.
  - Implementers never dispatch subagents and never edit ticket, comment or
    ledger files; the orchestrator does.
  - The plan's checkpoints (Tasks 0, 4 and 9) each have a ticket comment.
- **Authorized:** implementation of Tasks 0–9, including the test edits
  above, and pushing this ticket branch after each task's commit (owner,
  2026-11-11: "changes on the branch should be being pushed to origin so
  others who are not local can review").
- **Not authorized.** Each needs the owner's own go-ahead, asked at the
  time:
  - a push of any branch other than this ticket's;
  - a pull request, a merge, closing the ticket, or worktree cleanup;
  - dispatching native CI;
  - creating a ticket, for example for a library gap;
  - `ticket save` or `comment add` on any other ticket. `comment add`
    publishes the target ticket's branch and checkpoints into its worktree.

## Read first, in order

1. `git show origin/main:AGENTS.md`. Take the current copy from main, not
   the worktree's. Its Testing Strategy (2026-11-06) governs every test run.
2. `.manyhands/tickets/01M757DHFAVPEVY4TWQTVKQAMJ/ticket.md` and every comment
   in `.manyhands/comments/01M757DHFAVPEVY4TWQTVKQAMJ/`, oldest first.
3. `docs/Cycles/wave-04-cycle-02-tickets-and-documents.md`, the Cycle
   document: Scope 1–10, Required Contract 1–8, Acceptance A1–A15,
   decisions 1–21, Known Limits.
4. `docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-design.md`,
   the design. In particular:
   - Baseline Facts 1–33 (at `a0a734a4`; `src/` is unchanged since);
   - Parts A–I, especially Part A's state table, Part B's `derive` rules,
     Part D's Enter rules, Part G's focus rules and Part I's read;
   - Required Outcomes P1–P32, the Testing table, Named Edits E-1 to E-14,
     Rulings R-1 to R-38, Known Limits K-1 to K-14 and U-3.
5. `docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-implementation.md`,
   the plan: Global Constraints, Review Focus, the File Map and Tasks 0–9.
6. `docs/Waves/wave-04-workspace-design.md`, section Cycle 02 (amended at
   approval) and Owner Decisions 1–10.
7. `docs/Cycles/wave-04-cycle-01-workspace-shell.md` and
   `docs/plans/2026-11-08-wave-04-cycle-01-workspace-shell-design.md`:
   Cycle 01's rulings and contracts still hold (its R-1, R-9, R-22, R-29,
   decision 9, contract 4 and 7 are the ones this Cycle leans on).
8. `docs/design/03-panels.md`, `06-language-and-states.md`,
   `07-design-tokens.md`, and the prototype source
   `docs/design/mockups/source/Workspace.dc.html` for exact values.
9. `.agents/skills/implementing-a-cycle/SKILL.md` and its references, for
   the worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan.

## State at handoff

- **No code yet.** Nothing was built or tested during planning. Task 0 runs
  the static gates only; the fast tier runs once, at review-ready (Task 9).
- **In flight beside this Cycle:** none in Wave 04. Cycle 03 (Repositories,
  Activity and Needs attention) is not started; it will share the panel
  header, its focus rule and `ListRow`. If it reaches main first, stop for
  a short checkpoint with the owner (plan, Global Constraints).
- **Unverified at handoff:** U-3 only, that the header can draw
  Workspace-built base buttons whose focus counts as the panel's region.
  Task 8 proves it, or stops for the owner (decision 18's alternative is a
  toolbar row in the panel body).
- **Planning lessons the plan carries as rules:**
  - The test snapshot cannot read an accessible description; tooltip and
    description text is tested through its pure function.
  - Cycle 01's keyboard-walk helper `ringed` reads the region element's
    own focus; E-14 widens it for header toolbar buttons.
  - Retry must go through `reload_lists`; `load_lists` clears first.
  - `set_selected_item` and `reveal_item` on the Kit tree expand ancestors
    and emit `Expanded`; restore selection with
    `set_selected_index(index_of(…))`.
  - The unreadable-file fixture writes an invalid-UTF-8 `.md` in a raw
    ticket directory with no item worktree (discovery drops primary-copy
    problems inside a ticket directory that has one).
  - No seam fails a read outright: use `DelayingBackend::replace_service`,
    or the model tables, as the plan says; a state no seam can make stops
    the task for the owner.

## Binding rules

- **Cargo.** Run Cargo only through Devenv, with `--locked`.
- **Boundary.** No task changes anything under `src/` outside
  `src/desktop/`, nor `Cargo.toml`, `Cargo.lock` or `packaging/`. After each
  task's commit,
  `git diff --stat origin/main...HEAD -- src ':!src/desktop' Cargo.toml Cargo.lock packaging`
  prints nothing. A library gap stops the task: it needs a ticket, which
  needs the owner's go-ahead.
- **Desktop code** uses GPUI only through `gpui_kit::*`, never as a glob.
- **Tables first.** Each model table in the plan is written as a test
  before its code. If the code answers any row differently, stop for a
  ruling. Never edit a table to fit.
- **Test runs** (AGENTS.md Testing Strategy, which overrides any broader
  run):
  - between steps, `cargo check` or `clippy` and only the tests just written
    or changed, by exact full module path with `-- --exact`, and check that
    more than zero tests ran;
  - no whole desktop bin suite between tasks;
  - the static gates and the fast tier run once, at review-ready (Task 9).
    No SSH target: nothing under its paths changes;
  - no `sleep` to pass time: inject the clock or the duration;
  - stop and narrow if a command runs past about 15 minutes, and record
    which tests ran at each checkpoint;
  - never build while a test run is in progress.
- **No appearance tests.** Nothing asserts colours, sizes, icons, tones or
  that a view renders. An appearance test the redesign breaks is deleted
  under E-13, named in the ledger.
- **Merging.** Merge `origin/main` at Task 0, at Task 5 and whenever main
  moves. Run `git merge-tree --write-tree HEAD origin/main` before each
  checkpoint. Never rebase: the branch is pushed.
- **Pushing.** After each task's commit, publish the ticket branch from the
  main checkout:
  `devenv shell -- cargo run -q --locked --bin manyhands-cli -- item sync --repo /home/vhodges/work/src/manyhands --id 01M757DHFAVPEVY4TWQTVKQAMJ --non-interactive --json --request-id <ULID>`,
  and check that `HEAD` equals `origin/manyhands/ticket/01M757DHFAVPEVY4TWQTVKQAMJ`.
  It answered `publication: published` during planning.
- **The real registry.** Run `manyhands-cli` against real data only from the
  up-to-date main checkout (`/home/vhodges/work/src/manyhands`), never a build
  of this branch. Fast-forward main before each use.
- **Commits.** One per task, conventional message, ending with your own
  session's `Co-Authored-By` trailer.
- **Execution ledger.** Task 0 creates
  `docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-execution.md`,
  and every task commits it. Record each ruling there as "Ruling: decision —
  why — cost if wrong", and every named or further test edit (E-n) with its
  reason.
- **Ticket comments.** At Task 0, Task 4 and review-ready (Task 9), with
  `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id
  01M757DHFAVPEVY4TWQTVKQAMJ --non-interactive --json --request-id <ULID>
  --input <file>` from the main checkout, only while the ticket worktree is
  clean. The input is `{"comment_id": "<ULID>", "body": "…"}`. Take ULIDs
  from `manyhands-cli id new`. The comment publishes this ticket's branch.
- **Ignored files at close.** Building in the worktree creates `target/` and
  `.devenv/` there; a later `ticket close` stops on them. Leave them for
  now; the close is not authorized.
- **The on-screen checklist** (Task 9) is the owner's to run. Ask; never
  assume a result.

## Stop and ask the owner when

- a contract question comes up that the Cycle, design and plan do not
  settle;
- a row of a model table answers differently from the code;
- Task 0 finds a changed fact, or `Cargo.lock` moved `gpui-kit`,
  `gpui-base` or `gpui-component`;
- U-3 fails in Task 8 (the header cannot draw the toolbar or count its
  focus as the region);
- a list state of Part A cannot be made in a test with the existing seams;
- a test outside the E-list breaks and is neither an appearance test nor a
  needed edit;
- something outside the tests breaks that no task names;
- a library gap is found, or a new ticket seems needed;
- Wave 04 Cycle 03 reaches main while this Cycle is in flight.
