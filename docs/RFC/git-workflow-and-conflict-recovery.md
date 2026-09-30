---
title: "Git Workflow and Conflict Recovery RFC"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YQ4C7F0H3K5M7N9P1R3T5V"
---

# Git Workflow and Conflict Recovery RFC

## Summary

This RFC defines the Git lifecycle for repository initialization, isolated item
contexts, local checkpoints, future synchronization, merge recovery, promotion,
and ticket closure. It standardizes on `git2`/libgit2 and does not require a
system Git executable at runtime.

Wave 1 implements the local initialization, context, checkpoint, and recovery
subset. SSH authentication and remote execution are Wave 2 dependencies, but
their Git-state boundaries are defined here so later work cannot weaken them.

## Scope and Dependencies

This RFC implements `MH-REPO-*`, `MH-COLLAB-001` through `MH-COLLAB-007`, and
the related resilience requirements in the [PRD](../PRD/mvp.md). It consumes
the paths, IDs, and validation rules in the
[canonical schema RFC](canonical-content-and-comment-schema.md).

The Wave 2 authentication RFC owns SSH key material and callbacks. The
repository/index RFC owns physical persistence of operation-recovery records.
The desktop and CLI RFCs own interaction details, not lifecycle semantics.

## Git Backend

All Git operations MUST use the Rust `git2` crate backed by libgit2. Manyhands
MUST open a repository inside the background operation that uses it; repository
handles and other libgit2 objects MUST NOT be retained in desktop model state or
passed between threads.

The application MUST treat the selected repository root as a non-bare working
repository. It MUST resolve its common Git directory before creating worktrees
or coordinating operations. Failure to open the repository, resolve its Git
directory, or acquire an operation lock is recoverable and MUST identify the
repository and requested operation.

## Initialization and Commit Identity

Enablement is a deliberate action. When it creates
`.manyhands/config.toml`, Manyhands MUST:

1. Verify that the configured primary worktree has no uncommitted or conflicted
   changes.
2. Ensure `.manyhands/worktrees/` is present in that repository's local
   `.git/info/exclude` file. It MUST NOT modify tracked `.gitignore` for this
   local implementation detail.
3. Write valid canonical configuration.
4. Create one primary-branch commit with subject `Initialize Manyhands`.
5. Refresh local discovery and report the commit OID or a recoverable failure.

For an unborn repository, the same initialization commit establishes the first
commit on the confirmed primary branch before an item worktree is created.
Enablement MUST NOT publish this commit automatically.

Wave 1 owns local commit-identity resolution. It MUST first use the effective
repository or global Git `user.name` and `user.email`. If either is unavailable,
Manyhands MUST prompt for both values and, after explicit confirmation, write
them to local repository Git configuration before committing. It MUST NOT invent
an application-wide identity or require external Git configuration.

## Context Branches and Worktrees

An item's context branch is deterministic:

```text
manyhands/ticket/<ULID>
manyhands/document/<ULID>
```

The local worktree path is deterministic:

```text
<repository-root>/.manyhands/worktrees/<ULID>/
```

The item kind and ULID come only from conforming canonical content. A branch
that does not match this convention is never a Manyhands context. A branch that
matches but lacks the identified conforming item is a visible recovery problem,
not a context to reuse or materialize.

Before creating, reusing, synchronizing, promoting, or closing a context,
Manyhands MUST serialize the operation with other Manyhands operations for the
same repository. The persistence RFC defines the cross-process lock and durable
operation record. A manual lifecycle action takes precedence over polling.

When a user creates an item or starts editing an item:

- With no local editable context, create the deterministic branch from the
  configured primary branch and add the deterministic worktree.
- With exactly one local editable context, reuse it.
- With multiple local editable contexts, return each branch and worktree label
  and require the caller to choose one.

Context creation MUST leave other worktrees, branches, and canonical item paths
unchanged. A partially created branch or worktree is retained only when needed
for recovery; otherwise cleanup is limited to resources created by the failed
operation and is recorded for retry.

## Scoped Checkpoints

A checkpoint validates canonical content before staging. It stages only paths
owned by the requested event:

| Event | Permitted staged paths |
| --- | --- |
| Initialization | `.manyhands/config.toml` |
| Document save | The selected document Markdown path |
| Ticket save | `.manyhands/tickets/<id>/ticket.md` |
| Comment submit | `.manyhands/comments/<item-id>/<comment-id>.md` |
| Ticket close | The ticket Markdown path containing closure metadata |

Unrelated modified, untracked, deleted, or conflicted paths MUST remain
untouched. A checkpoint MUST detect that its permitted paths have no effective
change and report a no-op without creating an empty commit.

Commit subjects are deterministic and do not include user-authored titles:

```text
Initialize Manyhands
Checkpoint document <ULID>
Checkpoint ticket <ULID>
Checkpoint comment <ULID>
Close ticket <ULID>
```

After a successful checkpoint, Manyhands records the commit OID in the
operation record, requests an index refresh, and reports the outcome. If writing
succeeds but committing fails, the files remain in the worktree for retry. If
committing succeeds but indexing fails, the commit remains authoritative and the
operation reports `index pending` rather than retrying the commit.

## Synchronization and Merge Policy

Synchronization is deliberate except for the constrained polling behavior in
the umbrella RFC. Wave 2 authentication supplies the configured shared SSH key
and remote callbacks; this RFC defines the Git behavior after authentication.

For item-context synchronization, Manyhands fetches the configured primary ref
and matching `manyhands/<kind>/<id>` ref. It merges relevant fetched context and
primary changes into the selected context when required, then publishes the
current context when safe. It MUST NOT automatically rebase published commits.

For deliberate primary synchronization, Manyhands requires a clean primary
worktree, fetches the configured primary ref, safely integrates it into local
primary, and publishes the result. A dirty or conflicted primary worktree blocks
the operation without staging, stashing, committing, discarding, or overwriting
its changes.

Background polling may fetch and fast-forward only under the PRD conditions. It
MUST NOT create checkpoints, merge, rebase, push, or clean up state.

## Promotion, Closure, and Cleanup

Managed-document promotion and ticket closure require confirmation before their
first lifecycle step. Both perform final validation and checkpointing as needed,
synchronize when a publication remote exists, integrate the context into
primary with a non-fast-forward merge commit, publish primary when configured,
and only then remove the local worktree and local and remote context branches.

Ticket closure writes its schema-defined closure metadata before integration.
Document promotion does not close the document. If no publication remote exists,
the local merge and cleanup complete and the primary result is marked publish
pending for later deliberate primary synchronization.

A publication success followed by branch-deletion failure is a partial success.
The operation record MUST permit a retry that performs cleanup only. No retry
may duplicate a merge, checkpoint, or canonical content.

## Conflict and Interruption Recovery

When a merge conflicts, Manyhands MUST retain the affected context or primary
state and record the unresolved operation. It MUST never select one side,
discard local changes, or conceal conflict markers. The desktop and CLI RFCs
will define the interaction that edits a resolution and creates its recovery
checkpoint before retrying the pending operation.

The operation record has enough state to identify the repository, item when
applicable, branch, worktree, action, completed steps, and last error. On the
next use, reconciliation compares that record with actual Git refs, worktrees,
and commits. Git state wins over a stale record.

## Wave 1 Acceptance

Wave 1 Git work is complete when real temporary repositories demonstrate:

- Enablement writes the tracked config, adds the local exclude rule, and creates
  exactly one `Initialize Manyhands` commit, including on an unborn repository.
- Missing Git identity is resolved through confirmed repository-local
  configuration before a commit.
- Context creation uses the exact branch and worktree conventions, reuses one
  context, and requires selection for multiple contexts.
- Document, ticket, and comment checkpoints stage only their permitted paths,
  preserve unrelated changes, and never create empty commits.
- Write, commit, index, and partial-context failures retain recoverable state
  and report the completed step without canonical data loss.
