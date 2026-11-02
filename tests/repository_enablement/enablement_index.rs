use super::*;
use std::path::Path;

use manyhands::{
    repository::{Mutation, TicketCreateInput},
    results::ResultCode,
};
use support::{items, mutation};

pub(super) fn assert_configuration_current(root: &Path) {
    // Open afresh: this must be the on-disk index, not a cached handle.
    let repository = Repository::open(root).unwrap();
    let path = Path::new(canonical::CONFIG_PATH);
    let committed = repository
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(path)
        .unwrap();
    let entry = repository
        .index()
        .unwrap()
        .get_path(path, 0)
        .expect("configuration must be indexed");
    assert_eq!(
        (entry.id, entry.mode),
        (committed.id(), committed.filemode() as u32)
    );
    assert_eq!(repository.status_file(path).unwrap(), git2::Status::CURRENT);
}

fn assert_onboarding_continues(service: &RepositoryService, root: &Path) {
    assert_configuration_current(root);
    let created = mutation::execute_with(
        service,
        mutation::REQUEST_1,
        Mutation::TicketCreate(TicketCreateInput {
            root: root.to_owned(),
            id: items::item_id(mutation::TICKET_NEW),
            draft: mutation::World::draft(
                "First ticket",
                "Created immediately after onboarding.\n",
            ),
            deps: Vec::new(),
            parent: None,
            initials: Some("ab".to_owned()),
        }),
    );
    assert_eq!(created.code, ResultCode::Ok, "{}", created.message);
    service
        .add_remote(add_remote_request(
            root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    assert!(matches!(
        service
            .set_publication_remote(publication_request(root, Some("origin")))
            .unwrap(),
        PublicationRemoteOutcome::Changed { .. }
    ));
}

#[test]
fn create_absent_and_empty_leave_index_current_and_allow_onboarding() {
    for exists in [false, true] {
        let data = tempfile::tempdir().unwrap();
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("project");
        if exists {
            std::fs::create_dir(&root).unwrap();
        }
        let service = RepositoryService::open_at(data.path()).unwrap();
        assert!(matches!(
            service
                .create_and_enable(create_request(&root, "main"))
                .unwrap(),
            EnableRepositoryOutcome::Enabled { .. }
        ));
        assert_onboarding_continues(&service, &root);
    }
}

#[test]
fn enable_born_and_unborn_leave_index_current_and_allow_onboarding() {
    for born in [false, true] {
        let fixture = if born {
            support::born_repository()
        } else {
            support::unborn_repository()
        };
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let mut request = enable_request(&fixture.root, "main");
        request.identity = create_request(&fixture.root, "main").identity;
        service.enable(request).unwrap();
        assert_onboarding_continues(&service, &fixture.root);
    }
}

#[test]
fn enable_moves_unborn_head_and_publishes_configuration_index() {
    let fixture = support::unborn_repository();
    fixture.repository.set_head("refs/heads/old").unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let mut request = enable_request(&fixture.root, "trunk");
    request.identity = create_request(&fixture.root, "trunk").identity;
    service.enable(request).unwrap();
    assert_eq!(
        fixture.repository.head().unwrap().shorthand(),
        Some("trunk")
    );
    assert_configuration_current(&fixture.root);
}

#[test]
fn lock_failure_keeps_success_and_domain_retry_repairs_missing_entry() {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let request = enable_request(&fixture.root, "main");
    let index = fixture.repository.path().join("index");
    let before = std::fs::read(&index).unwrap();
    let lock = fixture.repository.path().join("index.lock");
    std::fs::write(&lock, b"foreign lock").unwrap();
    let EnableRepositoryOutcome::Enabled { commit_oid } = service.enable(request.clone()).unwrap()
    else {
        panic!("commit remains authoritative");
    };
    assert_eq!(std::fs::read(&index).unwrap(), before);
    assert_eq!(std::fs::read(&lock).unwrap(), b"foreign lock");
    std::fs::remove_file(lock).unwrap();
    assert_eq!(
        service.enable(request).unwrap(),
        EnableRepositoryOutcome::AlreadyEnabled
    );
    assert_eq!(support::head_commit(&fixture.repository), Some(commit_oid));
    assert_configuration_current(&fixture.root);
}

#[test]
fn retry_uses_current_head_configuration_and_preserves_later_staging() {
    for staged in [false, true] {
        let fixture = support::born_repository();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let request = enable_request(&fixture.root, "main");
        let repository = &fixture.repository;
        let lock = repository.path().join("index.lock");
        std::fs::write(&lock, b"foreign lock").unwrap();
        service.enable(request.clone()).unwrap();
        std::fs::remove_file(lock).unwrap();

        // HEAD advances with a different valid configuration. Build the tree
        // independently, leaving the original failed publication unrepaired.
        let path = Path::new(canonical::CONFIG_PATH);
        let mut source = std::fs::read_to_string(fixture.root.join(path)).unwrap();
        source.push_str("\n# configuration committed later\n");
        std::fs::write(fixture.root.join(path), &source).unwrap();
        let parent = repository.head().unwrap().peel_to_commit().unwrap();
        let mut tree = repository
            .treebuilder(Some(&parent.tree().unwrap()))
            .unwrap();
        let manyhands = parent.tree().unwrap().get_name(".manyhands").unwrap().id();
        let mut subtree = repository
            .treebuilder(Some(&repository.find_tree(manyhands).unwrap()))
            .unwrap();
        subtree
            .insert(
                "config.toml",
                repository.blob(source.as_bytes()).unwrap(),
                0o100644,
            )
            .unwrap();
        tree.insert(".manyhands", subtree.write().unwrap(), 0o040000)
            .unwrap();
        let tree = repository.find_tree(tree.write().unwrap()).unwrap();
        let signature = repository.signature().unwrap();
        let advanced = repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Later configuration",
                &tree,
                &[&parent],
            )
            .unwrap();

        let staged_entry = if staged {
            std::fs::write(fixture.root.join(path), b"user staged content\n").unwrap();
            let mut index = repository.index().unwrap();
            index.add_path(path).unwrap();
            index.write().unwrap();
            std::fs::write(fixture.root.join(path), &source).unwrap();
            Some(format!("{:?}", index.get_path(path, 0).unwrap()))
        } else {
            None
        };
        service.enable(request).unwrap();
        assert_eq!(support::head_commit(repository), Some(advanced));
        if let Some(staged_entry) = staged_entry {
            let reopened = Repository::open(&fixture.root).unwrap();
            assert_eq!(
                format!("{:?}", reopened.index().unwrap().get_path(path, 0).unwrap()),
                staged_entry
            );
        } else {
            assert_configuration_current(&fixture.root);
        }
        assert_eq!(
            std::fs::read_to_string(fixture.root.join(path)).unwrap(),
            source
        );
    }
}

#[test]
fn foreign_lock_on_unborn_enable_leaves_no_index_until_domain_retry() {
    let fixture = support::unborn_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let mut request = enable_request(&fixture.root, "main");
    request.identity = create_request(&fixture.root, "main").identity;
    let lock = fixture.repository.path().join("index.lock");
    std::fs::write(&lock, b"foreign lock").unwrap();
    assert!(matches!(
        service.enable(request.clone()).unwrap(),
        EnableRepositoryOutcome::Enabled { .. }
    ));
    assert!(!fixture.repository.path().join("index").exists());
    assert_eq!(std::fs::read(&lock).unwrap(), b"foreign lock");
    std::fs::remove_file(lock).unwrap();
    service.enable(request).unwrap();
    assert_configuration_current(&fixture.root);
}
