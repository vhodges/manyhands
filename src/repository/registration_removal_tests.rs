use super::*;
use crate::repository::{CommitIdentity, EnableRepositoryRequest, LeaseKind, REGISTRY_FILE};

thread_local! {
    static BEFORE_CACHE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}

pub(super) fn before_cache() {
    if let Some(hook) = BEFORE_CACHE.with(|slot| slot.borrow_mut().take()) {
        hook();
    }
}

struct Fixture {
    service: RepositoryService,
    data: tempfile::TempDir,
    root: PathBuf,
    _directory: tempfile::TempDir,
    displaced: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self::named(None)
    }

    fn named(name: Option<&str>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = name.map_or_else(
            || directory.path().to_owned(),
            |name| directory.path().join(name),
        );
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let data = tempfile::tempdir().unwrap();
        let service = RepositoryService::open_at(data.path()).unwrap();
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        drop(Repository::init_opts(&root, &options).unwrap());
        service
            .enable(EnableRepositoryRequest {
                root: root.clone(),
                primary_branch: "main".into(),
                identity: Some(CommitIdentity {
                    name: "Removal Test".into(),
                    email: "removal@example.invalid".into(),
                }),
                operation_id: OperationId::new(),
            })
            .unwrap();
        Self {
            service,
            data,
            root,
            _directory: directory,
            displaced: tempfile::tempdir().unwrap(),
        }
    }

    fn db(&self) -> Connection {
        open_registry(&self.data.path().join(REGISTRY_FILE), &mut |_| {}).unwrap()
    }

    fn missing(&self) {
        fs::rename(&self.root, self.displaced.path().join("repository")).unwrap();
    }

    fn request(&self) -> RemoveRegistrationRequest {
        RemoveRegistrationRequest {
            root: self.root.clone(),
            operation_id: OperationId::new(),
        }
    }

    fn remove(&self) -> Result<RemoveRegistrationOutcome, RepositoryError> {
        self.service.remove_registration(self.request())
    }

    fn count(&self, table: &str) -> i64 {
        self.db()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    fn remote(
        &self,
        action: &str,
        phase: &str,
        index_pending: bool,
        reconciliation_required: bool,
    ) {
        let context = action == "synchronize_context";
        self.db().execute(
            "INSERT INTO remote_operation_records(repository_id,operation_ulid,configuration_generation,remote_name,primary_branch,primary_ref,primary_tracking_ref,action,priority,phase,created_at,updated_at,context_ref,context_tracking_ref,kind,item_id,local_branch,index_pending,reconciliation_required,authoritative_kind,authoritative_oid)
             SELECT id,?2,0,'origin','main','refs/heads/main','refs/remotes/origin/main',?3,?4,?5,1,1,?6,?7,?8,?9,?10,?11,?12,?13,?14 FROM repositories WHERE root_path=?1",
            params![self.root.to_str().unwrap(), OperationId::new().to_string(), action,
                if action == "poll" { "poll" } else { "manual" }, phase,
                context.then_some("refs/heads/item"), context.then_some("refs/remotes/origin/item"),
                context.then_some("ticket"), context.then_some("01K7F6H9J2N4Q6S8V0X2Z4B6DF"),
                (action != "poll").then_some("refs/heads/main"), index_pending, reconciliation_required,
                index_pending.then_some("published"), index_pending.then_some("1111111111111111111111111111111111111111")],
        ).unwrap();
    }
}

#[test]
fn pending_local_work_is_preserved_without_a_repository() {
    for (state, step) in [
        ("created", None),
        ("indexing", None),
        ("completed", Some("comment_registration_pending")),
    ] {
        let f = Fixture::new();
        f.db().execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at) SELECT id,root_path,?1,'save_ticket','item',?2,?3,0 FROM repositories", params![OperationId::new().to_string(), state, step]).unwrap();
        let count = f.count("operation_records");
        f.missing();
        assert_eq!(
            f.remove().unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "{state}/{step:?}"
        );
        assert_eq!(f.count("repositories"), 1);
        assert_eq!(f.count("operation_records"), count);
        let found: (String, Option<String>, i64) = f.db().query_row("SELECT state,completed_step,observed_at FROM operation_records WHERE action='save_ticket'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
        assert_eq!(found, (state.into(), step.map(str::to_owned), 0));
    }
}

#[test]
fn unfinished_remote_evidence_is_preserved_without_a_repository() {
    for (action, phase, index, reconcile) in [
        ("synchronize_context", "reserved", false, false),
        ("synchronize_primary", "fetch_observed", false, false),
        ("poll", "interrupted", false, false),
        ("synchronize_context", "failed", false, false),
        ("synchronize_primary", "completed", true, false),
        ("synchronize_primary", "cancelled", false, true),
    ] {
        let f = Fixture::new();
        f.remote(action, phase, index, reconcile);
        f.missing();
        assert_eq!(
            f.remove().unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "{action}/{phase}"
        );
        assert_eq!(f.count("repositories"), 1);
        assert_eq!(f.count("remote_operation_records"), 1);
        let found: (String, bool, bool) = f
            .db()
            .query_row(
                "SELECT phase,index_pending,reconciliation_required FROM remote_operation_records",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(found, (phase.into(), index, reconcile));
    }
}

#[test]
fn active_automatic_poll_is_asked_to_yield_without_deleting_its_registration() {
    let f = Fixture::new();
    f.remote("poll", "reserved", false, false);
    f.missing();
    assert_eq!(
        f.remove().unwrap_err().kind,
        RepositoryErrorKind::RepositoryBusy
    );
    assert_eq!(f.count("repositories"), 1);
    let yielded: bool = f
        .db()
        .query_row(
            "SELECT yield_requested FROM remote_operation_records",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(yielded);
}

#[test]
fn unfinished_parent_lifecycles_are_not_cascaded_away() {
    for table in ["document_promotions", "ticket_closures"] {
        let f = Fixture::new();
        let (column, value) = if table == "ticket_closures" {
            (",closure_kind", ",'record'")
        } else {
            ("", "")
        };
        f.db().execute(&format!("INSERT INTO {table}(repository_id,operation_id,item_id,expectation,step,updated_at{column}) SELECT id,?1,?2,zeroblob(32),'confirmed',1{value} FROM repositories"), params![OperationId::new().to_string(), "01K7F6H9J2N4Q6S8V0X2Z4B6DF"]).unwrap();
        f.missing();
        assert_eq!(
            f.remove().unwrap_err().kind,
            RepositoryErrorKind::RecoveryRequired,
            "{table}"
        );
        assert_eq!(f.count(table), 1);
        assert_eq!(f.count("repositories"), 1);
    }
}

#[test]
fn an_accepted_request_without_a_domain_row_still_blocks_registry_only_removal() {
    let f = Fixture::new();
    f.db().execute("INSERT INTO request_records(request_ulid,attempt,scope_key,command,target,intent_digest,state,accepted_at) VALUES(?1,1,?2,'ticket create','item',?3,'accepted',1)", params![OperationId::new().to_string(), f.root.to_str().unwrap(), "a".repeat(64)]).unwrap();
    f.missing();
    assert_eq!(
        f.remove().unwrap_err().kind,
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(f.count("request_records"), 1);
    assert_eq!(f.count("repositories"), 1);
}

#[test]
fn an_own_removal_can_retry_after_its_root_disappears() {
    let f = Fixture::new();
    let request = f.request();
    let failing = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::BeforeRegistrationRemovalTransaction,
    )
    .unwrap();
    assert_eq!(
        failing
            .remove_registration(request.clone())
            .unwrap_err()
            .kind,
        RepositoryErrorKind::InjectedFailure
    );
    f.missing();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(
        reopened.remove_registration(request).unwrap(),
        RemoveRegistrationOutcome::Removed
    );
    assert_eq!(f.count("repositories"), 0);
    assert_eq!(f.count("operation_records"), 0);
}

#[test]
fn changed_registration_is_refused_before_creating_a_removal_journal() {
    let f = Fixture::new();
    let target = f.service.removal_target(&f.root).unwrap();
    let rows = f.count("operation_records");
    f.db()
        .execute("UPDATE repositories SET enabled_at=enabled_at+1", [])
        .unwrap();
    f.missing();
    assert_eq!(
        f.service
            .remove_registration_checked(target, OperationId::new())
            .unwrap_err()
            .kind,
        RepositoryErrorKind::ExternalChange
    );
    assert_eq!(f.count("repositories"), 1);
    assert_eq!(f.count("operation_records"), rows);
}

#[test]
fn bootstrap_and_cache_contention_leave_registration_unchanged() {
    for kind in [LeaseKind::Bootstrap, LeaseKind::CacheRead] {
        let f = Fixture::new();
        let held = RepositoryService::hold_lease_for_testing(&f.root, f.data.path(), kind).unwrap();
        f.missing();
        let start = std::time::Instant::now();
        assert_eq!(
            f.remove().unwrap_err().kind,
            RepositoryErrorKind::RepositoryBusy
        );
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        drop(held);
        assert_eq!(f.count("repositories"), 1);
        assert_eq!(f.remove().unwrap(), RemoveRegistrationOutcome::Removed);
    }
}

#[test]
fn reappearing_root_requires_a_retry_with_the_live_repository_lease() {
    let f = Fixture::new();
    f.missing();
    let root = f.root.clone();
    let moved = f.displaced.path().join("repository");
    BEFORE_CACHE
        .with(|slot| *slot.borrow_mut() = Some(Box::new(move || fs::rename(moved, root).unwrap())));
    assert_eq!(
        f.remove().unwrap_err().kind,
        RepositoryErrorKind::RepositoryBusy
    );
    assert_eq!(f.count("repositories"), 1);
    assert_eq!(f.remove().unwrap(), RemoveRegistrationOutcome::Removed);
}

#[cfg(unix)]
#[test]
fn replacing_git_at_the_same_path_does_not_reuse_the_old_lease() {
    let f = Fixture::new();
    let root = f.root.clone();
    let moved = f.displaced.path().join("git");
    BEFORE_CACHE.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move || {
            fs::rename(root.join(".git"), moved).unwrap();
            drop(Repository::init(&root).unwrap());
            // A pathname alone is not proof that the current lock is ours.
            fs::write(root.join(".git/manyhands-operation.lock"), b"").unwrap();
        }))
    });
    assert_eq!(
        f.remove().unwrap_err().kind,
        RepositoryErrorKind::RepositoryBusy
    );
    assert_eq!(f.count("repositories"), 1);
    assert_eq!(f.remove().unwrap(), RemoveRegistrationOutcome::Removed);
}

#[test]
fn operation_identity_mismatch_stays_distinct_from_another_pending_operation() {
    let f = Fixture::new();
    let request = f.request();
    f.db().execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,observed_at) SELECT id,root_path,?1,'save_ticket','item','created',1 FROM repositories", [request.operation_id.to_string()]).unwrap();
    f.missing();
    assert_eq!(
        f.service.remove_registration(request).unwrap_err().kind,
        RepositoryErrorKind::OperationMismatch
    );
    assert_eq!(f.count("repositories"), 1);
}

#[test]
fn unknown_completed_local_evidence_is_not_silently_discarded() {
    let f = Fixture::new();
    f.db().execute("INSERT INTO operation_records(repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at) SELECT id,root_path,?1,'save_ticket','item','completed','future_step',1 FROM repositories", [OperationId::new().to_string()]).unwrap();
    f.missing();
    assert_eq!(
        f.remove().unwrap_err().kind,
        RepositoryErrorKind::RecoveryRequired
    );
    assert_eq!(f.count("repositories"), 1);
}

#[test]
fn an_accepted_removal_is_the_only_accepted_owner_that_may_proceed() {
    for owners in [1, 2] {
        let f = Fixture::new();
        let request = f.request();
        for _ in 0..owners {
            let request_id = OperationId::new().to_string();
            f.db().execute("INSERT INTO request_records(request_ulid,attempt,scope_key,command,target,intent_digest,state,accepted_at) VALUES(?1,1,?2,'repo remove','',?3,'accepted',1)", params![request_id, f.root.to_str().unwrap(), "a".repeat(64)]).unwrap();
            f.db().execute("INSERT INTO request_operations(request_ulid,ordinal,family,operation_ulid) VALUES(?1,0,'local',?2)", params![request_id, request.operation_id.to_string()]).unwrap();
        }
        f.missing();
        let result = f.service.remove_registration(request);
        if owners == 1 {
            assert_eq!(result.unwrap(), RemoveRegistrationOutcome::Removed);
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                RepositoryErrorKind::RecoveryRequired
            );
            assert_eq!(f.count("repositories"), 1);
        }
        assert_eq!(f.count("request_records"), owners);
    }
}

#[test]
fn deletion_failure_rolls_back_dependents_and_the_same_operation_can_retry() {
    let f = Fixture::new();
    f.remote("poll", "completed", false, false);
    f.db().execute_batch("CREATE TRIGGER fail_removal BEFORE DELETE ON repositories BEGIN SELECT RAISE(ABORT,'test removal rollback'); END;").unwrap();
    let operation_rows = f.count("operation_records");
    let contexts = f.count("contexts");
    let request = f.request();
    f.missing();
    assert_eq!(
        f.service
            .remove_registration(request.clone())
            .unwrap_err()
            .kind,
        RepositoryErrorKind::Sqlite
    );
    assert_eq!(f.count("repositories"), 1);
    assert_eq!(f.count("operation_records"), operation_rows + 1);
    assert_eq!(f.count("contexts"), contexts);
    assert_eq!(f.count("remote_operation_records"), 1);
    f.db().execute_batch("DROP TRIGGER fail_removal").unwrap();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(
        reopened.remove_registration(request).unwrap(),
        RemoveRegistrationOutcome::Removed
    );
    for table in [
        "repositories",
        "operation_records",
        "contexts",
        "remote_operation_records",
    ] {
        assert_eq!(f.count(table), 0, "{table}");
    }
}

#[test]
fn registration_removal_preserves_confirmation_keys_trust_and_local_drafts() {
    let f = Fixture::new();
    let preview = f.service.prepare(&crate::repository::Mutation::RepoRemove(
        crate::repository::RepoRemoveInput {
            root: f.root.clone(),
        },
    ));
    assert_eq!(preview.code, crate::results::ResultCode::Ok);
    let key = f.data.path().join("external-key");
    let draft = f.data.path().join("recoverable-draft.md");
    fs::write(&key, b"fixture key bytes").unwrap();
    fs::write(&draft, b"recoverable draft bytes").unwrap();
    f.db().execute("INSERT INTO shared_ssh_keys(id,label,ownership,private_key_path,private_source_state,public_metadata_state,selected) VALUES(?1,'fixture','imported',?2,'available','not-provided',1)", params![OperationId::new().to_string(), key.to_str().unwrap()]).unwrap();
    f.db().execute("INSERT INTO ssh_host_pins(host,port,algorithm,sha256) VALUES('fixture.invalid',22,'ssh-ed25519',?1)", ["SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"]).unwrap();
    let rows = |table: &str| {
        let db = f.db();
        let mut query = db.prepare(&format!("SELECT * FROM {table}")).unwrap();
        query
            .query_map([], |r| {
                (0..r.as_ref().column_count())
                    .map(|i| r.get::<_, rusqlite::types::Value>(i))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    };
    let tables = ["confirmation_records", "shared_ssh_keys", "ssh_host_pins"];
    let before: Vec<_> = tables.iter().map(|table| rows(table)).collect();
    f.missing();
    assert_eq!(f.remove().unwrap(), RemoveRegistrationOutcome::Removed);
    for (table, expected) in tables.iter().zip(before) {
        assert_eq!(rows(table), expected, "{table}");
    }
    assert_eq!(fs::read(key).unwrap(), b"fixture key bytes");
    assert_eq!(fs::read(draft).unwrap(), b"recoverable draft bytes");
}

#[test]
fn the_live_git_lease_is_still_required_for_removal() {
    let f = Fixture::new();
    let held =
        RepositoryService::hold_lease_for_testing(&f.root, f.data.path(), LeaseKind::Repository)
            .unwrap();
    assert_eq!(
        f.remove().unwrap_err().kind,
        RepositoryErrorKind::RepositoryBusy
    );
    drop(held);
    assert_eq!(f.count("repositories"), 1);
    assert_eq!(f.remove().unwrap(), RemoveRegistrationOutcome::Removed);
}

#[test]
fn the_exact_stored_platform_spelling_removes_a_missing_unicode_root() {
    let f = Fixture::named(Some("repository with spaces café"));
    let listed = f.service.list_repositories().unwrap();
    let stored = PathBuf::from(&listed.items[0].root);
    assert_eq!(stored.to_str(), f.root.to_str());
    #[cfg(windows)]
    assert!(stored.to_str().unwrap().starts_with(r"\\?\"));
    f.missing();
    assert_eq!(
        f.service
            .remove_registration(RemoveRegistrationRequest {
                root: stored,
                operation_id: OperationId::new()
            })
            .unwrap(),
        RemoveRegistrationOutcome::Removed
    );
    assert!(!f.root.exists());
}

#[test]
fn filesystem_failures_after_a_positive_observation_remain_failures_on_every_platform() {
    for (io_kind, domain_kind) in [
        (
            io::ErrorKind::PermissionDenied,
            RepositoryErrorKind::InaccessibleRepository,
        ),
        (
            io::ErrorKind::NotFound,
            RepositoryErrorKind::InaccessibleRepository,
        ),
        (io::ErrorKind::Other, RepositoryErrorKind::Io),
    ] {
        let root = Path::new("observed-root");
        let failed = path_failure(root, io::Error::from(io_kind));
        assert_eq!(failed.kind, domain_kind);
        assert_eq!(failed.operation, OPERATION);
        assert_eq!(failed.root.as_deref(), Some(root));
    }
}

#[cfg(unix)]
#[test]
fn dangling_git_symlink_is_not_absent_git() {
    let f = Fixture::new();
    fs::rename(f.root.join(".git"), f.displaced.path().join("git")).unwrap();
    let missing = f.displaced.path().join("missing");
    std::os::unix::fs::symlink(&missing, f.root.join(".git")).unwrap();
    assert_eq!(
        f.remove().unwrap_err().kind,
        RepositoryErrorKind::InaccessibleRepository
    );
    assert_eq!(f.count("repositories"), 1);
    assert_eq!(fs::read_link(f.root.join(".git")).unwrap(), missing);
}

#[cfg(unix)]
#[test]
fn permission_denial_is_inaccessible_and_never_registry_only_removal() {
    use std::os::unix::fs::PermissionsExt;
    struct Restore(PathBuf, fs::Permissions);
    impl Drop for Restore {
        fn drop(&mut self) {
            fs::set_permissions(&self.0, self.1.clone()).unwrap();
        }
    }
    let f = Fixture::new();
    let _restore = Restore(f.root.clone(), fs::metadata(&f.root).unwrap().permissions());
    fs::set_permissions(&f.root, fs::Permissions::from_mode(0o000)).unwrap();
    match fs::read_dir(&f.root) {
        Ok(_) => {
            eprintln!(
                "permission-denial filesystem case skipped: this user can read a mode-000 directory"
            );
            return;
        }
        Err(error) => assert_eq!(error.kind(), io::ErrorKind::PermissionDenied),
    }
    assert_eq!(
        f.remove().unwrap_err().kind,
        RepositoryErrorKind::InaccessibleRepository
    );
    assert_eq!(f.count("repositories"), 1);
}
