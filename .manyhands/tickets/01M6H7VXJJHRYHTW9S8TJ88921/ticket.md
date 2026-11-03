---
slug: vh-5kvmy
id: 01M6H7VXJJHRYHTW9S8TJ88921
title: 'Desktop follow-up: structure and tests from the D4 review'
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Structural and test follow-ups from the Wave 03 D4 whole-branch quality review (ticket 01M4CC0VN1KTQFSVR99378TDHV, review at `26fcb8fa`). None blocked D4; each is a maintainability or robustness improvement. Paths are under `src/desktop/`.

## Items

1. **Classify share outcomes once.**
   - Problem: comments are classified at drain time (`services/submissions.rs`, `Submissions::finished`). Shares are classified again on every render: `view/operations/outcomes.rs` calls `share_result`, which locks the worker twice and reruns `sync_outcome`. Between a retry or Give up and the drain seeing `Queued`, an entry's text can flip for a frame.
   - Fix: carry `RecordState` and `GaveUp` on `WorkerEvent::Finished`, classify once in `Operations::apply`, store a `SyncResult`, and have `Submissions::finished` read the same event fields.
2. **Share the read-only guard.**
   - Problem: `view/item/composer/guard.rs` copies `stop`, `moved`, `inserts_text` and the 13-action stop list from `view/editor/read_only.rs`. An editing action added to one list but not the other silently opens an edit hole in the composer.
   - Fix: move them into one `pub(crate)` function in `editor/read_only.rs`.
3. **Make `CommentMutationDto::from_stored` public** (library).
   - Problem: `model/publication/record.rs` `comment_of` re-implements it (`src/repository/mutation/dto.rs`, `pub(crate)`).
   - Fix: make it public, then delete the desktop copy.
4. **Move composer state off render.**
   - Problem: `sync_composers` (`view/item/composer/place.rs`) runs on every `ItemPane` render. It updates composers, adopts drafts (starting claims), applies posts (locking the worker and cloning keys for each queued job), and updates the registry.
   - Fix: drive adoption and post application from `cx.observe` of `Submissions` and drafts.
   - Cheap win: compare by reference in `Submissions::posting`.
   - Similarly, the workspace's `observe_self` (`share_remotes`) should compare `Option<&str>` before allocating.
5. **Prune per-session maps.**
   - `Operations::shares` (and `stopping`) on `dismiss`.
   - `Posting::met` per tab.
   - Keep `Submissions` entries after Discard (P7 requires the comment ID to stay reserved).
6. **Split large files**, with no behaviour change:
   - `services/worker.rs` (~1110 lines): give-up and record state into `worker/record.rs`.
   - `model/operations.rs` (~1010): progress and recovery helpers into child modules.
   - `view/operations.rs` (~790): session and pending sections into their own files.
   - `services/mod.rs` (~545): the drain's post handling into `submissions`.
   - Test files over ~900 lines: `view/item/share_tests.rs`, `model/publication_tests.rs`, `view/editing_privacy_tests.rs`.
7. **Test hygiene.**
   - Move duplicated helpers into `test_support`: `hold_after_acceptance` (3 copies), `store_comment`, `open_ticket`, `item_sync`, and the `until_end_of` poll loop.
   - Assert the kind of request a `set_request_hook_for_testing` hook catches, and keep launch refresh off in shells that use it.
   - Give comment-draft claims their own `ReadKind` rather than sharing the `Drafts` FIFO gate with listings.
   - Gate `AdoptedComment::state`, which only tests read.

## Acceptance

- Each item is done or explicitly declined with a reason.
- Desktop behaviour is unchanged.
- The `AGENTS.md` static gates and fast tier pass.
- Item 3 is filed or done as a library change under the no-library-change rules of whichever Cycle takes it.
