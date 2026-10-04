---
title: "Wave 01 Cycle 03: Isolated Local Authoring"
date: 2026-10-02
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7A3C5E7G9J1M3P5R7T9V1X3"
---

# Wave 01 Cycle 03: Isolated Local Authoring

## Parent Wave

This is Cycle 03 of [Wave 01: Foundations](../Waves/wave-01-foundations.md).
It adds safe local item contexts, canonical authoring, and scoped checkpoints
before Cycle 04 adds discovery and durable operation records.

## Purpose

Implement headless local authoring for canonical documents, tickets, and
comments. A caller can create or reuse one deterministic local context for an
item, write valid canonical Markdown, and checkpoint only that event's paths
without becoming a Git operator.

The Cycle preserves caller-owned drafts and all existing Git and filesystem
state. It does not contact a remote, refresh discovery, persist drafts or
operation records, or offer multiple local editing contexts for one item.

## Prerequisites

- Cycles 01 and 02 exit criteria remain satisfied, including the canonical
  schema, enabled-repository configuration, local registry, and disposable
  real-repository fixtures.
- Wave 01's entry gate remains satisfied.
- The approved canonical schema, Git workflow, repository/index, and test
  strategy RFCs remain unchanged or receive an approved amendment.
- Implementation starts from a clean `main` branch.

## RFC and PRD Traceability

| Source | Cycle responsibility |
| --- | --- |
| [Canonical content and comment schema RFC](../RFC/canonical-content-and-comment-schema.md) | Create and preserve canonical document, ticket, and comment representations, paths, IDs, unknown front matter, Markdown bodies, and comment relationships. |
| [Git workflow and conflict recovery RFC](../RFC/git-workflow-and-conflict-recovery.md) | Provision deterministic local contexts, make scoped temporary-index checkpoints, and preserve retryable local state. |
| [Repository index persistence and refresh RFC](../RFC/repository-index-persistence-and-refresh.md) | Mark the existing repository registration refresh-required after a successful checkpoint; do not add discovery or durable operation tables. |
| [Test and compatibility strategy RFC](../RFC/test-and-compatibility-strategy.md) | Exercise authoring and recovery against disposable real repositories with named failure injection. |
| `MH-CONTENT-001` to `MH-CONTENT-003` | Create, edit, and move canonical documents; create and edit canonical tickets. Discovery remains Cycle 04 work. |
| `MH-COMMENT-001` to `MH-COMMENT-002` | Create root comments and replies with validated parent relationships and deterministic schema data. |
| `MH-COLLAB-001` to `MH-COLLAB-003` | Provision or reuse one local context, checkpoint changed local saves, and report local comment publication state without a remote operation. |
| `MH-NFR-001`, `MH-NFR-006`, and `MH-NFR-008` | Preserve local-first authoring, idempotent retries, and recoverable local work. |

## In Scope

- A small headless authoring extension to the shared repository domain API. It
  has no GPUI, GPUI Kit, desktop, CLI, SSH, or network dependency.
- Creation and reuse of one deterministic local context per document or ticket:
  `manyhands/<kind>/<ULID>` at
  `.manyhands/worktrees/<ULID>/`.
- Caller-supplied canonical ULIDs for new documents, tickets, and comments, so
  a caller can retry the same draft after an interrupted operation.
- Creation, validation, editing, and moving of documents under `docs/`, while
  retaining their ULIDs, Markdown bodies, and semantic unknown front matter.
- Creation, validation, and editing of tickets at
  `.manyhands/tickets/<ULID>/ticket.md`.
- Creation of root comments and replies at their canonical paths. The service
  assigns a UTC `created_at` value and validates the target item and parent in
  the selected context.
- Scoped checkpoint commits for documents, tickets, and comments using a
  temporary Git index. A document move scopes its checkpoint to exactly its
  source deletion and destination addition or modification.
- No-op saves that create no empty commit, and retries that checkpoint a prior
  uncommitted owned-path write exactly once.
- Typed, non-durable recovery outcomes for partial context provisioning, item
  writes, checkpoint commits, and refresh invalidation.
- Registry `refresh_required` invalidation after successful local checkpoints.
- Local comment outcomes of `publish pending` with no publication remote and
  `sync deferred` with a configured publication remote.

## Out of Scope

- Remote contact, SSH authentication, fetch, push, synchronization, polling,
  merge, rebase, promotion, ticket closure, or worktree cleanup.
- Multiple local editing contexts, context selection, remote-context
  materialization, or any alternate context naming convention. Wave 2 owns the
  remote multi-context protocol and user choice behavior.
- SQLite context, item, problem, or durable operation-recovery records; full
  scans; discovery queries; cache rebuild; and index refresh. Cycle 04 owns
  those capabilities.
- Cross-process repository leases and restart reconciliation evidence. Cycle 05
  owns the durable coordination and foundation recovery gate.
- Persisted draft bodies, application UI state, desktop views, CLI commands,
  editor behavior, or automatic repair of malformed content.
- Modification of global Git configuration, tracked `.gitignore`, unrelated
  staged paths, unrelated files, branches, worktrees, remotes, or commits.

## Planned Implementation Changes

| Path | Change |
| --- | --- |
| `src/repository.rs` | Extend the headless repository service with deterministic context provisioning, canonical item writes, scoped temporary-index checkpoints, typed recovery outcomes, and registration invalidation. |
| `tests/support/mod.rs` | Extend disposable `git2` fixtures with enabled repositories, context state, exact staging assertions, and Cycle 03 failure injection. |
| `tests/local_authoring.rs` | Add focused real-repository integration coverage for authoring, checkpointing, and recovery. |
| `docs/Waves/wave-01-foundations.md` | Correct Cycle 03 and Cycle 04 scope to the approved one-local-context boundary. |
| `docs/RFC/git-workflow-and-conflict-recovery.md` | Define the two-path staging boundary for a document move. |
| `docs/RFC/repository-index-persistence-and-refresh.md` | Limit Wave 1 indexing to one deterministic local context per item and defer remote shared-context materialization to Wave 2. |

No new runtime dependency is required. The public API MUST remain synchronous,
headless shared domain logic. Git handles and temporary indexes MUST be opened
only for the operation using them and never retained by callers.

## Authoring Contract

### Repository and Context Preconditions

Every authoring operation requires an enabled, non-bare repository whose root
has the configured primary branch checked out. The tracked configuration path
must match `HEAD`; unrelated primary-worktree changes may remain present because
context provisioning branches from the primary `HEAD` commit and never stages or
writes those changes.

For a document or ticket ULID, the expected local context is its exact branch
and worktree path. When both are absent, Manyhands creates the branch from the
configured primary `HEAD`, then creates the worktree. When the exact branch and
worktree contain the identified conforming item, Manyhands reuses that context.

If branch creation fails, no branch or worktree is created. If worktree creation
fails after branch creation, the branch remains for an exact retry. A matching
branch and worktree with no item may resume only the same create request with
the caller-retained ULID; a mismatched branch, worktree, path, item kind, or
item ID returns a typed recovery outcome without mutation. A context collision
never produces a context-selection result or a second local worktree.

### Canonical Writes

Callers provide the canonical ULID and draft content for each new item. The
service validates the ULID, path, known fields, and resulting canonical
relationships before it writes. It resolves a usable repository or effective
Git identity before writing; an identity-required outcome leaves the draft with
the caller and changes no Markdown.

Document creation rejects an occupied destination. Document editing reads and
validates the existing document before updating it so its unknown front matter
is retained semantically and its body is retained exactly when unchanged. A
move validates both canonical document paths, rejects an occupied destination,
and retains the document ULID.

Ticket creation and editing affect only the ticket's canonical Markdown path.
A comment or reply requires a conforming target document or ticket in the
selected context. A reply additionally requires a conforming parent comment for
the same target item. Comments are immutable after creation in this Cycle.

### Checkpoints and Local Outcomes

Before checkpointing, the service validates every owned path against canonical
schema rules. It builds a temporary index from the context `HEAD` tree and adds
only these paths:

| Event | Permitted temporary-index paths |
| --- | --- |
| Document save | The selected document Markdown path |
| Document move | The selected document's former and destination Markdown paths |
| Ticket save | `.manyhands/tickets/<ULID>/ticket.md` |
| Comment submit | `.manyhands/comments/<item-id>/<comment-id>.md` |

The live Git index is never written. Unrelated modified, staged, untracked,
deleted, or conflicted paths remain untouched. If every owned path already
matches `HEAD`, the result is a no-op. If a prior write remains uncommitted,
retry checkpoints that owned-path change once even when the supplied draft is
unchanged.

Checkpoint subjects are deterministic:

```text
Checkpoint document <ULID>
Checkpoint ticket <ULID>
Checkpoint comment <ULID>
```

After a successful commit, the service sets the existing registration's
`refresh_required` flag. It does not scan or update discovery. A registry-write
failure returns the authoritative commit OID with refresh pending; retry
invalidates discovery without creating another commit.

A comment checkpoint never contacts a remote. With no configured publication
remote, its result is saved locally with `publish pending`. With a configured
publication remote, its result is saved locally with `sync deferred`. Wave 2
replaces the latter with the RFC-required immediate synchronization attempt.

## Recovery Considerations

Cycle 03 persists no draft or operation journal. The caller retains a draft and
its ULID until a write succeeds; after a successful write, the worktree file is
the recoverable draft state. Real Git branches, worktrees, files, commits, and
the repository registration are authoritative on retry.

- A context-branch failure leaves no new branch or worktree.
- A worktree-creation failure retains only the deterministic branch created by
  the failed operation; retry creates the missing worktree.
- An item-write failure preserves the prior file contents. A successful write
  followed by checkpoint failure leaves the valid changed file in its context.
- A successful checkpoint followed by registration invalidation failure retains
  the commit and reports refresh pending; retry performs invalidation only.
- No Cycle 03 path deletes a branch, worktree, draft, file, commit, or unrelated
  user change during automatic recovery.

Because cross-process leasing and durable reconciliation are deferred, a caller
must serialize concurrent authoring requests for the same repository until
Cycle 05 supplies the repository-scoped lease. A concurrent Git-state conflict
is recoverable and must identify the repository, item, expected branch, and
worktree for remediation.

## Test and Fixture Plan

`tests/local_authoring.rs` MUST cover:

- Deterministic branch and worktree creation from primary `HEAD`, one-context
  reuse, and visible recovery for duplicate, missing, mismatched, or partial
  expected context state.
- Document creation, edit, move, path-collision rejection, ULID retention, and
  semantic unknown-front-matter retention.
- Ticket creation and editing at the canonical ticket path.
- Root-comment and reply creation, target and parent validation, immutable
  timestamps, and local `publish pending` versus `sync deferred` outcomes.
- Exact temporary-index staging for each event, including both and only the
  source and destination of a document move. Tests prove the live index and
  unrelated paths are unchanged.
- No-op saves, retries of uncommitted owned-path writes, deterministic commit
  subjects, and absence of empty commits.
- Missing identity before a write, failed item writes, failed checkpoint commits,
  and registry invalidation failures. Each test asserts the actual preserved
  filesystem and Git state, then proves retry completes only unfinished work.
- No remote transport or credential callback for any Cycle 03 operation.

Fixtures use `git2` and temporary directories. They configure local identities
inside fixture repositories and never read developer repositories, global Git
configuration, application data, SSH keys, or a system Git executable.

## Verification

During implementation, run the focused target after each completed behavior
slice:

```sh
devenv shell -- cargo test --locked --test local_authoring
```

Before declaring the Cycle complete, run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

No desktop or CLI smoke test is required because this Cycle introduces neither
desktop nor CLI behavior.

## Exit Criteria

Cycle 03 is complete when:

- Headless shared code creates and reuses exactly one deterministic local item
  context without mutating unrelated primary or worktree state.
- Documents, tickets, root comments, and replies are created or edited at their
  canonical paths with valid schema data and caller-retained stable IDs.
- Document moves retain their ULID and checkpoint exactly the former and
  destination paths without overwriting another document.
- Changed document, ticket, and comment saves create one inspectable scoped
  checkpoint; no-op saves create none.
- A checkpoint changes neither the live index nor unrelated staged, untracked,
  modified, deleted, or conflicted paths.
- Every named partial-context, write, checkpoint, and invalidation failure
  preserves recoverable state, and retry creates no duplicate item, comment,
  branch, worktree, or commit.
- Comment submission reports a local-only outcome without contacting a remote.
- The focused target and full required Rust verification suite pass.

## Handoff

Cycle 04 consumes the enabled-repository registration and its `refresh_required`
flag to scan the primary worktree and the one recognized local item context per
item. It MUST treat the flag as a request for a canonical scan, not proof that a
checkpoint completed. Cycle 04 adds discovery, visible problems, and durable
operation records without changing canonical Markdown or Git state.

Cycle 05 adds the repository-scoped cross-process lease and proves restart and
failure reconciliation across the completed Wave 1 lifecycle. Wave 2 owns
remote shared-context materialization and collaboration merge recovery; it does
not introduce a multiple-editable-context choice result.
