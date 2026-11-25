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

Status: **draft** (revision 2).

## Intent And Approval Boundary

In a pull-request repository, make `ticket close` record the closure on the ticket's branch, publish the branch and give the pull request link, never committing to or pushing the primary. Show a closure the primary does not hold yet as `closing`, keep its dependents blocked, let it be withdrawn, let two clones' closures settle themselves, and close a ticket whose branch merged first (with its remote branch gone) or that has no branch, through a new branch holding only the closure. Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket branch, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `c5e335a2`; Task 0 re-checks every one.
- "Pull-request repository" means a valid configuration with `integration = "pull_request"` (Wave 06 Cycle 01).
- "The tracking primary" is `refs/remotes/<publication remote>/<primary>`, as last fetched. "The local primary" is `refs/heads/<primary>`.
- "The primary's copy" of a ticket in a read is the row the index stores for it in a context that is not an active item context (rank `PRIMARY_COPY` in `row_rank`): the root checkout's working tree (K-9).
- "A provisional closure" is a closure (`closed_at` and `closed_by`) that the ticket's branch holds and the relevant primary does not (RFC, Provisional closure).
- "The runner" is the new pull-request closure runner of Part D. Its three actions are `record` (close), `withdraw` (Part E) and `new_branch` (Part F).
- "Stops" means a step ended without its operation ending: the request record stays accepted, and the same request continues it.

## Baseline Facts

Each fact was read in the source by the planning research, spot-checked by the author, and checked by the round 1 reviews at `c5e335a2`.

1. **The closure state.** `src/repository/read/dto.rs` `ClosureState { Open, Closed }` (a `contract_enum!`, about line 338) and `ClosureDto { state, closed_at, closed_by }`, documented "`closed_at` is set exactly when `state` is `closed`". `src/repository/read/items.rs` `closure()` (about 1354) derives the state from `closed_at` alone. Lists build it from the effective row (`stored_item_dto`), `show` from the effective copy's file (`parsed_item_dto`), and `show --path` through `read_path` (about 2373-2440).
2. **The effective copy.** `effective_rows` and `effective_copy` (`items.rs` about 2035-2120) keep one row per item, ranked by `row_rank` (about 1984-1992): the item's own worktree (`OWN_WORKTREE`), then any non-active context (`PRIMARY_COPY`, `Unverified` included). Every production read holds all stored rows (`stored_items`, about 869; the callers at about 2177, 2257, 2396 and `graph.rs` about 949), including the primary's copy of an item that has an active context (Table A of `docs/plans/2026-11-19-fast-writes-design.md`, about 189). The helpers that give `deps[].state` and `parent.state` (`Targets::of`, about 301-316; `Targets::stored_node` and `Related::of`, about 351-382) and the filter (`TicketFilter::matches`, about 81-93) see only effective rows. No helper gives the primary's copy's closure for an item that has an active context.
3. **Readiness is boolean.** `src/repository/read/graph.rs` `TicketNode { id, closed: bool, parent, deps }` (about 54-61) is filled from `row.closed_at.is_some()`. `blockers`, `is_ready`, `readiness_state`, the open-cycle test, `unplannable`, `batches` and `critical_path` read it; a tree line's state (about 493-497) and the deps root (about 1103) are set from it. `ticket_readiness` (about 1029) keeps rows with `row.closed_at.is_none()`. `items.rs`'s private `Target { Document, Comment, OpenTicket, ClosedTicket }` (about 285) gives `deps[].state` and is matched exhaustively in `Related::check` (about 435).
4. **The closure filter.** `ClosureFilter { Open, Closed, All }` (`items.rs` about 45) is applied in memory. The CLI's value parser (`src/bin/manyhands-cli/parse.rs` about 86-88) takes `open|closed|all`, and `dispatch.rs` (about 58-62) maps any other value to `All` with a wildcard, tested only by `tests/cli/content.rs` (about 352). The help text is pinned in `help_ticket_list.json`, `help_resource_ticket.json`, `help_root.json` and `catalog_tests.rs` (about 230); `parse_tests.rs` (about 408 and 433) holds the value lists.
5. **Exhaustive matches a new value breaks.** `ClosureState`: `src/desktop/model/relationships.rs` (about 336), `tickets.rs` (about 240), `relations_edit.rs` (about 327), `tests/ticket_closure/journeys.rs` (about 390), `src/desktop/model/tickets_tests.rs` (about 136). `DependencyState`: `relationships.rs` (about 101 and 298), `src/repository/read/graph_tests.rs` (about 88). `ReadinessReasonCode`: `src/desktop/model/tickets.rs` (about 159). `ClosureFilter`: `src/desktop/view/tickets/filter_bar.rs`, `src/desktop/model/list_view.rs`, `src/desktop/model/ticket_view.rs`. `PreviewDetailsDto`: `src/bin/manyhands-cli/human_admin.rs` (about 98) and `src/desktop/view/confirm.rs`. `PreviewEffect`: `human_admin.rs` `effect_phrase` (about 165). `MutationDataDto`: `dto.rs` `authority`, `tests/support/mutation.rs` (six) and `tests/support/repair.rs` (one). `OperationAction`: `src/desktop/model/activity/copy/states.rs` `ACTION_WORDS: [(OperationAction, &str); 23]` (about 261). Seven checks compare `== ClosureState::Closed`, among them `bind/ticket.rs` `refuse_closed`, `src/drafts/compare.rs` `lifecycle_closed`, `src/desktop/model/edit.rs` `edit_offered` and the desktop's "Close ticket" action (`src/desktop/view/item/header.rs`).
6. **The closed-ticket guards.** `src/repository/ticket_guard.rs` `require_ticket_open(repo, primary_branch, item)` refuses `TicketClosed` when the local primary's tip or the context branch's tip holds either closure key; it takes no mode. Callers: `src/repository.rs` `refuse_closed_ticket` (about 299; used by `prepare_context`, `save_ticket_with_effective_config` and `checkpoint_comment_locally_with`), `context_release.rs` (about 231), `mutation/bind/republish.rs` (about 202) and `mutation/bind/resolve_history.rs` (about 226); each can read the configuration's `integration()`. The binding precheck `bind/ticket.rs` `refuse_closed` (about 237) refuses when the effective item reads `Closed`. A save edits the worktree file with the draft's fields and carries `closed_at`/`closed_by` through from that file (`draft_of`, `holds`, about 296-345 and 1008-1031).
7. **Closure immutability.** `src/repository/integration/validate.rs` `preserves_item_invariants` (about 199) requires, for a ticket, `old.closed_at.is_none() || (old.closed_at == new.closed_at && old.closed_by == new.closed_by)`. `validates_resolution_sides` (about 224) applies it to base, local and incoming against a resolution, and `validates_external_integration` (about 505) against an external merge. Remote callers: `remote/sync.rs` (about 3317 and 3434, holding a window number; about 4898 and 5304, holding a `ConflictPathToken`) and `remote/conflict_read.rs` (about 768, holding a `Recorded` without a window). A primary-target synchronization's local side is the local primary (`sync_tests.rs` `resolution_fixture`, about 2346-2440), and tests pin that a closure only the local primary holds cannot be dropped (`sync_tests.rs` about 11008 and 11622); local synchronizations pin the same for a context-only closure (`local_sync/conflict_tests.rs` about 716 and 780). Window 0, the legacy envelope, has no intent and no `primary_oid` (`remote/state.rs` about 3266-3270). `classify_conflict_entry` (about 171-184) gives `IncompatibleClosures` for two different closures, which `conflict_eligibility` (about 42) makes `ExternalResolutionRequired`. A clean merge is committed from a memory-only `PreparedMerge`, whose fields are private to `integration/prepare.rs` (about 44-52), without canonical validation (`remote/sync.rs` about 3712-3718 and 3786-3860). `prepare_clean_merge` calls `merge_commits` with libgit2's defaults (`prepare.rs` about 90 and 173). Canonical parse and serialize check only that both fields are present or absent.
8. **The direct close.** One runner (`src/repository/promotion/run.rs` `run_lifecycle`, `promote_inner`) with the durable parent `ticket_closures` (`src/repository/closure/record.rs` about 229, whose DDL is validated exactly). Its closure checkpoint is written under the repository lease immediately before the merge and never pushed on the branch (Wave 02 Cycle 10 decision C1); `written_closure_unfinished` (`record.rs` about 774) refuses any synchronization of the ticket context while a record is between `closing` and `branch_deleted`. Its children run through `synchronize_promotion_child_observed` with a `ChildExecution` (`promotion/lifecycle_record.rs` about 520-574), which reads only a promotion or closure parent. Its digest (`closure/observe.rs` `expectation`, about 1004) does not bind the mode, and `ClosureExpectation` redacts its `Debug` because the digest input holds `closed_by`. `closed_at` comes from `checkpoint.rs` `now_whole_utc_seconds()` (about 70), with no clock seam in execution. `build_closure` (`checkpoint.rs` about 138) reads the ticket from a given commit's tree and commits `Close ticket <ULID>` unreferenced, with author and committer time `closed_at`, so the same inputs give the same OID; `move_context_to_checkpoint_with` (about 314) moves the context safely; `closed_ticket_bytes` (`observe.rs` about 75) writes the fields canonically and refuses a ticket that already holds one. These are visible inside `crate::repository`. `closed_by` is `"<name> <<email>>"` of the configured identity (`observe.rs` about 46; Wave 02 Cycle 10 decision C2, the CLI contract's decision 6). No record, answer or diagnostic holds `closed_by` (Wave 02 Cycle 10 Contract 12; `LifecycleMutationDto`'s documentation, `dto.rs` about 1190; `tests/cli/lifecycle.rs` `application_data_holds_no_closed_by_or_title_beyond_the_index`, about 923).
9. **A contained context today.** For a context contained in the base by ancestry and an open base ticket, the direct close records the closure and merges ("Merged by hand without closure fields: close it now", `closure/observe.rs` about 741). With a publication remote, a context whose remote branch was observed deleted and that is not contained gives `RemoteContextDeleted`, and absent tracking with unknown or unresolved history gives `HistoryUnknown` or `HistoryUnresolved` (about 779-797), read from `remote_context_states` with no network (`promotion/observe.rs` `remote_context_history`, about 891).
10. **The synchronization.** `src/repository/remote/sync.rs` `synchronize_remote_observed` (about 6171, `item sync`'s entry) is public, takes a fresh operation ID and makes its own reservation; it needs no parent record. A context target merges in two stages, the remote copy then the fetched primary (`remote/merge.rs` about 20-31), and publishes; its fetch also fetches the primary (`refs.rs` `context_fetch_refspec`, about 89-96). It answers `Published`, `AlreadyCurrent` or `PublishPending`; or `Transport`, `PushRejected`, `ConflictPending`, `RemoteContextDeleted`. Push refspecs are unforced (`refs.rs` about 99-106); a push the remote refuses as non-fast-forward is `push_rejected`. A synchronization that stops with an error keeps its reservation and its record stays accepted (`mutation/bind/sync.rs` documentation, about 43-47), except `HistoryUnresolved` and `PrimaryHasLocalCommits`, which end it (`sync.rs` about 4243-4270); `RemoteBranchDeleted` and `MergeRequired` are let go only by `request cancel` (`mutation/outcome.rs` `never_gets_past`). Calling it again with the same ID replays a finished operation, answers `RecoveryRequired` for an unfinished one without `restart: true`, and `Interrupted` for a cancelled one (about 8672-8697). A context synchronization refuses with `WorktreeConflicted` when the root has a merge, rebase or cherry-pick in progress (about 745-757). `bind/sync.rs` `MERGING` is `pub(super)`; `SynchronizationSettings::default()` is the domain's setting. In tests, `promotion_test_drive_local` (`sync.rs` about 3920, `cfg(test)`) runs a synchronization's real reservation and stages against a given incoming commit.
11. **Remote ref writes.** There is no guarded non-fast-forward replacement of a remote ref. The one guarded remote write besides a push is `transport/remote.rs` `delete_exact` (about 269), used only by close's and promote's cleanup under their parent's reservation.
12. **Retirement.** `src/repository/cleanup.rs` provides `ContextCleanup` (`acquire`, which takes the repository lease, about 138; `full_check`; `remove_context_worktree`, about 334; `delete_context_branch`, about 477; `require_local_cleanup_absent`, about 540; `delete_tracking_ref`, about 572, which requires its caller to hold and check a reservation) and the SQL step `retire_context` (about 953), which deletes the item's `remote_context_states` row and upserts `retired_contexts`. Its one production caller is the direct close's and promote's cleanup (`promotion/lifecycle_record.rs` about 972). A poll's retirement and `context retire` run through `src/repository/context_retirement/` (`execution.rs` about 246; `request_scope.rs` `delete_retired_contexts` and `delete_context_rows`, about 441-484), not through `retire_context`. After either, a push of a branch with the same name is a fresh publication: `fetch_publication_evidence_for` gives `NeverPublished` and `push_absence` skips the retired history (`remote/state.rs` about 2610-2632; `sync.rs` about 7963-8052). With no context and `ObservedPublished` history, a context synchronization answers `RemoteContextDeleted` (`refs.rs` about 262-268).
13. **Making a branch.** `src/repository.rs` `prepare_context_unlocked` (about 2432) creates `manyhands/<kind>/<id>` only at the local primary's HEAD, and requires the root on the primary and the item on the local primary (about 2459-2530). `remote/poll.rs` `materialize_context_unlocked` (about 322) creates one at a given commit, but is private to the poll, needs a poll owner row and refuses a commit the primary holds. `mark_contexts_dirty` (`repository.rs` about 12915) marks contexts for the next refresh.
14. **The content proof.** `src/repository/context_retirement/proof.rs` `preservation` (about 24) returns `TicketOpen` before any comparison when the primary's ticket is open (about 37). Its Folded loop (about 63-81) passes when every path the branch changed since the one merge base equals the primary's tip, except that the ticket's own file may be the open copy of the primary's closed one (`older_open_ticket`). It keeps no list. Its tests are in `context_retirement/tests.rs`.
15. **Requests, cancellation and operations.** A request record holds one operation (`mutation/bind/mod.rs` `Accepted::of_record`, about 529). `comment add` allocates its child synchronization's ID in a table of its own (`src/repository/recovery.rs` `comment_publication_bindings`, about 47-62). `LifecycleBinding<Close>` (`bind/lifecycle.rs` about 99) binds `ticket close`. `request cancel` and the cancel loop reach a lifecycle through `progress.rs` `asks_a_parent` and `cancel_parent` (about 631-657), which send every `ticket close` to `cancel_ticket_closure`, reading only `ticket_closures`; `cancel_request` finishes nothing (about 670-681). `operation list` reads the promotion and closure tables (`read/status.rs` about 584 and 639) and hides only the remote rows whose ID is such a parent's (about 705-737); `recovery.rs` `attribute_remote_row` (about 1154-1199) attributes children only through those tables and the comment bindings; `operation resume` (`bind/resume.rs` `found_remote`, about 351-470) refuses only comment synchronizations, and reaches lifecycles through `found_lifecycle` (about 291), answering `internal_error` for any other command (about 1307). `recovery.rs` `LIFECYCLE_COMMANDS` (about 1040) only suppresses the author shown. The CLI retries only `busy`, or a `partial` answer whose first action is the same request (`src/bin/manyhands-cli/mutate.rs` about 101-117).
16. **Result codes and data.** `src/results.rs` `result_codes!` assigns each code a class; class `None` exits 0, and `Outcome::of_mutation` (about 737) gives `noop` only for `already_applied`, otherwise `success`. Ordered lists compared in order: `src/results_tests.rs` `DESIGN_CODES` (71, `already_applied` last) and the messages, `schemas/v1/envelope.schema.json` and `schemas/v1/request.schema.json`. `mutation/outcome.rs` `recovery` (about 778) matches every code. `MutationDataDto::from_stored` maps `ticket close` only to the lifecycle DTO (`dto.rs` about 2564), and `item sync`, `repo sync` and `operation resume` to the synchronization's host data or its tagged conflict data (about 2544-2551 and 2588); `authority` names a host only for that host data (about 2441-2479). `item sync` answers with no data when it publishes.
17. **Preview shapes.** `PreviewEffect` (`dto.rs` about 160-198; 37 values in `schemas/v1/preview_effect.schema.json`) already holds `context_created` (about 184), repair's effect, phrased "create the editing context" (`human_admin.rs` about 190). `PreviewDetailsDto` is untagged (about 248-261). `LifecycleBlocker` has 24 values. The closure blocker mask in `ticket_closures` is full (22 bits).
18. **The CLI's inventories.** `tests/cli/inventory.rs` counts mutating verbs (44), catalogued verbs (81) and verbs taking input (24), lists each verb's output schemas (`DATA`), parses the CLI contract's taxonomy table (`the_catalog_and_the_rfc_taxonomies_name_the_same_verbs`, about 397-413), and `no_verb_is_planned` accepts only the delivery labels `C1` to `C5`. `src/bin/manyhands-cli/catalog_tests.rs`'s `DESIGN` table lists every verb by delivery label (a later verb joined `C5`, as vh-hyksw's did). `tests/support/conformance.rs` `HOMES` (46) gives each bound command a home target, and `run_part` (about 369-395) needs one row per command of each home; a `Remote` home's row is driven by an SSH case of its own (`tests/mutation_remote.rs` `CONFORMANCE`, about 4793). `tests/cli/skills.rs` `every_mutation_sits_in_one_autonomy_column` (about 459) needs every `--prepare` verb in `AUTONOMY` and in `skills/manyhands-overview/references/autonomy.md`. `tests/support/cli.rs` `CLI_CASES` (about 217) must list every CLI golden. `tests/repository_enablement.rs` (about 581-652) asserts the exact list of registry tables.
19. **The link builder.** `src/forge.rs` `recognise(remote_url, setting)`, `pull_request_link(forge, source, target) -> PullRequestLink { source, target, forge, url }` and `human_line`. `PullRequestLink` is not `Serialize`, and nothing outside `src/forge.rs` uses it. It needs the remote's URL. The SSH harness's URL (`ssh://fixture@127.0.0.1:<port>/fixture.git`) gives no URL without a `[forge]` template.
20. **Test seams.** The transport is SSH only; no library test can push or fetch through Manyhands without the SSH harness. Close's library tests use an unreachable SSH URL and hand-written tracking refs (`closure/test_support.rs`) and inject a publisher through `promotion/run.rs` `PromotionIo`; `promotion/run_tests.rs` `FakeIo` reserves through `reserve_promotion_child`, which needs a `ChildExecution`. `context_retirement/acceptance_tests.rs` (about 313-360) uses a local bare repository driven by plain `git2` and feeds the poll with `poll::synthetic_fetch`. A poll classifies a deleted remote ticket branch as `Closed` when `proof::completion` passes on the primary (`poll.rs` about 279-304). Close's exit tests re-run the test binary at a failure point (`closure/run_tests.rs` `exit_at`).
21. **Migrations.** `src/repository/discovery.rs` `migrate_registry` ends with `migrate_pull_request_mode` (about 2809), which adds its schema by checking it and records the marker `wave_06_pull_request_mode_v1`.

## Part A: The Closing State

**The values.** `ClosureState` gains `Closing => "closing"`, after `Closed`. `DependencyState` gains `Closing => "closing"`, after `Unresolved`; `parent.state` shares it. `ReadinessReasonCode` gains `ClosingDependency => "closing_dependency"`, after `DependencyCycle` (R-8). The schema enums list the values in that order (`item.schema.json` `closure.state`, `parent.state`; `dependency.schema.json`; `dependency_tree_node.schema.json`; `readiness_reason.schema.json`), and the documentation that names the two states is updated (`ClosureDto`, `ReadinessState`, `DependencyDto`, `ReadinessDto`, `items.rs` about 42, `graph.rs` about 56 and 293, `item.schema.json` about 99). `ClosureDto` gains `publication` (Part C).

**The derivation (R-6, R-7).** For each ticket a read computes one fact from the stored rows it holds: whether its primary's copy (rank `PRIMARY_COPY`) holds `closed_at`. Then:

| Effective copy | Primary's copy | State |
| --- | --- | --- |
| no `closed_at` | holds `closed_at` | `closed` |
| no `closed_at` | open, or absent | `open` |
| holds `closed_at`, and is the primary's copy | — | `closed` |
| holds `closed_at` (an active context's copy) | holds `closed_at` | `closed` |
| holds `closed_at` | open, or absent | `closing` |

The fact is passed, per ID, wherever the state is needed: `closure()` for lists, `show` and `show --path`; `Targets::of`, `Targets::stored_node` and `Related::of` for `deps[].state` and `parent.state`; `TicketFilter::matches`; and `TicketGraph`, for readiness, tree lines and the deps root. The derivation does not read the mode (decision 11).

**Readiness (decision 9).** `TicketNode` keeps `closed`, now true only for `closed`, and gains `closing`. Every readiness and planning decision reads `closed`, so a closing ticket counts as open for its dependents: an edge to a closing ticket gives `closing_dependency` in place of `open_dependency`, and tree lines and the deps root report `closing`. The queries leave closing tickets themselves out: `ticket ready` and `ticket blocked` keep tickets that are neither `closed` nor `closing`; `ticket plan` and `ticket critical-path` are computed with closing tickets as open nodes, then closing tickets are removed from the output, empty batches are dropped and batches are numbered again from 1 (R-9). A closing ticket's own `readiness` field is computed as an open ticket's. `tickets_made_ready` is unchanged.

**The filter.** `ClosureFilter` gains `Closing`. `Open` matches `open` and `closing`; `Closing` only `closing`; `Closed` only `closed`. The CLI's value parser takes `open|closed|closing|all`, `dispatch.rs` maps `closing` explicitly, and the help text says so.

**Human output.** `item_line`, `item_show` and `dependency_line` print the state's `as_str()`, so they print `closing` unchanged. `item_show` adds `Publication: pending` or `Publication: published` for a closing ticket, and the link's `human_line()` when the item has one.

**The desktop (R-22).** Compile-level arms only: `ClosureState::Closing` maps to `Lifecycle::Open`, `DependencyState::Closing` as `Open`, `ClosingDependency` as `OpenDependency`, and `ClosureFilter::Closing` gets an arm the desktop never selects. Cycle 06 designs the desktop's closing state.

## Part B: The Guards, The Save Check And Immutability

**The domain guard (R-10).** `require_ticket_open` gains the repository's integration: in a pull-request repository it reads only the local primary's tip; in direct mode it reads both tips, as today. `refuse_closed_ticket`, `bind/republish.rs` and `bind/resolve_history.rs` pass the mode of the configuration they hold; `context_release.rs` passes `Direct` (a closing ticket has a commit of its own, so it cannot be released anyway). So in a pull-request repository a closing ticket accepts `ticket save`, slug assignment, context prepare, `comment add`, `item republish` and `item resolve-history`, and a ticket whose local primary holds a closure refuses them with `ticket_closed`. Commits made with Git in the ticket's worktree are not Manyhands writes and meet no guard (RFC).

**The binding precheck.** `refuse_closed` keeps comparing with `ClosureState::Closed`, so it lets a closing ticket through in both modes, and the domain guard decides. The other `== ClosureState::Closed` checks (Baseline Fact 5) keep their comparison: drafts and the desktop's edit action treat a closing ticket as open (editable), and the desktop's "Close ticket" stays offered, answering `already_closing` (R-22).

**The save check (R-11).** A ticket save, in both modes, refuses with `invalid_input` when the resulting file's `closed_at` or `closed_by` differ from the ticket at the context branch's head (or from an open ticket, for a context with no ticket yet). It runs in the domain save under the lease, before the checkpoint. Without it a hand edit of the worktree file could record or change a closure, because the precheck no longer refuses a file that reads `closing`.

**Immutability (R-12).** `preserves_item_invariants` gains an optional parsed primary-side copy of the same path. For a ticket, a change of closure from `old` to `new` is accepted when:

- `old` holds no closure; or
- `new` holds the same closure as `old`; or
- a primary-side copy is given, it does not hold `old`'s closure, and `new`'s closure is none, the primary side's, or one that some side of the merge holds and the primary side does not.

So a closure the primary side holds stays immutable, and a provisional closure may be dropped (a withdraw) or replaced (Part G). The primary-side copy is given only by a remote synchronization of a ticket context: in its Context stage it is the integration window's `primary_oid` (the fetched primary), and in its Primary stage the stage's incoming commit (the same fetched primary). A primary-target synchronization, a local synchronization, and any caller with no pinned window (window 0, or `remote/conflict_read.rs`'s `Recorded` until Task 3 gives it the window) give none, and the rule is today's. `validates_resolution_sides` and `validates_external_integration` take the optional primary-side commit and pass its copy. `classify_conflict_entry` is unchanged.

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

Every runner answer and every `item sync` answer that gives a link upserts its row (Part H). One helper deletes an item's row, and both retirement paths call it in their finishing transaction: `cleanup.rs` `retire_context` (the direct close's and promote's cleanup, and Part F) and `context_retirement`'s deletion of the retired context's rows (a poll's retirement and `context retire`). A withdraw keeps it, because the pull request may stay open. The table and Part D's `pull_request_closures` are created by one named, additive migration, marker `wave_06_pull_request_close_v1`, run after `migrate_pull_request_mode`, adding its tables by checking the schema, in an immediate transaction with a read-only precheck. A build without this Cycle refuses a pull-request repository's version 2 configuration, so it never retires a context whose link this table holds.

**On the item (R-15).** `ItemDto` gains `pull_request`, after `readiness`: the record's link in `ticket show` and `ticket show --path` for a ticket that has one; null in every list, for a document (Cycle 04 fills documents), and for a ticket without a record. `ClosureDto` gains `publication`: for a closing ticket in `show`, `pending` when the context branch's head is not the commit its remote tracking ref holds (including when it has none), and `published` when it is; null otherwise and in every list (RFC: "shows in `ticket show` as closing, with publication pending"). Both fields are required in `item.schema.json`, and null in a list means "not read here".

## Part D: The Pull-Request Close

**The hand-off (R-1, R-2).** `ticket close` keeps its verb. Its binding becomes a wrapper that, at prepare, reads the configuration it already reads and prepares the runner (in a pull-request repository) or today's direct close; at re-entry it finds the request's own record in `pull_request_closures` or `ticket_closures` and continues that one. The runner's digest has its own domain, `manyhands pull request closure v1`, so a prepare made in one mode is `external_change` when confirmed in the other. The runner lives in `src/repository/closure/pull_request.rs` with its record; it shares `closed_ticket_bytes`, `build_closure`'s commit building and `move_context_to_checkpoint_with`, and does not use `ticket_closures`, `ChildExecution` or `written_closure_unfinished` (Baseline Fact 8). Its expectation type redacts its `Debug`, as `ClosureExpectation` does, and its entry takes an observer and a cancellation probe, as `close_ticket_observed` does.

**The record.** `pull_request_closures`, one row per confirmed request's operation:

| Column | Meaning |
| --- | --- |
| `operation_ulid` | primary key: the request's operation |
| `repository_id`, `item_ulid` | the ticket |
| `action` | `record`, `withdraw` or `new_branch` |
| `expectation` | the consented digest |
| `first_sync_ulid` | the first synchronization's operation (null for `new_branch`) |
| `publish_sync_ulid` | the publishing synchronization's operation |
| `step` | unfinished: `confirmed`, `synchronized`, `retired`, `recorded`, `branched`; finished: `published`, `already_closing`, `already_applied`, `refused` |
| `parent_oid` | the head the commit was built on |
| `closed_at` | the frozen time, whole UTC seconds, written before the commit (as C6) |
| `checkpoint_oid` | the closure or withdraw commit, written before any ref moves |

It never holds `closed_by`, a title, a body or a URL (Baseline Fact 8). A unique partial index on unfinished rows per `(repository_id, item_ulid)` lets one unfinished row hold the ticket (R-18).

**Prepare** reads local state only, with no network, lease or write. The ticket's situation is decided in this order:

1. the local or the tracking primary's ticket holds a closure: blocked `ticket_closed` (blocker `already_closed`);
2. a context exists and its remote branch was observed deleted: Part F's `new_branch`, keeping a provisional closure the context's ticket holds (R-21);
3. no context exists: if a remote tracking ref of the ticket's branch exists, blocked with the new blocker `remote_context_unmaterialized` (recovery `poll.once`); otherwise Part F's `new_branch`, retiring the remote history if it was published;
4. the context's ticket holds a closure: `record` with closure `existing`;
5. otherwise: `record`.

Then the blockers: not a ticket; an unfinished runner row or direct close on the ticket (`recovery_required`, blocker `local_operation_pending`, naming the holder's `request.retry`); a pending comment publication on the ticket (as C12); the context nonconforming, dirty or conflicted; a synchronization conflict pending; a merge, rebase or cherry-pick in progress at the root (`WorktreeConflicted`, Baseline Fact 10); `history_unknown` and `history_unresolved` as the direct close reports them; no identity (`identity_required`). There is no blocker for the root's branch or cleanliness otherwise, because nothing touches the primary. The direct close's prepare also blocks on an unfinished runner row.

The digest binds the domain, the action, the item, the context branch, the primary branch, the remote, the context's head (or none), the closure effect (`record` with the `closed_by` text, or `existing` with both fields), and for `new_branch` the tracking primary, the old context's tip and the unheld entries (RFC: "the mode, the item, the branch, the remote, the target primary and the `closed_by` text"). It does not bind a `record` closure's `closed_at` (C6). The preview's details are a new `PreviewDetailsDto` variant, tagged `"type": "pull_request_closure"`, holding the action, the item and its slug, the branches, the remote, the context and tracking-primary OIDs, the closure effect, the retired context and the unheld entries (Part F), and the `pull_request` object. The effects:

| Action | Effects |
| --- | --- |
| `record` | `context_synchronized`, `closure_recorded`, `context_published`, `pull_request_link` |
| `record`, closure `existing` | `context_synchronized`, `pull_request_link` |
| `withdraw` | `context_synchronized`, `closure_withdrawn`, `context_published` |
| `new_branch` | `worktree_removed` and `branch_deleted` (for an old context), `context_created`, `closure_recorded` (absent when kept), `context_published`, `pull_request_link` |

**Confirm, `record` action.** The plan builds this test-first against the table below, with a design checkpoint.

1. Re-observe, compare the digest, and check the effective identity's `closed_by` against the consented text (`external_change` on any difference; nothing written). Insert the row at `confirmed` with both synchronization IDs; an insert refused by the unique index is `recovery_required` naming the holder, with nothing written.
2. Run the first synchronization under `first_sync_ulid`: the ticket context's synchronization through `synchronize_remote_observed` with `SynchronizationSettings::default()`, and `restart: true` when the row already exists. It merges the remote copy and the fetched primary and publishes. A replay of a finished one is its result; `Interrupted` is answered as `cancelled`. Step `synchronized`.
3. Under the repository lease, with the context clean and at its branch's head, in this order:
   1. the tracking primary's ticket holds a closure: step `refused`; `ticket_closed`, blocker `already_closed`;
   2. the row is at `recorded`: if the head is `parent_oid`, move the context to `checkpoint_oid`; if `checkpoint_oid` is the head or an ancestor of it, go on; otherwise (a save moved the head before the ref moved) build the closure again on the head with the stored `closed_at`, store the new `parent_oid` and `checkpoint_oid`, and move;
   3. the head's ticket holds a closure: if its `closed_at` is the row's, go on (this request's own); otherwise step `already_closing`, and the answer is `already_closing` with the link;
   4. otherwise: check the identity again; take `closed_at` from the injected clock (R-23); build the closure commit on the head with `closed_ticket_bytes`, unreferenced; store `parent_oid`, `closed_at` and `checkpoint_oid` (step `recorded`); move the context to it with `move_context_to_checkpoint_with`.
4. Run the publishing synchronization under `publish_sync_ulid` (with `restart: true` when it exists). Step `published`.
5. Upsert the link record. The answer is `ok` with the link when the head's ticket holds this request's closure, and `already_closing` with the link when it holds another's (Part G kept the published one). Refresh the index outside the lease, as a save does.

**What a stop answers (R-5).** A step that stops on its synchronization answers as `item sync` answers that stop: the same code, recovery and data (host data, the tagged conflict data, or none), with the effects adding `write` and `checkpoint` `committed` once step 3 has moved the ref. The request record stays accepted and the row unfinished; the same request continues from the stored step (decision 8). `request cancel` and Ctrl-C answer `cancelled` (exit 130) at a safe point and leave the row unfinished, as the CLI contract says for `ticket close`; `progress.rs` asks the runner's row for the request's cancellation and cancels a running child. The runner's two synchronizations are attributed to its row: `operation list` does not list them as operations of their own, `recovery.rs` names the runner's request as their holder, and `operation resume` of either answers `original_request_required` naming the request (R-4).

### Required Outcomes Of Part D

| # | Situation | Required outcome |
| --- | --- | --- |
| D1 | New request, context ticket open, primary open | `ok`; one `Close ticket <ULID>` commit on the branch holding `closed_at` and `closed_by`; the branch published; `pull_request` in data; the row `published`; nothing held after; the local primary unchanged; no synchronization targets the primary |
| D2 | The branch holds a closure at prepare (its own earlier one, or another clone's) | preview `existing`; step 2 publishes anything pending; `already_closing`, exit 0, the link; no closure commit |
| D3 | Step 2 brings in another clone's published closure | `already_closing`; no closure commit |
| D4 | After step 2 the tracking primary holds the closure | `ticket_closed`, blocker `already_closed`; no closure commit; row `refused` |
| D5 | Step 2 stops: transport, key, host, `push_rejected` | as `item sync`'s stop; nothing written by the close; the row `confirmed`; the same request resumes step 2 |
| D6 | Step 2 answers `remote_branch_deleted` | that code and recovery (`request cancel`); nothing written; the same request continues it; once the request is cancelled and a poll or fetch has recorded the deletion, a new `ticket close --prepare` offers Part F |
| D7 | Step 2 meets a conflict | `recovery_required` with the conflict data, as `item sync` answers; nothing written by the close; the row `confirmed` |
| D8 | Step 4 stops: transport, key, host | as `item sync`'s stop, with `write` and `checkpoint` `committed`; the row `recorded`; the ticket reads `closing`, `publication` `pending`; the same request resumes step 4 |
| D9 | Step 4 answers `push_rejected` because another clone published first | as D8; the same request's next call fetches, Part G keeps the published closure, publishes; `already_closing` |
| D10 | Step 4 meets a conflict | as D8, with the conflict data |
| D11 | A crash after `checkpoint_oid` is stored and before the ref moves; then the same request | the ref moves to the stored commit; no second commit; the same `closed_at`; `ok` |
| D12 | As D11, but a save moved the head before the retry | one closure commit on the new head with the stored `closed_at`; the save's commit kept; `ok` |
| D13 | A crash after the ref moved, at `synchronized` or before the row's final step; then the same request | no second commit; step 4 or the replayed publication; `ok` |
| D14 | A new request while a row is unfinished | prepare `recovery_required` naming the holder's `request.retry`; a confirm whose insert loses the race answers the same; nothing written |
| D15 | `request cancel` or Ctrl-C during the close | `cancelled`, exit 130; the row unfinished; the same request continues it; no reversal |
| D16 | Prepared in one mode, confirmed after a switch | `external_change`; nothing written |
| D17 | The identity changed between prepare and confirm | `external_change` at step 1; nothing written |
| D18 | Replay of a finished request | the stored envelope, read back through `from_stored` |
| D19 | `operation list` and `operation resume` while a row is unfinished | the row listed with the request that continues it; its synchronizations not listed on their own; `operation resume` of the row or of a synchronization: `original_request_required` |

**A merged branch whose remote branch survives (decision 6).** This is the `record` action. When the primary holds the branch's work by ancestry, step 2's merge of the fetched primary is a fast-forward; when every path the branch changed since its one merge base equals the primary's tip, the merge's result is the primary's tree path by path. Either way the closure commit is then the only change against the primary. Otherwise, or when libgit2's merge reports a conflict (for example on a rename), D7 applies.

## Part E: Withdraw

`ticket withdraw-closure --id <ULID>`: a confirmed verb, `--prepare` and `--confirm` as close, no input, the same runner with the action `withdraw`, in either mode (decision 11). Its digest has the runner's domain and binds the mode.

**Prepare.** Part D's blockers, except situations 2 and 3. A ticket whose context ticket holds no closure answers `already_applied` with nothing to confirm (decision 11). Otherwise the digest binds the action, the item, the branch, the remote and the closure being withdrawn (both fields).

**Confirm.** Steps 1, 2 and 4 of Part D. Step 3, under the lease, in this order: the tracking primary's ticket holds a closure: `ticket_closed`, blocker `already_closed` (withdrawing it would be a reopen); a `recorded` row: Part D's retry rule; the head's ticket holds no closure: `already_applied`; it holds a different closure than the consented one: `external_change`; otherwise the commit `Withdraw closure of ticket <ULID>`, whose ticket is the head's with both fields removed and serialized canonically (a new byte helper beside `closed_ticket_bytes`), built unreferenced, recorded, then the ref moved. The answer is `ok` with the link (the pull request may stay open), and the link record is kept.

| # | Situation | Required outcome |
| --- | --- | --- |
| W1 | A closing ticket, in a pull-request repository and in a direct one | `ok`; one withdraw commit; the branch published; the ticket reads `open`; its dependents read blocked by `open_dependency`; the link record kept |
| W2 | No closure on the branch | `already_applied` at prepare, or at step 3 after another clone's withdraw arrived; nothing written |
| W3 | The tracking primary holds the closure | `ticket_closed`; nothing written |
| W4 | An unfinished direct close or runner row holds the ticket | prepare `recovery_required`, naming the holder |
| W5 | Step 2 or step 4 stops | as D5 and D8 (D6's recovery without Part F) |
| W6 | Another clone's synchronization merges a withdraw into its unchanged closure | the clean merge keeps the withdraw; a conflicting merge accepts a resolution that keeps it (Part B) |
| W7 | After W1, `ticket close` again | Part D's `record`, a new `closed_at` |
| W8 | A crash after the ref moved; then the same request | no second commit; `ok` |

## Part F: The New Branch

Used in a pull-request repository by close for a ticket whose context's remote branch was observed deleted (decision 1, narrowed by decision 6), and for a ticket with no context (decision 7).

**Prepare.** Besides Part D's blockers:

- the tracking primary must hold the ticket, open; otherwise the first case keeps the blocker `remote_context_deleted` (the way out is `item republish`) and the second answers `context_missing`;
- an old context must be clean, unconflicted and at its branch's head, read as the existing blockers read it (the cleanup's full check runs at confirm).

The preview names the tracking primary `P` the new branch starts from, the old context's tip `T` when there is one, and the **unheld entries**: each path whose entry in `T` differs from its entry in the one merge base of `T` and `P`, and also differs from its entry in `P`, with its change against `P` (`added`, `modified`, `deleted`), compared per path without rename detection, unbounded and sorted by path (R-19). An empty list means the content proof passes. History with no merge base or more than one is `recovery_required`. A new function in `proof.rs` computes the list from the existing private `entries`; it treats the ticket file like any other path. A provisional closure the old context's ticket holds is kept (closure `existing`, R-21).

**Confirm.**

1. Re-observe, compare and check the identity as Part D step 1 (`external_change` when `P`, `T` or the unheld entries changed). Insert the row with `closed_at` from the injected clock, or the kept closure's, and `publish_sync_ulid`.
2. With an old context, or published remote history: under a remote reservation, so that no poll or synchronization acts on the item meanwhile, run the cleanup's full check (a refusal is answered as the direct close's cleanup answers it, with nothing changed), remove the worktree, delete the local branch, check local absence, delete the tracking ref if one exists, and in one transaction `retire_context` and the link deletion. Which row holds the reservation, and how `operation list` attributes it to the runner, is Task 8's design checkpoint. Nothing on the remote changes. Step `retired`.
3. Under the repository lease, by the state of `manyhands/ticket/<ULID>`:
   - absent: build the closure commit `C` on `P` (the ticket from `P`'s tree with the closure's fields), store `checkpoint_oid` (step `recorded`), create the branch at `C` (step `branched`), add its worktree at `.manyhands/worktrees/<ULID>`, and mark the contexts dirty with `mark_contexts_dirty`;
   - at `checkpoint_oid` (a retry): add the worktree if it is missing, then step 4;
   - elsewhere, holding a closure with the row's `closed_at`: step 4; holding another closure: step 4, then `already_closing`; holding none: `recovery_required`.

   The function that creates a ticket's context branch at a given commit and adds its worktree is new, beside `prepare_context_unlocked`, without that function's checks that the root is on the primary and the item on the local primary (Baseline Fact 13).
4. Publish under `publish_sync_ulid`: a fresh publication (Baseline Fact 12). Step `published`.
5. Upsert the link record; answer `ok` with the link, or `already_closing` per step 3.

| # | Situation | Required outcome |
| --- | --- | --- |
| F1 | Old context, remote branch observed deleted, content proven merged | preview with no unheld entries; confirm: the old worktree and branch gone, a branch whose only commit over `P` is the closure, published, `ok` with the link; nothing deleted on the remote |
| F2 | As F1, content unproven (a squash after which the primary moved on, or a declined pull request whose branch was deleted) | the preview lists the unheld entries; after confirm, as F1; the old branch's commits are no longer referenced by a branch (K-6) |
| F3 | No context, ticket open on the tracking primary, never published | no retirement; as F1 otherwise |
| F4 | No context, remote history published and deleted | the history retired in step 2; as F3 |
| F5 | No context, a remote ticket branch exists | blocked `remote_context_unmaterialized` |
| F6 | The tracking primary lacks the ticket | `remote_context_deleted` (old context) or `context_missing` (none) |
| F7 | `P`, `T` or the old worktree changed between prepare and confirm | `external_change`, or the cleanup's refusal; nothing changed |
| F8 | The old context held a provisional closure | the new branch keeps its `closed_at` and `closed_by` |
| F9 | A crash at each step boundary (after `retired`, after `recorded`, after `branched` before the worktree); then the same request | continues from the stored step; one branch, one closure commit (the same OID: same tree, parent and time), the stored `closed_at` |
| F10 | A new request while the row is unfinished | `recovery_required` naming the holder |
| F11 | Step 4 stops | as D8; the branch is local and reads `closing` |
| F12 | The branch exists at step 3 holding another closure | step 4, then `already_closing` |

## Part G: Two Clones

**Settling a closure-only conflict (decision 2, R-13).** In a remote synchronization of a ticket context, the merge of the remote copy into the local branch (the Context stage) settles a conflict itself when all of these hold:

1. the only conflicted path is the ticket's own `ticket.md`;
2. the local (ours) and remote (theirs) tickets both parse and both hold closures, and the two closures differ;
3. the integration window's primary (the fetched primary) holds no closure of the ticket;
4. with ours' ticket rewritten to hold theirs' closure (canonically, through a new byte helper that replaces the fields of a closed ticket), the three-way merge of that path is clean.

The result is the clean merge of step 4, so the remote branch's closure (the one published first) is kept and the local one dropped. The function lives in `integration/prepare.rs` and returns `MergePreparation::Clean`, so the synchronization commits it as the stage's ordinary clean merge; it is called in `integrate_divergence` where `prepare_clean_merge` returns `Conflict`, before any conflict is installed. Every other conflict is unchanged.

| # | Situation | Required outcome |
| --- | --- | --- |
| G1 | Two different provisional closures, nothing else conflicting | clean; theirs kept; the synchronization publishes; the ticket reads `closing` with theirs' fields |
| G2 | Different closures, and another conflicting hunk in the same file or another path | unchanged: `IncompatibleClosures`, external repair; a resolution or repair that keeps either closure is accepted where the window is known (Part B) |
| G3 | The fetched primary holds one of the closures | unchanged: external repair, keeping the primary's |
| G4 | The same closure on both sides | unchanged: a clean merge |
| G5 | Base closed, ours withdrawn, theirs unchanged | unchanged: a clean merge keeps the withdraw (W6) |
| G6 | Base closed with `A`, ours re-recorded `B` after a withdraw, theirs `A` | unchanged: a clean merge keeps `B` |

This changes Wave 02 Cycle 10's C11 ("two different closures can never be resolved in-product") for provisional closures only.

## Part H: Codes, Data, CLI And Contract

**`already_closing` (decision 4, R-16).** A new result code, class `None` (exit 0), outcome `success` (it is not added to `noop`, because its step 2 can publish), message "The ticket's branch already holds a closure.", appended after `already_applied` in every ordered list, in `recovery`'s shared arm.

**The data (decisions 3 and 10).** A new `MutationDataDto` variant, `PullRequestClosureMutationDto`, tagged `"type": "pull_request_closure"` (a `contract_enum`, as Cycle 01's tag), with:

- `action`: `record`, `withdraw` or `new_branch`;
- `closure`: `{ recording, closed_at }`, `recording` one of `recorded`, `existing`, `withdrawn`, `none`; never `closed_by`;
- `checkpoint_oid`: the commit this request made, or null;
- `publication`: `published` or `current`;
- `retired_context`: the old context's tip, or null;
- `pull_request`: the object, or null for `withdraw` with no link record;
- `blockers`: the `LifecycleBlocker` values of a blocked answer, otherwise empty.

It is the data of `ok`, `already_closing`, `already_applied` and the blocked answers of the runner. A stop carries the synchronization's own data (R-5). Its schema is `schemas/v1/pull_request_closure_mutation.schema.json`, in `DATA_ROOTS`. `from_stored` gains a tag-guarded arm for `ticket close` and `ticket withdraw-closure`, before their general arms, and those two commands join the arms that read the synchronization's host and conflict data.

**`item sync`'s link (decision 12, R-29).** When `item sync` of a ticket context in a pull-request repository publishes, or finds current, a branch whose ticket holds a provisional closure, its answer carries data tagged `"type": "pull_request_link"`, holding `pull_request`, and it upserts the link record. Every other `item sync` answer is unchanged. Its schema is `schemas/v1/pull_request_link.schema.json`, in `DATA_ROOTS`, with a tag-guarded `from_stored` arm for `item sync`.

**Blockers.** `LifecycleBlocker` gains `remote_context_unmaterialized`, appended; the runner reuses the other values Part D names.

**The verb (R-28).** `Command::TicketWithdrawClosure` (`"ticket withdraw-closure"`), confirmed, with an operation, stopping at safe points, finishing as a synchronization; `Mutation::TicketWithdrawClosure(TicketWithdrawClosureInput { root, id })`; a catalog entry taking `--repo`, `--id`, `--request-id`, `--prepare`, `--confirm` and no input, delivered under the label `C5`; dispatch, parse and human output as `ticket close`; the autonomy column of `ticket close`; its conformance row in the `Confirmation` home (R-24).

**Operations (R-18).** An unfinished runner row is listed by `operation list` with the new action `pull_request_close` (an `OperationAction` value, in `operation.schema.json`, with the desktop's word "Pull request close" in `ACTION_WORDS`), for close and withdraw alike, naming the request that continues it.

**Preview effects.** `PreviewEffect` gains `context_published`, `pull_request_link` and `closure_withdrawn`, appended; `new_branch` reuses `context_created`, whose documentation widens to a context made from either primary. `effect_phrase` and the schema's description cover them.

**Human output.** The preview prints its effects, the closure (with `closed_by`, as the direct preview does), and for `new_branch` each unheld entry as `not on <primary>: <change> <path>`, through the output's field escaping. The answer prints `closure: recorded <closed_at>`, `closure: already recorded`, `closure: withdrawn` or nothing, then `publication: <state>`, then the link's `human_line()` on its own line. `item sync`'s human output adds the link's line when its answer carries one.

**The desktop.** Compile-level arms for the new preview, data and verb, and the action word (R-22).

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | Part A's derivation table in lists, `show` and `show --path`; `deps[].state`, `parent.state` and tree lines `closing` | `tests/read_items.rs`, one table-driven test, beside `presentation_across_contexts`; the `items.rs` unit tests for `Related::of` |
| P2 | A dependent of a closing ticket is blocked with `closing_dependency` and ready once the primary's copy holds the closure; `ticket ready`, `blocked`, `plan` and `critical-path` leave a closing ticket out | `tests/read_relationships.rs`, one table-driven test; the `graph.rs` unit tests for the renumbered batches |
| P3 | `--closure open`, `closing`, `closed`, `all`; the parser, dispatch and help | `tests/read_items.rs`; `parse_tests.rs`; `tests/cli/content.rs`; the help goldens |
| P4 | The guard per mode at every caller, the save check, comments and saves on a closing ticket | `src/repository/ticket_guard_tests.rs`, a table over the callers; `tests/mutation_bridges/context.rs` |
| P5 | Part B's immutability rule: provisional closures in a ticket context's stages, the strict rule for primary targets, local synchronizations and callers with no window | `src/repository/integration/tests.rs`, a table; one row per validator through `sync_tests.rs`'s existing callers |
| P6 | The link record: migration once, upsert replaces, deletion by each retirement path; `ticket show`'s `pull_request` and `closure.publication` | `src/repository/discovery.rs`'s tests; `src/repository/cleanup_tests.rs`; a `context_retirement` test; `tests/read_items.rs` |
| P7 | D1 to D19 | `src/repository/closure/pull_request_tests.rs` (D18 and D19 in Task 12), with an injected publisher and clock |
| P8 | W1 to W8 | the same file |
| P9 | F1 to F12, and the unheld entries | the same file; `src/repository/context_retirement/tests.rs` |
| P10 | G1 to G6, and the merged branch of Part D's last paragraph | `sync_tests.rs`, the pure function's table and one `integrate_divergence` row modelled on its existing callers |
| P11 | The owner's lifecycles, end to end in the library | `pull_request_tests.rs`, with a local bare repository driven by `git2` as the forge, `promotion_test_drive_local` for the synchronizations' stages, and `synthetic_fetch` for the poll |
| P12 | End to end over SSH: close, close again, withdraw, close again in a pull-request repository; the server's primary unchanged throughout; the goldens | one SSH case in `mutation_remote` |
| P13 | The CLI contract with no network: one preview golden, `already_applied` for withdraw, the blocked answers, a confirm stopped at step 2 | `tests/cli/lifecycle.rs` |
| P14 | `item sync`'s link | a binding test beside `bind/sync_tests.rs`'s, and the P12 case's `item sync` step |
| P15 | Every registration (codes, enums, schemas, verbs, goldens, inventories, conformance, autonomy) | the existing registration tests |

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). P12 is the only SSH case (decision 11): no non-SSH publication remote can be selected, so only a case over the harness can produce the success goldens, and it shows that the server's primary is never pushed in pull-request mode (the RFC's Required Evidence 1 and 2). Its edge cases are in P7 to P11. No real waits: the runner takes its clock. The injected publisher of P7 to P11 refuses a synchronization whose target is the primary, and every runner test asserts that the local primary did not move and that no synchronization targeted the primary; that is how "a test fails if one tries" holds in the library. The publisher runs `promotion_test_drive_local` against the local bare repository's branch where a merge must happen (D9, W6, P11), and completes the row as `FakeIo` does elsewhere. Goldens: one per output contract (the preview, each data shape) and one per result code the contract tables require.

At review-ready the SSH tier is every SSH-harness target: `ticket_closure` (`src/repository/closure/`), `mutation_remote` (`src/repository/mutation/`), the six targets of `src/repository/remote/` (`ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`), `comment_publication` (the guard in `checkpoint_comment_locally_with`), `document_promotion` (`cleanup.rs`), and `ssh_fixture`, with `--lib repository::local_sync::` for the validators' local callers.

## Named Edits To Pre-Existing Tests

The owner approved E-16 in decision 11; the others are asked at approval.

- **E-1** retired: `tests/read_contract.rs` `contract_enumerations` is built from `::ALL`, and only the schema files change.
- **E-2** (Task 1) the exhaustive `ClosureState` matches in `tests/ticket_closure/journeys.rs` and `src/desktop/model/tickets_tests.rs`, and the `DependencyState` match in `src/repository/read/graph_tests.rs`: a `Closing` arm that fails the test, as an unexpected state.
- **E-3** (Task 1) `open|closed|all` to `open|closed|closing|all` in `help_ticket_list.json`, `help_resource_ticket.json`, `help_root.json` and `catalog_tests.rs`; `parse_tests.rs`'s valid-value list gains `closing`; `tests/cli/content.rs` gains a `closing` row.
- **E-4** (Task 4) every item golden under `tests/fixtures/read_v1/` and `tests/fixtures/cli_v1/` gains `"pull_request": null`, and every ticket's `closure` gains `"publication": null`, regenerated with `MANYHANDS_UPDATE_GOLDEN=1` for the tests that own them and read before committing.
- **E-5** (Task 5) `src/results_tests.rs`: `already_closing` in `DESIGN_CODES` (71 to 72) and the messages.
- **E-6** (Task 5) `tests/mutation_contract.rs`: `already_closing` in both `CODES_` tables with the reason "golden written in Task 14"; **E-7** (Task 14) removes it.
- **E-8** (Task 10) `tests/support/mutation.rs` (six arms) and `tests/support/repair.rs` (one): the two new variants.
- **E-9** (Tasks 5 and 10) `tests/read_contract.rs`'s registrations for the new schema enums: the three effects, the data's and the link's `type`, the preview's `type`, the closure's `publication`, the `pull_request` `forge` (unless Cycle 02 registered it), the blocker and the operation action.
- **E-10** (Task 11) `src/repository/mutation/command.rs` tests that hard-code command sets (about 770, 870, 899, 1018): the new verb in the stopping, confirmed and synchronizing sets.
- **E-11** (Task 11) `tests/cli/inventory.rs`: mutating 44 to 45, catalogued 81 to 82, `NO_INPUT` and `DATA` rows for the new verb, and `ticket close`'s and `item sync`'s new schemas.
- **E-12** (Task 11) `tests/support/conformance.rs` `HOMES` (46 to 47) and a row in `tests/mutation_confirmation`, whose rules 1 and 2 are exempt with the reason that a finished withdraw needs a server, pinned by P12's same-request replay.
- **E-13** (Task 11) the help listings: `help_root.json`, `help_root_text.txt`, `help_resource_ticket.json`, a new `help_ticket_withdraw_closure.json` in `tests/cli/help.rs`'s list; `catalog_tests.rs`'s `DESIGN` table (the `C5` row); `tests/support/cli.rs` `CLI_CASES` for every new CLI golden.
- **E-14** (Task 2) `src/repository/ticket_guard_tests.rs`: its 5 `require_ticket_open` calls pass `Integration::Direct`; (Task 3) the direct validator calls at `sync_tests.rs` about 4015 and 11480 pass no primary side.
- **E-15** (Task 15) `tests/cli/recovery_tables.rs`, `tests/cli/collaboration_exits.rs` and the audit's C5 counts, when D-6 adds `already_closing` to `ticket close`'s codes and the new verb's row.
- **E-16** (Task 1) **reverses an assertion's intent; approved by the owner in decision 11.** `tests/read_relationships.rs` `ready_and_blocked_are_decided_across_primary_and_two_item_worktrees` (about 290-438; "C waits for A, which is closed where it counts: in A's own context"): A now reads `closing` and is left out of the queries, B and C are blocked by `closing_dependency`, D by `open_dependency` on B; the assertions at about 325-336, 349, 369, 383, 404, 415, 421-428 and 430 change; then A's closure is committed on the primary too, and B and C read ready as before. `closing_a_dependency_makes_its_dependent_ready_without_the_dependent_being_observed_again` (about 919-984): A is given a primary copy (committed on the primary before its context is made), the SQL update closes A's primary row, and the test keeps its intent (B ready without being observed again); its last part asserts that a closure in A's worktree alone leaves B blocked.
- **E-17** (Task 1) `src/repository/read/items_tests.rs` `ticket_filters_compare_whole_values_and_closure_reads_only_closed_at` (about 334): its closure rows read the derived state; the `Related::of` calls and the `StoredItem` and `TicketNode` literals in `items_tests.rs` and `graph_tests.rs` gain the new field or argument.
- **E-18** (Task 4) `tests/repository_enablement.rs` (about 581-652): the registry tables list gains `pull_request_closures` and `pull_request_links`; the `ItemDto` literals (`src/desktop/model/drafts/create.rs`, `src/drafts/compare_tests.rs`, the desktop's save, editor-text, ticket-view, documents and edit tests, the CLI's `fill_tests.rs` and `human_tests.rs`) gain `pull_request`, and the `ClosureDto` literals gain `publication`.
- **E-19** (Task 11) `tests/cli/skills.rs` `AUTONOMY` and `skills/manyhands-overview/references/autonomy.md`: the new verb in `ticket close`'s column; (Task 12) `src/desktop/model/activity/copy/states.rs` `ACTION_WORDS` (23 to 24) and `activity_tests/states_copy.rs`, with the new `OperationAction`.
- **E-20** (Task 10) `tests/cli/lifecycle.rs` `application_data_holds_no_closed_by_or_title_beyond_the_index` gains a pull-request-mode row (an added row, no assertion changed).

Task 0's sweep adds what else it finds: tests that read a ticket's state inside an unfinished direct close, or with a closure only in its own context (each such edit reverses an intent, as E-16 does, and is listed for the owner); JSON assertions of item and closure objects; exhaustive matches on `Command`, `Mutation`, `PreviewDetailsDto`, `PreviewEffect`, `LifecycleBlocker`, `OperationAction`, `ClosureFilter`, `DependencyState`, `ReadinessReasonCode`; calls of `require_ticket_open`, `preserves_item_invariants`, `validates_resolution_sides`, `validates_external_integration`; counted lists of schema files, data roots, operation actions, effects and migration markers; goldens that hold `readiness.reasons`; and plan or critical-path tests that a removed closing ticket renumbers.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `ticket close` row (the pull-request path, its preview, `already_closing`, the new branch, its stops and cancellation as the existing rule); a taxonomy row for `ticket withdraw-closure`, and its rows in the recovery tables with their goldens; the Ctrl-C window list and `request cancel`'s reach for the new verb; "only `document promote` and `ticket close` are confirmed" gains the verb; `operation list`'s, `operation show`'s and `operation resume`'s answers for a runner row and its synchronizations; `closure.state`, `closure.publication`, `deps[].state` and `parent.state` `closing`; `--closure closing`; `closing_dependency` and the queries that leave closing tickets out; the item's `pull_request`; `item sync`'s link data; the new data and preview shapes; the success code `already_closing`.
- **D-2** `docs/RFC/canonical-content-and-comment-schema.md`: closure immutability applies once the primary holds the closure (RFC decision 20).
- **D-3** `docs/RFC/git-workflow-and-conflict-recovery.md`: the immutability rule in a ticket context's resolutions and external integrations, and the closure-only conflict that settles itself (changing Wave 02 Cycle 10's C11 for provisional closures).
- **D-4** `docs/RFC/ticket-relationships-and-short-codes.md`: closed and closing, the derivation from the primary's copy (amending "the index's effective copy" for closure), readiness, `closing_dependency`, and the queries that leave closing tickets out.
- **D-5** `docs/RFC/repository-index-persistence-and-refresh.md`: the two new registry tables and their migration.
- **D-6** `docs/research/wave-03-api-audit.md`: `already_closing` in `ticket close`'s codes; the new verb.
- **D-7** The shipped skills, in `skills/` and `.agents/skills/`: `manyhands-tickets` (close in pull-request mode, closing, withdraw, the new branch, a stopped close), `manyhands-finding-work` (the closing state, `--closure closing`, `closing_dependency`, the queries, `pull_request` and `publication` on `ticket show`), `manyhands-overview` (the three closure states; the new verb in `autonomy.md`), `manyhands-sharing-and-sync` (`item sync`'s link), `manyhands-recovery` (a stopped pull-request close is continued by its request). Each says that promote in a pull-request repository still pushes the primary until Cycle 04.
- **D-8** `docs/RFC/pull-request-mode.md`: dated amendments for decisions 6 (a surviving branch closes in place), 7 (a ticket with no context), 8 (a stopped close holds until its request continues; `item sync` finishes a publication only once nothing holds it), 9 (closing tickets left out of the queries), 10 (`closed_by` not in the answer) and 11 (withdraw of nothing is `already_applied`; one SSH case proves Required Evidence 1 and 2).
- **D-9** `docs/Waves/wave-06-pull-request-workflow.md`, Cycle 03's checklist, through `manyhands-documents` (the Wave document is a managed document): at approval, with the owner's go-ahead for its save and promotion.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** A separate runner and record for the pull-request close, not `ticket_closures` — the direct record's steps, its never-push rule (C1) and its child fencing are direct mode's — a second record to maintain.
- **R-2** The prepare's mode is bound by the digest's domain, an accepted request finishes in the path its record names, and each path's prepare blocks on the other's unfinished record — Cycle 01 decision 11's rule — a switch between prepare and confirm costs a re-prepare.
- **R-3** The runner's synchronizations are ordinary context synchronizations with IDs from its record, re-entered with `restart: true` — `ChildExecution` needs a direct parent (Baseline Fact 8) — a stopped child holds the remote as any stopped synchronization does (decision 8).
- **R-4** The runner's synchronizations are attributed to its row and cannot be resumed on their own — a resume outside the runner would leave the row behind — the guards in `read/status.rs`, `recovery.rs` and `bind/resume.rs` follow comment add's.
- **R-5** A stop answers as `item sync` answers that stop, with the close's own effects — the host and conflict actions then work unchanged — the stop's data does not say which close step stopped; the effects and the operation list do.
- **R-6** The closing state is derived from the two copies in both modes, and an open effective copy whose primary's copy is closed reads `closed` — the RFC defines it so (decision 11), and reads then need no mode — an interrupted direct close reads `closing` where it read `closed`.
- **R-7** The primary's copy is the root's working-tree row at rank `PRIMARY_COPY` — it is what the index stores — an uncommitted closure in the root reads `closed` while the guard reads the branch tips (K-9).
- **R-8** A new reason `closing_dependency` — the RFC says `ticket ready` and `ticket blocked` say so — one more enum value for callers.
- **R-9** `ticket plan` and `ticket critical-path` compute with closing tickets as open and leave them out of the output, renumbering batches — decision 9 — a dependent of a closing ticket sits in batch 1 or later without its blocker shown in the plan.
- **R-10** The guard reads the configuration's mode at each call, and `context_release` stays direct — RFC: in pull-request mode only the primary's copy refuses — a repository switched to direct with a provisional closure on a branch refuses writes to that ticket until a direct close merges it.
- **R-11** A save whose closure fields differ from the branch head's is refused with `invalid_input`, in both modes — the precheck no longer refuses a file that reads `closing` — a person who hand-edits those fields gets `invalid_input`.
- **R-12** Immutability is relaxed only in a remote synchronization of a ticket context, against its stage's fetched primary — primary targets and local synchronizations pin today's rule (Baseline Fact 7), and only a ticket's branch holds provisional closures — a resolution made before a fetch may drop a closure the forge has merged; the next synchronization merges it back from the primary.
- **R-13** Only a closure-only conflict settles itself, keeping the remote's closure, in `prepare.rs` — decision 2; a clean merge is built there (Baseline Fact 7) — the local `closed_at` and `closed_by` are lost without notice.
- **R-14** One helper deletes the link record, called by both retirement paths — a poll's retirement does not pass through `retire_context` (Baseline Fact 12) — a third retirement path added later must call it too.
- **R-15** `pull_request` and `closure.publication` are filled only by `ticket show` — lists read the index, which holds neither — every item golden changes once (E-4).
- **R-16** `already_closing` is class `None` with outcome `success`, appended last — exit 0 (decision 4), and appending keeps every code's index — a caller that expects `noop` for nothing written sees `success`.
- **R-17** Withdrawing a ticket with no closure is `already_applied` (decision 11), and withdrawing one the primary holds is `ticket_closed` with `already_closed` — the existing refusal for a closed ticket — none.
- **R-18** An unfinished runner row holds its ticket, is listed by `operation list` with the action `pull_request_close`, and is continued only by its request — a half-done new branch must not meet a second request — a person must find the request ID in the listing.
- **R-19** The unheld entries are computed per path against the one merge base, without rename detection, unbounded — the preview already lists the whole branch's changes unbounded — a rename shows as a deletion and an addition.
- **R-20** The new branch starts from the tracking primary as last fetched, bound by the digest — prepare reads local state only — a stale tracking primary gives a pull request behind the primary, which the forge shows.
- **R-21** The new branch keeps a provisional closure the old context held — the closure was already recorded and published once — none.
- **R-22** The desktop gets compile-level arms and the action word "Pull request close" only — Cycle 06 owns its design — the desktop shows a closing ticket as open, offers "Close ticket" (answering `already_closing`) and shows a generic preview.
- **R-23** The runner takes its clock as a parameter (`*_at(now)`), and production passes the wall clock — AGENTS.md: no real waits — none.
- **R-24** One SSH case, in `mutation_remote` beside the lifecycle cases (decision 11), and the new verb's conformance row in the `Confirmation` home with rules 1 and 2 exempt — a `Remote` home would need a second SSH case — those two rules are pinned only by P12.
- **R-25** The link is built from the publication remote's URL as `git2` reports it; P12 configures a `[forge] pull_request_url` template so its golden holds a URL — `forge::recognise` takes a URL, and the harness's URL is no forge's — an unusual URL gives no link, never a failure.
- **R-26** Cycle 02's `pull_request` schema and DTO are reused, or added identically — decision 5 — a merge of two identical files.
- **R-27** Step 3 checks the tracking primary, and reads and the guard check the local primary — the tracking primary is fresher after step 2, and reads need no network — after a forge merge and before the local primary moves, the ticket reads `closing` while withdraw refuses it as closed (K-7).
- **R-28** The new verb is delivered under the label `C5` and sits in `ticket close`'s autonomy column — the label set is pinned, and a later verb joined `C5` before (vh-hyksw); withdraw is consented as close is — none.
- **R-29** `item sync`'s link is new tagged data only when it publishes or finds current a closing ticket's branch in a pull-request repository — decision 12, with every other `item sync` answer unchanged — one more data shape for `item sync`'s callers.

## Known Limits

The Cycle document's K-1 to K-9.

## Unverified

- **U-1** Which remote row holds Part F step 2's reservation, and how `finish_parent`'s checks (`reservation.rs` about 1204) apply to it; Task 8's design checkpoint decides, under R-18's attribution rule.
- **U-2** Whether `promotion_test_drive_local` publishes to a local bare repository or only merges locally; Task 6's design checkpoint confirms what the test publisher adds.

## Alternatives Rejected

- Reusing `ticket_closures` with new steps: its CHECK lists and blocker mask need a table rebuild, and its C1 rule forbids exactly what this path does (R-1).
- Replacing a surviving remote branch: decision 6.
- Ending a stopped close on `request cancel`: decision 8.
- Listing closing tickets in `ticket ready`: decision 9.
- `closed_by` in the answer: decision 10.
- Relaxing immutability for every synchronization: R-12.
- A durable hold across the pull request's review: the RFC's anti-goal.

## Risks

| Risk | Mitigation |
| --- | --- |
| The runner's step and retry rules are wrong in a case the tables miss | Built test-first against the tables, with a design checkpoint (plan Tasks 6 to 8) |
| The validators' new parameter changes direct-mode merges | P5's strict rows; the SSH tier at review-ready |
| E-4 and E-18 touch many goldens and literals | One regeneration, the diff read: only the two null fields added |
| The SSH tier runs every SSH target (about two hours) | Once, at review-ready; between steps only single cases by exact name |
| Cycle 02 lands first with a different `pull_request` shape | Task 0 and Task 11 read main; a difference stops for the owner |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1 | source and feasibility | 5 | 14 | 15 | Resolved in revision 2: the link deleted by both retirement paths (B1, R-14); immutability relaxed only for a ticket context's stages (B2, R-12); step 3's order (B3); `context_created` reused (B4); the registration tests named and moved (B5, E-11 to E-13, E-18, E-19, R-24, R-28); cancellation and the children's attribution (I1, I2, decision 8, R-4); `restart: true` (I3, R-3); the test publisher (I4); Part G in `prepare.rs` (I5, R-13); the derivation's reach (I6, E-17); E-16 in full (I7); stop and refusal data (I8, R-5); `closed_by` (I9, decision 10); Part F's gaps (I10, F4, F5, F9); the merged-branch claim (I11); Task 2's test (I12); Task 4's edits (I13, E-18); the minors in place |
| 1 | contract and consistency | 5 | 15 | 19 | Resolved in revision 2: `closure.publication` (B1); the immutability rule (B2); step 3's order (B3); the departures as decisions 8 to 11 and D-8 (B4); cancellation as the CLI contract's rule (B5, decision 8); privacy (I6, decision 10); the closed primary with an open branch (I7, R-6); Part D's precedence (I8); Part F's steps (I9); the retry rule (I10, D11 to D13); stop data and record states (I11, R-5); the race (I12, D14); the publisher (I13); Cycle 01 decision 11 (I14, K-8); D-1's clauses (I15); withdraw's rows (I16); R-6 and R-7 (I17, decisions 9 and 11); the plan's split (I18); goldens (I19); `setup_pending` (I20); the minors in place |
