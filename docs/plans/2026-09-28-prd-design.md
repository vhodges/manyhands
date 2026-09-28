# Manyhands PRD Design

## Goal

Create one normative product requirements document at `docs/PRD.md`. It will
turn the project charter into a testable scope for a collaboration-complete
dogfooding release, while reserving implementation choices for RFCs.

## Product Boundary

The dogfooding release supports trusted Git collaborators working in local
repositories. They can manage Markdown-backed tickets and managed documents,
including threaded comments, through their full lifecycle:

- Enable, add, inspect, and remove repositories and their remotes.
- Discover, create, view, edit, and organize tickets and documents.
- Create and reply to threaded Markdown comments for either item type.
- Work in isolated branches and worktrees.
- Save durable checkpoints, synchronize through configured remotes, and close
  eligible tickets by merging and cleaning up their work.

The release does not introduce Manyhands accounts, roles, or permissions. It
relies on local filesystem access, Git repository access, and existing Git
credentials.

## Automation Requirement

Collaboration workflows are a strict product requirement, not an optional
convenience. Content lifecycle actions automatically initiate the Git and
worktree operations they need while keeping the user informed of intent,
progress, success, and recoverable errors.

- Creating a ticket or beginning an edit creates or reuses its isolated branch
  and worktree.
- Saving writes and validates content, automatically commits a checkpoint, and
  refreshes the local index.
- Creating or replying to a comment writes and automatically commits its
  Markdown representation.
- Sync is explicit: it fetches and reconciles as appropriate, then publishes
  when safe.
- Closing a ticket is explicit and confirmed: it performs the final save,
  commit, sync, merge, push, and cleanup sequence.

Automatic merge, push, and destructive cleanup never occur without a
deliberate user action and confirmation. Failures leave repository state
recoverable and show plain-language remediation.

## Information Model

Markdown files in enabled repositories are canonical. Git history is the
collaboration record. A rebuildable application-local SQLite index supports
lists, filters, polling, active-edit precedence, and status display but is
never authoritative over repository content.

Tickets require a title, type, status, and lifecycle metadata. Managed
documents require an explicit management marker and live in a repository
documentation root. Both can carry optional project and team metadata and
threaded comments. An RFC will decide exact file paths, front matter keys,
identifiers, branch conventions, primary-branch discovery, and comment layout.

When the primary branch and an active edit contain the same item, lists and
views prefer the active-edit version and label its editing context.

## Interfaces And Quality

The desktop application is the primary dogfooding interface. It supplies
repository navigation, content lists, tabbed items, metadata editing,
comments, lifecycle controls, progress feedback, and recovery guidance.

The headless CLI exposes compatible safe domain operations for agents and CI,
with machine-readable output and meaningful exit codes. It does not need to
duplicate desktop-only authoring ergonomics.

The PRD will require local-first behavior, Windows/macOS/Linux support, no
secret persistence, responsive UI during background work, idempotent or
recoverable lifecycle operations, keyboard accessibility, and reliable index
rebuild. Acceptance criteria will include offline editing followed by sync,
conflict recovery, comment-thread persistence, and successful ticket-close
cleanup journeys.

## Deferred Work And Risks

Boards, status rollups, templates, meta-repository propagation, multi-repo
planning views, and application-level authorization are explicitly deferred.

The PRD will record unresolved product and delivery risks: merge and conflict
policy, collaboration identity, remote-authentication UX, interrupted
worktree cleanup, external filesystem edits, editor selection, and scale
limits. The relevant RFCs must resolve these before implementation waves
depend on them.
