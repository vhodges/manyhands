use super::*;
use std::{collections::BTreeMap, fs, path::Path};

const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const DOC: &str = "docs/document.md";

struct Fixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    repo: git2::Repository,
    context: git2::Repository,
    base: Oid,
}

fn write(repo: &git2::Repository, path: &str, bytes: impl AsRef<[u8]>) {
    let path = repo.workdir().unwrap().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn commit(repo: &git2::Repository) -> Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}
fn checkout(repo: &git2::Repository, oid: Oid) {
    repo.reset(
        &repo.find_object(oid, None).unwrap(),
        git2::ResetType::Hard,
        None,
    )
    .unwrap();
}
fn merge(repo: &git2::Repository, first: Oid, second: Oid, subject: &str) -> Oid {
    let a = repo.find_commit(first).unwrap();
    let b = repo.find_commit(second).unwrap();
    let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(
        None,
        &signature,
        &signature,
        subject,
        &b.tree().unwrap(),
        &[&a, &b],
    )
    .unwrap()
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
                    coordination::snapshot_file_hash(base, &entry.path()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
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
        service
            .save_document(SaveDocumentRequest {
                target: AuthoringTarget {
                    root: root.path().into(),
                    kind: AuthoringKind::Document,
                    item_id: ID.parse().unwrap(),
                    intent: ContextIntent::Create,
                    operation_id: OperationId::new(),
                },
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
            rusqlite::Connection::open(f.data.path().join(REGISTRY_FILE))
                .unwrap()
                .execute("UPDATE remote_polling_state SET history_unknown=0", [])
                .unwrap();
            f.tracking(base);
        }
        // Include the already-created coordination file in byte-preservation images.
        drop(repository_lease(&f.repo, f.root.path(), RepositoryOperation::Read).unwrap());
        f
    }
    fn tracking(&self, oid: Oid) {
        self.repo
            .reference("refs/remotes/origin/main", oid, true, "fixture")
            .unwrap();
    }
    fn ready(&self) -> DocumentPromotionPreview {
        match self
            .service
            .preview_document_promotion(PreviewDocumentPromotionRequest {
                root: self.root.path().into(),
                item_id: ID.parse().unwrap(),
            })
            .unwrap()
        {
            DocumentPromotionPreflight::Ready(p) => p,
            other => panic!("not ready: {other:?}"),
        }
    }
    fn request(&self) -> IntegrationRequest {
        IntegrationRequest::from_preview(&self.ready())
    }
    fn run(
        &self,
        request: &IntegrationRequest,
        identity: Option<&ConfirmedCommitIdentity>,
    ) -> Result<Integrated, DocumentPromotionError> {
        integrate(&self.service, self.root.path(), request, identity)
    }
    fn primary(&self) -> Oid {
        self.repo.refname_to_id("refs/heads/main").unwrap()
    }
    fn context_oid(&self) -> Oid {
        self.context.head().unwrap().target().unwrap()
    }
    fn missing_identity(&self) {
        self.repo
            .config()
            .unwrap()
            .set_str("user.name", "")
            .unwrap();
        self.repo
            .config()
            .unwrap()
            .set_str("user.email", "")
            .unwrap();
    }
    fn confirmation(&self, request: &IntegrationRequest) -> ConfirmedCommitIdentity {
        let expected_configuration = match self.run(request, None) {
            Err(DocumentPromotionError::IdentityRequired {
                expected_configuration,
            }) => expected_configuration,
            other => panic!("expected identity boundary: {other:?}"),
        };
        ConfirmedCommitIdentity {
            confirmation_id: OperationId::new(),
            identity: CommitIdentity {
                name: "Confirmed".into(),
                email: "confirmed@example.invalid".into(),
            },
            expected_configuration,
        }
    }
    fn clean(&self) {
        remote::sync::require_clean_target(&self.repo, &SynchronizationTarget::Primary).unwrap();
        remote::sync::require_clean_target(
            &self.context,
            &SynchronizationTarget::Context {
                kind: AuthoringKind::Document,
                item_id: ID.parse().unwrap(),
            },
        )
        .unwrap();
    }
}
fn merged(result: Integrated) -> Oid {
    match result {
        Integrated::Merged { oid } => oid,
        other => panic!("not merged: {other:?}"),
    }
}

#[test]
fn poll_advance_requires_resynchronization_without_writing_objects_or_any_git_bytes() {
    let f = Fixture::new(true);
    let request = f.request();
    // A separate primary line, fetched by a poll, after context synchronization.
    write(&f.repo, "primary-only", "poll advance\n");
    let advanced = commit(&f.repo);
    checkout(&f.repo, f.base);
    f.tracking(advanced);
    assert_eq!(f.ready().expectation(), request.expected);
    let before = files(f.root.path());
    assert!(matches!(
        f.run(&request, None),
        Err(DocumentPromotionError::PrimaryAdvanced)
    ));
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn digest_is_recomputed_under_lease_before_any_write_including_identity_config() {
    let f = Fixture::new(true);
    f.missing_identity();
    let mut request = f.request();
    let confirmation = f.confirmation(&request);
    write(&f.context, "additional", "new context content\n");
    commit(&f.context);
    request.context.oid = f.context_oid(); // synchronized evidence, old consent
    let before = files(f.root.path());
    assert!(matches!(
        f.run(&request, Some(&confirmation)),
        Err(DocumentPromotionError::ReconfirmationRequired)
    ));
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn remote_merge_has_exact_subject_parent_order_and_context_tree_from_behind_primary() {
    let f = Fixture::new(true);
    let context = f.context_oid();
    let old = f.repo.find_commit(f.base).unwrap().parent_id(0).unwrap();
    checkout(&f.repo, old);
    let request = f.request();
    let context_before = files(f.context.workdir().unwrap());
    let oid = merged(f.run(&request, None).unwrap());
    let commit = f.repo.find_commit(oid).unwrap();
    assert_eq!(
        commit.message(),
        Some(format!("Promote document {ID}").as_str())
    );
    assert_eq!(
        commit.parent_ids().collect::<Vec<_>>(),
        vec![f.base, context]
    );
    assert_eq!(
        commit.tree_id(),
        f.repo.find_commit(context).unwrap().tree_id()
    );
    assert_eq!(f.primary(), oid);
    assert_eq!(files(f.context.workdir().unwrap()), context_before);
    assert_eq!(
        fs::read(f.root.path().join(DOC)).unwrap(),
        fs::read(f.context.workdir().unwrap().join(DOC)).unwrap()
    );
    f.clean();
}

#[test]
fn local_divergence_imports_prepared_tree_and_valid_document() {
    let f = Fixture::new(false);
    let original = fs::read_to_string(f.root.path().join(DOC)).unwrap();
    write(
        &f.repo,
        DOC,
        original.replace("title: Document", "title: Primary title"),
    );
    let base = commit(&f.repo);
    let expected = fs::read_to_string(f.root.path().join(DOC))
        .unwrap()
        .replace("original body", "context body");
    let blob = Oid::hash_object(git2::ObjectType::Blob, expected.as_bytes()).unwrap();
    assert!(!f.repo.odb().unwrap().exists(blob));
    let oid = merged(f.run(&f.request(), None).unwrap());
    let commit = f.repo.find_commit(oid).unwrap();
    assert_eq!(
        commit.parent_ids().collect::<Vec<_>>(),
        vec![base, f.context_oid()]
    );
    assert_eq!(
        commit
            .tree()
            .unwrap()
            .get_path(Path::new(DOC))
            .unwrap()
            .id(),
        blob
    );
    assert_eq!(
        fs::read_to_string(f.root.path().join(DOC)).unwrap(),
        expected
    );
    assert!(
        matches!(crate::canonical::parse_item(Path::new(DOC), &expected), Ok(crate::canonical::CanonicalItem::Document(doc)) if doc.id.to_string() == ID)
    );
    f.clean();
}

#[test]
fn contained_records_base_without_identity_commit_or_primary_update() {
    let f = Fixture::new(true);
    f.tracking(f.context_oid()); // base contains C, local primary remains behind base
    f.missing_identity();
    let request = f.request();
    let before = files(f.root.path());
    assert_eq!(
        f.run(&request, None).unwrap(),
        Integrated::AlreadyContained {
            oid: f.context_oid()
        }
    );
    assert_eq!(files(f.root.path()), before);
    assert_eq!(f.primary(), f.base);
}

#[test]
fn missing_identity_writes_nothing_then_confirmation_persists_before_commit() {
    let f = Fixture::new(false);
    f.missing_identity();
    f.repo
        .config()
        .unwrap()
        .set_str("manyhands.initials", "kept")
        .unwrap();
    f.repo
        .config()
        .unwrap()
        .set_str("unrelated.setting", "kept too")
        .unwrap();
    let request = f.request();
    let before = files(f.root.path());
    let confirmation = f.confirmation(&request);
    assert_eq!(files(f.root.path()), before);
    let prepared =
        prepare_integration(&f.service, f.root.path(), &request, Some(&confirmation)).unwrap();
    // Persistence has already occurred at the lease-held preparation boundary.
    let config = git2::Repository::open(f.root.path())
        .unwrap()
        .config()
        .unwrap();
    assert_eq!(config.get_string("user.name").unwrap(), "Confirmed");
    assert_eq!(
        config.get_string("user.email").unwrap(),
        "confirmed@example.invalid"
    );
    assert_eq!(config.get_string("manyhands.initials").unwrap(), "kept");
    assert_eq!(config.get_string("unrelated.setting").unwrap(), "kept too");
    assert_eq!(f.primary(), f.base);
    let oid = merged(prepared.write_commit().unwrap().apply().unwrap());
    let commit = f.repo.find_commit(oid).unwrap();
    assert_eq!(commit.author().name(), Some("Confirmed"));
    assert_eq!(
        commit.committer().email(),
        Some("confirmed@example.invalid")
    );
    f.clean();
}

#[test]
fn proposed_identity_relative_include_override_is_refused_before_install() {
    let f = Fixture::new(false);
    f.missing_identity();
    let config = f.repo.path().join("config");
    let mut source = fs::read(&config).unwrap();
    source.extend_from_slice(b"\n[include]\npath = identity.inc\n");
    fs::write(&config, source).unwrap();
    fs::write(f.repo.path().join("identity.inc"), b"[user]\nemail =\n").unwrap();
    let request = f.request();
    let confirmation = f.confirmation(&request);
    let before = files(f.root.path());
    let result = f.run(&request, Some(&confirmation));
    assert!(
        result.is_err(),
        "proposed local identity is overridden by the real relative include: {result:?}"
    );
    assert_eq!(files(f.root.path()), before);
    assert_eq!(f.primary(), f.base);
}

#[test]
fn proposed_identity_conditional_includes_use_original_repository_context() {
    for condition in [
        "onbranch:main".to_owned(),
        "onbranch:other".to_owned(),
        "gitdir".to_owned(),
    ] {
        let f = Fixture::new(false);
        f.missing_identity();
        let active = condition != "onbranch:other";
        let condition = if condition == "gitdir" {
            format!(
                "gitdir:{}",
                f.repo
                    .path()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/")
                    .trim_end_matches('/')
            )
        } else {
            condition
        };
        let config = f.repo.path().join("config");
        let mut source = fs::read(&config).unwrap();
        source.extend_from_slice(
            format!("\n[includeIf \"{condition}\"]\npath = identity.inc\n").as_bytes(),
        );
        fs::write(&config, source).unwrap();
        // Nest another relative include to exercise the complete included order.
        fs::write(
            f.repo.path().join("identity.inc"),
            b"[include]\npath = nested.inc\n[quality]\nactive = true\n",
        )
        .unwrap();
        fs::write(f.repo.path().join("nested.inc"), b"[user]\nemail =\n").unwrap();
        let effective = git2::Repository::open(f.root.path())
            .unwrap()
            .config()
            .unwrap();
        assert_eq!(
            effective.get_bool("quality.active").unwrap_or(false),
            active
        );
        let request = f.request();
        let confirmation = f.confirmation(&request);
        let before = files(f.root.path());
        let result = f.run(&request, Some(&confirmation));
        if active {
            assert!(
                result.is_err(),
                "active conditional include would override the proposed identity: {result:?}"
            );
            assert_eq!(files(f.root.path()), before);
        } else {
            merged(result.unwrap());
            let effective = git2::Repository::open(f.root.path())
                .unwrap()
                .config()
                .unwrap();
            assert_eq!(
                effective.get_string("user.name").unwrap(),
                confirmation.identity.name
            );
            assert_eq!(
                effective.get_string("user.email").unwrap(),
                confirmation.identity.email
            );
        }
    }
}

#[test]
fn proposed_identity_preserves_includes_that_do_not_override_the_result() {
    for identity_before_include in [false, true] {
        let f = Fixture::new(false);
        let mut config = f.repo.config().unwrap();
        config.remove("user.name").unwrap();
        config.remove("user.email").unwrap();
        drop(config);
        let config = f.repo.path().join("config");
        // Leave exactly one user section, at the intended side of the include;
        // libgit2 otherwise writes into the first (now empty) fixture section.
        let mut source = fs::read_to_string(&config)
            .unwrap()
            .lines()
            .filter(|line| line.trim() != "[user]")
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes();
        source.push(b'\n');
        let user = b"\n[user]\nname =\nemail =\n";
        if identity_before_include {
            source.extend_from_slice(user);
        }
        source.extend_from_slice(b"\n[includeIf \"onbranch:main\"]\npath = identity.inc\n[include]\npath = unrelated.inc\n");
        if !identity_before_include {
            source.extend_from_slice(user);
        }
        fs::write(&config, source).unwrap();
        fs::write(f.repo.path().join("identity.inc"), if identity_before_include {
            b"[quality]\nactive = true\n".as_slice()
        } else {
            b"[quality]\nactive = true\n[user]\nname = Included\nemail = included@example.invalid\n".as_slice()
        }).unwrap();
        let unrelated = b"[quality]\nsetting = preserved\n";
        fs::write(f.repo.path().join("unrelated.inc"), unrelated).unwrap();
        let request = f.request();
        let confirmation = f.confirmation(&request);
        let oid = merged(f.run(&request, Some(&confirmation)).unwrap());
        let effective = git2::Repository::open(f.root.path())
            .unwrap()
            .config()
            .unwrap();
        assert_eq!(
            effective.get_string("user.name").unwrap(),
            confirmation.identity.name
        );
        assert_eq!(
            effective.get_string("user.email").unwrap(),
            confirmation.identity.email
        );
        assert_eq!(
            effective.get_string("quality.setting").unwrap(),
            "preserved"
        );
        assert!(effective.get_bool("quality.active").unwrap());
        assert_eq!(
            fs::read(f.repo.path().join("unrelated.inc")).unwrap(),
            unrelated
        );
        assert_eq!(
            f.repo.find_commit(oid).unwrap().committer().email(),
            Some(confirmation.identity.email.as_str())
        );
    }
}

#[test]
fn stale_confirmation_is_refused_without_writes() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let confirmation = f.confirmation(&request);
    f.repo
        .config()
        .unwrap()
        .set_str("user.name", "partial change")
        .unwrap();
    let before = files(f.root.path());
    assert!(matches!(
        f.run(&request, Some(&confirmation)),
        Err(DocumentPromotionError::ExternalChange)
    ));
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn empty_or_nul_confirmation_is_refused_without_writes() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let mut confirmation = f.confirmation(&request);
    for (name, email) in [
        ("", "email"),
        ("name", ""),
        ("bad\0name", "email"),
        ("name", "bad\0email"),
    ] {
        confirmation.identity = CommitIdentity {
            name: name.into(),
            email: email.into(),
        };
        let before = files(f.root.path());
        assert!(matches!(
            f.run(&request, Some(&confirmation)),
            Err(DocumentPromotionError::ExternalChange)
        ));
        assert_eq!(files(f.root.path()), before);
    }
}

#[test]
fn complete_effective_identity_wins_even_over_invalid_stale_confirmation() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let mut confirmation = f.confirmation(&request);
    confirmation.identity.name.clear();
    f.repo
        .config()
        .unwrap()
        .set_str("user.name", "Existing")
        .unwrap();
    f.repo
        .config()
        .unwrap()
        .set_str("user.email", "existing@example.invalid")
        .unwrap();
    let config = fs::read(f.repo.path().join("config")).unwrap();
    let oid = merged(f.run(&request, Some(&confirmation)).unwrap());
    assert_eq!(
        f.repo.find_commit(oid).unwrap().committer().name(),
        Some("Existing")
    );
    assert_eq!(fs::read(f.repo.path().join("config")).unwrap(), config);
}

#[test]
fn confirmed_identity_retains_sync_validation_without_bridge_email_policy() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let mut confirmation = f.confirmation(&request);
    confirmation.identity.email = "no-at-sign".into();
    let oid = merged(f.run(&request, Some(&confirmation)).unwrap());
    assert_eq!(
        f.repo.find_commit(oid).unwrap().committer().email(),
        Some("no-at-sign")
    );
    assert_eq!(
        f.repo.config().unwrap().get_string("user.email").unwrap(),
        "no-at-sign"
    );
}

#[test]
fn atomic_identity_failure_leaves_no_half_write_and_retry_works() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let confirmation = f.confirmation(&request);
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterLocalIdentityName,
    )
    .unwrap();
    let before = files(f.root.path());
    assert!(matches!(
        integrate(&failing, f.root.path(), &request, Some(&confirmation)),
        Err(DocumentPromotionError::Repository(_))
    ));
    assert_eq!(files(f.root.path()), before);
    merged(f.run(&request, Some(&confirmation)).unwrap());
}

#[test]
fn foreign_config_lock_is_preserved_and_nothing_is_written() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let confirmation = f.confirmation(&request);
    fs::write(f.repo.path().join("config.lock"), b"foreign").unwrap();
    let before = files(f.root.path());
    assert!(matches!(
        f.run(&request, Some(&confirmation)),
        Err(DocumentPromotionError::Repository(_))
    ));
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn noncooperating_config_change_under_lock_is_not_overwritten() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let confirmation = f.confirmation(&request);
    let config = f.repo.path().join("config");
    let mut changed = fs::read(&config).unwrap();
    changed.extend_from_slice(b"\n[unrelated]\nsetting = later\n");
    let later = changed.clone();
    f.service
        .set_local_identity_lock_hook_for_testing(move || fs::write(config, changed).unwrap());
    assert!(matches!(
        f.run(&request, Some(&confirmation)),
        Err(DocumentPromotionError::ExternalChange)
    ));
    assert_eq!(fs::read(f.repo.path().join("config")).unwrap(), later);
    assert_eq!(f.primary(), f.base);
    assert!(!f.repo.path().join("config.lock").exists());
}

#[test]
fn confirmation_and_effective_identity_are_reobserved_under_config_lock_before_install() {
    for complete in [false, true] {
        let f = Fixture::new(false);
        f.missing_identity();
        let request = f.request();
        let confirmation = f.confirmation(&request);
        let config = f.repo.path().join("config");
        let mut changed = fs::read(&config).unwrap();
        changed.extend_from_slice(b"\n[user]\nname = Concurrent\n");
        if complete {
            changed.extend_from_slice(b"email = concurrent@example.invalid\n");
        }
        let expected = changed.clone();
        f.service
            .set_local_identity_lock_hook_for_testing(move || fs::write(config, changed).unwrap());
        let result = f.run(&request, Some(&confirmation));
        if complete {
            let oid = merged(result.unwrap());
            assert_eq!(
                f.repo.find_commit(oid).unwrap().committer().name(),
                Some("Concurrent")
            );
        } else {
            assert!(matches!(
                result,
                Err(DocumentPromotionError::ExternalChange)
            ));
            assert_eq!(f.primary(), f.base);
        }
        assert_eq!(fs::read(f.repo.path().join("config")).unwrap(), expected);
        assert!(!f.repo.path().join("config.lock").exists());
    }
}

#[test]
fn primary_changed_since_observation_is_external_change_without_writes() {
    let f = Fixture::new(true);
    let request = f.request();
    write(&f.repo, "new-primary", "advance\n");
    commit(&f.repo);
    f.tracking(f.primary());
    let before = files(f.root.path());
    assert!(matches!(
        f.run(&request, None),
        Err(DocumentPromotionError::ExternalChange)
    ));
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn changed_context_since_synchronized_evidence_is_external_change() {
    let f = Fixture::new(true);
    let request = f.request();
    commit(&f.context);
    let before = files(f.root.path());
    assert!(matches!(
        f.run(&request, None),
        Err(DocumentPromotionError::ExternalChange)
    ));
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn ignored_checkout_collision_refuses_without_moving_or_writing_objects() {
    let f = Fixture::new(false);
    write(&f.context, "collision", "tracked on context\n");
    commit(&f.context);
    let exclude = f.repo.path().join("info/exclude");
    let mut ignored = fs::read(&exclude).unwrap();
    ignored.extend_from_slice(b"\ncollision\n");
    fs::write(exclude, ignored).unwrap();
    write(&f.repo, "collision", "ignored user bytes\n");
    let request = f.request();
    let before = files(f.root.path());
    assert!(f.run(&request, None).is_err());
    let after = files(f.root.path());
    let changed = after
        .iter()
        .filter(|(path, bytes)| before.get(*path) != Some(*bytes))
        .map(|(path, _)| path)
        .collect::<Vec<_>>();
    assert!(changed.is_empty(), "changed paths: {changed:?}");
    assert_eq!(files(f.root.path()), before);
    assert_eq!(f.primary(), f.base);
}

#[test]
fn persistence_seam_holds_one_lease_and_creates_no_commit_before_integrating_evidence() {
    let f = Fixture::new(false);
    let request = f.request();
    let before = files(f.root.path());
    let prepared = prepare_integration(&f.service, f.root.path(), &request, None).unwrap();
    assert_eq!(prepared.evidence().base, f.base);
    assert_eq!(prepared.evidence().context.oid, f.context_oid());
    assert_eq!(prepared.evidence().kind, PlannedIntegration::MergeCommit);
    assert_eq!(files(f.root.path()), before);
    // The later integrating checkpoint can enter a transaction without a
    // retained observation/read transaction or a recursively held cache guard.
    remote::state::with_transaction(&f.service, f.root.path(), |tx, _| {
        tx.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
            .map_err(RepositoryError::sqlite)
    })
    .unwrap();
    assert!(
        matches!(repository_lease(&f.repo, f.root.path(), RepositoryOperation::Read), Err(e) if e.kind == RepositoryErrorKind::RepositoryBusy)
    );
    let candidate = prepared.write_commit().unwrap();
    assert_eq!(f.primary(), f.base); // F3: commit exists, no primary ref effect yet
    merged(candidate.apply().unwrap()); // F4: caller can now persist integrated
}

#[test]
fn confirmed_config_is_kept_if_later_ref_application_fails() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let confirmation = f.confirmation(&request);
    let candidate = prepare_integration(&f.service, f.root.path(), &request, Some(&confirmation))
        .unwrap()
        .write_commit()
        .unwrap();
    fs::write(f.repo.path().join("refs/heads/main.lock"), b"foreign").unwrap();
    assert!(candidate.apply().is_err());
    assert_eq!(f.primary(), f.base);
    assert_eq!(
        f.repo.config().unwrap().get_string("user.name").unwrap(),
        "Confirmed"
    );
}

#[test]
fn blockers_are_rechecked_before_identity_persistence() {
    let f = Fixture::new(false);
    f.missing_identity();
    let request = f.request();
    let confirmation = f.confirmation(&request);
    write(&f.repo, "ordinary.txt", "dirty\n");
    let before = files(f.root.path());
    assert!(
        matches!(f.run(&request, Some(&confirmation)), Err(DocumentPromotionError::Blocked(blockers)) if blockers == vec![PromotionBlocker::PrimaryDirty])
    );
    assert_eq!(files(f.root.path()), before);
}

#[test]
fn reconciliation_recognizes_tip_and_first_parent_reachable_merge() {
    let f = Fixture::new(false);
    let request = f.request();
    let promoted = merge(
        &f.repo,
        f.base,
        f.context_oid(),
        &format!("Promote document {ID}"),
    );
    checkout(&f.repo, promoted);
    assert_eq!(
        reconcile_integration(&f.repo, f.base, &request.context),
        IntegrationState::Integrated { oid: promoted }
    );
    write(&f.repo, "later", "fast forward\n");
    let later = commit(&f.repo);
    assert_eq!(
        reconcile_integration(&f.repo, f.base, &request.context),
        IntegrationState::Integrated { oid: promoted }
    );
    let sync = merge(&f.repo, later, f.context_oid(), "Merge remote primary");
    checkout(&f.repo, sync);
    assert_eq!(
        reconcile_integration(&f.repo, f.base, &request.context),
        IntegrationState::Integrated { oid: promoted }
    );
}

#[test]
fn reconciliation_ignores_dangling_candidate_and_accepts_poll_fast_forward_below_base() {
    let f = Fixture::new(true);
    let request = f.request();
    merge(
        &f.repo,
        f.base,
        f.context_oid(),
        &format!("Promote document {ID}"),
    );
    let old = f.repo.find_commit(f.base).unwrap().parent_id(0).unwrap();
    checkout(&f.repo, old);
    assert_eq!(
        reconcile_integration(&f.repo, f.base, &request.context),
        IntegrationState::NotIntegrated
    );
    checkout(&f.repo, f.base); // poll fast-forward, still no promotion merge
    assert_eq!(
        reconcile_integration(&f.repo, f.base, &request.context),
        IntegrationState::NotIntegrated
    );
}

#[test]
fn reconciliation_requires_exact_subject_and_ordered_two_parents() {
    let f = Fixture::new(false);
    let request = f.request();
    for (a, b, subject) in [
        (f.base, f.context_oid(), "other subject".to_owned()),
        (f.context_oid(), f.base, format!("Promote document {ID}")),
        (
            f.base,
            f.context_oid(),
            format!("Promote document {}", ItemId::generate()),
        ),
    ] {
        let oid = merge(&f.repo, a, b, &subject);
        checkout(&f.repo, oid);
        assert_eq!(
            reconcile_integration(&f.repo, f.base, &request.context),
            IntegrationState::RecoveryRequired
        );
    }
    checkout(&f.repo, f.context_oid()); // ordinary FF is not the required merge
    assert_eq!(
        reconcile_integration(&f.repo, f.base, &request.context),
        IntegrationState::RecoveryRequired
    );
}

#[test]
fn reconciliation_does_not_search_second_parent_and_uncertainty_is_a_pending_state() {
    let f = Fixture::new(false);
    let request = f.request();
    let promoted = merge(
        &f.repo,
        f.base,
        f.context_oid(),
        &format!("Promote document {ID}"),
    );
    let other = merge(&f.repo, f.context_oid(), promoted, "unrelated merge");
    checkout(&f.repo, other);
    let pending: IntegrationState = reconcile_integration(&f.repo, f.base, &request.context);
    assert_eq!(pending, IntegrationState::RecoveryRequired); // category, never operation Err
    f.repo
        .find_reference("refs/heads/main")
        .unwrap()
        .delete()
        .unwrap();
    assert_eq!(
        reconcile_integration(&f.repo, f.base, &request.context),
        IntegrationState::RecoveryRequired
    );
}

#[test]
fn integration_errors_redact_repository_paths_and_messages() {
    let error = DocumentPromotionError::Repository(RepositoryError::new(
        RepositoryOperation::Read,
        Some("private-path-sentinel".into()),
        RepositoryErrorKind::Git,
        "private-content-sentinel",
    ));
    let debug = format!("{error:?}");
    assert!(!debug.contains("private-"));
    assert!(debug.contains("Git"));
}

#[test]
fn ref_error_after_candidate_becomes_reachable_returns_integrated_not_error() {
    let f = Fixture::new(false);
    let request = f.request();
    let candidate = prepare_integration(&f.service, f.root.path(), &request, None)
        .unwrap()
        .write_commit()
        .unwrap();
    let oid = candidate.oid;
    let reference = f.repo.path().join("refs/heads/main");
    let _hook = remote::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforeLocalMutation {
            // Deterministic noncooperating ref effect after checkout, while the
            // helper owns the ref lock: its returned error cannot be trusted.
            fs::write(&reference, format!("{oid}\n")).unwrap();
        }
    });
    assert_eq!(candidate.apply().unwrap(), Integrated::Merged { oid });
    assert_eq!(f.primary(), oid);
    f.clean();
}

#[test]
fn checkout_effect_without_ref_success_returns_pending_recovery_not_error() {
    let f = Fixture::new(false);
    let request = f.request();
    let candidate = prepare_integration(&f.service, f.root.path(), &request, None)
        .unwrap()
        .write_commit()
        .unwrap();
    let head = f.repo.path().join("HEAD");
    let old = f.base;
    let _hook = remote::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforeLocalMutation {
            fs::write(&head, format!("{old}\n")).unwrap();
        }
    });
    assert_eq!(candidate.apply().unwrap(), Integrated::RecoveryRequired);
    assert_eq!(f.primary(), f.base);
    assert_eq!(
        fs::read(f.root.path().join(DOC)).unwrap(),
        fs::read(f.context.workdir().unwrap().join(DOC)).unwrap()
    );
}

#[test]
fn remote_local_primary_ahead_of_base_is_blocked_under_lease_without_writes() {
    let f = Fixture::new(true);
    let mut request = f.request();
    write(&f.repo, "unpublished", "local only\n");
    request.primary_oid = commit(&f.repo);
    let before = files(f.root.path());
    assert!(
        matches!(f.run(&request, None), Err(DocumentPromotionError::Blocked(blockers)) if blockers == vec![PromotionBlocker::PrimaryAheadOfRemote])
    );
    assert_eq!(files(f.root.path()), before);
}
