---
id: 01M7D6V5B8M8DW7JXXXAF7T8R1
item_id: 01M7AMAPNHHTNECXBF4CA5WPKV
created_at: 2026-11-14T14:19:38.618692344Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Task 0 complete (baseline, references, ledger), 2026-11-14.

**Precondition.** Cycle 03b is on origin/main: `ea6d899b` "Integrate closed ticket 01M784Y94Q3AXT6XJX38F5VK1V".

**Commits** on `manyhands/ticket/01M7AMAPNHHTNECXBF4CA5WPKV` (pushed, HEAD `003eb518` equals origin):
- `82df6942` merge of origin/main (`de9a2095` + `ea6d899b`). One docs conflict, `docs/design/08-gpui-kit-mapping.md`: kept 03b's Needs attention row and Cycle 04's Settings layout row.
- `9f89a3bb` design citations re-mapped to 03b's merged code: 86 citation/fact edits, no ruling, contract, A6 row or decision changed.
- `003eb518` execution ledger `docs/plans/2026-11-13-wave-04-cycle-04-settings-sheet-execution.md`.

**Tests run.** None (Task 0 runs the static gates only). At `82df6942`: `cargo check --all-features --locked`, `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all exit 0. `Cargo.lock` is unchanged against `ba95e1b7` (gpui-kit, gpui-base, gpui-component 0.6.6). The boundary diff prints nothing.

**U-5 answered: 03b's merged code carries what the design needs.**
- `choose_activity` (`D/view/activity/actions.rs:32-49`) holds `origin: FocusHandle` and runs `attend`, `row_action`, `run_share_offer` -> `open_route` and `run_recovery` synchronously inside `run_activity_action` (`:53-140`), so a `settings_origin` set around it is feasible. Deferred routes (`item_route`, dialog routes) already have the right focused handle.
- The popover closes silently at the next render when a sheet is open (`needs_attention.rs:125-133`); `keep_focus_in_footer` and `focus_lost`'s popover rule both sit behind the sheet/modal checks (`footer_status.rs:190`, `focus.rs:287`, `:301`).

**One 03b trigger Part D misses: W-36.** The Activity "Share main" row (`share_main` -> `open_remotes`, `requests.rs:340-341`): route `Remotes(root)`, focus `remote-add-name`, origin the Activity list.

**Further test edits (decision 7), candidates:** E-19 (a definite break: `needs_attention_tests/actions.rs` `holds`, E-12's kind, Task 1), E-20 (seam-driven, probably passes, Task 1), E-21 (E-18-style assertion adds on the 03b route tests, Task 6), E-22 (`PINNED_KEY` literal, Task 7), E-23 (a check in Task 4). Assigned in the ledger; each is recorded as an edit only when made.

**Rulings** (ledger): 1 the trailer is this session's (Sonnet 5.5); 2 A6's read-after-a-draw binds over the plan's same-update line; 3 Task 2 also writes F1/F2 (the sheet-keeps-focus rule is built there, P4 only in Task 6); 4 the interim Sharing & keys arrangement for Tasks 4-5; 5 Task 0's split; 6 module runs follow the handoff; 7 W-36 and E-19 onward live in the ledger; 8 the Activity ⋯ menu and the dock menu are F8 layers and Task 2 adds the accessors they lack; 9 `settings/attention.rs` to Task 6 and `confirm.rs` `PINNED_KEY` to Task 7; 10 Activity row titles in `services/requests.rs` keep the old words, listed for the owner; 11 R-3 keeps "21 test files".

**Branches published:** this ticket's branch only.

**Next:** Task 1 (the swap). No stop condition hit.