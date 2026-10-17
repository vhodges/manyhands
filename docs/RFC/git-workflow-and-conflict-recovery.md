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

All Manyhands product Git operations MUST use the Rust `git2` crate backed by
libgit2. The test-only SSH Git server defined by the test strategy RFC may run
Git protocol helpers for disposable fixture repositories; it is not a product
runtime dependency. Manyhands MUST open a repository inside the background
operation that uses it; repository
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

For an existing context, the item kind and ULID come only from conforming
canonical content. A branch that does not match this convention is never a
Manyhands context. A matching branch that lacks the identified conforming item
is a visible recovery problem and is not an editable context to reuse or
materialize. Cycle 03 may resume its exact partial create request when the
caller retains the requested kind and ULID and the observed branch and worktree
match the deterministic expectation, including when the worktree or requested
item write is still missing.

Cycle 03 callers MUST serialize authoring operations for the same repository.
Cycle 04 adds durable operation records after observed external steps. Cycle 05
adds the repository-scoped cross-process lease and reconciliation evidence that
make serialization mandatory across desktop and CLI processes. A manual
lifecycle action takes precedence over polling once polling is introduced.

When a user creates an item or starts editing an item:

- With no local editable context, create the deterministic branch from the
  configured primary branch and add the deterministic worktree.
- With exactly one local editable context, reuse it.
- A mismatched, duplicate, malformed, renamed, or otherwise exceptional local
  or remote context is a visible recovery condition; no context choice is
  returned.
- Wave 2 retains exactly one shared recognized context branch per item in a
  local clone. Collaborators synchronize that branch and recover divergent work
  through the merge protocol below; Wave 2 does not introduce a second context
  identity or a caller context-selection result.

Context creation MUST leave other worktrees, branches, and canonical item paths
unchanged. In Cycle 03, a partially created deterministic branch or worktree is
retained for an exact caller retry. Cycle 04 records recovery state after
observing it, and Cycle 05 proves durable reconciliation. No automatic cleanup
may delete a partial resource while it remains recoverable.

## Scoped Checkpoints

A checkpoint validates canonical content before staging. It stages only paths
owned by the requested event:

| Event | Permitted staged paths |
| --- | --- |
| Initialization | `.manyhands/config.toml` |
| Document save | The selected document Markdown path, or its former and destination Markdown paths for a move |
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

After a successful Cycle 03 checkpoint, Manyhands returns the commit OID and
marks the repository registration refresh-required without running discovery.
Cycle 04 records the observed commit OID in the operation record and requests
an index refresh. If writing succeeds but committing fails, the files remain in
the worktree for retry. If committing succeeds but invalidation or indexing
fails, the commit remains authoritative and the operation reports `index
pending` rather than retrying the commit.

## Synchronization and Merge Policy

Synchronization is deliberate except for the constrained polling behavior in
the umbrella RFC. Wave 2 authentication supplies the configured shared SSH key
and remote callbacks; this RFC defines the Git behavior after authentication.

### Remote Refs

For a configured publication remote `R`, configured primary branch `P`, and
shared item branch `C`, the authoritative remote refs are:

```text
P = refs/heads/<primary-branch>
C = refs/heads/manyhands/<kind>/<ULID>
T(P) = refs/remotes/<R>/<primary-branch>
T(C) = refs/remotes/<R>/manyhands/<kind>/<ULID>
```

The remote-tracking namespace is local metadata, never canonical content. A
deliberate item synchronization fetches only `P` and its exact `C`; primary
synchronization fetches only `P`; a poll fetches `P` and these two context
families:

```text
+refs/heads/<primary-branch>:refs/remotes/<R>/<primary-branch>
+refs/heads/manyhands/document/*:refs/remotes/<R>/manyhands/document/*
+refs/heads/manyhands/ticket/*:refs/remotes/<R>/manyhands/ticket/*
```

The leading `+` permits a remote-tracking ref to reflect a remote rewind; it
does not permit a local branch or remote branch to be force-pushed. Before a
poll prunes stale tracking refs, it records the advertised remote ref set and
compares it with the preceding observation so a deleted remote context is
visible rather than indistinguishable from a failed fetch.

An absent `P` is a recoverable publication-remote problem and changes no local
branch. An absent `C` for a context that has never been published permits first
publication. An absent `C` that was previously observed as published is a
remote-branch-deleted recovery state. It preserves the local context and
requires explicit caller confirmation before republishing; neither polling nor
an ordinary synchronization recreates it automatically.

### Deliberate Item Synchronization

Item synchronization requires a configured publication remote, selected and
usable shared key, approved host trust, a recognized local context, and a clean,
non-conflicted context worktree. After fetching, it re-observes `C`, `T(C)`, and
`T(P)` before every local mutation:

1. If `T(C)` exists, integrate it into `C`: fast-forward when local `C` is an
   ancestor, make no change when `T(C)` is an ancestor, or create a non-rebase
   merge when they diverge.
2. If `T(P)` is not an ancestor of the resulting `C`, integrate it into `C`:
   fast-forward when `C` is an ancestor or create a non-rebase merge otherwise.
3. If `T(C)` was absent and `C` has never been published, use the result of step
   2 as the first publication state. If it was previously published, return the
   remote-branch-deleted outcome unless the caller supplied explicit republish
   confirmation.
4. Push `C:C` with an ordinary, non-force refspec. A server rejection or a
   changed remote advertisement is recovery-required; the operation fetches and
   reconciles before a later retry rather than overwriting remote work.
5. Refresh discovery after the push is observed. A refresh failure returns index
   pending and never repeats an observed push or merge.

Merge commits use deterministic subjects that do not include user-authored
titles:

```text
Merge remote context <ULID>
Merge primary into <kind> <ULID>
Resolve synchronization <kind> <ULID>
```

### Deliberate Primary Synchronization

Primary synchronization requires a clean, non-conflicted primary worktree. It
fetches `P`, then fast-forwards local `P` when it is an ancestor of `T(P)`, makes
no local integration change when `T(P)` is an ancestor, or creates a non-rebase
merge when they diverge. It pushes `P:P` with an ordinary non-force refspec and
refreshes discovery after the push is observed. A dirty or conflicted primary
worktree blocks the operation without staging, stashing, committing,
discarding, or overwriting its changes.

### Polling

A poll is a remote lifecycle action, not an index-only refresh. It fetches the
poll ref set, records remote observations, and then may fast-forward a local
primary or existing context only when its worktree is clean, non-conflicted, and
its local branch is strictly behind its matching tracking ref. It validates a
new remote `C` from its fetched tree before creating exactly one local tracking
branch and deterministic worktree for that context. It then invokes the
non-mutating index refresh.

Polling MUST NOT push, checkpoint, merge, rebase, force-update, stash,
overwrite, discard, or clean up state. A dirty, divergent, conflicted,
malformed, inaccessible, renamed, unrecognized, unmaterialized, or remotely
deleted context remains locally preserved and visible with recovery guidance.

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

A successful primary publication followed by remote context-branch deletion
failure is a partial success. The local worktree and local context branch remain
until the remote deletion succeeds, preventing a later poll from rematerializing
a still-published context. The operation record MUST permit a retry that
performs remote deletion followed by local cleanup only. Once remote deletion is
observed, cleanup removes the local worktree before deleting the local context
branch. No retry may duplicate a checkpoint, merge, primary publication, or
canonical content.

Remote branch deletion uses the observed remote context OID as its precondition.
If the remote branch changed or deletion acknowledgement is ambiguous, cleanup
stops and records recovery-required; it never deletes local context state based
only on a stale record.

## Remote Coordination And Cancellation

The Wave 01 repository-common-Git-directory advisory lease remains limited to
short local Git, filesystem, and SQLite transitions. Network transport, caller
interaction, and full index scans MUST NOT hold it. Wave 02 adds a durable,
repository-scoped remote-operation reservation with operation identity, action,
manual-or-poll priority, phase, yield request, and cancellation state. The
repository/index RFC owns its physical representation.

A manual lifecycle request encountering an active poll records a yield request
and returns a typed retryable `poll yielding` outcome. The poll checks for that
request before transport, in transport progress callbacks, after fetch, between
context observations, and before each local mutation. It aborts only at those
safe points, releases its reservation, and records an interrupted poll state;
it never abandons an in-progress short atomic transition. The retried manual
operation takes a normal manual reservation and re-observes Git state.

Cancellation follows the same safe-point rule. A cancelled operation reports
whether it stopped before a durable transition or completed the current atomic
step into a recoverable state. Every operation reacquires the short repository
lease and re-observes refs, worktrees, and canonical paths after each network
step and before changing local state.

## Conflict and Interruption Recovery

When a merge conflicts, Manyhands MUST retain the affected context or primary
state and record the unresolved operation. It MUST never select one side,
discard local changes, or conceal conflict markers. The desktop and CLI RFCs
define the interaction that edits a resolution and creates its recovery
checkpoint before retrying the pending operation.

The operation record has enough state to identify the repository, item when
applicable, branch, worktree, action, publication remote, affected local and
tracking refs, observed local and remote object IDs, completed steps,
reservation/cancellation state, and redacted error category. It contains no
credentials, passphrases, private-key data, remote response body, or Markdown
draft. On the next use, reconciliation compares that record with actual Git
refs, worktrees, commits, and canonical Markdown. Git state wins over a stale
record.

Conflict resolution requires a caller-supplied resolution and expected
observations for every conflicted canonical path. After acquiring the short
lease, Manyhands verifies those observations, validates the resolution, writes
only the conflicted owned paths, creates the deterministic resolution checkpoint,
and resumes only the recorded incomplete synchronization step. A changed
observation returns external-change or recovery-required without overwriting
the worktree.

## Approved Wave 03 Request And Consent Boundaries

The [CLI](cli-contract.md), [desktop](desktop-information-architecture-and-editor.md)
and [runtime](application-runtime-and-polling.md) RFCs, approved on 2026-10-05,
bind front-end requests and confirmation to the domain operations above.
Request identity binds command, repository, target and semantic input. Replaying
an ID with changed input is rejected; completed effects are observed before
resuming only unfinished work. Records contain no draft bodies or credentials.
Insufficient evidence after cache loss requires recovery instead of blind replay.

Confirmation binds the previewed action, refs/OIDs, affected paths, primary,
remote and cleanup effects. External changes invalidate the preview and require
new confirmation. The operation's own recorded transitions do not invalidate
consent for unchanged remaining effects. A ticket's effect summary covers the
whole context branch, including code outside canonical item paths. No adapter
may expand save/refresh/startup into publication or cleanup authorization.

In-application resolution writes only owned canonical Markdown paths.
Noncanonical code, binary and unsupported structural conflicts remain visible
with external-tool guidance. After external repair, re-observe actual Git state
and require deliberate resume of eligible remaining work; do not stage arbitrary
code or treat removal of text markers as proof of resolved Git conflict state.
This does not expand the existing owned-path write boundary.

## Wave 1 Acceptance

Wave 1 Git work is complete when real temporary repositories demonstrate:

- Enablement writes the tracked config, adds the local exclude rule, and creates
  exactly one `Initialize Manyhands` commit, including on an unborn repository.
- Missing Git identity is resolved through confirmed repository-local
  configuration before a commit.
- Context creation uses the exact branch and worktree conventions and reuses
  one context. A mismatched or duplicate expected local context is visible and
  recoverable without returning a choice result; Wave 2 retains that
  one-shared-context rule for remote-materialized state.
- Document, ticket, and comment checkpoints stage only their permitted paths,
  preserve unrelated changes, and never create empty commits.
- Write, commit, index, and partial-context failures retain recoverable state
  and report the completed step without canonical data loss.
