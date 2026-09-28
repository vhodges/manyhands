# Manyhands PRD Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a normative PRD that defines the collaboration-complete
dogfooding release, including automatic Git/worktree orchestration driven by
content lifecycle events.

**Architecture:** Write one self-contained Markdown document at `docs/PRD.md`.
Use stable requirement IDs and acceptance criteria to make the product scope
traceable, and retain implementation-specific decisions as explicit RFC
dependencies rather than prescribing solutions prematurely.

**Tech Stack:** Markdown, Git-backed repositories, rebuildable SQLite index,
Rust desktop application, Rust CLI.

---

### Task 1: Establish Product Baseline And Requirement Format

**Files:**
- Create: `docs/PRD.md`
- Reference: `docs/charter.md:13-152`
- Reference: `docs/plans/2026-09-28-prd-design.md:1-96`

**Step 1: Create the document frame**

Add a concise title and sections for product purpose, problem statement,
goals, non-goals, target users, scope, and requirement conventions. Explain
that requirement IDs use `MH-<area>-<number>` and each requirement contains a
priority, rationale, and acceptance criteria.

**Step 2: Record the dogfooding release boundary**

State that the release supports trusted Git collaborators working in enabled
local repositories. Include managed documents, tickets, threaded comments,
branches/worktrees, automatic checkpoint commits, explicit sync, and explicit
ticket closing. Explicitly defer boards, rollups, templates, meta-repository
propagation, multi-repo planning, and application-level authorization.

**Step 3: Verify the baseline is explicit**

Run: `rg -n "^## (Goals|Non-Goals|Target Users|Scope|Requirement Conventions)$|trusted Git collaborators|automatic checkpoint" docs/PRD.md`

Expected: Matches each baseline section and both scope constraints.

**Step 4: Review the boundary against the charter**

Confirm every initial capability selected for dogfooding has a clear place in
the document, and that deferred charter ideas are stated as non-goals rather
than lost.

### Task 2: Define Repository, Content, And Index Requirements

**Files:**
- Modify: `docs/PRD.md`
- Reference: `docs/charter.md:28-40,56-90,116-121`
- Reference: `docs/plans/2026-09-28-prd-design.md:43-55`

**Step 1: Write repository lifecycle requirements**

Add `MH-REPO-*` requirements for detecting/enabling a repository, adding and
removing local repositories, displaying/configuring remotes, and handling
repositories without remotes. Require plain-language recovery for invalid
paths and unavailable repositories.

**Step 2: Write content and comment requirements**

Add `MH-CONTENT-*` and `MH-COMMENT-*` requirements for discovery, creation,
viewing, editing, metadata validation, documents, tickets, and threaded
comments. Require an active edit to take precedence over a primary-branch copy
in lists and views, with its context labeled.

**Step 3: Write indexing and update requirements**

Add `MH-INDEX-*` requirements that define the filesystem and Git history as
authoritative, the application-local SQLite index as rebuildable cache, and
manual/polling refresh behavior. Require malformed or externally changed
content to be visible with remediation rather than silently omitted.

**Step 4: Verify requirement identifiers and acceptance criteria**

Run: `rg -n "^### MH-(REPO|CONTENT|COMMENT|INDEX)-[0-9]+|^\*\*Acceptance criteria:\*\*" docs/PRD.md`

Expected: Each product requirement area has uniquely identifiable,
acceptance-criteria-backed requirements.

### Task 3: Define Automatic Collaboration Requirements

**Files:**
- Modify: `docs/PRD.md`
- Reference: `docs/charter.md:70-78,88-89,106-114`
- Reference: `docs/plans/2026-09-28-prd-design.md:24-41`

**Step 1: State the automation principle**

Add an unambiguous `MH-COLLAB-001` requirement: collaboration tooling must be
automatic wherever possible and invoked by content lifecycle events, rather
than exposing a sequence of Git operations to the user.

**Step 2: Specify event-driven workflows**

Add requirements and acceptance criteria for these triggers:

- Creation or edit start provisions or reuses the item’s branch/worktree.
- Saving writes the Markdown, validates metadata, auto-commits, refreshes the
  index, and reports success or a recoverable failure.
- Comment creation or reply writes and auto-commits the comment.
- Sync is user-initiated and safely fetches, reconciles, and publishes.
- Ticket close is user-initiated and confirmed; it saves, commits, syncs,
  merges, pushes, and removes branch/worktree state only after successful
  prerequisites.

**Step 3: Define safety and recovery constraints**

Require progress feedback, safe cancellation, non-destructive failures, and
plain-language remediation for credentials, remote availability, conflicts,
invalid metadata, missing paths, and filesystem failures. Prohibit automatic
merge, push, and deletion without deliberate user action and confirmation.

**Step 4: Verify every lifecycle trigger is testable**

Run: `rg -n "MH-COLLAB|auto-commit|user-initiated|confirmation|recoverable" docs/PRD.md`

Expected: The automation rule, each event trigger, user-consent boundary, and
failure behavior are all stated as requirements with observable outcomes.

### Task 4: Define Interface, Quality, And Release Requirements

**Files:**
- Modify: `docs/PRD.md`
- Reference: `docs/charter.md:47-54,123-143`
- Reference: `docs/plans/2026-09-28-prd-design.md:57-96`
- Reference: `AGENTS.md:3-9`

**Step 1: Write desktop and CLI requirements**

Add `MH-UX-*` and `MH-CLI-*` requirements. Make the desktop application the
primary dogfooding interface with navigation, content lists, tabs, metadata,
comments, lifecycle controls, progress, and recovery. Require the headless
CLI to offer compatible safe domain operations with machine-readable output
and meaningful exit codes.

**Step 2: Write non-functional requirements**

Add `MH-NFR-*` requirements for local-first operation, supported desktop
platforms, responsive background work, no secret persistence, keyboard
accessibility, recoverable/idempotent operations, index rebuilding, and
preservation of canonical content.

**Step 3: Define validation journeys, risks, and RFC dependencies**

Add end-to-end acceptance journeys for offline authoring followed by sync,
conflict recovery, durable comment threading, and ticket close/merge cleanup.
List open RFC decisions: content schema and storage paths, merge policy,
identity, remote authentication UX, interrupted cleanup, external edits,
editor choice, and scale limits.

**Step 4: Review the finished PRD**

Run: `rg -n "^## (Desktop And CLI|Non-Functional Requirements|End-to-End Acceptance Journeys|Risks And Open Decisions|RFC Dependencies|Roadmap)$" docs/PRD.md`

Expected: Every finishing section is present. Read the document from start to
finish to ensure each must/shall statement has testable criteria and no RFC
implementation choice is presented as an unvalidated product fact.

### Task 5: Check the Documentation Change

**Files:**
- Verify: `docs/PRD.md`
- Verify: `docs/plans/2026-09-28-prd-design.md`
- Verify: `docs/plans/2026-09-28-prd-implementation.md`

**Step 1: Inspect the documentation diff**

Run: `git diff --check && git diff -- docs/PRD.md docs/plans/2026-09-28-prd-design.md docs/plans/2026-09-28-prd-implementation.md`

Expected: No whitespace errors; only the intended documentation changes appear.

**Step 2: Confirm existing repository guidance is preserved**

Run: `git status --short`

Expected: The intended PRD files are visible alongside any pre-existing
unrelated worktree changes, which must not be modified or committed.

**Step 3: Commit only on explicit request**

Do not create a commit unless the user explicitly asks for one. If requested,
stage only the intended documentation files and use a concise documentation
commit message.
