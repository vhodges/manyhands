---
slug: vh-0ry0a
id: 01M6K42GPWY8GSCXX8MXXKE462
title: Let agents act under their own registered identities
type: exploration
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Support several registered identities, and optionally keys, so that agents can perform operations under their own identity.

## Today

- **One effective identity per repository.** Commits use Git configuration (`user.name`/`user.email`), resolved by `effective_identity` (`src/repository/remote/sync.rs`) and `resolve_identity` (`src/repository.rs`). `repo identity-set` writes repository-local config; there is no list of identities to choose from.
- **The only per-caller value** is `comment add`'s `created_by` (C4 decision 9), written into the comment file; its commit still uses the configured identity.
- **Lifecycle verbs** use the configured identity only (C5 decision 6). Closure validation requires `closed_by` to equal the closure checkpoint's signature (`src/repository/closure/checkpoint.rs`), and the consent digest binds `closed_by`.
- **The library's confirmed identity** only fills a missing identity, and persists to repository config.
- **Keys:** one user-local selected key for every Git-over-SSH operation, at most one selected (`docs/RFC/authentication-and-credential-handling.md`). Related: `01M67Y754SJ1FTYQJFR37Z4ZTY` (ssh-agent).

## Workaround

Separate local OS users, each with its own clone, registration and key.

## Explore

- A named identity per request, chosen without writing repository config, so that concurrent agents don't overwrite each other; commits, `closed_by` and comment authors all from it.
- Lifecycle consent: the identity chosen at prepare and carried to confirm, keeping closure's signature rule.
- Optionally, a key per identity or per request, which changes the authentication RFC's one-selected-key rule.
- Interaction with recovery records, replay and `effective_identity`'s cascade (`01M6K08364Z6VX2TYVZD86PG1Q`).

Outcome: an RFC proposal for the owner, since this reverses C5 decision 6 and the one-key rule; not code.

Raised by the owner on 2026-11-04 after Wave 03 C5 (`01M4CC0VMXTNXNH78C11DYXS8H`).
