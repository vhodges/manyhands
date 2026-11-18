---
slug: vh-4y6na
id: 01M7R2GPPF7F7JRKHX8C5ZZ4PF
title: Desktop and library correctness leftovers from the D1, D4, D5 and 2.10 follow-ups
type: task
status: open
manyhands_managed: true
manyhands_kind: ticket
---
Open items from five follow-up tickets that Wave 04's UI rework did not absorb, checked against main at `5d80bc15` on 2026-11-18. The user-visible desktop gaps and the small correctness holes are here. The hygiene and test items are in the companion ticket, see the "Companion" line below. Each item cites the old ticket it came from and where it lives now. Every behavioural fix comes with a test that fails without it, per AGENTS.md.

Companion: `01M7R2GPPF135ZT7M4TZCR09XT` (hygiene, test support and tidy-up).

Supersedes the open parts of `01M5F3GVDYTH1Q5V5FCS9YF7V3` (settings UX, D1), `01M5CS4MH25QQ769HV61ZRKHXG` (tests and polish, D1), `01M6RAS63EWVHWWVBSZ836RXG2` (D5 follow-ups) and `01M57SBA7MKR9BZR7PYECSJ05G` (2.10 follow-up). Each of those carries a closing comment that says what it took, what Wave 04 did, and what was dropped.

## Desktop, user-visible

- [ ] A host approval refused with `host_replacement_required` offers no Replace. `view/confirm.rs:978-1010` shows both pins with only Close (and Retry on busy). Show the envelope's `host.replace` recovery. The test `approving_a_pinned_host_shows_both_pins` currently asserts there is no confirm button, so it changes with the fix. From `01M5F3GV`.
- [ ] The "From:" identity line is hidden when no email is configured (`src/desktop/view/settings/repositories/manage.rs:566-577`, the `None => body` arm). Say that no email is set instead of dropping the line. From `01M5F3GV`.
- [ ] A conforming comment's own problems (an invalid `created_by`, unrepresentable metadata) are never shown. `.problems` is read only in `malformed()` (`view/item/discussion/rows.rs:234`), which is reached only when `comment.id` is `None`. From `01M5CS4M`.
- [ ] `item_not_found` shows two Refresh buttons. The item banner (`view/item/banners.rs:194`) and the comments section (`view/item/discussion.rs:245`) both draw one, because `ItemView::stopped` (`model/item.rs:161`) copies the item's failure into the comments state, so `status.rs:71` yields `item-refresh` and `comments-refresh`. Show one. From `01M5CS4M`.
- [ ] When the dependents read fails, the Dependencies group has no names and no explanation: `group(&DEPENDENCY_GROUP, None, …)` at `view/relationships.rs:140-142` passes no status. Say why. From `01M5CS4M`.
- [ ] The tab close buttons sit inside `Role::TabList` (`view/tabs.rs:524` and `:651`, asserted at `tabs_tests.rs:765`). Decide the right role structure for the strip and its close controls, and check it with a screen reader before changing the assertion. From `01M5F3GV`.
- [ ] Tab element IDs are keyed by strip index (`.id(("tab", ix))` at `view/tabs.rs:581`, and `tab-title`, `tab-close` and the rest). Key them by `TabId` so a reorder or close does not move element identity. From `01M5CS4M`.
- [ ] A recovery that arrives for a different repository while a host check is connecting is silently dropped (`view/settings/hosts.rs:455` skips `start` when `connecting()`). Check whether that loses a recovery the person needs. The fix that stopped a second `observe_host` is correct. From `01M5F3GV`.
- [ ] "Keeping mine" survives beside "Keep mine" in the item-edit review (`model/review.rs:31`, used at `view/item/edit/review_view.rs:115`). Wave 04 Cycle 05b settled the resolve sheet's wording as "Chosen" and "Edited", so pick the matching word here. From `01M6RAS6`.

## Desktop, robustness

- [ ] Key the lifecycle locks by path, not `String`. A root that is not valid UTF-8 silently turns the lock off. `LockKey { root: RepoKey (String), id }` is at `model/lifecycle_lock.rs:43`, and the `to_str()` conversions are at `services/mod.rs:1083` (`lifecycle_of`), `model/operations/lifecycle.rs:37` (`lifecycle_answered`) and `view/lifecycle/prepare.rs:237` and `:265`. From `01M6RAS6`.
- [ ] Harden `local_time` in `view/confirm.rs:592-626`. No time zone is initialised before threads start (there is no `tzset` anywhere), the `cfg` is plain `#[cfg(unix)]` (`:612`) and not narrowed to targets that have `tm_gmtoff`, and the test (`view/confirm_tests.rs:337`) only checks the length and a `:56` suffix, so it cannot fail on a wrong offset. `offset_at` in `view/item/discussion.rs:109` reuses `local_offset`, so comment times are affected too. From `01M5CS4M`.
- [ ] `build_tree` is quadratic (`model/documents.rs:92` does a linear `paths.iter().position` per document and folder, and `level()` at `:116-132` re-filters all folders at every node). Index folders and paths. From `01M5CS4M`.

## Library

- [ ] `save_ticket` with a document target provisions a document context before it refuses (`src/repository.rs:2503-2570`: `prepare_context_unlocked` runs before the `!matches!(context.kind, AuthoringKind::Ticket)` check at `:2568`). The document save already refuses first (`:2113-2124`, "Before any context is provisioned"). Mirror it, with a test. No front end reaches this path today. From `01M57SBA`.

## Acceptance

- Each item is done, or dropped with a reason in a comment on this ticket.
- Each behavioural fix has a test that fails without it.
- The AGENTS.md static gates and fast tier pass, plus the SSH-tier targets for any changed path under `src/repository/`.
- Close only after review approval.
