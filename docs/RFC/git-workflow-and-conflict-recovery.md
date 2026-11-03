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

Amended 2026-10-30 by C4 (decisions 3, 7 and 11):
- **The bindings merge.** Wave 03's `item sync` and `repo sync` bindings
  merge divergent history as this section and primary synchronization
  describe. Until C4 they refused it, with `merge_required`, which they no
  longer return.
- **Republish is its own confirmed command.** The explicit republish
  confirmation of step 3 is not an option of item synchronization, whose
  consent is fixed per command; item synchronization always returns the
  remote-branch-deleted outcome there. `item republish` is confirmed. Its
  preview is built from stored evidence of the deletion, and its execution
  fetches, refuses a context advertised again or a local tip that moved,
  and then pushes exactly the previewed local commit with an ordinary
  non-force push. It integrates nothing: steps 1 and 2 do not run, so the
  pushed commit is the one the caller saw. The next item synchronization
  integrates primary as usual.

### Dedicated Local-Primary Into Context Synchronization

**Amendment, 2026-10-27:** the [closure-recovery library gate](../Cycles/wave-03-closure-recovery-gate.md)
implements a distinct local operation on its ticket branch at `03a27cb`, pending
final controller gates, whole-branch review and merge. Observe an existing registered
document/ticket context, then execute the unconfirmed F2 `context sync-local`
identity with that observation. This consumes **committed local primary**, including
when a publication remote is configured. It does not fetch, push, refresh primary,
create a context or use uncommitted primary content as merge input.

The target must name its registered branch and initially be clean. Equal/contained
input is a no-op plus discovery settlement; a behind context fast-forwards; clean
divergence creates one merge with ordered parents `[context, primary]`. Accepted
retries retain those exact parents rather than silently incorporating a newer primary.
A fresh operation incorporates subsequent primary commits. Missing effective commit
identity requires explicit identity setting and retry, never implicit config writes.

Matching written ticket closure at `Closing` is a narrow admission exception;
integrating/cleanup or unrelated lifecycle owners are not. Local completion does
not resume closure: an explicit original closure continuation adopts the validated
descendant while preserving its checkpoint and closure provenance. The
[API audit](../research/wave-03-api-audit.md#closure-recovery-library-amendment-2026-10-27)
names the actual Rust entry points. CLI and desktop adapters remain downstream.

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

Amended 2026-10-30 by C4 (decision 8): a synchronization that stops on a
conflict answers `recovery_required`, and its data names the conflict's
operation and next step, `resolve` for an all-canonical set or
`external_repair` otherwise. No recovery action or result code is added for
it.

The operation record has enough state to identify the repository, item when
applicable, branch, worktree, action, publication remote, affected local and
tracking refs, observed local and remote object IDs, completed steps,
reservation/cancellation state, and redacted error category. It contains no
credentials, passphrases, private-key data, remote response body, or Markdown
draft. On the next use, reconciliation compares that record with actual Git
refs, worktrees, commits, and canonical Markdown. Actual Git state determines
whether a recorded effect happened; it does not manufacture missing journal
authority or authorize adoption of foreign state. The dedicated local operation
records local refs/frozen parents and typed companion evidence, not a fake
publication remote or remote operation row.

Conflict resolution requires a caller-supplied resolution and expected
observations for every conflicted canonical path. After acquiring the short
lease, Manyhands verifies those observations, validates the resolution, writes
only the conflicted owned paths, creates the deterministic resolution checkpoint,
and resumes only the recorded incomplete synchronization step. A changed
observation returns external-change or recovery-required without overwriting
the worktree.

**Local recovery amendment, 2026-10-27,** scoped to the local operation on
2026-10-30 by C4 (decision 20): local inspection and explicit immutable
side reads issue opaque tokens; resolution submits a complete `{token, source}`
set with a top-level observation through `context resolve-local`. Display paths
are presentation text, not writable input. The starting request S and resolution
request R have separate durable identities. After R is accepted, exact R retry
reconciles owned progress before fresh-token validation; in the local
operation, partial R without a fully prepared candidate requires its original
source bytes. `operation resume` names R through `original_request_required`
rather than inventing source. Finished receipts are immutable; the local
operation's recoverable cancellation retains Accepted requests and acknowledges
observed cancellation generations. Local abandonment requires fresh
artifact-free proof and atomically settles all associated Accepted S/R/U
requests without deleting IDs.

Amended 2026-10-30 by C4 (decisions 13, 18 and 20), the remote rule beside
it. A remote synchronization's conflict is resolved with `conflict resolve`,
which also submits a complete `{token, source}` set with a top-level
observation. The same request reconciles its recorded attempt before any
live token is checked, and finishes an interrupted resolution with the same
commit.
- A new request may supersede a remote attempt that has only written paths,
  while every path it wrote still holds exactly what it wrote. The old
  attempt is closed. The local rule has no such supersede.
- With a written path since changed, a new request answers
  `original_request_required`, naming the attempt's request. The resume rule
  above applies to remote too: `operation resume` on an attempt with only
  written paths names its request through `original_request_required`.
- `operation resume` finishes an attempt whose candidate commit is
  prepared, from durable state, with no input.
- A remote cancellation is final: a new request completes the work.
- An outside repair is refused while an attempt is unfinished, and accepted
  again once none is.

For fresh conflicts, distinct nonempty parsed closure pairs across **all three
sides** make the whole set external-only. Known incompatible pairs in local
preparation refuse before retaining an owner or installing conflicts. External-only
is not a winning-closure policy: an external resolution still needs a clean commit
with the exact ordered recorded parents and all canonical invariants preserved.
Removing markers, staging arbitrary files or choosing one incompatible closure is
not proof of completion.

Amended 2026-10-30 by C4 (its design's fact 85 and ruling R-40, decision 6): the
same limit holds on remote. A ticket that two clones closed differently
makes a remote synchronization's conflict external-only
(`incompatible_closures`), and an outside repair that chooses one closure is
not promised to be accepted. Abandoning a pending remote conflict stays on
`01M4H33R34Z7C7EEKTY1ZCT950`, so until it lands such a conflict may have no
in-product way out.

Recovery assumes cooperative Manyhands writers using the common-Git lease. Observed
foreign edits are preserved and refused; arbitrary concurrent filesystem namespace
replacement is outside that guarantee. Interrupted coarse checkout or backend
ref/log/lock effects resume only from an exact recorded baseline or fully proven
result. Ambiguity requires operator preservation/quiescence/repair and subsequent
proof on retry, not automatic lock deletion, reset or rollback. Proven owned
metadata members retire independently: already-absent members count as retired,
exact-owned remnants can retire, and foreign remnants refuse. Journal destruction
does not reconstruct request identity from Git; dirty/conflicted/lock-bearing
states remain blocked. These are library guarantees, not delivered CLI/human
artifacts or an in-app editing workflow. Historical Cycle ledgers retain their
original results; local Linux evidence does not establish Windows/macOS execution.

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

Amended 2026-10-30 by C4 (decision 12): the CLI's `conflict show` reports
each conflicted path's kind and structure, and resolution stays set-wide, so
a conflict with any path not resolvable in-app is repaired outside as a
whole. A noncanonical path's structure is the shared classifier's reading of
the index stages, not a claim about the file's type: a text code file edited
on both sides reads `binary`, and an add/add reads `delete`. After an outside
repair, `conflict show` reports it, and `operation resume` continues it.

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
