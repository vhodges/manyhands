---
title: "Wave 03 D4 Desktop Discussion And Deliberate Synchronization Design"
date: 2026-11-02
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6E6967XJGW2S3SX1PKP0GGG"
---

# Wave 03 D4 Desktop Discussion And Deliberate Synchronization Design

## Intent And Approval Boundary

This design turns the [D4 Cycle](../Cycles/wave-03-desktop-04-discussion-and-sync.md)
into mechanisms an implementer can build without surprises. It covers the
desktop only. It consumes C4's bindings through the F2 boundary and changes
no library file (Cycle, Dependency boundary).

Approving this design approves its rulings (R-n), named test edits (E-n)
and the lane plan. It does not authorize implementation, pushing beyond
the ticket branch, merging, or closing the ticket.

**Approved 2026-11-02** (revision 5), with rulings R-n, named edits
E-1 to E-11 and the lane plan. Implementation was authorized separately the
same day, by subagent-driven development.

"Fact n" refers to Baseline Facts. Paths are relative to the repository
root; `desktop/` means `src/desktop/`. Revision 2 (after review round 1)
rewrote the posting and retry mechanisms; see the Review Record.

## Baseline Facts At `00bef3da`

Every fact below was confirmed in the source, during planning or by the
round-1 source review. Task 0 re-reads any fact whose file a later merge
of main changes.

**Library boundary (consumed, not changed).**

1. `RepositoryService::execute<P: SessionCredentialProvider>(&self, call: MutationCall, credentials: &mut SessionCredentials<P>, progress: &mut dyn ProgressSink, cancel: &CancellationToken) -> Envelope<MutationDataDto>` is synchronous on the caller's thread (`src/repository/mutation/execute.rs:41-45`).
2. `Mutation::CommentAdd(CommentAddInput { root, kind, id, comment_id, parent_id, body, created_by })` (`mutation/bind/comment.rs:84`), public. The caller supplies `comment_id`; `parent_id: None` is a root; `created_by: None` takes the Git identity (C4 decision 9). No confirmation.
3. `comment add` provisions or reuses the item's editing context itself (`checkpoint_comment_locally` → `prepare_context_unlocked` with `ContextIntent::Edit`, `src/repository.rs`).
4. Standing (`bind/comment.rs:16-31`): published or no-remote is finished; a pending publication is owed and "the same request publishes the comment without writing it again"; a child that is final (cancelled) is finished (`standing_of`, `comment.rs:451-460`); `remote_branch_deleted` is owed.
5. `CommentMutationDto { type, item_id, comment_id, parent_id, created_by, synchronization_id, publication, pending_reason, conflict }` (`mutation/dto.rs:611-632`); `CommentPendingReason` is `no_publication_remote | recovery_required | synchronization` (`dto.rs:592-597`).
6. `PublicationEffect` `not_requested | published | current | pending`; `CheckpointEffect` `not_requested | unchanged | committed | pending`; `WriteEffect` includes `written`; `DiscoveryEffect` `not_requested | current | pending` (`src/results.rs:55-81`).
7. `Mutation::ItemSync(ItemSyncInput { root, kind, id })`, `Mutation::RepoSync(RepoSyncInput { root })` (`bind/sync.rs:94-106`), merging. A conflict answers `recovery_required` with `SynchronizationConflict(SynchronizationConflictMutationDto { type, conflict: ConflictNextDto { operation_id, next_action } })` (`dto.rs:921-957`). With no publication remote both answer `publication_remote_required` before acceptance (`sync.rs:1379-1383`).
8. A synchronization that stops with an error after reserving keeps the reservation and its record stays accepted; "the same request takes it up again, and `request cancel` followed by the same request ends it" (`bind/sync.rs` module doc).
9. `ProgressStage`: `accepted, observing, writing, connecting, fetching, updating_local, pushing, verifying, indexing, stopping` (`progress.rs:83-96`). After acceptance the token is honoured only at a synchronization's safe points (`Watch::safe_point`, `sync.rs:432-501`); `comment add`'s child goes through the same `Watch`; `operation resume` honours it at a synchronization's safe points (`command.rs:357-365`).
10. `cancel_request(&self, request_id) -> Envelope<CancelDto>` (`progress.rs:400`) marks any accepted record of a command that stops at safe points (`progress.rs:430-460`); on a finished record it answers `acknowledged` only if the stored result was `cancelled` (`progress.rs:420-427`). `show_request(request_id) -> Envelope<RequestDto>` reports the record's `state` (`mutation/mod.rs:388`). Only a request's first call emits `Accepted` (`execute.rs:336`); a same-ID re-entry reports later stages only.
11. Error mapping (`outcome.rs:109-154`): a cancelled or unavailable passphrase → `unlock_required`; `UnlockFailed` → `unlock_failed`; `NoSelectedKey` → `no_selected_key`; key problems → `key_unavailable`/`key_rejected`; host approval/replacement → `host_approval_required`/`host_replacement_required` (with host data for syncs only, K-3); `RuntimeUninitialized` → `internal_error` (`outcome.rs:151-153`).
12. Replay (`execute.rs:34-39, 199-229`): a finished record replays its stored result; an accepted record is re-entered; the same ID with different input is `request_mismatch`. "A cancellation is final. A same-ID retry returns the stored `cancelled` result. A new request completes the work: `item sync` … for a comment's … publication" (`docs/RFC/cli-contract.md:993-997`).
13. A second `comment add` with a new request ID and an already-bound `comment_id` is refused by `bind_comment_publication`'s collision check as `OperationMismatch` (`src/repository/recovery.rs:291-297`), which maps to `internal_error` (`outcome.rs:79`). `occupied_path` means the comment's path holds different content (`src/repository.rs:3030-3046`).
14. `busy`, `poll_yielding`, `remote_unavailable`, `transport_unavailable` and `registration_pending` always offer `request.retry` of their own request (`outcome.rs:843-850`), whatever the record's state. `request.cancel` is offered only with the request's own ID for codes the same request never gets past (`outcome.rs:983-990`) and by `conflict resolve`'s continuation; `offering_cancel` only filters it (`outcome.rs:1263-1276`). A `busy` answer does not name the request holding the reservation. A sync reserves before it uses the transport (`remote/sync.rs`), so `unlock_required` and transport failures after reserving may arrive with the reservation held and the record accepted (fact 8); transient codes refused before reserving do not.
14a. Quitting cancels the running job's token (`desktop/services/worker.rs:495-499`); an accepted safe-point job then stops at its next safe point, and a stopped comment publication is final (fact 4).
14b. `identity_required` keeps a comment's record accepted (`bind/comment.rs:556-566`), and a sync's merge commit can stop on it too (`bind/sync.rs` module doc); `Intent::RepoIdentitySet` exists (`services/requests.rs`).
14c. `RequestId::parse(&str)` accepts any canonical uppercase ULID (`mutation/identity.rs:21-24`), so a request ID can equal a comment ID.
14d. A Give up's re-sent request does not always end `cancelled`: with an unsettled push it asks the remote first and may end `recovery_required` (`bind/sync.rs:250-264, 468-486`), and `cancel_request` on a fenced record finishes it with the fenced answer (`progress.rs:436-453`).
14e. `OperationDto.item_id` is null for `submit_comment` rows (`read/dto.rs:866-868`; known limit `01M4W5SCBV6WFJP11Y7PJF5VS8`): no operation read names a comment.
15. `list_comments` returns canonical order flattened with `depth`, malformed files last, from the effective copy, and only what the index has seen (`read/comments.rs:216-270`). No read reports publication state or counts unpublished work.
16. `RepositoryService::set_request_hook_for_testing(hook)` runs after acceptance and before the domain call (`mutation/mod.rs:163-173`).
17. Transport-backed calls return `RuntimeUninitialized` before preparing a key or asking for a passphrase (`transport/operation.rs:83-85`), so no desktop test reaches a passphrase prompt through the library.

**Desktop (changed).**

18. One mutation worker thread owns the only `SessionCredentials`. `WorkerEvent::Finished(DesktopRequestId, Box<Envelope<MutationDataDto>>)` carries the whole envelope; the token reaches `execute` (`desktop/services/worker.rs:51-60, 621-631`). Request IDs live in memory in `Submitted` (`worker.rs:130-142`).
19. `WorkerHandle::cancel`: `Removed` if queued, `Signalled` only if running and not accepted, else `TooLate` (`worker.rs:437-454`); `accepted` is set on `Accepted` (`worker.rs:793`).
20. `busy` is re-admitted under the same ID after 100/200/400 ms unless the token was cancelled (`worker.rs:640-668`).
21. `Intent` has no comment or sync variant (`services/requests.rs:43-228`); `Intent::label(&self) -> &'static str` (`requests.rs:267`) feeds the operation area and the quit dialog (`view/quit.rs:78`).
22. The operation area's Cancel step calls `worker.cancel(id)` (`view/workspace/requests.rs:323-325`); `RecoveryStep::CheckHost` needs an authority (`model/operations.rs:590-593`).
23. `resume_offer` shows Resume unavailable for every synchronization row (`model/operations.rs:614-626`).
24. `LATER = [COMMENT, SHARE, APPROVE_AND_MERGE, CLOSE_TICKET]` (`model/item.rs:48`); `header_later` (`view/item/header.rs:167-176`); the discussion's disabled Comment uses `NoteAt::BelowOnPage` (`view/item/discussion.rs:262-273`), its only construction (`header.rs:109, 147`).
25. Draft sessions are keyed by `DraftId` (`services/drafts.rs`); `begin(target, base, current)` creates a draft (`drafts.rs:347-381`); `edit` schedules a flush (`drafts.rs:384-402`). Comment drafts are excluded from `stored_for`, restore and save (`drafts.rs:313-337`, `drafts/restore.rs:54, 348-355`, `model/drafts/save.rs:93-99`). D2's retirement is `complete(SaveRecord)` (`drafts/retire.rs:52`, `pub(super)`); the store's `DraftClaim::retire(saved_revision)` is at `src/drafts/store.rs:555`. The Drafts page draws `COMMENT_LATER` in `view/drafts/rows.rs:150`.
26. The library draft schema carries comments: `DraftTarget { …, comment: Option<CommentTarget { comment_id, parent_id }> }`, `DraftFields::Comment { body }` (`src/drafts/schema.rs`).
27. Release on leave requires the context in `CreatedContexts` with no save committed since, and no draft of the item listed, comment drafts included (`services/release.rs:1-30`).
28. A finished request's `changes_items` already triggers the item tab's reload (`view/item/reads.rs:149-150`, `model/operations.rs:221`).
29. `ItemPane`'s fields, `new` and `render` live in `view/item.rs` (461 lines). `keys.rs` has no Ctrl Enter.
30. Desktop tests are bin unit tests under `desktop-test-support`, run in CI by `cargo test --locked --no-fail-fast --features desktop-test-support --bin manyhands` (`.github/workflows/build.yml:128-129`). `test_support/delaying.rs` holds a job before `execute` (`hold_job`, :84-96) or after it returns (`hold_job_return`, :173-185). `fixtures.rs::publish_over_ssh` and `pin_host` exist. The passphrase dialog's tests raise prompts through `WorkerHandle::provider`/`send_prompt` (`worker.rs:412-420`; `passphrase_tests.rs:1-7`).
31. `ItemId::generate()` is public (`src/canonical.rs:19`).

**Unverified, settled in Task 0 by reading.**

- U-1. What `item sync` answers for an item with no editing context.
- U-2. The exact envelope of a comment posted with `publish_over_ssh` in a desktop test: expected `internal_error`, checkpoint committed, publication pending, `pending_reason: synchronization`, owed (fact 11).
- U-3. For a comment, which codes offer `request.retry` while the record is finished (fact 14 lists the transient ones); R-12 treats those by `show_request`'s state, not by the offer alone.
- U-4. Retired by R-16: an edited text always posts under a new comment ID.
- U-5. After a restart, how a reservation held by a request of the previous session is ended: whether `operation resume` of the pending remote row, then Stop at a safe point, ends it (fact 9: resume honours the token).

Any answer that differs from expectation stops for the owner.

## API Audit Refresh (Entry-Gate Item 3)

Done at planning, against `00bef3da`, from the facts above:

| Audit row | D4 consumes | State at `00bef3da` |
| --- | --- | --- |
| D23 composer (W3-10) | `CommentAddInput`, drafts schema | Library delivered (facts 2, 26); desktop absent. |
| D24 saved once, retry publication (W3-10) | `CommentMutationDto`, standing, replay | Merged with C4 at `89c8383b` (facts 4, 5, 12); the row's "on the ticket branch" is stale. |
| D25 explicit sync dialogs (W3-10/11) | `ItemSync`, `RepoSync`, outcomes | Merged (fact 7); D4 shows scope without a dialog (decision 9). |
| D04 context provisioning (W3-09/10) | `comment add` provisions (fact 3) | Merged. |
| D06 operation area | D1's operation area | Merged; D4 extends it (decision 2). |
| D30 stale-completion guards | `Stamp`, generations | Merged; D4 adds composer-level guards (contract 8). |

The approval commit amends rows D24 and D25 (Document Amendments).

## Execution Lanes (Decision 1)

```
Part 0 (serial, ticket worktree)
   ├── Lane A: comment drafts ─────────────┐
   ├── Lane B: share, progress, recovery ──┤ parallel, lane worktrees
   └── Lane C: the composer widget ────────┘
                         Lane D: discussion integration (ticket worktree)
                         Part E: keyboard, privacy, native evidence, gate
```

The lanes share only Part 0's seams, and each owns a disjoint file set.
Lane C builds the composer as a standalone entity with an event
interface, so it needs neither drafts nor outcomes; Lane D wires A, B and
C together.

**Lane mechanics.**
- Lane worktrees live under the gitignored `.worktrees/` of the main
  checkout, on local branches `d4/lane-a`, `d4/lane-b`, `d4/lane-c`,
  created from the ticket branch at the Part 0 checkpoint. They are never
  pushed and never under `manyhands/ticket/`.
- Every task, in a lane or not, gets a fresh implementer and then a fresh
  reviewer before the next task in that lane starts. Implementers never
  dispatch subagents.
- **Only the orchestrating session** writes the execution ledger and
  ticket comments, merges branches and pushes. It works in the ticket
  worktree, only when that worktree is clean, between merges. Lane
  implementers never edit ticket, comment or ledger files.
- **Main.** The orchestrator merges `origin/main` into the ticket branch at
  the Part 0 checkpoint, at each lane merge, before Task 12, and whenever
  main moves (`AGENTS.md`, Working on a ticket). A lane takes main by
  merging the ticket branch into itself at its next task boundary. Lanes
  never merge `origin/main` directly and never rebase.
- **Lane merge.** At a lane checkpoint the orchestrator merges the ticket
  branch into the lane, runs the lane's targeted tests and the whole
  desktop bin suite there, then merges the lane into the ticket branch
  with `--no-ff`, runs the whole desktop bin suite again, pushes the
  ticket branch, and removes the lane worktree and branch.
- **Out-of-row need.** A task that needs a file outside its row stops. The
  orchestrator makes the change as its own commit on the ticket branch,
  reviewed like a task, and each running lane merges the ticket branch at
  its next task boundary.
- Each worktree has its own Cargo target directory; three cold desktop
  builds are the cost of isolated builds.

**File ownership** (paths under `src/desktop/` unless stated).

| Owner | Files |
| --- | --- |
| Part 0 | `services/requests.rs`, `services/requests/build.rs`, `services/requests_tests.rs`, `services/mod.rs`, `services/worker.rs`, `services/worker_tests.rs`, `services/prompts.rs`, `services/prompts_tests.rs`, `services/submissions.rs` (new), `services/submissions_tests.rs` (new), `test_support/delaying.rs`, `model/mod.rs`, `model/publication.rs` (new), `model/publication_tests.rs` (new), `model/item.rs`, `model/item_tests.rs`, `keys.rs`, `view/item.rs`, `view/item_tests.rs`, `view/item/header.rs`, `view/item/header_tests.rs`, `view/item/discussion.rs`, `view/item/discussion_tests.rs`, `view/item/{share,composer,posting}.rs` (stubs); for the Part 0 seam commit (Task 5a, 2026-11-02) also `view/tabs.rs` (the item-route subscription), the `view/workspace/requests.rs` route stub and the two hook calls in `view/item/reads.rs` |
| Lane A | `services/drafts.rs` (one `mod` line), `services/drafts/comments.rs` (new), `services/drafts/comments_tests.rs` (new), `model/drafts/rows.rs`, `model/drafts/rows_tests.rs`, `view/drafts.rs`, `view/drafts/rows.rs`, `view/drafts/row_actions.rs`, `view/drafts_tests.rs` |
| Lane B | `view/item/share.rs`, `view/item/share_tests.rs` (new), `view/item/pill.rs`, `view/item/banners.rs`, `view/item/header.rs`, `view/item/header_tests.rs`, `view/sidebar.rs`, `model/operations.rs`, `model/operations_tests.rs`, `view/operations.rs`, `view/operations_tests.rs`, `view/passphrase.rs`, `view/passphrase_tests.rs`, `view/workspace/requests.rs`, `model/item.rs` (only to remove `SHARE`'s dead-code allowance, Task 8), and child modules `view/operations/*.rs` and `model/operations/*.rs` split from those files (orchestrator, Task 9 review) |
| Lane C | `view/item/composer.rs`, `view/item/composer/*` (new), `view/item/composer_tests.rs` (new), additive changes to `view/editor.rs`. The composer is not wrapped in `EditHost` (its `ToggleRich` and Escape would shadow the composer's keys). |
| Lane D | `view/item/posting.rs`, `view/item/posting_tests.rs` (new), `view/item/discussion.rs`, `view/item/discussion/rows.rs`, `view/item/discussion_tests.rs`, `model/discussion.rs`, `view/item/reads.rs`, `view/item.rs`, `view/item/pill.rs` (after Lane B merges; "Updating the list…"), `services/mod.rs` (the drain hook that retires comment drafts, P9), `services/created_contexts.rs`, `services/created_contexts_tests.rs`; `view/item/composer.rs` and `view/item/composer/*` after Lane C merges (placing the composers, Task 12); for Task 14, the Drafts page's held comment rows (`model/drafts/rows.rs`, `view/drafts/*`, and `services/drafts.rs` or `services/drafts/comments.rs` if needed): Show must reach the tab holding a comment draft, and Discard of a held comment draft must reset that composer (Task 12 review); and removing the `ItemPane` fields' dead-code allowances in `view/item.rs` |
| Part E | `view/keyboard_tests.rs` (one `mod` line), `view/keyboard_tests/discussion.rs` (new), `view/editing_privacy_tests.rs`, `docs/plans/2026-11-02-wave-03-desktop-04-on-screen-checklist.md` (new) |

Files named in two rows pass between owners in sequence: `view/item/header.rs`, `discussion.rs`, `services/mod.rs` and `view/item.rs` from Part 0 to Lanes B or D; `view/item/pill.rs` from Lane B to Lane D. Part 0 finishes before the lanes start, and Lane D starts only after Lanes A, B and C have merged.

## Part 0: Seams

### Intents (Task 1)

| Intent | Mutation | `job_kind` | `changes_items` | `stops_at_safe_points` | `label()` |
| --- | --- | --- | --- | --- | --- |
| `CommentAdd { root, kind, id, comment_id: ItemId, parent_id: Option<ItemId>, body: CommentBody }` | `CommentAdd(… created_by: None)` | `JobKind::Comment` | yes | yes | "Post comment" |
| `ItemSync { root, kind, id }` | `ItemSync` | `JobKind::SynchronizeItem` | yes | yes | "Share item" |
| `RepoSync { root }` | `RepoSync` | `JobKind::SynchronizeRepo` | yes | yes | "Share main" |
| `OperationResume` (existing) | unchanged | unchanged | unchanged | **yes** (fact 9) | unchanged |

All on `Lane::User`. `CommentBody` wraps `String`, derives neither `Debug`
nor `Display`, and is only read by `build`. Labels are fixed text (fact
21); no title is interpolated.

### Safe-point stop (Task 2, R-1)

`Running` gains `stops_at_safe_points`. `cancel` signals the token when the
running job is not accepted, or is accepted and stops at safe points, and
returns `Signalled`. A local command stays `TooLate` after acceptance. The
worker marks a job accepted on `Accepted` **or on any later stage**, since
a same-ID re-entry never emits `Accepted` (fact 10). Stop is shown only
when the job's repository has a publication remote (a no-remote post has
no safe point).

### Give up (Task 2, decision 8)

**Give up** ("Stop trying to share") is offered exactly when the attempt's
`record_accepted` is true (below) and its intent stops at safe points. It
runs as a `Work::GiveUp` job on the user lane: `show_request`; if the
record is still `accepted`, `cancel_request(request_id)` and then the same
request again; otherwise "Already finished". The re-sent request usually
ends `cancelled` (facts 8, 10), but not always (fact 14d): its answer is
rendered through `sync_outcome`/`comment_outcome` like any other, and it
may need the network or prompt (a declined key answers at once).

**`record_accepted`.** After every finished attempt of a comment or sync
request, the worker calls `show_request` and attaches the record's state
to the `Finished` event; the outcome functions take it in `Attempt`. A
Give up is refused while a retry of the same request is queued or running,
and a retry is refused while its Give up is (source round 2, M9).

This is the way out of a reservation held by a request that stopped with
an error, including `unlock_required` after Not now (fact 14).

### Busy repositories

A `busy` answer names no holder (fact 14). The desktop names a possible
one: a request of this session on the same root whose last attempt has
`record_accepted` and a code answered after reserving
(`unlock_required`, `unlock_failed`, key and host codes,
`transport_unavailable`, `remote_unavailable`, `push_rejected`,
`remote_branch_deleted`), shown as "May be waiting for {its label}" with a
link to its row; otherwise "Another share may be holding this repository"
with a link to the pending operations list. Retry stays offered.

### Declined keys (Task 2, R-13)

A session set of declined key IDs, `Arc<Mutex<HashSet<…>>>`, shared by the
worker's `DesktopProvider` and `WorkerHandle`. Not now (`Cancelled`) adds
the `UnlockRequest`'s `key_id`; a later prompt for that key answers
`Cancelled` at once and its request finishes `unlock_required`. **Unlock…**
removes the key, then retries. The finished request holds the
repository's reservation (fact 14); Give up releases it.

### Submissions (Task 2, `services/submissions.rs`)

A session registry, shared by every item tab, keyed by
`(repository, comment_id)`:

```rust
pub(crate) struct Submission {
    pub request: DesktopRequestId,     // the worker request, for retry
                                       // (its library RequestId is the comment ID, R-16)
    pub draft: Option<DraftId>,        // retired by Lane D's drain hook
    pub state: SubmissionState,        // Posting | Finished(CommentOutcome)
}
```

- `submit(intent, draft) -> Option<DesktopRequestId>` refuses while the
  comment ID is `Posting` (contract 3, A5); the composer then shows
  "Already posting" rather than nothing.
- `retry(comment_id)` resubmits the same request through `worker.retry`.
- `restart(old_comment_id, new_comment_id, intent)` forgets the old
  submission and submits under a new comment ID (and so a new request ID);
  only where R-12 allows.
- **Durable request (R-16).** A post's library `RequestId` is
  `RequestId::parse(&comment_id.to_string())` (fact 14c). The worker gains
  `submit_with_request_id`. The composer's draft is flushed and the flush
  awaited before Submit, so a restored draft's text equals what was
  posted. The first post after a relaunch is therefore the same request:
  a finished record replays its result, an accepted one is re-entered and
  checkpoints a written file once and publishes once (fact 4).
- **It is updated from the services event drain** (`Services::start`,
  `services/mod.rs`), not from tabs, so an outcome with no tab open still
  lands, and later submits are not refused forever (round 2, I3).

### Outcomes (Task 3, `model/publication.rs`)

`comment_outcome(&Envelope<MutationDataDto>, Attempt) -> CommentOutcome`,
where `Attempt { resend: bool, record: RecordState }`, where `RecordState` is `Accepted | Finished | Unknown` (`show_request` can fail; Unknown offers Retry and Check again only, never Edit text or Post as new — P17). Give up is offered on exactly the rows that name it. Rows
in precedence order (first match wins). "Saved" means `checkpoint:
committed`.

| # | Condition | `CommentOutcome` | Offered |
| --- | --- | --- | --- |
| 0 | `code: request_mismatch` (any checkpoint state; no data) | `Mismatched { saved }` | Discard, Copy text; never Retry or Give up (P12) |
| 1 | `Comment` data, `publication: published` or `current` | `Shared` | — |
| 2 | saved, `pending_reason: no_publication_remote` | `SavedHere` | — |
| 3 | saved, `outcome: cancelled`, publication `pending` | `SavedStopped` | **Share** the item (new `ItemSync`, fact 12) |
| 4 | saved, `conflict: Some(next)` | `SavedConflict { next }` | none (decision 13) |
| 5 | saved, `code: remote_branch_deleted` | `SavedBranchDeleted` | Give up; no Share (D5's republish) |
| 6 | saved, `code: unlock_required` or `unlock_failed` | `SavedNeedsUnlock` | Unlock…, Give up |
| 7 | saved, `code: no_selected_key`, `key_unavailable`, `key_rejected` | `SavedNeedsKey` | Open Sharing & keys, Retry sharing, Give up |
| 8 | saved, `code: host_approval_required`, `host_replacement_required` | `SavedNeedsHost` | Check host, Retry sharing, Give up |
| 9c | saved, `code: identity_required` (the child's merge) | `SavedNeedsIdentity` | Set identity, Retry sharing, Give up |
| 9a | saved, other pending, `record_accepted` | `SavedNotShared { code }` | Retry sharing, Give up |
| 9b | saved, other pending, record finished | `SavedNotSharedFinal { code }` | Share (a new `item sync`) |
| 10 | not saved, `outcome: cancelled` | `NotSavedStopped` | Post again (R-12) |
| 11 | not saved, `code: identity_required` | `NotSavedIdentity` | Set identity, then Retry; Discard, Copy text |
| 12 | not saved, `code: occupied_path` | `NotSavedConflicting` | Discard; text kept |
| 13 | not saved, `code: internal_error` on any re-send of a comment ID (adopted first post or a Retry) | `MaybeSaved` | Check again (R-8, P15) |
| 14 | not saved, `effects.write: written` or `record_accepted` | `NotSavedOwed { code }` | Retry; Discard, Copy text (R-12) |
| 15 | otherwise | `NotSaved { code }` | Edit text, which posts under a new comment ID (R-12) |

`sync_outcome(&Envelope<MutationDataDto>) -> SyncOutcome`, in order:
`published` → `Published`; `current` → `Current`; `cancelled` →
`Stopped { completed }` (completed = the effects show completed work,
`results.rs:312-321`); conflict data → `Conflict { next }`;
`remote_branch_deleted` → `BranchDeleted`; `unlock_*` → `NeedsUnlock`; key
codes → `NeedsKey`; host codes → `NeedsHost { data }`; `identity_required` → `NeedsIdentity` (Set identity, Retry); `busy` → `Busy`
(holder found as in Busy repositories); `publication_remote_required` →
`NoRemote`; `item_not_found` on Share item → `NothingToShare` ("Nothing to share yet: this item has no saved changes."; owner, 2026-11-02, U-1); otherwise `Failed { code, retry }`. `sync_outcome` also takes `record_accepted`.

Lanes B and D render these and never read envelope fields themselves.

### Removing the placeholders (Task 4)

`LATER` becomes `[APPROVE_AND_MERGE, CLOSE_TICKET]`; `COMMENT` is removed;
`SHARE` stays, marked `// Called from Task 8.`; `header_later` lists only
the primary action; the discussion's disabled Comment and
`NoteAt::BelowOnPage` with its branch are removed (fact 24). Named edits
E-1 to E-7.

### Slots and keys (Task 4)

`ItemPane` gains `share: ShareState`, `composers: Composers` and
`posting: Posting`, each defined with `Default` and an empty `render` in
its own stub file, rendered once in the design's order.

`keys.rs`, `CommentComposer` context: `PostComment` (`secondary-enter`),
`CancelComposer` (`escape`), `ComposerToggleMode` (`secondary-shift-m`,
the page's binding, `01-item-frame.md` Keyboard). Tab and Shift-Tab
indent and outdent in the composer's editor, as in the body editor; F6 and
Shift-F6 (the global region keys) leave the composer (owner, 2026-11-02,
decision 19). Escape in a composer cancels; in the body editor it
still moves focus to the header (R-59 scoped by Document Amendment 3).

## Lane A: Comment Drafts

`services/drafts/comments.rs` adds to `DraftRegistry`:

```rust
pub(crate) fn begin_comment(&mut self, item: ItemKey, target: CommentTarget, body: String, cx) -> Task<Result<DraftId, DraftErrorKind>>;
pub(crate) fn comment_drafts(&self, item: &ItemKey) -> Vec<CommentDraft>;   // listed and live, per thread
pub(crate) fn adopt_comment(&mut self, id: DraftId, cx) -> Task<Result<AdoptedComment, DraftErrorKind>>;
pub(crate) fn retire_comment(&mut self, id: DraftId, cx);
pub(crate) fn discard_comment(&mut self, id: DraftId, cx);

pub(crate) struct CommentDraft { pub id: DraftId, pub target: CommentTarget, pub live: bool }
pub(crate) struct AdoptedComment { pub id: DraftId, pub target: CommentTarget, pub body: String, pub state: RecoveryState }
```

- The **caller** allocates the comment ID (Lane D, `ItemId::generate()`)
  and passes it in `target`, so a failed draft store still leaves an ID to
  post with ("Draft not kept").
- `comment_drafts` waits for the first listing to land; Lane D begins a
  draft only after it has (source finding 17f).
- `retire_comment` is new code over `DraftClaim::retire(saved_revision)`
  (fact 25); it does not reuse D2's save-bound `complete`.
- `edit` and flushing are unchanged. `stored_for` still ignores comment
  drafts.
- The Drafts page's comment rows offer **Open**, which opens the item tab;
  Lane D's composer adopts the draft (E-8, E-9).

## Lane B: Share, Progress And Recovery

### Share controls

- **Item header:** `Share` (secondary, `cloud-upload`) after Edit or
  Save/Done. Tooltip: "Brings in others' changes and shares this item's
  saved work through {remote}. Unsaved edits are not included." With no
  publication remote: **Share…**, which opens Settings › Remotes for the
  repository (new workspace route).
- Share is enabled whenever the repository has a publication remote and
  no share of this item is running; the library answers for an item with
  no context (U-1, decision 10).
- **Repository detail** (`view/sidebar.rs`): **Share main**, `RepoSync`,
  same rules.
- **Repeated press:** while a share of the same target is queued or
  running, the button reads "Sharing…" and is disabled.

### Stage text

| Stage | Share | Post |
| --- | --- | --- |
| `accepted`, `observing` | "Getting ready…" | "Getting a private copy ready…" |
| `writing` | — | "Saving…" |
| `connecting` | "Connecting to {remote}…" | same |
| `fetching` | "Bringing in changes…" | same |
| `updating_local` | "Merging changes…" | same |
| `pushing` | "Sending your saved changes…" | same |
| `verifying` | "Checking it arrived…" | same |
| `indexing` | "Updating the list…" | same |
| `stopping` | "Stopping at a safe point…" | same |

The pill reads "Sharing · {stage}" for a share and "Posting · {stage}" for
a post; a post with no remote never says "Sharing".

### Stop at a safe point

An accepted entry whose intent stops at safe points shows **Stop at a safe
point** in place of D1's "Finishing" note; pressed, it calls `cancel` and
reads "Stopping…". The outcome text comes from `Stopped { completed }`:
"Stopped before anything was sent. Your saved work is still on this
computer." or "Stopped after part of the work finished. Your saved work is
on this computer; Share sends the rest."

### Outcomes and routes

| `SyncOutcome` | Pill / entry text | Control |
| --- | --- | --- |
| `Published` | "Shared" | — |
| `Current` | "Same as the shared copy" | — |
| `Stopped` | as above | Share (new request) |
| `Conflict` | "Needs attention" + item banner (decision 13) | — |
| `NeedsUnlock` | "Your key is locked" | Unlock… (R-3, R-13) |
| `NeedsKey` | "Choose a key to share with" | Open Sharing & keys (new route) |
| `NeedsHost` | "Check {host} before sharing" | Check host (existing route with the host's authority) |
| `Busy` | "Waiting for {label}" or "Another share is holding this repository" | Show it; Retry |
| `NoRemote` | "This repository has nowhere to share to" | Set up sharing (Remotes route) |
| `BranchDeleted` | "The shared copy of this item was deleted. Republishing it arrives in a later release." | Give up; no Share |
| `Failed { retry }` | the code's fixed message | Retry if `retry`; Give up if owed |

Every outcome whose `record_accepted` is true also offers **Give up**
(decision 8); a `Failed` outcome whose record is finished offers Share,
not Retry.

The conflict banner reads: "You and someone else both changed this since
it was last shared. Your work is safe. Resolving it in Manyhands arrives
in a later release." It offers no action (decision 13).

New workspace routes, in `view/workspace/requests.rs`: Remotes page for a
root; Keys page; Check host for a root with no authority (K-3), which
opens the Hosts page for that root.

### Resume (E-10, decision 11)

Resume is offered for a pending row that is a comment publication or a
remote `item`/`repo` synchronization, using the row's `operation_id`. It
stays unavailable for local-context synchronizations and resolutions (D5).

### The passphrase dialog

Subtitle "Sharing {repository} is waiting for your passphrase." Cancel is
relabelled **Not now** (the `CANCEL` constant's text; tests use the
constant, so no test edit). The note adds "If you choose Not now, your
work stays saved on this computer. Unlock later from the waiting share, or
stop trying to share it."

## Lane C: The Composer Widget

`Composer` (`view/item/composer.rs`) is an entity independent of drafts
and posting.

- **Editor:** a `zorite_editor::EditorState` from `open_editor`, compact
  (no page chrome; min height 70 px new, 64 px reply; grows). D3's guard,
  focus and load-normalization rulings apply (R-10).
- **Mini toolbar:** "Replying to {name}" (reply only), Bold, Italic, Inline
  code, Link, Bulleted list, the Rich text / Markdown switch, and the
  draft indicator ("Draft kept" / "Draft not kept. Post to protect your
  text.") from a `RecoveryState` the owner sets. Arrow keys move along it.
- **Placeholder:** "Add a comment. Markdown works here." (new) / "Write a
  reply. Markdown works here." (reply). A reply shows the user's avatar.
- **Footer** (R-9): with a remote, "Posting also shares other saved work
  in this {ticket|document}. Unsaved edits above are not included."; with
  none, "Saved on this computer. Unsaved edits above are not included."
  Reply footer hint: "Ctrl Enter to post · Esc to cancel". Buttons: new —
  **Post and sync** / **Save comment locally**; reply — Cancel and
  **Reply and sync** / **Save reply locally**.
- **Events:** `Changed(String)`, `Submit`, `Cancel`, `Discard`, plus the
  `Failed` actions below.
- **States:** `Idle`; `Posting` (read-only, "Posting…", no `Submit`);
  `Failed { message, actions: FailedActions }` (read-only, red message).
  `FailedActions` is set by the owner from the outcome: any of **Retry**,
  **Edit text** (back to `Idle`), **Set identity**, **Discard**, **Check
  again**. The composer emits `Retry`, `EditText`, `SetIdentity`,
  `Discard`, `CheckAgain` and does not decide which are offered.
- **Discard prompt:** owned by the composer. `Cancel` with text shows
  "Discard this reply?" in a reply composer and "Discard this comment?" in
  the new-comment composer (Discard · Keep editing), and emits `Discard`
  on Discard; with no text it emits `Cancel` silently. Kit dialogs close at
  once, as D3's do.

## Lane D: Discussion Integration

### Placement

- The new-comment composer follows the threads. Its comment ID is
  allocated with `ItemId::generate()` on the first keystroke after the
  item's comment drafts are listed, and `begin_comment` is called with it.
- **Reply** appears on every comment and opens the single reply composer in
  that comment's thread, after its last reply, with focus in it. The
  reply's parent is the thread's root (decision 5); "Replying to {name}"
  names the comment it was pressed on.
- Moving the reply composer keeps the old thread's draft and adopts the
  new thread's draft if any. **The reply composer cannot move while it is
  Posting**; Reply elsewhere is disabled with "Finish or cancel the reply
  being posted" (round 2, I7).
- Opening an item adopts its comment drafts.

### Required outcomes of posting recovery

This table (P1–P18) is the contract for Task 13, which is test-first with a design
checkpoint: the implementer writes one test per row, then proposes the
state machine to the orchestrator before writing code.

| # | Situation | Required outcome |
| --- | --- | --- |
| P1 | A post's checkpoint commits and its publication succeeds | One file, one checkpoint; row shown once; draft retired; composer cleared. |
| P2 | The checkpoint commits, publication fails, the user presses Retry sharing any number of times | No further file or checkpoint; the same request each time; chip until shared. |
| P3 | The checkpoint commits, the user stops the publication, or quits while it runs | Saved, publication cancelled (final); Share (a new `item sync`) publishes it; no same-ID retry is offered. |
| P4 | The process dies at any point after Submit (before the write, after the write, after the checkpoint, during publication) | On relaunch the adopted draft's post is **the same request** (R-16), re-sent first (P11); the listing is consulted only when no record exists (P16): a finished record replays its result, an accepted one checkpoints a written file once and publishes once. Never a second comment. |
| P5 | A post answers `MaybeSaved` (`internal_error` on a re-send, P15) | "This comment may already be saved." with **Check again** (`IndexRefresh`, listing, `show_request`). **Post as a new comment** (new comment ID, same text) only under P16. Operation rows cannot be matched to a comment (fact 14e). |
| P6 | A post fails before writing anything, and the record is finished | Edit text is allowed; the edited text posts under a new comment ID and request (R-16). |
| P7 | A post fails after writing, or its record is accepted (`identity_required`, transient codes) | Retry (and Set identity for `identity_required`); no Edit text. Discard (with "This comment may still be saved later if it was written") and Copy text keep the user from being trapped; after Discard the comment ID stays reserved and later posts use new IDs. |
| P8 | `occupied_path` | Text kept; Discard offered; never retired silently. |
| P9 | The tab is closed while posting | The outcome lands in `Submissions` from the drain; the draft is retired there on a committed checkpoint; reopening shows the comment and any chip. |
| P10 | A second Submit while posting | Refused, with "Already posting". |
| P11 | Relaunch with an adopted draft whose post has a record (accepted or finished) | The same request is re-sent **before** any listing check, and its answer is rendered; the draft is retired only on a committed checkpoint. A written-but-uncommitted file that the listing shows (it reads the working copy, `read/comments.rs:210-216`) never retires the draft by itself. |
| P12 | `request_mismatch` (the posted text, parent or repository path differs from the record, `bind/comment.rs:775-805`) | Never Retry or Give up, which would loop. The text is kept with Copy text and Discard; if the record's checkpoint is committed the comment is shown as saved. Submit stores the exact posted body with the draft, and an adopted draft that was submitted stays read-only until its same request has been re-sent (load normalization, `model/editor_text.rs:24-34`, must not change it). |
| P13 | Edit text after a finished failure (P6), then a crash | The draft is re-targeted to the new comment ID (retired and begun again, flush awaited) before the new Submit, so relaunch adopts the new ID, never the old. |
| P14 | Give up whose re-send still answers with the record accepted (unlock, key, transport) | Offers the step that lets it finish (Unlock…, Open Sharing & keys, Retry), not Give up again. |
| P15 | `internal_error` on any re-send of a comment ID, including a Retry after `operation abandon` left the binding and file (`bridges/abandon.rs:563`) | `MaybeSaved`; never Edit text under a new ID while the file may exist. |
| P16 | Post as a new comment | Offered only after a successful `IndexRefresh`, a listing with `complete: true` and a current index, `show_request` answering no record, and the comment not listed. |
| P17 | `show_request` fails after an attempt | Record state Unknown: Retry and Check again only. |
| P18 | Discard after a written, uncommitted file (P7) | The copy says the file stays on this computer and may still appear in the thread; no chip. |

### Posting

1. `Submit`: before the first post of an **adopted** draft, P4's checks
   run. Then the composer goes `Posting` and `Submissions::submit` sends
   `CommentAdd` with the draft's comment ID and parent, the text and
   `created_by: None`.
2. A pending row at 60% opacity is drawn in the thread from the text,
   keyed by comment ID, until the outcome.
3. Outcomes apply to a composer only if its current comment ID matches
   (round 2, I7). On the outcome:

| `CommentOutcome` | Thread | Composer | Draft |
| --- | --- | --- | --- |
| `Shared` | listed row | cleared | retired |
| `SavedHere` | row + "Saved on this computer" | cleared | retired |
| `SavedNotShared`, `SavedNeedsUnlock`, `SavedNeedsKey`, `SavedNeedsHost`, `SavedBranchDeleted` | row + amber chip "Saved on this computer · not shared yet" + the row's controls | cleared | retired |
| `SavedStopped` | row + chip + **Share** | cleared | retired |
| `SavedConflict` | row + chip; item banner (decision 13) | cleared | retired |
| `NotSavedStopped`, `NotSaved` | no row | `Failed`: Edit text (P6) | kept |
| `NotSavedOwed` | no row | `Failed`: Retry (P7) | kept |
| `NotSavedIdentity` | no row | `Failed`: Set identity, Retry | kept |
| `NotSavedConflicting` | no row | `Failed`: Discard (P8) | kept |
| `MaybeSaved` | no row | `Failed`: Check again (P5) | kept |

- Draft retirement on a committed checkpoint happens in the services
  drain (P9): Lane D adds the hook that calls `retire_comment` for the
  `Submission`'s draft.
- The chip's controls act on the comment's `Submission`: **Retry sharing**
  and **Unlock…** retry the same request (owed, fact 4); **Share** submits
  a new `ItemSync`; **Give up** as in Part 0; others use Lane B's routes.
  The chip lives in `Submissions` for the session, survives reloads, and
  clears when the comment's retry, or any share of the item, answers
  `Shared`, `Published` or `Current`.
- After a committed checkpoint the tab reloads its comments (fact 28); if
  discovery is `pending`, a background `IndexRefresh` is queued first and
  the header shows "Updating the list…" (RFC desktop IA :204-205).
- A committed comment checkpoint removes the context from
  `CreatedContexts` (R-4).

### Body draft separation

Posting reads only the composer's text. `comment add` stages only
`.manyhands/comments/<item>/<comment>.md` (RFC Git workflow :134), so
unsaved body text is never committed or shared (contract 2, A12).

## Part E: Keyboard, Privacy And Native Evidence

- **Tab order:** tab strip → header actions (…, Edit, Share) → chips →
  toolbar → page → discussion heading, Back to top, then each comment and
  its Reply → open reply composer → new composer. Inside a composer Tab
  indents; F6 leaves it (decision 19).
- **Privacy:** comment sentinels never reach operation entries, the quit
  dialog, notifications, dialog messages or logs.

### Native checklist (Linux Wayland, release build, product owner)

Setup: an SSH remote the owner controls; a protected key selected **before**
the remote is configured (K-4); the remote selection committed (K-2); a
second clone changed with **plain `git`** (commit, push), which is an
external change, not a second front end. Every journey is keyboard-only.

1. Smoke.
2. Discussion: two roots, two replies to one root, a reply to a reply,
   Ctrl Enter, Esc with and without text, Ctrl Shift M in a composer,
   kill the process mid-reply and restore; the desktop's **Rebuild
   index**; reopen; order and parents hold.
3. Share: item Share, Share main, already current, an incoming change
   from the second clone merged.
4. Offline: remote unreachable; post (saved, not shared); Retry sharing
   once reachable; post again offline, quit after it has failed (owed),
   relaunch, Resume; post once more and quit **while** it is sending
   (K-10): after relaunch Share publishes it; one comment file and one
   checkpoint per comment throughout.
5. Credentials: first share prompts; Not now; a second request does not
   prompt; Unlock… prompts once; wrong passphrase (K-7); a host removed
   from the pins → Check host → Approve → Retry.
6. Responsiveness: a stalled remote; type in the body and a composer;
   Stop at a safe point; Share again.
7. Conflict: the second clone changes the same ticket line with `git`;
   Share shows the preserved conflict banner; the body is unchanged.

The worktree-switch step of the PRD journey is G1's (Wave line 72:
"ordering/rebuild recovery in G1").

## Privacy

Comment text lives in `CommentBody`, the composer and the draft store
only. Labels are fixed (fact 21).

## Testing

| Target | What |
| --- | --- |
| `services/requests_tests.rs` | Intents build their mutations; labels fixed; `stops_at_safe_points`. |
| `services/worker_tests.rs` | An accepted `RepoSync` (with `publish_over_ssh`, held by `set_request_hook_for_testing`, fact 16) is `Signalled` and its token cancelled; a local job stays `TooLate`; `give_up` sends `cancel_request` then the same request and reports its answer; `submit_with_request_id`; each finished attempt carries its record state (Accepted, Finished, Unknown). |
| `services/prompts_tests.rs` | A declined key answers `Cancelled` without a prompt; clearing it prompts again. |
| `services/submissions_tests.rs` | Refuses while posting; a post's request ID is its comment ID; retry keeps it; restart moves to a new comment ID and re-targets the draft (P13); a `Finished` event with no tab open updates the submission (services harness). |
| `model/publication_tests.rs` | Every row of both tables, in precedence order, from constructed envelopes. |
| `services/drafts/comments_tests.rs` | Caller's comment ID kept; per-thread drafts; a flush happens after the idle delay (advancing the test clock); adopt after a dropped registry; retire. |
| `view/item/share_tests.rs` | Share states and guard; stage text from `Operations::apply` of constructed `ProgressEvent`s; each `SyncOutcome` row from constructed envelopes; routes; the conflict banner; the repository detail's Share main. |
| `view/passphrase_tests.rs` | New subtitle and Not now, prompts raised through `WorkerHandle::provider` (fact 30). |
| `view/item/composer_tests.rs` | Events, states, keys, discard prompt, labels and footer by remote and kind. |
| `view/item/posting_tests.rs` | Real library, no remote: Save comment locally. `publish_over_ssh`: one file, one checkpoint after two Retry sharing presses (U-2). A failed checkpoint keeps text. Guard. Dirty body never committed. Adopted draft already listed is retired. Constructed envelopes for every other outcome row. Stale outcomes (contract 8): an outcome landing after new typing or a tab switch never replaces newer text. No implicit publication (contract 1): navigation, Save, Done, closing and quitting submit no comment or sync intent. |
| `view/keyboard_tests/discussion.rs` | The keyboard journey with no remote. |
| `view/editing_privacy_tests.rs` | Comment sentinels. |

A7 and A13 cannot reach a real prompt or host check in the bin (fact
17); their desktop tests use constructed envelopes and provider-raised
prompts, and native §5 proves the journey.

## Named Edits To Pre-Existing Tests

| E | Test | Edit | Owner |
| --- | --- | --- | --- |
| E-1 | `model/item_tests.rs::the_unavailable_actions_are_the_designs` | `LATER` loses Comment and Share. | Part 0 |
| E-2 | `view/item_tests.rs::a_nonconforming_entry_opens_by_path_with_its_problem_and_repair_offered` | The filter no longer removes "Comment". | Part 0 |
| E-3 | `view/item/header_tests.rs::share_and_the_primary_action_are_disabled_with_their_note` | Only the primary action; renamed `the_primary_action_is_disabled_with_its_note`. | Part 0 |
| E-4 | `view/item/header_tests.rs::a_nonconforming_item_shows_needs_fixing_with_repair_offered` | `later-action` indices shift by one. | Part 0 |
| E-5 | `view/item/header_tests.rs::an_inspect_tab_of_a_conforming_item_needs_no_fixing` | The later rows `[SHARE, CLOSE_TICKET]` become `[CLOSE_TICKET]` (wording corrected in Task 0). | Part 0 |
| E-6 | `view/item/header_tests.rs::tab_order_runs_strip_header_chips_page` | One later action; Lane B adds Share after Edit. | Part 0, Lane B |
| E-7 | `view/item/discussion_tests.rs::comment_is_disabled_with_its_note` | Deleted; Lane D adds `the_new_comment_composer_follows_the_threads`. | Part 0 |
| E-8 | `view/drafts_tests.rs::each_row_reads_its_state_and_offers_what_its_policy_allows` | A comment row offers Open. | Lane A |
| E-9 | `model/drafts/rows_tests.rs::a_comment_draft_is_never_restored_in_d2` | Body-path restore still refused; Open offered. | Lane A |
| E-10 | `model/operations_tests.rs::resume_is_offered_for_a_local_operation_and_not_for_a_synchronization` | Resume offered for a remote sync row; still not for a local-context sync. | Lane B |
| E-11 | `view/item/discussion_tests.rs::tab_runs_through_each_comment_and_reveals_its_caret` | Tab reaches each comment's Reply after the comment. | Lane D |
| E-12 | `services/requests_tests.rs`: `every_intent`, `variant`, `changes_items_names_the_intents_that_can_change_items`, `every_intent_builds_its_own_command` | The intent lists grow from 29 to 32 with the three new intents; the "kind of its own" assertion is kept, which is why Share item and Share main have separate `JobKind`s (owner, 2026-11-02, found by Task 0's sweep). | Part 0 |
| E-13 | `view/keyboard_tests/frame.rs::the_strip_header_chips_and_page_run_in_tab_order` | Share is a tab stop after Edit, before the first chip, as E-6 (owner, 2026-11-02, found by Lane B Task 8). Applied by Lane B in Part E's file. | Lane B |
| E-14 | `view/drafts_tests.rs::a_comment_rows_open_opens_its_item` | Open itself still claims nothing; the opened tab then adopts the draft (owner, 2026-11-02, Task 12). | Lane D |
| E-15 | `view/keyboard_tests/frame.rs::the_discussion_is_reached_by_its_key_walked_and_left` | One more Tab after each comment (its Reply) and the matching Shift-Tabs, as E-11; adjusted for the collapsed new-comment start (decision 22). | Lane D |

The round-1 source review swept `src/desktop` for these terms and found
no other breakage; Task 0 sweeps again after merging main.

## Document Amendments

Applied in the approval commit:

1. **Wave 03, D4 section:** "Amended 2026-11-02 by D4": decisions 1–4 and
   the confirmed ones among 5–15.
2. **Desktop IA RFC:** actions table names Share, Share main, Retry
   sharing, Reply and sync, Save reply locally (decision 7); the
   publication-pending marker is session-only until a library read exists
   (decision 14, K-1).
3. **Desktop IA RFC, D3 amendment R-59:** Escape-to-header applies to the
   body editor; in a comment composer Escape cancels
   (`04-comments.md:51-53`).
4. **API audit** rows D24 (merged) and D25 (no dialog in D4).
5. **AGENTS.md**, Dogfooding Workflow: one line recording decision 1's lane
   exception for D4.
6. **Ticket** `01M4CC0VN1KTQFSVR99378TDHV`: `deps` gains C4
   `01M4CC0VMWEQQ9G4W0DAG7ADPY`.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | Token cancel after acceptance only for intents that stop at safe points (including `OperationResume`); no `cancel_request` for running jobs. | Stop does nothing; Task 2's test pins the token. |
| R-3 | Unlock… = clear the declined mark, then Retry under the same request ID. | Several locked keys prompt in turn. |
| R-4 | A committed comment checkpoint removes the context from `CreatedContexts`. | Leaving could release a context holding an unpublished comment. |
| R-8 | Before the first post of an adopted draft, a refreshed listing is checked; the post is then the same request (P4, R-16). `MaybeSaved` offers Check again; Post as a new comment only when no record exists and the listing lacks the comment (P5). | If the checks miss a saved comment, a second comment could be posted; P4/P5 tests pin each check. |
| R-9 | Footer copy by remote and kind. | Copy. |
| R-10 | A restored comment draft's load normalization is not an edit. | A spurious flush. |
| R-12 | When the post wrote its file or its record is accepted (`record_accepted`), a same-request Retry is offered, with Discard and Copy text (P7). When nothing was written and the record is finished, Edit text is allowed and posts under a new comment ID (P6). | Wrong: a finished failure replays forever, or an orphaned pending post duplicates the comment later. |
| R-16 | A post's request ID is its comment ID; the draft is flushed before Submit (fact 14c). | If request records are lost (they are not a journal), the same request is treated as new and a bound comment collides as `internal_error`, which P5 handles. |
| R-14 | The worker treats any progress stage after `Accepted` as acceptance (fact 10). | Stop would not show on a retry. |
| R-15 | Stop is shown only when the repository has a publication remote. | A no-remote post would show a Stop that does nothing. |
| R-13 | A declined key is not prompted again until Unlock…. | A second request gets `unlock_required` without asking. |
| R-17 | (Task 3 review, 2026-11-02; the design was silent.) A finished record only replays, so no outcome offers a same-request resend (Retry sharing, Unlock and resend, Retry) when its record is Finished, except a share's transient codes the library still retries (`busy`, `poll_yielding`, `remote_unavailable`, `transport_unavailable`, `registration_pending`). Comment rows 6, 7, 8 and 9c then offer their fix step (if any) and Share; row 14 (written, uncommitted, its journal row final) offers Discard, Copy text and Check again, never Edit text; a share offers its fix step and Share. Outcomes carry the record they were classified under, and after a Give up no outcome offers Give up again (P14). | A dead-end button that replays the same answer forever. |
| R-18 | (Lane A Task 6 review, 2026-11-02.) `retire_comment` returns `Task<Result<(), DraftErrorKind>>` that resolves after the delete lands, flushing any pending edit first and retiring a stored-only draft at its claimed revision. P13 re-targets as: begin the new draft, await it, retire the old one, await it, then Submit. The P9 drain hook may detach it. Open for Task 13's design checkpoint: whether `Submission` records the posted revision and passes it to retirement (sturdier than relying on flush-before-submit and a read-only composer), and that P12's "submitted" state comes from `show_request` (a record exists), since the draft schema has no such flag. | Relaunch could adopt the old comment ID, or a posted comment's draft could stay listed. |
| R-19 | (Task 13 review, 2026-11-03; derived from R-17 and the no-dead-end rule.) A post's composer follows its submission: while `Submissions` has the post in flight (including a Retry from the operation area) the composer shows Posting, and Check again, Edit text's re-target and Post as a new comment refuse. "May already be saved" (row 13, and Check again's unreadable-record result) also offers Copy text and Discard, with P7's note, beside Check again. Decision 24 retires a listed comment's draft only when the listed body equals the draft byte for byte; otherwise it is a mismatch (Copy text, Discard). | A second comment from an operation-area Retry; text trapped behind Check again; a draft retired against different text. |

R-2, R-5, R-6, R-7 and R-11 of revision 1 became owner decisions 8, 10,
9, 13 and 14 (Cycle).

## Known Limits

- **K-1.** No read reports unpublished comments; the chip is session-only
  (decision 14).
- **K-2.** `01M669FJR27FRHCNPWAMD6VH4V`: a fresh `remote select` leaves the
  index dirty; setup commits it.
- **K-3.** `01M67BN4C1SY2HQBGNE9NBYMW8`: a comment's host stop has no host
  data; Check host opens the root's Hosts page.
- **K-4.** `01M66HFV44YXNQ7ANJVA16TQ1H`: select the key before configuring
  the remote.
- **K-5.** `01M4R19PC0T6ZPKM39X9DF26HX`: a share or post that stopped with an
  error holds the repository's reservation; others answer `busy` until
  Retry or Give up. A `busy` answer names no holder (fact 14); the desktop
  finds one in this session, else points to the pending operations. After
  a restart, U-5 settles the way out.
- **K-6.** `01M4H33R34Z7C7EEKTY1ZCT950`: a pending remote conflict cannot be
  abandoned.
- **K-7.** `01M5E6DYJ6GN7DMW4424TH3V78`: a wrong passphrase reads as an
  unreachable remote; the dialog's inline error cannot be shown.
- **K-8.** One worker: a share waiting on the network or a prompt delays
  queued saves (decision 12).
- **K-9.** A context made by posting is not released by the desktop.
- **K-10.** Quitting while a post or share is sending stops it at a safe
  point (fact 14a); a stopped publication is final, so after relaunch no
  Resume row or chip marks the comment, and Share publishes it (decision
  15). D6 owns shutdown. After such a quit, a relaunch may show the
  comment as "being posted when Manyhands closed" until Retry replays
  its saved result (Task 13 checkpoint, Q6).
- **K-11.** The reply composer's avatar shows a placeholder: the item
  tab has no source for the user's identity. An adopted reply's
  "Replying to" names the root's author, since the pressed comment is
  not stored in the draft.
- **K-12.** A comment draft whose adoption fails (for example
  `OpenElsewhere`) is adopted again only when the tab reopens.
- **K-13.** A share holding the repository is not named once its
  operation entry is dismissed ("Busy repositories").
- **K-14.** A comment draft whose item is gone still offers Open on the
  Drafts page; the tab shows its error and cannot adopt it (Discard and
  Export still work).
- **K-15.** After a restart a comment whose publication is still pending
  shows no Resume: the library lists its synchronization without
  `next_action: resume` (`read/status.rs:316-331`). Share on its item
  publishes it. Decisions 11 and 14 hold for shares, not comments, until
  library ticket `01M6GHWZ5CXXFC7YB3EVJ6Q4A7`.
- **K-16.** If a resumed share's synchronization is not stopped and its
  transport then fails, the repository's sharing reservation stays held
  (listed as reconciling, with no Resume or Give up). In the same session
  the share's Give up frees it; after a restart nothing in the desktop
  can, the reservation never expires, and every other share and post in
  that repository answers busy. Recovery: `manyhands-cli operation resume
  <operation-id>` once the remote is reachable. Library ticket
  `01M6GHWZS03WCADS1A1JXV4PMP`.

## Alternatives Rejected

- Serial execution; lanes in one worktree; a plain-text composer; building
  Activity and Needs attention (decisions 1–3).
- Mapping `occupied_path` to "already saved" (revision 1): it can discard
  unsaved text (fact 13).
- Retry sharing after a cancellation under the same request (revision 1):
  a cancellation is final (fact 12).

## Risks

| Risk | Control |
| --- | --- |
| C5 merges mid-Cycle and moves sync/resume code | Merge main whenever it moves; re-check facts 4–17. |
| Lane merge conflicts | Ownership table; out-of-row changes go through the orchestrator. |
| Zorite quirks in a second editor | Lane C reuses D3's editor functions and tests. |
| Real-host behaviour untestable in the bin | Native §§3–7; constructed envelopes for every outcome row. |
| Three cold builds | Accepted with decision 1. |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Round 1, source and feasibility | 5 Blocking, 10 Important, 7 Minor | — | All applied in revision 2 or turned into decisions 8–14. | Report kept in the planning comment. |
| Round 1, contract and consistency | 3 Blocking, 18 Important, 1 Minor group | — | All applied in revision 2 or turned into decisions. | Same. |
| Source B1 | A restored draft collides as `internal_error`, not `occupied_path` | Ruling R-8 | Listing check before first post; Post as a new comment. | Fact 13. |
| Source B2 | `occupied_path` means different content | Settled | `NotSavedConflicting`, text kept. | Fact 13. |
| Source B3, contract I7 | No real prompt in the bin | Settled | Provider-raised prompts and constructed envelopes; native §5. | Fact 17. |
| Source B4 | `hold_job` cannot hold past acceptance | Settled | `set_request_hook_for_testing` with `publish_over_ssh`. | Fact 16. |
| Source B5 | Task 4 leaves dead code | Settled | `COMMENT` and `NoteAt::BelowOnPage` removed; `SHARE` marked. | Fact 24. |
| Contract B1, source I7 | Cancellation is final | Settled | `SavedStopped` → Share. | Fact 12. |
| Contract B2, source I10 | Finished failure replays | Ruling R-12 | Retry only when offered. | Fact 12. |
| Contract B3 | CLI as second actor | Settled | Plain `git`; desktop Rebuild index. | Wave track rules 2, 4. |
| Source I8 | Give up could cancel another request | Decision 8 | Own request only; `WaitingFor` (superseded by round 2 B1). | Fact 14. |
| Source I9 | Request ID lost with the composer | Settled | `Submissions` registry. | Fact 18. |
| Source I11, I12 | Lane A/B files outside rows | Settled | Rows extended. | Facts 22, 25. |
| Contract I8, I9 | Lane mechanics | Settled | One merge rule; orchestrator-only bookkeeping. | AGENTS.md. |
| Contract I13 | Esc and Tab in composers | Settled | Part 0 keys; Amendment 3. | RFC desktop IA :182-185. |
| Contract I21 | Second request re-prompts | Ruling R-13 | Declined keys. | Runtime RFC :110-113. |
| Round 2, scoped re-review with adversarial brief (`6ed12ae6`) | 2 Blocking, 7 Important, 9 Minor | — | Revision 3. | Planning comment. |
| Round 2 B1 | `busy` names no holder; fact 14 was false | Settled | Fact 14 corrected; `WaitingFor` dropped; Busy repositories. | `outcome.rs:843-850, 983-990`. |
| Round 2 B2 | R-8 could post twice after a crash | Settled | Required outcomes P4, P5; drain retirement. | `recovery.rs:287-297`. |
| Round 2 I1 | Not now holds the reservation | Decision 8 widened | Give up for any owed safe-point request. | Fact 14. |
| Round 2 I2 | Give up on a deleted branch led back to it | Settled | No Share after BranchDeleted. | Fact 4. |
| Round 2 I3 | Submissions with no tab | Settled | Updated from the drain (P9). | `services/mod.rs:228-300`. |
| Round 2 I4 | A stop after the push called published "not shared" | Settled | Published rows first. | `bind/comment.rs` `saved`. |
| Round 2 I5 | Edit text could orphan a written post | Ruling R-12 | P6/P7. | `bind/comment.rs:556-602`. |
| Round 2 I6 | Quit stops a running publication | Decision 15, K-10 | Native §4. | Fact 14a. |
| Round 2 I7 | Reply composer moved while posting | Settled | Move disabled; outcomes matched by comment ID. | — |
| Round 3, scoped with adversarial brief (`0fa796dd`) | 1 Blocking, 5 Important, 6 Minor | — | Revision 4 (not independently re-reviewed; skill rule for a third round). | Planning comment. |
| Round 3 B1 | P4/P5 could not match operations to comments | Ruling R-16 | Durable request ID = comment ID; P4/P5 restated. | Facts 14c, 14e. |
| Round 3 I1 | Give up's re-send may not end cancelled | Decision 8 reworded | Render its answer. | Fact 14d. |
| Round 3 I2, I5 | Outcomes need record state; Give up too wide | Settled | `record_accepted`; rows 9a/9b. | Fact 10. |
| Round 3 I3 | Holder lookup named non-holders | Settled | Narrowed; "may be waiting". | Fact 14. |
| Round 3 I4 | Owed unsaved post trapped text | Ruling R-12 | Discard and Copy text. | — |
| Round 4, narrow review of revision 4 (`2fdd1809`) | 2 Blocking, 6 Important, 3 Minor | — | Recorded as required outcomes P11–P18 and row-order fixes (rows 0, 9c, 13); the mechanism is left to Task 13's design checkpoint. Not re-reviewed. | Planning comment. |
