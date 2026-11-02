---
title: "Wave 03 D4 Desktop Discussion And Deliberate Synchronization Design"
date: 2026-11-02
status: "draft"
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

"Fact n" below refers to Baseline Facts. Paths are relative to the
repository root; `desktop/` means `src/desktop/`.

## Baseline Facts At `00bef3da`

Facts marked **(verified)** were read in the source during planning.
Facts marked **(research)** come from read-only research agents and are
re-checked in Task 0 before any code depends on them.

**Library boundary (consumed, not changed).**

1. **(verified)** `RepositoryService::execute<P: SessionCredentialProvider>(&self, call: MutationCall, credentials: &mut SessionCredentials<P>, progress: &mut dyn ProgressSink, cancel: &CancellationToken) -> Envelope<MutationDataDto>` is synchronous on the caller's thread (`src/repository/mutation/execute.rs:41-45`).
2. **(research)** `Mutation::CommentAdd(CommentAddInput { root, kind, id, comment_id, parent_id, body, created_by })` (`src/repository/mutation/bind/comment.rs:84`). The caller supplies `comment_id`; `parent_id: None` is a root comment; `created_by: None` takes the Git identity (C4 decision 9). No confirmation (`mutation/mod.rs:510-516`).
3. **(verified)** `comment add` provisions or reuses the item's editing context itself: `checkpoint_comment_locally` calls `prepare_context_unlocked` with `ContextIntent::Edit` (`src/repository.rs`, the block after "comment submission requires an edit target").
4. **(verified)** Standing (R-34 of C4, `bind/comment.rs:16-31`): a published comment, or one with no remote, is finished; a pending publication is owed and "the same request publishes the comment without writing it again"; a final child (cancelled) is finished; `remote_branch_deleted` is owed, and "`request cancel` followed by the same request ends it".
5. **(verified)** `CommentMutationDto { type, item_id, comment_id, parent_id, created_by, synchronization_id, publication: PublicationEffect, pending_reason: Option<CommentPendingReason>, conflict: Option<ConflictNextDto> }` (`mutation/dto.rs:611-632`). `CommentPendingReason` is `no_publication_remote | recovery_required | synchronization` (`dto.rs:592-597`).
6. **(verified)** `PublicationEffect` is `not_requested | published | current | pending`; `CheckpointEffect` is `not_requested | unchanged | committed | pending`; `DiscoveryEffect` is `not_requested | current | pending` (`src/results.rs:63-81`).
7. **(research)** `Mutation::ItemSync(ItemSyncInput { root, kind, id })` and `Mutation::RepoSync(RepoSyncInput { root })` (`bind/sync.rs:94-106`) merge (`MERGING`, `sync.rs:145-147`). A conflict answers `recovery_required` with `MutationDataDto::SynchronizationConflict(SynchronizationConflictMutationDto { type, conflict: ConflictNextDto { operation_id, next_action } })` (`dto.rs:921-957`).
8. **(verified)** A synchronization that stops with an error between safe points keeps the repository's reservation and its record stays accepted; "the same request takes it up again, and `request cancel` followed by the same request ends it" (`bind/sync.rs` module doc).
9. **(verified)** `ProgressStage` is `accepted, observing, writing, connecting, fetching, updating_local, pushing, verifying, indexing, stopping` (`mutation/progress.rs:83-96`). Cancellation is honoured after acceptance only at a synchronization's safe points; a local step cannot be stopped once accepted (`progress.rs:8-13`, research).
10. **(research)** `cancel_request(&self, request_id: RequestId) -> Envelope<CancelDto>` works across threads and processes (`progress.rs:400`).
11. **(research)** Transport error mapping (`mutation/outcome.rs:109-154`): `UnlockCancelled`/`ProviderUnavailable` → `unlock_required`; `UnlockFailed` → `unlock_failed`; `NoSelectedKey` → `no_selected_key`; key problems → `key_unavailable`/`key_rejected`; `HostApprovalRequired` → `host_approval_required`; `HostReplacementRequired` → `host_replacement_required`; `HostTrustChanged` → `external_change`. Host codes carry `SynchronizationMutationDto { host, port, presented_*, pinned_* }` for syncs, but not for `comment add` (Known Limit K-3).
12. **(research)** Replay (`execute.rs:34-39, 199-229`): a finished record replays its stored result; an accepted record is re-entered; the same ID with different input is `request_mismatch`.
13. **(research)** `list_comments` returns roots and replies in canonical order flattened with `depth`, malformed files after the threads, from the effective copy (the context if one exists), and only what the index has seen (`read/comments.rs:216-270`).
14. **(research)** No read DTO carries a comment's or an item's publication state, and none counts unpublished work. Publication is known only from a mutation result.

**Desktop (changed).**

15. **(verified)** One mutation worker thread owns the process's only `SessionCredentials` (`desktop/services/worker.rs`). `WorkerEvent::Finished(DesktopRequestId, Box<Envelope<MutationDataDto>>)` carries the whole envelope (`worker.rs:51-60`). The cancellation token reaches `execute` (`worker.rs:621-631`).
16. **(verified)** `WorkerHandle::cancel` returns `Removed` for a queued job, `Signalled` only for a running job with `accepted: false`, and `TooLate` otherwise (`worker.rs:437-454`). `running.accepted = true` is set by the progress forwarder on `Accepted` (`worker.rs:793`).
17. **(verified)** A `busy` answer is re-admitted under the same request ID after 100/200/400 ms unless the token was cancelled (`worker.rs:640-668`, `services/mod.rs` `BUSY_RETRY_DELAYS`).
18. **(verified)** `Intent` (`desktop/services/requests.rs:43-228`) has no `CommentAdd`, `ItemSync` or `RepoSync` variant.
19. **(verified)** The operation area's Cancel recovery step calls `worker.cancel(id)` (`desktop/view/workspace/requests.rs:323-325`), which does nothing for a finished request (fact 16).
20. **(verified)** `resume_offer` shows Resume unavailable for any synchronization row (`desktop/model/operations.rs:614-626`), per D1 ("Synchronization resume stays unavailable until D4").
21. **(verified)** `LATER = [COMMENT, SHARE, APPROVE_AND_MERGE, CLOSE_TICKET]` (`desktop/model/item.rs:47`). `header_later` builds Share and the kind's primary action (`desktop/view/item/header.rs:167-176`). The discussion draws a disabled Comment with `later_action(... COMMENT ...)` (`desktop/view/item/discussion.rs:262-269`).
22. **(verified)** The draft registry's sessions are keyed by `DraftId`; tabs attach to one draft each (`desktop/services/drafts.rs`, `services/drafts/restore.rs:62-90`). `begin(target, base, current)` creates a draft off the foreground and returns its ID (`drafts.rs:347-381`); `edit` schedules a flush (`drafts.rs:384-402`).
23. **(research)** Comment drafts are excluded on purpose: `stored_for` filters `target.comment.is_none()` (`drafts.rs:313-337`); restore refuses them (`drafts/restore.rs:54, 348-355`); save refuses them (`model/drafts/save.rs:93-99`); the Drafts page says `COMMENT_LATER` (`view/drafts.rs:60`).
24. **(verified)** The library draft schema already carries comments: `DraftTarget { repository, item_kind, item_id, comment: Option<CommentTarget { comment_id, parent_id }> }`, `DraftFields::Comment { body }` (`src/drafts/schema.rs`, used in `model/drafts/save_tests.rs:283-291`).
25. **(verified)** Release on leave requires that the context be in `CreatedContexts` with no save committed since, and that the store list no draft of the item, "a comment's draft on the item counts" (`desktop/services/release.rs:1-30`).
26. **(verified)** `ItemId::generate()` exists (`src/canonical.rs:19`).
27. **(research)** `ItemPane` is one entity whose fields, `new()` and `render()` live in `desktop/view/item.rs` (461 lines); every feature adds a field and a `sync_*` call there. `keys.rs` holds all bindings and has no Ctrl Enter.
28. **(research)** Desktop tests are unit tests in the `manyhands` bin under `desktop-test-support`; CI runs them with `cargo test --locked --no-fail-fast --features desktop-test-support --bin manyhands` (`.github/workflows/build.yml:128-129`). The transport cannot be initialized in them, so no desktop test reaches a host (D1 Cycle :259).
29. **(research)** `test_support/fixtures.rs::publish_over_ssh` configures `ssh://git@example.invalid/...` and contacts nothing; `pin_host` pins a host for tests.

**Unverified, checked in Task 0.**

- U-1. What `item sync` answers for an item with no editing context.
- U-2. The exact code a comment's publication answers in a desktop test, where the transport is uninitialized (`RuntimeUninitialized`). A4's tests assume it is a publication failure after the checkpoint, with `pending_reason: synchronization`.
- U-3. Whether `changes_items` on an intent already triggers the item tab's comment reload, or the tab must ask.

## API Audit Refresh (Entry-Gate Item 3)

Rows D23 and D24 of `docs/research/wave-03-api-audit.md` name D4; the
"C4 delivery reconciliation (2026-10-30)" section lists the bindings above.
Task 0 records in the ledger that each binding named in facts 2–12 is on
origin/main at the merged base, with its file and line.

## Execution Lanes (Decision 1)

```
Part 0 (serial, ticket worktree)
   │
   ├── Lane A: comment drafts ─────────┐
   ├── Lane B: share, progress, recovery ┤   (parallel, lane worktrees)
   └── Lane C: the composer widget ─────┘
                                        │
                     Lane D: discussion integration (serial, ticket worktree)
                                        │
                     Part E: keyboard, privacy, native checklist, gate
```

**Why this split.** The lanes share only Part 0's seams. Each lane owns a
disjoint set of files (table below); none edits `view/item.rs`,
`keys.rs`, `services/requests.rs`, `services/mod.rs` or `worker.rs`, which
Part 0 prepares for all of them. Lane C builds the composer as a
standalone entity with an event interface, so it needs neither Lane A's
drafts nor Lane B's outcomes; Lane D wires the three together.

**Lane mechanics.**
- Each lane runs in its own worktree under the gitignored `.worktrees/`
  directory of the main checkout, on a local branch `d4/lane-a`,
  `d4/lane-b`, `d4/lane-c`, created from the ticket branch at the Part 0
  checkpoint. Lane branches are not pushed and are not under
  `manyhands/ticket/`, so Manyhands discovery never sees them.
- Each lane task gets a fresh implementer and a fresh reviewer, as in D3.
  Implementers never dispatch subagents. The orchestrating session merges
  each lane into the ticket branch when its lane checkpoint passes
  (`git merge --no-ff d4/lane-x` in the ticket worktree), runs the merged
  targeted tests, and deletes the lane worktree and branch.
- Lanes merge in the order they finish. A lane that finishes second merges
  the ticket branch into itself first, re-runs its targeted tests, then is
  merged.
- Each worktree has its own Cargo target directory. A cold desktop build
  costs time and disk per lane; that is the price of isolated builds.
- `origin/main` is merged into the ticket branch at the Part 0 checkpoint
  and before Lane D, not into lane branches.

**File ownership.**

| Owner | Files |
| --- | --- |
| Part 0 | `services/requests.rs`, `services/requests/build.rs`, `services/requests_tests.rs`, `services/mod.rs`, `services/worker.rs`, `services/worker_tests.rs`, `test_support/delaying.rs`, `model/mod.rs` (one `mod` line), `model/publication.rs` (new), `model/publication_tests.rs` (new), `model/item.rs`, `model/item_tests.rs`, `keys.rs`, `view/item.rs`, `view/item_tests.rs`, `view/item/header.rs` (removing Share from the later actions only), `view/item/header_tests.rs`, `view/item/discussion.rs` (removing the disabled Comment only), `view/item/discussion_tests.rs`, `view/workspace/requests.rs`, `view/item/share.rs`, `view/item/composer.rs`, `view/item/posting.rs` (three new stub files) |
| Lane A | `services/drafts.rs` (one `mod` line), `services/drafts/comments.rs` (new), `services/drafts/comments_tests.rs` (new), `services/drafts/restore.rs`, `model/drafts/rows.rs`, `model/drafts/rows_tests.rs`, `view/drafts.rs`, `view/drafts_tests.rs` |
| Lane B | `view/item/share.rs`, `view/item/share_tests.rs` (new), `view/item/pill.rs`, `view/item/banners.rs`, `view/item/header.rs`, `view/item/header_tests.rs`, `view/sidebar.rs`, `model/operations.rs`, `model/operations_tests.rs`, `view/operations.rs`, `view/operations_tests.rs`, `view/passphrase.rs`, `view/passphrase_tests.rs` |
| Lane C | `view/item/composer.rs`, `view/item/composer/*` (new), `view/item/composer_tests.rs` (new), additive changes to `view/editor.rs` |
| Lane D | `view/item/posting.rs`, `view/item/posting_tests.rs` (new), `view/item/discussion.rs`, `view/item/discussion/rows.rs`, `view/item/discussion_tests.rs`, `model/discussion.rs`, `view/item/reads.rs`, `view/item.rs`, `services/created_contexts.rs` |
| Part E | `view/keyboard_tests/discussion.rs` (new), `view/editing_privacy_tests.rs`, `docs/plans/…-checklist.md` (new), ledger, ticket |

Lane B and Part 0 both edit `view/item/header.rs` and its tests, at
different times: Part 0 removes Share from the later actions, Lane B adds
the real Share. A file outside its owner's row stops the task.

## Part 0: Seams

### Intents

`services/requests.rs` gains three intents, built in `requests/build.rs`:

| Intent | Mutation | Lane | `job_kind` | `changes_items` | Stops at safe points |
| --- | --- | --- | --- | --- | --- |
| `CommentAdd { root, kind, id, comment_id: ItemId, parent_id: Option<ItemId>, body: CommentBody }` | `CommentAdd(CommentAddInput { …, created_by: None })` | User | `JobKind::Comment` | yes | yes (its publication) |
| `ItemSync { root, kind, id }` | `ItemSync(ItemSyncInput)` | User | `JobKind::Synchronize` | yes | yes |
| `RepoSync { root }` | `RepoSync(RepoSyncInput)` | User | `JobKind::Synchronize` | yes | yes |

- `CommentBody` is a newtype over `String` that does not derive `Debug`
  and whose label is fixed text (contract 10).
- `JobKind::Comment` and `JobKind::Synchronize` are new; `DelayingBackend`
  can hold them as it holds the others.
- `Intent::stops_at_safe_points(&self) -> bool` is true for the three.

### Safe-point stop (R-1)

`Running` gains `stops_at_safe_points: bool`, set from the intent when the
job starts. `WorkerHandle::cancel` signals the token when the running job
is not accepted (as today) **or** is accepted and stops at safe points.
It returns `Signalled` in both cases. A local command stays `TooLate`
after acceptance (fact 9). The library answers at its next safe point, or
completes the current atomic step and says so in its effects.

### Give up on a held reservation (R-2)

The Cancel recovery step (fact 19) gains a second path: when the request
is finished, the workspace asks the worker to `give_up(id)`. The worker
calls `service().cancel_request(request_id)` on its own thread, then
re-admits the same request with the same intent, which ends `cancelled`
(fact 8, fact 4). This is C4's documented way out of a held reservation
and of `remote_branch_deleted`.

### Outcomes (`model/publication.rs`)

Pure functions over an envelope, unit-tested with constructed envelopes:

- `comment_outcome(&Envelope<MutationDataDto>) -> CommentOutcome`:

| Envelope | `CommentOutcome` |
| --- | --- |
| `effects.checkpoint` not `committed` and no `Comment` data | `NotSaved { code }` (checkpoint failed or refused) |
| `Comment` data, `publication: published` or `current` | `Shared` |
| `publication: pending`, `pending_reason: no_publication_remote` | `SavedHere` |
| `publication: pending`, `conflict: Some(next)` | `SavedConflict { next }` |
| `publication: pending`, other reason | `SavedNotShared { code, reason }` |
| `code: occupied_path` | `AlreadySaved` (R-8) |

- `sync_outcome(&Envelope<MutationDataDto>) -> SyncOutcome`: `Published`,
  `Current`, `Pending { code }`, `Conflict { next }`, `Stopped { completed:
  bool }` (from `outcome: cancelled` and whether the effects show
  completed work), `NeedsUnlock`, `NeedsKey { code }`, `NeedsHost { data:
  Option<SynchronizationMutationDto> }`, `Failed { code }`.

Lanes B and D render these; neither reads an envelope's fields directly.

### Removing the placeholders

`LATER` becomes `[APPROVE_AND_MERGE, CLOSE_TICKET]`. `header_later` no
longer lists Share, and the discussion no longer draws a disabled Comment.
Until Lanes B and D land, the header has no Share and the discussion has
no composer. Named edits E-1 to E-7.

### Slots in `ItemPane`

`ItemPane` gains three fields, each a type defined in its own new file
with a `Default` and an empty `render`:

- `share: ShareState` (`view/item/share.rs`, Lane B);
- `composers: Composers` (`view/item/composer.rs`, Lane C defines the
  widget, Lane D the placement);
- `posting: Posting` (`view/item/posting.rs`, Lane D).

`render` calls each slot once, in the design's order. Lanes fill the
files; they do not touch `view/item.rs` again until Lane D.

### Keys

`keys.rs` registers, in a `CommentComposer` key context: `PostComment`
(`secondary-enter`) and `CancelComposer` (`escape`). In the item context:
`ShareItem` (no default key; reached by Tab). The actions' handlers are
stubs until their lane.

## Lane A: Comment Drafts

`services/drafts/comments.rs` adds to `DraftRegistry`:

- `begin_comment(item: ItemKey, parent: Option<ItemId>, body) -> Task<Result<DraftId, DraftErrorKind>>`:
  generates the comment ID with `ItemId::generate()` (fact 26) and calls
  `begin` with `DraftTarget { comment: Some(CommentTarget { comment_id,
  parent_id }) }` and `DraftFields::Comment { body }`, no base. **The
  comment ID is allocated once, here, and lives in the draft** (contract
  3).
- `comment_drafts(item: &ItemKey) -> Vec<(DraftId, CommentTarget)>`:
  this process's sessions and the store's unclaimed listing for the
  item, one per `(item, parent)` thread (the newest wins; others stay on
  the Drafts page).
- `adopt_comment(id) -> Task<…>`: claims a stored comment draft as a
  session, as restore does for a body draft.
- `retire_comment(id)`: retires the draft (D2's `retire`), used after a
  committed checkpoint.
- `discard_comment(id)`: D2's discard.

`edit` and flushing are unchanged (fact 22): a comment session flushes
within 1 s like a body session. `stored_for` still ignores comment drafts,
so the body's restore path is untouched.

The Drafts page lists comment drafts as today ("Comment on {title}") and
now offers **Open**, which opens the item and the thread's composer with
the draft, in place of `COMMENT_LATER` (E-8). Restore through the body's
path still refuses comment drafts (fact 23); comment drafts restore only
through `adopt_comment`. E-9 updates the D2 test that pins "never
restored".

## Lane B: Share, Progress And Recovery

### Share in the item header

- `Share` (secondary, `cloud-upload`) sits after Edit or Save/Done.
  Tooltip: "Bring in others' changes and share yours through {remote}.
  Unsaved edits are not included." With no publication remote: "Share…",
  which opens the Remotes settings page for the repository
  (`04-item-area.md:28`).
- Enabled when the item has an editing context (R-5). Otherwise disabled
  with the note "Nothing to share yet: this item has no private copy."
- Pressed: submits `ItemSync` under a new request ID. While it runs, the
  button reads "Sharing…" and is disabled (repeated-submit guard), and
  the pill shows the stage (below).

### Share in the repository detail

The selected repository's detail (`view/sidebar.rs`) gains **Share main**,
submitting `RepoSync`. Same guard, stage text and outcomes. No dialog
(R-6).

### Stage text

| Stage | Text |
| --- | --- |
| `accepted`, `observing` | "Getting ready…" |
| `connecting` | "Connecting to {remote}…" |
| `fetching` | "Bringing in changes…" |
| `updating_local` | "Merging changes…" |
| `pushing` | "Sending your saved changes…" |
| `verifying` | "Checking it arrived…" |
| `indexing` | "Updating the list…" |
| `writing` | "Saving…" |
| `stopping` | "Stopping at a safe point…" |

The pill shows "Sharing · {stage}" with a spinner while an item's share
or post runs. The operation entry shows the same text.

### Stop at a safe point

A running entry whose intent stops at safe points shows **Stop at a safe
point** in place of D1's "Finishing" note once accepted (fact 16,
R-1). Pressed, it calls `worker.cancel`; the button becomes "Stopping…".
The outcome says either "Stopped before anything was sent. Your saved
work is still on this computer." or "Stopped after {step}. {what
completed}." from `SyncOutcome::Stopped { completed }`.

### Outcomes

| `SyncOutcome` | Pill / operation entry | Item banner |
| --- | --- | --- |
| `Published` | "Shared" (Activity wording, `06-language`) | none |
| `Current` | "Same as the shared copy" | none |
| `Pending` | "Saved on this computer · not shared yet" + Retry sharing | none |
| `Conflict` | "Needs attention" | "You and someone else both changed this since it was last shared. Your work is safe; resolving it arrives in a later release." No action (R-7). |
| `NeedsUnlock` | "Your key is locked" + **Unlock…** | none |
| `NeedsKey` | "Choose a key to share with" + **Open Sharing & keys** | none |
| `NeedsHost` | "Check {host} before sharing" + **Check host** | none |
| `Failed` | the code's fixed message + the library's recovery controls | none |

**Unlock…** is the existing Retry under the same request ID, labelled for
the code (R-3). Retrying runs the request again; the worker's provider
prompts once.

**Retry sharing** is Retry under the same request ID.

**Give up** is the Cancel recovery control on a finished request (R-2),
shown when the library offers `request.cancel`.

**Resume** is offered for a pending synchronization or comment
publication row, submitting `OperationResume` (E-10). For a comment row it
publishes without writing (fact 4).

### The passphrase dialog

The dialog's subtitle names what waits: "Sharing {repository} is waiting
for your passphrase." Cancel is relabelled **Not now**, and the note
gains "If you choose Not now, you can keep working and unlock later."
Behaviour is unchanged: Not now answers `Cancelled`, the request finishes
`unlock_required`, nothing prompts again until the user presses Unlock…
(contract 6).

## Lane C: The Composer Widget

`Composer` is an entity in `view/item/composer.rs`, independent of drafts
and posting.

- **Editor.** A `zorite_editor::EditorState` opened through
  `view/editor.rs`'s existing `open_editor`, in a compact mode: no page
  chrome, min height 70 px (new) or 64 px (reply), grows with content. The
  Rich text / Markdown switch sits in the mini toolbar. Front matter does
  not exist here; the body is the whole text. D3's guard and focus
  rulings apply; D3's load normalization rules apply to a restored draft
  (R-10).
- **Mini toolbar.** "Replying to {name}" (reply only), Bold, Italic,
  Inline code, Link, Bulleted list (`04-comments.md:45-49`), the mode
  switch, and the draft indicator on the right ("Draft kept" / "Draft not
  kept. Post to protect your text."), fed by a `RecoveryState` the owner
  sets.
- **Footer.** The explanation (R-9) and the buttons: new comment —
  **Post and sync** / **Save comment locally**; reply — Cancel and
  **Reply and sync** / **Save reply locally**, chosen by a
  `has_remote: bool` the owner sets.
- **Events.** `ComposerEvent::Changed`, `Submit`, `Cancel`. `Submit` comes
  from the button or `PostComment`; it is not emitted while the composer
  is `Posting` or empty. `Cancel` comes from Cancel or `CancelComposer`.
- **State.** `Idle`, `Posting` (read-only, button "Posting…"),
  `Failed { message }` (red inline message naming the step, with Retry,
  which emits `Submit`).
- **Discard prompt.** On `Cancel` with text present, the owner shows
  "Discard this reply?" (Discard · Keep editing) with the Kit dialog,
  closed at once as D3's dialogs are.

Tests drive the composer in a test window with no drafts or worker.

## Lane D: Discussion Integration

### Placement

- The new-comment composer sits after the threads. It begins its draft on
  the first keystroke (`begin_comment(item, None, text)`), not on focus.
- **Reply** appears under every comment. It opens the single reply
  composer inside that comment's thread, after its last reply, with focus
  in it. A reply always targets the thread's root (decision 5): Reply on a
  reply targets its root, and "Replying to {name}" names the comment it
  was pressed on.
- Moving the reply composer to another thread keeps the first thread's
  draft (it stays a stored draft for that thread) and adopts the second
  thread's draft if one exists.
- Opening an item restores its comment drafts: the new-comment draft into
  the new composer, and a reply draft into its thread's reply composer
  (the newest, if several threads have one, opens; the others show
  "Draft reply" on their Reply button).

### Posting

1. `Submit` with text: the composer goes `Posting`. If the submission has
   no request ID yet, one is made and kept with the composer for the
   session.
2. `CommentAdd` is submitted with the draft's comment ID and parent, the
   composer's text and `created_by: None`.
3. A pending row at 60% opacity is drawn in the thread from the
   composer's text, keyed by the comment ID, until the outcome lands.
4. On the outcome (`comment_outcome`):

| Outcome | Thread | Composer | Draft |
| --- | --- | --- | --- |
| `Shared` | the row from the listing | cleared, `Idle` | retired |
| `SavedHere` | row + "Saved on this computer" (no failure styling) | cleared | retired |
| `SavedNotShared` | row + amber chip "Saved on this computer · not shared yet" + **Retry sharing** | cleared | retired |
| `SavedConflict` | row + chip + "Needs attention" operation entry (Lane B's conflict wording) | cleared | retired |
| `NotSaved` | no row | text kept, `Failed` with the code's message | kept |
| `AlreadySaved` | the row from the listing | cleared | retired |

- **Retry sharing** on the chip retries the same request (fact 4: it
  publishes without writing). Within the session the chip is keyed by
  comment ID and survives list reloads; it clears when a retry or a share
  of the item answers published or current.
- The draft is retired as soon as the checkpoint is committed, so a
  restart never offers to post a saved comment again (R-8).
- After any outcome with a committed checkpoint, the item's comment list
  is reloaded; if the result's discovery is `pending`, an `IndexRefresh`
  is queued in the background first (U-3).
- A committed comment checkpoint removes the item's context from
  `CreatedContexts`, as a committed save does, so leaving never releases
  a context holding a comment (R-4, fact 25).
- **Repeated submit.** While `Posting`, `Submit` is not emitted, and the
  registry refuses a second `CommentAdd` for a comment ID already queued
  or running (contract 3).

### Body draft separation

The body draft and comment drafts are different drafts with different
targets (fact 24). Posting reads only the composer's text. `comment add`
checkpoints only `.manyhands/comments/<item>/<comment>.md` (RFC Git
workflow :134), so unsaved body text, which lives only in the draft store
and the editor, is never committed or shared (contract 2, A12).

## Part E: Keyboard, Privacy And Native Evidence

- **Tab order** (`01-item-frame.md`, Keyboard): tab strip → header actions
  (…, Edit, Share) → chips → toolbar → page → discussion (Back to top,
  each comment, its Reply) → reply composer when open → new composer.
- **Ctrl Shift C** still jumps to the discussion heading; from there Tab
  reaches the first comment.
- **Privacy tests** extend `editing_privacy_tests.rs` with comment text
  sentinels: absent from operation entries, notifications, dialog messages
  and logs, through post, failure and retry.

### Native checklist (Linux Wayland, product owner, release build)

Written in Part E as `docs/plans/2026-11-02-wave-03-desktop-04-on-screen-checklist.md`,
in D3's format (Do / Expect, results table). Setup: a real SSH remote the
owner controls (a bare repository over local `sshd`, or a hosted remote),
a protected key, two clones; the second actor is `manyhands-cli` against
the second clone (track rule 4). Sections:

1. Smoke.
2. Discussion: two roots, two replies to one root, a reply to a reply,
   Ctrl Enter, Esc with and without text, Rich/Markdown switch, crash
   (kill the process) mid-reply and restore; `index rebuild` via the CLI,
   reopen, order holds.
3. Share: item Share, Share main, already current, an incoming change from
   the second clone merged.
4. Offline: remote unreachable, post (saved, not shared), Retry sharing
   after the network returns; quit and relaunch with a pending
   publication, Resume.
5. Credentials: first share prompts; Not now, keep working, Unlock…;
   wrong passphrase; host removed from the pins → Check host → Approve →
   Retry.
6. Responsiveness: a stalled remote (firewalled port), typing in the body
   and a composer, Stop at a safe point, quit while stopping.
7. Conflict: both clones change the same ticket line; Share shows the
   preserved conflict banner; nothing is lost; the CLI's `conflict show`
   sees it.

## Privacy

Comment text is held in `CommentBody` and the composer's editor only. It
enters the draft store (as D2 stores body text) and the library call.
Operation entries show the intent's fixed label ("Post comment on
{title}", "Share {title}", "Share main"), never the text.

## Testing

| Target | What |
| --- | --- |
| `services/requests_tests.rs` | Intents build the right mutation; `CommentBody` has no `Debug`; labels are fixed. |
| `services/worker_tests.rs` | An accepted sync or comment job is `Signalled` and its token cancelled; a local job stays `TooLate`; `give_up` calls `cancel_request` then re-admits the same request ID. |
| `model/publication_tests.rs` | Every row of both outcome tables from constructed envelopes. |
| `services/drafts/comments_tests.rs` | One comment ID per draft; per-thread drafts; flush within 1 s; adopt after a dropped registry; retire. |
| `view/item/share_tests.rs` | Share enabled/disabled; repeated-submit guard; stage text; each outcome's wording and control, with `DelayingBackend` holding the job. |
| `view/item/composer_tests.rs` | Events, states, Ctrl Enter, Esc, discard prompt, labels by remote. |
| `view/item/posting_tests.rs` | No-remote post (real repository, real library); unreachable-remote post (U-2): one file, one row, chip, Retry sharing reuses the request; checkpoint failure keeps text; ordering after rebuild; dirty body not committed; already-saved draft retired. |
| `view/keyboard_tests/discussion.rs` | The full keyboard journey with no remote. |
| `view/editing_privacy_tests.rs` | Comment sentinels. |

No new integration target; the desktop bin's tests already run in CI
(fact 28).

## Named Edits To Pre-Existing Tests

| E | Test | Edit | Owner |
| --- | --- | --- | --- |
| E-1 | `model/item_tests.rs::the_unavailable_actions_are_the_designs` | `LATER` loses Comment and Share. | Part 0 |
| E-2 | `view/item_tests.rs::a_nonconforming_entry_opens_by_path_with_its_problem_and_repair_offered` | The later-action filter no longer removes "Comment". | Part 0 |
| E-3 | `view/item/header_tests.rs::share_and_the_primary_action_are_disabled_with_their_note` | Only the primary action is later; renamed `the_primary_action_is_disabled_with_its_note`. | Part 0 |
| E-4 | `view/item/header_tests.rs::a_nonconforming_item_shows_needs_fixing_with_repair_offered` | `later-action` indices shift by one. | Part 0 |
| E-5 | `view/item/header_tests.rs::an_inspect_tab_of_a_conforming_item_needs_no_fixing` | `("later-action", 1)` becomes `0`. | Part 0 |
| E-6 | `view/item/header_tests.rs::tab_order_runs_strip_header_chips_page` | The later-action loop covers one action; Lane B then adds Share after Edit. | Part 0, Lane B |
| E-7 | `view/item/discussion_tests.rs::comment_is_disabled_with_its_note` | Replaced by `the_new_comment_composer_follows_the_threads` (Lane D). Part 0 deletes it. | Part 0 |
| E-8 | `view/drafts_tests.rs::each_row_reads_its_state_and_offers_what_its_policy_allows` | A comment row offers Open, not `COMMENT_LATER`. | Lane A |
| E-9 | `model/drafts/rows_tests.rs::a_comment_draft_is_never_restored_in_d2` | Restore through the body path still refuses; Open is offered. | Lane A |
| E-10 | `model/operations_tests.rs::resume_is_offered_for_a_local_operation_and_not_for_a_synchronization` | Resume is offered for a pending synchronization row. | Lane B |
| E-11 | `view/item/discussion_tests.rs::tab_runs_through_each_comment_and_reveals_its_caret` | Tab reaches each comment's Reply after the comment. | Lane D |

Task 0's sweep re-checks this list against the merged base; D3's lesson
is that the first real Edit broke nine unlisted tests. Any test outside
this list that breaks stops for the owner.

## Document Amendments

Applied in the approval commit:

1. **Wave 03, D4 section:** "Amended 2026-11-02 by D4 (decisions 1–7)":
   lanes; operation area in place of Activity/Needs attention; zorite
   composers; Known Limits; replies answer the root; Linux-only native
   evidence; the design's words.
2. **Desktop IA RFC, actions table:** "Sync item / Sync primary" are shown
   as **Share** and **Share main**; "Retry publication" as **Retry
   sharing** (decision 7, if confirmed).
3. **Ticket `01M4CC0VN1KTQFSVR99378TDHV`:** `deps` gains C4
   `01M4CC0VMWEQQ9G4W0DAG7ADPY`; its body's dependency line names C4.

## Rulings

| R | Ruling | Cost if wrong |
| --- | --- | --- |
| R-1 | The worker cancels an accepted job's token only for intents that stop at safe points; it does not call `cancel_request` for a running job in this process. | If the token were not honoured after acceptance, Stop would do nothing; worker tests pin it against the real library's sync binding (fact 9). |
| R-2 | Give up = `cancel_request` + the same request, only for a finished request offering `request.cancel`. | A wrong offer could cancel a publication the user wanted; it is shown only when the library offers it. |
| R-3 | Unlock… is Retry under the same request ID, labelled for `unlock_required`/`unlock_failed`. No separate unlock intent. | A user with several locked keys may be prompted for each in turn; acceptable for D4. |
| R-4 | A committed comment checkpoint removes the context from `CreatedContexts`. | Without it, leaving could try to release a context holding an unpublished comment. |
| R-5 | Item Share is enabled only when the item has an editing context (U-1 checks the library's answer otherwise). | If the library handles a context-less item usefully, D4 hides a working action; cheap to enable later. |
| R-6 | Share runs without a dialog; progress lives in the pill and the operation area. | The design has no Share dialog; adding one later does not change the mechanism. |
| R-7 | A conflict banner offers no action in D4. | Users must use the CLI to resolve until D5. |
| R-8 | A comment draft is retired at the committed checkpoint; `occupied_path` on post maps to `AlreadySaved`. | If retired too early, a crash between checkpoint and retirement leaves a draft whose post answers `occupied_path`, handled as already saved. |
| R-9 | The composer explanation reads "Posting also shares other saved work in this {ticket|document}. Unsaved edits above are not included." | Copy only. |
| R-10 | A restored comment draft is loaded with D3's load-normalization rule: normalization is not an edit. | A restored draft could otherwise show as changed and flush needlessly. |
| R-11 | The "not shared yet" chip is session memory keyed by comment ID; after restart the pending publication shows as an operation row with Resume, not as a chip. | After restart the thread does not mark the comment; the operation area does. K-1. |

## Known Limits

- **K-1.** No read reports unpublished comments (fact 14), so the chip and
  any count are lost on restart.
- **K-2.** `01M669FJR27FRHCNPWAMD6VH4V`: after `remote select`,
  `config.toml` is index-modified and the next sync or post answers
  `worktree_not_clean`. Checklist setup commits the selection first.
- **K-3.** `01M67BN4C1SY2HQBGNE9NBYMW8`: a comment stopped for an
  untrusted host carries no host data and offers only Retry. D4 offers
  Check host on the repository's root (the Hosts page lists its hosts).
- **K-4.** `01M66HFV44YXNQ7ANJVA16TQ1H`: a key selected after the remote
  is configured can block a context's first publication. Checklist setup
  selects the key first.
- **K-5.** `01M4R19PC0T6ZPKM39X9DF26HX`: a blocked sync holds the
  reservation; Give up (R-2) is the way out.
- **K-6.** `01M4H33R34Z7C7EEKTY1ZCT950`: a pending remote conflict cannot
  be abandoned.
- **K-7.** `01M5E6DYJ6GN7DMW4424TH3V78`: a wrong passphrase is reported as
  `transport_unavailable`, so the message says the remote could not be
  reached.
- **K-8.** One worker thread: a long share or a pending passphrase prompt
  delays every queued save. The UI stays responsive; the queue does not.
- **K-9.** A context made by posting (fact 3) is never released by the
  desktop; closure (D5) removes it.

## Alternatives Rejected

- **Serial subagent-driven execution** (D3, C4): proven, but no
  parallelism; rejected by decision 1.
- **Lanes in one worktree:** one lane's broken build blocks the others.
- **A plain-text composer:** needs a Wave amendment; rejected by decision 3.
- **Building Activity and Needs attention:** doubles the UI scope;
  rejected by decision 2.

## Risks

| Risk | Control |
| --- | --- |
| C5 merges to main mid-Cycle and moves the sync/resume code | Merge main at the Part 0 checkpoint and before Lane D; re-check facts 4–12. |
| Lane merges conflict despite the ownership table | Part 0 owns every shared file; a task touching a file outside its row stops. |
| Zorite quirks in a second, small editor (D3 R-59, R-79) | Lane C reuses D3's editor host functions and their tests. |
| Real-host behaviour untestable in the bin | Native checklist §§3–7; U-2 pins the failure path automatically. |
| Disk and time for three cold builds | Accepted with decision 1. |

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | See the Cycle's Review Record. | — | — | — |
