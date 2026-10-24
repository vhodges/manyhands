---
manyhands_managed: true
manyhands_kind: comment
id: "01M5NVM7NF7K3C3KGRWWT51JVC"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T02:25:22Z"
---

Task 8 (drafts in tabs) is complete: `2f0eeae` and the fix `4cd606d`.

**What it adds.**

- **Banner.** An item tab whose item has a readable draft this process has
  not claimed shows "A recovered draft of this item is kept" with Restore.
  Opening the tab restores nothing. A draft held by another window keeps
  Restore offered, so the user can retry.
- **Restore.** It claims the draft off the UI thread and attaches it to the
  tab, opening the tab only through `Workspace::open_item` with
  `ItemLocator::Identified` (R25). It refuses a second draft of the same
  item ("Another draft of this item is open"). A draft removed since the
  listing reads "This draft is no longer kept", and the listing reloads.
  Restore writes nothing to the repository (a snapshot of files and refs is
  equal before and after).
- **Restored-draft section.** It shows the draft's fields, its recovery
  state ("Saved for recovery at <local date and time>", "Not yet saved for
  recovery", or the failure with Retry), the target state using the
  design's labels (`target_label`, distinct for every state), and
  Compare, which expands `DraftComparisonView`. Edit and Save are disabled
  and read "Available with the editor in a later release". Refresh
  reclassifies; it never touches the draft's text or base.
- **Close.** The tab's Close button and `secondary-w` open a dialog for a
  tab with a draft attached: Keep draft (flush, then release; the draft is
  offered again), Discard, Cancel, and a disabled Save. A failed Keep leaves
  the tab open with the failure. Focus returns to the content region after
  the dialog closes the tab.
- **Removal.** The removal confirmation reads "Drafts of this repository's
  items stay in Drafts." Removing a repository, or its leaving the list,
  closes its tabs and releases their sessions. A session whose write fails
  then stays live with its text and Retry, and is released as soon as a
  later write stores it.
- **Moved drafts.** Tab attachments follow a draft that Task 7 stored again
  under a new ID.

**Tests.** `desktop::view::item` 23, `desktop::view::tabs` 21,
`desktop::view::settings` 93, `desktop::view::draft_comparison` 3,
`desktop::view::workspace` 17, `desktop::view::keyboard` 12,
`desktop::services::drafts` 32. The tests use real repositories, a real
store and child processes as second actors. Check and clippy are clean
with `--features desktop` alone and with `--all-targets --all-features`;
fmt is clean.

**Review.** Approved, spec compliant, no Critical or Important findings.
All five Review Focus failure modes pass. The fix round:

- restores focus after a dialog-driven close;
- releases a claim whose Restore lands after its tab or repository is gone;
- completes a release left pending by a failed write, once a later write
  lands `Saved`. The release checks `Saved` and not busy, so it cannot
  drop unwritten text;
- checks that the disabled controls are inert. The Kit's Button reports no
  disabled flag, so the tests check that each is a button, isn't reported
  enabled, and does nothing when clicked.

**Rulings and deferred.**

- Two small edits to `view/tabs.rs`: Close passes the window so it can
  open the dialog, and a pane accessor.
- `Services::draft_read` is removed; it had no remaining caller.
- Releasing during a pending save ends the session. The draft stays on
  disk and later reads `AlreadySaved`.
- The banner appears only once the item has loaded. The Drafts page covers
  gone and unreadable items (Task 10).
- `view/confirm.rs` is 793 lines (788 before; plan-mandated).
- Claims are dropped on the UI thread. This doesn't block; it is noted for
  the final sweep.

Task 9 starts next.
