---
title: "Wave 03 D5 Desktop Conflict Recovery, Promotion And Closure Design"
date: 2026-11-03
status: "draft"
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

The documents are drafted against C5's tip `f59aceb6`, which is not merged
(Cycle decision 1). Facts marked **(C5)** are read there; all others at
`e39bb004`. Task 0 re-checks each after C5 merges.

Conventions: "Fact n" refers to Baseline Facts; `D/` is `src/desktop/`; `R/`
is `src/repository/`; "the handoff" is `docs/design/05-sheets-and-dialogs.md`.

## Baseline Facts At `e39bb004` And C5 `f59aceb6`

**Library boundary (consumed).**

1. `RepositoryService::prepare(&Mutation) -> Envelope<PrepareDataDto>`
   (`R/mutation/confirm.rs`) and `execute(MutationCall, &mut
   SessionCredentials<P>, &mut dyn ProgressSink, &CancellationToken)`
   (`R/mutation/execute.rs`). Consent lives 600 s
   (`CONFIRMATION_LIFETIME_SECONDS`); an accepted request's consent stands
   however old.
2. **(C5)** `Mutation::DocumentPromote(DocumentPromoteInput{root, id})` and
   `Mutation::TicketClose(TicketCloseInput{root, id})`
   (`R/mutation/bind/lifecycle_verb.rs:37,44`). No identity or input;
   `closed_by` is the configured identity (C5 decision 6). Both confirm,
   stop at safe points and have an operation (`R/mutation/command.rs:153,366`).
3. **(C5)** `PreviewDetailsDto::Lifecycle(Box<LifecyclePreviewDto>)`
   (`R/mutation/dto.rs:243`); the DTO (`:310-330`) holds `root`, `kind`,
   `item_id`, `slug`, `branch`, `primary_branch`, `remote`, `context_oid`,
   `primary_oid`, `changes: Vec<LifecyclePathChangeDto{path, change}>`,
   `integration`, `publication`, `cleanup{remote_branch, worktree,
   local_branch}` and `closure: Option<{recording, closed_by}>`. No title,
   body, `closed_at` or made-ready list.
4. **(C5)** A blocked prepare is `PrepareDataDto::LifecycleBlocked(
   LifecycleBlockedDto{blockers})` (`dto.rs:365,686`); its code is
   `identity_required`, `ticket_closed` or `recovery_required`
   (`R/mutation/bind/lifecycle.rs:153`). A held item answers
   `recovery_required` naming the holding request or `operation.resume`
   (C5 decision 33).
5. **(C5)** `MutationDataDto::Lifecycle(Box<LifecycleMutationDto>)`
   (`dto.rs:726,831`): `recording`, `integration`, `publication`,
   `publication_pending`, `cleanup`, `cleanup_pending`, `blockers`,
   `made_ready: Vec<LifecycleMadeReadyDto{id, slug}>` (`:821`) and
   `made_ready_state` (`:809`: `current`, `index_pending`,
   `index_unavailable_final`, `not_computed`, `not_applicable`). No item
   slug.
6. **(C5)** Sending the same request ID again continues an unfinished
   parent, even after cancel (C5 decision 8), with no consent recheck.
   `cancel_request` reaches the lifecycle and its children.
7. **(C5)** The way-out rules live in the CLI binary:
   `src/bin/manyhands-cli/human_lifecycle.rs` `branch_deleted` (`:156`),
   `merge_refused_route` (`:192`), `child_conflict_route` (`:218`),
   `route_of` (`:259`), `way_out` (`:289`), `held` (`:349`).
8. **(C5)** `list_operations` lists unfinished promotion and closure
   parents with no resume (`R/read/status.rs:299,411`).
9. `ProgressStage` (`R/mutation/progress.rs:83-94`): `accepted`,
   `observing`, `writing`, `connecting`, `fetching`, `updating_local`,
   `pushing`, `verifying`, `indexing`, `stopping`. **(C5)** A lifecycle
   reports `accepted` then `connecting` only (`command.rs` `stage()`).
10. `show_conflict` (`R/remote/conflict_read.rs:190`) returns `ConflictDto`
    (`:132`): `operation_id`, `target`, `item_id`, `stage`, `next_action`,
    `eligibility`, `observation`, `historical`, and `paths:
    Vec<ConflictPathDto{display_path, token, kind, structure,
    content_available, base, local, incoming}>`. `kind` is `document`,
    `ticket`, `comment` or `noncanonical`; `structure` includes `binary`,
    `rename`, `delete`, `identity_changed` and `incompatible_closures`.
    **(C5)** It also accepts a lifecycle parent's ID (C5 decision 34).
11. `Mutation::ConflictResolve(ConflictResolveInput{root, operation_id,
    observation, resolutions: Vec<ConflictResolutionInput{token, source}>})`
    (`R/mutation/bind/conflict.rs:90,99`). `source` is the path's whole
    result text.
12. Local boundary: `observe_local_context_sync` (`R/local_sync/observe.rs:360`),
    `inspect_local_context_sync` and `read_local_context_conflict`
    (`R/read/local_sync.rs:110,117`; DTOs `:47,73`). Local entries carry
    `display_path`, `token` and `content_available`, with no kind.
    `Mutation::LocalContextSync` and `Mutation::LocalContextResolve`
    (`R/mutation/bind/local_sync.rs:10,20`).
13. `Mutation::ItemRepublish(ItemRepublishInput{root, kind, id})`
    (`R/mutation/bind/republish.rs:64`); `PreviewDetailsDto::Republish`.
14. `canonical::parse_item` and `canonical::serialize_item`
    (`src/canonical.rs:439,911`) are public.

**Desktop (changed).**

15. Header: `later_action` (`D/view/item/header.rs:104`) draws Close
    ticket… and Approve and merge… disabled with `LATER_NOTE`;
    `header_later` (`:131`); labels `D/model/item.rs:35-47`.
16. Conflict banner: `Banner::Conflict` (`D/view/item/banners.rs:101`),
    pushed with plain text `CONFLICT_BANNER` (`:221`;
    `D/model/operations/outcomes.rs:43`) when the share outcome or
    `post_conflict` (`D/view/item/share.rs:178`) says so. The operation ID
    is in `SyncOutcome::Conflict{next}` and `CommentOutcome::SavedConflict
    {next}` (`D/model/publication.rs:62`).
17. `BRANCH_DELETED` (`outcomes.rs:39`) says republishing arrives later.
18. `resume_offer` (`D/model/operations.rs:924`) shows a local-context
    synchronization's Resume unavailable (`SYNCHRONIZATION_UNAVAILABLE`,
    `:55`).
19. Confirmation: `confirm` and `confirm_followed` (`D/view/confirm.rs:161,174`),
    `prepare_again` (`:186`), `STALE_CODES` (`:123`), `detail_fields`
    (`:611`, Republish arm `:700`), `preview_body` (`:778`). **(C5)** adds a
    `Lifecycle` arm showing Branch, Primary and Remote.
20. Worker: `submit`, `submit_with_request_id`, `give_up`, `record_state`,
    `reconfirm`, `in_flight` (`D/services/worker.rs:333,353,384,392,424,535`).
    `Intent` (`D/services/requests.rs:60`) and `JobKind`
    (`D/services/mod.rs:99`) have no lifecycle, conflict, local-sync or
    republish variant.
21. Reads: `Services::read` and `read_job` (`D/services/reads.rs:117,149`).
22. Drafts: `flush_now` (`D/services/drafts/flush.rs:35`), `save`
    (`D/services/drafts/save.rs:82`).
23. Stale-edit review: `D/model/review.rs` (three values per field,
    choices, body result) and `D/view/item/edit/review_view.rs`
    (`result_view` `:251`).
24. `ReadOnly::new` (`D/view/editor/read_only.rs:65`) is a guarded
    read-only editor.
25. `EditTabs::unkept` (`D/services/edit_tabs.rs:57`) counts tabs editing a
    repository; repository removal closes only those counted at Confirm.

**Unverified, settled in Task 0 by reading.** Any answer that differs from
expectation stops for the owner.

- **U-1.** A successful `conflict resolve` continues the stopped share and
  may push (decision 13's words).
- **U-2.** `LocalContextSyncInspectionDto.paths` can be classified as
  canonical through `canonical::item_path_kind` (`src/canonical.rs:855`).
- **U-3.** A lifecycle stopped by a child conflict carries an
  `operation_id` the conflict verbs accept (C5 decision 34).
- **U-4.** `serialize_item` of a parsed side reproduces that side's bytes
  for an unchanged field set, so a "Keep mine" for every part submits
  mine unchanged.

## API Audit Refresh (Entry-Gate Item 3)

| Audit row | D5 consumes | State at C5 `f59aceb6` |
| --- | --- | --- |
| Promotion/close preflight and cleanup bindings | `DocumentPromote`, `TicketClose`, prepare, execute | Delivered by C5 (Facts 2–6). |
| Conflict inspection read, remote | `show_conflict`, `ConflictResolve` | On main (Facts 10–11). |
| Conflict inspection read, local | observe, inspect, read, `LocalContextSync`, `LocalContextResolve` | On main (Fact 12). |
| Republish | `ItemRepublish` | On main (Fact 13). |
| Lifecycle way out | none | Missing: L1. |
| Lifecycle resume listing | `list_operations` | Missing: L2. |
| Lifecycle progress | `ProgressSink` | Missing: L3. |
| Made-ready titles | ticket read | Existing read; no library change. |

## Execution Order (Decision 8)

Serial, on the ticket branch, in `.manyhands/worktrees/01M4CC0VN2K8DBHNMZ46NTX5JJ`.

```
Task 0 (merge main with C5; re-check) → Part 0: L1 → L2 → L3
  → Part A: seams → lifecycle dialog → outcomes → after-lifecycle
  → Part B: conflict reads → sheet model → sheet view → local sync → external
  → Part C: republish, resume
  → Part D: journeys, privacy, checklist, gates
```

## Part 0: Library (Decisions 2, 3, 9, 10)

Files: `R/mutation/way_out.rs` (new), `R/mutation/mod.rs` (export),
`R/read/status.rs`, `R/mutation/progress.rs`, `R/promotion/lifecycle.rs`,
`R/promotion/mod.rs`, `R/closure/mod.rs`, `R/mutation/bind/lifecycle.rs`,
`src/bin/manyhands-cli/human_lifecycle.rs`, and tests under
`tests/mutation_remote/`, `tests/read_status.rs` and `tests/cli*`.

### L1: Way out

```rust
pub enum LifecycleWayOut {
    ContinueSameRequest,             // retry or resume the same request ID
    RepairLocally { item_id: String },       // context sync-local
    ResolveChildConflict { operation_id: String },
    Republish { item_id: String },
    ResumeOperation { operation_id: String },
    WaitForRequest { request_id: String },   // a held item
    Inspect,
    None,
}
pub fn lifecycle_way_out(envelope: &Envelope<MutationDataDto>) -> Vec<LifecycleWayOut>;
pub fn lifecycle_prepare_way_out(envelope: &Envelope<PrepareDataDto>) -> Vec<LifecycleWayOut>;
```

The rules are moved, not rewritten, from Fact 7, keeping their order. The
CLI's `route_of`, `way_out` and `held` become renderers of the returned
values. Proof: every C5 CLI golden and `cli_remote` lifecycle test passes
unchanged; library tests cover each variant from a constructed envelope.

### L2: Resume for lifecycle parents

`promotion_operations` and `closure_operations` (`R/read/status.rs`) set
`next_action: Resume` for an unfinished parent no accepted request holds,
and `None` while one does (C5's `original_request_required`). The stale
"no boundary resume" comments go. Named edits: the `read_status` and CLI
`operation list`/`repo status` goldens that pin `null` for these rows
(E-1, E-2).

### L3: Lifecycle progress

New `ProgressStage` values: `recording` (closure metadata checkpoint),
`integrating` (merge into primary), `cleaning_up` (worktree and branch
removal). The domain runners take a `&mut dyn ProgressSink` beside the
probe and emit, in order: `writing` (final checkpoint), the child sync's
own stages, `recording`, `integrating`, `pushing`, `cleaning_up`. A stage
is emitted only when its step runs, so a cleanup-only replay reports
`cleaning_up` alone. The CLI RFC's stage list is amended (Document
Amendment 3). Named edits: tests that assert a lifecycle's progress is
`accepted, connecting` (E-3, found by Task 0's sweep).

## Part A: Close Ticket… And Approve And Merge…

### A1: Seams

- `Intent::DocumentPromote{root, id}`, `Intent::TicketClose{root, id}`,
  with `JobKind`s, build arms and labels "Approve and merge" and "Close
  ticket".
- `LATER` empties; `header_later` and `later_action` go. E-4 edits the
  header tests that assert the later notes.

### A2: The lifecycle dialog

A new view, `D/view/lifecycle.rs` (with `lifecycle/` submodules under 500
lines each), opened from the header. It wraps the existing confirmation
boundary (`ConfirmDialog::prepare`, `confirm`, `reconfirm`) rather than
`confirm.rs`'s generic layout, because its stages outlive the request's
submission.

| Stage | Shows | Keys |
| --- | --- | --- |
| Saving | "Saving your latest edits…" (decision 11) | Esc cancels before prepare |
| Preparing | Spinner | Esc cancels |
| Confirm | Lead line; step list; changed paths; code note; Git line; expiry | Enter confirms; Esc cancels |
| Blocked | Each blocker in words, with L1's way out as a button | Esc closes |
| Running | Step states from L3 stages; Stop at a safe point | Esc hides; work continues |
| Failed | Failed step and cause; what did and did not happen; Retry; way out | Enter retries; Esc closes |
| Done | Outcome line; Now ready to start (closure) | Enter closes |

**Steps** (handoff 2c, reworded per item kind):

1. Save your latest edits.
2. Share with origin, or "Keep on this computer (sharing pending)" when
   `publication` is `pending_no_remote`.
3. Merge into main, or "Already in main" when `integration` is
   `already_contained`.
4. Mark as closed (closure only): "Closed by {closed_by}", or "Already
   marked closed" when `recording` is `existing`.
5. Tidy up: the worktree, the local branch and, when present, the remote
   branch, from `cleanup`.

**Changed paths.** One row per `changes[]` entry: icon by change, the path
in mono, and a tag from `canonical::item_path_kind` (ticket, document,
comment) or `code`. Any `code` row adds "This working copy includes code
as well as the {item}. All of it is merged." No line counts: the preview
has none (R-3).

**Save first (decision 11).** If the item's tab holds unsaved edits, the
dialog calls `flush_now` then `save`, and waits for its outcome. A saved
answer moves on to prepare; any other answer closes the dialog and the tab
shows that save's own outcome. Nothing is prepared.

**Stale.** An answer in `STALE_CODES` or `external_change` after confirm
shows "Something changed since you confirmed" and Prepare again, which
re-renders the summary from a new preview (contract 3).

### A3: Outcomes

From `LifecycleMutationDto`, in this order:

| Field | Done | Partial |
| --- | --- | --- |
| integration | "Merged into main" / "Already in main" | — |
| publication | "Shared" / "Already shared" | "Not shared yet: {reason}" with Share main |
| cleanup | "Tidied up" | "Not tidied up yet: {reason}" with Retry |
| closure | "Closed by {closed_by}" | — |

A partial result keeps the dialog in Failed with the remaining steps
pending; Retry sends the same request ID (Fact 6). L1's way out adds its
button: Repair locally…, Resolve conflict…, Republish…, Resume, or Open
repository details. **No retry repeats a step** (contract 4): the library
guarantees it; the tests count commits (A9).

**Made ready (decision 6).** On Done for a closure, the `made_ready` list
by slug, with titles from the ticket read; `index_pending` adds "The list
may be incomplete until indexing finishes."; `not_computed` and
`index_unavailable_final` say "Could not work out which tickets are now
ready." and offer Open ready tickets.

**The operation area** carries the same request: its entry names the
command and item, shows the latest stage, and on a partial or failed
answer offers Open… which reopens the dialog on that request.

### A4: After a lifecycle (decision 12)

On cleanup `complete`, tabs on the item reopen it from primary, and
`created_contexts` forgets the context. A closed ticket's header shows a
Closed chip and its front-matter sheet shows Closed by and Closed on, from
the ticket read's lifecycle fields, apart from Status. Unsent comment
drafts stay on the Drafts page; Open on one reopens the item from primary.

## Part B: Resolve Conflict Sheet

### B1: Reads

- Remote: `show_conflict(operation_id)` on the background executor.
- Local: `inspect_local_context_sync(operation_id)`, then
  `read_local_context_conflict(operation_id, token)` per path.
- Each read is stamped; a late read never replaces a newer one.

### B2: Model (`D/model/conflict.rs`)

Per path, from base, mine (local) and theirs (incoming):

- **Canonical path** (`kind` other than `noncanonical`; for a local entry,
  U-2): parse all three with `canonical::parse_item`. Each front-matter
  field that differs between mine and theirs and was changed by both is a
  part with Keep mine / Keep {theirs}. Changed by one side only is taken
  from that side with no choice, as `D/model/review.rs` does. The body is
  one part with Keep mine / Keep theirs / Keep both, and an editable
  Result once chosen.
- **Result source** is `serialize_item` of mine with each chosen field and
  the body result (U-4).
- **External** if any path is `noncanonical`, has `structure` other than
  `regular`, fails to parse, or `eligibility` is
  `external_resolution_required`; then the whole set is external (GRFC
  468–474).

Keep both joins mine, a blank line and theirs (R-4).

### B3: Sheet view (`D/view/conflict.rs`, `conflict/`)

Handoff 2g: title "Choose what to keep · {title}", progress "{n} of {m}
chosen", one card per part with Before / Yours / Theirs read-only columns
(Fact 24 for the body; mono text for fields), the Result editor, and
Cancel · Save resolution, enabled only when every part is chosen. The
note reads "Saving finishes the share that stopped." (decision 13).

Save resolution submits `ConflictResolve` or `LocalContextResolve` with the
read's `observation` and every path's `{token, source}`. Answers:

| Answer | Shows |
| --- | --- |
| applied | Banner clears; the stopped share's or lifecycle's outcome follows |
| `external_change` / stale observation | "This changed while you were choosing." Refreshed comparison; choices kept where the part is unchanged (A3) |
| `original_request_required` | Retry with the named request |
| other | The operation area's recovery |

### B4: External guidance

An external set shows one card per path: "This file needs fixing outside
Manyhands", with the reason (code, binary, renamed or deleted, or "Both
copies record a different closure" for `incompatible_closures`), Open
folder, and Check again, which re-reads and offers Resume only when the
library's `next_action` says so. No Save resolution. No winning closure.

### B5: Entry points (decision 14)

The banner gains Review changes. The operation area's entry for a
conflicted share, post, local sync or lifecycle (L1
`ResolveChildConflict`) gains Review changes. Pending rows whose
`next_action` is resolve gain it after a restart.

### B6: Local synchronization

Repair locally… (L1 `RepairLocally`): observe, then `LocalContextSync`
with that observation; a conflict opens the sheet; a clean result offers
Retry on the lifecycle. `resume_offer` offers Resume for local-context
synchronizations and resolutions (E-5 edits the tests pinning
`SYNCHRONIZATION_UNAVAILABLE`).

## Part C: Republish And Resume

- `Intent::ItemRepublish{root, kind, id}`; the share outcome for
  `remote_branch_deleted` and L1's `Republish` offer Republish…, through
  `confirm_followed` with the existing Republish arm. `BRANCH_DELETED`
  loses "arrives in a later release" (E-6).
- Resume on a lifecycle row (L2) sends `OperationResume` and opens the
  dialog in Running on that request.

## Required Outcomes

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | Save fails before a lifecycle | No prepare; the save's own outcome shows |
| P2 | Observation changes after prepare | Prepare again; new summary; no run |
| P3 | Consent expires before Confirm | `confirmation_expired`; Prepare again |
| P4 | Stop at a safe point mid-run | Stopped; same request continues on Retry |
| P5 | Process exits mid-run | After relaunch, Resume on the row (L2) continues |
| P6 | Remote branch deletion refused | Shared and merged; not tidied; Retry tidies only |
| P7 | Retry after a partial result | No new checkpoint, merge or publication commit |
| P8 | Child conflict during a lifecycle | Review changes; after resolving, the same request finishes or answers `external_change` |
| P9 | Refused closure checkpoint merge | Repair locally…; then Retry |
| P10 | Incompatible closures | External; no winner |
| P11 | A side changes while the sheet is open | Refreshed comparison |
| P12 | Held item at prepare | Names the holding request or Resume |
| P13 | Close a ticket whose status text is `closed` | Runs; lifecycle Closed shown apart from status |
| P14 | Already-closed ticket | `ticket_closed`; no run |

## Native Checklist (Linux Wayland, Release Build, Product Owner)

Written in the final task, in D4's format: §1 Smoke; §2 Close and promote
(with remote, local-only, code on the branch); §3 Conflict (remote share,
post, local repair); §4 Failures (key refused, remote deletion refused,
stop, quit and relaunch); §5 Republish; §6 Keyboard only; §7 Results.

## Privacy

No title, body, comment, conflict side or result text enters an
operation entry, log or error. Path names appear only in the dialog and the
sheet. Sentinel tests (A16) seed each.

## Testing

| Target | What |
| --- | --- |
| `tests/mutation_remote/d5_way_out.rs` | L1 variants; CLI parity |
| `tests/read_status.rs` | L2 rows |
| `tests/mutation_remote/d5_progress.rs` | L3 order, cleanup-only replay stages |
| `D/view/lifecycle_tests.rs` (+ submodules) | Stages, save first, stale, outcomes, made-ready, P1–P7, P12–P14 |
| `D/model/conflict_tests.rs` | Parts, defaults, Keep both, external classification |
| `D/view/conflict_tests.rs` | Sheet, P8–P11 |
| `D/view/keyboard_tests/` | A1, A2 |
| Existing SSH fixture (`D/test_support/fixtures.rs`) | A7, A14 |

## Named Edits To Pre-Existing Tests

| E | Test | Edit | Owner |
| --- | --- | --- | --- |
| E-1 | `tests/read_status.rs` rows for unfinished promotion/closure | `next_action` null → `resume` | L2 |
| E-2 | CLI goldens for `operation list` / `repo status` with a lifecycle row | Same | L2 |
| E-3 | Tests asserting a lifecycle's progress (Task 0 sweep) | New stages | L3 |
| E-4 | `D/view/item/header_tests.rs`, `D/model/item_tests.rs`, `D/view/item_tests.rs` | Later notes → live actions | A1 |
| E-5 | `D/model/operations_tests.rs:465,534` | Unavailable → Offered | B6 |
| E-6 | `D/view/operations/outcomes_tests.rs:434`, `give_up_tests.rs:186` | `BRANCH_DELETED` text and Republish offer | C |
| E-7 | `D/view/item/share_tests.rs:698-701` | Banner gains Review changes | B5 |

Task 0's sweep confirms the list against merged C5.

## Document Amendments

Applied in the approval commit:

1. Wave D5 section: decisions 2, 5, 8; the library-part exception (rule 5).
2. Desktop RFC: "Mark resolved" → "Save resolution" (as D4 decision 7);
   per-part choice is a deliberate per-file choice.
3. CLI RFC: the new progress stages (decision 10) and `next_action` for
   lifecycle rows (decision 9).
4. API audit: the rows above.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | The lifecycle dialog is its own view over the confirmation boundary, not a `confirm.rs` arm | A second dialog shape to maintain |
| R-2 | Tags come from `canonical::item_path_kind`, else `code` | A misfiled tag; content unaffected |
| R-3 | No `+n −m` counts: the preview has none | Less detail than the handoff |
| R-4 | Keep both = mine, blank line, theirs; body only | A user edits the result by hand |
| R-5 | Single-side changes are taken without a choice | The user may want to see them; Before/Yours/Theirs still shows them |
| R-6 | Any external path makes the set external | Canonical paths wait for outside repair |
| R-7 | Partial results stay in the dialog's Failed stage | Dialog reads "failed" for a mostly-done run; the copy says what did happen |

## Known Limits

- **K-1.** `01M4H33R34Z7C7EEKTY1ZCT950`: a pending remote conflict cannot be
  abandoned.
- **K-2.** A retained conflict under a raised generation (`key select`,
  `key clear`, a failed poll plan) stays stuck (C5 Known Limits).
- **K-3.** A child holding a resolution whose branch is then deleted loops
  (C5 Known Limits).
- **K-4.** The classifier reads an edited text code file as `binary` and
  add/add as `delete`; both go outside (GRFC 468–474).
- **K-5.** `01M4R19PC0T6ZPKM39X9DF26HX`, `01M6GHWZS03WCADS1A1JXV4PMP`: a
  stopped share may hold the repository's reservation (D4 K-5, K-16).
- **K-6.** A resumed lifecycle that released its child and one that kept it
  answer alike; only repository details shows the reservation.
- **K-7.** No `+n −m` line counts in the summary (R-3).
- **K-8.** Made-ready titles are read after the close; a ticket deleted
  meanwhile shows by slug only.
- **K-9.** A context made by posting is not released by the desktop (D4
  K-9); a later close tidies it.

## Alternatives Rejected

- A separate library ticket for L1–L3 (owner chose decision 2).
- Recovery actions in envelopes for the way out (decision 3).
- A per-path mine/theirs/edit sheet (decision 4).
- A provisional made-ready list at confirm (decision 6).
- Parallel lanes (decision 8).

## Risks

| Risk | Control |
| --- | --- |
| C5 changes after this draft | Task 0 re-checks Facts 2–9 and stops on a difference |
| L3 plumbing touches the domain runners | Stages only; test-first; `document_promotion`, `ticket_closure`, `mutation_remote` run |
| `serialize_item` normalizes a side (U-4) | Task 0 settles; if it does, untouched paths submit the side's bytes verbatim |
| Save-first races an external change | The save's own `external_change` answer stops the dialog (P1) |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | See the Cycle document | — | Decisions 1–14 | Cycle Review Record. |
