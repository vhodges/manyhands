---
title: "Wave 03 D5 Desktop Conflict Recovery, Promotion And Closure Design"
date: 2026-11-03
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6HPQ3P18JW40SD2P8HFBB65"
---

# Wave 03 D5 Desktop Conflict Recovery, Promotion And Closure Design

## Intent And Approval Boundary

This design covers D5's library part (L1–L3), the lifecycle dialog for
Close ticket… and Approve and merge…, the resolve conflict sheet, local
synchronization and repair, republish, and their evidence. Approval covers
the mechanisms, rulings, named edits and task order. It does not authorize
implementation, pushing beyond the ticket branch, merging or closing.

Revision 5, after review rounds 1–3 (Review Record). Revisions 4 and 5 had
no independent review. The documents were drafted against C5's tip
`f59aceb6` (Cycle decision 1) and re-grounded on merged C5 (`2d912d10`,
merged here as `cd2e922e`) on 2026-11-04 (Re-grounding, below). Facts
marked **(C5)** hold on merged C5; all others at `e39bb004`, unchanged by
C5.

## Re-grounding On Merged C5 (2026-11-04)

C5's commits after `f59aceb6` (`aa0934a3..323924af`) were compared with
every (C5) fact. What changed for D5:

- **Decision 35.** A published context with no tracking ref and no fetch
  that saw it gone answers `history_unknown`, not `remote_context_deleted`
  (`R/promotion/observe.rs` `remote_context_history`;
  `RemoteSnapshot::context_deletion_observed`). The CLI's `way_out` now
  routes `history_unknown`, like `primary_missing`, to `poll once` with its
  own note (`UNKNOWN_NOTE`). L1 takes this rule (Fact 7, L1).
- **Add/add note.** A blocked prepare with `integration_conflict` also
  prints `EXTERNAL_REPAIR_NOTE` (decision 30): the repair's
  `context sync-local` may answer `external_resolution_required`. L1 keeps
  the route `RepairLocally`; the note stays the CLI's text, and the
  desktop's Repair locally… already handles that answer (B4).
- **Holder reads** (`records::operation_or_parent_holder`, decision 34) and
  `LifecycleVerb::holder` taking a read session: internal, no boundary
  change.
- `set_made_ready_hook_for_testing` is now an associated function
  (`RepositoryService::set_made_ready_hook_for_testing(hook)`); D5's tests
  do not use it.
- Line citations in `human_lifecycle.rs` moved by +6 to +17; Facts 7 and
  20 and L1 cite the merged lines.

No (C5) fact's meaning changed otherwise; U-1 to U-5 stay for Task 0.

Conventions: "Fact n" refers to Baseline Facts; `D/` is `src/desktop/`; `R/`
is `src/repository/`; "the handoff" is `docs/design/05-sheets-and-dialogs.md`;
GRFC is the Git workflow and conflict recovery RFC.

## Baseline Facts At `e39bb004` And Merged C5 (`cd2e922e`)

**Library boundary (consumed).**

1. `RepositoryService::prepare(&Mutation) -> Envelope<PrepareDataDto>`
   (`R/mutation/confirm.rs`) and `execute(MutationCall, &mut
   SessionCredentials<P>, &mut dyn ProgressSink, &CancellationToken)`
   (`R/mutation/execute.rs`). Consent lives 600 s
   (`CONFIRMATION_LIFETIME_SECONDS`, `confirm.rs:47`); an accepted
   request's consent stands however old.
2. **(C5)** `Mutation::DocumentPromote(DocumentPromoteInput{root, id})` and
   `Mutation::TicketClose(TicketCloseInput{root, id})`
   (`R/mutation/bind/lifecycle_verb.rs:37,44`). No identity or input;
   `closed_by` is the configured identity (C5 decision 6). Both confirm,
   stop at safe points and have an operation (`R/mutation/command.rs:153,366`).
   C5 binds consent to the item's change set and relation, not to moving
   OIDs (C5 contract 1).
3. **(C5)** `PreviewDetailsDto::Lifecycle(Box<LifecyclePreviewDto>)`
   (`R/mutation/dto.rs:243`); the DTO (`:310-330`) holds `root`, `kind`,
   `item_id`, `slug`, `branch`, `primary_branch`, `remote`, `context_oid`,
   `primary_oid`, `changes: Vec<LifecyclePathChangeDto{path, change}>`,
   `integration`, `publication`, `cleanup{remote_branch, worktree,
   local_branch}` and `closure: Option<{recording, closed_by}>`. No title,
   body, `closed_at`, line counts or made-ready list.
4. **(C5)** A blocked prepare is `PrepareDataDto::LifecycleBlocked(
   LifecycleBlockedDto{blockers})` (`dto.rs:365,686`); 23 blocker values
   (`:335-358`), including `primary_dirty`, `primary_ahead_of_remote`,
   `primary_missing`, `context_missing`, `integration_conflict`,
   `remote_context_deleted` and `conflict_pending`. Its code is
   `identity_required`, `ticket_closed` or `recovery_required`
   (`R/mutation/bind/lifecycle.rs:153`). A held item answers
   `recovery_required` with one `request.retry` or `operation.resume` and
   no data (C5 decision 33). At execute, a stop carries the same blockers
   (`stop_answer`, `:1223`).
5. **(C5)** `MutationDataDto::Lifecycle(Box<LifecycleMutationDto>)`
   (`dto.rs:726,831-858`): `recording`, `integration`, `publication`,
   `publication_pending`, `cleanup`, `cleanup_pending`, `blockers`,
   `made_ready: Option<Vec<LifecycleMadeReadyDto{id, slug}>>` (`:821`) and
   `made_ready_state: Option<LifecycleMadeReadyState>` (`:809`: `current`,
   `index_pending`, `index_unavailable_final`, `not_computed`,
   `not_applicable`). No item slug and no `closed_by`.
6. **(C5)** Sending the same request ID again continues an unfinished
   parent, even after cancel (C5 decision 8), with no consent recheck. A
   stopped request with no parent record is deleted, and the next step is
   a new prepare (C5 contract 2). `cancel_request` reaches the lifecycle and
   its children.
7. **(C5)** The way-out rules live in the CLI binary:
   `src/bin/manyhands-cli/human_lifecycle.rs` `branch_deleted` (`:162`),
   `merge_refused_route` (`:198`), `child_conflict_route` (`:224`),
   `route_of` (`:265`), `way_out` (`:295-361`, which also routes
   `primary_ahead_of_remote` to `poll once` then `repo sync`,
   `primary_missing` and `history_unknown` (C5 decision 35) to `poll once`,
   and `[context_missing]` to an item save), `held` (`:366`), `prepare`
   (`:408`, which adds `EXTERNAL_REPAIR_NOTE` for `integration_conflict`).
   Its tests are bin unit tests, `src/bin/manyhands-cli/lifecycle_tests.rs`.
8. **(C5)** `promotion_operations` and `closure_operations`
   (`R/read/status.rs:365,413`) list unfinished parents with
   `next_action: None`. `lifecycle_standing` (`R/mutation/bind/resume.rs:246`)
   treats a refused parent as not resumable. A run stopped by Stop, quit
   or a crash leaves its request `accepted`; `operation resume` on its
   parent answers `original_request_required` naming that request
   (`bind/resume.rs:335-349`).
9. `ProgressStage` (`R/mutation/progress.rs:83-94`): `accepted`,
   `observing`, `writing`, `connecting`, `fetching`, `updating_local`,
   `pushing`, `verifying`, `indexing`, `stopping`. Its values are pinned by
   `schemas/v1/progress_event.schema.json` (`tests/read_contract.rs:1139-1143`)
   and matched exhaustively by `D/model/operations.rs:584-610`
   (`stage_text`). **(C5)** A lifecycle reports `accepted` then
   `connecting` only (before L3). The step runner is `run_lifecycle`
   (`R/promotion/run.rs:60`), shared by closure through
   `R/closure/lifecycle.rs`; its child synchronization gets only a
   `CancellationObserver` of safe points (`run.rs:773-786`).
10. `show_conflict` (`R/remote/conflict_read.rs:190`) returns `ConflictDto`
    (`:132`): `operation_id`, `target`, `item_id`, `stage`, `next_action`,
    `eligibility`, `observation`, `historical`, and `paths:
    Vec<ConflictPathDto{display_path, token, kind, structure,
    content_available, base, local, incoming}>`. **(C5)** It also accepts a
    lifecycle parent's ID (C5 decision 34).
11. `Mutation::ConflictResolve(ConflictResolveInput{root, operation_id,
    observation, resolutions: Vec<ConflictResolutionInput{token, source}>})`
    (`R/mutation/bind/conflict.rs:90,99`). `source` is the path's whole
    result text. A successful resolution of a share's conflict continues
    that synchronization (`bind/conflict.rs:31-43`). **(C5)** For a
    lifecycle child, "the parent's runner then adopts the published child"
    when the parent request is sent again (`bind/conflict.rs:45-52`).
12. Local boundary: `observe_local_context_sync` (`R/local_sync/observe.rs:360`),
    `inspect_local_context_sync` and `read_local_context_conflict`
    (`R/read/local_sync.rs:110,117`). The inspection DTO (`:47`) carries
    `state`, `reason`, `next_action`, `observation`, `eligibility` and
    `paths: Vec<{display_path, token, content_available}>`, with no kind or
    structure. `Mutation::LocalContextSync` and
    `Mutation::LocalContextResolve` (`R/mutation/bind/local_sync.rs:10,20`).
    The CLI's `context resolve-local` takes the observation the inspection
    reports. A partial local resolution needs its original source bytes on
    retry (GRFC local amendment).
13. `Mutation::ItemRepublish(ItemRepublishInput{root, kind, id})`
    (`R/mutation/bind/republish.rs:64`); `PreviewDetailsDto::Republish`.
14. `canonical::parse_item`, `serialize_item` and `item_path_kind`
    (`src/canonical.rs:439,911,855`) are public. `serialize_item` writes
    unknown keys first and known keys in canonical order, LF only; a round
    trip preserves values, not bytes (`tests/canonical_foundation.rs:755-770`).
15. The ticket read's `ClosureDto` carries `closed_at` and `closed_by`
    (`R/read/dto.rs:480-488`); `slug` is nullable for tickets made before
    the relationships RFC.

**Desktop (changed).**

16. Header: `later_action` (`D/view/item/header.rs:104`) draws Close
    ticket… and Approve and merge… disabled with `LATER_NOTE`;
    `header_later` (`:131`); labels `D/model/item.rs:35-47`.
17. Conflict banner: `Banner::Conflict` (`D/view/item/banners.rs:101`),
    pushed with plain text `CONFLICT_BANNER` (`:221`;
    `D/model/operations/outcomes.rs:43`) when the share outcome or
    `post_conflict` (`D/view/item/share.rs:178`) says so. The operation ID
    is in `SyncOutcome::Conflict{next}` and `CommentOutcome::SavedConflict
    {next}` (`D/model/publication.rs:62`).
18. `BRANCH_DELETED` (`outcomes.rs:39`) says republishing arrives later;
    `ResultCode::RemoteBranchDeleted` offers Give up only.
19. `resume_offer` (`D/model/operations.rs:924`) shows a local-context
    synchronization's Resume unavailable (`SYNCHRONIZATION_UNAVAILABLE`,
    `:55`); `is_synchronization` (`:879`) would also catch a lifecycle row.
20. Confirmation: `confirm` and `confirm_followed` (`D/view/confirm.rs:161,174`),
    `offers_prepare_again` (`:139`), `STALE_CODES` (`:123`), `detail_fields`
    (`:611`, Republish arm `:700`). `ConfirmDialog::prepare` is private
    (`:349`). **(C5)** adds a `Lifecycle` arm showing Branch, Primary and
    Remote (`:706`).
21. Services: `Services::prepare` (`D/services/reads.rs:177`), `read` and
    `read_job` (`:117,149`). Worker: `submit`, `submit_with_request_id`,
    `give_up`, `record_state`, `reconfirm`, `in_flight`
    (`D/services/worker.rs:333,353,384,392,424,535`). `Intent`
    (`D/services/requests.rs:60`) and `JobKind` (`D/services/mod.rs:99`)
    have no lifecycle, conflict, local-sync or republish variant. The
    desktop keeps no request IDs across a restart.
22. Drafts: `flush_now` (`D/services/drafts/flush.rs:35`), `save`
    (`D/services/drafts/save.rs:82`).
23. Stale-edit review: `D/model/review.rs` and
    `D/view/item/edit/review_view.rs` (`result_view` `:251`).
24. `ReadOnly::new` (`D/view/editor/read_only.rs:65`).
25. `EditTabs::unkept` (`D/services/edit_tabs.rs:57`).
26. Closure display exists: `D/view/item/details.rs:197-224` shows
    Lifecycle, Closed at and Closed by apart from Status; `pill.rs:84,101-109`
    shows Closed.
27. Test seams: `DelayingBackend::hold_job_return_answering`
    (`D/test_support/delaying.rs:115-121`) substitutes an execute answer;
    there is no canned prepare or read. `publish_over_ssh`
    (`D/test_support/fixtures.rs:870-905`) contacts nothing, and desktop
    transport calls return `RuntimeUninitialized` (D4 fact 17).

**Unverified, settled in Task 0 by reading.** Any answer that differs from
expectation stops for the owner.

- **U-1.** A successful `LocalContextResolve` finishes the local
  synchronization without a push.
- **U-2.** `canonical::item_path_kind` classifies a local entry's
  `display_path` (escaping covers only non-printables and backslash,
  `R/local_sync/request.rs:317-330`).
- **U-3.** After resolving a lifecycle child's conflict, sending the
  lifecycle request ID again finishes it, or answers `external_change` when
  the change set moved (C5 decision 34).
- **U-4.** `RemoteOperationSafePoint` values are enough to emit the
  child's `connecting`, `fetching` and `pushing` from `run.rs`, with no
  change under `R/remote/` or `R/local_sync/`. If not, the child's stages
  are not forwarded (K-11) rather than widening Part 0.
- **U-5.** Where the D2 draft store's directory is resolved in the
  desktop, so the resolution store sits beside it under the same
  protection.

**Settled in Task 0 (2026-11-04).** U-1 and U-3 hold. U-2, U-4 and U-5
hold in part; the owner ruled on each (Cycle decisions 22–24, corrections
accepted the same day):

- **U-1 holds.** `R/local_sync/resolve.rs` moves only the local context ref
  (`complete_candidate`, `finish`); nothing under `R/local_sync/` pushes.
- **U-2 partly.** `display_path` escapes every byte outside `' '..='~'`
  (`request.rs:317-330`), and `classify_canonical_path` rejects a `\`
  (`canonical.rs:773-778`). Tickets, comments and ASCII documents classify;
  a document path with non-ASCII bytes or a control character does not and
  goes to the external guidance (decision 23, K-14).
- **U-3 holds.** A resend adopts the completed child (`run.rs:824-826`); a
  moved change set abandons with `ReconfirmationRequired`
  (`run.rs:615-629`), answered `external_change`
  (`bind/lifecycle.rs:797-798`). Proven by
  `tests/mutation_remote/c5_child_conflict.rs` `resolved_then_the_same_request`
  and `unconsented_resolution`.
- **U-4 partly.** The child's safe points map to `fetching`,
  `updating_local` and `pushing` as `bind/sync.rs` `stage_at` maps a
  share's, but no safe point yields `connecting`. `run_lifecycle` emits
  `connecting` itself when it starts the context child's synchronization
  (before `io.synchronize`), inside Part 0's files. The cancel-acknowledgement
  synchronization (`run.rs:865-871`) forwards no stages, and a resend that
  adopts an already-published child shows none for Share with origin.
  K-11 no longer applies.
- **U-5 partly.** The store is `<data_dir>/drafts`, opened in the library
  (`DraftStore::open_default`, `src/drafts/store.rs:222`; `D/mod.rs:98`),
  and its protection (`protect::Folder`) is `pub(crate)`. Decision 22: Part
  0 adds L4, a public protected folder by name, which the desktop's
  resolution store uses.

## API Audit Refresh (Entry-Gate Item 3)

| Audit row | D5 consumes | State on merged C5 (`cd2e922e`) |
| --- | --- | --- |
| Promotion/close preflight and cleanup bindings | `DocumentPromote`, `TicketClose`, prepare, execute | Delivered by C5 (Facts 2–6). |
| Conflict inspection read, remote | `show_conflict`, `ConflictResolve` | On main (Facts 10–11). |
| Conflict inspection read, local | observe, inspect, read, `LocalContextSync`, `LocalContextResolve` | On main (Fact 12). |
| Republish | `ItemRepublish` | On main (Fact 13). |
| Lifecycle way out | none | Missing: L1. |
| Lifecycle resume listing | `list_operations` | Missing: L2. |
| Lifecycle progress | `ProgressSink` | Missing: L3. |
| Made-ready titles | ticket read | Existing read. |

Drafted against an unmerged branch (Cycle decision 17) and re-checked on
merged C5 on 2026-11-04: every row stands.

## Execution Order (Decision 8)

Serial, on the ticket branch, in `.manyhands/worktrees/01M4CC0VN2K8DBHNMZ46NTX5JJ`.

```
Task 0 → Part 0: L1 → L2 → L3 → checkpoint
  → Part A: seams → dialog (prepare, confirm) → outcomes → made-ready → after-lifecycle
  → Part B: model → sheet → external → entry points and local sync
  → Part C: republish, resume
  → Part D: keyboard → interruption and counts → privacy and consent → gates
```

## Part 0: Library (Decisions 2, 3, 10, 11)

Files: `R/mutation/way_out.rs` and `way_out_tests.rs` (new),
`R/mutation/mod.rs` (export), `R/read/status.rs`,
`R/mutation/bind/resume.rs` (share `lifecycle_standing`),
`R/mutation/progress.rs`, `R/mutation/command.rs` (the lifecycle's
pre-call stage, and its table test at `:767-773`),
`schemas/v1/progress_event.schema.json`,
`R/promotion/run.rs`, `R/promotion/lifecycle.rs` (L2's shared
`resumable` predicate, owner 2026-11-04), `R/closure/lifecycle.rs`,
`R/mutation/bind/lifecycle.rs`, `R/mutation/bind/lifecycle_verb.rs`,
`R/promotion/mod.rs` and `R/closure/mod.rs` (L3's observer parameter only,
owner 2026-11-04), `src/bin/manyhands-cli/human_lifecycle.rs`,
`D/model/operations.rs` (`stage_text` arms only, to keep the tree green),
`src/drafts/mod.rs` and `src/drafts/protect.rs` with its platform modules
(L4, decision 22), and tests in `tests/read_status.rs`, `tests/mutation_remote.rs` (its
`mod d5_progress;`), `tests/mutation_remote/d5_progress.rs` and the CLI
targets.

### L1: Way out

```rust
pub enum LifecycleWayOut {
    SendAgain,                                   // the same request ID
    RetryRequest { request_id: String },         // a held item (C5 decision 33)
    CancelThenSendAgain { request_id: String },  // a deleted branch
    ResumeOperation { operation_id: String },
    RepairLocally { item_id: String },           // context sync-local
    ResolveChildConflict { operation_id: String },
    Republish { item_id: String },
    SharePrimary,                                // repo sync
    PollOnce,                                    // poll once
    SaveItem { item_id: String },                // context_missing
    RepairOutside,                               // a document's integration_conflict
    Inspect,
}
pub fn lifecycle_way_out(envelope: &Envelope<MutationDataDto>) -> Vec<LifecycleWayOut>;
pub fn lifecycle_prepare_way_out(
    kind: ItemDtoKind,
    envelope: &Envelope<PrepareDataDto>,
) -> Vec<LifecycleWayOut>;
```

`kind` is a parameter because `LifecycleBlockedDto` holds only blockers,
as the CLI's `prepare(kind, …)` does (`human_lifecycle.rs:408`).

Every rule of Fact 7 is moved, keeping its order; the CLI renders the
returned values. `RepairOutside` applies to **execute** results only (a
document's `integration_conflict`, which the CLI's `route_of` leaves
without a route, `:289`); the CLI renders nothing for it. A blocked
prepare's `integration_conflict` gives `RepairLocally` whatever the kind
(`:344-360`); `kind` serves only `SaveItem`'s wording (`:338-342`).
`PollOnce` covers `primary_missing` and `history_unknown` (C5 decision 35);
the CLI keeps choosing its note (`POLL_NOTE` or `UNKNOWN_NOTE`) and the
add/add note (`EXTERNAL_REPAIR_NOTE`) from the blockers, so notes stay
CLI text and the variants carry no wording. Proof: C5's `lifecycle_tests.rs`, `cli` and `cli_remote`
lifecycle tests pass unchanged; `way_out_tests.rs` (lib unit tests, fast
tier) covers each variant.

### L2: Resume for lifecycle parents (decision 10)

`promotion_operations` and `closure_operations` set `next_action: Resume`
for every unfinished parent that `lifecycle_standing` finds resumable,
held or not; refused and finished parents stay `None`. The stale "no
boundary resume" comments go. The desktop's Resume handles
`original_request_required` (Part C). Named edits E-1, E-2.

### L3: Lifecycle progress (decision 11)

New `ProgressStage` values, added to the schema file with their
descriptions (non-breaking under the CLI contract's enum rule):

| Stage | Step it lights |
| --- | --- |
| `accepted`, `observing` (pre-call) | none yet |
| `connecting`, `fetching`, `updating_local`, `pushing` from the **context** child (U-4) | Share with origin |
| `recording` | Mark as closed |
| `integrating` | Merge into main |
| `publishing` (once, when the **primary** child starts; its own safe points are not forwarded) | Share main with origin |
| `cleaning_up` | Tidy up |

`Command::stage()` reports `observing`, not `connecting`, before a
promotion's or closure's domain call (`command.rs:480-490`), so a
local-only or cleanup-only run never shows "Connecting…". `run_lifecycle`
takes a domain observer of steps and of the context child's safe points,
by `ChildRole`; the mutation layer maps them to `ProgressStage` and the
request's sink. A step's stage is emitted only when the step runs, so a
cleanup-only replay reports `accepted, observing, cleaning_up` and a
local-only run reports no `publishing`. `stage_text` gains words for the
four stages. Named edit E-3, including `tests/cli_remote/journeys_c5.rs:793`
(`progress: connecting` before `transport_unavailable`, which the
`connecting` that `run_lifecycle` emits at the context child's start keeps,
per U-4's settlement).

### L4: Protected folder by name (decision 22)

`drafts::folder` (today `#[doc(hidden)]`, `src/drafts/mod.rs:52`) becomes a
documented public API: `Folder::open_named(data: &Path, name: &str)`
(a single path component; anything else is refused) holds
`<data>/<name>` under `protect::Folder`'s verdicts and lock, and offers
`create_new`, `replace` (write to a new owner-only file, then rename),
`read`, `list` and `remove`. The drafts store's own folder stays
`protect::Folder::open(data)`; `Folder::open` keeps its meaning. No type
derives `Debug` over file contents. Lib unit tests cover a bad name, owner-only
modes on Unix, `replace` atomicity and `list` order. The desktop's
`D/services/resolutions.rs` opens `resolutions` beside `drafts` (B3).

## Part A: Close Ticket… And Approve And Merge…

### A1: Seams

- `Intent::DocumentPromote{root, id}`, `Intent::TicketClose{root, id}`,
  with `JobKind`s, build arms and labels "Approve and merge" and "Close
  ticket". Exhaustive matches take C5's new variants.
- `LATER` empties; `header_later` and `later_action` go (E-4). A
  lifecycle-closed ticket offers no Close ticket….
- `resume_offer` returns `NotOffered` for lifecycle rows until Part C, not
  `SYNCHRONIZATION_UNAVAILABLE`.
- Test seams (`D/test_support/`): a canned prepare answer and a canned
  `ConflictDto` read beside `hold_job_return_answering` (decision 9).

### A2: The lifecycle dialog

A new view, `D/view/lifecycle.rs` with `lifecycle/` submodules, opened
from the header. It calls `Services::prepare`; the first run is
`Worker::submit(intent, Lane::User, Some(confirmation), secret)`; after
Prepare again it is `Worker::reconfirm`; Retry, Continue and the
resend after Resume are `Worker::submit_with_request_id`, which passes no
confirmation (an accepted record needs none) and no secret, so a
protected key on a resend takes D4's unlock-then-Retry path. It uses
`offers_prepare_again` and does not reuse `ConfirmDialog` (R-1). Header: kind icon, title "Close ticket" or
"Approve and merge", subtitle "{slug or short ULID} · {title} · {repository}",
and the context branch (MH-COLLAB-006).

| Stage | Shows | Keys |
| --- | --- | --- |
| Saving | "Saving your latest edits…" (decision 13) | Esc cancels before prepare |
| Preparing | Spinner | Esc cancels |
| Confirm | Lead; step list; changed paths; code note; Git line; expiry | Enter confirms; Esc cancels |
| Blocked | Each blocker in words, with its way out | Esc closes |
| Running | Step states from L3; Stop at a safe point | Esc hides; work continues |
| Failed | Failed step and cause; what did and did not happen; Retry; way out; credential recovery | Enter retries; Esc closes |
| Done | Outcome lines; Now ready to start (closure) | Enter closes |

Focus returns to the header action on close.

**Steps** (handoff 2c, closure order per GRFC 305, R-8):

1. Save your latest edits.
2. Share with origin, or "Keep on this computer (sharing pending)" when
   `publication` is `pending_no_remote`.
3. Mark as closed (closure only): "Closed by {closed_by}", or "Already
   marked closed" when `recording` is `existing`.
4. Merge into main, or "Already in main" when `integration` is
   `already_contained`.
5. Share main with origin (omitted when local-only).
6. Tidy up: the worktree, the local branch and, when present, the remote
   branch, from `cleanup`.

**Changed paths.** One row per `changes[]` entry: change icon, mono path,
and a tag from `item_path_kind` (ticket, document, comment) or `code`. Any
`code` row adds "This working copy includes code as well as the {item}.
All of it is merged." No line counts (R-3).

**Save first (decision 13).** With unsaved edits, the dialog calls
`flush_now` then `save` and waits. A saved answer moves on to prepare; any
other answer closes the dialog, and the tab shows that save's outcome.

**Read-only while unfinished (decisions 12, 18, 20, 21).** Every tab and
window on the item is read-only, with "Closing…" or "Merging…", while
any **lock source** holds. Stated as required outcomes after three
review rounds; Task 9 is test-first with a design checkpoint.

Lock sources:

- **Dialog.** A lifecycle dialog for the item is open from Saving to
  Confirm; released when the dialog is dropped, by Cancel, Esc or its
  window closing.
- **Worker.** The worker has a `TicketClose` or `DocumentPromote` for the
  item, or an `OperationResume` on its lifecycle operation, queued or in
  flight.
- **Record.** L2's row for the item is in a **locking step** (below). Rows
  are read at launch, on the operation area's existing refresh, on window
  focus, and after each lifecycle answer. Rows that cannot be read (the
  repository removed or unreadable) give no record lock.

| L | Recorded step (C5 `closure/record.rs:31-35`; promotion's equivalents) | Lock |
| --- | --- | --- |
| L-1 | `confirmed`, `context_synchronized` | None from the record: a stop here unlocks (decision 18) |
| L-2 | `closing` (closure checkpoint written, not supersedable, `closure/record.rs:437-440`) | Locked (decision 20) |
| L-3 | `integrating`, `integrated`, `published`, `publication_pending`, `remote_deleted`, `branch_deleted` | Locked |
| L-4 | A cleanup pending for `context_changed`, `ignored_files` or `remote_context_changed`, known from the latest answer this session or from a Retry after restart | Unlocked; the item shows "Merged, but not tidied up: {reason}" with Open folder and Check again, which re-observes and then Retries (decision 21, K-13) |
| L-5 | `complete`, `abandoned`, `superseded`, or no row | None |
| L-6 | Before acceptance: refused or stale prepare or consent, `busy`, `request_mismatch`, Cancel | None once the dialog closes |

**Stale (contract 3).** When the library refuses the consent (an answer in
`STALE_CODES`, `external_change` or `confirmation_expired`), the dialog says
"Something changed since you confirmed" and offers Prepare again, which
re-renders the summary.

**Blocked.** Each blocker in words with its L1 way out. A dirty primary is
`primary_dirty` at prepare, or the same blocker in a stop at execute; both
show "Main has unsaved changes outside Manyhands" and Open folder.

### A3: Outcomes

| Field | Done | Failed |
| --- | --- | --- |
| integration | "Merged into main" / "Already in main" | — |
| publication | "Shared" / "Already shared" / "Sharing pending (no remote)" | "Not shared: {reason}" with Retry |
| cleanup | "Tidied up" | "Not tidied up yet: {reason}" with Retry |

A local-only run whose cleanup completes is **Done** with "Sharing
pending"; Share main is offered once a remote exists (R-7). A remote
publication or cleanup that is pending is **Failed** with the completed
steps checked; Retry sends the same request ID (Fact 6). The way-out
buttons follow L1: Repair locally…, Resolve conflict…, Republish…, Resume,
Continue (`RetryRequest`: resends the holding request), Share main, Open
repository details, or guidance text for `PollOnce`, `SaveItem` and
`RepairOutside`.

**Deleted branch** (`CancelThenSendAgain`, the CLI's sequence,
`human_lifecycle.rs:162-196`): one button, "Stop and republish…", which
(1) cancels and resends the lifecycle request through `Worker::give_up`
(`worker.rs:384`); it goes on only on `GaveUp::Resent` with a `cancelled`
answer, which releases the child's reservation and shows "Stopped; ready
to republish"; any other result (`give_up` false, `AlreadyFinished`, or
another answer) stops the sequence and shows that answer; (2) opens
Republish… (confirmed); (3) on its success, resends the lifecycle request
(Retry). `Intent::stops_at_safe_points` (`requests.rs:436-441`) gains
`TicketClose`, `DocumentPromote` and `ItemRepublish` (P21).

**Already running.** Resume and Continue are not offered while the worker
has the item's lifecycle in flight, matching `TicketClose`,
`DocumentPromote`, and `OperationResume` on that lifecycle's operation ID
(`Worker::in_flight`, `worker.rs:535`); a resend answered `busy` reads "Still
running" with no failure (P22). Credential stops reuse D4's recovery: Unlock…, Open Sharing &
keys, Check host. A stop at a safe point reports which steps completed and
whether cancellation finished (MH-COLLAB-006).

Closed by on Done comes from the preview kept in the dialog, or from a
ticket read after Resume (Fact 5 has none).

**Made ready (decision 6).** On Done for a closure: `made_ready` by slug
(short ULID when null), with titles from the ticket read. `index_pending`
adds "The list may be incomplete until indexing finishes."; `not_computed`
and `index_unavailable_final` say "Could not work out which tickets are now
ready." with Open ready tickets.

**Operation area.** The entry names the command and item, shows the latest
stage, and on a Failed or stopped answer offers Open…, which reopens the
dialog on that request.

### A4: After a lifecycle (decision 14)

On cleanup `complete`, tabs on the item reopen it from primary, and
`created_contexts` forgets the context. The existing closure display
(Fact 26) shows Closed, Closed at and Closed by apart from Status;
a characterization test pins it. Unsent comment drafts stay on the Drafts
page. A failed or partial run keeps the context and every draft.

## Part B: Resolve Conflict Sheet

### B1: Reads

- Remote: `show_conflict(operation_id)`; observation from `ConflictDto`.
  For a lifecycle child after a restart, the operation ID is the item's
  L2 lifecycle row's, which the conflict verbs accept (C5 decision 34).
- Local: `inspect_local_context_sync(operation_id)` for state, eligibility
  and observation, then `read_local_context_conflict` per token.
- Reads run on the background executor, stamped; a late read never
  replaces a newer one.

### B2: Model (`D/model/conflict.rs`)

- **External set** (whole set goes outside, GRFC 468–474):
  - remote: any path `noncanonical`, `structure` other than `regular`,
    `content_available` false, or `eligibility`
    `external_resolution_required`;
  - local: `eligibility` `external_resolution_required`, `reason`
    `unsupported_conflict` or `immutable_closure_disagreement`, any
    `content_available` false, or a path `item_path_kind` rejects (U-2);
  - any side that `parse_item` rejects.
- **Canonical path:** parse base, mine (local) and theirs (incoming). A
  field changed by both, differently, is a part with Keep mine / Keep
  theirs; one changed by one side is taken from it (R-5). The body is one
  part with Keep mine / Keep theirs / Keep both (R-4) and an editable
  Result.
- **Result source** (R-9): every part "mine" submits mine's bytes
  verbatim; otherwise `serialize_item` of mine with the chosen fields and
  body result, and the card notes "This file will be saved in the
  standard layout."

### B3: Sheet view (`D/view/conflict.rs`, `conflict/`)

Handoff 2g: title "Choose what to keep · {title}", progress "{n} of {m}
chosen", one card per part with Before / Yours / Theirs (read-only
editors for the body, mono text for fields), the Result editor, and Cancel
· Save resolution, enabled only when every part is chosen. Enter in the
Result editor is a newline; Ctrl+Enter saves. The note follows the source
(decision 15): "Saving finishes the share that stopped." / "Saving
finishes bringing main into this item." (local) / "Saving lets the close
continue." / "Saving lets the merge continue."

Save resolution submits `ConflictResolve` or `LocalContextResolve` (new
intents `Intent::ConflictResolve`, `Intent::LocalContextResolve`, with
`JobKind`s and build arms) with B1's observation and every path's
`{token, source}`.

**Kept sets (decision 19, R-12).** Before submission, the set is written
per request ID to `D/services/resolutions.rs`'s store in the desktop's data
directory, in `<data_dir>/resolutions` through L4 (decision 22), with the
drafts' protection and no `Debug`. Each set records its request ID, root, operation ID, item and
remote or local kind. It is deleted when its request finishes (applied,
refused or abandoned), and a **launch sweep** calls `show_request` for every
kept set and deletes finished and not-found ones. Reopening the sheet for
an operation with a kept, unfinished set offers Retry with **that** set,
never a new request; if the user had made different choices (another
window), the sheet says "An earlier resolution is still being saved; it
is used instead." U-5 is settled in part: the D2 store is `<data_dir>/drafts`,
and its protection reaches the desktop only through L4.

| Answer | Shows |
| --- | --- |
| applied, share or local | Banner clears; the share's or sync's outcome follows |
| applied, lifecycle child | The desktop resends the lifecycle's request ID when it holds it; otherwise Resume on the item's L2 lifecycle row, then the Part C resend (R-10, U-3); the dialog shows Running |
| `external_change` / stale observation | "This changed while you were choosing." Refreshed comparison; choices kept where the part is unchanged |
| `original_request_required{request_id}` | Retry with the kept set of that request ID, after a restart too. A remote request with written paths only and no kept set: a new resolution that supersedes it (GRFC 402-403) |
| stopped | Resume on the row (P15) |

### B4: External guidance

An external set shows one card per path: "This file needs fixing outside
Manyhands", with the reason (code, binary, renamed or deleted, or "Both
copies record a different closure" for incompatible closures), Open folder,
and Check again, which re-reads and offers Resume only when the library's
`next_action` says so. No Save resolution; no winning closure.

### B5: Entry points (decision 16)

Review changes on the conflict banner; on the operation area's entry for a
conflicted share, post, local sync or lifecycle (L1
`ResolveChildConflict`); and on pending rows whose `next_action` is
resolve, after a restart.

### B6: Local synchronization

Repair locally… (L1 `RepairLocally`, new `Intent::LocalContextSync`):
observe, then `LocalContextSync` with that observation. A conflict opens
the sheet. A clean result offers the lifecycle's way on from L1: Retry
when a request holds the lifecycle, Resume when none does, Prepare again
when the earlier stop was a blocked prepare. A refusal or stop shows its
answer, and Resume on the row continues it (P16). `resume_offer` offers Resume for local-context
synchronizations and resolutions (E-5), keeping lifecycle rows apart.

## Part C: Republish And Resume

- `Intent::ItemRepublish{root, kind, id}`, with `JobKind` and build arm. The share outcome for
  `remote_branch_deleted` and L1's `Republish` offer Republish…, through
  `confirm_followed` with the existing Republish arm. `BRANCH_DELETED`
  loses "arrives in a later release" (E-6). A stopped republish is sent
  again with the same request ID (P17).
- **Resume (decision 10).** Resume on a lifecycle row sends
  `OperationResume`. On `original_request_required{request_id}` the desktop
  sends a rebuilt `DocumentPromote` or `TicketClose` intent with that
  request ID (`submit_with_request_id`) and opens the dialog in Running.
  Resume is not offered while the worker has the item's lifecycle in
  flight (P22).

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | Save fails before a lifecycle | No prepare; the save's own outcome shows |
| P2 | The library refuses consent after a new path | Prepare again; new summary; no run |
| P3 | Consent expires before Confirm | `confirmation_expired`; Prepare again |
| P4 | Stop at a safe point mid-run | Stopped; completed steps named; Retry sends the same request |
| P5 | Process exits mid-run | After relaunch, Resume → `original_request_required` → the same request continues |
| P6 | Remote branch deletion refused | Merged and shared; not tidied; Retry tidies only |
| P7 | Retry after a partial result | No new checkpoint, merge or publication commit |
| P8 | Child conflict during a lifecycle | Review changes; after resolving, the same request finishes or answers `external_change` |
| P9 | Refused closure checkpoint merge | Repair locally…; then Retry |
| P10 | Incompatible closures | External; no winner |
| P11 | A side changes while the sheet is open | Refreshed comparison |
| P12 | Held item at prepare | Continue (resends the holding request), or Resume |
| P13 | Close a ticket whose status text is `closed` | Runs; lifecycle Closed shown apart from status |
| P14 | Already-closed ticket | `ticket_closed`; no run |
| P15 | Remote resolution interrupted | Resume on its row finishes the prepared attempt |
| P16 | Local sync refused or stopped | Its answer; Resume on the row |
| P17 | Republish stopped | Same request again; one publication |
| P18 | Lifecycle stopped with no parent record | New prepare offered |
| P19 | Dirty primary | Blocked at prepare, or stopped at execute; Open folder; nothing merged |
| P20 | Edit attempted while a lifecycle is unfinished | Read-only in every tab and window of the item; after a restart too |
| P21 | Shared branch deleted mid-lifecycle | Stop and republish…: cancel and resend (`cancelled`, reservation released), Republish, resend; no loop on `busy` |
| P22 | Resume or Continue while the item's lifecycle is in flight | Not offered; a resend answered `busy` reads "Still running" |
| P23 | Cleanup refused after integration, then the user tries to edit | Still read-only; Retry tidies; the context is never changed meanwhile |
| P24 | Restart after a partial local resolution | Retry with that request's kept set finishes it |
| P25 | Unlock paths | L-1, L-4, L-5, L-6: tabs unlock; a stop before `closing` or `integrating` unlocks |
| P26 | The CLI finishes or starts the item's lifecycle while the desktop is open | The next row refresh or window focus unlocks or locks |

## Native Checklist (Linux Wayland, Release Build, Product Owner)

Written in the final task, in D4's format, against a real SSH remote: §1
Smoke; §2 Close and promote (remote, local-only, code on the branch); §3
Conflict (share, post, local repair, lifecycle child); §4 Failures (key
refused, remote deletion refused, stop, quit and relaunch with Resume); §5
Republish; §6 Keyboard only; §7 Results.

## Privacy

No title, body, comment, side or result text enters an operation entry,
log or error. Paths appear only in the dialog and the sheet. Sentinel tests
seed each.

## Testing (Decision 9)

Desktop tests use real local repositories for local-only runs and local
conflicts. **Interruption is real:**

- P4: a new test seam in the worker's progress sink holds a local-only
  close at `integrating`. L3 emits `integrating` before the cancel check
  that precedes the merge (`run.rs:715-716`) and outside any transaction,
  so Stop lands before the merge.
- P5: following `view/item/posting_tests/relaunch.rs`, a close is broken
  by the library failure point `BeforeRequestSettlement`
  (`execute.rs:425`, through `open_at_with_failure_point_for_testing`, as
  `move_document_tests.rs:283` does); the test asserts `request_show` is
  `accepted`, then opens a fresh shell over the same data directory and
  drives Resume → `original_request_required` → resend.
- Commits are counted on the real repositories (A8, A9).
**Canned answers** (`hold_job_return_answering`, a canned prepare, a
canned `ConflictDto`) are used only where the real call is refused with no
effect, or to render a remote outcome the desktop cannot reach; each
such test says so in its doc comment and asserts nothing about
repository state. Remote behaviour is proven by C5's and Part 0's library
and CLI SSH tests, and by the native checklist against a real remote.

| Target | What |
| --- | --- |
| `R/mutation/way_out_tests.rs` (lib) | L1 variants |
| `tests/read_status.rs` | L2 rows: unfinished, held, refused |
| `tests/mutation_remote/d5_progress.rs` | L3 order; cleanup-only; local-only |
| `D/view/lifecycle_tests.rs`, `lifecycle/*_tests.rs` | A2–A4; P1–P4, P6, P7, P12–P14, P18, P19 |
| `D/model/conflict_tests.rs` | B2 |
| `D/view/conflict_tests.rs`, `conflict/*_tests.rs` | B3–B6; P8–P11, P15, P16 |
| `D/view/operations/resume_tests.rs` | P5, P22 |
| `D/view/operations/republish_tests.rs` | P17, P21 |
| `D/services/resolutions_tests.rs` | Kept sets: write, read after reopen, delete on finish; P24 |
| `D/view/lifecycle/interruption_tests.rs` | P4, P5, A8, A9 (real, local-only) |
| `D/view/lifecycle/lock_tests.rs` | L-1–L-6; P20, P23, P25, P26 |
| `D/view/keyboard_tests/lifecycle.rs`, `conflict.rs` | A1, A2 |

## Named Edits To Pre-Existing Tests

| E | Test | Edit | Owner |
| --- | --- | --- | --- |
| E-1 | `tests/read_status.rs` rows for unfinished promotion/closure | `next_action` null → `resume` | L2 |
| E-2 | CLI goldens with a lifecycle row (`tests/cli*`) | Same | L2 |
| E-3 | Lifecycle progress pins: `tests/cli_remote/journeys_c5.rs:793` (keeps `connecting` from the forwarded child if U-4 holds; otherwise asserts `progress: observing`); the five fast-tier goldens `tests/fixtures/cli_v1/ticket_close_local_only_text.txt:21`, `ticket_close_made_ready_text.txt:21`, `ticket_close_checkpoint_merge_refused_text.txt:2`, `document_promote_local_only_text.txt:19`, `document_promote_already_contained_text.txt:19` (asserted by `tests/cli/lifecycle.rs`); `command.rs`'s stage table test (`:767-773`), splitting `ItemRepublish` from the lifecycles | New stages; `observing` pre-call | L3 |
| E-4 | `D/view/item/header_tests.rs`, `D/model/item_tests.rs`, `D/view/item_tests.rs` | Later notes → live actions. `LATER_NOTE` stays: `D/view/close_draft.rs:44,421,424` uses it | A1 |
| E-5 | `D/model/operations_tests.rs:465,534` | Unavailable → Offered | B6 |
| E-6 | `D/view/operations/outcomes_tests.rs:431-444`, `give_up_tests.rs:186,242`, `D/model/publication_tests.rs:597,764-772` | `BRANCH_DELETED` text; Republish offer | C |
| E-7 | `D/view/item/share_tests.rs` conflict-banner test (from `:690`, the text at `:703` and its "no control" assertion) | `CONFLICT_BANNER` drops "Resolving it in Manyhands arrives in a later release"; the banner gains Review changes | B5 |
| E-8 | `D/view/item/posting_tests/decisions.rs:143-149` (asserts `CONFLICT_BANNER` and that the banner offers no action) | The banner's text as E-7; it offers Review changes | B5 |
| E-9 | `R/promotion/record_tests.rs:696` `promotion_status_reports_parent_step_once_and_never_offers_boundary_resume` (owner 2026-11-04, Task 2) | `next_action` None → `Resume`; renamed for the new rule | L2 |
| E-10 | `D/services/requests_tests.rs`: `every_intent`, `variant`, `every_intent_builds_its_own_command`, `changes_items_names_the_intents_that_can_change_items`, `syncs_comments_and_resume_stop_at_safe_points_and_saves_do_not` (owner 2026-11-04, Task 5; mirrors D4's E-12) | Add `DocumentPromote` and `TicketClose` to the intent tables (32 → 34), their command names, `changes_items` range to `17..=33`, safe-point set to `17 \| 29..=33` | A1 |
| E-11 | `D/view/keyboard_tests/frame.rs` `the_strip_header_chips_and_page_run_in_tab_order`, `D/view/keyboard_tests/discussion.rs` `tab_order_runs_header_share_chips_page_discussion_composer` (owner 2026-11-04, Task 5) | The header's live lifecycle action `item-lifecycle-action` joins Tab order after `item-share`; the discussion array goes from 10 to 11 | A1 |
| E-12 | `D/view/keyboard_tests/frame.rs` `the_strip_header_chips_and_page_run_in_tab_order`, the document walk (standing test-expansion approval, 2026-11-04) | `item-lifecycle-action` (Approve and merge…) between `item-share` and `chip-status`, forward and back | A1 |
| E-13 | `D/view/keyboard_tests/discussion.rs` `tab_order_runs_header_share_chips_page_discussion_composer`, the editing walk (standing test-expansion approval) | `item-lifecycle-action` after Share, before `chip-type`; the action is still drawn while editing | A1 |
| E-14 | `D/view/lifecycle_tests.rs` `a_blocked_prepare_lists_blockers_and_way_out` (Task 6a's own test; standing test-expansion approval, Task 7) | Also asserts the Open repository details button (`lifecycle-inspect`) beside the guidance row | A3 |

Task 0's sweep (2026-11-04) confirmed the list against merged C5 and added
E-8; E-2 found no CLI golden with a lifecycle `next_action` row, so it may
be empty. `D/model/publication_tests.rs:265,477` (a post's deleted branch
offers Give up only) stay unchanged (decision 24, K-15).

## Document Amendments

Applied in the approval commit:

1. Wave D5 section: decisions 2, 5, 8–12; the library-part exception to
   track rules 5 and 6.
2. Desktop RFC: "Mark resolved" → "Save resolution", matching the
   handoff; per-part choices are deliberate per-file choices.
3. `schemas/v1/progress_event.schema.json` stage descriptions (with L3);
   CLI RFC note that lifecycle rows list `resume` (decision 10).
4. API audit: the rows above.
5. Ticket `deps` gains C5 `01M4CC0VMXTNXNH78C11DYXS8H` (by `ticket save`).
6. Wave entry-gate item 3: D5's audit drafted against unmerged C5 and
   re-checked on merged C5 on 2026-11-04 (decision 17), as C4 and C5
   recorded theirs.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | The lifecycle dialog is its own view over the services, not a `ConfirmDialog` arm | A second dialog shape to maintain |
| R-2 | Tags come from `item_path_kind`, else `code` | A misfiled tag; content unaffected |
| R-3 | No line counts: the preview has none | Less detail than the handoff |
| R-4 | Keep both = mine, blank line, theirs; body only | A user edits the result by hand |
| R-5 | Single-side changes are taken without a choice | Still visible in the columns |
| R-6 | Any external path makes the set external | Canonical paths wait for outside repair |
| R-7 | Local-only with cleanup complete is Done; remote pending is Failed | Wording only |
| R-8 | Closure steps: Mark as closed before Merge (GRFC 305), against the handoff's order | None: the library's order |
| R-9 | All-mine submits mine's bytes; otherwise normalized | A whole-file layout change on a chosen merge |
| R-10 | After a lifecycle child's resolution, the desktop sends the lifecycle request again | One extra request send |
| R-11 | A local resolution's observation is the inspection's | The library refuses a stale one |
| R-12 | A resolution's submitted set is kept per request ID until that request finishes (decision 19) | A stale file after a crash; deleted when the request is next seen finished |
| R-13 | `RepairOutside` is any `recovery_required` result with `integration_conflict` that no other rule routes, a ticket's included (Task 1 review, 2026-11-04) | A closure the CLI can repair locally shown outside guidance; the CLI prints nothing for it either way |
| R-14 | `integrating` is emitted only when a merge runs (integration kind `merged`); an already-contained item emits none (Task 3 checkpoint) | "Merge into main" lights without its stage on an already-contained run; the step list still reads the result |
| R-15 | `recording` is not emitted for a closure whose accepted preview says the closure already exists; the mapping layer in `bind/lifecycle.rs` filters it (Task 3 checkpoint) | "Mark as closed" briefly shows running for a closure that was already recorded |
| R-16 | "Cleanup-only replay" (P/L3) is a local-only, already-contained, already-closed first run: exactly `accepted, observing, cleaning_up`. A remote one adds `publishing`, because the primary child runs (already current); a same-ID resend never reports `accepted` (Task 3 checkpoint) | The name over-promises for remote runs; the tests pin each order |
| R-17 | Save first (decision 13) covers a stored or restored draft of the item that has no live session in the tab: it is saved through `flush_now` then `save`, the same as a live session. Unsent comment drafts don't block (decision 14) (Task 6a) | A draft the person has not looked at in this session is saved without being shown again |
| R-18 | A tab with no item ID (a new item not yet created, or a nonconforming Inspect tab) keeps the lifecycle action visible but disabled, with its reason in `item-lifecycle-note`; pressing it opens nothing (Task 5 review Minor 1, Task 6a) | The action shows where it can't run |
| R-19 | The dialog's short ID for an item without a slug is the last 8 characters of its ULID (`D/view/lifecycle.rs` `short_id`); Done (Task 8) reuses it (Task 6a review) | Two items may share a short ID; the full ID is in the preview |

## Known Limits

- **K-1.** `01M4H33R34Z7C7EEKTY1ZCT950`: a pending remote conflict cannot be
  abandoned.
- **K-2.** A retained conflict under a raised generation stays stuck (C5).
- **K-3.** A child holding a resolution whose branch is then deleted loops
  (C5).
- **K-4.** The classifier reads an edited text code file as `binary` and
  add/add as `delete`; both go outside (GRFC 468–474).
- **K-5.** `01M4R19PC0T6ZPKM39X9DF26HX`, `01M6GHWZS03WCADS1A1JXV4PMP`: a
  stopped share may hold the repository's reservation (D4 K-5, K-16).
- **K-6.** A resumed lifecycle that released its child and one that kept it
  answer alike.
- **K-7.** No line counts in the summary (R-3).
- **K-8.** A made-ready ticket deleted before its title is read shows by
  slug only.
- **K-9.** A context made by posting is not released by the desktop (D4
  K-9); a later close tidies it.
- **K-10.** A partial local resolution whose kept set was lost (data
  directory removed) cannot finish; abandoning the local synchronization
  is the way out (GRFC local rule).
- **K-11.** Withdrawn: U-4 held in part (Task 0). A resend that adopts an
  already-published context child shows no stages for Share with origin.
- **K-12.** `PollOnce` is guidance text until D6.
- **K-13.** A merged item whose tidy-up is refused because its working
  copy, ignored files or remote branch changed (decision 21) is unlocked
  and stays untidied until that change is undone or cleaned outside the
  app; Check again then Retries.
- **K-14.** A local conflict on a document whose path has non-ASCII bytes
  or a control character goes to the external guidance: its escaped
  `display_path` does not classify (U-2, decision 23).
- **K-15.** A comment post whose item's branch was deleted offers Give up
  only, as before D5; Republish covers shares and lifecycles (decision 24).
- **K-16.** R-15 holds only where the binding observes the item in the
  same call: a same-ID resend that re-enters through an existing parent
  record still reports `recording` for a closure that already exists and has
  not yet written its checkpoint (Task 3).
- **K-17.** A lifecycle's progress events carry no effects; its result
  reports them (Task 3 review).
- **K-18.** `Folder::open_named` deletes stale temporaries; a `replace` that
  another process runs on the same folder at that moment fails and is
  retried (Task 3b review).

## Alternatives Rejected

- A separate library ticket for L1–L3 (decision 2).
- Recovery actions in envelopes for the way out (decision 3).
- A per-path mine/theirs/edit sheet (decision 4).
- A provisional made-ready list at confirm (decision 6).
- Parallel lanes (decision 8).
- An SSH fixture for desktop tests (decision 9).
- The holder's request ID on the operation row (decision 10).
- Inferring publication from stage order (decision 11).
- Editable tabs with edits held as drafts (decision 12).
- Unlocking the tabs on any failure, even after integration (decision 18).
- Keeping resolution sets in memory only (decision 19).

## Risks

| Risk | Control |
| --- | --- |
| C5 changes after this draft | Task 0 re-checks Facts 2–9 and stops on a difference |
| L3 touches the step runner | Test-first with a design checkpoint; `document_promotion`, `ticket_closure`, `mutation_remote` run |
| Canned answers drift from the library | Each canned answer is built from a library type and named after the library test that produces it |
| Save-first races an external change | The save's own answer stops the dialog (P1) |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Round 1, source S1 | New stages break `stage_text`, the schema test; wrong runner files | Settled | Part 0 files; L3 rewritten | Facts 9 |
| Round 1, source S2; contract C1 | Resume after a crash hidden by L2 | Owner decision | Decision 10; Part C | Fact 8 |
| Round 1, source S3; contract C4 | No SSH remote for desktop tests | Owner decision | Decision 9; Testing | Fact 27 |
| Round 1, source S4 | L1 lacks CLI routes | Settled | Variants added | Fact 7 |
| Round 1, source S5; contract C21 | Renderer tests not run; tests in SSH tier | Settled | Lib unit tests; `--bin manyhands-cli` | Fact 7 |
| Round 1, source S6 | L2 visibility; refused parents | Settled | Share `lifecycle_standing` | Fact 8 |
| Round 1, source S7 | Round trip is not byte-exact | Ruling R-9 | B2 | Fact 14 |
| Round 1, source S8; contract C7 | Lifecycle child resolution needs a resend | Ruling R-10 | B3 | Fact 11 |
| Round 1, source S9; contract C9 | Local external rule; local observation | Settled, R-11 | B1, B2 | Fact 12 |
| Round 1, source S10 | E-6 incomplete | Settled | E-6 widened; sweep | Fact 18 |
| Round 1, source S11 | Closure display already exists | Settled | Characterization | Fact 26 |
| Round 1, source S12 | Private `ConfirmDialog::prepare` | Settled | Real seams named | Facts 20–21 |
| Round 1, source S13; contract C16 | Interruption mechanism unstated | Owner decision 9 | P5 via canned answers; library and native | Fact 27 |
| Round 1, source S14–S20 | Minor | Settled | Applied | — |
| Round 1, contract C2 | Closure step order | Ruling R-8 | Steps | GRFC 305 |
| Round 1, contract C2; question 2 | Primary push indistinguishable | Owner decision | Decision 11: `publishing` | — |
| Round 1, contract C3 | Local-only read as Failed | Ruling R-7 | A3 | MH-COLLAB-005/007 |
| Round 1, contract C6; question 3 | Edits while running | Owner decision | Decision 12 | — |
| Round 1, contract C8 | Partial local R after restart | Ruling R-12, K-10 | B3 | GRFC local amendment |
| Round 1, contract C10 | Sheet note wrong for local and lifecycle | Proposed decision 15 | B3 | — |
| Round 1, contract C11 | Dirty primary path | Settled | A2 Blocked; P19 | Fact 4 |
| Round 1, contract C12 | Track rule 7 gaps | Settled | P15–P18 | Wave 367 |
| Round 1, contract C13 | Credential recovery in the dialog | Settled | A3 | Handoff 05:100 |
| Round 1, contract C14 | Closure method | Settled | Plan Task 16 | AGENTS.md |
| Round 1, contract C15 | Entry-gate item 3; rule 6 | Proposed decision 17 | Amendments 1, 6 | Wave 133–144 |
| Round 1, contract C16 | Tasks too large | Settled | Plan split | — |
| Round 1, contract C17–C24 | Minor | Settled | Applied | — |
| Round 2, scoped (`0d7a851c`) | 1 Blocking, 11 Important, 8 Minor | — | Revision 3 | Below |
| Round 2, R2-1 | Lifecycle pre-call stage is `connecting` | Settled | `command.rs` joins Part 0; `observing` pre-call | `command.rs:480-490` |
| Round 2, R2-2 | Primary child's stages light the wrong step | Settled | Forward by `ChildRole` | `run.rs:790,944` |
| Round 2, R2-3 | L1 prepare routing needs `kind`; document repair note | Settled | `kind` parameter; `RepairOutside` | `human_lifecycle.rs:330-337,391` |
| Round 2, R2-4 | Deleted-branch sequence undefined | Settled | Stop and republish…; P21 | `human_lifecycle.rs:156-190` |
| Round 2, R2-5 | Lock unlocks after integration; lost on restart | Owner decision | Decision 18; P20, P23, P25 | `run.rs:1337-1364` |
| Round 2, R2-6 | Canned interruption tests vacuous | Settled | Real local-only interruption; canned answers restricted | `delaying.rs:111-141` |
| Round 2, R2-7, R2-8 | Kept set lost on restart; singular | Owner decision | Decision 19; R-12; P24 | GRFC 386-407 |
| Round 2, R2-9 | Child resolution after restart | Settled | Resume the L2 row, then resend | Fact 21 |
| Round 2, R2-10 | Resume offered while running | Settled | P22 | C5 `status.rs:290-300` |
| Round 2, R2-11 | No task adds conflict and local-sync intents | Settled | B3, B6, Part C; plan Tasks 10, 14 | `requests.rs:59-260` |
| Round 2, R2-12 | First run needs `submit` with consent | Settled | A2 | `worker.rs:353-360` |
| Round 2, R2-13–R2-20 | Minor | Settled | Applied | — |
| Round 3, scoped (`97f966f6`) | 1 Blocking, 6 Important, 5 Minor | — | Revision 4; not re-reviewed. The lock (third round in one mechanism) is stated as required outcomes L-1–L-6 with a design checkpoint in Task 9 | Below |
| Round 3, R3-1 | Lock never unlocks before the merge | Owner decision | Decision 20; L-1–L-3 | C5 `closure/record.rs:31-42,437-440` |
| Round 3, R3-2 | Lock stuck after supersedable cleanup refusal | Owner decision | Decision 21; L-4; K-13 | C5 `promotion/record.rs:69-74` |
| Round 3, R3-3 | Lock sources leave gaps | Settled | Dialog source; refresh points; unreadable rows; P26 | — |
| Round 3, R3-4 | `RepairOutside` misreads the CLI | Settled | Execute-only; CLI renders none | C5 `human_lifecycle.rs:280,323,329-343` |
| Round 3, R3-5 | `give_up` preconditions | Settled | Sequence conditions; `stops_at_safe_points` | `worker.rs:384,452-454`; `requests.rs:436-441` |
| Round 3, R3-6 | P5 does not relaunch | Settled | Fresh shell over the data directory; `BeforeRequestSettlement` | `posting_tests/relaunch.rs` |
| Round 3, R3-7 | Kept sets: sweep, key, two windows | Settled | Launch sweep; keyed fields; notice | — |
| Round 3, R3-8–R3-12 | Minor | Settled | Applied | — |
