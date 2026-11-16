---
slug: vh-grthf
id: 01M7K13QGQ7CH3C9AG1AAF3MJB
title: 'One global SSH key for all repositories: support a key per repository or host'
type: gap
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Manyhands uses one SSH key for every repository on a machine. There is no way
to give a repository, or a remote host, a key of its own.

**What the code does.** The registry (`shared_ssh_keys`) holds any number of
registered keys, but exactly one is `selected`; `select_shared_key` clears the
flag on all the others. `prepare_ssh` in `src/repository/transport/operation.rs`
takes that selected key for every connection, whichever repository or host it
is for. `.manyhands/config.toml` names a `publication_remote` but no key, and it
is committed and shared, so it could not hold a per-machine key choice anyway.
`SessionCredentials` caches one passphrase (`cached: Option<CachedPassphrase>`),
so two keys unlocked in one session would evict each other.

**Why it matters.** Found 2026-11-16 on a work Mac. The owner's repositories sit
on different servers: a work server where the key is registered, and the
Manyhands demo clone whose `origin` points at a host the key does not
authorize or that is unreachable. One key for both means the second repository
fails on every poll. That also led to the shared passphrase being evicted on
each failure (fixed in ticket 01M7JZTKVFBBQ7MDAXEYE03X5W, but the failing
repository still reports a rejected key each poll). People who keep work and
personal keys, or one deploy key per repository, cannot use Manyhands for both.

**Request.** Let a repository, and optionally a remote host, use a chosen key
instead of the one global selection.

To settle in planning:
- **Where the choice lives.** A machine-local mapping in the registry, keyed by
  canonical repository root or by remote authority (host and port), with the
  global selection as the default. Not in `.manyhands/config.toml`, which is
  shared and committed. Decide whether host-level or repository-level wins when
  both are set.
- **Passphrase cache.** One cached passphrase per key, each keyed by key id and
  source token as today, so an unlocked work key survives a failure on a
  personal-key repository.
- **Prompt and attention.** The unlock prompt and the needs-attention entries
  name the key and repository they are for; a background poll for key B must not
  be blocked by key A being locked.
- **Settings and CLI.** Per-repository selection in the desktop settings and a
  `key select --repo` form; show which key a repository uses in status output.
- **Existing behavior.** `NoSelectedKey`, `SelectionChanged` and `selection
  clears cache` semantics (`tests/ssh_transport/state.rs`) carry over per key.
- **Relationship to other tickets.** Overlaps ticket 01M7JJ02N7XR3T2D6XT7BMV7TE
  (more transports and authentication methods, including an SSH agent or no
  managed key); decide whether a per-repository credential belongs to a shared
  credential-selection design.
- **Migration.** Registry schema change; per AGENTS.md notes, ticket-branch
  builds migrate one-way, so plan the migration and its test.

Test plan, per the Testing Strategy: decision logic for resolving which key a
repository uses as library unit tests with a registry in a tempdir; the
per-key cache as unit tests; at most one SSH-harness case, an end-to-end check
that two repositories on two fixtures use two keys in one session.
