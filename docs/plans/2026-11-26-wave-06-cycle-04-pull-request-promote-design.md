---
id: 01M8BXXGDN9HPD8BM9SJTHHVB2
title: 'Wave 06 Cycle 04 Design: Document Promote In Pull-Request Mode'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 04 Design: Document Promote In Pull-Request Mode

Cycle document: [Wave 06 Cycle 04](../Cycles/wave-06-cycle-04-pull-request-promote.md). Ticket: vh-rphcs (`01M87684C9GM9BJP5QQ429KMAK`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md), the Cycle document's Product-Owner Decisions, and [Wave 06 Cycle 03](../Cycles/wave-06-cycle-03-pull-request-close.md)'s design, whose branch records, link record and item field this Cycle builds on.

Status: **draft**, revision 1, 2026-11-26. Not yet reviewed.

## Intent And Approval Boundary

In a pull-request repository, make `document promote` synchronize and publish the document's branch and answer with the pull request link, writing nothing on the branch and never committing to or pushing the primary. Promoting again while the link is outstanding synchronizes again and answers `already_applied` with the same link. A document counts as promoted, in promote, `document show` and `item sync`, only when the content proof shows that a primary holds the branch's content, so a squash merge counts and an edit of a merged document does not count before its own pull request merges. The pull-request promote is a mode of the existing document-promotion lifecycle (decision 1). Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket branch, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `bef571fd` (Cycle 03's branch, which holds main `6c7937c8`); Task 0 re-checks every one after Cycle 03 merges.
- "Pull-request repository" means a valid configuration with `integration = "pull_request"` (Wave 06 Cycle 01). Such a configuration always names a publication remote.
- "The local primary" is `refs/heads/<primary>`. "The tracking primary" is `refs/remotes/<publication remote>/<primary>`, as last fetched.
- "The branch" is `manyhands/document/<ULID>`. "`L`" is its local tip. "`R`" is its remote tracking tip, `refs/remotes/<publication remote>/manyhands/document/<ULID>`, when that ref exists.
- "The proof" is `context_retirement/proof.rs` `preservation` (Baseline Fact 11). "The primary holds the document" is the rule of Part B, R-3.
- "A link row" is the document's row in `pull_request_links` (kind `document`). "An outstanding link" is a link row while the primary does not hold the document (decision 3).
- "A branch record" is a lifecycle record whose path is `RecordPath::Branch` (Cycle 03). "A pull-request promotion" is a `document_promotions` row whose `integration_mode` is `pull_request`. "A direct promotion" is today's.
- "Stops" means a step ended without the lifecycle's operation ending: the request record stays accepted, the promotion record stays unfinished, and the same request continues it.

## Baseline Facts

Read in the source at `bef571fd` by the planning research and re-checked by the author.

1. **Promote's prepare.** `LifecycleBinding::prepare` (`mutation/bind/lifecycle.rs`) calls `Promote::observe` (`bind/lifecycle_verb.rs` about 291), then `RepositoryService::preview_document_promotion` (`promotion/mod.rs` about 501), `observe::preview` and `prepare_for` (`promotion/observe.rs` about 42 and 346), `observe_git` (about 948) and `expectation` (about 1367). The digest's domain is `manyhands promotion v1` and binds the item, the document's path, the context and primary branches, the remote, the planned integration, the change set and the cleanup; it binds the OIDs only when there is no publication remote. The binding's digest wraps it (`bind/lifecycle.rs` `digest_of`, about 197). Nothing in `promotion/` reads the repository's mode.
2. **"Already promoted" today is ancestry.** `planned_result` (`observe.rs` about 918) gives `Contained` when the context is an ancestor of the base, the tracking primary when there is a remote (about 1044); confirm then answers `ok` with `integration: already_contained` and cleans up (`tests/cli/lifecycle.rs` `a_context_primary_already_holds_is_only_cleaned_up`, about 626). No promote path answers `already_applied`, except `operation resume` of a complete record (`bind/resume.rs` about 256 and 973). After a squash merge the context is not an ancestor, so a direct promote plans a merge again.
3. **Promote's blockers.** The domain's `PromotionBlocker` (`promotion/record.rs` `BLOCKERS`, 17 values) maps to the contract's `LifecycleBlocker` through `promotion_blocker` (`lifecycle_verb.rs` about 172); `identity_required` is added by the binding for a merge-commit plan. `primary_presence` (`observe.rs` about 798) gives `primary_missing`, `primary_not_checked_out`, `primary_dirty` and `primary_conflicted`; with a remote, a missing tracking primary is `primary_missing` and a local primary ahead of it is `primary_ahead_of_remote` (about 1023-1043).
4. **The remote history.** With a publication remote and no tracking ref of the branch, `remote_context_history` (`observe.rs` about 887-905) reads the fetch evidence: `Deleted` blocks with `remote_context_deleted`, `Unknown` with `history_unknown`, `Unresolved` with `history_unresolved` (about 1074-1090). A first push makes no tracking ref (decision 29), so after the branch's first publication a prepare reads `Unknown` until a poll fetches the branch.
5. **The record.** `document_promotions` (`promotion/record.rs` `SCHEMA`, about 141-169) has no mode, action or answer column. Its `step` CHECK lists `confirmed, context_synchronized, integrating, integrated, published, publication_pending, remote_deleted, worktree_removing, worktree_removed, branch_deleted, complete, abandoned, superseded`; `INDEX` keeps one unfinished record per item. `migrate` (about 172) creates it under `cycle_09_document_promotions` and rebuilt it once under `promotion_refusal_blockers_v2` by rename and `INSERT … SELECT *`; it is called inside the registry-schema transaction (`discovery.rs` about 2749). `PromotionStep::permits` (about 35) and `validate` (about 242) enforce the steps in Rust; `validate` requires `base_oid`, `integration_kind` and `integration_oid` from `integrating` on, `complete` included. `validate_schema_object` compares the stored DDL exactly.
6. **Cycle 03's branch path in the runner.** `promotion/lifecycle.rs` has `RecordPath { Direct, Branch }` (about 178), `BranchStep { Closing, Finished }` (about 186) and `BranchEnd { ContextGone, ContextChild }` (about 197); `LifecycleRecord::path()` defaults to `Direct` (about 217), `answered()` to false (about 222), `synchronizes_context()` to true (about 228). `promote_inner` sends a branch record to `promote_branch` (`run.rs` about 917): the context child through `context_step` (about 1139), recording `context_synchronized`; then `Lifecycle::branch_step`, whose `Closing` leads to the publication child (`finish_branch`, about 999) and whose `Finished` means the record ended with its answer. A context child's stop that `ends_context_child` names (about 1306: `RemoteContextDeleted`, `HistoryUnresolved`, `TicketClosed`) lets the child go (`let_go`, about 1320) and calls `Lifecycle::end_branch`; any other stop leaves the record `confirmed` for the same request. Before its child, `promote_branch` answers `interrupted` when the record's remote is not the configured one (about 936).
7. **Promotion's branch methods are stubs.** `DocumentPromotionLifecycle` (`promotion/lifecycle.rs` about 866) sets `type BranchFinish = std::convert::Infallible`; `branch_step`, `end_branch`, `begin_publication` and `branch_answer` answer `recovery_required` (about 1102-1140), and `branch_outcome` is `unreachable!` (about 1145). `PromotionRecord` never overrides `path()`. `DocumentPromotionResult` is `{ Complete, IndexPending }` (`promotion/mod.rs` about 188).
8. **The children.** `child_allowed` (`promotion/lifecycle_record.rs` about 474) allows a `Context` child at `confirmed` on either path, a direct record's `Primary` child at `integrated` or `publication_pending`, and a branch record's `Publication` child only at a closure's `closing` or `publication_pending` (it reads `closure_step`, which a promotion leaves `None`).
9. **Cycle 03's closure branch records.** `ticket_closures` was rebuilt under `wave_06_closure_modes_v1` with `integration_mode`, `action` and `answer_code`, keeping its earlier DDL frozen as `SCHEMA_V2` (`closure/record.rs` about 327-427). `ClosureRecord::path()` (about 256) is `Direct` only for `(direct, close)`. `complete_branch_record` (about 1294) and `abandon_branch_record` (about 1325) store `answer_code` and call `give_link` (about 1459), which upserts the link row with the kind `"ticket"` written in. `closure/checkpoint.rs` `branch_link` (about 1036) builds a link from the repository, the primary, the forge setting, the remote and the item. `TicketClosureResult::Branch` carries `BranchClosureOutcome` (`closure/mod.rs` about 455-519); the binding's `Called::Branch` (`bind/lifecycle_verb.rs` about 82) is built only by `Close`, and `branch_code` (about 508) maps a closure's answer to its code.
10. **The link record.** `src/repository/pull_request_links.rs` offers `link_of`, `upsert_link` and `delete_link` (`pub(super)`). Its table (`discovery.rs` about 2944) accepts the kinds `ticket` and `document`, under the marker `wave_06_pull_request_links_v1`, migrated last (about 2760). Both retirement paths delete an item's row whatever its kind: `cleanup.rs` `retire_context` (about 1018) and `context_retirement/record.rs` `finalize_index` (about 2044), each tested with documents (`cleanup_tests.rs` `retire_context_deletes_the_link`, about 1494; `context_retirement/execution_tests.rs` `a_finalized_retirement_deletes_the_link`). Nothing upserts a document's row.
11. **The content proof.** `preservation(repository, local, primary, kind, id)` (`context_retirement/proof.rs` about 27) refuses a shallow repository, then requires the primary to hold exactly one parsing managed item with the ID (`completed_item`). It answers `Ancestor` when the primary is `local` or descends from it; otherwise, with exactly one merge base, it answers `Folded` when every path of the base's and `local`'s flattened trees has in `local` the base's entry or the primary's, and `ContentMismatch` otherwise. Each call flattens three whole trees. The poll uses it for a document whose remote branch was observed deleted, against the local primary (`remote/poll.rs` `completed_deleted_context`, about 279-307), giving `PollDisposition::Promoted`; `context_retirement/tests.rs` `retirement_proof_document_presence_is_not_preservation` (about 268) pins that presence alone is not enough.
12. **Retirement.** `poll once --retire-completed` captures the dispositions `Closed` and `Promoted` (`context_retirement/source.rs` about 176). Retirement requires the remote branch gone (`context_retirement/record.rs` `require_fetch_absence`, about 1680) and the proof against the root's checked-out primary (`context_retirement/run.rs` `compare_local_proof`, about 927). `context release` refuses a context with anything published (`context_release.rs`, its header), so retirement is the only way a published document context ends.
13. **`item sync` of a document.** `synchronize_remote_observed` (`remote/sync.rs` about 6316) runs the stages `[Context, Primary]`, or `[Primary]` without a tracking ref (`remote/merge.rs` about 20-31): the first merges the remote copy, the second merges the fetched primary into the branch ("Merge primary into document <ULID>"), then it publishes. `Published` answers `ok`, `AlreadyCurrent` `already_applied`, `RemoteContextDeleted` `remote_branch_deleted` (`mutation/outcome.rs` about 207 and 358-368); it answers no data except for host stops and conflicts. A context target is never pull-only (`sync.rs` about 6474). It checks the context's worktree, not the root's.
14. **Cycle 03's pieces not yet on `bef571fd`.** Cycle 03's Tasks 10 to 16 are not started. Its design names, which this Cycle builds on: `item sync`'s link data tagged `"type": "pull_request_link"` (`PullRequestLinkDataDto`, `schemas/v1/pull_request_link.schema.json`) when `item sync` of a ticket context in a pull-request repository publishes or finds current a closing ticket's branch, with the test `item_sync_gives_the_link_of_a_closing_ticket`; the close's `PullRequestClosureMutationDto` and `PullRequestClosurePreviewDto`; and its one SSH case `a_pull_request_close_publishes_only_the_ticket_branch` in `tests/mutation_remote/c5_lifecycle.rs`.
15. **The item's link.** `ItemDto.pull_request` (`read/dto.rs` about 571) is documented null "in every list, for a document and for a ticket that has none" (about 565-570), and so is `item.schema.json`. `read/items.rs` `fill_complete_read` (about 1533), called by `show_item` and `show_path`, returns early for anything but a ticket. `tests/read_items.rs` `ticket_show_gives_the_link_and_the_publication` (about 6220) gives a document a link row and asserts its `pull_request` null (about 6248-6257); that document is committed on the primary and has no context (about 6225-6226), and the test's comment says "nor does a document's read, whatever row it has".
16. **Effects, data and codes.** `PreviewEffect` has `context_synchronized`, `context_published` and `pull_request_link` (`mutation/dto.rs` about 226-227); nothing produces the last two yet. Promote's preview effects are built in `bind/lifecycle.rs` (about 1077-1108). `MutationDataDto` (`dto.rs` about 1070) and `PreviewDetailsDto` (about 316) have no promotion branch shapes; `from_stored` maps `DocumentPromote | TicketClose` to the lifecycle DTO (about 2906). `already_applied` is class `None`, outcome `noop`. `tests/mutation_contract.rs` `DATA_ROOTS` has 74 entries (about 1605).
17. **The setup gate.** `Command::setup_gate` puts `DocumentPromote` in `SetupGate::Refused` (`mutation/command.rs` about 321); it is applied before the verb's prepare, before the preview at confirm and inside the acceptance transaction (`mutation/execute.rs` about 276, 359 and 415; `mutation/confirm.rs` about 346). So a pull-request promote answers `setup_pending` with no check of its own.
18. **The mode.** `RepositoryConfig::integration()` (`canonical.rs` about 146) gives `Integration { Direct, PullRequest }` (about 91). A promotion's primary child is exempt from pull-only (`sync.rs` about 6474, `execution.is_none()`), so today a promote in a pull-request repository still pushes the primary (Cycle 03 K-5).
19. **Test seams.** The lifecycle's library tests inject the publisher through `promotion/run.rs` `PromotionIo` with `run_with_io` (about 23 and 327); `promotion/run_tests.rs` `FakeIo` (about 357-459) stands in for the children, and Cycle 03 extends it to push a context branch to a local bare repository with `git2`. `promotion_test_drive_local_bound` (`sync.rs` about 3983) runs a child's real stages against an incoming commit. `poll::synthetic_fetch` (`poll.rs` about 1435) replaces the poll's fetch; `context_retirement/acceptance_tests.rs` (about 313-400) drives a local bare repository with plain `git2` as the forge. Publication through Manyhands is SSH only (`transport/endpoint.rs` `parse_ssh_endpoint` refuses `file`).
20. **Promote's goldens.** `tests/fixtures/cli_v1/document_promote_*` are written without a network (`tests/cli/lifecycle.rs`); `tests/fixtures/mutation_v1/document_promote_*` are written over SSH by `tests/mutation_remote/c5_lifecycle.rs` and registered in `tests/support/mutation_golden.rs` (about 56-65).

## Part A: The Record

**The rebuild (R-1).** `document_promotions` is rebuilt once by a named migration, marker `wave_06_promotion_modes_v1`, run in `promotion/record.rs` `migrate` after `promotion_refusal_blockers_v2`, by rename and copy with an explicit column list. Today's `SCHEMA` stays frozen as `SCHEMA_V2` for the earlier rebuild, as Cycle 03 kept `ticket_closures`'s; the stored DDL holds no SQL comments, because it is compared exactly; the create path writes the new DDL and both markers. The table gains:

| Column | Meaning |
| --- | --- |
| `integration_mode` | `direct` or `pull_request`, `NOT NULL DEFAULT 'direct'`: the mode the record began in, fixed for its life (Cycle 01 decision 11) |
| `answer_code` | a finished pull-request promotion's answer (Part C), written in the transaction that finishes it, abandon included; a same-request re-entry and a replay answer from it |

Existing rows read `direct`. A direct promotion is unchanged in every rule. A pull-request promotion:

- moves `confirmed` → `context_synchronized` → `complete`, or ends `abandoned` from `confirmed` or `context_synchronized`;
- never holds `integration_oid` or `integration_kind`, and never reaches `integrating`, `integrated`, `published`, `publication_pending`, `remote_deleted`, `worktree_removing`, `worktree_removed`, `branch_deleted` or `superseded`;
- has `answer_code` exactly when it is `complete` or `abandoned`;
- keeps its context child's category in `publication_category` (`published` or `already_current`) once its child completed;
- never holds a URL, a title or a body.

CHECKs state these per mode, and `PromotionStep::permits` and `validate` read the mode. The unique unfinished index keeps one unfinished record per document, of either mode. `PromotionRecord::path()` is `Branch` for `pull_request`, and `answered()` is `answer_code.is_some()`.

The answers stored in `answer_code` are a new `PromotionAnswer`: `promoted` (`ok`), `already_promoted` (`already_applied` with the link), `primary_holds` (`already_applied` without a link), `remote_branch_deleted`, `history_unresolved` and `external_change`.

## Part B: Prepare

**Dispatch (R-2).** `document promote --prepare` reads the configuration. In a pull-request repository the observation is the pull-request observation, whose digest's domain is `manyhands pull request promotion v1`; otherwise it is today's, unchanged. Confirm re-observes the same way, so a prepare made in one mode and confirmed in the other is `external_change`, with nothing written. A same-request retry finds its record and continues it in the record's mode.

**The primary holds the document (R-3).** It holds it when `preservation(L, P)` passes for `P` the local primary or the tracking primary (whichever exists; either suffices), and `R` is absent, equal to `L` or an ancestor of `L`. `R` ahead of or beside `L` is another clone's work this clone has not merged, and the document is then not held. The rule is one function, `primary_holds_document`, used by prepare, by confirm's answer table, by `document show` (Part E) and by `item sync` (Part F).

**Prepare reads local state only**, with no network, lease or write. In a pull-request repository it decides, in this order:

1. not a document: `not_a_document`, as today; no context: `context_missing`, as today;
2. the blockers below;
3. the primary holds the document: `already_applied` at prepare, with the data of Part D (`primary_holds` true, `pull_request` null) and no record (R-4). There is nothing to confirm; cleanup is retirement's;
4. the branch's remote history is `Deleted` (a fetch saw it published, then gone): blocked with `remote_context_deleted`, as today, whose way out is `item republish`;
5. a link row exists: the situation `again`, previewing `context_synchronized` and `pull_request_link`;
6. otherwise: the situation `first`, previewing `context_synchronized`, `context_published` and `pull_request_link`.

**The blockers** are the direct promote's where they apply: `context_nonconforming`, `context_dirty`, `context_conflicted`, `conflict_pending`, `history_unresolved`, a merge, rebase or cherry-pick in progress at the root (`primary_conflicted`, as Cycle 03's ruling 6 mapped it), an unfinished promotion record of the document (`recovery_required`, naming its request, the CLI contract's decision 33) and an unfinished synchronization of the branch (`busy`, naming it). These do not apply, because nothing touches the primary (R-6): `primary_missing`, `primary_not_checked_out`, `primary_dirty`, `primary_ahead_of_remote`, `context_behind_remote`, `integration_conflict`, `document_invalid_after_integration` and the binding's `identity_required`. `history_unknown` does not block either (R-5): the context child's fetch reads the history, as `item sync`'s does.

**The digest (R-7)** binds the domain, the item, the document's path, the branch, the primary branch, the remote, `L`, `R` (or none) and the situation (`first` or `again`). It does not bind the tracking or local primary, which move. A save, another clone's publication fetched, or a link given between prepare and confirm is therefore `external_change`.

**The preview's details** are a new `PreviewDetailsDto` variant, `PullRequestPromotionPreviewDto`, tagged `"type": "pull_request_promotion_preview"`, holding the item, its path, the branch, the primary, the remote, `L`, `R`, the situation and the `pull_request` object as it would be given now.

## Part C: Confirm

1. Re-observe and compare the digest (`external_change` on a difference, nothing written). `begin_record` with `integration_mode` `pull_request`.
2. The context child, under `child()`, in `promote_branch`: it synchronizes the branch with its remote copy and the remote's primary and publishes it, as `item sync` does (Baseline Fact 13). Step `context_synchronized`, with the child's category.
   - A stop answers as `item sync` answers that stop (the same code and data, Cycle 03 R-5), and the record stays `confirmed` for the same request. `request cancel` and Ctrl-C answer `cancelled` and leave it unfinished (Cycle 03 decision 8).
   - A child that ends with nothing it can get past (`remote_branch_deleted`, `history_unresolved`; `ends_context_child`) is let go, and `end_branch` answers by the table below, from its first row.
3. `branch_step`, under the lease, answers by the table below and finishes the record in one transaction that stores `answer_code` and, where the table says, upserts the link row (kind `document`, `given_at` from the request's clock). It returns `Finished`: a pull-request promotion has no publication child and never reaches `finish_branch` (R-1).

**The answer**, the first row that matches:

| After the context child | Answer | Record | Link row |
| --- | --- | --- | --- |
| the primary holds the document, judged with the tracking primary the child fetched (whatever the child answered) | `already_applied`, `primary_holds` true, no link | `complete`, `primary_holds` | not written |
| the child ended with `remote_branch_deleted` | `remote_branch_deleted`, way out `item republish` | `abandoned`, `remote_branch_deleted` | not written |
| the child ended with `history_unresolved` | `history_unresolved`, as `item sync` answers it | `abandoned`, `history_unresolved` | not written |
| the situation was `again` | `already_applied`, the link | `complete`, `already_promoted` | upserted |
| otherwise | `ok`, the link | `complete`, `promoted` | upserted |

The link is built when the record finishes, from the publication remote's URL as `git2` reports it, the configuration's forge setting, the branch and the primary, through `forge::recognise` and `forge::pull_request_link` (Cycle 03 R-25). A missing URL never fails the promote.

### Required Outcomes Of Part C

| # | Situation | Required outcome |
| --- | --- | --- |
| C1 | `first`, a new document | `ok`; the branch published; `pull_request` in data; the record `complete` (`promoted`); the link row written; the local primary unchanged; no synchronization targets the primary |
| C2 | `first`, an edit of a merged document | as C1 |
| C3 | `again`, nothing new | the child answers `already_current`; `already_applied` with the same link; `publication` `current`; the row's `given_at` renewed |
| C4 | `again` after a save | the child publishes the save; `already_applied` with the link; `publication` `published` |
| C5 | `again` after the remote's primary moved | the child merges the primary into the branch ("Merge primary into document") and publishes; `already_applied` with the link |
| C6 | prepare when the primary holds the document (the forge merged, and a poll or sync fetched it) | `already_applied` at prepare with `primary_holds` true and no link; no record; nothing written |
| C7 | the forge merged by squash between prepare and confirm, its remote branch kept | the child merges the primary in (clean) and publishes; the primary holds the document; `already_applied`, `primary_holds`; no row written |
| C8 | the forge merged and deleted the branch between prepare and confirm | the child ends with `remote_branch_deleted`; the tracking primary it fetched holds the document; `already_applied`, `primary_holds` (U-1) |
| C9 | the branch deleted without a merge (a declined pull request) | `remote_branch_deleted`, way out `item republish`; the record abandoned; nothing written |
| C10 | the child stops: transport, key, host, `push_rejected`, `busy`, `poll_yielding`, `identity_required` | as `item sync`'s stop; the record `confirmed`; its prepare held for a new request; the same request continues under `child()` |
| C11 | the child meets a conflict | `recovery_required` with the conflict data; the child retained; resolved through the conflict verbs by the record's ID; the same request continues |
| C12 | a crash after the child completed and before the record finished; then the same request | the child replays; no second push; the answer table |
| C13 | a crash after the record finished and before the envelope was stored; then the same request, or a replay | the answer rebuilt from `answer_code`, so `already_applied` and `remote_branch_deleted` come back as themselves |
| C14 | a new request while a record is unfinished | prepare `recovery_required` naming the holder's request; a confirm `busy`; nothing written |
| C15 | `request cancel` or Ctrl-C | `cancelled`, exit 130; the record unfinished; the same request continues with `child()`'s replacement of a cancelled child |
| C16 | prepared in one mode, confirmed after a switch (either way) | `external_change`; nothing written |
| C17 | a save, a fetch of another clone's publication, or a link given between prepare and confirm | `external_change`; nothing written |
| C18 | the publication remote changed after the record began and before its child completed | the record abandoned with `external_change` (U-2); nothing published by it |
| C19 | `operation list`, `operation show`, `operation resume`, `operation abandon` while a record is unfinished | listed as a `promote` with its request; resume answers `original_request_required`; abandon of its child refused |
| C20 | a direct promotion, in a direct repository | unchanged: every existing test and golden keeps its result |

## Part D: The Data, The Preview And The Contract

**The data (R-8).** A new `MutationDataDto` variant, `PullRequestPromotionMutationDto`, tagged `"type": "pull_request_promotion"`, with `publication` (`published`, `current`, or null when no child ran), `primary_holds` (boolean) and `pull_request` (the object, or null). It is the data of a pull-request promotion's `ok` and `already_applied` answers and of a pull-request prepare's `already_applied`. A child's stop carries the synchronization's own data, and `remote_branch_deleted` and `history_unresolved` carry none, as `item sync`'s do. Its schema is `schemas/v1/pull_request_promotion_mutation.schema.json`, a `DATA_ROOTS` entry.

**The preview.** `PullRequestPromotionPreviewDto` (Part B), schema `schemas/v1/pull_request_promotion_preview.schema.json`, a `DATA_ROOTS` entry as `lifecycle_preview.schema.json` is. Its blocked prepares answer with the existing `lifecycle_blocked` shape.

**`from_stored`.** A tag-guarded arm reads the new data for `document promote` before the `DocumentPromote | TicketClose` lifecycle arm, which is guarded on the lifecycle shape (as Cycle 03 guards it for `TicketClose`).

**No new result code, blocker, verb or operation action.** A pull-request promotion is a `document_promotions` row, listed by `operation list` as a `promote` with its step.

**A prepare's `already_applied` carries data.** Today a prepare's `already_applied` carries none (Baseline Fact 16's codes; the CLI contract's promote row). The pull-request prepare's carries the new data, so a caller learns that the primary holds the document without a confirm (D-1).

**Human output.** The preview prints its effects and the link's `human_line()`. The answer prints `publication: <state>` when a child ran, then the link's `human_line()` on its own line, or `<primary> already holds this document` when `primary_holds` is true.

**The desktop.** Compile-level arms for the new preview and data (R-14). "Approve and merge" keeps its words until Cycle 06.

## Part E: Reads: The Document's Link

`fill_complete_read` fills `pull_request` for a document as well as a ticket (decision 3, R-9): for a document with a link row and an active context, the row's link while the primary does not hold the document, and null once it does. It is null for a document with no row or no active context, and in every list, as today. It reads the row in either mode, as the ticket's does. `ItemDto.pull_request`'s documentation and `item.schema.json`'s description change to match (D-1). `ClosureDto.publication` is unchanged.

## Part F: `item sync`'s Link For A Document

Cycle 03's rule for a closing ticket (its Part H, decision 12) gains documents (decision 4, R-10): when `item sync` of a document context in a pull-request repository publishes, or finds current, the branch, and the document has a link row, and the primary does not hold the document after the synchronization, the answer carries Cycle 03's `pull_request_link` data with the row's link. `item sync` never writes a document's row. Every other `item sync` answer is unchanged.

## Part G: The Lifecycle's Adaptations

These are required, and Task 3's design checkpoint writes each one down against the source before code:

1. `PromotionRecord::path()` and `answered()` read `integration_mode` and `answer_code`; `begin_record` and `refuse_record` take the mode.
2. `DocumentPromotionLifecycle`'s `branch_step` (the answer table under the lease, finishing the record, returning `Finished`) and `end_branch` (an ended child's answer, by the table from its first row) are implemented; `begin_publication` and `branch_answer` stay unreachable for a promotion (it has no publication child), and `BranchFinish` stays `Infallible`.
3. `DocumentPromotionResult` gains `Branch(BranchPromotionOutcome)` (and its index-pending twin, as the closure's), holding the answer, the link and the child's publication; `branch_outcome` builds it.
4. `Promote::observe` dispatches on the mode; the binding builds `Called::Branch` for `Promote`; `branch_code` (or a promotion twin) maps `PromotionAnswer` to its code; `reached_*` and the recorded effects read the path; `way_out.rs` gives `remote_branch_deleted` its `item republish` way out; the CLI's retry guard reads the new data.
5. The digest's domain constant sits beside `manyhands promotion v1`.
6. `promote_branch`'s remote check before the child ends a pull-request promotion with `external_change` rather than leaving it `interrupted` (U-2).
7. The places that read a record's second slot (`remote_identity_of`, `children_contexts`, `audit`) answer for a promotion branch record with an empty second slot.
8. A pull-request promotion takes no new hold. While it is unfinished it holds what an unfinished direct promotion holds before integration: its document's prepare for a new request, and its context child's reservation while that child stands.
9. The link helper is shared: Cycle 03's `give_link` and `branch_link` take the kind (or a document twin sits beside them), so the document's row is written with kind `document`.

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | The rebuild once from a v2 table and on a fresh registry; existing rows read `direct`; the columns, steps and CHECKs per mode | `src/repository/promotion/record_tests.rs` |
| P2 | Part B: dispatch per mode, the situations in order, the blockers kept and dropped, `history_unknown` not blocking, each digest field | `src/repository/promotion/observe_tests.rs`, one table |
| P3 | R-3: a new document and an edit of a merged document, each before a merge and after a merge commit, a squash and a fast-forward, against the local and the tracking primary, with `R` absent, equal, behind and ahead | `src/repository/promotion/observe_tests.rs`, one table |
| P4 | C1 to C19 and the answer table | a `pull_request` module of `src/repository/promotion/run_tests.rs`, with `PromotionIo` (a publisher that refuses a primary target) and the clock injected |
| P5 | The owner's lifecycle, end to end in the library: promote, promote again, a save and promote again, then a forge merge by merge commit, squash and fast-forward, with the branch deleted, for a new document and an edit of a merged one; before the merge neither the poll nor promote reports the document promoted, and the link is offered again; after it the poll reports `Promoted`, retirement succeeds and the link row is gone | the same module, with a local bare repository driven by `git2` as the forge, `promotion_test_drive_local_bound` for the children's stages and `synthetic_fetch` for the poll |
| P6 | `document show` and `show --path`: the link while outstanding, null once held, null without a row or a context, null in lists | `tests/read_items.rs`, beside `ticket_show_gives_the_link_and_the_publication` |
| P7 | `item sync`'s link for a document, and none when held or without a row | a binding test beside Cycle 03's `item_sync_gives_the_link_of_a_closing_ticket` |
| P8 | The CLI contract with no network: the pull-request preview, a prepare's `already_applied` when the primary holds the document, a blocked prepare, and a confirm stopped at the context child | `tests/cli/lifecycle.rs`, with goldens in `tests/fixtures/cli_v1/` |
| P9 | End to end over SSH: promote, promote again and `item sync` of a document in a pull-request repository, the server's primary unchanged throughout; the success goldens | Cycle 03's SSH case extended (decision 6) |
| P10 | Every registration: the data and preview schemas, `DATA_ROOTS`, `from_stored`, the enumerations | the existing registration tests |
| P11 | A direct promote is unchanged | the existing promotion tests, and the `document_promotion` target at review-ready |

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). The library tests carry the edge cases: P1 to P7 need no network, and P5 simulates the forge with a local bare repository written by `git2`, as Cycle 03's library lifecycle tests do. The publisher used by P4 and P5 refuses a synchronization of the primary, and every such test asserts that the local primary did not move: that is how "a test fails if one tries" holds in the library. P8's goldens come from the CLI with no network. P9 needs the SSH harness, because no non-SSH publication remote can be selected (Baseline Fact 19) and the success goldens of a publishing answer come only from a real publication; it adds steps to Cycle 03's one case rather than a case of its own (decision 6). No real waits: the runner takes its clock.

At review-ready the SSH tier is the targets AGENTS.md's table selects for the paths this Cycle touches: `document_promotion` (`src/repository/promotion/`), `mutation_remote` (`src/repository/mutation/`), `ticket_closure` (the shared link helper in `src/repository/closure/`, and the runner both lifecycles share) and `cli_remote` (the CLI's human output for promote).

## Named Edits To Pre-Existing Tests

- **E-1** `tests/read_items.rs` `ticket_show_gives_the_link_and_the_publication` (about 6244-6256): its comment "nor does a document's read, whatever row it has" becomes "nor does a read of a document without an active context". Its assertion stands, because its document has no context (Baseline Fact 15); P6's test pins a document with a context. This changes the comment's stated intent, not the assertion, and is named here for the owner's approval.
- **E-2** `tests/mutation_contract.rs` `DATA_ROOTS` gains the two new roots (its length and list).
- **E-3** `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` (about 1075): its marker list (about 1131-1143, with Cycle 03's `wave_06_closure_modes_v1` last) gains `wave_06_promotion_modes_v1` in its migration order.
- **E-4** `src/repository/promotion/record_tests.rs`: `migration_preserves_legacy_rows_is_idempotent_and_audited` (about 366), `half_created_migration_and_missing_marked_table_fail_closed` (about 403), `record_forward_transitions_and_invalid_backwards_recover` (about 417), `schema_rejects_free_text_categories_steps_and_refusal_payloads` (about 560) and `privacy_column_inventory_for_both_new_tables` (about 581) gain the mode's rows and the two columns; their existing rows are unchanged. Task 0 confirms each and adds any other that pins the table.
- **E-5** The exhaustive matches on `DocumentPromotionResult` in tests (Task 0 lists them) gain the `Branch` arms.

Task 0's sweep greps for: `DocumentPromotionResult::` (`promotion/run_tests.rs` matches it at about 1365, 1445, 1586, 2760, 5065, 5285, 5393 and 5708, with `if let`/`let … else` and `matches!`, which a new variant does not break unless a match is exhaustive), `PromotionStep::`, `document_promotions` and `promotion_refusal_blockers_v2` in `tests/` and `src/**/*tests*.rs` (among them the column scans of `tests/ticket_closure.rs` about 449 and `tests/document_promotion.rs` about 886, and the row inserts of `tests/read_status.rs` about 1798), `manyhands promotion v1`, `"pull_request": null` in document goldens, `DATA_ROOTS`, and the skills' sentence that promote still pushes the primary until Cycle 04. Each hit that this design changes is added as E-6 onwards; one that removes, weakens or reverses an assertion's intent stops for the owner.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`: the `document promote, ticket close` row and the promote rows of the answers table (the pull-request promote, its situations, preview, digest, answers, stops, cancellation and replay; a prepare's `already_applied` with data); `document show`'s `pull_request` for a document; `item sync`'s link for a document; the new data and preview shapes.
- **D-2** `docs/RFC/pull-request-mode.md`: dated amendments for decisions 2, 3 and 4, and for the departures the Cycle document lists.
- **D-3** `docs/RFC/repository-index-persistence-and-refresh.md`: the `document_promotions` rebuild and its migration.
- **D-4** The shipped skills, in `skills/` and `.agents/skills/`: `manyhands-documents` (promote in a pull-request repository: the link, promoting again, "the primary already holds this document", a deleted branch, and cleanup after the merge through retirement), `manyhands-finding-work` (`document show`'s `pull_request`), `manyhands-sharing-and-sync` (`item sync`'s link for a document); and Cycle 03's sentences that promote still pushes the primary until Cycle 04 are removed.
- **D-5** `docs/Waves/wave-06-pull-request-workflow.md`, Cycle 04's checklist, through `manyhands-documents`: at closure, with the owner's go-ahead.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** The pull-request promote is a branch record of `document_promotions` with one child, and `branch_step` finishes it — decision 1, and nothing is written on the branch, so there is no hook and no publication child — `finish_branch` and the publication slot are dead code for a promotion.
- **R-2** A record's mode is fixed when it begins; prepare chooses the observation by the configuration, and the pull-request observation's digest has its own domain — Cycle 01 decision 11, Cycle 03 R-2 — a switch between prepare and confirm costs a re-prepare.
- **R-3** "The primary holds the document" is the proof for `L` against the local or the tracking primary, with `R` absent, equal to `L` or behind it — the RFC's content proof; the tracking primary is fresher after a fetch, the local primary is what a poll fast-forwards; `R` ahead is work the clone has not merged — a document whose remote copy has another clone's later saves reads not held until a synchronization merges them.
- **R-4** When the primary holds the document, prepare answers `already_applied` with data and no record — there is nothing to publish, and cleanup belongs to retirement — a prepare's `already_applied` gains data (D-1).
- **R-5** `history_unknown` does not block the pull-request observation — the context child's fetch reads the history, as `item sync`'s does, and a first push leaves no tracking ref, so "promote again" straight after a promote would otherwise need a poll first — a branch deleted since is found at confirm (`remote_branch_deleted`), not at prepare.
- **R-6** The pull-request prepare drops the primary and integration blockers and `identity_required` — nothing touches the primary, and the child's own stops (including identity, for a merge of the primary into the branch) answer as `item sync`'s — a root that is dirty or on another branch does not stop a promote.
- **R-7** The digest binds `L`, `R` and the situation — the consent is to publish what was previewed — a save between prepare and confirm costs a re-prepare.
- **R-8** A new data type and preview type for the pull-request promote — Cycle 03 decision 3 (the close's data type is the close's own) — two schemas more.
- **R-9** `document show` fills `pull_request` for a document while the primary does not hold it, in either mode — decision 3 — up to two proofs per show, each flattening three whole trees, and only for a document with a link row.
- **R-10** `item sync` reads a document's row and never writes it — decision 4 (it never creates a link) — `given_at` is renewed only by promote.
- **R-11** `primary_holds` finishes the record without touching the link row, which retirement deletes — decision 3 (the link ends by the proof) — a stale row outlives the merge until retirement, hidden by reads.
- **R-12** The proof after the child uses the tracking primary that child fetched, before the situation's rows — a forge merge during the promote reports the document held, not a new link — none.
- **R-13** An ended child abandons the record with its code, after the held row — Cycle 03's D6 — none.
- **R-14** The desktop gets compile-level arms only — Cycle 06 owns its design — the desktop shows a generic preview and no link actions until then.
- **R-15** No new SSH case: Cycle 03's case gains the promote steps — AGENTS.md's Testing Strategy, decision 6 — that case runs a few seconds longer, and fails for either Cycle's regression.
- **R-16** The runner takes its clock from the request (Cycle 03's `LifecycleRequest.clock`) for `given_at` — AGENTS.md's no real waits — none.
- **R-17** `again` renews the row's `given_at` and rebuilds the link from the current remote URL and forge setting — the link is what this clone would give now — a changed forge setting changes the link a person already opened.

## Known Limits

The Cycle document's K-1 to K-8.

## Unverified

- **U-1** The context child's fetch moves the tracking primary before the child stops with `remote_branch_deleted` (C8). Task 3's design checkpoint reads `remote/sync.rs`'s fetch and stage order; if it does not, C8 answers `remote_branch_deleted`, and the next prepare, after a poll, answers `already_applied` with `primary_holds`. Either way nothing is written and no link is given.
- **U-2** `promote_branch` answers `interrupted` when the record's remote is no longer configured, without ending the record (Baseline Fact 6). Task 3's checkpoint makes a pull-request promotion end with `external_change` there (Part G item 6), so that no record is held for good; if that needs a change to the shared runner's closure path, it stops for the owner.
- **U-3** Cycle 03's names in Baseline Fact 14 are its design's, not yet code. Task 0 re-checks them after Cycle 03 merges and uses the names as landed; a change of shape stops for the owner.

## Design Checkpoint Obligations

Task 3 writes down, before code, each of Part G's items against the source, and settles U-1 and U-2. Task 2 writes down `primary_holds_document`'s signature (it lives in `promotion/observe.rs`, so `context_retirement/` does not change), and how it finds `R` and both primaries without a fetch.

## Alternatives Rejected

- Handing the promote to `item sync`'s binding with no lifecycle record: decision 1; a stopped promote would be listed, resumed and replayed as an `item sync`, the separate-runner shape Cycle 03 left after three review rounds.
- Answering `already_applied` at prepare with no synchronization while the link is outstanding: decision 2; the pull request would lag the person's saves.
- A link that ends only when its row is deleted: decision 3; `document show` would offer a merged pull request's link.
- A blocker `already_promoted` when the primary holds the document: there is nothing to do, which is `already_applied`.
- Deleting the link row when the primary holds the document: the proof already hides it, and retirement deletes it in the transaction that ends the context.
- A new SSH case: decision 6.

## Risks

| Risk | Mitigation |
| --- | --- |
| The runner's shared code changes break a direct promote or a close | Every direct row of P4 and P11, the `document_promotion` and `ticket_closure` targets at review-ready |
| The rebuild of `document_promotions` damages a real registry, one way | The frozen v2 DDL, an explicit column list, P1 from a v2 table holding rows, the migration in the registry-schema transaction |
| Cycle 03 lands with other names or shapes | Task 0 re-grounds every Baseline Fact after it merges (U-3) |
| The proof's cost in `document show` | Only for a document with a link row; two proofs at most |
| The answer table misreports a forge merge during a promote | C7 and C8 rows, and P5's merges |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| — | — | — | — | — | Not yet reviewed |
