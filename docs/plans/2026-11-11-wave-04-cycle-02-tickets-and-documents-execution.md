# Wave 04 Cycle 02 Execution Ledger

Written only by the orchestrator. Ticket `01M757DHFAVPEVY4TWQTVKQAMJ`
(`vh-r2n89`), branch `manyhands/ticket/01M757DHFAVPEVY4TWQTVKQAMJ`.
Plan: `2026-11-11-wave-04-cycle-02-tickets-and-documents-implementation.md`.
Brief: `2026-11-11-wave-04-cycle-02-handoff.md`.

## Authority Recorded During Execution

- **2026-11-11, owner (session message).** "If testing scope needs to
  increase to cover additional tests not accounted for in the plan is
  pre-approved." Applied as: additional tests beyond the plan's list, written
  or run, are pre-approved for this Cycle when a task shows a real gap. The
  AGENTS.md Testing Strategy still governs where a test goes (cheapest layer,
  extend an existing test first), the 15-minute stop, no appearance tests and
  no real waits. Each added test is listed under "Additional tests" below
  with its reason; each edit to an existing test is an E-15 onward entry.

## Task 0: Baseline

- **Base:** `origin/main` = `48653d01882ba52fd4fd2d58cdb890856f96c237`, already
  an ancestor of the branch (`git merge-base --is-ancestor` true).
- **HEAD before the merge:** `38c64b3ce193cfa083a5cd1fd6a316696a411b27`. The merge
  was "Already up to date"; HEAD after is the same. The remote ticket branch
  equalled HEAD.
- **Worktree:** clean; `git worktree list` shows this path on the ticket
  branch. The main checkout was clean at `48653d01`.
- **Static gates on the branch, cold build:**
  - `devenv shell -- cargo check --all-features --locked`: pass.
  - `devenv shell -- cargo fmt --check`: pass.
  - `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`: pass.
- **Facts re-check:** `git diff --stat a0a734a4 HEAD -- src/desktop src/repository/read docs/Waves docs/RFC docs/design Cargo.lock`
  shows only the approval commit's Document Amendments (RFC relationships,
  Wave 04, `03-panels.md`, `08-gpui-kit-mapping.md`). `src/` and `Cargo.lock`
  are unchanged, so Baseline Facts 1–33 stand and `gpui-kit`, `gpui-base` and
  `gpui-component` did not move.
- **Tests run at this checkpoint:** none (Task 0 runs the static gates only).

## E-List Sweep

The plan's three `git grep` commands returned 142, 128 and 429 lines (699).
A read-only agent classified them, and the orchestrator re-checked every gap
below against the source. Line numbers are at `38c64b3c`.

**Test sites an E-n names** (the implementer of the named task edits them;
`D/` is `src/desktop/`):

| E | Task | Sites |
| --- | --- | --- |
| E-1 | 2 | `D/model/tickets_tests.rs`: `the_default_query_lists_open_tickets` :23, `the_query_maps_closure_and_readiness_and_nothing_else` :38, `a_filter_change_drops_the_last_filters_read` :145, `set_query_keeps_the_query_and_drops_the_last_read` :174, `blocked_reasons_name_the_blocker…` :255, `an_untitled_blocker_is_named_by_its_short_code` :301, `rows_read_under_blocked_carry_their_reasons` :351, `a_readiness_change_drops_the_last_readinesss_read` :391; imports :14 |
| E-2 | 2 | `D/view/tickets_tests.rs`: delete :159 and :395; helpers `library()` :49, `blocked_list` :449, query :322; users :183, :94, :464, :530 |
| E-3 | 3 | `D/view/tickets_tests.rs`: :94, :281, :464, :530, :571, :602; imports :13-14; helpers `visible()`/`expected()` :62-85 |
| E-4 | 4 | `D/view/find_tests.rs`: all 8 tests (:95 … :266), helpers `find()` :50, `find_focused()` :83, the `FindSlug` hold :237 |
| E-5 | 4 | `D/model/find_tests.rs`: :17 to a `parse_id` table; `matches_are_the_librarys_and_none_says_so` :55 and helper `found()` :45 go |
| E-6 | 3, 4, 8 | `D/view/keyboard_tests.rs`: `change_the_filters` :345, `choose` :264, `find` :430, `find_by_short_code_and_ulid` :450, imports :20, :24, walk calls :603-607 |
| E-7 | 8 | `D/view/keyboard_tests/editing.rs`: `create_by_keyboard_alone` :700, `move_and_new_folder_by_keyboard_alone` :791; `D/view/documents_tests.rs` `new_folder_opens_its_dialog` :108 |
| E-8 | 5, 6, 7 | `D/view/documents_tests.rs`: :59, :151, :239, helper `flatten` :43, imports :9; `D/view/new_folder_tests.rs` :32, :216-221; `D/view/move_document_tests.rs` `tree_paths` :274, users :432, :981, :1107 |
| E-9 | 6 | `D/view/keyboard_tests/actions.rs` :224; `D/view/keyboard_tests/editing.rs` :940 (`open_document` :946, :990); helper `keyboard_tests/frame.rs:174` |
| E-10 | 2, 3 | `D/view/dock_tests/state.rs` :31; `D/view/lifecycle/made_ready_tests.rs` :288, :350; `made_ready_review_tests.rs` :65, :119 |
| E-11 | 1 | `D/assets_tests.rs` :49 |
| E-12 | 3 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` (array `[..; 22]` :13) |
| E-14 | 8 | `D/view/keyboard_tests.rs` `on` :150, `ringed` :618 |

**Sites no E-n names** (pre-authorized, decision 16; recorded as E-15 onward
when edited):

- **E-15 (Task 6, expected).** `D/view/polling_privacy_tests.rs:687` asserts
  `("region-documents", PATH)`; `PATH` is the document's file path, which
  Part F moves from the `doc-path` label to the tooltip. The snapshot cannot
  read a description (fact 11), so the pair never appears. Edit: drop the pair
  (the title still shows) and fix the comments at :15, :96, :493. The path is
  still planted and still asserted absent from every surface that must not
  show it, so the privacy check keeps its force.
- **Task 5 may touch `D/model/documents_tests.rs:225, 251, 270, 271`**
  (`model.load("/a")`) if `Documents::load`'s signature changes, and the
  `model.state()` assertions (:226, :228, :237, :261, :274) if `state()` stops
  being a pass-through. Keep `Documents::state()` a pass-through, as
  `Tickets::state()` is, so only a signature change would edit them.
- **Plan error corrected.** E-8 and Task 7 put "the exhaustive `match` at
  `:100-111`" in `D/view/documents_tests.rs`. It is `shape()` in
  **`D/model/documents_tests.rs:100-111`** (an exhaustive match on `TreeNode`);
  Task 7 breaks it by adding the unreadable form. `D/view/documents_tests.rs`
  `:100-111` is only a `doc-problem` assertion (E-8 removes it). Both files
  are already in the tasks' `Files:` lists.

**Tests that depend on a changed helper but no E-n names** (check, expect no
edit): `keyboard_tests` callers of `find()` (`actions.rs:53, 83`;
`editing.rs:140, 219, 1074`; `conflict.rs:209`; `discussion.rs:178, 301`;
`drafts.rs:383`; `lifecycle.rs:149`; `frame.rs:191, 192, 306, 386`), with
`frame.rs:228` relying on Find making Tickets the list used last;
`create_tests.rs` and `editing_privacy_tests.rs:496` (click `new-ticket-button`
and `new-document-button`, which keep their ids; Task 8 keeps them enabled once
a repository is selected); `operations_tests.rs:234, 416` (click `filter-all`;
R-4 keeps its id and click target); the Tab-order tests
`keyboard_tests/shell/journey.rs:80-104` (`TAB_LIMIT` 40) and
`keyboard_tests.rs:174`; hold-and-count tests that must survive
`ReadKind::IndexStatus` in Task 7 (`tabs_tests.rs:251-270`, `:541-560`,
`dock_tests/state.rs:129-147`, `keyboard_tests/shell/journey.rs:49`).
`view/item/sheet/fields_tests.rs:461`
(`suggestions_come_from_the_loaded_tickets_values`) must pass unchanged
(R-33; it needs the closed ticket `Gone` excluded from the single
all-tickets read).

**Production sites.** Every site the sweep found in `src/desktop/` is in a
task's `Files:` list except these:

- **G1 (Task 4).** `ReadKind::FindSlug` has a second user:
  `D/view/item/relations_edit/picker.rs:164` (the relations picker's short-code
  lookup) with tests at `relations_edit_tests.rs:290, 308`. Only Find's use
  (`view/workspace/find.rs:59-62`, `find_tests.rs:237`) goes. See the ruling
  below.
- **G2 (Task 4).** Callers of `clear_lists` and `load_lists` outside every
  `Files:` list: `D/view/workspace/repositories.rs:31, 35, 113, 120, 135`,
  `D/view/workspace/requests.rs:147`. Task 4 changes `clear_lists` to keep the
  filter text and a held ID read unless the selected repository differs from
  the last-loaded one, and `load_lists` itself starts with `clear_lists`
  (`lists.rs:28`), so the comparison must be made against the incoming
  selection inside `clear_lists`/`load`, not at those callers. These callers
  stay as they are unless a test shows otherwise.
- **G3 (Tasks 2–3).** Between Task 2 and Task 3, `D/view/dock/panels.rs:172`
  counts `state().rows()`, which is every ticket whatever the filter. No test
  asserts that count; Task 3 corrects it with the `listed` count.

**Noise** (no change reaches these): `ReadKind::` in services, item, settings,
lifecycle, drafts and operations tests; `new-ticket`, `new-document`,
`new-folder` ids in the create and new-folder dialog tests; `NONCONFORMING` in
`draft_comparison*`, `item/details.rs`, `item_tests.rs`, `sheet_tests.rs`
(`NONCONFORMING_FILE`); `TreeNode::` helpers in `model/documents*`; `fn ringed`
at `keyboard_tests/drafts.rs:105` and `shell/support.rs:53`; `relative_time` and
`tint` in `item/header_tests.rs` (:21 keeps its import); `.state()` in
`services/layout.rs`, `model/find_tests.rs`; `ReadKind::Tickets` in
`relations_edit/picker.rs:100` (reads `TicketFilter::default()` directly).

## Rulings

- **Ruling: `ReadKind::FindSlug` stays — Task 4's plan removes it "with its
  last use", but the relations picker (`picker.rs:164`) is a second use, and
  the plan's file lists do not name it — keeping the variant changes nothing for
  the picker and removes only Find's call, so no other file is touched — cost if
  wrong: one enum variant and its two tests remain; the owner can rename it to a
  picker-specific kind in a later cleanup.** Task 4 removes Find's call and
  test hold, not the variant.
- **Ruling: `Documents::state()` stays a pass-through to its `KeptList`'s
  state — the plan states it for `Tickets::state()` only; the existing
  documents model tests call `state()` — cost if wrong: Task 5 edits those
  assertions (E-15 onward, recorded).**

- **Ruling (Task 1): `list_body` ignores `narrowed` for Documents — Part A's
  table defines no Documents narrowed row, the Documents panel has no filters,
  and Global Constraints make every string the design's verbatim; the plan's
  "`closure` and `elsewhere` are ignored for Documents" is read to include
  `narrowed` (implementer and reviewer agree) — cost if wrong: a Documents
  filter added later must add its own copy and rows.**
- **Ruling (Task 1): `skeleton_rows(id, count, label, cx)` takes an accessible
  name — the plan's `skeleton_rows(id, count, cx)` leaves the skeleton without
  `.test_support()` or a name, so Task 3 could not give it "Loading tickets";
  Task 3 is the only caller and passes `loading_text(kind)` — cost if wrong:
  one parameter to drop.**
- **Ruling (Task 1): file-level `#![allow(dead_code)]` with a "Used from
  Tasks …" comment in `D/model/list_view.rs` and `D/view/rows.rs`, in place of
  the plan's `cfg_attr(not(test), …)` — `cargo clippy --all-targets -D warnings`
  warns in the test build, where no test reads the labels or draws a row — cost
  if wrong: dead code in those two files stays hidden until the allows go.**
  Removal steps: Task 3 removes the one in `rows.rs` (Task 6 for anything still
  unused after Task 3) and removes `list_view.rs`'s once Task 6 has used the
  Documents paths; each of those tasks checks that the file builds without it.
  `KeptList` keeps the plan's `cfg_attr` form; Task 2 removes it.
- **Out-of-list edit (Task 1):** `D/view/shell/tokens.rs` lost two stale
  `allow(dead_code)` attributes on `selection` and `teal`, now used by
  `rows.rs`. No test changed.

- **Ruling (Task 2): the tooltip's Git line with no branch reads
  `Git: no branch · worktree {w}` — `NO_BRANCH` is the whole phrase "no
  branch" and the details sheet's `provenance_label` substitutes it for the
  "branch {x}" fragment, so the literal `Git: branch no branch · …` is
  ungrammatical; a shared `branch_label` in `D/model/item.rs` serves both —
  cost if wrong: one string and its test.** (Reviewer and orchestrator agree.)
- **Ruling (Task 2): a nonconforming row with no problem code reads
  `Needs fixing. Open it to fix.` — the design gives no copy for that edge;
  held as `NEEDS_FIXING_NO_GUIDANCE` with a comment — cost if wrong: one
  string.**
- **Ruling (Task 2): `Tickets::load(repo)` takes no extra parameters and does
  not clear the filter text on a repository change; it returns the changed
  flag and Task 4 acts on it (R-35) — cost if wrong: none, Task 4 owns it.**
- **Ruling (Task 2): a conforming row with no lifecycle counts as Open for the
  details sheet's suggestions (R-33 filters on `Lifecycle::Open`; a
  nonconforming row has none and is excluded).**
- **Transitional behaviour between Task 2 and Task 3** (no test catches it;
  Task 3's alert row and `listed` count supersede each): `dock/panels.rs:172`
  counts every ticket, closed included (G3); `lists.rs:138`
  `list_status("tickets-state", state())` sees all rows, so the "empty" label
  does not show when a filter lists nothing but tickets exist; a busy reload
  over kept rows shows the kept Ready state with no busy label or Retry.
- **Handover to Task 3:** (a) Task 3 owns the "a filter change starts no read"
  assertion (`the_filter_bar_works_without_a_read`), since E-2 deleted the only
  tests that touched filter-change reads; (b) Task 3 replaces
  `#[cfg(test)] Workspace::set_ticket_query` with `set_filter_text` (R-30) in
  the made-ready set-ups and deletes it; (c) Task 3 removes the `sync_tickets`
  strip of `reasons`; (d) Task 3 removes `#[allow(unused_imports)] // Used from
  Task 3` on the `relative_time` re-export in `view/item.rs`, and the
  method-level `cfg_attr` allows on `KeptList::reloading` and
  `busy_over_kept`; (e) Task 3 gives marker and pill copy shared consts
  (`CLOSED_LINE` etc. already exist in `ticket_view.rs`); (f) Task 7 feeds
  `TicketView.unreadable` (hard-coded 0 now; `footnote` already reads it).
- **Process note (Task 2): the implementer wrote the implementation before
  running any test, so the tests were not seen failing first as written.**
  The reconstructed evidence is a compile failure with the new model files
  reduced to `mod tests;`. The review compensated: the reviewer mentally
  mutated the tables, and the implementer mutated production code for every
  review-added assertion (reasons filled only under Blocked; "need" to "needs";
  plural verb; hint plural) and saw each fail, then reverted (diff checked).
  Later briefs repeat that test-first is binding.

- **Ruling (Task 3): the stale and never-refreshed list states are made with
  the registry SQL seam (`rusqlite` on `data_dir().join(REGISTRY_FILE)`, as
  `scheduler_quit_tests.rs` does): `UPDATE repositories SET refresh_required =
  1` for stale; `refreshed_at = NULL` plus a `contexts` delete for never
  refreshed — the implementer first said no seam could make them and used a
  `chmod 000` ticket folder for the incomplete alert, which the reviewer showed
  fails on Windows and as root; the handoff's "a list state cannot be made"
  trigger does not apply — cost if wrong: a registry schema change breaks two
  UPDATE statements, each asserted to hit exactly one row.** The incomplete
  list alert is covered only by `list_view`'s table test
  (`each_state_has_its_own_body`); no view test makes it.
- **Ruling (Task 3): `sync_tickets` runs on every render from `held_by` and
  `pending_conflicts`, so markers appear with no read (P-markers); it clones the
  shown rows each time and `set` returns early when they are equal — R-32
  accepts the cost — cost if wrong: O(rows × drafts) per render; fix by
  comparing in place or caching by a generation.**
- **Ruling (Task 3): the filter bar's segments are focusable divs (tab stop,
  `Role::Tab`, `aria_selected`, name with count); the nonconforming row's code
  cell is an empty spacer with no `ticket-slug`; the footnote draws only when a
  read exists; `New ticket…` stays the body button until Task 8; Find's bar
  still draws state and cards below the filter bar until Task 4.**
- **Ruling (Task 3): dead-code allows left, each with its removal step.**
  `CANT_BE_READ_PILL` and `CANT_BE_READ_TYPE` (`ticket_view.rs`) carry plain
  `#[allow(dead_code)] // Used from Task 7`, and `ListRow::{indent, gap,
  expanded, on_click}` (`rows.rs`) carry method-level `// Used from Task 6`,
  because the `cfg_attr(not(test))` form warns in the test build. The file-level
  allow in `rows.rs` is gone; `list_view.rs`'s stays until Task 6. Tasks 6 and 7
  remove theirs.
- **Out-of-list edits (Task 3):** `view/dock.rs` (`mod tests` widened to
  `pub(in crate::desktop::view)`) and `view/dock_tests/support.rs` (a
  `panel_count` helper) so a tickets test reads the header count by node id;
  `view/item.rs` and `model/*` lost their "Used from Task 3" allows.
- **Handover to Task 4:** `Tickets::load` returns the changed flag and the
  filter input is cleared in `open_ready_tickets` only; Task 4 acts on the flag
  (R-35), removes Find's `FindSlug` call and test hold, and must keep
  `ReadKind::FindSlug` (the relations picker uses it).
- **Handover to Task 6:** `DOCUMENTS_CONTEXT` has `home`/`end` bound with no
  handler (`keys.rs`); confirm the tree does not swallow Home and End.
- **Review notes not acted on (Task 3, Minor):** `tickets_tests.rs` grew from
  610 to 1374 lines (the 500-line limit is for new source files); consider a
  split when Task 4 touches it. The per-render clone above.
- **Process note (Task 3): the three new tests were seen failing first against
  the old code; the E-3, E-6, E-10 and E-12 edits were written with the code
  (E-10 and E-12 failed to compile first).** The review mutated twelve
  behaviours (closed count, sort direction, Show all, Clear filters, Retry,
  count text, markers, `select_edge`, selection by id, readiness menu, Why
  blocked, `open_ready_tickets`); eleven were caught, and Retry was not, so the
  test now holds the read and asserts the kept rows, no skeleton, "Updating…"
  and list focus (seen failing under the `load_lists` mutation, then reverted).
  `list_states_act_and_a_reload_keeps_the_rows` flaked about 1 in 8 before the
  implementer moved the failed and unavailable steps last and held
  `IndexRebuild`; the cause is not identified; 35 of 35 passed in the review and
  5 of 5 after the fixes.

- **Ruling (Task 4): `ReadKind::FindSlug` stays and `D/services/mod.rs` is
  untouched** — `picker.rs:164` and `relations_edit_tests.rs:290,308` still use
  it; Task 4 removed only Find's call and its test hold (as the Task 1 ruling
  said).
- **Ruling (Task 4): blank Enter with rows listed focuses the list on the first
  row and opens nothing** — design Part D bullet 3 ("otherwise, when rows are
  listed") read literally; pinned by `blank_text_opens_nothing` — cost if wrong:
  one branch and one test.
- **Ruling (Task 4): Enter on blank or non-ID text clears an older ID status;
  typing and Escape do not; a non-blank Enter with no repository says "Select
  an available repository to find in", blank Enter says nothing** — the design
  is silent; Find behaved the same — cost if wrong: copy and one branch.
- **Ruling (Task 4): text typed before any repository was selected is cleared
  when one is first selected** (`last_loaded` None to Some is a change, per
  Task 2's table) — pinned by `the_text_is_cleared_once_per_change_of_repository`.
- **Ruling (Task 4): `clear_lists` compares `Tickets::is_change_to(selected)`
  and drops a held ID read at selection; `change_applied: Option<Option<RepoKey>>`
  makes the drop and the text clear happen once per change** — a plain
  `Option<RepoKey>` could not tell "no selection applied" from "nothing
  applied", and the review's repro let a held read land after the repository
  was forgotten (`a_selection_that_goes_drops_a_held_id_read`) — cost if wrong:
  a held read or typed text survives a change.
- **Known gaps (Task 4, Minor, not acted on):** a re-click of the already
  selected repository keeps the text and the held read (the old code dropped the
  read; R-35's "same repository" rule agrees); a vanished repository that is
  retried (`retry_resolution`) keeps the same selection, so a held read for it
  is kept; Escape or typing leaves a stale "Nothing in {repo} has that ID." line
  until the next Enter; the Enter-again retry of a failed ID read has no view
  test, and the busy and failure wording is pinned only by the pure
  `status_text` table. `read_id` keeps `#[allow(clippy::result_large_err)]`
  (clippy fails without it). `sole_match`'s `row.id.is_some()` is redundant
  and kept for intent.
- **Handover to Task 8:** the body `New ticket…` button is still a Tab stop
  between `filter-status` and the hint's Show all; Task 8 moves it into the
  header toolbar.
- **Process note (Task 4): the implementer added copy constants and stubs
  before the first run so the tests failed on assertions; ten of twelve view
  tests, `parse_id`, `sole_match` and the keyboard walk were seen failing first;
  two model tests already passed against the old code and now also pin Busy and
  `parse_id`.** The review's mutations found three gaps (once-per-change guard,
  first selection, blank-Enter clear), now pinned by tests that were seen failing
  under each mutation.

- **Ruling (Task 5): the destination's ancestors are revealed (R-27) through
  `Changed.reveal` in `D/model/operations.rs`, applied by `operations_changed`
  in `view/workspace/requests.rs` via `Documents::reveal(repo, path)`; `D/view/
  new_folder.rs` is untouched** — the plan named `new_folder.rs:302` and
  `requests.rs:161`, but `new_folder.rs:302` is the dialog's `finish` and the
  Move dialog lives in `ItemPane`, neither with a workspace handle, so a hook on
  the finished request covers New folder and Move (dialog and details sheet),
  works for a repository that is not the selected one (the entry applies at its
  next load), and reveals ancestors only, for ok, `discovery_pending` and
  `already_applied` — cost if wrong: about 26 lines in `operations.rs` and one
  table test, `a_folder_made_or_a_document_moved_names_its_place_to_reveal`.
  (Reviewer agrees.)
- **Ruling (Task 5): `Documents::load` returns `(Stamp, bool)` like
  `Tickets::load` (true on a first load or a change of repository); `combine`
  does not apply Loading, so one landed read does not drop kept rows; a busy
  read over a kept tree keeps the rows, a failure replaces them (Part A, R-17);
  `last_loaded` survives `clear`.** `load_documents` ignores the flag
  (`clear_lists` already clears the pane; the review removed a dead guard).
- **Ruling (Task 5): `doc_row` picks where the design is silent** — a folder
  with child nodes beats `empty`, which beats the leaf `Count(0)`; an untitled
  conforming document is labelled by its file name (pinned by a table row);
  a nonconforming row's guidance comes from `problems.first()`, as Tickets'
  `nonconforming` and the old tree do (the item banner filters
  `CONTEXT_PROBLEMS`; if this is ever fixed, fix both lists together); the empty
  folder's tooltip is its one explanation line.
- **Ruling (Task 5): `Expansion::collapse_all` collapses every folder in the
  tree, empty and leaf ones included** (P20 "every folder collapsed"; Part G
  disables the button when no folder is expanded). Quirk: an empty folder
  collapsed this way stays collapsed when it later gains a document, against
  "a new folder starts expanded". Optional change for Task 8: collapse only
  folders with child nodes — cost if wrong: one function and one test comment.
- **Ruling (Task 5): until Task 6 wires `TreeEvent`, a collapse by key or mouse
  is not written to the set** (a rebuild re-expands it, as today); only
  `Documents::set_expanded` (tests) and `reveal` change it.
- **Transitional behaviour between Task 5 and Task 6:** `Documents::state()` is
  a pass-through, so a busy read over kept rows leaves `state()` as the kept
  Ready and the old "busy" `list_status` label is gone; only `busy_over_kept()`
  knows and no view reads it yet. No test covers the Documents busy path.
- **Handover to Task 6:** (a) the Documents busy-over-kept alert row plus the
  rows kept, tested through `hold_read(Folders)` and `hold_read(Documents)`;
  (b) delete the view's `folder_name` and `node_label`, which duplicate the
  model's `last_segment` and the `doc_row` label (an untitled conforming
  document is labelled by its full path in the view and its file name in
  `doc_row`); (c) remove the Task 6 `cfg_attr` allows on `last_segment`,
  `Expansion::set`, `Trailing`, `DocKind`, `DocRow`, `EMPTY_FOLDER_TOOLTIP`,
  `ADOPT_TOOLTIP`, `doc_row` and `Documents::set_expanded`; (d) wire `TreeEvent`
  to `set_expanded`; (e) confirm Home and End are not swallowed by the tree.
- **Handover to Task 7:** move `Expansion`, `visible_selection`, `Trailing`,
  `DocKind`, `DocRow`, `doc_row` and the two tooltip constants (about 200 lines)
  from `D/model/documents.rs` (530 lines, an existing file) into
  `D/model/doc_rows.rs` at the start of the task, before `DocKind::Unreadable`
  and the files list are added, and record the new file here.
- **Handover to Task 8:** the two `collapse_all` allows (`Expansion::collapse_all`,
  `Documents::collapse_all`) are removed there, and the scope question above.
- **Process note (Task 5):** the implementer's batch runs used module-prefix
  filters over unchanged neighbours (177 tests in 112 s, run once) rather than
  exact names; AGENTS.md prefers exact names. Later briefs repeat exact names.
  Not seen failing first (written after the code, then mutation-checked): the
  kept-tree test, the reveal test, and the added assertions in E-17 to E-19.
  The reviewer ran 18 mutations: 16 caught, one equivalent (pane reads
  `state()` vs `list()`), one not caught (the untitled label), now pinned.
- **Renumbering:** the implementer called its test edits E-15 to E-18, but the
  ledger reserves E-15 for Task 6's `polling_privacy_tests.rs` edit; Task 5's
  four are E-16 to E-19 below.

- **Ruling (Task 6): `list_action` takes a `ListKind`** so Refresh, Retry and
  Rebuild focus the panel's own list or tree (R-28) and `NewItem` opens that
  panel's dialog; the old code hard-coded Tickets. `lists.rs`'s `tickets_state`
  became the generic `list_state(kind, ids, body)` shared by both panels;
  Tickets' ids are unchanged — cost if wrong: one parameter. (Reviewer agrees.)
- **Ruling (Task 6): E-15 replaces the `("region-documents", PATH)` pair with
  `("region-documents", TITLE)` instead of only dropping it.** `region-documents`
  is in `ALLOWED`, so the PATH pair was a positive proof that the sweep reads
  that surface. The path is now tooltip and description only, which the
  snapshot cannot read, so the sweep cannot check it there; the TITLE pair gives
  the same proof from the row's label, and fails if the row stops drawing it.
  The `PATH` sentinel stays in `SENTINELS`, swept and never allowed anywhere.
- **Ruling (Task 6): `doc-nonconforming` is named by the row's first tooltip
  line** (the adopt or fix reason; the design gives no name), pinned against
  `doc_row(..).tooltip[0]`; `doc-count` and `doc-empty` are named by their text.
  Minor, not acted on: the reason is also in the row's `aria_description`, so a
  screen reader reads it twice; `document_row` takes `state: (bool, bool, bool)`
  where named fields would read better.
- **Ruling (Task 6): `icon` and `named_icon` moved from `tickets/row.rs` to
  `rows.rs`** and are shared; a pure move with a one-line import change in a
  Task 3 file.
- **Out-of-list edits (Task 6):** `view/rows.rs`, `view/tickets/row.rs`,
  `model/documents.rs` (`node_count`, `folder_path`, allow removals),
  `model/list_view.rs` (file-level allow removed), `keys.rs` and
  `keyboard_tests/frame.rs` (comments only), `polling_privacy_tests.rs` (E-15).
- **Handovers a to e from Tasks 3 and 5 are done**, and no `Used from Task 6`
  text remains. Remaining allows: `ticket_view.rs` `CANT_BE_READ_*` (Task 7) and
  the two `collapse_all` methods (Task 8).
- **Known gaps (Task 6, not acted on):** `focused` (the region ring) is untested
  by design (appearance, AGENTS.md); P26 (hide and show keeps the state and
  selection) has no mutation that breaks only it — the pane, its `TreeState` and
  the expansion set are Workspace-owned and hide clears nothing — and is pinned
  by the `folders_keep_their_state_and_toggle_by_key` step as a regression
  guard; Documents' tab "· Updating…" is untested until Task 8's P29 test (the
  code path is the shared `counted`); `view/documents_tests.rs` is about 840
  lines (was 262) — split `the_tree_says_each_list_state_and_acts_on_it` and its
  helpers into `documents_tests/states.rs` when Task 7 or 8 next touches it.
- **Process note (Task 6):** the six documents tests were seen failing on their
  first divergent assertion only; the later steps (P19, P26, Home/End, alerts,
  Retry) were not seen failing first and were mutation-proven instead (the
  implementer's 25 and the reviewer's 11 mutations). The review found three
  gaps (a document row click opens it; P25 re-baseline after the refresh; the
  failed alert's message) and one removed assertion (Enter on an empty folder
  does nothing) that was reinstated; each is now mutation-proven. Two assertions
  stayed removed because their ids no longer exist (`doc-path`, `doc-problem`
  absence). The tests ran by exact name only, as the brief required.

- **Task 7 split (done first, a pure move):** `Expansion`, `visible_selection`,
  `Trailing`, `DocKind`, `DocRow`, `doc_row` and the two tooltip constants moved
  from `D/model/documents.rs` to the new `D/model/doc_rows.rs` (207 lines);
  `parent` and `last_segment` became `pub(super)`; importers were updated, not
  re-exported. The reviewer diffed it: every moved line is found at HEAD except
  the header, imports and the `Unreadable` additions. The two Task 5 plan tests
  stay in `model/documents_tests.rs` under the same paths.
- **Ruling (Task 7): the unreadable files are held beside the lists and, for
  Documents, also inside the `KeptList` tree (stripped and re-inserted when a
  status read lands)** — a kept tree has no source reads left to rebuild from
  (`forget()` resets them), so patching it is the only way to show a status read
  that lands during a reload; `combine` re-inserts from the held files so a
  later rebuild neither loses nor doubles them. Added `KeptList::map_rows`
  (Documents only). (Reviewer agrees.)
- **Ruling (Task 7): `unreadable_rows` keeps a `SourceUnreadable` problem only
  when it has a path and a worktree, the path ends in `.md`, and it starts with
  `.manyhands/tickets/` (tickets) or `docs/` (documents); `primary` is
  `worktree == root`; a file named twice in one worktree is one row.** The `.md`
  suffix is the design's "names a `.md` file" rule and the only signal
  (`IndexProblemDto` has just code, path and worktree): an unreadable directory
  named `x.md` would be listed as a file. Verified against real data: the
  planted primary ticket directory and document are `primary: true`.
- **Ruling (Task 7): an unreadable ticket row's `context` is invented minimally
  (Primary if `primary`, else Unverified, no branch); nothing reads it** — the
  tooltip returns early. A later reader of `row.context` would see a made-up
  copy kind; revisit if one appears.
- **Ruling (Task 7): an unreadable document row draws `circle-x` as its
  leading icon (red label) and again as the `doc-unreadable` trailing mark**,
  following the design table literally; it may look doubled on screen — a
  checklist item for the owner.
- **Ruling (Task 7): `open_ready_tickets` reloads only tickets and starts no
  status read** (Part I says each load reads the status once; R-29 reads once;
  K-10 costs a full read). Known gap: a status read already in flight for the old
  tickets stamp is dropped there, so on a first load the Tickets list shows no
  unreadable rows until the next full load while Documents is unaffected; if the
  status read had already landed, the files stay. No test covers the race.
- **Known gaps (Task 7, Minor, not acted on):** `derive` clones every row once
  more when unreadable files exist (cosmetic); removing `cx.notify()` after the
  tickets apply is not caught (frames are forced in tests); the failed status
  step asserts that rows and alerts are unchanged, not the failure itself (the
  mutation of `split_status`'s Failed branch is caught, so it is effective);
  `open_target`'s guard in `find.rs` is defensive and unreachable (commented).
- **Handover to Task 8:** first step, a pure-move split of `view/tickets_tests.rs`
  (1558 lines): move `an_unreadable_file_is_listed_with_its_reason` and its
  helpers into a `tickets_tests/` submodule, before adding toolbar tests;
  `tickets.rs` is at 495 lines and Task 8 will push it over (move toolbar code
  out as needed); `view/documents_tests.rs` (846) and `model/documents_tests.rs`
  (about 1040) can wait until Task 9.
- **Process note (Task 7):** all seven new or extended tests were seen failing
  first. A script truncated `model/documents_tests.rs` mid-task and deleted
  `the_tree_is_kept_over_a_reload_of_the_same_repository` (a Task 5 test); the
  implementer first wrote "not found" and then, when asked, restored it from HEAD
  and audited every modified test file (function names, assert counts, every
  removed line); the reviewer re-ran the audit and found no other loss. The
  review's 11 mutations were caught except the footnote's count above one (the
  `derive`-side count), now pinned. The implementer found that
  `DelayingBackend::replace_service` can fail only the status read, so the
  failed step is a real view test, not only a model-table row.

## Test Edits

| E | Site | Reason | Task | Removes an assertion |
| --- | --- | --- | --- | --- |
| E-11 | `D/assets_tests.rs` `every_item_view_icon_loads` | The table and its array length (37 to 47) gain `repeat`, `bug`, `sparkles`, `list-checks`, `file-plus`, `folder-plus`, `chevrons-down-up`, `arrow-up-down`, `list-filter`, `file-exclamation-point` | 1 | no |
| E-1 | `D/model/tickets_tests.rs` | The five tests became `a_read_lands_only_for_its_repository_and_load` (one table, 12 steps, plus the review's reasons-under-the-default-query step); `the_default_query_lists_open_tickets` asserts the default query and `read_filter() == TicketFilter::default()`; the two `blocked_reasons` tests use `TicketFilter` literals; `empty()` and the `TicketRead` import removed | 2 | yes: the filter-to-library-filter mapping and "a filter change drops the read" assertions, as E-1 says |
| E-2 | `D/view/tickets_tests.rs` | Deleted `a_filter_change_drops_the_held_read_of_the_last_filter` and `a_readiness_change_drops_the_held_read_of_the_last_one`; `library()`, `blocked_list` and the readiness query take `TicketFilter` literals; unused imports removed | 2 | yes: those two tests' assertions, as E-2 says |
| E-10 (made-ready part) | `D/view/lifecycle/made_ready_tests.rs` (:288, :350), `made_ready_review_tests.rs` (:65, :119) | The set-ups start from Closed with typed text through `set_ticket_query`, assert the model query before the click, and after `open_ready_tickets` assert (Open, Some(Ready), "") from the model; `filter-ready` and `filter-blocked` element reads become model reads until Task 3 draws `filter-readiness` | 2 | no coverage lost beyond swapping element reads for model reads |
| E-3 | `D/view/tickets_tests.rs` (the six tests, plus `arrows_move_in_the_list_and_enter_opens`, which only clicks `filter-all`) | Rows no longer carry "Showing …", inline `ticket-reason-*`, problem-code cells or `row_description`. The tests now assert the `ticket-closed` marker, the "needs fixing" pill and type cell, Why blocked lines for the selected row, `ticket_tooltip` lines, segment names with counts and "Readiness: …"; each still compares with `list_tickets(TicketFilter{closure, readiness})` | 3 | yes: "Showing …", inline reasons, problem-code cells, `row_description` |
| E-6 | `D/view/keyboard_tests.rs` `change_the_filters` | New Tab order (segments, readiness, input) and the readiness menu; the "Showing …" checks are gone | 3 | yes: "Showing …" |
| E-10 (hidden panel part) | `D/view/dock_tests/state.rs` `a_hidden_panel_keeps_its_state` | The kept text is `s`, and the test also chooses a sort and Ready and checks the model query | 3 | no |
| E-10 (made-ready part, finished) | `made_ready_tests.rs`, `made_ready_review_tests.rs` | `set_ticket_query` replaced by `set_closure` plus `set_filter_text` through a `type_filter` helper; readiness from `filter-readiness`'s name; the cleared input asserted | 3 | no |
| E-12 | `D/keys_tests.rs` `each_key_is_bound_to_its_action` | `home` and `end` added to the table (array length 24) | 3 | no |
| E-4 | `D/view/find_tests.rs` | All eight tests rewritten for the filter input. Renamed: `secondary_p_focuses_find` to `secondary_p_focuses_the_filter_input`; `a_ulid_opens_its_ticket_closed_or_not` to `a_ulid_opens_its_item_whatever_the_filters`; `a_shared_short_code_lists_both_and_choosing_one_opens_it_by_id` to `several_matches_move_focus_to_the_list_on_the_first`; `no_match_says_so_and_opens_nothing` to `nothing_found_says_so_and_opens_nothing`; `a_held_search_never_lands_after_the_selection_changed` to `a_held_id_read_is_dropped_when_another_repository_is_selected`. `find_needs_a_selected_repository_and_a_new_selection_clears_it` split into `enter_needs_a_selected_repository` and `selecting_another_repository_clears_the_text_and_its_rows`. `a_matchs_open_is_reached_by_tab_and_enter_opens_it` deleted. `find_does_nothing_while_a_dialog_is_open` keeps its name | 4 | yes: match cards, their context, ID and lifecycle labels, Open buttons, the `FIND_MATCHES` and `FIND_NONE` texts, the `FindSlug` hold |
| E-5 | `D/model/find_tests.rs` | The short-code test became the `parse_id` table; `matches_are_the_librarys_and_none_says_so` and its `found()` helper deleted; `an_earlier_search_is_dropped` uses `apply_item`; `an_id_found…` gains a Busy step | 4 | yes: the short-code parse and `apply_matches` assertions |
| E-6 (find part) | `D/view/keyboard_tests.rs` | The walk's `find` is split into `filter` (types) and `find` (an ID, Enter, focus stays in the input); `find_by_short_code_and_ulid` checks no tab opens, focus in the list on the first row, Enter opens it by ID, then a closed ticket's ULID opens with focus staying in the input | 4 | yes: the `find-state`/`FIND_MATCHES` check and the match-card Tab steps |
| E-16 | `D/model/documents_tests.rs`, four sites | `model.load("/a")` becomes `let (stamp, _) = …` because `Documents::load` returns `(Stamp, bool)` | 5 | no |
| E-17 | `D/view/documents_tests.rs` `the_tree_shows_what_the_library_reads_give` | Added: select `Nested`, collapse `docs/guides` through the model, reload, assert rows `["empty","guides","Top"]` and the selection on `guides` | 5 | no |
| E-18 | `D/view/new_folder_tests.rs` `new_folder_creates_and_explains_local_folders_and_lists_it` | Added: collapse `docs/plans` before the second create, then wait for the `drafts` row (needs the reveal) | 5 | no |
| E-19 | `D/view/move_document_tests.rs` `move_from_the_sheet_commits_both_paths_and_the_tab_follows` | Added: collapse `docs/archive` before the move, so its final tree assertion needs the reveal | 5 | no |
| E-20 | `D/model/documents_tests.rs`, `D/view/documents_tests.rs`, `D/view/documents/row.rs`, `D/view/documents.rs` | Import-only edits for the `doc_rows.rs` split | 7 | no |
| E-21 | `D/view/documents_tests.rs` `find_document` | The exhaustive match on `TreeNode` gains `Unreadable(_)`: `Document(_) \| Unreadable(_) => None`. (The plan's `:100-111` match is `shape()` in `model/documents_tests.rs`, which gained a `Shape::Unreadable` arm as a plan-named extension.) | 7 | no |
| E-15 | `D/view/polling_privacy_tests.rs` (:687, comments :15, :96, :493) | Part F moves the path from `doc-path` to the tooltip, which the snapshot cannot read; the `("region-documents", PATH)` pair becomes `("region-documents", TITLE)` (see Rulings, Task 6); comments updated | 6 | yes, the `PATH` pair (replaced by a `TITLE` pair; the existing title pair stays; the `PATH` sentinel is still swept as absent) |
| E-8 | `D/view/documents_tests.rs`, `new_folder_tests.rs`, `move_document_tests.rs` | `flatten` uses `doc_row(node, true).label`, so rows read `marker.md` and the like; `doc-empty` reads `EMPTY_TRAILING`; `doc-nonconforming` is named by `doc_row`'s tooltip[0]; an open folder has no `doc-count`; `arrows_move_in_the_tree_and_enter_opens` asserts Enter toggles a folder and a click on a document row opens it by target (item and by path); `a_held_tree_of_a_never_replaces_bs` reads `only-in-b.md`; `tree_paths` calls `documents_pane().document_paths(cx)` (visible rows only, so E-19's reveal is still exercised) | 6 | yes: the `doc-path` second line, the "Nonconforming" text, the `doc-problem` code, "Empty folder", and the `doc-path`/`doc-problem` absence checks (their ids no longer exist) |
| E-9 | `D/view/keyboard_tests/actions.rs`, `editing.rs` | Look documents up by file name (`MARKER_FILE`, `NOTES_FILE`) instead of the old path labels | 6 | no |

## Additional Tests

| Test | Why it catches something existing tests don't | Task |
| --- | --- | --- |
| `documents::tests::unreadable_files_sit_in_their_folders_and_are_kept_with_the_tree` | The plan extends only the tickets model test; Documents' merge of the files into the tree (placement, counts, selection, keep and replace rules, a status read during a reload) is new logic with no other test | 7 |
| Extra cases in `ticket_view::tests::the_footnote_counts_what_needs_attention` (two unreadable rows) and in `an_unreadable_file_is_listed_with_its_reason` (real-data `primary`) | Review hardening: a count above one, and `primary == (worktree == root)` against real paths | 7 |
| `documents::tests::the_tree_says_each_list_state_and_acts_on_it` | The plan's one test names only stale and never-refreshed for Documents; this covers the skeleton while the first read is held, a reload with "Updating…", the busy-over-kept alert (via `hold_read(Folders)`, `hold_read(Documents)` and a cache-write lease), Retry through `reload_lists`, empty with New document…, P27 folders-only, stale and never-refreshed with Refresh (registry SQL), failed with its message, and an unavailable index with Rebuild | 6 |
| `documents::tests::a_document_with_unsaved_work_is_marked_at_once` | `doc-unsaved` and its no-read rule (P14) would otherwise have no test | 6 |
| `documents::tests::the_tree_is_kept_over_a_reload_of_the_same_repository` | The kept tree, `last_loaded` and the changed flag are new `KeptList` behaviour no plan test pins | 5 |
| `operations::tests::a_folder_made_or_a_document_moved_names_its_place_to_reveal` | A table over ok, `discovery_pending`, `already_applied` and refused; pins the new `Changed.reveal` | 5 |
| `documents::tests::rows_read_as_the_design_writes_them`, extra rows (11 over the plan's seven kinds, including an untitled document) | Review hardening: the file-name label for an untitled document | 5 |
| `ticket_view::tests::a_sole_match_is_the_one_conforming_ticket_the_text_matches` | The pure logic P9's Enter depends on, at the cheapest layer | 4 |
| `find::tests::the_status_line_says_where_the_id_read_is` | A pure table of `status_text` over every `FindState`; the only place the busy and failure wording is pinned, since no view seam makes a busy ID read cheaply | 4 |
| `find::tests::enter_selects_from_the_rows_the_text_lists` | R-32: Enter brings the pane up to date before it selects; real keystrokes render between typing and Enter, so only a same-frame call shows it | 4 |
| `find::tests::the_text_is_cleared_once_per_change_of_repository` | Review hardening: text typed before the first selection clears; text typed while a resolve is held is kept; mutations of the guard and of "first load is a change" passed every other test | 4 |
| `workspace::tests::a_selection_that_goes_drops_a_held_id_read` | Review hardening: select A, `reload_lists`, hold an ID read, forget A; failed first, found a regression | 4 |
| Task 3 added none beyond the plan's three (`the_filter_bar_works_without_a_read`, `list_states_act_and_a_reload_keeps_the_rows`, `markers_appear_without_a_read`); the review's fixes extended the second (Retry held, stale and never-refreshed Refresh, focus after Retry, Refresh and Rebuild) | Retry through `load_lists` would drop the kept rows and the count | 3 |
| `status::tests::statuses_tint_by_their_words` | Pins "ready" as Done and "waiting"/"not started" as none (design: status colour, decision 20); a pure function, not an appearance test | 2 |
| `tickets::tests::only_text_or_readiness_narrows_the_query`, `tickets::tests::a_sort_toggles_by_column` | `narrowed` and `toggled` are logic with no other test and would be dead code under `--all-targets` | 2 |
| `ticket_view::tests::the_footnote_counts_what_needs_attention` and extra `derive` rows ("Closed after Ready", text "vh-" under Closed, footnotes on the All and "zzz" rows) | Pins the plural and singular branches of the footnote and the hint, and the unreadable text Task 7 reuses; the plan's own rows are unchanged | 2 |
| `list_view::tests::each_state_has_its_own_body`, case: empty rows with `listed` 1 expects `Rows` | Review hardening: pins P27 against a regression that used `rows.is_empty()` | 1 |
| `list_state::tests::kept_rows_follow_reloads`, row `Marked`: an incomplete, stale-index ready state keeps both through busy and reload | Review hardening: a busy read over kept rows restores the kept state whole | 1 |

## Tasks

| Task | Commit range | Tests run (exact names) | Review | Notes |
| --- | --- | --- | --- | --- |
| 0 | base `38c64b3c`, ledger `c6a22b97`, comment `002428b1` | none | n/a | gates green |
| 1 | base `002428b1`, task commit `e7026fe1` | `desktop::model::list_state::tests::kept_rows_follow_reloads`, `desktop::model::list_view::tests::each_state_has_its_own_body`, `desktop::assets::tests::every_item_view_icon_loads` (1 run each, passed; seen failing first: two compile errors, one "repeat errored"); fmt, `check --features desktop`, clippy all pass | one fresh reviewer: 0 Blocking, 1 Important (Documents `narrowed`), 5 Minor; fixed and re-run | rulings above; reviewer's items 3 (kind-free alert label) and 6 (recorded) not changed |
| 2 | base `e7026fe1`, task commit in the git log | `ticket_view::tests::{derive_filters_counts_and_sorts, ages_read_as_the_prototype_writes_them, tooltips_carry_the_id_context_and_reasons, the_footnote_counts_what_needs_attention}`; `markers::tests::markers_follow_drafts_conflicts_and_closure`; `status::tests::statuses_tint_by_their_words`; `tickets::tests::{a_read_lands_only_for_its_repository_and_load, the_default_query_lists_open_tickets, only_text_or_readiness_narrows_the_query, a_sort_toggles_by_column, blocked_reasons_name_the_blocker_the_missing_id_and_the_cycle, an_untitled_blocker_is_named_by_its_short_code}`; `model::item::tests::provenance_names_the_context_kind_its_branch_and_worktree`; `view::item::tests::the_provenance_says_primary_or_active_with_its_branch_and_worktree`; `view::lifecycle::made_ready_review_tests::open_ready_tickets_on_the_shown_repository`; `view::lifecycle::made_ready_tests::index_unavailable_final_offers_open_ready_tickets`; `view::tickets::tests::{arrows_move_in_the_list_and_enter_opens, each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry, ready_and_blocked_show_the_librarys_rows_with_reasons_and_keep_the_nonconforming_entry, the_why_blocked_panel_gives_the_selected_rows_reasons_in_full, the_panel_says_incomplete_where_the_library_does}`; `view::item::edit_tests::sheet_fields::suggestions_come_from_the_loaded_tickets_values` (unchanged): 22 run, all passed on the final tree; fmt, check, clippy pass | one fresh reviewer: 0 Blocking, 2 Important (reasons under the default query; footnote plurals), 8 Minor; fixed, re-run | see Rulings, process note, handover |
| 3 | base `f360e8bb`, task commit in the git log | `view::tickets::tests::{the_filter_bar_works_without_a_read, list_states_act_and_a_reload_keeps_the_rows (5 of 5 runs), markers_appear_without_a_read, each_closure_filter_shows_the_librarys_rows_and_keeps_the_nonconforming_entry, ready_and_blocked_show_the_librarys_rows_with_reasons_and_keep_the_nonconforming_entry, the_why_blocked_panel_gives_the_selected_rows_reasons_in_full, the_panel_says_incomplete_where_the_library_does, every_choice_has_its_own_label_and_the_chosen_one_is_said_in_text, a_long_why_blocked_panel_scrolls_by_keyboard, arrows_move_in_the_list_and_enter_opens}`; `view::keyboard_tests::every_core_action_works_by_keyboard_alone`; `view::dock::tests::state::a_hidden_panel_keeps_its_state`; `keys::tests::each_key_is_bound_to_its_action`; the two made-ready tests (earlier in the task): 12 run together on the final tree, all passed; fmt, check, clippy pass | one fresh reviewer: 0 Blocking, 3 Important (Retry not pinned to `reload_lists`; `chmod` trick breaks on Windows and as root; stale and never-refreshed states makeable by SQL), 5 Minor (drag ghost fixed; per-render clone, allows, Documents Home/End, test file length recorded); Important and the drag ghost fixed, re-run | see Rulings (Task 3), E-3, E-6, E-10, E-12, handovers to Tasks 4 and 6 |
| 4 | base `8946bff7`, task commit in the git log | `view::find::tests::*` (14: the 13 plus `the_text_is_cleared_once_per_change_of_repository`); `model::find::tests::*` (3); `model::ticket_view::tests::a_sole_match_is_the_one_conforming_ticket_the_text_matches`; `view::workspace::tests::a_selection_that_goes_drops_a_held_id_read`; `view::keyboard_tests::every_core_action_works_by_keyboard_alone`: 20 passed on the final tree (the last gate edit restored one clippy `allow`; fmt, check, clippy re-run green by the orchestrator). Neighbours, by filter, earlier in the task and unchanged since: 20 `Walk::find` callers and 64 `clear_lists` neighbours passed | one fresh reviewer: 0 Blocking, 2 Important (a held ID read survived the selection going to None; the once-per-change guard and first-selection clear untested), 3 Minor; both Important and the blank-Enter test fixed, re-run | see Rulings (Task 4), E-4, E-5, E-6 (find part), Additional tests, handover to Task 8 |
| 5 | base `ce46c643`, task commit in the git log (Step 0: origin/main still `48653d01`, already merged) | `model::documents::tests::{expansion_survives_rebuilds_and_collapse_all, rows_read_as_the_design_writes_them, the_tree_is_kept_over_a_reload_of_the_same_repository, the_tree_is_ready_once_both_reads_are, either_read_failing_fails_the_tree, an_old_repositorys_reads_are_dropped}`; `model::operations::tests::a_folder_made_or_a_document_moved_names_its_place_to_reveal`; `view::documents::tests::the_tree_shows_what_the_library_reads_give`; `view::new_folder::tests::new_folder_creates_and_explains_local_folders_and_lists_it`; `view::move_document::tests::move_from_the_sheet_commits_both_paths_and_the_tab_follows`; `view::workspace::tests::a_busy_list_reload_keeps_the_last_rows_with_a_label`: each 1 passed on the final tree; fmt, check, clippy green. Earlier in the task, by module prefix, once: 177 neighbour tests passed (documents, new_folder, move_document, repair, operations, item, polling_privacy, workspace, dock state, tabs, and four keyboard walks) | one fresh reviewer: 0 Blocking, 0 Important, 7 Minor; the untitled-label row and the dead `load_documents` guard fixed, re-run; the rest recorded | see Rulings (Task 5), E-16 to E-19, Additional tests, handovers to Tasks 6, 7 and 8 |
| 6 | base `ec8ecb39`, task commit in the git log | `view::documents::tests::{folders_keep_their_state_and_toggle_by_key, the_tree_shows_what_the_library_reads_give, arrows_move_in_the_tree_and_enter_opens, a_held_tree_of_a_never_replaces_bs, the_tree_says_each_list_state_and_acts_on_it, a_document_with_unsaved_work_is_marked_at_once}`; `keyboard_tests::every_core_action_works_by_keyboard_alone`; `polling_privacy_tests::polling_privacy`: 8 passed together on the final tree. Earlier in the task, by exact name, 20 passed (adding `documents::tests::new_folder_opens_its_dialog`, the new_folder and move_document tests, `operations::tests::a_refresh_keeps_the_selected_document`, `keyboard_tests::actions::a_nonconforming_entry_opens_by_keyboard`, `keyboard_tests::editing::{repair_and_adopt_by_keyboard_alone, create_by_keyboard_alone, move_and_new_folder_by_keyboard_alone}`, `create::tests::*`, `editing_privacy_tests::sentinels_reach_no_surface_through_create_move_and_repair`, `workspace::tests::a_busy_list_reload_keeps_the_last_rows_with_a_label`); the reviewer ran `tickets::tests::list_states_act_and_a_reload_keeps_the_rows` and `dock::tests::state::a_hidden_panel_keeps_its_state` on the same code; fmt, check, clippy green | one fresh reviewer: 0 Blocking, 1 Important (a document row click opening it untested), 5 Minor; the Important and three Minor (P25 re-baseline, failure message, Enter on an empty folder) fixed, re-run; the rest recorded | see Rulings (Task 6), E-8, E-9, E-15, Additional tests, known gaps |
| 7 | base `c42ab081`, task commit in the git log | `model::unreadable::tests::unreadable_files_go_to_their_list`; `model::ticket_view::tests::{derive_filters_counts_and_sorts, tooltips_carry_the_id_context_and_reasons, the_footnote_counts_what_needs_attention}`; `model::documents::tests::{rows_read_as_the_design_writes_them, expansion_survives_rebuilds_and_collapse_all, unreadable_files_sit_in_their_folders_and_are_kept_with_the_tree, the_tree_is_kept_over_a_reload_of_the_same_repository}`; `model::tickets::tests::a_read_lands_only_for_its_repository_and_load`; `view::tickets::tests::an_unreadable_file_is_listed_with_its_reason`; `polling_privacy_tests::polling_privacy`: each 1 passed on the final tree; fmt, check, clippy green. By exact name, 60 neighbours passed before the review fixes (list-state, read-count, tabs, dock, operations, find, new_folder, move_document, made-ready and keyboard tests; nothing needed an edit for the extra IndexStatus read) | one fresh reviewer: 0 Blocking, 2 Important (open_ready_tickets starts no status read: recorded; tickets_tests.rs size: split moves to Task 8), 5 Minor; the footnote count, the find.rs comment and the real-data `primary` fixed, re-run | see Rulings (Task 7), the split, E-20, E-21, Additional tests, handover to Task 8 |
