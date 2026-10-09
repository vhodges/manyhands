use std::fs;

use git2::{Config, Repository, Signature, Time, WorktreeAddOptions};
#[cfg(unix)]
use manyhands::repository::OwnedPathBoundary;
use manyhands::repository::{
    AuthoringKind, AuthoringTarget, CommentPublicationState, CommentSubmissionOutcome,
    ContextIntent, ContextProvisionOutcome, DocumentDraft, EnableRepositoryOutcome,
    EnableRepositoryRequest, ExpectedPathObservation, ExternalChangeExpectation, FailurePoint,
    LocalCheckpoint, RepositoryErrorKind, RepositoryOperation, RepositoryService,
    SaveDocumentRequest, SaveOutcome, SaveTicketRequest, SubmitCommentRequest, TicketDraft,
};

mod support;

#[cfg(unix)]
static OWNED_PATH_HOOK_TEST_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();

#[cfg(unix)]
fn owned_path_hook_test_lock() -> std::sync::MutexGuard<'static, ()> {
    OWNED_PATH_HOOK_TEST_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("owned-path hook test lock")
}

#[cfg(unix)]
fn repository_with_symlinked_parent() -> (tempfile::TempDir, support::TestRepository) {
    let mut fixture = support::born_repository();
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("parent-alias");
    std::os::unix::fs::symlink(
        fixture.root.parent().unwrap().canonicalize().unwrap(),
        &parent,
    )
    .unwrap();
    fixture.root = parent.join(fixture.root.file_name().unwrap());
    assert_ne!(fixture.root, fixture.root.canonicalize().unwrap());
    (directory, fixture)
}

#[test]
fn fixture_repositories_declare_local_lf_checkout_policy() {
    for fixture in [support::born_repository(), support::unborn_repository()] {
        let local = Config::open(&fixture.repository.path().join("config")).unwrap();
        assert!(!local.get_bool("core.autocrlf").unwrap());
        assert!(
            !fixture
                .repository
                .config()
                .unwrap()
                .get_bool("core.autocrlf")
                .unwrap()
        );
    }
    let fixture = support::born_repository();
    assert_eq!(
        support::commit_tree_path(
            &fixture.repository,
            support::head_commit(&fixture.repository).unwrap(),
            "fixture.txt"
        ),
        Some(b"fixture\n".to_vec())
    );
    let linked = fixture.root.join("lf-checkout");
    fixture
        .repository
        .worktree("lf-checkout", &linked, None)
        .unwrap();
    assert_eq!(fs::read(linked.join("fixture.txt")).unwrap(), b"fixture\n");
}

#[cfg(unix)]
#[test]
fn registered_worktree_helpers_compare_physical_paths_without_losing_names_or_count() {
    let fixture = support::born_repository();
    let worktree = fixture.root.join("registered");
    fixture
        .repository
        .worktree("registered", &worktree, None)
        .unwrap();
    let alias = fixture.root.join("registered-alias");
    std::os::unix::fs::symlink(&worktree, &alias).unwrap();
    // Git metadata may name an existing filesystem alias rather than its canonical spelling.
    fs::write(
        fixture
            .repository
            .commondir()
            .join("worktrees/registered/gitdir"),
        format!("{}\n", alias.join(".git").display()),
    )
    .unwrap();
    let raw = fixture
        .repository
        .find_worktree("registered")
        .unwrap()
        .path()
        .to_owned();
    assert_ne!(raw, worktree.canonicalize().unwrap());
    assert_eq!(
        raw.canonicalize().unwrap(),
        worktree.canonicalize().unwrap()
    );
    let before = support::repository_and_worktree_snapshot(&fixture);
    let git_before = support::repository_git_file_bytes(&fixture);
    assert_eq!(
        worktree_paths(&fixture.repository),
        vec![worktree.canonicalize().unwrap()]
    );
    assert_eq!(
        registered_worktrees(&fixture.repository),
        vec![("registered".to_owned(), worktree.canonicalize().unwrap())]
    );
    assert_eq!(
        support::open_linked_worktree(&alias)
            .worktree
            .canonicalize()
            .unwrap(),
        worktree.canonicalize().unwrap()
    );
    assert!(support::repository_and_worktree_snapshot(&fixture) == before);
    assert!(support::repository_git_file_bytes(&fixture) == git_before);
}

#[test]
fn registered_worktree_helpers_keep_missing_registrations_in_the_exact_set() {
    let fixture = support::born_repository();
    let existing = fixture.root.join("existing");
    let missing = fixture.root.join("missing");
    fixture
        .repository
        .worktree("existing", &existing, None)
        .unwrap();
    fixture
        .repository
        .worktree("missing", &missing, None)
        .unwrap();
    fs::remove_dir_all(&missing).unwrap();
    let missing_git_path = fixture
        .repository
        .find_worktree("missing")
        .unwrap()
        .path()
        .to_owned();
    let mut expected_paths = vec![existing.canonicalize().unwrap(), missing_git_path.clone()];
    expected_paths.sort();
    assert_eq!(worktree_paths(&fixture.repository), expected_paths);
    assert_eq!(
        registered_worktrees(&fixture.repository),
        vec![
            ("existing".to_owned(), existing.canonicalize().unwrap()),
            ("missing".to_owned(), missing_git_path),
        ]
    );
}

#[test]
fn fixture_enabled_repository_keeps_its_service_data_directory_alive() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);

    assert!(enabled.data_directory.path().is_dir());
    assert!(
        enabled
            .data_directory
            .path()
            .join(manyhands::repository::REGISTRY_FILE)
            .is_file()
    );
    assert_eq!(
        enabled.service.inspect(&fixture.root).unwrap().head_branch,
        Some("main".to_owned())
    );
}

#[test]
fn fixture_linked_worktree_snapshot_exposes_live_checked_out_state() {
    assert_linked_worktree_snapshot(support::born_repository());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_linked_worktree_snapshot_exposes_live_checked_out_state() {
    let (_alias_directory, fixture) = repository_with_symlinked_parent();
    assert_linked_worktree_snapshot(fixture);
}

fn assert_linked_worktree_snapshot(fixture: support::TestRepository) {
    let worktree = fixture.root.join(".manyhands/worktrees/document-context");
    fs::create_dir_all(worktree.parent().unwrap()).unwrap();
    fixture
        .repository
        .worktree("document-context", &worktree, None)
        .unwrap();

    let snapshot = support::open_linked_worktree(&worktree);

    assert_eq!(
        snapshot.worktree.canonicalize().unwrap(),
        worktree.canonicalize().unwrap()
    );
    assert_eq!(snapshot.head_branch, "document-context");
    assert_eq!(
        Some(snapshot.head_commit),
        support::head_commit(&fixture.repository)
    );
    let linked_repository = Repository::open(&worktree).unwrap();
    assert_eq!(
        snapshot.index,
        support::index_bytes(&linked_repository).unwrap()
    );
    assert_eq!(
        support::commit_tree_path(&fixture.repository, snapshot.head_commit, "fixture.txt"),
        Some(b"fixture\n".to_vec())
    );
}

#[test]
fn prepare_context_creates_a_document_context_at_its_deterministic_location() {
    assert_document_context_location(support::born_repository());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_document_context_uses_its_deterministic_location() {
    let (_alias_directory, fixture) = repository_with_symlinked_parent();
    assert_document_context_location(fixture);
}

fn assert_document_context_location(fixture: support::TestRepository) {
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        service
            .enable(EnableRepositoryRequest {
                root: fixture.root.clone(),
                primary_branch: "main".to_owned(),
                identity: None,
                operation_id: support::operation_id(),
            })
            .unwrap(),
        EnableRepositoryOutcome::Enabled { .. },
    ));
    clean_configuration_index(&fixture);
    let item_id = support::document_id();

    let context = service
        .prepare_context(AuthoringTarget {
            root: fixture.root.clone(),
            kind: AuthoringKind::Document,
            item_id,
            intent: ContextIntent::Create,
            operation_id: support::operation_id(),
        })
        .unwrap();

    let ContextProvisionOutcome::Created(context) = context else {
        panic!("the first context provision must create a worktree");
    };
    assert_eq!(
        context.branch,
        "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV"
    );
    assert_eq!(
        context.worktree,
        fixture
            .root
            .canonicalize()
            .unwrap()
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV")
    );
    assert_eq!(
        support::open_linked_worktree(&context.worktree).head_branch,
        context.branch
    );
}

#[test]
fn context_ticket_create_uses_its_deterministic_branch_and_worktree() {
    assert_ticket_context_location(support::born_repository());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_ticket_context_uses_its_deterministic_branch_and_worktree() {
    let (_alias_directory, fixture) = repository_with_symlinked_parent();
    assert_ticket_context_location(fixture);
}

fn assert_ticket_context_location(fixture: support::TestRepository) {
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let context = enabled
        .service
        .prepare_context(target(
            &fixture.root,
            AuthoringKind::Ticket,
            support::ticket_id(),
            ContextIntent::Create,
        ))
        .unwrap();

    let ContextProvisionOutcome::Created(context) = context else {
        panic!("the first context provision must create a worktree");
    };
    assert_eq!(
        context.branch,
        "manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW"
    );
    assert_eq!(
        context.worktree,
        fixture
            .root
            .canonicalize()
            .unwrap()
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAW")
    );
}

#[test]
fn context_create_reuses_the_exact_existing_context() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let first = enabled
        .service
        .prepare_context(target(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Create,
        ))
        .unwrap();
    let first = context_from(first);

    let second = enabled
        .service
        .prepare_context(target(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Create,
        ))
        .unwrap();
    let ContextProvisionOutcome::Reused(second) = second else {
        panic!("an exact existing context must be reused");
    };
    assert_eq!(second.branch, first.branch);
    assert_eq!(second.worktree, first.worktree);
}

#[test]
fn context_edit_creates_a_worktree_from_the_primary_target() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let context = enabled
        .service
        .prepare_context(target(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Edit,
        ))
        .unwrap();
    let context = context_from(context);

    assert_eq!(
        fs::read_to_string(context.worktree.join("docs/fixture.md")).unwrap(),
        support::document_source()
    );
}

#[test]
fn context_edit_with_local_autocrlf_true_observes_exact_crlf_and_preserves_document_body() {
    let fixture = support::born_repository();
    let mut local = Config::open(&fixture.repository.path().join("config")).unwrap();
    local.set_bool("core.autocrlf", true).unwrap();
    let lf = concat!(
        "---\nmanyhands_managed: true\nmanyhands_kind: document\n",
        "id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Fixture document\n",
        "future_key: retained\n---\nExact body\nSecond line\n"
    );
    let crlf = concat!(
        "---\r\nmanyhands_managed: true\r\nmanyhands_kind: document\r\n",
        "id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\r\ntitle: Fixture document\r\n",
        "future_key: retained\r\n---\r\nExact body\r\nSecond line\r\n"
    );
    commit_source(&fixture, "docs/filtered.md", lf);
    assert_eq!(
        support::commit_tree_path(
            &fixture.repository,
            support::head_commit(&fixture.repository).unwrap(),
            "docs/filtered.md"
        ),
        Some(lf.as_bytes().to_vec())
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    assert_eq!(
        fs::read(fixture.root.join("docs/filtered.md")).unwrap(),
        lf.as_bytes()
    );
    assert_eq!(
        fs::read(context.worktree.join("docs/filtered.md")).unwrap(),
        crlf.as_bytes()
    );
    let observed =
        expected_context_observation(&fixture.root, &support::document_id(), "docs/filtered.md");
    assert_eq!(
        observed,
        ExpectedPathObservation::from_bytes(crlf.as_bytes())
    );
    assert_ne!(observed, ExpectedPathObservation::from_bytes(lf.as_bytes()));

    let body = "Exact body\r\nSecond line\r\n";
    let mut stale = document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some("docs/filtered.md"),
        "docs/filtered.md",
        "Edited",
        body,
    );
    stale.expected_source = Some(ExpectedPathObservation::from_bytes(lf.as_bytes()));
    let before = support::repository_and_worktree_snapshot(&fixture);
    let git_before = support::repository_git_file_bytes(&fixture);
    let records_before = operation_record_rows(&enabled.service);
    let error = document_error(enabled.service.save_document(stale));
    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    assert!(support::repository_and_worktree_snapshot(&fixture) == before);
    assert!(support::repository_git_file_bytes(&fixture) == git_before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);

    let (saved, commit) = saved_checkpoint(
        enabled
            .service
            .save_document(document_request(
                &fixture.root,
                ContextIntent::Edit,
                Some("docs/filtered.md"),
                "docs/filtered.md",
                "Edited",
                body,
            ))
            .unwrap(),
    );
    assert_eq!(saved.worktree, context.worktree);
    let edited = fs::read_to_string(saved.worktree.join("docs/filtered.md")).unwrap();
    let manyhands::canonical::CanonicalItem::Document(document) =
        manyhands::canonical::parse_item(std::path::Path::new("docs/filtered.md"), &edited)
            .unwrap()
    else {
        panic!("expected document");
    };
    assert_eq!(document.body, body);
    assert_eq!(document.title, "Edited");
    assert_eq!(
        document.unknown.get(serde_yaml::Value::from("future_key")),
        Some(&serde_yaml::Value::from("retained"))
    );
    // Checkout applies Git filters; the owned-file checkpoint stores authored bytes exactly.
    let committed =
        support::commit_tree_path(&fixture.repository, commit, "docs/filtered.md").unwrap();
    assert_eq!(committed, edited.as_bytes());
    assert!(local.get_bool("core.autocrlf").unwrap());
    assert!(
        Repository::open(&saved.worktree)
            .unwrap()
            .config()
            .unwrap()
            .get_bool("core.autocrlf")
            .unwrap()
    );
}

#[test]
fn context_rejects_an_unenabled_repository_without_creating_a_branch() {
    let fixture = support::born_repository();
    let service = RepositoryService::open_at(tempfile::tempdir().unwrap().path()).unwrap();
    let branch = "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV";

    let error = context_error(service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::RepositoryNotEnabled);
    assert!(
        fixture
            .repository
            .find_branch(branch, git2::BranchType::Local)
            .is_err()
    );
}

#[test]
fn context_rejects_an_invalid_repository_configuration() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    fs::write(fixture.root.join(".manyhands/config.toml"), "not valid = [").unwrap();

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidConfiguration);
    assert!(
        fixture
            .repository
            .find_branch(
                "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
                git2::BranchType::Local
            )
            .is_err()
    );
}

#[test]
fn context_rejects_a_primary_repository_on_the_wrong_branch() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    fixture
        .repository
        .branch(
            "other",
            &fixture.repository.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap();
    fixture.repository.set_head("refs/heads/other").unwrap();

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::WrongCheckedOutBranch);
    assert!(
        !fixture
            .root
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .exists()
    );
}

#[test]
fn context_rejects_an_authoring_branch_that_is_the_primary_branch_without_mutation() {
    let fixture = support::born_repository();
    let item_id = support::document_id();
    let branch = format!("manyhands/document/{item_id}");
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture.repository.branch(&branch, &head, false).unwrap();
    fixture
        .repository
        .set_head(&format!("refs/heads/{branch}"))
        .unwrap();
    let data = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(data.path()).unwrap();
    assert!(matches!(
        service
            .enable(EnableRepositoryRequest {
                root: fixture.root.clone(),
                primary_branch: branch.clone(),
                identity: None,
                operation_id: support::operation_id(),
            })
            .unwrap(),
        EnableRepositoryOutcome::Enabled { .. },
    ));
    clean_configuration_index(&fixture);
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let before = context_state(&fixture, &service, Some(&configuration_path));

    let error = context_error(service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(&fixture, &service, Some(&configuration_path), &before);
}

#[test]
fn context_rejects_an_uninspectable_registered_worktree_without_mutation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let branch = format!("manyhands/document/{item_id}");
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture.repository.branch(&branch, &head, false).unwrap();
    let stale = fixture.root.join(".manyhands/worktrees/stale");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    let reference = fixture
        .repository
        .find_reference(&format!("refs/heads/{branch}"))
        .unwrap();
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));
    fixture
        .repository
        .worktree("stale", &stale, Some(&options))
        .unwrap();
    fs::remove_dir_all(&stale).unwrap();
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let before = context_state(&fixture, &enabled.service, Some(&configuration_path));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(
        &fixture,
        &enabled.service,
        Some(&configuration_path),
        &before,
    );
}

#[cfg(unix)]
#[test]
fn context_rejects_a_non_utf8_registered_worktree_name_without_mutation() {
    use std::os::unix::ffi::OsStringExt;

    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let registered = fixture.root.join(".manyhands/worktrees/registered");
    fs::create_dir_all(registered.parent().unwrap()).unwrap();
    fixture
        .repository
        .worktree("registered", &registered, None)
        .unwrap();
    let worktrees_directory = fixture.repository.commondir().join("worktrees");
    let invalid_name = std::ffi::OsString::from_vec(b"non-utf8-\xff".to_vec());
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let filesystem_state_before =
        context_state(&fixture, &enabled.service, Some(&configuration_path));
    let snapshot_before = support::repository_and_worktree_snapshot(&fixture);
    let git_files_before = support::repository_git_file_bytes(&fixture);
    let records_before = operation_record_rows(&enabled.service);
    let names_before = registered_worktree_names(&fixture.repository);

    if let Err(error) = fs::rename(
        worktrees_directory.join("registered"),
        worktrees_directory.join(&invalid_name),
    ) {
        match error.raw_os_error() {
            Some(libc::EILSEQ) if cfg!(target_os = "macos") => {}
            _ => panic!("invalid-byte worktree fixture rename failed unexpectedly: {error}"),
        }
        // APFS can reject the invalid bytes before the application can inspect them.
        // This branch proves filesystem rejection and nonmutation, not the application guard.
        assert_context_state(
            &fixture,
            &enabled.service,
            Some(&configuration_path),
            &filesystem_state_before,
        );
        assert_eq!(
            support::repository_and_worktree_snapshot(&fixture),
            snapshot_before
        );
        assert_eq!(
            support::repository_git_file_bytes(&fixture),
            git_files_before
        );
        assert_eq!(operation_record_rows(&enabled.service), records_before);
        assert_eq!(registered_worktree_names(&fixture.repository), names_before);
        return;
    }

    let before = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let worktree_names_before = registered_worktree_names(&fixture.repository);
    assert!(worktree_names_before.contains(&b"non-utf8-\xff".to_vec()));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(
        &fixture,
        &enabled.service,
        Some(&configuration_path),
        &before,
    );
    assert_eq!(
        registered_worktree_names(&fixture.repository),
        worktree_names_before
    );
}

#[cfg(unix)]
#[test]
fn context_ignores_a_top_level_docs_symlink() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let external = tempfile::tempdir().unwrap();
    let external_document = external.path().join("collision.md");
    fs::write(&external_document, support::document_source()).unwrap();
    symlink(external.path(), fixture.root.join("docs")).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&external_document));

    let outcome = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    ));

    assert!(matches!(outcome, Ok(ContextProvisionOutcome::Created(_))));
    assert_eq!(
        fs::read(&external_document).unwrap(),
        before.relevant_file.unwrap()
    );
    assert_eq!(support::index_bytes(&fixture.repository), before.index);
    assert_eq!(registry_count(&enabled.service), before.registry_count);
}

#[cfg(unix)]
#[test]
fn document_edit_rejects_a_symlinked_reused_context_without_touching_external_repository() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    commit_source(&fixture, "docs/edit.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let external = support::born_repository();
    commit_source(&external, "docs/edit.md", &support::document_source());
    let external_commit = external
        .repository
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap();
    external
        .repository
        .branch(&context.branch, &external_commit, false)
        .unwrap();
    external
        .repository
        .set_head(&format!("refs/heads/{}", context.branch))
        .unwrap();
    let external_head = support::head_commit(&external.repository);
    let external_index = support::index_bytes(&external.repository);
    let external_fixture = fs::read(external.root.join("fixture.txt")).unwrap();
    fs::remove_dir_all(&context.worktree).unwrap();
    symlink(&external.root, &context.worktree).unwrap();
    let before = context_state(&fixture, &enabled.service, None);
    let primary_status = status_entries(&fixture.repository);

    let result = enabled.service.save_document(document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some("docs/edit.md"),
        "docs/edit.md",
        "Edited",
        "Edited body\n",
    ));

    let error = document_error(result);
    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(&fixture, &enabled.service, None, &before);
    assert_eq!(status_entries(&fixture.repository), primary_status);
    assert_eq!(support::head_commit(&external.repository), external_head);
    assert_eq!(support::index_bytes(&external.repository), external_index);
    assert_eq!(
        fs::read(external.root.join("fixture.txt")).unwrap(),
        external_fixture
    );
    assert!(
        fs::symlink_metadata(&context.worktree)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_link(&context.worktree).unwrap(), external.root);
    fs::remove_file(&context.worktree).unwrap();
}

#[test]
fn context_rejects_document_paths_exceeding_the_traversal_depth_without_mutation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let mut directory = fixture.root.join("docs");
    for segment in 0..17 {
        directory = directory.join(format!("nested-{segment}"));
    }
    fs::create_dir_all(&directory).unwrap();
    let candidate = directory.join("document.md");
    fs::write(&candidate, support::document_source()).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&candidate));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_context_state(&fixture, &enabled.service, Some(&candidate), &before);
}

#[cfg(unix)]
#[test]
fn context_rejects_a_symlinked_worktree_base_without_mutation() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let base = fixture.root.join(".manyhands/worktrees");
    let external = tempfile::tempdir().unwrap();
    let sentinel = external.path().join("sentinel");
    fs::write(&sentinel, "external\n").unwrap();
    symlink(external.path(), &base).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&sentinel));

    let result = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    ));

    assert!(!external.path().join("01ARZ3NDEKTSV4RRFFQ69G5FAV").exists());
    fs::remove_file(&base).unwrap();
    fs::create_dir(&base).unwrap();
    let error = context_error(result);
    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(&fixture, &enabled.service, Some(&sentinel), &before);
    assert_eq!(fs::read_to_string(sentinel).unwrap(), "external\n");
}

#[cfg(unix)]
#[test]
fn context_rejects_a_dangling_deterministic_worktree_symlink_without_mutation() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let base = fixture.root.join(".manyhands/worktrees");
    fs::create_dir_all(&base).unwrap();
    let external = tempfile::tempdir().unwrap();
    let external_target = external.path().join("missing-worktree");
    let leaf = base.join("01ARZ3NDEKTSV4RRFFQ69G5FAV");
    symlink(&external_target, &leaf).unwrap();
    let before = context_state(&fixture, &enabled.service, None);
    let primary_status = status_entries(&fixture.repository);

    let result = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    ));

    fs::remove_file(&leaf).unwrap();
    let error = context_error(result);
    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(&fixture, &enabled.service, None, &before);
    assert_eq!(status_entries(&fixture.repository), primary_status);
    assert!(!external_target.exists());
}

#[cfg(unix)]
#[test]
fn context_rejects_a_dangling_worktree_base_ancestor_without_mutation() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let base = fixture.root.join(".manyhands/worktrees");
    let external = tempfile::tempdir().unwrap();
    let external_target = external.path().join("missing-base");
    symlink(&external_target, &base).unwrap();
    let before = context_state(&fixture, &enabled.service, None);
    let primary_status = status_entries(&fixture.repository);

    let result = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    ));

    assert!(result.is_err());
    assert_context_state(&fixture, &enabled.service, None, &before);
    assert_eq!(status_entries(&fixture.repository), primary_status);
    assert!(!external_target.exists());
    fs::remove_file(&base).unwrap();
    fs::create_dir(&base).unwrap();
    let error = context_error(result);
    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
}

#[cfg(unix)]
#[test]
fn context_rejects_a_real_symlinked_worktree_base_with_existing_leaf_without_mutation() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let base = fixture.root.join(".manyhands/worktrees");
    let external = tempfile::tempdir().unwrap();
    let external_leaf = external.path().join("01ARZ3NDEKTSV4RRFFQ69G5FAV");
    fs::create_dir(&external_leaf).unwrap();
    fs::write(external_leaf.join("sentinel"), "external\n").unwrap();
    symlink(external.path(), &base).unwrap();
    let before = context_state(&fixture, &enabled.service, None);
    let primary_status = status_entries(&fixture.repository);

    let result = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    ));

    assert!(result.is_err());
    assert_context_state(&fixture, &enabled.service, None, &before);
    assert_eq!(status_entries(&fixture.repository), primary_status);
    assert_eq!(
        fs::read_to_string(external_leaf.join("sentinel")).unwrap(),
        "external\n"
    );
    fs::remove_file(&base).unwrap();
    fs::create_dir(&base).unwrap();
    let error = context_error(result);
    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
}

#[test]
fn context_rejects_ticket_directories_exceeding_the_traversal_entry_limit_without_mutation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let tickets = fixture.root.join(".manyhands/tickets");
    for entry in 0..1025 {
        fs::create_dir_all(tickets.join(format!("ticket-{entry}"))).unwrap();
    }
    let sentinel = tickets.join("ticket-0/sentinel");
    fs::write(&sentinel, "sentinel\n").unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&sentinel));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_context_state(&fixture, &enabled.service, Some(&sentinel), &before);
}

#[test]
fn context_rejects_ticket_directories_exceeding_the_traversal_depth_without_mutation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let sentinel = fixture.root.join(".manyhands/tickets/item/nested/sentinel");
    fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
    fs::write(&sentinel, "sentinel\n").unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&sentinel));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_context_state(&fixture, &enabled.service, Some(&sentinel), &before);
}

#[test]
fn context_rejects_comment_directories_exceeding_the_traversal_entry_limit_without_mutation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let comments = fixture.root.join(".manyhands/comments");
    for entry in 0..1025 {
        fs::create_dir_all(comments.join(format!("item-{entry}"))).unwrap();
    }
    let sentinel = comments.join("item-0/sentinel");
    fs::write(&sentinel, "sentinel\n").unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&sentinel));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_context_state(&fixture, &enabled.service, Some(&sentinel), &before);
}

#[test]
fn context_rejects_comment_directories_exceeding_the_traversal_depth_without_mutation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let sentinel = fixture
        .root
        .join(".manyhands/comments/item/nested/sentinel");
    fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
    fs::write(&sentinel, "sentinel\n").unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&sentinel));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_context_state(&fixture, &enabled.service, Some(&sentinel), &before);
}

#[test]
fn context_edit_rejects_an_absent_or_malformed_primary_target() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let absent = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Edit,
    )));
    assert_eq!(absent.kind, RepositoryErrorKind::MissingAuthoringTarget);

    commit_source(&fixture, "docs/malformed.md", "not managed markdown\n");
    let malformed = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Edit,
    )));
    assert_eq!(malformed.kind, RepositoryErrorKind::MissingAuthoringTarget);
    assert!(
        !fixture
            .root
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .exists()
    );
}

#[test]
fn context_create_rejects_a_primary_item_id_collision() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::OccupiedItemPath);
    assert!(
        fixture
            .repository
            .find_branch(
                "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
                git2::BranchType::Local
            )
            .is_err()
    );
}

#[test]
fn context_create_rejects_a_primary_cross_kind_item_id_collision_without_mutation() {
    let fixture = support::born_repository();
    let item_id = support::document_id();
    let ticket_path = fixture
        .root
        .join(format!(".manyhands/tickets/{item_id}/ticket.md"));
    commit_source(
        &fixture,
        ticket_path
            .strip_prefix(&fixture.root)
            .unwrap()
            .to_str()
            .unwrap(),
        &ticket_source_for(&item_id),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let before = context_state(&fixture, &enabled.service, Some(&ticket_path));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::OccupiedItemPath);
    assert_context_state(&fixture, &enabled.service, Some(&ticket_path), &before);
}

#[test]
fn context_create_rejects_duplicate_primary_canonical_item_ids_without_mutation() {
    let fixture = support::born_repository();
    let item_id = support::document_id();
    commit_source(&fixture, "docs/duplicate.md", &support::document_source());
    let ticket_path = fixture
        .root
        .join(format!(".manyhands/tickets/{item_id}/ticket.md"));
    commit_source(
        &fixture,
        ticket_path
            .strip_prefix(&fixture.root)
            .unwrap()
            .to_str()
            .unwrap(),
        &ticket_source_for(&item_id),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let before = context_state(&fixture, &enabled.service, Some(&ticket_path));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::OccupiedItemPath);
    assert_context_state(&fixture, &enabled.service, Some(&ticket_path), &before);
}

#[test]
fn context_create_branch_retry_creates_only_the_missing_worktree() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let branch = format!("manyhands/document/{item_id}");
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture.repository.branch(&branch, &head, false).unwrap();
    let branch_before = fixture
        .repository
        .find_branch(&branch, git2::BranchType::Local)
        .unwrap()
        .get()
        .target();
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let before = context_state(&fixture, &enabled.service, Some(&configuration_path));

    let outcome = enabled
        .service
        .prepare_context(target(
            &fixture.root,
            AuthoringKind::Document,
            item_id,
            ContextIntent::Create,
        ))
        .unwrap();

    let ContextProvisionOutcome::Created(context) = outcome else {
        panic!("the retry must create the missing worktree");
    };
    assert_eq!(
        fixture
            .repository
            .find_branch(&branch, git2::BranchType::Local)
            .unwrap()
            .get()
            .target(),
        branch_before
    );
    assert_eq!(
        worktree_paths(&fixture.repository),
        vec![std::fs::canonicalize(&context.worktree).unwrap()]
    );
    assert_eq!(support::index_bytes(&fixture.repository), before.index);
    assert_eq!(
        fs::read(configuration_path).unwrap(),
        before.relevant_file.unwrap()
    );
    assert_eq!(status_entries(&fixture.repository), before.statuses);
    assert_eq!(support::head_commit(&fixture.repository), before.head);
    assert_eq!(commit_count(&fixture.repository), before.commit_count);
    assert_eq!(registry_count(&enabled.service), before.registry_count);
}

#[test]
fn recovery_branch_creation_failure_leaves_no_context_and_retry_creates_one() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let before = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let failing = support::FailOnce::at(FailurePoint::BeforeContextBranchCreation)
        .open_service(enabled.data_directory.path());
    let operation_id = support::operation_id();
    let request = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Create,
            operation_id,
        )
    };

    assert_eq!(
        context_error(failing.prepare_context(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let after_failure = context_state(&fixture, &enabled.service, Some(&configuration_path));
    assert_eq!(after_failure.refs, before.refs);
    assert_eq!(after_failure, before);

    let context = context_from(enabled.service.prepare_context(request()).unwrap());
    let after_retry = context_state(&fixture, &enabled.service, Some(&configuration_path));
    assert_initial_linked_worktree(&fixture.repository, &context);
    let expected_refs = refs_with_branch(&before.refs, &context.branch, before.head.unwrap());
    assert_eq!(after_retry.refs, expected_refs);
    assert_eq!(
        after_retry.branches,
        branches_with_branch(&before.branches, &context.branch, before.head.unwrap())
    );
    assert_eq!(
        after_retry.worktrees,
        paths_with_worktree(&before.worktrees, &context.worktree)
    );
    assert_eq!(
        after_retry.registered_worktrees,
        registered_with_context(&before.registered_worktrees, &context)
    );
    assert_eq!(after_retry.index, before.index);
    assert_eq!(after_retry.statuses, before.statuses);
    assert_eq!(after_retry.head, before.head);
    assert_eq!(after_retry.commit_count, before.commit_count);
    assert_eq!(after_retry.relevant_file, before.relevant_file);
    assert_eq!(after_retry.registry_count, before.registry_count);
    assert_eq!(after_retry.registry_rows, before.registry_rows);
}

#[test]
fn recovery_worktree_creation_failure_retains_branch_and_retry_adds_one_worktree() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let branch = format!("manyhands/document/{item_id}");
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let pre_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let failing = support::FailOnce::at(FailurePoint::BeforeWorktreeCreation)
        .open_service(enabled.data_directory.path());
    let operation_id = support::operation_id();
    let request = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Document,
            item_id.clone(),
            ContextIntent::Create,
            operation_id,
        )
    };

    assert_eq!(
        context_error(failing.prepare_context(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let failure_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let branch_before = fixture
        .repository
        .find_branch(&branch, git2::BranchType::Local)
        .unwrap()
        .get()
        .target()
        .unwrap();
    assert_eq!(
        branch_targets(&fixture.repository).len(),
        pre_state.branches.len() + 1
    );
    assert_eq!(
        failure_state.refs,
        refs_with_branch(&pre_state.refs, &branch, pre_state.head.unwrap())
    );
    assert_eq!(
        failure_state.branches,
        branches_with_branch(&pre_state.branches, &branch, pre_state.head.unwrap())
    );
    assert_eq!(failure_state.worktrees, pre_state.worktrees);
    assert_eq!(
        failure_state.registered_worktrees,
        pre_state.registered_worktrees
    );
    assert_context_resources_unchanged(&pre_state, &failure_state);

    let context = context_from(enabled.service.prepare_context(request()).unwrap());
    let retry_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    assert_initial_linked_worktree(&fixture.repository, &context);
    assert_eq!(context.branch, branch);
    assert_eq!(
        retry_state.refs,
        refs_with_branch(&pre_state.refs, &branch, branch_before)
    );
    assert_eq!(
        fixture
            .repository
            .find_branch(&context.branch, git2::BranchType::Local)
            .unwrap()
            .get()
            .target(),
        Some(branch_before)
    );
    assert_eq!(
        branch_targets(&fixture.repository).len(),
        pre_state.branches.len() + 1
    );
    assert_eq!(
        worktree_paths(&fixture.repository),
        vec![std::fs::canonicalize(&context.worktree).unwrap()]
    );
    assert_eq!(
        retry_state.branches,
        branches_with_branch(&pre_state.branches, &branch, branch_before)
    );
    assert_eq!(
        retry_state.worktrees,
        paths_with_worktree(&pre_state.worktrees, &context.worktree)
    );
    assert_eq!(
        retry_state.registered_worktrees,
        registered_with_context(&pre_state.registered_worktrees, &context)
    );
    assert_context_resources_unchanged(&pre_state, &retry_state);
}

#[test]
fn context_create_rejects_an_expected_worktree_with_the_requested_id_at_the_wrong_kind() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                item_id.clone(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let wrong_kind = context
        .worktree
        .join(format!(".manyhands/tickets/{item_id}/ticket.md"));
    fs::create_dir_all(wrong_kind.parent().unwrap()).unwrap();
    fs::write(&wrong_kind, ticket_source_for(&item_id)).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&wrong_kind));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(&fixture, &enabled.service, Some(&wrong_kind), &before);
}

#[test]
fn context_edit_rejects_an_expected_worktree_with_the_requested_id_at_the_wrong_kind() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/requested.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                item_id.clone(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    fs::remove_file(context.worktree.join("docs/requested.md")).unwrap();
    let wrong_kind = context
        .worktree
        .join(format!(".manyhands/tickets/{item_id}/ticket.md"));
    fs::create_dir_all(wrong_kind.parent().unwrap()).unwrap();
    fs::write(&wrong_kind, ticket_source_for(&item_id)).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&wrong_kind));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Edit,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(&fixture, &enabled.service, Some(&wrong_kind), &before);
}

#[test]
fn context_create_rejects_an_expected_worktree_with_duplicate_requested_ids() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                item_id.clone(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let document = context.worktree.join("docs/requested.md");
    fs::create_dir_all(document.parent().unwrap()).unwrap();
    fs::write(&document, support::document_source()).unwrap();
    let ticket = context
        .worktree
        .join(format!(".manyhands/tickets/{item_id}/ticket.md"));
    fs::create_dir_all(ticket.parent().unwrap()).unwrap();
    fs::write(&ticket, ticket_source_for(&item_id)).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&ticket));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(&fixture, &enabled.service, Some(&ticket), &before);
}

#[test]
fn context_edit_rejects_an_expected_worktree_with_duplicate_requested_ids() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/requested.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                item_id.clone(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let ticket = context
        .worktree
        .join(format!(".manyhands/tickets/{item_id}/ticket.md"));
    fs::create_dir_all(ticket.parent().unwrap()).unwrap();
    fs::write(&ticket, ticket_source_for(&item_id)).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&ticket));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Edit,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(&fixture, &enabled.service, Some(&ticket), &before);
}

#[test]
fn context_create_reuses_an_expected_worktree_with_an_unrelated_canonical_item() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                item_id.clone(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let different_item = context.worktree.join("docs/different.md");
    fs::create_dir_all(different_item.parent().unwrap()).unwrap();
    fs::write(&different_item, document_source_for(&support::ticket_id())).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&different_item));

    let outcome = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    ));

    let ContextProvisionOutcome::Reused(reused) = outcome.unwrap() else {
        panic!("an exact context with unrelated items must be reused");
    };
    assert_eq!(reused.worktree, context.worktree);
    assert_context_state(&fixture, &enabled.service, Some(&different_item), &before);
}

#[test]
fn context_edit_reuses_an_expected_worktree_with_unrelated_primary_items() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/requested.md", &support::document_source());
    let ticket_path = fixture.root.join(format!(
        ".manyhands/tickets/{}/ticket.md",
        support::ticket_id()
    ));
    commit_source(
        &fixture,
        ticket_path
            .strip_prefix(&fixture.root)
            .unwrap()
            .to_str()
            .unwrap(),
        &support::ticket_source(),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let first = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let linked_ticket = first.worktree.join(format!(
        ".manyhands/tickets/{}/ticket.md",
        support::ticket_id()
    ));
    let before = context_state(&fixture, &enabled.service, Some(&linked_ticket));

    let outcome = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Edit,
    ));

    let ContextProvisionOutcome::Reused(reused) = outcome.unwrap() else {
        panic!("an exact edit context with unrelated items must be reused");
    };
    assert_eq!(reused.worktree, first.worktree);
    assert_context_state(&fixture, &enabled.service, Some(&linked_ticket), &before);
}

#[test]
fn context_edit_provisions_and_reuses_a_deterministic_ticket_context() {
    let fixture = support::born_repository();
    let item_id = support::ticket_id();
    commit_source(
        &fixture,
        &format!(".manyhands/tickets/{item_id}/ticket.md"),
        &support::ticket_source(),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let first = enabled
        .service
        .prepare_context(target(
            &fixture.root,
            AuthoringKind::Ticket,
            item_id.clone(),
            ContextIntent::Edit,
        ))
        .unwrap();
    let ContextProvisionOutcome::Created(first) = first else {
        panic!("the first ticket edit context must be created");
    };

    let second = enabled
        .service
        .prepare_context(target(
            &fixture.root,
            AuthoringKind::Ticket,
            item_id,
            ContextIntent::Edit,
        ))
        .unwrap();
    let ContextProvisionOutcome::Reused(second) = second else {
        panic!("the exact ticket edit context must be reused");
    };
    assert_eq!(first.branch, "manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW");
    assert_eq!(first.worktree, second.worktree);
}

#[test]
fn context_ignores_primary_worktree_directory_content_during_validation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let ignored = fixture
        .root
        .join(".manyhands/worktrees/other/docs/collision.md");
    fs::create_dir_all(ignored.parent().unwrap()).unwrap();
    fs::write(&ignored, support::document_source()).unwrap();

    let outcome = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    ));

    assert!(matches!(outcome, Ok(ContextProvisionOutcome::Created(_))));
    assert!(ignored.is_file());
}

#[test]
fn context_create_rejects_a_primary_nested_comment_id_collision_without_mutation() {
    let fixture = support::born_repository();
    let item_id = support::document_id();
    let ticket_id = support::ticket_id();
    commit_source(
        &fixture,
        &format!(".manyhands/tickets/{ticket_id}/ticket.md"),
        &support::ticket_source(),
    );
    let comment_path = fixture
        .root
        .join(format!(".manyhands/comments/{ticket_id}/{item_id}.md"));
    commit_source(
        &fixture,
        comment_path
            .strip_prefix(&fixture.root)
            .unwrap()
            .to_str()
            .unwrap(),
        &comment_source_for(&item_id, &ticket_id),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let before = context_state(&fixture, &enabled.service, Some(&comment_path));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::OccupiedItemPath);
    assert_context_state(&fixture, &enabled.service, Some(&comment_path), &before);
}

#[test]
fn context_ignores_non_utf8_canonical_document_candidates() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let candidate = fixture.root.join("docs/non-utf8.md");
    fs::create_dir_all(candidate.parent().unwrap()).unwrap();
    fs::write(&candidate, [0xff, 0x00]).unwrap();
    let before = context_state(&fixture, &enabled.service, Some(&candidate));

    let outcome = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    ));

    assert!(matches!(outcome, Ok(ContextProvisionOutcome::Created(_))));
    assert_eq!(support::index_bytes(&fixture.repository), before.index);
    assert_eq!(fs::read(candidate).unwrap(), before.relevant_file.unwrap());
    assert_eq!(registry_count(&enabled.service), before.registry_count);
}

#[test]
fn context_create_rejects_an_additional_worktree_checked_out_on_its_authoring_branch() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                item_id.clone(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let alternate = fixture.root.join(".manyhands/worktrees/additional");
    let head = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture
        .repository
        .branch("additional", &head, false)
        .unwrap();
    let reference = fixture
        .repository
        .find_reference("refs/heads/additional")
        .unwrap();
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));
    fixture
        .repository
        .worktree("additional", &alternate, Some(&options))
        .unwrap();
    let alternate_repository = Repository::open(&alternate).unwrap();
    fs::write(
        alternate_repository.path().join("HEAD"),
        format!("ref: refs/heads/{}\n", context.branch),
    )
    .unwrap();
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let before = context_state(&fixture, &enabled.service, Some(&configuration_path));

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        item_id,
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_context_state(
        &fixture,
        &enabled.service,
        Some(&configuration_path),
        &before,
    );
}

#[test]
fn context_ignores_unrelated_binary_files_during_canonical_source_collection() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    fs::write(fixture.root.join("unrelated.bin"), [0xff, 0x00]).unwrap();

    let outcome = enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    ));

    assert!(matches!(outcome, Ok(ContextProvisionOutcome::Created(_))));
}

#[test]
fn context_rejects_a_dirty_configuration_path_without_creating_a_context() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    fs::write(
        fixture.root.join(".manyhands/config.toml"),
        "format_version = 1\nprimary_branch = \"main\"\n\n",
    )
    .unwrap();

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Create,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::DirtyConfigurationPath);
    assert!(
        !fixture
            .root
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .exists()
    );
}

#[test]
fn context_edit_rejects_mismatched_contexts_without_selecting_an_alternate_worktree() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let branch = "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let commit = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    fixture.repository.branch(branch, &commit, false).unwrap();
    let alternate = fixture.root.join(".manyhands/worktrees/alternate");
    fs::create_dir_all(alternate.parent().unwrap()).unwrap();
    let reference = fixture
        .repository
        .find_reference(&format!("refs/heads/{branch}"))
        .unwrap();
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));
    fixture
        .repository
        .worktree("alternate", &alternate, Some(&options))
        .unwrap();

    let error = context_error(enabled.service.prepare_context(target(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Edit,
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert!(alternate.is_dir());
    assert!(
        !fixture
            .root
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .exists()
    );
}

#[test]
fn document_create_writes_canonical_markdown_and_a_scoped_checkpoint() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let outcome = enabled
        .service
        .save_document(document_request(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/new.md",
            "New document",
            "Body preserved exactly\n",
        ))
        .unwrap();

    let (context, commit_oid) = saved_checkpoint(outcome);
    let source = fs::read_to_string(context.worktree.join("docs/new.md")).unwrap();
    assert!(matches!(
        manyhands::canonical::parse_item(std::path::Path::new("docs/new.md"), &source),
        Ok(manyhands::canonical::CanonicalItem::Document(document))
            if document.id == support::document_id()
                && document.title == "New document"
                && document.body == "Body preserved exactly\n"
    ));
    assert_eq!(
        support::commit_tree_path(
            &Repository::open(&context.worktree).unwrap(),
            commit_oid,
            "docs/new.md"
        ),
        Some(source.into_bytes())
    );
}

#[test]
fn document_edit_preserves_unknown_metadata_and_unchanged_body() {
    let fixture = support::born_repository();
    let mut source = support::document_source();
    source = source.replace(
        "title: Fixture document",
        "title: Fixture document\nfuture_key: retained",
    );
    source.push_str("Exact body\n");
    commit_source(&fixture, "docs/edit.md", &source);
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let outcome = enabled
        .service
        .save_document(document_request(
            &fixture.root,
            ContextIntent::Edit,
            Some("docs/edit.md"),
            "docs/edit.md",
            "Edited",
            "Exact body\n",
        ))
        .unwrap();

    let (context, commit_oid) = saved_checkpoint(outcome);
    let edited = fs::read_to_string(context.worktree.join("docs/edit.md")).unwrap();
    assert!(edited.contains("future_key: retained"));
    assert!(edited.ends_with("Exact body\n"));
    assert!(
        matches!(manyhands::canonical::parse_item(std::path::Path::new("docs/edit.md"), &edited), Ok(manyhands::canonical::CanonicalItem::Document(document)) if document.id == support::document_id())
    );
    let repository = Repository::open(&context.worktree).unwrap();
    let commit = repository.find_commit(commit_oid).unwrap();
    assert_eq!(
        commit.message(),
        Some("Checkpoint document 01ARZ3NDEKTSV4RRFFQ69G5FAV")
    );
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, "docs/edit.md"),
        Some(edited.into_bytes())
    );
}

#[test]
fn document_move_removes_source_and_checkpoints_only_the_owned_pair() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/old.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let (context, commit_oid) = saved_checkpoint(
        enabled
            .service
            .save_document(document_request(
                &fixture.root,
                ContextIntent::Edit,
                Some("docs/old.md"),
                "docs/new.md",
                "Moved",
                "Moved body\n",
            ))
            .unwrap(),
    );
    let repository = Repository::open(&context.worktree).unwrap();
    assert!(!context.worktree.join("docs/old.md").exists());
    assert!(context.worktree.join("docs/new.md").is_file());
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, "docs/old.md"),
        None
    );
    assert!(support::commit_tree_path(&repository, commit_oid, "docs/new.md").is_some());
}

#[test]
fn document_exact_create_retry_checkpoints_once_then_is_a_no_op() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let operation_id = support::operation_id();
    let (_, first) = saved_checkpoint(
        enabled
            .service
            .save_document(document_request_with_operation_id(
                &fixture.root,
                ContextIntent::Create,
                None,
                "docs/new.md",
                "Retry",
                "Retry body\n",
                operation_id,
            ))
            .unwrap(),
    );
    let outcome = enabled
        .service
        .save_document(document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/new.md",
            "Retry",
            "Retry body\n",
            operation_id,
        ))
        .unwrap();
    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(first)
    );
    assert_eq!(registry_refresh_required(&enabled.service), 1);
}

#[test]
fn document_create_retry_at_a_different_path_preserves_completed_context() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::operation_id();
    let request = |path| {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            None,
            path,
            "Title",
            "Body\n",
            operation_id,
        )
    };
    let (context, commit_oid) = saved_checkpoint(
        enabled
            .service
            .save_document(request("docs/first.md"))
            .unwrap(),
    );
    let first = context.worktree.join("docs/first.md");
    let second = context.worktree.join("docs/second.md");
    let before = rejection_state(&fixture, &enabled.service, &context, &[&first, &second]);

    let error = document_error(enabled.service.save_document(request("docs/second.md")));

    assert!(matches!(
        error.kind,
        RepositoryErrorKind::OperationMismatch
            | RepositoryErrorKind::OccupiedItemPath
            | RepositoryErrorKind::MismatchedAuthoringContext
    ));
    assert_eq!(
        rejection_state(&fixture, &enabled.service, &context, &[&first, &second]),
        before
    );
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
}

#[test]
fn document_rejects_occupied_noncanonical_mismatched_and_missing_paths_without_writing() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/source.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let occupied = context.worktree.join("docs/occupied.md");
    fs::create_dir_all(occupied.parent().unwrap()).unwrap();
    fs::write(&occupied, document_source_for(&support::ticket_id())).unwrap();
    let before = fs::read(&occupied).unwrap();

    for (source, destination) in [
        (Some("docs/source.md"), "notes/no.md"),
        (Some("docs/missing.md"), "docs/new.md"),
        (Some("docs/source.md"), "docs/occupied.md"),
    ] {
        let error = document_error(enabled.service.save_document(document_request(
            &fixture.root,
            ContextIntent::Edit,
            source,
            destination,
            "Title",
            "Body\n",
        )));
        assert!(matches!(
            error.kind,
            RepositoryErrorKind::InvalidPath
                | RepositoryErrorKind::MissingAuthoringTarget
                | RepositoryErrorKind::OccupiedItemPath
                | RepositoryErrorKind::RecoveryRequired
        ));
    }
    assert_eq!(fs::read(occupied).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn document_rejects_symlinked_destination_parent_without_touching_external_file() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let external = tempfile::tempdir().unwrap();
    let external_file = external.path().join("new.md");
    fs::write(&external_file, "external\n").unwrap();
    symlink(external.path(), context.worktree.join("docs")).unwrap();

    let error = document_error(enabled.service.save_document(document_request(
        &fixture.root,
        ContextIntent::Create,
        None,
        "docs/new.md",
        "Title",
        "Body\n",
    )));
    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_eq!(fs::read_to_string(external_file).unwrap(), "external\n");
}

#[test]
fn document_missing_source_does_not_create_its_parent_directories() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/current.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let missing_parent = context.worktree.join("docs/missing/deep");
    let destination_parent = context.worktree.join("docs/destination/deep");

    let error = document_error(enabled.service.save_document(document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some("docs/missing/deep/source.md"),
        "docs/destination/deep/new.md",
        "Edited",
        "Body\n",
    )));

    assert_eq!(error.kind, RepositoryErrorKind::MissingAuthoringTarget);
    assert!(!missing_parent.exists());
    assert!(!destination_parent.exists());
}

#[test]
fn document_move_recovery_accepts_matching_source_and_destination_pair() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/source.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let expected = canonical_document("Moved", "Body\n");
    fs::write(context.worktree.join("docs/source.md"), &expected).unwrap();
    fs::write(context.worktree.join("docs/destination.md"), &expected).unwrap();
    let pre_state = rejection_state(
        &fixture,
        &enabled.service,
        &context,
        &[
            &context.worktree.join("docs/source.md"),
            &context.worktree.join("docs/destination.md"),
        ],
    );

    let (saved, _) = saved_checkpoint(
        enabled
            .service
            .save_document(document_request(
                &fixture.root,
                ContextIntent::Edit,
                Some("docs/source.md"),
                "docs/destination.md",
                "Moved",
                "Body\n",
            ))
            .unwrap(),
    );
    let retry_state = rejection_state(
        &fixture,
        &enabled.service,
        &saved,
        &[
            &saved.worktree.join("docs/source.md"),
            &saved.worktree.join("docs/destination.md"),
        ],
    );
    assert_checkpoint_completion_delta(
        &pre_state,
        &retry_state,
        &saved.branch,
        support::head_commit(&Repository::open(&saved.worktree).unwrap()).unwrap(),
        vec![None, Some(expected.clone().into_bytes())],
        statuses_with_owned_checkpoint_delta(
            &pre_state.statuses,
            &[
                (
                    "docs/source.md",
                    git2::Status::INDEX_NEW | git2::Status::WT_DELETED,
                ),
                (
                    "docs/destination.md",
                    git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
                ),
            ],
        ),
    );

    assert!(!saved.worktree.join("docs/source.md").exists());
    assert_eq!(
        fs::read_to_string(saved.worktree.join("docs/destination.md")).unwrap(),
        expected
    );
}

#[test]
fn document_move_recovery_completes_source_only_and_destination_only_states() {
    for destination_only in [false, true] {
        let fixture = support::born_repository();
        commit_source(&fixture, "docs/source.md", &support::document_source());
        let enabled = support::enabled_repository(&fixture);
        clean_configuration_index(&fixture);
        let context = context_from(
            enabled
                .service
                .prepare_context(target(
                    &fixture.root,
                    AuthoringKind::Document,
                    support::document_id(),
                    ContextIntent::Edit,
                ))
                .unwrap(),
        );
        let expected = canonical_document("Moved", "Body\n");
        if destination_only {
            fs::write(context.worktree.join("docs/destination.md"), &expected).unwrap();
            fs::remove_file(context.worktree.join("docs/source.md")).unwrap();
        } else {
            fs::write(context.worktree.join("docs/source.md"), &expected).unwrap();
        }
        let pre_state = rejection_state(
            &fixture,
            &enabled.service,
            &context,
            &[
                &context.worktree.join("docs/source.md"),
                &context.worktree.join("docs/destination.md"),
            ],
        );

        let (saved, oid) = saved_checkpoint(
            enabled
                .service
                .save_document(document_request(
                    &fixture.root,
                    ContextIntent::Edit,
                    Some("docs/source.md"),
                    "docs/destination.md",
                    "Moved",
                    "Body\n",
                ))
                .unwrap(),
        );
        let retry_state = rejection_state(
            &fixture,
            &enabled.service,
            &saved,
            &[
                &saved.worktree.join("docs/source.md"),
                &saved.worktree.join("docs/destination.md"),
            ],
        );
        assert_checkpoint_completion_delta(
            &pre_state,
            &retry_state,
            &saved.branch,
            oid,
            vec![None, Some(expected.clone().into_bytes())],
            statuses_with_owned_checkpoint_delta(
                &pre_state.statuses,
                &[
                    (
                        "docs/source.md",
                        git2::Status::INDEX_NEW | git2::Status::WT_DELETED,
                    ),
                    (
                        "docs/destination.md",
                        git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
                    ),
                ],
            ),
        );
        assert!(!saved.worktree.join("docs/source.md").exists());
        assert_eq!(
            fs::read_to_string(saved.worktree.join("docs/destination.md")).unwrap(),
            expected
        );
        assert_eq!(
            support::head_commit(&Repository::open(saved.worktree).unwrap()),
            Some(oid)
        );
    }
}

#[test]
fn document_completed_move_retry_is_a_noop_without_its_old_source() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/source.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Edit,
            Some("docs/source.md"),
            "docs/destination.md",
            "Moved",
            "Moved body\n",
            operation_id,
        )
    };

    let (context, commit_oid) = saved_checkpoint(enabled.service.save_document(request()).unwrap());
    assert!(!context.worktree.join("docs/source.md").exists());
    let mut retry = request();
    retry.expected_source = Some(manyhands::repository::ExpectedPathObservation::Missing);
    let outcome = enabled.service.save_document(retry).unwrap();

    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
}

#[test]
fn document_completed_move_retry_retries_only_registry_invalidation() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/source.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::new_operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Edit,
            Some("docs/source.md"),
            "docs/destination.md",
            "Moved",
            "Moved body\n",
            operation_id,
        )
    };
    let (context, commit_oid) = saved_checkpoint(enabled.service.save_document(request()).unwrap());
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute("UPDATE repositories SET refresh_required = 0", [])
                .unwrap()
        })
        .unwrap();
    let failing = support::FailOnce::at(FailurePoint::BeforeRegistryWrite)
        .open_service(enabled.data_directory.path());

    let mut retry = request();
    retry.expected_source = Some(manyhands::repository::ExpectedPathObservation::Missing);
    let outcome = failing.save_document(retry).unwrap();
    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::Checkpointed { commit_oid: pending },
            ..
        } if pending == commit_oid
    ));
    assert_eq!(registry_refresh_required(&enabled.service), 0);
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );

    let mut retry = request();
    retry.expected_source = Some(manyhands::repository::ExpectedPathObservation::Missing);
    let outcome = enabled.service.save_document(retry).unwrap();
    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(registry_refresh_required(&enabled.service), 1);
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
}

#[test]
fn document_destination_only_move_rejects_unrelated_tracked_source_path() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/selected.md", &support::document_source());
    commit_source(
        &fixture,
        "docs/unrelated.md",
        &document_source_for(&support::ticket_id()),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let destination = context.worktree.join("docs/destination.md");
    fs::write(&destination, canonical_document("Moved", "Body\n")).unwrap();
    fs::remove_file(context.worktree.join("docs/selected.md")).unwrap();
    fs::remove_file(context.worktree.join("docs/unrelated.md")).unwrap();
    let before = rejection_state(&fixture, &enabled.service, &context, &[&destination]);

    assert!(
        enabled
            .service
            .save_document(document_request(
                &fixture.root,
                ContextIntent::Edit,
                Some("docs/unrelated.md"),
                "docs/destination.md",
                "Moved",
                "Body\n",
            ))
            .is_err()
    );
    assert_eq!(
        rejection_state(&fixture, &enabled.service, &context, &[&destination]),
        before
    );
}

#[test]
fn recovery_document_before_item_write_preserves_absent_destination_for_retry() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let path = context.worktree.join("docs/new.md");
    let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    let failing = support::FailOnce::at(FailurePoint::BeforeItemWrite)
        .open_service(enabled.data_directory.path());
    let operation_id = support::new_operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/new.md",
            "Title",
            "Body\n",
            operation_id,
        )
    };

    let error = document_error(failing.save_document(request()));
    assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    let failure_state = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_eq!(failure_state, before);
    let (_, oid) = saved_checkpoint(enabled.service.save_document(request()).unwrap());
    let after_retry = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_checkpoint_completion_delta(
        &before,
        &after_retry,
        &context.branch,
        oid,
        vec![Some(canonical_document("Title", "Body\n").into_bytes())],
        statuses_with_owned_checkpoint_delta(
            &before.statuses,
            &[(
                "docs/new.md",
                git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
            )],
        ),
    );
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(oid)
    );
    assert_eq!(after_retry.commit_count, before.commit_count + 1);
    assert_eq!(after_retry.worktrees, before.worktrees);
    assert_eq!(
        after_retry.registered_worktrees,
        before.registered_worktrees
    );
    assert_eq!(after_retry.registry_rows, before.registry_rows);
}

#[test]
fn recovery_document_before_item_write_preserves_existing_owned_bytes_for_retry() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/edit.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let failing = support::FailOnce::at(FailurePoint::BeforeItemWrite)
        .open_service(enabled.data_directory.path());
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let path = context.worktree.join("docs/edit.md");
    let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    let operation_id = support::new_operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Edit,
            Some("docs/edit.md"),
            "docs/edit.md",
            "Changed",
            "Body\n",
            operation_id,
        )
    };

    assert_eq!(
        document_error(failing.save_document(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let failure_state = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_eq!(failure_state, before);
    let (_, oid) = saved_checkpoint(enabled.service.save_document(request()).unwrap());
    let after_retry = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_checkpoint_completion_delta(
        &before,
        &after_retry,
        &context.branch,
        oid,
        vec![Some(canonical_document("Changed", "Body\n").into_bytes())],
        statuses_with_owned_checkpoint_delta(
            &before.statuses,
            &[(
                "docs/edit.md",
                git2::Status::INDEX_MODIFIED | git2::Status::WT_MODIFIED,
            )],
        ),
    );
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(oid)
    );
    assert_eq!(after_retry.commit_count, before.commit_count + 1);
    assert_eq!(after_retry.worktrees, before.worktrees);
    assert_eq!(
        after_retry.registered_worktrees,
        before.registered_worktrees
    );
    assert_eq!(after_retry.registry_rows, before.registry_rows);
}

#[test]
fn recovery_document_move_before_item_write_preserves_both_paths_then_checkpoints_once() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/source.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let source = context.worktree.join("docs/source.md");
    let destination = context.worktree.join("docs/destination.md");
    let expected = canonical_document("Moved", "Body\n");
    fs::write(&destination, &expected).unwrap();
    fs::write(context.worktree.join("unrelated.txt"), "unrelated\n").unwrap();
    let before = rejection_state(
        &fixture,
        &enabled.service,
        &context,
        &[&source, &destination],
    );
    let unrelated_before = statuses_excluding(
        &Repository::open(&context.worktree).unwrap(),
        &["docs/source.md", "docs/destination.md"],
    );
    let failing = support::FailOnce::at(FailurePoint::BeforeItemWrite)
        .open_service(enabled.data_directory.path());
    let operation_id = support::new_operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Edit,
            Some("docs/source.md"),
            "docs/destination.md",
            "Moved",
            "Body\n",
            operation_id,
        )
    };

    assert_eq!(
        document_error(failing.save_document(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let failure_state = rejection_state(
        &fixture,
        &enabled.service,
        &context,
        &[&source, &destination],
    );
    assert_eq!(failure_state, before);

    let (_, commit_oid) = saved_checkpoint(enabled.service.save_document(request()).unwrap());
    let repository = Repository::open(&context.worktree).unwrap();
    let retry_state = rejection_state(
        &fixture,
        &enabled.service,
        &context,
        &[&source, &destination],
    );
    assert_checkpoint_completion_delta(
        &before,
        &retry_state,
        &context.branch,
        commit_oid,
        vec![None, Some(expected.clone().into_bytes())],
        statuses_with_owned_checkpoint_delta(
            &before.statuses,
            &[
                (
                    "docs/source.md",
                    git2::Status::INDEX_NEW | git2::Status::WT_DELETED,
                ),
                (
                    "docs/destination.md",
                    git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
                ),
            ],
        ),
    );
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(&destination).unwrap(), expected);
    assert_eq!(support::head_commit(&repository), Some(commit_oid));
    assert_eq!(commit_count(&repository), before.commit_count + 1);
    assert_eq!(
        branch_targets(&fixture.repository).len(),
        before.branches.len()
    );
    assert_eq!(
        branch_targets(&fixture.repository)
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>(),
        before
            .branches
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(worktree_paths(&fixture.repository), before.worktrees);
    assert!(context.worktree.join("unrelated.txt").is_file());
    assert_eq!(registry_count(&enabled.service), before.registry_count);
    assert_eq!(support::index_bytes(&repository), before.index);
    assert_eq!(
        statuses_excluding(&repository, &["docs/source.md", "docs/destination.md"]),
        unrelated_before
    );
    assert_eq!(
        registered_worktrees(&fixture.repository),
        before.registered_worktrees
    );
    assert_eq!(registry_rows(&enabled.service), before.registry_rows);
}

#[test]
fn fresh_service_replay_of_interrupted_move_preserves_an_externally_changed_source() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/source.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let source = context.worktree.join("docs/source.md");
    let destination = context.worktree.join("docs/destination.md");
    let source_expected = ExpectedPathObservation::from_bytes(&fs::read(&source).unwrap());
    let operation_id = support::new_operation_id();
    let request = || {
        let mut request = document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Edit,
            Some("docs/source.md"),
            "docs/destination.md",
            "Moved",
            "Body\n",
            operation_id,
        );
        request.expected_source = Some(source_expected.clone());
        request.expected_destination = ExpectedPathObservation::Missing;
        request
    };
    let failing = support::FailOnce::at(FailurePoint::BeforeItemWrite)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        document_error(failing.save_document(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let external = canonical_document("External source", "External body\n");
    fs::write(&source, &external).unwrap();
    let before_replay = support::repository_and_worktree_snapshot(&fixture);

    let fresh = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let error = document_error(fresh.save_document(request()));

    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    assert_eq!(fs::read_to_string(source).unwrap(), external);
    assert!(!destination.exists());
    assert_eq!(
        support::repository_and_worktree_snapshot(&fixture),
        before_replay
    );
}

#[test]
fn document_noop_registry_failure_returns_pending_then_invalidates_without_commit() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::new_operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/new.md",
            "Title",
            "Body\n",
            operation_id,
        )
    };
    let (context, oid) = saved_checkpoint(enabled.service.save_document(request()).unwrap());
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute("UPDATE repositories SET refresh_required = 0", [])
                .unwrap()
        })
        .unwrap();
    let failing = support::FailOnce::at(FailurePoint::BeforeRegistryWrite)
        .open_service(enabled.data_directory.path());

    let outcome = failing.save_document(request()).unwrap();
    assert!(
        matches!(outcome, SaveOutcome::Saved { checkpoint: LocalCheckpoint::Checkpointed { commit_oid } , .. } if commit_oid == oid)
    );
    assert_eq!(registry_refresh_required(&enabled.service), 0);
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(oid)
    );
    let outcome = enabled.service.save_document(request()).unwrap();
    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(registry_refresh_required(&enabled.service), 1);
    assert_eq!(
        support::head_commit(&Repository::open(context.worktree).unwrap()),
        Some(oid)
    );
}

#[test]
fn document_missing_registration_rejects_a_new_blake3_creation_claim() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::new_operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/new.md",
            "Title",
            "Body\n",
            operation_id,
        )
    };
    let (context, oid) = saved_checkpoint(enabled.service.save_document(request()).unwrap());
    enabled
        .service
        .remove_registration(manyhands::repository::RemoveRegistrationRequest {
            root: fixture.root.clone(),
            operation_id: support::operation_id(),
        })
        .unwrap();

    let error = document_error(enabled.service.save_document(request()));
    assert_eq!(error.kind, RepositoryErrorKind::OperationMismatch);
    assert_eq!(
        support::head_commit(&Repository::open(context.worktree).unwrap()),
        Some(oid)
    );
}

#[test]
fn recovery_document_before_checkpoint_commit_preserves_written_file_for_exact_retry() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let path = context.worktree.join("docs/new.md");
    let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    let unrelated_before = unrelated_status_entries(&Repository::open(&context.worktree).unwrap());
    let failing = support::FailOnce::at(FailurePoint::BeforeCheckpointCommit)
        .open_service(enabled.data_directory.path());
    let operation_id = support::operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/new.md",
            "Title",
            "Body\n",
            operation_id,
        )
    };

    let error = document_error(failing.save_document(request()));
    assert_eq!(error.kind, RepositoryErrorKind::InjectedFailure);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        canonical_document("Title", "Body\n")
    );
    let after_failure = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_checkpoint_failure_preserves_resources(
        &before,
        &after_failure,
        vec![Some(canonical_document("Title", "Body\n").into_bytes())],
        vec![(Some("docs/new.md".to_owned()), git2::Status::WT_NEW)],
    );
    assert_eq!(
        unrelated_status_entries(&Repository::open(&context.worktree).unwrap()),
        unrelated_before
    );
    let (_, oid) = saved_checkpoint(enabled.service.save_document(request()).unwrap());
    let after_retry = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_checkpoint_completion_delta(
        &before,
        &after_retry,
        &context.branch,
        oid,
        vec![Some(canonical_document("Title", "Body\n").into_bytes())],
        statuses_with_owned_checkpoint_delta(
            &before.statuses,
            &[(
                "docs/new.md",
                git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
            )],
        ),
    );
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(oid)
    );
    assert_eq!(after_retry.commit_count, before.commit_count + 1);
    assert_eq!(after_retry.worktrees, before.worktrees);
    assert_eq!(
        after_retry.registered_worktrees,
        before.registered_worktrees
    );
    assert_eq!(after_retry.registry_rows, before.registry_rows);
    assert_eq!(
        unrelated_status_entries(&Repository::open(&context.worktree).unwrap()),
        unrelated_before
    );
}

#[test]
fn recovery_document_registry_failure_returns_refresh_pending_and_retry_does_not_commit_again() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let failing = support::FailOnce::at(FailurePoint::BeforeRegistryWrite)
        .open_service(enabled.data_directory.path());
    let operation_id = support::operation_id();
    let request = || {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/new.md",
            "Title",
            "Body\n",
            operation_id,
        )
    };
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute("UPDATE repositories SET refresh_required = 0", [])
                .unwrap()
        })
        .unwrap();
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let pre_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let checkpoint_context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let checkpoint_path = checkpoint_context.worktree.join("docs/new.md");
    let checkpoint_pre = linked_worktree_state(&checkpoint_context, &[&checkpoint_path]);

    let SaveOutcome::Saved {
        context,
        checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
    } = failing.save_document(request()).unwrap()
    else {
        panic!("checkpoint must be pending only after its commit");
    };
    let path = context.worktree.join("docs/new.md");
    assert_eq!(context.worktree, checkpoint_context.worktree);
    let failure_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let checkpoint_failure = linked_worktree_state(&context, &[&path]);
    assert_linked_checkpoint_delta(
        &checkpoint_pre,
        &checkpoint_failure,
        commit_oid,
        vec![Some(canonical_document("Title", "Body\n").into_bytes())],
        vec![(
            Some("docs/new.md".to_owned()),
            git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
        )],
    );
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        canonical_document("Title", "Body\n")
    );
    assert_checkpoint_failure_creates_only_owned_context(
        &pre_state,
        &failure_state,
        &context,
        commit_oid,
    );
    assert_eq!(registry_refresh_required(&enabled.service), 0);
    let outcome = enabled.service.save_document(request()).unwrap();
    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
    let retry_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let checkpoint_retry = linked_worktree_state(&context, &[&path]);
    assert_eq!(checkpoint_retry, checkpoint_failure);
    assert_retry_only_updates_registry_state(&failure_state, &retry_state);
    assert_registry_refresh_is_the_only_row_change(
        &failure_state.registry_rows,
        &retry_state.registry_rows,
    );
}

#[test]
fn document_checkpoint_preserves_unrelated_linked_worktree_git_state() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let repository = Repository::open(&context.worktree).unwrap();
    support::conflict_worktree(&repository, &context.worktree);
    assert!(repository.index().unwrap().has_conflicts());
    fs::write(context.worktree.join("fixture.txt"), "modified\n").unwrap();
    fs::write(context.worktree.join("staged.txt"), "staged\n").unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(std::path::Path::new("staged.txt")).unwrap();
    index.write().unwrap();
    fs::write(context.worktree.join("untracked.txt"), "untracked\n").unwrap();
    fs::remove_file(context.worktree.join(".manyhands/config.toml")).unwrap();
    let before_index = support::index_bytes(&repository).unwrap();
    let before_status = unrelated_status_entries(&repository);
    assert!(
        before_status
            .iter()
            .any(|(_, status)| status.contains(git2::Status::CONFLICTED))
    );

    let (_, commit_oid) = saved_checkpoint(
        enabled
            .service
            .save_document(document_request(
                &fixture.root,
                ContextIntent::Create,
                None,
                "docs/new.md",
                "Title",
                "Body\n",
            ))
            .unwrap(),
    );

    let repository = Repository::open(&context.worktree).unwrap();
    assert_eq!(support::index_bytes(&repository).unwrap(), before_index);
    assert_eq!(unrelated_status_entries(&repository), before_status);
    assert!(repository.index().unwrap().has_conflicts());
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, "docs/new.md"),
        Some(canonical_document("Title", "Body\n").into_bytes())
    );
    let commit = repository.find_commit(commit_oid).unwrap();
    let parent = commit.parent(0).unwrap();
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, "fixture.txt"),
        support::commit_tree_path(&repository, parent.id(), "fixture.txt")
    );
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, ".manyhands/config.toml"),
        support::commit_tree_path(&repository, parent.id(), ".manyhands/config.toml")
    );
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, "staged.txt"),
        None
    );
}

#[test]
fn document_rejections_preserve_all_observed_local_state() {
    for case in [
        "noncanonical-source",
        "noncanonical-destination",
        "source-id-mismatch",
        "destination-id-mismatch",
        "unsafe-source",
        "unsafe-destination",
        "missing-source",
    ] {
        let fixture = support::born_repository();
        commit_source(&fixture, "docs/source.md", &support::document_source());
        let enabled = support::enabled_repository(&fixture);
        clean_configuration_index(&fixture);
        let context = context_from(
            enabled
                .service
                .prepare_context(target(
                    &fixture.root,
                    AuthoringKind::Document,
                    support::document_id(),
                    ContextIntent::Edit,
                ))
                .unwrap(),
        );
        let source = context.worktree.join("docs/source.md");
        let destination = context.worktree.join("docs/destination.md");
        let (source_path, destination_path) = match case {
            "noncanonical-source" => {
                let noncanonical = context.worktree.join("notes/source.md");
                fs::create_dir_all(noncanonical.parent().unwrap()).unwrap();
                fs::write(&noncanonical, support::document_source()).unwrap();
                ("notes/source.md", "docs/destination.md")
            }
            "noncanonical-destination" => ("docs/source.md", "notes/destination.md"),
            "source-id-mismatch" => {
                fs::write(&source, document_source_for(&support::ticket_id())).unwrap();
                ("docs/source.md", "docs/destination.md")
            }
            "destination-id-mismatch" => {
                fs::write(&destination, document_source_for(&support::ticket_id())).unwrap();
                ("docs/source.md", "docs/destination.md")
            }
            "unsafe-source" => {
                fs::remove_file(&source).unwrap();
                fs::create_dir(&source).unwrap();
                ("docs/source.md", "docs/destination.md")
            }
            "unsafe-destination" => {
                fs::create_dir(&destination).unwrap();
                ("docs/source.md", "docs/destination.md")
            }
            "missing-source" => ("docs/missing.md", "docs/destination.md"),
            _ => unreachable!(),
        };
        let before = rejection_state(
            &fixture,
            &enabled.service,
            &context,
            &[&source, &destination],
        );
        assert!(
            enabled
                .service
                .save_document(document_request(
                    &fixture.root,
                    ContextIntent::Edit,
                    Some(source_path),
                    destination_path,
                    "Title",
                    "Body\n",
                ))
                .is_err(),
            "{case} must be rejected"
        );
        assert_eq!(
            rejection_state(
                &fixture,
                &enabled.service,
                &context,
                &[&source, &destination]
            ),
            before,
            "{case} must not mutate state"
        );
    }
}

#[test]
fn document_identity_required_preserves_context_and_creates_no_destination_parent() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let mut local = Repository::open(&context.worktree)
        .unwrap()
        .config()
        .unwrap();
    local.remove("user.name").unwrap();
    local.remove("user.email").unwrap();
    drop(local);
    let destination = context.worktree.join("docs/nested/new.md");
    let before = rejection_state(&fixture, &enabled.service, &context, &[&destination]);
    let effective = Config::new().unwrap();

    let outcome = enabled
        .service
        .save_document_with_identity_config_for_testing(
            document_request(
                &fixture.root,
                ContextIntent::Create,
                None,
                "docs/nested/new.md",
                "Title",
                "Body\n",
            ),
            &effective,
        )
        .unwrap();

    assert!(matches!(outcome, SaveOutcome::IdentityRequired { .. }));
    assert!(!destination.parent().unwrap().exists());
    assert_eq!(
        rejection_state(&fixture, &enabled.service, &context, &[&destination]),
        before
    );
}

#[cfg(unix)]
#[test]
fn document_symlink_parent_rejections_preserve_all_observed_local_state() {
    use std::os::unix::fs::symlink;

    for source_parent in [true, false] {
        let fixture = support::born_repository();
        commit_source(&fixture, "docs/source.md", &support::document_source());
        let enabled = support::enabled_repository(&fixture);
        clean_configuration_index(&fixture);
        let context = context_from(
            enabled
                .service
                .prepare_context(target(
                    &fixture.root,
                    AuthoringKind::Document,
                    support::document_id(),
                    ContextIntent::Edit,
                ))
                .unwrap(),
        );
        let external = tempfile::tempdir().unwrap();
        if source_parent {
            fs::remove_file(context.worktree.join("docs/source.md")).unwrap();
            fs::remove_dir(context.worktree.join("docs")).unwrap();
            symlink(external.path(), context.worktree.join("docs")).unwrap();
        } else {
            symlink(external.path(), context.worktree.join("docs/destination")).unwrap();
        }
        let source = context.worktree.join("docs/source.md");
        let destination = if source_parent {
            context.worktree.join("docs/destination.md")
        } else {
            context.worktree.join("docs/destination/item.md")
        };
        let before = rejection_state(
            &fixture,
            &enabled.service,
            &context,
            &[&source, &destination],
        );
        assert!(
            enabled
                .service
                .save_document(document_request(
                    &fixture.root,
                    ContextIntent::Edit,
                    Some("docs/source.md"),
                    if source_parent {
                        "docs/destination.md"
                    } else {
                        "docs/destination/item.md"
                    },
                    "Title",
                    "Body\n",
                ))
                .is_err()
        );
        assert_eq!(
            rejection_state(
                &fixture,
                &enabled.service,
                &context,
                &[&source, &destination]
            ),
            before
        );
    }
}

#[test]
fn ticket_create_writes_exact_canonical_body_and_scoped_checkpoint() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let (context, commit_oid) = saved_checkpoint(
        enabled
            .service
            .save_ticket(ticket_request(
                &fixture.root,
                ContextIntent::Create,
                "New ticket",
                "Body exactly\n",
            ))
            .unwrap(),
    );
    let path = ticket_path(&context);
    let source = fs::read_to_string(&path).unwrap();
    assert!(matches!(
        manyhands::canonical::parse_item(&ticket_relative_path(), &source),
        Ok(manyhands::canonical::CanonicalItem::Ticket(ticket))
            if ticket.id == support::ticket_id()
                && ticket.title == "New ticket"
                && ticket.ticket_type == "feature"
                && ticket.status == "open"
                && ticket.project.as_deref() == Some("manyhands")
                && ticket.team.as_deref() == Some("core")
                && ticket.closed_at.is_none()
                && ticket.closed_by.is_none()
                && ticket.unknown.is_empty()
                && ticket.body == "Body exactly\n"
    ));
    let repository = Repository::open(&context.worktree).unwrap();
    assert_eq!(
        repository.find_commit(commit_oid).unwrap().message(),
        Some("Checkpoint ticket 01ARZ3NDEKTSV4RRFFQ69G5FAW")
    );
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, ticket_relative_path()),
        Some(source.into_bytes())
    );
}

#[test]
fn ticket_edit_preserves_unknown_closure_and_exact_body() {
    let fixture = support::born_repository();
    let mut source = support::ticket_source();
    source = source.replace(
        "status: open\n---",
        "status: open\nfuture_key: retained\nclosed_at: 2026-10-02T12:34:56Z\nclosed_by: Manyhands Test <manyhands-test@example.invalid>\n---",
    );
    source.push_str("Exact body\n");
    commit_source(
        &fixture,
        &ticket_relative_path().display().to_string(),
        &source,
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let (context, _) = saved_checkpoint(
        enabled
            .service
            .save_ticket(ticket_request(
                &fixture.root,
                ContextIntent::Edit,
                "Edited",
                "Exact body\n",
            ))
            .unwrap(),
    );
    let edited = fs::read_to_string(ticket_path(&context)).unwrap();
    assert!(edited.contains("future_key: retained"));
    assert!(edited.contains("closed_at: 2026-10-02T12:34:56Z"));
    assert!(edited.contains("closed_by: Manyhands Test <manyhands-test@example.invalid>"));
    assert!(edited.ends_with("Exact body\n"));
}

#[test]
fn ticket_exact_create_retry_checkpoints_pending_work_then_noops() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let operation_id = support::new_operation_id();
    let request = || {
        ticket_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            "Retry",
            "Retry body\n",
            operation_id,
        )
    };

    let (_, commit_oid) = saved_checkpoint(enabled.service.save_ticket(request()).unwrap());
    let outcome = enabled.service.save_ticket(request()).unwrap();
    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
}

#[test]
fn ticket_missing_registration_rejects_a_new_blake3_creation_claim() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::new_operation_id();
    let request = || {
        ticket_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            "Title",
            "Body\n",
            operation_id,
        )
    };
    let (context, commit_oid) = saved_checkpoint(enabled.service.save_ticket(request()).unwrap());
    enabled
        .service
        .remove_registration(manyhands::repository::RemoveRegistrationRequest {
            root: fixture.root.clone(),
            operation_id: support::operation_id(),
        })
        .unwrap();

    let error = document_error(enabled.service.save_ticket(request()));
    assert_eq!(error.kind, RepositoryErrorKind::OperationMismatch);
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
}

#[test]
fn recovery_ticket_registry_failure_returns_refresh_pending_then_invalidates_without_another_commit()
 {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let failing = support::FailOnce::at(FailurePoint::BeforeRegistryWrite)
        .open_service(enabled.data_directory.path());
    let operation_id = support::new_operation_id();
    let request = || {
        ticket_request_with_operation_id(
            &fixture.root,
            ContextIntent::Create,
            "Title",
            "Body\n",
            operation_id,
        )
    };
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute("UPDATE repositories SET refresh_required = 0", [])
                .unwrap()
        })
        .unwrap();
    let configuration_path = fixture.root.join(".manyhands/config.toml");
    let pre_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let checkpoint_context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let checkpoint_path = ticket_path(&checkpoint_context);
    let checkpoint_pre = linked_worktree_state(&checkpoint_context, &[&checkpoint_path]);

    let SaveOutcome::Saved {
        context,
        checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
    } = failing.save_ticket(request()).unwrap()
    else {
        panic!("ticket checkpoint must be pending only after its commit");
    };
    let path = ticket_path(&context);
    assert_eq!(context.worktree, checkpoint_context.worktree);
    let failure_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let checkpoint_failure = linked_worktree_state(&context, &[&path]);
    assert_linked_checkpoint_delta(
        &checkpoint_pre,
        &checkpoint_failure,
        commit_oid,
        vec![Some(canonical_ticket("Title", "Body\n").into_bytes())],
        vec![(
            Some(ticket_relative_path().display().to_string()),
            git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
        )],
    );
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        canonical_ticket("Title", "Body\n")
    );
    assert_checkpoint_failure_creates_only_owned_context(
        &pre_state,
        &failure_state,
        &context,
        commit_oid,
    );
    assert_eq!(registry_refresh_required(&enabled.service), 0);

    let outcome = enabled.service.save_ticket(request()).unwrap();
    assert!(matches!(
        outcome,
        SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(registry_refresh_required(&enabled.service), 1);
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
    let retry_state = context_state(&fixture, &enabled.service, Some(&configuration_path));
    let checkpoint_retry = linked_worktree_state(&context, &[&path]);
    assert_eq!(checkpoint_retry, checkpoint_failure);
    assert_retry_only_updates_registry_state(&failure_state, &retry_state);
    assert_registry_refresh_is_the_only_row_change(
        &failure_state.registry_rows,
        &retry_state.registry_rows,
    );
}

#[test]
fn ticket_identity_required_before_creating_its_parent() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let mut config = Repository::open(&context.worktree)
        .unwrap()
        .config()
        .unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    drop(config);
    let path = ticket_path(&context);
    assert!(!path.parent().unwrap().exists());

    let effective = Config::new().unwrap();
    let outcome = enabled
        .service
        .save_ticket_with_identity_config_for_testing(
            ticket_request(&fixture.root, ContextIntent::Create, "Title", "Body\n"),
            &effective,
        )
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::IdentityRequired { .. }));
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn recovery_ticket_write_and_checkpoint_failures_preserve_retryable_state() {
    for point in [
        FailurePoint::BeforeItemWrite,
        FailurePoint::BeforeCheckpointCommit,
    ] {
        let fixture = support::born_repository();
        let enabled = support::enabled_repository(&fixture);
        clean_configuration_index(&fixture);
        let context = context_from(
            enabled
                .service
                .prepare_context(target(
                    &fixture.root,
                    AuthoringKind::Ticket,
                    support::ticket_id(),
                    ContextIntent::Create,
                ))
                .unwrap(),
        );
        let path = ticket_path(&context);
        let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);
        let unrelated_before = statuses_excluding(
            &Repository::open(&context.worktree).unwrap(),
            &[ticket_relative_path().to_str().unwrap()],
        );
        let failing = support::FailOnce::at(point).open_service(enabled.data_directory.path());
        let operation_id = support::new_operation_id();
        let request = || {
            ticket_request_with_operation_id(
                &fixture.root,
                ContextIntent::Create,
                "Title",
                "Body\n",
                operation_id,
            )
        };

        assert_eq!(
            ticket_error(failing.save_ticket(request())).kind,
            RepositoryErrorKind::InjectedFailure
        );
        let failure_state = rejection_state(&fixture, &enabled.service, &context, &[&path]);
        if point == FailurePoint::BeforeItemWrite {
            assert_eq!(failure_state, before);
        } else {
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                canonical_ticket("Title", "Body\n")
            );
            assert_checkpoint_failure_preserves_resources(
                &before,
                &failure_state,
                vec![Some(canonical_ticket("Title", "Body\n").into_bytes())],
                vec![(
                    (ticket_relative_path().to_str().map(str::to_owned)),
                    git2::Status::WT_NEW,
                )],
            );
        }
        let (_, commit_oid) = saved_checkpoint(enabled.service.save_ticket(request()).unwrap());
        let repository = Repository::open(&context.worktree).unwrap();
        let after_retry = rejection_state(&fixture, &enabled.service, &context, &[&path]);
        assert_checkpoint_completion_delta(
            &before,
            &after_retry,
            &context.branch,
            commit_oid,
            vec![Some(canonical_ticket("Title", "Body\n").into_bytes())],
            statuses_with_owned_checkpoint_delta(
                &before.statuses,
                &[(
                    &ticket_relative_path().display().to_string(),
                    git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
                )],
            ),
        );
        assert_eq!(support::head_commit(&repository), Some(commit_oid));
        assert_eq!(commit_count(&repository), before.commit_count + 1);
        assert_eq!(
            branch_targets(&fixture.repository).len(),
            before.branches.len()
        );
        assert_eq!(worktree_paths(&fixture.repository), before.worktrees);
        assert_eq!(registry_count(&enabled.service), before.registry_count);
        assert_eq!(
            after_retry.registered_worktrees,
            before.registered_worktrees
        );
        assert_eq!(after_retry.registry_rows, before.registry_rows);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            canonical_ticket("Title", "Body\n")
        );
        assert_eq!(after_retry.index, before.index);
        assert_eq!(
            statuses_excluding(&repository, &[ticket_relative_path().to_str().unwrap()]),
            unrelated_before
        );
    }
}

#[test]
fn ticket_rejects_mismatched_kind_id_unsafe_file_and_symlink_parent_without_mutation() {
    for (case, expected) in [
        (
            "wrong-kind",
            RepositoryErrorKind::MismatchedAuthoringContext,
        ),
        ("wrong-id", RepositoryErrorKind::MismatchedAuthoringContext),
        ("unsafe-file", RepositoryErrorKind::Io),
    ] {
        let fixture = support::born_repository();
        commit_source(
            &fixture,
            &ticket_relative_path().display().to_string(),
            &support::ticket_source(),
        );
        let enabled = support::enabled_repository(&fixture);
        clean_configuration_index(&fixture);
        let context = context_from(
            enabled
                .service
                .prepare_context(target(
                    &fixture.root,
                    AuthoringKind::Ticket,
                    support::ticket_id(),
                    ContextIntent::Edit,
                ))
                .unwrap(),
        );
        let path = ticket_path(&context);
        match case {
            "wrong-kind" => fs::write(&path, document_source_for(&support::ticket_id())).unwrap(),
            "wrong-id" => fs::write(&path, ticket_source_for(&support::document_id())).unwrap(),
            "unsafe-file" => {
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
            }
            _ => unreachable!(),
        }
        let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);

        let error = ticket_error(enabled.service.save_ticket(ticket_request(
            &fixture.root,
            ContextIntent::Edit,
            "Title",
            "Body\n",
        )));

        assert_eq!(
            error.kind, expected,
            "{case} must report its typed rejection"
        );
        assert_eq!(
            rejection_state(&fixture, &enabled.service, &context, &[&path]),
            before,
            "{case} must preserve refs, worktrees, files, index, status, commits, and registry"
        );
    }
}

#[cfg(unix)]
#[test]
fn ticket_rejects_a_symlinked_canonical_parent() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let external = tempfile::tempdir().unwrap();
    let parent = context.worktree.join(".manyhands/tickets");
    fs::create_dir_all(parent.parent().unwrap()).unwrap();
    symlink(external.path(), &parent).unwrap();
    let before = rejection_state(&fixture, &enabled.service, &context, &[&parent]);

    let error = ticket_error(enabled.service.save_ticket(ticket_request(
        &fixture.root,
        ContextIntent::Create,
        "Title",
        "Body\n",
    )));
    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert!(
        !external
            .path()
            .join(support::ticket_id().to_string())
            .exists()
    );
    assert_eq!(
        rejection_state(&fixture, &enabled.service, &context, &[&parent]),
        before
    );
}

#[test]
fn ticket_rejects_the_same_id_at_a_different_ticket_path_without_mutation() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let other = context.worktree.join(".manyhands/tickets/other/ticket.md");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    fs::write(&other, canonical_ticket("Other", "Other body\n")).unwrap();
    let before = fs::read(&other).unwrap();

    assert!(
        enabled
            .service
            .save_ticket(ticket_request(
                &fixture.root,
                ContextIntent::Create,
                "Title",
                "Body\n",
            ))
            .is_err()
    );
    assert_eq!(fs::read(&other).unwrap(), before);
    assert!(!ticket_path(&context).exists());
}

#[test]
fn ticket_checkpoint_preserves_unrelated_live_index_and_worktree_state() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let repository = Repository::open(&context.worktree).unwrap();
    support::conflict_worktree(&repository, &context.worktree);
    assert!(repository.index().unwrap().has_conflicts());
    fs::write(context.worktree.join("fixture.txt"), "modified\n").unwrap();
    fs::write(context.worktree.join("staged.txt"), "staged\n").unwrap();
    let mut index = repository.index().unwrap();
    index.add_path(std::path::Path::new("staged.txt")).unwrap();
    index.write().unwrap();
    fs::write(context.worktree.join("untracked.txt"), "untracked\n").unwrap();
    fs::remove_file(context.worktree.join(".manyhands/config.toml")).unwrap();
    let before_index = support::index_bytes(&repository).unwrap();
    let before_status = status_entries(&repository);

    let (_, commit_oid) = saved_checkpoint(
        enabled
            .service
            .save_ticket(ticket_request(
                &fixture.root,
                ContextIntent::Create,
                "Title",
                "Body\n",
            ))
            .unwrap(),
    );
    let repository = Repository::open(&context.worktree).unwrap();
    assert_eq!(support::index_bytes(&repository).unwrap(), before_index);
    assert!(repository.index().unwrap().has_conflicts());
    assert_eq!(
        status_entries(&repository)
            .into_iter()
            .filter(|(path, _)| path.as_deref() != ticket_relative_path().to_str())
            .collect::<Vec<_>>(),
        before_status
    );
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, ticket_relative_path()),
        Some(canonical_ticket("Title", "Body\n").into_bytes())
    );
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, "staged.txt"),
        None
    );
    let parent = repository
        .find_commit(commit_oid)
        .unwrap()
        .parent(0)
        .unwrap();
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, "fixture.txt"),
        support::commit_tree_path(&repository, parent.id(), "fixture.txt")
    );
    assert_eq!(
        support::commit_tree_path(&repository, commit_oid, ".manyhands/config.toml"),
        support::commit_tree_path(&repository, parent.id(), ".manyhands/config.toml")
    );
}

#[test]
fn comment_document_root_writes_only_its_path_and_checkpoints() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let (context, commit_oid, publication) = saved_comment(
        enabled
            .service
            .submit_comment(comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Root body\n",
            ))
            .unwrap(),
    );
    let path = comment_path(&context, &support::root_comment_id());
    let source = fs::read_to_string(&path).unwrap();
    assert!(matches!(
        manyhands::canonical::parse_item(&comment_relative_path(&support::document_id(), &support::root_comment_id()), &source),
        Ok(manyhands::canonical::CanonicalItem::Comment(comment))
            if comment.id == support::root_comment_id()
                && comment.item_id == support::document_id()
                && comment.parent_id.is_none()
                && comment.body == "Root body\n"
                && comment.unknown.is_empty()
    ));
    let repository = Repository::open(&context.worktree).unwrap();
    assert!(matches!(
        publication,
        CommentPublicationState::PublishPending
    ));
    assert_eq!(
        repository.find_commit(commit_oid).unwrap().message(),
        Some("Checkpoint comment 01ARZ3NDEKTSV4RRFFQ69G5FAX")
    );
    assert_eq!(
        support::commit_tree_path(
            &repository,
            commit_oid,
            comment_relative_path(&support::document_id(), &support::root_comment_id())
        ),
        Some(source.into_bytes())
    );
}

#[test]
fn comment_ticket_reply_validates_parent_and_defers_configured_publication() {
    let fixture = support::born_repository();
    commit_source(
        &fixture,
        &ticket_relative_path().display().to_string(),
        &support::ticket_source(),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let (context, _, _) = saved_comment(
        enabled
            .service
            .submit_comment(comment_request(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Root\n",
            ))
            .unwrap(),
    );
    enabled
        .service
        .add_remote(manyhands::repository::AddRemoteRequest {
            root: fixture.root.clone(),
            name: "origin".to_owned(),
            url: "ssh://example.invalid/manyhands".to_owned(),
            operation_id: support::operation_id(),
        })
        .unwrap();
    enabled
        .service
        .set_publication_remote(manyhands::repository::SetPublicationRemoteRequest {
            root: fixture.root.clone(),
            name: Some("origin".to_owned()),
            operation_id: support::operation_id(),
        })
        .unwrap();

    let (reply_context, _, publication) = saved_comment(
        enabled
            .service
            .submit_comment(comment_request(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Edit,
                support::reply_id(),
                Some(support::root_comment_id()),
                "Reply\n",
            ))
            .unwrap(),
    );
    let source = fs::read_to_string(comment_path(&reply_context, &support::reply_id())).unwrap();
    assert_eq!(context.worktree, reply_context.worktree);
    assert!(matches!(publication, CommentPublicationState::SyncDeferred));
    assert!(matches!(
        manyhands::canonical::parse_item(&comment_relative_path(&support::ticket_id(), &support::reply_id()), &source),
        Ok(manyhands::canonical::CanonicalItem::Comment(comment))
            if comment.parent_id == Some(support::root_comment_id())
                && comment.item_id == support::ticket_id()
    ));
}

#[test]
fn comment_exact_retry_retains_timestamp_checkpoints_pending_then_noops() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::new_operation_id();
    let request = || {
        comment_request_with_operation_id(
            comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Retry\n",
            ),
            operation_id,
        )
    };
    let (context, commit_oid, _) =
        saved_comment(enabled.service.submit_comment(request()).unwrap());
    let path = comment_path(&context, &support::root_comment_id());
    let created_at = match manyhands::canonical::parse_item(
        &comment_relative_path(&support::document_id(), &support::root_comment_id()),
        &fs::read_to_string(&path).unwrap(),
    )
    .unwrap()
    {
        manyhands::canonical::CanonicalItem::Comment(comment) => comment.created_at,
        _ => unreachable!(),
    };

    let outcome = enabled.service.submit_comment(request()).unwrap();
    assert!(matches!(
        outcome,
        CommentSubmissionOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    let retried_created_at = match manyhands::canonical::parse_item(
        &comment_relative_path(&support::document_id(), &support::root_comment_id()),
        &fs::read_to_string(&path).unwrap(),
    )
    .unwrap()
    {
        manyhands::canonical::CanonicalItem::Comment(comment) => comment.created_at,
        _ => unreachable!(),
    };
    assert_eq!(retried_created_at, created_at);
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
}

#[test]
fn comment_rejects_invalid_target_parent_and_conflicting_id_without_mutation() {
    for case in [
        "create",
        "missing-target",
        "missing-parent",
        "cross-item",
        "conflicting-id",
    ] {
        let fixture = support::born_repository();
        commit_source(&fixture, "docs/fixture.md", &support::document_source());
        let enabled = support::enabled_repository(&fixture);
        clean_configuration_index(&fixture);
        let context = context_from(
            enabled
                .service
                .prepare_context(target(
                    &fixture.root,
                    AuthoringKind::Document,
                    support::document_id(),
                    ContextIntent::Edit,
                ))
                .unwrap(),
        );
        let path = comment_path(&context, &support::root_comment_id());
        let request = match case {
            "create" => comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
                support::root_comment_id(),
                None,
                "Body\n",
            ),
            "missing-target" => comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::ticket_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Body\n",
            ),
            "missing-parent" => comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                Some(support::reply_id()),
                "Body\n",
            ),
            "cross-item" => {
                let other = context.worktree.join(comment_relative_path(
                    &support::ticket_id(),
                    &support::reply_id(),
                ));
                fs::create_dir_all(other.parent().unwrap()).unwrap();
                fs::write(
                    other,
                    comment_source_for(&support::reply_id(), &support::ticket_id()),
                )
                .unwrap();
                comment_request(
                    &fixture.root,
                    AuthoringKind::Document,
                    support::document_id(),
                    ContextIntent::Edit,
                    support::root_comment_id(),
                    Some(support::reply_id()),
                    "Body\n",
                )
            }
            "conflicting-id" => {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(
                    &path,
                    comment_source_for(&support::root_comment_id(), &support::document_id())
                        .replace("\n---\n", "\n---\nOther\n"),
                )
                .unwrap();
                comment_request(
                    &fixture.root,
                    AuthoringKind::Document,
                    support::document_id(),
                    ContextIntent::Edit,
                    support::root_comment_id(),
                    None,
                    "Body\n",
                )
            }
            _ => unreachable!(),
        };
        let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);
        assert!(
            enabled.service.submit_comment(request).is_err(),
            "{case} must reject"
        );
        assert_eq!(
            rejection_state(&fixture, &enabled.service, &context, &[&path]),
            before,
            "{case} must not mutate state"
        );
    }
}

#[cfg(unix)]
#[test]
fn comment_rejects_symlinked_parent_without_writing_outside_context() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let external = tempfile::tempdir().unwrap();
    let parent = context.worktree.join(".manyhands/comments");
    fs::create_dir_all(parent.parent().unwrap()).unwrap();
    symlink(external.path(), &parent).unwrap();
    let before = rejection_state(&fixture, &enabled.service, &context, &[&parent]);

    let error = comment_error(enabled.service.submit_comment(comment_request(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Edit,
        support::root_comment_id(),
        None,
        "Body\n",
    )));
    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert!(
        !external
            .path()
            .join(support::document_id().to_string())
            .exists()
    );
    assert_eq!(
        rejection_state(&fixture, &enabled.service, &context, &[&parent]),
        before
    );
}

#[test]
fn recovery_comment_registry_failure_preserves_live_index_and_retries() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let repository = Repository::open(&context.worktree).unwrap();
    fs::write(context.worktree.join("unrelated.txt"), "unrelated\n").unwrap();
    let mut index = repository.index().unwrap();
    index
        .add_path(std::path::Path::new("unrelated.txt"))
        .unwrap();
    index.write().unwrap();
    let before_index = support::index_bytes(&repository).unwrap();
    let path = comment_path(&context, &support::root_comment_id());
    enabled
        .service
        .with_registry_connection_for_testing(|connection| {
            connection
                .execute("UPDATE repositories SET refresh_required = 0", [])
                .unwrap()
        })
        .unwrap();
    let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    let failing = support::FailOnce::at(FailurePoint::BeforeRegistryWrite)
        .open_service(enabled.data_directory.path());
    let operation_id = support::new_operation_id();
    let request = || {
        comment_request_with_operation_id(
            comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Body\n",
            ),
            operation_id,
        )
    };

    let CommentSubmissionOutcome::Saved {
        checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
        ..
    } = failing.submit_comment(request()).unwrap()
    else {
        panic!("comment commit must retain refresh recovery");
    };
    assert_eq!(
        support::index_bytes(&Repository::open(&context.worktree).unwrap()).unwrap(),
        before_index
    );
    let after_failure = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_eq!(after_failure.worktrees, before.worktrees);
    assert_eq!(
        after_failure.registered_worktrees,
        before.registered_worktrees
    );
    assert_eq!(after_failure.index, before.index);
    assert_eq!(after_failure.commit_count, before.commit_count + 1);
    assert_eq!(after_failure.registry_rows, before.registry_rows);
    assert_checkpoint_completion_delta(
        &before,
        &after_failure,
        &context.branch,
        commit_oid,
        after_failure.files.clone(),
        statuses_with_owned_checkpoint_delta(
            &before.statuses,
            &[(
                &comment_relative_path(&support::document_id(), &support::root_comment_id())
                    .display()
                    .to_string(),
                git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
            )],
        ),
    );
    let outcome = enabled.service.submit_comment(request()).unwrap();
    assert!(matches!(
        outcome,
        CommentSubmissionOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(
        support::head_commit(&Repository::open(&context.worktree).unwrap()),
        Some(commit_oid)
    );
    let after_retry = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_retry_only_updates_registry(&after_failure, &after_retry);
    assert_registry_refresh_is_the_only_row_change(
        &after_failure.registry_rows,
        &after_retry.registry_rows,
    );
}

#[test]
fn recovery_comment_checkpoint_failure_preserves_absent_parent_and_retries() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let path = comment_path(&context, &support::root_comment_id());
    let mut config = Repository::open(&context.worktree)
        .unwrap()
        .config()
        .unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    drop(config);
    let effective = Config::new().unwrap();
    let operation_id = support::new_operation_id();
    let request = || {
        comment_request_with_operation_id(
            comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Body\n",
            ),
            operation_id,
        )
    };
    let outcome = enabled
        .service
        .submit_comment_with_identity_config_for_testing(request(), &effective)
        .unwrap();
    assert!(matches!(
        outcome,
        CommentSubmissionOutcome::IdentityRequired { .. }
    ));
    assert!(!path.parent().unwrap().exists());

    let mut config = Repository::open(&context.worktree)
        .unwrap()
        .config()
        .unwrap();
    config.set_str("user.name", "Manyhands Test").unwrap();
    config
        .set_str("user.email", "manyhands-test@example.invalid")
        .unwrap();

    let before_checkpoint = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    let comment_relative =
        comment_relative_path(&support::document_id(), &support::root_comment_id());
    let unrelated_before = statuses_excluding(
        &Repository::open(&context.worktree).unwrap(),
        &[comment_relative.to_str().unwrap()],
    );
    let failing = support::FailOnce::at(FailurePoint::BeforeCheckpointCommit)
        .open_service(enabled.data_directory.path());
    assert_eq!(
        comment_error(failing.submit_comment(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert!(path.is_file());
    let after_failure = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    let expected_comment_bytes = fs::read(&path).unwrap();
    assert_checkpoint_failure_preserves_resources(
        &before_checkpoint,
        &after_failure,
        vec![Some(expected_comment_bytes)],
        vec![(
            Some(comment_relative.to_str().unwrap().to_owned()),
            git2::Status::WT_NEW,
        )],
    );
    let created_at = match manyhands::canonical::parse_item(
        &comment_relative_path(&support::document_id(), &support::root_comment_id()),
        &fs::read_to_string(&path).unwrap(),
    )
    .unwrap()
    {
        manyhands::canonical::CanonicalItem::Comment(comment) => comment.created_at,
        _ => unreachable!(),
    };
    let retry_outcome = enabled.service.submit_comment(request()).unwrap();
    assert!(matches!(
        &retry_outcome,
        CommentSubmissionOutcome::Saved {
            checkpoint: LocalCheckpoint::Checkpointed { .. },
            ..
        }
    ));
    let CommentSubmissionOutcome::Saved {
        checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
        ..
    } = retry_outcome
    else {
        unreachable!()
    };
    let retried_created_at = match manyhands::canonical::parse_item(
        &comment_relative_path(&support::document_id(), &support::root_comment_id()),
        &fs::read_to_string(&path).unwrap(),
    )
    .unwrap()
    {
        manyhands::canonical::CanonicalItem::Comment(comment) => comment.created_at,
        _ => unreachable!(),
    };
    assert_eq!(retried_created_at, created_at);
    let after_retry = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_checkpoint_completion_delta(
        &before_checkpoint,
        &after_retry,
        &context.branch,
        commit_oid,
        after_failure.files.clone(),
        statuses_with_owned_checkpoint_delta(
            &before_checkpoint.statuses,
            &[(
                &comment_relative.display().to_string(),
                git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
            )],
        ),
    );
    assert!(path.is_file());
    assert_eq!(after_retry.index, before_checkpoint.index);
    assert_eq!(
        statuses_excluding(
            &Repository::open(&context.worktree).unwrap(),
            &[comment_relative.to_str().unwrap()]
        ),
        unrelated_before
    );
    assert_eq!(after_retry.commit_count, before_checkpoint.commit_count + 1);
    assert_eq!(after_retry.worktrees, before_checkpoint.worktrees);
    assert_eq!(
        after_retry.registered_worktrees,
        before_checkpoint.registered_worktrees
    );
    assert_eq!(after_retry.registry_rows, before_checkpoint.registry_rows);
}

#[test]
fn comment_rejects_any_existing_context_problem_without_mutation() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let malformed = context
        .worktree
        .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAW/bad.md");
    fs::create_dir_all(malformed.parent().unwrap()).unwrap();
    fs::write(&malformed, "not canonical markdown\n").unwrap();
    let path = comment_path(&context, &support::root_comment_id());
    let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);

    let error = comment_error(enabled.service.submit_comment(comment_request(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Edit,
        support::root_comment_id(),
        None,
        "Body\n",
    )));
    assert_eq!(error.kind, RepositoryErrorKind::MissingAuthoringTarget);
    assert!(!path.parent().unwrap().exists());
    assert_eq!(
        rejection_state(&fixture, &enabled.service, &context, &[&path]),
        before
    );
}

#[test]
fn comment_missing_registration_returns_refresh_pending_then_retries_only_invalidation() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::new_operation_id();
    let request = || {
        comment_request_with_operation_id(
            comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Body\n",
            ),
            operation_id,
        )
    };
    enabled
        .service
        .remove_registration(manyhands::repository::RemoveRegistrationRequest {
            root: fixture.root.clone(),
            operation_id: support::operation_id(),
        })
        .unwrap();

    let CommentSubmissionOutcome::Saved {
        context,
        checkpoint,
        ..
    } = enabled.service.submit_comment(request()).unwrap()
    else {
        panic!("a changed comment with no registration must retain its checkpoint OID");
    };
    let (LocalCheckpoint::RefreshPending { commit_oid }
    | LocalCheckpoint::Checkpointed { commit_oid }) = checkpoint
    else {
        panic!("a changed comment must retain its checkpoint OID");
    };
    assert!(matches!(
        enabled
            .service
            .enable(EnableRepositoryRequest {
                root: fixture.root.clone(),
                primary_branch: "main".to_owned(),
                identity: None,
                operation_id: support::operation_id(),
            })
            .unwrap(),
        EnableRepositoryOutcome::AlreadyEnabled
    ));
    let outcome = enabled.service.submit_comment(request()).unwrap();
    assert!(matches!(
        outcome,
        CommentSubmissionOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }
    ));
    assert_eq!(
        support::head_commit(&Repository::open(context.worktree).unwrap()),
        Some(commit_oid)
    );
    assert_eq!(registry_refresh_required(&enabled.service), 1);
}

#[test]
fn recovery_comment_before_item_write_preserves_absent_parent_and_retries() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let path = comment_path(&context, &support::root_comment_id());
    let repository = Repository::open(&context.worktree).unwrap();
    fs::write(context.worktree.join("unrelated.txt"), "unrelated\n").unwrap();
    let before = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    let failing = support::FailOnce::at(FailurePoint::BeforeItemWrite)
        .open_service(enabled.data_directory.path());
    let operation_id = support::new_operation_id();
    let request = || {
        comment_request_with_operation_id(
            comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Body\n",
            ),
            operation_id,
        )
    };

    assert_eq!(
        comment_error(failing.submit_comment(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    assert!(!path.parent().unwrap().exists());
    let failure_state = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_eq!(failure_state, before);
    let retry_outcome = enabled.service.submit_comment(request()).unwrap();
    assert!(matches!(
        &retry_outcome,
        CommentSubmissionOutcome::Saved {
            checkpoint: LocalCheckpoint::Checkpointed { .. },
            ..
        }
    ));
    let CommentSubmissionOutcome::Saved {
        checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
        ..
    } = retry_outcome
    else {
        unreachable!()
    };
    let retry_state = rejection_state(&fixture, &enabled.service, &context, &[&path]);
    assert_checkpoint_completion_delta(
        &before,
        &retry_state,
        &context.branch,
        commit_oid,
        retry_state.files.clone(),
        statuses_with_owned_checkpoint_delta(
            &before.statuses,
            &[(
                &comment_relative_path(&support::document_id(), &support::root_comment_id())
                    .display()
                    .to_string(),
                git2::Status::INDEX_DELETED | git2::Status::WT_NEW,
            )],
        ),
    );
    assert!(
        repository
            .workdir()
            .unwrap()
            .join("unrelated.txt")
            .is_file()
    );
}

#[test]
fn comment_root_ticket_and_reply_document_have_expected_checkpoint_and_publication() {
    let fixture = support::born_repository();
    commit_source(
        &fixture,
        &ticket_relative_path().display().to_string(),
        &support::ticket_source(),
    );
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let (ticket_context, ticket_commit, ticket_publication) = saved_comment(
        enabled
            .service
            .submit_comment(comment_request(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Edit,
                "01J00000000000000000000003".parse().unwrap(),
                None,
                "Ticket root\n",
            ))
            .unwrap(),
    );
    assert!(matches!(
        ticket_publication,
        CommentPublicationState::PublishPending
    ));
    assert_eq!(
        Repository::open(&ticket_context.worktree)
            .unwrap()
            .find_commit(ticket_commit)
            .unwrap()
            .message(),
        Some("Checkpoint comment 01J00000000000000000000003")
    );

    let (document_context, _, _) = saved_comment(
        enabled
            .service
            .submit_comment(comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::root_comment_id(),
                None,
                "Document root\n",
            ))
            .unwrap(),
    );
    let (reply_context, reply_commit, reply_publication) = saved_comment(
        enabled
            .service
            .submit_comment(comment_request(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
                support::reply_id(),
                Some(support::root_comment_id()),
                "Document reply\n",
            ))
            .unwrap(),
    );
    assert_eq!(document_context.worktree, reply_context.worktree);
    assert!(matches!(
        reply_publication,
        CommentPublicationState::PublishPending
    ));
    assert_eq!(
        Repository::open(&reply_context.worktree)
            .unwrap()
            .find_commit(reply_commit)
            .unwrap()
            .message(),
        Some("Checkpoint comment 01ARZ3NDEKTSV4RRFFQ69G5FAY")
    );
}

#[test]
fn comment_rejects_a_valid_same_id_comment_at_another_canonical_path() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    commit_source(
        &fixture,
        &ticket_relative_path().display().to_string(),
        &support::ticket_source(),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let conflicting = context.worktree.join(comment_relative_path(
        &support::ticket_id(),
        &support::root_comment_id(),
    ));
    fs::create_dir_all(conflicting.parent().unwrap()).unwrap();
    fs::write(
        &conflicting,
        comment_source_for(&support::root_comment_id(), &support::ticket_id()),
    )
    .unwrap();
    let path = comment_path(&context, &support::root_comment_id());
    let before = rejection_state(&fixture, &enabled.service, &context, &[&path, &conflicting]);

    let error = comment_error(enabled.service.submit_comment(comment_request(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Edit,
        support::root_comment_id(),
        None,
        "Body\n",
    )));
    assert_eq!(error.kind, RepositoryErrorKind::OccupiedItemPath);
    assert_eq!(
        rejection_state(&fixture, &enabled.service, &context, &[&path, &conflicting]),
        before
    );
}

#[derive(Debug, PartialEq)]
struct RejectionState {
    refs: Vec<(String, bool, Option<git2::Oid>)>,
    branches: Vec<(String, git2::Oid)>,
    worktrees: Vec<std::path::PathBuf>,
    registered_worktrees: Vec<(String, std::path::PathBuf)>,
    index: Option<Vec<u8>>,
    statuses: Vec<(Option<String>, git2::Status)>,
    head: git2::Oid,
    commit_count: usize,
    files: Vec<Option<Vec<u8>>>,
    registry_count: i64,
    registry_rows: Vec<RegistryRow>,
}

#[derive(Debug, PartialEq)]
struct RegistryRow {
    id: i64,
    root_path: String,
    enabled_at: i64,
    accessibility: String,
    config_blob_oid: String,
    refresh_required: i64,
}

fn rejection_state(
    fixture: &support::TestRepository,
    service: &RepositoryService,
    context: &manyhands::repository::ItemContext,
    files: &[&std::path::Path],
) -> RejectionState {
    let repository = Repository::open(&context.worktree).unwrap();
    RejectionState {
        refs: reference_state(&fixture.repository),
        branches: branch_targets(&fixture.repository),
        worktrees: worktree_paths(&fixture.repository),
        registered_worktrees: registered_worktrees(&fixture.repository),
        index: support::index_bytes(&repository),
        statuses: status_entries(&repository),
        head: repository.head().unwrap().target().unwrap(),
        commit_count: commit_count(&repository),
        files: files.iter().map(|file| fs::read(file).ok()).collect(),
        registry_count: registry_count(service),
        registry_rows: registry_rows(service),
    }
}

fn assert_checkpoint_failure_preserves_resources(
    before: &RejectionState,
    after: &RejectionState,
    files: Vec<Option<Vec<u8>>>,
    statuses: Vec<(Option<String>, git2::Status)>,
) {
    assert_eq!(after.refs, before.refs);
    assert_eq!(after.branches, before.branches);
    assert_eq!(after.worktrees, before.worktrees);
    assert_eq!(after.registered_worktrees, before.registered_worktrees);
    assert_eq!(after.index, before.index);
    assert_eq!(after.statuses, statuses);
    assert_eq!(after.head, before.head);
    assert_eq!(after.commit_count, before.commit_count);
    assert_eq!(after.files, files);
    assert_eq!(after.registry_count, before.registry_count);
    assert_eq!(after.registry_rows, before.registry_rows);
}

#[derive(Debug, PartialEq)]
struct LinkedWorktreeState {
    head: git2::Oid,
    commit_count: usize,
    index: Option<Vec<u8>>,
    statuses: Vec<(Option<String>, git2::Status)>,
    files: Vec<Option<Vec<u8>>>,
}

fn linked_worktree_state(
    context: &manyhands::repository::ItemContext,
    files: &[&std::path::Path],
) -> LinkedWorktreeState {
    let repository = Repository::open(&context.worktree).unwrap();
    LinkedWorktreeState {
        head: repository.head().unwrap().target().unwrap(),
        commit_count: commit_count(&repository),
        index: support::index_bytes(&repository),
        statuses: status_entries(&repository),
        files: files.iter().map(|path| fs::read(path).ok()).collect(),
    }
}

fn assert_initial_linked_worktree(
    primary: &Repository,
    context: &manyhands::repository::ItemContext,
) {
    let state = linked_worktree_state(context, &[]);
    let linked = Repository::open(&context.worktree).unwrap();
    assert_eq!(state.head, support::head_commit(primary).unwrap());
    assert_eq!(state.commit_count, commit_count(primary));
    assert!(state.index.is_some());
    assert_eq!(
        linked.index().unwrap().write_tree().unwrap(),
        linked.head().unwrap().peel_to_commit().unwrap().tree_id()
    );
    assert!(state.statuses.is_empty());
}

fn assert_linked_checkpoint_delta(
    before: &LinkedWorktreeState,
    after: &LinkedWorktreeState,
    commit_oid: git2::Oid,
    files: Vec<Option<Vec<u8>>>,
    statuses: Vec<(Option<String>, git2::Status)>,
) {
    assert_eq!(after.head, commit_oid);
    assert_eq!(after.commit_count, before.commit_count + 1);
    assert_eq!(after.index, before.index);
    assert_eq!(after.statuses, statuses);
    assert_eq!(after.files, files);
}

fn assert_retry_only_updates_registry(before: &RejectionState, after: &RejectionState) {
    assert_eq!(after.refs, before.refs);
    assert_eq!(after.branches, before.branches);
    assert_eq!(after.worktrees, before.worktrees);
    assert_eq!(after.registered_worktrees, before.registered_worktrees);
    assert_eq!(after.index, before.index);
    assert_eq!(after.statuses, before.statuses);
    assert_eq!(after.head, before.head);
    assert_eq!(after.commit_count, before.commit_count);
    assert_eq!(after.files, before.files);
    assert_eq!(after.registry_count, before.registry_count);
}

fn assert_checkpoint_completion_delta(
    before: &RejectionState,
    after: &RejectionState,
    branch: &str,
    commit_oid: git2::Oid,
    files: Vec<Option<Vec<u8>>>,
    statuses: Vec<(Option<String>, git2::Status)>,
) {
    let mut expected_refs = before.refs.clone();
    let reference = format!("refs/heads/{branch}");
    let (_, _, target) = expected_refs
        .iter_mut()
        .find(|(name, _, _)| name == &reference)
        .unwrap();
    *target = Some(commit_oid);
    assert_eq!(after.refs, expected_refs);
    let mut expected_branches = before.branches.clone();
    let (_, target) = expected_branches
        .iter_mut()
        .find(|(name, _)| name == branch)
        .unwrap();
    *target = commit_oid;
    assert_eq!(after.branches, expected_branches);
    assert_eq!(after.worktrees, before.worktrees);
    assert_eq!(after.registered_worktrees, before.registered_worktrees);
    assert_eq!(after.index, before.index);
    assert_eq!(after.statuses, statuses);
    assert_eq!(after.head, commit_oid);
    assert_eq!(after.commit_count, before.commit_count + 1);
    assert_eq!(after.files, files);
    assert_eq!(after.registry_count, before.registry_count);
    assert_eq!(after.registry_rows, before.registry_rows);
}

fn statuses_with_owned_checkpoint_delta(
    before: &[(Option<String>, git2::Status)],
    owned: &[(&str, git2::Status)],
) -> Vec<(Option<String>, git2::Status)> {
    let mut expected = before
        .iter()
        .filter(|(path, _)| {
            !owned
                .iter()
                .any(|(owned_path, _)| path.as_deref() == Some(*owned_path))
        })
        .cloned()
        .collect::<Vec<_>>();
    expected.extend(
        owned
            .iter()
            .map(|(path, status)| (Some((*path).to_owned()), *status)),
    );
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    expected
}

fn reference_state(repository: &Repository) -> Vec<(String, bool, Option<git2::Oid>)> {
    let mut references = repository
        .references()
        .unwrap()
        .map(|reference| {
            let reference = reference.unwrap();
            (
                reference.name().unwrap().to_owned(),
                reference.symbolic_target().is_some(),
                reference.target(),
            )
        })
        .collect::<Vec<_>>();
    references.sort();
    references
}

fn assert_registry_refresh_is_the_only_row_change(before: &[RegistryRow], after: &[RegistryRow]) {
    assert_eq!(after.len(), before.len());
    for (before, after) in before.iter().zip(after) {
        assert_eq!(after.id, before.id);
        assert_eq!(after.root_path, before.root_path);
        assert_eq!(after.enabled_at, before.enabled_at);
        assert_eq!(after.accessibility, before.accessibility);
        assert_eq!(after.config_blob_oid, before.config_blob_oid);
        assert_eq!(before.refresh_required, 0);
        assert_eq!(after.refresh_required, 1);
    }
}

fn status_entries(repository: &Repository) -> Vec<(Option<String>, git2::Status)> {
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    repository
        .statuses(Some(&mut options))
        .unwrap()
        .iter()
        .map(|entry| (entry.path().map(str::to_owned), entry.status()))
        .collect()
}

fn unrelated_status_entries(repository: &Repository) -> Vec<(Option<String>, git2::Status)> {
    status_entries(repository)
        .into_iter()
        .filter(|(path, _)| path.as_deref() != Some("docs/new.md"))
        .collect()
}

fn statuses_excluding(
    repository: &Repository,
    owned_paths: &[&str],
) -> Vec<(Option<String>, git2::Status)> {
    status_entries(repository)
        .into_iter()
        .filter(|(path, _)| !owned_paths.contains(&path.as_deref().unwrap_or_default()))
        .collect()
}

fn target(
    root: &std::path::Path,
    kind: AuthoringKind,
    item_id: manyhands::canonical::ItemId,
    intent: ContextIntent,
) -> AuthoringTarget {
    target_with_operation_id(root, kind, item_id, intent, support::operation_id())
}

fn target_with_operation_id(
    root: &std::path::Path,
    kind: AuthoringKind,
    item_id: manyhands::canonical::ItemId,
    intent: ContextIntent,
    operation_id: manyhands::repository::OperationId,
) -> AuthoringTarget {
    AuthoringTarget {
        root: root.to_owned(),
        kind,
        item_id,
        intent,
        operation_id,
    }
}

fn document_request(
    root: &std::path::Path,
    intent: ContextIntent,
    source_path: Option<&str>,
    destination_path: &str,
    title: &str,
    body: &str,
) -> SaveDocumentRequest {
    document_request_with_operation_id(
        root,
        intent,
        source_path,
        destination_path,
        title,
        body,
        support::operation_id(),
    )
}

fn document_request_with_operation_id(
    root: &std::path::Path,
    intent: ContextIntent,
    source_path: Option<&str>,
    destination_path: &str,
    title: &str,
    body: &str,
    operation_id: manyhands::repository::OperationId,
) -> SaveDocumentRequest {
    SaveDocumentRequest {
        target: target_with_operation_id(
            root,
            AuthoringKind::Document,
            support::document_id(),
            intent,
            operation_id,
        ),
        source_path: source_path.map(std::path::PathBuf::from),
        destination_path: std::path::PathBuf::from(destination_path),
        draft: DocumentDraft {
            title: title.to_owned(),
            body: body.to_owned(),
        },
        expected_source: source_path
            .map(|path| expected_context_observation(root, &support::document_id(), path)),
        expected_destination: expected_context_observation(
            root,
            &support::document_id(),
            destination_path,
        ),
    }
}

fn ticket_relative_path() -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        ".manyhands/tickets/{}/ticket.md",
        support::ticket_id()
    ))
}

fn ticket_path(context: &manyhands::repository::ItemContext) -> std::path::PathBuf {
    context.worktree.join(ticket_relative_path())
}

fn ticket_request(
    root: &std::path::Path,
    intent: ContextIntent,
    title: &str,
    body: &str,
) -> SaveTicketRequest {
    ticket_request_with_operation_id(root, intent, title, body, support::new_operation_id())
}

fn ticket_request_with_operation_id(
    root: &std::path::Path,
    intent: ContextIntent,
    title: &str,
    body: &str,
    operation_id: manyhands::repository::OperationId,
) -> SaveTicketRequest {
    SaveTicketRequest {
        target: target_with_operation_id(
            root,
            AuthoringKind::Ticket,
            support::ticket_id(),
            intent,
            operation_id,
        ),
        draft: TicketDraft {
            title: title.to_owned(),
            ticket_type: "feature".to_owned(),
            status: "open".to_owned(),
            project: Some("manyhands".to_owned()),
            team: Some("core".to_owned()),
            body: body.to_owned(),
        },
        expected_path: expected_context_observation(
            root,
            &support::ticket_id(),
            ticket_relative_path(),
        ),
    }
}

fn comment_relative_path(
    item_id: &manyhands::canonical::ItemId,
    comment_id: &manyhands::canonical::ItemId,
) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(".manyhands/comments/{item_id}/{comment_id}.md"))
}

fn comment_path(
    context: &manyhands::repository::ItemContext,
    comment_id: &manyhands::canonical::ItemId,
) -> std::path::PathBuf {
    context
        .worktree
        .join(comment_relative_path(&context.item_id, comment_id))
}

fn comment_request(
    root: &std::path::Path,
    kind: AuthoringKind,
    item_id: manyhands::canonical::ItemId,
    intent: ContextIntent,
    comment_id: manyhands::canonical::ItemId,
    parent_id: Option<manyhands::canonical::ItemId>,
    body: &str,
) -> SubmitCommentRequest {
    SubmitCommentRequest {
        target: target(root, kind, item_id.clone(), intent),
        comment_id: comment_id.clone(),
        parent_id,
        body: body.to_owned(),
        expected_destination: expected_context_observation(
            root,
            &item_id,
            comment_relative_path(&item_id, &comment_id),
        ),
    }
}

fn expected_context_observation(
    root: &std::path::Path,
    item_id: &manyhands::canonical::ItemId,
    relative: impl AsRef<std::path::Path>,
) -> manyhands::repository::ExpectedPathObservation {
    let context_root = root.join(".manyhands/worktrees").join(item_id.to_string());
    let path = if context_root.exists() {
        context_root.join(relative.as_ref())
    } else {
        root.join(relative)
    };
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            manyhands::repository::ExpectedPathObservation::from_bytes(&fs::read(path).unwrap())
        }
        _ => manyhands::repository::ExpectedPathObservation::Missing,
    }
}

fn comment_request_with_operation_id(
    mut request: SubmitCommentRequest,
    operation_id: manyhands::repository::OperationId,
) -> SubmitCommentRequest {
    request.target.operation_id = operation_id;
    request
}

fn canonical_ticket(title: &str, body: &str) -> String {
    manyhands::canonical::serialize_item(&manyhands::canonical::CanonicalItem::Ticket(
        manyhands::canonical::Ticket {
            id: support::ticket_id(),
            title: title.to_owned(),
            ticket_type: "feature".to_owned(),
            status: "open".to_owned(),
            project: Some("manyhands".to_owned()),
            team: Some("core".to_owned()),
            closed_at: None,
            closed_by: None,
            body: body.to_owned(),
            unknown: serde_yaml::Mapping::new(),
        },
    ))
    .unwrap()
}

fn canonical_document(title: &str, body: &str) -> String {
    manyhands::canonical::serialize_item(&manyhands::canonical::CanonicalItem::Document(
        manyhands::canonical::Document {
            id: support::document_id(),
            title: title.to_owned(),
            body: body.to_owned(),
            unknown: serde_yaml::Mapping::new(),
        },
    ))
    .unwrap()
}

fn saved_checkpoint(outcome: SaveOutcome) -> (manyhands::repository::ItemContext, git2::Oid) {
    let SaveOutcome::Saved {
        context,
        checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
    } = outcome
    else {
        panic!("document save must create a checkpoint");
    };
    (context, commit_oid)
}

fn saved_comment(
    outcome: CommentSubmissionOutcome,
) -> (
    manyhands::repository::ItemContext,
    git2::Oid,
    CommentPublicationState,
) {
    match outcome {
        CommentSubmissionOutcome::Saved {
            context,
            checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
            publication,
        }
        | CommentSubmissionOutcome::IndexPending {
            context,
            checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
            publication,
        } => (context, commit_oid, publication),
        _ => panic!("comment submission must create a checkpoint"),
    }
}

fn document_error(
    result: Result<SaveOutcome, manyhands::repository::RepositoryError>,
) -> manyhands::repository::RepositoryError {
    match result {
        Ok(_) => panic!("document save unexpectedly succeeded"),
        Err(error) => error,
    }
}

fn ticket_error(
    result: Result<SaveOutcome, manyhands::repository::RepositoryError>,
) -> manyhands::repository::RepositoryError {
    match result {
        Ok(_) => panic!("ticket save unexpectedly succeeded"),
        Err(error) => error,
    }
}

fn comment_error(
    result: Result<CommentSubmissionOutcome, manyhands::repository::RepositoryError>,
) -> manyhands::repository::RepositoryError {
    match result {
        Ok(_) => panic!("comment submission unexpectedly succeeded"),
        Err(error) => error,
    }
}

fn context_from(outcome: ContextProvisionOutcome) -> manyhands::repository::ItemContext {
    match outcome {
        ContextProvisionOutcome::Created(context) | ContextProvisionOutcome::Reused(context) => {
            context
        }
        ContextProvisionOutcome::IndexPending { context } => context,
    }
}

fn context_error(
    result: Result<ContextProvisionOutcome, manyhands::repository::RepositoryError>,
) -> manyhands::repository::RepositoryError {
    match result {
        Ok(_) => panic!("context preparation unexpectedly succeeded"),
        Err(error) => error,
    }
}

fn commit_source(fixture: &support::TestRepository, path: &str, source: &str) {
    let path = fixture.root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, source).unwrap();
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(path.strip_prefix(&fixture.root).unwrap())
        .unwrap();
    let tree = fixture
        .repository
        .find_tree(index.write_tree().unwrap())
        .unwrap();
    let parent = fixture.repository.head().unwrap().peel_to_commit().unwrap();
    let signature = Signature::new(
        "Manyhands Test",
        "manyhands-test@example.invalid",
        &Time::new(0, 0),
    )
    .unwrap();
    fixture
        .repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Add canonical item",
            &tree,
            &[&parent],
        )
        .unwrap();
    index.write().unwrap();
}

fn clean_configuration_index(fixture: &support::TestRepository) {
    let mut index = fixture.repository.index().unwrap();
    index
        .add_path(std::path::Path::new(".manyhands/config.toml"))
        .unwrap();
    index.write().unwrap();
}

#[derive(Debug, PartialEq)]
struct ContextState {
    refs: Vec<(String, bool, Option<git2::Oid>)>,
    branches: Vec<(String, git2::Oid)>,
    worktrees: Vec<std::path::PathBuf>,
    registered_worktrees: Vec<(String, std::path::PathBuf)>,
    index: Option<Vec<u8>>,
    statuses: Vec<(Option<String>, git2::Status)>,
    head: Option<git2::Oid>,
    commit_count: usize,
    relevant_file: Option<Vec<u8>>,
    registry_count: i64,
    registry_rows: Vec<RegistryRow>,
}

fn context_state(
    fixture: &support::TestRepository,
    service: &RepositoryService,
    relevant_file: Option<&std::path::Path>,
) -> ContextState {
    ContextState {
        refs: reference_state(&fixture.repository),
        branches: branch_targets(&fixture.repository),
        worktrees: worktree_paths(&fixture.repository),
        registered_worktrees: registered_worktrees(&fixture.repository),
        index: support::index_bytes(&fixture.repository),
        statuses: status_entries(&fixture.repository),
        head: support::head_commit(&fixture.repository),
        commit_count: commit_count(&fixture.repository),
        relevant_file: relevant_file.map(|path| fs::read(path).unwrap()),
        registry_count: registry_count(service),
        registry_rows: registry_rows(service),
    }
}

fn assert_context_state(
    fixture: &support::TestRepository,
    service: &RepositoryService,
    relevant_file: Option<&std::path::Path>,
    expected: &ContextState,
) {
    assert_eq!(
        &context_state(fixture, service, relevant_file),
        expected,
        "rejected context preparation must not mutate local state"
    );
}

fn refs_with_branch(
    refs: &[(String, bool, Option<git2::Oid>)],
    branch: &str,
    target: git2::Oid,
) -> Vec<(String, bool, Option<git2::Oid>)> {
    let mut expected = refs.to_vec();
    expected.push((format!("refs/heads/{branch}"), false, Some(target)));
    expected.sort();
    expected
}

fn branches_with_branch(
    branches: &[(String, git2::Oid)],
    branch: &str,
    target: git2::Oid,
) -> Vec<(String, git2::Oid)> {
    let mut expected = branches.to_vec();
    expected.push((branch.to_owned(), target));
    expected.sort();
    expected
}

fn paths_with_worktree(
    worktrees: &[std::path::PathBuf],
    worktree: &std::path::Path,
) -> Vec<std::path::PathBuf> {
    let mut expected = worktrees.to_vec();
    expected.push(std::fs::canonicalize(worktree).unwrap());
    expected.sort();
    expected
}

fn registered_with_context(
    worktrees: &[(String, std::path::PathBuf)],
    context: &manyhands::repository::ItemContext,
) -> Vec<(String, std::path::PathBuf)> {
    let mut expected = worktrees.to_vec();
    expected.push((
        context.item_id.to_string(),
        std::fs::canonicalize(&context.worktree).unwrap(),
    ));
    expected.sort();
    expected
}

fn assert_context_resources_unchanged(before: &ContextState, after: &ContextState) {
    assert_eq!(after.index, before.index);
    assert_eq!(after.statuses, before.statuses);
    assert_eq!(after.head, before.head);
    assert_eq!(after.commit_count, before.commit_count);
    assert_eq!(after.relevant_file, before.relevant_file);
    assert_eq!(after.registry_count, before.registry_count);
    assert_eq!(after.registry_rows, before.registry_rows);
}

fn assert_checkpoint_failure_creates_only_owned_context(
    before: &ContextState,
    after: &ContextState,
    context: &manyhands::repository::ItemContext,
    commit_oid: git2::Oid,
) {
    assert_eq!(
        after.refs,
        refs_with_branch(&before.refs, &context.branch, commit_oid)
    );
    assert_eq!(
        after.branches,
        branches_with_branch(&before.branches, &context.branch, commit_oid)
    );
    assert_eq!(
        after.worktrees,
        paths_with_worktree(&before.worktrees, &context.worktree)
    );
    assert_eq!(
        after.registered_worktrees,
        registered_with_context(&before.registered_worktrees, context)
    );
    assert_context_resources_unchanged(before, after);
}

fn assert_retry_only_updates_registry_state(before: &ContextState, after: &ContextState) {
    assert_eq!(after.refs, before.refs);
    assert_eq!(after.branches, before.branches);
    assert_eq!(after.worktrees, before.worktrees);
    assert_eq!(after.registered_worktrees, before.registered_worktrees);
    assert_eq!(after.index, before.index);
    assert_eq!(after.statuses, before.statuses);
    assert_eq!(after.head, before.head);
    assert_eq!(after.commit_count, before.commit_count);
    assert_eq!(after.relevant_file, before.relevant_file);
    assert_eq!(after.registry_count, before.registry_count);
}

fn branch_targets(repository: &Repository) -> Vec<(String, git2::Oid)> {
    let mut branches = repository
        .branches(Some(git2::BranchType::Local))
        .unwrap()
        .map(|branch| {
            let (branch, _) = branch.unwrap();
            (
                branch.name().unwrap().unwrap().to_owned(),
                branch.get().target().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    branches.sort();
    branches
}

fn worktree_paths(repository: &Repository) -> Vec<std::path::PathBuf> {
    let mut paths = repository
        .worktrees()
        .unwrap()
        .iter()
        .flatten()
        .map(|name| physical_worktree_path(repository.find_worktree(name).unwrap().path()))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn registered_worktrees(repository: &Repository) -> Vec<(String, std::path::PathBuf)> {
    let mut worktrees = repository
        .worktrees()
        .unwrap()
        .iter()
        .flatten()
        .map(|name| {
            (
                name.to_owned(),
                physical_worktree_path(repository.find_worktree(name).unwrap().path()),
            )
        })
        .collect::<Vec<_>>();
    worktrees.sort();
    worktrees
}

fn physical_worktree_path(path: &std::path::Path) -> std::path::PathBuf {
    match path.canonicalize() {
        Ok(physical) => physical,
        // Stale registrations remain in the exact set, with their Git spelling intact.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => path.to_owned(),
        Err(error) => panic!("registered worktree path cannot be resolved: {error}"),
    }
}

fn commit_count(repository: &Repository) -> usize {
    let mut walk = repository.revwalk().unwrap();
    walk.push_head().unwrap();
    walk.count()
}

#[cfg(unix)]
fn registered_worktree_names(repository: &Repository) -> Vec<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;

    let mut names = fs::read_dir(repository.commondir().join("worktrees"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().as_bytes().to_vec())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn registry_count(service: &RepositoryService) -> i64 {
    service
        .with_registry_connection_for_testing(|connection| {
            connection
                .query_row("SELECT COUNT(*) FROM repositories", [], |row| row.get(0))
                .unwrap()
        })
        .unwrap()
}

fn registry_rows(service: &RepositoryService) -> Vec<RegistryRow> {
    service
        .with_registry_connection_for_testing(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, root_path, enabled_at, accessibility, config_blob_oid, refresh_required \
                     FROM repositories ORDER BY id",
                )
                .unwrap();
            statement
                .query_map([], |row| {
                    Ok(RegistryRow {
                        id: row.get(0)?,
                        root_path: row.get(1)?,
                        enabled_at: row.get(2)?,
                        accessibility: row.get(3)?,
                        config_blob_oid: row.get(4)?,
                        refresh_required: row.get(5)?,
                    })
                })
                .unwrap()
                .map(Result::unwrap)
                .collect()
        })
        .unwrap()
}

fn registry_refresh_required(service: &RepositoryService) -> i64 {
    service
        .with_registry_connection_for_testing(|connection| {
            connection
                .query_row("SELECT refresh_required FROM repositories", [], |row| {
                    row.get(0)
                })
                .unwrap()
        })
        .unwrap()
}

fn operation_record_rows(
    service: &RepositoryService,
) -> Vec<manyhands::repository::RecoveryInspection> {
    let root = service
        .with_registry_connection_for_testing(|connection| {
            connection
                .query_row("SELECT root_path FROM repositories LIMIT 1", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap()
        })
        .unwrap();
    service
        .recovery_inspection(std::path::Path::new(&root))
        .unwrap()
}

fn assert_redacted_authoring_failure(
    error: &manyhands::repository::RepositoryError,
    service: &RepositoryService,
    forbidden: &[&str],
) {
    let rendered = format!("{error:?}\n{error}");
    for forbidden in forbidden {
        assert!(!rendered.contains(forbidden));
    }
    let data_directory = service
        .with_registry_connection_for_testing(|connection| {
            connection
                .query_row("PRAGMA database_list", [], |row| row.get::<_, String>(2))
                .unwrap()
        })
        .unwrap();
    support::assert_operation_records_exclude(
        std::path::Path::new(&data_directory)
            .parent()
            .expect("registry has a parent directory"),
        forbidden,
    );
}

fn ticket_source_for(item_id: &manyhands::canonical::ItemId) -> String {
    support::ticket_source().replace(&support::ticket_id().to_string(), &item_id.to_string())
}

fn document_source_for(item_id: &manyhands::canonical::ItemId) -> String {
    support::document_source().replace(&support::document_id().to_string(), &item_id.to_string())
}

fn comment_source_for(
    comment_id: &manyhands::canonical::ItemId,
    item_id: &manyhands::canonical::ItemId,
) -> String {
    support::root_comment_source()
        .replace(&support::document_id().to_string(), &item_id.to_string())
        .replace(
            &support::root_comment_id().to_string(),
            &comment_id.to_string(),
        )
}

#[test]
fn recovery_ticket_branch_creation_failure_leaves_no_context_then_retries() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let config = fixture.root.join(".manyhands/config.toml");
    let pre_state = context_state(&fixture, &enabled.service, Some(&config));
    let branch = format!("manyhands/ticket/{}", support::ticket_id());
    let failing = support::FailOnce::at(FailurePoint::BeforeContextBranchCreation)
        .open_service(enabled.data_directory.path());
    let operation_id = support::operation_id();
    let request = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Ticket,
            support::ticket_id(),
            ContextIntent::Create,
            operation_id,
        )
    };

    assert_eq!(
        context_error(failing.prepare_context(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let failure_state = context_state(&fixture, &enabled.service, Some(&config));
    assert_eq!(failure_state, pre_state);

    let context = context_from(enabled.service.prepare_context(request()).unwrap());
    let retry_state = context_state(&fixture, &enabled.service, Some(&config));
    assert_initial_linked_worktree(&fixture.repository, &context);
    let primary_head = pre_state.head.unwrap();
    assert_eq!(context.branch, branch);
    assert_eq!(
        retry_state.refs,
        refs_with_branch(&pre_state.refs, &context.branch, primary_head)
    );
    assert_eq!(
        retry_state.branches,
        branches_with_branch(&pre_state.branches, &context.branch, primary_head)
    );
    assert_eq!(
        retry_state.worktrees,
        paths_with_worktree(&pre_state.worktrees, &context.worktree)
    );
    assert_eq!(
        retry_state.registered_worktrees,
        registered_with_context(&pre_state.registered_worktrees, &context)
    );
    assert_context_resources_unchanged(&pre_state, &retry_state);
}

#[test]
fn recovery_ticket_worktree_creation_failure_retains_branch_then_retries() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let config = fixture.root.join(".manyhands/config.toml");
    let pre_state = context_state(&fixture, &enabled.service, Some(&config));
    let branch = format!("manyhands/ticket/{}", support::ticket_id());
    let failing = support::FailOnce::at(FailurePoint::BeforeWorktreeCreation)
        .open_service(enabled.data_directory.path());
    let operation_id = support::operation_id();
    let request = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Ticket,
            support::ticket_id(),
            ContextIntent::Create,
            operation_id,
        )
    };

    assert_eq!(
        context_error(failing.prepare_context(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let failure_state = context_state(&fixture, &enabled.service, Some(&config));
    let primary_head = pre_state.head.unwrap();
    assert_eq!(
        failure_state.refs,
        refs_with_branch(&pre_state.refs, &branch, primary_head)
    );
    assert_eq!(
        failure_state.branches,
        branches_with_branch(&pre_state.branches, &branch, primary_head)
    );
    assert_eq!(failure_state.worktrees, pre_state.worktrees);
    assert_eq!(
        failure_state.registered_worktrees,
        pre_state.registered_worktrees
    );
    assert_context_resources_unchanged(&pre_state, &failure_state);

    let context = context_from(enabled.service.prepare_context(request()).unwrap());
    let retry_state = context_state(&fixture, &enabled.service, Some(&config));
    assert_initial_linked_worktree(&fixture.repository, &context);
    assert_eq!(context.branch, branch);
    assert_eq!(
        retry_state.refs,
        refs_with_branch(&pre_state.refs, &context.branch, primary_head)
    );
    assert_eq!(
        retry_state.branches,
        branches_with_branch(&pre_state.branches, &context.branch, primary_head)
    );
    assert_eq!(
        retry_state.worktrees,
        paths_with_worktree(&pre_state.worktrees, &context.worktree)
    );
    assert_eq!(
        retry_state.registered_worktrees,
        registered_with_context(&pre_state.registered_worktrees, &context)
    );
    assert_context_resources_unchanged(&pre_state, &retry_state);
}

#[test]
fn recovery_document_edit_worktree_creation_failure_retries_exact_primary_branch() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let failing = support::FailOnce::at(FailurePoint::BeforeWorktreeCreation)
        .open_service(enabled.data_directory.path());
    let operation_id = support::operation_id();
    let request = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Edit,
            operation_id,
        )
    };

    assert_eq!(
        context_error(failing.prepare_context(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let branch = format!("manyhands/document/{}", support::document_id());
    let before_retry = fixture
        .repository
        .find_branch(&branch, git2::BranchType::Local)
        .unwrap()
        .get()
        .target();
    assert!(worktree_paths(&fixture.repository).is_empty());
    let context = context_from(enabled.service.prepare_context(request()).unwrap());
    assert_eq!(
        fixture
            .repository
            .find_branch(&branch, git2::BranchType::Local)
            .unwrap()
            .get()
            .target(),
        before_retry
    );
    assert_eq!(
        worktree_paths(&fixture.repository),
        vec![std::fs::canonicalize(context.worktree).unwrap()]
    );
}

#[test]
fn context_replay_worktree_interruption_uses_one_journaled_context() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::operation_id();
    let target = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Create,
            operation_id,
        )
    };
    let failing = support::FailOnce::at(FailurePoint::BeforeWorktreeCreation)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        context_error(failing.prepare_context(target())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let pending = enabled.service.recovery_inspection(&fixture.root).unwrap();
    assert_eq!(pending.len(), 1);
    assert!(matches!(
        pending.as_slice(),
        [manyhands::repository::RecoveryInspection::Pending { operation_id: found, .. }]
            if *found == operation_id
    ));

    let fresh = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let context = context_from(fresh.prepare_context(target()).unwrap());
    assert_eq!(
        worktree_paths(&fixture.repository),
        vec![fs::canonicalize(&context.worktree).unwrap()]
    );
    assert!(fresh.recovery_inspection(&fixture.root).unwrap().is_empty());
}

#[test]
fn fresh_service_replays_branch_only_interruption_with_one_deterministic_branch() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::operation_id();
    let request = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Create,
            operation_id,
        )
    };
    let failing = support::FailOnce::at(FailurePoint::BeforeWorktreeCreation)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        context_error(failing.prepare_context(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let branch = format!("manyhands/document/{}", support::document_id());
    let target = fixture
        .repository
        .find_branch(&branch, git2::BranchType::Local)
        .unwrap()
        .get()
        .target()
        .unwrap();
    assert!(worktree_paths(&fixture.repository).is_empty());

    let fresh = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let context = context_from(fresh.prepare_context(request()).unwrap());
    assert_eq!(context.branch, branch);
    assert_eq!(
        fixture
            .repository
            .find_branch(&context.branch, git2::BranchType::Local)
            .unwrap()
            .get()
            .target(),
        Some(target)
    );
    assert_eq!(
        worktree_paths(&fixture.repository),
        vec![fs::canonicalize(context.worktree).unwrap()]
    );
}

#[test]
fn fresh_service_replays_worktree_only_interruption_with_one_deterministic_worktree() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::operation_id();
    let request = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Create,
            operation_id,
        )
    };
    let failing = support::FailOnce::at(FailurePoint::BeforeWorktreeCreation)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        context_error(failing.prepare_context(request())).kind,
        RepositoryErrorKind::InjectedFailure
    );
    let fresh = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let first = context_from(fresh.prepare_context(request()).unwrap());
    let replay = context_from(fresh.prepare_context(request()).unwrap());
    assert_eq!(replay.branch, first.branch);
    assert_eq!(replay.worktree, first.worktree);
    assert_eq!(
        worktree_paths(&fixture.repository),
        vec![fs::canonicalize(first.worktree).unwrap()]
    );
}

#[test]
fn stale_document_edit_preserves_external_replacement() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let path = context.worktree.join("docs/fixture.md");
    let expected =
        manyhands::repository::ExpectedPathObservation::from_bytes(&fs::read(&path).unwrap());
    let external = canonical_document("External", "External body\n");
    fs::write(&path, &external).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);
    let mut request = document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some("docs/fixture.md"),
        "docs/fixture.md",
        "Replacement",
        "Replacement body\n",
    );
    request.expected_source = Some(expected.clone());
    request.expected_destination = expected;

    let error = match enabled.service.save_document(request) {
        Ok(_) => panic!("stale edit must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

#[test]
fn stale_document_move_source_preserves_pre_save_repository_and_worktree_state() {
    assert_stale_document_move_source(support::born_repository());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_stale_document_move_source_preserves_state() {
    let (_alias_directory, fixture) = repository_with_symlinked_parent();
    assert_stale_document_move_source(fixture);
}

fn assert_stale_document_move_source(fixture: support::TestRepository) {
    commit_source(&fixture, "docs/source.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let source = context.worktree.join("docs/source.md");
    let source_expected = ExpectedPathObservation::from_bytes(&fs::read(&source).unwrap());
    let destination_expected = ExpectedPathObservation::Missing;
    let external = canonical_document("External source", "external source body\n");
    fs::write(&source, &external).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);
    let records_before = operation_record_rows(&enabled.service);
    let mut request = document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some("docs/source.md"),
        "docs/destination.md",
        "Requested move",
        "draft source body\n",
    );
    request.expected_source = Some(source_expected);
    request.expected_destination = destination_expected;

    let error = document_error(enabled.service.save_document(request));
    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    let diagnostic = error.external_change().unwrap();
    assert_eq!(diagnostic.root, fixture.root.canonicalize().unwrap());
    assert_eq!(diagnostic.operation, RepositoryOperation::SaveDocument);
    assert_eq!(diagnostic.item_id, support::document_id());
    assert_eq!(diagnostic.context, context.worktree);
    assert_eq!(diagnostic.path, std::path::PathBuf::from("docs/source.md"));
    assert_eq!(diagnostic.expectation, ExternalChangeExpectation::Changed);
    assert_redacted_authoring_failure(
        &error,
        &enabled.service,
        &["external source body", "draft source body"],
    );
    assert_eq!(fs::read_to_string(&source).unwrap(), external);
    assert!(!context.worktree.join("docs/destination.md").exists());
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);
}

#[test]
fn stale_document_move_destination_preserves_pre_save_repository_and_worktree_state() {
    assert_stale_document_move_destination(support::born_repository());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_stale_document_move_destination_preserves_state() {
    let (_alias_directory, fixture) = repository_with_symlinked_parent();
    assert_stale_document_move_destination(fixture);
}

fn assert_stale_document_move_destination(fixture: support::TestRepository) {
    commit_source(&fixture, "docs/source.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let source = context.worktree.join("docs/source.md");
    let source_expected = ExpectedPathObservation::from_bytes(&fs::read(&source).unwrap());
    let destination = context.worktree.join("docs/destination.md");
    let external = canonical_document("External destination", "external destination body\n");
    fs::write(&destination, &external).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);
    let records_before = operation_record_rows(&enabled.service);
    let mut request = document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some("docs/source.md"),
        "docs/destination.md",
        "Requested move",
        "draft destination body\n",
    );
    request.expected_source = Some(source_expected);
    request.expected_destination = ExpectedPathObservation::Missing;

    let error = document_error(enabled.service.save_document(request));
    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    let diagnostic = error.external_change().unwrap();
    assert_eq!(diagnostic.root, fixture.root.canonicalize().unwrap());
    assert_eq!(diagnostic.operation, RepositoryOperation::SaveDocument);
    assert_eq!(diagnostic.item_id, support::document_id());
    assert_eq!(diagnostic.context, context.worktree);
    assert_eq!(
        diagnostic.path,
        std::path::PathBuf::from("docs/destination.md")
    );
    assert_eq!(diagnostic.expectation, ExternalChangeExpectation::Missing);
    assert_redacted_authoring_failure(
        &error,
        &enabled.service,
        &["external destination body", "draft destination body"],
    );
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        support::document_source()
    );
    assert_eq!(fs::read_to_string(&destination).unwrap(), external);
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);
}

#[test]
fn stale_ticket_edit_preserves_pre_save_repository_and_worktree_state() {
    assert_stale_ticket_edit(support::born_repository());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_stale_ticket_edit_preserves_state() {
    let (_alias_directory, fixture) = repository_with_symlinked_parent();
    assert_stale_ticket_edit(fixture);
}

fn assert_stale_ticket_edit(fixture: support::TestRepository) {
    commit_source(
        &fixture,
        &ticket_relative_path().display().to_string(),
        &support::ticket_source(),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Ticket,
                support::ticket_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let path = ticket_path(&context);
    let expected = ExpectedPathObservation::from_bytes(&fs::read(&path).unwrap());
    let external = canonical_ticket("External ticket", "external ticket body\n");
    fs::write(&path, &external).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);
    let records_before = operation_record_rows(&enabled.service);
    let mut request = ticket_request(
        &fixture.root,
        ContextIntent::Edit,
        "Requested ticket",
        "draft ticket body\n",
    );
    request.expected_path = expected;

    let error = document_error(enabled.service.save_ticket(request));
    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    let diagnostic = error.external_change().unwrap();
    assert_eq!(diagnostic.root, fixture.root.canonicalize().unwrap());
    assert_eq!(diagnostic.operation, RepositoryOperation::SaveTicket);
    assert_eq!(diagnostic.item_id, support::ticket_id());
    assert_eq!(diagnostic.context, context.worktree);
    assert_eq!(diagnostic.path, ticket_relative_path());
    assert_eq!(diagnostic.expectation, ExternalChangeExpectation::Changed);
    assert_redacted_authoring_failure(
        &error,
        &enabled.service,
        &["external ticket body", "draft ticket body"],
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);
}

#[test]
fn stale_comment_creation_preserves_pre_save_repository_and_worktree_state() {
    assert_stale_comment_creation(support::born_repository());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_stale_comment_creation_preserves_state() {
    let (_alias_directory, fixture) = repository_with_symlinked_parent();
    assert_stale_comment_creation(fixture);
}

fn assert_stale_comment_creation(fixture: support::TestRepository) {
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let path = comment_path(&context, &support::root_comment_id());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let external = comment_source_for(&support::root_comment_id(), &support::document_id())
        + "external comment body\n";
    fs::write(&path, &external).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);
    let records_before = operation_record_rows(&enabled.service);
    let mut request = comment_request(
        &fixture.root,
        AuthoringKind::Document,
        support::document_id(),
        ContextIntent::Edit,
        support::root_comment_id(),
        None,
        "draft comment body\n",
    );
    request.expected_destination = ExpectedPathObservation::Missing;

    let error = comment_error(enabled.service.submit_comment(request));
    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    let diagnostic = error.external_change().unwrap();
    assert_eq!(diagnostic.root, fixture.root.canonicalize().unwrap());
    assert_eq!(diagnostic.operation, RepositoryOperation::SubmitComment);
    assert_eq!(diagnostic.item_id, support::document_id());
    assert_eq!(diagnostic.context, context.worktree);
    assert_eq!(
        diagnostic.path,
        comment_relative_path(&support::document_id(), &support::root_comment_id())
    );
    assert_eq!(diagnostic.expectation, ExternalChangeExpectation::Missing);
    assert_redacted_authoring_failure(
        &error,
        &enabled.service,
        &["external comment body", "draft comment body"],
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);
}

#[cfg(unix)]
#[test]
fn stale_owned_symlink_is_rejected_without_following_or_writing_outside_repository() {
    use std::os::unix::fs::symlink;

    let fixture = support::born_repository();
    commit_source(&fixture, "docs/fixture.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let source = context.worktree.join("docs/fixture.md");
    let expected = ExpectedPathObservation::from_bytes(&fs::read(&source).unwrap());
    let external = tempfile::tempdir().unwrap();
    let outside = external.path().join("outside.md");
    fs::write(&outside, "outside must remain unchanged\n").unwrap();
    fs::remove_file(&source).unwrap();
    symlink(&outside, &source).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);
    let records_before = operation_record_rows(&enabled.service);
    let mut request = document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some("docs/fixture.md"),
        "docs/fixture.md",
        "Requested",
        "draft body\n",
    );
    request.expected_source = Some(expected.clone());
    request.expected_destination = expected;

    let error = document_error(enabled.service.save_document(request));
    assert_eq!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert_eq!(
        fs::read_to_string(&outside).unwrap(),
        "outside must remain unchanged\n"
    );
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);
}

#[cfg(unix)]
#[test]
fn owned_leaf_swap_at_read_boundary_rejects_document_update_without_staging_outside_bytes() {
    use std::os::unix::fs::symlink;

    let _hook_guard = owned_path_hook_test_lock();
    let fixture = support::born_repository();
    let relative = std::path::PathBuf::from("docs/nofollow-read.md");
    commit_source(
        &fixture,
        relative.to_str().unwrap(),
        &support::document_source(),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let source = context.worktree.join(&relative);
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside.md");
    let outside_bytes = b"outside bytes must not be staged\n";
    fs::write(&outside_file, outside_bytes).unwrap();
    let repository = Repository::open(&context.worktree).unwrap();
    let head_before = support::head_commit(&repository);
    let index_before = support::index_bytes(&repository);
    let records_before = operation_record_rows(&enabled.service);
    let hook_outside_file = outside_file.clone();

    enabled.service.set_owned_path_hook_for_testing(
        relative.clone(),
        OwnedPathBoundary::Read,
        move || {
            fs::remove_file(&source).unwrap();
            symlink(&hook_outside_file, &source).unwrap();
        },
    );
    let error = document_error(enabled.service.save_document(document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some(relative.to_str().unwrap()),
        relative.to_str().unwrap(),
        "Updated",
        "updated body\n",
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_ne!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert!(!format!("{error:?}").contains("outside bytes"));
    assert_eq!(fs::read(&outside_file).unwrap(), outside_bytes);
    assert_eq!(support::head_commit(&repository), head_before);
    assert_eq!(support::index_bytes(&repository), index_before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);
}

#[cfg(unix)]
#[test]
fn owned_parent_swap_at_replace_boundary_rejects_document_create_without_writing_outside() {
    use std::os::unix::fs::symlink;

    let _hook_guard = owned_path_hook_test_lock();
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let relative = std::path::PathBuf::from("docs/nofollow-parent/new.md");
    let parent = context.worktree.join(relative.parent().unwrap());
    fs::create_dir_all(&parent).unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("new.md");
    let repository = Repository::open(&context.worktree).unwrap();
    let head_before = support::head_commit(&repository);
    let index_before = support::index_bytes(&repository);
    let records_before = operation_record_rows(&enabled.service);

    enabled.service.set_owned_path_hook_for_testing(
        relative.clone(),
        OwnedPathBoundary::Replace,
        move || {
            fs::remove_dir(&parent).unwrap();
            symlink(outside.path(), &parent).unwrap();
        },
    );
    let error = document_error(enabled.service.save_document(document_request(
        &fixture.root,
        ContextIntent::Create,
        None,
        relative.to_str().unwrap(),
        "Created",
        "created body\n",
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_ne!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert!(!outside_file.exists());
    assert_eq!(support::head_commit(&repository), head_before);
    assert_eq!(support::index_bytes(&repository), index_before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);
}

#[cfg(unix)]
#[test]
fn owned_source_swap_at_remove_boundary_rejects_document_move_without_destination_or_commit() {
    use std::os::unix::fs::symlink;

    let _hook_guard = owned_path_hook_test_lock();
    let fixture = support::born_repository();
    let source_relative = std::path::PathBuf::from("docs/nofollow-source.md");
    let destination_relative = std::path::PathBuf::from("docs/nofollow-destination.md");
    commit_source(
        &fixture,
        source_relative.to_str().unwrap(),
        &support::document_source(),
    );
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                support::document_id(),
                ContextIntent::Edit,
            ))
            .unwrap(),
    );
    let source = context.worktree.join(&source_relative);
    let destination = context.worktree.join(&destination_relative);
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside.md");
    let outside_bytes = b"outside source must not be removed or read\n";
    fs::write(&outside_file, outside_bytes).unwrap();
    let repository = Repository::open(&context.worktree).unwrap();
    let head_before = support::head_commit(&repository);
    let index_before = support::index_bytes(&repository);
    let records_before = operation_record_rows(&enabled.service);
    let hook_outside_file = outside_file.clone();

    enabled.service.set_owned_path_hook_for_testing(
        source_relative.clone(),
        OwnedPathBoundary::Remove,
        move || {
            fs::remove_file(&source).unwrap();
            symlink(&hook_outside_file, &source).unwrap();
        },
    );
    let error = document_error(enabled.service.save_document(document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some(source_relative.to_str().unwrap()),
        destination_relative.to_str().unwrap(),
        "Moved",
        "moved body\n",
    )));

    assert_eq!(error.kind, RepositoryErrorKind::InvalidPath);
    assert_ne!(error.kind, RepositoryErrorKind::MismatchedAuthoringContext);
    assert!(!format!("{error:?}").contains("outside source"));
    assert_eq!(fs::read(&outside_file).unwrap(), outside_bytes);
    assert!(!destination.exists());
    assert_eq!(support::head_commit(&repository), head_before);
    assert_eq!(support::index_bytes(&repository), index_before);
    assert_eq!(operation_record_rows(&enabled.service), records_before);
}

#[test]
fn document_same_id_different_move_source_is_an_operation_mismatch() {
    let fixture = support::born_repository();
    commit_source(&fixture, "docs/one.md", &support::document_source());
    commit_source(&fixture, "docs/two.md", &support::document_source());
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::operation_id();
    let request = |source| {
        document_request_with_operation_id(
            &fixture.root,
            ContextIntent::Edit,
            Some(source),
            "docs/destination.md",
            "Moved",
            "Body\n",
            operation_id,
        )
    };

    let _ = enabled.service.save_document(request("docs/one.md"));
    let error = document_error(enabled.service.save_document(request("docs/two.md")));
    assert_eq!(error.kind, RepositoryErrorKind::OperationMismatch);
}

#[test]
fn externally_populated_missing_document_destination_is_redacted_external_change() {
    assert_populated_missing_document_destination(support::born_repository());
}

#[cfg(unix)]
#[test]
fn symlinked_parent_populated_missing_document_destination_is_redacted_external_change() {
    let (_alias_directory, fixture) = repository_with_symlinked_parent();
    assert_populated_missing_document_destination(fixture);
}

fn assert_populated_missing_document_destination(fixture: support::TestRepository) {
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let item_id = support::document_id();
    let context = context_from(
        enabled
            .service
            .prepare_context(target(
                &fixture.root,
                AuthoringKind::Document,
                item_id.clone(),
                ContextIntent::Create,
            ))
            .unwrap(),
    );
    let destination = context.worktree.join("docs/new.md");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(
        &destination,
        canonical_document("External", "secret external body\n"),
    )
    .unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);

    let mut request = document_request(
        &fixture.root,
        ContextIntent::Create,
        None,
        "docs/new.md",
        "Requested",
        "secret draft body\n",
    );
    request.expected_destination = manyhands::repository::ExpectedPathObservation::Missing;
    let error = document_error(enabled.service.save_document(request));

    assert_eq!(error.kind, RepositoryErrorKind::ExternalChange);
    let diagnostic = error.external_change().expect("external-change diagnostic");
    assert_eq!(diagnostic.root, fixture.root.canonicalize().unwrap());
    assert_eq!(diagnostic.item_id, item_id);
    assert_eq!(diagnostic.context, context.worktree);
    assert_eq!(diagnostic.path, std::path::PathBuf::from("docs/new.md"));
    assert_eq!(
        diagnostic.expectation,
        manyhands::repository::ExternalChangeExpectation::Missing
    );
    assert!(!format!("{error:?}").contains("secret"));
    assert!(!format!("{error}").contains("secret"));
    assert_eq!(support::repository_and_worktree_snapshot(&fixture), before);
}

#[test]
fn rejected_authoring_request_does_not_strand_a_pending_lifecycle_record() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);

    let rejected = document_error(enabled.service.save_document(document_request(
        &fixture.root,
        ContextIntent::Edit,
        Some("docs/missing.md"),
        "docs/new.md",
        "Rejected",
        "Body\n",
    )));
    assert_eq!(rejected.kind, RepositoryErrorKind::MissingAuthoringTarget);

    assert!(matches!(
        enabled.service.save_document(document_request(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/new.md",
            "Accepted",
            "Body\n",
        )),
        Ok(SaveOutcome::Saved { .. })
    ));
}

#[test]
fn recovery_document_write_transition_persistence_failure_blocks_other_ids_until_same_id_replays() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::new_operation_id();
    let request = document_request_with_operation_id(
        &fixture.root,
        ContextIntent::Create,
        None,
        "docs/new.md",
        "Title",
        "Body\n",
        operation_id,
    );
    let failing = support::FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        document_error(failing.save_document(request.clone())).kind,
        RepositoryErrorKind::Sqlite
    );
    let fresh = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    assert_eq!(
        document_error(fresh.save_document(document_request(
            &fixture.root,
            ContextIntent::Create,
            None,
            "docs/other.md",
            "Other",
            "Body\n",
        )))
        .kind,
        RepositoryErrorKind::RecoveryRequired
    );

    let (_, checkpoint) = saved_checkpoint(fresh.save_document(request).unwrap());
    assert_ne!(checkpoint, git2::Oid::zero());
    assert!(fresh.recovery_inspection(&fixture.root).unwrap().is_empty());
}

#[test]
fn context_git_failure_after_branch_creation_stays_pending_until_exact_retry() {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    clean_configuration_index(&fixture);
    let operation_id = support::new_operation_id();
    let request = || {
        target_with_operation_id(
            &fixture.root,
            AuthoringKind::Document,
            support::document_id(),
            ContextIntent::Create,
            operation_id,
        )
    };
    let failing = support::FailOnce::at(FailurePoint::AfterContextBranchBeforeWorktreeGitFailure)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        context_error(failing.prepare_context(request())).kind,
        RepositoryErrorKind::Git
    );
    let fresh = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    assert_eq!(
        context_error(fresh.prepare_context(target(
            &fixture.root,
            AuthoringKind::Ticket,
            support::ticket_id(),
            ContextIntent::Create,
        )))
        .kind,
        RepositoryErrorKind::RecoveryRequired,
    );
    assert!(matches!(
        fresh.prepare_context(request()),
        Ok(ContextProvisionOutcome::Created(_))
    ));
}
