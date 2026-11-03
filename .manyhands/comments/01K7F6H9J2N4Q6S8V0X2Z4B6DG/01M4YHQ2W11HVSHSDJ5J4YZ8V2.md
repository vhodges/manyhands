---
id: 01M4YHQ2W11HVSHSDJ5J4YZ8V2
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-15T01:09:37Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Owner authorized diagnosing and correcting the first native failures, with
normal pushes and dispatches. Three further five-target Build runs followed; no
force push. Latest:
[38659928602](https://github.com/vhodges/manyhands/actions/runs/38659928602) on
`3960926`, **failure** (2026-10-15T00:25:20Z–01:07:45Z).

| Target | Result on `3960926` |
| --- | --- |
| Linux x86_64 | PASS; 1,700 configured tests, including 84 polling SSH cases |
| Linux aarch64 | PASS; 1,700 configured tests, including 84 polling SSH cases |
| macOS aarch64 | PASS; 1,688 configured tests, including 83 polling SSH cases |
| Windows x86_64 | FAIL `poll_automatic_in_transfer_yields_to_manual` after 48 polling cases; 1,181 passed |
| Windows aarch64 | FAIL same case after 48 polling cases; 1,181 passed |

Windows library fixture (fixed in `24ae070`): the poll identity fixture wrote
worktree admin `gitdir` links with native path text. libgit2 splits that link on
`/` only, so on Windows the linked workdir resolved to the process directory.
The fixture now shares the existing Git-metadata link encoder used by the sync
fixtures. Both Windows targets pass all 579 library tests since run
[38624653318](https://github.com/vhodges/manyhands/actions/runs/38624653318).

In-transfer yield (macOS fixed in `3960926`; Windows open): temporary stage and
relay-state observations (runs 38624653318 and
[38631189454](https://github.com/vhodges/manyhands/actions/runs/38631189454),
commits `91aebec`/`a3021a4`, since removed) showed the client poll stopping and
returning `Interrupted` correctly, then the helper audit barrier timing out. On
macOS and both Windows targets the fixture relay forwarded about 131 KB more,
then froze with its upload-pack helper active for 25+ s; on Linux it ended
within 500 ms. libgit2 requests `SOCK_CLOEXEC`, which is `0` on macOS, and
Windows socket handles are inheritable, so the in-process fixture's helper holds
a copy of the client socket and the abandoned connection never closes. The
relay now ends a send that makes no progress for 5 s, which kills the helper.
That fixed macOS. Windows still fails after about 19 s; the remaining Windows
cause is not yet diagnosed.

Production observation (not changed here): on macOS and Windows, any child
process started during a libgit2 SSH operation inherits the connection socket.

The branch no longer merges cleanly with `origin/main` (`56ddf23`): conflicts in
`.github/workflows/build.yml`, `src/repository.rs`,
`src/repository/recovery.rs`, `src/repository/remote/state.rs` and
`tests/support/ssh_server.rs`. Native acceptance remains blocked. Ticket remains
open.
