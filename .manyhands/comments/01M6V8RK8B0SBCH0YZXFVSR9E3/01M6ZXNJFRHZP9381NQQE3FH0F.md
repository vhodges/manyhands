---
id: 01M6ZXNJFRHZP9381NQQE3FH0F
item_id: 01M6V8RK8B0SBCH0YZXFVSR9E3
created_at: 2026-11-09T10:29:09.821825862Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Dock checkpoint (Tasks 1 to 4b, 2026-11-09). Ledger: docs/plans/2026-11-08-wave-04-cycle-01-workspace-shell-execution.md.

**Commits.** Task 1 `a9caf276` (chrome tokens, E-7). Task 2 `0db81083`, fix `500ffd72` (pure layout model). Task 3 `1820d575` (needs_attention, window_title). Task 4a `2c772231` (five regions, Navigation split; E-1 to E-4, one E-11). Task 4b `5922bbaa` (Kit dock area, panels, default layout, harness; E-10, one E-11). Every task was reviewed by a fresh reviewer and approved; Tasks 2 needed one fix round. Both Boundary commands print nothing after each task; Cargo.lock is untouched; origin/main is still 8da9afd4, so the Task 4b merge was a no-op.

**U-4 and U-6 hold.** A panel's render can update the Workspace (first dock test passes), and the 0.6:1.4 left split reads 0.3:0.7 within 1 px.

**Tests run (exact names).** New: layout model 3, attention 1, title 1, create kinds 1, dock 2. E-10's 96 helper callers passed. Between-step sweeps wider than the exact-name rule were run twice (Tasks 4a, 4b); noted in the ledger.

**Rulings.** Task 4b: one RegionPanel with a slot instead of five types; harness height constant SHELL_CHROME_HEIGHT (0 now, +38 in Task 6, +28 in Task 7); R-38 done at build; the extra builder test declined.

**Owner decision (2026-11-09).** One test, a_draft_not_saved_on_quit_by_keyboard, checks a Retry that falls outside Activity's clip: the draft-flush entry in operations.rs is one unwrapped line, and the Activity dock is as wide as the item area. Owner chose to widen that test (E-11, no assertion removed) and record the limit.

**Proposed Known Limit K-19 (for the owner).** At the default 1280 px window Activity is about 550 px wide. Three unwrapped rows in operations.rs (draft-flush Retry, pending repository Retry, pending controls such as Abandon) can sit outside the visible Activity. Not fixable in this Cycle (operations.rs is not edited); belongs to Cycle 03's Activity rebuild or D6.

**Assertions removed.** Only E-3's approved ones (the choice and pane-showing assertions). No other.

**Next.** Task 4c (panel header), then Task 5 (layout operations, with its design checkpoint). Carried to Task 5: compare arrangement and dock open state in the LayoutChanged handler, not split sizes.