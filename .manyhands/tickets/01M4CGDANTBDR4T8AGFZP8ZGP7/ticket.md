---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CGDANTBDR4T8AGFZP8ZGP7"
title: "Add ticket dependencies, hierarchy and short codes to Wave 03"
type: "task"
status: "closed"
project: "manyhands"
team: "core"
wave: "03"
---

Specify blocking dependencies, a parent hierarchy and a human short code for
tickets, and place the work in Wave 03. This ticket changes planning documents
only.

## Origin

On 2026-10-07 the product owner asked for a review of
[ticket-rs](https://docs.ticket-rs.io/blog/whitepaper) and
[Beads](https://github.com/gastownhall/beads) for features worth adopting,
naming `deps`, graph searches, a ticket hierarchy and a short code. The earlier
[short-ID research](../../../docs/research/sqid-generation.md) was reviewed
with them.

## Product-owner decisions (2026-10-07)

- The ULID is the canonical identity. The short code is a label for people;
  agents use ULIDs. A collision appears as several search results a person
  chooses between.
- Short code: optional project prefix, creator initials, and five characters
  derived from the ULID, for example `mh-vh-k9x2b`.
- Adopt: `deps`, ready and blocked queries, dependency trees, cycle detection
  with write-time rejection, made-ready reporting on close, `parent`, plan
  batches and critical path.
- Place the work in Wave 03.
- Skip: hierarchical IDs, a database as the canonical store, gates, compaction, messaging,
  similarity, duplicate detection, an MCP server and GitHub/Linear sync.
- Record as future enhancements: importance and bottleneck analytics, typed
  links, priority, assignee/claim and full-text search.

## Changed documents

- New [ticket relationships and short codes RFC](../../../docs/RFC/ticket-relationships-and-short-codes.md),  `status: approved`.
- [PRD](../../../docs/PRD/mvp.md) v0.6: `MH-CONTENT-005`, `MH-CONTENT-006` and
  the Ticket Graph And Workflow Enhancements roadmap entry.
- Pointers in the canonical schema, CLI, index and desktop RFCs; RFC register
  row W3-12.
- [Wave 03](../../../docs/Waves/wave-03-dogfooding.md): authority,
  traceability, exclusions, ownership, approval record and the scope or exit
  evidence of F1, F2, C1, C3, C5, D1, D3 and D5.

## Choices made while drafting, for review

The decisions above did not settle these; the RFC states one answer for each.

- A short code is never accepted as `--id`. It is only a search key, so the
  approved "full ULIDs as mutation selectors" rule is unchanged.
- The short code is stored in `slug` at creation and never rewritten. It
  cannot be recomputed, because the initials are not in the ULID.
- The code is the leading 25 bits of the BLAKE3 hash of the ULID text in
  Crockford Base32, not Sqids, which cannot shorten 128 bits to five
  characters.
- Initials come from the confirmed Git identity name, or a repository-local
  `manyhands.initials` override. The prefix is an optional
  `ticket_slug_prefix` in the tracked configuration.
- Existing tickets have no short code until one is assigned explicitly.
- One blocking dependency type. `parent` groups and never blocks, with no
  rollup, so the PRD's rollup non-goal stands.
- A dependency on a ticket not found locally counts as blocking and is shown
  as unresolved, because it may be on an unfetched branch.
- Cycles and duplicate short codes that arrive through a merge are reported,
  never repaired.
- Desktop scope grew beyond the four Cycles first proposed: D1 displays the
  fields and D3 edits them; C5 and D5 report tickets made ready by a close.

## Approval (2026-10-07)

The product owner approved the RFC, the PRD and RFC amendments, the Wave 03
allocation including the desktop Cycles, and the drafting choices above, with
one note: Manyhands does have a SQLite database, as an index and cache that
drives the interface and searches. The RFC now says so, serves every query
from that index, and rejects only a database as the canonical store.

The Wave 03 Cycle tickets for F1, F2, C1, C3, C5, D1, D3 and D5 are refreshed
on their own branches with the added scope.

## Closure (2026-10-07)

The product owner authorized closing this ticket on its branch and merging to
local main. Both closure conditions are met by that merge. No push or worktree
removal is authorized.

## Infrastructure

- Branch: `manyhands/ticket/01M4CGDANTBDR4T8AGFZP8ZGP7`.
- Worktree: `.manyhands/worktrees/01M4CGDANTBDR4T8AGFZP8ZGP7`.
- Created from freshly fetched `origin/main` / local `main` `0b9c7c2`.
- CLI ticket operations are not implemented, so canonical files are used.

## Closure conditions

- The product owner approves the RFC and amendments; the RFC's `status`
  becomes `approved` and the approval is recorded there, in Wave 03 and here.
- The change is merged to main.
