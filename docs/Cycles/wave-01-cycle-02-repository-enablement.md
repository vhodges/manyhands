---
title: "Wave 01 Cycle 02: Repository Enablement"
date: 2026-09-30
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6Z0A1B2C3D4E5F6G7H8J9K0"
---

# Wave 01 Cycle 02: Repository Enablement

## Parent Wave

This is Cycle 02 of [Wave 01: Foundations](../Waves/wave-01-foundations.md).
It makes a local Git repository safely enableable and durably registered before
later Cycles add item contexts, checkpoints, and discovery.

## Purpose

Implement headless local repository lifecycle operations for Manyhands. The
Cycle lets callers inspect an existing repository, create a new local
repository, enable it with an explicitly confirmed primary branch, manage its
local remotes, and remove its application-local registration. It preserves the
canonical configuration, Git history, and user work as the authoritative state;
the application-local registry is durable but rebuildable metadata.

No Cycle 02 operation contacts a remote. A configured SSH publication remote
only records a future publication target.

## Prerequisites

- Cycle 01 exit criteria remain satisfied, including its canonical schema API
  and disposable real-repository fixtures.
- Wave 01 entry gate remains satisfied.
- The approved canonical schema, Git workflow, repository/index, and test
  strategy RFCs remain unchanged or receive an approved amendment.
- Implementation starts from a clean `main` branch.

## RFC and PRD Traceability

| Source | Cycle responsibility |
| --- | --- |
| [Canonical content and comment schema RFC](../RFC/canonical-content-and-comment-schema.md) | Write and validate version 1 repository configuration, including the explicitly selected primary branch and optional publication remote. |
| [Git workflow and conflict recovery RFC](../RFC/git-workflow-and-conflict-recovery.md) | Use `git2`/libgit2 for local repository inspection and initialization; preserve user work; resolve local commit identity; create the initialization commit. |
| [Repository index persistence and refresh RFC](../RFC/repository-index-persistence-and-refresh.md) | Persist application-local repository registration by canonical root path and mark an enabled repository for later discovery refresh. |
| [Test and compatibility strategy RFC](../RFC/test-and-compatibility-strategy.md) | Exercise local lifecycle behavior through disposable real repositories, failure injection, and no-network assertions. |
| `MH-REPO-001` to `MH-REPO-004` | Add, create, enable, inspect, remove, and locally manage repositories and remotes without automatic publication. |
| `MH-INDEX-001` to `MH-INDEX-002` | Establish rebuildable local registration and enablement invalidation; do not implement content discovery yet. |
| `MH-NFR-001`, `MH-NFR-006`, and `MH-NFR-007` | Keep repositories usable offline; make lifecycle failures recoverable and idempotent; do not mutate unrelated user work. |

## In Scope

- A small headless repository-lifecycle API exported by `src/lib.rs`; it has no
  GPUI, GPUI Kit, desktop, or CLI dependency.
- Local inspection of a selected path, including canonical root, repository
  kind, primary-branch candidates, current primary-worktree state, local commit
  identity availability, enabled configuration, and configured remotes.
- Creation of a new non-bare repository at an explicitly chosen valid local
  path, with its explicitly chosen initial primary branch.
- Enablement of existing born and unborn non-bare repositories after explicit
  primary-branch confirmation.
- Idempotent `.git/info/exclude` management for `.manyhands/worktrees/` without
  changing tracked `.gitignore` files.
- Version 1 `.manyhands/config.toml` initialization and exactly one
  `Initialize Manyhands` commit when enablement creates that configuration.
- Resolution of the effective repository or global Git `user.name` and
  `user.email`, with a typed identity-required outcome when either is absent.
  A separately confirmed caller-supplied identity may be written only to the
  local repository configuration.
- Application-local SQLite registration keyed by canonical repository root,
  including enabled time, observed configuration identity, accessibility state,
  and a later-discovery invalidation marker. Cycle 04 extends this database for
  contexts, items, problems, and operation recovery.
- Local inspection, addition, removal, and listing of named Git remotes without
  fetch, push, or any other network contact.
- Explicit selection or clearing of one publication remote. Selection accepts
  only an existing remote with an SSH URL; non-SSH remotes remain visible and
  manageable but cannot become a publication remote.
- Tracked publication-remote configuration changes that stage only
  `.manyhands/config.toml` and use the deterministic commit subject
  `Configure Manyhands publication remote`.
- Removal of a repository from Manyhands by deleting only its application-local
  registration and invalidation state.

## Out of Scope

- SSH key creation, import, selection, passphrase handling, host verification,
  authentication callbacks, or any remote contact.
- Fetch, push, synchronization, remote polling, remote-context materialization,
  merge, rebase, promotion, ticket closure, or branch/worktree cleanup.
- Item-context provisioning, document/ticket/comment creation or editing,
  scoped item checkpoints, and automatic worktree cleanup.
- Content scanning, SQLite item/context/problem tables, discovery queries,
  cache rebuild, filesystem watching, and polling schedules. These are Cycle 04
  work.
- Cross-process repository leases and durable operation-recovery records. These
  are Cycle 05 work.
- Desktop views, prompts, editor behavior, CLI commands, JSON output, daemon
  behavior, or automatic repair of marker-only content.
- Modification of global Git configuration, tracked `.gitignore`, user content,
  unrelated staged changes, or unrelated Git branches and worktrees.

## Planned Implementation Changes

| Path | Change |
| --- | --- |
| `Cargo.toml` | Move `git2` to production dependencies and add headless SQLite and application-data-directory dependencies for the local repository registry. |
| `Cargo.lock` | Record resolved production dependency versions. |
| `src/lib.rs` | Export the shared repository-lifecycle module without introducing frontend dependencies. |
| `src/repository.rs` | Add local inspection, creation, enablement, identity resolution, registry, remote-management, and recoverable-outcome domain logic. |
| `tests/support/mod.rs` | Extend disposable `git2` fixtures with identity-free, dirty, conflicted, remote-bearing, and failure-injection repository states. |
| `tests/repository_enablement.rs` | Add focused real-repository integration coverage for the Cycle exit gate. |

The implementation MAY split `src/repository.rs` into focused internal modules
only when that materially improves readability or testability. The public API
MUST remain small, synchronous, headless domain logic usable by both front
ends. It MUST open `git2` repository handles only for the operation using them;
no handle may be retained in application or future desktop model state.

## Lifecycle Contract

### Inspection and Registration

Inspection canonicalizes the requested root and distinguishes inaccessible,
non-Git, bare, and usable non-bare repositories without mutation. It reports
the local branches and remotes needed for a caller to present an explicit
primary-branch or publication-remote choice. It does not infer either choice
from a remote name, upstream, or Git push-default configuration.

Registering a repository records only application-local metadata. A canonical
root may have at most one registration. Removing it deletes only that
registration and its local invalidation state; it MUST NOT write files, alter
Git configuration, remove remotes, move worktrees, or create/delete commits or
branches.

### Creation and Enablement

Creation accepts an explicitly selected empty or nonexistent local target and
an explicitly confirmed initial primary branch. If it creates a directory and
repository but cannot complete initialization, cleanup is limited to empty
resources created by that failed operation; pre-existing paths are never
removed or changed.

For an existing born repository, enablement requires the selected primary
branch to exist locally. For an existing unborn repository, the confirmed
branch becomes its initial `HEAD` before the initialization commit. Before any
mutation, the primary worktree must be accessible, clean, and non-conflicted.
An existing valid Manyhands configuration is never overwritten: a repeated
enablement registers and invalidates discovery only, without a second
initialization commit.

When enablement creates configuration, it performs these local steps in order:

1. Resolve a usable commit identity from effective repository or global Git
   configuration, or return identity required before writing any repository
   state.
2. Add `.manyhands/worktrees/` once to the local `.git/info/exclude` file.
3. Write valid canonical `.manyhands/config.toml` with the confirmed primary
   branch and any explicitly selected valid SSH publication remote.
4. Stage only `.manyhands/config.toml` and create one primary-branch commit
   with subject `Initialize Manyhands`.
5. Record the canonical root locally and mark it invalidated for a later
   discovery refresh.

No step publishes, fetches, merges, rebases, stashes, discards, or cleans up
user work. A successful initialization commit remains authoritative if local
registration subsequently fails; retry reconciles the existing configuration
and commit, then performs registration only.

### Remote Management

Adding or removing an unselected remote changes only local Git remote
configuration. Remote creation and inspection must not invoke transport or
credential callbacks. A remote is eligible for publication selection only when
its configured URL is SSH; HTTP(S), `file`, and local-path remotes remain valid
local remotes but are rejected as publication targets with actionable feedback.

The selected publication remote is canonical tracked configuration. Selecting,
replacing, or clearing it validates the resulting configuration, writes only
`.manyhands/config.toml`, stages only that path, and commits it with
`Configure Manyhands publication remote`. Removing the selected remote is
rejected until the selection is cleared or changed. A failed configuration
checkpoint restores only state created or changed by that operation; it does
not affect unrelated remotes, index entries, staged paths, branches, or files.

## Recovery Considerations

Every rejected input or preflight failure identifies the repository root and
failed condition, preserves existing user files and Git state, and can be
retried after correction. This includes inaccessible, non-Git, bare, dirty,
conflicted, unwritable, and primary-branch-missing repositories; malformed or
unsupported existing Manyhands configuration; missing identity; invalid remote
names or URLs; and a missing selected publication remote.

Lifecycle tests inject failures at configuration write, initialization commit,
publication-remote configuration checkpoint, and local registry write. If a
failure occurs before a commit succeeds, the operation restores the prior
configuration and `.git/info/exclude` contents when it changed them, and leaves
no registration. If the commit succeeds, its OID and canonical configuration
win over local-registry state. A retry observes that state and must not create a
duplicate initialization or publication-remote commit.

## Test and Fixture Plan

`tests/repository_enablement.rs` MUST cover:

- Inspection of usable, unborn, non-Git, bare, and inaccessible selected paths
  without mutation or network contact.
- Creation and enablement of an empty local repository with a confirmed initial
  branch, exact configuration, local exclude rule, local identity, and one
  `Initialize Manyhands` commit.
- Enablement of an existing clean repository and an existing unborn repository,
  including primary-branch validation and exact staging boundaries.
- Repeated enablement of a conforming repository, proving one configuration
  initialization commit and one canonical-root registration only.
- Rejection of dirty, conflicted, unwritable, malformed-configured,
  unsupported-configured, non-bare, and missing-primary-branch states without
  changing user files, refs, commits, staged state, remotes, or registration.
- Effective identity use and identity-required behavior before any mutation,
  plus explicitly confirmed repository-local identity configuration.
- Application-local registry persistence, canonical-root uniqueness, enablement
  invalidation, and removal without repository mutation.
- Local remote listing, add, remove, SSH-only publication selection, selection
  replacement/clearing, rejection of HTTP(S) and local-path publication
  candidates, and rejection of removal while selected.
- Exact configuration-only staging and commit behavior for publication-remote
  selection changes, with no fetch, push, or transport callback.
- Failure injection after configuration write, commit creation, publication
  configuration changes, and registry write. Each case asserts the actual
  recoverable state and proves that retry performs only unfinished work.

Fixtures use `git2` and temporary directories. They configure local identities
inside fixture repositories and do not read developer repositories, global Git
configuration, application data, SSH keys, or a system Git executable.

## Verification

During implementation, run the focused test target after each completed
behavior slice:

```sh
devenv shell -- cargo test --locked --test repository_enablement
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

Cycle 02 is complete when:

- Headless shared code can inspect, create, enable, register, remove, and
  locally manage remotes for supported local repositories through `git2`.
- Existing clean and unborn repositories enable with a confirmed primary branch,
  a valid tracked configuration, an idempotent local exclude rule, and exactly
  one initialization commit when configuration is first created.
- Existing conforming repositories register without configuration replacement or
  duplicate initialization commits.
- Missing identities and all rejected repository states remain recoverable and
  leave user files, Git state, and unrelated local registration unchanged.
- Remote inspection and management use no network contact; only an explicitly
  selected SSH remote can become the canonical publication remote.
- Publication-remote changes checkpoint only canonical configuration, and
  removing a repository changes only application-local state.
- Failures preserve or restore the appropriate recoverable state, and retries
  do not duplicate configuration, commits, remotes, or registrations.
- The focused target and full required Rust verification suite pass.

## Handoff

Cycle 03 consumes the enabled-repository configuration, resolved local identity,
and disposable repository helpers to provision item contexts and create scoped
local checkpoints. It MUST NOT redefine enablement, primary-branch selection,
remote selection, initialization commits, or local-registration semantics.

Cycle 04 extends the application-local database from the Cycle 02 repository
registry to contexts, discovered items, visible problems, and operation
recovery. It uses Cycle 02 invalidation as an input to full canonical scans and
must not treat registry state as an authoritative copy of Git or Markdown.
