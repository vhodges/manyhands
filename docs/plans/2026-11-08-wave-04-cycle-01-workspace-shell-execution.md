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
