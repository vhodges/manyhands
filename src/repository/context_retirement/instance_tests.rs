use super::super::record;
use super::*;
use crate::repository::*;
const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
struct Fixture {
    _root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    request: RetireContextRequest,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let git = git2::Repository::init(root.path()).unwrap();
        git.set_head("refs/heads/main").unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        let tree = git
            .find_tree(git.treebuilder(None).unwrap().write().unwrap())
            .unwrap();
        git.commit(Some("HEAD"), &sig, &sig, "fixture", &tree, &[])
            .unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        service
            .enable(EnableRepositoryRequest {
                root: root.path().into(),
                primary_branch: "main".into(),
                identity: None,
                operation_id: OperationId::new(),
            })
            .unwrap();
        let config = root.path().join(".manyhands/config.toml");
        let mut text = std::fs::read_to_string(&config).unwrap();
        text.push_str("publication_remote = \"origin\"\n");
        std::fs::write(config, text).unwrap();
        git.remote("origin", "ssh://fixture@127.0.0.1:1/fixture")
            .unwrap();
        let relative = format!(".manyhands/tickets/{ID}/ticket.md");
        std::fs::create_dir_all(
            root.path()
                .join(std::path::Path::new(&relative).parent().unwrap()),
        )
        .unwrap();
        std::fs::write(root.path().join(&relative),format!("---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ID}\ntitle: Fixture\ntype: task\nstatus: open\nclosed_at: '2026-11-04T00:00:00Z'\nclosed_by: 'Fixture <fixture@example.invalid>'\n---\nBody\n")).unwrap();
        let mut index = git.index().unwrap();
        index.add_path(std::path::Path::new(&relative)).unwrap();
        index.write().unwrap();
        let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = git.head().unwrap().peel_to_commit().unwrap();
        let tip = git
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "completed fixture",
                &tree,
                &[&parent],
            )
            .unwrap();
        let reference = git
            .reference(
                &format!("refs/heads/manyhands/ticket/{ID}"),
                tip,
                false,
                "fixture",
            )
            .unwrap();
        let worktree = root.path().join(".manyhands/worktrees").join(ID);
        std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
        git.worktree(
            ID,
            &worktree,
            Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
        )
        .unwrap();
        let request = RetireContextRequest {
            root: root.path().canonicalize().unwrap(),
            kind: AuthoringKind::Ticket,
            item_id: ID.parse().unwrap(),
            operation_id: OperationId::new(),
        };
        Self {
            _root: root,
            data,
            service,
            request,
        }
    }
    fn git(&self) -> git2::Repository {
        git2::Repository::open(&self.request.root).unwrap()
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }
    fn selected(&self) -> (ContextCleanup, Selection, Box<RemoteReservation>) {
        let cleanup = ContextCleanup::acquire(&self.request.root).unwrap();
        let git = self.git();
        let selection = Selection::capture(&git, &self.request, &cleanup).unwrap();
        let plan =
            crate::repository::remote::observation::observation_plan(&self.request.root).unwrap();
        let configuration = self
            .service
            .observation_configuration(&self.request.root, &plan)
            .unwrap();
        let record::BranchClaim::Execute(owner) = record::claim_present(
            &self.service,
            &self.request,
            &selection,
            plan,
            configuration,
        )
        .unwrap() else {
            panic!("valid capture")
        };
        (cleanup, selection, owner)
    }
}
fn copy_directory(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_directory(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}
#[test]
#[cfg(unix)]
fn retirement_instance_selection_pins_exact_admin_before_install() {
    let f = Fixture::new();
    let (cleanup, selection, owner) = f.selected();
    let path = admin(&f.git(), &f.request.item_id);
    let old = f.git().commondir().join("old-admin-fixture");
    std::fs::rename(&path, &old).unwrap();
    copy_directory(&old, &path);
    assert!(
        selection
            .install(&f.service, &f.request, &owner, &cleanup)
            .is_err(),
        "a fresh install permit must not stamp a same-OID replacement admin"
    );
    assert!(!path.join(MARKER).exists());
    assert!(!old.join(MARKER).exists());
    assert_eq!(
        f.db()
            .query_row(
                "SELECT instance_verified FROM context_retirement_records",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
#[test]
fn retirement_instance_matching_installed_token_can_finish_sql_only() {
    let f = Fixture::new();
    let (cleanup, selection, owner) = f.selected();
    f.db().execute_batch("CREATE TRIGGER instance_test_interrupt BEFORE UPDATE OF instance_verified ON context_retirement_records BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    assert!(
        selection
            .install(&f.service, &f.request, &owner, &cleanup)
            .is_err()
    );
    let path = admin(&f.git(), &f.request.item_id).join(MARKER);
    let installed = std::fs::read(&path).unwrap();
    f.db()
        .execute_batch("DROP TRIGGER instance_test_interrupt")
        .unwrap();
    assert!(
        finish_provisional(&f.service, &f.request, &owner, &cleanup).is_ok(),
        "matching full durable marker, not the intent alone, may finish verified SQL"
    );
    assert_eq!(std::fs::read(path).unwrap(), installed);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT instance_verified FROM context_retirement_records",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}
#[test]
fn retirement_instance_missing_marker_verification_never_installs() {
    for recreated in [false, true] {
        let f = Fixture::new();
        let (cleanup, selection, owner) = f.selected();
        drop(selection);
        let path = admin(&f.git(), &f.request.item_id);
        if recreated {
            let old = f.git().commondir().join("old-admin-fixture");
            std::fs::rename(&path, &old).unwrap();
            copy_directory(&old, &path);
        }
        let before = super::super::execution_tests::registry_image(&f.db());
        assert!(finish_provisional(&f.service, &f.request, &owner, &cleanup).is_err());
        assert!(!path.join(MARKER).exists());
        assert_eq!(
            super::super::execution_tests::registry_image(&f.db()),
            before
        );
    }
}
#[test]
fn retirement_instance_corrupt_collision_is_never_repaired() {
    for data in [
        Vec::new(),
        b"partial".to_vec(),
        vec![b'x'; LIMIT as usize + 1],
        bytes(
            AuthoringKind::Document,
            &ID.parse().unwrap(),
            OperationId::new(),
        ),
        [
            bytes(
                AuthoringKind::Ticket,
                &ID.parse().unwrap(),
                OperationId::new(),
            ),
            b"trailing".to_vec(),
        ]
        .concat(),
    ] {
        let f = Fixture::new();
        let path = admin(&f.git(), &f.request.item_id).join(MARKER);
        std::fs::write(&path, &data).unwrap();
        let cleanup = ContextCleanup::acquire(&f.request.root).unwrap();
        assert!(Selection::capture(&f.git(), &f.request, &cleanup).is_err());
        assert_eq!(std::fs::read(path).unwrap(), data);
    }
}
#[test]
fn retirement_instance_valid_existing_token_reused_without_writing() {
    let f = Fixture::new();
    let path = admin(&f.git(), &f.request.item_id).join(MARKER);
    let nonce = OperationId::new();
    let data = bytes(AuthoringKind::Ticket, &ID.parse().unwrap(), nonce);
    std::fs::write(&path, &data).unwrap();
    let (cleanup, selection, owner) = f.selected();
    assert_eq!(selection.nonce(), nonce);
    assert!(!selection.create);
    selection
        .install(&f.service, &f.request, &owner, &cleanup)
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), data);
}
#[cfg(unix)]
#[test]
fn retirement_instance_symlink_directory_and_hardlink_collisions_preserved() {
    for kind in ["symlink", "directory", "hardlink"] {
        let f = Fixture::new();
        let path = admin(&f.git(), &f.request.item_id).join(MARKER);
        let target = f.data.path().join("marker-fixture");
        let data = bytes(
            AuthoringKind::Ticket,
            &ID.parse().unwrap(),
            OperationId::new(),
        );
        std::fs::write(&target, &data).unwrap();
        match kind {
            "symlink" => std::os::unix::fs::symlink(&target, &path).unwrap(),
            "directory" => std::fs::create_dir(&path).unwrap(),
            _ => std::fs::hard_link(&target, &path).unwrap(),
        }
        let cleanup = ContextCleanup::acquire(&f.request.root).unwrap();
        assert!(Selection::capture(&f.git(), &f.request, &cleanup).is_err());
        assert_eq!(std::fs::read(target).unwrap(), data);
        let metadata = std::fs::symlink_metadata(path).unwrap();
        assert!(if kind == "symlink" {
            metadata.file_type().is_symlink()
        } else if kind == "directory" {
            metadata.is_dir()
        } else {
            metadata.is_file()
        });
    }
}

#[test]
fn retirement_instance_p2_root_head_requires_valid_direct_existing_binding() {
    for head in [
        "invalid\n",
        "ref: refs/heads/retirement-alias\n",
        "ref: refs/heads/missing\n",
        "1111111111111111111111111111111111111111\n",
    ] {
        let f = Fixture::new();
        let git = f.git();
        let tip = git
            .refname_to_id(&format!("refs/heads/manyhands/ticket/{ID}"))
            .unwrap();
        let cleanup = ContextCleanup::acquire(&f.request.root).unwrap();
        git.reference_symbolic(
            "refs/heads/retirement-alias",
            &format!("refs/heads/manyhands/ticket/{ID}"),
            false,
            "root alias fixture",
        )
        .unwrap();
        std::fs::write(git.path().join("HEAD"), head).unwrap();
        assert!(
            identity(&git, &f.request, tip).is_err(),
            "uncertain root HEAD must refuse before pruning"
        );
        assert_eq!(
            std::fs::read_to_string(git.path().join("HEAD")).unwrap(),
            head
        );
        assert!(!admin(&git, &f.request.item_id).join(MARKER).exists());
        drop(cleanup);
    }
}
