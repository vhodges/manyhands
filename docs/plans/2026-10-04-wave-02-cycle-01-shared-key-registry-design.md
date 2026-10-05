---
title: "Wave 02 Cycle 01 Shared-Key Registry Design"
date: 2026-10-04
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01M43H1YNZXXZBQ166ZJWT536S"
---

# Wave 02 Cycle 01 Shared-Key Registry Design

## Goal

Extend the shared headless domain with a user-local, non-secret SSH-key registry
that stores one selected shared key without adding key material, transport, or
repository mutation.

## Authority

This design implements the proposed [Cycle 01 authority](../Cycles/wave-02-cycle-01-shared-key-registry.md)
and the approved authentication RFC. It preserves one application-global shared
key, imported-key ownership, no private-key parsing before transport, and the
no-secret persistence boundary.

## Module Boundary

`RepositoryService` already owns the data directory, registry opening, schema
migration, SQLite transactions, and cross-process cache lock. Cycle 01 extends
that service rather than creating a second application-state database. Its
shared-key APIs do not accept a repository root and do not inspect or mutate Git
or tracked content.

## Persisted Model

`shared_ssh_keys` is an application-global table. It has an opaque ULID primary
key, nonempty label, `imported` or `generated` ownership, normalized private-key
path, optional normalized public-key path, optional SHA-256 public fingerprint,
private-source observation, public-metadata observation, and `selected` flag.

The normalized private-key path is unique. A partial unique index on
`selected = 1` makes the at-most-one selection invariant durable and race safe.
Registration normalizes syntactically without resolving symlinks or requiring
the path to exist. Source observation uses filesystem metadata only, so the
private file is never opened or parsed.

## Public Contract

The service exposes typed IDs, ownership, observations, request structures, and
outcomes. A caller can register, list, select, clear selection, unregister, and
request a generated-key deletion preflight. Selection replacement is atomic.
Unregistering a selected key returns an explicit deselect-or-replace outcome.

Public companion parsing uses `ssh-key` to read an OpenSSH public-key file and
persist its SHA-256 fingerprint only. A companion failure is represented by
public-metadata-unavailable state, not an error containing file contents. A
private source that is missing, inaccessible, or not a regular file remains
registered with recovery guidance. Private-key format validation is deferred to
the selected-key transport work in Cycle 03.

The generated-key deletion preflight is intentionally ephemeral. An unselected
generated registration returns confirmation-required; an imported, selected, or
unknown key returns a typed non-destructive outcome. Cycle 02 must obtain fresh
confirmation immediately before it deletes any owned file.

## Failure And Privacy Boundary

All validation errors use stable categories and fixed guidance. They do not
include private-key bytes, passphrases, public-key comments, raw parser errors,
or arbitrary filesystem error strings. The registry may retain paths and parsed
public fingerprints because the approved RFC defines them as non-secret metadata.

The existing corrupt-registry replacement behavior can lose key registrations
because they are intentionally local state. Cycle 01 does not alter that
recovery mechanism. Its guidance must tell callers that source files remain
untouched and that registrations can be recreated; it must not claim that the
registry is rebuildable from Git.

## Tests

Focused integration tests use `RepositoryService::open_at(tempdir)` and fake
private files containing unique secret sentinels. They verify durable path-only
uniqueness, atomic selection, explicit deselection, imported-file preservation,
generated deletion preflight, source and companion recovery, and re-opened
registry state. Raw SQLite and `-wal` bytes are scanned for the sentinels and
public-key comments to prove the service never read or persisted them.
