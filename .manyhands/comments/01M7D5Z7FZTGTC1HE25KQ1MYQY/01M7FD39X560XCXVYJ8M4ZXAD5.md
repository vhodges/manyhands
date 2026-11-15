---
id: 01M7FD39X560XCXVYJ8M4ZXAD5
item_id: 01M7D5Z7FZTGTC1HE25KQ1MYQY
created_at: 2026-11-15T10:47:25.841950036Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 4 checkpoint, 2026-11-15 (execution session).

**Where.** Tasks 0 to 4 are done and pushed. Task 4 (the Add sheet) is commit `0d5cae5c` (base `2c398b03`). Cycle 04 landed on main on 2026-11-15; `origin/main` (`14ca0938`) was merged into this branch at `9059778f` (it changes only Cycle 04's `ticket.md`; no code change, no test re-run needed). From now on the merge source is main.

**Design checkpoint (the handoff's stop).** Task 4's implementer wrote P3, P4, P5, P6, P7, P8, P15 and P23 over B9's rows G1 to G15, G17 to G19, G21 and G23 before any code, all failing first. Four rows were answered differently by Cycle 04's merged code and one sub-case was ambiguous, so the orchestrator stopped and the owner ruled (all four the recommended option):

- **B-1:** the Add sheet's open records the panel's selection as Settings' `seen`, so E-17c's "footer after Inspect(b)" rows keep `a`.
- **B-2 and B-4:** "from Settings" is carried separately from the origin handle, so G4 holds with no recorded origin; an in-place route keeps the return.
- **B-3:** the whole `add_closed` waits while a dialog is open (B3's text is corrected: steps 0 and 1 wait too).
- **B-5:** while an add is pending, focus goes to the button's own handle.

Orchestrator rulings on the other concerns (Ruling 19): follow R-29 (`registered_root` on `DesktopRequest`, E-17d); a confirmation stacked under an Unlock prompt closing the wrong dialog is a Known Limit (K-16: the Kit closes the topmost dialog), no ticket; typed text is cleared only for the same open.

**A Kit fact for the design.** One Escape can run a closed sheet's close again two to six times, and each repeat closes whatever sheet is open then. Cycle 04 never met it; the Add sheet does, because its close opens Settings. Fixed by opening Settings after the event (`defer_in`); Task 0's "the Kit's close order leaves the slot free" holds for the first close only.

**Tests.** The orchestrator's module run (17 modules, each matched at least one test): **198 passed, 0 failed.** The implementer's exact-name runs: 35 of 35, and a mutation check failed G23, G19 and P15's Ctrl+, row as expected. Static gates pass.

**Test edits** (no test function removed or renamed). Assertion changes and removals, as the review-ready summary will list them:

- changed: E-1e (reads the sheet's own scroll), E-5, E-6, E-7, E-8 (Settings no longer opens from `+`), Cycle 04's P2 and P8 (the Add form left the page, so a repository is selected first and the Add button is the landing);
- removed: P4's "F1 Inspect(root): the choice stays (R-55)" (the route is gone; P4's E-17c rows assert it through the Add sheet), P22's `pending_add` row (moved to this Cycle's P8, G12, stronger), and E-17c's "Inspect(b), panel now b: chosen" fact;
- added: E-17d, `registered_root: None` on 22 literals in 21 test files and 2 production literals.

**Review** (opus): spec compliant, quality approved, no Critical or Important. Nine Minors are parked in the ledger. Two for the owner's eye: Ruling 15 is realised on every open of the Add sheet, `+` included (as its text reads), so after `+` a changed panel selection is consumed and the next plain Settings open no longer follows it; restricting it to Inspect is one line. And a success landing on a later open of the sheet leaves a stale "not registered" check for the folder just added (a pointer for Task 5).

**Branches published:** `manyhands/ticket/01M7D5Z7FZTGTC1HE25KQ1MYQY` (this one), at `f4059706` plus this comment. No other branch.

**Next:** Task 5 (Add an existing folder).
