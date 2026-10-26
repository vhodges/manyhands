---
manyhands_managed: true
manyhands_kind: comment
id: "01M5QEN5R00HQYK1QNTTR8522Y"
item_id: "01M4CC0VMZ21Z79B09YQRA0P7J"
created_at: "2026-10-24T17:17:10Z"
---

**D2, Recoverable Drafts, is ready for review.** Task 12 and the final
whole-branch review are done. The branch is
`manyhands/ticket/01M4CC0VMZ21Z79B09YQRA0P7J`; its head merges `main` at
`a684995`.

**Task 12: keyboard, privacy and evidence.**

- **Commits.** `84e85ac`, `787ea14`, `4b2673f`, `a4d865f`, the final-review
  fix `59d7e0a` and a comment tidy, `4343729`.
- **Keyboard tests** (`keyboard_tests/drafts.rs`, `keyboard_tests/drafts/quit.rs`)
  drive these by keystroke only, each asserting visible focus:
  - `secondary-shift-d` opens Drafts, and Tab reaches every row action;
  - Inspect, then Escape;
  - Export through the simulated dialog;
  - Discard: Escape keeps the draft, Tab to Discard and Enter discards it;
  - Restore, then Compare and the close-tab choices in the tab;
  - quitting with a held write, and its Retry.
- **The keyboard defects these tests found are fixed:**
  - Enter on a focused dialog button now presses that button, not the
    dialog's default;
  - each dialog button shows a focus ring;
  - the safe choice is the first Tab stop (Cancel; Keep draft when a tab
    is closed).
- **The privacy test** plants a sentinel in a ticket draft's body. It then
  drives restore, refresh, a failing flush, a stale save, a save the
  library refuses and an export. The sentinel appears in no operation
  entry, message, notification, dialog, or `Debug` of a `DraftError`.
- **Evidence.** The Cycle's acceptance table now has its Held by and
  Status columns filled. It marks what the native checklist and the
  post-merge native run still owe. It claims nothing for Windows or macOS.

**D1 fixes, made at the owner's request.** D1's confirmation dialog
(`b408efc`) and its passphrase dialog (`8b41a6b`) had the same Enter bug.
Both are fixed, each with its own keyboard tests
(`keyboard_tests/confirm.rs`, `keyboard_tests/passphrase.rs`).

**Final review: Ready.**

- No Critical or Important findings.
- All the owner's limits hold:
  - no change under `src/repository*`, `canonical.rs`, `results.rs` or
    `runtime.rs`;
  - `src/lib.rs` gains only `pub mod drafts;`;
  - no new dependency;
  - no glob GPUI import, and `src/drafts/` imports nothing from the
    desktop;
  - no `Debug` on a type that holds draft text;
  - ruling R25 holds.
- All five failure modes hold:
  1. a late save cannot retire newer text;
  2. a crash mid-write loses nothing;
  3. an unknown file is kept;
  4. a base moved by another actor's change is caught;
  5. two processes cannot write one draft.
- 52 deferred items were adjudicated and none must be fixed. Rulings 104
  and 151 are worth a second look in D3.
- **Fix.** One minor fix (`59d7e0a`), approved on re-review:
  - the Drafts page no longer takes focus from behind an open dialog;
  - an unreadable draft that another process holds offers only
    Export raw….

**Gate, at `59d7e0a`.** Every step exits 0, with 3744 tests passed and
0 failed:

- check, fmt and clippy with `--all-features`, and the tests;
- check and clippy with `--features desktop`, both for the bin and
  alone;
- `scripts/check-cli-no-gpui.sh` and the CLI run.

After the gate, the branch changed only by a doc comment and a merge of
`main`, which touched only `scripts/ticket-loop`. Fmt and the desktop
check are clean on the comment. `git merge-tree --write-tree HEAD
origin/main` is clean after a fresh fetch.

**Known limits.** These ship as they are:

- If the `drafts/` folder is deleted while a draft is restored, the lock
  stays on the old folder until the session ends. This is for D3.
- A directory named `<ULID>.json` in `drafts/` is listed as unreadable and
  cannot be discarded.
- The read-only-folder tests fail when run as root.
- Some files have grown past the usual size:
  - `view/operations.rs`, 658 lines;
  - `model/operations.rs`, 568;
  - `view/confirm.rs`, about 805;
  - `keyboard_tests.rs`, 588.
- D1's other harmless wrappers are unchanged: `selection-remove`, the
  recovery-action controls, and the settings fallbacks.
- There is no editor for draft text. That is by design; editing comes in
  D3. Seed a draft with `examples/seed_draft.rs`.

**Still owed.**

- The owner's native checklist on Linux Wayland.
- The native CI run, which runs after the merge.

Merging, opening a PR, dispatching the Build workflow, closing the ticket
and removing the worktree each wait for the owner's approval.
