---
manyhands_managed: true
manyhands_kind: comment
id: "01M4YY6DXC7Z8RBKYQHPK7HBP2"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-15T04:47:43Z"
---

Native matrix green on the ticket branch. Owner authorized a Windows
diagnostic run, keep-going test execution, dropping Windows aarch64 from the
matrix, and a confirming run. Latest
[38677739080](https://github.com/vhodges/manyhands/actions/runs/38677739080) on
`b794460`: **success** (2026-10-15T03:43:25Z–04:47:08Z).

| Target | Result on `b794460` |
| --- | --- |
| Linux x86_64 | PASS; 1,700 configured tests, including 84 polling SSH cases |
| Linux aarch64 | PASS; 1,700 configured tests, including 84 polling SSH cases |
| macOS aarch64 | PASS; 1,688 configured tests, including 83 polling SSH cases |
| Windows x86_64 | PASS; 1,597 configured tests, including 82 polling SSH cases |

Windows aarch64 is no longer a matrix target (`7fa3b4f`, owner decision).

Keep-going (`81dc3a6`): `cargo test --no-fail-fast` (as on main), and the SSH
harness runs every isolated case before failing its target.

Diagnostic run
[38670260226](https://github.com/vhodges/manyhands/actions/runs/38670260226)
(`626432a`; markers since removed) showed, on Windows x86_64:

- In-transfer yield: the 5 s stall bound fired, the upload-pack helper was killed
  and exited, then the relay waited forever in `exit_status_request`. Another
  inheritor still holds the client socket, so the session stays wedged. Fixed in
  `b794460`: an abandoned relay skips its channel teardown messages and records
  its audit.
- `poll_recovery_database_wal_backup_privacy` (previously masked) also failed. It
  held `BEGIN IMMEDIATE` on the live WAL database during the raw-file privacy
  scan; on Windows that write lock is a mandatory `-shm` byte-range lock that
  blocks the read. Fixed in `b794460` by holding the journal on the backup, as
  the synchronization and merge privacy cases already do. This cause is inferred
  from that comparison and the green result, not observed directly.
- Every other interrupt, disconnect and cancel polling case passed on Windows.

Production observation (unchanged): on macOS and Windows, any child process
started during a libgit2 SSH operation inherits the connection socket.

Outstanding: the branch does not merge cleanly with `origin/main` (`56ddf23`);
conflicts in `.github/workflows/build.yml`, `src/repository.rs`,
`src/repository/recovery.rs`, `src/repository/remote/state.rs` and
`tests/support/ssh_server.rs`. The integrated branch needs its own native run.
Ticket remains open.
