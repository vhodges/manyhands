---
id: 01M70VNRH4T8YCK3QED3SZ8RWF
item_id: 01M70PYS3B3YWEFTY0QBJGCF6N
created_at: 2026-11-09T19:13:41.979777994Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Verification on `50fe563e` (2026-11-09). Review-ready.

Everything ran through devenv with `--locked`, on a machine with no global Git identity.

**Static gates, whole tree:** `cargo check --all-features`, `cargo fmt --check`, and `cargo clippy --all-targets --all-features -- -D warnings` are all clean.

**Module:** `--lib -- repository::context_retirement::` gives 142 passed, 0 failed (189 s).

**Fast tier, once, as in AGENTS.md plus `--no-fail-fast`:** 30 targets, 5664 passed, 4 failed, 58 min. All 27 integration targets and the CLI bin passed. No SSH tier applies, because `src/repository/context_retirement/` is in no SSH row.

| Target | Result |
| --- | --- |
| `--lib` | 1563 passed, 1 failed (1052 s) |
| `--bin manyhands` (desktop) | 1579 passed, 3 failed (1093 s) |
| `--bin manyhands-cli` | 317 passed |
| integration targets | all ok (`cli` 477, `mutation_replay` 384, `mutation_confirmation` 184, …) |

None of the four failures touch this change. Each fails only under the full parallel load and passes alone:

1. `repository::closure::checkpoint::tests::remote_primary_advance_from_context_synchronized_is_primary_advanced` (`checkpoint_tests.rs:1413`). It passed 8 out of 8 runs alone (0.9 s each). Its two `ClosureRecord`s print identically but compare unequal. The custom `Debug` hides `updated_at`, which every update sets to `max(updated_at, now())` in whole seconds, so a second rollover under load is the likely difference. That is not verified. This is the same signature as open ticket 01M6K07QZDEMXY1GTF6CC739Y6 (a sibling test in the same module), so it probably belongs there. The 93 tests that now run instead of failing fast add load to the lib target, which may make this flake more likely.
2. `desktop::services::submissions::tests::a_second_submit_while_posting_is_refused`: "no end in 10s" (`submissions_tests.rs:100`, a 10 s wall-clock `WAIT`). Passed 3 out of 3 alone.
3. `desktop::services::submissions::tests::retry_keeps_the_request_id`: same cause and site. Passed 3 out of 3 alone.
4. `desktop::view::keyboard_tests::editing::edit_the_mode_key_save_and_done_by_keyboard_alone`: "the edit editor opened did not happen" (`harness.rs:315`, a real-time wait). Passed 3 out of 3 alone. The Wave 04 Cycle 01 branch raises this wait in `42440a0d`, and main does not have that yet.

Main's fast tier never reached the desktop bin before, because `--lib` failed first. So items 2 to 4 have probably been hidden on main, not introduced. `src/desktop/` is out of scope here and was not edited.

**Merge check:** origin/main is still `f4442618`. It is an ancestor of HEAD, and `git merge-tree --write-tree HEAD origin/main` is clean.

**Not done, for the owner:** review and close of this ticket; a note on 01M6K07QZDEMXY1GTF6CC739Y6 (a comment there would push its branch); a ticket for the desktop wall-clock waits; and whether AGENTS.md's fast-tier command should gain `--no-fail-fast`.
