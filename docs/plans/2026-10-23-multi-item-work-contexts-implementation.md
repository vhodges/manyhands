---
title: "Multi-Item Work Contexts Implementation Plan"
date: 2026-10-23
status: "approved 2026-10-23, not started"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5NSWA10F2M81ZJNFQJC7RVG"
---

# Multi-Item Work Contexts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A ticket or document worktree contributes to the index every item
its branch changed, not only its own item. Reads say where each copy lives.

**Architecture:** A new `contributions` module works out which managed paths
a worktree's branch changed: from the merge base with primary, committed or
not. It validates those files with only the primary sources they depend on.
Discovery stores the result as item rows with a role (`own`, `changed`,
`comments`), plus deletion rows. The read layer picks one effective copy per
item ID: own context, then primary, then the earliest branch. It reports
every other copy as a version. The CLI and the desktop show the versions.

**Tech Stack:** Rust 2024, git2 (libgit2), rusqlite, serde, GPUI Kit (desktop
only).

**Spec:** `docs/plans/2026-10-23-multi-item-work-contexts-design.md`. Ticket
`01M5NKA3MY7CW4D90M866YS946`, with `01M4EPRY203WRKEDQH8403T35B` folded in.

## Global Constraints

- Run Rust commands through Devenv: `devenv shell -- cargo …`.
- Never rebase the ticket branch. Merge `origin/main` into it from the ticket
  worktree.
- A refresh writes no canonical Markdown and changes no Git state.
- `MAX_CONTEXT_CHANGES` is `1024`, counted in path order per context.
- The managed path prefixes are `docs/`, `.manyhands/tickets/` and
  `.manyhands/comments/`.
- The new problem codes are `context_changes_unavailable` and
  `context_changes_over_budget`. They are stored as
  `context-changes-unavailable` and `context-changes-over-budget`.
- The stored role values are `own`, `changed` and `comments`, and the stored
  change values are `added`, `modified` and `moved`. The stored primary
  states are `absent`, `identical` and `different`.
- JSON additions stay within v1, and the closed schemas in `schemas/v1/` are
  updated in place. New fields: `ItemDto.versions`,
  `ItemDto.primary_comparison`, `ItemContextDto.merged_into_primary`,
  `ItemDto.context_changes`, `CommentDto.context`, `CommentDto.versions` and
  `DocumentFolderDto.context`.
- The repository snapshot (`read_repository_snapshot`) reads `own` rows only.
- The poll gate `conforming_context_tree` does not change.
- Run only the relevant tests between tasks. The full suite and every gate
  run once, in Task 9.
- Each Rust task also passes
  `devenv shell -- cargo check --features desktop --locked` and its clippy
  counterpart. `--all-features` once hid a test-support leak from the
  desktop-only build.
- A new integration test target is added to the test list in
  `.github/workflows/build.yml`.
- Commits end with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **An item worktree left behind after its branch merged.** Its HEAD is an
   ancestor of primary's, so the merge base is its HEAD. It contributes only
   uncommitted work, and its context says `merged_into_primary: true`. Pinned
   in Task 4, `a_merged_worktree_contributes_only_uncommitted_work`.
2. **A changed file that is not text, or not an item at all.** It must
   produce a problem on that worktree and never fail the refresh. Pinned in
   Task 4, `a_changed_file_that_is_not_utf8_is_a_problem_on_its_context`.
3. **A managed path replaced by a symbolic link on a branch.** The link is
   not read. The item counts as deleted on that branch, and primary's copy
   stays effective. Pinned in Task 2, `a_symbolic_link_is_not_read`.
4. **Two branches that each add a document with the same new ID.** One
   effective copy comes from the earlier branch by ULID, and the other is a
   version with `duplicate_id`. Pinned in Task 5,
   `two_branches_adding_one_id_keep_the_earlier_and_flag_the_other`.
5. **A refresh while a branch is being edited.** A contributed file that
   changes between the two observations of one refresh must count as a
   changed context and be retried, like the own item. Pinned in Task 4,
   `a_contributed_file_changing_mid_refresh_is_retried`.

---

## File Structure

- Create `src/repository/contributions.rs`: changed-path detection, the
  primary view, and contribution classification. It has no SQLite, and the
  only discovery types it uses are the ones it defines.
- Create `src/repository/contributions_tests.rs`: unit tests over temporary
  Git repositories and hand-built sources.
- Modify `src/repository.rs`:
  - module declaration;
  - persistence (`persist_context`, `PersistedContext`,
    `persist_context_refresh`, `persist_comments`);
  - `changed_contexts`;
  - the snapshot query.
- Modify `src/repository/discovery.rs`: the observation types,
  `observe_root`, `observe_active_contexts`, role assignment, contributed
  folders, schema lists.
- Modify `src/results.rs`: two problem codes.
- Modify `src/repository/read/items.rs`: stored rows, ranking, versions,
  folders, `show_item_in`.
- Modify `src/repository/read/comments.rs`: contributed comments and their
  versions.
- Modify `src/repository/read/dto.rs`: the new DTO fields and enums.
- Modify `schemas/v1/item.schema.json`, `item_context.schema.json`,
  `comment.schema.json`, `document_folder.schema.json`,
  `problem.schema.json` and `index_problem.schema.json`.
- Modify `src/bin/manyhands-cli/parse.rs` and `dispatch.rs`: `--context`
  with `--id`.
- Modify `src/desktop/model/item.rs`, `src/desktop/view/item/details.rs` and
  `src/desktop/view/documents.rs`: the branch mark, versions and the primary
  comparison.
- Create `tests/context_contributions.rs`: integration tests for discovery
  and reads.
- Create `tests/index_scale.rs`: an ignored refresh benchmark.
- Modify `tests/support/items.rs`: a `commit_in` helper.
- Modify `tests/cli/content.rs`: `--id` with `--context`.
- Modify the docs RFCs: index, Git workflow, CLI contract, desktop.

---

### Task 0: Start of work

No code. Work starts after the CLI and desktop tracks, so this plan's line
references will have moved.

**Files:** none changed except the ticket's comments.

- [ ] **Step 1: Merge main into the ticket branch**

```bash
cd .manyhands/worktrees/01M5NKA3MY7CW4D90M866YS946
git fetch origin
git merge origin/main
```

- [ ] **Step 2: Re-check the references this plan relies on**

```bash
grep -n "fn collect_authoring_item_sources\|fn observe_active_contexts\|fn observe_root\|fn leave_active_items_to_their_contexts\|const ITEM_READ_COLUMNS\|const ITEM_READ_OBJECTS" src/repository/discovery.rs
grep -n "fn persist_context\b\|fn persist_context(\|fn persist_context_refresh\|fn persist_comments\|fn changed_contexts\|fn read_repository_snapshot(" src/repository.rs
grep -n "fn stored_items\|fn row_rank\|fn effective_rows\|fn effective_copy\|fn stored_folders\|pub fn show_item\|pub fn show_path" src/repository/read/items.rs
grep -n "pub fn list_comments\|fn stored_comments" src/repository/read/comments.rs
grep -n "context requires --path" src/bin/manyhands-cli/parse.rs
grep -n "conforming_context_tree" src/repository/remote/poll.rs
```

Expected: every name is found. If one is not, find what replaced it and add
a dated note under the task that uses it before changing code. If C3 added a
`path observe` answer about a path in a worktree, record how it treats a
file the branch changed. Task 7 must keep it consistent.

- [ ] **Step 3: Check the scope is still right**

Read the CLI contract's `show` rows and the desktop RFC's navigation
section as they stand on main. If a Cycle already added `--context` with
`--id`, or a versions field, stop and ask the owner how to reconcile it.

- [ ] **Step 4: Record the start on the ticket**

Add a comment under `.manyhands/comments/01M5NKA3MY7CW4D90M866YS946/`
naming the merged main commit and any references that moved. Use the
Manyhands CLI if C3 has shipped comment creation; otherwise write the file
directly (AGENTS.md). Commit it as `docs: start multi-item contexts work`.

---

### Task 1: Refresh benchmark and baseline

**Files:**
- Create: `tests/index_scale.rs`. Its only test is ignored, so it is not
  added to the native test list in `.github/workflows/build.yml`.

**Interfaces:**
- Consumes: `support::born_repository`, `support::enabled_repository`,
  `RepositoryService::refresh_repository`, `canonical::ItemId::generate`.
- Produces: the `before` numbers recorded on the ticket.

- [ ] **Step 1: Write the benchmark**

```rust
//! Times an index refresh over synthetic repositories. Ignored: run by
//! hand, before and after a discovery change, on the same machine.
//!
//! `MANYHANDS_SCALE_CELLS` picks the cells, as `<items>x<worktrees>` joined
//! by commas; the default is the ticket's full matrix. Each item file is
//! small, but a checkout holds every file, so a cell needs about
//! `items × (worktrees + 1) × 4 KiB` of disk. The 50,000 × 50 cell needs
//! about 10 GiB.

use std::{fs, path::Path, time::Instant};

use git2::{IndexAddOption, Repository, Signature, Time, WorktreeAddOptions};
use manyhands::{
    canonical::ItemId,
    repository::{RefreshOutcome, RefreshRepositoryRequest},
};

mod support;

const DEFAULT_CELLS: &str =
    "1000x1,1000x10,1000x50,10000x1,10000x10,10000x50,50000x1,50000x10,50000x50";
/// Files each worktree changes: new documents, never committed.
const CHANGES_PER_WORKTREE: usize = 10;

fn document(id: &ItemId, title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n---\n# {title}\n"
    )
}

fn ticket(id: &ItemId, title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\ntype: task\nstatus: open\n---\nBody.\n"
    )
}

fn comment(id: &ItemId, item: &ItemId) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item}\ncreated_at: 2026-10-23T00:00:00Z\n---\nA comment.\n"
    )
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Six documents in ten, three tickets, one comment on a ticket.
fn populate(root: &Path, items: usize) {
    let mut last_ticket = None;
    for n in 0..items {
        let id = ItemId::generate();
        match n % 10 {
            0..=5 => write(root, &format!("docs/f{:03}/d{n}.md", n / 100), &document(&id, &format!("D{n}"))),
            6..=8 => {
                write(root, &format!(".manyhands/tickets/{id}/ticket.md"), &ticket(&id, &format!("T{n}")));
                last_ticket = Some(id);
            }
            _ => {
                let item = last_ticket.clone().expect("a ticket comes first");
                write(root, &format!(".manyhands/comments/{item}/{id}.md"), &comment(&id, &item));
            }
        }
    }
}

fn commit_all(repository: &Repository) {
    let mut index = repository.index().unwrap();
    index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = Signature::new("Scale", "scale@example.invalid", &Time::new(1_700_000_000, 0)).unwrap();
    let parent = repository.head().unwrap().peel_to_commit().unwrap();
    repository.commit(Some("HEAD"), &signature, &signature, "scale", &tree, &[&parent]).unwrap();
}

#[test]
#[ignore = "benchmark: run by hand"]
fn refresh_scales() {
    let cells = std::env::var("MANYHANDS_SCALE_CELLS").unwrap_or_else(|_| DEFAULT_CELLS.to_owned());
    for cell in cells.split(',') {
        let (items, worktrees) = cell.split_once('x').unwrap();
        let (items, worktrees): (usize, usize) = (items.parse().unwrap(), worktrees.parse().unwrap());
        let fixture = support::born_repository();
        let enabled = support::enabled_repository(&fixture);
        populate(&fixture.root, items);
        let contexts: Vec<ItemId> = (0..worktrees).map(|_| ItemId::generate()).collect();
        for id in &contexts {
            write(&fixture.root, &format!("docs/contexts/{id}.md"), &document(id, "Context"));
        }
        commit_all(&fixture.repository);
        let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
        for id in &contexts {
            let branch = fixture.repository.branch(&format!("manyhands/document/{id}"), &head, false).unwrap();
            let path = fixture.root.join(".manyhands/worktrees").join(id.to_string());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut options = WorktreeAddOptions::new();
            options.reference(Some(branch.get()));
            fixture.repository.worktree(&id.to_string(), &path, Some(&options)).unwrap();
            for n in 0..CHANGES_PER_WORKTREE {
                let new = ItemId::generate();
                write(&path, &format!("docs/work/{new}.md"), &document(&new, &format!("W{n}")));
            }
        }
        let started = Instant::now();
        let outcome = enabled
            .service
            .refresh_repository(RefreshRepositoryRequest {
                root: fixture.root.clone(),
                operation_id: support::new_operation_id(),
            })
            .unwrap();
        let elapsed = started.elapsed();
        assert!(matches!(outcome, RefreshOutcome::Refreshed { .. }));
        println!("{items} items, {worktrees} worktrees: {:.3} s", elapsed.as_secs_f64());
    }
}
```

- [ ] **Step 2: Run it on main's code (the baseline)**

Run: `devenv shell -- cargo test --release --locked --test index_scale -- --ignored --nocapture`
Expected: one line per cell, and PASS. If disk is short, run a subset with
`MANYHANDS_SCALE_CELLS=1000x1,1000x10,10000x1,10000x10` and say which
cells were run.

- [ ] **Step 3: Record the baseline and commit**

Add a ticket comment with the machine, OS, commit and the timings. Then:

```bash
git add tests/index_scale.rs .manyhands/comments/01M5NKA3MY7CW4D90M866YS946/
git commit -m "test: time an index refresh at scale" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Changed paths

**Files:**
- Create: `src/repository/contributions.rs`
- Create: `src/repository/contributions_tests.rs`
- Modify: `src/repository.rs` (add `mod contributions;` beside `mod discovery;`)

**Interfaces:**
- Produces:
  - `pub(super) const MAX_CONTEXT_CHANGES: usize`
  - `pub(super) struct ChangedPath { pub(super) path: PathBuf, pub(super) base: Option<Vec<u8>>, pub(super) disk: Option<Vec<u8>> }`
  - `pub(super) enum ChangedPaths { Unavailable, Found { paths: Vec<ChangedPath>, over_budget: bool } }`
  - `pub(super) fn changed_paths(repository: &git2::Repository, worktree: &Path, primary_head: Option<git2::Oid>) -> ChangedPaths`
  - `pub(super) fn merged_into_primary(repository: &git2::Repository, primary_head: Option<git2::Oid>) -> bool`

- [ ] **Step 1: Write the failing tests**

`src/repository/contributions_tests.rs`:

```rust
use std::{fs, path::Path};

use git2::{Commit, IndexAddOption, Oid, Repository, Signature, Time, WorktreeAddOptions};

use super::*;

const BRANCH: &str = "manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FC0";

struct Fixture {
    _dir: tempfile::TempDir,
    primary: Repository,
    worktree: Repository,
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn signature() -> Signature<'static> {
    Signature::new("Test", "test@example.invalid", &Time::new(1_700_000_000, 0)).unwrap()
}

/// Commits the working tree as it is, deletions included, on HEAD, with
/// `extra` as further parents.
fn commit_all(repository: &Repository, extra: &[Oid]) -> Oid {
    let mut index = repository.index().unwrap();
    index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let mut parents: Vec<Commit> = repository.head().ok().and_then(|h| h.peel_to_commit().ok()).into_iter().collect();
    parents.extend(extra.iter().map(|oid| repository.find_commit(*oid).unwrap()));
    let parents: Vec<&Commit> = parents.iter().collect();
    repository.commit(Some("HEAD"), &signature(), &signature(), "commit", &tree, &parents).unwrap()
}

/// Primary with `docs/a.md` and `docs/b.md` committed, and a linked
/// worktree on `BRANCH` from that commit.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("primary");
    let primary = Repository::init(&root).unwrap();
    write(&root, "docs/a.md", "a\n");
    write(&root, "docs/b.md", "b\n");
    let first = commit_all(&primary, &[]);
    let branch = primary.branch(BRANCH, &primary.find_commit(first).unwrap(), false).unwrap();
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(branch.get()));
    let path = dir.path().join("worktree");
    primary.worktree("context", &path, Some(&options)).unwrap();
    let worktree = Repository::open(&path).unwrap();
    Fixture { _dir: dir, primary, worktree }
}

impl Fixture {
    fn primary_root(&self) -> &Path {
        self.primary.workdir().unwrap()
    }
    fn worktree_root(&self) -> &Path {
        self.worktree.workdir().unwrap()
    }
    fn primary_head(&self) -> Option<Oid> {
        self.primary.head().unwrap().target()
    }
    /// Each changed path, whether the merge base has it and whether the
    /// disk does.
    fn changed(&self) -> Vec<(String, bool, bool)> {
        match changed_paths(&self.worktree, self.worktree_root(), self.primary_head()) {
            ChangedPaths::Found { paths, over_budget: false } => paths
                .into_iter()
                .map(|c| (c.path.to_str().unwrap().to_owned(), c.base.is_some(), c.disk.is_some()))
                .collect(),
            other => panic!("unexpected {other:?}"),
        }
    }
}

fn change(path: &str, base: bool, disk: bool) -> (String, bool, bool) {
    (path.to_owned(), base, disk)
}

#[test]
fn a_branch_that_changed_nothing_has_no_changed_paths() {
    assert_eq!(fixture().changed(), []);
}

#[test]
fn committed_staged_unstaged_and_untracked_changes_are_found() {
    let f = fixture();
    let wt = f.worktree_root();
    write(wt, "docs/a.md", "a2\n");
    commit_all(&f.worktree, &[]);
    write(wt, "docs/c.md", "c\n");
    let mut index = f.worktree.index().unwrap();
    index.add_path(Path::new("docs/c.md")).unwrap();
    index.write().unwrap();
    write(wt, "docs/b.md", "b2\n");
    write(wt, "docs/new/d.md", "d\n");
    assert_eq!(
        f.changed(),
        [
            change("docs/a.md", true, true),
            change("docs/b.md", true, true),
            change("docs/c.md", false, true),
            change("docs/new/d.md", false, true),
        ]
    );
}

#[test]
fn a_change_put_back_on_disk_is_not_found() {
    let f = fixture();
    write(f.worktree_root(), "docs/a.md", "a2\n");
    commit_all(&f.worktree, &[]);
    write(f.worktree_root(), "docs/a.md", "a\n");
    assert_eq!(f.changed(), []);
}

#[test]
fn a_deleted_file_is_found_with_its_merge_base_bytes() {
    let f = fixture();
    fs::remove_file(f.worktree_root().join("docs/b.md")).unwrap();
    commit_all(&f.worktree, &[]);
    match changed_paths(&f.worktree, f.worktree_root(), f.primary_head()) {
        ChangedPaths::Found { paths, .. } => {
            assert_eq!(paths, [ChangedPath { path: "docs/b.md".into(), base: Some(b"b\n".to_vec()), disk: None }]);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn primary_moving_on_is_not_a_change_of_the_branch() {
    let f = fixture();
    write(f.primary_root(), "docs/a.md", "a2\n");
    commit_all(&f.primary, &[]);
    assert_eq!(f.changed(), []);
}

#[test]
fn merging_primary_moves_the_merge_base() {
    let f = fixture();
    let wt = f.worktree_root();
    write(wt, "docs/b.md", "b2\n");
    commit_all(&f.worktree, &[]);
    write(f.primary_root(), "docs/a.md", "a2\n");
    let main = commit_all(&f.primary, &[]);
    // The merge: primary's a.md and the branch's b.md, with both parents.
    write(wt, "docs/a.md", "a2\n");
    commit_all(&f.worktree, &[main]);
    assert_eq!(f.changed(), [change("docs/b.md", true, true)]);
}

#[test]
fn files_outside_the_managed_paths_are_not_found() {
    let f = fixture();
    let wt = f.worktree_root();
    write(wt, "src/notes.md", "x\n");
    write(wt, "docs/readme.txt", "x\n");
    write(wt, ".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FC0/notes.md", "x\n");
    write(wt, "docs/a\\b.md", "x\n");
    assert_eq!(f.changed(), []);
}

#[test]
fn ignored_files_are_not_found() {
    let f = fixture();
    write(f.worktree_root(), ".gitignore", "docs/scratch/\n");
    write(f.worktree_root(), "docs/scratch/x.md", "x\n");
    assert_eq!(f.changed(), []);
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_is_not_read() {
    let f = fixture();
    let wt = f.worktree_root();
    fs::remove_file(wt.join("docs/a.md")).unwrap();
    std::os::unix::fs::symlink("b.md", wt.join("docs/a.md")).unwrap();
    // Not read, so the item is gone on this branch: a deletion.
    assert_eq!(f.changed(), [change("docs/a.md", true, false)]);
}

#[test]
fn the_budget_stops_in_path_order() {
    let f = fixture();
    for n in 0..=MAX_CONTEXT_CHANGES {
        write(f.worktree_root(), &format!("docs/n/{n:04}.md"), "x\n");
    }
    match changed_paths(&f.worktree, f.worktree_root(), f.primary_head()) {
        ChangedPaths::Found { paths, over_budget } => {
            assert!(over_budget);
            assert_eq!(paths.len(), MAX_CONTEXT_CHANGES);
            assert_eq!(paths.last().unwrap().path, Path::new(&format!("docs/n/{:04}.md", MAX_CONTEXT_CHANGES - 1)));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn no_merge_base_is_unavailable() {
    let f = fixture();
    assert_eq!(changed_paths(&f.worktree, f.worktree_root(), None), ChangedPaths::Unavailable);
    let tree = f.primary.head().unwrap().peel_to_tree().unwrap();
    let orphan = f.primary.commit(None, &signature(), &signature(), "orphan", &tree, &[]).unwrap();
    assert_eq!(changed_paths(&f.worktree, f.worktree_root(), Some(orphan)), ChangedPaths::Unavailable);
}

#[test]
fn merged_into_primary_follows_ancestry() {
    let f = fixture();
    assert!(merged_into_primary(&f.worktree, f.primary_head()));
    write(f.worktree_root(), "docs/a.md", "a2\n");
    let branch_head = commit_all(&f.worktree, &[]);
    assert!(!merged_into_primary(&f.worktree, f.primary_head()));
    // Primary fast-forwards to the branch.
    let name = f.primary.head().unwrap().name().unwrap().to_owned();
    f.primary.reference(&name, branch_head, true, "fast-forward").unwrap();
    assert!(merged_into_primary(&f.worktree, f.primary_head()));
    assert!(!merged_into_primary(&f.worktree, None));
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `devenv shell -- cargo test --locked --lib contributions`
Expected: compile errors: `changed_paths`, `ChangedPaths`, `ChangedPath`,
`MAX_CONTEXT_CHANGES` and `merged_into_primary` are not defined.

- [ ] **Step 3: Write the module**

`src/repository/contributions.rs`:

```rust
//! What an authoring context's branch changed among the managed files.
//!
//! A worktree is a full checkout, so most of what it holds is a copy of
//! primary as of the branch point. A path is the branch's own work only when
//! its bytes on disk differ from the merge base of the context's HEAD and
//! primary's HEAD, committed or not. Everything else is primary's to report.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use git2::{DiffOptions, ObjectType, Oid, Repository, StatusOptions, Tree};

/// The most changed paths one context contributes. Past it the context
/// contributes the first ones in path order and says that it stopped.
pub(super) const MAX_CONTEXT_CHANGES: usize = 1024;

/// The managed path prefixes, as Git pathspecs.
const MANAGED_PATHSPECS: [&str; 3] = ["docs", ".manyhands/tickets", ".manyhands/comments"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ChangedPath {
    /// Relative to the worktree, with forward slashes.
    pub(super) path: PathBuf,
    /// The merge base's blob at `path`, or `None` when it has none there.
    pub(super) base: Option<Vec<u8>>,
    /// The regular file at `path` now, or `None` when there is none.
    pub(super) disk: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ChangedPaths {
    /// Either side has no commit, there is no merge base, or Git could not
    /// list the changes.
    Unavailable,
    Found {
        /// In path order, at most `MAX_CONTEXT_CHANGES`.
        paths: Vec<ChangedPath>,
        /// More paths changed than were taken.
        over_budget: bool,
    },
}

pub(super) fn changed_paths(
    repository: &Repository,
    worktree: &Path,
    primary_head: Option<Oid>,
) -> ChangedPaths {
    let Some(base) = merge_base_tree(repository, primary_head) else {
        return ChangedPaths::Unavailable;
    };
    let Some(candidates) = candidates(repository, &base) else {
        return ChangedPaths::Unavailable;
    };
    let mut paths = Vec::new();
    for path in candidates {
        if !is_managed_markdown(&path) {
            continue;
        }
        let base_bytes = blob_at(repository, &base, &path);
        let disk = regular_file(&worktree.join(&path));
        if base_bytes == disk {
            continue;
        }
        if paths.len() == MAX_CONTEXT_CHANGES {
            return ChangedPaths::Found { paths, over_budget: true };
        }
        paths.push(ChangedPath { path, base: base_bytes, disk });
    }
    ChangedPaths::Found { paths, over_budget: false }
}

/// Whether the context's HEAD is primary's HEAD or one of its ancestors:
/// its branch has nothing primary lacks.
pub(super) fn merged_into_primary(repository: &Repository, primary_head: Option<Oid>) -> bool {
    let (Some(primary), Some(head)) = (primary_head, head_oid(repository)) else {
        return false;
    };
    head == primary || repository.graph_descendant_of(primary, head).unwrap_or(false)
}

fn head_oid(repository: &Repository) -> Option<Oid> {
    repository.head().ok()?.target()
}

fn merge_base_tree(repository: &Repository, primary_head: Option<Oid>) -> Option<Tree<'_>> {
    let base = repository.merge_base(head_oid(repository)?, primary_head?).ok()?;
    repository.find_commit(base).ok()?.tree().ok()
}

/// Every path the branch's commits since the merge base touched, and every
/// path that differs from HEAD in the index or on disk, untracked ones
/// included and ignored ones not. A path may be listed though its bytes
/// match the merge base; the caller compares.
fn candidates(repository: &Repository, base: &Tree<'_>) -> Option<BTreeSet<PathBuf>> {
    let mut paths = BTreeSet::new();
    let head = repository.head().ok()?.peel_to_tree().ok()?;
    let mut diff_options = DiffOptions::new();
    for spec in MANAGED_PATHSPECS {
        diff_options.pathspec(spec);
    }
    let diff = repository
        .diff_tree_to_tree(Some(base), Some(&head), Some(&mut diff_options))
        .ok()?;
    for delta in diff.deltas() {
        for file in [delta.old_file(), delta.new_file()] {
            paths.extend(file.path().map(Path::to_owned));
        }
    }
    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    for spec in MANAGED_PATHSPECS {
        status_options.pathspec(spec);
    }
    let statuses = repository.statuses(Some(&mut status_options)).ok()?;
    for entry in statuses.iter() {
        // The index stores paths as text, so one that is not is skipped.
        if let Ok(path) = std::str::from_utf8(entry.path_bytes()) {
            paths.insert(PathBuf::from(path));
        }
    }
    Some(paths)
}

/// `docs/**/*.md`, `.manyhands/tickets/<id>/ticket.md` and
/// `.manyhands/comments/**/*.md`, written as text with no backslash and no
/// `.git` component: the paths primary's discovery reads as items.
fn is_managed_markdown(path: &Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    if text.contains('\\') || path.components().any(|part| part.as_os_str() == ".git") {
        return false;
    }
    let markdown = path.extension().is_some_and(|extension| extension == "md");
    (text.starts_with("docs/") && markdown)
        || (text.starts_with(".manyhands/tickets/")
            && text.ends_with("/ticket.md")
            && path.components().count() == 4)
        || (text.starts_with(".manyhands/comments/") && markdown)
}

fn blob_at(repository: &Repository, tree: &Tree<'_>, path: &Path) -> Option<Vec<u8>> {
    let entry = tree.get_path(path).ok()?;
    if entry.kind() != Some(ObjectType::Blob) || !matches!(entry.filemode(), 0o100644 | 0o100755) {
        return None;
    }
    Some(repository.find_blob(entry.id()).ok()?.content().to_vec())
}

/// The bytes of a regular file. A symbolic link is never followed.
fn regular_file(path: &Path) -> Option<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    fs::read(path).ok()
}

#[cfg(test)]
#[path = "contributions_tests.rs"]
mod tests;
```

Add `mod contributions;` next to `mod discovery;` in `src/repository.rs`.
Until Task 4 uses them, the items are dead code. Add
`#![allow(dead_code)]` at the top of `contributions.rs` with the comment
`// Used by discovery from Task 4.`, and remove it in Task 4.

- [ ] **Step 4: Run the tests to see them pass**

Run: `devenv shell -- cargo test --locked --lib contributions`
Expected: PASS, 12 tests (11 on Windows, without the symbolic link test).

- [ ] **Step 5: Commit**

```bash
git add src/repository/contributions.rs src/repository/contributions_tests.rs src/repository.rs
git commit -m "feat: find the managed paths a context's branch changed" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: What a context contributes

**Files:**
- Modify: `src/repository/contributions.rs`
- Modify: `src/repository/contributions_tests.rs`
- Modify: `src/repository/discovery.rs` (make `canonical_item_id`
  `pub(super)`)

**Interfaces:**
- Consumes: `ChangedPath` (Task 2); `discovery::canonical_item_id(&CanonicalItem) -> &ItemId`.
- Produces:
  - `pub(super) enum PrimaryState { Absent, Identical, Different }` with `pub(super) fn stored(self) -> &'static str`
  - `pub(super) struct PrimaryComparison { pub(super) state: PrimaryState, pub(super) compared_head: Option<Oid>, pub(super) path: Option<PathBuf> }`
  - `pub(super) enum ContributedChange { Added, Modified, Moved }` with `stored()`
  - `pub(super) struct ObservedDeletion { pub(super) id: ItemId, pub(super) kind: canonical::ItemKind, pub(super) path: PathBuf }`
  - `pub(super) struct PrimaryView<'a> { pub(super) head: Option<Oid>, .. }` with `new(head, &'a [(PathBuf, String)])`, `source(&self, &Path) -> Option<&'a str>` and `compare(&self, &ItemId, &str) -> PrimaryComparison`
  - `pub(super) struct Contribution { pub(super) sources, pub(super) changes: BTreeMap<PathBuf, ContributedChange>, pub(super) deletions: Vec<ObservedDeletion>, pub(super) duplicates: Vec<canonical::ValidationProblem>, pub(super) unreadable: Vec<PathBuf> }`
  - `pub(super) fn contribute(own_sources: &[(PathBuf, String)], changed: &[ChangedPath], primary: &PrimaryView<'_>) -> Contribution`

- [ ] **Step 1: Write the failing tests**

Append to `src/repository/contributions_tests.rs`:

```rust
use crate::canonical::{ItemId, ItemKind, ValidationCode};

const A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
const OWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC0";
const C1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
const C2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";

fn doc(id: &str, title: &str) -> String {
    format!("---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: {title}\n---\n# {title}\n")
}

fn comment(id: &str, item: &str, parent: Option<&str>) -> String {
    let parent = parent.map(|p| format!("parent_id: {p}\n")).unwrap_or_default();
    format!("---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: {id}\nitem_id: {item}\n{parent}created_at: 2026-10-23T00:00:00Z\n---\nText.\n")
}

fn own_ticket() -> (PathBuf, String) {
    (
        format!(".manyhands/tickets/{OWN}/ticket.md").into(),
        format!("---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {OWN}\ntitle: Own\ntype: task\nstatus: open\n---\nBody.\n"),
    )
}

fn primary_sources() -> Vec<(PathBuf, String)> {
    vec![
        ("docs/a.md".into(), doc(A, "A")),
        ("docs/b.md".into(), doc(B, "B")),
        (format!(".manyhands/comments/{A}/{C1}.md").into(), comment(C1, A, None)),
    ]
}

fn changed(path: &str, base: Option<&str>, disk: Option<&str>) -> ChangedPath {
    ChangedPath {
        path: path.into(),
        base: base.map(|text| text.as_bytes().to_vec()),
        disk: disk.map(|text| text.as_bytes().to_vec()),
    }
}

fn paths(sources: &[(PathBuf, String)]) -> Vec<&str> {
    sources.iter().map(|(path, _)| path.to_str().unwrap()).collect()
}

#[test]
fn an_edit_of_a_primary_document_is_modified() {
    let primary = primary_sources();
    let view = PrimaryView::new(None, &primary);
    let result = contribute(&[own_ticket()], &[changed("docs/a.md", Some(&doc(A, "A")), Some(&doc(A, "A2")))], &view);
    assert_eq!(result.changes.get(Path::new("docs/a.md")), Some(&ContributedChange::Modified));
    assert!(result.duplicates.is_empty());
    assert_eq!(paths(&result.sources), [&format!(".manyhands/tickets/{OWN}/ticket.md")[..], "docs/a.md"]);
}

#[test]
fn a_new_document_is_added() {
    let primary = primary_sources();
    let view = PrimaryView::new(None, &primary);
    let result = contribute(&[own_ticket()], &[changed("docs/new.md", None, Some(&doc(NEW, "New")))], &view);
    assert_eq!(result.changes.get(Path::new("docs/new.md")), Some(&ContributedChange::Added));
    assert!(result.duplicates.is_empty());
}

#[test]
fn a_document_the_branch_moved_is_moved_and_its_old_path_deleted() {
    let primary = primary_sources();
    let view = PrimaryView::new(None, &primary);
    let result = contribute(
        &[own_ticket()],
        &[changed("docs/a.md", Some(&doc(A, "A")), None), changed("docs/x/a.md", None, Some(&doc(A, "A")))],
        &view,
    );
    assert_eq!(result.changes.get(Path::new("docs/x/a.md")), Some(&ContributedChange::Moved));
    assert_eq!(result.deletions, [ObservedDeletion { id: A.parse().unwrap(), kind: ItemKind::Document, path: "docs/a.md".into() }]);
    assert!(result.duplicates.is_empty());
}

#[test]
fn an_id_the_branch_introduced_that_primary_has_is_a_duplicate() {
    let primary = primary_sources();
    let view = PrimaryView::new(None, &primary);
    let result = contribute(&[own_ticket()], &[changed("docs/z.md", None, Some(&doc(A, "Copy")))], &view);
    assert_eq!(result.changes.get(Path::new("docs/z.md")), Some(&ContributedChange::Added));
    assert_eq!(result.duplicates.len(), 1);
    assert_eq!(result.duplicates[0].path, Path::new("docs/z.md"));
    assert_eq!(result.duplicates[0].code, ValidationCode::DuplicateId);
    assert!(result.duplicates[0].message.contains("docs/a.md"));
}

#[test]
fn a_copy_identical_to_primary_is_not_contributed() {
    let primary = primary_sources();
    let view = PrimaryView::new(None, &primary);
    // Changed against the merge base, but primary has since made the same change.
    let result = contribute(&[own_ticket()], &[changed("docs/b.md", Some("old\n"), Some(&doc(B, "B")))], &view);
    assert!(result.changes.is_empty());
}

#[test]
fn a_changed_comment_is_validated_with_its_item_and_that_items_comments_from_primary() {
    let primary = primary_sources();
    let view = PrimaryView::new(None, &primary);
    let reply = format!(".manyhands/comments/{A}/{C2}.md");
    let result = contribute(&[own_ticket()], &[changed(&reply, None, Some(&comment(C2, A, Some(C1))))], &view);
    assert_eq!(
        paths(&result.sources),
        [&format!(".manyhands/comments/{A}/{C1}.md")[..], &reply, &format!(".manyhands/tickets/{OWN}/ticket.md"), "docs/a.md"]
    );
    assert_eq!(result.changes.len(), 1);
    let validation = crate::canonical::validate_context(result.sources);
    assert!(validation.problems.is_empty(), "{:?}", validation.problems);
}

#[test]
fn a_changed_file_that_is_not_utf8_is_unreadable() {
    let primary = primary_sources();
    let view = PrimaryView::new(None, &primary);
    let bad = ChangedPath { path: "docs/bad.md".into(), base: None, disk: Some(vec![0xff, 0xfe]) };
    let result = contribute(&[own_ticket()], &[bad], &view);
    assert_eq!(result.unreadable, [PathBuf::from("docs/bad.md")]);
    assert!(result.changes.is_empty());
}

#[test]
fn the_own_items_paths_are_left_to_the_own_item() {
    let primary = primary_sources();
    let view = PrimaryView::new(None, &primary);
    let own = own_ticket();
    let result = contribute(&[own.clone()], &[changed(own.0.to_str().unwrap(), None, Some(&own.1))], &view);
    assert!(result.changes.is_empty());
    assert_eq!(result.sources, [own]);
}

#[test]
fn the_primary_comparison_says_absent_identical_or_different() {
    let primary = primary_sources();
    let head = Some(Oid::from_str("0123456789abcdef0123456789abcdef01234567").unwrap());
    let view = PrimaryView::new(head, &primary);
    let a: ItemId = A.parse().unwrap();
    let identical = view.compare(&a, &doc(A, "A"));
    assert_eq!((identical.state, identical.compared_head, identical.path.as_deref()), (PrimaryState::Identical, head, Some(Path::new("docs/a.md"))));
    assert_eq!(view.compare(&a, &doc(A, "A2")).state, PrimaryState::Different);
    let new = view.compare(&NEW.parse().unwrap(), &doc(NEW, "New"));
    assert_eq!((new.state, new.path), (PrimaryState::Absent, None));
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `devenv shell -- cargo test --locked --lib contributions`
Expected: compile errors for `PrimaryView`, `contribute`,
`ContributedChange`, `ObservedDeletion`, `PrimaryState`.

- [ ] **Step 3: Write the code**

In `src/repository/discovery.rs`, change `fn canonical_item_id` to
`pub(super) fn canonical_item_id`.

Append to `src/repository/contributions.rs`, above the test module, and
extend its `use` lines with
`std::collections::BTreeMap`, `crate::canonical::{self, CanonicalItem, ItemId, ValidationCode, ValidationProblem}`
and `super::discovery::canonical_item_id`:

```rust
/// How an item's copy in a worktree compares with primary's copy of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrimaryState {
    Absent,
    Identical,
    Different,
}

impl PrimaryState {
    pub(super) fn stored(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Identical => "identical",
            Self::Different => "different",
        }
    }
}

/// The comparison, the primary HEAD it was made against, and where primary
/// has the item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PrimaryComparison {
    pub(super) state: PrimaryState,
    pub(super) compared_head: Option<Oid>,
    pub(super) path: Option<PathBuf>,
}

/// What the branch did to a contributed item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ContributedChange {
    /// The ID is new on the branch, or the branch also introduced an ID
    /// another copy has; `Contribution::duplicates` tells them apart.
    Added,
    /// The merge base had the item at this path.
    Modified,
    /// The branch deleted the path the merge base had the item at.
    Moved,
}

impl ContributedChange {
    pub(super) fn stored(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Moved => "moved",
        }
    }
}

/// A path whose item the merge base had and the branch deleted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ObservedDeletion {
    pub(super) id: ItemId,
    pub(super) kind: canonical::ItemKind,
    pub(super) path: PathBuf,
}

/// Primary's sources as this refresh read them, with where each ID is.
/// Built once per refresh and shared by every context.
pub(super) struct PrimaryView<'a> {
    pub(super) head: Option<Oid>,
    sources: BTreeMap<&'a Path, &'a str>,
    paths: BTreeMap<ItemId, &'a Path>,
    comments: BTreeMap<ItemId, Vec<&'a Path>>,
}

impl<'a> PrimaryView<'a> {
    pub(super) fn new(head: Option<Oid>, sources: &'a [(PathBuf, String)]) -> Self {
        let mut view = Self {
            head,
            sources: BTreeMap::new(),
            paths: BTreeMap::new(),
            comments: BTreeMap::new(),
        };
        for (path, source) in sources {
            view.sources.insert(path, source);
            let Ok(item) = canonical::parse_item(path, source) else {
                continue;
            };
            if let CanonicalItem::Comment(comment) = &item {
                view.comments.entry(comment.item_id.clone()).or_default().push(path);
            }
            // Of two copies of an ID, the first in path order is kept;
            // primary's validation reports the duplicate.
            view.paths.entry(canonical_item_id(&item).clone()).or_insert(path);
        }
        view
    }

    pub(super) fn source(&self, path: &Path) -> Option<&'a str> {
        self.sources.get(path).copied()
    }

    pub(super) fn compare(&self, id: &ItemId, source: &str) -> PrimaryComparison {
        let path = self.paths.get(id).copied();
        let state = match path.and_then(|path| self.source(path)) {
            None => PrimaryState::Absent,
            Some(primary) if primary == source => PrimaryState::Identical,
            Some(_) => PrimaryState::Different,
        };
        PrimaryComparison {
            state,
            compared_head: self.head,
            path: path.map(Path::to_owned),
        }
    }
}

/// What one context contributes, before validation.
#[derive(Debug, Default)]
pub(super) struct Contribution {
    /// What to validate, in path order: the own item's sources, the changed
    /// files that are text, and the primary sources a changed comment needs.
    pub(super) sources: Vec<(PathBuf, String)>,
    /// Each changed file that is text and not identical to primary's copy
    /// at that path, with what the branch did to its item.
    pub(super) changes: BTreeMap<PathBuf, ContributedChange>,
    pub(super) deletions: Vec<ObservedDeletion>,
    /// A `duplicate_id` problem for each ID the branch introduced that
    /// primary also has.
    pub(super) duplicates: Vec<ValidationProblem>,
    /// Changed files that are not UTF-8.
    pub(super) unreadable: Vec<PathBuf>,
}

pub(super) fn contribute(
    own_sources: &[(PathBuf, String)],
    changed: &[ChangedPath],
    primary: &PrimaryView<'_>,
) -> Contribution {
    let own_paths: BTreeSet<&Path> = own_sources.iter().map(|(path, _)| path.as_path()).collect();
    let own_ids: BTreeSet<ItemId> = own_sources
        .iter()
        .filter_map(|(path, source)| canonical::parse_item(path, source).ok())
        .map(|item| canonical_item_id(&item).clone())
        .collect();
    let theirs = changed.iter().filter(|change| !own_paths.contains(change.path.as_path()));
    let mut contribution = Contribution {
        sources: own_sources.to_vec(),
        ..Contribution::default()
    };

    let mut deleted_ids = BTreeSet::new();
    for change in theirs.clone().filter(|change| change.disk.is_none()) {
        let Some(item) = parsed(&change.path, change.base.as_deref()) else {
            continue;
        };
        let id = canonical_item_id(&item).clone();
        deleted_ids.insert(id.clone());
        contribution.deletions.push(ObservedDeletion {
            id,
            kind: item_kind(&item),
            path: change.path.clone(),
        });
    }

    let mut branch_ids = BTreeSet::new();
    let mut commented_items = BTreeSet::new();
    for change in theirs.clone() {
        let Some(bytes) = &change.disk else {
            continue;
        };
        let Ok(source) = String::from_utf8(bytes.clone()) else {
            contribution.unreadable.push(change.path.clone());
            continue;
        };
        if primary.source(&change.path) == Some(source.as_str()) {
            continue;
        }
        let kind = match canonical::parse_item(&change.path, &source) {
            Ok(item) => {
                let id = canonical_item_id(&item).clone();
                if let CanonicalItem::Comment(comment) = &item {
                    commented_items.insert(comment.item_id.clone());
                }
                let base_id = parsed(&change.path, change.base.as_deref())
                    .map(|base| canonical_item_id(&base).clone());
                let kind = if base_id.as_ref() == Some(&id) {
                    ContributedChange::Modified
                } else if deleted_ids.contains(&id) {
                    ContributedChange::Moved
                } else {
                    if let Some(at) = primary.paths.get(&id) {
                        contribution.duplicates.push(ValidationProblem {
                            path: change.path.clone(),
                            code: ValidationCode::DuplicateId,
                            message: format!(
                                "the branch gives this file an ID primary already has, at {}",
                                at.to_string_lossy()
                            ),
                        });
                    }
                    ContributedChange::Added
                };
                branch_ids.insert(id);
                kind
            }
            // Not an item. Validation says why; the file is still the
            // branch's work.
            Err(_) => ContributedChange::Added,
        };
        contribution.changes.insert(change.path.clone(), kind);
        contribution.sources.push((change.path.clone(), source));
    }

    // A changed comment is validated with its item and that item's other
    // comments as primary has them, unless the branch has its own copy.
    let replaced: BTreeSet<&Path> = changed.iter().map(|change| change.path.as_path()).collect();
    let needed = commented_items
        .iter()
        .filter(|item| !own_ids.contains(*item) && !branch_ids.contains(*item))
        .flat_map(|item| {
            primary.paths.get(item).into_iter().copied().chain(
                primary.comments.get(item).into_iter().flatten().copied(),
            )
        })
        .collect::<BTreeSet<_>>();
    for path in needed {
        if replaced.contains(path) || own_paths.contains(path) {
            continue;
        }
        let Some(source) = primary.source(path) else {
            continue;
        };
        if canonical::parse_item(path, source)
            .is_ok_and(|item| branch_ids.contains(canonical_item_id(&item)))
        {
            continue;
        }
        contribution.sources.push((path.to_owned(), source.to_owned()));
    }
    contribution.sources.sort_by(|left, right| left.0.cmp(&right.0));
    contribution
}

fn parsed(path: &Path, bytes: Option<&[u8]>) -> Option<CanonicalItem> {
    let source = std::str::from_utf8(bytes?).ok()?;
    canonical::parse_item(path, source).ok()
}

fn item_kind(item: &CanonicalItem) -> canonical::ItemKind {
    match item {
        CanonicalItem::Document(_) => canonical::ItemKind::Document,
        CanonicalItem::Ticket(_) => canonical::ItemKind::Ticket,
        CanonicalItem::Comment(_) => canonical::ItemKind::Comment,
    }
}
```

If `canonical::Comment`'s field for its item is not `item_id`, use the field
`validate_context` reads at `src/canonical.rs:586`.

- [ ] **Step 4: Run the tests to see them pass**

Run: `devenv shell -- cargo test --locked --lib contributions`
Expected: PASS, 21 tests (20 on Windows).

- [ ] **Step 5: Commit**

```bash
git add src/repository/contributions.rs src/repository/contributions_tests.rs src/repository/discovery.rs
git commit -m "feat: classify what a context's changed files contribute" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Discovery stores what each context contributes

**Files:**
- Modify: `src/repository/discovery.rs`: types, `observe_root`,
  `observe_active_contexts`, `observe_items`, `observe_comment_thread`,
  `ITEM_READ_COLUMNS`, `ITEM_READ_OBJECTS`
- Modify: `src/repository.rs`: `PersistedContext`, `persist_context`,
  `persist_context_refresh` and its two callers, the rebuild loop
  (`for active in &observation.active_contexts`), `persist_comments`,
  `changed_contexts`, the snapshot item query
- Modify: `src/results.rs`: two problem codes
- Modify: `schemas/v1/problem.schema.json`, `schemas/v1/index_problem.schema.json`
- Modify: `src/repository/contributions.rs`: remove `#![allow(dead_code)]`
- Modify: `tests/support/items.rs`: `commit_in`
- Create: `tests/context_contributions.rs`
- Modify: `.github/workflows/build.yml`: add `--test context_contributions`

**Interfaces:**
- Consumes: everything Tasks 2 and 3 produce.
- Produces, in `discovery`:
  - `pub(super) enum ItemRole { Own, Changed(ContributedChange), Comments }` with `pub(super) fn stored(self) -> (&'static str, Option<&'static str>)` returning (role, change)
  - `ObservedItem.role: ItemRole`, `ObservedItem.primary: Option<PrimaryComparison>`
  - `ObservedCommentThread.contributed: bool`
  - `ActiveContextObservation.deletions: Vec<ObservedDeletion>`, `.folders: Vec<ObservedFolder>`, `.merged_into_primary: bool`
  - `RootObservationProblem::Changes { path: PathBuf, code: ProblemCode, message: String }`
- Produces, in the index:
  - columns `discovered_items.role|change|primary_state|primary_compared_oid|primary_path`, `discovered_comments.contributed`, `contexts.merged_into_primary`;
  - table `discovered_deletions`.

- [ ] **Step 1: Add the test helper**

In `tests/support/items.rs`, after `commit`:

```rust
/// Commits `paths` in the worktree at `worktree` as they are on disk, at
/// `seconds`. A path missing from disk is committed as deleted.
pub fn commit_in(worktree: &Path, paths: &[&str], seconds: i64) {
    let repository = git2::Repository::open(worktree).unwrap();
    let mut index = repository.index().unwrap();
    index.read(true).unwrap();
    for path in paths {
        if worktree.join(path).exists() {
            index.add_path(Path::new(path)).unwrap();
        } else {
            index.remove_path(Path::new(path)).unwrap();
        }
    }
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repository.head().unwrap().peel_to_commit().unwrap();
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(seconds, 0),
    )
    .unwrap();
    repository
        .commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &[&parent])
        .unwrap();
}
```

- [ ] **Step 2: Write the failing tests**

`tests/context_contributions.rs`:

```rust
//! A ticket or document worktree contributes every item its branch
//! changed, and the reads say where each copy lives.

use std::{fs, path::Path};

use manyhands::{canonical::ItemId, repository::RefreshOutcome};
use rusqlite::params;
use support::items::{
    DOCUMENT_A, DOCUMENT_C, commit, commit_in, comment_path, comment_source, context_worktree,
    create_document_context, document_source, index, refresh, refresh_completely, write,
};

mod support;

const NEW: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD9";
const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE5";
const CREATED: &str = "2026-10-23T00:00:00Z";

/// Primary has `docs/shared.md` (`DOCUMENT_C`) committed. `DOCUMENT_A` has
/// a worktree, holding its own draft at `docs/first.md`.
fn repository() -> (support::TestRepository, support::EnabledRepository, std::path::PathBuf) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    write(&fixture.root, "docs/shared.md", &document_source(DOCUMENT_C, "Shared", ""));
    commit(&fixture, &["docs/shared.md"], 1_000);
    create_document_context(&enabled.service, &fixture.root, DOCUMENT_A, "docs/first.md");
    let worktree = context_worktree(&fixture.root, DOCUMENT_A);
    (fixture, enabled, worktree)
}

/// (worktree, item ID, path, role, change) of every item row.
fn rows(data: &Path) -> Vec<(String, String, String, String, Option<String>)> {
    let connection = index(data);
    let mut statement = connection
        .prepare(
            "SELECT contexts.worktree_path, items.item_id, items.canonical_path, items.role, items.change
               FROM discovered_items AS items JOIN contexts ON contexts.id = items.context_id
              ORDER BY 1, 2, 3",
        )
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn row(worktree: &Path, id: &str, path: &str, role: &str, change: Option<&str>) -> (String, String, String, String, Option<String>) {
    (worktree.to_str().unwrap().to_owned(), id.to_owned(), path.to_owned(), role.to_owned(), change.map(str::to_owned))
}

fn worktree_rows(data: &Path, worktree: &Path) -> Vec<(String, String, String, String, Option<String>)> {
    rows(data).into_iter().filter(|r| r.0 == worktree.to_str().unwrap()).collect()
}

/// (code, path) of the problems stored for the context at `worktree`.
fn problems(data: &Path, worktree: &Path) -> Vec<(String, Option<String>)> {
    let connection = index(data);
    let mut statement = connection
        .prepare(
            "SELECT problems.code, problems.path FROM problems JOIN contexts ON contexts.id = problems.context_id
              WHERE contexts.worktree_path = ?1 ORDER BY 1, 2",
        )
        .unwrap();
    statement
        .query_map(params![worktree.to_str().unwrap()], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn a_branch_that_changed_nothing_contributes_only_its_own_item() {
    let (fixture, enabled, worktree) = repository();
    refresh_completely(&enabled.service, &fixture.root);
    assert_eq!(
        worktree_rows(enabled.data_directory.path(), &worktree),
        [row(&worktree, DOCUMENT_A, "docs/first.md", "own", None)]
    );
}

#[test]
fn a_document_added_on_a_branch_is_contributed_uncommitted_or_committed() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/plans/new.md", &document_source(NEW, "New", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let added = row(&worktree, NEW, "docs/plans/new.md", "changed", Some("added"));
    assert!(worktree_rows(enabled.data_directory.path(), &worktree).contains(&added));
    commit_in(&worktree, &["docs/plans/new.md"], 2_000);
    refresh_completely(&enabled.service, &fixture.root);
    assert!(worktree_rows(enabled.data_directory.path(), &worktree).contains(&added));
}

#[test]
fn an_edit_of_a_primary_document_is_contributed_as_modified() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/shared.md", &document_source(DOCUMENT_C, "Shared, edited", ""));
    refresh_completely(&enabled.service, &fixture.root);
    assert!(worktree_rows(enabled.data_directory.path(), &worktree)
        .contains(&row(&worktree, DOCUMENT_C, "docs/shared.md", "changed", Some("modified"))));
}

#[test]
fn a_document_the_branch_moved_from_the_merge_base_is_moved_and_its_old_path_deleted() {
    let (fixture, enabled, worktree) = repository();
    fs::remove_file(worktree.join("docs/shared.md")).unwrap();
    write(&worktree, "docs/archive/shared.md", &document_source(DOCUMENT_C, "Shared", ""));
    refresh_completely(&enabled.service, &fixture.root);
    assert!(worktree_rows(enabled.data_directory.path(), &worktree)
        .contains(&row(&worktree, DOCUMENT_C, "docs/archive/shared.md", "changed", Some("moved"))));
    let connection = index(enabled.data_directory.path());
    let deleted: Vec<(String, String, String)> = connection
        .prepare("SELECT item_id, kind, canonical_path FROM discovered_deletions")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(deleted, [(DOCUMENT_C.to_owned(), "document".to_owned(), "docs/shared.md".to_owned())]);
}

#[test]
fn a_copy_primary_already_has_or_that_was_put_back_is_not_contributed() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/shared.md", &document_source(DOCUMENT_C, "Shared, edited", ""));
    commit_in(&worktree, &["docs/shared.md"], 2_000);
    write(&worktree, "docs/shared.md", &document_source(DOCUMENT_C, "Shared", ""));
    refresh_completely(&enabled.service, &fixture.root);
    assert!(!worktree_rows(enabled.data_directory.path(), &worktree).iter().any(|r| r.1 == DOCUMENT_C));
}

#[test]
fn merging_main_stops_contributing_what_main_changed() {
    let (fixture, enabled, worktree) = repository();
    write(&fixture.root, "docs/shared.md", &document_source(DOCUMENT_C, "Shared v2", ""));
    commit(&fixture, &["docs/shared.md"], 3_000);
    refresh_completely(&enabled.service, &fixture.root);
    // The worktree still has v1: the branch did not change it.
    assert!(!worktree_rows(enabled.data_directory.path(), &worktree).iter().any(|r| r.1 == DOCUMENT_C));
}

#[test]
fn a_comment_on_an_item_the_branch_did_not_change_is_held_by_a_comments_row() {
    let (fixture, enabled, worktree) = repository();
    let path = comment_path(DOCUMENT_C, COMMENT);
    write(&worktree, &path, &comment_source(DOCUMENT_C, COMMENT, None, CREATED, ""));
    refresh_completely(&enabled.service, &fixture.root);
    assert!(worktree_rows(enabled.data_directory.path(), &worktree)
        .contains(&row(&worktree, DOCUMENT_C, "docs/shared.md", "comments", None)));
    let connection = index(enabled.data_directory.path());
    let stored: Vec<(String, bool)> = connection
        .prepare(
            "SELECT comments.comment_id, comments.contributed FROM discovered_comments AS comments
               JOIN discovered_items AS items ON items.id = comments.item_id
              WHERE items.role = 'comments'",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(stored, [(COMMENT.to_owned(), true)]);
}

#[test]
fn an_invalid_contributed_file_is_a_problem_on_its_context_which_stays_active() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/broken.md", "no front matter\n");
    refresh_completely(&enabled.service, &fixture.root);
    assert!(problems(enabled.data_directory.path(), &worktree)
        .contains(&("missing-front-matter".to_owned(), Some("docs/broken.md".to_owned()))));
    assert!(worktree_rows(enabled.data_directory.path(), &worktree).iter().any(|r| r.1 == DOCUMENT_A));
}

#[test]
fn a_changed_file_that_is_not_utf8_is_a_problem_on_its_context() {
    let (fixture, enabled, worktree) = repository();
    fs::write(worktree.join("docs/binary.md"), [0xff, 0xfe, 0x00]).unwrap();
    refresh_completely(&enabled.service, &fixture.root);
    assert!(problems(enabled.data_directory.path(), &worktree)
        .contains(&("source".to_owned(), Some("docs/binary.md".to_owned()))));
}

#[test]
fn an_id_the_branch_introduced_that_primary_has_is_a_duplicate_problem() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/copy.md", &document_source(DOCUMENT_C, "Copy", ""));
    refresh_completely(&enabled.service, &fixture.root);
    assert!(problems(enabled.data_directory.path(), &worktree)
        .contains(&("duplicate-id".to_owned(), Some("docs/copy.md".to_owned()))));
    assert!(worktree_rows(enabled.data_directory.path(), &worktree)
        .contains(&row(&worktree, DOCUMENT_C, "docs/copy.md", "changed", Some("added"))));
}

#[test]
fn the_changed_file_budget_is_a_problem_and_stops_in_path_order() {
    let (fixture, enabled, worktree) = repository();
    for n in 0..1_030 {
        write(&worktree, &format!("docs/many/{n:04}.md"), &document_source(&ItemId::generate().to_string(), "Many", ""));
    }
    refresh_completely(&enabled.service, &fixture.root);
    assert!(problems(enabled.data_directory.path(), &worktree)
        .iter()
        .any(|(code, _)| code == "context-changes-over-budget"));
    let many = worktree_rows(enabled.data_directory.path(), &worktree)
        .into_iter()
        .filter(|r| r.2.starts_with("docs/many/"))
        .count();
    assert!(many <= 1_024);
}

#[test]
fn a_merged_worktree_contributes_only_uncommitted_work() {
    let (fixture, enabled, worktree) = repository();
    // Primary fast-forwards to the worktree's branch: the branch is merged.
    let branch = git2::Repository::open(&worktree).unwrap().head().unwrap().target().unwrap();
    let name = fixture.repository.head().unwrap().name().unwrap().to_owned();
    fixture.repository.reference(&name, branch, true, "merge").unwrap();
    fixture.repository.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();
    write(&worktree, "docs/after.md", &document_source(NEW, "After merge", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let stored = worktree_rows(enabled.data_directory.path(), &worktree);
    assert!(stored.contains(&row(&worktree, NEW, "docs/after.md", "changed", Some("added"))));
    assert!(!stored.iter().any(|r| r.1 == DOCUMENT_C));
    let merged: Option<bool> = index(enabled.data_directory.path())
        .query_row("SELECT merged_into_primary FROM contexts WHERE worktree_path = ?1", params![worktree.to_str().unwrap()], |row| row.get(0))
        .unwrap();
    assert_eq!(merged, Some(true));
}

#[test]
fn the_primary_comparison_is_stored_for_the_own_item() {
    let (fixture, enabled, worktree) = repository();
    refresh_completely(&enabled.service, &fixture.root);
    let (state, oid, path): (Option<String>, Option<String>, Option<String>) = index(enabled.data_directory.path())
        .query_row(
            "SELECT primary_state, primary_compared_oid, primary_path FROM discovered_items WHERE item_id = ?1",
            params![DOCUMENT_A],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    // The draft exists only in its worktree.
    assert_eq!(state.as_deref(), Some("absent"));
    assert_eq!(oid, Some(fixture.repository.head().unwrap().target().unwrap().to_string()));
    assert_eq!(path, None);
}

#[test]
fn a_folder_only_a_branch_has_is_stored_for_that_context() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/plans/deep/new.md", &document_source(NEW, "New", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let folders: Vec<String> = index(enabled.data_directory.path())
        .prepare("SELECT discovered_folders.path FROM discovered_folders JOIN contexts ON contexts.id = discovered_folders.context_id WHERE contexts.worktree_path = ?1 ORDER BY 1")
        .unwrap()
        .query_map(params![worktree.to_str().unwrap()], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(folders, ["docs/plans", "docs/plans/deep"]);
}

#[test]
fn the_repository_snapshot_holds_own_rows_only() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/shared.md", &document_source(DOCUMENT_C, "Shared, edited", ""));
    let RefreshOutcome::Refreshed { snapshot } = refresh(&enabled.service, &fixture.root) else {
        panic!("expected a stable refresh");
    };
    let shared: Vec<_> = snapshot.items.iter().filter(|item| item.id.to_string() == DOCUMENT_C).collect();
    assert_eq!(shared.len(), 1);
    assert_eq!(shared[0].context, fixture.root.canonicalize().unwrap());
}

#[test]
fn a_refresh_with_contributions_changes_no_git_or_canonical_state() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/plans/new.md", &document_source(NEW, "New", ""));
    write(&worktree, "docs/shared.md", &document_source(DOCUMENT_C, "Shared, edited", ""));
    let before = support::repository_and_worktree_snapshot(&fixture);
    refresh_completely(&enabled.service, &fixture.root);
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

#[test]
fn a_contributed_file_changing_mid_refresh_is_retried() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/new.md", &document_source(NEW, "New", ""));
    let path = worktree.join("docs/new.md");
    // Runs between the two observations of one refresh, as in
    // tests/discovery_rebuild.rs's active worktree race.
    enabled.service.set_observation_hook_for_testing(move || {
        fs::write(&path, document_source(NEW, "Changed during the refresh", "")).unwrap();
    });
    let RefreshOutcome::RetryRequired { context: Some(context), .. } = refresh(&enabled.service, &fixture.root) else {
        panic!("expected the worktree to be retried");
    };
    assert_eq!(context, worktree);
    refresh_completely(&enabled.service, &fixture.root);
}
```

`snapshot.items[..].context` and `.id` are the `RepositorySnapshot` item
fields that `tests/discovery_rebuild.rs:786` uses. If `context` in
`RetryRequired` is a stored path string rather than a `PathBuf`, compare it
the way `tests/discovery_rebuild.rs:1870-1890` does.

Add `--test context_contributions` after `--test discovery_rebuild` in the
`cargo test` line of `.github/workflows/build.yml`.

In the test module of `src/repository/discovery.rs`, after
`an_index_from_before_folders_gains_the_table_and_is_marked_once`, add:

```rust
    // An index written by the build before contributions: no role, change,
    // primary or contributed columns, no merged flag and no deletions.
    #[test]
    fn an_index_from_before_contributions_gains_them_and_reads_rows_as_own() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.sqlite3");
        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO repositories (id, root_path, enabled_at, accessibility, refresh_required, refreshed_at)
                 VALUES (1, '/repository', 1, 'accessible', 0, 7);
                 INSERT INTO contexts (id, repository_id, kind, worktree_path)
                 VALUES (1, 1, 'primary', '/repository');
                 INSERT INTO discovered_items
                    (context_id, item_id, kind, canonical_path, title, activity_at, activity_source)
                 VALUES (1, '01ARZ3NDEKTSV4RRFFQ69G5FAV', 'document', 'docs/a.md', 'A', 0, 'git');
                 DROP TABLE discovered_deletions;
                 ALTER TABLE discovered_items DROP COLUMN role;
                 ALTER TABLE discovered_items DROP COLUMN change;
                 ALTER TABLE discovered_items DROP COLUMN primary_state;
                 ALTER TABLE discovered_items DROP COLUMN primary_compared_oid;
                 ALTER TABLE discovered_items DROP COLUMN primary_path;
                 ALTER TABLE discovered_comments DROP COLUMN contributed;
                 ALTER TABLE contexts DROP COLUMN merged_into_primary;",
            )
            .unwrap();
        assert_eq!(missing_item_read_schema(&connection).unwrap().len(), 8);
        drop(connection);

        let mut connection = open_registry(&path, &mut |_| {}).unwrap();
        migrate_registry(&mut connection).unwrap();

        assert!(missing_item_read_schema(&connection).unwrap().is_empty());
        let role: String = connection
            .query_row("SELECT role FROM discovered_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(role, "own");
        let marked: bool = connection
            .query_row("SELECT refresh_required FROM repositories", [], |row| row.get(0))
            .unwrap();
        assert!(marked);
    }
```

`DROP COLUMN` needs SQLite 3.35 or later. If the bundled SQLite is older,
make the old table instead: create it without the columns, copy the rows
across, then rename it. `migrate_registry` does the same to `repositories`
at `src/repository/discovery.rs:1579`.

- [ ] **Step 3: Run the tests to see them fail**

Run: `devenv shell -- cargo test --locked --test context_contributions`
Expected: FAIL. `discovered_items.role` does not exist (`no such column:
items.role`), and so on.

Run: `devenv shell -- cargo test --locked --lib an_index_from_before_contributions`
Expected: FAIL at the first `DROP`, because the columns do not exist yet.

- [ ] **Step 4: Add the problem codes**

In `src/results.rs`, inside `problem_codes!`, after `BranchProblem`:

```rust
    ContextChangesUnavailable => "context_changes_unavailable", Some("context-changes-unavailable"),
        "Merge the primary branch into the context's branch, then refresh the index.";
    ContextChangesOverBudget => "context_changes_over_budget", Some("context-changes-over-budget"),
        "Merge the branch, or spread its changes over fewer files, then refresh the index.";
```

Add both code strings to the code enums in `schemas/v1/problem.schema.json`
and `schemas/v1/index_problem.schema.json`, beside `branch_problem`.

- [ ] **Step 5: Extend the observation types**

In `src/repository/discovery.rs`:

```rust
use super::contributions::{
    self, ChangedPaths, ContributedChange, MAX_CONTEXT_CHANGES, ObservedDeletion,
    PrimaryComparison, PrimaryView,
};
use crate::results::ProblemCode;

/// An observed item's place in its context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ItemRole {
    /// The context's own copy: for primary, every item it holds.
    Own,
    /// An item the context's branch changed.
    Changed(ContributedChange),
    /// Holds only comments the branch changed, on an item it did not.
    Comments,
}

impl ItemRole {
    /// The stored role and change.
    pub(super) fn stored(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Own => ("own", None),
            Self::Changed(change) => ("changed", Some(change.stored())),
            Self::Comments => ("comments", None),
        }
    }
}
```

- To `ObservedItem` add `pub(super) role: ItemRole` and
  `pub(super) primary: Option<PrimaryComparison>`. Where `observe_items`
  builds each `ObservedItem`, set `role: ItemRole::Own, primary: None`.
- To `ObservedCommentThread` add `pub(super) contributed: bool`. In
  `observe_comment_thread`, set it to `true`.
- To `ActiveContextObservation` add `pub(super) deletions: Vec<ObservedDeletion>`,
  `pub(super) folders: Vec<ObservedFolder>` and
  `pub(super) merged_into_primary: bool`.
- To `RootObservationProblem` add
  `Changes { path: PathBuf, code: ProblemCode, message: String }`.

- [ ] **Step 6: Observe contributions**

In `observe_root`, build the primary view after `observe_items` and pass it
on:

```rust
    let primary = PrimaryView::new(head.oid, &sources);
    let active_contexts = match configuration {
        RootConfiguration::Valid(_) => {
            observe_active_contexts(repository, root, &primary, &folders, &mut problems)
        }
        RootConfiguration::Missing | RootConfiguration::Invalid(_) => Vec::new(),
    };
```

Change the signature of `observe_active_contexts` to take
`primary: &PrimaryView<'_>, primary_folders: &[ObservedFolder]` after
`root`. Inside, replace everything from `let mut sources = Vec::new();`
to `contexts.push(...)` with:

```rust
        let mut own_sources = Vec::new();
        collect_authoring_item_sources(
            &expected_path,
            kind,
            &path_id,
            &mut own_sources,
            &mut context_problems,
        );
        let own_paths: BTreeSet<PathBuf> =
            own_sources.iter().map(|(path, _)| path.clone()).collect();
        let changes = match contributions::changed_paths(&repository, &expected_path, primary.head) {
            ChangedPaths::Unavailable => {
                context_problems.push(RootObservationProblem::Changes {
                    path: expected_path.clone(),
                    code: ProblemCode::ContextChangesUnavailable,
                    message: "what the branch changed cannot be found: it has no merge base with the primary branch".to_owned(),
                });
                Vec::new()
            }
            ChangedPaths::Found { paths, over_budget } => {
                if over_budget {
                    context_problems.push(RootObservationProblem::Changes {
                        path: expected_path.clone(),
                        code: ProblemCode::ContextChangesOverBudget,
                        message: format!(
                            "the branch changed more than {MAX_CONTEXT_CHANGES} managed files; the first {MAX_CONTEXT_CHANGES} in path order are indexed"
                        ),
                    });
                }
                paths
            }
        };
        let contribution = contributions::contribute(&own_sources, &changes, primary);
        let sources = contribution.sources;
        let mut validation = canonical::validate_context(sources.clone());
        if !validation
            .items
            .iter()
            .any(|item| authoring_item_matches(item, kind, &path_id))
        {
            problems.push(context_problem(
                path,
                "the authoring branch's identified item is missing or invalid",
            ));
            continue;
        }
        for unreadable in &contribution.unreadable {
            context_problems.push(source_problem(
                expected_path.join(unreadable),
                "the canonical source cannot be read as UTF-8",
            ));
        }
        // Problems in files primary lent for validation are primary's.
        validation.problems.retain(|problem| {
            own_paths.contains(&problem.path) || contribution.changes.contains_key(&problem.path)
        });
        validation.problems.extend(contribution.duplicates);
        let mut items = observe_items(&repository, &expected_path, &sources, &validation);
        assign_roles(&mut items, kind, &path_id, &contribution.changes, primary, &sources);
        let folders = contributed_folders(&items, primary_folders);
        contexts.push(ActiveContextObservation {
            context: RootContextObservation {
                kind: DiscoveryContextKind::Active,
                branch: head.branch.clone(),
                worktree: expected_path,
                item_id: Some(path_id),
            },
            merged_into_primary: contributions::merged_into_primary(&repository, primary.head),
            head,
            sources,
            validation,
            items,
            deletions: contribution.deletions,
            folders,
            problems: context_problems,
        });
```

Add these functions after `observe_active_contexts`:

```rust
/// Gives each observed item its place in the context: the own item, an item
/// the branch changed, or only the holder of comments the branch changed.
/// An item that is none of these was validated only so that a changed
/// comment could be, and is dropped.
fn assign_roles(
    items: &mut Vec<ObservedItem>,
    kind: crate::repository::AuthoringKind,
    own_id: &canonical::ItemId,
    changes: &BTreeMap<PathBuf, ContributedChange>,
    primary: &PrimaryView<'_>,
    sources: &[(PathBuf, String)],
) {
    items.retain_mut(|item| {
        if item.id == *own_id && item.kind == kind {
            item.role = ItemRole::Own;
        } else {
            let holds_changed_comments = keep_changed_comments(&mut item.comments, changes);
            item.role = match changes.get(&item.path) {
                Some(change) => ItemRole::Changed(*change),
                None if holds_changed_comments => ItemRole::Comments,
                None => return false,
            };
        }
        if item.role != ItemRole::Comments {
            item.primary = sources
                .iter()
                .find(|(path, _)| *path == item.path)
                .map(|(_, source)| primary.compare(&item.id, source));
        }
        true
    });
}

/// Keeps the comments at changed paths and the unchanged ancestors their
/// replies need, marking which are which. Whether any was kept.
fn keep_changed_comments(
    threads: &mut Vec<ObservedCommentThread>,
    changes: &BTreeMap<PathBuf, ContributedChange>,
) -> bool {
    threads.retain_mut(|thread| {
        thread.contributed = changes.contains_key(&thread.path);
        let replies = keep_changed_comments(&mut thread.replies, changes);
        thread.contributed || replies
    });
    !threads.is_empty()
}

/// The folders under `docs/` that the context's changed documents are in
/// and primary did not list. Git keeps no empty folder, so none is empty.
fn contributed_folders(items: &[ObservedItem], primary: &[ObservedFolder]) -> Vec<ObservedFolder> {
    let listed: BTreeSet<&str> = primary.iter().map(|folder| folder.path.as_str()).collect();
    let mut folders = BTreeSet::new();
    for item in items.iter().filter(|item| {
        item.kind == crate::repository::AuthoringKind::Document
            && matches!(item.role, ItemRole::Changed(_))
    }) {
        let Some(mut path) = item.path.to_str() else {
            continue;
        };
        while let Some((parent, _)) = path.rsplit_once('/') {
            if parent == "docs" {
                break;
            }
            if !listed.contains(parent) {
                folders.insert(parent.to_owned());
            }
            path = parent;
        }
    }
    folders
        .into_iter()
        .map(|path| ObservedFolder { path, empty: false })
        .collect()
}
```

Rename the doc comment of `collect_authoring_item_sources` to say it
collects the *own* item's sources. Contributions are added by `contribute`.

Remove `#![allow(dead_code)]` from `contributions.rs`.

- [ ] **Step 7: Extend the schema**

In `src/repository/discovery.rs`:

```rust
const ITEM_READ_COLUMNS: [(&str, &str, &str); 11] = [
    ("discovered_items", "closed_by", "TEXT"),
    ("discovered_items", "unknown_metadata", "TEXT"),
    ("repositories", "refreshed_at", "INTEGER"),
    ("discovered_items", "slug", "TEXT"),
    (
        "discovered_items",
        "role",
        "TEXT NOT NULL DEFAULT 'own' CHECK (role IN ('own', 'changed', 'comments'))",
    ),
    ("discovered_items", "change", "TEXT CHECK (change IN ('added', 'modified', 'moved'))"),
    (
        "discovered_items",
        "primary_state",
        "TEXT CHECK (primary_state IN ('absent', 'identical', 'different'))",
    ),
    ("discovered_items", "primary_compared_oid", "TEXT"),
    ("discovered_items", "primary_path", "TEXT"),
    ("discovered_comments", "contributed", "INTEGER NOT NULL DEFAULT 1 CHECK (contributed IN (0, 1))"),
    ("contexts", "merged_into_primary", "INTEGER CHECK (merged_into_primary IN (0, 1))"),
];
```

Extend the doc comment above it: "an item's role in its context, what a
branch did to it, how it compares with primary, whether a stored comment is
the context's own change, and whether a context is merged into primary".

Append to `ITEM_READ_OBJECTS`, raising its length by one:

```rust
    (
        "table",
        "discovered_deletions",
        "CREATE TABLE discovered_deletions (
            id INTEGER PRIMARY KEY,
            context_id INTEGER NOT NULL REFERENCES contexts(id) ON DELETE CASCADE,
            item_id TEXT NOT NULL,
            kind TEXT NOT NULL CHECK (kind IN ('document', 'ticket', 'comment')),
            canonical_path TEXT NOT NULL,
            UNIQUE (context_id, item_id)
        )",
    ),
```

Add to the doc comment of `ITEM_READ_OBJECTS`: "A deletion is an item the
merge base of a context's branch had and the branch deleted. It goes with its
context row."

- [ ] **Step 8: Persist it**

In `src/repository.rs`:

- `PersistedContext` gains `deletions: &'a [ObservedDeletion]` and
  `merged_into_primary: Option<bool>`. The `folders` comment becomes: "the
  directories under `docs/` the observation listed: all of primary's, and
  for an item context those its changed documents need that primary lacks".
- `persist_context_refresh` gains the parameters
  `folders: &[discovery::ObservedFolder]`,
  `deletions: &[ObservedDeletion]` and `merged_into_primary: Option<bool>`.
  Pass them into `PersistedContext` in place of the
  `if context.worktree == root` expression. Its root caller passes
  `&before.folders, &[], None`. Its active caller passes
  `&active.folders, &active.deletions, Some(active.merged_into_primary)`.
- The rebuild loop (`for active in &observation.active_contexts`) passes the
  same three values for each active context, and `&[]`, `None` for the root.
- `persist_context`:

```rust
    transaction.execute(
        "INSERT INTO contexts (repository_id, kind, branch, worktree_path, item_id, head_oid, merged_into_primary) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![repository_id, context_kind_name(observed.context.kind), observed.context.branch, observed.context.worktree.to_str(), observed.context.item_id.as_ref().map(ToString::to_string), observed.head.oid.map(|oid| oid.to_string()), observed.merged_into_primary],
    ).map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    let context_id = transaction.last_insert_rowid();
    for item in observed.items {
        let (role, change) = item.role.stored();
        let primary = item.primary.as_ref();
        transaction.execute(
            "INSERT INTO discovered_items (context_id, item_id, kind, canonical_path, title, ticket_type, status, project, team, closed_at, activity_at, activity_source, closed_by, unknown_metadata, slug, role, change, primary_state, primary_compared_oid, primary_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![context_id, item.id.to_string(), authoring_kind_name(item.kind), item.path.to_str(), item.title, item.ticket_type, item.status, item.project, item.team, item.closed_at.map(OffsetDateTime::unix_timestamp), item.activity_at.unix_timestamp(), activity_source_name(item.activity_source), item.closed_by, item.unknown_metadata.to_stored(), item.relationships.slug, role, change, primary.map(|p| p.state.stored()), primary.and_then(|p| p.compared_head).map(|oid| oid.to_string()), primary.and_then(|p| p.path.as_deref()).and_then(Path::to_str)],
        ).map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
        // … relationships and comments as before
    }
    for deletion in observed.deletions {
        transaction
            .execute(
                "INSERT OR IGNORE INTO discovered_deletions (context_id, item_id, kind, canonical_path) VALUES (?1, ?2, ?3, ?4)",
                params![context_id, deletion.id.to_string(), item_kind_name(deletion.kind), deletion.path.to_str()],
            )
            .map_err(|error| RepositoryError::sqlite(error).for_operation(operation, root))?;
    }
```

  Add `fn item_kind_name(kind: canonical::ItemKind) -> &'static str`
  (`"document"`, `"ticket"`, `"comment"`) beside `context_kind_name`.
- In the problem loop, add the arm
  `RootObservationProblem::Changes { path, code, message } => (Some(path.as_path()), code.stored().expect("a stored problem code"), message.as_str()),`.
- `persist_comments` stores `comment.contributed`:
  `INSERT INTO discovered_comments (item_id, comment_id, parent_comment_id, canonical_path, created_at, contributed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)`.
- `changed_contexts`: in the active-context comparison, add
  `&& candidate.merged_into_primary == context.merged_into_primary` and add
  `candidate.deletions`, `candidate.folders` to both `format!` strings.
- In `read_repository_snapshot`'s item query, add
  `AND discovered_items.role = 'own'` to the `WHERE` clause, with the
  comment: "Contributed copies are for the reads; the snapshot is each
  context's own items, one copy per ID."

Every other `match` on `RootObservationProblem` gets a `Changes` arm. Find
them with `grep -n "RootObservationProblem::" src -r`.

- [ ] **Step 9: Run the tests to see them pass**

Run: `devenv shell -- cargo test --locked --test context_contributions`
Expected: PASS.

Then run the targets this task can break:

```
devenv shell -- cargo test --locked --lib discovery
devenv shell -- cargo test --locked --test discovery_rebuild --test local_authoring --test read_items --test read_comments --test read_relationships
```

Expected: PASS, except tests that pinned "an item worktree contributes only
its own item". For each failure:

- if the test made a branch change another item and asserted it is ignored,
  change its assertion to the new rule and say so in the commit message;
- if it failed for any other reason, the implementation is wrong. Fix the
  code, not the test.

Candidates named in the spec are `tests/discovery_rebuild.rs:3668-3945` and
`tests/local_authoring.rs:1549-1659`, `:2005`, `:2125`. Delete none.

- [ ] **Step 10: Check the desktop build**

Run: `devenv shell -- cargo check --features desktop --locked`
Expected: no errors.

- [ ] **Step 11: Commit**

```bash
git add src/repository.rs src/repository/discovery.rs src/repository/contributions.rs src/results.rs schemas/v1/problem.schema.json schemas/v1/index_problem.schema.json tests/support/items.rs tests/context_contributions.rs .github/workflows/build.yml
git commit -m "feat: index every item a work context's branch changed" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If Step 9 changed pinned tests, add a paragraph to the message, before the
co-author line. Name each test and the one-item assumption it made.

---

### Task 5: Item reads: effective copy, versions, primary comparison, folders

**Files:**
- Modify: `src/repository/read/dto.rs`
- Modify: `src/repository/read/items.rs`
- Modify: `schemas/v1/item.schema.json`, `schemas/v1/item_context.schema.json`,
  `schemas/v1/document_folder.schema.json`
- Modify: `tests/context_contributions.rs`
- Modify: golden fixtures under `tests/fixtures/read_v1/` and
  `tests/fixtures/cli_v1/` (regenerated)

**Interfaces:**
- Consumes: the Task 4 columns and table.
- Produces:
  - DTO enums `ItemChange { Added, Modified, Moved, Deleted, Primary }` and `PrimaryComparisonState { Absent, Identical, Different }`, both through `contract_enum!`;
  - `pub struct ItemVersionDto { pub context: ItemContextDto, pub path: String, pub change: ItemChange, pub problems: Vec<ProblemDto> }`;
  - `pub struct PrimaryComparisonDto { pub state: PrimaryComparisonState, pub compared_head_oid: Option<String> }`;
  - `ItemDto.versions: Vec<ItemVersionDto>`, `ItemDto.primary_comparison: Option<PrimaryComparisonDto>`, `ItemContextDto.merged_into_primary: Option<bool>`, `DocumentFolderDto.context: Option<ItemContextDto>`;
  - `pub(super) enum StoredRole { Own, Changed(ItemChange), Comments }` and `StoredItem.role`, `StoredItem.primary: Option<StoredPrimary>`;
  - `pub(super) fn versions(stored: &[StoredItem], deletions: &[StoredDeletion], root: Option<&StoredContext>, effective: &StoredItem) -> Vec<ItemVersionDto>`;
  - `pub struct ContextChangeDto { pub id: String, pub kind: ItemDtoKind, pub title: Option<String>, pub path: String, pub change: ItemChange, pub effective: bool }` and `ItemDto.context_changes: Vec<ContextChangeDto>`;
  - `pub(super) fn context_changes(repo: &ResolvedRepository, stored: &[StoredItem], effective: &[&StoredItem], deletions: &[StoredDeletion], own: &StoredItem) -> Vec<ContextChangeDto>`.

- [ ] **Step 1: Write the failing tests**

Add `DOCUMENT_B` and `TICKET_A` to the `support::items` import of
`tests/context_contributions.rs`, then append:

```rust
use manyhands::repository::read::{ItemChange, ItemContextKind, PrimaryComparisonState};

fn repo(enabled: &support::EnabledRepository, root: &Path) -> manyhands::repository::ResolvedRepository {
    enabled.service.resolve_repository(root).unwrap()
}

#[test]
fn a_branch_only_document_is_listed_and_shown_from_its_branch() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/plans/new.md", &document_source(NEW, "New", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let list = enabled.service.list_documents(&repo).unwrap();
    let new = list.items.iter().find(|item| item.id.as_deref() == Some(NEW)).unwrap();
    assert_eq!(new.context.worktree, worktree.to_str().unwrap());
    assert!(new.versions.is_empty());
    assert_eq!(list.index.state, manyhands::repository::read::IndexState::Current);
    let shown = enabled.service.show_item(&repo, &NEW.parse().unwrap()).unwrap();
    assert_eq!(shown.title.as_deref(), Some("New"));
    assert_eq!(shown.primary_comparison.unwrap().state, PrimaryComparisonState::Absent);
}

#[test]
fn primary_wins_and_the_branch_copy_is_a_version() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/shared.md", &document_source(DOCUMENT_C, "Shared, edited", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let shown = enabled.service.show_item(&repo, &DOCUMENT_C.parse().unwrap()).unwrap();
    assert_eq!(shown.title.as_deref(), Some("Shared"));
    assert_eq!(shown.context.kind, ItemContextKind::Primary);
    assert_eq!(shown.primary_comparison, None);
    assert_eq!(shown.versions.len(), 1);
    assert_eq!(shown.versions[0].context.worktree, worktree.to_str().unwrap());
    assert_eq!(shown.versions[0].path, "docs/shared.md");
    assert_eq!(shown.versions[0].change, ItemChange::Modified);
    let listed = enabled.service.list_documents(&repo).unwrap();
    let listed = listed.items.iter().find(|item| item.id.as_deref() == Some(DOCUMENT_C)).unwrap();
    assert_eq!(listed.versions, shown.versions);
}

#[test]
fn a_deletion_on_a_branch_is_a_version() {
    let (fixture, enabled, worktree) = repository();
    fs::remove_file(worktree.join("docs/shared.md")).unwrap();
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let shown = enabled.service.show_item(&repo, &DOCUMENT_C.parse().unwrap()).unwrap();
    assert_eq!(shown.context.kind, ItemContextKind::Primary);
    assert_eq!(shown.versions.iter().map(|v| v.change).collect::<Vec<_>>(), [ItemChange::Deleted]);
}

#[test]
fn an_own_draft_that_differs_from_primary_names_primarys_copy_as_a_version() {
    let (fixture, enabled, _) = repository();
    // DOCUMENT_C gets a worktree of its own, and its draft differs.
    create_document_context(&enabled.service, &fixture.root, DOCUMENT_C, "docs/shared.md");
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let shown = enabled.service.show_item(&repo, &DOCUMENT_C.parse().unwrap()).unwrap();
    assert_eq!(shown.context.kind, ItemContextKind::Active);
    let comparison = shown.primary_comparison.unwrap();
    assert_eq!(comparison.state, PrimaryComparisonState::Different);
    assert_eq!(comparison.compared_head_oid, Some(fixture.repository.head().unwrap().target().unwrap().to_string()));
    assert_eq!(shown.versions.len(), 1);
    assert_eq!(shown.versions[0].change, ItemChange::Primary);
    assert_eq!(shown.versions[0].context.kind, ItemContextKind::Primary);
    assert_eq!(shown.versions[0].path, "docs/shared.md");
    assert_eq!(shown.context.merged_into_primary, Some(false));
}

#[test]
fn the_four_comparison_cases_are_distinguishable() {
    // Different and absent are covered above; identical and merged here.
    let (fixture, enabled, _) = repository();
    create_document_context(&enabled.service, &fixture.root, DOCUMENT_C, "docs/shared.md");
    let own = context_worktree(&fixture.root, DOCUMENT_C);
    // Put the draft back to primary's bytes: identical.
    fs::write(own.join("docs/shared.md"), fs::read(fixture.root.join("docs/shared.md")).unwrap()).unwrap();
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let shown = enabled.service.show_item(&repo, &DOCUMENT_C.parse().unwrap()).unwrap();
    assert_eq!(shown.primary_comparison.unwrap().state, PrimaryComparisonState::Identical);
    assert!(shown.versions.is_empty());
    // Merged, then primary moves on: different, and merged.
    commit_in(&own, &["docs/shared.md"], 4_000);
    let branch = git2::Repository::open(&own).unwrap().head().unwrap().target().unwrap();
    let name = fixture.repository.head().unwrap().name().unwrap().to_owned();
    fixture.repository.reference(&name, branch, true, "merge").unwrap();
    fixture.repository.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();
    write(&fixture.root, "docs/shared.md", &document_source(DOCUMENT_C, "Shared v3", ""));
    commit(&fixture, &["docs/shared.md"], 5_000);
    refresh_completely(&enabled.service, &fixture.root);
    let shown = enabled.service.show_item(&repo, &DOCUMENT_C.parse().unwrap()).unwrap();
    assert_eq!(shown.primary_comparison.unwrap().state, PrimaryComparisonState::Different);
    assert_eq!(shown.context.merged_into_primary, Some(true));
}

#[test]
fn two_branches_adding_one_id_keep_the_earlier_and_flag_the_other() {
    let (fixture, enabled, first) = repository();
    create_document_context(&enabled.service, &fixture.root, DOCUMENT_B, "docs/second.md");
    let second = context_worktree(&fixture.root, DOCUMENT_B);
    write(&first, "docs/x.md", &document_source(NEW, "From A", ""));
    write(&second, "docs/y.md", &document_source(NEW, "From B", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let shown = enabled.service.show_item(&repo, &NEW.parse().unwrap()).unwrap();
    // DOCUMENT_A's ULID sorts before DOCUMENT_B's.
    assert_eq!(shown.title.as_deref(), Some("From A"));
    assert_eq!(shown.versions.len(), 1);
    assert_eq!(shown.versions[0].context.worktree, second.to_str().unwrap());
    assert_eq!(shown.versions[0].change, ItemChange::Added);
    assert_eq!(shown.versions[0].problems[0].code, manyhands::results::ProblemCode::DuplicateId);
}

#[test]
fn an_item_read_from_its_own_worktree_lists_what_else_its_branch_changed() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/plans/design.md", &document_source(NEW, "Design", ""));
    write(&worktree, "docs/shared.md", &document_source(DOCUMENT_C, "Shared, edited", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let own = enabled.service.show_item(&repo, &DOCUMENT_A.parse().unwrap()).unwrap();
    let changes: Vec<_> = own
        .context_changes
        .iter()
        .map(|change| (change.id.as_str(), change.path.as_str(), change.title.as_deref(), change.change, change.effective))
        .collect();
    assert_eq!(
        changes,
        [
            (NEW, "docs/plans/design.md", Some("Design"), ItemChange::Added, true),
            (DOCUMENT_C, "docs/shared.md", Some("Shared, edited"), ItemChange::Modified, false),
        ]
    );
    // The list carries the same, and an item read from primary has none.
    let list = enabled.service.list_documents(&repo).unwrap();
    let listed = list.items.iter().find(|item| item.id.as_deref() == Some(DOCUMENT_A)).unwrap();
    assert_eq!(listed.context_changes, own.context_changes);
    let shared = enabled.service.show_item(&repo, &DOCUMENT_C.parse().unwrap()).unwrap();
    assert!(shared.context_changes.is_empty());
}

#[test]
fn the_folder_list_includes_a_branchs_new_folders_with_their_context() {
    let (fixture, enabled, worktree) = repository();
    write(&worktree, "docs/plans/new.md", &document_source(NEW, "New", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let folders = enabled.service.list_document_folders(&repo).unwrap();
    let plans = folders.items.iter().find(|folder| folder.path == "docs/plans").unwrap();
    assert_eq!(plans.context.as_ref().unwrap().worktree, worktree.to_str().unwrap());
}

#[test]
fn relationship_reads_see_a_branch_only_ticket_but_not_another_branchs_deps_edit() {
    let (fixture, enabled, worktree) = repository();
    // Primary's TICKET_A depends on NEW, which only the branch has.
    write(&fixture.root, &support::items::ticket_path(TICKET_A), &support::items::ticket_source(TICKET_A, "Needs new", &format!("deps:\n  - {NEW}\n")));
    commit(&fixture, &[&support::items::ticket_path(TICKET_A)], 2_000);
    write(&worktree, &support::items::ticket_path(NEW), &support::items::ticket_source(NEW, "New", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let ticket = enabled.service.show_item(&repo, &TICKET_A.parse().unwrap()).unwrap();
    assert_eq!(ticket.deps[0].state, manyhands::repository::read::DependencyState::Open);
    // The branch adds a dependency to TICKET_A; primary's copy stays effective.
    write(&worktree, &support::items::ticket_path(TICKET_A), &support::items::ticket_source(TICKET_A, "Needs new", &format!("deps:\n  - {NEW}\n  - {DOCUMENT_C}\n")));
    refresh_completely(&enabled.service, &fixture.root);
    let ticket = enabled.service.show_item(&repo, &TICKET_A.parse().unwrap()).unwrap();
    assert_eq!(ticket.deps.len(), 1);
    assert_eq!(ticket.versions.len(), 1);
}
```

The worktree in the last test branched before `TICKET_A` was committed, so
`TICKET_A` is new on the branch as well. Primary's copy still wins, and the
branch's copy is a version with change `added` and a `duplicate_id` problem.
That is the spec's duplicate rule. If you want the test to show `modified`
instead, merge primary into the worktree first with `commit_in` after writing
primary's file there.

If the DTO enums are exported from a different module path than
`manyhands::repository::read`, use the path that `ItemContextKind` is
imported from in `tests/read_items.rs`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `devenv shell -- cargo test --locked --test context_contributions`
Expected: compile errors. `versions`, `primary_comparison`,
`merged_into_primary`, `ItemChange`, `PrimaryComparisonState` and
`DocumentFolderDto::context` do not exist.

- [ ] **Step 3: Add the DTOs**

In `src/repository/read/dto.rs`:

```rust
contract_enum!(
    /// What a version is, next to the item's effective copy.
    ItemChange {
        Added => "added",
        Modified => "modified",
        Moved => "moved",
        Deleted => "deleted",
        Primary => "primary",
    }
);

contract_enum!(
    /// How an item's copy in a worktree compares with primary's.
    PrimaryComparisonState {
        Absent => "absent",
        Identical => "identical",
        Different => "different",
    }
);

/// A copy of an item that is not its effective copy: one a branch changed,
/// added under an ID another copy has, or deleted; or primary's copy of an
/// item read from its own worktree, when the two differ. `problems` holds
/// `duplicate_id` for an added copy, which another copy outranks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ItemVersionDto {
    pub context: ItemContextDto,
    /// Relative to `context.worktree`, with forward slashes.
    pub path: String,
    pub change: ItemChange,
    pub problems: Vec<ProblemDto>,
}

/// How an item read from a worktree compares with primary's copy of it, as
/// the refresh found, and the primary HEAD it compared against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PrimaryComparisonDto {
    pub state: PrimaryComparisonState,
    pub compared_head_oid: Option<String>,
}
```

- `ItemDto` gains, after `context`:
  `pub versions: Vec<ItemVersionDto>` ("every other copy, in worktree then
  path order; empty in a nonconforming entry") and
  `pub primary_comparison: Option<PrimaryComparisonDto>` ("null when the
  effective copy is primary's, and for a nonconforming entry").
- `ItemContextDto` gains `pub merged_into_primary: Option<bool>` ("null for
  primary; for an item context, whether its HEAD is primary's or an ancestor
  of it").
- `DocumentFolderDto` gains `pub context: Option<ItemContextDto>` ("null for
  primary's folders; the context whose changed documents need this folder").
- `ItemDto` gains `pub context_changes: Vec<ContextChangeDto>` ("for an item
  read from its own worktree, every other document or ticket its branch
  changed or deleted, in path order; empty for any other item"), with:

```rust
/// A document or ticket that an item's own worktree changed, besides the
/// item itself: what to review alongside it. `title` is null for a
/// deletion. `effective` says that this branch's copy is the item's
/// effective copy, as for a document only this branch has; otherwise the
/// copy is one of that item's versions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextChangeDto {
    pub id: String,
    pub kind: ItemDtoKind,
    pub title: Option<String>,
    /// Relative to the item's `context.worktree`, with forward slashes.
    pub path: String,
    /// `added`, `modified`, `moved` or `deleted`.
    pub change: ItemChange,
    pub effective: bool,
}
```

- Every `ItemDto { … }` literal in `src/repository/read/` gets
  `versions: Vec::new(), primary_comparison: None, context_changes: Vec::new()`
  (`grep -n "ItemDto {" src/repository/read`).
- Every `ItemContextDto { … }` literal gets its `merged_into_primary`.
- Export `ItemChange`, `ItemVersionDto`, `PrimaryComparisonDto` and
  `PrimaryComparisonState` wherever `ItemContextKind` is re-exported.

Update the schemas. In `schemas/v1/item_context.schema.json`, add
`"merged_into_primary"` to `required`, and to `properties` add
`"merged_into_primary": { "type": ["boolean", "null"] }`. In
`schemas/v1/item.schema.json`, add `"versions"` and `"primary_comparison"`
to `required`, and to `properties`:

```json
"versions": {
  "type": "array",
  "items": {
    "type": "object",
    "additionalProperties": false,
    "required": ["context", "path", "change", "problems"],
    "properties": {
      "context": { "$ref": "item_context.schema.json" },
      "path": { "type": "string" },
      "change": { "type": "string", "enum": ["added", "modified", "moved", "deleted", "primary"] },
      "problems": { "type": "array", "items": { "$ref": "problem.schema.json" } }
    }
  }
},
"primary_comparison": {
  "type": ["object", "null"],
  "additionalProperties": false,
  "required": ["state", "compared_head_oid"],
  "properties": {
    "state": { "type": "string", "enum": ["absent", "identical", "different"] },
    "compared_head_oid": { "type": ["string", "null"] }
  }
}
```

Use the `$ref` form the file already uses for `context` and `problems`.
Add `"context_changes"` to `required` too, with:

```json
"context_changes": {
  "type": "array",
  "items": {
    "type": "object",
    "additionalProperties": false,
    "required": ["id", "kind", "title", "path", "change", "effective"],
    "properties": {
      "id": { "type": "string" },
      "kind": { "type": "string", "enum": ["document", "ticket"] },
      "title": { "type": ["string", "null"] },
      "path": { "type": "string" },
      "change": { "type": "string", "enum": ["added", "modified", "moved", "deleted"] },
      "effective": { "type": "boolean" }
    }
  }
}
``` In
`schemas/v1/document_folder.schema.json`, add `"context"` to `required` and
`"context": { "oneOf": [{ "type": "null" }, { "$ref": "item_context.schema.json" }] }`.

- [ ] **Step 4: Read the new columns**

In `src/repository/read/items.rs`:

```rust
/// An item row's place in its context; see `discovery::ItemRole`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StoredRole {
    Own,
    /// `Added`, `Modified` or `Moved`.
    Changed(ItemChange),
    Comments,
}

/// What the refresh found about primary's copy of a worktree's item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StoredPrimary {
    pub(super) state: PrimaryComparisonState,
    pub(super) compared_head_oid: Option<String>,
    pub(super) path: Option<String>,
}

/// A document or ticket a context's branch deleted. Comment deletions are
/// not read: a comment's versions come from the comment reads.
pub(super) struct StoredDeletion {
    pub(super) context: StoredContext,
    pub(super) id: String,
    pub(super) kind: ItemDtoKind,
    pub(super) path: String,
}

fn stored_role(role: &str, change: Option<&str>) -> Result<StoredRole, ReadError> {
    Ok(match (role, change) {
        ("own", None) => StoredRole::Own,
        ("comments", None) => StoredRole::Comments,
        ("changed", Some("added")) => StoredRole::Changed(ItemChange::Added),
        ("changed", Some("modified")) => StoredRole::Changed(ItemChange::Modified),
        ("changed", Some("moved")) => StoredRole::Changed(ItemChange::Moved),
        _ => return Err(invalid_stored_data()),
    })
}

fn stored_primary_state(state: &str) -> Result<PrimaryComparisonState, ReadError> {
    Ok(match state {
        "absent" => PrimaryComparisonState::Absent,
        "identical" => PrimaryComparisonState::Identical,
        "different" => PrimaryComparisonState::Different,
        _ => return Err(invalid_stored_data()),
    })
}
```

- `StoredContext` gains `merged_into_primary: Option<bool>`, and `dto()`
  copies it.
- `StoredItem` gains `pub(super) role: StoredRole` and
  `pub(super) primary: Option<StoredPrimary>`.
- In `stored_items`, select also `items.role, items.change,
  items.primary_state, items.primary_compared_oid, items.primary_path,
  contexts.merged_into_primary` (columns 19–24). Add
  `AND items.role != 'comments'` to the `WHERE` clause, with the comment: "A
  row that only holds a branch's comments is not a copy of its item; the
  comment reads find it themselves." Fill the new fields with
  `stored_role(&role, change.as_deref())?` and, when `primary_state` is not
  null, `StoredPrimary { state: stored_primary_state(&state)?, compared_head_oid, path }`.
- Add `stored_deletions(connection, repo) -> Result<Vec<StoredDeletion>, ReadError>`
  selecting `contexts.kind, contexts.branch, contexts.worktree_path,
  contexts.head_oid, contexts.merged_into_primary, discovered_deletions.item_id,
  discovered_deletions.kind, discovered_deletions.canonical_path` for the
  registration, with `discovered_deletions.kind != 'comment'`, ordered by
  worktree then path. Read `kind` with `ItemDtoKind::from_stored`.
- Add `stored_root_context(connection, repo) -> Result<Option<StoredContext>, ReadError>`.
  It selects the context row whose `worktree_path` is `repo.root()` as text.

- [ ] **Step 5: Rank the rows and list the versions**

Replace `OWN_WORKTREE`, `OTHER_WORKTREE` and `row_rank` with:

```rust
const OWN_WORKTREE: u8 = 0;
const PRIMARY: u8 = 1;
const CONTRIBUTED: u8 = 2;
const OTHER_WORKTREE: u8 = 3;

/// The order in which the rows of one item are tried: its own worktree's,
/// then one outside any item worktree, then a copy a branch changed, and
/// last an own-role copy in another item's worktree.
///
/// That last kind is never what a refresh stores now; an index written by
/// an older build can hold one. It is answered from only when nothing else
/// is there, and the index is then reported as behind.
fn row_rank(repo: &ResolvedRepository, row: &StoredItem) -> u8 {
    match row.role {
        StoredRole::Changed(_) => CONTRIBUTED,
        _ if is_own_worktree_row(repo, row) => OWN_WORKTREE,
        _ if row.context.kind != ItemContextKind::Active => PRIMARY,
        _ => OTHER_WORKTREE,
    }
}

/// Rows of one item in the order they are tried. Contributed copies go by
/// their branch's ULID, so the earliest-created branch's copy comes first;
/// the others keep the index's order.
fn row_order<'a>(repo: &ResolvedRepository, row: &'a StoredItem) -> (u8, &'a str) {
    let rank = row_rank(repo, row);
    let ulid = match rank {
        CONTRIBUTED => row.context.branch.as_deref().and_then(|branch| branch.rsplit('/').next()).unwrap_or(""),
        _ => "",
    };
    (rank, ulid)
}
```

In `effective_rows` and `effective_copy`, sort with
`rows.sort_by_key(|row| row_order(repo, *row))`. The `*row` matters: the key
borrows from the stored row, not from the closure's argument. Replace "the index held the
item more than once" with "more than one own-role row":

```rust
            is_behind |= rows.iter().filter(|row| row.role == StoredRole::Own).count() > 1;
```

In `effective_copy`, apply the same change where `rows.len() > 1` sets
`behind`. Update both doc comments: "held an item more than once" becomes
"held two own copies of one item". Several branch copies are normal.

Add:

```rust
/// Every copy of `effective`'s item that is not it: each other row a
/// branch changed, each deletion, and primary's copy of an item read from
/// its own worktree when the two differ. In worktree, then path, order.
pub(super) fn versions(
    stored: &[StoredItem],
    deletions: &[StoredDeletion],
    root: Option<&StoredContext>,
    effective: &StoredItem,
) -> Vec<ItemVersionDto> {
    let mut versions = Vec::new();
    for row in stored.iter().filter(|row| row.id == effective.id && !std::ptr::eq(*row, effective)) {
        let StoredRole::Changed(change) = row.role else {
            continue;
        };
        // An added copy that is not effective is outranked by another copy
        // of the same ID.
        let problems = match change {
            ItemChange::Added => vec![problem(ProblemCode::DuplicateId, &row.path)],
            _ => Vec::new(),
        };
        versions.push(ItemVersionDto { context: row.context.dto(), path: row.path.clone(), change, problems });
    }
    for deletion in deletions.iter().filter(|deletion| deletion.id == effective.id) {
        versions.push(ItemVersionDto {
            context: deletion.context.dto(),
            path: deletion.path.clone(),
            change: ItemChange::Deleted,
            problems: Vec::new(),
        });
    }
    if effective.context.kind == ItemContextKind::Active
        && let (Some(root), Some(StoredPrimary { state: PrimaryComparisonState::Different, path: Some(path), .. })) =
            (root, &effective.primary)
    {
        versions.push(ItemVersionDto { context: root.dto(), path: path.clone(), change: ItemChange::Primary, problems: Vec::new() });
    }
    versions.sort_by(|left, right| (&left.context.worktree, &left.path).cmp(&(&right.context.worktree, &right.path)));
    versions
}

/// The comparison to report: only for an item read from a worktree.
pub(super) fn primary_comparison(row: &StoredItem) -> Option<PrimaryComparisonDto> {
    if row.context.kind != ItemContextKind::Active {
        return None;
    }
    row.primary.as_ref().map(|primary| PrimaryComparisonDto {
        state: primary.state,
        compared_head_oid: primary.compared_head_oid.clone(),
    })
}

/// What else `own`'s worktree changed, when `own` is read from the worktree
/// created for it: each other row a branch changed there and each document
/// or ticket it deleted, in path order. Empty for any other item.
/// `effective` holds one row per ID, as `effective_rows` gives them.
pub(super) fn context_changes(
    repo: &ResolvedRepository,
    stored: &[StoredItem],
    effective: &[&StoredItem],
    deletions: &[StoredDeletion],
    own: &StoredItem,
) -> Vec<ContextChangeDto> {
    if own.role != StoredRole::Own || !is_own_worktree_row(repo, own) {
        return Vec::new();
    }
    let worktree = &own.context.worktree;
    let mut changes: Vec<ContextChangeDto> = stored
        .iter()
        .filter(|row| &row.context.worktree == worktree)
        .filter_map(|row| match row.role {
            StoredRole::Changed(change) => Some(ContextChangeDto {
                id: row.id.clone(),
                kind: row.kind,
                title: Some(row.title.clone()),
                path: row.path.clone(),
                change,
                effective: effective.iter().any(|chosen| std::ptr::eq(*chosen, row)),
            }),
            _ => None,
        })
        .chain(deletions.iter().filter(|deletion| &deletion.context.worktree == worktree).map(|deletion| {
            ContextChangeDto {
                id: deletion.id.clone(),
                kind: deletion.kind,
                title: None,
                path: deletion.path.clone(),
                change: ItemChange::Deleted,
                effective: false,
            }
        }))
        .collect();
    changes.sort_by(|left, right| left.path.cmp(&right.path));
    changes
}
```

If the crate's edition or MSRV rejects `if … && let`, write it as a nested
`if let`.

In `show_item`, after `item.observation = Some(observation);`, load
`stored_deletions` and `stored_root_context`. Then set:

- `item.versions = versions(&stored, &deletions, root.as_ref(), row)`;
- `item.primary_comparison = primary_comparison(row)`;
- `item.context_changes = context_changes(repo, &stored, &effective_rows(repo, &stored).0, &deletions, row)`.

In `list_items`, load the deletions, the root context and `effective_rows`
once per list. Set the same three fields on each listed item from its
effective row, where the list maps each effective row to `stored_item_dto`.

- [ ] **Step 6: List contributed folders**

In `stored_folders`, drop `AND contexts.worktree_path = ?2` and select
`contexts.kind, contexts.branch, contexts.worktree_path, contexts.head_oid,
contexts.merged_into_primary` too. Order by path, then
`contexts.worktree_path = ?2 DESC` so primary's row comes first for a path.
Keep the first row for each path. Set `context: None` for primary's rows and
`Some(stored context dto)` for the others. Update its doc comment and
`list_document_folders`' ("every directory under `docs/` in the primary
worktree, and those a branch's changed documents need, …").

- [ ] **Step 7: Run the tests to see them pass**

Run: `devenv shell -- cargo test --locked --test context_contributions`
Expected: PASS.

- [ ] **Step 8: Regenerate the golden fixtures and review the diff**

Run: `MANYHANDS_UPDATE_GOLDEN=1 devenv shell -- cargo test --locked --test read_contract --test read_items --test cli`
Then: `git diff --stat tests/fixtures`. Expected: the only changes are added
`versions: []`, `primary_comparison: null`, `merged_into_primary` and
folder `context: null` fields. If any other value changed, stop and fix the
code.

Run, without the variable:
`devenv shell -- cargo test --locked --test read_contract --test read_items --test read_comments --test read_relationships --test read_boundary --test cli`
Expected: PASS, apart from pinned one-item tests. Handle those as in Task 4
Step 9. The candidates are `tests/read_items.rs:2813`, `:4045`,
`tests/read_relationships.rs:290`.

- [ ] **Step 9: Commit**

```bash
git add src/repository/read schemas/v1 tests/context_contributions.rs tests/fixtures
git commit -m "feat: read one effective copy per item and list its versions" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Comment reads

**Files:**
- Modify: `src/repository/read/comments.rs`
- Modify: `src/repository/read/dto.rs`
- Modify: `schemas/v1/comment.schema.json`
- Modify: `tests/context_contributions.rs`
- Modify: golden fixtures (regenerated)

**Interfaces:**
- Consumes: `discovered_comments.contributed`, `StoredRole`, `ItemVersionDto`
  (Task 5).
- Produces: `CommentDto.context: ItemContextDto` and
  `CommentDto.versions: Vec<ItemVersionDto>`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/context_contributions.rs`:

```rust
const ROOT_COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE4";

#[test]
fn a_branchs_comment_on_a_primary_item_is_listed_with_its_context() {
    let (fixture, enabled, worktree) = repository();
    let root_path = comment_path(DOCUMENT_C, ROOT_COMMENT);
    write(&fixture.root, &root_path, &comment_source(DOCUMENT_C, ROOT_COMMENT, None, CREATED, ""));
    commit(&fixture, &[&root_path], 2_000);
    // The reply is on the branch only; its parent is primary's comment,
    // which the branch does not have.
    let reply = comment_path(DOCUMENT_C, COMMENT);
    write(&worktree, &reply, &comment_source(DOCUMENT_C, COMMENT, Some(ROOT_COMMENT), "2026-10-23T01:00:00Z", ""));
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let list = enabled.service.list_comments(&repo, &DOCUMENT_C.parse().unwrap()).unwrap();
    let ids: Vec<_> = list.items.iter().map(|c| c.id.as_deref().unwrap()).collect();
    assert_eq!(ids, [ROOT_COMMENT, COMMENT]);
    assert_eq!(list.items[0].context.kind, ItemContextKind::Primary);
    assert_eq!(list.items[1].context.worktree, worktree.to_str().unwrap());
    assert_eq!(list.items[1].depth, 1);
    assert!(list.complete);
}

#[test]
fn a_branchs_edit_of_a_primary_comment_is_a_version_of_it() {
    let (fixture, enabled, worktree) = repository();
    let path = comment_path(DOCUMENT_C, COMMENT);
    write(&fixture.root, &path, &comment_source(DOCUMENT_C, COMMENT, None, CREATED, ""));
    commit(&fixture, &[&path], 2_000);
    // The branch gets primary's comment, then edits it.
    write(&worktree, &path, &comment_source(DOCUMENT_C, COMMENT, None, CREATED, ""));
    commit_in(&worktree, &[&path], 2_100);
    write(&worktree, &path, &comment_source(DOCUMENT_C, COMMENT, None, CREATED, "note: edited\n"));
    refresh_completely(&enabled.service, &fixture.root);
    let repo = repo(&enabled, &fixture.root);
    let list = enabled.service.list_comments(&repo, &DOCUMENT_C.parse().unwrap()).unwrap();
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].context.kind, ItemContextKind::Primary);
    assert!(!list.items[0].unknown_metadata.contains_key("note"));
    assert_eq!(list.items[0].versions.len(), 1);
    assert_eq!(list.items[0].versions[0].context.worktree, worktree.to_str().unwrap());
    assert_eq!(list.items[0].versions[0].change, ItemChange::Modified);
}
```

The worktree in the second test branched before primary committed the
comment, so the branch's first commit of it is an addition. Then the
uncommitted edit differs from the merge base, which lacks the file. The
branch's copy is stored as `added` and is a version of primary's. In a
comment's version list, an `added` copy of an existing ID is reported as
`modified`, because a comment has no path of its own to collide on. The
test asserts that.

- [ ] **Step 2: Run the tests to see them fail**

Run: `devenv shell -- cargo test --locked --test context_contributions comment`
Expected: compile errors. `CommentDto` has no `context` or `versions`.

- [ ] **Step 3: Add the fields**

In `dto.rs`, `CommentDto` gains:

```rust
    /// Where this comment was read from: the item's effective context, or
    /// the context of a branch that added it.
    pub context: ItemContextDto,
    /// Other copies of this comment that branches changed.
    pub versions: Vec<ItemVersionDto>,
```

Set them in every `CommentDto { … }` literal: `context` is the list's
context, and `versions` is empty. Update `schemas/v1/comment.schema.json`:
add both to `required`, add `context` as a `$ref` to `item_context.schema.json`,
and add `versions` with the same shape as in `item.schema.json`.

- [ ] **Step 4: Read contributed comments**

In `src/repository/read/items.rs`, add a constructor that `comments.rs` can
use, and use it in `stored_items` as well:

```rust
impl StoredContext {
    /// A context row's five stored columns, in `contexts` column order.
    pub(super) fn from_stored(
        kind: &str,
        branch: Option<String>,
        worktree: String,
        head_oid: Option<String>,
        merged_into_primary: Option<bool>,
    ) -> Result<Self, ReadError> {
        Ok(Self { kind: stored_context_kind(kind)?, branch, worktree, head_oid, merged_into_primary })
    }
}
```

In `src/repository/read/comments.rs`:

```rust
/// A comment a branch changed on this item, outside its effective context.
struct ContributedComment {
    context: StoredContext,
    path: String,
    id: String,
}

/// The comments stored as contributed under rows of item `id` in contexts
/// other than the one at `effective`, from `changed` and `comments` rows,
/// by context then path.
fn contributed_comments(
    connection: &Connection,
    repo: &ResolvedRepository,
    id: &str,
    effective: &str,
) -> Result<Vec<ContributedComment>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT contexts.kind, contexts.branch, contexts.worktree_path, contexts.head_oid,
                contexts.merged_into_primary, comments.canonical_path, comments.comment_id
           FROM discovered_comments AS comments
           JOIN discovered_items AS items ON items.id = comments.item_id
           JOIN contexts ON contexts.id = items.context_id
          WHERE contexts.repository_id = ?1 AND items.item_id = ?2
            AND items.role IN ('changed', 'comments') AND comments.contributed = 1
            AND contexts.worktree_path != ?3
          ORDER BY contexts.worktree_path, comments.canonical_path",
    )?;
    let rows = statement
        .query_map(rusqlite::params![repo.registration_id(), id, effective], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<bool>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(kind, branch, worktree, head_oid, merged, path, id)| {
            Ok(ContributedComment {
                context: StoredContext::from_stored(&kind, branch, worktree, head_oid, merged)?,
                path,
                id,
            })
        })
        .collect()
}
```

In `list_comments`, after the loop that reads the effective context's files
into `sources` and before `canonical::validate_context(sources)`:

```rust
            // Comments branches changed on this item. Each is read from its
            // own context. One whose ID the effective context also has is a
            // version of that comment. One with a new ID joins the list,
            // read from where it lives.
            let effective_ids: BTreeSet<&str> = indexed.iter().map(|comment| comment.id.as_str()).collect();
            let mut versions: BTreeMap<String, Vec<ItemVersionDto>> = BTreeMap::new();
            let mut origin: BTreeMap<String, StoredContext> = BTreeMap::new();
            for contributed in contributed_comments(connection, repo, &item.to_string(), worktree)? {
                if effective_ids.contains(contributed.id.as_str()) {
                    versions.entry(contributed.id.clone()).or_default().push(ItemVersionDto {
                        context: contributed.context.dto(),
                        path: contributed.path.clone(),
                        change: ItemChange::Modified,
                        problems: Vec::new(),
                    });
                    continue;
                }
                match read_item_file(repo, &contributed.context.worktree, &contributed.path)? {
                    ItemFileRead::Found(file) => {
                        is_behind |= file.newer_than(refreshed_at);
                        match String::from_utf8(file.bytes) {
                            Ok(source) if !origin.contains_key(&contributed.id) => {
                                origin.insert(contributed.id.clone(), contributed.context.clone());
                                sources.push((PathBuf::from(&contributed.path), source));
                            }
                            // A second branch's copy of a new comment is a version of the first.
                            Ok(_) => versions.entry(contributed.id.clone()).or_default().push(ItemVersionDto {
                                context: contributed.context.dto(),
                                path: contributed.path.clone(),
                                change: ItemChange::Added,
                                problems: vec![problem(ProblemCode::DuplicateId, &contributed.path)],
                            }),
                            Err(_) => complete = false,
                        }
                    }
                    ItemFileRead::Missing => is_behind = true,
                    ItemFileRead::NotAFile => complete = false,
                }
            }
```

Every comment has a path under `.manyhands/comments/<item>/` with its own ID
as the file name. Two contexts' copies of a new comment therefore share a
path, and only the first goes into `sources`, so `validate_context` sees no
duplicate path.

Then, where the entries are turned into `CommentDto`s, set `context` to
`origin.get(id).map_or_else(|| row.context.dto(), StoredContext::dto)` and
`versions` to `versions.remove(id).unwrap_or_default()`. The thread order and
depth come from `parent_id` and `created_at`, as today, so a branch's reply
sits under its primary parent.

Variable names here (`indexed`, `worktree`, `is_behind`, `complete`,
`sources`, `refreshed_at`, `item`, `row`) are the ones `list_comments` uses
today (`src/repository/read/comments.rs:258-380`). If Task 0 found them
renamed, use the new names.

- [ ] **Step 5: Run the tests to see them pass**

Run: `devenv shell -- cargo test --locked --test context_contributions`
Expected: PASS.

- [ ] **Step 6: Regenerate goldens, check, and run the comment tests**

Run: `MANYHANDS_UPDATE_GOLDEN=1 devenv shell -- cargo test --locked --test read_contract --test read_comments --test cli`,
then review `git diff tests/fixtures`. Only the added comment `context` and
`versions` fields may change. Then run, without the variable:
`devenv shell -- cargo test --locked --test read_comments --test read_contract --test cli`.
Expected: PASS, apart from pinned one-item tests. The candidates are
`tests/read_comments.rs:484` and `:1247`; handle them as in Task 4 Step 9.

- [ ] **Step 7: Commit**

```bash
git add src/repository/read schemas/v1/comment.schema.json tests/context_contributions.rs tests/fixtures
git commit -m "feat: list a branch's comments on an item with their context" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: CLI: `show --id … --context`

**Files:**
- Modify: `src/repository/read/items.rs` (`show_item_in`)
- Modify: `src/bin/manyhands-cli/parse.rs`, `src/bin/manyhands-cli/dispatch.rs`
- Modify: `src/bin/manyhands-cli/` usage text for `document show` and
  `ticket show` (find it with `grep -rn "requires --path\|--context" src/bin/manyhands-cli`)
- Modify: `tests/cli/content.rs`

**Interfaces:**
- Consumes: `StoredRole`, `StoredPrimary` (Task 5), `read_path`.
- Produces: `pub fn show_item_in(&self, repo: &ResolvedRepository, id: &ItemId, context: &Path) -> Result<ItemDto, ReadError>`.

- [ ] **Step 1: Write the failing test**

In `tests/cli/content.rs`:

```rust
#[test]
fn show_by_id_with_a_context_reads_that_contexts_copy() {
    let (fixture, enabled) = items::contract_repository();
    items::create_document_context(&enabled.service, &fixture.root, items::DOCUMENT_B, "docs/other.md");
    let worktree = items::context_worktree(&fixture.root, items::DOCUMENT_B);
    // The branch edits primary's DOCUMENT_A at docs/guide.md.
    items::write(&worktree, "docs/guide.md", &items::document_source(items::DOCUMENT_A, "Guide, edited", ""));
    items::refresh_completely(&enabled.service, &fixture.root);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let env = CliEnv::new(enabled.data_directory.path().to_owned());
    let effective = checked(&env, &fixture.root, &["document", "show", "--id", items::DOCUMENT_A], "ok", 0, Some("item.schema.json"), item_scope(repo.scope(), items::DOCUMENT_A));
    assert_eq!(effective["data"]["context"]["kind"], "primary");
    assert_eq!(effective["data"]["versions"][0]["change"], "modified");
    // The context's own item lists what else its branch changed.
    let own = checked(&env, &fixture.root, &["document", "show", "--id", items::DOCUMENT_B], "ok", 0, Some("item.schema.json"), item_scope(repo.scope(), items::DOCUMENT_B));
    assert_eq!(own["data"]["context_changes"][0]["id"], items::DOCUMENT_A);
    assert_eq!(own["data"]["context_changes"][0]["change"], "modified");
    assert_eq!(own["data"]["context_changes"][0]["path"], "docs/guide.md");
    let version = checked(
        &env,
        &fixture.root,
        &["document", "show", "--id", items::DOCUMENT_A, "--context", worktree.to_str().unwrap()],
        "ok",
        0,
        Some("item.schema.json"),
        item_scope(repo.scope(), items::DOCUMENT_A),
    );
    assert_eq!(version["data"]["title"], "Guide, edited");
    assert_eq!(version["data"]["context"]["worktree"], worktree.to_str().unwrap());
    // A context with no copy of the item: not found.
    let missing = checked(
        &env,
        &fixture.root,
        &["document", "show", "--id", items::DOCUMENT_C, "--context", worktree.to_str().unwrap()],
        "error",
        // Use the exit status the contract gives item_not_found.
        3,
        None,
        item_scope(repo.scope(), items::DOCUMENT_C),
    );
    assert_eq!(missing["code"], "item_not_found");
}
```

Use `contract_repository`'s real document IDs and paths. The test at
`tests/cli/content.rs:140-180` names `DOCUMENT_A` at `docs/guide.md`. Use
`checked`'s real signature for an error result: copy it from an existing
`item_not_found` test in `tests/cli/`.

- [ ] **Step 2: Run the test to see it fail**

Run: `devenv shell -- cargo test --locked --test cli show_by_id_with_a_context`
Expected: FAIL with the usage error `--context requires --path.`

- [ ] **Step 3: Implement**

In `src/repository/read/items.rs`, after `show_path`:

```rust
    /// Item `id`'s copy in `context`, whether it is the effective copy or a
    /// version. `context` is the repository root or an item context's
    /// worktree, as for `show_path`. Naming the root for an item read from
    /// its own worktree reads primary's copy at the path the refresh found
    /// it. A context with no copy of the item, including one that deleted
    /// it, is `item_not_found`.
    pub fn show_item_in(
        &self,
        repo: &ResolvedRepository,
        id: &ItemId,
        context: &Path,
    ) -> Result<ItemDto, ReadError> {
        let wanted = id.to_string();
        let worktree = named_worktree(repo, Some(context)).map_err(|error| repo.failure(error))?;
        let path = self
            .read_session(RepositoryOperation::Read, |connection| {
                let stored = stored_items(connection, repo)?;
                let here = stored.iter().find(|row| {
                    row.id == wanted && Path::new(&row.context.worktree) == worktree.as_path()
                });
                if let Some(row) = here {
                    return Ok(row.path.clone());
                }
                let at_root = Path::new(repo.root()) == worktree.as_path();
                let (rows, _) = effective_rows(repo, &stored);
                rows.into_iter()
                    .find(|row| row.id == wanted)
                    .filter(|_| at_root)
                    .and_then(|row| row.primary.as_ref()?.path.clone())
                    .ok_or_else(|| item_not_found(repo, false))
            })
            .map_err(|error| {
                let mut error = repo.failure(error);
                error.scope.item_id = Some(wanted.clone());
                error
            })?;
        self.show_path(repo, Some(&worktree), Path::new(&path))
    }
```

Use whatever `named_worktree` returns: the type `read_path` passes to
`named_context`, converted to the same `Path`. Keep the comparison
consistent with how `read_path` resolves `context`.

In `parse.rs`, delete the `--context requires --path.` check. In
`dispatch.rs`'s `show`:

```rust
    let context = args.get_one::<String>("context").map(PathBuf::from);
    let data = if let Some(path) = args.get_one::<String>("path") {
        service.show_path(&repo, context.as_deref(), Path::new(path))?
    } else {
        let id = item_id(args, &repo)?;
        scope.item_id = Some(id.to_string());
        match context.as_deref() {
            Some(context) => service.show_item_in(&repo, &id, context)?,
            None => service.show_item(&repo, &id)?,
        }
    };
```

Update the `--context` help text: "with --path, the context to read the path
in; with --id, the context whose copy of the item to read".

- [ ] **Step 4: Run the CLI tests**

Run: `devenv shell -- cargo test --locked --test cli --bin manyhands-cli`
Expected: PASS. If a test asserted the old usage error, change it to assert
the new behaviour and name it in the commit message.

- [ ] **Step 5: Commit**

```bash
git add src/repository/read/items.rs src/bin/manyhands-cli tests/cli
git commit -m "feat: show an item's copy in a named context by ID" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Desktop

**Files:**
- Modify: `src/desktop/model/item.rs`
- Modify: `src/desktop/view/item/details.rs`
- Modify: `src/desktop/view/documents.rs`
- Modify: `src/desktop/view/item_tests.rs`, `src/desktop/model/item_tests.rs`
  (create it if absent, with the `#[path]` test pattern the other models
  use)

**Interfaces:**
- Consumes: `ItemDto.versions`, `ItemDto.primary_comparison`,
  `ItemDto.context_changes`, `ItemContextDto.merged_into_primary` (Task 5);
  `ItemLocator::Inspect`.
- Produces:
  - `pub(crate) fn contributed_from(item: &ItemDto) -> Option<&str>`;
  - `pub(crate) fn version_label(version: &ItemVersionDto) -> String`;
  - `pub(crate) fn primary_comparison_label(comparison: &PrimaryComparisonDto, merged: Option<bool>) -> String`;
  - `pub(crate) fn context_change_label(change: &ContextChangeDto) -> String`.

- [ ] **Step 1: Write the failing model tests**

```rust
use manyhands::repository::read::{ItemChange, ItemContextDto, ItemContextKind, ItemVersionDto, PrimaryComparisonDto, PrimaryComparisonState};

use super::*;

fn context(kind: ItemContextKind, branch: Option<&str>) -> ItemContextDto {
    ItemContextDto { kind, branch: branch.map(str::to_owned), worktree: "/w".to_owned(), head_oid: None, merged_into_primary: None }
}

#[test]
fn a_document_on_another_items_branch_is_marked_with_that_branch() {
    let mut item = crate::desktop::test_support::item_dto("01ARZ3NDEKTSV4RRFFQ69G5FD9");
    item.context = context(ItemContextKind::Active, Some("manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FC0"));
    assert_eq!(contributed_from(&item), Some("manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FC0"));
    item.context = context(ItemContextKind::Active, Some("manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FD9"));
    assert_eq!(contributed_from(&item), None, "its own worktree");
    item.context = context(ItemContextKind::Primary, Some("main"));
    assert_eq!(contributed_from(&item), None);
}

#[test]
fn a_version_is_labelled_with_its_change_branch_and_path() {
    let version = ItemVersionDto {
        context: context(ItemContextKind::Active, Some("manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FC0")),
        path: "docs/a.md".to_owned(),
        change: ItemChange::Modified,
        problems: Vec::new(),
    };
    assert_eq!(version_label(&version), "Modified on manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FC0, docs/a.md");
    let primary = ItemVersionDto { context: context(ItemContextKind::Primary, Some("main")), change: ItemChange::Primary, ..version };
    assert_eq!(version_label(&primary), "Primary copy on main, docs/a.md");
}

#[test]
fn the_primary_comparison_says_what_and_against_which_commit() {
    let comparison = PrimaryComparisonDto { state: PrimaryComparisonState::Different, compared_head_oid: Some("0123456789abcdef".to_owned()) };
    assert_eq!(primary_comparison_label(&comparison, Some(false)), "Differs from primary at 0123456");
    assert_eq!(primary_comparison_label(&comparison, Some(true)), "Differs from primary at 0123456; this branch is merged");
    let absent = PrimaryComparisonDto { state: PrimaryComparisonState::Absent, compared_head_oid: None };
    assert_eq!(primary_comparison_label(&absent, Some(false)), "Not in primary");
}

#[test]
fn a_context_change_is_labelled_with_its_change_path_and_title() {
    let change = ContextChangeDto {
        id: "01ARZ3NDEKTSV4RRFFQ69G5FD9".to_owned(),
        kind: ItemDtoKind::Document,
        title: Some("Design".to_owned()),
        path: "docs/plans/design.md".to_owned(),
        change: ItemChange::Added,
        effective: true,
    };
    assert_eq!(context_change_label(&change), "Added docs/plans/design.md: Design");
    let deleted = ContextChangeDto { title: None, change: ItemChange::Deleted, effective: false, ..change };
    assert_eq!(context_change_label(&deleted), "Deleted docs/plans/design.md");
}
```

Add `ContextChangeDto` and `ItemDtoKind` to the test module's `use` line.

If `test_support` has no `item_dto` builder, build an `ItemDto` the way the
existing desktop model tests do (`grep -rn "ItemDto {" src/desktop`).

- [ ] **Step 2: Run them to see them fail**

Run: `devenv shell -- cargo test --locked --features desktop,desktop-test-support --lib desktop::model::item`
Expected: compile errors for the four functions.

- [ ] **Step 3: Implement the model functions**

In `src/desktop/model/item.rs`:

```rust
/// The branch an item was read from when that branch belongs to another
/// item: a document a ticket's branch added, say. `None` for primary and
/// for an item's own worktree.
pub(crate) fn contributed_from(item: &ItemDto) -> Option<&str> {
    if item.context.kind != ItemContextKind::Active {
        return None;
    }
    let branch = item.context.branch.as_deref()?;
    let owner = branch.rsplit('/').next()?;
    (item.id.as_deref() != Some(owner)).then_some(branch)
}

pub(crate) fn version_label(version: &ItemVersionDto) -> String {
    let change = match version.change {
        ItemChange::Added => "Added",
        ItemChange::Modified => "Modified",
        ItemChange::Moved => "Moved",
        ItemChange::Deleted => "Deleted",
        ItemChange::Primary => "Primary copy",
    };
    let branch = version.context.branch.as_deref().unwrap_or(NO_BRANCH);
    format!("{change} on {branch}, {}", version.path)
}

pub(crate) fn primary_comparison_label(comparison: &PrimaryComparisonDto, merged: Option<bool>) -> String {
    let short = |oid: &Option<String>| oid.as_deref().map(|oid| oid.chars().take(7).collect::<String>()).unwrap_or_default();
    let text = match comparison.state {
        PrimaryComparisonState::Absent => "Not in primary".to_owned(),
        PrimaryComparisonState::Identical => format!("Same as primary at {}", short(&comparison.compared_head_oid)),
        PrimaryComparisonState::Different => format!("Differs from primary at {}", short(&comparison.compared_head_oid)),
    };
    match merged {
        Some(true) => format!("{text}; this branch is merged"),
        _ => text,
    }
}

/// One line per item a worktree's branch changed besides its own item.
pub(crate) fn context_change_label(change: &ContextChangeDto) -> String {
    let what = match change.change {
        ItemChange::Added => "Added",
        ItemChange::Modified => "Modified",
        ItemChange::Moved => "Moved",
        ItemChange::Deleted => "Deleted",
        // A context change is never primary's copy.
        ItemChange::Primary => "Changed",
    };
    match &change.title {
        Some(title) => format!("{what} {}: {title}", change.path),
        None => format!("{what} {}", change.path),
    }
}
```

- [ ] **Step 4: Write the failing view tests**

In `src/desktop/view/item_tests.rs`, using the file's `shell`, `open`,
`ticket` and `shown` helpers:

```rust
/// A primary ticket that another ticket's branch edited lists that version,
/// and opening it shows the branch's copy read-only.
#[gpui_kit::test]
fn an_item_changed_on_another_branch_lists_that_version(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, PRIMARY, "In primary", "");
        fixture.commit_ticket(root, SAVED, "To be saved", "");
    });
    shell.fixture.save_ticket(&root, SAVED, "Saved");
    let worktree = root.join(".manyhands/worktrees").join(SAVED);
    let path = format!(".manyhands/tickets/{PRIMARY}/ticket.md");
    let edited = std::fs::read_to_string(root.join(&path)).unwrap().replace("In primary", "Edited on the branch");
    std::fs::write(worktree.join(&path), edited).unwrap();
    shell.fixture.refresh(&root);

    open(&shell, ticket(&root, PRIMARY), cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("In primary"));
    let item = shown(&shell, &root, PRIMARY);
    assert_eq!(shell.rows("item-version", cx), item.versions.iter().map(version_label).collect::<Vec<_>>());
    shell.click("item-version-0", cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Edited on the branch"));
    assert_eq!(shell.label("provenance-worktree", cx), Some(key(&worktree)));
}

/// A ticket read from its own worktree lists the documents its branch
/// added or edited, and each opens read-only from that worktree: a Cycle's
/// design and plan reviewed from its ticket.
#[gpui_kit::test]
fn a_ticket_lists_what_else_its_branch_changed(cx: &mut TestAppContext) {
    let (shell, root) = shell(cx, |fixture, root| {
        fixture.commit_ticket(root, SAVED, "To be saved", "");
    });
    shell.fixture.save_ticket(&root, SAVED, "Saved");
    let worktree = root.join(".manyhands/worktrees").join(SAVED);
    let design = "docs/plans/design.md";
    std::fs::create_dir_all(worktree.join("docs/plans")).unwrap();
    std::fs::write(
        worktree.join(design),
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FD9\ntitle: Design\n---\n# Design\n",
    )
    .unwrap();
    shell.fixture.refresh(&root);

    open(&shell, ticket(&root, SAVED), cx);
    let item = shown(&shell, &root, SAVED);
    assert_eq!(shell.rows("context-change", cx), item.context_changes.iter().map(context_change_label).collect::<Vec<_>>());
    assert_eq!(shell.rows("context-change", cx), ["Added docs/plans/design.md: Design"]);
    shell.click("context-change-0", cx);
    assert_eq!(shell.label("item-title", cx).as_deref(), Some("Design"));
    assert_eq!(shell.label("provenance-worktree", cx), Some(key(&worktree)));
}
```

`shell.rows` and `shell.click` stand for the harness's row and click
helpers: use the names `documents_tests.rs` and `tabs_tests.rs` use.

- [ ] **Step 5: Implement the views**

- `src/desktop/view/item/details.rs`, in the provenance section after the
  head-commit field:
  - when `item.primary_comparison` is `Some`, add
    `field("provenance-primary", "Primary", primary_comparison_label(c, item.context.merged_into_primary), cx)`;
  - when `item.versions` is not empty, add a `v_flex` with id
    `"versions-section"`: a heading "Also changed in", then one clickable
    row per version, with id `("item-version", ix)` and text
    `version_label(version)`. A row opens
    `ItemLocator::Inspect { worktree: version.context.worktree.clone(), path: version.path.clone() }.in_repository(repo)`
    through the same open action other provenance links use. A `Deleted`
    version is listed but not clickable, because there is no file to open.
  - when `item.context_changes` is not empty, add a `v_flex` with id
    `"context-changes-section"`: a heading "Changed on this branch", then one
    clickable row per change, with id `("context-change", ix)` and text
    `context_change_label(change)`. A row opens
    `ItemLocator::Inspect { worktree: item.context.worktree.clone(), path: change.path.clone() }.in_repository(repo)`
    in the same way. A `Deleted` change is listed but not clickable.
- `src/desktop/view/documents.rs`: when `contributed_from(item)` is
  `Some(branch)`, the document node's label is `"{title} · {branch}"`. Make
  the same change in `flatten` in `documents_tests.rs`, if it builds labels
  itself, so `the_tree_shows_what_the_library_reads_give` still compares
  like with like.

The item view is read-only until D3. If D3 has added editing by the time
this runs, an `Inspect` target must open read-only. Check D3's tab mode for
inspected items and keep it read-only.

- [ ] **Step 6: Run the desktop tests**

Run: `devenv shell -- cargo test --locked --features desktop,desktop-test-support --lib desktop::`
Expected: PASS, with the desktop test count up by six.

Run: `devenv shell -- cargo clippy --features desktop --locked -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/desktop
git commit -m "feat: show a branch's documents and an item's other copies in the desktop" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: RFCs, benchmark, gates and review-ready

**Files:**
- Modify: `docs/RFC/repository-index-persistence-and-refresh.md`
- Modify: `docs/RFC/git-workflow-and-conflict-recovery.md`
- Modify: `docs/RFC/cli-contract.md`
- Modify: `docs/RFC/desktop-information-architecture-and-editor.md`
- Modify: the design doc's `status`
- Create: ticket comments

- [ ] **Step 1: Amend the index RFC**

Under "Index-Only Refresh Algorithm", add a dated block:

```markdown
Amended 2026-10-23 by ticket `01M5NKA3MY7CW4D90M866YS946` (reverses the
2026-10-07 decision of ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G`): an active item
context contributes its own item and comments, and also every managed file
its branch changed. A file is changed when its bytes on disk, committed or
not, differ from the merge base of the context's HEAD and primary's HEAD,
and also from primary's current copy. Unchanged copies stay primary's. The
diff selects contributions; it is not incremental indexing, and primary is
still scanned in full.

One copy per item ID is effective: its own context's, else primary's, else
the copy on the branch whose ULID sorts first. Every other copy, and each
deletion on a branch, is a version that reads list. An item read from its
own worktree says how it compares with primary's copy and against which
primary commit. Versions are read-only, and the effective copy is
deterministic. A consumer is therefore still never required to choose a
context. Editing a version (Level 2) must amend this paragraph.
```

- [ ] **Step 2: Amend the Git workflow RFC**

Under "Context Branches and Worktrees":

```markdown
Amended 2026-10-23 by ticket `01M5NKA3MY7CW4D90M866YS946`: a context still
owns exactly one item, but its branch may carry changes to other items,
such as a Cycle's documents beside its ticket. They are indexed as versions
and merge with the branch. Scoped checkpoints are unchanged and stage only
the event's own paths; staging a contributed item's paths is Level 2 work.
```

- [ ] **Step 3: Amend the CLI contract**

After the 2026-10-20 amendment line under the command list:

```markdown
Amended 2026-10-23 by ticket `01M5NKA3MY7CW4D90M866YS946`: `document show`
and `ticket show` accept `--context` with `--id`, reading that context's
copy of the item, effective or not. Naming the root for an item read from its
own worktree reads primary's copy. Item DTOs gain `versions`,
`primary_comparison` and `context_changes`. The last lists, for an item read
from its own worktree, the other documents and tickets its branch changed.
Item contexts gain `merged_into_primary`, comments gain `context` and
`versions`, and folders gain `context`. All are additions within v1. Two problem codes are added: `context_changes_unavailable` and
`context_changes_over_budget`.
```

Change the `show` rows (lines ~119 and ~126) from "Show takes `--id` or
`--path` with optional `--context`" to "Show takes `--id` or `--path`, each
with optional `--context`". In the C1 amendment, leave "only `--path`
accepts" as history; the new amendment supersedes it.

- [ ] **Step 4: Amend the desktop RFC**

Under "Navigation and item identity":

```markdown
Amended 2026-10-23 by ticket `01M5NKA3MY7CW4D90M866YS946`: the document tree
lists documents that exist only on another item's branch, labelled with that
branch. An item's provenance lists its other copies under "Also changed in"
and, when read from its own worktree, how it compares with primary. An item
read from its own worktree also lists, under "Changed on this branch", the
other documents and tickets its branch changed, so a Cycle's documents can be
reviewed from its ticket. A copy opens read-only, labelled with its context.
```

- [ ] **Step 5: Run the benchmark after the change**

Run the same command and cells as Task 1 Step 2, on the same machine.
Record both tables in a ticket comment with the ratio for each cell. If any
cell is more than twice as slow as its baseline, stop. Report it to the
owner with the numbers; this is the trigger the spec names for a
per-repository flag.

- [ ] **Step 6: Run every gate**

```bash
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo check --features desktop --locked
devenv shell -- cargo clippy --features desktop --locked -- -D warnings
devenv shell -- cargo run --locked --bin manyhands-cli
```

Expected: all pass. Record the outputs' summary lines in a ticket comment.

- [ ] **Step 7: Check against main**

```bash
git fetch origin
git merge-tree --write-tree HEAD origin/main
```

Expected: a tree OID and no conflict lines. If main moved, merge it (never
rebase) and repeat Step 6.

- [ ] **Step 8: Mark review-ready**

- Set the design doc's `status` to `"implemented, awaiting review"`.
- Add a review-ready ticket comment with:
  - the commit range;
  - the benchmark comparison;
  - each pinned test that changed, and why;
  - the gate results.
- Update `01M4EPRY203W` with a comment pointing at the review-ready commit.
- Commit as `docs: record review-ready evidence for 01M5NKA3`, and push the
  ticket branch.

Do not close either ticket. Closing waits for review approval (AGENTS.md).
