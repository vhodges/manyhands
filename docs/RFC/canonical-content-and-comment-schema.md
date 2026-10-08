---
title: "Canonical Content and Comment Schema RFC"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YQ1Z2V6B8N4M3R5T7W9X0A"
---

# Canonical Content and Comment Schema RFC

## Summary

This RFC defines the version 1 canonical representation for enabled-repository
configuration, managed documents, tickets, and comments. Markdown and tracked
repository configuration are canonical; the local SQLite index defined by the
repository/index RFC is only a rebuildable cache.

This RFC implements the representation decisions assigned to it by the approved
[MVP architecture RFC](mvp-rfc.md). It is a Wave 1 prerequisite for local
content, context identity, checkpointing, and discovery work.

## Scope

This RFC specifies:

- The tracked enabled-repository marker and configuration.
- Paths and stable identities for documents, tickets, and comments.
- Required Markdown front matter and validation behavior.
- Comment threading and deterministic ordering.
- Explicit migration of existing marker-only documents.

It does not specify Git branch operations, SQLite tables, SSH credentials,
desktop interaction, or CLI grammar.

## Normative Terms

The terms **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative. A
conforming implementation MUST preserve unknown front-matter keys and
user-authored Markdown body content during a read-modify-write operation.

The [desktop/editor RFC](desktop-information-architecture-and-editor.md),
approved on 2026-10-05, applies that preservation contract to rich-text and
source modes. Viewing, switching modes and no-change saves preserve original
source. Rich-text editing retains unsupported constructs and untouched source;
it does not silently reserialize or simplify an entire document. Actual metadata
edits preserve unknown values even when YAML formatting changes. As amended on
2026-10-07, a desktop save after an actual edit may also write the selected
editor's recorded, meaning-preserving load normalization of the body; the
desktop/editor RFC bounds that exception. It does not apply to the CLI or the
headless library.

Browsing and repair MUST NOT silently change identity. Adoption of marker-only
content explicitly previews any generated ID; existing valid IDs remain stable.
Repair cannot bypass uniqueness, closure fields or the no-reopening boundary.
These interface rules introduce no canonical schema or ID migration.

## Repository Configuration

An enabled repository MUST contain this tracked file:

```text
.manyhands/config.toml
```

The containing `.manyhands` directory is the repository marker. Its tracked
contents are canonical. The local worktree directory described by the Git RFC
is not canonical and is excluded separately.

Version 1 configuration has this shape:

```toml
format_version = 1
primary_branch = "main"
publication_remote = "origin"
```

`format_version` and `primary_branch` are required. `publication_remote` is
optional and is omitted for a local-only repository. `primary_branch` and
`publication_remote` are nonempty Git reference and remote names respectively.
Unknown configuration keys MUST be retained when rewriting the file so future
versions can add settings safely.

The optional keys `ticket_slug_prefix` and `ticket_slug_code_length` are
defined by the [ticket relationships and short codes RFC](ticket-relationships-and-short-codes.md).
Both may be absent.

An absent, malformed, unsupported-version, or invalid configuration is a
visible repository problem. It MUST NOT cause Markdown content to be omitted or
rewritten.

## Canonical Paths

Version 1 uses the following paths relative to repository root:

```text
.manyhands/config.toml
.manyhands/tickets/<ticket-ulid>/ticket.md
.manyhands/comments/<item-ulid>/<comment-ulid>.md
docs/**/*.md
```

`docs` is lower-case and is the only managed-document root in version 1.
Managed document filenames and subdirectories are user chosen. Moving a
document changes its path but MUST NOT change its ID. Tickets use a stable
directory because future attachments can be added without changing the ticket
path. Attachments are out of scope for Wave 1.

`.manyhands/worktrees/` is a local implementation directory, not canonical
content of the primary context. The primary-context scan MUST NOT recurse into
it. Active Manyhands worktrees MUST instead be enumerated and scanned
independently as editing contexts.

## Identifiers

Every document, ticket, and comment uses a canonical ULID. The serialized form
MUST be the 26-character, uppercase Crockford Base32 representation. IDs are
globally unique across all item kinds in a repository.

The ID is a stable identity, not a display label. It MUST NOT change when an
item is renamed, moved, promoted, closed, or copied into an editing context.
New IDs are generated only when a user creates a new canonical item or comment.

## Markdown Front Matter

Canonical item and comment files use YAML front matter delimited by `---`.
Required fields are listed below; values not listed here are permitted and MUST
be preserved.

### Managed Documents

Managed documents live under `docs/` and require:

```yaml
---
manyhands_managed: true
manyhands_kind: document
id: "01K6YQ1Z2V6B8N4M3R5T7W9X0A"
title: "Document title"
---
```

`title` is a nonempty string. The Markdown body after the front matter is the
document's canonical body. A separate description field is not required.

### Tickets

Tickets live at `.manyhands/tickets/<id>/ticket.md` and require:

```yaml
---
manyhands_managed: true
manyhands_kind: ticket
id: "01K6YQ2A4D8F1H3J5K7M9N0P2Q"
title: "Example ticket"
type: "feature"
status: "open"
project: "manyhands"
team: "core"
---
```

`title`, `type`, and `status` are required nonempty strings. `project` and
`team` are optional nonempty strings. The Markdown body is the optional ticket
description. Type and status are free-form project values; version 1 does not
define a controlled vocabulary.

A closed ticket additionally has both fields below. Their absence means the
ticket is open, regardless of its free-form `status` value.

```yaml
closed_at: "2026-09-30T12:34:56Z"
closed_by: "Vince Hodges <vhodges@gmail.com>"
```

`closed_at` is an RFC 3339 UTC timestamp. `closed_by` is the confirmed Git
identity used for the closing checkpoint. The closure lifecycle owns changes to
these fields.

A ticket may also carry the optional `deps`, `parent` and `slug` fields
defined by the [ticket relationships and short codes RFC](ticket-relationships-and-short-codes.md).
They never decide conformity, they do not replace the ULID as identity, and
that RFC owns their validation.

### Comments

Comments live at `.manyhands/comments/<item-id>/<comment-id>.md` and require:

```yaml
---
manyhands_managed: true
manyhands_kind: comment
id: "01K6YQ3B6E9G2J4K6M8N0P2R4S"
item_id: "01K6YQ1Z2V6B8N4M3R5T7W9X0A"
created_at: "2026-09-30T12:35:00Z"
---
```

`item_id` identifies a document or ticket. Root comments omit `parent_id`.
Replies include `parent_id` with another comment ULID for the same `item_id`.
`created_at` is an RFC 3339 UTC timestamp set when the comment is created and
never rewritten. The Markdown body is the comment content.

Comments form a tree of arbitrary depth. Root comments sort by `created_at`,
then `id`; direct replies use the same ordering beneath their parent. A missing
parent, cross-item parent, cyclic parent relationship, duplicate ID, or invalid
timestamp is nonconforming.

## Validation and Recovery

Validation is non-destructive. The scanner MUST report these conditions as
visible problems rather than hide or repair files:

- Content outside an allowed canonical path.
- Missing, malformed, or unsupported front matter.
- A required field with the wrong type, an empty value, or an invalid ULID.
- A kind/path mismatch, such as a ticket outside its ticket directory.
- Duplicate item IDs inside one scanned context.
- Comments whose item or parent cannot be resolved in that context.

Existing files that contain only `manyhands_managed: true` are discovered as
nonconforming documents. A repair workflow MAY suggest a title or generate a
ULID, but it MUST write the new metadata only after an explicit user edit and
normal checkpoint lifecycle. Indexing and refresh MUST NOT silently upgrade
them.

## Migration and Compatibility

`format_version = 1` is the only version supported by this RFC. Later schema
versions MUST provide an explicit compatibility and migration decision before
rewriting existing canonical content. A newer configuration or unknown required
content version is a visible problem and preserves all files unchanged.

The schema does not import arbitrary Markdown outside `docs/`, migrate a
capitalized `Docs/` directory, infer IDs from paths, or infer ticket status from
filenames. Those actions require a later approved migration RFC.

## Contracts With Other RFCs

The Git workflow RFC consumes item kind and ULID to derive
`manyhands/<kind>/<id>` context branches. It creates and checkpoints only the
canonical paths defined here.

The repository/index RFC validates these rules while scanning primary and active
worktree contexts. It records a nonconforming problem without changing files.

The test and compatibility RFC MUST include fixtures for every required field,
unknown-key preservation, ID stability across moves and contexts, invalid
comments, and explicit repair of a marker-only document.

## Wave 1 Acceptance

Wave 1 schema work is complete when automated tests demonstrate that:

- A valid configuration, document, ticket, and threaded comment tree parse and
  round-trip without body or unknown-key loss.
- Tickets and documents retain the same ULID when copied into an editing
  context, and documents retain it after a path move.
- Invalid and marker-only content is visible as a problem and is never silently
  upgraded or omitted.
- Comment root and reply order is deterministic after a full index rebuild.
