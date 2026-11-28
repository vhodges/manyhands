---
id: 01M7GY81P1YY40WJGYCKVDD2YM
item_id: 01M7D7JK5MSY6J471JEKG3CZXR
created_at: 2026-11-16T01:06:21.412315301Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint after Tasks 5 and 6 (Resolve conflict as a sheet), 2026-11-15. Each task: a fresh implementer, then a fresh Opus reviewer; both approved with no Critical or Important findings. Detail is in the execution ledger `docs/plans/2026-11-15-wave-04-cycle-05b-close-and-resolve-execution.md`.

**Commits.**
- Task 5 `c22ae1ae` Resolve conflict as a sheet the workspace keeps (`SheetOwner::Resolve`, `show_resolve`, every way in, `conflict::open` as a Kit sheet; Ruling O-8 pulled B3's footer-press rule forward).
- Task 6 `7b2b4511` the focus table in tests (P6 for H1 to H16, P7), red, then `2651c63b` Resolve conflict's closes, focus and refusals (`close_resolve`, `resolve_waiting`, `keep_focus_in_resolve`, the refusals in Ctrl+, / `show_settings` / `show_add_repository`, the republish waits, `focus_content` and `apply_review` gating, and `unkept_asks_first` closing the sheet).

**Design checkpoint (Task 6), owner decisions of 2026-11-15** (see the earlier comment): H13 reworded to the Result editor's own right-click menu (Ruling O-9); Edits not kept now closes the sheet at the ask (Ruling O-10).

**Tests.** Implementers ran exact names only (Task 6: P6 and P7 red first, 12 of 54 and 7 of 21 facts, then green; every row that passed at write time has a mutation proof). Orchestrator module runs: Task 5, 17 filters, 262 passed; Task 6, 10 filters (`desktop::model::conflict`, `view::conflict`, `view::keyboard_tests`, `view::workspace`, `view::activity::tests`, `view::needs_attention::tests`, `view::lifecycle`, `view::operations`, `view::item::{share, posting}`), **500 passed, 0 failed**, 637 s. Static gates were clean on each task. The whole fast tier runs once at review-ready.

**One test edit after the red commit (E-23).** P6's H12 "no dialog reopens" fact now compares an app-wide lifecycle-dialog entity before and after the answer instead of reading it once; the reviewer judged it no weaker.

**Deferred minor findings** (in the ledger): one waiting-close slot can be overwritten by a stale `applied` answer (fix in Task 9 or the final review); a fourth copy of the Root-to-Workspace lookup; H1's Shift+Tab has only a weak mutation proof; a 0.25 s moment of no focus after a Confirm closes over the sheet.

Tasks 7 to 9 follow (header, cards, footer; keyboard journeys; review-ready).