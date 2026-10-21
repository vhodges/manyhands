---
manyhands_managed: true
manyhands_kind: comment
id: "01M5BDWHYQ1PBXWSDHM2S8K9K9"
item_id: "01M4CC0VMY5RBMP70QVBXWXM6A"
created_at: "2026-10-20T01:12:50Z"
---

Part A checkpoint (Tasks 1–7). The code is at `3b0e653`, with
`origin/main` `c62dc97` merged at `eb2f953`. That merge brings
`01M4R19P94`, the Part C prerequisite, onto the branch. Nothing is pushed.

**Tasks.** Each of the seven tasks was implemented, reviewed on its own, and
fixed until its review came back clean. Tasks 4, 5, 6 and 7 each needed one
fix round. The owner approved the extra `Cargo.lock` packages from
`gpui-kit/test-support` (ruling R-T1a) and rulings R-T2b and R-T3a.

**Independent Part A review.** The verdict was "ready for Part B with
fixes". One fix wave addressed every finding. The wave was re-reviewed, and
all nine items were confirmed addressed:

- **Critical:** `cargo build --features desktop` did not compile, because
  two views used a type that exists only with test support. The gates had
  used `--all-features` only. Every Verify line and checkpoint now also runs
  `cargo check` and `clippy --features desktop --bin manyhands`.
- **The repository selector** now works from the keyboard, as a Kit list
  with a text marker on the selected row.
- **Dialogs keep the keyboard.** Region keys and Refresh no longer escape
  an open dialog; Quit still works.
- **Fewer reads after a job.** Pending operations are no longer re-read for
  every repository after each job, which had caused the `busy` answers. A
  busy or failed reload keeps the last list, labelled.
- **Test waits** use a 10 s deadline instead of GPUI's hidden 3 s timer.
- **Prepare again** is offered for `external_change` only when the library
  offers `request.prepare` (ruling R-A4).
- **Previews retry `busy`.** Ruling R-A1 reverses R-T2b: `prepare` takes
  the cache write lock, so it can be busy, and the dialog now offers Retry.
- **Removed registrations.** A selected registration that another process
  removed now shows Unavailable.

**Gate at `3b0e653`:** every command below exited 0.

- `cargo check --all-features --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `cargo check --locked --features desktop --bin manyhands`
- `cargo clippy --locked --features desktop --bin manyhands -- -D warnings`
- `cargo test --all-features --locked --no-fail-fast`
- `cargo run --locked --bin manyhands-cli`
- `sh scripts/check-cli-no-gpui.sh`

The fix wave also passed the desktop tests three times in a row (152 tests).
The merge-tree against `origin/main` is clean.

**Desktop smoke:** `cargo run --locked --features desktop --bin manyhands`
on Wayland (`wayland-1`), with `XDG_DATA_HOME` pointed at an empty
temporary directory. It stayed up for 20 s until the timeout ended it, with no
panic or error output, and created its index only in that directory. It was
not inspected visually; that belongs to the native checklist.

**Library defects found during Part A, filed with owner approval:**

- `01M5B0Z6PNP7X4WH4JV3N0VQJ6`: enablement leaves the Git index stale.
  After `repo create`, the next `ticket create` answers
  `worktree_not_clean`. D1's test fixture works around it and cites the
  ticket.
- `01M5B163PQTE9C1T6NMHATR3KX`: item reads never report context problems.
  The duplicate-context recovery-view acceptance row is **not met** until it
  merges (ruling R-T6a; see the Cycle document's library limits).

**Carried forward:**

- **Task 8:**
  - re-read open items on refresh;
  - keep each tab's source field across reloads;
  - per-region focus fallback;
  - split `workspace.rs`;
  - guard new handlers while a dialog is open.
- **Tasks 12–16:** wire the recovery actions that still show as unavailable.
- **Task 17:** keyboard audit items.
- **Final review:** the remaining minor findings.

**Decisions for the owner:**

1. Does Part B (Tasks 8–10) proceed as planned?
2. Does Task 12 (onboarding) wait for `01M5B0Z6`, or ship with the
   limitation recorded?
