---
id: 01M7NM93YJNSC7S84FBY28AW4W
item_id: 01M67Y754SJ1FTYQJFR37Z4ZTY
created_at: 2026-11-17T20:48:23.128076531Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Checkpoint 2026-11-17: plan steps 2 to 4 are implemented (commit a7b601aa). The agent route works end to end; steps 5 and 6 (messages, docs and policy text) remain.

**What changed**

- `src/repository/transport/agent.rs`: the agent client. It lists the agent's identities, matches the selected key's public blob (read from its private key file, which shows the public half even when encrypted), and signs only with that key. A closed set of unusable reasons (unset, unreachable, protocol, key unreadable, key type unsupported, key absent) with no agent detail in any of them. Bounded message size and timeouts (5 s to list, 30 s to sign, since a confirm-on-use agent may wait for a person). ed25519 and ECDSA only; an RSA selected key counts as unusable and behaves as before.
- `src/repository/transport/agent_credential.rs`: the one `unsafe` bridge, built from the spike. `git2` is now pinned `=0.20.4`, and there is a const size/align assertion and a unit test that the wrapped pointer is a custom-signing credential that drops cleanly.
- `operation.rs`, `callbacks.rs`, `remote.rs`: the agent attempt runs before the passphrase flow, and the same agent credential is used on the reconnects a synchronization makes. A refused sign or a rejected key falls back to the key file and today's prompt. A failure that is not about the key (host trust, network) ends there, since the key file flow would only repeat it.
- `RepositoryService` captures SSH_AUTH_SOCK when it opens (`with_agent_socket_for_testing` for tests).

**Tests**

- Four library unit tests (`repository::transport::agent` and `agent_credential`): a table of every unusable reason plus the success case, a table of sign outcomes (bare blob, agent refuses, agent names another algorithm), the decoy-never-signs check, and the credential wrapping test. They use one shared fake agent (`src/repository/transport/fake_agent.rs`).
- One SSH-harness case, `verify_through_agent` in `tests/ssh_transport/session.rs`, replacing the spike case. A passphrase-protected key whose unlocked form is only in the agent (decoy listed first): verify succeeds with zero prompts and exactly one sign request for the selected key. A second agent that refuses to sign then falls back to the key file and prompts once. The agent protocol and failure details stay in the unit tests.

**Checkpoint record**

- Ran: `cargo clippy --all-targets --all-features -- -D warnings` (clean, whole tree), `cargo fmt`, the four new unit tests, `--lib repository::transport::` (37 passed, includes the call sites I edited in `tests.rs`), and `--test ssh_transport verify_through_agent -- --exact` (passed).
- Not run: the fast tier and the SSH tier. `src/repository/transport/` and `tests/support/ssh_*` are both touched, so review-ready needs every SSH-harness target (about 1 h 36 min), once.
- Not yet verified: a real Mac and a real `ssh-agent` (the only agent so far is the fake one). Worth a manual check at review-ready: `ssh-add` a protected key, set `SSH_AUTH_SOCK`, and run `manyhands-cli comment add` non-interactively.

**Still to do**

5. The `unlock_required` note should say `ssh-add` lets unattended runs proceed.
6. Amend docs/Cycles/wave-02-cycle-03-authenticated-ssh-transport.md, the authentication RFC and docs/RFC/test-and-compatibility-strategy.md; update the CLI skill and user docs (include the macOS keychain note); then the AGENTS.md follow-on to direct agent comments to `comment add`.
