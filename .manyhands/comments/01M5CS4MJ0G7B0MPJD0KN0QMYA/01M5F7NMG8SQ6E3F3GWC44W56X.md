---
id: 01M5F7NMG8SQ6E3F3GWC44W56X
item_id: 01M5CS4MJ0G7B0MPJD0KN0QMYA
created_at: 2026-10-21T12:41:09Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 2: Shared helpers verified

Added GPUI-free `model/read_state.rs` and `model/names.rs`. Settings and
repository management now use one Read, including the original None state.
Ticket lists, Find, relationships, tabs and blocker names use TicketName's
components/fallbacks, preserving each surface's labels and layout.

Factored ItemLocator into `model/navigation.rs`. OpenTarget::for_item and
TicketRow's stored locator share DTO address selection; relationship targets
retain their resolved/valid-ID admission and use the same repository binding.
No slug becomes an identity and no full ItemDto is retained just for targeting.

Ran each planned command as
`devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`:

- `desktop::model::`: 89 passed.
- `desktop::view::tickets`: 9 passed.
- `desktop::view::find`: 8 passed.
- `desktop::view::relationships`: 8 passed.
- `desktop::view::item`: 15 passed.
- `desktop::view::settings`: 90 passed.

All 219 passed with no failures or ignored tests. Ran `devenv shell -- cargo fmt`
and `git diff --check`. No test files changed. Reviewed the naming/target diff
for changes to fallback order, punctuation, separate cells and ID validation.
Proceeding to Task 3. Work remains uncommitted.
