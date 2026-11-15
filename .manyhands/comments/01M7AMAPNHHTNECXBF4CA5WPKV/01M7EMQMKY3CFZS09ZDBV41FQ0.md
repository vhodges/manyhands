---
id: 01M7EMQMKY3CFZS09ZDBV41FQ0
item_id: 01M7AMAPNHHTNECXBF4CA5WPKV
created_at: 2026-11-15T03:41:37.799382596Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
External review handled, 2026-11-14.

The owner pasted two external reviews (8 findings each); the reviewer's line numbers were not file lines (they look like diff offsets), so every finding was verified against the code by a read-only investigator before any change. Details, with evidence and rulings 56 to 59, are in the ledger section "External review".

**This branch's review (8 findings).**
- Fixed: the Settings scroll offset now returns to the top when the menu entry shown changes (a reopen of the same entry keeps it, per decision 6): Ruling 56. `open_settings_on` no longer ignores `root` (the `page` parameter is gone). P19's inline real-time poll uses the shared helper. A comment on the retarget's `let _`. Commit `5329ee40`.
- Refuted, no change: `land_retarget`'s `take()` (R-22's "once", Ruling 44), and reselecting the panel's repository overriding a Manage choice (R-14 as you approved it, P10).
- Accepted earlier: the Kit workaround in `kit_button.rs` (Ruling 55).
- Parked as cosmetic: the remotes and Trusted servers read twice on open when the chosen repository changes; per-key focus handles never pruned.
- Copy tests stay: P18 and P21 are design outcomes, and P21 does not scan sources. The real-time polls are harness helpers that pre-date the branch.

**A defect the checking found, not in the review (fixed): a route's field could sit below the fold.** The Remotes routes (W-9, W-10, W-11, W-15, W-24, W-31, W-35) and the Hosts routes on a first visit to Sharing & keys with several keys focused their field correctly, but the page's reads then grew the content above it, and the field ended 700 to 1100 px below the visible area with no second reveal. The Settings sheet's own scroll region now re-reveals its focused control while the user has not scrolled (Ruling 57); the item regions and the content column are unchanged. Commit `3e48d416`; P8 gained rows that were red before. Checklist row S-10.15 covers it on screen.

**The flaky test.** `the_more_menu_acts_on_the_chosen_repository` failed in 2 of 3 full runs of the desktop binary here. It pre-dates the branch (Cycle 03b's T15-2; the test and path are identical on main). Cause supported by the code, not reproduced: the test never waits for its index rebuild, which can hold the data directory's cache lock when the removal's preview needs it, so the preview answers Busy and the test waits 180 s. Test-only hardening, commit `f0cfaa39`: wait for the rebuild, and fail at once with "answered busy". It touches Cycle 03b's test because it fails this branch's gate. Native CI after merge may still hit this class intermittently.

**The second review (Cycle 03b's code on main, 8 findings).** None touch files this branch changes, and none were changed here. Verified: the footer bounds map is stale for a frame at most (a few pixels, self-healing); the share-result classification cannot be reached; `rows_fit_a_narrow_activity` is a layout test that 03b's T15-1 kept on purpose (yours to delete); the titles and scheduler maps are 03b's parked Minors (K-4); the Needs attention popover's Enter logic parallels the Activity panel's; per-burst row derivation is unmeasured; the double timezone lookup per row is microseconds. New to 03b's record: the bounds map, the share classification, the duplicated Enter logic, the per-burst re-derive and the timezone lookup. Whether any becomes a follow-up is yours to decide; no ticket was created.

**Gates.** After the fixes: `cargo check --all-features --locked`, `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` exit 0 at `3e48d416`; the desktop binary ran again (1938 passed, 1 failed: the flake above, before its hardening); the hardened test and its module pass. The fast tier was not run again after `f0cfaa39` (test-only, 12 lines).

**Branches published:** this ticket's branch only. Still yours, not done: the on-screen checklist run (now S-10.15 too), review approval, `ticket close`, a pull request, native CI.