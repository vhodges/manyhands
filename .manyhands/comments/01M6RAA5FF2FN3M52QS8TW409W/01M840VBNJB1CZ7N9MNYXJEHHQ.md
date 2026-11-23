---
id: 01M840VBNJB1CZ7N9MNYXJEHHQ
item_id: 01M6RAA5FF2FN3M52QS8TW409W
created_at: 2026-11-23T11:11:16.888131055Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Two more items from vh-f3wts's final review (2026-11-23).

A real wait to remove: src/repository/index_step_tests.rs an_index_step_that_cannot_take_the_lease_leaves_the_write_owing_its_hand_off waits out the real 250 ms LEASE_TIMEOUT. LEASE_TIMEOUT is a const (src/repository/coordination.rs:14, deadline at :313), so make it injectable and drive the test with a tiny timeout. The same test does not isolate the fallback mark that the timeout leaves; pin it.

Flakes seen under load during vh-f3wts's tiers, each passing alone: desktop::view::keyboard_tests::editing::edit_the_mode_key_save_and_done_by_keyboard_alone (failed once in the fully parallel desktop run, then passed 3 of 3 alone at 7.6-7.8 s each), and desktop::services::scheduler::attention_answer_tests::check_now_failure_becomes_attention (World::settle is timing-based). closure::checkpoint::tests::changed_identity_at_the_hook_is_external_change is vh-7ezxq.