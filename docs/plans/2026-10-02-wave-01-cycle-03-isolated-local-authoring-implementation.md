---
title: "Wave 01 Cycle 03 Isolated Local Authoring Implementation Plan"
date: 2026-10-02
status: approved
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K7B5E7G9J1M3P5R7T9V1X3Z5"
---

# Wave 01 Cycle 03 Isolated Local Authoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver deterministic local document and ticket contexts, canonical
authoring, scoped checkpoints, and recoverable local comment submission without
remote contact or unrelated Git mutation.

**Architecture:** Extend the existing headless `RepositoryService` rather than
introducing a second service. It derives and validates context paths from an
item's caller-supplied ID, uses the existing canonical module to parse and
serialize Markdown, and builds each commit with an in-memory index based on the
context `HEAD`. SQLite remains a repository-registration invalidation flag only.

**Tech Stack:** Rust 2024, existing `git2`/libgit2, `rusqlite`, `time`,
`tempfile`, canonical domain module, Cargo, and Devenv/Nix.

**Commit Policy:** Do not create commits unless the user explicitly requests
one. If requested, stage only the completed task's files and use the suggested
commit message.

---

## Authorities And Fixed Decisions

Read these before implementation:

- `AGENTS.md`
- `docs/Cycles/wave-01-cycle-03-isolated-local-authoring.md`
- `docs/plans/2026-10-02-wave-01-cycle-03-isolated-local-authoring-design.md`
- `docs/RFC/canonical-content-and-comment-schema.md`
- `docs/RFC/git-workflow-and-conflict-recovery.md`
- `docs/RFC/repository-index-persistence-and-refresh.md`
- `docs/RFC/test-and-compatibility-strategy.md`

The implementation must preserve these approved decisions:

- Use `git2`/libgit2 only. Do not invoke a system Git executable or configure a
  remote transport callback.
- Add no dependencies, no desktop code, no CLI code, and no new SQLite tables.
- `AuthoringKind` has only `Document` and `Ticket`; comments use their target
  item's context and never receive their own branch or worktree.
- An editable item's only Wave 1 context is
  `manyhands/<kind>/<ULID>` at `.manyhands/worktrees/<ULID>/`.
- A create caller retains its ULID and draft. A save resolves Git identity before
  writing Markdown and returns a typed identity-required outcome when absent.
- A document move stages precisely the source deletion and destination change.
- Save commits use a temporary index and never alter the live index.
- After a checkpoint, only `repositories.refresh_required` changes locally.
  Cycle 03 neither scans discovery nor persists an operation record.
- Comments never contact a remote. They report `PublishPending` with no remote
  and `SyncDeferred` when configuration names a publication remote.
- Cycle 03 callers serialize operations. Cross-process lease enforcement belongs
  to Cycle 05.

## Public Contract

Add the types and methods from the approved design to `src/repository.rs`.
Keep constructors and helpers private; export only authoring requests, context
observations, outcomes, and typed errors a future frontend or CLI needs.

```rust
pub enum AuthoringKind { Document, Ticket }
pub enum ContextIntent { Create, Edit }

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

pub enum ContextProvisionOutcome { Created(ItemContext), Reused(ItemContext) }
pub struct DocumentDraft { pub title: String, pub body: String }
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
pub struct SaveTicketRequest { pub target: AuthoringTarget, pub draft: TicketDraft }
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
    Saved { context: ItemContext, checkpoint: LocalCheckpoint },
}
pub enum CommentPublicationState { PublishPending, SyncDeferred }
pub enum CommentSubmissionOutcome {
    IdentityRequired { context: ItemContext },
    Saved {
        context: ItemContext,
        checkpoint: LocalCheckpoint,
        publication: CommentPublicationState,
    },
}
```

Add `prepare_context`, `save_document`, `save_ticket`, and `submit_comment` to
`RepositoryService`. Each method must recompute and validate its deterministic
context; do not trust a caller-provided worktree path.

### Task 1: Add The Authoring Test Target And Public API Skeleton

**Files:**
- Modify: `src/repository.rs:18-173`
- Create: `tests/local_authoring.rs`

**Step 1: Write the failing API-construction test**

Create `tests/local_authoring.rs` with `mod support;`, import the public
authoring types, and add a test that builds a document `AuthoringTarget` from a
fixed parsed `ItemId` and calls `RepositoryService::prepare_context` against a
fixture repository.

```rust
#[test]
fn prepare_context_creates_a_document_context_at_its_deterministic_location() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        service.enable(EnableRepositoryRequest {
            root: fixture.root.clone(),
            primary_branch: "main".to_owned(),
            identity: None,
        }).unwrap(),
        EnableRepositoryOutcome::Enabled { .. },
    ));
    let item_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();

    let context = service.prepare_context(AuthoringTarget {
        root: fixture.root.clone(),
        kind: AuthoringKind::Document,
        item_id,
        intent: ContextIntent::Create,
    }).unwrap();

    assert!(matches!(context, ContextProvisionOutcome::Created(_)));
}
```

**Step 2: Run the test to verify it fails**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring prepare_context_creates_a_document_context_at_its_deterministic_location
```

Expected: FAIL because the public authoring types and method do not exist.

**Step 3: Add the API declarations only**

Add the public request, context, checkpoint, and comment-outcome types to
`src/repository.rs`. Add the four `RepositoryOperation` variants and only the
new error kinds needed to distinguish an unenabled repository, missing or
mismatched target, occupied item path, and injected authoring failure. Add
method signatures that return a deliberate `todo!()` only temporarily; do not
add any filesystem or Git behavior in this task.

**Step 4: Run formatting and confirm the test now fails only at the stub**

Run:

```sh
devenv shell -- cargo fmt --check
devenv shell -- cargo test --locked --test local_authoring prepare_context_creates_a_document_context_at_its_deterministic_location
```

Expected: formatting passes; the test fails at the deliberate implementation
stub rather than a missing import or type error.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/local_authoring.rs
git commit -m "feat: define local authoring contract"
```

### Task 2: Add Enabled-Repository And Authoring Fixtures

**Files:**
- Modify: `tests/support/mod.rs:17-246`
- Modify: `tests/local_authoring.rs`

**Step 1: Write fixture-contract tests**

Add tests proving a helper can enable a born `main` repository in an isolated
temporary data directory and return both the `RepositoryService` and that
fixture-owned data directory. Add a helper assertion that opens a linked context
worktree and returns its `HEAD` branch, commit, live-index bytes, and working
path.

**Step 2: Run the fixture tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring fixture_
```

Expected: FAIL because the authoring fixture helpers do not exist.

**Step 3: Add minimal reusable helpers**

Add `enabled_repository`, fixed ULID parsers, canonical document/ticket source
writers, `head_commit`, `index_bytes`, commit-tree path inspection, and a
worktree-opening helper. Keep fixture commits and identities local to the
temporary repository. Do not add a shell-Git helper or a mock repository.

**Step 4: Run the fixture tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring fixture_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add tests/support/mod.rs tests/local_authoring.rs
git commit -m "test: add local authoring fixtures"
```

### Task 3: Validate Authoring Preconditions And Prepare Contexts

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/local_authoring.rs`

**Step 1: Write failing context tests**

Add cases that assert:

- A create target makes `manyhands/document/<ULID>` from primary `HEAD` and
  `.manyhands/worktrees/<ULID>/` with that branch checked out.
- A second exact create request reuses the branch and worktree rather than
  creating another resource.
- An edit target with no existing context creates one only when primary has the
  requested conforming document or ticket.
- An unenabled repository, wrong primary `HEAD`, missing target item, duplicate
  primary item ID, dirty configuration path, mismatched branch, and mismatched
  worktree return typed errors without changing refs, worktrees, files, live
  index, or registry rows.

**Step 2: Run the context tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring context_
```

Expected: FAIL because context preflight and provisioning are absent.

**Step 3: Implement context observation and preflight**

Use `canonical_repository_root`, `read_configuration_for`,
`checked_out_branch`, and `ensure_configuration_path_clean` to validate the
primary root. Add private helpers that:

- Derive branch name, worktree name, and path from `AuthoringKind` and
  `ItemId`.
- Collect canonical sources in a supplied worktree without descending into
  `.manyhands/worktrees`.
- Find a conforming document or ticket by ID and kind in that source set.
- Validate that an existing worktree's path and checked-out branch exactly match
  the expected context.

For `Edit`, require the requested item in primary only when no valid context
already exists. For `Create`, reject a primary ID collision but accept an exact
partial deterministic context for retry.

**Step 4: Create or reuse the branch and worktree**

Create a local branch from the primary `HEAD` with `Repository::branch`. Create
the ignored worktree base as needed, then create the worktree with
`Repository::worktree` and `WorktreeAddOptions::reference` set to that branch's
reference. Use the item ID string as the worktree name. Never force a branch,
check out another branch in primary, or create a second context path.

**Step 5: Run the context tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring context_
```

Expected: PASS.

**Step 6: Commit if explicitly requested**

```sh
git add src/repository.rs tests/local_authoring.rs tests/support/mod.rs
git commit -m "feat: provision deterministic item contexts"
```

### Task 4: Implement Document Creation, Editing, Moves, And Checkpoints

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/local_authoring.rs`

**Step 1: Write failing document tests**

Add tests that assert:

- A new document writes valid canonical Markdown at a requested `docs/` path,
  creates its deterministic context, and commits `Checkpoint document <ULID>`.
- Editing a document updates title/body but preserves semantic unknown front
  matter and exact unchanged body content.
- Moving a document retains its ID, removes only the source, creates only the
  destination, and commits both paths in one checkpoint.
- An exact same-ULID create retries a written-but-uncommitted document as one
  checkpoint and returns `NoChange` after its completed checkpoint; a different
  destination item remains an occupied-path error.
- A move retry resumes source-only, destination-only, and matching-both-path
  states only when its destination matches the requested document state, without
  duplicating a document or a commit.
- A noncanonical path, mismatched ID, missing source, unsafe existing file, or
  Unix symlinked destination-parent component returns a typed error without
  overwriting content or staging an unrelated path.
- A context with unrelated modified, staged, untracked, deleted, and conflicted
  paths commits only the owned document path or move pair and leaves its live
  index bytes and unrelated status entries unchanged.

**Step 2: Run the document tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring document_
```

Expected: FAIL because `save_document` is not implemented.

**Step 3: Implement `save_document`**

Prepare the target context. For creation, require `source_path == None`, build
`canonical::Document` with the caller ULID and an empty unknown mapping, then
serialize it. For an edit, parse the source from the context, require a matching
document ID, copy its unknown mapping, update title/body, and serialize it.
Validate destination paths through `canonical::parse_item` after serialization.

Before every new parent directory or owned file write, walk its relative
components from the context root, reject symlinks and non-directories, and
create only missing directories. An exact existing create is a retry only when
its parsed kind, ID, and caller-controlled fields match; otherwise reject it.
For a move, accept source-only, destination-only, or matching-both-path partial
state, complete the missing write or source removal, and checkpoint the pair.

Add private write and checkpoint helpers that resolve identity before writing,
atomically replace owned files, build an `Index::new()` from context `HEAD`, add
only owned blobs, remove a move source, and compare the resulting tree with
`HEAD`. Equal trees still call `mark_registered_refresh_required`; report
`NoChange` after it succeeds or `RefreshPending` with context `HEAD` after it
fails. Changed trees commit with `Checkpoint document <ULID>` and invalidate the
registry only after commit success. Never write the live index. Add
`BeforeItemWrite` and `BeforeCheckpointCommit` failure points at the external
write and commit boundaries.

**Step 4: Run the document tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring document_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/local_authoring.rs tests/support/mod.rs
git commit -m "feat: author and checkpoint managed documents locally"
```

### Task 5: Implement Ticket Creation And Editing

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/local_authoring.rs`

**Step 1: Write failing ticket tests**

Add tests for a new ticket at its exact canonical path, an edit preserving
unknown front matter and existing closure metadata, a mismatched ticket ID or
kind rejected without mutation, scoped commit-tree contents, completed-create
no-op, and a prior uncommitted ticket write retried as one checkpoint.

**Step 2: Run the ticket tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring ticket_
```

Expected: FAIL because `save_ticket` is not implemented.

**Step 3: Implement `save_ticket`**

Prepare the ticket context and use only
`.manyhands/tickets/<ULID>/ticket.md`. For creation, construct a canonical
ticket from the caller draft with no closure metadata or unknown keys. For edit,
parse the existing ticket, retain its unknown mapping plus `closed_at` and
`closed_by`, and replace only caller-editable fields. Serialize, validate,
atomically write through the safe-parent helper, and checkpoint only the ticket
path with `Checkpoint ticket <ULID>`. Apply the same exact-create retry rule as
documents.

**Step 4: Run the ticket tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring ticket_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/local_authoring.rs tests/support/mod.rs
git commit -m "feat: author tickets in local contexts"
```

### Task 6: Implement Root Comments And Replies

**Files:**
- Modify: `src/repository.rs`
- Modify: `tests/local_authoring.rs`

**Step 1: Write failing comment tests**

Add tests that assert:

- A root comment reuses or provisions its target item's document or ticket
  context, writes only its canonical comment path, and commits
  `Checkpoint comment <ULID>`.
- A reply validates an existing parent comment for the same target item and
  serializes a UTC `created_at` timestamp exactly once.
- Retrying a written-but-uncommitted or completed exact comment retains the
  original `created_at`, creates at most one commit, and returns a no-op once
  the checkpoint and invalidation are complete.
- Missing target, create-intent target, missing parent, cross-item parent,
  different duplicate comment ID, unsafe parent path, and invalid comment body
  leave worktree files, refs, live index, and unrelated paths unchanged.
- No publication remote returns `PublishPending`; a configured SSH publication
  remote returns `SyncDeferred`; neither case contacts a remote.

**Step 2: Run the comment tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring comment_
```

Expected: FAIL because `submit_comment` is not implemented.

**Step 3: Implement `submit_comment`**

Require `ContextIntent::Edit`, prepare the target context, collect and validate
canonical context sources, and find the target item plus optional parent comment.
When the comment path is absent, build a `canonical::Comment` with
`OffsetDateTime::now_utc()`, an empty unknown mapping, and the caller body. When
it exists, accept it only as an exact semantic retry and preserve its
`created_at`; otherwise return the duplicate error. Validate the serialized or
existing comment in the full context before writing or checkpointing it. Use the
safe-parent helper, checkpoint only its canonical comment path, and map the
configured publication-remote presence to the approved local publication state.

**Step 4: Run the comment tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring comment_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/local_authoring.rs tests/support/mod.rs
git commit -m "feat: checkpoint local comments and replies"
```

### Task 7: Prove Partial Provisioning And Checkpoint Recovery

**Files:**
- Modify: `src/repository.rs:118-124`
- Modify: `tests/support/mod.rs:29-45`
- Modify: `tests/local_authoring.rs`

**Step 1: Write failing recovery tests**

Add one `FailOnce` test per boundary:

- `BeforeContextBranchCreation` leaves no branch or worktree; retry creates one
  of each.
- `BeforeWorktreeCreation` retains only the exact deterministic branch; retry
  adds one worktree without replacing the branch.
- `BeforeItemWrite` preserves prior bytes and leaves no checkpoint.
- `BeforeCheckpointCommit` preserves the valid written Markdown, creates no
  commit, and retry creates one checkpoint.
- `BeforeRegistryWrite` after a checkpoint returns `RefreshPending`; retry sets
  `refresh_required` without another commit.
- A pre-seeded partial document move with only its destination or both matching
  paths, whose destination matches the requested document state, resumes to one
  destination and one checkpoint; a no-op save still retries registry
  invalidation.

For every case, snapshot refs, worktree names/paths, commit count, owned file
bytes, live-index bytes, unrelated statuses, and registry rows before failure
and after retry.

**Step 2: Run the recovery tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring recovery_
```

Expected: FAIL because the named authoring failure points and recovery mapping
are incomplete.

**Step 3: Extend the existing narrow failure hook**

Add `BeforeContextBranchCreation` and `BeforeWorktreeCreation` to
`FailurePoint` and invoke the existing single-use hook immediately before those
operations. `BeforeItemWrite` and `BeforeCheckpointCommit` were added in Task
4. Reuse `BeforeRegistryWrite` for invalidation. Do not mock `git2`, SQLite, or
filesystem behavior and do not add a durable operation record.

**Step 4: Run the recovery tests to verify they pass**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring recovery_
```

Expected: PASS.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/support/mod.rs tests/local_authoring.rs
git commit -m "test: cover local authoring recovery"
```

### Task 8: Run Cycle Verification And Review Boundaries

**Files:**
- Modify only if an approved authority changes: `docs/Cycles/wave-01-cycle-03-isolated-local-authoring.md`

**Step 1: Run the focused Cycle target**

Run:

```sh
devenv shell -- cargo test --locked --test local_authoring
```

Expected: PASS with provisioning, document, ticket, comment, checkpoint, and
recovery coverage complete.

**Step 2: Run required repository verification**

Run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

Expected: every command exits zero.

**Step 3: Review Cycle boundaries**

Confirm the patch added no direct GPUI dependency, system Git invocation,
remote transport, SSH credentials, discovery scan/index update, SQLite table or
operation record, cross-process lease, context selection, cleanup, merge, or
promotion behavior. Confirm that only the existing registration invalidation
flag changes after a checkpoint.

**Step 4: Update documentation only for an approved authority change**

Do not change the approved Cycle, Wave, or RFC documents to restate
implementation details. Amend them only if a discovered contradiction receives
product-owner approval first.

**Step 5: Commit if explicitly requested**

```sh
git add src/repository.rs tests/support/mod.rs tests/local_authoring.rs
git commit -m "feat: add isolated local authoring"
```

## Completion Evidence

The Cycle is ready for implementation approval when every Cycle 03 exit
criterion has a focused test and the approved boundaries above remain intact.
The Cycle is ready to declare complete only after Task 8 confirms the focused
and full required Rust verification commands pass.
