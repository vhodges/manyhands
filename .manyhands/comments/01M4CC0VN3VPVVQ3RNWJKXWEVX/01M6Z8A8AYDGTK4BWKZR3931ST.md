---
id: 01M6Z8A8AYDGTK4BWKZR3931ST
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-09T04:16:12.187701828Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Coordination note from Wave 04 Cycle 01 (Workspace Shell, ticket 01M6V8RK8B0SBCH0YZXFVSR9E3), 2026-11-08, added at Task 0.

**Needs attention.** Cycle 01 adds one read-only function, `needs_attention(&Operations, &DraftRegistry) -> usize`, in the new `src/desktop/model/attention.rs`. It counts finished requests that are partial, blocked or errored, draft flush failures, drafts unavailable, pending rows, and failed pending reads. `model/operations.rs` has no attention entries and Cycle 01 does not edit it. Whichever of D6 and Cycle 01 merges second adds D6's states (key locked, host not approved, key missing, configuration required, recovery hold) to that function, so the footer count includes them.

**Operation area.** Cycle 01 wraps the operation area as it is in a dockable Activity panel. It does not edit `src/desktop/view/operations.rs`, `src/desktop/view/operations/` or `src/desktop/model/operations.rs`.

**Quit.** A quit that has to wait changes no layout. "Finishing {label}", Quit now and any "still stopping" line stay in the Activity panel and are visible when Activity is shown. Quit now and Retry take focus only when Activity is displayed (`wait_to_quit` and `not_saved_chosen` in `view/workspace/lifecycle.rs`).

**Expect textual conflicts** in `view/workspace/repositories.rs`, the `Workspace` struct and constructor, the shared test harness (`view/harness.rs` window size and the new `Shell::use_list`), and the keyboard tests (`Region::Navigation` becomes `Region::Tickets` or `Region::Documents`; F6 order changes). The second to merge merges main and adapts.

If D6 reaches main during Cycle 01, Cycle 01 stops at that merge for a short design checkpoint with the owner.