---
title: "Wave 03 F1: Headless Read Boundary And Result Model"
date: 2026-10-07
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4CJE3VG98CT2SH2Q135REA4"
---

# Wave 03 F1: Headless Read Boundary And Result Model

## Purpose And Authority

Give both front ends one headless way to read Manyhands state, so neither the
CLI nor the desktop reads SQLite, Git or canonical files itself. The Cycle adds
read services to the library, a shared result and error model, and versioned
JSON v1 data transfer objects (DTOs) for every read, including the ticket
relationship queries. It delivers no CLI verb and no desktop screen.

This is Cycle F1 of [Wave 03](../Waves/wave-03-dogfooding.md#f1-headless-read-boundary-and-result-model),
tracked by ticket `01M4CC0VMQ7R15A7M9SPN3KB67`. The
[CLI contract](../RFC/cli-contract.md),
[ticket relationships and short codes](../RFC/ticket-relationships-and-short-codes.md),
[canonical schema](../RFC/canonical-content-and-comment-schema.md),
[repository/index](../RFC/repository-index-persistence-and-refresh.md),
[authentication](../RFC/authentication-and-credential-handling.md) and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs are
authoritative. The
[design](../plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-design.md)
and [implementation plan](../plans/2026-10-07-wave-03-foundation-01-read-boundary-and-results-implementation.md)
accompany this document. The product owner approved all three on 2026-10-07,
with the decisions recorded below. Implementation authorization is a separate
gate and has not been given.

## Entry Evidence And Dependency Boundary

- Reused the existing ticket worktree and branch; no replacement was created.
- Fresh `git fetch origin main` observed `origin/main` at
  `ceb1be49477cfec5e14093082d00cf01a5367fb3`. The clean ticket head rebased
  from `691fb8b09b9962df76d5be59c1ff31a0e5f91208` to
  `bfecd00d8e31692cf82c5a76d2c5f63e740b7416` without conflicts. The fetched
  base is an ancestor of the result and the worktree was clean.
- The main checkout's untracked `.superpowers/` and every other ticket worktree
  were left untouched.
- Wave 02 Cycles 01–04, which F1 requires, are closed and on main. Cycle 05 is
  also closed. Cycle 06 is in flight in its own worktree and Cycles 07–10 are
  not started; nothing from them is used here.
- Wave 03 entry-gate item 3 requires the API audit to be refreshed for what F1
  consumes. The design's read-surface audit does that at `ceb1be4` by source
  inspection. It ran no Rust command.
- Entry-gate item 7, the required Rust checks at the implementation baseline,
  is the first task of the implementation plan and has not been run.
- **A defect in the existing index blocks this Cycle.** Discovery records
  every item found in each item worktree's checkout, not only the item that
  worktree exists to edit (`src/repository/discovery.rs:270-298`, `303`).
  With two item worktrees, any item present in both is stored twice, and the
  snapshot read then fails with "stored item ID is not globally unique"
  (`src/repository.rs:5349-5356`). An item's primary row is also dropped
  whenever any worktree holds a copy, however stale. So "the effective copy"
  that every read in this Cycle depends on is not well defined once a
  repository has more than one item worktree, which this project's own
  repository does. This was found by reading the source during planning and  has not been reproduced by a test. Decision 1 below sets the rule; defect
  ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G` carries the fix and must merge before this Cycle is
  implemented.

Before implementation, rebase this ticket on then-current main, repeat the
ancestry check and re-check the audit rows against any library change that
landed in between.

## Scope

- **Target resolution.** Resolve a caller's path to one registered repository.
  A linked worktree root, including an item worktree, maps to its repository.
  Enumerate registered repositories without knowing a root in advance.
- **Result model.** One envelope with `schema_version` 1, the outcome, stable
  result code, redacted message, scope, effects, data and recovery actions the
  CLI RFC defines; a stable code for every failure a read can return; and a
  failure class the CLI maps to an exit status.
- **Redaction.** Remote URLs lose embedded credentials. No DTO, message or
  error carries backend error text, SQL, a `Debug` rendering or secret
  material.
- **Read services and DTOs** for:
  - repository list, inspection and effective Git identity with its source;
  - remotes with redacted locations and the publication selection;
  - shared SSH keys: list, one key, and public-key text;
  - host trust pins: list and one authority;
  - documents and tickets: list, and one complete item with source, body,
    metadata, unknown metadata, context and an observation token;
  - a nonconforming resource by exact path;  - an item's comment threads, ordered, with bodies and, where recorded, authors;
  - index status, polling status and the operation inventory;
  - a new canonical ULID.
- **Ticket list filters.** Exact `status`, `type` and `project`; lifecycle
  closure `open`, `closed` or `all`; short code; ready and blocked.
- **Ticket relationships.** Tickets carry `slug`, `parent`, `deps`, readiness
  and relationship problems. Queries: ready, blocked with reasons, dependencies
  in both directions, children, cycles, plan, critical path and find by short
  code. The index gains edge records rebuilt from canonical files.
- **Published contract.** JSON Schemas for the envelope and every DTO, with
  golden JSON fixtures.

Every read is local. It contacts no network, refreshes nothing implicitly, and
changes no canonical file, Git object, ref, worktree or configuration.

## Explicit Exclusions And Downstream Obligations

This Cycle does not parse commands, print human output, choose exit numbers or
write help; C1 owns those. It builds no desktop view; D1 does. It writes no
canonical content, configuration or key, performs no network request, replays
no request and schedules no polling.

It does not write `deps`, `parent` or `slug`, generate a short code, or reject
a cycle on save. F2 owns those. F1 reads the three fields from tickets that
already have them, however they were written.

It does not list empty document folders. The index records items, not folders;
F2 owns folder creation and its listing.

It does not walk Git history to attribute a comment. See decision 4.

Two reads named in the Wave document cannot be complete at this baseline:

- **Conflict inspection.** No conflict record exists until Wave 02 Cycle 06.
  This read is deferred to its first consumer, C4 or D5, as a shared-library
  change (decision 2).
- **Polling status.** The stored policy and latest outcome are reported. The
  next eligible time and per-attempt history do not exist until Wave 02
  Cycle 08; those fields are present and null.

## Required Read Contract

A read may do the application-local bookkeeping the library already does when
it opens its data directory: create that directory, open and migrate the index
database, and create its lock file. This is today's behavior, kept as is. A
read may do nothing else. Specifically:

- No canonical file, Git object, ref, worktree, Git configuration or key file
  is created, changed or removed.
- No refresh or scan runs. A read reports what the index holds and says when
  that is stale, never having been refreshed, or unavailable.
- No network contact occurs, and no credential is requested or unlocked.
- A read takes only the shared index lock. It never blocks behind, or steals,
  a repository operation lease.

Lists are complete and deterministically ordered. An empty result, a stale
index, a never-refreshed index and an unavailable index are four different
answers. Only an unavailable index is a failure; the others return the rows
the index has and say which state it is in. A file that fails the canonical
schema's required-field or front-matter rules appears in its list as a
nonconforming entry with a null ID, its exact path and a problem code; it is
never hidden. A relationship problem never makes a ticket nonconforming; it is
reported on the ticket.

An item is read from its effective copy: the copy in the worktree created to
edit that item when one exists, otherwise the primary copy. The DTO names
which. A copy of an item that merely happens to be checked out in another
item's worktree is never the effective copy.

A ticket is closed when it carries lifecycle closure metadata, whatever its
`status` text says.

## Acceptance And Exit Evidence

| Contract | Required evidence |
| --- | --- |
| Target resolution | Tests resolve a repository root, a linked worktree root and an item worktree to the same registration, and return typed results for an unregistered repository, a subdirectory, a bare repository and a missing path. |
| Read services | Library integration tests against real repositories cover every read service, with deterministic ordering and complete lists. |
| Nonconforming content | Marker-only, malformed and misplaced files appear in lists with null IDs and problem codes, and can be read by exact path. |
| Closure | An open ticket whose status text is `closed` is listed as lifecycle-open; a lifecycle-closed ticket is found by the `closed` filter. |
| Effective copy | With two item worktrees that both contain a third item, that item is listed once, from primary; each worktree's own item is listed once, from its worktree. |
| Stale and unavailable index | A never-refreshed repository, a refresh-required repository, a degraded index and an empty repository each produce their own distinct result; only the degraded index is a failure. |
| Relationships | Ready, blocked, trees in both directions, children, cycles, plan, critical path and find by short code are proven across primary and active worktrees, with an unresolved dependency, a merged-in cycle and a duplicate short code each reported and none repaired. |
| No side effects | Before-and-after snapshots show canonical files, Git refs, worktrees, configuration and key files unchanged by every read. Every read test runs with the Git transport uninitialized, where any network-capable path fails closed, so a passing read cannot have reached one. |
| Redaction | Serialized output for every DTO and error contains no credential from a remote URL, no backend error text, no stored guidance text and no sentinel secret planted in fixtures. |
| Published contract | A JSON Schema exists for the envelope and each DTO; each golden fixture is produced by the library, matches its stored file byte for byte and satisfies its schema. |
| Front-end independence | The library builds, and the three new test targets pass, without the `desktop` feature and without a display. |
| Regression | The four required Devenv checks and the CLI smoke test pass. |

Native execution on Windows and macOS is not claimed by this Cycle; see the
decision below.

## Product-Owner Decisions (2026-10-07)

The product owner decided all six as recommended and approved the documents.

1. **Which copy of an item an item worktree contributes.** An item worktree
   contributes only the item it was created to edit, with that item's
   comments. Every other item is read from primary. The product owner called
   this the right compromise. Its accepted cost: a managed document created by
   hand inside a ticket's branch is not listed until that branch merges. The
   fix is its own defect ticket, `01M4CKWWRA1DHFPMWKPNK7CQ1G`, merged before F1 implementation
   starts.
2. **Conflict inspection** is deferred from F1 to the first of C4 or D5, as a
   shared-library change. The Wave document is amended on this branch.
3. **`serde_json`** is added as a direct dependency.
4. **Comment author.** The comment schema gains an optional `created_by`
   field, written at creation from F2 onward. F1 reports it when present and
   null otherwise, and walks no Git history. The canonical schema RFC is
   amended on this branch.
5. **Native evidence.** F1 is proven on Linux; its tests join the native
   workflow's list; Windows and macOS execution, including native path
   matching, is an open obligation carried to G1.
6. **Upgrade behavior.** The first time a newer build opens an existing index,
   every registered repository is marked stale until refreshed. Lists still
   return what they have and say they are stale.

The two wording corrections to the relationships RFC proposed in the review
record are also applied on this branch. These RFC and Wave amendments reach
main when this branch merges.

## Review Record

The three documents were reviewed against the Wave document, the governing
RFCs and the source by an independent reviewer on 2026-10-07. Its three
blocking findings and the corrections they caused are the first three rows.
Rows classified "Decision" were decided on 2026-10-07 as recorded above.

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| Independent review; `discovery.rs:270-298`, `repository.rs:5349-5356` | The index has no single effective copy once two item worktrees exist; the first draft assumed one row per item. | Decision | Decision 1. | Effective-copy acceptance row; a reproducing test is the first step of the fix. |
| Independent review; relationships RFC validation | The first draft stored relationship problems where they would be listed as nonconforming entries, and passed stored guidance text, which can hold backend error text, into DTOs. | Settled | Relationship problems are stored per item and reported on the ticket. Nonconforming entries come only from schema-conformity codes. DTO guidance is fixed text per code; stored guidance is never serialized. | Redaction and nonconforming tests. |
| Independent review; `recovery.rs:90-104`, `keys/registry.rs:593-620` | Two of the three operation stores have no start or update time, and key-material operations belong to no repository. | Settled | Order operations by operation ID, which is a ULID and so sorts by creation time. Report `updated_at` only where stored. Key-material operations are listed with application scope. | Operation ordering test. |
| Wave 03 F1 scope | Conflict inspection has no data model at this baseline. | Decision | Decision 2. | Wave document amendment if accepted. |
| Project convention | Earlier plans barred new Cargo dependencies without approval. | Decision | Decision 3. | `Cargo.lock` change limited to the direct-dependency entry. |
| CLI RFC comment DTO | The DTO requires an author; the schema stores none. Git-derived authorship is costly and unstable. | Decision | Decision 4. | Schema RFC amendment if accepted. |
| Test strategy, Wave 03 gate | Native CI is disabled; F1 cannot produce native evidence. | Decision | Decision 5. | Workflow test list updated; obligation recorded for G1. |
| Index RFC | An index that predates this Cycle has no edges or new columns. | Decision | Decision 6. | Migration test. |
| API audit C0 | Opening the service creates the data directory, migrates SQLite and creates a lock file. | Ruling | Keep today's behavior and state it in the read contract. Cost if wrong: add a non-creating open for reads. | No-side-effect snapshots exclude only the application data directory. |
| CLI RFC | "Does not authorize directly serializing Rust enums." | Ruling | DTOs are separate types with every field and enum name pinned by schema and golden fixture; no domain type derives `Serialize`. | Golden fixtures; existing `compile_fail` doctests stay. |
| Relationships RFC | F1 must read `deps`, `parent` and `slug` before F2 writes them. | Ruling | Parse them as a read-only view; the raw values stay in unknown metadata, so the existing save path keeps their values. | Value round-trip test through the existing save path. |
| Relationships RFC, `deps` | The RFC says a hand-written list is rewritten "on the next save that changes `deps`". The existing serializer re-emits all front matter on every save, so formatting changes on any save. | Ruling | Not F1's to change; values are preserved. Proposed RFC wording correction: "on the next save". F2 owns the canonical form. | Noted for F2. |
| Relationships RFC, index | The RFC says an edge records its context and whether its target resolved, and that the index adds a readiness state. | Ruling | An edge's context is its item row's context. Resolution and readiness are computed at read time, because both depend on other tickets. Proposed RFC wording correction to say so. Cost if wrong: add stored columns. | Readiness tests after closing a dependency without rescanning the dependent. |
| CLI RFC list DTO | Lists omit only body and source, but the index stores neither `closed_by` nor unknown metadata. | Ruling | Add both to the index so lists do not read every file. | List golden fixtures. |
| Index RFC | Freshness needs a last-refreshed time the schema lacks. | Ruling | Add a nullable column written by refresh and rebuild. | Refresh and rebuild regression tests. |
| CLI RFC target resolution | A subdirectory of a repository is not mentioned. | Ruling | Accept a repository root or a linked worktree root; reject a subdirectory with a typed result naming the root. Loosening later is additive. | Resolution tests. |
| Stale reads | An item's file can be newer than, or missing from, what the index recorded. | Ruling | A complete read returns what is on disk and marks the index stale; a missing file is `item_not_found` with a refresh recovery. A read never returns indexed metadata for content it could not read. | Stale and deleted-file tests. |
| Redaction | Which parts of a location are secret. | Ruling | Passwords, HTTP user-info, queries and fragments are removed. SSH user names, scp-like locations and local paths are kept; they are configuration, not credentials. `created_by` and `closed_by` are canonical content and are returned as written. | Redaction table tests. |
| CLI RFC item DTO | Items carry an observation token before F2 defines observations. | Ruling | An opaque, versioned digest of context, path and source bytes. F2 may change what it covers under a new version prefix. | Token stability tests. |
| Published contract | Schemas need a conformance check without a validator dependency. | Ruling | Hand-written schemas using a small keyword subset, checked by a test-support checker. Cost if wrong: add a dev-dependency validator. | Checker rejects deliberately wrong instances. |
| Platform | Roots are matched as stored strings; Windows verbatim prefixes and case-insensitive file systems are untested. | Open obligation | Linux behavior is specified and tested here. Native path behavior is recorded for G1 with decision 5. | Named in the ticket's open obligations. |
| Size | F1 grew when relationship reads were added. | Settled | Ten tasks; Tasks 1–7 are the read boundary and 8–9 the relationships, so a split remains possible if review shows it is needed. | Plan. |
| Lifecycle | Planning could be read as authorization. | Settled | The ticket stays open; approval of the three documents and implementation authorization are separate. | Ticket checkpoints. |

Record planning, per-task progress, decisions, baseline and final verification,
review and review-ready status as ticket comments. Publishing, merging, closing
and worktree cleanup each need their own authorization.
