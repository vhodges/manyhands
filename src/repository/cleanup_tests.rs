use super::*;
use crate::repository::*;
use git2::{Oid, Repository};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const OTHER: &str = "01BX5ZZKBKACTAV9WEVGEMMVRZ";

struct Fixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    repo: Repository,
    linked: Repository,
    kind: AuthoringKind,
    oid: Oid,
    doc: PathBuf,
}
fn commit(repo: &Repository) -> Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let sig = git2::Signature::new(
        "Fixture",
        "fixture@example.invalid",
        &git2::Time::new(123, 0),
    )
    .unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}
fn files(root: &Path) -> BTreeMap<PathBuf, Option<[u8; 32]>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Option<[u8; 32]>>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                out.insert(path.strip_prefix(root).unwrap().into(), None);
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    Some(*blake3::hash(&fs::read(path).unwrap()).as_bytes()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
impl Fixture {
    fn new(kind: AuthoringKind) -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let repo = Repository::init(root.path()).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        for (key, value) in [
            ("user.name", "Fixture"),
            ("user.email", "fixture@example.invalid"),
            ("core.autocrlf", "false"),
        ] {
            repo.config().unwrap().set_str(key, value).unwrap();
        }
        fs::write(root.path().join("ordinary"), b"original\n").unwrap();
        commit(&repo);
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        commit(&repo);
        let target = AuthoringTarget {
            root: root.path().into(),
            kind,
            item_id: ID.parse().unwrap(),
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        };
        let doc = match kind {
            AuthoringKind::Document => {
                service
                    .save_document(SaveDocumentRequest {
                        target,
                        source_path: None,
                        destination_path: "docs/document.md".into(),
                        draft: DocumentDraft {
                            title: "Fixture".into(),
                            body: "body\n".into(),
                        },
                        expected_source: None,
                        expected_destination: ExpectedPathObservation::Missing,
                    })
                    .unwrap();
                PathBuf::from("docs/document.md")
            }
            AuthoringKind::Ticket => {
                service
                    .save_ticket(SaveTicketRequest {
                        target,
                        draft: TicketDraft {
                            title: "Fixture".into(),
                            ticket_type: "task".into(),
                            status: "open".into(),
                            project: None,
                            team: None,
                            body: "body\n".into(),
                        },
                        expected_path: ExpectedPathObservation::Missing,
                    })
                    .unwrap();
                PathBuf::from(format!(".manyhands/tickets/{ID}/ticket.md"))
            }
        };
        let linked = Repository::open(root.path().join(".manyhands/worktrees").join(ID)).unwrap();
        let oid = linked.head().unwrap().target().unwrap();
        Self {
            root,
            data,
            service,
            repo,
            linked,
            kind,
            oid,
            doc,
        }
    }
    fn cleanup(&self) -> ContextCleanup {
        ContextCleanup::acquire(self.root.path()).unwrap()
    }
    fn precheck(&self, c: &ContextCleanup) -> Precheck {
        c.precheck_context(self.kind, &ID.parse().unwrap(), self.oid)
    }
    fn remove(&self, c: &ContextCleanup) -> WorktreeRemoval {
        c.remove_context_worktree(self.kind, &ID.parse().unwrap(), self.oid)
    }
    fn branch(&self) -> String {
        format!(
            "refs/heads/manyhands/{}/{ID}",
            authoring_kind_segment(&self.kind)
        )
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }
    fn setup_remote(&self) -> RemoteRefPlan {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        remote::state::with_transaction(&self.service, self.root.path(), |tx, id| {
            let generation = remote::state::configure(tx, id, Some(&plan), false)?;
            let target = plan.context(self.kind, &ID.parse().unwrap());
            let observation = RemoteRefObservation::from_advertisement(
                &plan,
                target.remote_ref(),
                self.oid,
                Some(self.oid),
            )
            .unwrap();
            remote::state::complete_batch(tx, id, &plan, generation, &[observation], 123)?;
            Ok(())
        })
        .unwrap();
        self.repo
            .reference(
                plan.context(self.kind, &ID.parse().unwrap()).tracking_ref(),
                self.oid,
                true,
                "fixture",
            )
            .unwrap();
        plan
    }
}

#[test]
fn cleanup_precheck_clean_and_dirty_cases_preserve_full_git_image() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for case in [
            "clean",
            "tracked",
            "untracked",
            "staged",
            "commit",
            "branch",
            "registration",
            "conflict",
            "nonconforming",
        ] {
            let f = Fixture::new(kind);
            let c = f.cleanup();
            let path = f.linked.workdir().unwrap();
            match case {
                "tracked" => fs::write(path.join("ordinary"), b"changed").unwrap(),
                "untracked" => fs::write(path.join("new"), b"keep").unwrap(),
                "staged" => {
                    fs::write(path.join("new"), b"keep").unwrap();
                    let mut i = f.linked.index().unwrap();
                    i.add_path(Path::new("new")).unwrap();
                    i.write().unwrap();
                }
                "commit" => {
                    fs::write(path.join("new"), b"keep").unwrap();
                    commit(&f.linked);
                }
                "branch" => {
                    f.repo
                        .branch("other", &f.repo.find_commit(f.oid).unwrap(), false)
                        .unwrap();
                    f.linked.set_head("refs/heads/other").unwrap();
                }
                "registration" => {
                    fs::remove_dir_all(f.repo.path().join("worktrees").join(ID)).unwrap()
                }
                "conflict" => {
                    fs::write(f.linked.path().join("MERGE_HEAD"), format!("{}\n", f.oid)).unwrap()
                }
                "nonconforming" => {
                    fs::write(path.join(&f.doc), b"not canonical").unwrap();
                    commit(&f.linked);
                }
                _ => (),
            }
            let before = files(f.root.path());
            assert_eq!(
                f.precheck(&c),
                if case == "clean" {
                    Precheck::Ok
                } else {
                    Precheck::Changed
                },
                "{case}"
            );
            assert_eq!(files(f.root.path()), before, "{case}");
            if case != "clean" {
                assert_ne!(f.remove(&c), WorktreeRemoval::Removed);
                assert_eq!(files(f.root.path()), before, "{case}");
            }
        }
    }
}

#[test]
fn cleanup_ignored_and_locked_are_preserved() {
    for case in ["ignored", "locked"] {
        let f = Fixture::new(AuthoringKind::Document);
        let c = f.cleanup();
        if case == "ignored" {
            let exclude = f.repo.path().join("info/exclude");
            let mut text = fs::read_to_string(&exclude).unwrap();
            text.push_str("\nprivate/\n");
            fs::write(exclude, text).unwrap();
            fs::create_dir(f.linked.workdir().unwrap().join("private")).unwrap();
            fs::write(
                f.linked.workdir().unwrap().join("private/keep"),
                b"preserve",
            )
            .unwrap();
        } else {
            f.repo
                .find_worktree(ID)
                .unwrap()
                .lock(Some("fixture"))
                .unwrap();
        }
        let before = files(f.root.path());
        assert_eq!(
            f.precheck(&c),
            if case == "ignored" {
                Precheck::IgnoredFiles
            } else {
                Precheck::Locked
            }
        );
        assert_eq!(f.remove(&c), WorktreeRemoval::Changed);
        assert_eq!(files(f.root.path()), before);
    }
}

fn hide_index_entry(repo: &Repository, path: &str, flag: &str) {
    let result = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.workdir().unwrap())
        .args(["update-index", flag, "--", path])
        .output()
        .unwrap();
    assert!(result.status.success(), "fixture index flag setup");
}

fn hidden_index_edit(flag: &str) {
    let f = Fixture::new(AuthoringKind::Document);
    let c = f.cleanup();
    hide_index_entry(&f.linked, "ordinary", flag);
    fs::write(
        f.linked.workdir().unwrap().join("ordinary"),
        b"hidden user edit\n",
    )
    .unwrap();
    assert!(f.linked.statuses(None).unwrap().is_empty());
    assert_eq!(
        f.linked.index().unwrap().write_tree().unwrap(),
        f.linked.head().unwrap().peel_to_tree().unwrap().id()
    );
    let index = fs::read(f.linked.path().join("index")).unwrap();
    let before = files(f.root.path());
    assert_eq!(f.precheck(&c), Precheck::Changed);
    assert_eq!(f.remove(&c), WorktreeRemoval::Changed);
    assert_eq!(fs::read(f.linked.path().join("index")).unwrap(), index);
    assert_eq!(
        fs::read(f.linked.workdir().unwrap().join("ordinary")).unwrap(),
        b"hidden user edit\n"
    );
    assert!(f.repo.find_worktree(ID).is_ok());
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn cleanup_hidden_index_assume_unchanged_preserves_edit_and_index() {
    hidden_index_edit("--assume-unchanged");
}
#[test]
fn cleanup_hidden_index_skip_worktree_preserves_edit_and_index() {
    hidden_index_edit("--skip-worktree");
}

// Build real populated submodules entirely from local objects and versioned
// .gitmodules. No clone, transport or global Git config mutation is involved.
fn add_local_submodule(parent: &Repository, name: &str) -> Repository {
    let path = parent.workdir().unwrap().join(name);
    fs::create_dir_all(&path).unwrap();
    let child = Repository::init(&path).unwrap();
    fs::write(path.join("ordinary"), b"submodule original\n").unwrap();
    let oid = commit(&child);
    fs::write(parent.workdir().unwrap().join(".gitmodules"), format!("[submodule \"{name}\"]\n\tpath = {name}\n\turl = ./local-fixture-only\n\tignore = all\n")).unwrap();
    let mut index = parent.index().unwrap();
    index.add_path(Path::new(".gitmodules")).unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o160000,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: oid,
            flags: name.len() as u16,
            flags_extended: 0,
            path: name.as_bytes().to_vec(),
        })
        .unwrap();
    index.write().unwrap();
    commit(parent);
    assert_eq!(
        parent
            .head()
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .get_path(Path::new(name))
            .unwrap()
            .filemode(),
        0o160000
    );
    assert!(parent.find_submodule(name).unwrap().open().is_ok());
    child
}

fn submodule_fixture(
    depth: usize,
    external_git: bool,
) -> (Fixture, Repository, Option<Repository>) {
    let mut f = Fixture::new(AuthoringKind::Document);
    let mut child = add_local_submodule(&f.linked, "module");
    let mut nested = if depth == 2 {
        Some(add_local_submodule(&child, "nested"))
    } else {
        None
    };
    if external_git {
        if let Some(repo) = nested.take() {
            let path = repo.workdir().unwrap().to_owned();
            let metadata = f.data.path().join("nested-git");
            drop(repo);
            fs::rename(path.join(".git"), &metadata).unwrap();
            fs::write(
                path.join(".git"),
                format!("gitdir: {}", remote::git_metadata_link_text(&metadata)),
            )
            .unwrap();
            nested = Some(Repository::open(&path).unwrap());
        }
        let path = child.workdir().unwrap().to_owned();
        let metadata = f.data.path().join("module-git");
        drop(child);
        fs::rename(path.join(".git"), &metadata).unwrap();
        fs::write(
            path.join(".git"),
            format!("gitdir: {}", remote::git_metadata_link_text(&metadata)),
        )
        .unwrap();
        child = Repository::open(&path).unwrap();
    }
    // Include the nested gitlink advance in the superproject's recorded commit.
    let mut index = f.linked.index().unwrap();
    let mut entry = index.get_path(Path::new("module"), 0).unwrap();
    entry.id = child.head().unwrap().target().unwrap();
    index.add(&entry).unwrap();
    index.write().unwrap();
    f.oid = commit(&f.linked);
    (f, child, nested)
}

#[test]
fn cleanup_submodule_ignored_files_are_preserved_recursively() {
    for depth in [1, 2] {
        let (f, child, nested) = submodule_fixture(depth, true);
        let c = f.cleanup();
        let leaf = nested.as_ref().unwrap_or(&child);
        fs::write(leaf.path().join("info/exclude"), b"private\n").unwrap();
        fs::write(
            leaf.workdir().unwrap().join("private"),
            b"ignored user data",
        )
        .unwrap();
        let mut options = git2::StatusOptions::new();
        options
            .include_ignored(true)
            .recurse_ignored_dirs(true)
            .include_untracked(true);
        assert!(f.linked.statuses(Some(&mut options)).unwrap().is_empty());
        let before = files(f.root.path());
        let data_before = files(f.data.path());
        assert_eq!(f.precheck(&c), Precheck::IgnoredFiles, "depth {depth}");
        assert_eq!(f.remove(&c), WorktreeRemoval::Changed);
        assert_eq!(files(f.root.path()), before);
        assert_eq!(files(f.data.path()), data_before);
    }
}

#[test]
fn cleanup_submodule_hidden_dirty_files_are_preserved_recursively() {
    for depth in [1, 2] {
        for change in ["untracked", "tracked", "assume_unchanged", "skip_worktree"] {
            let (f, child, nested) = submodule_fixture(depth, true);
            let c = f.cleanup();
            let leaf = nested.as_ref().unwrap_or(&child);
            if change == "assume_unchanged" {
                hide_index_entry(leaf, "ordinary", "--assume-unchanged");
            }
            if change == "skip_worktree" {
                hide_index_entry(leaf, "ordinary", "--skip-worktree");
            }
            fs::write(
                leaf.workdir().unwrap().join(if change == "untracked" {
                    "new"
                } else {
                    "ordinary"
                }),
                b"hidden submodule user data",
            )
            .unwrap();
            assert!(f.linked.statuses(None).unwrap().is_empty());
            let before = files(f.root.path());
            let data_before = files(f.data.path());
            assert_eq!(f.precheck(&c), Precheck::Changed, "{change} depth {depth}");
            assert_eq!(f.remove(&c), WorktreeRemoval::Changed);
            assert_eq!(files(f.root.path()), before);
            assert_eq!(files(f.data.path()), data_before);
        }
    }
}

#[test]
fn cleanup_submodule_clean_recursive_control_preserves_external_git_metadata() {
    let (f, child, nested) = submodule_fixture(2, true);
    let c = f.cleanup();
    assert!(!leases_are_free(f.data.path(), f.root.path()));
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    // Snapshot between SQL probes: opening/closing the test connection itself
    // may checkpoint/remove SQLite WAL/SHM files, independently of cleanup.
    let data_before = files(f.data.path());
    assert_eq!(f.precheck(&c), Precheck::Ok);
    assert!(!leases_are_free(f.data.path(), f.root.path()));
    assert_eq!(f.remove(&c), WorktreeRemoval::Removed);
    assert_eq!(files(f.data.path()), data_before);
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM remote_operation_records", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    assert!(child.path().is_dir());
    assert!(nested.unwrap().path().is_dir());
}

#[test]
fn cleanup_submodule_unknown_or_advanced_state_is_not_safe_to_remove() {
    for case in ["advanced", "missing_git_link"] {
        let (f, child, nested) = submodule_fixture(2, true);
        let c = f.cleanup();
        let leaf = nested.as_ref().unwrap_or(&child);
        if case == "advanced" {
            fs::write(
                leaf.workdir().unwrap().join("ordinary"),
                b"unpublished submodule commit",
            )
            .unwrap();
            commit(leaf);
        } else {
            fs::remove_file(leaf.workdir().unwrap().join(".git")).unwrap();
        }
        let before = files(f.root.path());
        let data_before = files(f.data.path());
        assert_eq!(f.precheck(&c), Precheck::Changed, "{case}");
        assert_eq!(f.remove(&c), WorktreeRemoval::Changed);
        assert_eq!(files(f.root.path()), before);
        assert_eq!(files(f.data.path()), data_before);
    }
}

#[test]
fn cleanup_submodule_embedded_git_data_is_not_proved_safe_to_remove() {
    let (f, _child, _nested) = submodule_fixture(1, false);
    let c = f.cleanup();
    let before = files(f.root.path());
    assert_eq!(f.precheck(&c), Precheck::Changed);
    assert_eq!(f.remove(&c), WorktreeRemoval::Changed);
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn cleanup_raw_prune_has_no_submodule_ignored_file_safety() {
    let (f, child, _nested) = submodule_fixture(1, true);
    fs::write(child.path().join("info/exclude"), b"private\n").unwrap();
    let file = child.workdir().unwrap().join("private");
    fs::write(&file, b"throwaway ignored fixture data").unwrap();
    let mut options = git2::WorktreePruneOptions::new();
    options.valid(true).working_tree(true).locked(false);
    f.repo
        .find_worktree(ID)
        .unwrap()
        .prune(Some(&mut options))
        .unwrap();
    assert!(!file.exists());
    assert!(!f.root.path().join(".manyhands/worktrees").join(ID).exists());
    assert!(f.repo.find_worktree(ID).is_err());
    assert!(child.path().exists());
}

#[test]
fn cleanup_removal_removes_only_owned_worktree_and_is_idempotent() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let c = f.cleanup();
        let other = f.root.path().join("other-worktree");
        f.repo.worktree(OTHER, &other, None).unwrap();
        let other_before = files(&other);
        let other_admin = f.repo.commondir().join("worktrees").join(OTHER);
        let other_admin_before = files(&other_admin);
        assert_eq!(f.remove(&c), WorktreeRemoval::Removed);
        assert!(!f.root.path().join(".manyhands/worktrees").join(ID).exists());
        assert!(!f.repo.path().join("worktrees").join(ID).exists());
        assert!(f.repo.find_worktree(ID).is_err());
        assert_eq!(f.repo.refname_to_id(&f.branch()).unwrap(), f.oid);
        assert_eq!(files(&other), other_before);
        assert_eq!(files(&other_admin), other_admin_before);
        assert!(f.repo.find_worktree(OTHER).is_ok());
        assert_eq!(f.remove(&c), WorktreeRemoval::AlreadyAbsent);
    }
}

#[test]
fn cleanup_partial_and_mixed_worktrees_require_recovery_without_more_removal() {
    for case in [
        "tracked_missing",
        "git_missing",
        "path_missing",
        "registration_missing",
    ] {
        let f = Fixture::new(AuthoringKind::Document);
        let c = f.cleanup();
        match case {
            "tracked_missing" => {
                fs::remove_file(f.linked.workdir().unwrap().join("ordinary")).unwrap()
            }
            "git_missing" => fs::remove_file(f.linked.workdir().unwrap().join(".git")).unwrap(),
            "path_missing" => fs::remove_dir_all(f.linked.workdir().unwrap()).unwrap(),
            _ => fs::remove_dir_all(f.repo.path().join("worktrees").join(ID)).unwrap(),
        }
        let before = files(f.root.path());
        assert_eq!(f.remove(&c), WorktreeRemoval::RecoveryRequired, "{case}");
        assert_eq!(files(f.root.path()), before);
    }
}

#[test]
fn cleanup_branch_deletion_checks_oid_and_every_checkout() {
    for case in [
        "deleted",
        "wrong",
        "checked_out",
        "other_checkout",
        "primary_checkout",
        "absent",
        "symbolic",
    ] {
        let f = Fixture::new(AuthoringKind::Ticket);
        let c = f.cleanup();
        if case != "checked_out" {
            assert_eq!(f.remove(&c), WorktreeRemoval::Removed);
        }
        match case {
            "wrong" => {
                f.repo
                    .reference(
                        &f.branch(),
                        f.repo.head().unwrap().target().unwrap(),
                        true,
                        "fixture",
                    )
                    .unwrap();
            }
            "other_checkout" => {
                let reference = f.repo.find_reference(&f.branch()).unwrap();
                let mut options = git2::WorktreeAddOptions::new();
                options.reference(Some(&reference));
                f.repo
                    .worktree(OTHER, &f.root.path().join("other"), Some(&options))
                    .unwrap();
            }
            "primary_checkout" => f.repo.set_head(&f.branch()).unwrap(),
            "absent" => f
                .repo
                .find_reference(&f.branch())
                .unwrap()
                .delete()
                .unwrap(),
            "symbolic" => {
                f.repo
                    .reference_symbolic(&f.branch(), "refs/heads/main", true, "fixture")
                    .unwrap();
            }
            _ => (),
        }
        let before = files(f.root.path());
        let result = c.delete_context_branch(f.kind, &ID.parse().unwrap(), f.oid);
        assert_eq!(
            result,
            match case {
                "deleted" => BranchDeletion::Deleted,
                "absent" => BranchDeletion::AlreadyAbsent,
                _ => BranchDeletion::Changed,
            },
            "{case}"
        );
        if case != "deleted" {
            assert_eq!(files(f.root.path()), before);
        }
    }
}

fn incomplete_checkout_registration(member: &str) {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let f = Fixture::new(kind);
        let c = f.cleanup();
        assert_eq!(f.remove(&c), WorktreeRemoval::Removed);
        let reference = f.repo.find_reference(&f.branch()).unwrap();
        let mut options = git2::WorktreeAddOptions::new();
        options.reference(Some(&reference));
        f.repo
            .worktree(OTHER, &f.root.path().join("other"), Some(&options))
            .unwrap();
        let admin = f.repo.commondir().join("worktrees").join(OTHER);
        assert_eq!(
            fs::read_to_string(admin.join("HEAD")).unwrap().trim(),
            format!("ref: {}", f.branch())
        );
        fs::remove_file(admin.join(member)).unwrap();
        // Confirm the actual locked libgit2 inventory silently omits this
        // registration, even though its own HEAD still checks out our branch.
        assert!(
            !f.repo
                .worktrees()
                .unwrap()
                .iter()
                .any(|name| name == Some(OTHER))
        );
        let before = files(f.root.path());
        assert_eq!(
            c.delete_context_branch(kind, &ID.parse().unwrap(), f.oid),
            BranchDeletion::Changed,
            "incomplete {member} must not hide a checkout"
        );
        assert_eq!(f.repo.refname_to_id(&f.branch()).unwrap(), f.oid);
        assert_eq!(files(f.root.path()), before);
    }
}

#[test]
fn cleanup_branch_guard_keeps_checkout_with_missing_commondir_registration() {
    incomplete_checkout_registration("commondir");
}

#[test]
fn cleanup_branch_guard_keeps_checkout_with_missing_gitdir_registration() {
    incomplete_checkout_registration("gitdir");
}

#[test]
fn cleanup_branch_guard_requires_complete_admin_evidence() {
    for case in [
        "missing_HEAD",
        "invalid_HEAD",
        "invalid_commondir",
        "invalid_gitdir",
        "member_directory",
        "entry_file",
        "inventory_file",
    ] {
        let f = Fixture::new(AuthoringKind::Document);
        let c = f.cleanup();
        assert_eq!(f.remove(&c), WorktreeRemoval::Removed);
        f.repo
            .worktree(OTHER, &f.root.path().join("other"), None)
            .unwrap();
        let inventory = f.repo.commondir().join("worktrees");
        let admin = inventory.join(OTHER);
        match case {
            "missing_HEAD" => fs::remove_file(admin.join("HEAD")).unwrap(),
            "invalid_HEAD" => fs::write(admin.join("HEAD"), b"invalid\n").unwrap(),
            "invalid_commondir" => {
                fs::write(admin.join("commondir"), b"missing-common-dir\n").unwrap()
            }
            "invalid_gitdir" => fs::write(admin.join("gitdir"), b"missing-git-link\n").unwrap(),
            "member_directory" => {
                fs::remove_file(admin.join("commondir")).unwrap();
                fs::create_dir(admin.join("commondir")).unwrap();
            }
            "entry_file" => fs::write(
                inventory.join("unreadable-registration"),
                b"not a directory",
            )
            .unwrap(),
            _ => {
                fs::remove_dir_all(&inventory).unwrap();
                fs::write(&inventory, b"not an inventory").unwrap();
            }
        }
        let before = files(f.root.path());
        assert_eq!(
            c.delete_context_branch(f.kind, &ID.parse().unwrap(), f.oid),
            BranchDeletion::Changed,
            "{case}"
        );
        assert_eq!(f.repo.refname_to_id(&f.branch()).unwrap(), f.oid);
        assert_eq!(files(f.root.path()), before, "{case}");
    }
}

#[test]
fn cleanup_retirement_git_first_sql_atomic_and_retryable() {
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        for case in ["expected", "absent", "sql_failure"] {
            let f = Fixture::new(kind);
            let plan = f.setup_remote();
            let c = f.cleanup();
            let target = plan.context(kind, &ID.parse().unwrap());
            let op = OperationId::new();
            if case == "absent" {
                f.repo
                    .find_reference(target.tracking_ref())
                    .unwrap()
                    .delete()
                    .unwrap();
            }
            let evidence = c
                .delete_tracking_ref(
                    Some(&plan),
                    kind,
                    &ID.parse().unwrap(),
                    Some(f.oid),
                    op,
                    f.oid,
                )
                .unwrap();
            assert!(f.repo.find_reference(target.tracking_ref()).is_err());
            let db = f.db();
            assert_eq!(
                db.query_row(
                    "SELECT publication_evidence FROM remote_context_states",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "observed_published"
            );
            assert_eq!(
                db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            if case == "sql_failure" {
                db.execute_batch("CREATE TRIGGER fail_retirement BEFORE INSERT ON retired_contexts BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
                assert!(
                    remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
                        retire_context(tx, id, &evidence)
                    })
                    .is_err()
                );
                assert_eq!(
                    db.query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                        .get::<_, i64>(0))
                        .unwrap(),
                    1
                );
                assert_eq!(
                    db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                        .get::<_, i64>(0))
                        .unwrap(),
                    0
                );
                assert_eq!(
                    db.query_row("SELECT count(*) FROM remote_ref_observations", [], |r| r
                        .get::<_, i64>(0))
                        .unwrap(),
                    1
                );
                assert!(f.repo.find_reference(target.tracking_ref()).is_err());
                db.execute_batch("DROP TRIGGER fail_retirement").unwrap();
            }
            remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
                retire_context(tx, id, &evidence)
            })
            .unwrap();
            assert_eq!(
                db.query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                db.query_row(
                    "SELECT operation_id,retired_oid FROM retired_contexts",
                    [],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                )
                .unwrap(),
                (op.to_string(), f.oid.to_string())
            );
            // Discovery refresh is a later lifecycle step. A still-cached local
            // context may be synthesized, but must now have retired eligibility.
            assert_eq!(
                f.service
                    .remote_snapshot(f.root.path())
                    .unwrap()
                    .publication_evidence_for(kind, &ID.parse().unwrap()),
                RemotePublicationEvidence::NeverPublished
            );
            RepositoryService::open_at(f.data.path()).unwrap();
        }
    }
}

#[test]
fn cleanup_tracking_ref_refusals_preserve_git_and_sql() {
    for case in [
        "wrong",
        "symbolic",
        "dangling",
        "unreadable",
        "new_after_delete",
    ] {
        let f = Fixture::new(AuthoringKind::Document);
        let plan = f.setup_remote();
        let c = f.cleanup();
        let target = plan.context(f.kind, &ID.parse().unwrap());
        let operation = OperationId::new();
        let evidence = if case == "new_after_delete" {
            Some(
                c.delete_tracking_ref(
                    Some(&plan),
                    f.kind,
                    &ID.parse().unwrap(),
                    Some(f.oid),
                    operation,
                    f.oid,
                )
                .unwrap(),
            )
        } else {
            None
        };
        match case {
            "wrong" => {
                f.repo
                    .reference(
                        target.tracking_ref(),
                        f.repo.head().unwrap().target().unwrap(),
                        true,
                        "fixture",
                    )
                    .unwrap();
            }
            "new_after_delete" => {
                // Simulate a foreign writer bypassing Git's held lock entirely.
                fs::write(
                    f.repo.path().join(target.tracking_ref()),
                    format!("{}\n", f.repo.head().unwrap().target().unwrap()),
                )
                .unwrap();
            }
            "symbolic" | "dangling" => {
                f.repo
                    .reference_symbolic(
                        target.tracking_ref(),
                        if case == "symbolic" {
                            "refs/heads/main"
                        } else {
                            "refs/heads/missing"
                        },
                        true,
                        "fixture",
                    )
                    .unwrap();
            }
            _ => fs::write(f.repo.path().join(target.tracking_ref()), b"not a ref\n").unwrap(),
        }
        let before = files(f.root.path());
        if let Some(evidence) = &evidence {
            assert!(
                remote::state::with_transaction(
                    &f.service,
                    f.root.path(),
                    |tx, id| retire_context(tx, id, evidence)
                )
                .is_err()
            );
        } else {
            assert!(
                c.delete_tracking_ref(
                    Some(&plan),
                    f.kind,
                    &ID.parse().unwrap(),
                    Some(f.oid),
                    operation,
                    f.oid
                )
                .is_err()
            );
        }
        assert_eq!(files(f.root.path()), before, "{case}");
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn cleanup_fresh_edit_after_local_retirement_starts_at_primary_head() {
    let f = Fixture::new(AuthoringKind::Document);
    f.db()
        .execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    f.repo
        .reset(
            &f.repo.find_object(f.oid, None).unwrap(),
            git2::ResetType::Hard,
            None,
        )
        .unwrap();
    fs::write(f.root.path().join("later"), b"primary advance").unwrap();
    let primary = commit(&f.repo);
    {
        let c = f.cleanup();
        assert_eq!(f.remove(&c), WorktreeRemoval::Removed);
        assert_eq!(
            c.delete_context_branch(f.kind, &ID.parse().unwrap(), f.oid),
            BranchDeletion::Deleted
        );
        let evidence = c
            .delete_tracking_ref(
                None,
                f.kind,
                &ID.parse().unwrap(),
                None,
                OperationId::new(),
                f.oid,
            )
            .unwrap();
        remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
            retire_context(tx, id, &evidence)
        })
        .unwrap();
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    let result = f
        .service
        .prepare_context(AuthoringTarget {
            root: f.root.path().into(),
            kind: f.kind,
            item_id: ID.parse().unwrap(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert!(matches!(result, ContextProvisionOutcome::Created(_)));
    let fresh = Repository::open(f.root.path().join(".manyhands/worktrees").join(ID)).unwrap();
    assert_eq!(fresh.head().unwrap().target(), Some(primary));
    assert_eq!(
        fs::read(fresh.workdir().unwrap().join(&f.doc)).unwrap(),
        fs::read(f.root.path().join(&f.doc)).unwrap()
    );
    assert_eq!(
        fs::read(fresh.workdir().unwrap().join("later")).unwrap(),
        b"primary advance"
    );
}

#[test]
fn cleanup_retirement_outer_rollback_and_commit_failure_keep_sql_not_deleted_git() {
    for case in ["outer_rollback", "commit_failure"] {
        let f = Fixture::new(AuthoringKind::Document);
        let plan = f.setup_remote();
        let c = f.cleanup();
        let evidence = c
            .delete_tracking_ref(
                Some(&plan),
                f.kind,
                &ID.parse().unwrap(),
                Some(f.oid),
                OperationId::new(),
                f.oid,
            )
            .unwrap();
        let result = remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
            retire_context(tx, id, &evidence)?;
            if case == "outer_rollback" {
                return Err(remote::state::recovery_required());
            }
            tx.execute_batch("PRAGMA defer_foreign_keys=ON").unwrap();
            // Statement succeeds; the real deferred FK fails only at COMMIT.
            tx.execute(
                "INSERT INTO retired_contexts VALUES(999,'document',?1,?2,?3)",
                rusqlite::params![OTHER, OperationId::new().to_string(), f.oid.to_string()],
            )
            .unwrap();
            Ok(())
        });
        assert!(result.is_err());
        // Reopen to inspect actual state; never infer it from the error alone.
        let db = f.db();
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM remote_ref_observations", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(
            f.repo
                .find_reference(plan.context(f.kind, &ID.parse().unwrap()).tracking_ref())
                .is_err()
        );
        drop(evidence);
        let retry = c
            .delete_tracking_ref(
                Some(&plan),
                f.kind,
                &ID.parse().unwrap(),
                Some(f.oid),
                OperationId::new(),
                f.oid,
            )
            .unwrap();
        remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
            retire_context(tx, id, &retry)
        })
        .unwrap();
    }
}

#[test]
fn cleanup_retirement_evidence_is_bound_to_repository_and_frozen_plan() {
    for case in ["repository", "configuration"] {
        let f = Fixture::new(AuthoringKind::Document);
        let plan = f.setup_remote();
        let c = f.cleanup();
        let evidence = c
            .delete_tracking_ref(
                Some(&plan),
                f.kind,
                &ID.parse().unwrap(),
                Some(f.oid),
                OperationId::new(),
                f.oid,
            )
            .unwrap();
        let other = Fixture::new(AuthoringKind::Document);
        let target = if case == "repository" { &other } else { &f };
        if case == "configuration" {
            f.db()
                .execute("UPDATE remote_polling_state SET remote_name='changed'", [])
                .unwrap();
        }
        assert!(
            remote::state::with_transaction(&target.service, target.root.path(), |tx, id| {
                retire_context(tx, id, &evidence)
            })
            .is_err()
        );
        assert_eq!(
            target
                .db()
                .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}

#[test]
fn cleanup_retirement_evidence_holds_ref_lock_through_sql_finalization() {
    let f = Fixture::new(AuthoringKind::Document);
    let plan = f.setup_remote();
    let c = f.cleanup();
    let target = plan.context(f.kind, &ID.parse().unwrap());
    let evidence = c
        .delete_tracking_ref(
            Some(&plan),
            f.kind,
            &ID.parse().unwrap(),
            Some(f.oid),
            OperationId::new(),
            f.oid,
        )
        .unwrap();
    assert!(
        matches!(f.repo.reference(target.tracking_ref(),f.oid,true,"fixture"),Err(e) if e.code()==git2::ErrorCode::Locked)
    );
    remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
        retire_context(tx, id, &evidence)
    })
    .unwrap();
    assert!(
        matches!(f.repo.reference(target.tracking_ref(),f.oid,true,"fixture"),Err(e) if e.code()==git2::ErrorCode::Locked)
    );
    drop(evidence);
    f.repo
        .reference(target.tracking_ref(), f.oid, true, "fixture")
        .unwrap();
}

#[test]
fn cleanup_tracking_guards_missing_expectation_zero_and_foreign_locks() {
    for case in ["no_expected", "zero", "lock", "branch_lock"] {
        let f = Fixture::new(AuthoringKind::Document);
        let plan = f.setup_remote();
        let c = f.cleanup();
        let target = plan.context(f.kind, &ID.parse().unwrap());
        if case == "lock" {
            fs::write(
                f.repo
                    .path()
                    .join(format!("{}.lock", target.tracking_ref())),
                b"foreign",
            )
            .unwrap();
        }
        if case == "branch_lock" {
            assert_eq!(f.remove(&c), WorktreeRemoval::Removed);
            fs::write(
                f.repo.path().join(format!("{}.lock", f.branch())),
                b"foreign",
            )
            .unwrap();
        }
        let before = files(f.root.path());
        if case == "branch_lock" {
            assert_eq!(
                c.delete_context_branch(f.kind, &ID.parse().unwrap(), f.oid),
                BranchDeletion::Changed
            );
        } else {
            let expected = match case {
                "no_expected" => None,
                "zero" => Some(Oid::zero()),
                _ => Some(f.oid),
            };
            assert!(
                c.delete_tracking_ref(
                    Some(&plan),
                    f.kind,
                    &ID.parse().unwrap(),
                    expected,
                    OperationId::new(),
                    f.oid
                )
                .is_err()
            );
        }
        assert_eq!(files(f.root.path()), before);
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn cleanup_retirement_preserves_other_item_and_historical_observations() {
    let f = Fixture::new(AuthoringKind::Document);
    let plan = f.setup_remote();
    remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
        let generation = remote::state::configure(tx, id, Some(&plan), false)?;
        let observations = [ID, OTHER].map(|item| {
            RemoteRefObservation::from_advertisement(
                &plan,
                plan.context(f.kind, &item.parse().unwrap()).remote_ref(),
                f.oid,
                Some(f.oid),
            )
            .unwrap()
        });
        remote::state::complete_batch(tx, id, &plan, generation, &observations, 124)?;
        Ok(())
    })
    .unwrap();
    let c = f.cleanup();
    let evidence = c
        .delete_tracking_ref(
            Some(&plan),
            f.kind,
            &ID.parse().unwrap(),
            Some(f.oid),
            OperationId::new(),
            f.oid,
        )
        .unwrap();
    remote::state::with_transaction(&f.service, f.root.path(), |tx, id| {
        retire_context(tx, id, &evidence)
    })
    .unwrap();
    let snapshot = f.service.remote_snapshot(f.root.path()).unwrap();
    assert_eq!(
        snapshot.publication_evidence_for(f.kind, &OTHER.parse().unwrap()),
        RemotePublicationEvidence::ObservedPublished
    );
    assert_eq!(snapshot.observations().len(), 1);
    assert_eq!(f.db().query_row("SELECT count(*) FROM remote_ref_observations WHERE batch_id IN (SELECT id FROM remote_observation_batches WHERE is_current=0)",[],|r|r.get::<_,i64>(0)).unwrap(),3);
    RepositoryService::open_at(f.data.path()).unwrap();
}

fn foreign_link(f: &Fixture) -> (tempfile::TempDir, Repository) {
    let foreign = tempfile::tempdir().unwrap();
    let repo = Repository::clone(f.root.path().to_str().unwrap(), foreign.path()).unwrap();
    repo.reference(&f.branch(), f.oid, true, "fixture").unwrap();
    repo.set_head(&f.branch()).unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    fs::write(
        f.linked.workdir().unwrap().join(".git"),
        format!("gitdir: {}", remote::git_metadata_link_text(repo.path())),
    )
    .unwrap();
    (foreign, repo)
}
#[test]
fn cleanup_foreign_git_link_cannot_authorize_prune() {
    let f = Fixture::new(AuthoringKind::Document);
    let c = f.cleanup();
    let (_foreign, _repo) = foreign_link(&f);
    let before = files(f.root.path());
    assert_eq!(f.precheck(&c), Precheck::Changed);
    assert_ne!(f.remove(&c), WorktreeRemoval::Removed);
    assert_eq!(files(f.root.path()), before);
}
#[test]
fn cleanup_foreign_git_link_cannot_hide_registered_checkout() {
    let f = Fixture::new(AuthoringKind::Document);
    let c = f.cleanup();
    let (_foreign, repo) = foreign_link(&f);
    // Redirect the link to a repository on a different branch. The registered
    // worktree's own admin HEAD still checks out the context branch.
    repo.set_head("refs/heads/main").unwrap();
    let before = files(f.root.path());
    assert_eq!(
        c.delete_context_branch(f.kind, &ID.parse().unwrap(), f.oid),
        BranchDeletion::Changed
    );
    assert_eq!(files(f.root.path()), before);
}
