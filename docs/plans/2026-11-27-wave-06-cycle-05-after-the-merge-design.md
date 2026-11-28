---
id: 01M8G1CXKNG56HZ4NFMKNAE3G7
title: 'Wave 06 Cycle 05 Design: After The Merge'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 05 Design: After The Merge

Cycle document: [Wave 06 Cycle 05](../Cycles/wave-06-cycle-05-after-the-merge.md). Ticket: vh-ssepw (`01M876851FT3NPSH9KDXKXR1AJ`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md), the Cycle document's Product-Owner Decisions, and [Wave 06 Cycle 04](../Cycles/wave-06-cycle-04-pull-request-promote.md) and [Cycle 03](../Cycles/wave-06-cycle-03-pull-request-close.md), whose content-proof rule, retirement fence, link record and `unheld` list this Cycle builds on.

Status: **draft**, revision 1, for review. Not approved.

## Intent And Approval Boundary

After a pull request merges on the forge, in any style, record when each item reached the primary, report it from the poll that brought it with the dependents it made ready, tell every context whose item is done from one in progress, and end such a context through a confirmed `context cleanup` that deletes a surviving remote branch only at the tip it observed and shows what the primary does not hold before the person confirms. Warn when the registered checkout is not on the primary. Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket branch, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `0213d32d` (main `15ea8eb5` merged into the ticket branch). Task 0 re-checks every one.
- "The local primary" is `refs/heads/<primary>`. "The tracking primary" is `refs/remotes/<publication remote>/<primary>`, as last fetched.
- "The branch" is `manyhands/ticket/<ULID>` or `manyhands/document/<ULID>`. "`L`" is its local tip. "`R`" is its remote tracking tip, when that ref exists.
- "A fast-forward" is a move of the local primary by `integration/artifacts.rs` `fast_forward`, from "`before`" to "`after`".
- "The proof" is `context_retirement/proof.rs` `preservation` (Baseline Fact 8). "Done on the primary" is, for a ticket, a closure in the local primary's copy of its file; for a document, the proof against the local primary (Cycle 04 decision 3).
- "An arrival" is an item whose done state a fast-forward brought: a ticket whose closure, or a document whose content, `after` holds and `before` did not (Part A).

## Baseline Facts

1. **Every move of the local primary ends in one function.** `integration/artifacts.rs` `fast_forward` (about 178) locks the ref, requires it at `before` and `HEAD` on it, checks the work tree out safely and sets the ref to `after`; it does not check that `after` descends from `before`. Its callers that move `refs/heads/<primary>`: the poll, `remote/poll.rs` `poll_fast_forward_unlocked` (about 616; the move about 763; `PollFastForward { target, before, after }` about 111); `remote/sync.rs` `reconcile_pending_candidate` (about 3255; a restart's move about 3574), `integrate_divergence` (about 3668; its `FastForward` stage about 3825, its `MergeRequired` stage about 3918 to a local commit "Merge remote primary") and `synchronize_remote_observed_inner` (about 6343; the move about 7205), each for a `Primary` target and each between two calls of `mark_local_update`; `promotion/integrate.rs` `apply_for` (about 452; the move about 460, `before` being `prepared.evidence.old_primary` and `after` the merge commit); and `setup/poll.rs` `activate` (about 504; the move about 536). They are reached by `poll once` and the desktop's poll; by `repo sync` in either mode (in a pull-request repository only the clean path, about 7205, and a restart); by the primary child of a direct close or promote; by a direct close's or promote's integration; and by a setup's activation. `remote/sync.rs` `apply_resolution_ref` (about 2021, `conflict resolve`) moves the primary without `fast_forward`.
2. **No move walks commits.** The poll diffs only `before`'s tree against `after`'s (`poll_fast_forward_unlocked` about 680-693, its obstruction scan). `activity.rs` `PrimaryActivityMap::advance` (about 214) walks the first-parent chain when the index observes the primary, not at a move, rebuilds from the whole chain when its base is not on it, and keeps its base on the primary `contexts` row, which an index rebuild deletes. `git2` 0.20.4's `Revwalk` has `push`, `hide`, `simplify_first_parent` and `set_sorting`; `remote/sync.rs` `commits_missing_from` (about 4163) uses one.
3. **A direct integration's commit.** `promotion/integrate.rs` `write_commit_for` (about 366) makes a two-parent commit `[base, context]` with `git2::Signature::now`, `base` being the tracking primary when a publication remote is set and otherwise the local primary; an already contained item makes no commit and `apply_for` moves nothing (`Integrated::AlreadyContained`). The runner (`promotion/run.rs` about 865-889) saves `integration_oid`, marks the primary dirty, calls `L::apply` and saves `Step::Integrated`; a resume at `Step::Integrating` (about 679-715) records `Integrated` from `reconcile_integration` without calling `apply`.
4. **The poll classifies the primary first.** `poll_after_fetch_into` (about 1885) puts the primary at ordinal 0 and processes targets in order (about 2052), so every context is classified after the primary moved. A context whose remote branch is advertised goes to `classify_local_target` (about 226), which never reads the primary and answers `current`, `fast_forwarded`, `local_ahead`, `diverged`, `dirty`, `conflicted` or `inaccessible`. Only a branch observed deleted (`PollTrackingState::RemotelyDeleted`) reaches `completed_deleted_context` (about 282): a ticket is `closed` by `proof::completion` on the local primary alone, whatever its branch holds, and a document `promoted` by `proof::preservation(L, local primary)`. Wave 03 Cycle 05's decision 28 (`materialize_context_unlocked` about 463-471, `primary_holds` about 789, ancestry only) does not materialize a remote branch whose tip the primary holds, and reports it `current`.
5. **Retirement acts on `closed` and `promoted` only.** `context_retirement/source.rs` `capture_target` (about 166-216, with the fence about 200-216), `context_retirement/handoff.rs` (about 143, 162, 455), and the desktop's `model/polling/attempt.rs` `AttemptSummary::of` (about 203-245).
6. **Where dispositions are listed.** `PollDisposition` (`remote/poll.rs` about 26) and `guidance` (about 47-84); `decode_frozen`'s list (about 1301-1317); the registry's CHECK on `remote_poll_dispositions.disposition` (`remote/state.rs` `poll_disposition_catalog!` and `POLL_DISPOSITION_SCHEMA_15`, `_16`, `_17` and the current `POLL_DISPOSITION_SCHEMA`, about 436-470; `migrate_poll_dispositions` about 571 with the marker `remote_poll_dispositions_unpublished_v1`; `disposition_name` about 791, `disposition` about 814); `mutation/bind/poll.rs` `dto_disposition` (about 289); `mutation/dto.rs` `PollTargetDisposition` (about 2741) and its `guidance`; `schemas/v1/poll_mutation.schema.json` (about 55-76) and `schemas/v1/remote_history.schema.json` (about 104-130); the pins of `tests/read_contract.rs` (about 1096-1102 and 1612-1618); the desktop's `services/poll_convert.rs` (`target_disposition` about 267, `disposition` about 476), `model/polling/attempt.rs` (`Disposition` about 103, `AttemptSummary::of`), its activity copy's `disposition_sentence` and `disposition_name` (`states.rs` about 185 and 204) and `model/polling/entry.rs` (about 210); `skills/manyhands-polling-and-index/references/poll-results.md`. The CLI's human report prints `as_str()` and `guidance()` with no per-value match (`src/bin/manyhands-cli/polling.rs` about 111-125).
7. **The poll's answer is pinned.** `PollMutationDto` (`mutation/dto.rs` about 2810) has five fields, and `from_stored` (about 2820) refuses any other count and any target of other than six; `PollRetirementMutationDto` (about 2894) nests it, and the stored decoder (about 3363-3372) tells an ordinary poll by `completion`. `RemotePollReport` (`remote/poll.rs` about 1203) is built in `poll_result_in_transaction` (about 2487) inside `finish_remote_operation_capturing` (about 2273), before the whole-repository refresh (about 2307), which changes only `Complete` against `IndexPending`. A retiring poll freezes `report_image` (`context_retirement/handoff.rs` about 98) in that same transaction, and `decode_frozen` (`remote/poll.rs` about 1227) is a byte-exact round trip at version 1; `poll_stored_result` (about 2476) rebuilds a report from `remote_poll_dispositions`, and the report's equality is asserted after a replay (`poll_tests.rs` `task6_postfetch_advances_primary_and_replays_without_transport`). The contract calls the data "untagged, unchanged five fields" (`cli-contract.md` about 580).
8. **The proof and retirement's own check.** `context_retirement/proof.rs`: `preservation` (about 27; `Ancestor` or `Folded`, refused as `InvalidPrimaryItem`, `TicketOpen`, `MissingObject`, `ShallowHistory`, `UnrelatedHistory`, `MultipleMergeBases` or `ContentMismatch`, with an exception for an older open copy of a ticket's file), `unheld_entries` (about 100, Cycle 03's, without that exception) and `completion` (about 214; for a document only presence). `context retire`'s own check (`context_retirement/run.rs` `compare_local_proof` about 927) is `preservation` against the checked-out local primary for both kinds, with `HEAD` on the primary and the item's file on disk equal to the primary's blob, and the context's worktree must pass `ContextCleanup::precheck_context` (`cleanup.rs` about 189: clean, untracked files and nested repositories included); a failure answers `external_change`. It needs the remote branch absent (`context_retirement/record.rs` `require_fetch_absence` about 1680).
9. **`context retire` is unconfirmed.** `mutation/command.rs` `needs_confirmation` answers false for `ContextRetire` (about 711), and `mutation/bind/retire.rs` `as_confirmed` returns `None` (about 307); its catalog entry declares `--prepare` and `--confirm`, which answer `not_confirmable` (`cli-contract.md` about 194). The poll's `--retire-completed` runs one `ContextRetireBinding` child per candidate (`mutation/bind/poll_retirement.rs` `run_live` about 196-272).
10. **The guarded remote delete and the direct cleanup.** `transport/remote.rs` `delete_exact` (about 318) answers `Absent` when the ref is not advertised and otherwise `Sent`, a rejection or a lost connection included, under the push-negotiation guard `transport/callbacks.rs` `guard_delete` (about 275; `delete_update_allowed` about 232: one update, from the expected tip to zero, on that ref); `observe_delete` (about 357) on a fresh connection answers `Deleted`, `StillPresent` or `Changed`. The direct cleanup (`promotion/run.rs` about 1975-2304, `Direct::delete` about 122) reserves the context's remote (`RemoteOperationTarget::for_context`, `reserve_remote_operation` or `restart_remote_lifecycle`), calls `remote/sync.rs` `with_scoped_action_remote` (about 4563) fenced by `reservation::owned`, then removes the worktree and the branch (`cleanup.rs` `ContextCleanup`), deletes the tracking ref (`delete_tracking_ref` about 572, which gives the `RetirementEvidence`), and finishes in `complete_record` with `cleanup::retire_context` (about 953: the `remote_context_states` row, `retired_contexts`, and `pull_request_links::delete_link` about 1019) and the index marks.
11. **`repo setup-cancel` is the template for a confirmed verb over a local record.** `mutation/bind/setup_cancel.rs` (`prepare` about 245, `preview` about 351, `row` about 292, `observe_again` about 330) and `setup/cancel.rs` (`run` about 372): a row begun with `begin_lifecycle` and `LocalStep::SetupCancelRecorded`, the lease let go before any connection, a `SetupChannel` seam (`setup/channel.rs` about 23-54) with a bare-repository `Stub` for tests (`setup/test_world.rs` about 96-160; `cancel_tests.rs` `Delete` cases about 31-44), each step idempotent and the same request re-running from the record. It holds no remote reservation.
12. **Remote reservations.** One active row per repository (`remote/state.rs` unique index about 1153); `reservation.rs` `reserve_remote_operation_kind` (about 1599) answers `busy`, or `poll_yielding` to a poll it overtakes; `remote_operation_records.action` is CHECK-limited to `poll`, `synchronize_context`, `synchronize_primary`, `promote` and `close` (about 1122), and a new value needs a table rebuild with foreign keys pointing at the table; `finish_parent` (about 1341) and `restart_remote_lifecycle` (about 1415) need a lifecycle parent row; `item sync` reserves `synchronize_context` with no parent (`remote/sync.rs` `synchronization_reserve_owned` about 4393). A verified publication by a `synchronize_context` operation deletes the item's `retired_contexts` row (`remote/state.rs` `consume_retirement_on_publication` about 5098).
13. **The retirement fence.** `closure/record.rs` `written_closure_holder` (about 1635), `branch_hold` (about 1679) and `hold_refusal` (about 1688); `promotion/record.rs` `pull_request_promotion_holder` (about 940); sites `context_retirement/authority.rs` (about 326-340 and 441-450), `source.rs` (about 200-216) and `execution.rs` (about 330-342).
14. **A setup record ends at activation.** `setup/record.rs` `delete` (about 345) removes it, so after a setup merged no record keeps its branch's published tip.
15. **Made-ready.** `read/graph.rs` `tickets_made_ready` (about 1044) reads the index's effective copy as it is now; a direct close's report chooses `current`, `index_pending`, `index_unavailable_final`, `not_computed` or `not_applicable` (`mutation/bind/lifecycle.rs` `made_ready` about 881); `LifecycleMadeReadyDto { id, slug }` and `LifecycleMadeReadyState` (`mutation/dto.rs` about 1497-1516).
16. **The registry.** The registry and the index share one SQLite file; `discovery.rs` `migrate_registry` (about 2559; its calls about 2746-2760) ends with `migrate_pull_request_links` (about 2966), whose pattern (check the state, an Immediate transaction, the table, the marker) is the model, outside the first transaction. `index rebuild` (`repository.rs` `rebuild_repository` about 2213, `persist_observation` about 8071) deletes `contexts` and `problems` only, so a table keyed on `repositories(id)` survives it; `repo remove` deletes the `repositories` row with foreign keys on, so such a table's rows cascade; a corrupt registry is replaced whole. The table list is pinned by `tests/repository_enablement.rs` `registry_creates_repository_and_discovery_metadata_schema` (about 581-654), and the first transaction's markers by `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` (about 1075-1144). Table names starting `remote_`, `retirement_` or `context_retirement_` are scanned or audited by other code.
17. **Reads.** `read/items.rs` `show_item` (about 2579) and `read_path` (about 2783) call `fill_complete_read` (about 1535) inside `read_session` after `same_indexed_metadata` (about 1853) has compared the file with the index, so a field it fills never makes a read stale; lists never call it. It reads `pull_request_links` through `repo.registration_id()`. `ClosureDto` (`read/dto.rs` about 617) has four fields, always serialized and required by `schemas/v1/item.schema.json`'s closed `closure`; `ItemDto` (about 520-590) is closed at the top as well. `ClosureDto` literals: `read/items.rs` (about 1514, 1520), `desktop/model/drafts/create.rs` (about 139), and the tests `closure/run_tests.rs` (about 1572), `drafts/compare_tests.rs` (about 109, 151), `desktop/model/edit_tests.rs` (about 92), `desktop/model/ticket_view_tests/fixtures.rs` (about 64), `bin/manyhands-cli/human_tests.rs` (about 90, 318, 345, 381, 400) and `tests/ticket_closure/journeys.rs` (about 406). Human `show` prints `Closed at:` and `Closed by:` (`bin/manyhands-cli/human.rs` `item_show` about 379-391).
18. **The root checkout.** No helper says "on the primary and clean" for everyone. The poll's guard is `remote/sync.rs` `materialized_target` (about 638-690: not a worktree, the primary ref present, `HEAD` symbolic to it) and `require_clean_target` (about 692-790: no in-progress operation, no conflict, no status entry with untracked files counted and ignored ones not, and an index equal to `HEAD`'s tree); setup's G5 (`setup/poll.rs` about 454-487) and `repository.rs` `worktree_state` (about 12176) differ from it. A root off the primary gives the primary target `inaccessible`, a dirty one `dirty`, a conflicted one `conflicted`, with guidance shared with contexts and shown only in human output. `repo inspect` (`read/admin.rs` `inspect_selected` about 420-508) reports problems in `configuration.problems`; `ProblemCode` (`results.rs` about 849) has read-only codes with no stored value (`operations_unreadable`, `setup_unreadable`); `schemas/v1/problem.schema.json` and `index_problem.schema.json` list the codes.
19. **Test support.** `remote/poll_tests.rs` `PollFixture` (about 1141) and `LocalFixture` (about 3319, 4295), with `remote/poll.rs` `synthetic_fetch` (about 1435), which replaces only the fetch; the Wave 06 lifecycle tests' local bare forge: `promotion/run_pull_request_lifecycle_tests.rs` (about 22-187: `Forge`, `merge_commit`, `squash`, `fast_forward`, `delete_branch`, `Merge`, `poll`, `disposition`) and `closure/run_pull_request_lifecycle_tests.rs` (about 18-250: `moves_on`, `poll_of`, `Peer`, `assert_retired_by`); `promotion/run_tests.rs` `FakeIo` (about 357-466) fakes `delete`; `activity.rs` `take_diffed` is a test-only counter of commits diffed.
20. **SSH.** No non-SSH publication remote can be selected (RFC, Testing), so a `poll once` and a remote delete run over SSH outside library tests. Cycle 03's case `tests/mutation_remote/c5_lifecycle.rs` `a_pull_request_close_publishes_only_the_ticket_branch` (about 3701-4070), which Cycle 04 extended with its own cursor, checks every server ref update with `only_pushed` over `receive_updates`; `tests/mutation_remote.rs` (about 5418-5430) writes a squash straight into the server's bare repository, which bypasses `receive_updates`.
21. **Shared vocabularies.** `LifecycleBlocker` (`mutation/dto.rs` about 421-450) is promote's and close's blocker list, with `context_missing`, `context_nonconforming`, `context_dirty`, `context_conflicted`, `conflict_pending`, `primary_missing`, `primary_not_checked_out`, `primary_dirty`, `primary_conflicted`, `context_behind_remote` and `history_unknown`; `PreviewEffect` (about 181-229) has `remote_branch_deleted`, `worktree_removed` and `branch_deleted`; `LifecyclePathChangeDto` (about 373) and its human rendering `not on <primary>: <change> <path>` (`bin/manyhands-cli/human_admin.rs` `pull_request_closure_details` about 171-186).
22. **Registrations.** `tests/cli/inventory.rs` counts 46 catalogued mutating verbs (about 451) and 83 catalogued commands (about 469), with `NO_INPUT` (about 35-100) and `DATA` (about 107-336); `tests/support/conformance.rs` `HOMES`; `tests/support/confirmation.rs` `preview_schema`; `tests/cli/skills.rs` `AUTONOMY` (about 461); `tests/cli/recovery_tables.rs` `FAMILIES`; `tests/mutation_contract.rs` `DATA_ROOTS` (79); the `Command` classifiers (`mutation/command.rs` about 85-711), `Mutation` (`mutation/mod.rs`), `bind/mod.rs` `recorded_effects`, `settle.rs`, `records.rs` and `dto.rs` `from_stored`; a local action's name in `recovery.rs` (about 3785), `read/dto.rs` (about 900), `read/status.rs` (about 243) and the operation schemas.

## Part A: The Arrival Record

**The table.** `src/repository/item_merges.rs` (a new module, beside `pull_request_links.rs`) owns `item_merges`, created by `discovery.rs` `migrate_item_merges` after `migrate_pull_request_links`, on `migrate_pull_request_links`'s pattern, with the marker `wave_06_item_merges_v1`:

```sql
CREATE TABLE item_merges (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('ticket','document')),
    item_ulid TEXT NOT NULL,
    commit_oid TEXT NOT NULL CHECK(length(commit_oid)=40 AND commit_oid NOT GLOB '*[^0-9a-f]*'),
    merged_at INTEGER NOT NULL CHECK(typeof(merged_at)='integer' AND merged_at BETWEEN 0 AND 253402300799),
    operation_ulid TEXT NOT NULL,
    PRIMARY KEY(repository_id, kind, item_ulid))
```

It survives `index rebuild` and goes with `repo remove` (Baseline Fact 16). Its functions: `merge_of(connection, repository_id, kind, item) -> Option<ItemMerge>`, `merges_by(connection, repository_id, operation) -> Vec<ItemMerge>`, and `record(tx, repository_id, operation, &[Arrival])`, an upsert per item, so a later arrival of the same item (a document promoted again) replaces the row.

**The rule (R-1 to R-4).** `item_merges::arrivals(repository, before, after) -> Result<Vec<Arrival>, git2::Error>`:

1. Nothing when `before` equals `after` or is not an ancestor of it.
2. The candidates: `diff_tree_to_tree(before's tree, after's tree)`, with no rename detection, keeping the paths added or modified at `after` that are a ticket's file (`.manyhands/tickets/<ULID>/ticket.md`) or a Markdown file under `docs/`. A ticket is a candidate when its `after` copy parses with `closed_at` and its `before` copy is absent, does not parse, or has no `closed_at`. A document is a candidate when its `after` copy parses as a managed document, which gives its ID; a move counts, since its new path is added.
3. The walk: a `Revwalk` that pushes `after`, hides `before` and simplifies to first parents, newest first. Each commit is diffed against its first parent (the empty tree for a root), restricted by a literal pathspec to the candidates' paths not yet assigned. The first commit that changes a candidate's path assigns it that commit and its committer time. The walk stops when every candidate is assigned or the commits run out.
4. An unassigned candidate (its change came only through a commit `before` holds) is dropped (R-4).

A test-only counter of commits diffed, as `activity.rs` `take_diffed`, lets P2 pin that no commit outside the range is read.

**Recording (R-5, R-6).** Each move of Baseline Fact 1 that moves `refs/heads/<primary>` calls `item_merges::record_after_move(service, root, operation, before, after)` once its move has succeeded, outside the move's own transaction: it computes the arrivals and records them in one Immediate transaction. A failure to compute or record is dropped: no row, and the verb's answer does not change. The sites: `poll_fast_forward_unlocked` for the primary target; `reconcile_pending_candidate`, both stages of `integrate_divergence` and `synchronize_remote_observed_inner` for a `Primary` target; the lifecycle runner after `L::apply` answers `Integrated::Merged`, with `before` the old primary it captured before `write_commit` (Task 3 settles the plumbing, U-3); and `setup/poll.rs` `activate` after its move. Each passes its own operation ID. A resume that skips the move, a move by `conflict resolve`, and a move made outside Manyhands record nothing (K-1, K-3).

## Part B: `merged_at` In Reads

- `ClosureDto` gains `merged_at: Option<String>`, in `closed_at`'s format (RFC 3339, UTC, whole seconds), required and nullable in `item.schema.json`. `fill_complete_read` fills it for a ticket whose closure state is `closed` and which has a row (R-7); it is null otherwise and in every list.
- `ItemDto` gains a top-level `merged_at: Option<String>`, required and nullable. `fill_complete_read` fills it for a document that has a row, read from its primary copy or its active context alike, since it says when the primary's copy arrived; it is null for a ticket (whose time is on its closure), for a nonconforming entry and in every list.
- Human `ticket show` and `document show` print `Merged at: <time>` after the closure lines, or after the title for a document, when it is set.

## Part C: The Poll's `merged` List

- `PollMutationDto` gains `merged: Vec<PollMergedDto>`, with `PollMergedDto { kind, item_id, merged_at, made_ready, made_ready_state }`; `made_ready` and `made_ready_state` are those of Baseline Fact 15 for a ticket and null for a document.
- The binding (`mutation/bind/poll.rs`) builds it after `poll` returns, so after the poll's refresh, from `item_merges::merges_by` its own operation ID, ordered by kind and item ID (R-8). `RemotePollReport`, its frozen image, `decode_frozen` and `poll_stored_result` do not change.
- `made_ready_state` is `current` when the poll answered `Complete` and `tickets_made_ready` read a complete graph that holds the ticket closed, and `index_pending` otherwise (R-9).
- `from_stored` accepts five fields, as an answer stored before this Cycle (`merged` empty), or six; a target keeps six. `PollRetirementMutationDto`'s `poll` is the six-field form, with the same `merged` (Task 6's checkpoint finds where the retiring answer is composed, its cached summaries included, U-4).
- Human output prints, after the targets, one line per entry, `merged: <kind> <item_id> at <merged_at>`, and for a ticket one line per dependent, `  made ready: <id> <slug>`, or `  made ready: <state>` when its state is not `current`; the generic field printer leaves `merged` out.

## Part D: Detection Per Context

Each poll classifies a context, after the primary moved (Baseline Fact 4), by this table. "Held" is a cheap pre-filter (R-12): for a ticket, the local primary's blob at its path parses with `closed_at`; for a document, the local primary's blob at its path equals `L`'s. "Proof" is `preservation(L, local primary)`, run only when held.

| Kind | Remote branch | Relation of `L` to `R` | Held | Proof | Disposition |
| --- | --- | --- | --- | --- | --- |
| ticket | observed deleted | — | yes | passes | `closed`, as today |
| ticket | observed deleted | — | yes | fails | `unheld` (new; today `closed`) |
| ticket | observed deleted | — | no | — | `remotely_deleted`, as today |
| document | observed deleted | — | — | passes | `promoted`, as today |
| document | observed deleted | — | — | fails | `remotely_deleted`, as today |
| ticket | present | equal | yes | passes | `merged` (new) |
| ticket | present | equal | yes | fails | `unheld` (new) |
| document | present | equal | yes | passes | `merged` (new) |
| either | present | equal | otherwise | — | `current`, as today |
| either | present | any other | — | — | as today |

- A document is never `unheld` (decision 3): the poll cannot tell a squash the primary moved on from an edit in progress (K-6).
- Retirement still acts on `closed` and `promoted` only (Baseline Fact 5), so `merged` and `unheld` are never retired by a poll, `context retire` or the desktop; their way out is `context cleanup`.
- Guidance: `merged`, "Done on the primary while its remote branch remains: run context cleanup to delete the branch and retire the context."; `unheld`, "Closed on the primary, but its branch holds changes the primary does not: review them with context cleanup --prepare."
- The two values join every list of Baseline Fact 6. The registry's CHECK gains them through a new frozen catalog: the current `POLL_DISPOSITION_SCHEMA` frozen as `POLL_DISPOSITION_SCHEMA_18`, a current one with `merged` and `unheld`, and an upgrade in `migrate_poll_dispositions` on the `unpublished` upgrade's pattern, with the marker `wave_06_poll_dispositions_merged_v1` (R-11; Task 5's checkpoint, U-5).

## Part E: `context cleanup`

`context cleanup --repo R --id I`, with `--prepare`, `--confirm <ULID>` and `--request-id`, no input and no `--kind` (the item's kind is read from it). It is confirmed and works the same in either mode (R-25).

### Required Outcomes Of `context cleanup`

| # | Situation | Required outcome |
| --- | --- | --- |
| CC1 | Prepare: an ID that is no item | `item_not_found`, nothing recorded |
| CC2 | Prepare: an item with no active context | blocked, `context_missing`, nothing recorded |
| CC3 | Prepare: an unfinished closure or promotion record holds the item (Baseline Fact 13) | `recovery_required` naming its holder, as `hold_refusal` builds it; nothing recorded |
| CC4 | Prepare: an unfinished `context cleanup` of the item from another request | `recovery_required` naming that request; the same request continues it |
| CC5 | Prepare: blockers | blocked, every blocker in `data.blockers` in this order: `context_nonconforming`, `context_dirty`, `context_conflicted`, `conflict_pending`, `primary_missing`, `primary_not_checked_out`, `primary_dirty`, `primary_conflicted`, `history_unknown` (no observed tip of a branch that may be on the remote), `context_behind_remote` (`R` ahead of `L` or beside it), `not_done_on_primary` (a ticket whose local primary copy has no closure; a document of which the local primary holds no copy, by its ID) |
| CC6 | Prepare: the proof cannot be decided (a shallow repository, missing objects, unrelated histories, several merge bases) | `recovery_required`, nothing recorded |
| CC7 | Prepare: otherwise | the preview `context_cleanup_preview`: item, kind, slug, branch, primary branch, remote (null with none), `context_oid` (`L`), `remote_context_oid` (`R`, null when the last fetch saw no remote branch), `primary_oid`, and `unheld` (empty when the proof passes, otherwise `unheld_entries(L, local primary)`, R-22); effects `remote_branch_deleted` (only when `R` is set), `worktree_removed`, `branch_deleted`; human output prints `not on <primary>: <change> <path>` per entry |
| CC8 | The consent digest | binds the item, kind, `L`, `R`, the `unheld` list, and the remote's name and endpoint; not the primary's commit |
| CC9 | Confirm: a difference from consent (a save on the branch, a fetch that moved `R`, a changed `unheld` list, a changed remote, a new blocker) | `external_change`, nothing recorded or deleted |
| CC10 | Confirm with `R` set: the branch advertised at `R` | the guarded delete at `R`, observed on a fresh connection: `Deleted` goes to CC13 with `remote_branch` `deleted` |
| CC11 | Confirm with `R` set: advertised at another tip | `external_change`: nothing deleted, nothing retired, the record ended; a poll and a new prepare follow (K-11, R-23) |
| CC12 | Confirm with `R` set: not advertised (the forge deleted it since) | CC13 with `remote_branch` `absent` |
| CC13 | The local retirement | the worktree, the local branch and the tracking ref removed, the link row deleted, the context recorded retired and the index marked, as the direct cleanup ends (Baseline Fact 10); answers `ok` with the data `context_cleanup` (`item_id`, `kind`, `operation_id`, `remote_branch`) and human `remote branch: <state>` and `context: retired` |
| CC14 | Confirm with `R` unset (no publication remote, never published, or observed deleted before prepare) | CC13 with `remote_branch` `none`, and no connection |
| CC15 | The delete not seen (still at `R` after it) | `push_rejected`; nothing retired; the record unfinished; the same request tries again |
| CC16 | A transport, key, host, `busy` or `poll_yielding` stop before or during the delete | that stop's code, as `item sync` answers it; nothing retired; the record unfinished; nothing remote held; the same request continues |
| CC17 | A process lost after the delete was sent and before it was observed | the same request observes again: absent goes to CC13; at `R` deletes again; elsewhere is CC11 |
| CC18 | A process lost after the deletion was observed, before or during the local retirement | the same request finishes the retirement with no connection |
| CC19 | From the deletion's fresh advertisement until the local retirement has finished | no other remote operation of the repository runs (`busy`, or a poll yields), so no synchronization or comment publishes the branch again and no poll materializes it again; a process lost in that span leaves it so until the same request finishes (R-24, K-12) |
| CC20 | `request cancel` or Ctrl-C before the delete is sent | `cancelled`, exit 130, nothing deleted, the record unfinished; after it is sent, the cleanup finishes |
| CC21 | A finished request sent again | replays its answer; a new prepare for the retired item is CC2 |
| CC22 | A poll's retirement or `context retire` retires the context while a stopped cleanup waits | the same request answers `ok`, with `remote_branch` as observed |

### Mechanism (Recommended; Task 7's Checkpoint Decides, R-19)

- **Its record.** An operation record of a new local action, `context_cleanup`, begun as `repo setup-cancel` begins its row (Baseline Fact 11), with steps `context_cleanup_recorded`, `remote_branch_deleted` and `retired`. It keeps `L`, `R`, the remote's name and endpoint digest, and its step, so that a re-run works without the live tracking ref (where they are kept is U-2).
- **The remote.** The cleanup reserves the item's context target with the existing action `synchronize_context` through `reserve_remote_operation`, under its own operation ID and with no parent, as `item sync` does (Baseline Fact 12); it holds the reservation from the deletion's fresh advertisement until the local retirement has finished, and releases it with `finish_remote_operation`; a stop before the delete is sent releases it. The transport goes through `with_scoped_action_remote` with `delete_exact` and `observe_delete` behind a seam that tests fake, as `SetupChannel` and `PromotionIo` are faked. Whether `synchronize_context` serves without changing the reservation's rules is U-1: if it does not, Task 7 stops for the owner with the alternatives (no reservation, as `repo setup-cancel`, with the cleanup's record holding the item against synchronizations; or a new action with the table rebuild of Baseline Fact 12).
- **The local retirement.** `ContextCleanup` removes the worktree and the branch, `delete_tracking_ref` gives the `RetirementEvidence`, and one transaction runs `cleanup::retire_context`, the index marks and the record's last step, as the direct cleanup's `complete_record` does (Baseline Fact 10).

## Part F: The Checkout Warning

- One helper, `remote::poll::root_checkout(root, primary) -> RootCheckout` (`OnCleanPrimary`, `NotOnPrimary`, `Dirty`, `Conflicted`), is the poll's own guard for the primary target, `materialized_target` then `require_clean_target` (R-16).
- `repo inspect`, for a registered repository whose configuration is valid and names a publication remote (R-17), adds to `configuration.problems` the new read-only code `primary_not_checked_out` for `NotOnPrimary` and `primary_dirty` for `Dirty` or `Conflicted`, with the path null. Guidance: "The root checkout is not on the primary branch, so polls do not bring merges in until it is: check out the primary branch." and "The root checkout has changes or an operation in progress, so polls do not bring merges in until it is clean: commit, move or remove them." Human output prints them as other problems.
- `poll once`'s human output adds, under the primary target's line when its disposition is `inaccessible`, `dirty` or `conflicted`, the line `  merges into <primary> are not noticed until the root is on <primary> with no changes`. Its JSON does not change (R-18).

## Part G: Registrations And Shapes

1. `Command::ContextCleanup` with an arm in every classifier, `Mutation::ContextCleanup`, the binding `mutation/bind/cleanup.rs` (a `Confirmed` binding), and the CLI's catalog entry (`context cleanup`, `REPO`, `ID`, `REQUEST_ID`, `PREPARE`, `CONFIRM`, `confirmed: true`), dispatch, handler and human renderer.
2. `ContextCleanupPreviewDto` as a `PreviewDetailsDto` variant and `ContextCleanupMutationDto` as a `MutationDataDto` variant with its `from_stored` arm; `schemas/v1/context_cleanup_preview.schema.json` and `context_cleanup_mutation.schema.json`; `remote_branch` is `deleted`, `absent` or `none`.
3. `LifecycleBlocker::NotDoneOnPrimary` (`not_done_on_primary`), and the cleanup's blocked prepare as `LifecycleBlockedDto` (R-20).
4. `PollDisposition` and `PollTargetDisposition` `Merged` and `Unheld`, at every place of Baseline Fact 6.
5. `PollMutationDto.merged` and `PollMergedDto`, in `poll_mutation.schema.json`.
6. `ClosureDto.merged_at` and `ItemDto.merged_at`, in `item.schema.json`.
7. `ProblemCode::PrimaryNotCheckedOut` and `PrimaryDirty`, in `problem.schema.json` and `index_problem.schema.json`.
8. `item_merges` and its migration (Part A).
9. The local action `context_cleanup` and its steps, at the places of Baseline Fact 22.
10. The desktop: compile-level arms only (R-14).
11. Goldens: `cli_v1` `context_cleanup_preview` and `context_cleanup_blocked` (JSON and text, no network); `mutation_v1` `context_cleanup_ok`, from the SSH steps; the poll goldens gain `merged` (E-4).

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). P1 to P9 are library or non-SSH tests; P10 adds steps to one existing SSH case. Every new row is run red once, against a deliberate stub when it passes at once, and recorded in the ledger.

- **P1** (Task 1) `item_merges_migration_creates_its_table_once`, on `pull_request_links_migration_creates_its_table_once`'s pattern (its columns, idempotence, the CHECKs, a drop of the table and of the marker, the cascade), and a row kept across `index rebuild`.
- **P2** (Task 2) `arrivals_follow_the_range`, one table: a merge commit, a squash and a fast-forward, each for a ticket's closure, a new document and an edit of a merged one; two arrivals in one range; a ticket file changed with no closure; a document deleted; a document moved; `before` not an ancestor; an unassigned candidate; and the counter equal to the range's first-parent commits, never more.
- **P3** (Task 3) `every_move_of_the_primary_records_its_arrivals`: the poll's, `repo sync`'s clean path and its divergence stages, a direct close's integration and a setup's activation, each one row with its operation ID; a recording that fails leaves the answer as it was.
- **P4** (Task 4) `tests/read_items.rs` `ticket_show_gives_merged_at_on_its_closure` and `document_show_gives_merged_at`: `show` and `show --path` give `merged_at`, lists give null, a closing ticket gives null; a human-output test that `show` prints `Merged at:`.
- **P5** (Task 5) `detection_per_context`, one table over Part D's rows, each kind, with the pre-filter's misses (a document edited after its merge, a ticket whose primary copy is open, a ticket whose branch holds a later code commit); `the_poll_disposition_catalog_upgrades_from_18`, from a table at the `_18` catalog holding rows.
- **P6** (Task 6) `a_poll_lists_its_arrivals_with_the_dependents_made_ready`: entries, order, `made_ready` with `current` and `index_pending`, a document's nulls, a poll that did not move the primary, a retiring poll's `poll`; and E-2's five-field decoding.
- **P7** (Tasks 7, 8) over CC1 to CC22, with the transport seam faked over a bare repository and the clock injected: `context_cleanup_prepare_outcomes` (CC1 to CC9), `context_cleanup_confirm_outcomes` (CC9 to CC15, CC21), `context_cleanup_stops_and_crashes` (CC16 to CC18, CC20) and `context_cleanup_holds_the_remote_between_delete_and_retirement` (CC19, CC22).
- **P8** (Task 9) `after_the_merge_lifecycles`: Part D and Part E end to end with the local bare forge, for a ticket and for a document (new, and an edit of a merged one), each merge style, the branch deleted at merge and kept; a squash after which the primary moved on; and a second clone made after the merge that has no `merged_at`.
- **P9** (Task 10) `tests/read_repository.rs` `repo_inspect_warns_when_the_root_is_off_the_primary`, `poll_tests.rs` `root_checkout_is_the_poll_s_guard`, and a human-output test of the poll's warning line, for a root on another branch, detached, dirty and clean, with no network for inspect.
- **P10** (Task 11) steps added to Cycle 03's SSH case: the forge's merge commit written into the server's bare repository with the ticket's branch kept, a poll (`merged`, the `merged` entry), `context cleanup --prepare` and `--confirm`, and the only server ref update since its cursor the deletion of the ticket's branch; the server's primary unchanged by Manyhands throughout. It writes `context_cleanup_ok` and a poll golden with a `merged` entry. It needs SSH because the deletion is a transport write and no non-SSH remote can be selected (Baseline Fact 20); its edge cases are P7's.

At review-ready the SSH tier is derived from `git diff --name-only origin/main...HEAD` against AGENTS.md's table. At planning the paths give: every `src/repository/remote/` target (`ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`), `mutation_remote` (`src/repository/mutation/`), `document_promotion` (`src/repository/promotion/`) and `ticket_closure` (the closure lifecycle tests' named edits).

## Named Edits To Pre-Existing Tests

- **E-1** `tests/repository_enablement.rs` `registry_creates_repository_and_discovery_metadata_schema` (about 581-654): the table list gains `item_merges`.
- **E-2** `mutation/dto.rs` `ordinary_poll_stored_reports_keep_their_untagged_shape_and_all_old_values` (about 3464): it asserts that a five-field record decodes, with `merged` empty, and that a six-field record round-trips, where today it asserts a five-field round trip.
- **E-3** `mutation/dto.rs` `poll_retirement_stored_contract_is_closed_and_cannot_complete_a_not_reached_prefix` (about 3054): its nested `poll` gains `merged`, if its records need it (Task 6 reads it).
- **E-4** The poll goldens gain `"merged": []`, or entries: `cli_v1` `poll_once_fast_forward_and_materialize`, `poll_once_dirty`, `poll_once_diverged`, `poll_once_malformed*`, `poll_once_remotely_deleted`, `poll_once_closed_ticket` (and their `_text` twins if a line changes), and `mutation_v1` `poll_once*` and `poll_retirement`, rewritten by the cases that write them (`tests/cli_remote/polling.rs`, `tests/mutation_remote/c5_poll.rs` and others Task 0 finds), each run by exact name with `MANYHANDS_UPDATE_GOLDEN=1`.
- **E-5** Every item golden gains the top-level `"merged_at": null`, and every closure in them `"merged_at": null`: `tests/fixtures/read_v1/` and `tests/fixtures/cli_v1/` (`ticket_*`, `cli_ticket_*`, `document_*`, `cli_document_*` and the lists, plans and trees that hold items), rewritten by their fast-tier targets with `MANYHANDS_UPDATE_GOLDEN=1`.
- **E-6** The struct literals: `ClosureDto` (Baseline Fact 17) gains `merged_at: None`; `ItemDto` literals that name every field gain `merged_at: None`; `PollMutationDto` literals (`bind/poll.rs`, `dto.rs`, `bin/manyhands-cli/polling_tests.rs`, the desktop's `poll_convert_tests.rs`, `scheduler_attention_answer_tests.rs`, `scheduler_loop_tests.rs`, `tests/mutation_remote/c5_poll.rs`) gain `merged: Vec::new()`.
- **E-7** A ticket `closed` whose proof fails becomes `unheld` (R-10): `remote/poll_tests.rs` `retirement_recognition_committed_primary_and_independent_ticket_completion` (about 25), whose unmatched rows expect `unheld`; and any of `tests/ticket_closure/journeys.rs` `contained_closed` (about 311), `tests/cli_remote/retirement.rs` (about 127) and the `poll_once_closed_ticket` golden that pins such a case (Task 0 reads each).
- **E-8** The Wave 06 lifecycle tests whose kept branch reads `current` after the merge now read `merged`: `closure/run_pull_request_lifecycle_tests.rs` `merged_first_with_the_remote_branch_kept_closes_in_place` (about 791) and `promotion/run_pull_request_lifecycle_tests.rs` `pull_request_promote_lifecycles` (about 277), where a row's ticket or document is done on the primary with its branch present and equal (Task 0 reads each row).
- **E-9** `tests/read_contract.rs`'s enumeration pins (about 725-734, 1096-1102, 1612-1618) and the blocker, problem and effect lists gain the new values.
- **E-10** `tests/mutation_contract.rs` `DATA_ROOTS` gains the two new roots (79 to 81).
- **E-11** `tests/cli/inventory.rs`: 46 to 47 mutating verbs and 83 to 84 commands; `NO_INPUT` and `DATA` gain `context cleanup` (`blocked`, `context_cleanup_preview`, `context_cleanup` and the synchronization stops' data Task 7 lists).
- **E-12** `tests/support/conformance.rs` `HOMES`, `tests/mutation_confirmation/conformance.rs` (the preview and blocked rows) and `tests/support/confirmation.rs` `preview_schema` gain `context cleanup`; `tests/support/mutation_golden.rs` `CASES` gains `context_cleanup_ok`.
- **E-13** `tests/cli/skills.rs` `AUTONOMY` gains `context cleanup` in the column for steps shown to the person first.
- **E-14** `tests/cli/recovery_tables.rs` `FAMILIES` and `tests/cli/collaboration_exits.rs`, if D-1 adds recovery-table rows for `context cleanup` (Task 12 confirms).
- **E-15** `remote/state_tests.rs` `retirement_disposition_exact_old_enum_upgrade_preserves_evidence_and_owner` (about 696) gains the upgrade from the frozen `_18` catalog.
- **E-16** `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` (about 1125-1144): its marker list gains `wave_06_poll_dispositions_merged_v1` (inside `remote::state::migrate`'s first transaction), and not `wave_06_item_merges_v1` (outside it).
- **E-17** `remote/poll_tests.rs` `poll_dispositions_have_only_fixed_guidance` (about 3198) and `tests/remote_polling.rs`'s guidance table (about 731) gain the two values.
- **E-18** The exhaustive matches in tests over `PollDisposition`, `PollTargetDisposition`, `MutationDataDto`, `PreviewDetailsDto` and the desktop's `Disposition` gain arms (`tests/support/mutation.rs`, `tests/support/repair.rs`, the desktop's tests; Task 0's sweep lists them).
- **E-19** `tests/mutation_remote/c5_lifecycle.rs` `a_pull_request_close_publishes_only_the_ticket_branch` (about 3701-4070): the added steps with their own cursor (P10); its earlier assertions unchanged.

Task 0's sweep greps for `PollDisposition::`, `PollTargetDisposition::`, `ClosureDto {`, `ItemDto {`, `PollMutationDto {`, `MutationDataDto::`, `PreviewDetailsDto::`, `ProblemCode::`, `LifecycleBlocker::`, `"closed"` and `"current"` beside `disposition` in `src/` and `tests/`, `remote_poll_dispositions`, `registry_migrations` and the poll and item goldens, and adds each hit beyond E-1 to E-19 to the ledger as E-20 onwards.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `poll once` row (the `merged` list, its `made_ready`, the dispositions `merged` and `unheld`, a ticket's `closed` needing the proof, the human warning line, and the closed output table's "untagged, unchanged five fields" becoming six with five-field records still decoding); the `repo inspect` row (the two problem codes); a new `context cleanup` row in the command taxonomy, with its preview, answers, stops, cancellation and replay; the `ticket show` and `document show` rows (`merged_at`); the `repo sync`, `ticket close`, `document promote` and setup rows (a move of the primary records `merged_at`); the `context retire` row (unchanged, pointing to `context cleanup` for `merged` and `unheld`); the codes and blockers tables.
- **D-2** `docs/RFC/pull-request-mode.md`: dated amendments for decisions 1 to 4 and the departures the Cycle document lists, under "After the merge", "When it reached the primary", "Checkout not on the primary", "CLI contract changes" and "Left to planning".
- **D-3** `docs/RFC/repository-index-persistence-and-refresh.md`: `item_merges`, kept across a rebuild, and the new catalog of `remote_poll_dispositions`.
- **D-4** The shipped skills, in `skills/` and `.agents/skills/`, identically where the trees agree: `manyhands-sharing-and-sync` (cleanup after a merge: the poll's `merged` list, `merged`, `unheld`, `context cleanup`, and the recommendation that the forge delete the source branch at merge); `manyhands-tickets` and `manyhands-documents` (their "a later Cycle (Wave 06 Cycle 05)" sentences, Cycle 04's K-9 and K-12 text, `merged_at`); `manyhands-polling-and-index` and its `references/poll-results.md` (the dispositions, the list, the checkout warning); `manyhands-recovery` (a setup branch left after its merge: delete it on the forge; Baseline Fact 14); `manyhands-overview/references/autonomy.md` (`context cleanup` shown to the person first).
- **D-5** `docs/research/wave-03-api-audit.md`: `context cleanup` and its codes; `poll once`'s new data.
- **D-6** At closure, with the owner's go-ahead, through `manyhands-documents`: Cycle 05's checklist in `docs/Waves/wave-06-pull-request-workflow.md`; the status of this Cycle's three documents set to done; then the three promoted.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** One function decides the arrivals of every move — decision 2 — a fault reaches every site at once, which P2's table pins.
- **R-2** A ticket arrives when its `after` copy has `closed_at` and its `before` copy has none; a document when its file is added or modified at `after` — the RFC's done states, a closure and the content — a document moved without a change of content arrives again.
- **R-3** `merged_at` is the committer time of the newest first-parent commit in the range that changed the item's file — it is the merge, squash or fast-forwarded commit the RFC names, by one rule for both kinds and with no blob read per commit — for a fast-forward it is a branch commit's time (K-2), and for a ticket saved after its closure on its branch, the save's.
- **R-4** A candidate that no commit of the walk changed gets no row — the walk never leaves the range — a rare arrival with no time.
- **R-5** Recording follows a move that succeeded, outside its transaction, and a failure is dropped — `merged_at` is for display, and a move must never fail for it — K-3.
- **R-6** Every move of Baseline Fact 1 records, and a resume that skips the move or `conflict resolve` does not — decision 2 — K-1 and K-3.
- **R-7** A ticket's time is on its closure and a document's on the item, both filled only by `show` and `show --path`, after the staleness comparison — the RFC's places, and lists read the index only — the desktop's lists show none until Cycle 06 asks for them.
- **R-8** The poll's `merged` list is built by the binding from the rows its own operation recorded, not carried in `RemotePollReport` — the report's frozen image, its replay from the table and its equality have no place for it (Baseline Fact 7) — a replay whose stored answer was lost rebuilds the list from rows a later arrival may have replaced.
- **R-9** `made_ready` and its state as a direct close gives them, read after the poll's refresh — one vocabulary — none.
- **R-10** A ticket's `closed` needs the proof as well as the closure, and one whose proof fails reads `unheld` — retirement refused such a context already (Baseline Fact 8), and the new value names its way out — a consumer that matched `closed` for such a ticket sees `unheld`, in either mode.
- **R-11** `merged` and `unheld` enter the registry through a new frozen catalog, upgraded one way as `unpublished` was — the CHECK lists every value (Baseline Fact 6) — one more one-way migration.
- **R-12** Detection runs the proof only past the cheap pre-filter — a check per context per poll must not read whole trees for items in progress — a document done at another path (moved on the primary) reads `current` (K-6).
- **R-13** `merged` and `unheld` only for a context whose `L` equals `R`, or whose branch is gone — the poll does not choose between a branch's local and remote commits — K-9.
- **R-14** The desktop gets compile-level arms only — Cycle 06 — the desktop shows nothing new until then.
- **R-15** No new SSH case: Cycle 03's case gains the cleanup steps — AGENTS.md; the deletion is this Cycle's only transport-dependent part — that case runs a little longer and fails for three Cycles' regressions.
- **R-16** `repo inspect`'s check is the poll's own guard — the warning must say exactly when a poll cannot move the primary — inspect reads an index unequal to `HEAD`'s tree as `primary_dirty`.
- **R-17** The warning only for a valid configuration that names a publication remote — no poll runs without one — a remote-less repository off its primary is not warned.
- **R-18** `poll once` warns in human output; its JSON keeps the primary's disposition — the disposition already says it, and the desktop's row is Cycle 06's — a JSON reader learns it from the disposition and `repo inspect`.
- **R-19** `context cleanup`'s behaviour is the CC table, and its mechanism is Task 7's checkpoint's, starting from Part E's recommendation — Cycle 04's re-run and fence mechanisms were settled only against outcome tables after three review rounds — the mechanism is reviewed at the checkpoint and in Task 7's and 8's reviews, not in planning.
- **R-20** The cleanup's blocked prepare reuses `LifecycleBlocker` and `LifecycleBlockedDto`, adding `not_done_on_primary` — one blocker vocabulary for the steps that end a context — close's and promote's blocker schema lists a value they never give.
- **R-21** The cleanup needs the root on a clean primary and the context clean, as retirement does — its local retirement needs both (Baseline Fact 8) — the person cleans up first.
- **R-22** `unheld` is empty when the proof passes and `unheld_entries(L, local primary)` when it fails — Cycle 03's list, while `unheld_entries` lacks the proof's ticket-file exception (Baseline Fact 8) — none.
- **R-23** A remote branch that moved since the last fetch is not deleted, and confirm answers `external_change` — decision 22's guard by the observed tip — the person polls and prepares again (K-11).
- **R-24** The cleanup's reservation spans its deletion and its local retirement — between them a synchronization could publish the branch again, and a poll could materialize a squash-merged branch again, since decision 28 is ancestry only (Baseline Fact 4) — a cleanup lost in that span holds the repository's remote until its request runs again (K-12), as a stopped direct cleanup does.
- **R-25** The verb works the same in either mode — retirement does, and a direct context can need it — none.
- **R-26** Times are RFC 3339 UTC in whole seconds, as `closed_at` — one format — none.

## Known Limits

The Cycle document's K-1 to K-12.

## Unverified

- **U-1** Whether a `synchronize_context` reservation serves the cleanup, which never publishes or observes history, without a change to the reservation's rules (Part E). Task 7's checkpoint decides, and stops for the owner if it does not.
- **U-2** Where the cleanup's record keeps `L`, `R`, the remote and its step: a small table of its own (one migration more) or the operation record's columns. Task 7's checkpoint decides.
- **U-3** How the lifecycle runner keeps the old primary for the recorder past `write_commit`, which consumes `prepared`. Task 3's checkpoint decides.
- **U-4** Where a retiring poll's answer is composed, its cached summaries included, so that its `poll` carries `merged`. Task 6's checkpoint finds it.
- **U-5** The catalog upgrade's exact shape against `migrate_poll_dispositions` and its marker checks. Task 5's checkpoint follows the `unpublished` upgrade.
- **U-6** Which `ItemDto` literals name every field and which use struct update syntax. Task 0's sweep.

## Design Checkpoint Obligations

Task 3 writes down each recording site's call, operation ID and `before`, and settles U-3. Task 5 writes down the catalog upgrade (U-5) and each place of Baseline Fact 6 it changes. Task 6 settles U-4. Task 7 writes down Part E's mechanism against every CC row before any code (U-1, U-2): a designer on the most capable model, a separate checker, and the orchestrator's approval; a row that cannot be met, or a mechanism that needs a change to the reservation's rules, a new remote action, a lifecycle table rebuild or the conflict verbs, stops for the owner.

## Alternatives Rejected

- A confirmed mode of `context retire`, or two verbs: decision 1.
- Recording at the poll's move and the direct integration only, or from a stored watermark at every index observation: decision 2.
- Allowing the manual cleanup for any published document context: decision 3.
- Deleting a surviving setup branch: decision 4.
- Carrying `merged` in `RemotePollReport`: R-8.
- No new dispositions, so that only the poll that brought an item flags it: a context that needs cleanup would be flagged once and then read `current` on every later poll.
- `PrimaryActivityMap` as the source of `merged_at`: it advances at indexing, rebuilds when its base is off the first-parent chain, and loses its base at an index rebuild (Baseline Fact 2).
- The local retirement before the remote deletion: a poll between them would materialize a squash-merged branch again (Baseline Fact 4).
- A new remote action value: a table rebuild with foreign keys pointing at it (Baseline Fact 12), unless Task 7's checkpoint finds no other way (U-1).
- A new SSH case: R-15.

## Risks

| Risk | Mitigation |
| --- | --- |
| A recording fault slows or fails a move of the primary | R-5: outside the move's transaction, dropped on failure; P3 asserts the answer unchanged |
| The walk reads history outside the range | P2's counter; `hide(before)` and first parents only |
| The disposition catalog's upgrade damages a real registry, one way | The frozen `_18` catalog, P5's upgrade from a table holding rows, E-15 and E-16, inside `remote::state::migrate`'s transaction as before |
| Detection slows every poll | R-12's pre-filter; the proof only for items done on the primary |
| `context cleanup` deletes a branch that moved | The guard at `R` (Baseline Fact 10), CC11, P7 and P10 |
| A synchronization republishes, or a poll re-materializes, a branch between its deletion and its retirement | CC19 and R-24, pinned by P7 |
| The cleanup's mechanism needs the reservation changed | U-1 and Task 7's checkpoint, which stops for the owner |
| Golden rewrites hide a regression | E-4 and E-5 are additive fields only; Task 0 lists every golden, and each rewrite's diff is read in the ledger |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| Planning | Five research agents at `0213d32d`, then three on the interfaces | — | — | — | Not a review round: Baseline Facts 1 to 22, the owner's decisions 1 to 4, and the mechanism traps behind R-8, R-11, R-24 and U-1 |
