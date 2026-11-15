---
id: 01M7DY4ACRKMQNPR086ENPKM4G
item_id: 01M7AMAPNHHTNECXBF4CA5WPKV
created_at: 2026-11-14T21:06:35.900640966Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Tasks 3 to 6 complete (one chosen repository, Sharing & keys, three-entry menu, ways in), 2026-11-14.

**Commits** on `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV` (ledger `6d86c8f7`; main still `ea6d899b`, `git merge-tree --write-tree HEAD origin/main` clean):
- Task 3, review approved after fix round 1: code `29ef847d`, fix `1b3ca49a` (an Inspect open could change the chosen repository; Ruling 25), ledger `00a179d5`.
- Task 4, approved first time: code `a7dda502`, ledger `86b51214`.
- Task 5, approved first time: code `d9327047`, ledger `16f7746b`.
- Task 6 (every way into Settings lands on its page and control, Part D), approved after fix round 1: code `4a9a0dd7`, fix `4fd65ef0` (a file-size rule: `add_path_focus` moved out of `repositories.rs` into `repositories/manage.rs`), ledger `6d86c8f7`.

**Task 6 evidence.** P4 `each_route_lands_on_its_page_and_control` is 100 rows, each read after two draws; P20 `routes_by_keyboard` is 16 rows by real keys; P2's AddRepository row. Each was RED before the code. The carried deferrals are all met: F2's "field not drawn" (the key-list route while the keys are still being read, then one retarget to the radio), `Hosts(path)` with the panel on another repository, Inspect leaving the choice alone, R-14 both ways, `aria_selected` and `first_focus` on the non-first pages, W-36 through `choose_activity`, and a row for every W no existing test raises. Module runs, once each: 122, 84 and 36 tests passed. fmt, check and clippy -D warnings are clean. Cargo.lock and the boundary are untouched. Test edits E-17, E-18, E-21 follow Part D; E-32 shares P5's helpers with P20; E-33 deletes `focus_on_open_survives_the_sheet_keeping_focus`, all six rows kept in P4.

**Rulings worth knowing** (all in the ledger): Approve host... with no server named now uses the row's own repository, not the panel's (R-23); Cycle 03b had no view test for that case, so three new P4 rows pin it. `open_settings(Keys)` builds the `Keys` route so the callers stay unchanged. The panel's Settings button (W-4) stays until Task 8.

**For the owner (nothing blocks):**
- F2's "field not drawn" half was a documented deferral from Task 2; Task 6 closes it.
- After a page dialog closes within 0.25 s of an earlier deferred close, focus goes to the dialog's origin and then jumps to the page's entry (F15 then F16).
- F14's EditsNotKept row is unix-only and P23's context-menu rows run only on Linux; macOS and Windows native CI after merge will not run them.
- The Kit buttons' key-ID element IDs half-work (the wrapper's ID is row-indexed), so after a key changes row, focus falls to a wrapper with no focus ring. This goes on the on-screen checklist, with the Kit button focus ring on the menu entries and the icons.
- "Make a new key" does nothing during a retry, and the "Selected key: ..." line is kept.
- Activity row titles in `services/requests.rs` keep the old vocabulary; they are not Settings copy.
- Parked Minors for Tasks 1 to 6 are in the ledger; the final review gets the list.

**Branches published:** this ticket's branch only.

**Next:** Task 7 (copy, Part E).