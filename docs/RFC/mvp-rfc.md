---
title: "MVP/Dogfooding Architecture RFC"
date: 2026-09-29
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
---

# MVP/Dogfooding Architecture RFC

## Summary

This RFC defines the architectural direction, consent boundaries, and delivery
governance for the Manyhands collaboration-complete dogfooding release. It is
the umbrella RFC for the approved product requirements in
[`docs/PRD/mvp.md`](../PRD/mvp.md), version 0.3.

It resolves cross-cutting decisions that must be consistent across content,
Git, credentials, indexing, desktop, and CLI work. It does not prescribe the
exact Markdown schema, file layout, database schema, command grammar, or UI
components. Focused RFCs own those decisions within the constraints here.

This RFC is `approved` and is an implementation authority. The PRD amendments
recorded here are adopted in PRD version 0.3.

## Motivation

Manyhands must make Git-backed collaboration safe for people who are not Git
operators while retaining Git repositories and Markdown as the source of
truth. The MVP must therefore make the lifecycle of an item understandable:
an item is edited in an isolated context, saved as a local checkpoint,
deliberately synchronized, and, when appropriate, deliberately promoted to the
primary branch.

The PRD correctly reserves many implementation decisions for RFCs. Without an
umbrella decision record, however, independently designed schemas, lifecycle
operations, and interfaces could make incompatible assumptions about primary
branches, publication remotes, recoverable failures, or cleanup. This RFC
establishes those shared rules before implementation Waves begin.

## Scope

This RFC governs the MVP/dogfooding release for trusted collaborators using
enabled local Git repositories. It covers:

- Repository-level primary-branch and publication-remote selection.
- Isolated editing contexts, local checkpointing, and discovery precedence.
- Background remote polling, safe fast-forward updates, synchronization, merge,
  conflict recovery, promotion, ticket closure, and cleanup consent boundaries.
- SSH transport, key selection, passphrase handling, and Git commit identity.
- The boundary between canonical repository state and application-local state.
- The focused RFC set and the Wave/Cycle delivery structure.

This RFC does not add boards, rollups, templates, meta-repository propagation,
multi-repository planning, application authorization, automatic Git-forge key
upload, HTTP(S) publishing, or ticket reopening. Those remain outside the MVP
unless the PRD is revised.

## Conformance

The terms **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative.

All focused RFCs, Waves, Cycles, desktop behavior, CLI behavior, and tests that
claim MVP conformance MUST comply with this RFC and the approved PRD. A focused
RFC MAY refine a mechanism, but it MUST NOT weaken a PRD acceptance criterion
or replace a decision made here without an approved amendment to this RFC and,
when product behavior changes, the PRD.

## Terms

| Term | Meaning |
| --- | --- |
| Enabled repository | A local Git repository that the user has opted into managing with Manyhands. |
| Primary branch | The explicitly configured branch that represents the repository's primary copy for Manyhands. |
| Publication remote | The optional, explicitly configured SSH remote to which Manyhands publishes collaboration work. |
| Editing context | An item-specific Git branch and worktree used for an item's isolated edits. |
| Checkpoint | An inspectable local Git commit containing a valid changed item or comment event. |
| Synchronization | A deliberate operation that fetches relevant remote changes, safely integrates them into an editing context when possible, and publishes current work. |
| Remote polling | An automatic, configured operation that fetches a publication remote, fast-forwards only clean local state, discovers recognized item contexts, and refreshes discovery. |
| Primary synchronization | A deliberate repository-level operation that safely integrates and publishes the configured primary branch. |
| Promotion | The confirmed lifecycle that merges a managed document's editing context into the primary branch and cleans up that context. |
| Ticket closure | The confirmed lifecycle that records ticket closure, merges its editing context into the primary branch, and cleans up that context. |
| Publish pending | A successful local checkpoint that has not been published because no publication remote is configured or synchronization could not complete. |

## Adopted PRD Amendments

PRD version 0.3 adopts the following product behavior for a complete local
lifecycle and safe remote collaboration.

### Managed-Document Promotion

PRD version 0.3 adds `MH-COLLAB-007: Promote Managed Documents Through a
Confirmed Lifecycle`:

**Priority:** Must

**Rationale:** A managed document edited in an isolated context needs an
explicit, safe path to become the primary copy without treating the document as
a closed ticket.

**Acceptance Criteria:**

- A user can deliberately request promotion of a managed document's editing
  context.
- Before promotion begins, Manyhands identifies the item, primary branch,
  publication remote when configured, merge, publication, and cleanup effects,
  then requires explicit user confirmation.
- After confirmation, Manyhands performs a final save and checkpoint when
  needed, synchronizes the editing context when a publication remote is
  configured, integrates it into the configured primary branch, publishes the
  integrated primary result when a publication remote is configured, and then
  removes the document's local and remote branch and worktree state.
- After successful promotion, the promoted document is visible in the primary
  copy. A later edit creates a new isolated context.
- If a promotion prerequisite fails, Manyhands identifies the item,
  repository, and context; preserves sufficient state to retry or recover; and
  performs no premature cleanup.

### Local-Only Comment Submission

PRD version 0.3 updates `MH-COLLAB-003` and `MH-COLLAB-004` so comment submission remains a
synchronization-triggering action when a publication remote is configured, but
does not make local authoring unavailable when a remote or network is absent.

After a comment or reply checkpoints successfully, Manyhands MUST report one
of these outcomes:

- Published or already current after synchronization completes.
- Saved locally with publication pending when no publication remote is
  configured.
- Saved locally with publication pending and actionable recovery when the
  synchronization attempt cannot complete, including an unavailable network,
  authentication failure, or conflict.

The comment or reply MUST remain discoverable in its local editing context in
all three cases. A later deliberate synchronization MUST be able to publish the
pending checkpoint without duplicating the comment.

### Consent Boundary Update

PRD version 0.3 updates `MH-COLLAB-006` so confirmed managed-document promotion is the only
non-ticket lifecycle permitted to merge into the primary branch or remove its
own local or remote branch and worktree state. Synchronization alone MUST NOT
merge into primary or perform cleanup.

PRD version 0.3 updates `MH-NFR-006` so its idempotency requirements apply equally to document
promotion. Update the scope and roadmap statement that only tickets are subject
to merge and cleanup to include confirmed managed-document promotion.

### Local-Only Promotion, Closure, and Later Publication

PRD version 0.3 adds a deliberate repository-level `Sync primary
branch` action. It fetches, safely integrates, and publishes the configured
primary branch after a publication remote is selected. The action MUST report
`published`, `already current`, or actionable recovery and MUST preserve local
primary work on failure.

PRD version 0.3 updates `MH-COLLAB-005` and `MH-COLLAB-007` so a confirmed closure or
promotion in a repository without a publication remote completes its local
integration and context cleanup, then reports the primary result as `publish
pending`. Once an SSH publication remote is configured, the user can use `Sync
primary branch` to publish that result. The lack of a remote MUST NOT block the
local-only lifecycle or cause context retention solely for later publication.

### Background Remote Polling and Safe Fast-Forward Updates

PRD version 0.3 updates `MH-COLLAB-004` and `MH-COLLAB-006` to permit configured background
remote polling as a narrowly constrained exception to deliberate
synchronization. For a repository with an SSH publication remote, Manyhands
MUST enable polling by default, start an initial poll at desktop application
launch or CLI daemon startup, and allow the user to pause polling or configure
its interval for that repository.

Polling MUST fetch remote state, including new remote branches, and MUST refresh
discovery after the fetch. It MUST fast-forward the configured primary branch or
an existing item context when that worktree is clean and its local branch is
strictly behind the corresponding remote-tracking branch. It MUST NOT
automatically push, commit, merge, rebase, stash, overwrite, discard, delete a
branch, or delete a worktree.

When polling discovers a new recognized Manyhands item-context branch, it MUST
create the corresponding local tracking branch and worktree, then index the
managed item folder and comments it contains. A dirty, divergent, conflicted,
deleted, renamed, malformed, or unrecognized remote context MUST remain
unmodified locally and be surfaced with an actionable status. Remote deletion
or invalidation MUST NOT remove an existing local worktree.

PRD version 0.3 updates `MH-INDEX-002` so a configured polling refresh includes this remote
discovery and safe local update behavior rather than only an application-local
index refresh.

PRD version 0.3 updates `MH-CLI-001` to require a documented CLI daemon mode that performs the
same configured polling lifecycle as the desktop application. One-shot CLI
commands MUST NOT create an implicit resident poller; they MAY explicitly run a
single poll, configure polling, or report polling status. The CLI-contract RFC
owns daemon supervision, process lifetime, locking, and machine-readable event
details.

### Shared SSH Key and Startup Unlock

PRD version 0.3 updates `MH-CRED-001` to replace repository- and remote-specific SSH-key
selection with one user-selected Manyhands key shared by every MVP
Git-over-SSH operation. The first configured remote poll in an application
session is a valid first key use and MUST prompt to unlock a protected shared
key. If the user cancels the prompt or unlocking fails, affected polling MUST
pause and report an Unlock or Sync recovery action without mutating local work.

The selected shared key remains subject to the existing generation, import,
session-only passphrase retention, removal, and redaction requirements.

## MVP Operating Model

### Repository Configuration

Each enabled repository MUST have an explicitly selected primary branch. During
enablement, Manyhands MAY suggest the remote default branch or current branch,
but the user MUST confirm the selected branch. The selected branch MUST exist
locally before it can be used for promotion or ticket closure. Repositories
without a remote remain supported.

An enabled repository MAY have one explicitly selected publication remote. A
repository with no publication remote is local-only: creation, editing,
checkpointing, indexing, and discovery remain available, while publication is
reported as pending. If a repository has multiple remotes, Manyhands MUST NOT
infer a publication target from a remote name, upstream, or Git's push-default
configuration.

For a repository with an SSH publication remote, remote polling MUST be enabled
by default. A user MUST be able to pause it or configure its interval for that
repository. A repository without a publication remote MUST NOT schedule remote
polling.

Primary-branch and publication-remote selections are repository-level
Manyhands configuration. The canonical-schema RFC MUST specify their tracked
representation and the application-local registry needed to opt a local clone
into management. SSH-key selection is local user configuration and MUST NOT be
written into managed Markdown or repository configuration.

Manyhands MAY list and manage remote URLs using supported Git facilities. Only
an SSH remote may be selected as the MVP publication remote. Existing HTTP(S)
remotes remain visible and manageable, but the interface MUST explain that they
cannot publish through Manyhands in this release and provide recovery guidance
to select or configure an SSH remote.

### Canonical and Application-Local State

Markdown content, repository configuration defined by the canonical-schema
RFC, and Git history are canonical. The application-local repository registry,
SQLite index, key registration metadata, passphrases, UI state, and operation
progress records are not canonical item content.

The SQLite index MUST remain rebuildable from accessible canonical state. It
MUST NOT rewrite Markdown, commits, branches, worktrees, remotes, or Git
configuration while refreshing or rebuilding. A lost or corrupt index cannot
invalidate a checkpoint, promotion, closure, or locally recoverable draft.

The repository/index RFC MUST define how interrupted lifecycle operations are
reconciled without treating an index record as proof that a Git operation
completed. Git and canonical Markdown state are authoritative when a record and
repository disagree.

### Editing Contexts and Checkpoints

Creating an item or editing an item that has no editable context MUST provision
an item-specific branch and worktree. If exactly one local editable context
exists, Manyhands MUST reuse it. If more than one exists, the user MUST choose
one before editing continues. Every context displayed in a list or item view
MUST be distinguishable by branch and worktree label.

The exact item identity, branch naming, worktree location, and detection of
active contexts are delegated to the canonical-schema and Git-workflow RFCs.
Those RFCs MUST ensure that provisioning one context cannot silently mutate
another item's content, metadata, or context.

Saving a changed, conforming item MUST write only the files required for that
item change, create one inspectable checkpoint, and refresh affected discovery
state. A checkpoint operation MUST NOT stage, commit, stash, overwrite, or
discard unrelated user changes. A no-change save MUST create no empty commit.
If writing, validation, commit creation, or index refresh fails, Manyhands MUST
report the completed and incomplete steps while preserving recoverable content.

### Comments and Pending Publication

Comment and reply creation follows the same context provisioning and checkpoint
rules as item editing. A comment submission is deliberate consent to attempt
synchronization after its checkpoint succeeds.

When no publication remote exists, Manyhands MUST not attempt a remote
operation. It MUST retain the checkpoint and report `publish pending`. When a
publication remote exists but synchronization cannot complete, Manyhands MUST
retain the checkpoint, report `publish pending`, identify the failed operation,
and offer a retry or recovery action. An item opened in that context MUST show
the locally checkpointed comment or reply.

## Synchronization, Integration, and Recovery

### Synchronization Contract

Manyhands MUST perform synchronization only after a direct user request, a
declared synchronization-triggering content action, or an explicitly confirmed
promotion or ticket-close action. Background remote polling is separately
authorized only within the constrained behavior below. It MUST NOT publish,
merge, rebase, create checkpoints, delete branches, delete worktrees, or
otherwise perform an irreversible lifecycle action.

A synchronization operation against a publication remote MUST:

1. Identify the repository, publication remote, and either the editing context
   or primary branch being synchronized.
2. Fetch relevant remote primary-branch and item-context state.
3. Safely integrate relevant remote changes into the selected editing context
   or primary branch when required.
4. Publish current context or primary-branch state when safe.
5. Refresh discovery state and report `published`, `already current`, or a
   recoverable outcome.

The Git-workflow RFC MUST specify the precise refs, ordering, detection of an
existing remote item branch, primary-branch synchronization behavior, and
operation journal needed to make retries idempotent. It MUST NOT use automatic
rebase. When divergent published history must be integrated, Manyhands MUST
preserve it with a merge. Fast-forward-only promotion is insufficient:
promoting an item into the primary branch MUST create a non-fast-forward merge
commit so the integrated item history remains inspectable after branch cleanup.

### Remote Polling Contract

Remote polling runs immediately at desktop application launch and in the CLI's
documented daemon mode for every repository with an SSH publication remote,
then at that repository's configured interval. One-shot CLI commands do not
start a resident poller, but MAY run one explicit poll. The repository/index and
CLI-contract RFCs MUST define the default interval, bounded retry/backoff,
scheduler ownership, daemon lifetime, and application-local scheduling state. A
user MUST be able to pause polling.

Polling MUST be serialized with manual synchronization, primary
synchronization, promotion, and ticket closure for the same repository. A
manual lifecycle operation takes precedence; polling MUST wait or stop at a
safe point before the manual operation changes Git state.

For each poll, Manyhands MUST:

1. Fetch the selected publication remote using the shared SSH key.
2. Inspect the configured primary branch and recognized remote item-context
   branches using the context convention defined by the Git-workflow RFC.
3. Fast-forward the local primary branch only if its worktree has no
   uncommitted or conflicted changes and the local branch is an ancestor of its
   remote-tracking branch.
4. Fast-forward an existing item context only if its worktree has no
   uncommitted or conflicted changes and its local branch is an ancestor of its
   remote-tracking branch.
5. Materialize each newly discovered, recognized, conforming remote
   item-context branch as exactly one local tracking branch and worktree, then
   index the managed item folder and its comments.
6. Refresh discovery state and report an informative polling outcome.

Polling MUST leave local state unchanged when a branch is dirty, divergent,
conflicted, deleted remotely, renamed remotely, malformed, inaccessible, or
not recognized as a Manyhands item context. It MUST surface the state and a
recovery action instead. In particular, remote deletion or invalidation of an
item branch MUST preserve an existing local worktree and mark it accordingly.

The Git-workflow RFC MUST define branch recognition, remote-ref selection,
fast-forward preconditions, local tracking-branch creation, and idempotent
worktree materialization. The canonical-schema RFC MUST define the validation
required before a remote branch is treated as a managed item context. The
repository/index RFC MUST define how unmaterialized, malformed, and
remote-deleted contexts appear in discovery without becoming canonical content.

### Conflict Recovery

If a merge encounters a content conflict, Manyhands MUST retain the affected
editing context or primary synchronization state in a recoverable state and
provide an in-application conflict resolution workflow. That workflow MUST
identify the item when applicable, repository, worktree, and affected Markdown
files; allow the user to inspect and edit a resolution; and create the required
recovery checkpoint before retrying the interrupted operation.

The desktop/editor and CLI RFCs MUST define equivalent safe interaction models
for resolving conflicts. Neither interface may silently select one side,
discard local work, overwrite user-authored content, or conceal an unresolved
conflict. The exact diff presentation and conflict-marker handling are owned by
the desktop/editor and CLI RFCs.

When an operation needs to update the primary worktree, any uncommitted or
conflicted primary-worktree state MUST block that operation. Manyhands MUST NOT
stage, commit, stash, discard, or overwrite that state. It MUST identify the
blocking paths and allow a retry after the user resolves them outside the
operation.

### Safe Cancellation and Interruption

Long-running lifecycle operations MUST expose progress and permit cancellation
only at safe points. A cancellation request MUST report either that cancellation
completed before the next irreversible step or that the operation continued to
a safe recovery state.

The Git-workflow RFC MUST define durable reconciliation for an application or
process interruption during context creation, synchronization, promotion,
closure, publication, or cleanup. Retrying MUST complete remaining work or
identify the recovery step; it MUST NOT duplicate Markdown, comments,
checkpoints, merges, or deletions.

## Promotion and Ticket Closure

### Managed-Document Promotion

Promotion is the only managed-document operation that can integrate into the
primary branch or clean up its editing context. The interface MUST name this
operation `Approve and merge` or use equivalent language that explicitly
communicates its effects.

Before promotion begins, Manyhands MUST show the managed document, editing
branch, target primary branch, configured publication remote when any, and the
facts that it will merge, publish, and remove the branch and worktree. The user
MUST confirm after seeing that information.

After confirmation, Manyhands MUST:

1. Perform a final validation, save, and checkpoint when needed.
2. Synchronize the editing context when a publication remote is configured and
   preserve any required recovery state.
3. Ensure the primary worktree is suitable for integration.
4. Integrate the editing branch into primary with a non-fast-forward merge.
5. Publish the integrated primary branch when a publication remote exists.
6. Remove the local worktree and local and remote item branches only after the
   required integration and publication steps succeed.

If no publication remote is configured, promotion MUST complete the local merge
and cleanup, then report the primary result as `publish pending`. If a
publication remote is later configured, `Sync primary branch` MUST be available
to publish it. If a step fails, cleanup MUST wait. A result in which primary
publication succeeds but branch deletion fails is a recoverable partial
success; retrying MUST perform only the remaining cleanup. Once promotion is
successful, a later edit of the document MUST provision a new editing context
from the current primary copy.

### Ticket Closure

Ticket closure uses the same merge, publication, and cleanup safety rules as
managed-document promotion, with the additional semantic effect of recording
that the ticket is closed before integration. The canonical-schema RFC MUST
define a separate closure timestamp and actor representation. It MUST NOT
overwrite the project's free-form ticket status solely to indicate closure.

The default ticket list MUST use the closure marker to hide closed tickets
while retaining a way to discover and view them. Ticket reopening is not an MVP
requirement and is not defined by this RFC.

Ticket closure requires explicit confirmation before its first lifecycle step.
The confirmation and progress feedback MUST name the ticket, editing branch,
target primary branch, publication remote when any, closure effect, merge,
publication, and cleanup effect. Cleanup begins only after the closed primary
copy has been successfully integrated and published when a publication remote
exists. If no publication remote is configured, closure MUST complete the local
merge and cleanup, then report the primary result as `publish pending`. A later
`Sync primary branch` action MUST be able to publish the result after an SSH
publication remote is configured.

## Identity, SSH, and Secret Handling

Manyhands MUST obtain a commit identity from the effective repository or global
Git configuration when one is available. If no usable identity is available
before a checkpoint, it MUST prompt for a name and email and, after user
confirmation, write that identity to the local repository's Git configuration.
It MUST NOT invent an application-wide identity or require users to configure
Git externally before they can complete the workflow.

For SSH operations, a user-selected shared Manyhands key MUST be used for every
MVP Git-over-SSH operation. Repository- and remote-specific key mappings are
out of scope. When polling first uses a protected shared key during an
application session, Manyhands MUST prompt for its passphrase. If the prompt is
cancelled or the key cannot be unlocked, it MUST pause affected polling and
surface an Unlock or Sync action without changing local content.

Imported keys MUST be registered by non-secret identifying metadata and a
reference to their source without copying or deleting externally supplied key
material. Manyhands-generated private keys MUST be stored only in an
operating-system-appropriate, owner-only location. Removing a generated key
MUST unregister it by default while retaining its files. A separate destructive
action MAY delete generated key files only after explicit confirmation.

Manyhands MUST prompt for a protected key's passphrase on first use in an
application session and retain it only for that session. It MUST NOT persist
passphrases, private key content, credentials, or OAuth-like tokens in
application data, Git configuration it writes, managed Markdown, indexes,
logs, progress messages, errors, or machine-readable CLI output.

The authentication RFC MUST specify key algorithms, protected storage, secure
deletion, shared-key selection, passphrase-memory lifetime, startup polling
prompt behavior, transport callbacks or process handling, and redaction
verification. It MUST support the selected key consistently for every
Git-over-SSH operation and report actionable key, repository, remote, and
operation context on failure.

## Focused RFCs

The following RFCs are required. Each MUST cite this RFC and the PRD
requirements it implements, define its own acceptance tests, and record any
remaining decision as a blocker rather than silently relying on an assumption.

| RFC | Required decisions and outputs | Required before |
| --- | --- | --- |
| Canonical content and comment schema | Repository marker/config, primary/publication settings representation, paths, IDs, front matter, comment threading, closure metadata, migration and compatibility fixtures. | Wave 1 content work |
| Git workflow and conflict/recovery | Context naming and provisioning, remote-ref protocol, branch recognition, fast-forward preconditions, merge order, checkpoint messages, operation serialization, retry, interruption reconciliation, cleanup. | Wave 1 lifecycle work |
| Repository/index persistence and refresh | Local registry, SQLite schema, scan/rebuild/refresh behavior, external changes, polling schedule/backoff, idempotent worktree materialization, scale limits. | Wave 1 discovery work |
| Authentication and credential handling | Git identity prompt, shared-key generation/import/storage/removal, session passphrases, startup polling unlock, SSH-only transport, redaction. | Wave 2 remote work |
| Desktop information architecture and editor | Navigation, open-item/context selection, accessible controls, polling status/pause/recovery, Markdown editing and conflict resolution. | Wave 3 desktop gate |
| CLI contract | Command taxonomy, safe input/output boundaries, polling configuration/status and daemon mode, JSON schema, recovery states, exit statuses, conflict interaction. | Wave 3 CLI gate |
| Test and compatibility strategy | Fixture repositories, polling and lifecycle fault injection, real remote journeys, cross-platform and Git matrix, performance limits. | Each Wave gate |

The focused RFCs MAY be drafted in parallel. Their decisions MUST be approved
in dependency order before the Wave that relies on them begins.

## Wave and Cycle Governance

A **Wave** is an integration milestone. A Wave contains ordered **Cycles**,
where each Cycle is a small, independently verifiable work package. A Cycle
MUST NOT depend on a later Cycle in the same Wave. A Wave completes only when
all of its Cycles and its integration gate have completed.

Wave documents MUST be created at:

```text
docs/Waves/wave-NN-<slug>.md
```

Each Wave document MUST state its outcome, PRD/RFC traceability, entry gate,
ordered Cycle list, integration gate, deferred work, risks, and the evidence
required to declare it complete.

Cycle documents MUST be created at:

```text
docs/Cycles/wave-NN-cycle-NN-<slug>.md
```

Each Cycle document MUST state its parent Wave, purpose, prerequisites,
in-scope and out-of-scope work, PRD/RFC traceability, implementation and test
changes, recovery considerations, verification commands or journeys, and exit
criteria. A Cycle that changes a lifecycle operation MUST include a failure or
interruption case in its verification.

The initial delivery sequence is:

| Wave | Outcome | Entry and integration gate |
| --- | --- | --- |
| Wave 1: Foundations | Canonical content, repository configuration, local editing contexts, checkpoints, and rebuildable discovery operate against real local repositories. | PRD version 0.3 and foundational RFCs approved; local lifecycle and index-rebuild evidence complete. |
| Wave 2: Collaboration | SSH polling and synchronization, pending publication, comments, conflict recovery, document promotion, and ticket closure operate safely against real remotes. | Authentication and Git recovery RFCs approved; polling, remote, conflict, promotion, closure, retry, and cleanup evidence complete. |
| Wave 3: Dogfooding | Desktop and CLI provide the complete accessible MVP workflow and demonstrate the PRD journeys on supported platforms. | Desktop, CLI, and test RFCs approved; real-repository end-to-end evidence complete. |

This table establishes sequencing, not a substitute for Cycle documents. No
implementation Cycle may be started until its Wave entry gate and direct RFC
dependencies are approved.

## Requirement Traceability

| PRD area | Governing RFC responsibility |
| --- | --- |
| `MH-PROD-001` to `MH-PROD-002` | This RFC, canonical schema, Git workflow, desktop, CLI |
| `MH-REPO-001` to `MH-REPO-004` | This RFC, canonical schema, repository/index, CLI, desktop |
| `MH-CRED-001` and `MH-NFR-003` | This RFC and authentication/credential handling |
| `MH-CONTENT-001` to `MH-CONTENT-004` | Canonical schema, Git workflow, repository/index, desktop, CLI |
| `MH-COMMENT-001` to `MH-COMMENT-002` | Canonical schema, Git workflow, repository/index, desktop, CLI |
| `MH-COLLAB-001` to `MH-COLLAB-006` | This RFC, Git workflow, authentication, desktop, CLI |
| `MH-COLLAB-007` | This RFC, Git workflow, desktop, CLI, test strategy |
| `MH-INDEX-001` to `MH-INDEX-003` | Repository/index persistence and refresh |
| `MH-UX-001`, `MH-CLI-001`, and `MH-NFR-002` to `MH-NFR-008` | Desktop, CLI, test/compatibility, with this RFC's consent and recovery constraints |

## Validation and Release Gates

Each focused RFC MUST define automated and manual evidence for the decisions it
owns. Unit tests alone are insufficient for lifecycle work. Tests MUST use real
temporary Git repositories and, where synchronization is involved, a real
remote fixture or an equivalent Git server fixture.

The test and compatibility RFC MUST require evidence for at least these cases:

- Offline item editing, local checkpointing, index refresh, and later explicit
  synchronization.
- Background polling that fast-forwards clean primary and item contexts, then
  refreshes discovery without a push, checkpoint, merge, rebase, or cleanup.
- Dirty, divergent, conflicted, remote-deleted, renamed, malformed, and
  unrecognized contexts preserved locally and surfaced for recovery.
- New recognized remote item-context branches materialized once as a local
  worktree with their managed item folder and comments indexed.
- Comment creation with successful publication, no publication remote, and a
  recoverable remote failure.
- Concurrent conflicting changes resolved in application, followed by a
  successful retry without lost content.
- Confirmed managed-document promotion, including cleanup only after primary
  publication when a remote exists, local-only pending publication, primary
  synchronization after a remote is added, and a later fresh editing context.
- Confirmed ticket closure with closure metadata, primary visibility, and
  retryable failure before cleanup, including local-only pending publication.
- Dirty or conflicted primary worktree blocking promotion without mutation.
- Index rebuild after loss, external edits, malformed content, and inaccessible
  paths without canonical rewrites.
- SSH key import, generated-key removal, passphrase session behavior, invalid
  key recovery, first-poll unlock cancellation, and secret-redaction checks.
- Serialization between a scheduled poll and a manual synchronization,
  promotion, or closure operation.
- Desktop polling and CLI daemon polling with documented one-shot CLI behavior,
  bounded shutdown, and no concurrent poller corruption.
- Desktop keyboard workflows and CLI human-readable, JSON, no-op, failure, and
  recovery exit behavior.

Wave 3 cannot complete on fabricated UI states or mock-only Git outcomes. It
MUST complete the PRD end-to-end journeys with real repositories and trusted
collaborators, including the required Windows, macOS, and Linux coverage
defined by the test and compatibility RFC.

## Deferred Decisions and Risks

The following decisions remain intentionally delegated and block the relevant
Cycle until their focused RFC resolves them:

- Exact repository marker/configuration format, paths, Markdown front matter,
  IDs, comment layout, compatibility, and migration policy.
- Git backend choice, including the boundary between an embedded Git library and
  a system Git executable, and its cross-platform behavior.
- Exact branch naming, worktree placement, ref names, merge implementation,
  operation journal, remote branch discovery, polling interval, backoff, and
  cross-process Git locking.
- SQLite tables, application-data locations, polling interval defaults, file
  watching, locking, and scale limits.
- Key algorithms, OS-specific secure storage/deletion, host-key verification,
  and authenticated transport implementation.
- Detailed desktop navigation, editor behavior, conflict presentation,
  accessibility mechanics, and CLI command/JSON schemas.
- Platform and installed-Git compatibility matrix, including case-sensitive
  paths, filesystem failures, protected remote branches, and interrupted
  process recovery.

Focused RFCs MUST explicitly identify their security, compatibility, and data
loss risks. A decision that cannot meet the PRD's local-first, recovery,
redaction, or no-silent-loss requirements MUST be escalated as an RFC or PRD
revision rather than accepted as an implementation limitation.

## Decision Log

| Decision | Status | Rationale |
| --- | --- | --- |
| `mvp-rfc.md` is an umbrella RFC. | Accepted | Cross-cutting lifecycle rules need one authority while specialized designs remain reviewable. |
| Waves contain ordered, independently verifiable Cycles. | Accepted | Makes delivery sequencing and evidence explicit. |
| Primary branch is explicit per enabled repository. | Accepted | Avoids unsafe inference in remote-less and nonstandard repositories. |
| Publication remote is explicit and optional per repository. | Accepted | Supports intentional collaboration and fully local repositories. |
| MVP publication requires SSH. | Accepted | Delivers configured-key behavior without persisting HTTP(S) credentials. |
| Managed documents have confirmed promotion with cleanup. | Adopted in PRD 0.3 | Isolated document edits otherwise lack a path to the primary copy. |
| Local comments can be publish pending. | Adopted in PRD 0.3 | Preserves local-first authoring when remote publication is impossible. |
| Local-only promotion and closure clean up with primary publication pending. | Adopted in PRD 0.3 | Preserves a complete local lifecycle and provides a deliberate later publication path. |
| Integrations preserve history with merges; promotion uses non-fast-forward merge. | Accepted | Retains inspectable checkpoints and avoids rewriting published work. |
| Conflicts are resolved in application. | Accepted | Non-developers must not become Git operators to recover. |
| Primary worktree changes block promotion without mutation. | Accepted | Protects unrelated user work from lifecycle automation. |
| Ticket type and status are free-form; closure is separate metadata. | Accepted | Supports project conventions while making closed tickets reliably discoverable. |
| Missing Git identity prompts for repository-local configuration. | Accepted | Allows automatic checkpoints without inventing an app identity. |
| One shared SSH key is used for all MVP Git-over-SSH operations. | Adopted in PRD 0.3 | Supports startup unlock and safe unattended polling without per-remote key prompts. |
| Generated-key removal unregisters by default. | Accepted | Avoids accidental private-key deletion. |
| SSH publication remotes poll by default and fast-forward only clean local state. | Adopted in PRD 0.3 | Keeps discovered managed work current without background merge, publication, or cleanup. |
| The CLI provides polling through a documented daemon mode. | Adopted in PRD 0.3 | Gives headless environments the same configured remote-update behavior without surprising one-shot invocations. |
