---
manyhands_managed: true
manyhands_kind: comment
id: "01M5F97BM8WBSCWVDHT13SC9BZ"
item_id: "01M5CS4MJ0G7B0MPJD0KN0QMYA"
created_at: "2026-10-21T13:08:19Z"
---

## Task 4: Keys split verified

KeysPage retains its fields, constructor/subscriptions, constants and test hooks
in `view/settings/keys.rs` (311 lines). Private children own reads (74 lines),
commands/secret handling (311), and rendering (353). Existing method paths,
masked inputs, clearing order, attempt bookkeeping, retry policy, focus keys
and UI IDs remain in place. No new test edit was needed.

Ran the three planned filters with
`devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`:

- `desktop::view::settings::keys`: 18 passed.
- `desktop::view::operations`: 12 passed.
- `desktop::view::keyboard`: 12 passed.

All 42 passed without failures/ignored tests. `cargo fmt` through Devenv and
`git diff --check` passed. Reviewed secret extraction and clearing: comparison
remains in place, one zeroizing holder is constructed, and both masked inputs
are cleared before returning. Proceeding to Task 5; work is uncommitted.
