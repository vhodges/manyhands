use super::*;
use std::{collections::BTreeMap, fs, net::TcpListener};

const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const DOC: &str = "docs/document.md";

#[cfg(unix)]
fn mode_collision(changed_mode: u32, category: PathChange) {
    use std::os::unix::{ffi::OsStrExt, fs::PermissionsExt};
    let f = Fixture::new(true);
    if category == PathChange::Modified {
        let context_document = fs::read(f.context.workdir().unwrap().join(DOC)).unwrap();
        write(&f.repo, "extra", b"before\n");
        let primary = commit(&f.repo);
        f.tracking("main", primary);
        let context = tree_commit(&f.repo, primary, &[(DOC, Some(&context_document))]);
        checkout(&f.context, context);
    }
    write(&f.context, "extra", b"ordinary.txt");
    let regular_oid = commit(&f.context);
    let primary_image = || {
        let commit = f.repo.head().unwrap().peel_to_commit().unwrap();
        let root_files = files(f.root.path())
            .into_iter()
            .filter(|(path, _)| {
                !path.starts_with(".git") && !path.starts_with(".manyhands/worktrees")
            })
            .collect::<BTreeMap<_, _>>();
        (
            commit.id(),
            commit.tree_id(),
            fs::read(f.repo.path().join("index")).unwrap(),
            f.repo
                .find_reference("refs/remotes/origin/main")
                .unwrap()
                .target(),
            root_files,
        )
    };
    let primary_before = primary_image();
    let regular = f.ready();
    assert_eq!(primary_image(), primary_before);
    let regular_tree = f.context.find_commit(regular_oid).unwrap().tree().unwrap();
    let regular_entry = regular_tree.get_path(Path::new("extra")).unwrap();
    assert_eq!(regular_entry.filemode(), 0o100644);
    assert_eq!(
        regular_entry.id(),
        Oid::hash_object(git2::ObjectType::Blob, b"ordinary.txt").unwrap()
    );
    assert_eq!(
        regular
            .changes()
            .iter()
            .find(|change| change.path() == Path::new("extra"))
            .unwrap()
            .change(),
        category
    );
    let extra = f.context.workdir().unwrap().join("extra");
    match changed_mode {
        0o120000 => {
            fs::remove_file(&extra).unwrap();
            std::os::unix::fs::symlink("ordinary.txt", &extra).unwrap();
            assert_eq!(
                fs::read_link(&extra).unwrap().as_os_str().as_bytes(),
                b"ordinary.txt"
            );
        }
        0o100755 => fs::set_permissions(&extra, fs::Permissions::from_mode(0o755)).unwrap(),
        _ => unreachable!(),
    }
    let changed_oid = commit(&f.context);
    assert_ne!(regular_oid, changed_oid);
    let changed = f.ready();
    assert_eq!(primary_image(), primary_before);
    let changed_tree = f.context.find_commit(changed_oid).unwrap().tree().unwrap();
    let changed_entry = changed_tree.get_path(Path::new("extra")).unwrap();
    assert_eq!(changed_entry.filemode(), changed_mode as i32);
    assert_eq!(changed_entry.id(), regular_entry.id());
    assert_eq!(changed_entry.kind(), Some(git2::ObjectType::Blob));
    assert_eq!(changed.changes(), regular.changes());
    assert_eq!(changed.primary_oid(), regular.primary_oid());
    assert_eq!(changed.tracking_primary(), regular.tracking_primary());
    assert_eq!(changed.integration(), regular.integration());
    assert_eq!(changed.document_path(), regular.document_path());
    assert_ne!(
        regular.expectation(),
        changed.expectation(),
        "T2-1: changed resulting Git mode must require different consent"
    );
}

#[cfg(unix)]
#[test]
fn t2_1_added_regular_to_symlink_changes_digest() {
    mode_collision(0o120000, PathChange::Added);
}
#[cfg(unix)]
#[test]
fn t2_1_added_executable_bit_changes_digest() {
    mode_collision(0o100755, PathChange::Added);
}
#[cfg(unix)]
#[test]
fn t2_1_modified_regular_to_symlink_changes_digest() {
    mode_collision(0o120000, PathChange::Modified);
}
#[cfg(unix)]
#[test]
fn t2_1_modified_executable_bit_changes_digest() {
    mode_collision(0o100755, PathChange::Modified);
}

#[test]
fn t2_1_deletion_encoding_has_zero_big_endian_u32_mode() {
    let f = Fixture::new(true);
    fs::remove_file(f.context.workdir().unwrap().join("ordinary.txt")).unwrap();
    commit(&f.context);
    let preview = f.ready();
    assert_eq!(
        preview
            .changes()
            .iter()
            .map(|c| (c.path(), c.change()))
            .collect::<Vec<_>>(),
        vec![
            (Path::new(DOC), PathChange::Modified),
            (Path::new("ordinary.txt"), PathChange::Deleted)
        ]
    );
    let doc = f
        .context
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(Path::new(DOC))
        .unwrap()
        .id();
    // Independent protocol vector for this known remote-backed two-change preview.
    fn field(wire: &mut Vec<u8>, bytes: &[u8]) {
        wire.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        wire.extend_from_slice(bytes);
    }
    let mut wire = b"manyhands promotion v1".to_vec();
    for bytes in [
        ID.as_bytes(),
        DOC.as_bytes(),
        format!("manyhands/document/{ID}").as_bytes(),
        b"main",
    ] {
        field(&mut wire, bytes);
    }
    wire.push(1); // configured remote
    field(&mut wire, b"origin");
    wire.push(0); // MergeCommit
    wire.extend_from_slice(&2_u64.to_be_bytes());
    field(&mut wire, DOC.as_bytes());
    wire.extend_from_slice(&[1, 1]); // Modified, present new OID
    wire.extend_from_slice(doc.as_bytes());
    wire.extend_from_slice(&[0, 0, 0x81, 0xa4]); // regular 100644, u32 big endian
    field(&mut wire, b"ordinary.txt");
    wire.extend_from_slice(&[2, 0]); // Deleted, no new OID
    let deletion_mode = wire.len();
    wire.extend_from_slice(&[0, 0, 0, 0]); // no resulting entry, explicit u32 sentinel
    wire.extend_from_slice(&[1, 1, 1]); // cleanup remote/worktree/local branch
    assert_eq!(
        preview.expectation().to_bytes(),
        *blake3::hash(&wire).as_bytes()
    );
    let mut omitted = wire.clone();
    omitted.drain(deletion_mode..deletion_mode + 4);
    assert_ne!(
        preview.expectation().to_bytes(),
        *blake3::hash(&omitted).as_bytes()
    );
    wire[deletion_mode..deletion_mode + 4].copy_from_slice(&[0, 0, 0x81, 0xa4]);
    assert_ne!(
        preview.expectation().to_bytes(),
        *blake3::hash(&wire).as_bytes()
    );
}

struct Fixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    repo: git2::Repository,
    context: git2::Repository,
    base: Oid,
}

fn commit(repo: &git2::Repository) -> Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
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

/// Create a commit without changing any checked-out file, index or ref.
fn tree_commit(repo: &git2::Repository, parent: Oid, edits: &[(&str, Option<&[u8]>)]) -> Oid {
    let mut index = git2::Index::new().unwrap();
    index
        .read_tree(&repo.find_commit(parent).unwrap().tree().unwrap())
        .unwrap();
    for (path, bytes) in edits {
        if let Some(bytes) = bytes {
            let entry = git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: bytes.len() as u32,
                id: repo.blob(bytes).unwrap(),
                flags: 0,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            };
            index.add(&entry).unwrap();
        } else {
            index.remove_path(Path::new(path)).unwrap();
        }
    }
    let tree = repo.find_tree(index.write_tree_to(repo).unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(
        None,
        &sig,
        &sig,
        "fixture detached",
        &tree,
        &[&repo.find_commit(parent).unwrap()],
    )
    .unwrap()
}

fn document_mode_commit(repo: &git2::Repository, parent: Oid, mode: u32) -> Oid {
    let parent = repo.find_commit(parent).unwrap();
    let mut index = git2::Index::new().unwrap();
    index.read_tree(&parent.tree().unwrap()).unwrap();
    let mut entry = index.get_path(Path::new(DOC), 0).unwrap();
    entry.mode = mode;
    index.add(&entry).unwrap();
    let tree = repo.find_tree(index.write_tree_to(repo).unwrap()).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(
        None,
        &signature,
        &signature,
        "fixture document mode",
        &tree,
        &[&parent],
    )
    .unwrap()
}

fn write(repo: &git2::Repository, path: &str, bytes: impl AsRef<[u8]>) {
    let path = repo.workdir().unwrap().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn checkout(repo: &git2::Repository, oid: Oid) {
    repo.reset(
        &repo.find_object(oid, None).unwrap(),
        git2::ResetType::Hard,
        None,
    )
    .unwrap();
}

impl Fixture {
    fn new(remote: bool) -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(root.path()).unwrap();
        repo.config()
            .unwrap()
            .set_bool("core.autocrlf", false)
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.name", "Fixture")
            .unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        repo.set_head("refs/heads/main").unwrap();
        write(&repo, "ordinary.txt", "original\n");
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
        if remote {
            service
                .add_remote(AddRemoteRequest {
                    root: root.path().into(),
                    name: "origin".into(),
                    url: "ssh://fixture@127.0.0.1:1/repository".into(),
                    operation_id: OperationId::new(),
                })
                .unwrap();
            service
                .set_publication_remote(SetPublicationRemoteRequest {
                    root: root.path().into(),
                    name: Some("origin".into()),
                    operation_id: OperationId::new(),
                })
                .unwrap();
        }
        let target = AuthoringTarget {
            root: root.path().into(),
            kind: AuthoringKind::Document,
            item_id: ID.parse().unwrap(),
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        };
        service
            .save_document(SaveDocumentRequest {
                target,
                source_path: None,
                destination_path: DOC.into(),
                draft: DocumentDraft {
                    title: "Document".into(),
                    body: "original body\n".into(),
                },
                expected_source: None,
                expected_destination: ExpectedPathObservation::Missing,
            })
            .unwrap();
        let context =
            git2::Repository::open(root.path().join(".manyhands/worktrees").join(ID)).unwrap();
        // Put the original document on primary so independent result-side edits are possible.
        checkout(&repo, context.head().unwrap().target().unwrap());
        let base = repo.head().unwrap().target().unwrap();
        let source = fs::read_to_string(context.workdir().unwrap().join(DOC))
            .unwrap()
            .replace("original body", "context body");
        write(&context, DOC, source);
        commit(&context);
        let f = Self {
            root,
            data,
            service,
            repo,
            context,
            base,
        };
        if remote {
            f.db()
                .execute("UPDATE remote_polling_state SET history_unknown=0", [])
                .unwrap();
            f.tracking("main", base);
        }
        f
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }
    fn tracking(&self, branch: &str, oid: Oid) {
        self.repo
            .reference(
                &format!("refs/remotes/origin/{branch}"),
                oid,
                true,
                "fixture",
            )
            .unwrap();
    }
    fn context_oid(&self) -> Oid {
        self.context.head().unwrap().target().unwrap()
    }
    fn preview(&self) -> DocumentPromotionPreflight {
        self.service
            .preview_document_promotion(PreviewDocumentPromotionRequest {
                root: self.root.path().into(),
                item_id: ID.parse().unwrap(),
            })
            .unwrap()
    }
    fn ready(&self) -> DocumentPromotionPreview {
        match self.preview() {
            DocumentPromotionPreflight::Ready(p) => p,
            other => panic!("expected ready: {other:?}"),
        }
    }
    fn blocked(&self, blockers: &[PromotionBlocker]) {
        assert_eq!(
            self.preview(),
            DocumentPromotionPreflight::Blocked {
                blockers: blockers.to_vec()
            }
        );
    }
    fn history(&self, evidence: &str) {
        self.db().execute("INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) SELECT id,?1,?2,'document',?3,?4,?5,0 FROM repositories", rusqlite::params![format!("refs/heads/manyhands/document/{ID}"), format!("refs/remotes/origin/manyhands/document/{ID}"), ID, evidence, if evidence == "history_unknown" {"history_unknown"} else {"remotely_deleted"}]).unwrap();
        if evidence == "observed_published" {
            self.db()
                .execute(
                    "UPDATE remote_context_states SET last_advertised_oid=?1",
                    [self.context_oid().to_string()],
                )
                .unwrap();
        }
    }
    fn pending_conflict(&self) {
        let db = self.db();
        let oid = self.base.to_string();
        db.execute("INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,local_branch,action,priority,phase,created_at,updated_at) SELECT id,?1,0,'origin','main','refs/heads/main','refs/remotes/origin/main','main','synchronize_primary','manual','interrupted',0,0 FROM repositories", [OperationId::new().to_string()]).unwrap();
        let id = db.last_insert_rowid();
        db.execute("INSERT INTO remote_integration_windows(operation_record_id,number,configuration_generation,owner_epoch,kind) VALUES(?1,0,0,0,'legacy')", [id]).unwrap();
        db.execute("INSERT INTO remote_integration_steps(operation_record_id,configuration_generation,owner_epoch,ordinal,stage,local_oid,incoming_oid,baseline_tree_oid,baseline_index_digest,conflict_digest,phase) VALUES(?1,0,0,0,'primary',?2,?2,?3,zeroblob(32),zeroblob(32),'conflict_pending')", rusqlite::params![id,oid,self.repo.find_commit(self.base).unwrap().tree_id().to_string()]).unwrap();
    }
}

#[test]
fn not_a_document() {
    let f = Fixture::new(false);
    let path = f.context.workdir().unwrap().join(DOC);
    let bytes = fs::read(&path).unwrap();
    fs::remove_file(path).unwrap();
    let ticket = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ID}\ntitle: Ticket\ntype: task\nstatus: open\n---\n\nBody\n"
    );
    write(
        &f.context,
        &format!(".manyhands/tickets/{ID}/ticket.md"),
        ticket,
    );
    commit(&f.context);
    // Requesting a ticket ID must not be mistaken for an absent document context.
    f.context
        .find_branch(&format!("manyhands/document/{ID}"), git2::BranchType::Local)
        .unwrap()
        .rename(&format!("manyhands/ticket/{ID}"), false)
        .unwrap();
    assert!(!bytes.is_empty());
    f.blocked(&[PromotionBlocker::NotADocument]);
}
#[test]
fn context_missing() {
    let f = Fixture::new(false);
    let wt = f.repo.find_worktree(ID).unwrap();
    wt.prune(Some(
        git2::WorktreePruneOptions::new()
            .valid(true)
            .working_tree(true),
    ))
    .unwrap();
    f.blocked(&[PromotionBlocker::ContextMissing]);
}

#[test]
fn context_branch_missing_is_blocked_without_panicking() {
    let f = Fixture::new(false);
    f.repo
        .find_reference(&format!("refs/heads/manyhands/document/{ID}"))
        .unwrap()
        .delete()
        .unwrap();
    f.blocked(&[PromotionBlocker::ContextMissing]);
}
#[test]
fn context_nonconforming() {
    let f = Fixture::new(false);
    write(&f.context, DOC, "not a managed document\n");
    commit(&f.context);
    f.blocked(&[PromotionBlocker::ContextNonconforming]);
}
#[test]
fn context_dirty_tracked() {
    let f = Fixture::new(false);
    write(&f.context, "ordinary.txt", "dirty");
    f.blocked(&[PromotionBlocker::ContextDirty]);
}
#[test]
fn context_on_wrong_branch_is_nonconforming() {
    let f = Fixture::new(false);
    f.repo
        .reference("refs/heads/wrong", f.context_oid(), false, "fixture")
        .unwrap();
    f.context.set_head("refs/heads/wrong").unwrap();
    f.blocked(&[PromotionBlocker::ContextNonconforming]);
}
#[test]
fn context_with_duplicate_document_is_nonconforming() {
    let f = Fixture::new(false);
    write(
        &f.context,
        "docs/duplicate.md",
        fs::read(f.context.workdir().unwrap().join(DOC)).unwrap(),
    );
    commit(&f.context);
    f.blocked(&[PromotionBlocker::ContextNonconforming]);
}
#[test]
fn context_dirty_staged() {
    let f = Fixture::new(false);
    write(&f.context, "ordinary.txt", "dirty");
    let mut i = f.context.index().unwrap();
    i.add_path(Path::new("ordinary.txt")).unwrap();
    i.write().unwrap();
    f.blocked(&[PromotionBlocker::ContextDirty]);
}
#[test]
fn context_dirty_untracked() {
    let f = Fixture::new(false);
    write(&f.context, "untracked", "dirty");
    f.blocked(&[PromotionBlocker::ContextDirty]);
}
#[test]
fn context_conflicted() {
    let f = Fixture::new(false);
    fs::write(f.context.path().join("MERGE_HEAD"), format!("{}\n", f.base)).unwrap();
    f.blocked(&[PromotionBlocker::ContextConflicted]);
}
#[test]
fn conflict_pending() {
    let f = Fixture::new(false);
    f.pending_conflict();
    f.blocked(&[PromotionBlocker::ConflictPending]);
}
#[test]
fn primary_missing() {
    let f = Fixture::new(false);
    f.repo
        .find_reference("refs/heads/main")
        .unwrap()
        .delete()
        .unwrap();
    f.blocked(&[PromotionBlocker::PrimaryMissing]);
}
#[test]
fn primary_tracking_missing() {
    let f = Fixture::new(true);
    f.repo
        .find_reference("refs/remotes/origin/main")
        .unwrap()
        .delete()
        .unwrap();
    f.blocked(&[PromotionBlocker::PrimaryMissing]);
}
#[test]
fn primary_not_checked_out() {
    let f = Fixture::new(false);
    f.repo.set_head_detached(f.base).unwrap();
    f.blocked(&[PromotionBlocker::PrimaryNotCheckedOut]);
}
#[test]
fn primary_dirty() {
    let f = Fixture::new(false);
    write(&f.repo, "ordinary.txt", "dirty");
    f.blocked(&[PromotionBlocker::PrimaryDirty]);
}
#[test]
fn primary_conflicted() {
    let f = Fixture::new(false);
    let mut index = f.repo.index().unwrap();
    let mut entry = index.get_path(Path::new("ordinary.txt"), 0).unwrap();
    index.remove_path(Path::new("ordinary.txt")).unwrap();
    entry.flags = (entry.flags & !0x3000) | 0x1000;
    index.add(&entry).unwrap();
    index.write().unwrap();
    assert!(index.has_conflicts());
    f.blocked(&[PromotionBlocker::PrimaryConflicted]);
}
#[test]
fn primary_ahead_of_remote() {
    let f = Fixture::new(true);
    write(&f.repo, "primary-only", "new");
    commit(&f.repo);
    f.blocked(&[PromotionBlocker::PrimaryAheadOfRemote]);
}
#[test]
fn primary_diverged_from_remote() {
    let f = Fixture::new(true);
    let remote = tree_commit(&f.repo, f.base, &[("remote-only", Some(b"remote"))]);
    f.tracking("main", remote);
    write(&f.repo, "primary-only", "new");
    commit(&f.repo);
    f.blocked(&[PromotionBlocker::PrimaryAheadOfRemote]);
}
#[test]
fn context_behind_remote() {
    let f = Fixture::new(true);
    let oid = tree_commit(
        &f.repo,
        f.context_oid(),
        &[("remote-only", Some(b"remote"))],
    );
    f.tracking(&format!("manyhands/document/{ID}"), oid);
    f.blocked(&[PromotionBlocker::ContextBehindRemote]);
}
#[test]
fn context_diverged_from_remote() {
    let f = Fixture::new(true);
    let oid = tree_commit(&f.repo, f.base, &[("remote-only", Some(b"remote"))]);
    f.tracking(&format!("manyhands/document/{ID}"), oid);
    f.blocked(&[PromotionBlocker::ContextBehindRemote]);
}
#[test]
fn local_integration_conflict() {
    let f = Fixture::new(false);
    let source = fs::read_to_string(f.repo.workdir().unwrap().join(DOC))
        .unwrap()
        .replace("original body", "primary body");
    write(&f.repo, DOC, source);
    commit(&f.repo);
    f.blocked(&[PromotionBlocker::IntegrationConflict]);
}
#[test]
fn remote_context_deleted() {
    let f = Fixture::new(true);
    f.history("observed_published");
    f.blocked(&[PromotionBlocker::RemoteContextDeleted]);
}
#[test]
fn remote_history_unknown() {
    let f = Fixture::new(true);
    f.db()
        .execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    f.blocked(&[PromotionBlocker::HistoryUnknown]);
}
#[test]
fn present_tracking_context_overrides_deleted_history() {
    let f = Fixture::new(true);
    f.history("observed_published");
    f.tracking(&format!("manyhands/document/{ID}"), f.context_oid());
    assert_eq!(f.ready().publication(), PlannedPublication::Publish);
}
#[test]
fn present_tracking_context_overrides_unknown_history() {
    let f = Fixture::new(true);
    f.db()
        .execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    f.tracking(&format!("manyhands/document/{ID}"), f.context_oid());
    f.ready();
}
#[test]
fn contained_absent_tracking_context_is_cleanup_only() {
    for evidence in ["observed_published", "history_unknown"] {
        let f = Fixture::new(true);
        f.history(evidence);
        f.tracking("main", f.context_oid());
        let p = f.ready();
        assert_eq!(p.integration(), PlannedIntegration::AlreadyContained);
        assert!(p.changes().is_empty());
        assert_eq!(
            p.cleanup(),
            PlannedCleanup {
                remote_branch: false,
                worktree: true,
                local_branch: true
            }
        );
    }
}
#[test]
fn result_document_invalid() {
    let f = Fixture::new(false);
    let source = fs::read_to_string(f.context.workdir().unwrap().join(DOC)).unwrap();
    write(&f.repo, "docs/duplicate.md", source);
    commit(&f.repo);
    f.blocked(&[PromotionBlocker::DocumentInvalidAfterIntegration]);
}
#[test]
fn result_document_metadata_invalid() {
    let f = Fixture::new(false);
    let source = fs::read_to_string(f.repo.workdir().unwrap().join(DOC))
        .unwrap()
        .replace("manyhands_kind: document", "manyhands_kind: ticket");
    write(&f.repo, DOC, source);
    commit(&f.repo);
    f.blocked(&[PromotionBlocker::DocumentInvalidAfterIntegration]);
}

#[test]
fn document_mode_executable_context_previews() {
    let f = Fixture::new(false);
    let executable = document_mode_commit(&f.repo, f.context_oid(), 0o100755);
    checkout(&f.context, executable);
    assert!(
        discovery::observe_root(&f.repo, f.root.path())
            .active_contexts
            .iter()
            .any(|active| { active.context.item_id.as_ref() == Some(&ID.parse().unwrap()) })
    );
    let preview = f.ready();
    assert_eq!(preview.integration(), PlannedIntegration::MergeCommit);
    assert_eq!(preview.document_path(), Path::new(DOC));
    assert_eq!(preview.context_oid(), executable);
    assert_eq!(preview.changes().len(), 1);
    assert_eq!(preview.changes()[0].path(), Path::new(DOC));
    assert_eq!(preview.changes()[0].change(), PathChange::Modified);
}

#[test]
fn document_mode_executable_contained_result_previews() {
    let f = Fixture::new(false);
    let executable = document_mode_commit(&f.repo, f.context_oid(), 0o100755);
    checkout(&f.repo, executable);
    assert_eq!(
        f.context
            .head()
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .get_path(Path::new(DOC))
            .unwrap()
            .filemode(),
        0o100644
    );
    assert_eq!(
        f.repo
            .head()
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .get_path(Path::new(DOC))
            .unwrap()
            .filemode(),
        0o100755
    );
    let preview = f.ready();
    assert_eq!(preview.integration(), PlannedIntegration::AlreadyContained);
    assert_eq!(preview.document_path(), Path::new(DOC));
    assert!(preview.changes().is_empty());
}

#[test]
fn document_mode_symlink_result_is_invalid() {
    let f = Fixture::new(true);
    // The blob deliberately still parses as the document: canonical text must
    // not make a non-regular result entry an acceptable source.
    let symlink = document_mode_commit(&f.repo, f.context_oid(), 0o120000);
    f.tracking("main", symlink);
    f.blocked(&[PromotionBlocker::DocumentInvalidAfterIntegration]);
}
#[test]
fn result_forbids_worktrees_entries() {
    let f = Fixture::new(true);
    let remote = tree_commit(
        &f.repo,
        f.base,
        &[(".manyhands/worktrees/forbidden", Some(b"forbidden"))],
    );
    f.tracking("main", remote);
    f.blocked(&[PromotionBlocker::DocumentInvalidAfterIntegration]);
}
#[test]
fn multiple_blockers_have_fixed_order() {
    let f = Fixture::new(true);
    write(&f.context, "ordinary.txt", "dirty");
    f.pending_conflict();
    write(&f.repo, "ordinary.txt", "dirty");
    f.history("observed_published");
    f.blocked(&[
        PromotionBlocker::ContextDirty,
        PromotionBlocker::ConflictPending,
        PromotionBlocker::PrimaryDirty,
        PromotionBlocker::RemoteContextDeleted,
    ]);
}
#[test]
fn primary_shared_merge_metadata_conflicts_both_targets() {
    let f = Fixture::new(false);
    fs::write(f.repo.path().join("MERGE_HEAD"), format!("{}\n", f.base)).unwrap();
    f.blocked(&[
        PromotionBlocker::ContextConflicted,
        PromotionBlocker::PrimaryConflicted,
    ]);
}

#[test]
fn contained_selects_base_tree() {
    let f = Fixture::new(false);
    checkout(&f.repo, f.context_oid());
    write(&f.repo, "later", "primary");
    commit(&f.repo);
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::AlreadyContained);
    assert!(p.changes().is_empty());
}

#[test]
fn contained_primary_move_preserves_context_document_path_and_digest_binding() {
    let f = Fixture::new(false);
    checkout(&f.repo, f.context_oid());
    fs::rename(
        f.repo.workdir().unwrap().join(DOC),
        f.repo.workdir().unwrap().join("docs/moved.md"),
    )
    .unwrap();
    commit(&f.repo);
    assert!(f.context.workdir().unwrap().join(DOC).is_file());
    assert!(!f.context.workdir().unwrap().join("docs/moved.md").exists());
    let preview = f.ready();
    assert_eq!(preview.integration(), PlannedIntegration::AlreadyContained);
    assert!(preview.changes().is_empty());
    assert_eq!(preview.document_path(), Path::new(DOC));
    let mut expected = preview.clone();
    expected.document_path = DOC.into();
    assert_eq!(preview.expectation(), expectation(&expected, &[]));
    expected.document_path = "docs/moved.md".into();
    assert_ne!(preview.expectation(), expectation(&expected, &[]));
}
#[test]
fn ancestor_selects_context_tree() {
    let f = Fixture::new(false);
    let p = f.ready();
    assert_eq!(p.integration(), PlannedIntegration::MergeCommit);
    assert_eq!(
        p.changes()
            .iter()
            .map(|c| (c.path(), c.change()))
            .collect::<Vec<_>>(),
        vec![(Path::new(DOC), PathChange::Modified)]
    );
    assert_eq!(p.publication(), PlannedPublication::PendingNoRemote);
    assert_eq!(p.context_oid(), f.context_oid());
    assert_eq!(p.primary_oid(), f.base);
    assert_eq!(p.context_branch(), format!("manyhands/document/{ID}"));
    assert_eq!(p.primary_branch(), "main");
    assert_eq!(p.document_path(), Path::new(DOC));
    assert_eq!(p.item_id().to_string(), ID);
    assert_eq!(p.publication_remote(), None);
    assert_eq!(p.tracking_primary(), None);
    assert_eq!(p.tracking_context(), None);
}
#[test]
fn local_divergence_selects_prepared_tree() {
    let f = Fixture::new(false);
    write(&f.repo, "primary-only", "primary");
    commit(&f.repo);
    let p = f.ready();
    assert_eq!(p.changes().len(), 1);
    assert_eq!(p.changes()[0].path(), Path::new(DOC));
}
#[test]
fn remote_stale_primary_selects_prepared_tree() {
    let f = Fixture::new(true);
    let remote = tree_commit(&f.repo, f.base, &[("remote-only", Some(b"primary"))]);
    f.tracking("main", remote);
    let p = f.ready();
    assert_eq!(p.changes().len(), 1);
    assert_eq!(p.changes()[0].path(), Path::new(DOC));
    assert_eq!(p.tracking_primary(), Some(remote));
}
#[test]
fn change_set_includes_all_paths_and_moves_without_rename_detection() {
    let f = Fixture::new(false);
    let source = fs::read(f.context.workdir().unwrap().join(DOC)).unwrap();
    fs::remove_file(f.context.workdir().unwrap().join(DOC)).unwrap();
    write(&f.context, "docs/moved.md", source);
    fs::remove_file(f.context.workdir().unwrap().join("ordinary.txt")).unwrap();
    write(&f.context, "outside.txt", "noncanonical");
    write(
        &f.context,
        &format!(".manyhands/comments/{ID}/01ARZ3NDEKTSV4RRFFQ69G5FAX.md"),
        "comment bytes",
    );
    commit(&f.context);
    let p = f.ready();
    assert_eq!(p.document_path(), Path::new("docs/moved.md"));
    assert_eq!(
        p.changes()
            .iter()
            .map(|c| (c.path().to_string_lossy().into_owned(), c.change()))
            .collect::<Vec<_>>(),
        vec![
            (
                format!(".manyhands/comments/{ID}/01ARZ3NDEKTSV4RRFFQ69G5FAX.md"),
                PathChange::Added
            ),
            (DOC.into(), PathChange::Deleted),
            ("docs/moved.md".into(), PathChange::Added),
            ("ordinary.txt".into(), PathChange::Deleted),
            ("outside.txt".into(), PathChange::Added)
        ]
    );
}

#[test]
fn digest_is_stable_and_debug_is_redacted() {
    let f = Fixture::new(true);
    let p = f.ready();
    assert_eq!(p.expectation(), f.ready().expectation());
    assert_eq!(
        PromotionExpectation::from_bytes(p.expectation().to_bytes()),
        p.expectation()
    );
    assert_eq!(
        format!("{:?}", p.expectation()),
        "PromotionExpectation(<redacted>)"
    );
    for text in [format!("{p:?}"), format!("{:?}", p.changes())] {
        for secret in [DOC, "origin", "manyhands/document", "context body"] {
            assert!(!text.contains(secret));
        }
    }
}
#[test]
fn digest_binds_each_field_change_blob_and_cleanup_member() {
    let f = Fixture::new(true);
    let p = f.ready();
    let blobs = vec![PromotionChangeEntry {
        oid: Some(
            f.context
                .head()
                .unwrap()
                .peel_to_tree()
                .unwrap()
                .get_path(Path::new(DOC))
                .unwrap()
                .id(),
        ),
        mode: 0o100644,
    }];
    let original = expectation(&p, &blobs);
    assert_eq!(original, p.expectation());
    let mut variants = Vec::new();
    let mut q = p.clone();
    q.item_id = "01ARZ3NDEKTSV4RRFFQ69G5FAX".parse().unwrap();
    variants.push(q);
    let mut q = p.clone();
    q.document_path = "docs/other.md".into();
    variants.push(q);
    let mut q = p.clone();
    q.context_branch.push_str("-other");
    variants.push(q);
    let mut q = p.clone();
    q.primary_branch.push_str("-other");
    variants.push(q);
    let mut q = p.clone();
    q.publication_remote = Some("upstream".into());
    variants.push(q);
    let mut q = p.clone();
    q.publication_remote = None;
    variants.push(q);
    let mut q = p.clone();
    q.integration = PlannedIntegration::AlreadyContained;
    variants.push(q);
    let mut q = p.clone();
    q.changes[0].path = "docs/other.md".into();
    variants.push(q);
    let mut q = p.clone();
    q.changes[0].change = PathChange::Added;
    variants.push(q);
    let mut q = p.clone();
    q.changes.clear();
    variants.push(q);
    for member in 0..3 {
        let mut q = p.clone();
        match member {
            0 => q.cleanup.remote_branch = false,
            1 => q.cleanup.worktree = false,
            _ => q.cleanup.local_branch = false,
        };
        variants.push(q);
    }
    for q in variants {
        let n = q.changes.len();
        assert_ne!(original, expectation(&q, &blobs[..n]));
    }
    assert_ne!(
        original,
        expectation(
            &p,
            &[PromotionChangeEntry {
                oid: Some(Oid::from_bytes(&[7; 20]).unwrap()),
                mode: 0o100644
            }]
        )
    );
    assert_ne!(
        original,
        expectation(&p, &[PromotionChangeEntry { oid: None, mode: 0 }])
    );
    assert_ne!(
        original,
        expectation(
            &p,
            &[PromotionChangeEntry {
                mode: 0o100755,
                ..blobs[0]
            }]
        )
    );
}
#[test]
fn digest_binds_every_change_entry() {
    let f = Fixture::new(true);
    write(&f.context, "z-other", "first");
    commit(&f.context);
    let first = f.ready();
    write(&f.context, "z-other", "second");
    commit(&f.context);
    assert_ne!(first.expectation(), f.ready().expectation());
}
#[test]
fn remote_digest_excludes_moving_oids() {
    let f = Fixture::new(true);
    let first = f.ready();
    let remote = tree_commit(&f.repo, f.base, &[("primary-only", Some(b"primary"))]);
    f.tracking("main", remote);
    assert_eq!(first.expectation(), f.ready().expectation());
    checkout(&f.repo, remote);
    let p = f.ready();
    assert_eq!(first.expectation(), p.expectation());
    let mut index = f
        .repo
        .merge_commits(
            &f.repo.find_commit(remote).unwrap(),
            &f.repo.find_commit(f.context_oid()).unwrap(),
            None,
        )
        .unwrap();
    let tree = f
        .repo
        .find_tree(index.write_tree_to(&f.repo).unwrap())
        .unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let oid = f
        .repo
        .commit(
            None,
            &sig,
            &sig,
            "Merge primary into document",
            &tree,
            &[
                &f.repo.find_commit(f.context_oid()).unwrap(),
                &f.repo.find_commit(remote).unwrap(),
            ],
        )
        .unwrap();
    checkout(&f.context, oid);
    f.tracking(&format!("manyhands/document/{ID}"), oid);
    let after = f.ready();
    assert_ne!(first.context_oid(), after.context_oid());
    assert_eq!(first.expectation(), after.expectation());
}
#[test]
fn local_digest_binds_primary_oid() {
    let f = Fixture::new(false);
    let first = f.ready();
    write(&f.repo, "primary-only", "primary");
    commit(&f.repo);
    assert_ne!(first.expectation(), f.ready().expectation());
}
#[test]
fn local_digest_binds_context_oid() {
    let f = Fixture::new(false);
    let first = f.ready();
    commit(&f.context);
    assert_ne!(first.context_oid(), f.context_oid());
    assert_ne!(first.expectation(), f.ready().expectation());
}

fn files(root: &Path) -> BTreeMap<PathBuf, [u8; 32]> {
    fn visit(base: &Path, path: &Path, out: &mut BTreeMap<PathBuf, [u8; 32]>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(base, &entry.path(), out);
            } else {
                out.insert(
                    entry.path().strip_prefix(base).unwrap().into(),
                    *blake3::hash(&fs::read(entry.path()).unwrap()).as_bytes(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}
fn registry_rows(f: &Fixture) -> BTreeMap<String, Vec<String>> {
    let db = f.db();
    let tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let mut stmt = db
                .prepare(&format!("SELECT * FROM \"{}\"", table.replace('"', "\"\"")))
                .unwrap();
            let count = stmt.column_count();
            let mut rows = stmt
                .query_map([], |row| {
                    (0..count)
                        .map(|n| row.get::<_, rusqlite::types::Value>(n))
                        .collect::<Result<Vec<_>, _>>()
                        .map(|values| format!("{values:?}"))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            rows.sort();
            (table, rows)
        })
        .collect()
}
#[test]
fn prepared_generated_blob_is_bound_without_odb_or_other_writes() {
    let f = Fixture::new(false);
    let original = fs::read_to_string(f.repo.workdir().unwrap().join(DOC)).unwrap();
    let primary = original.replace("title: Document", "title: Primary title");
    assert_ne!(original, primary);
    write(&f.repo, DOC, &primary);
    commit(&f.repo);
    let merged = primary.replace("original body", "context body");
    let merged_oid = Oid::hash_object(git2::ObjectType::Blob, merged.as_bytes()).unwrap();
    assert!(!f.repo.odb().unwrap().exists(merged_oid));
    let before = files(f.root.path());
    let rows = registry_rows(&f);
    let _lease = RepositoryService::hold_lease_for_testing(
        f.root.path(),
        f.data.path(),
        LeaseKind::Repository,
    )
    .unwrap();
    // Snapshot the lock file the test itself just created as well.
    let held = files(f.root.path());
    let p = f.ready();
    assert_eq!(
        p.expectation(),
        expectation(
            &p,
            &[PromotionChangeEntry {
                oid: Some(merged_oid),
                mode: 0o100644
            }]
        )
    );
    assert!(!f.repo.odb().unwrap().exists(merged_oid));
    assert_eq!(files(f.root.path()), held);
    assert_eq!(registry_rows(&f), rows);
    assert!(
        before
            .iter()
            .all(|(path, bytes)| held.get(path) == Some(bytes))
    );
}
#[test]
fn preview_is_read_only_offline_and_does_not_take_repository_lease() {
    let f = Fixture::new(true);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    f.repo
        .remote_set_url(
            "origin",
            &format!(
                "ssh://fixture@127.0.0.1:{}/repository",
                listener.local_addr().unwrap().port()
            ),
        )
        .unwrap();
    let _lease = RepositoryService::hold_lease_for_testing(
        f.root.path(),
        f.data.path(),
        LeaseKind::Repository,
    )
    .unwrap();
    let before = files(f.root.path());
    let data = registry_rows(&f);
    f.ready();
    f.ready();
    assert_eq!(files(f.root.path()), before);
    assert_eq!(registry_rows(&f), data);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
