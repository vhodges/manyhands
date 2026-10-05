---
title: "Wave 02 Cycle 01: Shared-Key Registry"
date: 2026-10-04
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01M43H1YNZDGGED5A33T8WC6CQ"
---

# Wave 02 Cycle 01: Shared-Key Registry

## Parent Wave

This is Cycle 01 of [Wave 02: Collaboration](../Waves/wave-02-collaboration.md).
It establishes the application-local, non-secret SSH-key registry that later
cycles use for key material, authenticated transport, and remote operations.

## Purpose

Provide a headless, user-local registry for SSH-key metadata and one selected
shared key. The registry records imported or generated ownership, labels,
absolute source paths, optional public-key metadata, and non-secret observation
state without copying, parsing, unlocking, or deleting private-key material.

## Prerequisites

- The approved PRD, MVP, authentication, Git workflow, repository/index, and
  test-strategy RFC amendments remain mutually consistent.
- This Cycle document and its implementation plan have been approved.
- The mandated Wave 01 verification suite succeeds against the current source
  and lockfile before any Cycle 01 implementation change begins.
- Implementation starts in this ticket's rebased worktree and branch.

## Traceability

| Source | Cycle responsibility |
| --- | --- |
| [Authentication and Credential Handling RFC](../RFC/authentication-and-credential-handling.md) | Persist non-secret user-local registration metadata, preserve imported-key ownership, select one shared key, and provide recovery guidance without secret disclosure. |
| [Repository Index Persistence and Refresh RFC](../RFC/repository-index-persistence-and-refresh.md) | Use application-local SQLite state while keeping Git and tracked content outside the registry boundary. |
| [Test and Compatibility Strategy RFC](../RFC/test-and-compatibility-strategy.md) | Prove deterministic metadata, selection, non-deletion, recovery, and absence of secret persistence. |
| `MH-CRED-001` and `MH-NFR-003` | Manage a selected SSH key without persisting private-key contents, passphrases, or credentials. |

## In Scope

- An application-global SQLite registry owned by the existing headless
  `RepositoryService`.
- Registering, listing, selecting, deselecting, and unregistering imported or
  generated key metadata.
- Required labels, opaque registration IDs, imported/generated ownership,
  absolute lexically normalized private-key paths, optional public-key paths,
  optional parsed public-key fingerprints, and non-secret observation state.
- A database-enforced invariant that at most one key is selected.
- Path-only uniqueness: a second registration for the same normalized private
  source path is rejected, while labels and public fingerprints may repeat.
- Best-effort companion public-key parsing. A missing or malformed companion
  preserves the registration and reports fingerprint-unavailable guidance.
- Best-effort private-source observation without reading private-key contents.
  Missing, inaccessible, or non-regular sources remain registered with typed
  recovery guidance.
- Explicit deselection or replacement before unregistering a selected key.
- A non-persistent generated-key deletion preflight that requires caller
  confirmation but never changes a key file.
- Focused SQLite and domain tests, including raw database and WAL checks that
  prove a private-file sentinel never persists.

## Out of Scope

- Key generation, private or public key-file writes, passphrase acquisition,
  in-memory secret storage, private-key format validation, and key deletion.
- Git-over-SSH callbacks, host verification, fetch, push, polling,
  synchronization, merge, promotion, ticket closure, or network operations.
- Any tracked configuration, canonical Markdown, Git configuration, remote, or
  worktree mutation.
- SSH-agent, default-key, credential-helper, system-Git, or HTTP(S) fallback.
- Persisted deletion authorization, durable deletion recovery, or background
  source monitoring. Cycle 02 owns generated material and destructive deletion;
  Cycle 03 owns first transport validation.

## Contract

The registry is user-local rather than repository-local. It stores only opaque
registration IDs, labels, ownership, normalized paths, optional public
fingerprints, selection, and non-secret state. It never stores private-key
bytes, passphrases, callback values, credential material, remote responses, or
public-key comments.

Registration requires an absolute path and normalizes only syntactic `.` and
`..` components; it never resolves symlinks or calls filesystem canonicalization
for identity. This lets a missing imported source remain a stable registration.
The source-path uniqueness rule is therefore deterministic even while a device
or mount is unavailable.

A companion public key is the only key file Cycle 01 reads. It is parsed as an
OpenSSH public key and stored as its standard SHA-256 fingerprint. An invalid,
missing, inaccessible, or unsupported companion never exposes its body in an
error or database field. Private-key format and backend compatibility remain
unknown until Cycle 03's selected-key transport operation.

Selection replacement and deselection are atomic SQLite transactions. Removal
of a selected registration returns a typed action to clear or replace it first.
Imported-key unregistration removes the local row only. Generated-key deletion
preflight rejects imported or selected keys and otherwise returns a fresh
confirmation-required outcome without persisting authorization or touching the
filesystem.

## Planned Changes

| Path | Change |
| --- | --- |
| `Cargo.toml` and `Cargo.lock` | Add the stable `ssh-key` public-key parser for OpenSSH companion metadata and SHA-256 fingerprints. |
| `src/repository.rs` | Add public shared-key registry IDs, request/outcome types, secret-safe errors, path/state helpers, and transactional `RepositoryService` methods. |
| `src/repository/discovery.rs` | Add idempotent application-global shared-key schema and its one-selected-key partial unique index. |
| `tests/repository_enablement.rs` | Extend schema and migration-idempotence assertions. |
| `tests/shared_key_registry.rs` | Add focused domain and persistence coverage for the Cycle contract. |

## Exit Evidence

- Registration metadata is unique by normalized private source path and survives
  reopening the application-local registry.
- Selection is atomically unique and a selected key cannot be unregistered
  before explicit deselection or replacement.
- Imported-source files are never deleted or modified by registration or
  unregistration.
- Generated deletion requires a fresh, non-persistent confirmation preflight.
- Missing or unusable private sources and malformed public companions remain
  discoverable with non-secret recovery guidance.
- Private-file sentinel text, passphrase sentinels, and public-key comments do
  not occur in registry rows, SQLite files, WAL files, errors, or outcomes.
