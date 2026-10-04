---
title: "MVP/Dogfood PRD"
date: 2026-10-04
status: approved
author: "Vince Hodges <vhodges@gmail.com> && Codex"
manyhands_managed: true
---

# Manyhands Product Requirements Document

**Status:** Approved MVP/Dogfooding release

**Version:** 0.4

**Owner:** The product owner maintains this document. Requirement IDs are stable once published; changes to intent or acceptance criteria require an updated version and changelog entry.

## Changelog

### 0.4 - 2026-10-04

- Replaced multiple editable-context selection with one shared deterministic
  context branch per item. Concurrent collaborators synchronize and recover
  conflicts on that shared branch.
- Distinguished non-mutating index refresh and rebuild from a remote poll's
  explicitly authorized fetch, fast-forward, and worktree materialization.

### 0.3 - 2026-09-29

- Approved the MVP architecture RFC and its document-promotion, local-only
  publication, remote-polling, shared-SSH-key, and CLI-daemon decisions.

### 0.2 - 2026-09-29

- Approved the initial collaboration-complete dogfooding requirements.

## Product Purpose

Manyhands is a local-first, Git-backed project management and documentation tool for enabled local repositories. It lets developers, agents, and non-developer collaborators work with Markdown-backed tickets and managed documents without requiring Git expertise. Git history is the collaboration record.

The product addresses the gap between Git-native work, which is effective for technical collaborators but inaccessible to many participants, and hosted project tools that separate project context from the repository where work occurs.

An enabled repository is a local Git repository that its user has opted into managing with Manyhands. A managed document is a Markdown document that Manyhands recognizes for management. An item is either a managed document or a ticket.

## Goals

- Make project documentation and tickets easy to create, discover, edit, discuss, and complete in a local repository.
- Preserve meaningful collaboration history in Git while insulating non-developer collaborators from Git workflows.
- Support trustworthy collaboration among existing Git collaborators without introducing a separate identity or authorization system.

## Non-Goals

- Boards, kanban views, scorecards, team rollups, or project rollups.
- Templates, meta-repository propagation, multi-repository planning, or application-level authorization.
- Replacing Git hosting, Git identity, or repository access controls.
- Automatically uploading SSH keys to Git forges; this is deferred roadmap work.

## Target Users

- Developers who work directly with local Git repositories and Markdown.
- Agents that create and update items in those repositories.
- Non-developer collaborators who need to participate in project documentation and ticket workflows without Git expertise.

## Scope

The dogfooding release supports enabled local repositories and trusted Git collaborators only. It includes repository onboarding; basic SSH key generation and management; managed-document and ticket creation, editing, and discovery; threaded comments on both item types; isolated branches and worktrees; an automatic checkpoint commit on save; deliberate synchronization and safe background remote polling; confirmed managed-document promotion; and explicit ticket closure that merges work and cleans up its isolated workspace.

### MH-PROD-001: Git-Backed Local Collaboration

**Priority:** Must

**Rationale:** Manyhands must keep the project source of truth and collaboration record in repositories its users already control.

**Acceptance Criteria:**

- A user can use Manyhands with a local Git repository they have opted into managing with Manyhands.
- Managed documents and tickets are Markdown-backed items in that repository.
- A successful user save results in an inspectable Git commit for the affected item.

### MH-PROD-002: Accessible Collaboration

**Priority:** Must

**Rationale:** Non-developer collaborators must be able to contribute without becoming Git operators.

**Acceptance Criteria:**

- A non-developer collaborator can create, discover, edit, and comment on managed documents and tickets through Manyhands without performing Git commands.
- A user can save an item change and Manyhands creates an automatic checkpoint commit without requiring the user to compose a commit.
- When a user deliberately requests synchronization directly or submits content through an RFC-defined synchronization-triggering action, Manyhands reports whether the current item work was successfully published to its configured remote, was already current, or needs user recovery.

### MH-SCOPE-001: Dogfooding Ticket Completion

**Priority:** Must

**Rationale:** The dogfooding release must validate a complete, reviewable ticket workflow rather than only artifact editing.

**Acceptance Criteria:**

- A user can make ticket changes in an isolated branch and worktree before closure.
- A user can explicitly close a ticket.
- A user must confirm ticket closure before Manyhands closes the ticket, merges its work, and cleans up the associated isolated workspace.

## Requirement Conventions

Normative product requirements use stable identifiers in the form `MH-<AREA>-<NUMBER>`, for example `MH-PROD-001`. Each normative requirement states its priority, rationale, and observable acceptance criteria.

This PRD defines product outcomes and user-observable behavior. RFCs own technical decisions, including repository opt-in configuration, managed-document recognition marker, schema, and path; Git command orchestration; merge strategy; synchronization protocol; data models; and interface implementation. For each content type, the synchronization RFC must identify which deliberate user actions, if any, trigger synchronization and the feedback shown before and after it. RFCs may refine how a requirement is met but must not change its product intent, priority, or acceptance criteria without a corresponding PRD revision.

## Repository Lifecycle

### MH-REPO-001: Add and Enable Existing Repositories

**Priority:** Must

**Rationale:** Users must be able to adopt repositories they already work in without moving or recreating them.

**Acceptance Criteria:**

- A user can add an accessible local Git repository and enable it for Manyhands.
- After enabling, the repository is available for Manyhands content discovery and management.
- During enablement, a user explicitly selects the repository's primary branch; Manyhands may suggest the remote default branch or current branch, but the selected branch must exist locally before it can be used for promotion or ticket closure.
- When the selected path does not exist, is not a Git repository, or cannot be accessed, Manyhands identifies the condition and provides an action the user can take to recover, such as selecting another path, restoring access, or initializing a repository.
- Repository opt-in uses an RFC-defined marker or configuration convention; its name, location, and format remain RFC-owned.

### MH-REPO-002: Create and Enable Local Repositories

**Priority:** Must

**Rationale:** A user must be able to start a managed project without first using separate Git tooling.

**Acceptance Criteria:**

- A user can create a new local Git repository at a valid, writable location and enable it for Manyhands in the same operation.
- After creation, the repository is available for Manyhands content discovery and management.
- As part of creation and enablement, a user confirms the repository's initial primary branch.
- If the location cannot be used, Manyhands explains why the repository was not created and provides a recovery action, such as choosing a writable or empty location.

### MH-REPO-003: Manage Enabled Repositories Offline

**Priority:** Must

**Rationale:** Local repositories remain useful when a remote is absent or a network is unavailable.

**Acceptance Criteria:**

- An enabled repository without configured remotes remains available for local content discovery and management.
- A user can remove an enabled repository from Manyhands.
- Removing a repository from Manyhands does not delete, move, or modify the local repository or its contents.

### MH-REPO-004: Inspect and Manage Remotes

**Priority:** Must

**Rationale:** Users need to inspect and adjust the remote configuration associated with an enabled repository.

**Acceptance Criteria:**

- A user can inspect the configured remotes for an enabled repository, including each remote's name and configured location.
- A user can add a named remote to an enabled repository and can subsequently inspect it.
- A user can remove a configured remote and it no longer appears in that repository's remote configuration.
- A user can select one configured SSH remote as the publication remote for Manyhands. Existing non-SSH remotes remain visible and manageable but cannot be selected for publication in the MVP.
- A repository with no remotes remains enabled and usable locally.

## SSH Credentials

### MH-CRED-001: Manage SSH Keys for Git Operations

**Priority:** Must

**Rationale:** Users need Git-over-SSH access without requiring separate key-generation or key-configuration tooling, while retaining the ability to use their existing SSH identities.

**Acceptance Criteria:**

- A user can generate an SSH key for use with Manyhands and may protect the generated private key with an optional passphrase.
- A user can add an existing SSH private key to Manyhands regardless of whether that key was generated by Manyhands or by another tool.
- A user can list configured SSH keys using non-secret identifying information, such as a label or public-key fingerprint, and can remove a key from Manyhands.
- Removing an externally supplied key from Manyhands does not delete its source key material. Removal and deletion behavior for a Manyhands-generated key remain RFC-owned and require explicit user confirmation before private key material is deleted.
- A user can select one configured SSH key as the shared Manyhands key for every Git-over-SSH operation.
- When the selected shared private key is protected by a passphrase, Manyhands prompts for that passphrase on its first use in an application session, including a first configured remote poll, and retains it only for the remainder of that session. It does not persist the passphrase.
- For every Git operation that authenticates over SSH, Manyhands uses the selected shared SSH key regardless of whether the key was generated by Manyhands or added from an existing source.
- If a configured key is missing, inaccessible, invalid, rejected by a remote, or cannot be unlocked, Manyhands identifies the affected key, repository, remote, and operation; preserves local work; and provides an actionable recovery path.
- Exact key algorithms, storage locations, key-generation mechanics, shared-key selection configuration, and secure deletion mechanics remain RFC-owned.

## Content

### MH-CONTENT-001: Discover Documentation Trees

**Priority:** Must

**Rationale:** Users must be able to find managed documentation in its repository organization.

**Acceptance Criteria:**

- A user can discover managed documents and folders in the documentation tree of an enabled repository.
- A user can view a discovered managed document.
- A user can create folders and organize managed documents within the documentation tree.
- Content that does not conform to managed-document requirements is shown as nonconforming rather than silently omitted, with guidance to correct it.

### MH-CONTENT-002: Create and Edit Managed Documents

**Priority:** Must

**Rationale:** Documentation must be maintainable without requiring users to work directly with repository files.

**Acceptance Criteria:**

- A user can create a managed document in a selected folder of the documentation tree.
- A user can edit and subsequently view a managed document.
- A user can move a managed document to a different folder in the documentation tree.
- Content validation identifies nonconforming managed-document content and explains what must be corrected; it does not silently hide the content.
- Exact document markers, schema, identifiers, and storage paths remain RFC-owned.

### MH-CONTENT-003: Create and Edit Tickets

**Priority:** Must

**Rationale:** Tickets need consistent core metadata while allowing project context when available.

**Acceptance Criteria:**

- A user can create, view, and edit a ticket.
- A ticket requires a title, type, and status before it is considered conforming.
- A ticket may include a description, project, and team.
- A ticket missing required metadata is displayed as nonconforming with guidance to supply the missing metadata; it is not silently hidden.
- Exact ticket schema, identifiers, and storage paths remain RFC-owned.

### MH-CONTENT-004: List Content Across Primary and Active Contexts

**Priority:** Must

**Rationale:** Users need a complete view of repository content while working in an active edit context.

**Acceptance Criteria:**

- Document and ticket lists include items discovered on primary branches and in active worktrees.
- Each displayed item shows its title, type and status when relevant, and the timestamp of its most recent content or metadata change.
- When an item has an active editing context, that context is labeled with its branch and worktree clearly enough to distinguish it from the primary copy.
- When exactly one active worktree contains an editable copy of the same item as a primary-branch copy, the list and item view display that active edit instead of the primary-branch copy.
- Each local clone recognizes at most one editable shared context branch for an
  item. Duplicate, mismatched, malformed, or otherwise exceptional local or
  remote context resources remain visible with recovery guidance and do not
  become a user context-selection result.
- Nonconforming discovered content remains visible in lists with its nonconforming state and recovery guidance.
- Primary-branch suggestion and same-item identification remain RFC-owned; primary-branch selection occurs during repository enablement.

## Comments

### MH-COMMENT-001: Create Comments and Replies

**Priority:** Must

**Rationale:** Discussions must remain attached to the documentation or ticket they concern.

**Acceptance Criteria:**

- A user can create a Markdown comment on a managed document or ticket.
- A user can reply to an existing comment on either item type.
- After reopening the item, the new comment or reply is available in its thread.
- Exact comment file layout and front matter remain RFC-owned.

### MH-COMMENT-002: Preserve Thread Relationships

**Priority:** Must

**Rationale:** A discussion is only useful when its parent and reply relationships remain intelligible over time.

**Acceptance Criteria:**

- Root comments display in ascending creation order, and each comment's direct replies display in ascending creation order beneath their parent.
- After rebuilding the index, switching worktrees, and reopening the item, root comments and each comment's direct replies retain their prescribed ascending creation order, and each reply remains visibly associated with its parent.

## Automated Collaboration Lifecycle

### MH-COLLAB-001: Automatically Orchestrate Isolated Editing Contexts

**Priority:** Must

**Rationale:** Collaboration lifecycle tooling must follow content work automatically so users can work with repository-backed items without operating Git directly.

**Acceptance Criteria:**

- When creating an item or starting an edit with no existing editable context, Manyhands automatically creates an isolated editing context comprising an item-specific Git branch and worktree, without asking the user to execute Git commands.
- When exactly one editable context exists for an item, Manyhands automatically reuses it without asking the user to execute Git commands.
- Each local clone reuses the one recognized shared context for an item. A
  duplicate, mismatched, malformed, or otherwise exceptional context remains a
  visible recovery state; Manyhands does not create a second context or ask the
  user to choose one.
- Exact context naming and provisioning mechanics remain RFC-owned.
- Manyhands reports the context setup progress and its completed or failed outcome to the user.
- Context setup for an item does not silently mutate another item's content, metadata, or editing context.
- If context setup fails, Manyhands identifies the affected item, repository, and context, preserves recoverable user work, and offers an action to retry or recover.

### MH-COLLAB-002: Checkpoint Changed Saves

**Priority:** Must

**Rationale:** A save must durably record valid content changes and keep discovery current without making users perform collaboration bookkeeping.

**Acceptance Criteria:**

- After content and metadata validation succeeds, saving a changed item writes its Markdown, automatically creates an inspectable checkpoint commit, updates the index, and reports the checkpoint outcome.
- A save with no content or metadata changes reports that no changes were made and does not create an empty checkpoint commit.
- If validation, Markdown writing, automatic checkpoint commit, or index refresh fails, the user's content remains recoverable; Manyhands clearly reports which save steps completed and which did not, and offers actionable remediation, such as correcting invalid content, restoring access, or retrying the incomplete step.
- A save does not automatically publish, merge, or clean up collaboration state.

### MH-COLLAB-003: Checkpoint Comments and Replies

**Priority:** Must

**Rationale:** Discussion is part of an item's durable collaboration record and must not depend on a separate manual commit step.

**Acceptance Criteria:**

- Creating a comment or reply invokes the applicable item-context provisioning
  or reuse lifecycle defined by MH-COLLAB-001 before Manyhands writes its
  Markdown representation and automatically creates an inspectable Git commit
  for that comment event, without requiring the user to create a Git commit.
- After successfully checkpointing a newly created comment or reply, Manyhands immediately synchronizes the comment's editing context as part of the same deliberate submit action under MH-COLLAB-004 when a publication remote is configured. Without a publication remote, Manyhands reports the comment or reply as saved locally with publication pending.
- Exact comment storage layout and checkpoint commit mechanics remain RFC-owned.
- Manyhands reports the checkpoint outcome and makes the comment or reply available when the item is reopened.
- If checkpoint creation fails, Manyhands identifies the affected item, repository, and context; preserves the submitted discussion for recovery; and offers a retry or recovery action.

### MH-COLLAB-004: Synchronize Deliberately and Poll Safely

**Priority:** Must

**Rationale:** Publishing and history-changing integration require deliberate user consent, while configured polling keeps safe local state current and discovers collaborator-created item contexts.

**Acceptance Criteria:**

- A user can deliberately initiate synchronization for an item or its editing context; Manyhands obtains relevant remote changes and, when safe, integrates those changes into the current item context and publishes current item work.
- A user can deliberately initiate primary-branch synchronization after a publication remote is configured; Manyhands obtains relevant remote changes, safely integrates them into the configured primary branch, and publishes the resulting primary state.
- The synchronization RFC identifies the deliberate actions that trigger synchronization for each content type. A content action is not synchronization-triggering unless the RFC declares it as such and the interface communicates that outcome to the user.
- Submitting a new comment or reply is a synchronization-triggering action: after its local checkpoint succeeds, Manyhands immediately attempts synchronization when a publication remote is configured and reports the combined outcome. When no publication remote is configured, or when synchronization cannot complete, it reports the saved local comment or reply as publication pending with actionable recovery.
- Manyhands reports whether synchronization published local work, found it already current, saved local work with publication pending, or requires recovery.
- Authentication, remote availability, and conflict errors identify the affected item, repository, and context, provide actionable remediation, and do not discard local work.
- For a repository with an SSH publication remote, Manyhands enables background remote polling by default at desktop application launch and CLI daemon startup. A user can pause polling or configure its interval for that repository.
- Background polling fetches remote state, including new remote branches, refreshes discovery, and fast-forwards the configured primary branch or an existing item context only when the worktree is clean and its local branch is strictly behind the corresponding remote-tracking branch.
- When background polling discovers a new recognized and conforming Manyhands item-context branch, Manyhands automatically creates its local tracking branch and worktree, then indexes the managed item folder and comments it contains.
- Background polling preserves dirty, divergent, conflicted, deleted, renamed, malformed, inaccessible, or unrecognized contexts; it identifies the state and provides recovery guidance without removing a local worktree.
- Background polling is serialized with manual synchronization, primary-branch synchronization, managed-document promotion, and ticket closure for the same repository. A manual lifecycle operation takes precedence and polling waits or stops at a safe point before that operation changes Git state.
- Background polling never pushes, commits, merges, rebases, stashes, overwrites, discards, deletes a branch, or deletes a worktree. Other remote Git operations occur only in response to a direct synchronization request, an RFC-defined synchronization-triggering user action, an explicitly confirmed managed-document promotion, or an explicitly confirmed ticket-close action.

### MH-COLLAB-005: Close Tickets Through a Confirmed Lifecycle

**Priority:** Must

**Rationale:** Closing a ticket must make its final state visible in the primary copy while preventing accidental integration or premature loss of recovery state.

**Acceptance Criteria:**

- Before closing a ticket, Manyhands requires explicit user confirmation that the close lifecycle will proceed.
- Once confirmed, Manyhands performs a final save and checkpoint when needed, synchronizes when a publication remote is configured, integrates the ticket into the configured primary branch, publishes the integrated result when a publication remote is configured, and removes the ticket's local and remote branch and worktree state. Without a publication remote, Manyhands completes the local integration and cleanup, reports the primary result as publication pending, and later allows primary-branch synchronization after a publication remote is configured.
- After successful closure, the ticket's closed state is visible in the primary copy.
- If any close prerequisite fails, Manyhands reports the affected item, repository, and context; preserves sufficient state to retry or recover without data loss; and does not perform premature cleanup.

### MH-COLLAB-006: Preserve Consent and Resilience Throughout Lifecycle Work

**Priority:** Must

**Rationale:** Automated local lifecycle work and configured remote updates must remain understandable, interruptible when safe, and recoverable without allowing background work to publish, merge, or destructively change user state.

**Acceptance Criteria:**

- Only a direct synchronization request, an RFC-defined synchronization-triggering user action, an explicitly confirmed managed-document promotion, or an explicitly confirmed ticket-close action may publish, merge, or perform branch and worktree cleanup. Synchronization alone never performs destructive branch or worktree cleanup.
- Configured background polling may fetch remote state, fast-forward clean local primary or item contexts, and provision a recognized new item worktree. It never publishes, creates checkpoints, merges, rebases, stashes, overwrites, discards, or performs cleanup.
- Integration of a ticket into the configured primary branch and removal of its local or remote branch and worktree state occur only as part of an explicitly confirmed ticket-close action after its prerequisites succeed. Integration and cleanup of a managed document occur only as part of an explicitly confirmed managed-document promotion after its prerequisites succeed.
- Long-running lifecycle work reports progress and outcome while keeping the interface responsive.
- A user may cancel long-running lifecycle work only at a safe point; Manyhands reports whether cancellation completed or work must continue to a safe recovery state.
- Lifecycle failures name the affected item, repository, and context and offer next actions to retry, recover, or inspect the problem.

### MH-COLLAB-007: Promote Managed Documents Through a Confirmed Lifecycle

**Priority:** Must

**Rationale:** A managed document edited in an isolated context needs an explicit, safe path to become the primary copy without treating the document as a closed ticket.

**Acceptance Criteria:**

- A user can deliberately request promotion of a managed document's editing context.
- Before promotion begins, Manyhands identifies the item, primary branch, publication remote when configured, merge, publication, and cleanup effects, then requires explicit user confirmation.
- After confirmation, Manyhands performs a final save and checkpoint when needed, synchronizes the editing context when a publication remote is configured, integrates it into the configured primary branch, publishes the integrated primary result when a publication remote is configured, and then removes the document's local and remote branch and worktree state.
- Without a publication remote, Manyhands completes the local integration and cleanup, reports the primary result as publication pending, and later allows primary-branch synchronization after a publication remote is configured.
- After successful promotion, the promoted document is visible in the primary copy. A later edit creates a new isolated context.
- If a promotion prerequisite fails, Manyhands identifies the item, repository, and context; preserves sufficient state to retry or recover; and performs no premature cleanup.

## Indexing

### MH-INDEX-001: Maintain a Rebuildable Discovery Index

**Priority:** Must

**Rationale:** Discovery must be efficient without creating a second source of truth for repository content.

**Acceptance Criteria:**

- Markdown and Git content remain canonical; the application-local SQLite index is a rebuildable discovery cache and is not authoritative for item content or metadata.
- The index captures discovered ticket and managed-document metadata and their editing context from primary branches and active worktrees.
- A user can rebuild the index after it is lost or corrupted, and rebuilding does not destroy or rewrite canonical Markdown or Git content.

### MH-INDEX-002: Refresh Discovery After Changes

**Priority:** Must

**Rationale:** Users need discovery results to reflect repository and content changes without treating stale cache data as canonical.

**Acceptance Criteria:**

- The index updates after repository lifecycle events that add, enable, remove, or change remotes for a repository.
- The index updates after managed-document, ticket, and comment changes that affect discovery metadata or editing context.
- A user can manually request a refresh and the resulting discovery data reflects the currently accessible canonical content.
- For a repository with an SSH publication remote, background polling is enabled by default at desktop application launch and CLI daemon startup; a user can pause polling or configure its interval for that repository.
- A configured poll fetches remote state, updates permitted local Git state,
  fast-forwards only clean primary and item contexts, and materializes a newly
  discovered recognized item context as a worktree before it invokes an
  index-only refresh of the managed item folder and comments.
- Polling leaves dirty, divergent, conflicted, deleted, renamed, malformed, inaccessible, or unrecognized contexts unchanged locally and surfaces them with recovery guidance.

### MH-INDEX-003: Surface External and Malformed Changes

**Priority:** Must

**Rationale:** Repository changes outside Manyhands and malformed content must be recoverable rather than invisible.

**Acceptance Criteria:**

- Following a refresh or rebuild, externally changed canonical content is reflected in discovery results.
- Malformed or inaccessible canonical content is visible as a problem with guidance to recover, rather than silently excluded or replaced.
- Recovering from index loss, corruption, external changes, or malformed content does not rewrite canonical content unless the user explicitly edits that content.

## Desktop And CLI

### MH-UX-001: Provide a Desktop Dogfooding Interface

**Priority:** Must

**Rationale:** The desktop application is the primary interface for dogfooding the end-to-end collaboration workflow.

**Acceptance Criteria:**

- A desktop user can navigate enabled repositories and choose ticket and managed-document lists.
- A desktop user can browse item folders and lists, open more than one item, and distinguish the open items and their editing contexts.
- A desktop user can manage the shared SSH key; view and edit item content and metadata; create, view, and reply to comments; and invoke the save, synchronization, managed-document promotion, and ticket-close lifecycle actions permitted by the selected item.
- While work covered by `MH-COLLAB-001` through `MH-COLLAB-007`, including background polling, is in progress or needs recovery, the desktop interface provides progress, outcome, polling pause and configuration controls, and actionable recovery feedback.
- The interface does not require a specific dock layout, component library, or editor technology.

### MH-CLI-001: Provide a Headless Domain Operations Interface

**Priority:** Must

**Rationale:** Agents and CI require safe access to equivalent domain operations without a graphical environment.

**Acceptance Criteria:**

- The CLI supports the safe repository inspection and management, SSH key management, discovery and index refresh, content and comment lifecycle, and applicable save, synchronization, managed-document promotion, and ticket-close operations covered by the PRD.
- The CLI documents a daemon mode that performs configured background remote polling. One-shot CLI operations do not create an implicit resident poller, and the CLI supports explicit one-shot polling plus polling configuration and status operations.
- Each CLI operation produces a human-readable result that identifies success, no-op, failure, or required recovery.
- The CLI documents a machine-readable output mode and produces its documented format when that mode is requested.
- An unsuccessful operation exits with a meaningful nonzero status, and a successful or documented no-op operation exits with status zero.
- The CLI runs without a display and does not require desktop dependencies.

## Non-Functional Requirements

### MH-NFR-001: Preserve Local-First Authoring

**Priority:** Must

**Rationale:** Users must be able to author safely when remotes or networks are unavailable.

**Acceptance Criteria:**

- A user can create or edit a conforming item and save its local checkpoint while offline.
- When connectivity later becomes available, the user can explicitly synchronize the saved work under `MH-COLLAB-004`.

### MH-NFR-002: Support Target Desktop Platforms

**Priority:** Must

**Rationale:** Dogfooding collaborators require supported desktop access across their common operating systems.

**Acceptance Criteria:**

- The desktop application is supported on Windows, macOS, and Linux.
- On Linux, the supported desktop application runs in a Wayland session.

### MH-NFR-003: Protect Credentials, SSH Keys, and Secrets

**Priority:** Must

**Rationale:** Collaboration tooling must not create additional secret exposure through stored state or diagnostics.

**Acceptance Criteria:**

- Manyhands does not persist passphrases, credentials, or private-key contents in its application data, indexes, managed Markdown, or Git metadata that it writes.
- A private SSH key generated by Manyhands is stored only in an RFC-defined SSH key location protected by operating-system-appropriate owner-only access controls; its passphrase is never stored.
- Manyhands does not include credentials or secrets in logs, progress messages, error reports, or machine-readable CLI output.

### MH-NFR-004: Keep Desktop Work Responsive

**Priority:** Must

**Rationale:** File, Git, and network operations must not prevent users from understanding or continuing safe desktop work.

**Acceptance Criteria:**

- File, Git, and network operations that can take noticeable time run without blocking desktop interaction.
- During such operations, the desktop interface remains responsive and presents the progress and recovery feedback required by `MH-UX-001` and `MH-COLLAB-006`.

### MH-NFR-005: Support Keyboard Core Workflows

**Priority:** Must

**Rationale:** Core collaboration work must remain available to keyboard users.

**Acceptance Criteria:**

- A keyboard-only desktop user can manage the shared SSH key, navigate repositories and item lists, open and edit an item, save it, create and reply to a comment, invoke synchronization, pause or configure polling, and confirm or cancel managed-document promotion and ticket closure.
- Keyboard focus is visible for each interactive control in these workflows.

### MH-NFR-006: Make Lifecycle Actions Recoverable and Idempotent

**Priority:** Must

**Rationale:** Retried collaboration actions must not silently duplicate, discard, or corrupt work.

**Acceptance Criteria:**

- Retrying a lifecycle action after an interruption or reported failure either completes the remaining work or reports the recovery step still required.
- Retrying a completed save, polling update, synchronization, managed-document promotion, or ticket-close action does not create duplicate canonical content, comments, checkpoint commits, merges, worktrees, or cleanup effects.

### MH-NFR-007: Preserve Canonical Content During Indexing

**Priority:** Must

**Rationale:** Rebuilding a cache must never become a hidden content migration or Git mutation.

**Acceptance Criteria:**

- An index-only refresh or rebuild does not modify canonical Markdown content,
  Git commits, branches, worktrees, remotes, or repository configuration.
- A remote poll is distinct from an index-only refresh. It may perform only the
  fetch, clean fast-forward, and recognized-context materialization explicitly
  permitted by `MH-COLLAB-004` before it invokes the non-mutating index refresh.
- The behavior remains consistent with `MH-INDEX-001` and `MH-INDEX-003` when canonical content is malformed or inaccessible.

### MH-NFR-008: Prevent Silent Data Loss

**Priority:** Must

**Rationale:** Users must be able to detect and recover from incomplete or conflicting work rather than lose it without notice.

**Acceptance Criteria:**

- If a write, checkpoint, synchronization, merge, cleanup, or index operation cannot complete, Manyhands preserves recoverable local work and identifies the incomplete operation.
- Manyhands does not overwrite, discard, or silently replace user-authored content or comments as part of automatic recovery.

## End-to-End Acceptance Journeys

### Offline Edit, Checkpoint, and Later Sync

1. With network access unavailable, a user opens an enabled repository, creates or opens a ticket or managed document, changes its content or metadata, and saves it.
2. The user observes a successful local automatic checkpoint commit and refreshed local discovery state, with no attempted implicit publication.
3. After network access is restored, the user explicitly synchronizes the same editing context and observes that its checkpointed work is published or receives actionable recovery feedback while the local work remains available.

This journey demonstrates `MH-COLLAB-002`, `MH-COLLAB-004`, `MH-NFR-001`, and `MH-NFR-008`.

### Concurrent Collaboration Conflict Recovery

1. A user has checkpointed local work for an item while a trusted collaborator publishes conflicting changes to that item.
2. The user explicitly synchronizes and Manyhands identifies the item, repository, and editing context requiring recovery without discarding the user's local work.
3. The user follows the provided recovery guidance, resolves the conflict through the supported workflow, and can retry synchronization to obtain a completed or clearly still-recoverable result.

This journey demonstrates `MH-COLLAB-004`, `MH-COLLAB-006`, `MH-NFR-006`, and `MH-NFR-008`.

### Background Remote Update and Discovery

1. A trusted collaborator publishes a primary-branch update and a new recognized, conforming item context to the configured SSH publication remote.
2. A desktop application or CLI daemon polls the remote and, without user intervention, fast-forwards clean local primary and item contexts, creates exactly one worktree for the newly discovered item context, and indexes its managed item folder and comments.
3. In a separate acceptance run, a dirty, divergent, malformed, or remotely deleted context remains locally preserved, is marked with recovery guidance, and no background push, checkpoint, merge, rebase, or cleanup occurs.

This journey demonstrates `MH-COLLAB-004`, `MH-COLLAB-006`, `MH-INDEX-002`, `MH-INDEX-003`, `MH-NFR-006`, and `MH-NFR-008`.

### Durable Threaded Discussion

1. A user creates root comments and replies on an item, including more than one reply to the same root comment, and observes each local checkpoint and synchronization outcome.
2. The repository index is rebuilt, the user changes worktree, and the item is reopened.
3. The user observes every root comment in ascending creation order, every direct reply in ascending creation order beneath its parent, and the original parent-reply relationships.

This journey demonstrates `MH-COMMENT-001`, `MH-COMMENT-002`, `MH-COLLAB-003`, `MH-COLLAB-004`, and `MH-INDEX-001`.

### Confirmed Managed-Document Promotion

1. A user opens a managed document in its isolated editing context, saves a checkpoint, and requests `Approve and merge`; before work begins, Manyhands presents the primary branch, publication remote when configured, merge, publication, and cleanup effects for explicit confirmation.
2. After confirmation, Manyhands completes the final save and checkpoint as needed, synchronizes when a publication remote is configured, integrates the document into primary, publishes the integrated result when possible, and then removes the document branch and worktree.
3. In a local-only acceptance run, promotion completes its local integration and cleanup with publication pending; after a publication remote is configured, the user explicitly synchronizes the primary branch to publish it. Reopening the document starts a fresh editing context.

This journey demonstrates `MH-COLLAB-004`, `MH-COLLAB-006`, `MH-COLLAB-007`, `MH-NFR-006`, and `MH-NFR-008`.

### Confirmed Close with Retryable Failure

1. A user opens a ticket with an isolated editing context and requests closure; before any close work begins, Manyhands presents the confirmation required by `MH-COLLAB-005`.
2. After the user confirms and all prerequisites succeed, the user observes final save and checkpoint activity as needed, synchronization, integration and publication to the configured primary branch when a publication remote exists, the closed primary copy, and only then branch and worktree cleanup. In a local-only run, closure reports the integrated primary result as publication pending and a later primary-branch synchronization publishes it after a remote is configured.
3. In a separate acceptance run, an injected failure at a close prerequisite leaves the ticket work and recovery state intact, identifies the failed step, performs no premature cleanup, and allows a retry after the failure is corrected.

This journey demonstrates `MH-COLLAB-005`, `MH-COLLAB-006`, `MH-NFR-006`, and `MH-NFR-008`.

## Success Evaluation

Dogfooding success is demonstrated by completing the end-to-end acceptance journeys with real repositories and trusted collaborators. Fabricated UI states or demo-only evidence do not establish success.

## Risks And Open Decisions

- **Content schema, identifiers, paths, and managed marker:** Incompatible or unstable representations could make content undiscoverable or strand existing work. Accountable role: Technical Lead. Mitigation and decision record: canonical content schema RFC must define migration and compatibility decisions.
- **Primary-branch discovery:** Incorrect primary-branch selection can show or integrate the wrong repository state. Accountable role: Technical Lead. Mitigation and decision record: Git workflow RFC must define discovery, ambiguity handling, and recovery.
- **Git merge, polling, and conflict policy:** Unsafe automation can lose work, create unreviewable history, or race a manual lifecycle operation. Accountable role: Technical Lead. Mitigation and decision record: Git workflow and conflict/recovery RFC must define merge, polling serialization, and user recovery behavior.
- **Git identity, SSH key, and remote authentication UX:** Missing identity, an unavailable or rejected configured key, or failed authentication can block checkpointing or publication and expose secrets if handled carelessly. Accountable role: Security Lead. Mitigation and decision record: authentication and credential-handling RFC must define key generation, import, selection, passphrase prompts and session retention, protected storage, removal, delegation, and redaction.
- **External filesystem edits:** Out-of-band moves, deletes, or malformed edits can invalidate discovery and editing assumptions. Accountable role: Technical Lead. Mitigation and decision record: repository/index persistence and refresh RFC must define detection and recovery behavior.
- **Interrupted worktree or cleanup:** Process interruption can leave branches, worktrees, or partial close state behind. Accountable role: Technical Lead. Mitigation and decision record: Git workflow and conflict/recovery RFC must define reconciliation and retry behavior.
- **Index consistency, polling, and scale:** Refreshes can become stale, expensive, or misleading across large repositories and many worktrees; automatic worktree materialization can amplify those effects. Accountable role: Technical Lead. Mitigation and decision record: repository/index persistence and refresh RFC must define consistency, polling, worktree materialization, and scale strategy.
- **Editor choice and Markdown fidelity:** Editing can unintentionally alter user-authored Markdown or fail to represent its semantics. Accountable role: Technical Lead. Mitigation and decision record: desktop information architecture and editor RFC must define fidelity expectations and unsupported-content behavior.
- **Trusted collaborator security boundary:** Git access alone may not express all desired collaboration restrictions. Accountable role: Security Lead. Mitigation and decision record: document and communicate the trusted Git collaborator limitation for dogfooding; future governance and access-model decisions are deferred roadmap work rather than a dogfooding release dependency.
- **Cross-platform filesystem, credentials, and Git differences:** Paths, case behavior, credential helpers, and Git installations vary by operating system. Accountable role: Technical Lead. Mitigation and decision record: test and compatibility strategy RFC must define supported environments and coverage.

## RFC Dependencies

The following RFCs must resolve product-level dependencies before their corresponding implementation waves. They specify how this PRD is met and do not decide, weaken, or replace its product requirements.

- **Canonical content and comment schema RFC:** Defines managed markers, identifiers, paths, front matter, comment representation, recognized item-context validation, and compatibility.
- **Git workflow and conflict/recovery RFC:** Defines branch and worktree lifecycle, primary-branch discovery, merge policy, synchronization orchestration, remote-ref protocol, polling serialization, fast-forward preconditions, and interruption recovery.
- **Repository/index persistence and refresh RFC:** Defines application-local state, rebuild and refresh mechanisms, external-change detection, polling schedule and backoff, idempotent worktree materialization, and scale behavior.
- **Authentication and credential-handling RFC:** Defines Git identity; shared SSH key generation, import, protected storage, selection, removal, and deletion; passphrase prompting and session-only retention; startup polling unlock; remote authentication interaction; credential delegation; and secret redaction. Automated SSH key upload to Git forges is explicitly deferred.
- **Desktop information architecture and editor RFC:** Defines navigation, multi-item interaction, polling feedback and controls, accessibility implementation, and Markdown editing fidelity.
- **CLI contract RFC:** Defines commands, safe operation boundaries, daemon-mode polling, machine-readable output format, exit-status taxonomy, and automation behavior.
- **Test and compatibility strategy RFC:** Defines journey testing, polling and lifecycle failure injection, supported platform coverage, Git/environment matrix, and compatibility expectations.

## Roadmap

### Collaboration-Complete Dogfooding Release

The first milestone is a collaboration-complete dogfooding release: trusted collaborators use real enabled repositories to create, discover, edit, comment on, checkpoint, poll, synchronize, and recover tickets and managed documents through the described journeys; tickets use the close, merge, and cleanup lifecycle, while managed documents use the confirmed promotion, merge, and cleanup lifecycle.

### Planning and Rollups

Follow-on work adds planning and board views, scorecards, team rollups, and project rollups.

### Templates and Scaffolding

Follow-on work adds templates and scaffolding for repository content and workflows.

### Git Forge Key Provisioning

Follow-on work adds optional automation for uploading a user's SSH public key to supported Git forges. This work must not require users to upload app-generated keys or replace existing configured SSH keys.

### Meta-Repository and Multi-Repository Management

Follow-on work adds meta-repository propagation and multi-repository management.

### Expanded Governance and Access Models

Follow-on work adds governance and access models beyond the trusted-collaborator boundary.
