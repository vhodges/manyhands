---
id: 01M7NQ8JE9MFKH8428R660RQBK
item_id: 01M67Y754SJ1FTYQJFR37Z4ZTY
created_at: 2026-11-17T21:40:30.40143703Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint 2026-11-17: plan steps 5 and 6 are done (commit 685a1b74). All six plan steps are implemented; what remains is review-ready verification and a real-agent check.

**Step 5, the unlock note.** The stopped-poll note (`UNLOCK_NOTE`, `src/bin/manyhands-cli/polling.rs`) now reads "polling again takes a new request, made at a terminal, where it can ask for the passphrase, or once the key is loaded in ssh-agent (ssh-add), which needs no passphrase." Its one golden (`tests/fixtures/cli_v1/poll_once_unlock_stop_text.txt`) follows. Other `unlock_required` wording is unchanged: the generic "The key must be unlocked." text is pinned in several places, and the passphrase prompt for `key generate` is about a new passphrase, which the agent cannot supply.

**Step 6, docs and policy.**
- Amended: the Cycle 3 scope and its evidence row, the authentication RFC (summary, callback rules, test contract, acceptance, a new "Agent Signing" section, and a decision-log row), the test and compatibility strategy, and the Wave 02 and Cycle 01 scope lines. Each says an agent may sign for the selected key and for no other, and that default-key, helper, password and anonymous fallbacks stay refused. The historical plans under `docs/plans/` are left as they were.
- Skills: `manyhands-keys-and-hosts` has a new section "A passphrase key through ssh-agent" (the route, `SSH_AUTH_SOCK` in the agent's environment, selected key only, ed25519/ECDSA on Linux and macOS, the fallback, `ssh-add`, the macOS keychain note, and how to compare `ssh-add -l` against `key list` without changing anything). Its `unlock_required` rows and the overview skill point to it.
- AGENTS.md: the stale "comments remain file edits until C4's `comment add`" line now directs agent comments to `comment add`, with the ssh-agent route for a protected key, and notes that `ticket close` also works unattended with the key loaded.

**Checkpoint record**

- Ran: `cargo fmt --check` (clean), `cargo clippy --all-targets --all-features -- -D warnings` (clean), `--test cli skill` (8 skills tests pass), and `--test cli_remote c5_polling_unlock_stop_is_final_then_a_new_request -- --exact` (the golden case, passes).
- Not run: the fast tier, the rest of the SSH tier, the real-agent check. `src/repository/transport/`, `tests/support/ssh_*` and `src/bin/manyhands-cli/` are all touched, so review-ready means the static gates, the fast tier once, and every SSH-harness target once.

**For the release owner**

- `skills/README.md` states "Minimum CLI version: 0.2.0". The agent section describes behavior that 0.2.0 does not have, so the minimum should be bumped to the release that ships this ticket. I did not change it: the version number is a release decision.
