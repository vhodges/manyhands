---
id: 01M7NK5SBPQB28JA9T1BWN86XW
item_id: 01M67Y754SJ1FTYQJFR37Z4ZTY
created_at: 2026-11-17T20:29:05.383299821Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Spike result (2026-11-17, commit be9c4d4f): the approach works. Plan step 1 is done.

**Owner decisions, 2026-11-17**

- Keep the `unsafe` shim (option 1 in the plan): turn the raw libgit2 credential into a `git2::Cred`, with `git2` pinned to an exact version, a compile-time size/align guard and a layout test. The reason is that `Cred::ssh_key_from_agent` makes libgit2 try every key in the agent in the agent's order, which can authenticate as the wrong identity or exhaust MaxAuthTries. Offer an upstream `Cred::ssh_custom` to git2-rs later and delete the shim when it ships.
- A refused or timed-out sign falls back to the key file, and from there to today's behavior (a prompt at a terminal, `unlock_required` without one).

**What the spike proved**

- New SSH-harness case `agent_selected_key_only` (`tests/ssh_fixture.rs`) with a fake agent on a Unix socket (`tests/support/ssh_agent.rs`). The agent holds a decoy ed25519 key listed first and the selected key second. The fetch succeeds, the fixture server accepts only the selected key, and the agent saw exactly one sign request, for the selected key's blob. The decoy was never asked to sign. This case is the planned single SSH case, so it stays.
- The credential is built with `libgit2_sys::git_cred_ssh_custom_new`, a sign callback and a payload pointer. `libgit2-sys =0.18.8` is now a direct dependency. `signature` is a new dev-dependency for the fake agent.

**Findings that change the plan**

1. **libssh2 wants the bare signature blob.** It adds the algorithm name itself when it builds the auth packet. The agent answers `string(algorithm) + string(blob)`, so the callback must unwrap it and return only the blob (64 bytes for ed25519). Returning the agent's full answer failed with "remote rejected authentication: Failed getting response", which is the symptom to remember.
2. **`libgit2-sys` declares the sign callback as returning nothing.** The C type returns `int`, which libssh2 reads. The shim defines the callback with `-> c_int` and casts the function pointer. This is the second reason the shim needs a test, and it goes in the shim's safety comment.
3. **`git2::Cred` is a single `*mut git_cred`**, so the size/align guard compiles today. The shim itself lives in `tests/support/ssh_agent.rs` for now and moves into `src/repository/transport/` in step 3.
4. **RSA.** libssh2 tries `rsa-sha2-512,rsa-sha2-256` first and `ssh-rsa` second, and it never tells the callback which one it picked. The algorithm name is inside the data to be signed, so the callback could parse it and set the agent's sign flags. That is not proven here. First slice: ed25519 and ECDSA only, and an RSA selected key counts as "key absent" and behaves as today. RSA support is a follow-up.

**Checkpoint record**

- Ran: `cargo check --offline --test ssh_fixture`, `cargo clippy --offline --test ssh_fixture -- -D warnings` (clean), and `cargo test --offline --test ssh_fixture agent_selected_key_only -- --exact` (passed, the single case, three times as the spike was fixed and cleaned).
- Not run: the fast tier, any other SSH case, the static gates across the whole tree. Those run once at review-ready.
- Cargo.lock gained only `libgit2-sys` and `signature 2.2.0` entries (built with `--offline`, because `--locked` refuses a changed manifest).

**Next (plan steps 2 to 4)**

Move the fake-agent protocol knowledge into the production agent client (`src/repository/transport/agent.rs`, Unix only), move the shim into `src/`, then wire the agent attempt into `with_authenticated_remote_policy_expected` ahead of the passphrase flow.
