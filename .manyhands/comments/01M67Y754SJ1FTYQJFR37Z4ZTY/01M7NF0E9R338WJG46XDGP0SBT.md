---
id: 01M7NF0E9R338WJG46XDGP0SBT
item_id: 01M67Y754SJ1FTYQJFR37Z4ZTY
created_at: 2026-11-17T19:16:23.91604144Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Planning notes from starting the "agent-only" version of this work on 2026-11-17 (a duplicate ticket was created by mistake and removed). Three additions to this ticket.

**Owner decisions, 2026-11-17**

- **Agent-only via SSH_AUTH_SOCK.** No `~/.ssh/config` parsing, so `IdentityAgent` and `UseKeychain` are not read. This settles the plan's open Windows question as far as the first slice goes: whatever libssh2 reaches through SSH_AUTH_SOCK is in, and Pageant or the OpenSSH named pipe is a follow-up unless the plan finds it free. A third-party agent (1Password and similar) works only when SSH_AUTH_SOCK points at it.
- **The use is unattended LLM agents driving `manyhands-cli`.** The CLI never prompts for a passphrase when it authenticates through the agent, and a run that needs a key file's passphrase prompts at most once.
- **The Cycle 3 policy is amended by this ticket.** docs/Cycles/wave-02-cycle-03-authenticated-ssh-transport.md and its plans say "no agent, default-key, helper, password or anonymous fallback", and tests check that a usable agent is ignored. Only the agent changes. Default-key, helper, password and anonymous fallbacks stay refused, and those texts and tests must be updated, not left contradicting the code.

**macOS keychain finding**

- The production transport is libgit2 (`git2`, credential callbacks in `src/repository/transport/callbacks.rs`), not the system `ssh`. It never reads `~/.ssh/config`, so `UseKeychain yes` cannot affect it. (`russh` is only the test fixture server.)
- `UseKeychain` is an Apple-only OpenSSH option: Apple's `ssh` fetches the passphrase from the keychain and loads the key into the agent. The agent does not read the keychain itself (`ssh-add --apple-load-keychain` does it by hand).
- So on macOS the agent route works when the key is already in the launchd-managed agent, which GUI apps and shells reach through SSH_AUTH_SOCK. A key that is only in the keychain and was never loaded shows as no identities. Needs a check on a real Mac (the owner's work machine), because planning is on Linux.

**Planning point: pinning the identity**

`git2::Cred::ssh_key_from_agent` takes only a username and tries the agent's keys in order, so it cannot meet "never offers the agent's other keys" and can hit the server's MaxAuthTries. Settle in the plan whether to write a small agent client (list identities, match the selected public key, sign only for that one) or find a libgit2 or libssh2 route that filters.
