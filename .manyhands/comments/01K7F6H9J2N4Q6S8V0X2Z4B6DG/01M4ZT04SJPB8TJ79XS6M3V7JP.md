---
id: 01M4ZT04SJPB8TJ79XS6M3V7JP
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-15T12:53:37Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Integrated branch green. Owner authorized merging current `origin/main`
(`56ddf23`) and fixing the one integration gap found. Merge commit `f91bbb9`
(parents `0ed116b`, `56ddf23`; no rebase), then fix `bb526e8`.
[Build 38726617896](https://github.com/vhodges/manyhands/actions/runs/38726617896)
on `bb526e8`: **success** (2026-10-15T11:20:22Z–12:52:55Z). The merge-only run
38720983976 on `f91bbb9` was cancelled once `bb526e8` superseded it.

| Target | Result on `bb526e8` |
| --- | --- |
| Linux x86_64 | PASS; 2,716 configured tests (772 lib, 1,483 ordinary, 461 SSH incl. 84 polling) |
| Linux aarch64 | PASS; 2,716 configured tests (772 lib, 1,483 ordinary, 461 SSH incl. 84 polling) |
| macOS aarch64 | PASS; 2,704 configured tests (763 lib, 1,481 ordinary, 460 SSH incl. 83 polling) |
| Windows x86_64 | PASS; 2,597 configured tests (732 lib, 1,411 ordinary, 454 SSH incl. 82 polling) |

Conflict resolutions in `f91bbb9`:

- `build.yml`: main's test list plus `remote_polling`; Windows aarch64 stays out.
- `src/repository.rs`: union of both sides' remote re-exports.
- `remote/state.rs`: main's `stored_phase()` (identical phases) plus Cycle 08
  `poll_kind` parsing.
- `recovery.rs`: main moved the active-remote refusal into `in_the_way()`;
  `begin_or_reconcile` asks an active automatic poll to yield
  (`RepositoryBusy`) before calling it, so other active remote operations still
  get `RecoveryRequired`. The `remote_active_reservation` unique index keeps
  this equivalent to the former single query.
- SSH fixture: both sides added an accepted-socket counter; main's
  `accepted_sockets()` is kept and the duplicate `connections` removed.
- `remote_polling.rs`: the fixture comment sets main's new `created_by` to
  `None`, serializing exactly as before.

Integration fix `bb526e8`: main's `require_refresh_can_begin`
(`create_folder`'s pre-check, documented as the refresh's own test) answered
`RecoveryRequired` during an automatic poll while the refresh itself answers
`RepositoryBusy`. It now answers `RepositoryBusy` and writes nothing; the
refresh makes the yield request. Explicit polls and other remote operations
remain `RecoveryRequired`. A new unit test failed first, then passed; the
existing pre-check test's minimal remote table gains the real `id` and
`priority` columns.

`origin/main` is unchanged at `56ddf23` and contained in the branch. Ticket
remains open; PR, merge to main and closure are separate owner decisions.
