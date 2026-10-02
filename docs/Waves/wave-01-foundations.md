---
title: "Wave 01: Foundations"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YQ7F0J3M6P8R0T2V4W6X8Y"
---

# Wave 01: Foundations

## Outcome

Wave 01 establishes the local, testable foundation required for Manyhands to
manage repository-backed content without remote collaboration. At its exit, a
real local Git repository can be enabled, initialize its tracked Manyhands
configuration, create and checkpoint documents, tickets, and comments in
isolated worktrees, and rebuild discovery from canonical Markdown and Git state.

The Wave ends with a recoverable, rebuildable local foundation. It does not
claim that remote synchronization, SSH credentials, background polling, desktop
workflows, or the full CLI contract are implemented.

## Authority and Traceability

This Wave implements the local foundation described by the approved
[MVP architecture RFC](../RFC/mvp-rfc.md) and PRD version 0.3. Its governing
technical RFCs are:

- [Canonical content and comment schema](../RFC/canonical-content-and-comment-schema.md)
- [Git workflow and conflict recovery](../RFC/git-workflow-and-conflict-recovery.md)
- [Repository index persistence and refresh](../RFC/repository-index-persistence-and-refresh.md)
- [Test and compatibility strategy](../RFC/test-and-compatibility-strategy.md)

Primary PRD traceability:

| Area | Requirements addressed in this Wave |
| --- | --- |
| Repository lifecycle | `MH-REPO-001` to `MH-REPO-004`, limited to local configuration and remote management without network contact |
| Content and comments | `MH-CONTENT-001` to `MH-CONTENT-003`, the single-local-context foundation of `MH-CONTENT-004`, and `MH-COMMENT-001` to `MH-COMMENT-002` |
| Local collaboration | `MH-COLLAB-001`, `MH-COLLAB-002`, and the local checkpoint portion of `MH-COLLAB-003` |
| Indexing | `MH-INDEX-001` to `MH-INDEX-003`, excluding remote polling |
| Quality | `MH-NFR-001`, `MH-NFR-006`, `MH-NFR-007`, and `MH-NFR-008` |

## Entry Gate

Wave implementation may begin only when all conditions hold:

- PRD version 0.3 remains approved.
- The four governing Wave 1 RFCs remain approved and mutually consistent.
- The implementation starts from a clean `main` branch with the existing
  Devenv/Cargo and CI baseline available.
- Cycle documents exist before their implementation begins and identify their
  exact implementation and test changes.

## Scope

Wave 01 includes:

- Version 1 `.manyhands/config.toml`, canonical ULID identities, and YAML
  front-matter validation for documents, tickets, and comments.
- Primary-branch selection, local repository enablement, initialization commits,
  local Git identity resolution, and local remote inspection/add/remove/SSH
  publication-remote selection without any remote contact.
- Deterministic item context branches and repo-local worktrees with scoped local
  checkpoints that preserve unrelated changes.
- One application-local SQLite database for repository registration, discovery,
  visible problems, and operation recovery.
- Full scans of primary and active local worktree contexts, refresh, remove, and
  rebuild behavior.
- Real-repository integration tests, failure injection, retry evidence, and the
  Wave 1 local acceptance gate.

## Explicit Exclusions

Wave 01 does not implement:

- SSH key generation, import, selection, passphrase prompts, or any remote Git
  authentication.
- Fetch, push, remote polling, CLI daemon behavior, or remote context
  materialization.
- Merge conflict resolution, document promotion, ticket closure, remote branch
  cleanup, or primary-branch publication.
- Desktop navigation, Markdown editor behavior, accessibility implementation, or
  the complete CLI command and JSON-output contract.
- Automatic migration of marker-only documents; they are discovered as
  nonconforming and require explicit repair.

## Required Foundation Contracts

- Git operations use `git2`/libgit2. A system Git executable is not a runtime
  dependency.
- Tracked repository configuration and Markdown are canonical. SQLite is a
  rebuildable cache and operation-recovery aid, never a second content source.
- Enabling writes `.manyhands/config.toml`, adds `.manyhands/worktrees/` to the
  local `.git/info/exclude`, and creates exactly one `Initialize Manyhands`
  commit after the primary worktree is validated clean.
- Item contexts use `manyhands/<kind>/<ULID>` branches and
  `.manyhands/worktrees/<ULID>/` paths.
- A primary-context scan never recursively treats the worktree base as canonical
  content. Each active worktree is scanned independently as an editing context.
- No Wave 01 action publishes, merges, rebases, stashes, discards, or deletes
  user work.

## Ordered Cycles

Individual Cycle documents MUST be created at the listed paths before each Cycle
begins. The summaries below define Cycle scope and exit evidence; they are not
file-level implementation plans.

### Cycle 01: Canonical Foundation

**Planned document:** `docs/Cycles/wave-01-cycle-01-canonical-foundation.md`

**Purpose:** Establish the schema library and disposable real-repository fixture
base that every later Cycle consumes.

**Prerequisites:** Wave entry gate.

**In scope:**

- Version 1 TOML configuration parsing and validation.
- Uppercase ULID validation and generation.
- YAML front-matter parsing, unknown-key preservation, canonical path checks,
  ticket/document/comment validation, and threaded-comment ordering.
- Visible nonconforming results for marker-only, malformed, duplicate-ID, and
  invalid parent/item relationships.
- Reusable disposable Git repository and canonical-content fixtures.

**Out of scope:** Repository registration, Git commits, worktree creation,
SQLite persistence, and any remote operation.

**Exit evidence:** Unit and fixture tests prove valid content round-trips without
body or unknown-key loss; invalid content is visible without a rewrite; comment
ordering is deterministic.

### Cycle 02: Repository Enablement

**Planned document:** `docs/Cycles/wave-01-cycle-02-repository-enablement.md`

**Purpose:** Make a local Git repository an enabled Manyhands repository safely
and durably.

**Prerequisites:** Cycle 01 schema and fixtures.

**In scope:**

- Add, create, enable, remove, and inspect local repositories.
- Explicit primary-branch confirmation and validation.
- Local inspection, addition, removal, and configuration of an SSH publication
  remote without fetching or publishing.
- Repository-local commit-identity resolution.
- `.git/info/exclude` management, configuration initialization, and the
  `Initialize Manyhands` commit for existing and unborn repositories.
- Application-local repository registration and index invalidation.

**Out of scope:** SSH credentials, remote contact, item editing contexts, and
content indexing beyond enablement invalidation.

**Recovery condition:** Dirty, conflicted, inaccessible, non-Git, or unwritable
repositories leave user files and Git state unchanged, identify the failed
step, and can be retried after correction.

**Exit evidence:** Real-repository tests cover existing and unborn initialization
plus each rejected repository state. Removing a repository changes only local
registration. No enablement action auto-publishes.

### Cycle 03: Isolated Local Authoring

**Planned document:** `docs/Cycles/wave-01-cycle-03-isolated-local-authoring.md`

**Purpose:** Create and edit canonical items in deterministic local contexts
without exposing Git operations to the user.

**Prerequisites:** Cycles 01 and 02.

**In scope:**

- Create and reuse one deterministic local branch and worktree per item.
- Create, move, validate, and edit documents; create and edit tickets; create
  root comments and replies.
- Scoped checkpoints for documents, tickets, and comments, with no empty
  commits and no staging or committing of unrelated paths.
- Local-only comment submission reported as publish pending when no publication
  remote is configured.
- Typed recoverable outcomes for failed writes, branch/worktree creation,
  commits, and post-commit refresh invalidation. Durable operation records are
  introduced in Cycle 04 and exercised across the lifecycle in Cycle 05.

**Out of scope:** Remote synchronization, merge/rebase, promotion, closure, and
automatic worktree cleanup.

**Recovery condition:** A failure preserves the draft, commit, branch, and
worktree state that already exists; retry performs only the incomplete step.

**Exit evidence:** Real-repository tests prove deterministic context paths,
single-context reuse, scoped commits, no-op saves, and preservation of
unrelated changes. A mismatched or duplicate expected context is visible as a
recoverable condition; multiple-context choice is deferred to Wave 2.

### Cycle 04: Discovery and Rebuild

**Planned document:** `docs/Cycles/wave-01-cycle-04-discovery-and-rebuild.md`

**Purpose:** Make canonical primary and active-context content discoverable from
a rebuildable application-local SQLite index.

**Prerequisites:** Cycles 01 through 03.

**In scope:**

- SQLite registration, context, item, problem, and operation-recovery records.
- Full per-context scans of primary and active worktrees.
- Item metadata and activity-time extraction, nonconforming problem records,
  and deterministic comment-thread reconstruction.
- Active-context presentation for the one deterministic local context, manual
  refresh, local repository removal, and destructive-cache rebuild recovery.

**Out of scope:** Filesystem watching, remote polling, remote context discovery,
and any index action that mutates canonical state.

**Recovery condition:** Lost or corrupt SQLite state is replaced by a fresh scan
while canonical Markdown, Git commits, branches, worktrees, remotes, and
configuration remain unchanged.

**Exit evidence:** Tests prove primary-plus-context discovery, visible malformed
content, equivalent rebuild results, and no canonical mutation during refresh or
rebuild.

### Cycle 05: Recovery and Foundation Gate

**Planned document:** `docs/Cycles/wave-01-cycle-05-recovery-and-foundation-gate.md`

**Purpose:** Integrate the prior local capabilities and prove that interruptions
and retries never lose or duplicate user work.

**Prerequisites:** Cycles 01 through 04.

**In scope:**

- Failure injection at configuration write, initialization commit, context
  branch/worktree creation, item write, checkpoint commit, and SQLite refresh.
- Cross-process repository operation lease implementation and coverage.
- Retry reconciliation based on actual Git state and canonical Markdown.
- The Wave 1 acceptance matrix and mandated Rust verification commands.

**Out of scope:** Remote/authentication failures, merge conflicts, desktop UI,
and CLI daemon lifecycle testing; these are later Wave evidence.

**Exit evidence:** Each injected failure preserves recoverable state and retries
only unfinished work. The full integration matrix passes against disposable real
repositories without duplicate items, comments, commits, branches, worktrees,
or cleanup effects.

## Integration Gate

Wave 01 is complete only when all five Cycles have exit evidence and the
following conditions are met:

- Existing and unborn repositories enable with exactly one initialization
  checkpoint and no automatic publication.
- Valid documents, tickets, comments, worktrees, local checkpoints, and
  discovery operate together against real temporary repositories.
- Marker-only, malformed, duplicate, inaccessible, dirty-primary, and injected
  lifecycle-failure cases remain visible and recoverable without silent loss.
- A full SQLite rebuild reproduces discovery from canonical state without
  modifying Markdown, Git commits, branches, worktrees, remotes, or repository
  configuration.
- The required Rust verification suite succeeds:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

- The CLI smoke test runs once the applicable Wave 1 command surface exists:

```sh
devenv shell -- cargo run --locked --bin manyhands-cli
```

- A desktop smoke test runs only when a Wave 1 Cycle introduces desktop code and
  an active desktop display is available:

```sh
devenv shell -- cargo run --locked --features desktop --bin manyhands
```

## Risks and Controls

| Risk | Control and required evidence |
| --- | --- |
| libgit2 worktree edge cases differ across platforms | Disposable real-repository tests cover unborn repositories, deterministic paths, repeated provisioning, and cleanup after partial creation. |
| Existing marker-only project documents become nonconforming | Detect visibly, preserve unchanged, and require explicit repair; do not auto-migrate. |
| Initialization or checkpoint failure leaves partial state | Persist operation steps, inspect real Git state on retry, and test every failure boundary. |
| SQLite cache diverges from repository state | Full per-context scanning, context-scoped transactions, and rebuild equivalence tests make Git and Markdown authoritative. |
| Desktop, CLI, or daemon processes race a repository operation | Cycle 03 callers serialize local authoring; Cycle 05 adds the repository-scoped lease and local contention coverage before Wave 2/3 daemon expansion. |
| Foundation scope expands into remote collaboration | Treat SSH, fetch/push, polling, merge, promotion, and closure as explicit Wave 2 exclusions. |

## Deferred Work

Wave 2 owns authentication, SSH transport, remote synchronization, polling,
remote context materialization, multi-context choice, merge recovery,
managed-document promotion, and ticket closure. Wave 3 owns the desktop
information architecture, editor behavior, full CLI contract, keyboard
journeys, and dogfooding acceptance on supported platforms.

No Cycle may use a deferred capability as a hidden prerequisite. If a Wave 1
implementation discovers that a deferred dependency is necessary, work pauses
for an RFC or Wave amendment rather than silently expanding scope.
