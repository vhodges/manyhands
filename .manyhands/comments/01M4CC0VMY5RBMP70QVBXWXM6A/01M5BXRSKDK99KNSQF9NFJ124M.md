---
manyhands_managed: true
manyhands_kind: comment
id: "01M5BXRSKDK99KNSQF9NFJ124M"
item_id: "01M4CC0VMY5RBMP70QVBXWXM6A"
created_at: "2026-10-20T05:50:24Z"
---

Part B checkpoint (Tasks 8–10) at `323c66e`. Not pushed. `origin/main` is
`6db8ab7`, merged at `2783bfa`, and the merge-tree check is clean.

**Tasks.** Each task was implemented and reviewed on its own:

- **Task 8, tabs.** Tabs are keyed by repository and ULID. `workspace.rs` is
  split into lists, tabs and quit modules.
- **Task 9, find and readiness filters.**
  - Find matches a ULID or a short code; a short-code search lists every match.
  - The Ready and Blocked filters run through `list_tickets`.
  - A "Why blocked" panel shows each reason in full (fix round 1).
- **Task 10, relationships panel.** It reads `ticket_dependencies(Both, 1)`
  (ruling R-T10a), so dependency rows can be named.

**Independent Part B review.** It found nothing Critical, and asked for fixes
before Part C. One fix wave addressed every item, and a re-review confirmed it:

- **I1, read load.**
  - One stamped read job per tab load, in the design's order.
  - A busy reload keeps what the tab shows, with "Could not refresh" and
    Retry. A failed reload shows its failure, such as `item_not_found`
    with Refresh (ruling R-B5).
  - Tabs reload only for intents that can change items (ruling R-B2).
  - User-lane jobs also retry `busy`, except attempts that carry a secret
    (ruling R-B3).
- **I2, keyboard scrolling.** PageUp, PageDown, Home and End scroll the
  content region, and a newly focused control is scrolled into view.
- **Minor fixes:**
  - An untitled blocker is shown by its short code.
  - Relationship-row focus is keyed by ticket.
  - `unless_dialog` is now shared (ruling R-B4: settings render in the main
    window's content area).

The owner confirmed that Part B is not a stop (ruling R-B1).

**Gate at `323c66e`:** every command exited 0.

- `cargo check --all-features --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `cargo check --locked --features desktop --bin manyhands`
- `cargo clippy --locked --features desktop --bin manyhands -- -D warnings`
- `cargo test --all-features --locked --no-fail-fast`: 3115 passed, 0 failed
- `cargo run --locked --bin manyhands-cli`
- `sh scripts/check-cli-no-gpui.sh`

The desktop tests also passed three runs in a row (223 tests).

**Part C base.** `01M4R19P94` has been on the branch since `eb2f953`.
Part C starts at `323c66e` (Task 11).

**Carried into Part C:**

- **Task 11:** a cancelled job must not be retried on `busy`. Quitting or
  closing the window answers a waiting prompt `Unavailable`.
- **Tasks 12–16:**
  - Wire the recovery actions that still show "Not available yet".
  - Settings are a content mode in the main window.
  - Keep focused controls below the fold scrollable into view.
- **Task 17:** the keyboard and accessibility audit items, and removing the
  blanket `dead_code` allows.
