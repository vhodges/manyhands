---
id: 01M5AT1YVE3NRNNVEVDHGDEKE5
item_id: 01M4CC0VMY5RBMP70QVBXWXM6A
created_at: 2026-10-19T19:26:15Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Task 1 checkpoint (commit `e4809a7`, base `02ab2d3`). Feasibility passed,
so the plan's stop does not apply:

- A `#[gpui_kit::test]` opens the main window, with `Root` as its first
  entity, on GPUI's test platform. It passes with `DISPLAY` and
  `WAYLAND_DISPLAY` unset
  (`desktop::mod_tests::view::root_is_the_first_entity_of_the_main_window`).
- A real `IndexRefresh` run on a worker thread delivers its `Finished`
  event to a model under `allow_parking`
  (`desktop::mod_tests::view::a_worker_refresh_reaches_a_model`).
- Also tested: the dialog layer renders, and startup failure shows Retry and
  recovers. `scripts/check-cli-no-gpui.sh` passes, exits 1 when run with
  `--features desktop`, and stops on a bad flag.

Changes to `Cargo.lock`:

- **Existing packages:** no version changed. `manyhands` gains a direct
  `futures` 0.3.34 edge.
- **New packages:** `gpui-kit/test-support`, through
  `desktop-test-support`, adds proptest 1.12.0 and the 10 packages it
  depends on: proptest-macro, bit-set 0.11, bit-vec 0.10, borsh-derive,
  convert_case 0.11, quick-error 1.2, rand_xorshift, rusty-fork, unarray and
  wait-timeout. The no-feature build and the `desktop` build cannot reach
  any of them.
- **Decision:** the plan anticipated only a direct edge. The controller
  accepted these packages (ruling R-T1a), because they are what the approved
  test-support feature costs. The owner can reverse this.

The Task 1 review is in progress.
