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
| 2 | done | 27ae78fd | 875590f8 | approved | carry-forward; evidence held while the plan is unavailable (implementer's decision, reviewed); review should-fix 1 and 2 carried to Task 3 |
| 3 | done | 54257d6b | 354c9599 | approved | per-context unknown rows; known-column fallback; held rows validated; review should-fix 1 carried to Task 5 |
| 4 | done | 03c6574a | bf7b7d6f | approved | `unpublished` disposition; table rebuild under marker `remote_poll_dispositions_unpublished_v1` |

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

## Task 2 Results

- Implementer:
  - `remote::state` 70, `remote::poll::tests` 79, `remote::sync::tests` 182
    and `remote::reservation::tests` 58, all passed;
  - clippy, fmt and `check --tests` clean.
- Reviewer: `remote::state::` 70 and `remote::tests::` 15 passed. Approved.
- **Decisions, accepted by the orchestrator after review:**
  1. While `policy.plan` is None, a current batch at the current
     generation is held: validated but not exposed. Context rows are
     skipped until a plan returns. The reviewer found no path by which held
     evidence authorizes an effect.
  2. A batch is carried only to a plan with the same remote name and
     primary branch.
- The restored assertion
  (`replacing_configuration_does_not_turn_known_publication_into_first_publication`)
  expects `ObservedPublished`. The reviewer confirmed it: the test is a
  same-identity credentials fence, not an identity change.
- Carried to Task 3:
  - **should-fix 1:** with NULL `known_*` columns (a registry from before
    Task 1) and no current batch, a change of plan could carry rows to a
    different remote. When the known columns are NULL, `forgets()` falls
    back to the policy's last non-NULL remote name and branch, and the
    table gets a row for this case;
  - **should-fix 2:** held context rows are still parsed and their enums
    checked during the startup audit. Only the plan-dependent comparisons
    are skipped.
- For final review (outside the plan's boundary): `read/status.rs:730`
  shows the held batch's `observed_at` while the plan is unavailable.

## Task 3 Results

- Implementer, all passed:
  - `remote::state::` 73, `refs::tests` 10, `poll::tests` 79, `sync::tests`
    182;
  - `cleanup` 27, `closure::observe::tests` 61, `promotion::observe::tests`
    53;
  - clippy, fmt and `check --tests` clean.
- Reviewer: `remote::state::` 73 and `remote::tests::` 15 passed. Approved.
- No existing R1 or invariant test needed an edit.
- Decision: an active context whose stored branch can't be parsed aborts a
  forgetting fence with `recovery_required`. The reviewer confirmed that the
  read at the start of `configure_known` already fails in that case, and
  that a credentials fence can't reach the insert loop.
- Carried to Task 5:
  - **should-fix 1:** NULL known columns, plan already None, rows kept, then
    a different remote. Evidence can't apply to the wrong remote (the
    tracking ref includes the remote name), but every read is then
    `recovery_required`. Fix: a carry also requires each kept row's
    tracking ref to equal the new plan's, and otherwise takes the forget
    path. Add test rows for NULL known + NULL plan + a different remote,
    and NULL known + the same remote (rows kept).
- Nits left: the `plan.ok_or_else` that can't be reached needs a comment;
  the real clock is consistent with the file.

## Task 4 Results

- Implementer: `remote::poll::tests` 80, `remote::state::` 74 and
  `--bins poll` 19, all passed; clippy, fmt and `check --tests` clean.
- Reviewer: `poll::tests` 80, `state::` 74 and `--test read_contract` 72,
  all passed. Approved, with no findings above a nit.
- Edits outside the plan's boundary, both needed and approved:
  - `schemas/v1/poll_mutation.schema.json` gains `unpublished`
    (`read_contract` checks it against the DTO);
  - `tests/remote_polling.rs` gains a match arm, so it compiles.
- The absent primary stays `history_unknown`. Nothing that consumes the
  disposition treats `unpublished` differently.
- For Task 11: `tests/remote_polling.rs` (around line 1613) and possibly
  `tests/cli_remote/polling.rs` may now see `unpublished` for a context
  that was never published.
