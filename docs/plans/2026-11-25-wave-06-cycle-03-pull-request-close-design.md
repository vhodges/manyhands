---
title: "Wave 06 Cycle 03 Design: Ticket Close In Pull-Request Mode, The Closing State And Withdraw"
date: 2026-11-25
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M899G2HA7WTVW4EYZFTNBWW2"
---

# Wave 06 Cycle 03 Design: Ticket Close In Pull-Request Mode, The Closing State And Withdraw

Cycle document: [Wave 06 Cycle 03](../Cycles/wave-06-cycle-03-pull-request-close.md). Ticket: vh-gy8gz (`01M87683QK8E34EYY8KS4VQG9Y`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md) and the Cycle document's Product-Owner Decisions.

Status: **draft** (revision 3).

## Intent And Approval Boundary

In a pull-request repository, make `ticket close` record the closure on the ticket's branch, publish the branch and give the pull request link, never committing to or pushing the primary. Show a closure the primary does not hold yet as `closing`, keep its dependents blocked, let it be withdrawn, let two clones' closures settle themselves, and close a ticket whose branch merged first (with its remote branch gone) or that has no branch, through a new branch holding only the closure. Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket branch, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `3207c728`; Task 0 re-checks every one.
- "Pull-request repository" means a valid configuration with `integration = "pull_request"` (Wave 06 Cycle 01).
- "The tracking primary" is `refs/remotes/<publication remote>/<primary>`, as last fetched. "The local primary" is `refs/heads/<primary>`.
- "The primary's copy" of a ticket in a read is the row the index stores for it in a context that is not an active item context (rank `PRIMARY_COPY` in `row_rank`): normally the root checkout's working tree (K-9).
- "A provisional closure" is a closure (`closed_at` and `closed_by`) that the ticket's branch holds and the relevant primary does not (RFC, Provisional closure). Two closures are "the same" when both fields are equal.
- "The runner" is the new pull-request closure runner of Part D. Its three actions are `record` (close), `withdraw` (Part E) and `new_branch` (Part F).
- "Stops" means a step ended without its operation ending: the request record stays accepted, the runner's row stays unfinished, and the same request continues it.
- "As the direct close" means as Wave 02 Cycle 10 and the CLI contract (the `ticket close` row; "A cancellation is final", about line 1343) say for `ticket close`, and as `promotion/run.rs` does it (decision 8).

## Baseline Facts

Each fact was read in the source by the planning research, spot-checked by the author, and checked by the review rounds at `c5e335a2` and `3207c728`.

1. **The closure state.** `src/repository/read/dto.rs` `ClosureState { Open, Closed }` (a `contract_enum!`, about line 338) and `ClosureDto { state, closed_at, closed_by }`, documented "`closed_at` is set exactly when `state` is `closed`". `src/repository/read/items.rs` `closure()` (about 1354) derives the state from `closed_at` alone. Lists build it from the effective row (`stored_item_dto`), `show` from the effective copy's file (`parsed_item_dto`), and `show --path` through `read_path` (about 2373-2440).
2. **The effective copy.** `effective_rows` and `effective_copy` (`items.rs` about 2035-2120) keep one row per item, ranked by `row_rank` (about 1984-1992): the item's own worktree (`OWN_WORKTREE`), then any non-active context (`PRIMARY_COPY`, `Unverified` included). Every production read holds all stored rows (`stored_items`, about 869; the callers at about 2177, 2257, 2396 and `graph.rs` about 949, after its moved-context substitution, about 949-962), including the primary's copy of an item that has an active context (Table A of `docs/plans/2026-11-19-fast-writes-design.md`, about 189). The helpers that give `deps[].state` and `parent.state` (`Targets::of`, about 301-316; `Targets::stored_node` and `Related::of`, about 351-382) and the filters (`TicketFilter::matches`, about 81-108, including the `--ready` and `--blocked` readiness filter) see only effective rows. No helper gives the primary's copy's closure for an item that has an active context.
3. **Readiness is boolean.** `src/repository/read/graph.rs` `TicketNode { id, closed: bool, parent, deps }` (about 54-61) is filled from `row.closed_at.is_some()`. `blockers`, `is_ready`, `readiness_state`, the open-cycle test, `unplannable`, `batches` and `critical_path` (about 665-700, ties broken by lowest ID) read it; a tree line's state (about 493-497) and the deps root (about 1103) are set from it. `ticket_readiness` (about 1029) keeps rows with `row.closed_at.is_none()`. `items.rs`'s private `Target { Document, Comment, OpenTicket, ClosedTicket }` (about 285) gives `deps[].state` and is matched exhaustively in `Related::check` (about 435). `plan.schema.json`, `graph.rs` (about 1167) and `dto.rs` (about 1063) document how batches are numbered.
4. **The closure filter.** `ClosureFilter { Open, Closed, All }` (`items.rs` about 45) is applied in memory. The CLI's value parser (`src/bin/manyhands-cli/parse.rs` about 86-88) takes `open|closed|all`, and `dispatch.rs` (about 58-62) maps any other value to `All` with a wildcard, tested only by `tests/cli/content.rs` (about 352). The help text is pinned in `help_ticket_list.json`, `help_resource_ticket.json`, `help_root.json` and `catalog_tests.rs` (about 230); `parse_tests.rs` (about 408 and 433) holds the value lists.
5. **Exhaustive matches a new value breaks.** `ClosureState`: `src/desktop/model/relationships.rs` (about 336), `tickets.rs` (about 240), `relations_edit.rs` (about 327), `tests/ticket_closure/journeys.rs` (about 390), `src/desktop/model/tickets_tests.rs` (about 136). `DependencyState`: `relationships.rs` (about 101 and 298), `src/repository/read/graph_tests.rs` (about 88). `ReadinessReasonCode`: `src/desktop/model/tickets.rs` (about 159). `ClosureFilter`: `src/desktop/view/tickets/filter_bar.rs`, `src/desktop/model/list_view.rs`, `src/desktop/model/ticket_view.rs`. `PreviewDetailsDto`: `src/bin/manyhands-cli/human_admin.rs` (about 98) and `src/desktop/view/confirm.rs` (about 750). `PreviewEffect`: `human_admin.rs` `effect_phrase` (about 165). `LifecycleBlocker`: `src/desktop/view/lifecycle/prepare.rs` `blocker_words` (about 47-87). `MutationDataDto`: `dto.rs` `authority`, `tests/support/mutation.rs` (six) and `tests/support/repair.rs` (one). `OperationAction`: `src/desktop/model/activity/copy/states.rs` `ACTION_WORDS: [(OperationAction, &str); 23]` (about 261); it is published in `operation.schema.json` and `blocker.schema.json`. Seven checks compare `== ClosureState::Closed`, among them `bind/ticket.rs` `refuse_closed`, `src/drafts/compare.rs` `lifecycle_closed`, `src/desktop/model/edit.rs` `edit_offered` and the desktop's "Close ticket" action (`src/desktop/view/item/header.rs`).
6. **The closed-ticket guards.** `src/repository/ticket_guard.rs` `require_ticket_open(repo, primary_branch, item)` refuses `TicketClosed` when the local primary's tip or the context branch's tip holds either closure key; it takes no mode. Callers: `src/repository.rs` `refuse_closed_ticket` (about 299; used by `prepare_context`, `save_ticket_with_effective_config` and `checkpoint_comment_locally_with`), `context_release.rs` (about 231), `mutation/bind/republish.rs` (about 202) and `mutation/bind/resolve_history.rs` (about 226); each can read the configuration's `integration()`. The binding precheck `bind/ticket.rs` `refuse_closed` (about 237) refuses when the effective item reads `Closed`. A save edits the worktree file with the draft's fields and carries `closed_at`/`closed_by` through from that file (`draft_of`, `holds`, about 296-345 and 1008-1031).
7. **Closure immutability.** `src/repository/integration/validate.rs` `preserves_item_invariants` (about 199) requires, for a ticket, `old.closed_at.is_none() || (old.closed_at == new.closed_at && old.closed_by == new.closed_by)`. `validates_resolution_sides` (about 224) applies it to base, local and incoming against a resolution, and `validates_external_integration` (about 505) against an external merge. Remote callers: `remote/sync.rs` (about 3317 and 3434, holding the step with its window number, stage and incoming commit; about 4898, holding the record and step; about 5304, holding the observation's window number) and `remote/conflict_read.rs` (about 768, holding a `Recorded` with the step's window number); `state::integration_window` gives a pinned window's `primary_oid`. A primary-target synchronization's local side is the local primary (`sync_tests.rs` `resolution_fixture`, about 2346-2440), and tests pin that a closure only the local primary holds cannot be dropped (`sync_tests.rs` about 11008 and 11622); local synchronizations pin the same for a context-only closure (`local_sync/conflict_tests.rs` about 716 and 780). Window 0, the legacy envelope, has no intent and no `primary_oid` (`remote/state.rs` about 3266-3270); a reservation inserts only window 0, and pinned windows come from `prepare_synchronization_window` (`sync.rs` about 6934). `classify_conflict_entry` (about 171-184) gives `IncompatibleClosures` for two different closures, which `conflict_eligibility` (about 42) makes `ExternalResolutionRequired`; `resolve_synchronization` refuses any set holding such a path (`sync.rs` about 5281), so only an external repair applies to it. A clean merge is committed from a memory-only `PreparedMerge`, whose fields are private to `integration/prepare.rs` (about 44-52), without canonical validation (`remote/sync.rs` about 3712-3718 and 3786-3860); `integrate_divergence` (about 3616) receives the stage's inputs, including the fetched primary, as `DivergenceInputs`. `prepare_clean_merge` calls `merge_commits` with libgit2's defaults (`prepare.rs` about 90 and 173). The integration module's tests are in `integration/tests.rs`. Canonical parse and serialize check only that both fields are present or absent.
8. **The direct close.** One runner (`src/repository/promotion/run.rs` `run_lifecycle`, `promote_inner`) with the durable parent `ticket_closures` (`src/repository/closure/record.rs` about 229, whose DDL is validated exactly, with attempt columns per child and the steps `abandoned` and `superseded`). Its closure checkpoint is written under the repository lease immediately before the merge and never pushed on the branch (Wave 02 Cycle 10 decision C1); `written_closure_unfinished` (`record.rs` about 774) refuses any synchronization of the ticket context while a record is between `closing` and `branch_deleted`, and the poll consults it before materializing or fast-forwarding a context (`poll.rs` about 383-394 and 662-671). Its children run through `synchronize_promotion_child_observed` with a `ChildExecution` (`promotion/lifecycle_record.rs` about 520-574), which reads only a promotion or closure parent. `run.rs` `child()` (about 1001-1090) replays a completed child, restarts a retained one, and replaces a cancelled, interrupted, failed or abandoned one with a new attempt (`next_attempt`), letting a child with nothing to lose go (`release_child`, about 988); a cancelled request's `cancel_requested` is cleared on its next call (about 584). Before a checkpoint exists a new request supersedes an unfinished record; after it, only the same request continues it (Wave 02 Cycle 10 C9; the CLI contract's decision 33). Its digest (`closure/observe.rs` `expectation`, about 1004) does not bind the mode, and `ClosureExpectation` redacts its `Debug` because the digest input holds `closed_by`. `closed_at` comes from `checkpoint.rs` `now_whole_utc_seconds()` (about 70), with no clock seam in execution. `build_closure` (`checkpoint.rs` about 138) reads the ticket from a given commit's tree and commits `Close ticket <ULID>` unreferenced, with author and committer time `closed_at`, so the same inputs give the same OID; `move_context_to_checkpoint_with` (about 314) checks out, then moves the ref, and the direct close reconciles a crash between the two as `ClosingState::CheckoutDone`; `closed_ticket_bytes` (`observe.rs` about 75) writes the fields canonically and refuses a ticket that already holds one. These are visible inside `crate::repository`. `closed_by` is `"<name> <<email>>"` of the configured identity (`observe.rs` about 46; Wave 02 Cycle 10 decision C2, the CLI contract's decision 6). No record, answer or diagnostic holds `closed_by` (Wave 02 Cycle 10 Contract 12; `LifecycleMutationDto`'s documentation, `dto.rs` about 1190; `tests/cli/lifecycle.rs` `application_data_holds_no_closed_by_or_title_beyond_the_index`, about 923).
9. **A contained context today.** For a context contained in the base by ancestry and an open base ticket, the direct close records the closure and merges ("Merged by hand without closure fields: close it now", `closure/observe.rs` about 741). With a publication remote, a context whose remote branch was observed deleted and that is not contained gives `RemoteContextDeleted`, and absent tracking with unknown or unresolved history gives `HistoryUnknown` or `HistoryUnresolved` (about 779-797), read from `remote_context_states` with no network (`promotion/observe.rs` `remote_context_history`, about 891).
10. **The synchronization.** `src/repository/remote/sync.rs` `synchronize_remote_observed` (about 6171, `item sync`'s entry) is public, takes a fresh operation ID and makes its own reservation; it needs no parent record. A context target merges in two stages, the remote copy then the fetched primary (`remote/merge.rs` about 20-31), and publishes; its fetch also fetches the primary (`refs.rs` `context_fetch_refspec`, about 89-96). It answers `Published`, `AlreadyCurrent` or `PublishPending`; or `Transport`, `PushRejected`, `ConflictPending`, `RemoteContextDeleted`. Push refspecs are unforced (`refs.rs` about 99-106); a push the remote refuses as non-fast-forward is `push_rejected`. A synchronization that stops with an error keeps its reservation and its record stays accepted (`mutation/bind/sync.rs` documentation, about 43-47), except `HistoryUnresolved` and `PrimaryHasLocalCommits`, which end it (`sync.rs` about 4243-4270); `RemoteBranchDeleted` and `MergeRequired` are let go only by `request cancel` (`mutation/outcome.rs` `never_gets_past`). Calling it again with the same ID replays a finished operation, answers `RecoveryRequired` for an unfinished one without `restart: true`, and `Interrupted` for a cancelled one, every time (about 8672-8697). A synchronization whose push check finds the local head changed answers `external_change` (about 8869). A context synchronization refuses with `WorktreeConflicted` when the root has a merge, rebase or cherry-pick in progress (about 745-757). `bind/sync.rs` `MERGING` is `pub(super)`; `SynchronizationSettings::default()` has the same `refuse_divergence: false`. In tests, `promotion_test_drive_local` (`sync.rs` about 3920-4040, `cfg(test)`) reserves and runs `integrate_divergence` against a given incoming commit in window 0; it neither pins a window nor publishes, and leaves the operation reserved. Tests that pin a window first exist (`sync_tests.rs` about 4994; `retirement_batch_test_support.rs`).
11. **Remote ref writes.** There is no guarded non-fast-forward replacement of a remote ref. The one guarded remote write besides a push is `transport/remote.rs` `delete_exact` (about 269), used only by close's and promote's cleanup under their parent's reservation. Fetches never prune: a tracking ref stays after its remote branch is deleted (`transport/remote.rs` about 640 and 696-760), and the poll records the deletion in `remote_context_states` as `remotely_deleted` (`remote/state.rs` about 3217). The poll's wildcard refspec gives every clone a tracking ref for every ticket branch (`refs.rs` about 55-60), and a poll does not materialize a branch whose tip the local or tracking primary holds by ancestry (`poll.rs` `primary_holds`, about 789; Decision 28, about 463-469).
12. **Retirement.** `src/repository/cleanup.rs` provides `ContextCleanup` (`acquire`, which takes the repository lease, about 138; `full_check`; `remove_context_worktree`, about 334, which reads a worktree already gone as removed; `delete_context_branch`, about 477; `require_local_cleanup_absent`, about 540; `delete_tracking_ref`, about 572, which requires its caller to hold and check a reservation) and the SQL step `retire_context` (about 953), which deletes the item's `remote_context_states` row and upserts `retired_contexts`. Its one production caller is the direct close's and promote's cleanup (`promotion/lifecycle_record.rs` about 972). A poll's retirement and `context retire` finish in `context_retirement/record.rs` `finalize_index` (about 1966), whose callers are `request_scope.rs` (about 468-488) and the refresh path (`repository.rs` about 7662); `delete_context_rows` is a general helper also called outside retirement. After either retirement, a push of a branch with the same name is a fresh publication: `fetch_publication_evidence_for` gives `NeverPublished` and `push_absence` skips the retired history (`remote/state.rs` about 2610-2632; `sync.rs` about 7963-8052). A context synchronization with no tracking ref and `ObservedPublished` history answers `RemoteContextDeleted` (`refs.rs` about 262-268).
13. **Making a branch.** `src/repository.rs` `prepare_context_unlocked` (about 2432) creates `manyhands/<kind>/<id>` only at the local primary's HEAD, and requires the root on the primary and the item on the local primary (about 2459-2530). `remote/poll.rs` `materialize_context_unlocked` (about 322) creates one at a given commit, but is private to the poll, needs a poll owner row and refuses a commit the primary holds. `mark_contexts_dirty` (`repository.rs` about 12915) marks contexts for the next refresh.
14. **The content proof.** `src/repository/context_retirement/proof.rs` `preservation` (about 24) returns `TicketOpen` before any comparison when the primary's ticket is open (about 37). Its Folded loop (about 63-81) passes when every path the branch changed since the one merge base equals the primary's tip, except that the ticket's own file may be the open copy of the primary's closed one (`older_open_ticket`). It keeps no list. Its tests are in `context_retirement/tests.rs`.
15. **Requests, cancellation and operations.** A request record holds one operation (`mutation/bind/mod.rs` `Accepted::of_record`, about 529). `comment add` allocates its child synchronization's ID in a table of its own (`src/repository/recovery.rs` `comment_publication_bindings`, about 47-62). `LifecycleBinding<Close>` (`bind/lifecycle.rs` about 99) binds `ticket close`. `request cancel` and the cancel loop reach a lifecycle through `progress.rs` `asks_a_parent` and `cancel_parent` (about 631-657), which send every `ticket close` to `cancel_ticket_closure`, reading only `ticket_closures`. `operation list` reads the promotion and closure tables (`read/status.rs` about 584 and 639) and hides only the remote rows whose ID is such a parent's (about 705-737); `recovery.rs` `attribute_remote_row` (about 1154-1199) attributes children only through those tables and the comment bindings; `reservation::abandonable` (about 801-812) excludes only those tables' children from `operation abandon`; `operation resume` (`bind/resume.rs` `found_remote`, about 351-470) refuses only comment synchronizations, and reaches lifecycles through `found_lifecycle` (about 291). `recovery.rs` `LIFECYCLE_COMMANDS` (about 1040) only suppresses the author shown. The CLI retries only `busy`, or a `partial` answer whose first action is the same request, and its blocker guard reads only lifecycle data (`src/bin/manyhands-cli/mutate.rs` about 101-117). A lifecycle's way out is computed from its data (`mutation/way_out.rs` `lifecycle_way_out`, about 114; `blocked()`'s `tracking` set, about 301-306, gives `PollOnce`); recovery action kinds are `RecoveryActionKind` (`results.rs` about 404-430).
16. **Result codes and data.** `src/results.rs` `result_codes!` assigns each code a class; class `None` exits 0, and `Outcome::of_mutation` (about 737) gives `noop` only for `already_applied`, otherwise `success`. Ordered lists compared in order: `src/results_tests.rs` `DESIGN_CODES` (71, `already_applied` last, strings) and the messages, `schemas/v1/envelope.schema.json` and `schemas/v1/request.schema.json`. `mutation/outcome.rs` `recovery` (about 778) matches every code. `MutationDataDto::from_stored` maps `DocumentPromote | TicketClose` to the lifecycle DTO in an unguarded arm (`dto.rs` about 2564) before the synchronization's host-data arm (about 2588), whose data has no tag; `item sync`, `repo sync` and `operation resume` read the tagged conflict data in an earlier arm (about 2544-2551). `authority` names a host only for that host data (about 2441-2479). `item sync` answers with no data when it publishes. A prepare's `already_applied` carries no data (`PrepareDataDto`, `dto.rs` about 888-901).
17. **Preview shapes.** `PreviewEffect` (`dto.rs` about 160-198; 37 values in `schemas/v1/preview_effect.schema.json`) already holds `context_created` (about 184), repair's effect, phrased "create the editing context" (`human_admin.rs` about 190). `PreviewDetailsDto` is untagged (about 248-261). `LifecycleBlocker` has 24 values. The closure blocker mask in `ticket_closures` is full (22 bits). Every schema `enum` is counted by `tests/read_contract.rs` `published_enumerations_list_exactly_the_registered_values` (about 1594-1611); the code, effect, blocker and action lists are registered from `::ALL`.
18. **The CLI's inventories.** `tests/cli/inventory.rs` counts mutating verbs (44), catalogued verbs (81) and verbs taking input (24), lists each verb's output schemas (`DATA`), parses the CLI contract's taxonomy table (`the_catalog_and_the_rfc_taxonomies_name_the_same_verbs`, about 397-413), and `no_verb_is_planned` accepts only the delivery labels `C1` to `C5`. `src/bin/manyhands-cli/catalog_tests.rs`'s `DESIGN` table lists every verb by delivery label (a later verb joined `C5`, as vh-hyksw's did). `tests/support/conformance.rs` `HOMES` (46) gives each bound command a home target, and `run_part` (about 369-395) needs one row per command of each home; a `Remote` home's row is driven by an SSH case of its own (`tests/mutation_remote.rs` `CONFORMANCE`, about 4793); `Expect::Exempt` exists. `tests/mutation_contract.rs` `every_bound_command_has_a_golden` (about 1587) needs a `mutation_v1` golden per bound command, `every_schema_is_reachable_from_a_published_root` (about 1736) and `every_data_root_describes_the_data_of_a_golden` (about 1780) tie schemas to `DATA_ROOTS` (69) and goldens. `tests/cli/skills.rs` `every_mutation_sits_in_one_autonomy_column` (about 459) needs every `--prepare` verb in `AUTONOMY` and in `skills/manyhands-overview/references/autonomy.md`. `tests/support/cli.rs` `CLI_CASES` (about 217) must list every CLI golden. `tests/repository_enablement.rs` (about 581-652) asserts the exact list of registry tables. `tests/cli/recovery_tables.rs` holds `LIFECYCLE` (used by `FAMILIES` and `C5_REQUIRED`) and `c5_codes()` (`verbs.len() == 10`), and `check_golden` requires each recovery-table row's golden.
19. **The link builder.** `src/forge.rs` `recognise(remote_url, setting)`, `pull_request_link(forge, source, target) -> PullRequestLink { source, target, forge, url }` and `human_line`. `PullRequestLink` is not `Serialize`, and nothing outside `src/forge.rs` uses it. It needs the remote's URL. The SSH harness's URL (`ssh://fixture@127.0.0.1:<port>/fixture.git`) gives no URL without a `[forge]` template.
20. **Test seams.** The transport is SSH only; no library test can push or fetch through Manyhands without the SSH harness. Close's library tests use an unreachable SSH URL and hand-written tracking refs (`closure/test_support.rs`) and inject a publisher through `promotion/run.rs` `PromotionIo`; `promotion/run_tests.rs` `FakeIo` reserves through `reserve_promotion_child`, which needs a `ChildExecution`. `context_retirement/acceptance_tests.rs` (about 313-360) uses a local bare repository driven by plain `git2` and feeds the poll with `poll::synthetic_fetch`. A poll classifies a deleted remote ticket branch as `Closed` when `proof::completion` passes on the primary (`poll.rs` about 279-304). Close's exit tests re-run the test binary at a failure point (`closure/run_tests.rs` `exit_at`).
21. **Migrations.** `src/repository/discovery.rs` `migrate_registry` ends with `migrate_pull_request_mode` (about 2809), which adds its schema by checking it and records the marker `wave_06_pull_request_mode_v1`. `remote_operation_records.action` is CHECKed to `poll`, `synchronize_context`, `synchronize_primary`, `promote`, `close` (`remote/state.rs` about 1122); a new value needs a table rebuild.

## Part A: The Closing State

**The values.** `ClosureState` gains `Closing => "closing"`, after `Closed`. `DependencyState` gains `Closing => "closing"`, after `Unresolved`; `parent.state` shares it. `ReadinessReasonCode` gains `ClosingDependency => "closing_dependency"`, after `DependencyCycle` (R-8). The schema enums list the values in that order (`item.schema.json` `closure.state`, `parent.state`; `dependency.schema.json`; `dependency_tree_node.schema.json`; `readiness_reason.schema.json`), and the documentation that names the two states is updated (`ClosureDto`, `ReadinessState`, `DependencyDto`, `ReadinessDto`, `items.rs` about 42, `graph.rs` about 56 and 293, `item.schema.json` about 99). `ClosureDto` gains `publication` (Part C).

**The derivation (R-6, R-7).** For each ticket a read computes one fact from the stored rows it holds, after `graph.rs`'s moved-context substitution: the closure (both fields) of its primary's copy (rank `PRIMARY_COPY`), or none. Then, with "the copy read" being the effective copy (lists, `show`) or the file `show --path` read:

| The copy read | Primary's copy | State | `closed_at`, `closed_by` shown |
| --- | --- | --- | --- |
| no `closed_at` | holds a closure | `closed` | the primary's copy's |
| no `closed_at` | open, or absent | `open` | none |
| holds a closure, and is the primary's copy | — | `closed` | its own |
| holds a closure (an active context's copy) | holds a closure | `closed` | the primary's copy's |
| holds a closure | open, or absent | `closing` | its own |

So `ClosureDto`'s rule holds: the fields are set exactly when the state is `closed` or `closing`. The fact is passed, per ID, wherever the state is needed: `closure()` for lists, `show` and `show --path`; `Targets::of`, `Targets::stored_node` and `Related::of` for `deps[].state` and `parent.state`; `TicketFilter::matches`; and `TicketGraph`, for readiness, tree lines and the deps root. The derivation does not read the mode (decision 11).

**Readiness (decision 9).** `TicketNode` keeps `closed`, now true only for `closed`, and gains `closing`. Every readiness decision reads `closed`, so a closing ticket counts as open for its dependents: an edge to a closing ticket gives `closing_dependency` in place of `open_dependency`, and tree lines and the deps root report `closing`. The queries leave closing tickets themselves out: `ticket ready`, `ticket blocked` and `ticket list --ready`/`--blocked` never list a closing ticket; `ticket plan` computes its batches with closing tickets as open nodes, then removes closing tickets from its batches and from `unplannable`, drops empty batches, and numbers the batches again from 1 after any filter (R-9); `ticket critical-path` gives closing tickets no length and leaves them out of its chain. A closing ticket's own `readiness` field is computed as an open ticket's. `tickets_made_ready` is unchanged.

**The filter.** `ClosureFilter` gains `Closing`. `Open` matches `open` and `closing`; `Closing` only `closing`; `Closed` only `closed`. The CLI's value parser takes `open|closed|closing|all`, `dispatch.rs` maps `closing` explicitly, and the help text says so.

**Human output.** `item_line`, `item_show` and `dependency_line` print the state's `as_str()`, so they print `closing` unchanged. `item_show` adds `Publication: pending` or `Publication: published` when `closure.publication` is set, and the link's `human_line()` when the item has one.

**The desktop (R-22).** Compile-level arms only: `ClosureState::Closing` maps to `Lifecycle::Open`, `DependencyState::Closing` as `Open`, `ClosingDependency` as `OpenDependency`, and `ClosureFilter::Closing` gets an arm the desktop never selects. Cycle 06 designs the desktop's closing state.

## Part B: The Guards, The Save Check And Immutability

**The domain guard (R-10).** `require_ticket_open` gains the repository's integration: in a pull-request repository it reads only the local primary's tip; in direct mode it reads both tips, as today. `refuse_closed_ticket`, `bind/republish.rs` and `bind/resolve_history.rs` pass the mode of the configuration they hold; `context_release.rs` passes `Direct` (a closing ticket has a commit of its own, so it cannot be released anyway). So in a pull-request repository a closing ticket accepts `ticket save`, slug assignment, context prepare, `comment add`, `item republish` and `item resolve-history`, and a ticket whose local primary holds a closure refuses them with `ticket_closed`. Commits made with Git in the ticket's worktree are not Manyhands writes and meet no guard (RFC). An unfinished runner row holds the ticket against these writes (Part D, R-17).

**The binding precheck.** `refuse_closed` keeps comparing with `ClosureState::Closed`, so it lets a closing ticket through in both modes, and the domain guard decides. The other `== ClosureState::Closed` checks (Baseline Fact 5) keep their comparison: drafts and the desktop's edit action treat a closing ticket as open (editable), and the desktop's "Close ticket" stays offered, answering `already_closing` (R-22).

**The save check (R-11).** A ticket save, in both modes, refuses with `invalid_input` when the resulting file's `closed_at` or `closed_by` differ from the ticket at the context branch's head (or from an open ticket, for a context with no ticket yet). It runs in the domain save under the lease, before the checkpoint. Without it a hand edit of the worktree file could record or change a closure, because the precheck no longer refuses a file that reads `closing`.

**Immutability in a ticket context (R-12).** `preserves_item_invariants` gains an optional parsed primary-side copy of the same path. When none is given, the rule is today's. When one is given, a ticket's closure in the result must be:

- the primary side's closure, when the primary side holds one;
- otherwise none, the local side's closure or the incoming side's closure (not the base's alone).

So a closure the primary side holds stays immutable and cannot be dropped, and a provisional closure may be dropped (a withdraw) or replaced (Part G), but never resurrected from the base. The primary-side copy is given only in a remote synchronization of a ticket context with a pinned window: in its Context stage it is the window's `primary_oid`, and in its Primary stage the stage's incoming commit (the same fetched primary). A primary-target synchronization, a local synchronization, and window 0 give none. `validates_resolution_sides` and `validates_external_integration` take the optional primary-side commit and pass its copy; `remote/conflict_read.rs` reads the window's `primary_oid` from the `Recorded` step's window number. `classify_conflict_entry` is unchanged, so a conflict holding two different closures stays external repair only (Baseline Fact 7).

**The primary's closure wins in a ticket context (R-30).** In a remote synchronization of a ticket context, when the stage's fetched primary (`DivergenceInputs`) holds a closure of the ticket and a clean merge result's ticket does not hold exactly that closure, the result's closure fields are set to the primary's, canonically, before the result is committed (in `integration/prepare.rs`, with Part G). So no clean merge on a ticket's branch drops or replaces a closure the primary holds: a withdraw that races the forge's merge, or a branch that kept a withdrawn closure, gets the primary's closure back.

## Part C: The Link Record, The Pull Request Object And The Item's Fields

**The object (decision 5).** `pull_request: { source, target, forge, url }`: `source` the ticket's branch, `target` the configured primary, `forge` a forge kind (`bitbucket-cloud`, `bitbucket-data-center`, `github`, `gitlab`) or null, `url` a string or null. Its schema is `schemas/v1/pull_request.schema.json` and its DTO `PullRequestDto`, both Cycle 02's (vh-c97se). Task 0 checks `origin/main`: if they are there this Cycle uses them; if not, this Cycle adds them in exactly that shape and under those names, and whichever Cycle lands second merges (R-26). `forge` is a schema enum registered in `contract_enumerations`.

**Building it (R-25).** From the publication remote's URL as `git2` reports it, the configuration's `forge()`, the branch `manyhands/ticket/<ULID>` and the primary, through `forge::recognise` and `forge::pull_request_link`. A missing URL never fails a step.

**The record (R-14).** A registry table, keyed per item, holding what this clone gave:

```sql
CREATE TABLE pull_request_links (
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('ticket','document')),
    item_ulid TEXT NOT NULL,
    source TEXT NOT NULL,
    target TEXT NOT NULL,
    forge TEXT,
    url TEXT,
    given_at INTEGER NOT NULL,
    PRIMARY KEY(repository_id, kind, item_ulid)
)
```

Every runner answer that carries a link, and every `item sync` answer that carries one (Part H), upserts its row, the runner's in the same transaction that finishes its row. One helper deletes an item's row, and both retirement paths call it in their finishing transaction: `cleanup.rs` `retire_context` (the direct close's and promote's cleanup, and Part F) and `context_retirement/record.rs` `finalize_index`'s branch that retires (a poll's retirement, `context retire` and the refresh path). A withdraw keeps it, because the pull request may stay open. The table and Part D's `pull_request_closures` are created by one named, additive migration, marker `wave_06_pull_request_close_v1`, run after `migrate_pull_request_mode`, adding its tables by checking the schema, in an immediate transaction with a read-only precheck.

**On the item (R-15).** `ItemDto` gains `pull_request`, after `readiness`: the record's link in `ticket show` and `ticket show --path` for a ticket that has one; null in every list, for a document (Cycle 04 fills documents), and for a ticket without a record. `ClosureDto` gains `publication`, filled by `show` and `show --path` for a closing ticket in a repository with a valid configuration and a publication remote: `published` when the remote tracking ref of `manyhands/ticket/<ULID>` holds the commit that brought the closure into the branch, `pending` otherwise (including with no tracking ref); null for any other state, without a publication remote, with an invalid configuration, and in every list (RFC: "shows in `ticket show` as closing, with publication pending"). Both fields are required in `item.schema.json`, and null in a list means "not read here".

## Part D: The Pull-Request Close

**The hand-off (R-1, R-2).** `ticket close` keeps its verb. Its binding becomes a wrapper. At prepare it reads the configuration and prepares the runner in a pull-request repository, or today's direct close otherwise. At confirm it reads the configuration again and dispatches by it: a direct prepare confirmed in a pull-request repository, or a runner prepare confirmed in a direct one, is `external_change` with nothing written (the direct path's confirm gains this check, since its digest does not bind the mode). At re-entry it finds the request's own record in `pull_request_closures` or `ticket_closures` and continues that one. The runner's digest has its own domain, `manyhands pull request closure v1`. Each path's insert checks the other table's unfinished rows on the ticket in the same immediate transaction. The runner lives in `src/repository/closure/pull_request.rs` with its record. It shares `closed_ticket_bytes`, `build_closure`'s commit building, `move_context_to_checkpoint_with` and its `CheckoutDone` reconciliation, and follows the direct close's lifecycle rules (Baseline Fact 8) through its own record; it does not use `ticket_closures` or `ChildExecution` (R-1, R-3). Its expectation type redacts its `Debug`, as `ClosureExpectation` does, and its entry takes an observer and a cancellation probe, as `close_ticket_observed` does.

**The record.** `pull_request_closures`, one row per confirmed request's operation:

| Column | Meaning |
| --- | --- |
| `operation_ulid` | primary key: the request's operation |
| `repository_id`, `item_ulid` | the ticket |
| `action` | `record`, `withdraw` or `new_branch` |
| `expectation` | the consented digest |
| `first_attempt`, `first_sync_ulid` | the first synchronization's attempt number and current operation (null for `new_branch`) |
| `publish_attempt`, `publish_sync_ulid` | the publishing synchronization's attempt number and current operation |
| `retire_ulid` | the reservation of Part F's step 2 (null otherwise) |
| `cancel_requested` | set by `request cancel`; cleared by the request's next call |
| `step` | unfinished: `confirmed`, `synchronized`, `worktree_removing`, `worktree_removed`, `branch_deleted`, `retired`, `recorded`, `branched`; finished: `published`, `already_closing`, `already_applied`, `refused`, `remote_deleted`, `superseded` |
| `parent_oid` | the head the commit was built on |
| `closed_at` | the frozen time, whole UTC seconds, written before the commit (as C6) |
| `checkpoint_oid` | the closure or withdraw commit, written before any ref moves |
| `retired_oid` | the old context's tip `T` (Part F) |
| `answer` | for a finished row: the code, the closure's recording and the publication, from which the answer is rebuilt |

It never holds `closed_by`, a title, a body or a URL (Baseline Fact 8). A unique partial index on unfinished rows per `(repository_id, item_ulid)` lets one unfinished row hold the ticket.

**Children, attempts, holds and cancellation (decision 8, R-3, R-17).** As the direct close:

- On each call of the same request, a child synchronization whose operation completed is replayed; one that holds a retained conflict or a resolution to publish is restarted under its ID (`restart: true`); one that was cancelled, interrupted, ended or abandoned with nothing to lose is let go and replaced by a new attempt with a new ID (the attempt column plus one). A row never finishes while one of its children is unfinished: the child is let go first, or the row stays unfinished.
- An unfinished row holds its ticket. A poll does not materialize, fast-forward or retire the ticket's context; `item sync` of the ticket, `ticket save`, slug assignment, context prepare, `comment add` on the ticket and `context release` answer `recovery_required` naming the holder's `request.retry`. The runner's children are attributed to its row: `operation list` does not list them on their own, `recovery.rs` names the runner's request as their holder, `operation abandon` refuses them, and `operation resume` of the row or of a child answers `original_request_required` naming the request.
- A new request on the ticket supersedes an unfinished row whose step is `confirmed` or `synchronized` and whose children hold nothing (each finished, or let go): the old row ends `superseded`, and the new request goes on. After `recorded`, or once Part F's retirement has begun, only the same request continues the row, and a new request's prepare answers `recovery_required` naming it (`request.cancel` first when a child holds the remote).
- `request cancel` sets `cancel_requested` and cancels a running child at a safe point; Ctrl-C answers `cancelled` (exit 130) at a safe point. The row stays unfinished; the same request continues it with new child attempts, or a new request supersedes it as above.

**Prepare** reads local state only, with no network, lease or write. The ticket's situation is decided in this order:

1. the local or the tracking primary's ticket holds a closure: blocked `ticket_closed` (blocker `already_closed`);
2. a context exists and its remote branch was observed deleted (`remote_context_states` `remotely_deleted`): Part F's `new_branch`, keeping a provisional closure the context's ticket holds (R-21);
3. no context exists:
   1. its remote branch was observed deleted: Part F's `new_branch`, retiring the remote history;
   2. a remote tracking ref of the ticket's branch exists and its tip is an ancestor of the tracking primary: Part F's `new_branch` without retirement; publishing its commit, a descendant of the tracking primary, fast-forwards the surviving branch (decision 6);
   3. a remote tracking ref exists otherwise: blocked with the new blocker `remote_context_unmaterialized`, whose way out is `PollOnce` (the poll materializes such a branch);
   4. no tracking ref: Part F's `new_branch`;
4. the context's ticket holds a closure: `record` with closure `existing`;
5. otherwise: `record`.

Then the blockers: not a ticket; an unfinished runner row or direct close on the ticket that this request cannot supersede (`recovery_required`, blocker `local_operation_pending`, naming the holder's `request.retry`); a pending comment publication on the ticket (as C12); the context nonconforming, dirty or conflicted; a synchronization conflict pending; a merge, rebase or cherry-pick in progress at the root (`WorktreeConflicted`, Baseline Fact 10); `history_unknown` and `history_unresolved` as the direct close reports them; no identity (`identity_required`). There is no blocker for the root's branch or cleanliness otherwise, because nothing touches the primary. The direct close's prepare also blocks on an unfinished runner row.

The digest binds the domain, the action, the item, the context branch, the primary branch, the remote, the context's head (or none), the closure effect (`record` with the `closed_by` text, or `existing` with both fields), and for `new_branch` the tracking primary, the old context's tip, a remote tracking ref's tip and the unheld entries (RFC: "the mode, the item, the branch, the remote, the target primary and the `closed_by` text"). It does not bind a `record` closure's `closed_at` (C6). The preview's details are a new `PreviewDetailsDto` variant, tagged `"type": "pull_request_closure_preview"`, holding the action, the item and its slug, the branches, the remote, the context and tracking-primary OIDs, the closure effect, the retired context and the unheld entries (Part F), and the `pull_request` object. The effects:

| Action | Effects |
| --- | --- |
| `record` | `context_synchronized`, `closure_recorded`, `context_published`, `pull_request_link` |
| `record`, closure `existing` | `context_synchronized`, `pull_request_link` (the synchronization publishes anything pending) |
| `withdraw` | `context_synchronized`, `closure_withdrawn`, `context_published` |
| `new_branch` | `worktree_removed` and `branch_deleted` (for an old context), `context_created`, `closure_recorded` (absent when kept), `context_published`, `pull_request_link` |

**Confirm, `record` action.** The plan builds this test-first against the tables below, with a design checkpoint.

1. Re-observe, compare the digest, and check the effective identity's `closed_by` against the consented text (`external_change` on any difference; nothing written). Insert the row at `confirmed` with the first attempts of both children (or supersede, or refuse, as above).
2. Run the first synchronization: the ticket context's synchronization through `synchronize_remote_observed` with `SynchronizationSettings::default()`, under the attempt rule. It merges the remote copy and the fetched primary and publishes. Step `synchronized`.
3. Under the repository lease:
   1. reconcile a crash inside the move (the worktree's `ticket.md` holds `checkpoint_oid`'s blob and the ref is at `parent_oid`: finish the move);
   2. require the context clean and at its branch's head; otherwise `recovery_required`, the row unfinished;
   3. the row is at `recorded`: if `checkpoint_oid` is the head or in its history, go to step 4; if the head is `parent_oid`, move the context to `checkpoint_oid` and go to step 4; otherwise (a commit made with Git moved the head) rebuild only when the head's ticket holds no closure and the effective identity's `closed_by` is the consented text, on the head with the stored `closed_at`, storing the new `parent_oid` and `checkpoint_oid`, then move; any other head goes to 3.4;
   4. the tracking primary's ticket holds a closure: the row finishes `refused` (code `ticket_closed`, blocker `already_closed`);
   5. the head's ticket holds a closure: for closure `existing`, if it is the consented one, the row finishes `already_closing`; any other head (none, or another closure) finishes `refused` with `external_change`. For `record`, the row finishes `already_closing`;
   6. otherwise: check the identity again (`external_change`, finished `refused`); take `closed_at` from the injected clock (R-23); build the closure commit on the head with `closed_ticket_bytes`, unreferenced; store `parent_oid`, `closed_at` and `checkpoint_oid` (step `recorded`); move the context to it with `move_context_to_checkpoint_with`; mark the context dirty and refresh the index outside the lease.
4. Run the publishing synchronization under the attempt rule.
5. Decide the answer, then finish the row, store its answer and upsert the link in one transaction, and refresh the index outside the lease. Then answer from the row.

**The answer after step 4.** In this order:

| After step 4 | Answer |
| --- | --- |
| the tracking primary holds this request's closure (the same as `checkpoint_oid`'s ticket) | `ok` (the forge merged it already) |
| the tracking primary holds another closure | `ticket_closed`, finished `refused` |
| the head holds this request's closure | `ok`, finished `published` |
| the head holds another closure | `already_closing` (Part G kept the published one), finished `already_closing` |
| the head holds no closure (another clone's withdraw of this closure merged in) | `ok`, recording `recorded`, finished `published`: this request's closure was recorded and published, and the withdraw is not undone |

**What a stop answers (R-5).** A step that stops on its synchronization answers as `item sync` answers that stop: the same code and data (host data, the tagged conflict data, or none), with the effects adding `write` and `checkpoint` `committed` once step 3 has moved the ref, and with any recovery action that names the child (`operation.resume`, a conflict's resume) rewritten to the runner's `request.retry`. The row stays unfinished. A `remote_branch_deleted` stop is handled by the attempt rule's "let go" only when nothing would be lost: at step 2 its recovery is `request.cancel`, after which a new request supersedes the row; at step 4 (the closure committed locally, the branch deleted on the remote) the runner lets the child go and finishes the row `remote_deleted`, answering `remote_branch_deleted` with the way out `ticket close` again, whose prepare takes situation 2 and keeps the closure (R-21).

### Required Outcomes Of Part D

| # | Situation | Required outcome |
| --- | --- | --- |
| D1 | New request, context ticket open, primary open | `ok`; one `Close ticket <ULID>` commit on the branch holding `closed_at` and `closed_by`; the branch published; `pull_request` in data; the row `published`; nothing held after; the local primary unchanged; no synchronization targets the primary |
| D2 | The branch holds a closure at prepare (its own earlier one, or another clone's) | preview `existing`; step 2 publishes anything pending; `already_closing`, exit 0, the link; no closure commit |
| D3 | Step 2 brings in another clone's published closure | `already_closing`; no closure commit |
| D4 | After step 2 the tracking primary holds the closure | `ticket_closed`, blocker `already_closed`; no closure commit; row `refused` |
| D5 | Step 2 stops: transport, key, host, `push_rejected`, `busy`, `poll_yielding` | as `item sync`'s stop; nothing written by the close; the row `confirmed`; the ticket held; the same request continues under the attempt rule |
| D6 | Step 2 answers `remote_branch_deleted` | that code; recovery `request.cancel`; nothing written; after the cancel a new request supersedes the row, and its prepare offers Part F once the deletion is recorded |
| D7 | Step 2 meets a conflict | `recovery_required` with the conflict data; nothing written by the close; the row `confirmed`; the child retained; the same request continues after the resolution |
| D8 | Step 4 stops: transport, key, host, `busy`, `poll_yielding` | as `item sync`'s stop, with `write` and `checkpoint` `committed`; the row `recorded`; the ticket reads `closing`, `publication` `pending`; held; the same request continues |
| D9 | Step 4 answers `push_rejected` because another clone published first | as D8; the next call fetches, Part G keeps the published closure, publishes; `already_closing` |
| D10 | Step 4 meets a conflict | as D8, with the conflict data |
| D11 | Step 4 answers `remote_branch_deleted` | the child let go; the row `remote_deleted`; `remote_branch_deleted`; the closure stays on the local branch; `ticket close` again takes Part F and keeps the closure |
| D12 | A crash after `checkpoint_oid` is stored and before the checkout; then the same request | the move is made to the stored commit; no second commit; the same `closed_at`; `ok` |
| D13 | A crash between the checkout and the ref move; then the same request | reconciled (3.1); no second commit; `ok` |
| D14 | A crash after the ref moved and before the row finished, including after the publishing child completed; then the same request | no second commit; step 4 under the attempt rule; the answer table; `ok` |
| D15 | A commit made with Git moved the head before the retry of D12 | rebuilt on the head with the stored `closed_at` when its ticket holds no closure and the identity matches; otherwise 3.4 and 3.5 decide |
| D16 | A new request while a row is unfinished | at `confirmed` or `synchronized` with nothing held by its children: the new request supersedes it; otherwise prepare `recovery_required` naming the holder's `request.retry`; a confirm whose insert loses the race answers the same; nothing written |
| D17 | `request cancel` or Ctrl-C during the close | `cancelled`, exit 130; the row unfinished, `cancel_requested` set; the same request continues with new child attempts, or a new request supersedes per D16 |
| D18 | A save, comment, `item sync`, slug, context prepare or poll of the ticket while a row is unfinished | `recovery_required` naming the request; the poll leaves the context alone |
| D19 | Prepared in one mode, confirmed after a switch (either way) | `external_change`; nothing written; a direct close never runs in a pull-request repository from a pull-request-mode confirm |
| D20 | The identity changed between prepare and confirm | `external_change` at step 1; nothing written |
| D21 | Replay of a finished request, including after a crash between finishing the row and storing the envelope | the stored envelope, or the answer rebuilt from the row |
| D22 | `operation list`, `operation resume`, `operation abandon` while a row is unfinished | the row listed with the request that continues it; its children not listed on their own; resume of the row or a child: `original_request_required`; abandon of a child refused |
| D23 | The prepared closure was `existing`, and before step 3 another clone's withdraw or other closure arrived | `external_change`, finished `refused`; no closure commit |

**A merged branch whose remote branch survives (decision 6).** This is the `record` action on a context. When the primary holds the branch's work by ancestry, step 2's merge of the fetched primary is a fast-forward; when every path the branch changed since its one merge base equals the primary's tip, the merge's result is the primary's tree path by path. Either way the closure commit is then the only change against the primary. Otherwise, or when libgit2's merge reports a conflict (for example on a rename), D7 applies. With no local context, situation 3.2 reaches the same result through Part F.

## Part E: Withdraw

`ticket withdraw-closure --id <ULID>`: a confirmed verb, `--prepare` and `--confirm` as close, no input, the same runner with the action `withdraw`, in either mode (decision 11). Its digest has the runner's domain and binds the mode. It follows Part D's rules for children, holds, supersede and cancellation.

**Prepare.** Part D's blockers, except situations 2 and 3. A ticket whose context ticket holds no closure answers `already_applied` with nothing to confirm and no data (decision 11; Baseline Fact 16). Otherwise the digest binds the action, the item, the branch, the remote and the closure being withdrawn (both fields).

**Confirm.** Steps 1, 2 and 4 of Part D, and step 5 with this action's answers. Step 3, under the lease: 3.1 and 3.2 of Part D; at `recorded`, as Part D's 3.3, rebuilding only when the head's ticket holds the consented closure; the tracking primary's ticket holds a closure: finished `refused` with `ticket_closed`, blocker `already_closed` (withdrawing it would be a reopen); the head's ticket holds no closure: finished `already_applied`; it holds another closure than the consented one: finished `refused` with `external_change`; otherwise the commit `Withdraw closure of ticket <ULID>`, whose ticket is the head's with both fields removed and serialized canonically (a new byte helper beside `closed_ticket_bytes`), built unreferenced, recorded, then the ref moved. After step 4: if the head's ticket holds the tracking primary's closure (R-30 restored it), the answer is `ticket_closed`, finished `refused`; otherwise `ok`. The answer gives the link record's link, or null when there is none; the record is kept, because the pull request may stay open.

| # | Situation | Required outcome |
| --- | --- | --- |
| W1 | A closing ticket, in a pull-request repository and in a direct one | `ok`; one withdraw commit; the branch published; the ticket reads `open`; its dependents read blocked by `open_dependency`; the link record kept |
| W2 | No closure on the branch | `already_applied` at prepare, or at step 3 after another clone's withdraw arrived; nothing written |
| W3 | The tracking primary holds the closure at step 3 | `ticket_closed`; nothing written |
| W4 | An unfinished direct close or runner row holds the ticket | prepare `recovery_required`, naming the holder (or supersedes it, per D16) |
| W5 | Step 2 or step 4 stops | as D5 to D10; a `remote_branch_deleted` at step 4 finishes `remote_deleted` with the withdraw kept locally |
| W6 | Another clone's synchronization merges a withdraw into its unchanged closure | the clean merge keeps the withdraw; a conflicting merge accepts an external repair that keeps it (Part B) |
| W7 | After W1, `ticket close` again | Part D's `record`, a new `closed_at` |
| W8 | A crash after the ref moved; then the same request | no second commit; `ok` |
| W9 | The forge merges the closure between step 2 and step 4 | step 4's Primary stage restores the primary's closure (R-30); `ticket_closed` |
| W10 | `request cancel` or Ctrl-C | as D17 |
| W11 | Another closure than the consented one is on the head at step 3 | `external_change`; nothing written |

## Part F: The New Branch

Used in a pull-request repository by close for a ticket whose context's remote branch was observed deleted (decision 1, narrowed by decision 6), and for a ticket with no context (decision 7, Part D's situation 3). It follows Part D's rules for children, holds, supersede and cancellation, except that once step 2 has begun only the same request continues the row.

**Prepare.** Besides Part D's blockers:

- the tracking primary must hold the ticket, open; otherwise an old context's case keeps the blocker `remote_context_deleted` (the way out is `item republish`) and a no-context case answers `context_missing`;
- an old context must be clean, unconflicted and at its branch's head, read as the existing blockers read it (the cleanup's full check runs at confirm).

The preview names the tracking primary `P` the new branch starts from, the old context's tip `T` when there is one, and the **unheld entries**: each path whose entry in `T` differs from its entry in the one merge base of `T` and `P`, and also differs from its entry in `P`, with its change against `P` (`added`, `modified`, `deleted`), compared per path without rename detection, unbounded and sorted by path (R-19). An empty list means the content proof passes. History with no merge base or more than one is `recovery_required`. A new function in `proof.rs` computes the list from the existing private `entries`; it treats the ticket file like any other path. A provisional closure the old context's ticket holds is kept (closure `existing`, R-21), and the digest binds it.

**Confirm.**

1. Re-observe, compare and check the identity as Part D step 1 (`external_change` when `P`, `T`, a tracking ref's tip or the unheld entries changed). Insert the row with `retired_oid` `T`, `closed_at` from the injected clock (or the kept closure's), and the first publishing attempt.
2. With an old context, or published remote history (situation 2 or 3.1): take the remote reservation for the ticket's context target under `retire_ulid` (a reservation the runner makes and completes itself, never run as a synchronization, attributed to its row as the children are; R-31), so that no poll or synchronization acts on the item meanwhile. Then, as the direct close's cleanup does them: the full check (a refusal finishes the row `refused` with the cleanup's code, nothing changed); step `worktree_removing`; remove the worktree (a worktree already gone reads as removed); step `worktree_removed`; delete the local branch; step `branch_deleted`; check local absence; delete the tracking ref if one exists; and in one transaction `retire_context`, the link deletion, step `retired` and the reservation's completion. Nothing on the remote changes.
3. Under the repository lease, by the state of `manyhands/ticket/<ULID>`:
   - absent and the row before `recorded`: build the closure commit `C` on `P` (the ticket from `P`'s tree with the closure's fields: the kept closure, re-read from `T`'s tree, or the stored `closed_at` with the consented `closed_by`), store `checkpoint_oid` (step `recorded`);
   - absent and the row at `recorded`: use the stored `checkpoint_oid` (the same tree, parent and time give the same OID);
   - then create the branch at `checkpoint_oid` (step `branched`), add its worktree at `.manyhands/worktrees/<ULID>` if missing, mark the contexts dirty and refresh the index outside the lease;
   - present at `checkpoint_oid` (a retry): add the worktree if missing, then step 4;
   - present elsewhere: an unfinished row holds the ticket, so no Manyhands write made it; `recovery_required`, the row unfinished.

   The function that creates a ticket's context branch at a given commit and adds its worktree is new, beside `prepare_context_unlocked`, without that function's checks that the root is on the primary and the item on the local primary (Baseline Fact 13).
4. Publish under the attempt rule: a fresh publication after a retirement (Baseline Fact 12), or a fast-forward of a surviving branch (situation 3.2).
5. As Part D's step 5, with Part D's answer table.

| # | Situation | Required outcome |
| --- | --- | --- |
| F1 | Old context, remote branch observed deleted, content proven merged | preview with no unheld entries; confirm: the old worktree and branch gone, a branch whose only commit over `P` is the closure, published, `ok` with the link; nothing deleted on the remote |
| F2 | As F1, content unproven (a squash after which the primary moved on, or a declined pull request whose branch was deleted) | the preview lists the unheld entries; after confirm, as F1; the old branch's commits are no longer referenced by a branch (K-6) |
| F3 | No context, ticket open on the tracking primary, never published, no tracking ref | no retirement; as F1 otherwise |
| F4 | No context, remote history observed deleted, its tracking ref still present | the history and the stale tracking ref retired in step 2; as F3 |
| F5 | No context, a remote ticket branch exists that the tracking primary does not hold | blocked `remote_context_unmaterialized`, way out `PollOnce` |
| F6 | No context, a surviving remote branch the tracking primary holds by ancestry | no retirement; the publication fast-forwards the surviving branch; `ok` |
| F7 | The tracking primary lacks the ticket | `remote_context_deleted` (old context) or `context_missing` (none) |
| F8 | `P`, `T`, a tracking tip or the old worktree changed between prepare and confirm | `external_change`, or the cleanup's refusal; nothing changed; the row finished |
| F9 | The old context held a provisional closure | the new branch keeps its `closed_at` and `closed_by`, re-read from `T` on every attempt |
| F10 | A crash at each step boundary (inside step 2 at each of its steps, after `retired`, after `recorded`, after `branched` before the worktree); then the same request | continues from the stored step; one branch, one closure commit (the same OID), the stored `closed_at` |
| F11 | A new request while the row is unfinished | `recovery_required` naming the holder |
| F12 | Step 4 stops | as D8; the branch is local and reads `closing` (the index was refreshed after step 3) |
| F13 | `request cancel` or Ctrl-C | as D17, without supersede once step 2 has begun |
| F14 | A save or comment on the ticket while the row is at `retired` | `recovery_required` naming the request (the row holds the ticket); no branch made |

## Part G: Two Clones

**Settling a closure-only conflict (decision 2, R-13).** In a remote synchronization of a ticket context, the merge of the remote copy into the local branch (the Context stage) settles a conflict itself when all of these hold:

1. the only conflicted path is the ticket's own `ticket.md`;
2. the local (ours) and remote (theirs) tickets both parse and both hold closures, and the two closures differ;
3. the stage's fetched primary (`DivergenceInputs`) holds no closure of the ticket;
4. with ours' ticket rewritten to hold theirs' closure (canonically, through a new byte helper that replaces the fields of a closed ticket), the three-way merge of that path is clean.

The result is the clean merge of step 4, so the remote branch's closure (the one published first) is kept and the local one dropped. The function lives in `integration/prepare.rs` with R-30's rewrite and returns `MergePreparation::Clean`, so the synchronization commits it as the stage's ordinary clean merge. It is called in `integrate_divergence` where `prepare_clean_merge` returns `Conflict` (Part G) or `Clean` (R-30), before any conflict is installed. Every other conflict is unchanged.

| # | Situation | Required outcome |
| --- | --- | --- |
| G1 | Two different provisional closures, nothing else conflicting | clean; theirs kept; the synchronization publishes; the ticket reads `closing` with theirs' fields |
| G2 | Different closures, and another conflicting hunk in the same file or another path | unchanged: `IncompatibleClosures`, external repair only; a repair that keeps either side's closure passes Part B's validation where the window is pinned |
| G3 | The fetched primary holds one of the closures | unchanged: external repair, which must keep the primary's |
| G4 | The same closure on both sides | unchanged: a clean merge |
| G5 | Base closed, ours withdrawn, theirs unchanged, the primary open | unchanged: a clean merge keeps the withdraw (W6) |
| G6 | Base closed with `A`, ours re-recorded `B` after a withdraw, theirs `A`, the primary open | unchanged: a clean merge keeps `B` |
| G7 | A clean merge (either stage) whose result drops or changes a closure the fetched primary holds | the result's closure set to the primary's (R-30) |

This changes Wave 02 Cycle 10's C11 ("two different closures can never be resolved in-product") for provisional closures only.

## Part H: Codes, Data, CLI And Contract

**`already_closing` (decision 4, R-16).** A new result code, class `None` (exit 0), outcome `success` (it is not added to `noop`, because its step 2 can publish), message "The ticket's branch already holds a closure.", appended after `already_applied` in every ordered list, in `recovery`'s shared arm. Withdraw's `already_applied` after step 2 keeps `noop`; its effects show what step 2 published (R-32).

**The data (decisions 3 and 10).** A new `MutationDataDto` variant, `PullRequestClosureMutationDto`, tagged `"type": "pull_request_closure"` (a `contract_enum`, as Cycle 01's tag), with:

- `action`: `record`, `withdraw` or `new_branch`;
- `closure`: `{ recording, closed_at }`, `recording` one of `recorded`, `existing`, `withdrawn`, `none`; never `closed_by`;
- `checkpoint_oid`: the commit this request made, or null;
- `publication`: `published` or `current`;
- `retired_context`: the old context's tip, or null;
- `pull_request`: the object, or null when there is no link record;
- `blockers`: the `LifecycleBlocker` values of a blocked answer, otherwise empty.

It is the data of the runner's `ok`, `already_closing`, `already_applied` (at step 3) and blocked answers. A stop carries the synchronization's own data (R-5). Its schema is `schemas/v1/pull_request_closure_mutation.schema.json`. `from_stored`: the `TicketClose` lifecycle arm is guarded on its data's lifecycle shape so that other `ticket close` data falls through; a tag-guarded arm reads this data for `ticket close` and `ticket withdraw-closure`; and the two verbs join the arms that read the synchronization's conflict data and, unguarded and last, its host data. `mutation/way_out.rs` computes the runner's way out from this data as `lifecycle_way_out` does from the lifecycle's, and the CLI's retry guard reads its blockers as it reads the lifecycle's (`mutate.rs`).

**`item sync`'s link (decision 12, R-29).** When `item sync` of a ticket context in a pull-request repository publishes, or finds current, a branch whose ticket holds a provisional closure, its answer carries data tagged `"type": "pull_request_link"`, holding `pull_request`, and it upserts the link record. Every other `item sync` answer is unchanged. Its schema is `schemas/v1/pull_request_link.schema.json`, with a tag-guarded `from_stored` arm for `item sync` before the host-data arm. Its human output (`human_collaboration.rs`) adds the link's line.

**Blockers.** `LifecycleBlocker` gains `remote_context_unmaterialized`, appended, and `way_out.rs`'s `tracking` set gains it, so its way out is `PollOnce`; the runner reuses the other values Part D names.

**The verb (R-28).** `Command::TicketWithdrawClosure` (`"ticket withdraw-closure"`), confirmed, with an operation, stopping at safe points, finishing as a synchronization; `Mutation::TicketWithdrawClosure(TicketWithdrawClosureInput { root, id })`; a catalog entry taking `--repo`, `--id`, `--request-id`, `--prepare`, `--confirm` and no input, delivered under the label `C5`; dispatch, parse and human output as `ticket close`; the autonomy column of `ticket close`; its conformance row in the `Confirmation` home (R-24); `progress.rs`'s cancellation reaches it as `ticket close`'s does.

**Operations (R-18).** An unfinished runner row is listed by `operation list` with the new action `pull_request_close` (an `OperationAction` value, in `operation.schema.json` and `blocker.schema.json`, with the desktop's word "Pull request close" in `ACTION_WORDS`), for close and withdraw alike, naming the request that continues it.

**Preview effects.** `PreviewEffect` gains `context_published`, `pull_request_link` and `closure_withdrawn`, appended; `new_branch` reuses `context_created`, whose documentation widens to a context made from either primary. `effect_phrase` and the schema's description cover them. The preview's schema is `schemas/v1/pull_request_closure_preview.schema.json`, reached as `lifecycle_preview.schema.json` is.

**Human output.** The preview prints its effects, the closure (with `closed_by`, as the direct preview does), and for `new_branch` each unheld entry as `not on <primary>: <change> <path>`, through the output's field escaping. The answer prints `closure: recorded <closed_at>`, `closure: already recorded`, `closure: withdrawn` or nothing, then `publication: <state>`, then the link's `human_line()` on its own line.

**The desktop.** Compile-level arms for the new preview (`confirm.rs`), data, blocker (`blocker_words`) and verb, and the action word (R-22).

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | Part A's derivation table in lists, `show` and `show --path`, with the fields shown; `deps[].state`, `parent.state` and tree lines `closing` | `tests/read_items.rs`, one table-driven test, beside `presentation_across_contexts` |
| P2 | A dependent of a closing ticket is blocked with `closing_dependency` and ready once the primary's copy holds the closure; `ticket ready`, `blocked`, `list --ready`/`--blocked`, `plan` and `critical-path` leave a closing ticket out, with batches numbered from 1 and the longest chain kept | `tests/read_relationships.rs`, one table-driven test; a `graph_tests.rs` row for the critical path |
| P3 | `--closure open`, `closing`, `closed`, `all`; the parser, dispatch and help | `tests/read_items.rs`; `parse_tests.rs`; `tests/cli/content.rs`; the help goldens |
| P4 | The guard per mode at every caller, the save check, comments and saves on a closing ticket | `src/repository/ticket_guard_tests.rs`, a table over the callers; `tests/mutation_bridges/context.rs` |
| P5 | Part B's rule: the primary side's closure kept, provisional closures dropped or replaced (never resurrected from the base) in a ticket context's pinned stages, today's rule elsewhere; R-30's rewrite | `src/repository/integration/tests.rs`, a table; one row per validator through `sync_tests.rs`'s existing callers, with a pinned window |
| P6 | The link record: migration once, upsert replaces, deletion by both retirement paths; `ticket show`'s `pull_request` and `closure.publication` | `src/repository/discovery.rs`'s tests; `src/repository/cleanup_tests.rs`; a `finalize_index` test in `context_retirement`; `tests/read_items.rs` |
| P7 | D1 to D23 | `src/repository/closure/pull_request_tests.rs` (D21's binding half and D22 in Task 12), with an injected publisher and clock |
| P8 | W1 to W11 | the same file |
| P9 | F1 to F14, and the unheld entries | the same file; `src/repository/context_retirement/tests.rs` |
| P10 | G1 to G7, and the merged branch of Part D's last paragraph | `src/repository/integration/tests.rs`, the pure function's table; one `integrate_divergence` row in `sync_tests.rs` with a pinned window |
| P11 | The owner's lifecycles, end to end in the library | `pull_request_tests.rs`, with a local bare repository driven by `git2` as the forge, a test publisher that pins a window and runs the real stages, and `synthetic_fetch` for the poll |
| P12 | End to end over SSH: close, close again, `item sync`, withdraw, close again in a pull-request repository; the server's primary unchanged throughout; the goldens | one SSH case in `mutation_remote` |
| P13 | The CLI contract with no network: one preview golden, the withdraw `already_applied` prepare golden (`mutation_v1`), the blocked answers, a confirm stopped at step 2 | `tests/cli/lifecycle.rs`, `tests/mutation_confirmation` |
| P14 | `item sync`'s link | a binding test beside `bind/sync_tests.rs`'s, and the P12 case's `item sync` step |
| P15 | Every registration (codes, enums, schemas, verbs, goldens, inventories, conformance, autonomy) | the existing registration tests |

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). P12 is the only SSH case (decision 11): no non-SSH publication remote can be selected, so only a case over the harness can produce the success goldens, and it shows that the server's primary is never pushed in pull-request mode (the RFC's Required Evidence 1 and 2). Its edge cases are in P7 to P11. No real waits: the runner takes its clock. The injected publisher of P7 to P11 refuses a synchronization whose target is the primary, and every runner test asserts that the local primary did not move and that no synchronization targeted the primary; that is how "a test fails if one tries" holds in the library. Where a merge must happen (D9, W6, W9, P11), the publisher pins a window first, as `sync_tests.rs` (about 4994) does, then runs the stages through `promotion_test_drive_local` or `integrate_divergence` against the local bare repository's branch, writes the bare repository and the tracking refs with `git2`, and completes the row as `FakeIo` does. Goldens: one per output contract (the preview, each data shape) and one per result code the contract tables require.

At review-ready the SSH tier is every SSH-harness target: `ticket_closure` (`src/repository/closure/`), `mutation_remote` (`src/repository/mutation/`), the six targets of `src/repository/remote/` (`ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`), `comment_publication` (the guard in `checkpoint_comment_locally_with`), `document_promotion` (`cleanup.rs`), and `ssh_fixture`, with `--lib repository::local_sync::` for the validators' local callers.

## Named Edits To Pre-Existing Tests

The owner approved E-16 in decision 11; the others are asked at approval.

- **E-1** retired: `tests/read_contract.rs`'s code, effect, blocker and action lists are built from `::ALL`; only the schema files change.
- **E-2** (Task 1) the exhaustive `ClosureState` matches in `tests/ticket_closure/journeys.rs` and `src/desktop/model/tickets_tests.rs`, and the `DependencyState` match in `src/repository/read/graph_tests.rs`: a `Closing` arm that fails the test, as an unexpected state.
- **E-3** (Task 1) `open|closed|all` to `open|closed|closing|all` in `help_ticket_list.json`, `help_resource_ticket.json`, `help_root.json` and `catalog_tests.rs`; `parse_tests.rs`'s valid-value list gains `closing`; `tests/cli/content.rs` gains a `closing` row.
- **E-4** (Task 4) every item golden under `tests/fixtures/read_v1/` and `tests/fixtures/cli_v1/` gains `"pull_request": null`, and every ticket's `closure` gains `"publication": null`, regenerated with `MANYHANDS_UPDATE_GOLDEN=1` for the tests that own them and read before committing.
- **E-5** (Task 5) `src/results_tests.rs`: `already_closing` in `DESIGN_CODES` (71 to 72) and the messages.
- **E-6** (Task 5) `tests/mutation_contract.rs`: `already_closing` in both `CODES_` tables with the reason "golden written in Task 14"; **E-7** (Task 14) removes it.
- **E-8** (Task 10) `tests/support/mutation.rs` (six arms) and `tests/support/repair.rs` (one): the two new variants.
- **E-9** `tests/read_contract.rs` `contract_enumerations`, each in the task that adds its schema: (Task 1) none beyond the schema files; (Task 4) `closure.publication` and, unless Cycle 02 registered it, the `pull_request` `forge`; (Task 11) the enums inside the preview's schema, and (Task 14) those inside the two data schemas (the data's and the link's `type`, the preview's `type`, `action`, `recording`, `publication`), each schema landing with its first golden.
- **E-10** (Task 11) `src/repository/mutation/command.rs` tests that hard-code command sets (about 770, including its lifecycle list, 870 and 1018): the new verb.
- **E-11** (Task 11) `tests/cli/inventory.rs`: mutating 44 to 45, catalogued 81 to 82, `NO_INPUT` and `DATA` rows for the new verb (with the synchronization stop schemas), and `ticket close`'s and `item sync`'s new schemas.
- **E-12** (Task 11) `tests/support/conformance.rs` `HOMES` (46 to 47) and a row in `tests/mutation_confirmation`, whose rules 1 and 2 are exempt with the reason that a finished withdraw needs a server, pinned by P12's same-request replay.
- **E-13** (Task 11) the help listings: `help_root.json`, `help_root_text.txt`, `help_resource_ticket.json`, a new `help_ticket_withdraw_closure.json` in `tests/cli/help.rs`'s list; `catalog_tests.rs`'s `DESIGN` table (the `C5` row); `tests/support/cli.rs` `CLI_CASES` for every new CLI golden.
- **E-14** (Task 2) `src/repository/ticket_guard_tests.rs`: its 5 `require_ticket_open` calls pass `Integration::Direct`; (Task 3) the direct validator calls at `sync_tests.rs` about 4015 and 11480 pass no primary side.
- **E-15** (Task 15) `tests/cli/recovery_tables.rs` (`LIFECYCLE`, `FAMILIES`, `C5_REQUIRED`, `c5_codes()`'s `verbs.len() == 10`, and the withdraw rows' goldens `check_golden` needs), `tests/cli/collaboration_exits.rs` and the audit's C5 counts, when D-6 adds `already_closing` to `ticket close`'s codes and the new verb's row.
- **E-16** (Task 1) **reverses an assertion's intent; approved by the owner in decision 11.** `tests/read_relationships.rs` `ready_and_blocked_are_decided_across_primary_and_two_item_worktrees` (about 290-438; "C waits for A, which is closed where it counts: in A's own context"): A now reads `closing` and is left out of the queries; B and C are blocked by `closing_dependency`, D by `open_dependency` on B; the ready-list lookups, the readiness assertions about A, B and C, the `list --ready`/`--blocked` agreement and the plan's batches (about 325-430) change; then A's closure is committed on the primary too, and B and C read ready as before. `closing_a_dependency_makes_its_dependent_ready_without_the_dependent_being_observed_again` (about 919-984): A is committed on the primary first and its context made with an edit intent, the SQL update closes only A's primary row, and the test keeps its intent (B ready without being observed again); its last part asserts that a closure in A's worktree alone leaves B blocked. Task 0 lists the exact lines.
- **E-17** (Task 1) `src/repository/read/items_tests.rs` `ticket_filters_compare_whole_values_and_closure_reads_only_closed_at` (about 334): its closure rows read the derived state; the `Related::of` calls and the `StoredItem` and `TicketNode` literals in `items_tests.rs` and `graph_tests.rs` (about 19, 28, 35, 400, 404, 654, 679) gain the new field or argument.
- **E-18** (Task 4) `tests/repository_enablement.rs` (about 581-652): the registry tables list gains `pull_request_closures` and `pull_request_links`; the `ItemDto` literals (`src/desktop/model/drafts/create.rs`, `src/drafts/compare_tests.rs`, the desktop's save, editor-text, ticket-view, documents and edit tests, the CLI's `fill_tests.rs` and `human_tests.rs`) gain `pull_request`, and the `ClosureDto` literals gain `publication`.
- **E-19** (Task 11) `tests/cli/skills.rs` `AUTONOMY` and `skills/manyhands-overview/references/autonomy.md`: the new verb in `ticket close`'s column; (Task 12) `src/desktop/model/activity/copy/states.rs` `ACTION_WORDS` (23 to 24) and `activity_tests/states_copy.rs`, with the new `OperationAction`.
- **E-20** (Task 11) `tests/cli/lifecycle.rs` `application_data_holds_no_closed_by_or_title_beyond_the_index` gains a pull-request-mode part: a confirm that stops at `transport_unavailable` (no `ok` is reachable without a server), whose stored record and data hold no `closed_by`. The test's shape changes for that part; its existing assertions do not.
- **E-21** `tests/mutation_contract.rs` `DATA_ROOTS` (69 to 71) in Task 14, with the two data schemas and their goldens; the preview schema is reached from the preview schema's details in Task 11, with its CLI golden.

Task 0's sweep adds what else it finds: tests that read a ticket's state inside an unfinished direct close, or with a closure only in its own context (each such edit reverses an intent, as E-16 does, and is listed for the owner); the direct close's made-ready read (`bind/lifecycle.rs` about 456-499), which now depends on the root's row; JSON assertions of item and closure objects; exhaustive matches on `Command`, `Mutation`, `PreviewDetailsDto`, `PreviewEffect`, `LifecycleBlocker`, `OperationAction`, `ClosureFilter`, `DependencyState`, `ReadinessReasonCode`; calls of `require_ticket_open`, `preserves_item_invariants`, `validates_resolution_sides`, `validates_external_integration`; counted lists of schema files, data roots, operation actions, effects and migration markers; goldens that hold `readiness.reasons`; and plan or critical-path tests that a removed closing ticket renumbers.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `ticket close` row (the pull-request path, its preview, `already_closing`, the new branch, its stops, holds, supersede and cancellation as the existing rules); a taxonomy row for `ticket withdraw-closure`, and its rows in the recovery tables with their goldens; "A cancellation is final … not for `document promote` or `ticket close`" extended to the new verb; the Ctrl-C window list and `request cancel`'s reach for the new verb; "only `document promote` and `ticket close` are confirmed" gains the verb; `operation list`'s, `operation show`'s, `operation resume`'s and `operation abandon`'s answers for a runner row and its children; `closure.state`, `closure.publication`, `deps[].state` and `parent.state` `closing`; `--closure closing`; `closing_dependency` and the queries that leave closing tickets out; the item's `pull_request`; `item sync`'s link data; the new data and preview shapes; the success code `already_closing`; the new blocker and its way out.
- **D-2** `docs/RFC/canonical-content-and-comment-schema.md`: closure immutability applies once the primary holds the closure (RFC decision 20).
- **D-3** `docs/RFC/git-workflow-and-conflict-recovery.md`: the immutability rule in a ticket context's resolutions and external integrations, R-30's rewrite, and the closure-only conflict that settles itself (changing Wave 02 Cycle 10's C11 for provisional closures).
- **D-4** `docs/RFC/ticket-relationships-and-short-codes.md`: closed and closing, the derivation from the primary's copy (amending "the index's effective copy" for closure), readiness, `closing_dependency`, and the queries that leave closing tickets out (with the plan's renumbering and the critical path's rule); `plan.schema.json`'s and the code's documentation of batch numbers.
- **D-5** `docs/RFC/repository-index-persistence-and-refresh.md`: the two new registry tables and their migration.
- **D-6** `docs/research/wave-03-api-audit.md`: `already_closing` in `ticket close`'s codes; the new verb.
- **D-7** The shipped skills, in `skills/` and `.agents/skills/`: `manyhands-tickets` (close in pull-request mode, closing, withdraw, the new branch, a stopped close), `manyhands-finding-work` (the closing state, `--closure closing`, `closing_dependency`, the queries, `pull_request` and `publication` on `ticket show`), `manyhands-overview` (the three closure states; the new verb in `autonomy.md`), `manyhands-sharing-and-sync` (`item sync`'s link), `manyhands-recovery` (a stopped pull-request close is continued by its request, and holds its ticket meanwhile). Each says that promote in a pull-request repository still pushes the primary until Cycle 04.
- **D-8** `docs/RFC/pull-request-mode.md`: dated amendments for decisions 6 (a surviving branch closes in place), 7 (a ticket with no context), 8 (a stopped close holds until its request continues; `item sync` finishes a publication only once nothing holds it), 9 (closing tickets left out of the queries), 10 (`closed_by` not in the answer) and 11 (withdraw of nothing is `already_applied`; one SSH case proves Required Evidence 1 and 2).
- **D-9** `docs/Waves/wave-06-pull-request-workflow.md`, Cycle 03's checklist, through `manyhands-documents` (the Wave document is a managed document): at approval, with the owner's go-ahead for its save and promotion.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** A separate runner and record for the pull-request close, not `ticket_closures` — the direct record's steps, its never-push rule (C1) and its blocker mask are direct mode's — a second record that must keep the direct close's lifecycle rules (Part D) by hand.
- **R-2** The mode is read at prepare and again at confirm, the runner's digest has its own domain, and each path refuses a confirm in the other mode and blocks on the other's unfinished record — Cycle 01 decision 11's rule, and the direct digest does not bind the mode — a switch between prepare and confirm costs a re-prepare.
- **R-3** The runner's synchronizations are ordinary context synchronizations with IDs from its record, replaced per attempt as the direct close's children are — `ChildExecution` needs a direct parent (Baseline Fact 8) — a stopped child holds the remote as any stopped synchronization does (decision 8).
- **R-4** The runner's children are attributed to its row: hidden from `operation list`, refused by `operation abandon`, and `operation resume` answers `original_request_required` — a resume or abandon outside the runner would leave the row behind — the guards in `read/status.rs`, `recovery.rs`, `reservation.rs` and `bind/resume.rs` follow comment add's.
- **R-5** A stop answers as `item sync` answers that stop, with the close's own effects and its recovery rewritten to the runner's request — the host and conflict actions then work unchanged — the stop's data does not say which close step stopped; the effects and the operation list do.
- **R-6** The closing state is derived from the two copies in both modes, and the primary's closure, when its copy holds one, decides `closed` and is shown — the RFC defines it so (decision 11), and reads then need no mode — an interrupted direct close reads `closing` where it read `closed`.
- **R-7** The primary's copy is the row at rank `PRIMARY_COPY`, normally the root's working tree — it is what the index stores — an uncommitted closure in the root reads `closed` while the guard reads the branch tips, and a root checked out on another branch shows that branch's copy (K-9).
- **R-8** A new reason `closing_dependency` — the RFC says `ticket ready` and `ticket blocked` say so — one more enum value for callers.
- **R-9** `ticket plan` computes with closing tickets as open, removes them from batches and `unplannable` and renumbers after any filter; `ticket critical-path` gives them no length — decision 9, and removing a ticket after the critical path's tie-break could lose the longest chain — a dependent of a closing ticket sits in a batch without its blocker shown in the plan.
- **R-10** The guard reads the configuration's mode at each call, and `context_release` stays direct — RFC: in pull-request mode only the primary's copy refuses — a repository switched to direct with a provisional closure on a branch refuses writes to that ticket until a direct close merges it.
- **R-11** A save whose closure fields differ from the branch head's is refused with `invalid_input`, in both modes — the precheck no longer refuses a file that reads `closing` — a person who hand-edits those fields gets `invalid_input`.
- **R-12** Immutability changes only in a ticket context's pinned synchronization stages, against the stage's fetched primary: the primary's closure must be kept, a provisional one may be dropped or replaced by a side's, never resurrected from the base — primary targets and local synchronizations pin today's rule (Baseline Fact 7), and only a ticket's branch holds provisional closures — a resolution made before a fetch may drop a closure the forge has merged; the next synchronization's R-30 restores it.
- **R-13** Only a closure-only conflict settles itself, keeping the remote's closure, in `prepare.rs` — decision 2; a clean merge is built there (Baseline Fact 7) — the local `closed_at` and `closed_by` are lost without notice.
- **R-14** One helper deletes the link record, called by `retire_context` and by `finalize_index`'s retiring branch — a poll's retirement does not pass through `retire_context` (Baseline Fact 12) — a later retirement path must call it too.
- **R-15** `pull_request` and `closure.publication` are filled only by `ticket show` — lists read the index, which holds neither — every item golden changes once (E-4).
- **R-16** `already_closing` is class `None` with outcome `success`, appended last — exit 0 (decision 4), and appending keeps every code's index — a caller that expects `noop` for nothing written sees `success`.
- **R-17** An unfinished runner row holds its ticket against every writer and the poll, as an unfinished direct close holds its item; a new request supersedes it only before a commit or a retirement — decision 8, and a half-done close must not meet another writer — comments and saves on that ticket wait for the request to run again.
- **R-18** An unfinished runner row is listed by `operation list` with the action `pull_request_close`, and is continued only by its request — a person must be able to find what holds the ticket — they must use the request ID in the listing.
- **R-19** The unheld entries are computed per path against the one merge base, without rename detection, unbounded — the preview already lists the whole branch's changes unbounded — a rename shows as a deletion and an addition.
- **R-20** The new branch starts from the tracking primary as last fetched, bound by the digest — prepare reads local state only — a stale tracking primary gives a pull request behind the primary, which the forge shows.
- **R-21** The new branch keeps a provisional closure the old context held, re-read from `T` — the closure was already recorded and published once, and the row does not hold `closed_by` — none.
- **R-22** The desktop gets compile-level arms and the action word "Pull request close" only — Cycle 06 owns its design — the desktop shows a closing ticket as open, offers "Close ticket" (answering `already_closing`) and shows a generic preview.
- **R-23** The runner takes its clock as a parameter (`*_at(now)`), and production passes the wall clock — AGENTS.md: no real waits — none.
- **R-24** One SSH case, in `mutation_remote` beside the lifecycle cases (decision 11), and the new verb's conformance row in the `Confirmation` home with rules 1 and 2 exempt — a `Remote` home would need a second SSH case — those two rules are pinned only by P12.
- **R-25** The link is built from the publication remote's URL as `git2` reports it; P12 configures a `[forge] pull_request_url` template so its golden holds a URL — `forge::recognise` takes a URL, and the harness's URL is no forge's — an unusual URL gives no link, never a failure.
- **R-26** Cycle 02's `pull_request` schema and DTO are reused, or added identically — decision 5 — a merge of two identical files.
- **R-27** Step 3 and the answer table check the tracking primary, and reads and the guard check the local primary — the tracking primary is fresher after a synchronization, and reads need no network — after a forge merge and before the local primary moves, the ticket reads `closing` while withdraw refuses it as closed (K-7).
- **R-28** The new verb is delivered under the label `C5` and sits in `ticket close`'s autonomy column — the label set is pinned, and a later verb joined `C5` before (vh-hyksw); withdraw is consented as close is — none.
- **R-29** `item sync`'s link is new tagged data only when it publishes or finds current a closing ticket's branch in a pull-request repository — decision 12, with every other `item sync` answer unchanged — one more data shape for `item sync`'s callers.
- **R-30** In a ticket context's synchronization, a clean merge result's ticket is given the fetched primary's closure when the primary holds one and the result does not hold it exactly — RFC decision 20 (a closure the primary holds is immutable), which clean merges are not validated for — a branch's withdraw that raced the forge's merge is undone, as it must be.
- **R-31** Part F's step 2 holds the remote through a reservation of the ticket's context target that the runner makes and completes itself under `retire_ulid`, attributed to its row — `delete_tracking_ref` needs a held reservation, and `remote_operation_records.action` cannot take a new value without a table rebuild (Baseline Fact 21) — a reservation row of action `synchronize_context` that never synchronizes, which the attribution rules hide.
- **R-32** Withdraw's `already_applied` keeps `noop` although its step 2 may have published — the effects show the publication — a caller reading only the outcome misses that step 2 published.

## Known Limits

The Cycle document's K-1 to K-9.

## Unverified

- **U-1** That a reservation made under `retire_ulid` for the context target can be taken, held across the cleanup and completed by the runner with `state.rs`'s existing transitions, without `finish_parent` (`reservation.rs` about 1204); Task 9's design checkpoint confirms it, and if it cannot, stops for the owner rather than adding an action value.

## Alternatives Rejected

- Reusing `ticket_closures` with new steps: its CHECK lists and blocker mask need a table rebuild, and its C1 rule forbids exactly what this path does (R-1).
- A third `LifecycleKind` on the direct runner's `ChildExecution` machinery: it would give the attempt, attribution and fence rules for free, but `LifecycleKind` assumes two kinds, the remote action CHECK needs a rebuild, and the direct runner's steps (integration, primary publication, cleanup) are not this path's; the runner keeps the same rules through its own record (Part D).
- Replacing a surviving remote branch: decision 6.
- Ending a stopped close on `request cancel`: decision 8.
- Listing closing tickets in `ticket ready`: decision 9.
- `closed_by` in the answer: decision 10.
- Relaxing immutability for every synchronization: R-12.
- A durable hold across the pull request's review: the RFC's anti-goal.

## Risks

| Risk | Mitigation |
| --- | --- |
| The runner's step and retry rules are wrong in a case the tables miss | Built test-first against the tables, with design checkpoints (plan Tasks 7 and 9); a third review round on these rules before approval |
| The validators' new parameter or R-30 changes direct-mode merges | P5's strict rows and G7; the SSH tier at review-ready |
| E-4 and E-18 touch many goldens and literals | One regeneration, the diff read: only the two null fields added |
| The SSH tier runs every SSH target (about two hours) | Once, at review-ready; between steps only single cases by exact name |
| Cycle 02 lands first with a different `pull_request` shape | Task 0 and Task 11 read main; a difference stops for the owner |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility | 5 | 14 | 15 | Resolved in revision 2: the link deleted by both retirement paths (B1, R-14); immutability relaxed only for a ticket context's stages (B2, R-12); step 3's order (B3); `context_created` reused (B4); the registration tests named and moved (B5, E-11 to E-13, E-18, E-19, R-24, R-28); cancellation and the children's attribution (I1, I2, decision 8, R-4); `restart: true` (I3, R-3); the test publisher (I4); the rule's replacement case and the local-sync tests (I5, R-12); Part G in `prepare.rs` (I6, R-13); the derivation's reach (I7, E-17); E-16 in full (I8); stop and refusal data (I9, R-5); `closed_by` (I10, decision 10); Part F's gaps (I11); the merged-branch claim (I12); Task 2's test (I13); Task 4's edits (I14, E-18); the minors in place |
| 1 | contract and consistency | 5 | 15 | 19 | Resolved in revision 2: `closure.publication` (B1); the immutability rule (B2); step 3's order (B3); the departures as decisions 8 to 11 and D-8 (B4); cancellation as the CLI contract's rule (B5, decision 8); privacy (I6, decision 10); the closed primary with an open branch (I7, R-6); Part D's precedence (I8); Part F's steps (I9); the retry rule (I10); stop data and record states (I11, R-5); the race (I12); the publisher (I13); Cycle 01 decision 11 (I14, K-8); D-1's clauses (I15); withdraw's rows (I16); R-6 and R-7 (I17, decisions 9 and 11); the plan's split (I18); goldens (I19); `setup_pending` (I20); the minors in place |
| 2 | scoped re-review | 2 | 15 | 26 | Resolved in revision 3: per-attempt children and supersede before a commit (B1, Part D, R-3, R-17); a non-SSH withdraw golden in Task 11 (B2); a pinned window in the test publisher and Part G on the stage's inputs (I1); Part G before the runner (I2, plan order); the retirement's reservation (I3, R-31, U-1); the kept closure re-read from `T` (I4, R-21); `finalize_index` (I5, R-14); `list --ready`/`--blocked` (I6); the primary's fields shown (I7); the critical path (I8, R-9); schema roots and enum registrations per task (I9, I13, E-9, E-21); the preview arms in Task 10 (I10); the desktop's `blocker_words` (I11); `PollOnce` (I12); E-20's shape (I14); `from_stored`'s order (I15); the minors in place |
| 2 | adversarial | 6 | 10 | 6 | Resolved in revision 3: child attempts, `cancel_requested`, supersede and abandon (B1); `remote_branch_deleted` at steps 2 and 4 (B2, D6, D11); the retry at `recorded` goes straight to step 4, and the answer table after step 4 (B3); R-30, the primary's closure in a clean merge (B4, W9, G7); situation 3's order (B5); the hold on every writer and the poll (B6, R-17, D18, F14); step 2's sub-steps (I1); `CheckoutDone` (I2); the rebuild's conditions (I3, D15); `existing` consent (I4, D23); finished refusals (I5); the answer rebuilt from the row (I6, D21); closures compared on both fields (I7); dispatch at confirm (I8, D19); the index after step 3 (I9); the rule's tightening (I10, R-12); the minors in place |
