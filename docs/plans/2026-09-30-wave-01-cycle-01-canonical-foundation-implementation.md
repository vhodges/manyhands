---
title: "Wave 01 Cycle 01 Canonical Foundation Implementation Plan"
date: 2026-09-30
status: draft
author: "Vince Hodges <vhodges@gmail.com> && OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01K6YR0B1B2C3D4E5F6G7H8J9K"
---

# Wave 01 Cycle 01 Canonical Foundation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver the version 1 canonical configuration and Markdown domain
contract, including non-destructive validation, semantic unknown-metadata
preservation, deterministic comment threading, and reusable real-Git fixtures.

**Architecture:** Add one headless `manyhands::canonical` module that owns
typed configuration, identity, path, front-matter, validation, and comment-tree
logic. It accepts caller-supplied repository-relative paths and source strings,
never traverses or writes a repository, and returns typed values or structured
problems. Integration tests use disposable `git2` repositories only to establish
fixture state; production code remains independent of GPUI, GPUI Kit, SQLite,
CLI, and lifecycle operations.

**Tech Stack:** Rust 2024, Serde, `serde_yaml`, `toml`, `ulid`, `time`, `git2`,
`tempfile`, Cargo, Devenv/Nix.

**Commit Policy:** Do not create commits unless the user explicitly requests
one. If requested, stage only the completed task's files and use the suggested
commit message.

---

## Public Contract

Implement the following public surface in `src/canonical.rs`; keep helper
parsers and traversal internals private until a later Cycle demonstrates a need
to expose them.

```rust
pub const CONFIG_PATH: &str = ".manyhands/config.toml";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ItemId(/* private ULID */);

impl ItemId {
    pub fn generate() -> Self;
}

impl std::fmt::Display for ItemId { /* canonical uppercase spelling */ }
impl std::str::FromStr for ItemId { type Err = ValidationProblem; }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemKind {
    Document,
    Ticket,
    Comment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepositoryConfig {
    pub primary_branch: String,
    pub publication_remote: Option<String>,
    pub unknown: toml::Table,
}

pub fn parse_repository_config(source: &str) -> Result<RepositoryConfig, ValidationProblem>;
pub fn serialize_repository_config(config: &RepositoryConfig) -> Result<String, ValidationProblem>;

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub id: ItemId,
    pub title: String,
    pub body: String,
    pub unknown: serde_yaml::Mapping,
}

impl Document {
    pub fn set_title(&mut self, title: String) -> Result<(), ValidationProblem>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ticket {
    pub id: ItemId,
    pub title: String,
    pub ticket_type: String,
    pub status: String,
    pub project: Option<String>,
    pub team: Option<String>,
    pub closed_at: Option<time::OffsetDateTime>,
    pub closed_by: Option<String>,
    pub body: String,
    pub unknown: serde_yaml::Mapping,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comment {
    pub id: ItemId,
    pub item_id: ItemId,
    pub parent_id: Option<ItemId>,
    pub created_at: time::OffsetDateTime,
    pub body: String,
    pub unknown: serde_yaml::Mapping,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalItem {
    Document(Document),
    Ticket(Ticket),
    Comment(Comment),
}

impl CanonicalItem {
    pub fn body(&self) -> &str;
}

pub fn parse_item(path: &std::path::Path, source: &str)
    -> Result<CanonicalItem, ValidationProblem>;
pub fn serialize_item(item: &CanonicalItem) -> Result<String, ValidationProblem>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationCode {
    InvalidPath,
    MissingFrontMatter,
    MalformedFrontMatter,
    MissingField,
    InvalidField,
    KindPathMismatch,
    DuplicateId,
    MissingCommentItem,
    MissingParent,
    CrossItemParent,
    CommentCycle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationProblem {
    pub path: std::path::PathBuf,
    pub code: ValidationCode,
    pub message: String,
}

pub struct ValidatedContext {
    pub items: Vec<CanonicalItem>,
    pub problems: Vec<ValidationProblem>,
}

pub struct CommentThread {
    pub comment: Comment,
    pub replies: Vec<CommentThread>,
}

pub fn validate_context(
    sources: impl IntoIterator<Item = (std::path::PathBuf, String)>,
) -> ValidatedContext;
pub fn ordered_comment_threads(context: &ValidatedContext) -> Vec<CommentThread>;
```

`CommentThread` contains a root `Comment` and recursively ordered direct
replies. A later Cycle owns repository traversal and discovery persistence; it
will feed canonical source files into `validate_context` rather than expanding
this Cycle's scope.

### Task 1: Establish The Headless Canonical Module And Dependencies

**Files:**
- Modify: `Cargo.toml:13-15`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs:1`
- Create: `src/canonical.rs`
- Create: `tests/canonical_foundation.rs`
- Create: `tests/support/mod.rs`

**Step 1: Prove the focused test target does not exist**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation
```

Expected: FAIL because Cargo has no test target named `canonical_foundation`.

**Step 2: Add only the Cycle 01 dependencies**

Add these direct dependencies after the optional `gpui-kit` dependency:

```toml
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
time = { version = "0.3", features = ["formatting", "parsing"] }
toml = "0.8"
ulid = "1"

[dev-dependencies]
git2 = "0.20"
tempfile = "3"
```

Keep `git2` and `tempfile` in `[dev-dependencies]`: production parsing does not
open repositories, while the required fixture layer does. Do not add SQLite,
SSH, polling, CLI, or desktop dependencies.

**Step 3: Add the empty public boundary and focused test harness**

Replace `src/lib.rs` with:

```rust
//! Headless domain logic shared by the Manyhands front ends.

pub mod canonical;
```

Create `src/canonical.rs` with the module documentation and create the test
files below so the target compiles before behavior is added:

```rust
// src/canonical.rs
//! Version 1 canonical configuration and Markdown domain contracts.
```

```rust
// tests/canonical_foundation.rs
mod support;
```

```rust
// tests/support/mod.rs
#![allow(dead_code)]
```

**Step 4: Refresh the lockfile and verify the skeleton**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation
```

Expected: PASS with zero tests. `Cargo.lock` contains the resolved direct and
development dependencies.

**Step 5: Commit if explicitly requested**

```sh
git add Cargo.toml Cargo.lock src/lib.rs src/canonical.rs tests/canonical_foundation.rs tests/support/mod.rs
git commit -m "feat: establish canonical domain module"
```

### Task 2: Add IDs, Kinds, And Repository Configuration

**Files:**
- Modify: `src/canonical.rs`
- Modify: `tests/canonical_foundation.rs`

**Step 1: Write failing identity and configuration tests**

Add focused tests that require the following behavior:

```rust
use std::str::FromStr;

use manyhands::canonical::{
    parse_repository_config, serialize_repository_config, ItemId, ValidationCode,
};

#[test]
fn item_ids_accept_only_canonical_uppercase_ulids() {
    assert!(ItemId::from_str("01K6YQ1Z2V6B8N4M3R5T7W9X0A").is_ok());
    assert!(ItemId::from_str("01k6yq1z2v6b8n4m3r5t7w9x0a").is_err());
    assert!(ItemId::from_str("01K6YQ1Z2V6B8N4M3R5T7W9X0").is_err());
    assert!(ItemId::from_str("01K6YQ1Z2V6B8N4M3R5T7W9X0I").is_err());
}

#[test]
fn config_round_trip_retains_unknown_values() {
    let source = r#"format_version = 1
primary_branch = "main"
publication_remote = "origin"
future = { enabled = true, levels = [1, 2] }
"#;

    let config = parse_repository_config(source).unwrap();
    let rewritten = serialize_repository_config(&config).unwrap();
    let reparsed = parse_repository_config(&rewritten).unwrap();

    assert_eq!(reparsed, config);
}

#[test]
fn invalid_configuration_is_a_structured_problem() {
    let problem = parse_repository_config("format_version = 2\nprimary_branch = \"\"")
        .unwrap_err();

    assert_eq!(problem.code, ValidationCode::InvalidField);
}
```

Also cover missing `format_version`, a non-integer version, an omitted optional
remote, an empty remote when present, malformed TOML, and generated IDs whose
display form passes `ItemId::from_str`.

**Step 2: Run the tests to confirm the public contract is absent**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation item_ids_accept_only_canonical_uppercase_ulids
```

Expected: FAIL with unresolved imports or missing functions from
`manyhands::canonical`.

**Step 3: Implement the minimal typed configuration and identity contract**

Add `ItemId`, `ItemKind`, `ValidationCode`, and `ValidationProblem`. Reject any
identifier whose source spelling is not 26 uppercase Crockford Base32
characters before parsing it with `ulid`; render the wrapped value with its
canonical `Display` implementation. `ItemId::generate()` must produce the same
canonical spelling.

Implement `RepositoryConfig` with `primary_branch`, optional
`publication_remote`, and `toml::Table` unknown values. Parse into a TOML table,
remove and validate the known keys, retain all remaining entries, and serialize
a table that merges the retained entries with authoritative known values. Treat
the version as exactly integer `1` and reject an empty or NUL-containing
branch/remote name. Cycle 02 will verify configured names against an opened
repository before enablement; Cycle 01 performs no repository I/O. Return a
`ValidationProblem` rooted at `.manyhands/config.toml` for every parse or field
error.

**Step 4: Run the configuration and identity slice**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation item_ids
devenv shell -- cargo test --locked --test canonical_foundation config_
```

Expected: PASS. The serialized configuration may reorder or reformat TOML, but
reparsing must preserve every unknown value semantically.

**Step 5: Commit if explicitly requested**

```sh
git add src/canonical.rs tests/canonical_foundation.rs
git commit -m "feat: add canonical configuration and identities"
```

### Task 3: Parse And Serialize Canonical Markdown Items

**Files:**
- Modify: `src/canonical.rs`
- Modify: `tests/canonical_foundation.rs`

**Step 1: Write failing valid-content and round-trip tests**

Add test inputs for one document, open ticket, closed ticket, root comment, and
reply. Each must include an unknown mapping or sequence and a body containing
leading blank lines, trailing newlines, and YAML-looking text. Assert the
following round-trip property for each valid input:

```rust
let item = parse_item(path, source).unwrap();
let rewritten = serialize_item(&item).unwrap();
let reparsed = parse_item(path, &rewritten).unwrap();

assert_eq!(reparsed, item);
assert!(rewritten.ends_with(item.body()));
```

Add a document title-update assertion. Change only its typed `title`, serialize
and reparse it, then assert the unknown YAML mapping remains equal and the body
is exactly unchanged.

**Step 2: Run the new Markdown tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation valid_
```

Expected: FAIL because `parse_item`, `serialize_item`, and the typed item
models are not implemented.

**Step 3: Implement lexical front-matter extraction and typed models**

Implement a private extractor with these exact rules:

- The first bytes must be `---` followed by `\n` or `\r\n`; otherwise return
  `MissingFrontMatter`.
- The closing delimiter must be a later line containing only `---`, followed by
  a line ending or end-of-file; otherwise return `MalformedFrontMatter`.
- Parse only the bytes between delimiters as a YAML mapping. Keep the remaining
  source bytes after the closing delimiter's line ending, including blank lines
  and newline convention, as `body` without normalization. An end-of-file
  closing delimiter therefore produces an empty body.

Remove common YAML keys (`manyhands_managed`, `manyhands_kind`, `id`, and the
kind-specific fields) from a `serde_yaml::Mapping`, validating their exact types
and values. Store the remaining mapping in `unknown`. Require
`manyhands_managed: true`, the matching string kind, canonical IDs, nonempty
required strings, optional nonempty project/team values, and comment `created_at`
as an RFC 3339 timestamp with a UTC offset. A ticket is closed only when both
`closed_at` and `closed_by` are present and valid; reject exactly one closure
field. Do not infer ticket closure from its free-form status.

Add `Document::set_title(&mut self, title: String) -> Result<(),
ValidationProblem>` as the only known-field mutator required in this Cycle; it
must reject an empty title. Serialization must restore required known fields,
merge retained unknown fields without permitting them to override known keys,
format typed timestamps as RFC 3339 UTC, and append the exact stored body.

**Step 4: Run the Markdown round-trip slice**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation valid_
devenv shell -- cargo test --locked --test canonical_foundation round_trip
```

Expected: PASS. YAML field ordering, indentation, scalar quoting, and comments
may change; unknown values and Markdown body bytes may not.

**Step 5: Commit if explicitly requested**

```sh
git add src/canonical.rs tests/canonical_foundation.rs
git commit -m "feat: add canonical Markdown item parsing"
```

### Task 4: Enforce Canonical Paths And Visible Single-File Problems

**Files:**
- Modify: `src/canonical.rs`
- Modify: `tests/canonical_foundation.rs`

**Step 1: Write failing canonical-path tests**

Add a table-driven test that supplies valid source for these paths:

```text
docs/guide.md
docs/guides/authoring.md
.manyhands/tickets/01K6YQ2A4D8F1H3J5K7M9N0P2Q/ticket.md
.manyhands/comments/01K6YQ1Z2V6B8N4M3R5T7W9X0A/01K6YQ3B6E9G2J4K6M8N0P2R4S.md
```

Require rejection of absolute and traversal paths, `Docs/guide.md`, non-Markdown
document extensions, files in `.manyhands/worktrees/`, a ticket directory ID
that differs from front matter, and comment directory/filename IDs that differ
from the front matter. Add cases for marker-only input, malformed YAML, missing
fields, wrong field types, invalid timestamps, and kind/path mismatches. Each
must return a `ValidationProblem` with its original supplied path and the
appropriate code; no input source may be rewritten.

**Step 2: Run the path and malformed-content tests to verify failure**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation canonical_paths
devenv shell -- cargo test --locked --test canonical_foundation malformed_
```

Expected: FAIL because path classification and the complete problem taxonomy
are not yet enforced.

**Step 3: Implement lexical path classification**

Classify only lexically repository-relative `Path` values; do not normalize an
input path before validation. Reject an absolute path, an empty path, `.` or
`..` components, and any component under
`.manyhands/worktrees`. Accept a document when its first component is exactly
`docs`, its final component ends in `.md`, and it has at least a filename;
accept both direct children and nested files. For tickets and comments, require
the exact canonical component counts and literal final names described by the
schema RFC, then compare embedded IDs with parsed front matter.

Perform classification before returning a typed item. Map no-front-matter,
marker-only, YAML syntax, missing field, wrong type, timestamp, ID, and
path/kind failures to the stable `ValidationCode` variants rather than panicking
or modifying the source.

**Step 4: Run all single-file parsing tests**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation -- --nocapture
```

Expected: PASS for every identity, configuration, valid Markdown, path, and
malformed-content case added so far.

**Step 5: Commit if explicitly requested**

```sh
git add src/canonical.rs tests/canonical_foundation.rs
git commit -m "feat: validate canonical content paths"
```

### Task 5: Validate Context Relationships And Deterministic Threads

**Files:**
- Modify: `src/canonical.rs`
- Modify: `tests/canonical_foundation.rs`

**Step 1: Write failing context-validation tests**

Use `validate_context` with path/source pairs to prove these cases:

- A document or ticket plus a root comment and reply yields no problems.
- Duplicate IDs across every combination of document, ticket, and comment yield
  `DuplicateId` problems without dropping unrelated valid items.
- A comment pointing to no document/ticket yields `MissingCommentItem`.
- A reply whose parent is absent yields `MissingParent`.
- A reply whose parent belongs to another item yields `CrossItemParent`.
- Two or more comments with circular parent relationships yield `CommentCycle`.
- A malformed file remains a problem while valid files from the same context
  remain available.

Add an ordering fixture with deliberately out-of-order YAML source. Assert roots
and direct replies are ordered by parsed `created_at`, then by `ItemId` when the
timestamps are equal:

```rust
let threads = ordered_comment_threads(&validated);
assert_eq!(threads.iter().map(|thread| thread.comment.id.to_string()).collect::<Vec<_>>(),
    vec!["01K6YQ3B6E9G2J4K6M8N0P2R4S", "01K6YQ3C6E9G2J4K6M8N0P2R4T"]);
```

**Step 2: Run the context tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation context_
devenv shell -- cargo test --locked --test canonical_foundation comment_order
```

Expected: FAIL because `validate_context`, cross-file relationship validation,
and `CommentThread` construction do not exist.

**Step 3: Implement two-pass validation and threading**

First parse each input independently, retaining parse failures as problems and
valid values as candidates. Build a repository-context-wide ID map across all
kinds, marking every collision as `DuplicateId`. Build the document/ticket map
from unique valid item candidates, then validate each unique comment's `item_id`
and optional parent against it. Detect cycles with a visiting/visited depth-first
walk over comment parent IDs.

Only assemble comments whose item and parent relationships are valid. Group them
by `item_id`, construct roots and direct-reply children, sort every sibling list
by `created_at` then `id`, and return a recursively ordered `CommentThread`
forest. Never alter the source strings, generate replacement values, or discard
independent valid items because another file is nonconforming.

**Step 4: Run the relationship and ordering slice**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation context_
devenv shell -- cargo test --locked --test canonical_foundation comment_order
```

Expected: PASS. Repeat the ordering assertion after serializing and reparsing
the same fixture to show that ordering depends on semantic timestamps and IDs,
not YAML source order.

**Step 5: Commit if explicitly requested**

```sh
git add src/canonical.rs tests/canonical_foundation.rs
git commit -m "feat: validate comment relationships and threads"
```

### Task 6: Add Disposable Real-Repository Fixture Support

**Files:**
- Modify: `tests/support/mod.rs`
- Modify: `tests/canonical_foundation.rs`

**Step 1: Write failing fixture-isolation tests**

Require the support module to expose this test-only contract:

```rust
pub struct TestRepository {
    pub tempdir: tempfile::TempDir,
    pub repository: git2::Repository,
    pub root: std::path::PathBuf,
}

pub fn unborn_repository() -> TestRepository;
pub fn born_repository() -> TestRepository;
```

Test that the unborn repository has no `HEAD` target, while the born repository
has one initial commit and a locally configured `user.name` and `user.email`.
Write canonical configuration and Markdown fixture files beneath `root`, feed
their strings and relative paths to `validate_context`, and assert the temporary
root disappears when `TestRepository` is dropped. Do not execute a system `git`
command or read global Git configuration.

**Step 2: Run fixture tests to verify they fail**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation disposable_
```

Expected: FAIL because the fixture helpers do not yet create real repositories.

**Step 3: Implement isolated `git2` fixtures**

Implement `unborn_repository()` with `TempDir::new()` and `git2::Repository::init`.
Implement `born_repository()` by configuring only the repository-local identity,
creating a simple initial file/index/tree, and committing it with that local
signature. Add helper functions that return valid version 1 config, document,
ticket, root-comment, and reply strings with the canonical IDs used by tests.

Do not add Manyhands repository enablement, `.git/info/exclude` changes,
worktrees, checkpoints, remotes, or network operations to the fixture layer.

**Step 4: Run fixture and focused contract tests**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation disposable_
devenv shell -- cargo test --locked --test canonical_foundation
```

Expected: PASS. The test output identifies no developer repository, global Git
configuration, SSH data, or persistent temporary path dependency.

**Step 5: Commit if explicitly requested**

```sh
git add tests/support/mod.rs tests/canonical_foundation.rs
git commit -m "test: add canonical real-repository fixtures"
```

### Task 7: Verify The Cycle Exit Gate

**Files:**
- Verify: `Cargo.toml`
- Verify: `Cargo.lock`
- Verify: `src/lib.rs`
- Verify: `src/canonical.rs`
- Verify: `tests/support/mod.rs`
- Verify: `tests/canonical_foundation.rs`
- Verify: `docs/plans/2026-09-30-wave-01-cycle-01-canonical-foundation-design.md`
- Verify: `docs/plans/2026-09-30-wave-01-cycle-01-canonical-foundation-implementation.md`

**Step 1: Check formatting and the intended diff**

Run:

```sh
devenv shell -- cargo fmt --check
git diff --check
git diff -- Cargo.toml Cargo.lock src/lib.rs src/canonical.rs tests/support/mod.rs tests/canonical_foundation.rs
```

Expected: No formatting or whitespace errors. The production library contains
no `gpui`, `gpui_kit`, SQLite, SSH, polling, CLI, desktop, repository-mutation,
or filesystem-walk code.

**Step 2: Run the focused Cycle evidence**

Run:

```sh
devenv shell -- cargo test --locked --test canonical_foundation
```

Expected: PASS with coverage for valid and invalid config; upper-case ULIDs;
document/ticket/comment parsing; marker-only/malformed/path/relationship
problems; semantic unknown-key and exact-body preservation; deterministic
ordering; and born/unborn `git2` fixtures.

**Step 3: Run the required repository verification suite**

Run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

Expected: Every command exits successfully. No desktop or CLI smoke test is
required because this Cycle adds neither front-end behavior nor command surface.

**Step 4: Review the worktree without touching unrelated changes**

Run:

```sh
git status --short
```

Expected: Only Cycle 01 source, test, lockfile, and managed plan files are
listed, plus any unrelated pre-existing changes left untouched.

**Step 5: Commit if explicitly requested**

```sh
git add Cargo.toml Cargo.lock src/lib.rs src/canonical.rs tests/support/mod.rs tests/canonical_foundation.rs docs/plans/2026-09-30-wave-01-cycle-01-canonical-foundation-design.md docs/plans/2026-09-30-wave-01-cycle-01-canonical-foundation-implementation.md
git commit -m "feat: implement canonical content foundation"
```
