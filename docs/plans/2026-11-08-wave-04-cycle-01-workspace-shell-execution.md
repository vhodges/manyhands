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

## Task 4b Design Checkpoint (2026-11-08, at `746d5b7d`, before code)

The implementer's checkpoint (read-only phase 1, no source edit, no build),
reviewed and ruled on by the orchestrator. `origin/main` is still `8da9afd4`.
Cites use the design's abbreviations.

**1. Panel entity (R-1).** One struct, `RegionPanel` in `D/view/dock/panels.rs`,
with `Slot { Panel(PanelKind), ItemArea }`, in place of the design's five
types (ruling Q1 below). Fields: `workspace: WeakEntity<Workspace>`, `slot`,
`visible: bool`, `focus: FocusHandle` (a clone of `Workspace.regions[..]`:
Repositories, Documents, Tickets, Activity to `Region::Operations`, ItemArea to
`Region::Content`), and `_observe: Subscription`. Impls: `BasePanel`
(`panel_name` from `PanelKind::name()` or `ITEM_AREA_NAME`; `visible` returns
`self.visible`; `closable` and `zoomable` false; default `dump`), the component
`Panel` (`title` static label in 4b; `title_suffix` `None`; `zoom_control`
`None`; `title_bar` false for the item area; `tab_name` the short label),
`Render`, `Focusable` (returns `self.focus.clone()`), `EventEmitter<PanelEvent>`.
- `visible` and `focus_handle` read only the panel's own fields. The Kit calls
  them while a `DockArea` or `TabGroup` update holds those entities
  (`B/dock/panel.rs` blanket `PanelView` impl; `B/dock/tab_group.rs:~682`).
  `set_visible` is `Called from Task 5`; every panel is visible in 4b.
- `render` upgrades the weak Workspace (`Empty` if gone) and calls a new
  `pub(in crate::desktop::view)` `Workspace::<slot>_region(window, cx)` in
  `workspace/render.rs`, which wraps today's body method in `Workspace::region`
  with `.size_full()`. A nested update is legal: `view.update` ends before the
  child's layout (`G/view.rs:164-177`, `:422-445`), so the Workspace is not
  leased while a panel renders.
- Each panel observes the Workspace (`cx.observe(&workspace_entity, notify)`).
  A `notify` during draw is dropped (`G/window.rs:167-192`), as today.

**2. Builder map (R-34).** A `DockPanels(HashMap<EntityId, five handles>)`
`Global` in `D/view/dock.rs`, keyed by the dock area's `EntityId`, each handle
`panel_handle(entity)` (`C/dock/panel.rs:325`). The five names are registered
once per App on first build (`register_panel`, `B/dock/registry.rs:111`; the
builder closure captures nothing per window). The Workspace inserts its entry
in `Workspace::new` right after creating the area, before any `load`, and
removes it in an `on_release` (guarded by `has_global`) whose `Subscription`
is kept in a Workspace field. The builder reads
`ctx.dock_area().entity_id()` (a `WeakEntity`, no upgrade), so it never reads
the area or the Workspace (fact 5) and two windows never swap panels.

**3. Renderer (R-2).** Built inside the area's constructor, as
`DockSkin::dock_area` does (`C/dock/mod.rs:132-151`):
`cx.new(|acx| { let skin = DockSkin::new(acx); DockArea::new("workspace-dock",
Some(LAYOUT_VERSION), window, acx).with_renderer(Rc::new(ShellRenderer::new(skin))) })`.
`LAYOUT_VERSION = 1` for now. `ShellRenderer` forwards `frame` (id
`dock-area`), `center_frame`, `split_frame`, `render_dock` (keeps the dock-edge
handle and its resize `LayoutChanged`, `C/dock/dock.rs:55-77`, `:219-231`) and
`build_placeholder` to the inner `DockSkin`. Its `tab_group_renderer` returns a
`PanelHeader` wrapping the skin's group renderer. In 4b `PanelHeader` draws
titles only: `frame` is its own `div().id(("panel-group", node))` (not the
skin's, which carries Zoom and Close handlers, `C/dock/tab_panel.rs:634`);
`render_tab_bar` is a title row for one visible panel whose `title_bar` is true,
a plain row of titles for several, nothing for the item area;
`render_active_panel` is uncached (nothing for a collapsed group, R-35);
`render_drop_indicator` is forwarded to the skin's group renderer. Tabs, drag
and drop are Task 4c.

**4. Bookkeeping (R-33).**
- Into `activity_region`: `finishing` (`D/view/quit.rs:87`),
  `session_section`, `pending_section`, `prune_abandon_focus()` after both,
  the `operation-area` container (id kept) in `region(Operations, ..)`.
  Pruning does not run while Activity is not displayed.
- Into `item_area_region`: `item_shown`, `selection`, the content region's
  `note_focus` before `content_body`, the `content` scroll container, Root's
  sheet layer (`Root::render_sheet_layer` in `deferred`, anchored at 0,0 and
  not clipped, `B/sheet.rs:123`, `G/elements/deferred.rs:65`), and
  `region(Content, ..)`.
- Repositories, Documents and Tickets each get one body method.
- Stays in `Workspace::render`: the pre-body hooks (`sync_tickets`,
  `apply_review`, `open_deferred_republish`, `sync_sheet`), the
  `focused_region` to `last_region` and `set_pane` tracking,
  `repositories_pane.set` (every frame, so a hidden Repositories panel keeps
  its list state), the root `id("workspace")` with key context "Workspace",
  every `on_action` through `unless_dialog` plus the unguarded Quit and
  QuitNow, the area child, and the dialog and notification layers. No title
  bar or footer yet.

**5. Default layout (R-38, fact 3).** In `Workspace::new`: `Layout::default()`
becomes a Workspace field. For each side that `has_panels`, build a `DockLayout`
from `layout.groups(side)`: one group is `DockLayout::tabs().panel_view(..)
.active_index(..)`; several are `v_split()` (Left, Right) or `h_split()`
(Bottom) with `.child(tabs, size)`, the left dock's two groups sized
`px(600.)` and `px(1400.)` (rescaled in proportion, `B/dock/dock_area.rs:~1470`).
In `area.update`: `set_center(item_area_handle)`, `set_dock(Left/Right/Bottom)`,
`set_dock_size(.., px(290/440/150))` (floor 100 px). Then, in the same update,
`dump`, `layout.set_arrangement(arrangement_from(&dump))` and
`accepted = dump` (R-38; field `Called from Task 5`). No `LayoutChanged`
subscription in 4b: the launch events have no listener, and Task 5 adds the
idempotent handler. `cycle_region` steps `f6_stops(&self.layout)`;
`focus_lost` falls back to a displayed region (R-9); with the default layout
every region is displayed, so the 4b order equals `REGIONS`.

**6. U-4 plan.** First test `desktop::view::dock::tests::each_panel_draws_its_region_inside_the_dock`
(`D/view/dock_tests.rs`, included from `dock.rs`): for each of `region-*` and
`operation-area`, `window.within("dock-area").try_find(id)` is `Some`, and each
region sits within its own `("panel-group", node)` scope, Repositories and
Documents in different groups. It fails first with `expected one scope
"dock-area" … found 0`. It can only pass if every panel's `render` completed
`workspace.update`; a leased Workspace would panic. **If it fails, stop for the
owner.** Fallback to propose, not implement: a per-panel mailbox
(`Rc<RefCell<Option<AnyElement>>>`) filled by `Workspace::render` and taken by
the panel, at the cost of building hidden panels' bodies.

**7. Harness (R-26).** `Shell::open` and `Shell::launch` set the bottom dock to
520 px after launch (a `dock_area()` accessor and `set_dock_size`);
`Shell::resize_keeping_content(width, height, cx)`; `Shell::open_as_launched`
resizes nothing (the app's 1280 × 840 bounds). E-10's ten sites: `scroll_tests.rs:35`
(1440 × 700, helper `tall()`), `settings/repositories_tests.rs:650` (640),
`keyboard_tests/discussion.rs:69` (helper `discussed()`), `editing_privacy_tests.rs:779`,
`item/posting_tests.rs:142`, `item/posting_tests/support.rs:96` (helper
`posting_shell()`, about 66 call sites), `item/frame_tests.rs:51` (helper
`shell(height, ..)`) and `:220`, `item/discussion_tests.rs:108` (shared
`shell(height, ..)`), `drafts_tests.rs:317` (6000).

**8. Source observations.** The group frame and `Workspace::region` both track
the active panel's focus handle (`B/dock/tab_group.rs:682`); `focusable_node_ids`
keeps the inner one (`G/key_dispatch.rs:223`), so focus checks should hold.
Body methods in `workspace/content.rs` are not visible outside
`view::workspace`, hence the wrappers in `render.rs`. Line cites in the design
for `D/mod.rs` and `C/dock/mod.rs` drifted a little; substance unchanged.

**9. Rulings (orchestrator, 2026-11-08).**
- **Q1, one struct.** Accepted as an implementation detail: the five stable
  names (`RepositoriesPanel` … `ItemAreaPanel`) are what persists and what
  `panel_name` returns; a macro can give five types if a later task needs it.
  Recorded as a deviation from "five thin entities" in Part C.
- **Q2, harness height.** R-26's 1726 and +326 include the 38 px title bar and
  28 px footer, which arrive in Tasks 6 and 7. A harness constant
  `SHELL_CHROME_HEIGHT` is 0 in 4b: window height = 1660 + the constant, and
  `resize_keeping_content` adds 260 + the constant to the height. Task 6 adds
  38, Task 7 adds 28, so the final values are 1726 and +326 as R-26 says, and
  the item area is 860 × 1140 at every task. Width stays 1590 and +150.
- **Q3, R-38 in 4b.** Accepted: the `accepted` field and
  `set_arrangement(arrangement_from(&dump))` are set at build.
- **Q4, extra builder test.** Declined: not in the approved plan, and Task 9's
  restore tests exercise `load` and the builders.
- **Carried by this task:** the 4a Minor doc-comment tidy (`keys.rs:69`,
  `create_tests.rs:10-11` and `:951`, `create.rs` `new_item`,
  `tickets_tests.rs:568-570`), where the files are touched, and the `NewItem`
  fallback to `Pane` stays as is unless a test needs otherwise.
- **File sizes.** `dock.rs` about 300, `dock/panels.rs` about 200,
  `dock/header.rs` about 200 (4c expects 450-500, so 4c puts tabs in
  `dock/header/tabs.rs`); `dock_tests.rs` will pass 500 in Task 5, so Task 5
  splits it into `dock_tests/` modules.
- **Test runs.** E-10's shared helpers reach many tests; run each by exact name
  and list them in the ledger, with `--test-threads` kept low (the Task 4a
  flake appeared under heavy parallel load). No prefix runs.

## Task 4b: Dock Area, Panels And Default Layout

- **Commit** `5922bbaa` (range `7df4ff21..5922bbaa`), 20 files: new `view/dock.rs`
  (193 lines), `dock/panels.rs` (165), `dock/header.rs` (147), `dock_tests.rs`
  (158); changed `workspace.rs`, `workspace/render.rs`, `workspace/focus.rs`,
  `harness.rs` (now 669 lines, up from 604, already over before), `model/layout.rs`
  (the 4b dead-code attributes removed), `view/mod.rs`, and the E-10 and E-11 test
  sites. The mechanism is the checkpoint's, with Q1 to Q3 as ruled.
- **U-4 holds.** `desktop::view::dock::tests::each_panel_draws_its_region_inside_the_dock`
  passes: every region and `operation-area` is drawn inside `dock-area`, each
  region in its own `("panel-group", node)` scope, five distinct groups, so a
  panel's `render` can update the Workspace. Red first only through a stub that
  panicked ("no dock area yet"). **U-6 holds:** left Repositories over Documents
  is 0.3 : 0.7 within 1 px.
- **New tests** (1 test each, passed, implementer and reviewer):
  `desktop::view::dock::tests::each_panel_draws_its_region_inside_the_dock` and
  `…::the_default_layout_is_1b` (groups, sizes 290/440/150, open docks, 0.3:0.7,
  focus on Repositories only). The second was not seen failing first; the
  reviewer mutated it mentally (size, swapped groups, split, closed dock, focus)
  and judged it discriminating.
- **E-10 applied** to all ten sites, same numbers, only unused `px`/`size`
  imports removed: `scroll_tests.rs` (`tall()`), `settings/repositories_tests.rs`,
  `keyboard_tests/discussion.rs` (`discussed()`), `editing_privacy_tests.rs`,
  `item/posting_tests.rs`, `item/posting_tests/support.rs` (`posting_shell()`),
  `item/frame_tests.rs` (two sites), `item/discussion_tests.rs`, `drafts_tests.rs`.
  Tests run by exact name, `--test-threads=4`: 96 reaching the changed helpers
  (63 `posting_shell`, 2 more via `discussion_tests::shell`, 18 `discussion_tests`
  `shell(height, ..)`, 5 `frame_tests` `shell(..)`, 2 `discussed()`, 2 `tall()`, and
  the direct sites), all passed; name lists in the session scratchpad.
  Also run: 35 (region helpers, F6 and selector tests, the sheet test, the dock
  tests) with one failure, below; and 117 (`operations::`, `drafts_entry`,
  `workspace::`, `quit`, `passphrase`, `drafts::tests`), all passed. The last two
  batches were prefix-style sweeps for geometry, wider than the exact-name rule
  (second occurrence after 4a); they found the one case below.
- **E-11 (pre-authorized, decision 17, owner-approved option 1 after a stop):
  `desktop::view::keyboard_tests::drafts::quit::a_draft_not_saved_on_quit_by_keyboard`.**
  After `Shell::open` it calls `shell.resize_keeping_content(1840., 1400., cx)`
  (400 px wider), with a comment. Reason: the draft-flush entry in
  `operations.rs` is a single unwrapped line (922 px) followed by Retry, so in
  the Activity dock's 860 px the Retry sits outside the clip and the test's
  "is on screen" check failed. **Removes no assertion.** The test checks that a
  control is visible to reach it, which E-8 says stops for the owner; the owner
  chose this option (2026-11-09, "Widen the test, record a limit").
- **Proposed Known Limit K-19 (not yet in the design; for the owner at
  review-ready).** Activity is the width of the item area (the bottom dock sits
  in the centre column, fact 8): about 550 px at the app's default 1280 px window.
  Three rows in `operations.rs`, which this Cycle must not edit, are unwrapped
  single lines with a control after text, and the Kit group clips on x, so at
  narrow widths the control can sit outside the visible Activity: the draft-flush
  entry's Retry, the pending repository row's Retry (a long name or state label),
  and the pending controls row (Resume, lifecycle resume, review, Abandon; the
  later ones, usually Abandon). The session entry's controls and message rows
  wrap, and `finishing` (text and Quit now) fits at 550 px. The rows are fixed
  where `operations.rs` is rebuilt (Cycle 03's Activity, or D6); the Activity
  panel's contents scroll vertically only. Whole-suite coverage of these rows is
  Task 12's.
- **Review:** APPROVED. Blocking none, Important none. Minor:
  `model/layout.rs:69` `short_label` still carries `#[allow(dead_code)] // Called
  from Task 4c` although `panels.rs:121` (`tab_name`) calls it, so only `icon` is
  still uncalled; `model/layout.rs:277-278` and `:388-389` label `set_arrangement`
  and `arrangement_from` `Called from Task 5` although `DockShell::build` calls
  both in 4b. The reviewer did not run the four Verify lines (it was limited to
  exact-name tests); the implementer's clean results stand. The reviewer ran 6
  test executions, all passed.
- **Carried to Task 4c:** correct those stale dead-code attributes; the 4a doc
  tidy still applies where files are touched. `PanelHeader`'s frame omits the
  skin's `bg(background)` and its content frame omits `pt_2` for multi-panel
  groups (appearance, for 4c).
- **Carried to Task 5 (reviewer notes).**
  1. R-38's accepted state is a dump taken at build, before any layout pass:
     `DockArea::dump` reads the tree's sizes (600/1400) before layout and
     measured sizes after. Task 5's handler must compare the arrangement and each
     dock's open state, and not exact split sizes, or the launch's own
     `LayoutChanged` would look like a change (reload, reveal, save).
  2. The test for R-38 does not check that `Layout.arrangement` and `accepted`
     match the dump; Task 5 should pin that.
  3. `Shell::open` sets the bottom dock to 520 px after launch, which the
     handler will see as a layout change; tests that must see no save use
     `Shell::open_as_launched`.
  4. Pruning of focus handles while Activity is hidden is correct by
     construction but untested; Task 5's hidden-Activity tests cover it.
  5. `Workspace::render` runs its `sync_*` hooks before the panels render, and
     panel render is uncached each frame; a later task that caches must keep that.
  6. Nothing tests that the `DockPanels` entry is removed on release (Q4
     declined).
- **Main:** `git fetch origin`: `origin/main` is `8da9afd4`, unchanged; the
  Task 4b merge is a no-op.
- **Boundary:** both commands print nothing; `Cargo.lock` untouched.

## Task 4c: Panel Header

- **Commits** `76080801` (range `e24d8ce9..76080801`) and the review fix `61bd28d1`.
  New `view/dock/header/tabs.rs` (349 lines); `header.rs` 144; `panels.rs` 201;
  `dock_tests.rs` 229; `model/documents.rs` gains `document_count`.
- **Header:** a single panel shows icon, title and count in a 32 px `chrome` row;
  a tabbed group shows one tab per visible panel (ids `panel-tab-{kind}`, active
  tab `panel` background with a 2 px `accent` line, short labels when a group has
  more than two tabs, R-32); tabs, titles and the space past the last tab are
  drag sources and drop targets through `TabGroupContext` with the app's own
  `DragPreview`; the drop indicator still comes from the skin; the item area has
  no header; a collapsed group draws one tab per visible panel, no active panel,
  no drag or drop. Counts (R-22) are read in `title_suffix` through the weak
  Workspace in the render path only, and only when the list is `Ready`:
  repositories `navigation().repositories().rows()`, tickets
  `tickets().state().rows()` (under the filters), documents `document_count`
  (documents, never folders), Activity none.
- **Test** (red first: `panel-tab-documents` not drawn; 1 test, passed, implementer,
  reviewer and again after the fix): `desktop::view::dock::tests::clicking_a_tab_selects_its_panel`.
  Reviewer: discriminating (wrong id, a click that does nothing and a missing
  `select_tab` each fail). No appearance test added. A temporary strip test was
  run (a prefix run of `dock::tests::`, 4 tests, a third slight breach of the
  exact-name rule) and not committed.
- **Review:** APPROVED. Blocking none; **Important 1, fixed in `61bd28d1`:**
  `past_the_last_tab` was given the number of drawn tabs, but the Kit's drop slot
  addresses the group's full panel list, so with `[A, hidden, B]` a drop past the
  last tab landed before B. It now passes `group.panels().len()`. The group with
  a hidden panel exists only from Task 5, which should test it. Minor, fixed in
  `61bd28d1`: stale attributes on `panel`, `muted` and `border_strong` removed;
  the hand-rolled `WhenDroppable` replaced by `FluentBuilder::when`. The reviewer
  ran the four Verify lines (clean) and four exact-name tests (passed).
- **Deviations and decisions:**
  - Strip-tab clicks are **inert** in 4c, because the Kit's `select_tab` focuses
    the active panel, which a collapsed group does not draw (R-35). **Task 5 must
    add the strip tab's `on_click` with the reveal.** The reviewer judged this
    acceptable and non-conflicting with Task 5.
  - The single-panel Activity title draws no icon: `activity` is not served by
    `ItemIcons` and `Assets` errors on an unknown path. `single()` filters
    `PanelKind::Activity` out, with a `// Task 6` comment. **Task 6 adds
    `Activity` to `ItemIcons` (`icon_assets!`, the `ICONS` list in
    `assets_tests.rs`) and must delete that filter.**
  - `PanelHeader::frame` gives panels the `panel` background and the item area the
    theme background. If the owner wants one, it changes in `PanelHeader::frame`.
  - The Kit's host-owned `AnyDrag` drops (`drop_item`) are not wired; nothing in
    the app uses them.
  - The 4a doc tidy was done (comments only): `keys.rs`, `create_tests.rs` (two
    places), `create.rs` `new_item`, `tickets_tests.rs` lines 34 and 252.
- **Carried:** `ChromeColors.selection` still carries `Called from Task 4c` and no
  task in the plan uses it; relabel it or remove the field at the final quality
  review. `teal`, `green`, `red` are Task 7's.
- **Verify:** all four lines clean (implementer; reviewer too). Both Boundary
  commands print nothing; `Cargo.lock` and `assets.rs` untouched.

## Task 5 Design Checkpoint (2026-11-09, at `b4c01022`, before code)

The implementer's read-only phase 1 (no edit, no commit, no build), reviewed
and ruled on by the orchestrator. Cites: `B/`, `C/`, `G/` as in the design;
`D/` is `src/desktop/`.

**1. Workspace API.** `impl Workspace` goes in a new `D/view/workspace/docks.rs`
(`layout`, `dock`, `regions` are private to `view::workspace`; Kit-side helpers
are `impl DockShell` in `D/view/dock/{reconcile,moves,accepted}.rs`). Four
methods, each `pub(in crate::desktop::view)` and taking `window, cx`:
`toggle_panel(kind)`, `toggle_dock(side)`, `reveal(kind)`, `dock_panel(kind, side)`.
Common tail: `let before = self.layout.clone()`, apply the edit, return if
`self.layout == before` (a no-op such as P9 does no reconcile, no focus move and
no save), then reconcile, `check_focus`, `schedule_layout_save`, `cx.notify()`.
Callers: Task 5 (`show_holder` and the strip-tab click call `reveal`), Task 7
(footer toggles, the `DockPanel` action, Needs attention), Task 8 (dock keys,
region keys, Find). Dead-code labels `Called from Task 7` or `8` except `reveal`.
The save is a stub until Task 9: a counter `DockShell.saves`, bumped by
`schedule_layout_save`, with a `#[cfg(test)]` reader.

**2. Reconcile** (`DockShell::reconcile(&mut self, layout, window, cx) -> Vec<Side>`,
returning the docks it opened, which is Task 9's R-37 seam). In order:
1. A side with `!has_panels` and a dock: `layout.set_size(side, area.dock_size)`
   then `remove_dock` (new `Layout::set_size`).
2. Visible flags: `visible = is_on(kind) || (side_of(kind) == Bottom && !is_open(Bottom))`
   (R-35: while the bottom dock is closed every panel in it is visible so the strip
   names each). Panels first, then the area.
3. Active tabs: where the panel lists match and `active` differs from the
   dump, `select_panel` (no focus move). This makes "hiding the active tab selects
   the next that is on, wrapping" hold in the Kit, whose own fallback is the first
   visible panel.
4. Open flags: for each side with panels, `toggle_dock` only if
   `is_dock_open != layout.is_open`.
5. `cx.notify()` on the area, since a hidden panel's own notify cannot dirty it.
Reconcile never adds a dock (only `dock_panel` does). P30: `toggle_dock` keeps
the tree, `NodeId`s and element state (`B/dock/dock_area.rs:301-320`, `:971`,
`:1020`); only `load` mints ids (`:724-729`), so `NodeId` stability is the test
oracle. It settles because each Kit edit is issued only where Kit and model
differ, and the handler refreshes `accepted` after reconcile.

**3. `LayoutChanged` handler** (R-6), subscribed in `DockShell::build`
(`cx.subscribe_in`, subscription stored in `DockShell`); events are delivered
after the edit (`G/app/context.rs:762-776`), so the Workspace and the area are
never both leased. `Workspace::layout_changed`:
1. Dump. If `same_layout(accepted, dump)`, stop. `same_layout` is a pure function
   in `dock/accepted.rs`, **not** strict `==` (clarification, below): same
   version and centre; per dock the same presence, placement and open flag and
   the size within 1 px; per tree the same names, children, Tabs `active_index`
   and Stack axis; Stack sizes compared as shares (`size / sum`) within 0.5%.
   Reason: `dump` reads measured sizes, falling back to the stored tree sizes
   when the measured size is zero (`B/dock/dock_area.rs:827-848`), so the dump
   before and after the first layout pass differ in pixels but not in shares
   (`:1470-1481`); a divider drag must still count as a change (P23, K-9).
2. If the centre is not exactly `["ItemAreaPanel"]`, a drop landed on the item
   area: `load(accepted)` (a full dump with closed docks marked closed), no
   accept, no reveal, no save (K-2: ids are re-minted, so focus and Kit button
   state in the area reset). Its own queued event finds `dump == accepted`.
3. Otherwise: compute `kit_opened(side) = !accepted.open(side) && dump.open(side)
   && !layout.is_open(side)` **before** `set_arrangement`; then
   `layout.set_arrangement(arrangement_from(&dump))` (reveals any panel whose side
   changed, R-36); then `layout.toggle_dock(side)` for each `kit_opened` side still
   not open (P34); sync sizes (`set_size`); reconcile; `accepted = dump` taken
   **after** reconcile; `check_focus`; `schedule_layout_save`.

**4. Moves** (`dock_panel`, R-5). Same side: return (P9). Target dock present
and holding panels: `move_panel` with `InsertTarget::Split` beside the root
(`Placement::Bottom` in a side dock, `Placement::Right` in the bottom dock);
the target may be closed. Target side without a dock (or an empty one): remove
an empty dock first, then `remove_panel` and `add_panel_view` at the side's last
or default size (`Layout::size`). Then read the dump and
`set_arrangement(arrangement_from(&dump))` (the panel is revealed as by a drag),
then the common tail; reconcile removes a source dock left with no panels and
closes one with none on. **The dump is authoritative; production never calls
`Layout::move_to`** (ruling D1). `move_to` stays as the pure oracle: the P8 test
applies it to a clone and compares groups, `is_on` and `is_shown` with the live
layout, so drift between the model and the Kit fails a test.

**5. Focus** (R-9). `check_focus` in `workspace/focus.rs`: if the focused region
is not displayed, `focus_region(Content)`. Runs at the tail of every operation
and at the end of handler step 3; a refused or no-op operation returns before
it. Focus-in: `cx.on_focus_in(&regions[i], ..)` for the four panel regions runs
the same check (a click on the closed strip outside its tabs focuses the group
frame's tracked handle, `B/dock/tab_group.rs:675-680`, `:709`). A tab click is the
exception: `select_tab` focuses the panel it selects (`:232-239`), and
`ActiveChanged` reaches the handler first. **Strip-tab click (Task 4c's carried
item, R-35, P33):** `ShellRenderer` and `PanelHeader` carry the weak Workspace;
the strip tab's `on_click` calls `reveal(kind)` and then focuses that panel's
region (ruling D2). `show_holder` (`workspace/requests.rs`) calls
`reveal(Activity)` first (R-10); entry focus handles survive while Activity is
hidden because pruning runs only in the Activity body.

**6. Test plan** (merged into tables where variations only differ in rows; no
P-outcome dropped; ruling D5). `dock_tests.rs` becomes `dock_tests/mod.rs`
(`#[path = "dock_tests/mod.rs"] mod tests;`; the three earlier tests keep their
paths) with children `support`, `accepted`, `toggles`, `moves`, `strip`, `state`,
each under 450 lines. Tests (paths under `desktop::view::dock::tests::`):
- `accepted::same_layout_tells_a_change_from_the_launch` (pure table: build dump
  vs post-layout dump equal; a divider moved, a dock ±2 px, an open flag, an
  active tab, a panel in the centre each differ; the `kit_opened` and
  `centre_is_item_area` helpers).
- `toggles::hiding_a_panel_gives_its_space_to_its_dock` (P2; see D6).
- `toggles::panel_toggles_keep_the_area_and_the_layout_in_agreement` (P3, P4, P13
  reveal, Part B; replaces the plan's `hiding_the_last_panel_closes_its_dock`,
  `revealing_an_inactive_tab_selects_it`, `hiding_the_active_tab_selects_the_next`;
  rows: hide to empty a dock; bottom dock with Activity's visible flag true,
  R-35; a group [A,B,C] hiding the active; revealing an inactive tab; a drag
  leaving only off panels).
- `toggles::a_dock_closed_and_opened_keeps_its_panels_groups_and_sizes` (P5, P30;
  focus on the item header's Edit button, a Kit button; `NodeId`s unchanged; Left
  and Bottom).
- `toggles::dock_toggles_follow_the_rules` (P6, P7; replaces
  `toggling_a_dock_with_every_panel_off_shows_them` and
  `a_side_with_no_panels_has_no_dock`).
- `moves::each_panel_docks_to_each_side` (P8, P9; absorbs
  `docking_a_panel_where_it_is_changes_nothing`; a cumulative walk on one shell,
  because a second `Shell::open` on one `TestAppContext` is unverified).
- `moves::a_drop_on_the_item_area_is_undone` (P10; rows via `move_panel` Tabs and
  Split on the centre, with and without a closed dock, and one real `drag_to`).
- `moves::a_panel_moved_into_a_tab_group_is_kept` (P11; the save and relaunch
  checks are `// Completed in Task 9.`).
- `moves::a_drop_past_the_last_tab_lands_after_it` (Task 4c's Important finding,
  with a hidden panel in the group).
- `strip::the_closed_strip_names_and_reveals_every_panel` (P33),
  `strip::opening_the_strip_by_its_edge_counts_as_its_toggle` (P34, with the
  false-positive row: a dock left with only off panels must not turn them on),
  `strip::focus_leaves_a_panel_that_is_not_displayed` (P12).
- `state::a_hidden_panel_keeps_its_state` (P2),
  `state::a_request_finishes_while_activity_is_hidden` (P28),
  `state::show_it_reveals_a_hidden_activity` (R-10).
- The 4b test `the_default_layout_is_1b` is extended, additively, with the R-38
  pin: `layout.groups(side)` equals the dump's arrangement, `same_layout(accepted,
  dump)` holds, and `saves == 0` after the run (4b's carried note 2).
Not covered by an automated test: the real dock-edge mouse drag and the drop
indicator over the item area (K-2) — the on-screen checklist; pixel sizes (no
tests); the debounce timing and file contents (Task 9); the R-37 clamp (Task 9).

**7. Launch.** `Shell::open` sets the bottom dock to 520 px after the first draw;
the handler accepts that as one layout change (`set_size(Bottom, 520)`, one
save). Save-count tests use deltas or `Shell::open_as_launched`. The build's own
events flush before the first draw, so the handler absorbs them at step 1; no flag.

**8. Document clarifications to carry to the final doc pass** (the approved
documents are not edited now; each is recorded here and flagged for the owner):
1. Part C, handler step 3, "a Kit-opened dock the layout had closed" is not
   safely implementable as worded: a drag that leaves a dock holding only off
   panels, or a shell-initiated toggle, would be mistaken for a Kit-opened dock
   (decision 6 would turn panels on and break P4). The three-term test
   `!accepted.open && dump.open && !layout.is_open`, evaluated before
   `set_arrangement`, fixes it.
2. Handler step 1, "the dump equals the accepted state", is the `same_layout`
   comparison above, not strict equality.
3. The plan says the code is "in `D/view/dock.rs`"; the `impl Workspace` is in
   `workspace/docks.rs` (field privacy).
4. The plan's 16 test names are merged into tables (ruling D5).

**9. Rulings (orchestrator, 2026-11-09).**
- **D1** accepted: the dump is authoritative after a menu move; `move_to` is the
  test oracle.
- **D2** accepted: a strip-tab click reveals **and focuses** the revealed panel
  (a tab click focuses what it selects, P12).
- **D3** accepted: shares within 0.5% and dock sizes within 1 px, as in section 3.
- **D4** accepted: the save stub is a counter.
- **D5** accepted: the merged, renamed tests above.
- **D6 rejected as proposed:** no pixel-geometry assertion (AGENTS.md: do not
  test layout). `hiding_a_panel_gives_its_space_to_its_dock` asserts the panel's
  region is absent, its `visible` flag false, the Kit's node visibility
  (`is_node_visible`) and that the save was scheduled — the model facts that give
  the space — not bounds.
- **D7, D10, D11** accepted: `dock_tests/mod.rs`; `workspace/docks.rs`;
  `Layout::set_size` and `#[cfg(test)]` accessors.
- **D8** accepted: real `drag_to` rows where feasible (P10, the 4c test) and a
  strip click row in P12; if a `drag_to` row cannot run in the test harness, fall
  back to the `move_panel` row and report it. If the strip click does not move
  focus to the hidden region in the harness, report it and degrade the row to
  "focus is in no hidden region".

## Task 5: Layout Operations

- **Commits** `bee6c917` (range `d8fee762..bee6c917`, 20 files) and the review
  follow-up `e6138fcb`. New code: `view/workspace/docks.rs` (180 lines; the four
  operations, `reveal_and_focus`, the handler `dock_event`, `settle`),
  `dock/reconcile.rs` (83), `dock/moves.rs` (76), `dock/accepted.rs` (97:
  `same_layout`, `centre_is_item_area`, `open_in`); edited `dock.rs` (239),
  `header.rs`, `header/tabs.rs` (366), `panels.rs`, `workspace.rs`,
  `workspace/focus.rs` (`check_focus`), `workspace/requests.rs` (`show_holder`
  reveals Activity first, R-10), `model/layout.rs` (`Layout::set_size`),
  `harness.rs` (`Shell::click_settled`, now 682 lines, already over before).
  `dock_tests.rs` became `dock_tests/mod.rs` (the three earlier tests keep their
  paths and bodies) with children `accepted`, `moves`, `state`, `strip`,
  `support`, `toggles`, each under 450 lines.
- **Mechanism as the checkpoint:** reconcile, the three-step handler with the
  `same_layout` comparison, dump-authoritative moves (production never calls
  `Layout::move_to`; the P8 walk holds it to the Kit result at every step),
  `check_focus` after every operation and on focus-in, the strip-tab click that
  reveals and focuses, the counter save stub `DockShell.saves` (Task 9 replaces
  its body). `load` is called only in handler step 2.
- **Tests** (new, all 1 test each, passed): under `desktop::view::dock::tests::`
  `accepted::same_layout_tells_a_change_from_the_launch` (11 rows),
  `accepted::the_centre_and_the_open_flags_are_read_from_a_dump`,
  `toggles::hiding_a_panel_gives_its_space_to_its_dock`,
  `toggles::panel_toggles_keep_the_area_and_the_layout_in_agreement`,
  `toggles::a_dock_closed_and_opened_keeps_its_panels_groups_and_sizes`,
  `toggles::dock_toggles_follow_the_rules`, `moves::each_panel_docks_to_each_side`
  (now 14 steps and the P9 row), `moves::a_drop_on_the_item_area_is_undone`
  (including a real `drag_to`), `moves::a_panel_moved_into_a_tab_group_is_kept`,
  `moves::a_drop_past_the_last_tab_lands_after_it` (4c's finding, with a real
  drag), `strip::the_closed_strip_names_and_reveals_every_panel`,
  `strip::opening_the_strip_by_its_edge_counts_as_its_toggle`,
  `strip::a_kit_edit_elsewhere_does_not_open_a_closed_dock` (added by the
  follow-up), `strip::focus_leaves_a_panel_that_is_not_displayed`,
  `state::a_hidden_panel_keeps_its_state`,
  `state::a_request_finishes_while_activity_is_hidden`,
  `state::show_it_reveals_a_hidden_activity`. The 4b/4c dock tests still pass,
  and the sheet test and F6 test were run. The reviewer ran 17 exact-name tests
  in one command (3.7 s wall); the follow-up ran 3. No prefix runs.
- **Mutation checks by the implementer (each reverted):** removing step 2's
  `load` fails P10; the active-tab `select_panel` fails the panel-toggle walk;
  the Kit-opened guard evaluated after `set_arrangement` fails P34 and the walk;
  the 4c drop slot counting drawn tabs fails the drop test; removing guard term
  2 fails `a_kit_edit_elsewhere_does_not_open_a_closed_dock` row (a), term 3 row
  (b). **Term 1** (`!accepted.open`) is defensive: reconcile keeps the accepted
  open flag equal to the layout's, so no state makes it the only failing term; a
  comment next to the guard says so.
- **Deviations and findings:**
  - D6 followed: no pixel-geometry assertion. `hiding_a_panel_gives_its_space_to_its_dock`
    asserts the region absent, the `visible` flag, the dock still open, a save
    scheduled, and `DockArea::is_empty(Left)` (`is_node_visible` is private).
  - `Shell::click_settled` (harness): the Kit's `window.click` dispatches
    mouse-down, mouse-up and a draw inside one update, so focus listeners ran
    before the tab group's events reached the Workspace. The reviewer inspected the
    ordering and judged it a **harness artefact**: in the app each mouse event is
    its own update and effects flush before the next draw. Test windows are
    inactive, so focus-in hooks are dormant in them; the strip and tab-click rows
    call `window.activate_window()`. `.test_support()` was added to
    `panel-tabs-end` and `("panel-strip", node)` (production-neutral).
  - The P8 walk checks Activity at 520 px (the harness opens the bottom dock at
    520), not the default 150.
- **E-11:** none beyond the additive R-38 pin added to my own earlier test
  `the_default_layout_is_1b` (4b's; not a pre-existing test); no assertion
  removed anywhere in this task.
- **Review:** APPROVED. Blocking none. Important 1 (a test gap, not a code
  defect): no row failed when a single term of the Kit-opened guard was removed;
  **fixed in `e6138fcb`** as above. Minor, fixed in `e6138fcb`: the P8 doc claimed
  a source dock "open or closed" but no step moved out of a closed dock (a step
  was added, Activity out of a dock closed by the person); `let _ =` swallowing
  `load`'s result is now a `debug_assert!`. Minor, not changed: the 0.5% share
  tolerance means a divider nudge of a few px is not accepted or saved until the
  next real event (ruled D3); each Kit-caused operation requests a save twice
  (once in `settle`, once in the handler), harmless with Task 9's debounce
  (tests use deltas); `check_focus` has no explicit modal guard, unlike
  `focus_lost`, but a dialog or sheet holds focus outside every region handle, so
  it is inert (not reachable as a defect today).
- **Verify:** all four lines clean (implementer, reviewer, follow-up); both
  Boundary commands print nothing; `Cargo.lock` untouched.
- **For the on-screen checklist:** the real dock-edge drag on the closed bottom
  strip (P34 is tested through the Kit's `toggle_dock`); the drop indicator over
  the item area before a refused drop and K-2's focus reset after it; a click on
  the closed strip outside its tabs in a real active window; dragging a tab past
  the last tab in a mixed group; the share of space a new group gets after a menu
  move (added with `size: None`, as a drag does); Orca on the strip tabs.
- **Pending for Task 9:** `DockShell.saves` becomes the debounced save;
  `reconcile` returns the docks it opened for the R-37 clamp; save checks in the
  Task 5 tests are marked `// Completed in Task 9.`
- **Main:** `origin/main` still `8da9afd4`; D6 not on main.

## Main merge during Task 6 (2026-11-09)

- `origin/main` moved from `8da9afd4` to `f4442618` ("Integrate closed ticket
  `01M6KHVD6D0DB8KYT6VGQP10Y0`", completed-context retirement; 219 files, mostly
  `src/repository/`, `tests/` and docs). **It is not Wave 03 D6**: D6's branch
  (`b05c86c6`) is not an ancestor of `origin/main`, so the stop condition does
  not apply.
- Checks before merging: `Cargo.lock` has the same `gpui-kit`, `gpui-base`,
  `gpui-component` (0.6.6) and `zorite-editor` (0.10.0); `AGENTS.md` unchanged;
  `git merge-tree --write-tree HEAD origin/main` was clean. Under `src/desktop`
  main changed only `view/confirm.rs` and `confirm_tests.rs`, `test_support/`,
  `view/harness.rs` (6 lines), `keyboard_tests/settings.rs`,
  `lifecycle/privacy_consent_tests.rs` and `settings/repositories_tests.rs`, as
  the Cycle document predicted.
- **Merge** `8ff2e239` (HEAD before `d4c68824`), no conflicts, `git merge`, no
  rebase. `origin/main` is an ancestor of HEAD.
- **Checks on the merged tree:** the test binary builds; the seven tests main
  changed or added, run by exact name, all pass:
  `desktop::view::confirm::tests::preserving_abandon_confirmation_names_retained_resources_and_request`,
  `…::keyboard_tests::settings::add_an_existing_repository_by_keyboard`,
  `…::lifecycle::privacy_consent_tests::a17_a_stranded_close_is_not_resumed_without_consent`,
  and four in `…::settings::repositories::tests::` (`a_partial_create_is_completed_by_retry_under_the_same_request`,
  `adding_a_repository_without_identity_asks_for_one_and_keeps_it`,
  `create_in_an_empty_directory_previews_nothing_then_creates_and_refreshes`,
  `pressing_confirm_twice_creates_one_repository`). The Task 6 reviewer then ran
  the four Verify lines on the merged tree (clean) and eight tests (passed).

## Task 6: Title Bar And Window Title

- **Commit** `d4c68824` (range `253f206d..d4c68824`, 11 files).
  - `mod.rs`: `main_window_options` = `TitleBar::window_options()` + the existing
    1280 x 840 bounds + `WindowDecorations::Client`; `open_test_window_over` goes
    through `open_main_window`; the startup-failure window keeps today's options.
  - `view/shell.rs`: `title_bar(..)`, the Kit `TitleBar::new()` at 38 px with the
    `chrome` background and `border`, an 18 px app icon and the title text;
    `on_close_window` calls `request_quit`.
  - `workspace.rs` and `workspace/render.rs`: `window_title` field; the title is
    computed each render, `set_window_title` is called only when it changes, and
    the bar draws the same string.
  - `assets.rs`: `Activity` in `icon_assets!` (the Kit's own Lucide `activity.svg`
    exists in `gpui-kit-assets`); `brand/manyhands.png` (64 x 64, 3789 bytes, from
    `packaging/icons/manyhands-1024.png` with `magick … -resize 64x64 -strip`)
    served by `DesktopAssets` through `include_bytes!`.
  - `header/tabs.rs`: the Activity-icon stop-gap filter deleted (Task 4c's carry).
  - `harness.rs`: `SHELL_CHROME_HEIGHT` 0 to 38; the item area is still
    860 x 1140 (window 1698 high; `resize_keeping_content` adds 298, and Task 7's
    28 makes E-10's 326).
- **Tests** (1 test each, passed): new
  `desktop::view::shell::title::tests::the_window_title_follows_the_selected_repository`
  (red first: `None` against `Some("Manyhands")`; covers none, a, b, and the in-app
  removal) and `desktop::assets::tests::the_app_icon_loads_as_a_small_png` (PNG
  magic and a 16 KiB bound, so the 1024 px file cannot ship by accident; judged an
  asset contract, not appearance). **E-11 (additive, no assertion removed):**
  `desktop::assets::tests::every_item_view_icon_loads`, `ICONS` 36 to 37 with
  `"activity"`. The implementer also ran the geometry tests
  (`frame_tests::the_page_fills_the_region_and_the_discussion_starts_below_it`,
  `dock::tests::the_default_layout_is_1b`, `dock::tests::each_panel_draws_its_region_inside_the_dock`,
  `scroll::tests::the_page_and_end_keys_scroll_the_content_region`,
  `sheet_tests::no_workspace_key_acts_behind_the_sheet`).
- **Review:** APPROVED. Blocking none, Important none.
  - **Second-actor removal:** when another process removes the registration, the
    selection stays as an Unavailable entry, so the title keeps naming the
    repository; only an in-app removal calls `navigation.forget` and resets the
    title. The reviewer judged this **intended**: the title and the sidebar row
    both use `RepositoryEntry::name()`, matching Scope 2 ("the name the
    Repositories panel shows"). No owner decision needed; a later preference to
    drop the name for an Unavailable selection would be a new decision.
  - **Close control:** the test window reports server decorations, so the control
    is never drawn; the `on_close_window` to `request_quit` route has no test and
    is on the Wayland checklist (A16). `request_quit` calls `quit_now` itself when
    it returns true, so there is no double quit.
  - "Set only when it changes" cannot be observed by the test window (it records
    the last title only); no test, known gap.
  - Minor, carried: the title text has `whitespace_nowrap` but no truncation (a very
    long repository name in a narrow window could run under the window controls);
    the regeneration command for the 64 px PNG is recorded above; the PNG's alpha
    (corners transparent or white) is unchecked; on macOS the Kit places the
    traffic lights for a 34 px bar, so at 38 px they may sit slightly high (native
    CI or the owner).
- **For the on-screen checklist:** the title bar (drag, double-click, the three
  controls by mouse and Tab, resize edges), closing with an unsaved draft (K-4),
  the 64 px icon on the dark `chrome` background (white corners?), a long
  repository name, and the macOS traffic-light position after merge.
- **Verify:** all four lines clean (implementer, reviewer on the merged tree);
  both Boundary commands print nothing; `Cargo.lock` and `packaging/` untouched.

## Task 7: Footer And Dock Menu By Mouse

- **Commits** `b2ec5b65` (range `3532c4d3..b2ec5b65`, 12 files) and the review
  follow-up `37caee64`. New: `view/footer.rs` (315 lines: the controls, the pure
  `plan(layout, attention)`, the name functions `panel_toggle_name`,
  `dock_toggle_name`, `attention_text`), `view/footer/menu.rs` (121),
  `view/workspace/footer.rs` (254; the `impl Workspace` glue, an extra file the
  plan did not list, because `Workspace`'s fields are private to
  `view::workspace`), `view/footer_tests/mod.rs` (465) and `support.rs` (197).
  Changed: `keys.rs` (actions `ToggleLeftDock`, `ToggleRightDock`,
  `ToggleBottomDock`, `OpenDockMenu` and the data action `DockPanel { panel, side }`
  with `#[action(no_json)]`, no bindings yet), `workspace.rs`, `workspace/render.rs`
  (five `unless_dialog` handlers, the footer below the dock area, the menu layer),
  `workspace/docks.rs`, `model/attention.rs` and `shell/tokens.rs` (labels removed),
  `harness.rs` (`SHELL_CHROME_HEIGHT` 66 = 38 + 28; the item area is still
  860 x 1140).
- **As built:** nine footer controls, each a Workspace-owned focus wrapper
  (role, `aria_label`, `aria_toggled`) around a Kit ghost button with
  `tab_stop(false)`; handles keyed by control; a panel toggle reads on when its
  panel is **displayed**; the footer groups follow `layout.groups(side)`; Needs
  attention calls `reveal_and_focus(Activity)`; Settings calls
  `open_settings(SettingsPage::Repositories)`. The dock menu is a Kit `PopupMenu`
  drawn deferred and anchored from the Workspace root; `open_dock_menu(kind,
  Placement::{Pointer, Toggle(bounds)})` is shared with Task 8's key path (the
  `OpenDockMenu` handler opens at the focused toggle's last-drawn bounds; the
  "PanelToggle" key context is already on the panel toggles). A secondary click is
  taken in the capture phase with `prevent_default`.
- **Tests** (new, 1 test each, passed): under `desktop::view::footer::tests::`
  `a_panel_toggle_hides_and_reveals_its_panel`,
  `a_dock_toggle_reveals_its_hidden_panels`,
  `a_secondary_click_opens_the_dock_menu_with_its_side_checked`,
  `needs_attention_counts_and_leads_to_activity`,
  `settings_opens_the_settings_page`, `footer_controls_do_nothing_behind_a_dialog`,
  `toggle_names_follow_their_state`, and (follow-up)
  `a_dock_menu_closes_when_a_modal_opens_over_it`. No existing test was edited.
  A risk run of 15 existing Tab-sensitive journeys passed, by exact name (the nine
  new Tab stops did not break them): `keyboard_tests::every_core_action_works_by_keyboard_alone`,
  `keyboard_tests::frame::{the_details_sheet_opens_by_enter_traps_focus_and_gives_it_back,
  the_discussion_is_reached_by_its_key_walked_and_left,
  the_read_view_selects_copies_and_lets_tab_move_on}`,
  `keyboard_tests::discussion::tab_order_runs_header_share_chips_page_discussion_composer`,
  `keyboard_tests::actions::{copy_by_keyboard, each_relationship_group_is_followed_by_keyboard,
  resume_by_keyboard, cancel_a_queued_request_by_keyboard,
  retry_an_unavailable_repository_by_keyboard, remove_an_unavailable_repository_by_keyboard}`,
  `keyboard_tests::drafts::{the_drafts_page_works_by_keyboard_alone,
  restore_compare_and_close_work_by_keyboard_alone}`,
  `item::page_tests::{tab_moves_focus_out_of_the_read_editor,
  tabbing_onto_a_read_editor_below_the_fold_reveals_its_caret}`. The
  `tab_reaches` tests with small budgets (`item::discussion_tests`, `item::header_tests`,
  `conflict_tests`, `conflict::external_tests`, `operations_tests` `tab_to`,
  `find_tests` `tab_keys`) are left to Task 12's fast tier.
- **Process lapse (recorded).** The tests were not written red-first: the
  implementer wired the implementation first to find the Kit types, then wrote
  the tests and checked them by mutation. The follow-up's new tests were
  red-first. Mutation checks seen (reverted): removing the guard in `press` fails
  the dialog test; `is_on` instead of `is_displayed` fails the toggle test;
  `reveal` instead of `reveal_and_focus` fails the Needs attention test.
- **Review:** APPROVED. Blocking none. Important, all fixed in `37caee64`:
  - **I-1 (a real bug):** a dock menu opened by pointer stayed open when a dialog
    or the sheet opened over it, and its layer took focus back from the modal on
    every draw. `dock_menu_layer` now drops the menu without a choice when a
    dialog or sheet is active. Test: `a_dock_menu_closes_when_a_modal_opens_over_it`
    (a dialog and the real details sheet; red first).
  - **I-2:** the implementer's claim that dropping the capture-phase
    `prevent_default` failed the focus-restore assertion was **wrong**: the old
    assertion passed either way. The test now dispatches a raw right mouse-down in
    one update and asserts Tickets' toggle keeps focus; removing `prevent_default`
    fails it (observed).
  - **I-3:** the sheet branch of the guard and the actions `OpenDockMenu` and
    `DockPanel` under a dialog now have rows. Removing
    `|| window.has_active_sheet(cx)` (both guards, one combined mutation) fails the
    test. The `OpenDockMenu` row is weak (`open_dock_menu` has its own guard); the
    `DockPanel` row pins the `unless_dialog` guard. Under the sheet a footer click
    lands on the sheet's overlay and closes it (fact 33), so those rows use `press`
    and dispatched actions.
  - Minor, fixed: P8's drawn x-positions (Tickets < Activity < the right dock
    toggle); `teal` and `green` relabelled "Cycle 03 (the update-check and to-share
    footer items)"; `footer_tests.rs` split (it passed 560 lines).
- **Deviations to carry to the final document pass:**
  1. The design says a toggle is "selected and toggled together". The inner Kit
     button gets `.selected(on)` and the **wrapper** carries `aria_toggled`, so
     assistive technology sees one toggle, not two. The reviewer judged this
     acceptable; the Kit snapshot's `label()` and `checked()` read from the wrapper.
  2. The dock menu opens **above** the toggle (bottom-left at the pointer or the
     toggle's top-left, kept inside the window with an 8 px margin), not "under the
     toggle" as Part E says, because the footer is at the window foot.
  3. `Shell::click_settled` is used for menu items (a harness artefact, as in Task 5:
     the Kit's `window.click` draws inside one update, between the focus change and
     the effects).
- **Carried:** `ChromeColors.selection` (no task uses it; resolve at the final
  quality review). Task 8 must cover Enter and Space on a footer wrapper (P20 "by
  click or by Enter"), Tab reach with names and states (P29), and the real `menu`
  and `shift-f10` keys for `Placement::Toggle`.
- **For the on-screen checklist:** a real right-click in an active Wayland window,
  tooltips (they draw on hover only), the look of the 28 px bar, the menu's
  position above the toggle, and Orca on a toggle.
- **Verify:** all four lines clean (implementer, reviewer, follow-up); both
  Boundary commands print nothing; `Cargo.lock` and `assets` untouched.

## Task 8: Keyboard

- **Commits** `bc2c7d5c` (range `b4899324..bc2c7d5c`, 11 files; the implementer's
  Sonnet trailer was corrected to the Opus 5.5 trailer the brief requires by a
  message-only amend of this unpushed commit, from `97eb2bba`) and the review
  follow-up `07d80e98`.
- **Bindings (`keys.rs`):** `secondary-b` `ToggleLeftDock`, `secondary-alt-b`
  `ToggleRightDock`, `secondary-j` `ToggleBottomDock`, all in no context;
  `menu` and `shift-f10` `OpenDockMenu` in "PanelToggle"; every handler through
  `unless_dialog`. `PANEL_TOGGLE_CONTEXT` moved to `keys.rs`. `focus_region` calls
  `reveal(kind)` first for any panel region, so `secondary-1`, `-2`, `-4` reveal
  and focus (`secondary-2` still follows the list used last); `focus_find`
  reveals Tickets first. A side effect, judged correct by the reviewer:
  `open_ready_tickets` also reveals a hidden Tickets panel. The reviewer grepped
  every `focus_region` and `focus_find` call site: none can resurrect a panel the
  person hid (`check_focus` and `focus_lost` fall back to Content).
- **E-5 applied:** `desktop::keys::tests::each_key_is_bound_to_its_action` grows
  from 17 to 22 rows (the three dock keys, `menu` and `shift-f10`); failed first
  at `secondary-b`; no row removed. **E-6 applied:**
  `desktop::view::item::sheet_tests::no_workspace_key_acts_behind_the_sheet` now
  starts with Documents off and the bottom dock closed (footer clicks before the
  sheet opens, fact 33), adds the three dock keys, and asserts after every key that
  the layout is unchanged; only a doc comment was reworded among the `-` lines, no
  assertion removed.
- **Tests** (new, `desktop::view::keyboard_tests::shell::`, 1 test each, passed):
  `dock_keys_toggle_each_dock`, `ctrl_b_in_an_editor_is_bold` (D3 editor rules;
  asserts `**two**` and the dock unchanged, then after Escape the dock key works),
  `region_keys_and_find_reveal_hidden_panels` (now 11 rows), `f6_skips_hidden_panels`,
  `a_panel_toggle_works_by_tab_and_enter` (Enter, Space, a dock toggle, Settings),
  `each_side_is_reached_by_the_dock_menu_keys` (`menu` once, `shift-f10` twice,
  arrows, Enter, Escape returns focus to the toggle), `needs_attention_works_by_enter`,
  `footer_controls_are_reached_by_tab`. Red first: seven failed for the right reason
  before the bindings; three exercised Task 7's wrapper and passed already
  (`a_panel_toggle_works_by_tab_and_enter`, `footer_controls_are_reached_by_tab`,
  the E-6 sheet test). Test helper `ringed_on` checks only the regions drawn
  (`ringed()` and `Walk::region` assume all five are drawn).
- **E-11 candidates (visibility-only, no assertion or behaviour changed):**
  `footer_tests/support.rs` helpers `pub(super)` to `pub(in crate::desktop::view)`
  with `mod support` and `footer::tests` widened; `keyboard_tests/editing.rs`
  `EDITED`, `BODY`, `editable`, `text`, `edit_by_e`, `Walk::escape_to_done` made
  `pub(super)` so `shell.rs` can reuse them. One doc comment in
  `footer_tests/mod.rs` reworded.
- **Review:** APPROVED. Blocking none.
  - **Important I-1, fixed in `07d80e98`.** `secondary-b` has no context, and
    zorite's Bold is in the "Editor" context; the reviewer verified in
    `gpui-pre-0.3.6/src/keymap.rs:165-190` and `:246-252` that a no-context binding
    has the same depth as the Editor binding, so they **tie and Bold wins only by
    registering later**. The implementer's comment ("deeper") was wrong, and the
    harness repeated the registration order, so a production reorder would not have
    failed a test. Fix: one `bind_all_keys(cx)` in `desktop/mod.rs` (the desktop's
    keys, then the editor's) used by `run()` and `Shell::start`; its doc comment says
    the order is load-bearing for Ctrl B; the keys.rs comment is corrected. The
    implementer swapped the order and showed `ctrl_b_in_an_editor_is_bold` fails
    ("Bold marked the text"), reverted. Other tests that call `keys::bind` alone
    (`workspace_tests.rs`, `mod_tests.rs`, `host_tests.rs`) were left alone.
  - Minor, fixed: three more rows in the region-key table (`secondary-2` on an
    inactive tab and with the left dock closed, `secondary-4` on an inactive tab;
    the reveal mutation `!is_on` fails row 2); `assert_eq!` in two places; helpers
    moved to `keyboard_tests/shell/support.rs` (`shell.rs` 419 lines).
  - Minor, not changed: the context of `menu` and `shift-f10` is not pinned by a
    test (the key table ignores context; the handler returns early off a toggle);
    dock keys behind a **dialog** (as opposed to the sheet) have no key-level test
    (the shared `unless_dialog` guard covers both); the expected names in
    `footer_controls_are_reached_by_tab` come from production helpers (Task 7's
    tests pin the strings literally); rows of the region-key table depend on state
    left by earlier rows (guards make drift fail loudly).
- **Carried to the checklist:** typing Ctrl B in the **Find input or another plain
  input** toggles the left dock (those are Kit inputs, not zorite's Editor), which
  is consistent with decision 2 (Bold in an *editor*); while a `$$` or `$…$` formula
  is being edited zorite clears its key context, so Ctrl B toggles the dock there and
  Bold would not apply anyway; the `menu` key name under Wayland (U-3) and the dock
  keys in an active window (Cmd on macOS) — the tests send the strings through the
  keystroke parser in an inactive test window.
- **Verify:** all four lines clean (implementer, reviewer, follow-up); both
  Boundary commands print nothing; `Cargo.lock` untouched; `origin/main` still
  `f4442618`.

## Task 9 Design Checkpoint (2026-11-09, at `c2f97f2f`, before code)

The implementer's read-only phase 1 (no edit, no commit, no build), reviewed and
ruled on by the orchestrator. `origin/main` is `f4442618`, merged. Nothing
blocks. The Kit's `DockAreaState`, `DockState`, `PanelState`, `PanelInfo` and
`DockPlacement` all derive `Serialize` and `Deserialize` (`B/dock/state.rs:7-86`,
`:123`), `DockAreaState` has no `deny_unknown_fields`, and `Pixels` serialises as
a bare number. `Workspace::new` reaches `services.backend()`, so the store is
available before `DockShell::build`.

**1. Store** (`D/services/layout.rs`, R-17, R-19). `Backend::layout()` returns
`Result<&LayoutStore, LayoutStoreError>` (no default, as `resolutions`);
`LibraryBackend` takes the store as a fourth constructor argument and
`library_services` opens it; `DelayingBackend` gets `with_layout(..)` (default
`Err(Unavailable)`); `Fixture` gets a lazy `layout_opened()` and
`layout_store()`, so every backend and every relaunch shares one store.
`LayoutStore { shared: Arc<Shared> }` (`Clone`), opened with
`Folder::open_named(data, "layout")`; the file is `layout.json`. `Shared` holds
`Mutex<{folder, newest}>`, an `AtomicU64` generation counter and, under the
test cfg, the `fail` flag and a queue of hold gates. API: `read()` (not found is
`Ok(None)`), `begin()` allocating the generation and taking the next hold,
`Write::run(bytes)` which **awaits the hold before the lock**, `write_now(bytes)`
for the quit (next generation, caller's thread, takes no hold), and internal
`write(generation, bytes)` (under the lock: older than `newest` returns
`Landed::Stale`; set `newest`; the fail seam returns `Err(Io)`; else
`folder.replace`). A failed attempt also fences older writes. Seams:
`fail_writes_for_testing(bool)` and `hold_writes_for_testing() -> Sender<()>`.
`LayoutStoreError` is `Copy` with fixed sentences and no path. A folder that cannot
open makes `layout()` an `Err` (saving off, restore notice); a `layout.json`
that is a directory makes `read` an `Err` while the store stays open (every
write then fails and shows the save notice).

**2. File and `restore`** (`D/model/layout/restore.rs`; `LAYOUT_VERSION = 1`
moves there). JSON `{version, area, on: {panel: bool}, shown: {left, right,
bottom}}`, `deny_unknown_fields`. Pure `restore(read) -> Restore { Saved(Box<Saved
{layout, area}>), Default { notice } }`; `Ok(None)` is `Default{notice: false}`,
`Err` or any reject `Default{notice: true}`; on success `area.version =
Some(LAYOUT_VERSION)` (R-28) and `layout.sizes` come from each dock's `size`.
Validation (all of Part G's rows): parse; version; the `on`/`shown` key sets
exactly; each dock's placement against its slot; dock size at least 100 px and
finite; node `info` against name (`Stack` is `StackPanel`, `Tabs` is `TabPanel`,
`Panel` is a leaf); container contents; `sizes` count and each split size above
0 and finite; the centre exactly `[ItemAreaPanel]`; no unknown leaf; no
`ItemAreaPanel` in a dock; every panel present exactly once; a dock with no
panels. Extra rows beyond Part G, because the Kit's dump never writes them:
`Stack` `axis` outside 0 or 1, a non-finite size, an empty `TabPanel`, a bare leaf
as a dock root, `Tabs` `active_index` at or past the child count. `layout.rs`
(438 lines) and `layout_tests.rs` (504) are split: `layout.rs` gets `mod restore;`;
`layout_tests.rs` keeps a 4-line `#[test] restore_restores_or_falls_back` and
`#[path = "layout/restore_cases.rs"] mod restore_cases;`, so the plan's path
`desktop::model::layout::tests::restore_restores_or_falls_back` is unchanged.

**3. Launch** (`D/view/dock/restore.rs`; R-38). `Workspace::new` reads the store
once, synchronously (a file of a few KB; the stores open synchronously too), then
`DockShell::build`: build with `Layout::default()` and keep `default_state =
dump`; for `Saved`, rebuild a state with each dock's open flag from
`layout.is_open(side)`, `load` it, run `centre_is_item_area` once, and if it fails
load `default_state`, use `Layout::default()` and show the restore notice;
`reconcile` (applies `visible` and the active tabs; no dock toggles because the
open flags already match); `accepted = dump` and
`layout.set_arrangement(arrangement_from(&accepted))` in the same update, so the
launch's queued events stop at step 1 (P37). First launch (`Ok(None)`) is the
default with no notice and `saves == 0`.

**4. Clamp** (`D/view/dock/clamp.rs`, R-37). Pure `clamp(width, height, L, R,
bottom, open) -> [Option<f32>; 3]`: if the open side docks leave under 320 px,
both are scaled by `(width - 320) / (L + R)` (floor 100); a bottom dock leaving
under 200 px becomes `max(100, height - 200)`. Only changed docks are written
through `set_dock_size`, which emits `LayoutChanged`, so the existing handler
syncs sizes and requests the save. Hooks: (a) `Workspace::render` defers a check
while the clamp has not run, reading `DockArea::bounds()`; zero bounds do nothing
and the next render defers again; otherwise it runs once (not `on_next_frame`:
tests never run it); (b) `settle` and the handler pass `reconcile`'s opened sides
to `clamp_docks`, so a closed oversized dock is scaled when it opens. A closed dock
is excluded from the sum in (a). R-37 governs (not Part C's "once at launch").

**5. Saving** (`D/view/dock/save.rs`, R-19, R-31). `request_save(cx)` bumps the
test counter `saves` (kept) and, with a store, sets `pending` and replaces a
`timer` task (`LAYOUT_SAVE_DELAY` = 500 ms, `background_executor().timer`;
replacing the task restarts the delay). When it fires, `Workspace::write_layout`
takes the layout as it is, a **fresh `area.dump`** (guarded by
`centre_is_item_area`, falling back to `accepted`) and `encode(..)`, calls
`store.begin()` and runs the write on the background executor. Results: `Written`
clears `pending` if no newer request exists and clears the save notice; `Stale`
changes nothing; `Err` shows the save notice; a result older than the newest
reported is ignored. **The quit's write is in `quit_now` only** (the first line,
before `cx.quit()`): it is the single funnel (`finish_job` Idle, `quit_if_finished`,
the `QuitNow` action; `request_quit` returns `true` only after `quit_now` ran), and
a write at `request_quit`'s entry would miss `a_change_during_a_quit_wait_is_kept`.
Part H's wording ("at `request_quit`'s entry") is stale since round 4.

**6. Notices** (R-21; `D/view/workspace/layout_notices.rs`, called from
`activity_region`, outside `view/operations*`). `Notices { restore, save }` in
`DockShell`; two dedicated Dismiss focus handles made in `Workspace::new`. Ids
`layout-restore-notice`, `layout-restore-dismiss`, `layout-save-notice`,
`layout-save-dismiss`; exact copy as the design; Dismiss is Quit now's wrapper
pattern (accessible name, Tab, ring). Never counted by `needs_attention`. With no
store only the restore notice can show.

**7. Tests.** `restore_restores_or_falls_back` (the one table, with the extra
rows and the valid rows), and the eight `restore_tests` in
`D/view/dock/restore_tests.rs` (included from `dock.rs`; paths
`desktop::view::dock::restore_tests::<name>`): `a_valid_file_restores_the_layout`,
`a_dock_too_large_for_the_window_is_clamped`,
`a_file_that_cannot_be_read_restores_the_default_and_says_so`,
`the_layout_survives_a_restart`,
`a_change_just_before_a_quit_that_exits_at_once_is_kept`,
`a_change_during_a_quit_wait_is_kept`, `a_stale_write_loses_to_the_quits`,
`a_failed_write_shows_the_save_notice`. `Shell::relaunch(self, cx)`: remove the
window, run until parked, `start(fixture, fresh: false, ..)` skipping
`gpui_kit::init` and `bind_all_keys`, resizing nothing. Task 5's marked save checks
and the relaunch in `a_panel_moved_into_a_tab_group_is_kept` are completed with a
`file_layout` helper (advance the clock, `run`, `restore(store.read())`). **E-9**:
`D/mod_tests.rs:193-198` also asserts `layout/` beside the service.

**8. Rulings (orchestrator, 2026-11-09).**
- **Quit write gating** accepted: `quit_now` writes only when a save was requested
  and not yet confirmed written (`pending`), so a no-change session writes nothing
  and a newer build's file is not clobbered at quit. This **deviates from the
  literal design** ("writes at once"); a failed earlier write leaves `pending`
  true, so the quit retries it.
- **Fresh dump** at save time accepted (the accepted state can lag a divider nudge
  by up to 0.5%, which would lose the last drag).
- **Save notice** shows again after Dismiss if a new write fails; the restore notice
  never returns in a session.
- **Notice placement** accepted: a `v_flex` in `region(Operations)` with the notices
  first, then `operation-area` at `flex_1().min_h_0()`, so the notices never scroll
  away. At the default 150 px each takes about 36 px (decision 15 and K-13 accept).
- **R-9 start focus**: Repositories if displayed, else the first F6 stop (Part C
  Focus); `Workspace::new` focuses Repositories unconditionally today, so Task 9
  adds this, with one assertion in `a_valid_file_restores_the_layout`.
- **Extra validation rows** accepted (section 2).
- **E-11 candidates, visibility only**: widen the `dock_tests/support.rs` helpers
  from `pub(super)` to `pub(in crate::desktop::view)`, as Task 8 did for
  `footer_tests`.
- Risks the tests must settle: `Shell::relaunch` after `quit_now` (the test
  platform's `quit` is a no-op; `remove_window` exists); a second
  `gpui_kit::init` or `bind_all_keys` (skipped by `fresh: false`); existing tests
  that shrink the window (E-10 sites, e.g. 640 + 150 px wide) and then open a dock
  would now be clamped (R-37): grep for them and report; `harness.rs` is already
  over the limit and gains about 25 lines.
- **Document clarification to carry:** Part H's "written at `request_quit`'s entry"
  becomes "written in `quit_now`".

## Task 9: Persistence

- **Commits** `d98231c0` (range `6543351c..d98231c0`, 26 files, about 1900 lines)
  and the review follow-up `b2523219` (12 files). `origin/main` is unchanged at
  `f4442618`, already merged; the Task 9 merge was a no-op. D6 is not on main.
- **As built** (the checkpoint's mechanism and rulings): `services/layout.rs`
  (`LayoutStore`, `Backend::layout()`, generations and both test seams behind one
  shared `Arc`; a file over 1 MiB is an error), `model/layout/restore.rs` (the pure
  `restore` and `encode`; `LAYOUT_VERSION` moved here), `view/dock/restore.rs`
  (launch), `view/dock/save.rs` (500 ms debounce, fresh dump, `pending`),
  `view/dock/clamp.rs` (R-37), `view/workspace/layout_notices.rs` (the restore and
  save notices), the quit's write at the first line of `quit_now` gated on
  `pending`, the R-9 start focus, `Shell::relaunch`. `operation-area` is now
  `w_full + flex_1 + min_h_0` inside a `size_full` wrapper holding the notices (id
  and vertical scrolling kept). `Services::backend()` lost its `#[cfg(test)]` gate
  (a reviewer's Minor: it widens the production surface; a narrower accessor would
  be tidier).
- **Tests**, each 1 test, all passed (implementer, reviewer, follow-up):
  - `desktop::model::layout::tests::restore_restores_or_falls_back` (one table;
    now with the extra rows: a `TabPanel`-named stack, a leaf with children, a dock
    and a split over 1e6 px, and valid boundary rows: exactly 100 px, exactly
    1,000,000 px, 0.5 px split, a nested split).
  - `desktop::view::dock::restore_tests::{a_valid_file_restores_the_layout,
    a_dock_too_large_for_the_window_is_clamped,
    a_file_that_cannot_be_read_restores_the_default_and_says_so,
    the_layout_survives_a_restart,
    a_change_just_before_a_quit_that_exits_at_once_is_kept,
    a_change_during_a_quit_wait_is_kept, a_stale_write_loses_to_the_quits,
    a_failed_write_shows_the_save_notice}`.
  - Follow-up, `desktop::view::dock::save_tests::{a_launch_that_changes_nothing_writes_nothing_even_at_quit,
    a_failed_write_is_tried_again_at_quit, a_dock_nudged_under_the_tolerance_is_saved,
    a_drop_on_the_item_area_is_not_written, a_dock_opened_by_its_edge_is_clamped,
    a_file_over_the_cap_restores_the_default_and_says_so,
    a_store_that_did_not_open_restores_the_default_and_saves_nothing}` and
    `desktop::view::dock::clamp::tests::the_clamp_fits_the_docks_to_the_area` (14
    rows). The round trip `restore(Ok(Some(encode(layout, &dump))))` is now inside
    `assert_agreement`, so every dock test that checks agreement also checks that
    live Kit dumps are accepted by `restore`: **no Kit-written state was refused**.
  - **E-9** applied: `desktop::mod_tests::view::a_data_directory_that_cannot_open_shows_the_startup_failure`
    also asserts `layout/` beside the service (red first: "layout beside the
    service"). **Task 5 completions** (additive, no assertion removed):
    `dock::tests::toggles::a_dock_closed_and_opened_keeps_its_panels_groups_and_sizes`
    checks the file per side after the save delay; `dock::tests::moves::a_panel_moved_into_a_tab_group_is_kept`
    checks the file, then `Shell::relaunch`, then the restored groups, the revealed
    Tickets and agreement. **E-11, visibility only:** `dock_tests/support.rs`
    helpers `pub(super)` to `pub(in crate::desktop::view)` (plus rustfmt re-wrapping),
    `dock_tests/mod.rs` `mod support`, `restore_tests` made `pub(in crate::desktop::view)`.
- **Test-first, honestly.** The Task 9 tests were mostly written together with the
  implementation and **not** seen red first; the implementer proved each
  discriminating by temporary mutation afterwards (dropping the active-index check,
  removing the stale skip, forcing the `pending` gate off, ignoring `Restore::Saved`,
  never setting the notice, disabling the first-frame and the open clamp). The one
  test that passed with the quit write disabled
  (`a_change_during_a_quit_wait_is_kept`, because the fake clock auto-advanced past
  the save delay) was fixed with `elapse` and a `hold_writes_for_testing` hold. The
  follow-up's new tests are mostly guards of existing behaviour, proven by
  mutation (one table of mutations was reported: gate off, `Err` arm clearing
  `pending`, `encode(&self.accepted)`, the centre-check guard, `opening ||` dropped,
  the over-cap arm, `Launch::read`'s `Err` arm, `.max(0.)` for the clamp floor,
  `MAX_SIZE` as `f32::MAX`, and so on).
- **Process breach, recorded.** To find any test the clamp broke, the implementer
  ran **all 1635 desktop `--bin` tests in four commands** (all passed, each under
  six minutes). The owner's brief says no whole-suite run before Task 12. My own
  instruction to report any clamp-broken test by name invited it, and I should have
  told the implementer to reason from the clamp's thresholds instead. The
  reviewer checked: the clamp fires only below 320 px of item width or 200 px of
  item height, the harness windows (1590 wide, 1140 or more tall) and the as-launched
  1280 x 840 are above both, the quit write needs `pending`, and the wrapper is
  geometrically equivalent with no notice; the sites that shrink the window keep
  446 px or more under the 520 px bottom dock and open no dock afterwards. No
  existing test could be named at risk. Task 12 still runs the fast tier.
- **Review:** APPROVED. Blocking none. Four Important, all **test gaps** (not code
  defects), fixed in `b2523219`: I-1 nothing asserted that a no-change quit writes
  nothing, or that a quit retries after a failed write; I-2 nothing distinguished the
  fresh dump from the accepted state; I-3 `clamped()` had no unit test; I-4 restore
  table rows that did not isolate their rule, and no round trip of live dumps.
  Minor fixed: sizes over 1e6 px are rejected; the file read is capped at 1 MiB
  (limitation: `Folder::read` has no size query and `src/drafts` is outside the
  Boundary, so the file is read in full and then checked; the cap sends an
  oversized file to the default with the notice, but does not bound that one read's
  memory); a dock opened by the Kit's edge drag is now clamped too (R-37 "whenever a
  dock opens"); `clamp::clamped` owns the 0.5 px no-change rule and returns
  `[Option<f32>; 3]`; stale comments and a stale `cfg_attr` removed; two missing
  tests added (launch with no file saves nothing; the store-did-not-open path).
  Minor not changed: `encode` could write NaN or inf as `null` that `restore` then
  rejects (unlikely); `axis` is not tied to the slot (a left dock with side-by-side
  groups loads, no crash); a clamp change under the 0.5% share tolerance stops at
  handler step 1 and is saved only with the next real save (ruled D3); a failed
  restore with no change leaves the bad file in place and the next launch shows the
  notice again (ruled); Windows behaviour of creating sentinel entries inside the
  folders the store holds open is unverified (native CI after merge); the keyboard
  Dismiss path (Tab, Enter, Space, ring, accessible name) is untested (the tests
  click the id) — **Task 11's journey should cover it**.
- **Decisions to carry to the final document pass:** Part H's "written at
  `request_quit`'s entry" becomes "written in `quit_now`, gated on a pending save";
  Part C's "clamped once at launch" is superseded by R-37; the 1 MiB file cap and the
  1e6 px size bound are additions to Part G's table.
- **Known limits to add to the design (K-20, K-21):** the quit's write runs on the UI
  thread and, when a background write is mid-`replace`, waits for it (K-14, already
  there); OS-initiated termination bypasses `quit_now`, so a change made after the
  last debounce and before a kill is lost.
- **For the on-screen checklist:** the clamp after the first real frame on a small
  window and on opening a closed oversized dock (the hook relies on the deferred
  closure running after the first draw's prepaint; the harness draws dirty windows
  after each update); the notices at the default 150 px Activity (two notices plus
  the scroller), the Dismiss ring, Tab and Enter; a restart after dragging a
  divider or a dock edge (K-9); the quit with a pending change; a corrupt
  `layout.json` and a directory named `layout.json`; a layout restored with
  Repositories hidden (focus on the first stop); an active-window run of the R-9
  start focus.
- **Verify:** all four lines clean (implementer, reviewer, follow-up); both
  Boundary commands print nothing; `Cargo.lock` untouched.

## Task 10: Quit With Activity Hidden

- **Commit** `d9d6385d` (range `1cd7d817..d9d6385d`, two files: `view/workspace/lifecycle.rs`
  and `view/workspace_tests.rs`, +147 test lines, none deleted).
- **Code (R-20):** `wait_to_quit` focuses Quit now and sets `last_region = Operations`
  only when `layout.is_displayed(Activity)`; otherwise focus stays where closing the
  dialogs and the sheet left it, and nothing reveals Activity or changes a layout.
  `not_saved_chosen`, Cancel: Activity displayed keeps Retry focused as today;
  Activity not displayed focuses the item area (`focus_region(Content)`, which also
  sets `last_region = Content`, so nothing dangles). Edits not kept already focused
  the item area. The reviewer checked every caller of `wait_to_quit` (SavingDrafts,
  Finishing, `unkept_asks_first`, `save_drafts`, `finish_job`) and every focus of
  `quit_focus` or Retry: no path remains that focuses an undrawn control.
- **Tests** (new, 1 test each, passed): `desktop::view::workspace::tests::a_waiting_quit_with_activity_hidden_changes_nothing`
  (P26: the layout and `layout_saves` unchanged, Quit now not drawn, focus and
  `last_region` unmoved, `secondary-4` reveals Activity showing "Finishing {label}"
  and Quit now, releasing the job reaches `QuitPhase::Quit`) and
  `desktop::view::workspace::tests::saving_drafts::drafts_not_saved_with_activity_hidden_focuses_the_item_area`
  (P27, `#[cfg(unix)]`, ran on this Linux machine: Cancel focuses the item area,
  `last_region` is Content, the layout is unchanged, `footer-attention` reads "1 needs
  attention", the phase is Running). Existing `finishing_closes_the_dialog_and_focuses_quit_now`
  and `saving_drafts::a_draft_that_cannot_be_saved_asks_before_quitting` still cover
  Activity displayed, unchanged. The implementer ran 26 existing quit-adjacent
  tests by exact name in one run, plus the 10 `edit_tests::quit::*`; all passed.
- **Test strength.** Written as plain end-state checks, both tests PASSED on the old
  code, because `focus_lost` and render restore focus and `last_region` at the next
  draw. They were changed to read focus and `last_region` inside the same update that
  triggers the quit or Cancel, before any frame draws; only then did they go red
  ("focus is not moved to a control not drawn"; the item-area read `Some(false)`).
  The reviewer copied the tree to the scratchpad and mutated it: both guards removed
  fails both tests; only the focus guard removed fails test 1; only the `last_region`
  guard removed fails test 1; replacing `not_saved_chosen`'s Content branch with
  nothing, or with always-Retry, fails test 2. The guards are independently pinned,
  and the same-update read does not depend on draw timing (`Shell::frame` renders
  one frame, then runs the closure in a single update). Test 2's final end-state
  asserts alone would pass on the old code; only the transient read protects P27.
- **Review:** APPROVED. Blocking none, Important none. Not stuck in the wait: `reveal`
  and the footer paths check no `QuitPhase`, `wait_to_quit` closes every dialog, the
  `QuitNow` action is unguarded, and there is no key for Quit now, so Activity must be
  opened to reach it (decision 7 accepts that). Minor, not changed: test 1 does not
  press Quit now after revealing Activity (covered by the unguarded action and the
  existing Enter-on-Quit-now test); P27's Edits-not-kept return with Activity hidden
  has no explicit test (the code does not depend on Activity; Task 12's fast tier runs
  `edit_quit_tests`); test 1 duplicates helpers from `footer::tests::support`.
- **E-11:** none; no existing test edited.
- **Verify:** all four lines clean (implementer and reviewer); both Boundary commands
  print nothing; `Cargo.lock` untouched. `origin/main` unchanged.

## Task 11: Keyboard Journey And Checklist

- **Commits** `140a2b31` (range `674eecfa..140a2b31`) and the review follow-up
  `53e2689c`. New: `view/keyboard_tests/shell/journey.rs` (266 lines after the
  follow-up), `docs/plans/2026-11-08-wave-04-cycle-01-on-screen-checklist.md`
  (384 lines, 105 rows, W-1 to W-12), a `mod journey;` line in
  `keyboard_tests/shell.rs`, and three helpers in `dock/restore_tests.rs` widened to
  `pub(in crate::desktop::view)` (`elapse`, `in_file`, `same`; visibility only, E-11).
- **Journey test**, 1 test, passed: `desktop::view::keyboard_tests::shell::journey::arrange_the_workspace_by_keyboard_alone`
  (the plan's path was `shell::arrange_…`; the new module adds `journey::`). With
  no clicks and no `Workspace::` calls: the restore notice dismissed by keyboard
  (Tab, ring, name `DISMISS`, Enter), Documents hidden from the footer by Tab and
  Enter, the save notice dismissed by Space, Tickets moved left by `menu`, `down`,
  `enter`, the bottom dock hidden and shown with `secondary-j`, Activity hidden from
  the footer and revealed by `secondary-4`, the clock advanced, `Shell::relaunch` finds
  the same layout (flags, groups, shown flags, sizes within 1 px), no notices, and
  `saves == 0`. After every step it compares a `sent()` snapshot (`jobs_seen`,
  `reads_in_order`, all requests, tab titles) — a superset of D5's A17 pattern and of
  Task 5's snapshot — so a reload or request caused by hiding or moving a panel would
  show. Non-key actions, all honest: the fixture setup, `elapse` (the fake clock), a
  `fs::remove_dir` of the bad `layout.json` directory (it stands for the person
  deleting the bad file; afterwards the next debounced write goes to a real file and
  the relaunch restores it), and `relaunch`. Failed first at compile time (the three
  helpers were private); checked by mutation (the notice Dismiss ignoring keyboard
  presses; dropping the `remove_dir`); the "worker received no request" assertions
  were not mutated. The Tab position of Dismiss is not asserted (`tab_to` presses Tab
  until the element holds focus, up to a limit), so it is not brittle.
- **Review (checklist):** the journey APPROVED as it stood; the reviewer requested
  changes to the checklist, all made in `53e2689c`:
  - **Important 1, safety.** The checklist moved the owner's real `layout.json`
    aside, deleted it, broke it, made many real changes, asked for a read-only data
    directory, and added scratch repositories to the real registry. It now runs
    against a scratch data directory from the first step: the implementer found that
    the **desktop does not read `MANYHANDS_DATA_DIR`** (only `manyhands-cli` does,
    `src/bin/manyhands-cli.rs:90`); it opens its data through
    `RepositoryService::open_default` (`src/desktop/mod.rs:64`), which uses
    `directories::ProjectDirs::from("com", "manyhands", "Manyhands")`
    (`src/repository.rs:7440-7445`), that is `$XDG_DATA_HOME/manyhands` on Linux when
    `XDG_DATA_HOME` is an absolute path, else `~/.local/share/manyhands`. The
    checklist launches with `devenv shell -- env XDG_DATA_HOME="$SCRATCH" cargo run
    …` after `SCRATCH=$(mktemp -d)`, has a check that the variable took
    (`ls "$SCRATCH/manyhands"`), and ends with `find ~/.local/share/manyhands -newer
    "$SCRATCH/.start"` printing nothing. The one real-path exception is shared SSH
    keys under `~/.ssh/manyhands`; the checklist says not to use them. It was also
    wrong that "crash recovery off" is a Settings toggle (it is the draft store failing
    to open); W-2.10, W-9.5 and W-10.1 now make `drafts` a file or read-only in the
    scratch directory.
  - **Important 2.** W-8.15's `jq` printed to stdout and never wrote the file; now
    `jq … > layout.new && mv layout.new layout.json` with a `python3 -c` equivalent,
    both docks required, and the expected result (the clamp, not the restore notice).
  - **Important 3.** W-3.13: a footer click while the details sheet is open lands on
    the sheet's overlay and closes it (fact 33); only a dialog makes the footer inert.
    Rewritten: a dialog case, and a keys-only sheet case.
  - **Minor, done:** the Orca strip-tab row (W-12.5) and the R-9 start focus (W-1.2,
    W-8.5); W-4.8's example; W-7.2 and W-7.3 sizes; K-8, K-10 and K-14 in the Known
    limits; a **Minimum pass** block at the top mapping the Acceptance rows to rows
    (optional rows listed); messages on every bare assert in the journey.
  - The checklist's result cells are all **empty**. Claude records only what the
    owner reports.
- **Missing items found by the reviewer, resolved:** Orca on the strip tabs; the R-9
  start focus. The macOS traffic-light position is left to native CI.
- **Verify:** all four lines clean (implementer, reviewer, follow-up); both Boundary
  commands print nothing; `Cargo.lock` untouched; `origin/main` unchanged.

## Task 12: Gates, Whole-Branch Reviews, Review-Ready Status

**Status: implementation complete; the fast tier is NOT green; the Cycle is not
called review-ready.** The owner decides how to treat the red lib stage (below).
Nothing was closed, merged, cleaned up or dispatched to CI; the only push is the
ticket branch, by the CLI's comment checkpoints.

**Base.** `origin/main` is `f4442618` (merged at `8ff2e239`); it is an ancestor of
HEAD. `AGENTS.md` is identical to main's. D6 is not on main. `git merge-tree
--write-tree HEAD origin/main` is clean. Both Boundary commands print nothing:
no change outside `src/desktop/`, none to `Cargo.toml`, `Cargo.lock` or
`packaging/`, none to `view/operations*` or `model/operations.rs`. The branch is 59
commits ahead of main.

**Gates**, run once on the final Rust tree (HEAD before the doc commits):

| Gate | Result |
| --- | --- |
| `cargo check --all-features --locked` | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | pass (replayed from a cached clean run of the same tree) |
| Fast tier, `--lib` | **FAIL: 1471 passed, 93 failed**, 160 filtered, 885 s |
| Fast tier, `--bins` (desktop `manyhands`) | **1645 passed, 1 failed**, 1087.5 s |
| `manyhands-cli` bin | 317 passed |
| The 27 integration targets of AGENTS.md's list | all pass (canonical_foundation 65, cli 477, discovery_rebuild 79, drafts 48, journal_rejection 26, key_material 39, key_storage 2, local_authoring 142, mutation_bridges 166, mutation_confirmation 184, mutation_contract 47, mutation_relationships 27, mutation_replay 384, read_boundary 20, read_characterization 0 passed and 1 ignored, read_comments 23, read_contract 73, read_credentials 25, read_items 82, read_relationships 17, read_repository 33, read_status 28, recovery_foundation_gate 52, remote_reservation 13, repository_enablement 103, session_credentials 11, shared_key_registry 39) |
| `scripts/check-cli-no-gpui.sh` | pass |
| `cargo run --locked --bin manyhands-cli` (smoke) | pass |
| `git merge-tree --write-tree HEAD origin/main` | clean |
| Boundary diffs | empty |

**How the fast tier was run.** AGENTS.md's literal command has no `--no-fail-fast`,
so cargo stops after the failing `--lib` target and the desktop bin and every
integration target would never run (the retirement ticket recorded the same, "Cargo
stopped before both bins and every integration"). The literal command was run once
(lib failed, exit 101, 16.5 minutes). The rest of the same target list was then run
once more, without `--lib`, with `--no-fail-fast` (about 39 minutes). The result is
the table above.

**The 93 lib failures.**
- **All 93 are in `repository::context_retirement::*`** (59 `execution_tests`, 25
  `record`, 8 `instance`, 1 `run`): the module main's closed ticket
  `01M6KHVD6D0DB8KYT6VGQP10Y0` (completed-context retirement) added. No other lib
  test fails. The list is in the session scratchpad (`t12-lib-failures.txt`).
- **Not caused by this Cycle:** `git diff origin/main...HEAD -- src/repository
  src/lib.rs` and `Cargo.lock` are empty, so the library code and its dependencies
  here are byte-identical to `origin/main`'s, and the library does not depend on
  the desktop (`check-cli-no-gpui` passes). **Not verified on a pristine
  origin/main build:** a lib baseline was not built (it would take about 15
  minutes of tests); the inference rests on that identity.
- **Cause not determined.** A failing test run alone (the built lib binary, one
  test) fails deterministically in 0.15 s, so it is not load: for example
  `retirement_live_inventory_invalid_fresh_authenticated_request_replays_refusal`
  panics at `src/repository/context_retirement/run.rs:1211` (an assertion on a
  refusal outcome), and the retirement ticket's own recorded green test
  `execution_tests::mutation_tests::retirement_batch_public_live_pass_retires_real_context_and_replays_without_transport`
  fails here with `Os { code: 2, kind: NotFound }` at `mutation_tests.rs:45`, reading
  a fixture's `.manyhands/config.toml`. This points to an environment or fixture
  difference, not to the Cycle, but it is a hypothesis.
- **The retirement ticket's own records show lib failures at its gates:**
  lib 1,495 passed and 2 failed, then 1,494 and 3 failed, and it closed "qualified
  owner-review-ready" on 2026-11-09 with "no fresh whole-suite/native success is
  asserted". Its numbers (2 or 3 failures) are far fewer than the 93 seen here.
- **Not fixed here:** the files are outside the Boundary and this Cycle's remit. The
  brief's stop condition applies. It needs the owner: follow-on work against main
  (a separate session, as AGENTS.md says of native failures), or a decision to
  accept a red lib stage for this Cycle's local gate.

**The desktop bin's one failure.**
`desktop::view::keyboard_tests::editing::edit_the_mode_key_save_and_done_by_keyboard_alone`
failed once, at `src/desktop/view/harness.rs:427`, "the edit editor opened did not
happen": the harness's real-time wait of 10 s for the edit editor, under the full
parallel run (1646 tests, 1087 s). It passed alone, twice, at 9.09 s each
(`--test-threads=1`). Task 4a saw the same failure once under load.
- **Slower than before this Cycle: measured.** The same test alone on a plain
  `git archive` of `origin/main` (no extra worktree), built in a scratch copy under the
  session scratchpad and since deleted, ran in **4.97 s and 5.09 s**. On the branch it takes about
  9.09 s alone. This Cycle made that test about 80% slower: the shell (a 1590 x 1698
  window, the dock area, panel headers, footer, title bar) draws more per frame, and
  the test waits in real time for the worker.
- **Risk:** other tests that wait in real time have less margin under load; this is the
  only one that failed in the full run. The harness limit is shared. Options for the
  owner: raise the harness wait (a test-infrastructure edit, E-11-class, not an
  assertion removal), find why frames are costlier, or accept the flake. No change
  was made. Treat this as a measured regression, not as noise.

**Whole-branch reviews** (two fresh read-only reviewers, no builds):
- **Spec review: READY for owner review, no Blocking.** Important 1: proposed K-19
  (Activity clips three unwrapped `operations.rs` rows at about 550 px, the default
  1280 px window) is user-visible and recorded only in this ledger and in the
  checklist, not in the Cycle's Known Limits. Important 2: the approved documents
  differ from the code in recorded ways and need amending (list below). Important 3:
  P22 is partly proved (the title bar's drawn text is not asserted; one string feeds
  both). It produced the P1 to P37 traceability table (all proved except: P22 partly,
  P34 and P36 partly, P7's "no footer toggle" asserted on the pure `plan()` only) and
  the Acceptance texts now in the Cycle document. It found **no removed or weakened
  assertion beyond E-3's**, and no unlisted test edit.
- **Quality review: APPROVED, no Blocking or Important.** No risky production
  `unwrap`/`expect` (the `DockPanels` builder `.expect` cannot fire because
  `restore` validates names before `load`); no real waits in the new tests; all Cycle
  commits carry the Opus 5.5 trailer. Minor, left (to avoid a Rust edit after the
  gate run; each is cheap and could be taken before merge): `ChromeColors.selection`
  has no user and a wrong "Called from Task 4c" label (delete the field, or relabel);
  `model/layout.rs` comment "(Task 5)" on `move_to` and "(Task 4c's review)" in
  `dock_tests/moves.rs` narrate history; `placement(side)` inline and `side as usize`
  in `clamp.rs` (style); `move_panel` finds the group twice; duplicated test helpers
  (`layout`, `saves`) and seven new `use support::*` globs in test files (a new
  convention); `item/tokens_tests.rs` accent/faint assertions are tautological;
  `Render` sets `navigation.pane` without a notify; the `NewItem` fallback to `Pane`
  when no list is focused (arguably contrary to P16); `Services::backend()` lost its
  `#[cfg(test)]` gate (a narrower `layout_store()` accessor would keep the surface
  tight); `harness.rs` (697 lines), `fixtures.rs` (985), `workspace_tests.rs` (1287)
  were already over the size guideline and grew; only `model/layout_tests.rs` (513) is
  a new file over it.

**Approved documents that now differ from the code** (the documents were not edited;
each needs the owner's amendment): Part H and R-19 (the layout is written in
`quit_now`, gated on a pending save, not at `request_quit`'s entry; an unchanged
session writes nothing); Part C ("clamped once at launch", superseded by R-37); Part G
(a 1 MiB file cap, sizes over 1e6 px rejected, extra rows for `axis`, an empty
`TabPanel`, a bare-leaf dock root and an out-of-range `active_index`); handler
step 1 (`same_layout`, not equality) and step 3 (the three-term Kit-opened test);
one `RegionPanel` struct for the five panels; `impl Workspace` in `workspace/docks.rs`
and `workspace/footer.rs`; the plan's test names merged into tables; Part E (the dock
menu opens above the toggle; `.selected` on the inner button with `aria_toggled` on the
wrapper); and proposed K-19, K-20 (OS termination bypasses `quit_now`) and K-21.

**Process lapses** (all recorded where they happened): the D6 `comment add`
published D6's ticket branch (Task 0); prefix-style test runs in Tasks 4a, 4b, 4c, and
a sweep of all 1635 desktop tests in Task 9 against the brief's "no whole-suite run
before Task 12"; Tasks 7 and 9's tests were mostly not written red-first (mutation
checks instead); the implementers' Task 8 commit used the wrong trailer and was
amended before it was pushed.

**Test edits** (E-list, as recorded per task): E-1 to E-10 applied; E-3's approved
assertion removals are the only removals; E-11 entries: Task 4a
`keyboard_tests::frame::the_strip_header_chips_and_page_run_in_tab_order` (`shift-f6`
and a region check), Task 4b `keyboard_tests::drafts::quit::a_draft_not_saved_on_quit_by_keyboard`
(window 400 px wider, owner-approved), Task 6 `assets_tests::every_item_view_icon_loads`
(`"activity"`), and visibility-only widenings in Tasks 8, 9 and 11.

**Awaiting the owner.** (1) How to treat the 93 red lib tests in main's
`context_retirement` module. (2) The measured slowdown of
`edit_the_mode_key_save_and_done_by_keyboard_alone` and its flake. (3) K-19 and
the document amendments above. (4) The on-screen checklist
`docs/plans/2026-11-08-wave-04-cycle-01-on-screen-checklist.md`, run against a
scratch data directory (`XDG_DATA_HOME`); Claude records only what the owner reports.
(5) Whether to take the Minor cleanups before merge. Not authorized and not done:
a PR, the merge, `ticket close`, worktree cleanup, and native CI.

## Owner replies to the Task 12 report (2026-11-09) and follow-up

The owner replied to the six items. In his words, the implementer of main's
retirement ticket "had a great deal of problems getting this done", and "in the
interest of time" he "short circuited parts of the process".

1. **Do those tests pass on main? No.** Run on a plain `git archive` of
   `origin/main` (`f4442618`) extracted in the session scratchpad, outside any git
   worktree and with its own target directory (deleted afterwards), `cargo test
   --all-features --locked --lib -- repository::context_retirement::` gave **49
   passed, 93 failed** in 35.8 s. **The 93 failures are the identical set** to this
   branch's (`diff` of the sorted lists is empty), with the same `Os { code: 2,
   kind: NotFound }` panic at `src/repository/context_retirement/mutation_tests.rs`
   (reading a fixture's `.manyhands/config.toml`). So the red lib stage is main's, not
   this Cycle's, and it does not depend on the worktree. Not checked: the registered
   main checkout itself (its local `main` is behind `origin/main`, and it was left
   alone). Follow-up work on main, in a separate session; not done here.
2. **Time allowance: raised.** Correction to earlier reports: the harness's real-time
   wait `WAIT` was already **60 s** (`src/desktop/test_support/mod.rs`), not the "10 s"
   the Task 4a implementer and the Task 12 report said; the flaky test failed after 60 s
   of real time under the full parallel run. It is now **180 s** (commit `42440a0d`, a
   constant and its comment only, no test or expectation changed). The implementer
   listed every user of `WAIT` (`wait_until`, `Shell::wait_for`, the draft-continue
   loop, `JobGate::wait_reached`, about 15 polling loops and about 15 `recv_timeout`
   holds) and found **no test that relies on a wait expiring** (no `should_panic`, no
   `catch_unwind`): a passing run returns as soon as its condition holds, and a
   genuine hang now takes 180 s instead of 60 s to report. Run by exact name, 7 tests
   (including the flaky one and `harness::tests::a_wait_on_the_worker_outlasts_three_seconds`)
   passed; the four Verify lines are clean; the Boundary is empty. Left alone, noted:
   six test files declare their own local 10 s `WAIT`; three comments
   (`conflict_tests.rs:164`, `privacy_tests.rs:273`, `resolutions_tests.rs:117`) say
   their `until_long` helpers wait "past the harness's `WAIT`", and the `RESOLVING`
   constant (90 s) no longer exceeds it. The tests pass; the wording is stale. Test
   performance is, in the owner's words, to be "at the top of our list" once the
   current waves land; the slowdown of
   `edit_the_mode_key_save_and_done_by_keyboard_alone` (about 5 s before this Cycle,
   about 9 s now) is a known input to that work. The full parallel run was **not**
   repeated after the change.
3. **K-19 accepted** ("It should be okay, contents will probably get revisited").
4. **The document amendments are approved** and applied in `746e347d` to the design
   and the Cycle document: the quit write in `quit_now` gated on a pending save, R-37
   replacing "clamped once at launch", Part G's additions (1 MiB cap, sizes over 1e6
   px, extra shapes), handler steps 1 and 3 (`same_layout`; the three-term Kit-opened
   test), one `RegionPanel` for the five panels, the dock menu opening above the toggle
   and `aria_toggled` on the wrapper, an "Amendments during implementation" note listing
   the E-11 entries, Known Limits K-19 to K-21, and Cycle decision 20. The approval
   status lines, earlier decisions and the Acceptance Status texts were not changed.
5. **The formal on-screen checklist is not being run now.** The owner tests and
   verifies as the work lands, and the checklist
   (`docs/plans/2026-11-08-wave-04-cycle-01-on-screen-checklist.md`) is kept for later.
   The "on screen (owner to run)" entries in the Cycle's Acceptance Status column
   therefore stay open; **no on-screen result is recorded.** (The Cycle document
   carries this as a note under the Acceptance table.)
6. **`ChromeColors.selection` stays in** (the light and dark palette). The quality
   reviewer's other Minor cleanups were not asked for and are not done.

**Standing after these changes.** Nothing in the Cycle's own code is known to be red.
The fast tier's `--lib` stage stays red for main's 93 `context_retirement` tests,
identical on `origin/main`. The Cycle is not closed, merged, cleaned up or sent to CI;
none of that has been authorized.

## Main's fixture fix merged (2026-11-09)

The 93 `repository::context_retirement::*` lib failures were a fixture problem on main:
four fixtures enabled a repository with no identity, and this machine has no global
Git identity. It was fixed on ticket `01M70PYS3B3YWEFTY0QBJGCF6N` (`50fe563e`) and
reached main as `d6789cb9`. The owner confirmed it.

- `origin/main` merged into the ticket branch (merge, no rebase): tip `f519ea4b`.
  `git merge-tree` was clean first. The merge brought in the fix and the other
  ticket's `.manyhands/` records only; nothing under `src/desktop/`.
- Both Boundary diffs print nothing.
- Test run, on the merged tree:
  `cargo test --all-features --locked --lib -- repository::context_retirement::`
  gave 142 passed, 0 failed, in 191 s. This replaces the earlier "93 failed" result.
- The fast tier was **not** re-run on this branch. The other ticket's run on main
  (58 min, with `--no-fail-fast`) had four load-only failures, each green alone:
  `closure::checkpoint::tests::remote_primary_advance_from_context_synchronized_is_primary_advanced`,
  desktop `submissions::tests::{a_second_submit_while_posting_is_refused,retry_keeps_the_request_id}`
  (10 s wall-clock wait) and `edit_the_mode_key_save_and_done_by_keyboard_alone`
  (real-time harness wait; `WAIT` is 180 s on this branch). The desktop ones belong
  with the test-performance work the owner put next.

**Standing now.** The "fast tier `--lib` red" caveat above no longer holds. The Cycle
is still not closed, merged, cleaned up or sent to CI; none of that is authorized.

## Owner's code review: ten findings (2026-11-09)

The owner pasted ten review leads (nothing had been run against the code). Each was
checked against the source first. Commits `dae17983` (findings 3, 5 to 10) and
`0a82eae4` (findings 1, 2, 4). Both Boundary diffs print nothing; fmt, check and both
clippy lines pass.

| # | Finding | Result |
| --- | --- | --- |
| 1, 4 | Ctrl B does nothing in read views; depends on binding order | Real. `ReadOnly` swallowed `Bold`, so the editor's binding beat the dock key and the dock never toggled. The suggested fix (a workspace context for the dock keys) would not work: a shallower context loses to the editor's. **Owner approved** the change to decision 2's reading: in a read-only editor Ctrl B now dispatches `ToggleLeftDock`; editable editors keep Bold. Test (red first): `desktop::view::tabs::tests::ctrl_b_in_the_read_view_toggles_the_left_dock`. |
| 2 | Two title bars after a startup Retry | Real. **Owner chose option A:** the startup failure window takes `main_window_options` and draws the same Kit title bar (`title_bar_closing`; close quits). The `main_window_options` comment, which said the failure window keeps `window_options`, is updated. Test extended (red first, bar removed): `desktop::mod_tests::view::a_data_directory_that_cannot_open_shows_the_startup_failure` asserts `title-bar-title`; the title element gained `test_support()` so tests can find it. |
| 3 | Shift+F6 from outside a region goes to the first stop | Real. Outside every region it now goes to the last stop; F6 still goes to the first. Test (red first): `keyboard_tests::shell::f6_from_outside_a_region_goes_to_the_first_stop_and_shift_f6_to_the_last`. |
| 5 | Dock out of step if `toggle_dock` refuses | Not reachable today: the Kit refuses only a non-collapsible open dock; the default is collapsible and the shell never changes it. Comment added in `reconcile.rs`; no behaviour change, no test. |
| 6 | `move_panel` doc wrong | Real; doc corrected (removing the active tab makes the tab that took its place active). |
| 7 | Appearance tests | Real; removed `the_app_icon_loads_as_a_small_png` and the chrome-colour assertions added to `view/item/tokens_tests.rs` (file back to main's). **These remove assertions I had added in this Cycle; none existed on main.** The `activity` row in the icon-load table stays (it checks that the asset loads). |
| 8 | Hand-copied `MIN_DOCK = 100.` | Real; `restore.rs` and `clamp.rs` use `gpui_kit::base::PANEL_MIN_SIZE` through a small `min_dock()` function (`Pixels::as_f32` is not `const`). |
| 9 | Footer counts attention twice per render | Real; counted once and passed to `footer_control`. |
| 10 | A deferred `clamp_first` per render | Real; `DockShell::queue_clamp` queues it once until it has run. |

Tests run (exact names only): the three new tests above; `f6_skips_hidden_panels`,
`f6_cycles_the_regions_and_secondary_digits_focus_each`, `needs_attention_works_by_enter`,
`the_clamp_fits_the_docks_to_the_area`, `a_dock_too_large_for_the_window_is_clamped`,
`a_dock_opened_by_its_edge_is_clamped`, `a_file_over_the_cap_restores_the_default_and_says_so`,
`a_valid_file_restores_the_layout`, `restore_restores_or_falls_back`,
`f6_stops_follow_the_arrangement`, `every_item_view_icon_loads`, `ctrl_b_in_an_editor_is_bold`,
`the_tab_keys_work_from_inside_the_read_view`, `tab_and_shift_tab_move_focus_out`,
`the_formatting_keys_and_undo_change_nothing`, the two other startup-failure tests and
the two title tests: all green. No whole-suite run. (One command of mine ran with an empty
test list for a moment, which would have meant every test; I stopped it at once and added
a non-empty guard to the later commands.)

## Owner's second code review: ten findings (2026-11-09)

A whole-branch review (72 commits) gave ten leads, again unchecked. Each was checked
against the source. Fixed in `9c2a848c`; three need the owner's decision.

| # | Finding | Result |
| --- | --- | --- |
| 1 | Quitting with Activity hidden gives no sign | Follows design R-20, which the design lists as a known cost ("A person with Activity hidden sees no sign of a waiting quit until they open Activity"). **Not changed; owner's call.** |
| 2 | One failed operation counts twice (request finished `Partial` and the same operation in the pending list) | Cycle decision 3 counts both lists as written; whether one operation can be in both was not checked. **Not changed; owner's call.** |
| 3 | A damaged `layout.json` shows the restore notice on every launch | Real by construction: R-38 saves nothing at launch, so the file stays until the layout changes. Writing the default at launch would contradict R-38. **Not changed; owner's call.** |
| 4 | Ctrl B fix is a workaround that depends on binding order | Real, and fixed properly. The read-only wrapper has the key context `ReadOnly`; `editor::keys` unbinds `zorite_editor::Bold` for `ReadOnly > Editor`; the dock keys are bound in the `Workspace` context, which lies shallower than `Editor`, so Bold wins in an editable editor whichever set is registered first. The workaround in `read_only.rs` (a `Bold` handler that dispatched `ToggleLeftDock`) is gone. Test (red first): `desktop::keys::tests::ctrl_b_is_bold_in_an_editable_editor_only_in_either_binding_order` resolves Ctrl B for an editable, a read-only and a plain stack under both registration orders. `ctrl_b_in_the_read_view_toggles_the_left_dock` and `ctrl_b_in_an_editor_is_bold` still pass. The `each_key_is_bound_to_its_action` test is unchanged (it ignores context). |
| 5 | `each_panel_draws_its_region_inside_the_dock` tests where things are drawn | **Removed** (with its `OPERATION_AREA` constant and import). It was the Task 3 proof of U-4; the other dock and keyboard tests drive every region through the dock, so U-4 stays covered. This removes assertions added in this Cycle. |
| 6 | Test-only code in production | `Layout::move_to` and `Arrangement::move_panel` are now `#[cfg(test)]`. `ChromeColors.selection` stays by the owner's earlier reply; `teal` and `green` are the same palette and were left with it (they carry `allow(dead_code)`). |
| 7 | Repeated definitions | `Side` gained `name`, `placement`, `dock` and `dock_mut`; the four side-to-dock tables, `side_key` and the match in `dock_toggle_name` use them; `view::dock::placement` is gone. `min_dock()` is defined once in `model/layout.rs`. |
| 8 | Repeated group lookups | `Arrangement::locate` returns side, group and tab once; `toggle_panel`, `move_panel` and `reveal` use it and lose their `unwrap`s. |
| 9 | Several dumps per layout change; Documents header recounts | **Not changed.** Each dump follows an edit (the one in `reconcile` reads state the earlier steps just changed), so sharing them risks stale reads; the count is one walk of the tree per render, cheaper than drawing the tree. Nothing was measured. Left for the performance work the owner put next. |
| 10 | The 1 MiB cap does not bound the read | Known limit K-21, owner-approved. A bounded read needs a change in `Folder::read`, outside the Boundary. **Not changed.** |

Tests run (exact names, 59 in one run): everything under `model::layout::`,
`view::dock::`, `view::dock_tests::`, `view::footer*`, `view::keyboard_tests::shell::`
and `keys::`, plus the Ctrl B tests, `no_workspace_key_acts_behind_the_sheet` and the
read-only guard tests: all green. fmt, check and both clippy lines pass; both Boundary
diffs print nothing. No whole-suite or fast-tier run.
