---
id: 01M7DHVX6MR2XCWZKJ0PN0TSVF
item_id: 01M7AMAPNHHTNECXBF4CA5WPKV
created_at: 2026-11-14T17:32:17.314854533Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 2 complete (where focus goes), design checkpoint passed, 2026-11-14.

**Commits** on `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV` (pushed, HEAD `6ae0ad14` equals origin; `git merge-tree --write-tree HEAD origin/main` clean, main still `ea6d899b`):
- Task 1 (the swap), reviewed first time: code `0d5501e7`, ledger `023ebf47`.
- Task 2, base `023ebf47`: code `5cddad28`, fix round 1 `6894d70d`, ledger `6ae0ad14`.

**The design checkpoint (A6, F1-F19).** The implementer wrote every row as a test, read after a draw, before any focus code. Result: 19 of 19 rows expressed; 16 failed for the right reason (focus assertions) before the mechanism, and all pass after. No row was edited or weakened, none was unmeetable.
- **One part could not be expressed against today's code: F2's "a field not drawn".** No existing route names an undrawn field. A6's Test column gives F2 to P4 (Task 6, with R-22's key list read while it is still loading), so it is a deferral, and Task 6's P4 must carry that row. Raising it here rather than silently.
- Two sub-rows cannot fail after a draw by nature (the save-banner F9 row; the P22 Hosts after-draw read) and say so in comments; discriminating rows sit beside them (an added F9 row, the Add row).

**Review.** Task 2's reviewer found 1 Important: the keep-focus-in-the-sheet check ran inside the Workspace's render, before child views draw, so a Drafts landing could leave focus behind the sheet until some unrelated redraw (the tests force their draws, so they could not see it). Fix round 1: the check now runs after the frame (`cx.defer_in`), still moves only focus it positively finds behind the sheet (A7.1-A7.3), and Drafts' refocus is dropped while a sheet is open. A new test reads focus with no forced draw (RED before, GREEN after). Scoped re-review: addressed, no new breakage.

**Tests.** P5 19/19, P22 9/9, P23 13/13, the F1/F2 test 6/6, Task 1's eight (P1, P2, P3, P6, P7, P8, P14, P15) pass, plus `a_landing_drawn_behind_settings_leaves_focus_inside`; 13 of them in one exact-name run after the fix. Module runs: 181 + 146 + 54 passed (Task 2's list, plus a named doubt), then 96 after the fix (`drafts::`, `workspace::tests`, `footer::tests`, `item::sheet_tests`). Task 1: 305-test module run 303 passed, 2 failed (one real, fixed; one a 180 s harness timeout under load, passed alone), then 101 passed. fmt, `cargo check --features desktop` and clippy `-D warnings` clean at each commit. No whole-suite run, no SSH target. The ledger lists every test by name.

**Rulings** (ledger, Rulings 12-24): the origin kept on `SettingsHost` (12); A2 step 3 done in Task 1 (13); R-39 taken early (14); a latent `in_window` production bug fixed (15: it leased Root, so every item route would have panicked); R-7 loses to A7.10 (20); Step 5 is a post-frame check with positive containment, not the listener (22); a chosen menu returns to its context (23); Drafts drops its refocus under any sheet (24). A7's ten items and how each is met are in the ledger.

**For the owner (nothing blocks):**
- After a page dialog closes within 0.25 s of an earlier deferred close, focus goes to the dialog's origin and then jumps to the page's entry when the Kit's restore fires (F15 then F16). A6 says so and the tests assert it; it may look odd on screen.
- F14's EditsNotKept row is unix-only and P23's context-menu rows run only on Linux; macOS and Windows native CI after merge will not run them.
- Task 1 removed assertions in E-1, E-2, E-4 and E-5 as the design's named edits say, and E-24/E-25 shortened two existing tests' windows (the sheet is as tall as the window); all are in the ledger.
- Activity row titles in `services/requests.rs` ("Select remote", "Approve host" and others) keep the old words; they are not Settings copy.

**Branches published:** this ticket's branch only.

**Next:** Task 3 (one chosen repository).