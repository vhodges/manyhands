---
title: "Wave 04 Cycle 03a Execution Handoff"
date: 2026-11-12
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M78J1CSKC1ZYTFPZAJTCVAPG"
---

# Wave 04 Cycle 03a Execution Handoff

The brief for the session that implements Wave 04 Cycle 03a, Repositories
Panel And Sharing Status.

## Where

- **Ticket:** `01M784Y8C73B39GR4JWENXE5D0` (`vh-3jpbf`).
- **Worktree:** `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M784Y8C73B39GR4JWENXE5D0`.
  Work only there.
- **Branch:** `manyhands/ticket/01M784Y8C73B39GR4JWENXE5D0`, pushed to
  `origin`.
- **HEAD at handoff:** the commit that adds this file, on top of
  `1b68793d` (the approval commit). Confirm it with
  `git log --oneline -3`.
- **Main at handoff:** `origin/main` at `37be101d`. The branch was created
  from it and holds it. Cycle 01 (`eb0798a2`), Cycle 02 (`37be101d`) and
  Wave 03 D6 (`48653d01`) are merged there.

## Authority

On 2026-11-12 the product owner approved the three documents (revision 4):
the Cycle document, the design and the implementation plan. The owner
wrote: "Docs are approved. We'll use subagent-driven development (there
should be a skill for this). Prepare the brief and handoff prompt."

The approval covers:

- decisions 1–25 in the Cycle document: 1–7, 19–22 and 24 answered by the
  owner, 8–18 and 23 proposed by the planner and confirmed, 25 the
  execution (decisions 9, 11 and 16 as reworded in revision 3);
- rulings R-1 to R-45, the Required Outcomes P1–P44, and the unshared
  list's table U1–U15 (design Part A);
- named test edits E-1 to E-16. **Any further edit to an existing test that
  execution finds necessary is pre-authorized** (decision 17): record each
  in the ledger as E-17 onward, with its reason; the task's reviewer checks
  it; an edit that removes an assertion is called out in the ledger and in
  the review-ready summary.

The approval commit, `1b68793d`, applied the design's Document
Amendments 1–8: the Wave's Cycle 03 split into 03a and 03b (and its
design-section table, Cycle 05's empty item area and the downstream
obligations), handoff `03`, `02`, `06` and `08`, the Cycle 01 document,
and the Wave 03 D6 design.

- **Execution:** subagent-driven and serial (decision 25). Follow
  `.agents/skills/implementing-a-cycle/SKILL.md` and the
  `superpowers:subagent-driven-development` skill.
  - Each task goes to a fresh implementer, and a fresh reviewer checks it
    before the next task starts.
  - Implementers never dispatch subagents and never edit ticket, comment or
    ledger files; the orchestrator does.
  - **Task 1 has a design checkpoint.** Its implementer writes
    `unshared_follows_attempts_shares_and_saves` from design Part A's table
    U1–U15 before any of Task 1's code. If any row cannot be expressed
    against the code, the implementer stops and reports; the orchestrator
    brings it to the owner before anything is implemented.
  - The plan's checkpoints (Tasks 0, 5 and 10) each have a ticket comment.
- **Authorized:** implementation of Tasks 0–10, including the test edits
  above, and pushing this ticket branch after each task's commit (owner,
  2026-11-11: "changes on the branch should be being pushed to origin so
  others who are not local can review").
- **Not authorized.** Each needs the owner's own go-ahead, asked at the
  time:
  - a push of any branch other than this ticket's;
  - a pull request, a merge, closing the ticket, or worktree cleanup;
  - dispatching native CI;
  - creating a ticket, for example for a library gap;
  - `ticket save` or `comment add` on any other ticket, including the
    closing notes on `01M654G5VGQMXTH3BX9GKE7JEY` (vh-d78zh, folded in) and
    `01M5CS4MH25QQ769HV61ZRKHXG` (D1 polish) that the plan's last step
    names. `comment add` publishes the target ticket's branch and
    checkpoints into its worktree.

## Read first, in order

1. `git show origin/main:AGENTS.md`. Take the current copy from main, not
   the worktree's. Its Testing Strategy (2026-11-06) governs every test run.
2. `.manyhands/tickets/01M784Y8C73B39GR4JWENXE5D0/ticket.md` and every comment
   in `.manyhands/comments/01M784Y8C73B39GR4JWENXE5D0/`, oldest first.
3. `docs/Cycles/wave-04-cycle-03a-repositories.md`, the Cycle document:
   Scope 1–9, Required Contract 1–8, Acceptance A1–A13, decisions 1–25,
   Known Limits.
4. `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-design.md`, the
   design. In particular:
   - Baseline Facts 1–70 (1–32 at `7034522f`, 33–70 found in review; `src/`
     is unchanged since `7034522f`);
   - Part A's rules and its table U1–U15, `status::stored_applies`, and the
     scheduler's side maps;
   - Part B's state table and its order (decision 8), the tooltip lines;
   - Part C's own row element, the kept row and the titles rules;
   - Part D's menu items, guards, the choice checked again, and its focus
     table; Part E's context and Check all rules; Part G's sum and focus;
     Part I's sweep;
   - Required Outcomes P1–P44, the Testing table, Named Edits E-1 to E-16,
     Rulings R-1 to R-45, Known Limits K-1 to K-11 and U-1 to U-7.
5. `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-implementation.md`,
   the plan: Global Constraints, Review Focus, the File Map and Tasks 0–10.
6. `docs/Waves/wave-04-workspace-design.md`, section Cycle 03a (amended at
   approval), Owner Decisions 1–10 and Testing.
7. `docs/Cycles/wave-04-cycle-02-tickets-and-documents.md` and its design
   (`docs/plans/2026-11-11-wave-04-cycle-02-tickets-and-documents-design.md`):
   Part G's header toolbars and decision 18, R-34 and R-37, the list-state
   helpers this Cycle reuses. `docs/Cycles/wave-04-cycle-01-workspace-shell.md`
   and its design: the footer, the dock menu, R-22, R-29, decisions 3, 7, 9
   and 14, contracts 4 and 7.
8. `docs/plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md`
   (as amended at approval): A2, A4, C4, W-5 and W-6. This Cycle reads
   D6's model and changes its Settings line (decision 10).
9. `docs/design/03-panels.md` (Repositories), `02-layout-and-docking.md`
   (Footer), `06-language-and-states.md`, `07-design-tokens.md`, and frames
   `docs/design/screenshots/1a` and `1b`.
10. `.agents/skills/implementing-a-cycle/SKILL.md` and its references, for
    the worktree preflight and delivery.

The Cycle governs the design, and the design governs the plan.

## State at handoff

- **No code yet.** Nothing was built or tested during planning. Task 0 runs
  the static gates only; the fast tier runs once, at review-ready (Task 10).
- **In flight beside this Cycle:** Wave 04 Cycle 03b
  (`01M784Y94Q3AXT6XJX38F5VK1V`, `vh-dt0aq`, Activity and Needs attention)
  is a placeholder, planned after this Cycle. Open follow-up tickets from
  D1 (`01M5CS4MH25QQ769HV61ZRKHXG`, `01M5F3GVDYTH1Q5V5FCS9YF7V3`), D4
  (`01M6H7VXJJHRYHTW9S8TJ88921`) and D5 (`01M6RAS63EWVHWWVBSZ836RXG2`) have
  no code on their branches. If 03b or any of them reaches main while this
  Cycle is in flight, stop for a short checkpoint with the owner (plan,
  Global Constraints).
- **Unverified at handoff** (the design's U-1 to U-7), each checked first
  by the task that relies on it:
  - U-1 and U-2 (Task 5): `menu` and `shift-f10` reach the `Repositories`
    context from the Kit list, and a right press reaches the row's hook.
  - U-3 (Task 9): whether a remote whose SSH URL carries a password is
    publication eligible; the privacy test covers both outcomes.
  - U-4 (Task 3): rows applied with `window.defer` after the frame.
  - U-5 (Task 8): the keyboard tests' `TAB_LIMIT` with two more footer stops.
  - U-6 (Task 1): a user-lane Check now of a locked key asks for the
    passphrase; if not, decision 9 goes back to the owner.
  - U-7 (Task 1): a `LocalContextResolve`'s envelope names its item.
- **Planning lessons the plan carries as rules:**
  - The Kit list overwrites a row's `selected` with its cursor and its
    wrapper carries `aria_selected(cursor)`, so the rows are the panel's
    own element; the chosen repository's mark keeps the id
    `("repo-selected", i)` and the label "Selected".
  - Panel bodies are built inside `workspace.update`: the Workspace
    computes the rows and pushes them; the row's pointer hook reaches the
    Workspace through a `WeakEntity` and a shared bounds map, and checks
    `modal_open` itself.
  - The test snapshot cannot read a tooltip, an accessible description or
    a menu item's disabled state, and menu separators shift indices:
    tooltip, description and enablement are tested through their pure
    functions, and window tests read menu items by label.
  - A window test with polling on must answer every poll (`answer_poll`,
    `answer_job(JobKind::PollOnce, …)` or `hold_job`), or the test double
    reaches the real library and the fixture's `example.invalid` remote.
  - `Phase::Running` is never entered in production: "checking" is
    `checking_now` or `in_flight()`. `last` is `None` at launch; only
    `last_completed` is seeded.
  - Mark staged dead code with a plain `#[allow(dead_code)] // Used from
    Task N`; the `cfg_attr(not(test))` form warns in the test build.
  - No `chmod` fixtures; use the registry SQL seam or the model tables.
  - Focus assertions read focus inside the same update that causes the
    change.

## Binding rules

- **Cargo.** Run Cargo only through Devenv, with `--locked`.
- **Boundary.** No task changes anything under `src/` outside
  `src/desktop/`, nor `tests/`, `Cargo.toml`, `Cargo.lock` or `packaging/`.
  After each task's commit,
  `git diff --stat origin/main...HEAD -- src ':!src/desktop' tests Cargo.toml Cargo.lock packaging`
  prints nothing. A library gap stops the task: it needs a ticket, which
  needs the owner's go-ahead.
- **Desktop code** uses GPUI only through `gpui_kit::*`, never as a glob.
- **Tables first.** Each model table in the plan, the unshared list's
  U1–U15 first of all, is written as a test before its code. If the code
  answers any row differently, stop for a ruling. Never edit a table to
  fit.
- **Test runs** (AGENTS.md Testing Strategy, which overrides any broader
  run):
  - between steps, `cargo check` or `clippy` and only the tests just written
    or changed, by exact full module path:
    `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <path> -- --exact`,
    and check that more than zero tests ran;
  - no whole desktop bin suite between tasks;
  - the static gates and the fast tier (with `--no-fail-fast`) run once, at
    review-ready (Task 10). No SSH target: nothing under its paths changes;
  - no `sleep` to pass time: inject the clock or the duration;
  - stop and narrow if a command runs past about 15 minutes, and record
    which tests ran at each checkpoint;
  - never build while a test run is in progress.
- **No appearance tests.** Nothing asserts colours, sizes, icons, tones or
  that a view renders. An appearance test the redesign breaks is deleted
  under E-14, named in the ledger.
- **Merging.** Merge `origin/main` at Task 0, at Task 6 and whenever main
  moves. Run `git merge-tree --write-tree HEAD origin/main` before each
  checkpoint. Never rebase: the branch is pushed.
- **Pushing.** After each task's commit, publish the ticket branch from the
  main checkout:
  `devenv shell -- cargo run -q --locked --bin manyhands-cli -- item sync --repo /home/vhodges/work/src/manyhands --id 01M784Y8C73B39GR4JWENXE5D0 --non-interactive --json --request-id <ULID>`,
  and check that `HEAD` equals `origin/manyhands/ticket/01M784Y8C73B39GR4JWENXE5D0`.
  It answered `publication: published` during planning. Devenv prints
  banner lines on stdout; take the JSON from the last line.
- **The real registry.** Run `manyhands-cli` against real data only from the
  up-to-date main checkout (`/home/vhodges/work/src/manyhands`), never a build
  of this branch. Fast-forward main before each use.
- **Commits.** One per task, conventional message, ending with your own
  session's `Co-Authored-By` trailer.
- **Execution ledger.** Task 0 creates
  `docs/plans/2026-11-12-wave-04-cycle-03a-repositories-execution.md`, and
  every task commits it. Record each ruling there as "Ruling: decision — why
  — cost if wrong", every named or further test edit (E-n) with its reason,
  and each U-check's answer.
- **Ticket comments.** At Task 0, Task 5 and review-ready (Task 10), with
  `manyhands-cli comment add --repo /home/vhodges/work/src/manyhands --id
  01M784Y8C73B39GR4JWENXE5D0 --non-interactive --json --request-id <ULID>
  --input <file>` from the main checkout, only while the ticket worktree is
  clean. The input is `{"comment_id": "<ULID>", "body": "…"}`. Take ULIDs
  from `manyhands-cli id new`. The comment publishes this ticket's branch.
- **Ignored files at close.** Building in the worktree creates `target/` and
  `.devenv/` there; a later `ticket close` stops on them. Leave them for
  now; the close is not authorized.
- **The on-screen checklist** (Task 10) is the owner's to run, including the
  desktop smoke test under Wayland. Ask; never assume a result.

## Stop and ask the owner when

- a contract question comes up that the Cycle, design and plan do not
  settle;
- a row of a model table, U1–U15 above all, answers differently from the
  code, or cannot be expressed against it (Task 1's design checkpoint);
- Task 0 finds a changed fact, or `Cargo.lock` moved `gpui-kit`,
  `gpui-base` or `gpui-component`;
- U-2 fails in Task 5 (no right press reaches a row), or U-6 fails in
  Task 1 (decision 9);
- a state of Part B or a list state of Part C cannot be made in a test
  with the existing seams;
- a test outside the E-list breaks and is neither an appearance test nor a
  needed edit;
- something outside the tests breaks that no task names;
- a library gap is found, or a new ticket seems needed;
- Wave 04 Cycle 03b, or a D1, D4 or D5 follow-up, reaches main while this
  Cycle is in flight.

## Handoff prompt

Paste this into the fresh session, started in the main checkout
`/home/vhodges/work/src/manyhands`:

```text
Implement Wave 04 Cycle 03a (Repositories Panel And Sharing Status), ticket
01M784Y8C73B39GR4JWENXE5D0 (vh-3jpbf). The owner approved its Cycle document,
design and plan on 2026-11-12 and chose subagent-driven, serial execution.

Work only in the ticket's worktree:
/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M784Y8C73B39GR4JWENXE5D0
on branch manyhands/ticket/01M784Y8C73B39GR4JWENXE5D0.

Start by reading the execution brief in that worktree,
docs/plans/2026-11-12-wave-04-cycle-03a-handoff.md, in full. It says what is
authorized and what is not, what to read and in what order, the binding
rules, and when to stop and ask me. Then use the project skill
.agents/skills/implementing-a-cycle/SKILL.md together with
superpowers:subagent-driven-development: one fresh implementer per task, then
a fresh reviewer, one task at a time, Tasks 0 to 10 of the plan. You, the
orchestrator, alone write the execution ledger and ticket comments and push
the ticket branch after each task.

Task 1 has a design checkpoint: its implementer writes the unshared-list
table test (design Part A, U1-U15) before any code; if a row cannot be
expressed against the code, stop and bring it to me.

Not authorized without asking me at the time: pushing any other branch, a
pull request, merging, closing the ticket, worktree cleanup, native CI,
creating tickets, or commenting on any other ticket.
```
