---
id: 01M713DRATWPQS3RBP8TCQAY85
item_id: 01M6V8RK8B0SBCH0YZXFVSR9E3
created_at: 2026-11-09T21:28:59.505348609Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Owner's code review: ten findings checked and fixed (2026-11-09). Ledger: section "Owner's code review: ten findings". Commits dae17983 and 0a82eae4.

- Real and fixed: Shift+F6 from outside a region now goes to the last stop (3); move_panel doc (6); the appearance tests I had added are removed (7); the dock minimum comes from the Kit's PANEL_MIN_SIZE (8); the footer counts attention once (9); the launch clamp is queued once (10).
- Ctrl B (1, 4): in a read-only editor it now toggles the left dock (the owner approved this reading of decision 2); editable editors keep Bold. The reviewer's suggested fix (a workspace context) would not have worked, since a shallower context loses to the editor's binding.
- Two title bars after Retry (2): the startup failure window now takes the main window options and draws the same Kit title bar (owner chose option A).
- Not reachable (5): the Kit refuses to close only a non-collapsible dock, and every dock here is collapsible. Comment added, no behaviour change.
- Tests written red first: tabs::tests::ctrl_b_in_the_read_view_toggles_the_left_dock, keyboard_tests::shell::f6_from_outside_a_region_goes_to_the_first_stop_and_shift_f6_to_the_last, and the startup-failure test extended. About twenty related tests were run by exact name, all green. No whole-suite or fast-tier run since. fmt, check and both clippy lines pass. Both Boundary diffs print nothing.

Not closed, merged, cleaned up, or sent to CI; none of that is authorized.