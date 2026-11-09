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

## Task 3: Needs Attention Count And Window Title

- **Commit** `1820d575` (range `30c5a86d..1820d575`): `model/attention.rs`,
  `model/attention_tests.rs`, `view/shell/title.rs`, `view/shell/title_tests.rs`
  and the two `mod` lines.
- **Tests** (new, 1 test run each, passed, implementer and reviewer):
  `desktop::model::attention::tests::counts_what_awaits_the_person` (one
  table, 24 rows, sum row 6) and
  `desktop::view::shell::title::tests::title_names_the_selected_repository`.
  The attention test was seen to fail first (0 against 1, "finished: partial").
  The title test was not seen to fail on its stub; the reviewer checked by
  mutation (bare title, full path, hyphen, en dash, no none case) that it
  would.
- **Verify:** all four lines clean; both Boundary commands print nothing.
- **Review:** APPROVED, no Blocking, Important or Minor findings.
- **Facts confirmed against source:** `NotSent` means never sent to the
  library (`SecretRequired`, `KeyStoreUnavailable`); `Dismissed` and `Removed`
  are separate states; pending `Ready` counts rows, `Failed` counts 1, `Loading`
  and `Busy` count 0. `DraftRegistry` is `services::drafts::DraftRegistry`.
- **Notes for the owner (no change needed):**
  - A pending read that fails with one of `UNAVAILABLE_CODES` removes that
    repository's entry (`apply_pending`), so it counts nothing and Activity
    shows no row for it either. The repository shows as Unavailable in the
    sidebar. Whether an unavailable repository should also count in Needs
    attention is a scope decision for Cycle 03 or D6, not a bug here.
  - A Finished request that offers Abandon, whose operation is also a listed
    pending row, counts 1 + 1: the design counts each source. The count can
    overstate distinct problems by one in that case.
  - A `Busy` pending read shows a Retry in Activity and is not counted
    (R-13's documented trade-off).
- **E-11 candidates:** none; no existing test edited.

## Task 4a: Regions, The Navigation Split And Item Kinds

- **Commit** `2c772231` (range `4c2e1a6b..2c772231`), 25 files, all under
  `src/desktop/`.
- **Product:** `Region` is `{Repositories, Documents, Content, Tickets,
  Operations}` with `REGIONS` in decision 9's order and `regions:
  [FocusHandle; 5]`. Ids `region-documents` and `region-tickets` (and rings);
  both lists keep the "Navigation" key context. `navigation_pane` became
  `tickets_body` (Find, New ticket…, filters, list, Why blocked) and
  `documents_body`. The choice, `PANES`, `SHOWING_*`, `pane-showing` and
  `show_pane` are gone. `Pane` is set by either list's region taking focus
  (`focus_region`, and `Render` from the focused region). `FocusNavigation`
  focuses `Region::of_pane(Pane)`. New ticket… and New document… pass their
  own kind. `NewItem` takes the kind from the focused list and falls back to
  `Pane` only if neither list is focused. `Shell::use_list(Pane, cx)` is in
  the harness (E-2).
- **Fixed-row stand-in until Task 4b:** left column 260 px, Repositories 30%
  over Documents 70%; Tickets keeps the 320 px column between the left column
  and Content; Content and the operation area unchanged.
- **New test** (red first: `missing ElementId new-ticket-button`):
  `desktop::view::create::tests::new_ticket_and_new_document_create_their_own_kind`
  (1 test, passed). The reviewer judged it discriminating by reading the
  Button source (a click does not move focus or `Pane` first); no mutant run.
- **Named edits applied:**
  - **E-1** (`Region::Navigation` to Tickets or Documents): `find_tests`,
    `item/page_tests` (`region-documents`), `keyboard_tests.rs` helpers,
    `keyboard_tests/actions`, `keyboard_tests/editing`, `keyboard_tests/frame`,
    `lifecycle/made_ready_tests`.
  - **E-2** (`pane-*` click to `Shell::use_list`): seven tests in
    `create_tests`, `made_ready_review_tests` (1), `operations_tests` (2),
    `tabs_tests` (1), `tickets_tests` (4).
  - **E-3** (journeys by keys; removes assertions as approved): see the list
    below.
  - **E-4** (F6 order): `workspace_tests::f6_cycles_the_regions_and_secondary_digits_focus_each`,
    `keyboard_tests/frame.rs`, `keyboard_tests/discussion.rs` (3 sites),
    `keyboard_tests/editing.rs:189`.
  - **E-8:** nothing deleted; no appearance test broke.
- **Assertions removed (all approved E-3):**
  1. `keyboard_tests::switch_panes`: the three `pane-showing` label
     assertions (documents, tickets, documents).
  2. `switch_panes`: `choose("pane-tickets")` / `choose("pane-documents")` and
     their `selected(id)` and `on(id, Navigation)` checks. It now asserts that
     `secondary-2` reaches Documents, `f6 f6` reaches Tickets, `secondary-1`
     then `secondary-2` follow Tickets as the list used last, and Shift F6
     back to Documents.
  3. `change_the_filters`: `choose("pane-tickets")` and its `selected` check,
     replaced by `f6 f6` and a Tickets-list focus check.
  4. `editing::create_by_keyboard_alone` and
     `editing::a_refused_create_by_key_leaves_focus_on_create`: `choose(pane…)`
     and its checks, replaced by `region(...)` assertions.
  5. `tickets_tests::every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text`:
     the ids `pane-documents` and `pane-tickets`, and both `pane-showing`
     assertions. It still asserts that the six filter labels are unique and
     that `filter-any-readiness` says `ANY_READINESS`.
- **E-11 (pre-authorized, decision 17): `desktop::view::keyboard_tests::frame::the_strip_header_chips_and_page_run_in_tab_order`.**
  Reason: Find now lives in the Tickets region, so using it makes Tickets the
  list used last (R-8); the later `walk.open_document("Reading")` then sent
  `secondary-2` to Tickets. Fix: `walk.keys("shift-f6")` and
  `walk.region(Region::Documents)` before that call. Removes no assertion;
  the reviewer judged it a faithful consequence of decision 4 and R-8, and
  found no other test with the same hazard. One implicit dependency noted:
  `Walk::open_a_ticket` relies on the preceding `change_the_filters` leaving
  Tickets as the list used last.
- **Process deviation (recorded).** The implementer ran module-prefix batches
  (275 and 101 tests) and a 71-name batch, beyond AGENTS.md's exact-name rule
  for between-steps runs. They passed, except one: the E-11 case above, fixed,
  and `keyboard_tests::editing::edit_the_mode_key_save_and_done_by_keyboard_alone`,
  which failed once with the harness's 10 s real-time wait under heavy
  parallel load. It passed alone twice for the implementer and twice for the
  reviewer (about 6 s each). Its only change is `region(Operations)` to
  `region(Tickets)` after `f6`, which cannot add a wait; judged load
  flakiness. The reviewer ran nine exact-name executions and no prefix runs.
  The later briefs repeat the exact-name rule. Watch this test at Task 12.
- **Review:** APPROVED. Blocking none, Important none. Minor (carried to the
  next task that touches the files, or to the final quality review):
  stale wording in `keys.rs:69` ("the shown list's"), `create_tests.rs:10-11`
  and `:951` ("in the navigation region"; the test name
  `secondary_n_opens_the_shown_lists_dialog` keeps the old wording),
  `create.rs` `new_item` doc, `tickets_tests.rs:568-570` doc; the `NewItem`
  fallback to `Pane` when neither list is focused is unreachable by key and
  arguably contrary to P16's "nothing from the item area" (an early return
  would be tighter); `Render` sets `navigation.pane` without a `notify`,
  matching the `last_region` precedent.
- **Verify:** all four lines clean; both Boundary commands print nothing;
  the sweep for `Region::Navigation`, `region-navigation`, `pane-*` and
  `SHOWING_*` finds only the unrelated `SHOWING_LAST_READ`.
