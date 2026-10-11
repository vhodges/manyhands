---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4PB82G07RZSQMNJZN672S3W"
title: "A partly damaged application database is not recognized as corrupt and cannot be rebuilt"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

The library replaces the application database only when opening and migrating
it fails with a structural SQLite error. Damage in pages that opening and
migrating never read passes that test, so nothing replaces the file, and no
command can rebuild it.

## Evidence

Found on 2026-10-11 during Wave 03 Cycle F2 Task 9 (ticket
`01M4CC0VMR8HSPZXQ1WX41GWVK`, branch `manyhands/ticket/01M4CC0VMR8HSPZXQ1WX41GWVK`). Built deterministically in a test
(`a_rebuild_of_a_database_whose_corruption_opening_it_does_not_meet_is_index_unavailable`
in `tests/mutation_replay/index.rs`): the root page of
`sqlite_autoindex_request_records_1` is overwritten after a WAL checkpoint.
The file opens and migrates; a lookup in that index fails as malformed.

In that state:

- every command that reads the damaged structure fails;
- `index rebuild` answers `index_unavailable`, and the recovery action that
  code offers is `index.rebuild`, the command that just failed;
- restarting the application changes nothing;
- the only way out is to remove the database file by hand.

A second form, with the root page of the `request_records` table itself
overwritten, reads no row without an error and fails later, at the insert. It
ends the same way and has no test.

The replacement itself is sound: it runs under the exclusive cache guard,
replaces only on `DatabaseCorrupt` or `NotADatabase`, and renames the old
file aside. The gap is only in what counts as corrupt
(`is_structural_sqlite_corruption` applied to open-and-migrate).

## Proposed direction

Widen the test the replacement uses, for example with SQLite's
`PRAGMA quick_check` when a rebuild is asked for, or by treating a malformed
read met by the rebuild as evidence. Keep the rename-aside and the exclusive
guard.

## Open points for planning

- Cost of an integrity check on a large database, and whether it runs only on
  a rebuild request.
- This decides when a database is replaced, so a false positive loses every
  registration, key registration and host approval. The test must be one that
  a healthy, busy or locked database can never fail.
- What `index_unavailable` should offer when the rebuild itself has failed.
