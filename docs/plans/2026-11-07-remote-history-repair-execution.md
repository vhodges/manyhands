# Remote History Repair Execution Ledger

Plan: [implementation](2026-11-07-remote-history-repair-implementation.md).
Ticket: `01M66HFV44YXNQ7ANJVA16TQ1H` (vh-hyksw). Approved by the owner on
2026-11-07, subagent-driven development. Implementers and reviewers are
Sonnet 5.5; the orchestrator writes this ledger and the ticket comments, and
pushes.

## Preflight 2026-11-07

- origin/main `d70142b6`, already merged into the ticket branch (no-op).
- HEAD before Task 0: `db7d1ae2` and the approval commit that follows it.

## Tasks

| Task | State | Base | Head | Review | Notes |
| --- | --- | --- | --- | --- | --- |
| 0 | done | 74bc1f94 | 74bc1f94 | n/a (no code) | baseline green; all four test-only flag resets dead |
| 1 | done | ea29a9ac | 9539e43c | approved | `ConfigurationChange`, known identity; interim weakening of `replacing_configuration_does_not_turn_known_publication_into_first_publication` (Task 2 restores it); review should-fix 1 and 2 carried to Task 2 |

## Test Filters (Task 0)

The plan's filters `remote::state_tests`, `remote::refs_tests` and
`remote::poll_tests::…` match no tests. Use these module paths with `--lib`:

- `repository::remote::state::persistence_tests` (the file `state_tests.rs`)
- `repository::remote::refs::tests`
- `repository::remote::poll::tests`
- `repository::remote::tests`, `repository::remote::sync::tests`,
  `repository::remote::reservation::tests`, `repository::remote::merge::tests`
- `repository::closure::observe::tests`, `repository::closure::run_tests`
- `repository::promotion::observe::tests`,
  `repository::promotion::integrate::tests`,
  `repository::promotion::run_tests` (about 105 s)

The `remote_polling` case Task 11 edits is registered as
`poll_history_unknown_after_registry_loss`, an SSH-harness target. Run it only
by exact name.

## Task 0 Results

All commands passed:

- `cargo check --all-features --locked`.
- `--test remote_polling poll_history_unknown_after_registry_loss -- --exact`:
  1 passed.
- `--lib repository::remote::poll::tests::task6_config_and_selected_key_source_after_fetch_fence_before_batch -- --exact`:
  1 passed.
- `--lib repository::remote::state::persistence_tests`: 68 passed.
- `--lib repository::remote::refs::tests`: 10 passed.
- **Dead resets.** Each `history_unknown=0` reset was temporarily replaced by
  SQL that raises an integer-overflow error if the flag is nonzero, then
  reverted. No module run hit it:
  - `closure/test_support.rs:203`: `closure::observe::tests` (61) and
    `closure::run_tests` (46);
  - `promotion/run_tests.rs:256`: 82 tests;
  - `promotion/integrate_tests.rs:167`: 31 tests;
  - `promotion/observe_tests.rs:375`: 53 tests.

  All four are dead. Task 11 removes them.

## Task 1 Results

- Implementer:
  - `cargo check --tests`, `fmt --check` and `clippy -D warnings` all
    clean;
  - the new `configure_reasons_fence_and_forget` passed;
  - `remote::state`, `refs`, `poll`, `cleanup`, `closure::record`: 204
    passed, after one named edit;
  - `remote::reservation`, `promotion::record`, `local_sync::admission`,
    `local_sync::review`, `closure::observe`, `remote::observation`: 161
    passed.
- Reviewer: `remote::sync::tests` 182 passed, and
  `remote::state::persistence_tests` 69 passed. Approved.
- Decision: the observation recheck with no current plan maps to
  `Unavailable` (the implementer's addition; the reviewer agreed).
- Carried to Task 2:
  - expose the forget decision and assert it in the table (review
    should-fix 1);
  - add a row for `Unavailable` followed by the same endpoint digest
    (should-fix 2).
- Nits left as they are: the known-identity UPDATE is separate, and the
  `Unavailable` doc comment.
