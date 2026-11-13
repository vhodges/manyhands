---
id: 01M7ACPS9WM00S0GB1CBCRS749
item_id: 01M5CS4MH25QQ769HV61ZRKHXG
created_at: 2026-11-13T12:04:22.781380884Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Two items are done by Wave 04 Cycle 03a (`01M784Y8C73B39GR4JWENXE5D0`, vh-3jpbf), merged to main on 2026-11-13:

- **The repository row exposes two `aria_selected` signals.** The row sets no `aria_selected`; one meaning of selected is pinned by `a_repository_rows_selected_state_has_one_meaning`.
- **`RepositoriesPane::set` notifies during render.** Rows are now applied after the frame (`window.defer`), checked by `rows_and_list_states`.

Every other item stays open here, including the `local_time` hardening, "Finishing" hiding the running stage (Cycle 03b takes that and the operations split), the item-view guards, and the list, tab and find items. Code locations refer to the D1 branch as reviewed at `ba89ea0` and may have moved. Left open at the owner's word.