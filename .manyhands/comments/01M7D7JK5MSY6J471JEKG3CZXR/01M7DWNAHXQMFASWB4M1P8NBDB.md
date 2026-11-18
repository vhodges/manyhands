---
id: 01M7DWNAHXQMFASWB4M1P8NBDB
item_id: 01M7D7JK5MSY6J471JEKG3CZXR
created_at: 2026-11-14T20:47:40.766143048Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Note for planning Cycle 05b, from Wave 04 Cycle 05a (ticket `01M7D5Z7FZTGTC1HE25KQ1MYQY`), approved 2026-11-14 and not yet implemented.

These parts of 05a reach this ticket's two surfaces:

- **The theme (05a decision 6).** Primary buttons turn amber everywhere through the Kit theme, the lifecycle dialog's and the Resolve conflict sheet's included, as soon as 05a lands. The focus ring and input border turn teal, and sheets and dialogs share one scrim colour.
- **The common frame (05a design Part A).** `src/desktop/view/frame.rs` gives the header, body and footer that 05b's surfaces should build on. A dialog's footer is drawn in its own body, never in the Kit's `footer` slot (05a R-19).
- **Button order (05a decision 15).** Footers keep the order they had before, except the Confirmation dialog and Unlock key. `05` draws the Close ticket dialog as Cancel · **Close ticket**; 05b decides its own order.
- **Escape (05a decision 7).** Each dialog keeps its Escape rule from before. `05`'s running stage, where Escape hides the dialog and the work continues, is 05b's to plan.

05a's documents are on its branch: `docs/Cycles/wave-04-cycle-05a-frame-add-repository-and-unlock.md` and its design. Plan 05b against 05a's merged code.
