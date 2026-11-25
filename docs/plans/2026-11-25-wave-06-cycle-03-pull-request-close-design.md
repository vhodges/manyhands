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

Status: **draft** (revision 1).

## Intent And Approval Boundary

In a pull-request repository, make `ticket close` record the closure on the ticket's branch, publish the branch and give the pull request link, never committing to or pushing the primary. Show a closure the primary does not hold yet as `closing`, keep its dependents blocked, let it be withdrawn, let two clones' closures settle themselves, and close a ticket whose branch merged first (with its remote branch gone) or that has no branch, through a new branch holding only the closure. Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket branch, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `9a73bdbd`; Task 0 re-checks every one.
- "Pull-request repository" means a valid configuration with `integration = "pull_request"` (Wave 06 Cycle 01).
- "The tracking primary" is `refs/remotes/<publication remote>/<primary>`, as last fetched. "The local primary" is `refs/heads/<primary>`. "The primary's copy" of a ticket in a read is the copy the index stores for the primary context, which is the local primary's working tree.
- "A provisional closure" is a closure (`closed_at` and `closed_by`) that the ticket's branch holds and the relevant primary does not (RFC, Provisional closure).
- "The runner" is the new pull-request closure runner of Part D. Its three actions are `record` (close), `withdraw` and `new_branch` (Part F).

## Baseline Facts

Each fact was read in the source at `9a73bdbd` by the planning research and spot-checked by the author.

1. **The closure state.** `src/repository/read/dto.rs` `ClosureState { Open, Closed }` (a `contract_enum!`, about line 338) and `ClosureDto { state, closed_at, closed_by }`, documented "`closed_at` is set exactly when `state` is `closed`". `src/repository/read/items.rs` `closure()` (about line 1354) derives the state from `closed_at` alone. Lists build it from the effective row (`stored_item_dto`), `show` from the effective copy's file (`parsed_item_dto`).
2. **The effective copy.** `effective_rows` and `effective_copy` (`items.rs` about 2035-2120) keep one row per item, ranked by `row_rank`: the item's own worktree first, then the primary's copy. Every read holds all stored rows (`stored_items`, about 869), including the primary's copy of an item that has an active context (Table A of the fast-writes design, `docs/plans/2026-11-19-fast-writes-design.md` about line 189). No helper returns the primary's copy's closure for such an item.
3. **Readiness is boolean.** `src/repository/read/graph.rs` `TicketNode { id, closed: bool, parent, deps }` (about 54-61) is filled from `row.closed_at.is_some()`. `blockers`, `is_ready`, `readiness_state`, the open-cycle test, `unplannable`, `batches` and `critical_path` read it. A tree line's state (about 493-497) and the deps root (about 1103) are set from it. `ticket_readiness` (about 1029) keeps rows with `row.closed_at.is_none()`. `items.rs`'s private `Target { Document, Comment, OpenTicket, ClosedTicket }` (about 285) gives `deps[].state` and is matched exhaustively in `Related::check` (about 435).
4. **The closure filter.** `ClosureFilter { Open, Closed, All }` (`items.rs` about 45) is applied in memory by `TicketFilter::matches` to the effective row's `closed_at`. The CLI's value parser (`src/bin/manyhands-cli/parse.rs` about 86-88) takes `open|closed|all`, and `dispatch.rs` (about 58-62) maps any other value to `All` with a wildcard. The help text `open|closed|all` is pinned in three help goldens, `catalog_tests.rs` and `parse_tests.rs`.
5. **Exhaustive matches a new value breaks.** `ClosureState`: `src/desktop/model/relationships.rs` (about 336), `tickets.rs` (about 240), `relations_edit.rs` (about 327), and `tests/ticket_closure/journeys.rs` (about 390), `src/desktop/model/tickets_tests.rs` (about 136). `DependencyState`: `relationships.rs` (about 101 and 298). `ReadinessReasonCode`: `src/desktop/model/tickets.rs` (about 159). `ClosureFilter`: `src/desktop/view/tickets/filter_bar.rs`, `src/desktop/model/list_view.rs`, `src/desktop/model/ticket_view.rs`. `PreviewDetailsDto`: `src/bin/manyhands-cli/human_admin.rs` (about 98) and `src/desktop/view/confirm.rs`. `PreviewEffect`: `human_admin.rs` `effect_phrase` (about 165). `MutationDataDto`: `dto.rs` `authority`, `tests/support/mutation.rs` (six) and `tests/support/repair.rs` (one). Seven checks compare `== ClosureState::Closed` and would treat a new value as open, among them `bind/ticket.rs` `refuse_closed`, `src/drafts/compare.rs` `lifecycle_closed` and the desktop's "Close ticket" action.
6. **The closed-ticket guards.** `src/repository/ticket_guard.rs` `require_ticket_open(repo, primary_branch, item)` refuses `TicketClosed` when the local primary's tip or the context branch's tip holds either closure key. It takes no mode. Callers: `src/repository.rs` `refuse_closed_ticket` (about 299; used by `prepare_context`, `save_ticket_with_effective_config` and `checkpoint_comment_locally_with`), `context_release.rs` (about 231), `mutation/bind/republish.rs` (about 202) and `mutation/bind/resolve_history.rs` (about 226). The binding precheck `bind/ticket.rs` `refuse_closed` (about 237) refuses when the effective item reads `Closed`; it guards `ticket save`, slug assignment and context prepare. A save edits the worktree file with the draft's fields and carries `closed_at`/`closed_by` through from that file (`draft_of`, `holds`, about 296-345 and 1008-1031).
7. **Closure immutability.** `src/repository/integration/validate.rs` `preserves_item_invariants` (about 199) requires, for a ticket, `old.closed_at.is_none() || (old.closed_at == new.closed_at && old.closed_by == new.closed_by)`. `validates_resolution_sides` (about 224) applies it to base, local and incoming against a resolution, and `validates_external_integration` (about 505) against an external merge. Callers are in `remote/sync.rs` (about 3317, 3434, 4898, 5304), `remote/conflict_read.rs` (about 768) and `local_sync/` (`resolve.rs`, `run.rs`). `classify_conflict_entry` (about 171-184) gives `IncompatibleClosures` when two sides hold different closures, and `conflict_eligibility` (about 42) makes that `ExternalResolutionRequired`. A clean merge is committed without canonical validation (`remote/sync.rs` about 3792-3860). Canonical parse and serialize check only that both fields are present or absent (`src/canonical.rs` about 530 and 980).
8. **The direct close.** One runner (`src/repository/promotion/run.rs` `run_lifecycle`, `promote_inner`) with the durable parent `ticket_closures` (`src/repository/closure/record.rs` about 229, whose DDL is validated exactly). Its closure checkpoint is written under the repository lease immediately before the merge and never pushed on the branch (Wave 02 Cycle 10 decision C1); `written_closure_unfinished` (`record.rs` about 774) refuses any synchronization of the ticket context while a record is between `closing` and `branch_deleted`. Its children run through `synchronize_promotion_child_observed` with a `ChildExecution` (`promotion/lifecycle_record.rs` about 520-574), which reads only a promotion or closure parent. Its digest (`closure/observe.rs` `expectation`, about 1004) does not bind the mode. `closed_at` comes from `checkpoint.rs` `now_whole_utc_seconds()` (about 70), with no clock seam in execution. `build_closure` (`checkpoint.rs` about 138) reads the ticket from a given commit's tree and commits `Close ticket <ULID>` unreferenced; `move_context_to_checkpoint_with` (about 314) moves the context safely; `closed_ticket_bytes` (`observe.rs` about 75) writes the fields canonically. `closed_by` is `"<name> <<email>>"` of the configured identity (`observe.rs` about 46; C5 decision 6).
9. **A contained context today.** For a context contained in the base by ancestry and an open base ticket, the direct close records the closure and merges ("Merged by hand without closure fields: close it now", `closure/observe.rs` about 741). With a publication remote, a context whose remote branch was observed deleted and that is not contained gives the blocker `RemoteContextDeleted` (about 779-797), read from `remote_context_states` with no network (`promotion/observe.rs` `remote_context_history`, about 891).
10. **The synchronization.** `src/repository/remote/sync.rs` `synchronize_remote_observed` (about 6171, `item sync`'s entry) takes a fresh operation ID and makes its own reservation; it needs no parent record. A context target merges in two stages, the remote copy then the fetched primary (`remote/merge.rs` about 20-31), and publishes. It answers `Published`, `AlreadyCurrent` or `PublishPending`; or `Transport`, `PushRejected`, `ConflictPending`, `RemoteContextDeleted`. Push refspecs are unforced (`remote/refs.rs` about 99-106); a push the remote refuses as non-fast-forward is `push_rejected`, retried by the CLI (`src/bin/manyhands-cli/mutate.rs` `RETRY_DELAYS_MS`) and by the same request with `restart`, which fetches and merges again. Nothing in a synchronization resolves a conflict automatically. A synchronization that stops with an error keeps its reservation and its record stays accepted (`mutation/bind/sync.rs` module documentation, about 43-47); `RemoteBranchDeleted` and `MergeRequired` are let go only by `request cancel` (`mutation/outcome.rs` `never_gets_past`).
11. **Remote ref writes.** There is no guarded non-fast-forward replacement of a remote ref. The one guarded remote write besides a push is `transport/remote.rs` `delete_exact` (about 269), used only by close's and promote's cleanup under their parent's reservation.
12. **Retirement.** `src/repository/cleanup.rs` provides `ContextCleanup` (`acquire`, `full_check`, `remove_context_worktree` about 334, `delete_context_branch` about 477, `require_local_cleanup_absent` about 540, `delete_tracking_ref` about 572) and the SQL step `retire_context` (about 953), which deletes the item's `remote_context_states` row and upserts `retired_contexts`. After it, a push of a branch with the same name is a fresh publication: `fetch_publication_evidence_for` gives `NeverPublished` and `push_absence` skips the retired history (`remote/state.rs` about 2610-2632; `sync.rs` about 7963-8052).
13. **Making a branch.** `src/repository.rs` `prepare_context_unlocked` (about 2432) creates `manyhands/<kind>/<id>` only at the local primary's HEAD. `remote/poll.rs` `materialize_context_unlocked` (about 322) creates one at a given commit, but is private to the poll, needs a poll owner row and refuses a commit the primary holds.
14. **The content proof.** `src/repository/context_retirement/proof.rs` `preservation` (about 24) returns `TicketOpen` before any comparison when the primary's ticket is open. Its Folded loop passes when every path the branch changed since the one merge base equals the primary's tip; it keeps no list. Nothing lists what the primary lacks. `remote/sync.rs` `commits_missing_from` (about 4106) lists commits, not paths.
15. **Requests and operations.** A request record holds one operation (`mutation/bind/mod.rs` `Accepted::of_record`, about 529, takes the first). `comment add` allocates its child synchronization's ID in a binding table of its own (`src/repository/recovery.rs` `comment_publication_bindings`, about 47-62) and resumes it under that ID. `LifecycleBinding<Close>` (`bind/lifecycle.rs` about 99) binds `ticket close`. `MutationDataDto::from_stored` maps `ticket close` only to the lifecycle DTO (`dto.rs` about 2564). `operation list` reads the promotion and closure tables (`read/status.rs` about 584 and 639); `operation resume` reaches lifecycles through `bind/resume.rs` `found_lifecycle` (about 291) and answers `internal_error` for any other command (about 1307). `src/repository/recovery.rs` `LIFECYCLE_COMMANDS: [&str; 2]` (about 1040).
16. **Result codes.** `src/results.rs` `result_codes!` assigns each code a class; class `None` exits 0, and `Outcome::of_mutation` (about 737) gives `noop` only for `already_applied`, otherwise `success`. Ordered lists compared in order: `src/results_tests.rs` `DESIGN_CODES` (71) and the messages, `schemas/v1/envelope.schema.json` and `schemas/v1/request.schema.json` (71 each). `mutation/outcome.rs` `recovery` (about 778) matches every code. `tests/mutation_contract.rs` requires a golden or a listed reason per code.
17. **Preview shapes.** `PreviewEffect` (`dto.rs` about 160-198, 37 values in `schemas/v1/preview_effect.schema.json`, whose description counts a promotion's and a closure's seven). `PreviewDetailsDto` is untagged (about 248-261). `LifecycleBlocker` has 24 values. The closure blocker mask in `ticket_closures` is full (22 bits).
18. **The CLI's inventories.** `tests/cli/inventory.rs` counts mutating verbs (44), catalogued verbs (81) and verbs taking input (24), lists each verb's output schemas, parses the CLI contract's taxonomy, and `no_verb_is_planned` accepts only the delivery labels `C1` to `C5`. `tests/support/conformance.rs` `HOMES` (46) needs a row per bound command. `tests/mutation_contract.rs` `every_bound_command_has_a_golden`. Help listings follow the catalog's order.
19. **The link builder.** `src/forge.rs` `recognise(remote_url, setting)`, `pull_request_link(forge, source, target) -> PullRequestLink { source, target, forge, url }` and `human_line`. `PullRequestLink` is not `Serialize`, and nothing outside `src/forge.rs` uses it. It needs the remote's URL, not its name.
20. **Test seams.** The transport is SSH only (`transport/endpoint.rs` `parse_ssh_endpoint`); no library test can push or fetch through Manyhands without the SSH harness. Close's library tests use an unreachable SSH URL and hand-written tracking refs (`closure/test_support.rs`), and inject a publisher through `promotion/run.rs` `PromotionIo` with `run_with_io` (cfg(test)); `promotion/run_tests.rs` `FakeIo` completes a child by writing its row and the tracking ref. `context_retirement/acceptance_tests.rs` (about 313-360) uses a local bare repository driven by plain `git2` and feeds the poll with `poll::synthetic_fetch`. A poll classifies a deleted remote ticket branch as `Closed` when `proof::completion` passes on the primary (`poll.rs` about 279-304).
21. **Migrations.** `src/repository/discovery.rs` `migrate_registry` ends with `migrate_pull_request_mode` (about 2809), which adds its schema by checking it (Wave 06 Cycle 01's ledger, D-G) and records the marker `wave_06_pull_request_mode_v1`.

## Part A: The Closing State

**The value.** `ClosureState` gains `Closing => "closing"`, after `Closed`. `DependencyState` gains `Closing => "closing"`, after `Unresolved`; `parent.state` shares it. `ReadinessReasonCode` gains `ClosingDependency => "closing_dependency"`, after `DependencyCycle` (R-8). Each schema enum lists the values in that order (`schemas/v1/item.schema.json` `closure.state`, `parent.state`; `dependency.schema.json`; `dependency_tree_node.schema.json`; `readiness_reason.schema.json`). `ClosureDto`'s documentation becomes: "`closed_at` and `closed_by` are set when `state` is `closed` or `closing`."

**The derivation (R-6).** A read that has a ticket's effective copy and its stored rows decides:

| Effective copy | Primary's copy | State |
| --- | --- | --- |
| no `closed_at` | any | `open` |
| holds `closed_at`, and is the primary's copy | — | `closed` |
| holds `closed_at` (an active context's copy) | holds `closed_at` | `closed` |
| holds `closed_at` | open, or absent from the index | `closing` |

One helper in `items.rs` takes the effective row and the item's primary-copy row (found by rank among the rows the read already holds) and gives the state; lists and `show` both call it, and `show` passes the primary's stored row with the file it parsed. The derivation does not read the mode: in direct mode a branch-only closure exists only inside an unfinished direct close, which then reads `closing` instead of `closed`.

**Readiness.** `TicketNode` keeps `closed`, now true only for `closed`, and gains `closing: bool`. Every readiness and planning decision reads `closed`, so a closing ticket counts as open. A dependency edge to a closing ticket gives `closing_dependency` in place of `open_dependency`. Tree lines and the deps root report `closing`. `ticket_readiness` keeps tickets whose state is not `closed`, so a closing ticket is listed by `ticket ready` or `ticket blocked` by its own dependencies, and by `ticket plan` (R-7). `tickets_made_ready` is unchanged: a pull-request close makes nothing ready.

**The filter.** `ClosureFilter` gains `Closing`. `Open` matches `open` and `closing`; `Closing` matches only `closing`; `Closed` only `closed`. The filter is applied to the derived state. The CLI's value parser takes `open|closed|closing|all`, `dispatch.rs` maps `closing` explicitly, and the help text says so.

**Human output.** `item_line`, `item_show` and `dependency_line` print the state's `as_str()`, so they print `closing` unchanged.

**The desktop (R-20).** Compile-level arms only: `ClosureState::Closing` maps to `Lifecycle::Open`, `DependencyState::Closing` as `Open`, `ClosingDependency` as `OpenDependency`, and `ClosureFilter::Closing` gets an arm the desktop never selects. Cycle 06 designs the desktop's closing state.

## Part B: The Guards, The Save Check And Immutability

**The domain guard (R-9).** `require_ticket_open` gains the repository's integration: in a pull-request repository it reads only the local primary's tip; in direct mode it reads both tips, as today. `refuse_closed_ticket`, `bind/republish.rs` and `bind/resolve_history.rs` pass the mode of the configuration they hold; `context_release.rs` passes `Direct` (a closing ticket has a commit of its own, so it cannot be released anyway). So in a pull-request repository a closing ticket accepts `ticket save`, slug assignment, context prepare, `comment add`, `item republish` and `item resolve-history`, and a ticket the local primary holds closed refuses them with `ticket_closed`.

**The binding precheck.** `refuse_closed` keeps comparing with `ClosureState::Closed`, so it lets a closing ticket through in both modes, and the domain guard decides.

**The save check (R-10).** A ticket save, in both modes, refuses with `invalid_input` when the resulting file's `closed_at` or `closed_by` differ from the ticket at the context branch's head (or from an open ticket, for a context with no ticket yet). The check runs in the domain save under the lease, before the checkpoint. Without it a hand edit of the worktree file could record or change a closure: the precheck no longer refuses a file that reads `closing`.

**Immutability (R-11).** `preserves_item_invariants` gains the primary-side copy of the same path, as an optional parsed item. For a ticket, a change of closure from `old` to `new` is accepted when:

- `old` holds no closure; or
- `new` holds the same closure as `old`; or
- the primary-side copy does not hold `old`'s closure, and `new`'s closure is the primary-side copy's closure or none.

So a closure the primary-side copy holds stays immutable, and a provisional closure may be dropped (a withdraw) or replaced (Part G). `validates_resolution_sides` and `validates_external_integration` take the primary-side commit and pass its copy. The primary side is the merge's own primary input: the integration window's `primary_oid` (the fetched primary) in a remote synchronization, and the local primary in a local synchronization. `classify_conflict_entry` is unchanged: a conflict that holds two different closures and other conflicting content is still `IncompatibleClosures`, external repair only, but the external repair can now succeed (K-2).

## Part C: The Link Record And The Pull Request Object

**The object (decision 5).** `pull_request: { source, target, forge, url }`: `source` the ticket's branch, `target` the configured primary, `forge` a forge kind (`bitbucket-cloud`, `bitbucket-data-center`, `github`, `gitlab`) or null, `url` a string or null. Its schema is `schemas/v1/pull_request.schema.json` and its DTO `PullRequestDto`, both Cycle 02's (vh-c97se). Task 0 checks `origin/main`: if they are there this Cycle uses them; if not, this Cycle adds them in exactly that shape and under those names, and whichever Cycle lands second merges (R-24). `forge` is a schema enum registered in `contract_enumerations`.

**Building it (R-23).** From the publication remote's URL as `git2` reports it (`Remote::url`), the configuration's `forge()`, the branch `manyhands/ticket/<ULID>` and the primary, through `forge::recognise` and `forge::pull_request_link`. A missing URL never fails a step.

**The record (R-13).** A registry table, keyed per item, holding what this clone gave:

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

Every answer of the runner that carries a link upserts its row. `cleanup.rs` `retire_context` deletes the item's row in the same transaction, so the link ends when the branch is retired: after the merge by a poll's retirement or `context retire`, by the direct close's cleanup, and by Part F's retirement of an old context (whose new branch then writes it again). A withdraw keeps it, because the pull request may stay open. The table is created by a named, additive migration, marker `wave_06_pull_request_close_v1`, run after `migrate_pull_request_mode`, adding its tables by checking the schema (as Cycle 01's migration does), in an immediate transaction with a read-only precheck. The same migration creates Part D's `pull_request_closures`.

**On the item (R-14).** `ItemDto` gains `pull_request`, after `readiness`: the record's link in `ticket show` for a ticket that has one, and null otherwise, in every list, and for a document (Cycle 04 fills documents). `item.schema.json` requires it.

## Part D: The Pull-Request Close

**The hand-off (R-1, R-2).** `ticket close` keeps its verb and binding entry. Its prepare reads the configuration it already reads: in a pull-request repository it prepares the runner's `record` action (or Part F's `new_branch`), otherwise today's direct close. The runner's digest has its own domain, `manyhands pull request closure v1`, so a prepare made in one mode is `external_change` when confirmed in the other. An accepted request finishes in the path it began (Cycle 01 decision 11): its retry finds its own record in `pull_request_closures` or in `ticket_closures` and continues that one. The runner lives in `src/repository/closure/pull_request.rs` with its record; it shares `closed_ticket_bytes`, `build_closure`'s commit building and `move_context_to_checkpoint_with`, and does not use `ticket_closures`, `ChildExecution` or `written_closure_unfinished` (Baseline Fact 8).

**The record.** `pull_request_closures`, one row per confirmed request's operation:

| Column | Meaning |
| --- | --- |
| `operation_ulid` | primary key: the request's operation |
| `repository_id`, `item_ulid` | the ticket |
| `action` | `record`, `withdraw` or `new_branch` |
| `expectation` | the consented digest |
| `first_sync_ulid` | the first synchronization's operation (null for `new_branch`) |
| `publish_sync_ulid` | the publishing synchronization's operation |
| `step` | `confirmed`, `synchronized`, `retired`, `recorded` (unfinished); `published`, `already_closing`, `already_applied`, `refused`, `publication_pending`, `abandoned` (finished) |
| `closed_at` | the frozen time, whole UTC seconds, written before the commit (as C6) |
| `checkpoint_oid` | the closure or withdraw commit, written before the ref moves |

It never holds `closed_by`, a title, a body or a URL (Wave 02 Cycle 10's privacy rule). A unique partial index on unfinished rows per `(repository_id, item_ulid)` lets one unfinished row hold the ticket (R-17).

**Prepare** reads local state only, with no network, lease or write. The blockers, in order: not a ticket; an unfinished runner row or an unfinished direct close on the ticket (`recovery_required`, naming the holder's `request.retry`); a pending comment publication on the ticket (as C12); the context nonconforming, dirty or conflicted, or a synchronization conflict pending; the ticket closed on the local or tracking primary (`ticket_closed`, blocker `already_closed`); no identity (`identity_required`). There is no blocker for the root checkout's branch or cleanliness, because nothing touches the primary. Without blockers it previews:

| Situation | Action | Effects |
| --- | --- | --- |
| a context whose ticket holds no closure, and whose remote branch was not observed deleted | `record` | `context_synchronized`, `closure_recorded`, `context_published`, `pull_request_link` |
| a context whose ticket holds a closure | `record`, closure `existing` | `context_synchronized`, `pull_request_link` |
| a context whose remote branch was observed deleted | `new_branch` (Part F) | Part F |
| no context, and the ticket open on the tracking primary | `new_branch` (Part F) | Part F |

The digest binds the domain, the action, the item, the context branch, the primary branch, the remote, the context's head (or none), the tracking primary for `new_branch`, the closure effect (`record` with the `closed_by` text, or `existing` with both fields), and for `new_branch` the old context's tip and the unheld entries. It does not bind a `record` closure's `closed_at` (C6). The preview's details are a new `PreviewDetailsDto` variant, tagged `"type": "pull_request_closure"`, holding the action, the item and its slug, the branches, the remote, the context and tracking-primary OIDs, the closure effect, the retired context (Part F), the unheld entries (Part F) and the `pull_request` object.

**Confirm, `record` action.** Its required outcomes are the table below; the plan builds them test-first with a design checkpoint.

1. Re-observe and compare the digest (`external_change` on a difference, as today). Begin the row at `confirmed`, allocating both synchronization IDs.
2. Run the first synchronization: the ticket context's synchronization under `first_sync_ulid`, as `item sync` runs it (Baseline Fact 10). It merges the remote copy and the fetched primary, and publishes. Step `synchronized`.
3. Under the repository lease: the context clean and at its branch's head; then
   - the head's ticket holds a closure: step `already_closing`; the answer is `already_closing` (Part H) with the link;
   - the tracking primary's ticket holds a closure: step `refused`; `ticket_closed`, blocker `already_closed`;
   - otherwise: check the effective identity's `closed_by` against the consented text (`external_change`); take `closed_at` from the row, or the injected clock (R-21), and store it; build the closure commit on the head with `closed_ticket_bytes`, unreferenced; store `checkpoint_oid` (step `recorded`); move the context to it with `move_context_to_checkpoint_with`.
4. Run the publishing synchronization under `publish_sync_ulid`. Step `published`.
5. Upsert the link record and answer `ok` with the link. Refresh the index outside the lease, as a save does.

No step commits to the local primary or pushes it: the synchronizations target the context, and their primary stage only merges the fetched primary into the ticket's branch.

### Required Outcomes Of Part D

| # | Situation | Required outcome |
| --- | --- | --- |
| D1 | New request, context ticket open, primary open | `ok`; one `Close ticket <ULID>` commit on the branch, holding `closed_at` and `closed_by`; the branch published; `pull_request` in data; the row `published`; nothing held after; local primary and tracking primary unchanged by the close; no synchronization targets the primary |
| D2 | The branch holds a closure at prepare (its own earlier one, or another clone's) | preview `existing`; confirm's step 2 publishes anything pending; `already_closing`, exit 0, the link; no commit made |
| D3 | Step 2 brings in another clone's published closure | `already_closing`; no commit made |
| D4 | After step 2 the tracking primary holds the closure | `ticket_closed`, blocker `already_closed`; no commit made |
| D5 | Step 2 stops: transport, key, host, `push_rejected` | the synchronization's code and recovery; nothing written; the row stays `confirmed`; the same request resumes step 2 |
| D6 | Step 2 answers `remote_branch_deleted` | that code; nothing written; recovery: `request cancel`, then `ticket close --prepare` again, which offers Part F |
| D7 | Step 2 meets a conflict | `recovery_required` with the conflict, as `item sync` answers; nothing written; the row stays `confirmed` |
| D8 | Step 4 stops: transport, key, host | the synchronization's code and recovery, with data saying the closure is recorded and its publication pending; the row stays `recorded`; the ticket reads `closing`; the same request resumes step 4 |
| D9 | Step 4 answers `push_rejected` because another clone published first | as D8; the retry (the CLI's or the same request's) fetches, Part G keeps the published closure, publishes; the answer is `already_closing` when the branch no longer holds this request's closure, else `ok` |
| D10 | A crash after `closed_at` and `checkpoint_oid` are stored and before the ref moves; then the same request | the ref is moved to the stored commit (no second commit, the same `closed_at`); step 4; `ok` |
| D11 | A crash after the ref moved; then the same request | the head holds `checkpoint_oid`; step 4; `ok` |
| D12 | A new request while a row is unfinished | prepare `recovery_required` naming the holder's `request.retry`; nothing written |
| D13 | `request cancel` of a request whose row is unfinished | its stopped synchronization is cancelled as any is; the row ends `abandoned` before step 3's commit, `publication_pending` after it; nothing is reverted; afterwards `item sync` or a new `ticket close` publishes |
| D14 | Prepared in direct mode, confirmed after the switch (or the reverse) | `external_change`; nothing written |
| D15 | The identity changed between prepare and confirm | `external_change`; nothing written |
| D16 | Replay of a finished request | the stored envelope, read back through `from_stored` |

**Closing a merged branch whose remote branch survives (decision 6).** This is the `record` action. When the primary holds the branch's work by ancestry or by the Folded content proof, step 2's merge of the fetched primary is clean and leaves the branch's tree equal to the primary's (Baseline Fact 14's loop: every path the branch changed equals the primary's entry), so the closure commit is the only change against the primary. Otherwise step 2 can conflict, and D7 applies.

## Part E: Withdraw

`ticket withdraw-closure --id <ULID>`: a confirmed verb, `--prepare` and `--confirm` as close, no input, the same runner with the action `withdraw`, in either mode.

**Prepare.** The blockers of Part D, except that a ticket whose context ticket holds no closure previews nothing to do: the answer is `already_applied` (R-16), with nothing to confirm. Otherwise the effects are `context_synchronized`, `closure_withdrawn`, `context_published`, and the digest binds the domain, the action, the item, the branch, the remote and the closure being withdrawn (both fields).

**Confirm.** Steps 1, 2 and 4 of Part D. Step 3, under the lease: the head's ticket holds no closure: `already_applied`; the tracking primary's ticket holds a closure: `ticket_closed`, blocker `already_closed` (withdrawing it would be a reopen); a different closure than the consented one: `external_change`; otherwise the commit `Withdraw closure of ticket <ULID>`, whose ticket is the head's with both fields removed, serialized canonically, built unreferenced, recorded, then the ref moved. The answer is `ok` with the link (the pull request may stay open) and the record kept.

| # | Situation | Required outcome |
| --- | --- | --- |
| W1 | A closing ticket | `ok`; one withdraw commit; the branch published; the ticket reads `open`; its dependents read blocked by `open_dependency`; the link record kept |
| W2 | No closure on the branch | `already_applied` at prepare, or at step 3 after another clone's withdraw arrived; nothing written |
| W3 | The tracking primary holds the closure | `ticket_closed`; nothing written |
| W4 | An unfinished direct close or runner row holds the ticket | prepare `recovery_required`, naming the holder |
| W5 | Step 4 stops | as D8 |
| W6 | Another clone merges a withdraw into its unchanged closure | the clean merge keeps the withdraw; a conflicting merge accepts a resolution that keeps it (Part B) |
| W7 | After W1, `ticket close` again | Part D's `record`, a new `closed_at` |

## Part F: The New Branch

Used in a pull-request repository by close for a ticket whose context's remote branch was observed deleted (its branch merged before it was closed, decision 1, narrowed by decision 6), and for a ticket with no context (decision 7).

**Prepare.** Besides Part D's blockers:

- the tracking primary must hold the ticket, open; otherwise the first case keeps today's blocker `remote_context_deleted` (the person republishes with `item republish` instead) and the second answers `context_missing`;
- an old context must pass the cleanup's full check (clean, unlocked, its head at its branch), or it is `recovery_required`.

The preview names the tracking primary `P` the new branch starts from, the old context's tip `T` when there is one, and the **unheld entries**: each path whose entry differs between the one merge base of `T` and `P` and `T`, and whose entry in `T` differs from `P`'s, with its change (`added`, `modified`, `deleted`), unbounded and sorted by path (R-18). An empty list means the content proof passes. History that has no single merge base is `recovery_required`. A new function in `proof.rs` computes the list from the existing private `entries`; it treats the ticket file like any other path. The effects are `worktree_removed` and `branch_deleted` for an old context, then `context_created`, `closure_recorded`, `context_published`, `pull_request_link`.

**Confirm.**

1. Re-observe and compare (`external_change` when `P`, `T` or the unheld entries changed). Begin the row with `closed_at` from the injected clock (stored, as C6) and `publish_sync_ulid`.
2. With an old context: retire it with the cleanup's functions, in the order promotion and closure use (Baseline Fact 12): worktree removal, local branch deletion, the tracking ref's absence, and `retire_context`. Step `retired`. Nothing on the remote changes.
3. Under the repository lease: if `manyhands/ticket/<ULID>` exists and its head's ticket holds a closure, skip to step 4 (an earlier attempt, or another request, made it); if it exists otherwise, `recovery_required`. Else build the closure commit on `P` (the ticket from `P`'s tree with both fields), record `checkpoint_oid` (step `recorded`), create the branch at it, add its worktree at `.manyhands/worktrees/<ULID>`, and mark the contexts dirty. The function that creates a context branch at a given commit is new, beside `prepare_context_unlocked` (Baseline Fact 13).
4. Publish under `publish_sync_ulid`: a fresh publication (Baseline Fact 12). Step `published`.
5. Upsert the link record; answer `ok` with the link.

| # | Situation | Required outcome |
| --- | --- | --- |
| F1 | Old context, remote branch observed deleted, content proven merged | preview with no unheld entries; confirm: the old worktree and branch gone, a branch whose only commit over `P` is the closure, published, `ok` with the link; nothing deleted on the remote |
| F2 | As F1, content unproven (a squash after which the primary moved on, or a declined pull request whose branch was deleted) | the preview lists the unheld entries; after confirm, as F1; the old branch's commits are no longer referenced (K-7) |
| F3 | No context, ticket open on the tracking primary | no retirement; as F1 otherwise |
| F4 | The tracking primary lacks the ticket | `remote_context_deleted` (old context) or `context_missing` (none) |
| F5 | `P`, `T` or the old worktree changed between prepare and confirm | `external_change` or the cleanup's refusal; nothing changed |
| F6 | A crash after step 2; then the same request | continues at step 3; one branch, one closure, the stored `closed_at` |
| F7 | A new request while the row is unfinished | `recovery_required` naming the holder's `request.retry` |
| F8 | Step 4 stops | as D8; the branch is local and `closing`; the same request resumes the publication |
| F9 | The branch already exists holding a closure at step 3 | step 4, then `already_closing` |

## Part G: Two Clones

**Settling a closure-only conflict (decision 2, R-12).** In a remote synchronization of a ticket context, the merge of the remote copy into the local branch (the Context stage, `remote/sync.rs` `integrate_divergence`) settles a conflict itself when all of these hold:

1. the only conflicted path is the ticket's own `ticket.md`;
2. the local (ours) and remote (theirs) tickets both parse and both hold closures, and the two closures differ;
3. the integration window's primary (the fetched primary) does not hold a closure of the ticket;
4. with ours' ticket rewritten to hold theirs' closure (canonically), the three-way merge of that path is clean.

The result takes the clean merge of step 4, so the remote branch's closure (the one published first) is kept and the local one is dropped. It is committed as the stage's ordinary clean merge, and the synchronization carries on as for a clean merge. Every other conflict is unchanged. The function that decides and builds the result is pure (repository, base, ours, theirs, primary, item) and lives beside `classify_conflict_entry`.

| # | Situation | Required outcome |
| --- | --- | --- |
| G1 | Two different provisional closures, nothing else conflicting | clean; theirs kept; the synchronization publishes; the ticket reads `closing` with theirs' fields |
| G2 | Different closures, and another conflicting hunk in the same file or another path | unchanged: `IncompatibleClosures`, external repair, now possible (Part B) |
| G3 | The fetched primary holds one of the closures | unchanged: external repair |
| G4 | The same closure on both sides | unchanged: a clean merge |
| G5 | Base closed, ours withdrawn, theirs unchanged | unchanged: a clean merge keeps the withdraw (W6) |
| G6 | Base closed with `A`, ours re-recorded `B` after a withdraw, theirs `A` | unchanged: a clean merge keeps `B` (theirs did not change the lines) |

## Part H: Codes, Data, CLI And Contract

**`already_closing` (decision 4, R-15).** A new result code, class `None` (exit 0), outcome `success` (it is not added to `noop`, because its step 2 can publish), message "The ticket's branch already holds a closure.", appended after `already_applied` in every ordered list. Its recovery is the shared arm's (none unless retried).

**The data (decision 3).** A new `MutationDataDto` variant, `PullRequestClosureMutationDto`, tagged `"type": "pull_request_closure"` (a `contract_enum`, as Cycle 01's tag), with:

- `action`: `record`, `withdraw` or `new_branch`;
- `closure`: `{ recording, closed_at, closed_by }`, `recording` one of `recorded`, `existing`, `withdrawn`;
- `checkpoint_oid`: the commit this request made, or null;
- `publication`: `published`, `current` or `pending`;
- `retired_context`: the old context's tip, or null;
- `pull_request`: the object, or null for `withdraw` with no link record.

Its schema is `schemas/v1/pull_request_closure_mutation.schema.json`, in `DATA_ROOTS`. A tag-guarded `from_stored` arm for `ticket close`, `ticket withdraw-closure` and `operation resume` comes before their general arms. A step that stops on a synchronization answers that synchronization's code with this data (`publication` `pending`), and a conflict's data as `item sync` gives it.

**The verb.** `Command::TicketWithdrawClosure` (`"ticket withdraw-closure"`), confirmed, with an operation, stopping at safe points, finishing as a synchronization; `Mutation::TicketWithdrawClosure(TicketWithdrawClosureInput { root, id })`; a catalog entry taking `--repo`, `--id`, `--request-id`, `--prepare`, `--confirm` and no input; dispatch, parse and human output as `ticket close`. `LIFECYCLE_COMMANDS` gains it (R-17).

**Operations (R-17).** An unfinished runner row is listed by `operation list` with the action `pull_request_close` (a new `OperationAction` value and `operation.schema.json` enum value), and `operation resume` of it answers `original_request_required`, naming the request.

**Preview effects.** `PreviewEffect` gains `context_published`, `pull_request_link`, `closure_withdrawn` and `context_created`, appended; `effect_phrase` and the schema description cover them.

**Human output.** The preview prints its effects, the closure, and for `new_branch` each unheld entry as `not on <primary>: <change> <path>`, through the output's field escaping. The answer prints `closure: recorded <closed_at> by <closed_by>`, `closure: already recorded`, or `closure: withdrawn`; `publication: <state>`; and the link's `human_line()` on its own line.

**The desktop.** Compile-level arms only for the new preview variant, data variant and verb (R-20).

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | Part A's derivation table, both copies, lists and `show`; `deps[].state`, `parent.state` and tree lines `closing` | `tests/read_items.rs`, one table-driven test, beside `presentation_across_contexts` |
| P2 | A dependent of a closing ticket is blocked with `closing_dependency`, and ready once the primary's copy holds the closure; `ticket ready`, `blocked` and `plan` list a closing ticket as open | `tests/read_relationships.rs`, one table-driven test |
| P3 | `--closure open`, `closing`, `closed`, `all` over open, closing and closed tickets; the parser and help | `tests/read_items.rs`; `parse_tests.rs`; the help goldens |
| P4 | The guard per mode (Part B), the save check, comments and saves on a closing ticket | `src/repository/ticket_guard_tests.rs`, a table and the save check; a binding test in `tests/mutation_bridges/context.rs` |
| P5 | Part B's immutability rule for each caller kind | `src/repository/integration/tests.rs`, a table of `preserves_item_invariants` and one row of each validator |
| P6 | The link record: migration once, upsert, delete on retirement; `ticket show`'s `pull_request` | a migration test beside Cycle 01's in `src/repository/discovery.rs`; `src/repository/cleanup_tests.rs`; `tests/read_items.rs` |
| P7 | D1 to D16 | `src/repository/closure/pull_request_tests.rs`, with an injected publisher and clock |
| P8 | W1 to W7 | the same file |
| P9 | F1 to F9, and the unheld entries | the same file; a table test in `src/repository/context_retirement/tests.rs` |
| P10 | G1 to G6 | `sync_tests.rs`'s `closure_conflict_classification` module, the pure function's table, and one synchronization-level row |
| P11 | The owner's lifecycles, end to end in the library: record, publish, a forge merge (merge commit, squash, fast-forward), then a poll answers closed and retirement succeeds; the recording clone before it fetches reads `closing`; declined, withdrawn, recorded again; a dependent blocked until the primary holds the closure; the merged-first branch with its remote branch kept (Part D) and gone (Part F) | `pull_request_tests.rs`, with a local bare repository driven by `git2` as the forge and `synthetic_fetch` for the poll |
| P12 | End to end over SSH: close, close again, withdraw, close again in a pull-request repository; the server's primary unchanged throughout; the goldens | one SSH case in `mutation_remote` |
| P13 | The CLI contract with no network: the preview of each action, `already_applied` for withdraw, the blockers | `tests/cli/lifecycle.rs` with the committed-configuration bypass and an unreachable SSH URL |
| P14 | Every registration (codes, enums, schemas, verbs, goldens, inventories) | the existing registration tests |

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). P12 is the only SSH case: no non-SSH publication remote can be selected (Baseline Fact 20), so only a case over the harness can produce the success goldens, and it shows that the server's primary is never pushed in pull-request mode (the RFC's Required Evidence 1 and 2, as Cycle 01 decision 6 showed it). Its edge cases are in P7 to P11. No real waits: the runner takes its clock, and P11 moves no time. The injected publisher of P7 to P11 refuses a synchronization whose target is the primary, and every runner test asserts that the local primary and the tracking primary did not move by the runner's doing; that is how "a test fails if one tries" holds in the library.

At review-ready the SSH tier is: `ticket_closure` (`src/repository/closure/`), `mutation_remote` (`src/repository/mutation/`), the six targets of `src/repository/remote/` (`ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`), `--lib repository::local_sync::` with `remote_synchronization` and `remote_merge_recovery` (the validators it calls), `comment_publication` (the guard in `checkpoint_comment_locally_with`) and `document_promotion` (`cleanup.rs` and `retire_context`). That is every SSH-harness target except `ssh_fixture`, so Task 13 runs them all.

## Named Edits To Pre-Existing Tests

- **E-1** (Task 1) `tests/read_contract.rs` `contract_enumerations`: `closure.state`, `parent.state`, the dependency and tree-node `state`, and the readiness reason `code` gain their new values, in order.
- **E-2** (Task 1) the exhaustive `ClosureState` matches in `tests/ticket_closure/journeys.rs` and `src/desktop/model/tickets_tests.rs`: a `Closing` arm that fails the test, as an unexpected state.
- **E-3** (Task 1) `open|closed|all` to `open|closed|closing|all` in `help_ticket_list.json`, `help_resource_ticket.json`, `help_root.json`, `catalog_tests.rs` and `parse_tests.rs` (whose valid-value list gains `closing`).
- **E-4** (Task 3) every item golden under `tests/fixtures/read_v1/` and `tests/fixtures/cli_v1/` gains `"pull_request": null`, regenerated with `MANYHANDS_UPDATE_GOLDEN=1` for the tests that own them and read before committing.
- **E-5** (Task 4) `src/results_tests.rs`: `already_closing` in `DESIGN_CODES` (71 to 72) and the messages.
- **E-6** (Task 4) `tests/mutation_contract.rs`: `already_closing` in both `CODES_` tables with the reason "golden written in Task 11"; **E-7** (Task 11) removes it.
- **E-8** (Task 4) `tests/support/mutation.rs` (six arms) and `tests/support/repair.rs` (one): the `PullRequestClosure` variant.
- **E-9** (Task 4) `tests/read_contract.rs`: the `preview_effect` enum's four values, the data's `type` enum, the preview's `type` enum, the `pull_request` `forge` enum (unless Cycle 02 registered it), the operation `action` value.
- **E-10** (Task 9) `src/repository/mutation/command.rs` tests that hard-code command sets (about 770, 870, 899, 1018): `ticket withdraw-closure` in the stopping, confirmed and synchronizing sets.
- **E-11** (Task 9) `tests/cli/inventory.rs`: mutating 44 to 45, catalogued 81 to 82, `NO_INPUT` and `DATA` rows for the new verb and `ticket close`'s new schema, and `no_verb_is_planned`'s labels gain the new verb's delivery label `W06C03`.
- **E-12** (Task 9) `tests/support/conformance.rs` `HOMES` (46 to 47): the new verb's home row.
- **E-13** (Task 9) the help listings: `help_root.json`, `help_root_text.txt`, `help_resource_ticket.json`, and a new `help_ticket_withdraw_closure.json` in `tests/cli/help.rs`'s list.
- **E-14** (Task 2) `src/repository/ticket_guard_tests.rs`: the 13 calls pass `Integration::Direct`; the integration tests' calls of the two validators and `preserves_item_invariants` pass no primary where they test direct behaviour.
- **E-15** (Task 12) `tests/cli/recovery_tables.rs` and `tests/cli/collaboration_exits.rs`, the audit's C5 counts, when D-6 adds `already_closing` to `ticket close`'s codes and the new verb's row.
- **E-16** (Task 1) **an edit that reverses an assertion's intent, for the owner's approval.** `tests/read_relationships.rs` `ready_and_blocked_are_decided_across_primary_and_two_item_worktrees` (about line 309: "C waits for A, which is closed where it counts: in A's own context. The copy of A on the primary branch, still open, decides nothing.") and `closing_a_dependency_makes_its_dependent_ready_without_the_dependent_being_observed_again` (about line 974: A closed in its own worktree unblocks B). Under Part A a closure that only a ticket's own context holds reads `closing` and blocks its dependents, as the RFC's provisional closure requires. The first test is edited to assert A `closing`, C blocked by `closing_dependency` and D unchanged, then A's closure committed on the primary too and C ready; the second to assert B still blocked after A's worktree closure, and ready once A's closure is on the primary.

Task 0's sweep adds what else it finds: tests that read a ticket's state inside an unfinished direct close, or with a closure only in its own context (now `closing`; each such edit reverses an intent, as E-16 does, and is listed for the owner); tests that build `ItemDto` literals; JSON assertions of item objects; exhaustive matches on `Command`, `Mutation`, `PreviewDetailsDto`, `PreviewEffect`, `OperationAction`, `ClosureFilter`, `DependencyState`, `ReadinessReasonCode`; calls of `require_ticket_open`, `preserves_item_invariants`, `validates_resolution_sides`, `validates_external_integration`; counted lists of schema files, data roots, operation actions and migration markers; and goldens that hold `readiness.reasons`.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `ticket close` row (the pull-request path, its preview, `already_closing`, the new branch); a taxonomy row for `ticket withdraw-closure`; `closure.state` and `deps[].state` `closing`; `--closure closing`; `closing_dependency`; the item's `pull_request`; the new data and preview shapes; `operation list`'s new action; the success code `already_closing`.
- **D-2** `docs/RFC/canonical-content-and-comment-schema.md`: closure immutability applies once the primary holds the closure (RFC decision 20).
- **D-3** `docs/RFC/git-workflow-and-conflict-recovery.md`: the immutability rule in resolutions and external integration, and the closure-only conflict that settles itself.
- **D-4** `docs/RFC/ticket-relationships-and-short-codes.md`: closed and closing, readiness, `closing_dependency`.
- **D-5** `docs/RFC/repository-index-persistence-and-refresh.md`: the two new registry tables and their migration.
- **D-6** `docs/research/wave-03-api-audit.md`: `already_closing` in `ticket close`'s codes; the new verb.
- **D-7** The shipped skills, in `skills/` and `.agents/skills/`: `manyhands-tickets` (close in pull-request mode, closing, withdraw, the new branch), `manyhands-finding-work` (the closing state, `--closure closing`, `closing_dependency`, `pull_request` on `ticket show`), `manyhands-overview` concepts (the three closure states), `manyhands-recovery` (an unfinished pull-request close is retried by its request). Each says that promote in a pull-request repository still pushes the primary until Cycle 04.
- **D-8** `docs/RFC/pull-request-mode.md`: dated amendments for decisions 3 (the link from `ticket close` and `ticket show`, not `item sync`), 6 (a surviving branch closes in place) and 7 (a ticket with no context).
- **D-9** `docs/Waves/wave-06-pull-request-workflow.md`, Cycle 03's checklist. Applied at approval.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** A separate runner and record for the pull-request close, not `ticket_closures` — the direct record's steps, its never-push rule (C1) and its child fencing are direct mode's — a second record to maintain.
- **R-2** The prepare's mode is bound by the digest's domain, and an accepted request finishes in the path its record names — the same rule as Cycle 01 decision 11 — a switch between prepare and confirm costs a re-prepare.
- **R-3** The runner's synchronizations are ordinary context synchronizations with IDs from its record, as `comment add`'s child is — `ChildExecution` needs a direct parent (Baseline Fact 8) — a stopped child holds the remote as any stopped synchronization does.
- **R-4** A close stopped by a synchronization leaves what that synchronization leaves, and the same request continues it; the RFC's "no operation, lease or reservation behind" holds for a close that answers `ok`, `already_closing` or a refusal — the synchronization's own contract (Baseline Fact 10) — a transport failure holds the remote until the request is retried or cancelled.
- **R-5** `request cancel` after the closure commit ends the row `publication_pending`, and then `item sync` or a new `ticket close` publishes — this is the RFC's "`item sync` finishes it" — before the cancel, `item sync` meets the stopped synchronization.
- **R-6** The closing state is derived from the two copies in both modes — the RFC defines it so, and reads then need no mode — an interrupted direct close reads `closing` where it read `closed`.
- **R-7** A closing ticket's own readiness is an open ticket's, and it is listed by `ticket ready`, `blocked` and `plan` — the RFC treats closing as open — an agent may see a ticket waiting for its merge as ready to work.
- **R-8** A new reason `closing_dependency` — the RFC says `ticket ready` and `ticket blocked` say so — one more enum value for callers.
- **R-9** The guard reads the configuration's mode at each call, and `context_release` stays direct — RFC: in pull-request mode only the primary's copy refuses — a repository switched to direct with a provisional closure on a branch refuses writes to that ticket until a direct close merges it.
- **R-10** A save whose closure fields differ from the branch head's is refused with `invalid_input`, in both modes — the precheck no longer refuses a file that reads `closing`, so a hand edit could otherwise record a closure — a person who hand-edits those fields gets `invalid_input`.
- **R-11** The primary side of immutability is the merge's own primary input — it is what the merge brings in, and in a pull-request repository the local primary only fast-forwards to it — a resolution made before a fetch may drop a closure the forge has merged; the next synchronization merges it back from the primary.
- **R-12** Only a closure-only conflict settles itself, keeping the remote's closure — decision 2 — the local `closed_at` and `closed_by` are lost without notice.
- **R-13** The link record is deleted by `retire_context` — one place every retirement passes — a link given for a branch that is never retired stays until it is.
- **R-14** `pull_request` is a top-level item field, filled only by `ticket show` — lists read the index, which does not hold it, and Cycle 04 fills documents — every item golden changes once (E-4).
- **R-15** `already_closing` is class `None` with outcome `success`, appended last — exit 0 (decision 4), and appending keeps every code's index — a caller that expects `noop` for nothing written sees `success`.
- **R-16** Withdrawing a ticket with no closure is `already_applied`, and withdrawing one the primary holds is `ticket_closed` with `already_closed` — idempotent, and the existing refusal for a closed ticket — no new code for these.
- **R-17** An unfinished runner row holds its ticket, is listed by `operation list`, and is continued only by its request (`operation resume` answers `original_request_required`) — a half-done new branch must not meet a second request — a person must find the request ID in the listing.
- **R-18** The unheld entries are computed against the one merge base, unbounded — the preview already lists the whole branch's changes unbounded — a large branch gives a long preview.
- **R-19** The new branch starts from the tracking primary as last fetched — prepare reads local state only and binds it — a stale tracking primary gives a pull request behind the primary, which the forge shows.
- **R-20** The desktop gets compile-level arms only — Cycle 06 owns its design — the desktop shows a closing ticket as open and a generic preview for the new path.
- **R-21** The runner takes its clock as a parameter (`*_at(now)`), and production passes the wall clock — AGENTS.md: no real waits — none.
- **R-22** One SSH case, in `mutation_remote` beside the lifecycle cases — Testing — its run time.
- **R-23** The link is built from the publication remote's URL as `git2` reports it — `forge::recognise` takes a URL — an unusual URL gives no link, never a failure.
- **R-24** Cycle 02's `pull_request` schema and DTO are reused, or added identically — decision 5 — a merge of two identical files.
- **R-25** Step 3 checks the tracking primary, and reads and the guard check the local primary — the tracking primary is fresher after step 2, and reads must not need a network — after a forge merge and before the local primary moves, the ticket reads `closing` while withdraw refuses it as closed.

## Known Limits

The Cycle document's K-1 to K-8.

## Unverified

- **U-1** That a synchronization stopped by a transport error before any push keeps its reservation, as `bind/sync.rs`'s module documentation says for an error; D5 and D8 follow whatever Task 0 finds, and the ledger records it.
- **U-2** That `item sync` of a context needs the root checkout neither on the primary nor clean; if it does, the runner inherits that refusal at step 2, and Part D's preview adds the blocker.
- **U-3** The exact `from_stored` arm order for three commands sharing one tag; Task 9's design checkpoint confirms it.

## Alternatives Rejected

- Reusing `ticket_closures` with new steps: its CHECK lists and blocker mask need a table rebuild, and its C1 rule forbids exactly what this path does (R-1).
- Replacing a surviving remote branch: decision 6.
- Extending the lifecycle data and `item sync`'s data with `pull_request`: decision 3.
- A durable hold across the pull request's review: the RFC's anti-goal.
- A new code for "nothing to withdraw": `already_applied` says it (R-16).

## Risks

| Risk | Mitigation |
| --- | --- |
| The runner's step and retry rules are wrong in a case the tables miss | Built test-first against the tables, with a design checkpoint (plan Tasks 5 to 7) |
| The validators' new parameter changes direct-mode merges | P5 rows for direct closures; the SSH tier at review-ready |
| E-4 touches many goldens | One regeneration, the diff read: only `"pull_request": null` added |
| The SSH tier runs every SSH target (about two hours) | Once, at review-ready; between steps only single cases by exact name |
| Cycle 02 lands first with a different `pull_request` shape | Task 0 reads main; a difference stops for the owner |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
