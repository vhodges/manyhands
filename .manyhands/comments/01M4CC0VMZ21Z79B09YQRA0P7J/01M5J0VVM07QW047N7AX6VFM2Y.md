---
manyhands_managed: true
manyhands_kind: comment
id: "01M5J0VVM07QW047N7AX6VFM2Y"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-22T14:39:57Z"
---

Task 2 is complete: owner protection, in commits `7333421` and `a9353f1`.

- **Built.** `src/drafts/protect.rs`, `protect/{unix,windows}.rs` and
  `protect_tests.rs` hold `DraftError` (kinds only, with a fixed
  sentence per kind), the data-directory and folder verdicts, and the
  folder held open.
  - On Unix every operation is relative to the held `drafts/` handle
    (decision 5). A swapped folder cannot redirect a write.
  - Group and other bits are cleared and never added.
  - A link, another owner, or a data directory writable by others is
    refused.
  - On Windows (decision 3), reparse points and other owners are refused,
    and the token user or default owner is accepted (ruling R4). No DACL
    is written.
- **Tests.** `--lib drafts::` passes 14 and `--test drafts` passes 13.
  Each permission test fails with a message if run as root. Clippy and fmt
  are clean.
- **Review.** An independent review found one Important issue: the
  primitives did not validate names, so `../x`, `a/b`, or an absolute
  name on Windows could leave the held folder. Fix round 1 added one name
  check to every method that takes a name. It refuses an empty name, `.`,
  `..`, `/`, `\\` and NUL, and also `:` on Windows. A scoped
  re-review confirmed the fix.
- **Small decision.** `Folder::create_new` takes `&mut self`, so its
  one retry can reopen the folder. Task 3 holds the folder in a mutex.
- **Known gap.** The Windows module is uncompiled here. Its calls were
  checked against the `windows-sys` 0.61.2 API. The native Build run
  after merge compiles and tests it.
- **Deferred minor points** go to the final review. One of them, a file
  that vanishes during a listing, is carried into Task 3.
