---
id: 01M7GWDR1WNNNTRTCMSV58GB51
item_id: 01M7D7JK5MSY6J471JEKG3CZXR
created_at: 2026-11-16T00:34:34.967867421Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 6 design checkpoint, 2026-11-15. Revision 4 of the design had no independent review, so Task 6's implementer wrote the focus table tests first (P6: design B3, H1 to H16; P7: nothing opens over the sheet), tests only, and saw each row fail or proved it can fail.

**Commit** `7b2b4511` test(desktop): Resolve conflict's focus table, red (on top of Task 5's `c22ae1ae` and its ledger commit). No production code. Boundary diff empty.

**Result: two rows differed from the design; the owner ruled on both (2026-11-15) and approved both recommendations.**
- **H13** (Root's context menu over the Result editor): there is no Root menu. The Result editor's right click opens the editor's own menu; focus stays in the editor and Escape goes to the Result stop, as H3 says. Ruling O-9: H13 now reads "the editor's own right-click menu: focus stays in the editor; Escape goes to the Result stop". The row passes now.
- **H11, Edits not kept**: `unkept_asks_first` closed the old dialog and still closes Settings, but left the Resolve sheet open under the ask. Ruling O-10: Task 6 adds a line so it closes Resolve too (Cancel then loses the choices, as the dialog did). The fact stays red until Phase B.

**Rows.** All 16 H rows and all 11 P7 sub-cases are written. P6: 12 contract facts red now, all waiting on Phase B code (H10, H11 at the ask, H12, H14, H15); 29 pass now, each failing under a mutation (18 mutations, all reverted; H1's Shift+Tab is the Kit's draw order and has only a weak proof). P7: 7 facts red (Ctrl+, , `show_settings`, `show_add_repository`, both Republish paths), 10 pass with mutation proofs. O-8: H8's footer press fails without `on_footer`'s focus; H8's "goes by itself" row is answered by Task 5's `footer_focus`.

**Tests run.** Exact names only: P6 and P7, plus 18 mutation batches; fmt and clippy clean. Not pushed by the implementer; this comment publishes the branch.

Phase B (the closing guards, `close_resolve`, `resolve_waiting`, `keep_focus_in_resolve`, the refusals, the R-18 waits) follows.