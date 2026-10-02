---
title: "Wave 01 Cycle 03 Isolated Local Authoring Design"
date: 2026-10-02
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7B4D6F8H0K2N4Q6S8V0X2Z4"
---

# Wave 01 Cycle 03 Isolated Local Authoring Design

## Goal

Extend the shared, headless `RepositoryService` so callers can provision one
deterministic local context for a document or ticket, write canonical documents,
tickets, and comments, and make a scoped local checkpoint without changing
unrelated Git state. The implementation preserves local-only authoring and
defers discovery, durable operation records, cross-process leases, and all
remote behavior to their approved later Cycles.

## Authorities

This design implements the approved
[Cycle 03 authority](../Cycles/wave-01-cycle-03-isolated-local-authoring.md)
and must preserve these decisions:

- `git2`/libgit2 is the only Git backend. No system Git executable or remote
  transport is permitted.
- A document or ticket has one local context branch,
  `manyhands/<kind>/<ULID>`, and one worktree,
  `.manyhands/worktrees/<ULID>/`.
- Callers retain a new item's ULID and draft until a write succeeds. Cycle 03
  writes no draft or operation-recovery record to SQLite.
- Cycle 03 callers serialize operations per repository. Cycle 05 introduces the
  cross-process lease and durable reconciliation evidence.
- A document move owns exactly its source deletion and destination addition or
  modification in one `Checkpoint document <ULID>` commit.
- A comment checkpoint never contacts a remote. It reports `publish pending`
  without a publication remote and `sync deferred` when one is configured.

## Module Boundary

Keep the implementation in `src/repository.rs`. `RepositoryService` already
owns the application-local registry, transient `git2` handles, identity
resolution, temporary-index commit construction, failure injection, and
`refresh_required` invalidation. Adding a separate authoring service would
duplicate private registry and repository-root handling without creating a
reusable boundary.

The Cycle adds no dependencies and leaves `src/lib.rs` and `Cargo.toml`
unchanged. It adds `tests/local_authoring.rs` for real-repository integration
coverage and extends `tests/support/mod.rs` only with reusable fixture helpers.

## Public Contract

The public API remains small and typed. It exposes authoring requests and
outcomes, not filesystem, Git, SQLite, or libgit2 internals.

```rust
pub enum AuthoringKind {
    Document,
    Ticket,
}

pub enum ContextIntent {
    Create,
    Edit,
}

pub struct AuthoringTarget {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub item_id: canonical::ItemId,
    pub intent: ContextIntent,
}

pub struct ItemContext {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub item_id: canonical::ItemId,
    pub branch: String,
    pub worktree: PathBuf,
}

pub enum ContextProvisionOutcome {
    Created(ItemContext),
    Reused(ItemContext),
}

pub struct DocumentDraft {
    pub title: String,
    pub body: String,
}

pub struct TicketDraft {
    pub title: String,
    pub ticket_type: String,
    pub status: String,
    pub project: Option<String>,
    pub team: Option<String>,
    pub body: String,
}

pub struct SaveDocumentRequest {
    pub target: AuthoringTarget,
    pub source_path: Option<PathBuf>,
    pub destination_path: PathBuf,
    pub draft: DocumentDraft,
}

pub struct SaveTicketRequest {
    pub target: AuthoringTarget,
    pub draft: TicketDraft,
}

pub struct SubmitCommentRequest {
    pub target: AuthoringTarget,
    pub comment_id: canonical::ItemId,
    pub parent_id: Option<canonical::ItemId>,
    pub body: String,
}

pub enum LocalCheckpoint {
    Checkpointed { commit_oid: git2::Oid },
    NoChange,
    RefreshPending { commit_oid: git2::Oid },
}

pub enum SaveOutcome {
    IdentityRequired { context: ItemContext },
    Saved {
        context: ItemContext,
        checkpoint: LocalCheckpoint,
    },
}

pub enum CommentPublicationState {
    PublishPending,
    SyncDeferred,
}

pub enum CommentSubmissionOutcome {
    IdentityRequired { context: ItemContext },
    Saved {
        context: ItemContext,
        checkpoint: LocalCheckpoint,
        publication: CommentPublicationState,
    },
}

impl RepositoryService {
    pub fn prepare_context(
        &self,
        target: AuthoringTarget,
    ) -> Result<ContextProvisionOutcome, RepositoryError>;

    pub fn save_document(
        &self,
        request: SaveDocumentRequest,
    ) -> Result<SaveOutcome, RepositoryError>;

    pub fn save_ticket(
        &self,
        request: SaveTicketRequest,
    ) -> Result<SaveOutcome, RepositoryError>;

    pub fn submit_comment(
        &self,
        request: SubmitCommentRequest,
    ) -> Result<CommentSubmissionOutcome, RepositoryError>;
}
```

`AuthoringTarget` is revalidated by every save or submit operation. A caller
cannot redirect an operation by constructing an `ItemContext` with a different
branch or worktree. `ItemContext` is an observation returned to the caller for
progress and recovery feedback.

The implementation extends `RepositoryOperation` with context preparation,
document save, ticket save, and comment submission. It adds typed error kinds
for an unenabled repository, invalid or missing authoring target, occupied
create path, and mismatched deterministic context. Existing `InvalidIdentity`,
I/O, Git, SQLite, and injected-failure categories remain the source-aware error
boundary.

## Context Provisioning

Every operation opens the selected primary repository through the existing
canonical-root helper, reads valid tracked configuration, confirms the
configured primary branch is checked out, and requires only
`.manyhands/config.toml` to match `HEAD`. Unrelated primary-worktree changes
remain untouched. A context always branches from the primary `HEAD` commit, not
from uncommitted primary files.

`prepare_context` derives the branch, worktree name, and path from the target's
kind and ULID. It uses `Repository::find_branch`, `Repository::branch`,
`Repository::worktrees`, `Repository::find_worktree`, and
`Repository::worktree` with `WorktreeAddOptions::reference` to create and
validate the exact local context.

For `ContextIntent::Edit`, a missing context first requires a conforming
document or ticket with the requested ID and kind in primary canonical content.
For `ContextIntent::Create`, the primary context must not already contain the
requested ID. The caller may retry the same create target when the deterministic
branch or worktree exists but the new item has not yet been written. A branch,
worktree, checked-out branch, item kind, item ID, or path that differs from the
expected context returns a recovery error without mutation. Comment submission
always requires `ContextIntent::Edit`; it may never provision a context for a
missing target item.

Branch creation precedes worktree creation. A branch-creation failure leaves no
new branch or worktree. If worktree creation fails, the branch remains for an
exact retry. Cycle 03 never removes a partial branch, partial worktree, or user
file automatically.

## Canonical Writes

Private source-collection helpers enumerate only canonical paths in one primary
or item worktree: `docs/**/*.md`, ticket Markdown, and comment Markdown. They
use `canonical::parse_item` and `canonical::validate_context`; they never walk
the primary `.manyhands/worktrees` directory or write while scanning.

For a new document or ticket, the service constructs a canonical value from the
caller draft and supplied ULID. For an edit, it parses the current item, checks
the ID and kind, updates only caller-editable known fields, and retains unknown
front matter. Ticket edits retain closure fields because ticket closure is out
of scope. A document move parses the source, rejects a noncanonical or occupied
destination, serializes the updated item at the destination, and removes only
the source after the destination write succeeds.

An exact retry is the exception to the occupied-destination rule. When a
create's destination already contains the same canonical kind and ID with the
same caller-controlled content, the service preserves it and resumes the
uncommitted checkpoint or returns a no-op after ensuring registry invalidation.
A different ID, kind, metadata, or body remains an occupied-path error. A
comment retry follows the same rule and retains the existing `created_at`
timestamp rather than generating another one.

A move treats its source and destination as one recoverable owned-path pair. A
retry may find only the source, only the destination, or both. Any present
source must identify the selected document ID, and any present destination must
also match the requested serialized document state. It completes the missing
write or source removal and then checkpoints the two paths. A destination with
another document or different requested content remains a recovery error.

The service resolves a usable local or effective identity before writing
Markdown. A missing identity returns `SaveOutcome::IdentityRequired` with the
prepared context and leaves all item paths unchanged. Files are replaced
atomically and must be regular, non-symlink files when they already exist.

Before creating a document, ticket, or comment parent, a helper walks every
relative path component from the context root. It creates only missing directory
components and rejects an existing symlink or non-directory component. This
prevents a canonical relative path from escaping its selected context.

Comment submission reuses or creates the target document or ticket context,
validates its item and optional same-item parent comment, uses
`OffsetDateTime::now_utc()` for `created_at`, and writes only the canonical new
comment file. Comments are not edited in Cycle 03.

## Scoped Checkpoints

After a successful item write, the service opens the context repository and
creates an in-memory `git2::Index` seeded from the context `HEAD` tree. It adds
only owned regular-file paths with blobs read from the worktree. For a document
move, it removes the source path and adds the destination path. The live index
is never opened for writing.

Before building the tree, the service validates the owned paths and compares the
resulting temporary-index tree to context `HEAD`. Equal trees return `NoChange`.
An existing owned-path difference created by a prior failed checkpoint is still
committed on retry. Deterministic subjects are:

```text
Checkpoint document <ULID>
Checkpoint ticket <ULID>
Checkpoint comment <ULID>
```

After a commit, `mark_registered_refresh_required` sets the Cycle 02 registry
flag. A failure at that point returns `RefreshPending` with the authoritative
OID. A no-op retry still calls invalidation: it returns `NoChange` after that
write succeeds or `RefreshPending` with the current context `HEAD` OID when it
does not. Retrying must update only the registry and never create another
checkpoint.

Comment publication state derives solely from valid repository configuration:
no publication remote produces `PublishPending`; any configured publication
remote produces `SyncDeferred`. Neither state invokes a transport callback.

## Failure Injection And Testability

Extend the existing test-only `FailurePoint` and `FailOnce` helper with:

```rust
BeforeContextBranchCreation,
BeforeWorktreeCreation,
BeforeItemWrite,
BeforeCheckpointCommit,
```

The existing `BeforeRegistryWrite` point covers invalidation after a successful
commit. Production behavior keeps the no-op hook. Tests use born, enabled local
repositories and real linked worktrees created through `git2`; they never use a
system Git executable, developer configuration, application data, SSH keys, or
network access.

Tests assert actual branch refs, worktree paths, file bytes, temporary commit
trees, live-index bytes, unrelated status entries, registry rows, and commit
counts. They cover completed-create no-ops, uncommitted create retries,
immutable comment timestamps, source-only/destination-only/both-path move
retries, and Unix symlink-component rejection. Each injected failure verifies
its preserved state, then retries exactly the remaining work.

## Non-Goals

This design does not add content discovery, a general repository scanner,
SQLite context/item/problem/operation tables, lease enforcement, remote
authentication or operations, background work, context cleanup, merge or
promotion flows, desktop behavior, CLI commands, or a durable draft store.
