---
manyhands_managed: true
manyhands_kind: comment
id: "01M5F8RMBGVWVYKCKDY5EC8434"
item_id: "01M5CS4MJ0G7B0MPJD0KN0QMYA"
created_at: "2026-10-21T13:00:16Z"
---

## Task 3: Workspace encapsulation checkpoint

Extracted `view/workspace/{content,find,focus,lifecycle,lists,render,repositories,requests}.rs`.
Workspace's fields are all private. Its root file is now 261 lines (from 1159);
the children range from 93 to 305 lines. Sibling views use read-only queries and
named actions. TabContent owns its tab entities/subscriptions/focus maps. The
private ContentArea groups tabs and Settings; its one selector derives current
visibility from SettingsHost and introduces no second flag. Existing entity,
subscription, scroll, prompt and worker ownership remains in place.

The first test compile found five test helpers accessing fields directly. The
owner explicitly selected "Allow accessor updates (Recommended)" for a scoped
exception: four `workspace.settings.clone()` calls became
`workspace.settings().clone()` and one `workspace.tickets_pane.rows(cx)` became
`workspace.tickets_pane().rows(cx)`. The settings accessor is test-only. The plan
and ticket record the exception; no assertions, scenarios or other test bodies
changed. Removed unused imports exposed by the extraction.

Ran all nine Task 3 commands as
`devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`:

- `desktop::view::workspace`: 17 passed.
- `desktop::view::documents`: 4 passed.
- `desktop::view::tickets`: 9 passed.
- `desktop::view::find`: 8 passed.
- `desktop::view::tabs`: 17 passed.
- `desktop::view::operations`: 12 passed.
- `desktop::view::passphrase`: 11 passed.
- `desktop::view::settings`: 90 passed.
- `desktop::view::keyboard`: 12 passed.

All 180 passed, with no failures or ignored tests. Also passed:

- `devenv shell -- cargo check --locked --features desktop --bin manyhands`
- `devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings`
- `devenv shell -- cargo fmt`
- `git diff --check`

The desktop-only commands verify the new query boundary compiles without test
support. Re-listed the inventory: exactly 362 desktop tests, with identical
SHA-256 `a20c47450f3a8613a7c0b90ff1fca93bddf53ab8e3bdbb39e65cc0bcdd291343`.
Reviewed the test diff: exactly the five authorized accessor substitutions.
Source audit found direct field access only in private Workspace implementation
children; the only external Settings hide/is_shown calls are in the content
router. Settings retains its own existing internal visibility transitions.

Tasks 1–3 are ready for owner checkpoint feedback. Tasks 4 (Keys split), 5 (item
split) and 6 (final gates/review/handoff) are pending. This is not full-ticket
review-ready status. Full all-feature gates and independent final code review
have not run; native CI has not been dispatched. All work remains uncommitted,
and the ticket is open.
