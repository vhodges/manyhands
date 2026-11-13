---
manyhands_managed: true
manyhands_kind: comment
id: "01M5NWQ0CK2Z65GN871JEXRWF4"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T02:44:21Z"
---

Task 9 (flush on quit) is complete: `ec9d1da` and the fix `235402f`.

**What it adds.** `QuitPhase::SavingDrafts` comes before D1's phases. If
any session has unflushed text or a write in flight, quitting flushes
them all and shows "Saving drafts for recovery" with Quit now, so a hung
write never traps the user. When every flush succeeds, D1's quit proceeds
unchanged. If any fails, a dialog reads "1 draft could not be saved for
recovery" ("<n> drafts …" for more), with Quit anyway and Cancel (the
default). Cancel keeps the window and focuses the first failed draft's
Retry. The window's close request takes the same path. With no unsaved
session, D1's quit is unchanged.

While drafts are saving, no confirmation dialog opens in front of Quit
now. A passphrase prompt is answered Unavailable only once the quit is
committed (every flush succeeded, Quit anyway, or Quit now). Until then it
is held, and Cancel shows it again unanswered.

**Tests.** `desktop::view::workspace` 24 (D1's quit tests unchanged),
`desktop::view::passphrase` 14, `desktop::view::confirm` 9. Each fix
was reverted once to show its test fails. Check and clippy are clean with
`--features desktop` alone and with `--all-targets --all-features`;
fmt is clean.

**Review.** Approved, spec compliant, no Critical or Important findings.
The fix round closed two minors: a confirmation could open over Quit now
during the flush, and a prompt was answered Unavailable before the quit
was committed. Deferred: the window-close path is tested only while
saving, not with the failure dialog; `model/operations.rs` is at 563
lines (534 before D2).

The Part B checkpoint follows.
