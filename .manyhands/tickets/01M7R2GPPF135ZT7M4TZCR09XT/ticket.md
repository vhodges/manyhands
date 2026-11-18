---
slug: vh-fdp7z
id: 01M7R2GPPF135ZT7M4TZCR09XT
title: Hygiene and test-support leftovers from the D1, D4, D5 and 2.10 follow-ups
type: task
status: open
manyhands_managed: true
manyhands_kind: ticket
---
Hygiene, test-support and tidy-up items left from five follow-up tickets after Wave 04's UI rework, checked against main at `5d80bc15` on 2026-11-18. None changes behaviour a person can see. The user-visible and correctness items are in the companion ticket, see the "Companion" line below. Each item cites the old ticket it came from and where it lives now.

Companion: `01M7R2GPPF7F7JRKHX8C5ZZ4PF` (user-visible desktop gaps and correctness holes).

Supersedes the open parts of `01M6H7VXJJHRYHTW9S8TJ88921` (D4 follow-ups), `01M6RAS63EWVHWWVBSZ836RXG2` (D5 follow-ups), `01M5F3GVDYTH1Q5V5FCS9YF7V3` (settings UX, D1), `01M5CS4MH25QQ769HV61ZRKHXG` (tests and polish, D1) and `01M57SBA7MKR9BZR7PYECSJ05G` (2.10 follow-up). Each of those carries a closing comment that says what it took, what Wave 04 did, and what was dropped.

## Cheap and mechanical

- [ ] AGENTS.md: update the SSH-tier timing ("about 1 h 36 min", line 27) with D5's measurement, `mutation_remote` alone about 84 min. From `01M6RAS6`.
- [ ] Remove the process references from production comments ("Task 11b review I-3", "orchestrator ruling (a)", `Task 27` and the like). The ticket counted about 61. Today `rg` finds about 186 hits in `src/desktop` and about 230 across 133 files in `src`, outside `*_tests.rs`. Also remove the `file:line` references that go stale (`services/requests.rs:587-592`, `stops_at_safe_points`). Consider a small guard, such as a rule in AGENTS.md or a test that greps for the patterns, so they do not regrow. From `01M6RAS6`.
- [ ] Library L1: drop the unused `_kind` parameter of `lifecycle_prepare_way_out` (`src/repository/mutation/way_out.rs:131`), flatten `resume()`'s `Option<Option<_>>` (`:156`), and let the route carry the send-again request ID so `human_lifecycle.rs:250` does not re-derive it with `again_of`. From `01M6RAS6`.

## Small structural fixes

- [ ] The composer's read-only guard copies `stop`, `moved` and `inserts_text` and the 13-action stop list from the editor. `view/item/composer/guard.rs:96-111` mirrors `view/editor/read_only.rs:239-255` (the comment at `guard.rs:94` says "mirror read_only.rs"), and `moved` is also copied in `view/editor/host.rs:435`. An editing action added to one list and not the other opens an edit hole in the composer. Move them into one `pub(crate)` function in `editor/read_only.rs`. From `01M6H7VX` item 2.
- [ ] Make `CommentMutationDto::from_stored` public (`src/repository/mutation/dto.rs:1258`, `pub(crate)` today) and delete the desktop re-implementation `comment_of` (`src/desktop/model/publication/record.rs:43`, about 35 lines). The desktop is a module of the bin crate, so library `pub(crate)` is unreachable from it. This is a library change, so file it under the library rules of whichever Cycle takes it. From `01M6H7VX` item 3.
- [ ] Prune per-session maps. `Operations::dismiss` (`model/operations.rs:634-647`) removes only `lifecycles` and `reviews`, so `shares`, `share_attempts` and `share_results` stay until the request is re-queued. `Posting::met` (`view/item/posting.rs:59`) is insert-only per tab. Keep `Submissions` entries after Discard, because P7 requires the comment ID to stay reserved. From `01M6H7VX` item 5.

## Tests

- [ ] Extend the ticket-closure privacy test's raw-store negative control. `tests/ticket_closure/privacy.rs:217-227` plants only a bare email in a raw store file. Plant a body, an endpoint, a secret, a title and `closed_by` too, so `ssh_privacy::scan` is proven to fail on each. The output and row-level controls already cover these (`privacy.rs:12-20` and `:84-110`). From `01M57SBA`.
- [ ] Share the desktop test waits and fixtures through `test_support`, with a `Shell::until_drawn_within(Duration, …)`. Today: `RESOLVING = 90 s` is copied four times (`view/conflict_tests.rs:51`, `view/keyboard_tests/conflict.rs:43`, `services/resolutions_tests.rs:118`, `view/lifecycle/privacy_tests.rs:119`), `until_long` twice (`view/conflict_tests.rs:168`, `view/operations/resume_tests.rs:161`), and the fixtures `ticket_shell` (5 copies), `close_through` (2), `until_present` (3), `until_lock` (3), `short_of_main` (2), `share_conflict` (2), `hold_after_acceptance` (4), `store_comment` (4), `open_ticket` (5), `item_sync` (4) and `until_end_of` (2). This is duplication and wait-constant drift, not test speed. From `01M6RAS6` and `01M6H7VX` item 7.
- [ ] Test gaps in behaviour that exists, each only if it can fail without the test (AGENTS.md: each new test must catch something existing tests do not): an absent-path Create preview (`add_repository_tests.rs:267` uses an empty directory only); the effects rows of a Partial result (`:591` does not assert them); Activity's Retry of a request that is not a protected key generation still goes through `retry(id, None)` (`view/workspace/requests.rs:598`, only the protected case is tested at `keys_tests.rs:858`); the key-source lines for a missing or unreadable file, and a busy or failed Copy (`settings/keys/rows.rs:175`); a typed failure for a non-busy key generate (only the busy case exists, `keys_tests.rs:416`). From `01M5F3GV`.

## Not scheduled: take when touching the code

These stay recorded here so nobody re-derives them. None is worth a change of its own.

- Split `services/worker.rs` (1564 lines, `worker/record.rs` for give-up and record state) and `services/mod.rs` (1150 lines, the drain's post handling into `submissions`). Both grew after the D4 ticket was written. Test files over about 900 lines: `view/item/share_tests.rs` (1006), `model/publication_tests.rs` (1014), `view/editing_privacy_tests.rs` (925), and many more now over 1400. From `01M6H7VX` item 6.
- Move the request drain out of `Services::start` into `services/drain.rs`, with an explicit split between what runs before `operations.apply` and what runs after the batch. `Services::start`'s drain also holds a strong `WorkerHandle` (`resending = worker.clone()`), which leaks a worker thread per desktop test. From `01M6RAS6`.
- Move composer state off render. `sync_composers` still runs from `ItemPane::render` (`view/item.rs:481`). Move it to `cx.observe` only if a measured problem appears. The cheap by-reference win in `Submissions::posting` and `Submissions::in_flight` (`submissions.rs:331`) can be taken alone. From `01M6H7VX` item 4.
- Library API tidy: rename `manyhands::drafts::folder::Folder` (`src/drafts/mod.rs:61-70`, for example `storage::ProtectedFolder`), and give `RepositoryService::data_directory()` (`src/repository/read/credentials.rs:275`) a better home such as a `DataDirectory` type. From `01M6RAS6`.
- Cache pending-row refreshes by `updated_at` (`services/pending_conflicts.rs:182-240` makes one probe per candidate). `services/requests.rs` is 883 lines. A bind-level test of the removed `d5_progress` SSH cases' boundary stages `[Accepted, Observing, Publishing, CleaningUp]` (`tests/mutation_remote/d5_progress.rs` keeps one end-to-end case). From `01M6RAS6`.
- Smaller test-hygiene items from the D4 review: assert the kind of request a `set_request_hook_for_testing` hook catches (the hook is `FnOnce()` with no argument, `src/repository/mutation/mod.rs:298`), give comment-draft claims their own `ReadKind` (`services/drafts/comments.rs:187` shares `ReadKind::Drafts`), and gate `AdoptedComment::state` properly (`comments.rs:86` has only `cfg_attr(not(test), allow(dead_code))`). From `01M6H7VX` item 7.

## Acceptance

- Each scheduled item is done, or dropped with a reason in a comment on this ticket. The "Not scheduled" list is not part of the acceptance.
- Desktop behaviour is unchanged.
- The AGENTS.md static gates and fast tier pass, plus the SSH-tier targets for any changed path under `src/repository/` or `tests/support/ssh_*`.
- Close only after review approval.
