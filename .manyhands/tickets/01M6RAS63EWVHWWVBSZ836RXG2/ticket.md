---
slug: vh-bczc7
id: 01M6RAS63EWVHWWVBSZ836RXG2
title: 'Wave 03 D5 follow-ups: structure, test harness and copy'
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-ups from Wave 03 D5's whole-branch quality review (ticket 01M4CC0VN2K8DBHNMZ46NTX5JJ). None blocks D5. Each item is one change; split into tickets as they are taken up.

## Structure

- [ ] Move the request drain out of `src/desktop/services/mod.rs` `Services::start` into `services/drain.rs`, with an explicit split between what runs before `operations.apply` (`following.ended`, `settled`, `resent.requeued`, `resent.not_run`, `resent.ended`) and what runs after the batch.
- [ ] Split the lifecycle answers, reviews and lock bookkeeping out of `src/desktop/model/operations.rs` (1138 lines) into `model/operations/lifecycle.rs`. Also look at `view/operations.rs` (880) and `services/requests.rs` (793).
- [ ] Key the lifecycle locks by path, not string: a root that is not valid UTF-8 silently turns the lock off (`view/lifecycle.rs`, `services/mod.rs` `lifecycle_of`, `model/operations.rs` `lifecycle_answered`).
- [ ] Pending-row refreshes make one `show_conflict` and one `show_item` per row on every finished request and window activation. Consider caching by row `updated_at`.

## Library API (Part 0)

- [ ] L4: `manyhands::drafts::folder::Folder` is now a general public protected-folder API that returns `DraftError` and is used for resolutions. Move or rename it (for example `storage::ProtectedFolder`).
- [ ] L1: drop the unused `_kind` parameter of `lifecycle_prepare_way_out` (`mutation/way_out.rs:76`); flatten `resume()`'s `Option<Option<_>>` (`:102`); let the route carry the send-again request ID so `human_lifecycle.rs` does not re-derive it.

## Tests (per AGENTS.md Testing Strategy)

- [ ] One shared `RESOLVING` wait and a `Shell::until_drawn_within(Duration, …)` in the desktop harness, replacing four `RESOLVING` copies and three `until_long` loops.
- [ ] Move the shared desktop fixtures to `test_support`: `ticket_shell` (4 copies), `close_through` (2), `until_present` (3), `until_lock` (2), `short_of_main` (2), `share_conflict` (2).
- [ ] Remove the 30 s `thread::sleep` in `view/operations_tests.rs:491`.

## Copy

- [ ] One name for the resolve sheet's entry points ("Review changes", "Resolve conflict…", title "Choose what to keep") and one ellipsis rule for buttons that open a sheet.
- [ ] "Keep mine" beside "Yours" and "Keeping yours" on the same card: pick one.
- [ ] Shared label constants instead of copies: `RESUME` (4), `SHARE_MAIN` (2), `KEEP_MINE` (2).

## Comments

- [ ] Remove the process references in production comments ("Task 11b review I-3", "orchestrator ruling (a)", about 61) and file:line references that will go stale (`services/requests.rs` `stops_at_safe_points`).

## Docs

- [ ] AGENTS.md: update the SSH-tier timing ("about 1 h 36 min") with D5's measurements (`mutation_remote` alone ran about 84 min at D5's Part 0).
