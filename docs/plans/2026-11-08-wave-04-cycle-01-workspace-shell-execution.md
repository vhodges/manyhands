# Wave 04 Cycle 01 Execution Ledger

The orchestrator's record for
[the plan](2026-11-08-wave-04-cycle-01-workspace-shell-implementation.md):
task state, commit ranges, decisions, review findings and verification. It
is written only by the orchestrator. Implementers never edit it.

## Authorization

- **Documents approved 2026-11-08.** The owner wrote: "Docs are Approved,
  tests are approved. Note if testing scope need to add additional existing
  tests, that is pre-authorized." The approval commit is `8b9cebd5`.
  - Named test edits E-1 to E-10 are approved.
  - Decision 17: any further edit to an existing test that this Cycle needs
    is pre-authorized. Each is recorded here as E-11 onward, with its
    reason, and checked by the task's reviewer. An edit that removes an
    assertion is called out here and in the review-ready summary.
- **Implementation authorized 2026-11-08,** recorded at `f307a05d`:
  "subagent-driven, implementation authorized, keep the quit buttons."
  Decisions 18 and 19: serial, one implementer per task, then a fresh
  reviewer, one at a time.
- **Execution brief 2026-11-08** (owner): the orchestrator writes this
  ledger and the ticket comments and pushes the ticket branch only.
  Implementers never edit the ticket, comment or ledger files and never
  dispatch subagents.
- **Still gated:** any push other than the ticket branch, a PR, the merge,
  closing the ticket, cleanup and native CI dispatch. Each needs the owner's
  own go-ahead.
- **Stop conditions** (brief): a contract, behaviour or scope question the
  documents do not settle; a test failing because product behaviour would
  have to change beyond the design; U-4 failing in Task 4b; a move of the
  `gpui-kit`, `gpui-base`, `gpui-component` or `zorite-editor` versions in
  `Cargo.lock`; D6 merging on main; anything needing a file outside the
  Boundary.

## Named edits (E-list)

E-1 to E-10 are the design's table. Edits beyond them are recorded here.

| E | Task | Test | Reason | Assertion removed? | Reviewer |
| --- | --- | --- | --- | --- | --- |
| E-1 to E-10 | see the design | approved | approved | E-3 removes assertions (approved) | per task |

## Follow-ups

- **Push by `comment add` (Task 0, disclosed).** See Task 0, Step 4.

## Base

- **Main.** `origin/main` is `8da9afd4`. It was merged into the ticket
  branch as `cc2af8d0` during planning.
- **Task 0 merge.** `git fetch origin` and `git merge origin/main` on
  2026-11-08: already up to date, `8da9afd4` is an ancestor of HEAD.
  HEAD before and after: `737c4669`. No conflicts.
- **Local main** has no unpushed commits (`git log origin/main..main`
  printed nothing).
- **Wave 03 D6** (`01M4CC0VN3VPVVQ3RNWJKXWEVX`) is not on `origin/main`
  (`c3586ac0` is not an ancestor).

## Task 0: Baseline, References, Ledger And Follow-Ups

**Step 1, baseline.** The run started at `737c4669`; the tree differed
only in documentation.

| Check | Result |
| --- | --- |
| `cargo check --all-features --locked` | pass (50 s) |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | pass (59 s) |

No test was run: the plan runs none at Task 0.

**Step 2, facts.** `git diff --stat 2b1dc4d5 HEAD -- src/desktop docs/Waves
docs/RFC docs/design Cargo.lock` shows only `docs/Waves/wave-04-workspace-design.md`,
`docs/design/02-layout-and-docking.md` and `docs/design/08-gpui-kit-mapping.md`,
which are the approval commit's Amendments. Nothing in `src/desktop` or
`Cargo.lock` moved, so facts 1–34 stand and the Kit versions are unchanged.

**Step 3, sweep.** Every count matches the plan.

| Command | Result |
| --- | --- |
| `Region::Navigation` / `region-navigation` | 8 files: `find_tests` 2, `item/page_tests` 4, `keyboard_tests.rs` 12, `actions` 2, `editing` 8, `frame` 2, `made_ready_tests` 1, `workspace_tests` 3 |
| `pane-tickets` / `pane-documents` / `pane-showing` / `SHOWING_*` | 7 files: `create_tests` 7, `keyboard_tests.rs` 10, `editing` 2, `made_ready_review_tests` 1, `operations_tests` 2, `tabs_tests` 1, `tickets_tests` 11 |
| `simulate_resize` in `src/desktop` | `harness.rs:119` and E-10's ten sites |

No new test site, so no E-11 from the sweep.

**Step 4, approval follow-ups.**

- **Amendments 1–9 and 11** are in the Wave and the handoff (checked
  against `git diff 2b1dc4d5 HEAD -- docs/Waves docs/design`).
- **Amendments 10 and 11, ticket edits.** Committed as `85f1d37c`: the
  Scope Summary's rows line defers to decision 11; the D6 Coordination's
  "attention entries" becomes the `needs_attention` function; "Quit feedback
  stays visible" becomes "Quit feedback stays in Activity" (decision 7,
  revised). `ticket save` (request `01M6Z89CF549ATSHEQJH6YYM7Y`) answered
  `noop`, `already_applied`: the committed `ticket.md` was already the
  canonical state, and `ticket show` reads both edits.
- **Note on D6's ticket.** D6's worktree was clean at HEAD `c3586ac0`, so
  the note went in with `comment add` (comment `01M6Z8A8AYDGTK4BWKZR3931ST`,
  request `01M6Z8A8CJ4DTYPZ95TG3QEKC8`), checkpoint `b05c86c6`. It records
  the `needs_attention` rule, the unchanged operation files, the quit
  feedback, and the files where a textual conflict is expected.
  - **Disclosure.** The result reported `publication: published`: the CLI
    fast-forwarded D6's ticket branch on `origin` from `c3586ac0` to
    `b05c86c6`. That commit adds only the comment file. The brief
    authorizes pushing this Cycle's ticket branch only, and the file-ticket
    skill says a request authorizes the CLI's local checkpoint, not
    publication. The push was a side effect of the command Task 0 Step 4
    names, not a separate push, and nothing was forced. It is not undone.
    The owner is told in the Task 0 report. No further command will run
    against another ticket.

**Step 5.** This ledger; commit and the ticket comment follow.

## Task 1: Chrome Tokens

- **Commit** `a9caf276` (range `98b810b1..a9caf276`): `shell.rs`,
  `shell/tokens.rs` (`ChromeColors`, `colors(dark)`, `chrome_colors(cx)`),
  `view/mod.rs`, `item/tokens.rs` (accent and faint read from the chrome
  tokens), `item/tokens_tests.rs`.
- **E-7** applied: `desktop::view::item::tokens::tests::header_colours_follow_dark_and_light_and_the_page_stays_white`
  gains the accent/faint comparisons and two `chrome_colors(cx)` checks.
  Purely additive, no assertion removed. Red first: compile failure
  `unresolved import crate::desktop::view::shell`.
- **Tests run:** that test, 1 run, 1 passed (implementer and reviewer).
- **Verify:** fmt, check, both clippy lines clean. Both Boundary commands
  print nothing; `Cargo.lock` untouched.
- **Deviation:** a per-mode accessor `colors(dark)` beside `chrome_colors(cx)`,
  as `item::tokens::colors` has, so the test compares without a `cx`. The
  not-yet-used fields carry `Called from Task N` labels: 4c (`panel`, `muted`,
  `selection`), 6 (`chrome`, `border`, `border_strong`, `hover`, `secondary`,
  `text`, `chrome_colors`), 7 (`teal`, `green`, `red`). Later tasks may
  correct the labels.
- **Review:** APPROVED, no findings. All 28 hex values checked against `07`.
- **E-11 candidates:** none.

## Task 2: Layout Model

- **Commits** (range `80107470..500ffd72`): `0db81083` adds `model/layout.rs`
  (443 lines), `model/layout_tests.rs` and the `mod layout;` line;
  `500ffd72` is the review fix.
- **API for later tasks** (all `pub(crate)`): `PanelKind` (`ALL`, `name`,
  `from_name`, `label`, `short_label`, `icon`), `ITEM_AREA_NAME`, `Side`,
  `Group`, `Arrangement`, `Stop`, `Layout` with queries (`is_on`, `is_shown`,
  `is_open`, `is_displayed`, `has_panels`, `side_of`, `groups`,
  `displayed_in`, `size`) and operations (`toggle_panel`, `toggle_dock`,
  `reveal`, `move_to`, `set_arrangement`), `f6_stops`, `arrangement_from`,
  `DEFAULT_LEFT_SPLIT = [0.6, 1.4]`. `set_arrangement` is the R-6/R-36 hook;
  it reveals any panel whose side changed.
- **Tests** (new, each run once by exact name, 1 test, passed):
  `desktop::model::layout::tests::the_default_is_1b`,
  `…::operations_follow_the_rules` (one table, 26 rows),
  `…::f6_stops_follow_the_arrangement` (10 rows plus two `arrangement_from`
  dumps). Red first: the target did not compile against the empty module.
- **Review, round 1:** CHANGES REQUESTED. Important: the P7 and P9 rows
  could not fail (deleting the `has_panels` or `side_of == Some(side)` guard
  left them green). Minor: `arrangement_from`'s skip/remap behaviour was
  unpinned; icon and short-label strings were asserted (appearance, AGENTS.md);
  blanket dead-code labels. **Fix** `500ffd72`: P7 starts with the empty side
  hidden; two P9 rows; a second `arrangement_from` dump (empty group, bare root
  leaf, unknown leaf selected); icon/short-label assertions dropped. The
  implementer showed by temporary mutation that each row fails without its
  guard, and that the reviewer's suggested `MoveTo('A','b')` row does not
  discriminate (replaced).
- **Review, round 2:** APPROVED, nothing found.
- **Removed assertions:** only the icon and short-label strings, which were
  this task's own new assertions (never an existing test).
- **Deviation:** `icon()` and `short_label()` carry plain
  `#[allow(dead_code)] // Called from Task 4c.`; the `cfg_attr(not(test))`
  form fails `--all-targets` clippy once the test no longer calls them. Task
  4c removes both attributes.
- **Interpretations accepted by the reviewer:** hiding the active tab selects
  the next panel that is on, wrapping; "active" means the panel the group
  displays (the Kit's fallback); a source dock left empty keeps its `shown`
  flag (never visible; a later move reveals it); `arrangement_from` skips
  non-panel leaves, drops empty groups and remaps the active index; `has_panels`
  is the "no toggle" predicate.
- **Carried to later tasks (observations):** no dock-size setter yet (Task 5's
  `set_dock_size` and the R-37 clamp need one); Task 9's `restore` needs a
  constructor from `on`, `shown`, sizes and arrangement, and an `on`/`shown`
  snapshot; `layout.rs` and its tests are near 500 lines, so Task 9 should put
  `restore` in a submodule such as `layout/restore.rs`; `move_to` mirrors the
  Kit move in the pure model, so Task 5 must keep the two in agreement.
- **E-11 candidates:** none. Boundary commands print nothing.
