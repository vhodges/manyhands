use super::*;
use std::path::Path;

use manyhands::{
    repository::{Mutation, RemoteSelectInput, RepoSyncInput, TicketCreateInput},
    results::{PublicationEffect, ResultCode},
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

// `01M669FJR2`: a remote selection leaves the index current for what it
// commits.

/// An enabled repository holding a remote, ready to select.
fn selectable() -> (
    support::TestRepository,
    tempfile::TempDir,
    RepositoryService,
) {
    let fixture = support::born_repository();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    let mut request = enable_request(&fixture.root, "main");
    request.identity = create_request(&fixture.root, "main").identity;
    service.enable(request).unwrap();
    service
        .add_remote(add_remote_request(
            &fixture.root,
            "origin",
            "git@example.invalid:project.git",
        ))
        .unwrap();
    assert_configuration_current(&fixture.root);
    (fixture, data, service)
}

/// What a selection made before the fix left: HEAD and the worktree hold the
/// new configuration, and the index the one before it.
fn stale_index_of_a_selection(root: &Path) {
    let repository = Repository::open(root).unwrap();
    let path = Path::new(canonical::CONFIG_PATH);
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    let parent = head.parent(0).unwrap();
    let entry = parent.tree().unwrap().get_path(path).unwrap();
    let mut index = repository.index().unwrap();
    let mut stale = index.get_path(path, 0).unwrap();
    stale.id = entry.id();
    index.add(&stale).unwrap();
    index.write().unwrap();
    assert_ne!(
        Repository::open(root).unwrap().status_file(path).unwrap(),
        git2::Status::CURRENT
    );
}

#[test]
fn remote_select_leaves_the_configuration_current() {
    let (fixture, _data, service) = selectable();
    assert!(matches!(
        service
            .set_publication_remote(publication_request(&fixture.root, Some("origin")))
            .unwrap(),
        PublicationRemoteOutcome::Changed { .. }
    ));
    assert_configuration_current(&fixture.root);
    // Clearing it commits the file again.
    assert!(matches!(
        service
            .set_publication_remote(publication_request(&fixture.root, None))
            .unwrap(),
        PublicationRemoteOutcome::Changed { .. }
    ));
    assert_configuration_current(&fixture.root);
}

#[test]
fn remote_select_leaves_other_index_entries_as_they_are() {
    let (fixture, _data, service) = selectable();
    std::fs::write(fixture.root.join("staged.txt"), b"staged\n").unwrap();
    std::fs::write(fixture.root.join("unstaged.txt"), b"tracked\n").unwrap();
    let staged = {
        let repository = Repository::open(&fixture.root).unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();
        index.get_path(Path::new("staged.txt"), 0).unwrap().id
    };
    let others = |root: &Path| -> Vec<(Vec<u8>, git2::Oid)> {
        Repository::open(root)
            .unwrap()
            .index()
            .unwrap()
            .iter()
            .filter(|entry| entry.path != canonical::CONFIG_PATH.as_bytes())
            .map(|entry| (entry.path, entry.id))
            .collect()
    };
    let before = others(&fixture.root);
    assert!(before.iter().any(|(_, id)| *id == staged));
    service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap();
    assert_eq!(others(&fixture.root), before);
    let repository = Repository::open(&fixture.root).unwrap();
    assert_eq!(
        repository.status_file(Path::new("staged.txt")).unwrap(),
        git2::Status::INDEX_NEW
    );
    assert_eq!(
        repository.status_file(Path::new("unstaged.txt")).unwrap(),
        git2::Status::WT_NEW
    );
    assert_configuration_current(&fixture.root);
}

#[test]
fn a_retry_of_a_remote_select_leaves_the_index_current() {
    let (fixture, _data, service) = selectable();
    let operation_id = support::operation_id();
    let request =
        || publication_request_with_operation_id(&fixture.root, Some("origin"), operation_id);
    let first = service.set_publication_remote(request()).unwrap();
    assert_configuration_current(&fixture.root);
    assert_eq!(service.set_publication_remote(request()).unwrap(), first);
    assert_configuration_current(&fixture.root);
}

#[test]
fn a_retry_heals_an_index_a_selection_left_stale() {
    let (fixture, _data, service) = selectable();
    let operation_id = support::operation_id();
    let request =
        || publication_request_with_operation_id(&fixture.root, Some("origin"), operation_id);
    let first = service.set_publication_remote(request()).unwrap();
    stale_index_of_a_selection(&fixture.root);
    assert_eq!(service.set_publication_remote(request()).unwrap(), first);
    assert_configuration_current(&fixture.root);
}

#[test]
fn a_new_no_op_selection_heals_an_index_a_selection_left_stale() {
    let (fixture, _data, service) = selectable();
    service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap();
    stale_index_of_a_selection(&fixture.root);
    let head = support::head_commit(&fixture.repository);
    service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap();
    assert_eq!(support::head_commit(&fixture.repository), head);
    assert_configuration_current(&fixture.root);
}

#[test]
fn a_new_no_op_selection_leaves_a_current_index_alone_under_a_lock() {
    let (fixture, _data, service) = selectable();
    service
        .set_publication_remote(publication_request(&fixture.root, Some("origin")))
        .unwrap();
    // Another Git process holds the index: a selection that has nothing to
    // write there has no reason to wait for it or to fail.
    let lock = fixture.repository.path().join("index.lock");
    std::fs::write(&lock, b"").unwrap();
    let answered =
        service.set_publication_remote(publication_request(&fixture.root, Some("origin")));
    std::fs::remove_file(&lock).unwrap();
    assert!(answered.is_ok());
    assert_configuration_current(&fixture.root);
}

#[test]
fn a_stop_between_the_commit_and_the_index_recovers_on_retry() {
    let (fixture, data, _service) = selectable();
    let failing = support::FailOnce::at(FailurePoint::AfterPublicationConfigurationCommit)
        .open_service(data.path());
    let operation_id = support::operation_id();
    let request =
        || publication_request_with_operation_id(&fixture.root, Some("origin"), operation_id);
    let before = support::head_commit(&fixture.repository);
    let error = failing.set_publication_remote(request()).unwrap_err();
    assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    let committed = support::head_commit(&fixture.repository).unwrap();
    assert_ne!(Some(committed), before);
    let repository = Repository::open(&fixture.root).unwrap();
    let path = Path::new(canonical::CONFIG_PATH);
    assert_ne!(
        repository.status_file(path).unwrap(),
        git2::Status::CURRENT,
        "the stop came before the index moved"
    );
    drop(repository);
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert_eq!(
        service.set_publication_remote(request()).unwrap(),
        PublicationRemoteOutcome::Changed {
            commit_oid: committed
        }
    );
    assert_eq!(support::head_commit(&fixture.repository), Some(committed));
    assert_configuration_current(&fixture.root);
}

/// The ticket's journey, through the boundary, with its stale index as the
/// control: refused as not clean, every effect `not_requested`, and with the
/// index brought up to date past that refusal to the transport's own answer.
#[test]
fn the_synchronization_after_a_selection_is_not_refused_as_not_clean() {
    let (fixture, _data, service) = selectable();
    let select = || {
        Mutation::RemoteSelect(RemoteSelectInput {
            root: fixture.root.clone(),
            name: Some("origin".to_owned()),
        })
    };
    let sync = || {
        Mutation::RepoSync(RepoSyncInput {
            root: fixture.root.clone(),
        })
    };
    let selected = mutation::execute_with(&service, mutation::REQUEST_1, select());
    assert_eq!(selected.code, ResultCode::Ok, "{}", selected.message);
    assert_configuration_current(&fixture.root);
    let replayed = mutation::execute_with(&service, mutation::REQUEST_1, select());
    assert_eq!(replayed.code, ResultCode::Ok);
    assert_configuration_current(&fixture.root);

    let synchronized = mutation::execute_with(&service, mutation::REQUEST_2, sync());
    assert_ne!(synchronized.code, ResultCode::WorktreeNotClean);
    assert_ne!(
        synchronized.effects.publication,
        PublicationEffect::NotRequested
    );

    stale_index_of_a_selection(&fixture.root);
    let refused = mutation::execute_with(&service, mutation::REQUEST_3, sync());
    assert_eq!(refused.code, ResultCode::WorktreeNotClean);
}
