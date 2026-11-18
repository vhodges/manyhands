---
id: 01M7NFT0NDHGB2JQJC08WVBZNW
item_id: 01M67Y754SJ1FTYQJFR37Z4ZTY
created_at: 2026-11-17T19:30:51.683349672Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Plan (light, 2026-11-17). Nothing is implemented and no tests have run yet. Built from reading `src/repository/transport/{callbacks,operation}.rs`, git2 0.20.4 and libgit2-sys 0.18.8 (libgit2 1.9.7).

**Approach: our own agent client plus a custom-signing credential**

- `git2::Cred::ssh_key_from_agent` cannot meet "never offers the agent's other keys". libgit2's `ssh_agent_auth` lists every identity and calls `libssh2_agent_userauth` on each in turn, with no filter. It also reads SSH_AUTH_SOCK itself, so there is no way to hand it a different socket.
- libgit2 does have the right primitive: `git_credential_ssh_custom_new(username, publickey, sign_callback, payload)`, which calls `libssh2_userauth_publickey`. libssh2 then offers only the one public key we give it, and calls our callback to sign the challenge. `libgit2-sys` declares it (`git_cred_ssh_custom_new` and `git_cred_sign_callback`). `git2` 0.20.4 does not expose it, and it has no public way to wrap a raw credential (`Binding` is private).
- So: a small Unix agent client (list identities, match the selected key's public-key blob, sign) and a credential built from the selected key's public blob whose sign callback asks the agent. The selected key's `.pub` path and fingerprint are already in the registry, so the match is by exact blob.

**Decision for the owner: how to turn the raw credential into a `git2::Cred`**

1. Recommended: a small `unsafe` shim that wraps the raw pointer, with `git2` pinned to `=0.20.4`, a compile-time size/align assertion, and a test that fails if `Cred`'s layout ever changes. Then offer `Cred::ssh_custom` upstream to git2-rs and delete the shim when it ships. `Cred` is a single-pointer struct today, but Rust does not guarantee that layout, hence the pin and the guard.
2. A filtering proxy agent on a private socket, reached by setting SSH_AUTH_SOCK in-process. Pure safe `git2`, but `setenv` in a multithreaded process is unsound and the desktop app is threaded. Not recommended.
3. Vendor or patch `git2` locally. Safe, but more maintenance than option 1.

Step 1 below is a spike that proves option 1 before anything else is built.

**Steps**

1. **Spike (decides the approach).** Against the russh fixture server, authenticate with a custom-signing credential for an ed25519 key whose private half is only in a fake in-process agent. Pass means the handshake succeeds and the fake agent saw exactly one sign request for the selected blob. Also find out what libssh2 requires of an RSA signature (rsa-sha2-256/512 against ssh-rsa) so the first slice can state which key types it supports. Ed25519 is what Manyhands generates.
2. **Agent client** (`src/repository/transport/agent.rs`, `cfg(unix)`). Read SSH_AUTH_SOCK once at the service edge and pass the path in, so tests inject a path and never touch the environment. Request identities, match, sign. Bounded message sizes, connect/read timeouts (a confirm-on-use agent may take seconds to approve a sign), and no key bytes, comments or paths in any error or log. Outcomes are a closed set: unset, unreachable, key absent, present. On non-Unix it reports unavailable, so Windows behaves exactly as today.
3. **Credential and callbacks.** Add an agent mode to `CallbackAttempt::credentials` next to the key-file mode, sharing the one-submission rule (`key_submissions`) and the endpoint and host-trust checks, which stay unchanged. The sign callback catches panics (none may unwind into C) and allocates the signature the way libssh2 expects.
4. **Operation flow** (`with_authenticated_remote_policy_expected`). Before the existing first no-passphrase connection, if the agent outcome is present, connect with the agent credential. Success returns and the passphrase provider is never called, which is what removes `unlock_required` and the held remote reservation for the unattended case. If the agent is unset, unreachable or lacks the key, nothing changes from today. If the agent connection fails for authentication (for example the agent refuses to sign), fall back to today's flow. Settle in review whether a refused sign should fall back or stop, since a prompt cannot help a confirm-on-use denial.
5. **Messages.** When a non-interactive run still ends in `unlock_required`, the note (`UNLOCK_NOTE` in `src/bin/manyhands-cli/polling.rs`, plus the other CLI spots) should say that loading the key with `ssh-add` lets unattended runs proceed. That is the actionable hint for an LLM agent. No agent detail beyond that.
6. **Docs and policy amendment.** Amend docs/Cycles/wave-02-cycle-03-authenticated-ssh-transport.md and the authentication RFC text ("no agent, default-key, helper, password or anonymous fallback"; "wrong, absent, default, or SSH-agent key" in docs/RFC/test-and-compatibility-strategy.md) to say the agent holding the selected key is allowed and nothing else changed. Update the CLI skill and user docs, including the macOS keychain note from the earlier comment. Follow-on from the ticket: AGENTS.md directs agent comments to `comment add`.

**Tests (Testing Strategy: cheapest layer first)**

- **Library unit tests, one table-driven test per concern**, using a fake agent on a tempdir Unix socket: identities parsing; the SSH_AUTH_SOCK unset, socket missing, dead socket, empty agent, agent lacks the key, agent answers FAILURE, malformed or oversized reply and timeout cases; and the agent holding several keys where the fake records every sign request and the test asserts only the selected blob was ever signed (the "never offers other keys" criterion). Plus the operation-flow decision: agent present means no provider call, anything else means today's flow.
- **One SSH-harness case, by exception.** It needs SSH because the sign-callback wire format and libssh2's use of it can only be proven by a real handshake. A protected selected key, its private half only in the fake agent, non-interactive verify succeeds with zero passphrase-provider calls. A second fixture variant for an agent that lacks the key is not added: that path is today's flow and is covered by the unit table.
- **Existing coverage stays valid.** `tests/support/ssh_harness.rs` already points SSH_AUTH_SOCK at a nonexistent socket, so the existing "agent environment is ignored" cases still pass for the right reason (no agent). Review them in step 6 for wording that now contradicts the policy.
- **Layout guard test** for the `Cred` shim (option 1).

**Checkpoints (record here as they happen)**

- Between steps: `cargo check`/`clippy` and only the tests just written, by exact name. Never a whole SSH target between steps.
- Review-ready: static gates, the fast tier once, and the SSH tier for `src/repository/transport/` and `tests/support/ssh_*`, meaning every SSH-harness target (about 1 h 36 min, run once).

**Risks and limits**

- Linux and macOS only in this slice. Windows (OpenSSH named pipe, Pageant) is a follow-up. The macOS keychain behavior is untested until the owner tries a Mac.
- RSA agent keys depend on the spike's finding; the fallback is to treat an RSA key as "absent" so it behaves as today.
- The shim is the one `unsafe` addition. It is small, guarded and temporary.
- The registered key must have a readable public key (`public_metadata_state` available). If not, the agent route is skipped and today's flow runs.

**Questions for the owner**

1. Approve option 1 (shim, pinned git2, upstream follow-up)?
2. On a refused or timed-out sign, fall back to the key file or stop with a distinct error?
