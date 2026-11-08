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
| 5 | done | 21ac5032 | cf100915 | approved | migration `remote_history_per_context_v1`; carry requires kept rows' refs to match the new plan (Task 3 should-fix) |
| 6 | done | ef45f239 | 946623a5 | approved on re-review | `remote_history` over the full manual poll; `RemoteHistoryError`; caller-supplied operation, request and token |
| 7 | done | 8fe524c6 | da7c8118 | approved on re-review | `item resolve-history`, plus the repository-level `remote resolve-history`; `mutation_contract` red only on missing goldens (Task 9) |
| 8 | done | 651774aa | f457e894 | approved on re-review | terminal `HistoryUnresolved`; `remote.history` recovery; `ResolveHistory` way out; refusal-blocker CHECK bounds widened, with rebuilds |
| 9 | done | 68a0b743 | 45d5acda | approved | CLI `remote history`, `item resolve-history`, `remote resolve-history`; goldens and schemas; one SSH case `cli_remote remote_history_repair_journey` |
| 10 | done | 59561bc0 | 56aba82d | approved | desktop guidance for unresolved history; Share failure `HistoryUnresolved` with no Retry |
| 11 | done | 2a025654 | 7f6717cb | approved | named test edits (SSH ones by reading); dead resets removed |
| 12 | done | 7f6717cb | cd7ed4e2 | approved | RFC and Wave 02 C4 amendments; CLI `choices=publish (release: desktop only)` |
| 13 | done | 22f72aab | 9efffcda | final review READY; gates green | round 1 found 18 failures (3 code bugs); round 2 green apart from one marker-list expectation, fixed |

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

## Task 5 Results And Part 1 Checkpoint (Tasks 1–5)

- Implementer, at the checkpoint, all passed:
  - clippy `-D warnings`, `fmt --check` and `check --tests` clean;
  - `remote::state::` 76, `poll::tests` 80, `refs::tests` 10, `sync::tests`
    182, `remote::tests::` 15.
- Reviewer: `remote::state::` 76 and `remote::tests::` 15 passed. Approved.
- With the carry guard temporarily disabled,
  `null_known_and_no_plan_rows_follow_the_new_plan` fails (it then reverts).
- Decisions:
  - a flag set to 1 with no remote configured is left set, and the marker
    is still recorded. This is fail-closed, behaves like cache loss, and
    Task 7's repair clears it once nothing reads unknown;
  - a stored plan that can't be parsed gives `recovery_required`.
- Nits left:
  - a doc comment line about the marker being recorded when the marker file
    is present;
  - no test row for flag 1 with no remote.

## Task 6 Results

- Commits: `e278ea76` (first cut), `439ab42d` (moved to the poll path) and
  `946623a5` (review fixes).
- Tests, all passed: `remote::history::tests` 2 (an 8-row table, plus an
  outcome table), `poll::tests` 80, `remote::tests::` 15; clippy, fmt and
  `check --tests` clean.
- **Orchestrator decision:** `remote_history` runs the full manual poll
  (`poll_publication_remote_observed`), not the observation alone. The
  observation reads only the advertisement and fetches no objects, so it
  can't show divergence when the remote holds commits we don't have. The
  design already names "the same observation path as `poll once`". The
  poll's ordinary side effects (fast-forward, materialization) are accepted.
  Each entry carries the poll's disposition.
- **Review finding (blocking, fixed):** a poll stopped by unlock, host
  approval, the key, the configuration or the transport reports
  `Completed`, but its outcome category isn't `Completed`. The fix is
  `RemoteHistoryError::{Poll, Stopped(category), Cancelled, Interrupted,
  Internal}`, with success only for `Completed` plus `Some(Completed)`, as
  in `bind/poll.rs`.
- Signature:
  `remote_history(root, operation_id, RequestId, CancellationToken, session)`.
- The resolved set is a before/after diff of fetch evidence, not captured
  inside the observation transaction (accepted, and documented in the doc
  comment).
- The accessor `RemoteSnapshot::history_unknown()` in `state.rs` is
  accepted.
- **Notes for Task 9:**
  - pass a fresh operation id unless resuming, since a replayed terminal
    result fetches nothing and `resolved` comes back empty;
  - clear the cancellation flag after `Cancelled`, as the poll binding does
    with `take_cancellation`.

## Plan Deviations

- **Transport is SSH-only**: `file://` endpoints are rejected. The plan's
  two CLI journeys over a `file://` remote (Task 9) become **one SSH case in
  `cli_remote`**, covering both journeys end to end. That is the single
  end-to-end SSH case AGENTS.md allows for a new feature. Why it needs SSH:
  no other transport exists, and `remote history` and `item sync` can't run
  without one. Every edge case stays in unit tests.
- Test filters: see "Test Filters (Task 0)".

## Task 7 Results

- Commits: `cea94440` (the verb) and `da7c8118` (review fixes).
- Tests:
  - `remote::resolve_tests` 4 and `mutation` 153 (bind 35) passed;
    `--test read_contract` 72 passed;
  - `--test mutation_contract`: 43 passed, 1 failed. The failure is
    `every_bound_command_has_a_golden`, for `item resolve-history` and
    `remote resolve-history`. It is **expected red until Task 9** adds the
    goldens;
  - clippy, fmt and `check --tests` clean.
- **Decisions:**
  - **Snapshot rule.** `read_snapshot_rows` accepts an explicit
    `never_published` row while the cache-loss flag is set. The reviewer
    confirmed the only writer is `record_context_resolved`, the owner's
    confirmed resolution. Contexts with no row stay unknown under the flag.
    Named edits: `evidence_defaults_by_flag_and_rows`, and
    `retirement_does_not_weaken_unknown_never_published_row_invariant`,
    renamed to
    `an_explicit_never_published_row_is_accepted_under_the_cache_loss_flag`.
  - **No remote reservation.** The Immediate registry transaction is the
    fence. The reviewer found no in-flight sync, poll or promotion it could
    affect.
  - **Way-outs.** The new `HistoryWayOut` (`ReleaseContext`, `Republish`,
    `Sync`, `RemoteHistory`) keys on result code and choice, so
    `RecoveryActionKind` is untouched.
  - **`--resolve-all` became a verb.** Decision 7's `--resolve-all` is the
    separate confirmable verb `remote resolve-history`
    (`Command::RemoteResolveHistory`). It clears the cache-loss flag when
    nothing reads unknown, then retires the marker file. A confirmable
    change doesn't fit on a read.
  - **Marker deletion.** It happens only when this call cleared the flag. A
    failure is reported (`HistoryMarkerState::Failed`) and doesn't undo the
    commit. A valid marker that can't be removed leaves the flag at 0 on
    the next open (the reviewer traced it).
- Edits outside the boundary, all forced by exhaustive matches:
  `human_admin.rs` and desktop `view/confirm.rs` (one arm each),
  `tests/support/mutation.rs` and `tests/support/repair.rs`, and
  `schemas/v1/preview_effect.schema.json` (`history_resolved`,
  `history_flag_cleared`).
- For Task 9 or 12: the goldens and the schemas for the two verbs' preview
  and result DTOs.
- Nits left:
  - the "undeletable marker" test builds an *invalid* marker (a directory),
    so it should be renamed;
  - `target()` returns an empty string for the repository-wide verb.

## Task 8 Results

- Commits: `3fa4750a` (the change) and `f457e894` (review fixes).
- Tests, all passed:
  - `closure::`, `promotion::`, `mutation` and `remote::sync::tests`: 709;
  - `--test read_contract` 72;
  - `--test mutation_contract`: only the known missing golden fails;
  - clippy, fmt and `check --tests` clean.
- **What changed:**
  - **Sync.** Evidence-based unknown is the terminal
    `SynchronizationError::HistoryUnresolved`. `ended_if_unresolved` cancels
    and acknowledges, as the `ExternalChange` and `PushRejected` arms do,
    which ends the record and releases the reservation. It is skipped when a
    push was intended, or for a lifecycle child owner. The answer is
    `recovery_required` with recovery `[remote.history]` and no
    `request.retry`. Push-proof unknown keeps `HistoryUnknown`: resumable,
    with `poll once`.
  - **Recovery kind.** New `RecoveryActionKind::RemoteHistory`
    (`remote.history`, argument `root`), added to the schema.
  - **Lifecycle.** New `PromotionBlocker`, `ClosureBlocker` and
    `LifecycleBlocker::HistoryUnresolved`, appended last, plus
    `LifecycleWayOut::ResolveHistory`. `ObservedPublished` with no deletion
    seen stays `HistoryUnknown` and `PollOnce`.
- **Review finding (blocking, fixed).** The stored refusal masks' CHECK
  bounds rejected the new bits. They are widened to 131071 (promotion) and
  4194303 (closure), with marker-gated table rebuilds
  `promotion_refusal_blockers_v2` and `closure_refusal_blockers_v2`, each
  with its own rebuild test.
- **Named edits:**
  - closure `blocker_mask_round_trips_all_21` became `…_all_22`;
  - new promotion `…_all_17`;
  - promotion `schema_rejects_…` now uses 131072;
  - closure `history_unknown` and promotion `remote_history_unknown` now
    expect `HistoryUnresolved`;
  - closure `every_blocker_has_a_distinct_fixed_position`.
  - The plan's `c5_lifecycle.rs:3235` doesn't exist.
- **Known limit (accepted).** A replayed final result recomputes recovery
  from the code and shows `repo.inspect`, not `remote.history`. Fixing it
  would need a new stored field.
- **Placeholders for Task 9 (CLI) and Task 10 (desktop):**
  - `human_lifecycle.rs` renders `ResolveHistory` and replaces the
    `UNKNOWN_NOTE` text;
  - desktop `operations.rs` maps `RemoteHistory` to Inspect;
  - lifecycle `outcome.rs` and `prepare.rs` have their own placeholders;
  - `tests/comment_publication.rs` gained two match arms.
- A record that looks Cancelled to other readers, while the request answers
  `recovery_required`, matches the existing `ExternalChange` behaviour.

## Task 9 Results And Part 2 Checkpoint (Tasks 6–9)

- Commits: `f789baea`, `0923020c`, `ef61cb85` and `45d5acda`.
- Tests, all passed:
  - `--bin manyhands-cli` 315;
  - `--test mutation_contract` 45, **green again**;
  - `--test read_contract` 73;
  - `--test cli` 474;
  - the SSH case `--test cli_remote remote_history_repair_journey -- --exact`
    (about 14 s);
  - clippy, fmt and `check --tests` clean.
- **What the SSH case covers:**
  - `key clear`, `key select`, then a new ticket's `item sync` publishes
    (the original bug);
  - lost history: `remote history` lists the unresolved context;
  - `item sync` refuses it with `remote.history` and holds no reservation;
  - `item resolve-history --as publish` (prepare and confirm), then
    `item sync` publishes;
  - a later ticket needs no repair;
  - the end state is `Contexts: none` and `Cache loss: no`.

  The loss is simulated by SQL, as in `Fixture::lose_history`. The harness
  has no second endpoint, so the identity-change fence is unit-tested in
  Tasks 1–3.
- **Decisions:**
  - `remote history` is an unjournaled `Kind::UnlockingRead`, like
    `host observe`. It takes no `--request-id` and is cancelled only by
    Ctrl-C. It runs a reserved poll, which may fast-forward and
    materialize. Its error codes match `poll once` arm for arm, and the
    reviewer checked them side by side.
  - Release is a note, not a `next:` command, because `context release` is
    desktop-only (RFC, around line 131). `--as release` answers
    `invalid_input`.
  - `UNKNOWN_NOTE` is kept for push-proof unknown, and the new
    `HISTORY_NOTE` covers `ResolveHistory`.
  - The catalog label is `Available("C5")`, the nearest existing label.
- Edits outside the CLI folder:
  - library DTOs `RemoteHistoryDto` and friends in `mutation/dto.rs`;
  - `RemoteHistoryError::result_code` and `host_authority`;
  - three `pub(crate)` widenings;
  - four schemas;
  - 8 `mutation_v1` goldens and 1 `read_v1` golden;
  - inventory and help goldens;
  - the minimal RFC catalog rows that `--test cli` requires.
- Carried to Task 12:
  - **should-fix 1:** RFC rows for the refused options of `remote history`,
    the golden and result-code tables for the new goldens, and the side
    effects and cancellation in the `remote history` amendment;
  - **should-fix 2:** the human listing renders
    `choices=publish (release: desktop only)`, while JSON keeps both. Update
    the SSH case's expected line, and record this deviation from the plan's
    example.
- Nits left:
  - `Stopped(host category)` offers no `host.approve`. This is probably
    unreachable, since host errors arrive as `Poll(Transport)`;
  - the progress line is written before `interrupt::reset()`.

## Task 10 Results

- Commits: `b00fde97`, and `56aba82d` (review fixes: the copy no longer
  promises a future sheet; the test pins the exact offers for the Item and
  Main targets).
- Tests passed, each by name:
  - `sync_unresolved_history_is_not_retried`;
  - `recovery_actions_map_to_controls`, a named edit that adds the
    `RemoteHistory` row;
  - `unresolved_history_guides_to_the_command_line`.

  clippy, fmt and check clean.
- Decisions:
  - **The Share failure.** A Share failure answered `recovery_required`
    with `remote.history` becomes `SyncOutcome::HistoryUnresolved`. It
    offers no Retry, Share or Republish; only Give up, while the record is
    accepted.
  - **Inspect stays the control.** `RemoteHistory` still maps to the Inspect
    control. That's an interim choice until the sheet (vh-ez4y9).
  - **The lifecycle text.** `HistoryUnresolved` and `ResolveHistory` share
    `HISTORY_UNRESOLVED_GUIDANCE`. `HistoryUnknown` and `PollOnce` are
    unchanged.

## Tasks 11–12 Results

- **Task 11 named edits** (the SSH-target ones were made by reading and
  first run at Task 13):
  - `tests/remote_polling.rs`: `poll_history_unknown_after_registry_loss` is
    renamed **`poll_unpublished_after_key_selection_in_new_registry`** and
    expects `Unpublished`. It never exercised cache loss; cache loss stays
    covered by `lose_registry`.
  - `tests/document_promotion.rs`: `HistoryUnresolved`. The `restart=true`
    replay is now `Interrupted`, because the ended record replays as
    cancelled.
  - `tests/document_promotion/journeys.rs`: `retained_r1`'s sync error is
    `HistoryUnresolved`.
  - `tests/ssh_transport/synchronization.rs`: `context_absence_boundaries`
    expects `HistoryUnresolved`.
  - `tests/ticket_closure/journeys.rs`: case 16 is renamed
    **`blocker_history_unresolved`**.
  - `tests/remote_observation.rs`:
    `same_name_endpoint_replacement_after_reopen_is_not_remote_deletion`
    expects `NeverPublished`, since it has no active context.
  - `tests/mutation_bridges/context.rs`
    (`release_with_an_unknown_publication_history_and_no_tracking_ref_is_ok`)
    is non-SSH and the plan missed it. It now uses a real identity change.
  - `remote/poll_tests.rs` task6: the key-source variant asserts the carry.
  - The four dead resets are removed, and the "invalid marker" label is
    fixed.
- **Confirmed unchanged:** the push-proof cases
  (`incompatible_generation_history_unknown`,
  `push_history_generation_fencing`), the cache-loss cases, and
  ObservedPublished-without-deletion cases.
- **Task 12:**
  - RFC: the known limit is marked fixed; the `history_unknown` guidance is
    split; the three verbs' rows and options; `unpublished` and
    `history_unresolved`; a "History repair goldens" paragraph. It is a
    paragraph, not table rows, because `tests/cli/recovery_tables.rs`
    fixes the families;
  - the Wave 02 C4 amendment;
  - the CLI listing wording.
- Tests, all passed:
  - `--test cli` 474, `--bin manyhands-cli` 315;
  - `read_contract` 73 and `mutation_contract` 45;
  - `closure::`, `promotion::` and the resolve modules: 375 passed, plus 1
    apparent flake, `closure::checkpoint::tests::changed_identity_at_the_hook_is_external_change`.
    It passed alone and in a module rerun.
- The reviewer approved. Nits left: the `unknown()` wording in
  `journeys.rs`, and the configuration arm of `poll_tests` task6 asserts
  nothing about batches.

## Task 13: Gate Round 1 (at `22f72aab`) And Fixes

**Round 1 results** (logs in the orchestrator's job directory):

- Green:
  - check, fmt, clippy;
  - `--lib repository::local_sync::` 160;
  - `remote_observation` 15, `remote_polling` 84, `remote_synchronization`
    69, `remote_merge_recovery` 29;
  - `cli_remote` 162, `ticket_closure` 78, `comment_publication` 41.
- Red:
  - **Fast tier: 1,404 passed, 2 failed.** Cargo stopped at the first
    failing binary, so `mutation_replay` and `mutation_bridges` never ran.
    Later rounds use `--no-fail-fast`.
  - `mutation_remote`: 694 passed, 9 failed.
  - `document_promotion`: 76 passed, 3 failed.
  - `ssh_transport`: 119 passed, 4 failed.
- A final whole-branch review (read-only) said **READY**. It had one
  should-fix (ghost unknown rows) and found every acceptance criterion
  met.

**Fixes** (`781fd112` to `61732e93`), reviewed and approved:

- Code bugs:
  - **`5784a4c5`:** the observation recheck fenced a late same-plan
    endpoint edit as `Credentials` with no digest, so nothing was forgotten.
    It now goes through `configure_endpoints` with the fresh digest, which
    forgets.
  - **`c51400c3`:** a forgetting fence wrote unknown rows for retired items.
    That broke R1 retirement in `push_absence`, and syncs answered
    `HistoryUnknown`. `insert_unknown_context_rows` now skips retired items.
    **Residual (documented):** after an identity change, a recreated
    context for a retired item reads `NeverPublished` on the new remote.
    Its first push is non-forced, so a ref already on the new remote isn't
    overwritten: the push is refused or diverges.
  - **`f7ab036d`:** the final-review should-fix. `unresolved_contexts` and
    `history_report` count only rows with an active context, so a ghost row
    no longer lists forever or blocks the flag.
- Test fixes:
  - **`b399c665`:** the conformance `HOMES` table gains the two new verbs
    (43 to 45), with driven rows in `tests/mutation_confirmation/conformance.rs`.
    This was the shared cause of the 9 `mutation_remote` failures, and also
    hit `mutation_replay` and `mutation_bridges`.
  - **`781fd112`:** `context_release` check C uses a real identity change,
    and the results registry lists `remote.history`.
  - **`483ed88a`** and **`61732e93`:** the `ssh_transport`
    `observe_selection_race`, `_lifecycle` and `_replacement` cases expect
    the carried batch, because key selection doesn't forget. The race case
    tells a carried batch apart by `observed_at` plus the generation
    advancing.
- Also: a comment on `configure_endpoints` naming, and the conformance
  mismatch row now targets an item with a context.
- Each failing case passed alone by exact name after its fix.

## Task 13: Gate Round 2 (at `fc8fbb3f`), Review-Ready

Every command ran once, through Devenv, with `--locked`:

| Command | Result |
| --- | --- |
| `cargo check --all-features` | ok |
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | ok |
| fast tier (AGENTS.md list, `--no-fail-fast`) | 5,503 passed, 1 failed (see below) |
| `--test ssh_transport` | 123 passed |
| `--test remote_observation` | 15 passed |
| `--test remote_polling` | 84 passed |
| `--test remote_synchronization` | 69 passed |
| `--test remote_merge_recovery` | 29 passed |
| `--test cli_remote` | 162 passed |
| `--test document_promotion` | 79 passed |
| `--test ticket_closure` | 78 passed |
| `--test mutation_remote` | 703 passed (about 87 min) |

- **The one failure.** `recovery_foundation_gate`
  `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly`
  pins the committed `registry_migrations` names. The four new markers
  (`closure_refusal_blockers_v2`, `promotion_refusal_blockers_v2`,
  `remote_history_per_context_v1`, `remote_poll_dispositions_unpublished_v1`)
  commit in the registry-schema transaction, before the recovery migration,
  so they are expected. This named edit then passed `--test
  recovery_foundation_gate` 52/52. Only that test file changed after
  round 2.
- **Not rerun in round 2, because nothing they cover changed since
  round 1:** `--lib repository::local_sync::` (160) and
  `--test comment_publication` (41).
- **Merge check.** `git merge-tree --write-tree HEAD origin/main` is clean
  against `d70142b6`.

**Notes for the owner:**

- **One-way schema.** This branch's binary rebuilds three registry tables
  (the poll-dispositions table and the promotion and closure refusal
  tables). Once it has opened a registry, a binary built from main will
  probably refuse that registry. So run the owner check on the real
  registry only after the merge and a rebuild of main, or on a copy.
- **Owner check, still to do** (plan Task 13): on the real registry,
  `remote history` lists nothing unresolved, and `key select` of the same
  key followed by `item sync` of a new ticket publishes.
- **Known limits:**
  - a replayed final sync result shows `repo.inspect`, not
    `remote.history`;
  - `release` is desktop-only; the sheet is vh-ez4y9;
  - `busy` doesn't name its holder (vh-vsfbp);
  - after an identity change, a retired item's recreated context relies on
    its first push being non-forced.
