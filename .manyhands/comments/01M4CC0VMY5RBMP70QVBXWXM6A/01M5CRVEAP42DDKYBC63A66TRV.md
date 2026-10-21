---
manyhands_managed: true
manyhands_kind: comment
id: "01M5CRVEAP42DDKYBC63A66TRV"
item_id: "01M4CC0VMY5RBMP70QVBXWXM6A"
created_at: "2026-10-20T13:43:42Z"
---

**Review-ready, local gates.** All 17 tasks are implemented. The code is at
`ba89ea0`; `origin/main` `6db8ab7` is merged and the merge-tree is
clean. Nothing is pushed, no PR exists, and CI was not dispatched.

**Reviews.**

- Every task was reviewed on its own and fixed until clean.
- Parts A and B each had an independent checkpoint review and a fix wave.
- The final whole-branch review found two Important items:
  - The CI release build used `--all-features`, which now ships GPUI test
    support. It now builds with `--features desktop,editor-probe` (ruling
    R-F1).
  - The Settings acceptance row understated its evidence. It is corrected.
- The final fix wave also clears the Keys page's passphrase fields when the
  page is hidden, and adds a password-URL sweep on a failed remote add
  (R-F2). A scoped re-review confirmed all of it.

**Final gate at `ba89ea0`:** every command exited 0.

- `cargo check --all-features --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `cargo check --locked --features desktop --bin manyhands`
- `cargo clippy --locked --features desktop --bin manyhands -- -D warnings`
- `cargo test --all-features --locked --no-fail-fast`: 3254 passed, 0 failed
- `cargo run --locked --bin manyhands-cli`
- `sh scripts/check-cli-no-gpui.sh`

The desktop tests also passed three runs in a row (362 tests). The desktop
smoke run (`cargo run --locked --features desktop --bin manyhands`) ran on
Wayland with an isolated `XDG_DATA_HOME` for 25 s, with no error output.

**Library boundary.** `src/lib.rs`, `src/repository*`,
`src/canonical.rs`, `src/results.rs` and `src/runtime.rs` are unchanged by
D1. Every library test entry point used already existed.

**Acceptance.** The Cycle document's table names a test for each row and gives
its status.

- Not met: the duplicate-context recovery view. It depends on
  `01M5B163PQTE9C1T6NMHATR3KX`.
- Partly or native only: the rows the native checklist must prove.

**Open obligations.**

- The owner's native Linux checklist: 14 steps, in the design's Native
  checklist section. It is pending.
- Windows and macOS native runs (G1).
- The CI desktop steps have never run.
- The library defects `01M4EHGE2K`, `01M4W5SC`, `01M4EPRY`, `01M5B0Z6` and
  `01M5B163`.

**Rulings made after the Part A approval and not yet confirmed by the owner.**
The cost if each is wrong is in brackets.

- **R-T9a:** a short-code Find lists every match, even a single one. Blocked
  also reads open tickets to name blockers. [One extra Enter; one extra read.]
- **R-T10a:** relationships read `ticket_dependencies(Both, 1)`, so
  dependency rows carry names. [One-line revert.]
- **R-B2:** tabs reload only after intents that can change items. [A tab shows
  pre-job data until Refresh.]
- **R-B3:** busy user jobs retry three times, except attempts that carry a
  secret. A job whose passphrase came from a prompt retries using the
  session's cached unlock. [Up to 0.7 s.]
- **R-B4:** settings are a content mode in the main window. [A move to a
  second window later.]
- **R-B5:** a busy reload keeps what is shown; a failed reload shows its
  failure. [A transient failure blanks a tab until Retry.]
- **R-T11a:** a passphrase prompt is shown above an open confirmation dialog.
  Queuing it would deadlock. [Two stacked modals.]
- **R-T12a:** the settings page list uses plain Kit controls, not the Kit
  `Settings` component. [A re-skin.]
- **R-T12b:** Add inspects the folder before it opens the preview. [One extra
  read.]
- **R-T13a and its extension:** tabs close when a good list read no longer
  lists their root. Our own Remove answered `already_applied` also counts as
  removed. [Tabs close after another process removes the registration.]
- **R-T14a:** `RemoteAdd` keeps the raw URL, which may hold a password, in
  the request records (the worker's and the UI's) so a Retry is the same
  request. It is never shown or formatted. [The credential stays in process
  memory for the session.]
- **R-T15a:** Retry of a protected key generation from the operation area
  opens the Keys page with focus on the passphrase field. [One extra step.]
- **R-T15b:** entering retry mode clears any typed passphrase. [Retyping.]
- **R-T16a:** `host.approve` and `host.replace` recovery actions open the
  Hosts page and check the named host. [One extra step.]
- **R-T16b:** tests may build a success host observation for the desktop's
  own model. [None.]
- **R-T17a:** Tab cycles focus window-wide, and region keys move between
  regions. The design is updated to match. [A follow-on adds per-region Tab.]
- **R-T17b:** while Finishing, open dialogs close and none opens. [A confirmed
  click does nothing visible.]
- **R-F1:** release builds use `--features desktop,editor-probe`. [Removing
  `editor-probe` is a one-word change.]
- **R-F2:** the two secret-hygiene fixes in the final wave. [None.]

**Proposed follow-ups.** The final review proposed two tickets; none is filed.

- Desktop tests and polish (about 17 items).
- Settings UX (about 20 items). This would include the duplication cleanup:
  `Read<T>`, the ticket-naming rules, and Workspace's `pub(super)` fields.

**Still needs separate authorization:** push, PR, merge, ticket closure
(final pre-merge push), CI dispatch, and worktree cleanup.
