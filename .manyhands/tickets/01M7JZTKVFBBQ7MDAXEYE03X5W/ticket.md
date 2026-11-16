---
slug: vh-t62dj
id: 01M7JZTKVFBBQ7MDAXEYE03X5W
title: SSH transport prompts for a passphrase when the key is unencrypted, and a protected key is asked again on automatic polls
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
The SSH transport raises an "authentication ambiguous" passphrase prompt
whenever its first connection attempt fails, whether or not the key can take a
passphrase, and the prompt is not cached when the attempt then fails again.

Found 2026-11-16 after the v0.2.0 release, while demoing on a work Mac.

**Symptom 1: an unencrypted key prompts.** `with_authenticated_remote_policy_expected`
(`src/repository/transport/operation.rs`) tries the key without a passphrase.
If that fails with an authentication failure or a generic SSH error, it raises
`UnlockReason::AuthenticationAmbiguous` and asks the provider for a passphrase
(guidance: "The key may need a passphrase, the server may have rejected it, or
the connection may have failed"). The transport never checks whether the key is
encrypted, although `unlock_generated_key` (`src/repository/keys/inspection.rs`)
already skips its prompt for an unencrypted key. `SecretPassphrase::new` rejects
an empty string, so for an unencrypted key the prompt can never succeed. A
passphrase is cached only after a connection that succeeds (`with_passphrase`),
so the prompt returns on every sync that fails. The owner's mock key had an
empty passphrase and was not registered with the server, so every sync asked.

**Fix.** Before raising the ambiguity prompt, parse the key file. For an
unencrypted OpenSSH key, return the real transport error (key rejected or
connection failed) without prompting. When the format cannot be determined,
keep the prompt as now.

**Symptom 2 (to investigate): the automatic in-app poll of a
passphrase-protected key.** The owner reports that with a protected key the
automatic poll in the desktop app keeps asking for the unlock instead of
unlocking once and reusing the in-memory passphrase. The library cache works
for `observe_publication_remote` (`tests/ssh_transport/observation.rs`,
`unlock_session`) and the desktop worker keeps one `SessionCredentials` for its
whole run (`src/desktop/services/worker.rs`), so find where an automatic poll
loses or never fills the cache, and where a background job is meant never to
prompt (`DesktopProvider`, `src/desktop/services/prompts.rs`). Reproduce with a
test before changing anything.

Test plan: unit tests for the decision logic with a local key file on disk, no
SSH harness for edge cases. At most one SSH-harness case, if needed, for the
end-to-end check. Changes under `src/repository/transport/` need every
SSH-harness target before review-ready (AGENTS.md).
