---
id: 01M7P9S9YCFJK7GZ9JY3PBC5K0
item_id: 01M67Y754SJ1FTYQJFR37Z4ZTY
created_at: 2026-11-18T03:05:12.728114811Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready 2026-11-17: all six plan steps are implemented and the review-ready runs are green on commit 6c882c9e. Level with origin/main (661fba18); `git merge-tree --write-tree HEAD origin/main` is clean.

**Ran, once, on this tree**
- Static gates: `cargo check --all-features --locked`, `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`. All clean.
- Fast tier (the AGENTS.md list, with `repository::local_sync::` skipped): 6088 passed, 0 failed, 1 ignored.
- SSH tier, every harness target (`src/repository/transport/`, `tests/support/ssh_*` and `src/bin/manyhands-cli/` are touched): `ssh_fixture` 35, `ssh_transport` 124 (includes the new `verify_through_agent`), `cli_remote` 166, `remote_observation` 15, `remote_synchronization` 69, `remote_merge_recovery` 29, `mutation_remote` 808, `comment_publication` 41, `remote_polling` 86, `document_promotion` 78, `ticket_closure` 77. Every case passed.

**What the tests do and do not prove.** The agent was a fake one (a test client and server speaking the ssh-agent protocol on a Unix socket), driven through the real libgit2/libssh2 path against the russh fixture server. That pins: the selected key alone is offered, a decoy listed first is never asked to sign, the passphrase is never prompted for when the agent signs, and a refusing agent falls back to the key file with one prompt. It does not prove anything about a real `ssh-agent`, `ssh-add`, or macOS.

**Still open, for the owner**
- Real-agent check on Linux and then on the Mac: `ssh-add` a passphrase-protected key, make sure `SSH_AUTH_SOCK` is set, and run `manyhands-cli comment add --non-interactive` with no terminal. Expected: it publishes with no prompt. Then `ssh-add -D` and run again: it should answer `unlock_required`, as before.
- `skills/README.md` still says "Minimum CLI version: 0.2.0"; bump it to the release that ships this.
- Review and approval, then `ticket close` (which can now run unattended with the key loaded in ssh-agent).
- Optional follow-up: offer `Cred::ssh_custom` upstream to git2-rs so the one `unsafe` shim can be deleted.

**Unrelated observation.** `index status` flags `.manyhands/comments/01M7MKQFCHPEXQ58N77D63S417/01M7N0BQH1Z8GJBBY6NYK6.md` as `invalid_path`: its comment ID is 22 characters, not 26. It is tracked on main and did not come from this ticket.
