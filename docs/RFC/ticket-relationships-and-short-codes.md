---
title: "Ticket Relationships And Short Codes RFC"
date: 2026-10-07
status: approved
author: "Vince Hodges <vhodges@gmail.com> && Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4CGDAQ56ZKTYHX5VVJ0KWJ0"
---

# Ticket Relationships And Short Codes RFC

## Status and intent

This RFC adds three optional ticket fields and the queries built on them:
blocking dependencies, a parent for hierarchy, and a short code people can
read, say and search for. It implements `MH-CONTENT-005` and `MH-CONTENT-006`
in [PRD v0.6](../PRD/mvp.md).

The product owner directed the scope on 2026-10-07, after a review of
[ticket-rs](https://docs.ticket-rs.io/blog/whitepaper) and
[Beads](https://github.com/gastownhall/beads), and decided that:

- The ULID stays the only canonical identity. The short code is a label for
  people. Agents and automation use ULIDs.
- A short code may collide. Collisions surface as several search results with
  enough context for a person to choose.
- The short code is creator initials plus five characters derived from the
  ULID, with an optional project prefix, for example `mh-vh-k9x2b`.

The product owner approved this RFC's detailed text on 2026-10-07. It extends the [canonical schema](canonical-content-and-comment-schema.md),
[CLI](cli-contract.md), [index](repository-index-persistence-and-refresh.md)
and [desktop](desktop-information-architecture-and-editor.md) RFCs; each
carries a pointer here. [Wave 03](../Waves/wave-03-dogfooding.md) assigns the
work to Cycles.

On 2026-10-08 the product owner authorized amendments that bring this RFC into
agreement with rulings made while implementing Wave 03 Cycle F1: a closed
dependency never blocks, so only a cycle among open tickets blocks; the
critical path is over plannable tickets; and the read details stated under
[Validation](#validation), [Queries](#queries) and [Index](#index).

## Canonical fields

All three fields are optional ticket front matter. A ticket without them is
conforming and behaves as it does today. They do not apply to documents or
comments, and they do not change `format_version`.

```yaml
---
manyhands_managed: true
manyhands_kind: ticket
id: "01K6YQ2A4D8F1H3J5K7M9N0P2Q"
title: "Example ticket"
type: "feature"
status: "open"
slug: "mh-vh-k9x2b"
parent: "01K6YQ0000000000000000000A"
deps:
  - "01K6YQ0000000000000000000B"
  - "01K6YQ0000000000000000000C"
---
```

### `deps`

`deps` lists the tickets that block this one. Each entry is the canonical ULID
of another ticket. The list is stored on the blocked ticket, so adding a
dependency edits only that ticket's file, in that ticket's editing context.

Manyhands writes `deps` as a YAML block sequence, one ID per line, sorted
ascending and without duplicates. One entry per line keeps two branches that
add different dependencies from conflicting on the same line. An empty list is
written by omitting the field. A hand-written flow sequence or unsorted list is
read normally and rewritten in the canonical form on the next save of the
ticket. The serializer re-emits all front matter on every save, so values are
preserved and formatting is not.

There is one dependency type, and it blocks. Non-blocking link types are
[future work](#deferred-and-rejected).

### `parent`

`parent` is the ULID of one other ticket. It groups tickets into a hierarchy
and never blocks: a child can be ready while its parent is open, and a parent
can be closed while children are open. To make work wait on other work, use
`deps`. A ticket has at most one parent. Depth is not limited.

Manyhands shows a ticket's parent and its direct children. It computes no
progress, count or status rollup; rollups remain a PRD non-goal.

### `slug`

`slug` is the ticket's short code. Its grammar is:

```text
slug     = [ prefix "-" ] initials "-" code
prefix   = 1*8 ( lowercase-letter / digit )
initials = 2*3 ( lowercase-letter / digit )
code     = 5*8 crockford-base32-lowercase
```

`slug` is written once, when the ticket is created, and is never rewritten:
not when the ticket is renamed, when the creator's name changes, or when the
repository's prefix changes. It is matched case-insensitively and displayed in
lowercase. A slug written by hand with uppercase letters is accepted and read
in lowercase; the file is not rewritten.

The parts are derived as follows.

- **Prefix.** The optional `ticket_slug_prefix` key in
  `.manyhands/config.toml`. When the key is absent the slug has no prefix.
- **Initials.** The repository-local Git configuration value
  `manyhands.initials` when set. Otherwise the first character of the first
  and of the last whitespace-separated word of the confirmed Git identity
  name, lowercased. A one-word name contributes its first two characters. If
  that does not yield two ASCII letters or digits, creation returns recovery
  asking for explicit initials; Manyhands does not invent them. Initials name
  a namespace, not a person: two people may share them, and an agent uses the
  initials of the identity it commits as.
- **Code.** The BLAKE3 hash of the ticket's 26-character canonical ULID text,
  taking the leading 25 bits and writing them as five lowercase Crockford
  Base32 characters. `ticket_slug_code_length` in `.manyhands/config.toml`
  may raise the length to between 6 and 8 characters, taking 5 bits per
  character from the same hash; it defaults to 5 and applies to tickets
  created afterwards.

The code is deterministic for a given ULID and length, but the whole slug is
not: the initials and prefix come from who created the ticket and where. That
is why the slug is stored and not recomputed.

A ticket created before this RFC has no `slug`. It stays valid and is shown by
its ULID. A person can give it one with the explicit assign operation below,
which uses the assigning identity's initials.

## Identity, selection and collisions

The ULID is the identity. Nothing in this RFC changes that, and the approved
rule that mutations select items by full ULID stands.

- A slug is never accepted where an item ID is required. `--id` takes a ULID.
- A slug is a search key. Looking one up returns every ticket that carries
  it: none, one or several.
- Slugs are not required to be unique, and Manyhands does not reject, repair
  or rewrite a duplicate. Two tickets with the same slug are both conforming.
  A lookup that matches several returns all of them with title, type, status,
  closure state, context and ULID, so a person can choose. It is never
  resolved by guessing.
- `deps` and `parent` hold ULIDs, never slugs.
- Every machine-readable result that carries a ticket carries its ULID. The
  slug is an additional, nullable field.

With five characters there are about 33.5 million codes per prefix and
initials. The chance that any two tickets in one such namespace share a code
is about 0.4% at 500 tickets, 1.5% at 1,000 and 31% at 5,000. Because a
collision costs one extra choice in a search result and never misdirects a
write, five characters is the default. Six gives about 1.2% at 5,000.

## Validation

These fields follow the canonical RFC's non-destructive validation. A problem
is reported and visible; no file is repaired or rewritten by scanning.

| Condition | Result |
| --- | --- |
| `deps` or `parent` is the wrong type, or contains a value that is not a ULID | Visible problem on the ticket. The invalid value is ignored for graph queries. The ticket stays readable and editable. |
| A `deps` entry or `parent` names the ticket itself | Visible problem; the entry is ignored. |
| A `deps` entry repeats an earlier entry | Visible problem; the repeat is ignored. |
| A `deps` entry or `parent` names an item that is not a ticket | Visible problem; the entry is ignored. |
| A `deps` entry names a ticket not found in any scanned context | An **unresolved dependency**. It counts as blocking, and the ticket shows a diagnostic naming the ID. A fetch, poll or merge can resolve it later. |
| `parent` names a ticket not found in any scanned context | The ticket is shown as a root with a diagnostic naming the ID. |
| `deps` entries form a cycle | Every ticket on the cycle shows a diagnostic naming the cycle. Only a cycle among open tickets blocks: each ticket on it is blocked. A closed dependency never blocks, so a cycle that passes through a closed ticket does not block by itself. |
| `parent` links form a cycle | Each ticket on the cycle is shown as a root with a diagnostic. |
| `slug` does not match the grammar | Visible problem. The value is preserved and is not searchable. |

None of these makes a ticket nonconforming in the canonical RFC's sense. Only
the required fields decide conformity.

A `deps`, `parent` or `slug` key whose value is null is read as absent and is
not a problem. A null entry inside a `deps` list is a wrong type.

Whether a target is a ticket, and whether a ticket is on a cycle, depend on
other items. Those two conditions are decided when a ticket is read, against
the items the index holds then, and are reported on the ticket as
`relationship_not_a_ticket`, `dependency_cycle` and `parent_cycle`.

## Writing relationships

`deps`, `parent` and `slug` are written through the ordinary ticket create and
save operations, with their existing observation, request-identity and
checkpoint rules. There is no separate relationship store.

- A create or save that would introduce a `deps` cycle or a `parent` cycle is
  rejected before any write, naming the cycle. The check uses the tickets
  visible in scanned contexts at that moment.
- A save may add a dependency on a ticket that is not found locally. The
  result reports it as unresolved; it is not an error, because the ticket may
  live on a branch not yet fetched.
- A cycle or duplicate slug can still arrive through a merge or a hand edit,
  since two branches are each valid alone. Those are handled by the validation
  rules above, never by an automatic fix.
- `slug` is generated by create. A caller cannot supply or change it. The
  assign operation adds one to a ticket that has none and is rejected for a
  ticket that already has one.
- Removing a closed ticket from another ticket's `deps` is never required: a
  closed dependency stops blocking and stays as history.

## Queries

All queries are served from the existing SQLite index, the same rebuildable
cache that drives the desktop lists and searches. They do no network contact, no implicit refresh
and no canonical write, and they run against the index's effective copy of
each ticket across the primary context and active worktrees.

A ticket is **closed** when it has lifecycle closure metadata (`closed_at` and
`closed_by`), independently of its free-form `status` text. A ticket is
**ready** when it is open and every entry in `deps` resolves to a closed
ticket. A ticket is **blocked** when it is open and not ready.

| Query | Result |
| --- | --- |
| Ready | Open tickets that are ready. |
| Blocked | Open tickets that are blocked, each with the reasons: open blockers, unresolved IDs, and membership of a cycle of open tickets. A cycle reason names at most 16 of the cycle's IDs, the lowest, and says whether that is all of them. |
| Dependencies of a ticket | The tree of tickets it depends on, the tree of tickets that depend on it, or both, to an optional depth. Each tree is returned as a flat list of lines in depth-first order, and each line carries its depth and closure state. A ticket reached twice is marked as repeated, not expanded again. |
| Cycles | Every `deps` cycle and every `parent` cycle among scanned tickets, open or closed. Each cycle is listed whole, once. |
| Children of a ticket | Its direct children; the ticket result also carries its parent. |
| Plan | Open tickets in ordered batches. Every ticket in a batch has all its open dependencies in earlier batches, so tickets in one batch can be worked in parallel. Tickets on a cycle of open tickets, behind an unresolved dependency, or downstream of either are listed separately as unplannable, with the reason. |
| Critical path | The longest chain of plannable tickets by dependency, as an ordered list. A plannable ticket is an open ticket that is not on a cycle, not behind an unresolved dependency and not downstream of either. Length is counted in tickets; there are no estimates. Ties resolve by the deterministic ordering below. |
| Find by slug | Every ticket whose slug matches, case-insensitively. |

Ready, blocked and plan accept the ticket list filters (`status`, `type`,
`project`). Results are ordered deterministically: by the ticket list ordering
unless the query defines its own, and by ULID within a batch or tie. Lists are
complete; silent truncation is forbidden.

A ticket closure result additionally reports the tickets that became ready
because of it.

## CLI

These verbs extend the [CLI taxonomy](cli-contract.md#command-taxonomy).
They follow its target, JSON, error and exit rules.

| Commands | Inputs and behavior |
| --- | --- |
| `ticket ready`, `ticket blocked` | Optional list filters. Blocked returns each ticket's reasons. |
| `ticket deps` | `--id` selects the ticket; `--direction down\|up\|both` (default `down`, the tickets it depends on); optional `--depth`. |
| `ticket children` | `--id` selects the parent. |
| `ticket cycles` | No input. Zero cycles is success with an empty result. |
| `ticket plan`, `ticket critical-path` | Optional list filters for plan. |
| `ticket find` | `--slug <text>`; returns every match. No match is success with an empty result, not an error. |
| `ticket slug-assign` | `--id` and `observation`; adds a generated slug to a ticket that has none. Optional `initials`. |

Existing verbs change as follows.

- `ticket create` and `ticket save` accept optional `deps` (a complete list
  that replaces the stored one; an empty list or null clears it) and `parent`
  (a ULID, or null to clear). `ticket create` accepts optional `initials`.
  Neither accepts `slug`.
- `ticket list` and `ticket show` include `slug`, `parent`, `deps`, readiness
  and any relationship problems. `ticket list` accepts `--slug` as a
  whole-code, case-insensitive filter and `--ready` / `--blocked`.
- `repo create` and `repo enable` accept optional `ticket_slug_prefix`, shown
  in the confirmation preview and written to the tracked configuration.
- `repo identity` reports the effective initials and their source.
  `repo identity-set` accepts optional `initials`, written to repository-local
  Git configuration and never to global configuration.
- `ticket close` reports the tickets its closure made ready.

Human output shows the slug beside the ULID wherever a ticket is named.

## Desktop

- Ticket lists and the ticket header show the slug. Copy controls offer the
  slug and the ULID separately.
- A find-by-slug entry lists every match with its context; choosing one opens
  that ticket by ULID.
- Ticket lists can filter to ready or blocked tickets. A blocked ticket shows
  its reasons.
- The ticket view shows its dependencies, the tickets that depend on it, its
  parent and its children, each opening the related ticket.
- The metadata controls edit `deps` and `parent` by choosing tickets, never by
  typing a slug as an identity. A rejected cycle is explained before any save.

No board, graph canvas or rollup view is required.

## Index

Manyhands already keeps a local SQLite index as a cache of canonical files; it
drives the desktop interface, lists and searches. This RFC extends it and adds
no second store. The index adds a slug to item discovery, an edge record
(source ticket, target ULID and kind, `deps` or `parent`) and a record of
each relationship problem found in a ticket's own file. An edge's context
is its source ticket's context. Whether a target resolves, and a ticket's
readiness, are computed when read, not stored, because both change when
another ticket is closed, fetched or merged. These records are rebuilt from
canonical files on refresh and rebuild, like every other index record. Losing
the index loses no relationship.

## Deferred and rejected

Deferred to the [PRD roadmap](../PRD/mvp.md#ticket-graph-and-workflow-enhancements):

- Importance and bottleneck analytics (PageRank, betweenness centrality).
- Typed non-blocking links such as related, discovered-from and supersedes.
- A priority field and priority ordering of ready work.
- Assignee and claiming.
- Full-text search across tickets and documents.

Considered and rejected by the product owner on 2026-10-07:

- Hierarchical IDs that encode the parent in the identity (`bd-a3f8.1`). They
  conflict with a ULID that never changes; `parent` carries the hierarchy.
- A database as the canonical store (Beads keeps issues in Dolt). Canonical
  ticket state stays in Markdown files; the SQLite index remains a rebuildable
  cache over them. Also rejected: gates on external conditions, automatic
  compaction of old tickets, and messaging between agents.
- Similarity search, duplicate detection, an MCP server, and synchronization
  with GitHub Issues or Linear.

Also not adopted: encoding the whole ULID with Sqids, as the earlier
[short-ID research](../research/sqid-generation.md) proposed. Sqids is a
reversible encoding, so 128 bits cannot come out as five characters; a
truncated hash is used instead.

## Required evidence

- Golden vectors fix the ULID-to-code derivation at each supported length and
  the initials derivation, including one-word, non-ASCII and overridden names.
- Canonical round-trips show `deps` written sorted, unique and one per line;
  unknown metadata preserved; `slug` unchanged by rename, identity change and
  prefix change.
- Real-repository tests cover ready, blocked, trees in both directions, plan
  and critical path across primary and active worktrees, with a dependency
  closed in one context, an unresolved dependency and a status text of
  `closed` on an open ticket.
- Write-time cycle rejection, and a cycle and a duplicate slug introduced by
  merging two individually valid branches, both reported and neither repaired.
- A slug lookup with zero, one and several matches; a slug supplied as `--id`
  is rejected as an invalid ID.
- No query performs network contact or changes canonical files.
