---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5CS4MH25QQ769HV61ZRKHXG"
title: "Desktop follow-up: tests and polish from the D1 reviews"
type: "task"
status: "open"
project: "manyhands"
team: "core"
---

The D1 desktop reviews (Wave 03 Cycle D1, ticket
`01M4CC0VMY5RBMP70QVBXWXM6A`) left a set of minor findings. The final
whole-branch review judged that none of them blocks merging D1, and grouped
them into this ticket. Each one is small. Most are a missing test or a
one-line guard.

This is desktop-only work under `src/desktop/`, and no library change is
expected. Pick it up after D1 merges, or fold it into the next desktop Cycle.
Code locations refer to the D1 branch as reviewed at `ba89ea0`.

## Items

**Test support and tests**

- Mark the `DelayingBackend` hold handles `#[must_use]`, if any are still
  missing it. Move the raw `git2` repository setup in
  `services/reads_tests.rs` (around line 951) into a `test_support::fixtures`
  helper.
- Assert that the item view's Repair action is disabled, not only that it
  does nothing (`view/item_tests.rs`).
- Test busy, failed and double inspection paths where the review named them.
  The settings-specific ones are in the settings ticket.

**Item view**

- Show a conforming comment's own problems, such as an invalid `created_by` or
  unrepresentable metadata (`view/item.rs`).
- On `item_not_found`, show one Refresh, not two (item and comments).
- Send no parent read after a failed item read. This is a one-line guard in
  the per-tab read job.
- When the dependency read fails, say why the Dependencies group has no names.
- Prune `row_focus` handles for rows that are no longer drawn.
- A parent that turns Unresolved keeps its earlier name. Clear it.

**Lists, tabs and find**

- `build_tree` is quadratic (`model/documents.rs`). Index folders and paths.
- The document list key `document:{worktree}:{path}` could collide between a
  conforming item and a nonconforming entry. The risk is low today. Make the
  key unique.
- Tab element IDs are keyed by strip index. Key them by `TabId`.
- A failed read of blocker names fails the whole Blocked list. Fall back to
  naming blockers by ULID.
- Clear or refresh Find matches when the repository is refreshed.

**Operation area and shell**

- `local_time` hardening in `view/confirm.rs`:
  - initialise the time zone before any thread starts;
  - narrow the `cfg` to targets that have `tm_gmtoff`;
  - strengthen the test.
- "Finishing" hides the running stage. Show the stage, and keep the disabled
  Finishing control.
- The repository row exposes two `aria_selected` signals, one for the Kit
  cursor and one for the chosen repository. Keep one meaning.
- `RepositoriesPane::set` notifies during render. Move it out of render.

## Acceptance

- Each item is fixed, or explicitly dropped with a reason in a comment.
- Each behavioural fix has a test that fails without it.
- The desktop gate passes:
  - the `desktop::` tests;
  - `cargo check` and `clippy` with `--features desktop`;
  - `fmt`;
  - `clippy` with `--all-features`.
