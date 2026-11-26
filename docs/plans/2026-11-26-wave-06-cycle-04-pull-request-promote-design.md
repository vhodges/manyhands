---
id: 01M8BXXGDN9HPD8BM9SJTHHVB2
title: 'Wave 06 Cycle 04 Design: Document Promote In Pull-Request Mode'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 04 Design: Document Promote In Pull-Request Mode

Cycle document: [Wave 06 Cycle 04](../Cycles/wave-06-cycle-04-pull-request-promote.md). Ticket: vh-rphcs (`01M87684C9GM9BJP5QQ429KMAK`). Authority: the [Pull-Request Mode RFC](../RFC/pull-request-mode.md), the Cycle document's Product-Owner Decisions, and [Wave 06 Cycle 03](../Cycles/wave-06-cycle-03-pull-request-close.md)'s design, whose branch records, link record and item field this Cycle builds on.

Status: **draft**, revision 2, 2026-11-26. Revision 1 had two independent reviews (Review Record); revision 2 has not been independently reviewed.

## Intent And Approval Boundary

In a pull-request repository, make `document promote` synchronize and publish the document's branch and answer with the pull request link, never committing to or pushing the primary; the only commits on the branch are the synchronization's own merges. Promoting again while the link is outstanding synchronizes again and answers `already_applied` with the same link. A document counts as promoted only when the content proof shows that a primary holds the branch's content, so a squash merge counts and an edit of a merged document does not count before its own pull request merges. The pull-request promote is a mode of the existing document-promotion lifecycle (decision 1). Approving this design approves the Cycle document's Required Contract and the rulings below. It does not authorize implementation, pushes other than the ticket branch, or merging.

## Conventions

- Names are cited as `file` and `function`. Line numbers are approximate, at `bef571fd` (Cycle 03's branch, which holds main `6c7937c8`); Task 0 re-checks every one after Cycle 03 merges.
- "Pull-request repository" means a valid configuration with `integration = "pull_request"` (Wave 06 Cycle 01), which is `format_version = 2` and always names a publication remote.
- "The local primary" is `refs/heads/<primary>`. "The tracking primary" is `refs/remotes/<publication remote>/<primary>`, as last fetched.
- "The branch" is `manyhands/document/<ULID>`. "`L`" is its local tip. "`R`" is its remote tracking tip, `refs/remotes/<publication remote>/manyhands/document/<ULID>`, when that ref exists.
- "The proof" is `context_retirement/proof.rs` `preservation` (Baseline Fact 11). "The primary holds the document" is the rule R-3 of Part B.
- "A link row" is the document's row in `pull_request_links` (kind `document`). "An outstanding link" is a link row while the primary does not hold the document (decision 3).
- "An active context" of a document is the one its read comes from when the index's row for the document has `context.kind` `active`: its own worktree on the branch.
- "A branch record" is a lifecycle record whose path is `RecordPath::Branch` (Cycle 03). "A pull-request promotion" is a `document_promotions` row whose `integration_mode` is `pull_request`. "A direct promotion" is today's.
- "A child that completed" is a context child whose synchronization finished (`published` or `already_current`). "A child that ended" is one that `ends_context_child` names, or a branch gone after its child (`BranchEnd`). "A child that stopped" is any other stop.

## Baseline Facts

Read in the source at `bef571fd` by the planning research, re-checked by the author, and checked by review round 1.

1. **Promote's prepare.** `LifecycleBinding::prepare` (`mutation/bind/lifecycle.rs`) calls `Promote::observe` (`bind/lifecycle_verb.rs` about 291), then `RepositoryService::preview_document_promotion` (`promotion/mod.rs` about 501), `observe::preview` and `prepare_for` (`promotion/observe.rs` about 42 and 346), `observe_git` (about 948) and `expectation` (about 1367). The digest's domain is `manyhands promotion v1` and binds the item, the document's path, the context and primary branches, the remote, the planned integration, the change set and the cleanup; it binds the OIDs only when there is no publication remote. The binding's digest wraps it (`bind/lifecycle.rs` `digest_of`, about 197), matching the two `Observed` shapes, `Ready` and `Blocked` (`lifecycle_verb.rs` about 66-69). Nothing in `promotion/` reads the repository's mode.
2. **"Already promoted" today is ancestry.** `planned_result` (`observe.rs` about 918) gives `Contained` when the context is an ancestor of the base, the tracking primary when there is a remote (about 1044); confirm then answers `ok` with `integration: already_contained` and cleans up (`tests/cli/lifecycle.rs` `a_context_primary_already_holds_is_only_cleaned_up`, about 626). No promote path answers `already_applied`, except `operation resume` of a complete record (`bind/resume.rs` about 256 and 976). After a squash merge the context is not an ancestor, so a direct promote plans a merge again.
3. **Promote's prepare and the runner's entry.** The domain's `PromotionBlocker` (`promotion/record.rs` `BLOCKERS`, 17 values) maps to the contract's `LifecycleBlocker` through `promotion_blocker` (`lifecycle_verb.rs` about 172). `primary_presence` (`observe.rs` about 798) gives `primary_missing`, `primary_not_checked_out`, `primary_dirty` and `primary_conflicted`; with a remote, a missing tracking primary is `primary_missing` and a local primary ahead of it is `primary_ahead_of_remote` (about 1023-1043). The binding adds `identity_required` for a merge-commit plan, and the runner's `require_identity` refuses such a preview without an identity before the record exists (`promotion/lifecycle.rs` about 1004-1027; `run.rs` about 579-581). Promotion's `type Preview` is `DocumentPromotionPreview`, whose `primary_oid` is not optional (`promotion/mod.rs` about 288), and `Ready` holds it too (`lifecycle_verb.rs` about 55); `DocumentPromotionPreflight` is matched exhaustively in about twelve files, among them the `cli_remote` and `document_promotion` tests. The mode reaches `begin_record` only through the observation (`lifecycle.rs` about 895-903, `run.rs` about 597-605), as the closure's does through `ClosureObservation.path` (`closure/lifecycle.rs` about 291-330). `runs_context_child()` is false for an `AlreadyContained` plan with no tracking context (`lifecycle.rs` about 344-348), and `context_step` then runs no child (`run.rs` about 1153-1167). The direct promote has no prepare blocker for another synchronization of the context; the closure's (`foreign_synchronization`, `closure/observe.rs` about 744-779) queries tickets only.
4. **The remote history.** With a publication remote and no tracking ref of the branch, `remote_context_history` (`observe.rs` about 891) reads the fetch evidence: `Deleted` blocks with `remote_context_deleted`, `Unknown` with `history_unknown`, `Unresolved` with `history_unresolved` (about 1074-1090). A first push makes no tracking ref (decision 29: `advance_tracking_to_authority`, `remote/sync.rs` about 9289, moves only an existing one), so after a new branch's first publication a prepare reads `Unknown` until a poll fetches the branch; three documents published by `item sync` on 2026-11-26 had no tracking ref afterwards. A context child with that evidence fetches the advertised branch and plans from it: an advertised branch is fetched, an absent one answers `RemoteContextDeleted`, and `HistoryUnknown` evidence answers `HistoryUnresolved` (`sync.rs` about 6719-6748 and 8921-8981; `refs.rs` about 264-268). The child's fetch moves the tracking primary before it stops with `RemoteContextDeleted` (`FetchTracking`).
5. **The record.** `document_promotions` (`promotion/record.rs` `SCHEMA`, about 141-169) has no mode, action or answer column. Its `step` CHECK lists `confirmed, context_synchronized, integrating, integrated, published, publication_pending, remote_deleted, worktree_removing, worktree_removed, branch_deleted, complete, abandoned, superseded`; `INDEX` keeps one unfinished record per item; `(step='abandoned')=(refusal_category IS NOT NULL)` (about 165). `migrate` (about 172) creates it under `cycle_09_document_promotions` and rebuilt it once under `promotion_refusal_blockers_v2` by rename and `INSERT … SELECT *` from the current `SCHEMA` (about 197-208); it is called inside the registry-schema transaction, before the closure's (`discovery.rs` about 2749-2750). `PromotionStep::permits` (about 35) and `validate` (about 242) enforce the steps in Rust: `validate` requires `context_oid` from `context_synchronized` on, `base_oid`, `integration_kind` and `integration_oid` from `integrating` on, `complete` included, a `published` or `already_current` category at `complete` with a remote, and a refusal exactly at `abandoned` (about 265). `validate_schema_object` compares the stored DDL exactly. `record_tests.rs` `promotion_refusal_blockers_v2_rebuilds_an_old_bound_table_once` (about 1417-1458) rebuilds from the current DDL with `INSERT … SELECT *`.
6. **Cycle 03's branch path in the runner.** `promotion/lifecycle.rs` has `RecordPath { Direct, Branch }` (about 178), `BranchStep { Closing, Finished }` (about 186) and `BranchEnd { ContextGone, ContextChild }` (about 197); `LifecycleRecord::path()` defaults to `Direct` (about 217), `answered()` to false (about 222), `synchronizes_context()` to true (about 228). `promote_inner` sends a branch record to `promote_branch` (`run.rs` about 917): before its child, the record's remote must be the configured one while `base_oid` is unset (`interrupted` otherwise, about 934-938), then the `AfterConfirmed` boundary and the cancellation check; the context child through `context_step` (about 1139), which records `context_synchronized` and `context_oid`; then `RunEvent::Recording` when `effect_written()` is false (about 980-982; the desktop words it "Marking it closed…", `progress.rs` about 122) and `Lifecycle::branch_step`, whose `Finished` returns `refresh` and never reaches `finish_branch` (about 984-990). A child that `ends_context_child` names (about 1306: `RemoteContextDeleted`, `HistoryUnresolved`, `TicketClosed`) is let go (`let_go`, about 1320) and `Lifecycle::end_branch` ends the record at `confirmed`; so does a branch gone after its child (`ContextGone`, about 950-956), which the closure answers `ReconfirmationRequired` (`closure/lifecycle.rs` about 567). Any other stop leaves the record `confirmed` for the same request.
7. **Promotion's branch methods are stubs.** `DocumentPromotionLifecycle` (`promotion/lifecycle.rs` about 864) sets `type BranchFinish = std::convert::Infallible`; `branch_step`, `end_branch`, `begin_publication` and `branch_answer` answer `recovery_required` (about 1102-1140), and `branch_outcome(r, publication)` (no database access) is `unreachable!` (about 1143-1149). `PromotionRecord` never overrides `path()`. `DocumentPromotionResult` is `{ Complete, IndexPending }` (`promotion/mod.rs` about 188), matched exhaustively at `bind/lifecycle_verb.rs` about 340-346, `promotion/run_tests.rs` about 5069-5074, `tests/document_promotion.rs` about 314-320, `tests/document_promotion/journeys.rs` about 257-261 and `tests/document_promotion/quality.rs` about 130-134. `make_outcome` passes the second slot's publication authority (`run.rs` about 2326-2337), which a promotion branch record never has.
8. **The children.** `child_allowed` (`promotion/lifecycle_record.rs` about 474) allows a `Context` child at `confirmed` on either path, a direct record's `Primary` child at `integrated` or `publication_pending`, and a branch record's `Publication` child only at a closure's `closing` or `publication_pending`.
9. **Cycle 03's closure branch records.** `ticket_closures` was rebuilt under `wave_06_closure_modes_v1` with `integration_mode` (Cycle 01's `Integration`), `action` and `answer_code`, keeping its earlier DDL frozen as `SCHEMA_V2` with an explicit column list (`closure/record.rs` about 327-433; its test's `V2_COLUMNS`, `closure/record_tests.rs` about 1782). `ClosureRecord::path()` (about 256) is `Direct` only for `(direct, close)`. `complete_branch_record` (about 1294) stores `answer_code`; `abandon_branch_record` (about 1325) stores the refusal `ReconfirmationRequired` with the answer (`closure/lifecycle.rs` about 580-588); both call `give_link` (about 1459), which upserts the link row with the kind `"ticket"` written in. `closure/checkpoint.rs` `branch_link` (about 1036) builds a link from the repository, the primary, the forge setting, the remote and the item. `type Outcome` for the closure is the enum `ClosureOutcome` (`closure/lifecycle.rs` about 683-698), and `TicketClosureResult::Branch` carries `BranchClosureOutcome` (`closure/mod.rs` about 455-519). The binding's `Called::Branch` (`bind/lifecycle_verb.rs` about 82-89) holds a `BranchClosureOutcome` and is built only by `Close`; `branch_code` (about 508) takes a closure's `BranchAnswer`.
10. **The link record.** `src/repository/pull_request_links.rs` offers `link_of`, `upsert_link` and `delete_link` (`pub(super)`). Its table (`discovery.rs` about 2944) accepts the kinds `ticket` and `document`, under the marker `wave_06_pull_request_links_v1`, migrated last (about 2760). Both retirement paths delete an item's row whatever its kind: `cleanup.rs` `retire_context` (about 1018) and `context_retirement/record.rs` `finalize_index` (about 2044), each tested with documents (`cleanup_tests.rs` `retire_context_deletes_the_link`, about 1495; `context_retirement/execution_tests.rs` `a_finalized_retirement_deletes_the_link`, about 228). Nothing upserts a document's row.
11. **The content proof.** `preservation(repository, local, primary, kind, id)` (`context_retirement/proof.rs` about 27) refuses a shallow repository, flattens `local`'s and the primary's trees loading every blob to verify it exists (`entries(…, true)`, about 38-39 and 192-197), then requires the primary to hold exactly one parsing managed item with the ID (`completed_item`, about 223, which reads and parses every Markdown file under `docs/`, `.manyhands/tickets/` and `.manyhands/comments/`). It answers `Ancestor` when the primary is `local` or descends from it; otherwise, with exactly one merge base, `Folded` when every path of the base's and `local`'s flattened trees has in `local` the base's entry or the primary's, and `ContentMismatch` otherwise. Checked against the cases: a new document never on the primary is `InvalidPrimaryItem`; an edit before its merge is `ContentMismatch`; after a squash it is `Folded`, after a fast-forward `Ancestor`, and after a squash followed by "Merge primary into document" `Folded`. The module is visible across `crate::repository`. The poll uses it for a document whose remote branch was observed deleted, against the local primary (`remote/poll.rs` `completed_deleted_context`, about 279-307), giving `PollDisposition::Promoted`; `context_retirement/tests.rs` `retirement_proof_document_presence_is_not_preservation` (about 268) pins that presence alone is not enough.
12. **Retirement.** `poll once --retire-completed` captures the dispositions `Closed` and `Promoted` (`context_retirement/source.rs` about 176). Retirement requires the remote branch gone (`context_retirement/record.rs` `require_fetch_absence`, about 1680) and the proof against the root's checked-out primary (`context_retirement/run.rs` `compare_local_proof`, about 927). `context release` refuses a context with anything published (`context_release.rs`, its header), so retirement is the only way a published document context ends.
13. **`item sync` of a document.** `synchronize_remote_observed` (`remote/sync.rs` about 6316) runs the stages `[Context, Primary]`, or `[Primary]` without a tracking ref (`remote/merge.rs` about 20-31): the first merges the remote copy, the second merges the fetched primary into the branch ("Merge primary into document <ULID>", `sync.rs` about 3872), then it publishes. `Published` answers `ok`, `AlreadyCurrent` `already_applied`, `RemoteContextDeleted` `remote_branch_deleted` (`mutation/outcome.rs` about 207 and 358-368); it answers no data except for host stops and conflicts. A context target is never pull-only (`sync.rs` about 6468-6474). It checks the context's worktree, not the root's. `SyncBinding::finished` (`bind/sync.rs`) has the service, the target's kind and item ID, and the root.
14. **Cycle 03's pieces not yet on `bef571fd`.** Cycle 03's Tasks 10 to 16 are not done. Its design names, which this Cycle builds on: `item sync`'s link data tagged `"type": "pull_request_link"` (`PullRequestLinkDataDto`, `schemas/v1/pull_request_link.schema.json`, golden `item_sync_pull_request_link`) and its test `item_sync_gives_the_link_of_a_closing_ticket`; the close's `PullRequestClosureMutationDto` and `PullRequestClosurePreviewDto`; withdraw's prepare that answers `already_applied` with no data (its Part E, Task 11), which adds that shape to the binding; the extension of `FakeIo` that pushes a context branch to a local bare repository (its Task 13); and its one SSH case `a_pull_request_close_publishes_only_the_ticket_branch` in `tests/mutation_remote/c5_lifecycle.rs` (its Task 14). Its library fake that fetches, merges and pushes, `PullRequestIo`, is private to `closure/run_pull_request_tests.rs` (about 618-760).
15. **The item's link.** `ItemDto.pull_request` (`read/dto.rs` about 571) is documented null "in every list, for a document and for a ticket that has none" (about 565-570), and so is `item.schema.json`. `read/items.rs` `fill_complete_read` (about 1533), called by `show_item` and `show_path`, returns early for anything but a ticket; it has the registry connection, the root and the configuration. `tests/read_items.rs` `ticket_show_gives_the_link_and_the_publication` (about 6220) gives a document a link row and asserts its `pull_request` null (about 6248-6257); that document is committed on the primary and has no context (about 6225-6226), and the test's comment says "nor does a document's read, whatever row it has".
16. **Effects, data, way outs and registrations.** `PreviewEffect` has `context_synchronized`, `context_published` and `pull_request_link` (`mutation/dto.rs` about 212 and 226-227); nothing produces the last two yet. Promote's preview effects are built in `bind/lifecycle.rs` (about 1077-1108). `MutationDataDto` (`dto.rs` about 1070) and `PreviewDetailsDto` (about 316) have no promotion branch shapes; `PrepareDataDto` is `Preview`, `HostReplacement`, `LifecycleBlocked` and `Blocked` (about 961-972); `from_stored` maps `DocumentPromote | TicketClose` to the lifecycle DTO (about 2906). `already_applied` is class `None`, outcome `noop`. `way_out.rs` gives a lifecycle answer's `remote_branch_deleted` the way out `item republish` from `MutationDataDto::Lifecycle`'s `item_id` (about 114-121, 194 and 246-251), and gives nothing without that data; the CLI contract's answers table has `document promote, ticket close | remote_branch_deleted | request.cancel` (`cli-contract.md` about 1568). Registrations a new data shape meets: `tests/mutation_contract.rs` `DATA_ROOTS` (74, about 1605) and `the_fixture_directories_are_exactly_the_registered_cases` (about 1511); `tests/support/cli.rs` `CLI_CASES` and `CLI_TEXT_CASES` (its `golden()` asserts the name, about 587); `tests/cli/inventory.rs`'s `DATA` row for `document promote` (about 265-270), checked by `every_verb_s_data_has_an_output_schema_and_its_goldens_match_it` (about 552-591); exhaustive `MutationDataDto` matches in `tests/support/mutation.rs` (six), `tests/support/repair.rs` (about 337-343), `tests/mutation_remote/c5_lifecycle.rs` (five), `human_admin.rs` and the desktop; `tests/cli/recovery_tables.rs` and `tests/cli/collaboration_exits.rs`, which parse the CLI contract's tables; `tests/read_contract.rs`'s enumerations.
17. **The setup gate.** `Command::setup_gate` puts `DocumentPromote` in `SetupGate::Refused` (`mutation/command.rs` about 321); it is applied before the verb's prepare, before the preview at confirm and inside the acceptance transaction (`mutation/execute.rs` about 276, 359 and 415; `mutation/confirm.rs` about 346). So a pull-request promote answers `setup_pending` with no check of its own.
18. **The mode.** `RepositoryConfig::integration()` (`canonical.rs` about 146) gives `Integration { Direct, PullRequest }` (about 91); a pull-request configuration needs `format_version = 2` (about 1457-1470). A promotion's primary child is exempt from pull-only (`sync.rs` about 6474, `execution.is_none()`), so today a promote in a pull-request repository still pushes the primary (Cycle 03 K-5).
19. **Test seams.** The lifecycle's library tests inject the publisher through `promotion/run.rs` `PromotionIo` with `run_with_io` (about 23 and 327). `promotion/run_tests.rs` `FakeIo` (about 357-459) records children, moves the tracking primary (about 410-414) and never fetches, merges or pushes; it does not refuse a primary target. `promotion_test_drive_local_bound` (`sync.rs` about 3983) runs a child's real stages against an incoming commit. `poll::synthetic_fetch` (`poll.rs` about 1435) replaces the poll's fetch; a retirement driven from outside `context_retirement` goes through `execute(Mutation::PollOnceRetiring)` with `synthetic_fetch` alone (`context_retirement/mutation_tests.rs` about 207-214), after the objects and tracking refs are fetched with plain `git2` (`context_retirement/acceptance_tests.rs` about 357-363); `context_retirement::run::synthetic_advertisement` is `pub(super)`. Publication through Manyhands is SSH only (`transport/endpoint.rs` `parse_ssh_endpoint` refuses `file`). A confirm that reaches a transport stop without a network needs a selected key and a closed port.
20. **Promote's goldens.** `tests/fixtures/cli_v1/document_promote_*` are written without a network (`tests/cli/lifecycle.rs`); `tests/fixtures/mutation_v1/document_promote_*` are written over SSH by `tests/mutation_remote/c5_lifecycle.rs` and registered in `tests/support/mutation_golden.rs` (about 56-65).

## Part A: The Record

**The rebuild (R-1).** `document_promotions` is rebuilt once by a named migration, marker `wave_06_promotion_modes_v1`, run in `promotion/record.rs` `migrate` after `promotion_refusal_blockers_v2`, by rename and copy with an explicit column list. Today's `SCHEMA` is frozen as `SCHEMA_V2`, and the `promotion_refusal_blockers_v2` step rebuilds from `SCHEMA_V2`, as Cycle 03 did for `ticket_closures`; the stored DDL holds no SQL comments, because it is compared exactly; the create path writes the new DDL and both markers. The table gains:

| Column | Meaning |
| --- | --- |
| `integration_mode` | `direct` or `pull_request`, `NOT NULL DEFAULT 'direct'`: the mode the record began in, fixed for its life (Cycle 01 decision 11), Cycle 01's `Integration` |
| `answer_code` | a finished pull-request promotion's answer (Part C), written in the transaction that finishes it, abandon included; a same-request re-entry and a replay answer from it |

Existing rows read `direct`. A direct promotion is unchanged in every rule. A pull-request promotion:

- moves `confirmed` → `context_synchronized` → `complete` after a child that completed, and `confirmed` → `abandoned` after a child that ended;
- never holds `integration_oid`, `integration_kind` or `base_oid`, and never reaches `integrating`, `integrated`, `published`, `publication_pending`, `remote_deleted`, `worktree_removing`, `worktree_removed`, `branch_deleted` or `superseded`;
- at `complete` holds `context_oid` and its context child's category in `publication_category` (`published` or `already_current`), written by `branch_step`;
- at `abandoned` holds the refusal `reconfirmation_required`, as Cycle 03's `abandon_branch_record` stores, and no publication category;
- has `answer_code` exactly when it is `complete` or `abandoned`;
- never holds a URL, a title or a body.

CHECKs state these per mode, and `PromotionStep::permits` and `validate` read the mode. The unique unfinished index keeps one unfinished record per document, of either mode. `PromotionRecord::path()` is `Branch` for `pull_request`, and `answered()` is `answer_code.is_some()`.

`answer_code` holds a new `PromotionAnswer`: `promoted` (`ok`), `already_promoted` (`already_applied` with the link), `primary_holds` (`already_applied` without a link), `remote_branch_deleted`, `history_unresolved` and `context_gone` (the branch gone after the child, answered as Cycle 03 answers `ContextGone`).

## Part B: Prepare

**Dispatch (R-2).** `document promote --prepare` reads the configuration. In a pull-request repository the observation is the pull-request observation, whose digest's domain is `manyhands pull request promotion v1`; otherwise it is today's, unchanged. Promotion's observation type carries the mode and the pull-request facts to the runner and to `begin_record`, as the closure's `ClosureObservation` carries its path (Part G item 12). Confirm re-observes the same way, so a prepare made in one mode and confirmed in the other is `external_change`, with nothing written. A same-request retry finds its record and continues it in the record's mode.

**The primary holds the document (R-3).** It holds it when the proof for `L` passes against the local primary or the tracking primary (whichever exists; either suffices), and `R` is absent, equal to `L` or an ancestor of `L`. `R` ahead of or beside `L` is another clone's work this clone has not merged, and the document is then not held. The rule is one function, `primary_holds_document`, in `promotion/observe.rs` (so `context_retirement/` does not change), taking the repository, the primary branch's name, the publication remote's name and the item, and reading the refs itself. Every `ProofRefusal` reads as not held. Prepare and confirm use it, and so does `item sync` (Part F); `document show` uses the cheaper twin of Part E (decision 9).

**Prepare reads local state only**, with no network, lease or write. In a pull-request repository it decides, in this order:

1. not a document: `not_a_document`, as today; no context: `context_missing`, as today;
2. the blockers below;
3. the primary holds the document: `already_applied` at prepare, with no data and no record (R-4), as Cycle 03's withdraw prepare answers it. There is nothing to confirm; cleanup is retirement's;
4. the branch's remote history is `Deleted` (a fetch saw it published, then gone): blocked with `remote_context_deleted`, as today, whose way out is `item republish`;
5. a link row exists: the situation `again`;
6. otherwise: the situation `first`.

**The effects.** `first` and `again` preview `context_synchronized` and `pull_request_link`, and also `context_published` when something may be published: `L` differs from `R`, or `R` is absent.

**The blockers** kept from the direct promote: `context_nonconforming`, `context_dirty`, `context_conflicted`, `conflict_pending`, `history_unresolved`, `primary_missing` (no tracking primary, as Cycle 03's branch close keeps it), a merge, rebase or cherry-pick in progress at the root (`primary_conflicted`, as Cycle 03's branch close maps it, `closure/observe.rs` about 1037-1041), and an unfinished promotion record of the document (`recovery_required`, naming its request: the binding's hold, `bind/lifecycle.rs` about 396-435, the CLI contract's decision 33). Dropped, because nothing touches the primary (R-6): `primary_not_checked_out`, `primary_dirty`, `primary_ahead_of_remote`, `context_behind_remote`, `integration_conflict`, `document_invalid_after_integration`, and the binding's `identity_required` with the runner's `require_identity`. `history_unknown` does not block (decision 10, R-5): the context child's fetch settles the history. Another unfinished synchronization of the branch is not a prepare blocker: the child answers `busy` at confirm (R-18).

**The digest (R-7)** binds the domain, the item, the document's path, the branch, the primary branch, the remote, `L`, `R` (or none) and the situation (`first` or `again`). It does not bind the tracking or local primary, which move. A save, another clone's publication fetched, a link given, or a merge fetched between prepare and confirm therefore changes the observation, and confirm answers `external_change`.

**The preview's details** are a new `PreviewDetailsDto` variant, `PullRequestPromotionPreviewDto`, tagged `"type": "pull_request_promotion_preview"`, holding the item, its path, the branch, the primary, the remote, `L`, `R`, the situation and the `pull_request` object: the link row's for `again`, the one that would be built now for `first`.

## Part C: Confirm

1. Re-observe and compare the digest (`external_change` on a difference, nothing written). `begin_record` with `integration_mode` `pull_request`.
2. The context child, under `child()`, in `promote_branch`; it always runs in pull-request mode (`runs_context_child` true, R-19). It synchronizes the branch with its remote copy and the remote's primary and publishes it, as `item sync` does (Baseline Fact 13). On completion, step `context_synchronized`.
   - A child that stopped answers as `item sync` answers that stop (the same code and data, Cycle 03 R-5), and the record stays `confirmed` for the same request. `request cancel` and Ctrl-C answer `cancelled` and leave it unfinished (Cycle 03 decision 8).
   - A child that ended is let go, and `end_branch` answers by the second table below.
3. After a child that completed, `branch_step`, under the lease, answers by the first table below and finishes the record in one transaction that stores `answer_code` and the child's category and, where the table says, writes the link row (kind `document`, `given_at` from the request's clock). It returns `Finished`: a pull-request promotion has no publication child and never reaches `finish_branch` (R-1). `RunEvent::Recording` is not emitted for a promotion (Part G item 11).

The remote check before the child (Baseline Fact 6) applies to a pull-request promotion as it applies to a closure's branch record before its commit, unchanged (R-20).

**After a child that completed**, the first row that matches:

| Situation | Answer | Record | Link row |
| --- | --- | --- | --- |
| R-3 holds, with the tracking primary as the child fetched it | `already_applied`, `primary_holds` true, no link | `complete`, `primary_holds` | untouched |
| the situation was `again` | `already_applied`, the row's link | `complete`, `already_promoted` | `given_at` renewed |
| otherwise | `ok`, the link built now | `complete`, `promoted` | written |

**After a child that ended** (the record at `confirmed`), the first row that matches:

| Situation | Answer | Record | Link row |
| --- | --- | --- | --- |
| R-3 holds, with the tracking primary as the child fetched it (whatever ended it) | `already_applied`, `primary_holds` true, no link | `abandoned`, `primary_holds` | untouched |
| `remote_branch_deleted` | `remote_branch_deleted`, way out `item republish`, then a new promote (decision 11) | `abandoned`, `remote_branch_deleted` | untouched |
| `history_unresolved` | `history_unresolved`, as `item sync` answers it | `abandoned`, `history_unresolved` | untouched |
| the branch gone after the child | as Cycle 03 answers `ContextGone` | `abandoned`, `context_gone` | untouched |

The link of `first` is built once, from the publication remote's URL as `git2` reports it, the configuration's forge setting, the branch and the primary, through `forge::recognise` and `forge::pull_request_link` (Cycle 03 R-25), and stored; `again` returns the stored link (R-17). A missing URL never fails the promote. A replay, and a same-request re-entry of a finished record, answer from `answer_code`, with the link read from the row, or null when the row is gone.

### Required Outcomes Of Part C

| # | Situation | Required outcome |
| --- | --- | --- |
| C1 | `first`, a new document | `ok`; the branch published; `pull_request` in data; the record `complete` (`promoted`); the link row written; the local primary unchanged; no synchronization targets the primary |
| C2 | `first`, an edit of a merged document | as C1 |
| C3 | `first`, the branch already published by `item sync` | `ok`; `publication` `current`; the link row written |
| C4 | `again`, nothing new | the child answers `already_current`; `already_applied` with the row's link; `publication` `current`; `given_at` renewed |
| C5 | `again` after a save | the child publishes the save; `already_applied` with the same link; `publication` `published` |
| C6 | `again` after the remote's primary moved | the child merges the primary into the branch ("Merge primary into document") and publishes; `already_applied` with the same link |
| C7 | `again` after the forge setting changed | `already_applied` with the row's link, not a rebuilt one |
| C8 | prepare when the primary holds the document (the forge merged, and a poll or a synchronization fetched it) | `already_applied` at prepare, no data; no record; nothing written |
| C9 | the forge merged by squash between prepare and confirm, its remote branch kept | the child merges the primary in (clean) and publishes; R-3 holds; `already_applied`, `primary_holds`; the record `complete`; the link row untouched |
| C10 | the forge merged and deleted the branch between prepare and confirm | the child ends with `remote_branch_deleted` after moving the tracking primary; R-3 holds; `already_applied`, `primary_holds`; the record `abandoned` |
| C11 | the branch deleted without a merge (a declined pull request) | `remote_branch_deleted` with the way out `item republish`, then a new promote; the record `abandoned`; nothing written |
| C12 | the child stops: transport, key, host, `push_rejected`, `busy` (another synchronization of the branch), `poll_yielding`, `identity_required` (the merge of the primary into the branch) | as `item sync`'s stop; the record `confirmed`; its prepare held for a new request; the same request continues under `child()` |
| C13 | the child meets a conflict | `recovery_required` with the conflict data; the child retained; resolved through the conflict verbs by the record's ID; the same request continues |
| C14 | a crash after the child completed and before the record finished; then the same request | the child replays; no second push; the first table |
| C15 | a crash after the record finished and before the envelope was stored; then the same request, or a replay | the answer rebuilt from `answer_code`, with the row's link (null if the row is gone) |
| C16 | a new request while a record is unfinished | prepare `recovery_required` naming the holder's request; a confirm `busy`; nothing written |
| C17 | `request cancel` or Ctrl-C | `cancelled`, exit 130; the record unfinished; the same request continues with `child()`'s replacement of a cancelled child |
| C18 | prepared in one mode, confirmed after a switch (either way) | `external_change`; nothing written |
| C19 | a save, a fetch of another clone's publication, a link given, or a poll that fetched the merge, between prepare and confirm | `external_change`; nothing written; a new prepare answers by Part B (`already_applied` when the merge was fetched) |
| C20 | the publication remote changed after the record began and before its child completed | as a closure's branch record before its commit (R-20) |
| C21 | `operation list`, `operation show`, `operation resume`, `operation abandon` while a record is unfinished | listed as a `promote` with its request; resume answers `original_request_required`; abandon of its child refused |
| C22 | the poll's retirement removes the context while a stopped record waits (the forge merged and deleted the branch); then the same request | the ended-child table's first row: `already_applied`, `primary_holds` (R-21) |
| C23 | `--confirm` with a confirmation ID after a prepare that answered `already_applied` | `confirmation_not_found`, as for any confirmation no prepare issued |
| C24 | a direct promotion, in a direct repository | unchanged: every existing test and golden keeps its result |

## Part D: The Data, The Preview And The Contract

**The data (R-8).** A new `MutationDataDto` variant, `PullRequestPromotionMutationDto`, tagged `"type": "pull_request_promotion"`, with `item_id`, `publication` (`published` or `current`, from the child's category; null when no child completed), `primary_holds` (boolean) and `pull_request` (the object, or null). It is the data of every finished answer of a pull-request promotion: `ok`, `already_applied`, and the ended answers `remote_branch_deleted`, `history_unresolved` and Cycle 03's `ContextGone` answer. A child that stopped carries the synchronization's own data. Its schema is `schemas/v1/pull_request_promotion_mutation.schema.json`, a `DATA_ROOTS` entry.

**The preview.** `PullRequestPromotionPreviewDto` (Part B), schema `schemas/v1/pull_request_promotion_preview.schema.json`, a `DATA_ROOTS` entry as `lifecycle_preview.schema.json` is. Its blocked prepares answer with the existing `lifecycle_blocked` shape.

**A prepare's `already_applied`** carries no data (R-4): in a pull-request repository it means the primary holds the document. It uses the prepare shape Cycle 03's withdraw adds (Baseline Fact 14).

**`from_stored`.** A tag-guarded arm reads the new data for `document promote` before the `DocumentPromote | TicketClose` lifecycle arm, which is guarded on the lifecycle shape (as Cycle 03 guards it for `TicketClose`).

**The way out (decision 11, R-22).** `way_out.rs` gives a `remote_branch_deleted` that carries the new data the way out `item republish` of its `item_id`, then a new `document promote --prepare` (not the same request: the record is finished). It is keyed on the data, so the direct promote's, the close's and `item sync`'s way outs do not change. The CLI contract's answers table gains a `document promote` (pull-request) row for each finished answer with its recovery, its `next:` lines and its golden or the reason it has none (D-1).

**No new result code, blocker, verb or operation action.** A pull-request promotion is a `document_promotions` row, listed by `operation list` as a `promote` with its step.

**Goldens (R-23).** One per output contract: `mutation_v1` `document_promote_pull_request_ok` (the data, over SSH); `cli_v1` `document_promote_pull_request_preview` (JSON and text), `document_promote_pull_request_primary_holds` (the prepare's `already_applied`, JSON and text) and `document_promote_pull_request_blocked` (JSON and text). `item sync`'s link for a document uses Cycle 03's `pull_request_link` data and golden.

**Human output.** The preview prints its effects and the link's `human_line()`. The answer prints `publication: <state>` when a child completed, then the link's `human_line()` on its own line, or `<primary> already holds this document` when `primary_holds` is true; a prepare's `already_applied` in a pull-request repository prints the same sentence. A unit test of the CLI's human output pins the three lines.

**The desktop.** Compile-level arms for the new preview and data (R-14). "Approve and merge" keeps its words until Cycle 06.

## Part E: Reads: The Document's Link

`fill_complete_read` fills `pull_request` for a document as well as a ticket (decisions 3 and 9, R-9): for a document read from its active context, with a link row, the row's link while the primary does not hold the document, and null once it does. It is null for a document with no row or no active context, and in every list, as today. It reads the row in either mode, as the ticket's does.

Show's check is the cheaper twin of R-3 (decision 9), `document_link_outstanding`: the same rule over the trees' entries, without loading blobs, with the document found at its path in `L` rather than by scanning every item. It differs from R-3 only in a repository with missing objects or a document ID held at two paths (K-11). Promote and `item sync` keep the full proof.

`ItemDto.pull_request`'s documentation and `item.schema.json`'s description change to match (D-1). `ClosureDto.publication` is unchanged.

## Part F: `item sync`'s Link For A Document

Cycle 03's rule for a closing ticket (its Part H, decision 12) gains documents (decision 4, R-10): when `item sync` of a document context in a pull-request repository publishes, or finds current, the branch, and the document has a link row, and the primary does not hold the document after the synchronization (R-3, the full proof), the answer carries Cycle 03's `pull_request_link` data with the row's link. `item sync` never writes a document's row, so `given_at` is not renewed. Every other `item sync` answer is unchanged, in a direct repository too.

## Part G: The Lifecycle's Adaptations

These are required. The design checkpoints of Tasks 2 and 3 write each one down against the source before code:

1. `PromotionRecord::path()` and `answered()` read `integration_mode` and `answer_code`; `begin_record` takes the mode from the observation.
2. `DocumentPromotionLifecycle`'s `branch_step` (the first table, under the lease, finishing the record, returning `Finished`) and `end_branch` (the second table) are implemented; `begin_publication` and `branch_answer` stay unreachable for a promotion (it has no publication child), and `BranchFinish` stays `Infallible`.
3. Promotion's `type Outcome` becomes an enum of a direct and a branch outcome, as the closure's `ClosureOutcome` is; `DocumentPromotionResult` gains `Branch(BranchPromotionOutcome)` and its index-pending twin; `branch_outcome` reads the record's `publication_category`; the exhaustive matches of Baseline Fact 7 gain arms.
4. The binding's `Called::Branch` holds either lifecycle's branch outcome, `Promote` builds it, and a promotion twin of `branch_code` maps `PromotionAnswer` to its code; the binding reads the link row to build the answer's `pull_request`, so a replay has it; `reached_*` and the recorded effects read the path; the CLI's retry guard reads the new data.
5. The digest's domain constant sits beside `manyhands promotion v1`.
6. The remote check before the child is unchanged (R-20).
7. The places that read a record's second slot (`remote_identity_of`, `children_contexts`, `audit`) answer for a promotion branch record with an empty second slot.
8. A pull-request promotion takes no new hold. While it is unfinished it holds what an unfinished direct promotion holds before integration: its document's prepare for a new request, and its context child's reservation while that child stands.
9. The link helper is shared: Cycle 03's `give_link` and `branch_link` take the kind (or a document twin sits beside them), so the document's row is written with kind `document`.
10. In pull-request mode `runs_context_child` is true and `require_identity` passes.
11. `RunEvent::Recording` is not emitted before a promotion's `branch_step`.
12. Promotion's `type Preview` becomes an observation type that carries the mode and the pull-request facts (`L`, `R`, the situation, the link), with the primary OID optional or absent in pull-request mode, so that `Ready`, the digest and `begin_record` take it; the checkpoint chooses the shape that changes the fewest of `DocumentPromotionPreflight`'s exhaustive matches, and lists each match it changes (E-12).
13. A library fake publisher that fetches, merges and pushes through a local bare repository, and refuses a synchronization of the primary, is shared by both lifecycles' tests: Cycle 03's `PullRequestIo` is lifted into a test-support module, or a promotion twin is written from it (U-2).

## Required Outcomes

| # | Outcome | Proof |
| --- | --- | --- |
| P1 | The rebuild once from a v2 table holding rows and on a fresh registry; existing rows read `direct`; the columns, steps, refusals and CHECKs per mode | `src/repository/promotion/record_tests.rs` |
| P2 | Part B: dispatch per mode, the situations in order, the effects, the blockers kept and dropped, `history_unknown` not blocking, each digest field | `src/repository/promotion/observe_tests.rs`, one table |
| P3 | R-3: a new document and an edit of a merged document, each before a merge and after a merge commit, a squash and a fast-forward, against the local and the tracking primary, with `R` absent, equal, behind, ahead and beside; a shallow repository and two merge bases read not held | `src/repository/promotion/observe_tests.rs`, one table |
| P4 | C1 to C24 and both answer tables | `src/repository/promotion/run_tests.rs`, a `pull_request` module, with the shared fake publisher (Part G item 13) and the clock injected |
| P5 | The owner's lifecycle, end to end in the library: promote, promote again, a save and promote again, then a forge merge by merge commit, squash and fast-forward, with the branch deleted, for a new document and an edit of a merged one; before the merge neither the poll nor promote reports the document promoted, and promote and `document show` offer the link; after it the poll reports `Promoted`, retirement succeeds and the link row is gone | the same module, with a local bare repository driven by `git2` as the forge, the shared fake publisher, `synthetic_fetch` for the poll and `execute(Mutation::PollOnceRetiring)` for the retirement |
| P6 | `document show` and `show --path`: the link while outstanding, null once held, null without a row or an active context, null in lists; the cheap check agrees with R-3 on P3's rows | `tests/read_items.rs`, beside `ticket_show_gives_the_link_and_the_publication`; a table beside P3 |
| P7 | `item sync`'s link for a document: given when outstanding, none when held, without a row, or in a direct repository; no row written | a binding test beside Cycle 03's `item_sync_gives_the_link_of_a_closing_ticket` |
| P8 | The CLI contract with no network: the pull-request preview, a prepare's `already_applied`, a blocked prepare, and a confirm stopped at the context child | `tests/cli/lifecycle.rs`, with the `cli_v1` goldens of Part D |
| P9 | The human output's three lines | a unit test beside the CLI's existing human-output tests |
| P10 | End to end over SSH: promote and promote again in a pull-request repository, the server's primary unchanged throughout; the success golden | Cycle 03's SSH case extended (decision 6) |
| P11 | Every registration: the data and preview schemas, `DATA_ROOTS`, `from_stored`, the enumerations, the CLI cases, the inventory, the recovery tables | the existing registration tests |
| P12 | A direct promote is unchanged | the existing promotion tests, and the `document_promotion` target at review-ready |

## Testing

Each test sits at the cheapest layer that catches its regression (AGENTS.md). The library tests carry the edge cases: P1 to P7 need no network, and P5 simulates the forge with a local bare repository written by `git2`, as Cycle 03's library lifecycle tests do. The shared fake publisher of P4 and P5 refuses a synchronization of the primary, and every such test asserts that the local primary did not move: that is how "a test fails if one tries" holds in the library. P8's goldens come from the CLI with no network, in a `format_version = 2` repository with a selected key and a publication remote on a closed port. P10 needs the SSH harness, because no non-SSH publication remote can be selected (Baseline Fact 19) and the success golden of a publishing answer comes only from a real publication; it adds steps to Cycle 03's one case rather than a case of its own (decision 6). No real waits: the runner takes its clock.

At review-ready the SSH tier is the targets AGENTS.md's table selects for the paths this Cycle touches: `document_promotion` (`src/repository/promotion/`, and its own test files, which E-5 edits), `mutation_remote` (`src/repository/mutation/`, and its own test files), `ticket_closure` (the shared link helper in `src/repository/closure/`, and the runner both lifecycles share) and `cli_remote` (the CLI's human output for promote).

## Named Edits To Pre-Existing Tests

- **E-1** `tests/read_items.rs` `ticket_show_gives_the_link_and_the_publication` (about 6244-6256): its comment "nor does a document's read, whatever row it has" becomes "nor does a read of a document without an active context". Its assertion stands, because its document has no context (Baseline Fact 15); P6's test pins a document with a context. This changes the comment's stated intent, not the assertion, and is named here for the owner's approval.
- **E-2** `tests/mutation_contract.rs` `DATA_ROOTS` gains the two new roots (its length and list).
- **E-3** `tests/recovery_foundation_gate.rs` `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly` (about 1075): its marker list, read `ORDER BY name` (about 1125), gains `wave_06_promotion_modes_v1` at its sorted place.
- **E-4** `src/repository/promotion/record_tests.rs`: `migration_preserves_legacy_rows_is_idempotent_and_audited` (about 366), `half_created_migration_and_missing_marked_table_fail_closed` (about 403), `record_forward_transitions_and_invalid_backwards_recover` (about 417), `schema_rejects_free_text_categories_steps_and_refusal_payloads` (about 560) and `privacy_column_inventory_for_both_new_tables` (about 581) gain the mode's rows and the two columns, their existing rows unchanged; `promotion_refusal_blockers_v2_rebuilds_an_old_bound_table_once` (about 1417-1458) is rewritten to build its old table from `SCHEMA_V2` with an explicit column list, as `closure/record_tests.rs` does with `V2_COLUMNS` (about 1782), its assertions unchanged.
- **E-5** The exhaustive matches on `DocumentPromotionResult` in tests gain the `Branch` arms: `promotion/run_tests.rs` about 5069-5074, `tests/document_promotion.rs` about 314-320, `tests/document_promotion/journeys.rs` about 257-261, `tests/document_promotion/quality.rs` about 130-134.
- **E-6** The exhaustive `MutationDataDto` matches in tests gain the new variant: `tests/support/mutation.rs` (six), `tests/support/repair.rs` (about 337-343), `tests/mutation_remote/c5_lifecycle.rs` (five).
- **E-7** `tests/support/cli.rs` `CLI_CASES` and `CLI_TEXT_CASES` gain Part D's three `cli_v1` cases.
- **E-8** `tests/cli/inventory.rs`'s `DATA` row for `document promote` (about 265-270) gains the two new schemas.
- **E-9** `tests/support/mutation_golden.rs` `CASES` gains `document_promote_pull_request_ok`.
- **E-10** `tests/cli/recovery_tables.rs` and `tests/cli/collaboration_exits.rs`: the rows D-1 adds to the contract's tables, with their goldens or reasons (Task 0 names the tables each parses).
- **E-11** `tests/read_contract.rs`'s enumerations: any enum the two new schemas declare.
- **E-12** The exhaustive `DocumentPromotionPreflight` matches that Part G item 12's shape changes (Task 2's checkpoint lists them).

Task 0's sweep greps for: `DocumentPromotionResult::`, `DocumentPromotionPreflight::`, `MutationDataDto::`, `PromotionStep::`, `document_promotions` and `promotion_refusal_blockers_v2` in `src/` and `tests/` (among them the column scans of `tests/ticket_closure.rs` about 449 and `tests/document_promotion.rs` about 886, and the row inserts of `tests/read_status.rs` about 1798), `manyhands promotion v1`, `"pull_request": null` in document goldens, and the skills' sentence that promote still pushes the primary until Cycle 04. Each hit that this design changes is added as E-13 onwards; one that removes, weakens or reverses an assertion's intent stops for the owner.

## Document Amendments

- **D-1** `docs/RFC/cli-contract.md`:
  - the `document promote, ticket close` row and the promote rows of the answers table: the pull-request promote, its situations, preview, digest, answers, stops, cancellation and replay; a prepare's `already_applied` with no data; the way out of its `remote_branch_deleted` (decision 11), with each finished answer's recovery, `next:` lines and golden or reason;
  - the known limit of decision 35: in a pull-request repository, `history_unknown` does not block promote's prepare (decision 10);
  - the vh-f3wts amendment's index hand-off of the primary does not apply to a pull-request promote, which writes no primary; its exit row owes only the document's context; documents have no comment-publication blocker, as the direct promote has none;
  - `document show`'s `pull_request` for a document, and `item sync`'s link for a document, changing Cycle 03's "null for documents" (its Contract 12) and its rule that every `item sync` answer carrying a link writes the row (its Part C);
  - the new data and preview shapes.
- **D-2** `docs/RFC/pull-request-mode.md`: dated amendments for decisions 1, 2, 3, 4, 10 and 12, and the departures the Cycle document lists.
- **D-3** `docs/RFC/repository-index-persistence-and-refresh.md`: the `document_promotions` rebuild and its migration.
- **D-4** The shipped skills, in `skills/` and `.agents/skills/`: `manyhands-documents` (promote in a pull-request repository: the link, promoting again, "the primary already holds this document", a deleted branch and its republish, K-9's stale link, and cleanup after the merge through retirement, recommending that the forge delete the source branch at merge); `manyhands-finding-work` (`document show`'s `pull_request`); `manyhands-sharing-and-sync` (`item sync`'s link for a document); `manyhands-recovery` (a stopped promote is continued by its request, and holds the document's prepare meanwhile); and Cycle 03's sentences that promote still pushes the primary until Cycle 04 are removed.
- **D-5** `docs/research/wave-03-api-audit.md`: `document promote`'s codes gain `already_applied` (at prepare and confirm), the synchronization's stops, `history_unresolved` and `remote_branch_deleted` as a finished answer.
- **D-6** At closure, with the owner's go-ahead, through `manyhands-documents`: Cycle 04's checklist in `docs/Waves/wave-06-pull-request-workflow.md`; the status of this Cycle's three documents set to done; then the three promoted, so their links to each other resolve on main.

## Rulings

Each: decision — why — cost if wrong.

- **R-1** The pull-request promote is a branch record of `document_promotions` with one child, and `branch_step` or `end_branch` finishes it — decision 1, and nothing of Manyhands' own is written on the branch, so there is no hook and no publication child — `finish_branch` and the publication slot are dead code for a promotion.
- **R-2** A record's mode is fixed when it begins; prepare chooses the observation by the configuration, and the pull-request observation's digest has its own domain — Cycle 01 decision 11, Cycle 03 R-2 — a switch between prepare and confirm costs a re-prepare.
- **R-3** "The primary holds the document" is the proof for `L` against the local or the tracking primary, with `R` absent, equal to `L` or behind it — the RFC's content proof; the tracking primary is fresher after a fetch, the local primary is what a poll fast-forwards; `R` ahead or beside is work the clone has not merged — a document whose remote copy has another clone's later saves reads not held until a synchronization merges them.
- **R-4** When the primary holds the document, prepare answers `already_applied` with no data and no record, as Cycle 03's withdraw prepare does — there is nothing to publish, cleanup belongs to retirement, and in a pull-request repository that code at prepare has one meaning — a caller learns the reason from the code and `document show`, not from data.
- **R-5** Decision 10's mechanics: the pull-request observation reads `Unknown` history as no blocker, and the context child's fetch settles it (Baseline Fact 4) — a branch deleted since is found at confirm (`remote_branch_deleted`), not at prepare.
- **R-6** The pull-request prepare drops the root and integration blockers, `identity_required` and `require_identity`, and keeps `primary_missing` — nothing touches the primary, the child's own stops (identity included, for a merge of the primary into the branch) answer as `item sync`'s, and the observation needs the tracking primary — a root that is dirty or on another branch does not stop a promote.
- **R-7** The digest binds `L`, `R` and the situation — the consent is to publish what was previewed — a save between prepare and confirm costs a re-prepare.
- **R-8** A new data type and preview type for the pull-request promote — Cycle 03 decision 3 (the close's data type is the close's own) — two schemas more.
- **R-9** `document show` fills `pull_request` for a document from its active context while the primary does not hold it, in either mode, by the cheap check — decisions 3 and 9 — show disagrees with promote only in a repository with missing objects or a duplicated document ID.
- **R-10** `item sync` reads a document's row and never writes it — decision 4 (it never creates a link) — `given_at` is renewed only by promote.
- **R-11** `primary_holds` finishes the record without touching the link row, which retirement deletes — decision 3 (the link ends by the proof) — a stale row outlives the merge until retirement, hidden by reads; K-9 is its edge.
- **R-12** After a child, R-3 is judged with the tracking primary as that child fetched it, for a child that completed or ended, before every other row — a forge merge during the promote reports the document held, not a new link — none.
- **R-13** A child that ended abandons the record with the refusal `reconfirmation_required` and its answer, as Cycle 03's `abandon_branch_record` does — the record's CHECK ties `abandoned` to a refusal — an abandoned row can carry a success answer (`primary_holds`), as Cycle 03's `already_closing` does.
- **R-14** The desktop gets compile-level arms only — Cycle 06 owns its design — the desktop shows a generic preview and no link actions until then.
- **R-15** No new SSH case: Cycle 03's case gains the promote steps — AGENTS.md's Testing Strategy, decision 6 — that case runs a few seconds longer, and fails for either Cycle's regression.
- **R-16** The runner takes its clock from the request (Cycle 03's `LifecycleRequest.clock`) for `given_at` — AGENTS.md's no real waits — none.
- **R-17** The link is built once, at `first`, and `again` returns the row's link and renews `given_at` — decision 2 (the same link) — after a change of forge setting the old link is offered until the row ends.
- **R-18** Another synchronization of the branch is the child's `busy` at confirm, not a prepare blocker — the direct promote has no such blocker, and the closure's queries tickets only — the person learns of it at confirm.
- **R-19** In pull-request mode the context child always runs, even when the plan would have been `AlreadyContained` — it is what publishes — none: a held document answers at prepare.
- **R-20** The remote check before the child applies as it does to a closure's branch record before its commit — decision 1, the lifecycle's rules; ending it otherwise would change the shared runner — a promote whose remote changed meanwhile behaves as such a close does.
- **R-21** No new fence against the poll's retirement while a record waits: a context retired meanwhile was proven held, and the next call answers `primary_holds` by the ended-child table — retirement requires the proof — none.
- **R-22** The way out of a pull-request promote's `remote_branch_deleted` is `item republish`, then a new promote, keyed on the new data — decision 11 — the direct promote, the close and `item sync` keep theirs; after a squash on which the primary moved on, the republish revives a merged branch (K-12).
- **R-23** One golden per output contract, and `item sync`'s document link reuses Cycle 03's golden — AGENTS.md's golden rule — the human lines are pinned by a unit test instead of text goldens for every answer.

## Known Limits

The Cycle document's K-1 to K-12.

## Unverified

- **U-1** Cycle 03's names in Baseline Fact 14 are its design's, not yet code. Task 0 re-checks them after Cycle 03 merges and uses the names as landed; a change of shape stops for the owner.
- **U-2** Whether Cycle 03's `PullRequestIo` can be lifted for both lifecycles' tests without changing Cycle 03's tests, or a promotion twin is cheaper (Part G item 13). Task 3's checkpoint decides.

## Design Checkpoint Obligations

Task 2 writes down `primary_holds_document`'s signature and how it finds `L`, `R` and both primaries without a fetch, and Part G items 1, 5, 10 and 12, with each `DocumentPromotionPreflight` match it changes. Task 3 writes down Part G items 2 to 4, 6 to 9, 11 and 13, and settles U-2. Each is checked against the source and approved by the orchestrator before code.

## Alternatives Rejected

- Handing the promote to `item sync`'s binding with no lifecycle record: decision 1; a stopped promote would be listed, resumed and replayed as an `item sync`, the separate-runner shape Cycle 03 left after three review rounds.
- Answering `already_applied` at prepare with no synchronization while the link is outstanding: decision 2; the pull request would lag the person's saves.
- A link that ends only when its row is deleted: decision 3; `document show` would offer a merged pull request's link.
- The full proof in `document show`: decision 9; it reads every blob of two trees on every show.
- Keeping `history_unknown` as a prepare blocker: decision 10; the usual path, `item sync` and then promote, would need a poll first.
- A blocker `already_promoted`, or data on a prepare's `already_applied`: there is nothing to do, which is `already_applied`, and Cycle 03's withdraw prepare already gives that answer with no data.
- Rebuilding the link on `again`: decision 2's "the same link".
- Ending a promote whose remote changed with a new answer: R-20.
- A new SSH case: decision 6.

## Risks

| Risk | Mitigation |
| --- | --- |
| The runner's shared code changes break a direct promote or a close | Every direct row of P4 and P12, Cycle 03's `d1_…` and `d2_…`, the `document_promotion` and `ticket_closure` targets at review-ready |
| The rebuild of `document_promotions` damages a real registry, one way | The frozen v2 DDL, an explicit column list, P1 from a v2 table holding rows, the migration in the registry-schema transaction |
| Cycle 03 lands with other names or shapes | Task 0 re-grounds every Baseline Fact after it merges (U-1) |
| The observation type change ripples through `DocumentPromotionPreflight`'s twelve files | Task 2's checkpoint chooses the shape with the fewest changes and lists them (E-12) |
| The answer tables misreport a forge merge during a promote | C9, C10, C19 and C22, and P5's merges |

## Review Record

| Round | Reviewers | Blocking | Important | Minor | Resolution |
| --- | --- | --- | --- | --- | --- |
| 1, revision 1 (`4e1bbb2e`) | Source and feasibility | 1 | 11 | 7 | Resolved in revision 2: the ended-child steps and refusal (Part A, R-13); the observation type, `primary_missing`, the context child and identity (Part B, Part G items 10 and 12, R-6, R-19); a prepare's `already_applied` (R-4); the outcome and replay plumbing (Part G items 3 and 4); `busy` (R-18); the way out's data (Part D); the edits (E-4 to E-9, E-12); the seams (Part G item 13, Baseline Fact 19); U-1 and the history premise confirmed (Baseline Fact 4); `RunEvent::Recording` (Part G item 11); the proof's cost (Baseline Fact 11, decision 9) |
| 1, revision 1 | Contract, consistency and completeness | 2 | 15 | 16 | Resolved in revision 2: the same link (R-17, C7); `repo inspect` (decision 12, K-10); the step graph (Part A); the task order (plan); the way out (decision 11, R-22, D-1, K-12); `history_unknown` (decision 10, D-1); the registrations (E-6 to E-11, D-5); the first table's reading (R-12); a merge fetched before confirm (C19); the effects and "first, already current" (Part B, C3); the stale link (K-9); "active context" (Conventions); the closure step (D-6); the plan's split of Task 3; the missing cases (C22, C23); U-2 (R-20); and the Minor findings in the text |
