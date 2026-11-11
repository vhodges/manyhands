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

## Test Edits

| E | Site | Reason | Task | Removes an assertion |
| --- | --- | --- | --- | --- |
| E-11 | `D/assets_tests.rs` `every_item_view_icon_loads` | The table and its array length (37 to 47) gain `repeat`, `bug`, `sparkles`, `list-checks`, `file-plus`, `folder-plus`, `chevrons-down-up`, `arrow-up-down`, `list-filter`, `file-exclamation-point` | 1 | no |
| E-1 | `D/model/tickets_tests.rs` | The five tests became `a_read_lands_only_for_its_repository_and_load` (one table, 12 steps, plus the review's reasons-under-the-default-query step); `the_default_query_lists_open_tickets` asserts the default query and `read_filter() == TicketFilter::default()`; the two `blocked_reasons` tests use `TicketFilter` literals; `empty()` and the `TicketRead` import removed | 2 | yes: the filter-to-library-filter mapping and "a filter change drops the read" assertions, as E-1 says |
| E-2 | `D/view/tickets_tests.rs` | Deleted `a_filter_change_drops_the_held_read_of_the_last_filter` and `a_readiness_change_drops_the_held_read_of_the_last_one`; `library()`, `blocked_list` and the readiness query take `TicketFilter` literals; unused imports removed | 2 | yes: those two tests' assertions, as E-2 says |
| E-10 (made-ready part) | `D/view/lifecycle/made_ready_tests.rs` (:288, :350), `made_ready_review_tests.rs` (:65, :119) | The set-ups start from Closed with typed text through `set_ticket_query`, assert the model query before the click, and after `open_ready_tickets` assert (Open, Some(Ready), "") from the model; `filter-ready` and `filter-blocked` element reads become model reads until Task 3 draws `filter-readiness` | 2 | no coverage lost beyond swapping element reads for model reads |
| E-15 (expected) | `D/view/polling_privacy_tests.rs:687`, comments :15, :96, :493 | Part F moves the path from `doc-path` to the tooltip, which the snapshot cannot read | 6 | yes, the `("region-documents", PATH)` pair; the title pair stays |

## Additional Tests

| Test | Why it catches something existing tests don't | Task |
| --- | --- | --- |
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
