use super::*;
use crate::repository::{keys::*, remote::state, *};
use crate::repository::{remote::observation::ObservationConfiguration, transport::*};
use std::fs;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub(in super::super) type BoundaryHook = Box<dyn FnMut(&RepositoryService, FailurePoint)>;
thread_local! { pub(in super::super) static HOOKS: RefCell<BTreeMap<PathBuf, BoundaryHook>> = RefCell::new(BTreeMap::new()); }
pub(super) fn checkpoint(service: &RepositoryService, root: &std::path::Path, point: FailurePoint) {
    HOOKS.with(|hooks| {
        let hook = hooks.borrow_mut().remove(root);
        if let Some(mut hook) = hook {
            hook(service, point);
            hooks.borrow_mut().insert(root.into(), hook);
        }
    });
}
pub(in super::super) type TrackingHook = Box<dyn FnOnce(&RepositoryService)>;
type OutcomeReadHook = Box<dyn FnOnce() -> Result<(), RepositoryError>>;
thread_local! { static OUTCOME_READ_HOOKS:RefCell<BTreeMap<PathBuf,OutcomeReadHook>>=RefCell::new(BTreeMap::new()); }
pub(super) fn outcome_authority_read(root: &std::path::Path) -> Result<(), RepositoryError> {
    match OUTCOME_READ_HOOKS.with(|h| h.borrow_mut().remove(root)) {
        Some(hook) => hook(),
        None => Ok(()),
    }
}
thread_local! { static PREPARED_OBSERVATION_HOOKS: RefCell<BTreeMap<PathBuf,TrackingHook>> = RefCell::new(BTreeMap::new()); }
pub(super) fn observation_prepared(service: &RepositoryService, root: &std::path::Path) {
    if let Some(hook) = PREPARED_OBSERVATION_HOOKS.with(|h| h.borrow_mut().remove(root)) {
        hook(service);
    }
}
thread_local! { pub(in super::super) static TRACKING_HOOKS: RefCell<BTreeMap<PathBuf, TrackingHook>> = RefCell::new(BTreeMap::new()); }
thread_local! { pub(in super::super) static COMMIT_HOOKS: RefCell<BTreeMap<PathBuf, TrackingHook>> = RefCell::new(BTreeMap::new()); }
pub(super) fn completion_returned(
    service: &RepositoryService,
    root: &std::path::Path,
    result: Result<(), RepositoryError>,
) -> Result<(), RepositoryError> {
    let hook = COMMIT_HOOKS.with(|h| h.borrow_mut().remove(root));
    if let Some(hook) = hook {
        hook(service);
        return Err(state::recovery_required());
    }
    result
}
pub(super) fn tracking_deleted(service: &RepositoryService, root: &std::path::Path) {
    let hook = TRACKING_HOOKS.with(|h| h.borrow_mut().remove(root));
    if let Some(hook) = hook {
        hook(service);
    }
}
fn hook(f: &Fixture, callback: impl FnMut(&RepositoryService, FailurePoint) + 'static) {
    HOOKS.with(|h| {
        h.borrow_mut()
            .insert(f.root.path().into(), Box::new(callback))
    });
}

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const DOC: &str = "docs/document.md";
pub(in super::super) struct NoCredentials;
impl SessionCredentialProvider for NoCredentials {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("local promotion requested credentials")
    }
}
struct Fixture {
    root: tempfile::TempDir,
    data: tempfile::TempDir,
    service: RepositoryService,
    repo: git2::Repository,
    context: git2::Repository,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        HOOKS.with(|h| h.borrow_mut().remove(self.root.path()));
        discovery::clear_scan_hook(self.root.path());
        discovery::clear_scan_hook(&self.root.path().join(".manyhands/worktrees").join(ITEM));
        PREPARED_OBSERVATION_HOOKS.with(|h| h.borrow_mut().remove(self.root.path()));
        OUTCOME_READ_HOOKS.with(|h| h.borrow_mut().remove(self.root.path()));
    }
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
fn checkout(repo: &git2::Repository, oid: Oid) {
    repo.reset(
        &repo.find_object(oid, None).unwrap(),
        git2::ResetType::Hard,
        None,
    )
    .unwrap();
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(root.path()).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
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
        service
            .save_document(SaveDocumentRequest {
                target: AuthoringTarget {
                    root: root.path().into(),
                    kind: AuthoringKind::Document,
                    item_id: ITEM.parse().unwrap(),
                    intent: ContextIntent::Create,
                    operation_id: OperationId::new(),
                },
                source_path: None,
                destination_path: DOC.into(),
                draft: DocumentDraft {
                    title: "private title sentinel".into(),
                    body: "private body sentinel\n".into(),
                },
                expected_source: None,
                expected_destination: ExpectedPathObservation::Missing,
            })
            .unwrap();
        let context =
            git2::Repository::open(root.path().join(".manyhands/worktrees").join(ITEM)).unwrap();
        Self {
            root,
            data,
            service,
            repo,
            context,
        }
    }
    fn preview(&self) -> DocumentPromotionPreview {
        match self
            .service
            .preview_document_promotion(PreviewDocumentPromotionRequest {
                root: self.root.path().into(),
                item_id: ITEM.parse().unwrap(),
            })
            .unwrap()
        {
            DocumentPromotionPreflight::Ready(p) => p,
            other => panic!("not ready: {other:?}"),
        }
    }
    fn request(&self) -> PromoteDocumentRequest {
        PromoteDocumentRequest {
            root: self.root.path().into(),
            operation_id: OperationId::new(),
            item_id: ITEM.parse().unwrap(),
            expected: self.preview().expectation(),
            approval: None,
            confirmed_identity: None,
        }
    }
    fn run(
        &self,
        request: PromoteDocumentRequest,
    ) -> Result<DocumentPromotionResult, DocumentPromotionError> {
        self.service
            .promote_document(request, &mut SessionCredentials::new(NoCredentials))
    }
    fn primary(&self) -> Oid {
        self.repo.refname_to_id("refs/heads/main").unwrap()
    }
    fn run_io(
        &self,
        req: PromoteDocumentRequest,
        io: &mut impl run::PromotionIo,
    ) -> Result<DocumentPromotionResult, DocumentPromotionError> {
        run::promote_with_io(&self.service, req, io)
    }
    fn remote(&self) {
        self.service
            .add_remote(AddRemoteRequest {
                root: self.root.path().into(),
                name: "origin".into(),
                url: "ssh://fixture@127.0.0.1:1/repository".into(),
                operation_id: OperationId::new(),
            })
            .unwrap();
        self.service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: self.root.path().into(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            })
            .unwrap();
        commit(&self.repo);
        let config = fs::read(self.root.path().join(".manyhands/config.toml")).unwrap();
        write(&self.context, ".manyhands/config.toml", config);
        let local = commit(&self.context);
        let primary = self.repo.find_commit(self.primary()).unwrap();
        let context = self.repo.find_commit(local).unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        let merged = self
            .repo
            .commit(
                None,
                &sig,
                &sig,
                "fixture context merge",
                &context.tree().unwrap(),
                &[&context, &primary],
            )
            .unwrap();
        checkout(&self.context, merged);
        self.repo
            .reference("refs/remotes/origin/main", self.primary(), true, "fixture")
            .unwrap();
        self.repo
            .reference(
                &format!("refs/remotes/origin/manyhands/document/{ITEM}"),
                self.context.head().unwrap().target().unwrap(),
                true,
                "fixture",
            )
            .unwrap();
        self.db()
            .execute("UPDATE remote_polling_state SET history_unknown=0", [])
            .unwrap();
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let configuration = self
            .service
            .observation_configuration(self.root.path(), &plan)
            .unwrap();
        state::with_transaction(&self.service, self.root.path(), |tx,id| {
            state::configure_endpoints(tx,id,&plan,&configuration.endpoint_digest())?;
            let generation = tx.query_row("SELECT configuration_generation FROM remote_polling_state WHERE repository_id=?1",[id],|r|r.get::<_,i64>(0)).unwrap();
            let context = plan.context(AuthoringKind::Document,&ITEM.parse().unwrap());
            let oid = self.context.head().unwrap().target().unwrap();
            let observation = RemoteRefObservation::from_advertisement(&plan,context.remote_ref(),oid,Some(oid)).unwrap();
            state::complete_batch(tx,id,&plan,generation,&[observation],123)?;
            Ok(())
        }).unwrap();
    }
    fn db(&self) -> rusqlite::Connection {
        rusqlite::Connection::open(self.data.path().join(REGISTRY_FILE)).unwrap()
    }
    fn stored(&self, op: OperationId) -> record::PromotionRecord {
        state::with_transaction(&self.service, self.root.path(), |tx, id| {
            record::read_record(tx, id, op)?.ok_or_else(state::recovery_required)
        })
        .unwrap()
    }
    fn assert_cleaned(&self) {
        assert!(
            !self
                .root
                .path()
                .join(".manyhands/worktrees")
                .join(ITEM)
                .exists()
        );
        assert!(self.repo.find_worktree(ITEM).is_err());
        assert!(
            self.repo
                .find_reference(&format!("refs/heads/manyhands/document/{ITEM}"))
                .is_err()
        );
    }
}
#[derive(Default)]
pub(in super::super) struct Calls {
    pub(in super::super) children: Vec<(SynchronizationTarget, OperationId, bool)>,
    pub(in super::super) deletes: usize,
}
#[derive(Default)]
pub(in super::super) struct FakeIo {
    pub(in super::super) calls: Rc<RefCell<Calls>>,
    pub(in super::super) failure: Option<(bool, SynchronizationError)>,
    pub(in super::super) deletion: Option<DeleteObservation>,
    pub(in super::super) already_current: bool,
}
impl run::PromotionIo for FakeIo {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        req: SynchronizeRemoteRequest,
        _: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        self.calls
            .borrow_mut()
            .children
            .push((req.target.clone(), req.operation_id, req.restart));
        let primary = req.target == SynchronizationTarget::Primary;
        if self.failure.as_ref().is_some_and(|(p, _)| *p == primary) {
            return Err(self.failure.take().unwrap().1);
        }
        let ConfigurationInspection::Valid(config) = read_configuration(&req.root).unwrap() else {
            panic!()
        };
        let plan = RemoteRefPlan::from_configuration("origin", &config.primary_branch).unwrap();
        let target = req.target.operation_target(&plan);
        let reservation = service
            .reserve_promotion_child(&req.root, req.operation_id, &target, execution)
            .unwrap();
        if matches!(reservation, RemoteReservationOutcome::Replay(_)) {
            service
                .restart_promotion_child(&req.root, req.operation_id, &target, execution)
                .unwrap();
        } else {
            assert!(matches!(reservation, RemoteReservationOutcome::Reserved(_)));
        }
        let repo = git2::Repository::open(&req.root).unwrap();
        let reference = match &req.target {
            SynchronizationTarget::Primary => plan.primary().remote_ref().to_owned(),
            SynchronizationTarget::Context { kind, item_id } => {
                plan.context(*kind, item_id).remote_ref().to_owned()
            }
        };
        let oid = repo.refname_to_id(&reference).unwrap();
        if primary {
            repo.reference("refs/remotes/origin/main", oid, true, "fixture")
                .unwrap();
        }
        state::with_transaction(service, &req.root, |tx, id| {
            execution.validate(tx,id,req.operation_id)?;
            tx.execute("UPDATE remote_operation_records SET phase='completed',outcome='completed',reconciliation_required=0,sync_checkpoint='discovery_pending',authoritative_kind=?3,authoritative_oid=?2,expected_oid=?2,local_oid=?2,primary_tracking_oid=?2,push_oid=?2,push_advertised_oid=?2 WHERE operation_ulid=?1", rusqlite::params![req.operation_id.to_string(), oid.to_string(),if self.already_current {"already_current"}else{"published"}]).map_err(|_| state::recovery_required())?;
            Ok(())
        }).unwrap();
        if !self.already_current {
            local_effect(
                service,
                &req.root,
                if primary {
                    "primary_push"
                } else {
                    "context_push"
                },
            );
        }
        Ok(SynchronizationResult::Complete(if self.already_current {
            SynchronizationOutcome::AlreadyCurrent {
                target: req.target,
                oid,
            }
        } else {
            SynchronizationOutcome::Published {
                target: req.target,
                oid,
            }
        }))
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        _: &RemoteRefPlan,
        _: &RemoteReservation,
        _: &ObservationConfiguration,
        _: Oid,
        _: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.calls.borrow_mut().deletes += 1;
        local_effect(s, &req.root, "remote_delete");
        Ok(self.deletion.take().unwrap_or(DeleteObservation::Deleted))
    }
}
pub(super) fn local_effect(service: &RepositoryService, root: &std::path::Path, effect: &str) {
    state::with_transaction(service, root, |tx, _| {
        tx.execute_batch("CREATE TABLE IF NOT EXISTS promotion_test_effects(effect TEXT NOT NULL)")
            .unwrap();
        tx.execute(
            "INSERT INTO promotion_test_effects(effect) VALUES(?1)",
            [effect],
        )
        .unwrap();
        Ok(())
    })
    .unwrap();
}

fn promotion_commits(repo: &git2::Repository) -> usize {
    let odb = repo.odb().unwrap();
    let mut count = 0;
    odb.foreach(|oid| {
        if repo
            .find_commit(*oid)
            .ok()
            .is_some_and(|c| c.summary() == Some(format!("Promote document {ITEM}").as_str()))
        {
            count += 1;
        }
        true
    })
    .unwrap();
    count
}
#[test]
fn every_interruption_reopens_with_one_candidate_and_only_unfinished_children() {
    for point in [
        FailurePoint::AfterConfirmed,
        FailurePoint::AfterContextSync,
        FailurePoint::BeforeIntegrationRef,
        FailurePoint::AfterIntegrationRef,
        FailurePoint::AfterPublication,
        FailurePoint::BeforeRemoteDeletion,
        FailurePoint::AfterRemoteDeletion,
        FailurePoint::DuringWorktreeRemoval,
        FailurePoint::BeforeBranchDeletion,
        FailurePoint::BeforeCompletion,
        FailurePoint::AfterCompletion,
        FailurePoint::RefreshFailure,
    ] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let mut io = FakeIo::default();
        let failing =
            RepositoryService::open_at_with_failure_point_for_testing(f.data.path(), point)
                .unwrap();
        let _ = run::promote_with_io(&failing, req.clone(), &mut io);
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        let result = outcome(run::promote_with_io(&reopened, req.clone(), &mut io).unwrap());
        assert_eq!(result.cleanup(), PromotionCleanup::Complete, "{point:?}");
        assert_eq!(promotion_commits(&f.repo), 1, "{point:?}");
        assert_eq!(io.calls.borrow().children.len(), 2, "{point:?}");
        assert_eq!(io.calls.borrow().deletes, 1, "{point:?}");
        f.assert_cleaned();
        let before = io.calls.borrow().children.len();
        assert_eq!(
            outcome(run::promote_with_io(&reopened, req, &mut io).unwrap()),
            result
        );
        assert_eq!(io.calls.borrow().children.len(), before);
    }
}
#[test]
fn tracking_git_sql_split_retains_history_parent_and_owner_and_retries_same_id() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let op = req.operation_id;
    let root = f.root.path().to_owned();
    let data = f.data.path().to_owned();
    TRACKING_HOOKS.with(|h| h.borrow_mut().insert(root.clone(), Box::new(move |service| {
        let r = load_for_test(service, &root, op);
        assert_eq!(r.step, record::PromotionStep::BranchDeleted);
        assert!(git2::Repository::open(&root).unwrap().find_reference(&format!("refs/remotes/origin/manyhands/document/{ITEM}")).is_err());
        let db = rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap();
        assert_eq!(db.query_row("SELECT count(*) FROM retired_contexts", [], |r| r.get::<_,i64>(0)).unwrap(), 0);
        db.execute_batch("CREATE TRIGGER promotion_completion_fault BEFORE UPDATE OF step ON document_promotions WHEN NEW.step='complete' BEGIN SELECT RAISE(ABORT,'fixed test fault'); END;").unwrap();
    })));
    let mut io = FakeIo::default();
    let result = authoritative(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert_eq!(f.stored(op).step, record::PromotionStep::BranchDeleted);
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                [op.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "advertising"
    );
    f.db()
        .execute_batch("DROP TRIGGER promotion_completion_fault")
        .unwrap();
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    outcome(run::promote_with_io(&reopened, req, &mut io).unwrap());
    assert_eq!(io.calls.borrow().deletes, 1);
    assert_eq!(promotion_commits(&f.repo), 1);
}
fn load_for_test(
    service: &RepositoryService,
    root: &std::path::Path,
    op: OperationId,
) -> record::PromotionRecord {
    state::with_transaction(service, root, |tx, id| {
        record::read_record(tx, id, op)?.ok_or_else(state::recovery_required)
    })
    .unwrap()
}
#[test]
fn wrong_and_symbolic_tracking_refs_stop_before_any_retirement_sql() {
    for symbolic in [false, true] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let root = f.root.path().to_owned();
        hook(&f, move |_, p| {
            if p == FailurePoint::BeforeCompletion {
                let repo = git2::Repository::open(&root).unwrap();
                let reference = format!("refs/remotes/origin/manyhands/document/{ITEM}");
                if symbolic {
                    repo.reference_symbolic(&reference, "refs/heads/main", true, "fixture")
                        .unwrap();
                } else {
                    repo.reference(
                        &reference,
                        repo.refname_to_id("refs/heads/main").unwrap(),
                        true,
                        "fixture",
                    )
                    .unwrap();
                }
            }
        });
        let mut io = FakeIo::default();
        let result = authoritative(f.run_io(req, &mut io).unwrap());
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::RecoveryRequired
            }
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
fn cache_loss_contained_context_cleans_without_another_merge_or_child() {
    let f = Fixture::new();
    let req = f.request();
    let stopped = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterPublication,
    )
    .unwrap();
    stopped
        .promote_document(req, &mut SessionCredentials::new(NoCredentials))
        .unwrap();
    let data = tempfile::tempdir().unwrap();
    let recovered = RepositoryService::open_at(data.path()).unwrap();
    recovered
        .enable(EnableRepositoryRequest {
            root: f.root.path().into(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    recovered
        .refresh_repository(RefreshRepositoryRequest {
            root: f.root.path().into(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let p = match recovered
        .preview_document_promotion(PreviewDocumentPromotionRequest {
            root: f.root.path().into(),
            item_id: ITEM.parse().unwrap(),
        })
        .unwrap()
    {
        DocumentPromotionPreflight::Ready(p) => p,
        other => panic!("{other:?}"),
    };
    assert_eq!(p.integration(), PlannedIntegration::AlreadyContained);
    let result = recovered
        .promote_document(
            PromoteDocumentRequest {
                root: f.root.path().into(),
                operation_id: OperationId::new(),
                item_id: ITEM.parse().unwrap(),
                expected: p.expectation(),
                approval: None,
                confirmed_identity: None,
            },
            &mut SessionCredentials::new(NoCredentials),
        )
        .unwrap();
    assert_eq!(outcome(result).cleanup(), PromotionCleanup::Complete);
    assert_eq!(promotion_commits(&f.repo), 1);
}
#[test]
fn post_child_new_path_reconfirms_but_primary_only_merge_preserves_consent() {
    for new_path in [false, true] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let base = f.primary();
        let mut io = MutatingChild {
            inner: FakeIo::default(),
            new_path,
            once: false,
        };
        let result = f.run_io(req.clone(), &mut io);
        if new_path {
            assert!(matches!(
                result,
                Err(DocumentPromotionError::ReconfirmationRequired)
            ));
            assert_eq!(f.primary(), base);
            assert_eq!(
                f.stored(req.operation_id).step,
                record::PromotionStep::Abandoned
            );
        } else {
            outcome(result.unwrap());
            assert!(f.root.path().join("primary-only").exists());
        }
    }
}
struct MutatingChild {
    inner: FakeIo,
    new_path: bool,
    once: bool,
}
impl run::PromotionIo for MutatingChild {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        if req.target != SynchronizationTarget::Primary && !self.once {
            self.once = true;
            let before = s
                .preview_document_promotion(PreviewDocumentPromotionRequest {
                    root: req.root.clone(),
                    item_id: ITEM.parse().unwrap(),
                })
                .unwrap();
            let repo = git2::Repository::open(&req.root).unwrap();
            let context =
                git2::Repository::open(req.root.join(".manyhands/worktrees").join(ITEM)).unwrap();
            if self.new_path {
                write(&context, "new-remote-path", "new content");
                commit(&context);
            } else {
                write(&repo, "primary-only", "collaborator");
                let primary = commit(&repo);
                repo.reference("refs/remotes/origin/main", primary, true, "fixture")
                    .unwrap();
                let prepared = match remote::sync::prepare_clean_merge(
                    &req.root,
                    primary,
                    context.head().unwrap().target().unwrap(),
                )
                .unwrap()
                {
                    remote::sync::MergePreparation::Clean(p) => p,
                    _ => panic!(),
                };
                let tree = repo
                    .find_tree(remote::sync::import_prepared_tree(&repo, &prepared).unwrap())
                    .unwrap();
                let a = repo
                    .find_commit(context.head().unwrap().target().unwrap())
                    .unwrap();
                let b = repo.find_commit(primary).unwrap();
                let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
                let merged = repo
                    .commit(
                        None,
                        &sig,
                        &sig,
                        "Merge primary into document",
                        &tree,
                        &[&a, &b],
                    )
                    .unwrap();
                assert_eq!(
                    repo.find_commit(merged).unwrap().parent_count(),
                    2,
                    "before reset"
                );
                checkout(&context, merged);
                let after = s
                    .preview_document_promotion(PreviewDocumentPromotionRequest {
                        root: req.root.clone(),
                        item_id: ITEM.parse().unwrap(),
                    })
                    .unwrap();
                if let (
                    DocumentPromotionPreflight::Ready(before),
                    DocumentPromotionPreflight::Ready(after),
                ) = (&before, &after)
                {
                    assert_eq!(
                        before.expectation(),
                        after.expectation(),
                        "primary-only consent changed; before {:?}, after {:?}",
                        before.changes(),
                        after.changes()
                    );
                } else {
                    panic!(
                        "primary-only observation: {before:?} / {after:?}, parents {:?}, actual {:?}, tracking {:?}, ancestor {:?}, commit parents {:?}",
                        [a.id(), b.id()],
                        context.head().unwrap().target(),
                        repo.refname_to_id(&format!(
                            "refs/remotes/origin/manyhands/document/{ITEM}"
                        )),
                        repo.graph_descendant_of(merged, a.id()),
                        repo.find_commit(merged)
                            .unwrap()
                            .parent_ids()
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
        self.inner.synchronize(s, req, observer, execution)
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        config: &ObservationConfiguration,
        oid: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.inner.delete(s, req, plan, owner, config, oid, item)
    }
}
#[test]
fn advance_after_f2_resets_context_attempt_and_integrates_once() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let root = f.root.path().to_owned();
    let mut once = false;
    hook(&f, move |_, p| {
        if p == FailurePoint::AfterContextSync && !once {
            once = true;
            let repo = git2::Repository::open(&root).unwrap();
            write(&repo, "primary-only", "advance");
            let oid = commit(&repo);
            repo.reference("refs/remotes/origin/main", oid, true, "fixture")
                .unwrap();
        }
    });
    let mut io = MutatingChild {
        inner: FakeIo::default(),
        new_path: false,
        once: true,
    };
    assert!(matches!(
        f.run_io(req.clone(), &mut io),
        Err(DocumentPromotionError::PrimaryAdvanced)
    ));
    assert_eq!(promotion_commits(&f.repo), 0);
    io.once = false;
    outcome(f.run_io(req, &mut io).unwrap());
    assert_eq!(promotion_commits(&f.repo), 1);
}

#[derive(Clone, Copy, Debug)]
enum ChildMode {
    Busy,
    Yielding,
    Cancelled,
    Interrupted,
    Rejected,
    Transport,
    Recovery,
    RetainedInterrupted,
    RetainedRecovery,
    Identity,
    Conflict,
    Uncertain,
    LostResult,
}
struct MatrixIo {
    inner: FakeIo,
    mode: ChildMode,
    primary: bool,
    injected: bool,
}
fn transport_error(kind: SshTransportErrorKind) -> SynchronizationError {
    SynchronizationError::Transport(SshTransportError {
        root: "/private/path/sentinel".into(),
        remote_name: "private-endpoint-sentinel".into(),
        direction: SshDirection::Push,
        selected_key_id: None,
        authority: None,
        kind,
    })
}
impl run::PromotionIo for MatrixIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let primary = req.target == SynchronizationTarget::Primary;
        let existing = state::with_transaction(s, &req.root, |tx, id| {
            state::read_operation(tx, id, req.operation_id)
        })
        .unwrap();
        if existing.as_ref().is_some_and(|r| r.cancel_requested) {
            self.inner.calls.borrow_mut().children.push((
                req.target,
                req.operation_id,
                req.restart,
            ));
            state::with_transaction(s, &req.root, |tx,_| {
                tx.execute("UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled',cancel_requested=0 WHERE operation_ulid=?1", [req.operation_id.to_string()]).map_err(|_|state::recovery_required())?; Ok(())
            }).unwrap();
            return Err(SynchronizationError::RecoveryRequired); // Actual Task 0 acknowledgement behavior.
        }
        if primary != self.primary || self.injected {
            return self.inner.synchronize(s, req, observer, execution);
        }
        self.injected = true;
        if matches!(self.mode, ChildMode::LostResult) {
            self.inner.synchronize(s, req, observer, execution)?;
            return Err(transport_error(SshTransportErrorKind::TransportUnavailable));
        }
        self.inner.calls.borrow_mut().children.push((
            req.target.clone(),
            req.operation_id,
            req.restart,
        ));
        if matches!(self.mode, ChildMode::Busy) {
            return Err(SynchronizationError::Busy);
        }
        if matches!(self.mode, ChildMode::Yielding) {
            return Err(SynchronizationError::PollYielding);
        }
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let target = req.target.operation_target(&plan);
        let RemoteReservationOutcome::Reserved(owner) = s
            .reserve_promotion_child(&req.root, req.operation_id, &target, execution)
            .unwrap()
        else {
            panic!("expected reserved child")
        };
        if matches!(
            self.mode,
            ChildMode::RetainedInterrupted
                | ChildMode::RetainedRecovery
                | ChildMode::Identity
                | ChildMode::Conflict
        ) {
            let repo = git2::Repository::open(&req.root).unwrap();
            let local = if primary {
                repo.refname_to_id("refs/heads/main").unwrap()
            } else {
                repo.refname_to_id(&format!("refs/heads/manyhands/document/{ITEM}"))
                    .unwrap()
            };
            let incoming = repo.find_commit(local).unwrap().parent_id(0).unwrap();
            s.prepare_promotion_test_child(&req.root, &owner, primary, local, incoming)
                .unwrap();
        }
        state::with_transaction(s, &req.root, |tx,_| {
            match self.mode {
                ChildMode::Cancelled => { tx.execute("UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled' WHERE operation_ulid=?1", [req.operation_id.to_string()]).unwrap(); },
                ChildMode::Interrupted | ChildMode::RetainedInterrupted => { tx.execute("UPDATE remote_operation_records SET phase='interrupted' WHERE operation_ulid=?1", [req.operation_id.to_string()]).unwrap(); },
                ChildMode::Uncertain => { tx.execute_batch("PRAGMA ignore_check_constraints=ON").unwrap(); tx.execute("UPDATE remote_operation_records SET owner_epoch=-1 WHERE operation_ulid=?1",[req.operation_id.to_string()]).unwrap(); },
                _ => (),
            } Ok(())
        }).unwrap();
        Err(match self.mode {
            ChildMode::Cancelled | ChildMode::Interrupted | ChildMode::RetainedInterrupted => {
                SynchronizationError::Interrupted
            }
            ChildMode::Rejected => SynchronizationError::PushRejected,
            ChildMode::Transport => transport_error(SshTransportErrorKind::TransportUnavailable),
            ChildMode::Identity => SynchronizationError::IdentityRequired {
                target: req.target,
                expected_configuration: ExpectedConfiguration::for_testing(),
            },
            ChildMode::Conflict => SynchronizationError::ConflictPending {
                target: req.target,
                operation_id: req.operation_id,
                stage: if primary {
                    SynchronizationStage::Primary
                } else {
                    SynchronizationStage::Context
                },
            },
            _ => SynchronizationError::RecoveryRequired,
        })
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        config: &ObservationConfiguration,
        oid: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.inner.delete(s, req, plan, owner, config, oid, item)
    }
}
#[test]
fn ownership_first_attempt_matrix_both_children_has_no_permanent_strand() {
    for primary in [false, true] {
        for mode in [
            ChildMode::Busy,
            ChildMode::Yielding,
            ChildMode::Cancelled,
            ChildMode::Interrupted,
            ChildMode::Rejected,
            ChildMode::Transport,
            ChildMode::Recovery,
            ChildMode::RetainedInterrupted,
            ChildMode::RetainedRecovery,
            ChildMode::Identity,
            ChildMode::Conflict,
            ChildMode::LostResult,
        ] {
            let f = Fixture::new();
            f.remote();
            let req = f.request();
            let op = req.operation_id;
            let mut io = MatrixIo {
                inner: FakeIo::default(),
                mode,
                primary,
                injected: false,
            };
            let first = f.run_io(req.clone(), &mut io);
            if matches!(mode, ChildMode::LostResult) {
                outcome(first.unwrap());
                assert_eq!(io.inner.calls.borrow().children.len(), 2);
                continue;
            }
            if primary {
                let result = authoritative(first.unwrap());
                assert!(matches!(
                    result.publication(),
                    PromotionPublication::Pending { .. }
                ));
            } else {
                assert!(
                    matches!(
                        first,
                        Err(DocumentPromotionError::ContextSynchronization(_))
                    ),
                    "{mode:?}"
                );
            }
            let first_id = if primary {
                f.stored(op).primary_sync_id.unwrap()
            } else {
                f.stored(op).context_sync_id.unwrap()
            };
            let result = outcome(
                f.run_io(req, &mut io)
                    .unwrap_or_else(|e| panic!("{primary}/{mode:?}: {e:?}")),
            );
            assert_eq!(
                result.cleanup(),
                PromotionCleanup::Complete,
                "{primary}/{mode:?}"
            );
            let stored = f.stored(op);
            let last_id = if primary {
                stored.primary_sync_id.unwrap()
            } else {
                stored.context_sync_id.unwrap()
            };
            let retained = matches!(
                mode,
                ChildMode::RetainedInterrupted
                    | ChildMode::RetainedRecovery
                    | ChildMode::Identity
                    | ChildMode::Conflict
            );
            if retained {
                assert_eq!(last_id, first_id, "{primary}/{mode:?}");
                assert!(
                    io.inner
                        .calls
                        .borrow()
                        .children
                        .iter()
                        .any(|(_, id, restart)| *id == first_id && *restart)
                );
            } else {
                assert_ne!(last_id, first_id, "{primary}/{mode:?}");
            }
            assert_eq!(promotion_commits(&f.repo), 1);
        }
    }
}
#[test]
fn uncertain_child_ownership_fails_closed_without_replacement_and_primary_returns_pending() {
    for primary in [false, true] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let op = req.operation_id;
        let mut io = MatrixIo {
            inner: FakeIo::default(),
            mode: ChildMode::Uncertain,
            primary,
            injected: false,
        };
        for _ in 0..2 {
            let result = f.run_io(req.clone(), &mut io);
            if primary {
                let result = authoritative(result.unwrap());
                assert_eq!(
                    result.publication(),
                    PromotionPublication::Pending {
                        reason: PromotionPublicationPending::Synchronization(
                            PromotionSynchronizationCategory::RecoveryRequired
                        )
                    }
                );
                assert_eq!(
                    result.cleanup(),
                    PromotionCleanup::Pending {
                        reason: PromotionCleanupPending::AwaitingPublication
                    }
                );
            } else {
                assert!(matches!(
                    result,
                    Err(DocumentPromotionError::ContextSynchronization(_))
                ));
            }
        }
        assert_eq!(
            io.inner.calls.borrow().children.len(),
            if primary { 2 } else { 1 }
        );
        let r = f.stored(op);
        assert_eq!(
            if primary {
                r.primary_attempt
            } else {
                r.context_attempt
            },
            1
        );
        f.db()
            .execute(
                "UPDATE remote_operation_records SET owner_epoch=1 WHERE owner_epoch=-1",
                [],
            )
            .unwrap();
        outcome(f.run_io(req, &mut io).unwrap());
    }
}
#[test]
fn fixed_error_and_request_formatting_never_includes_content_paths_or_endpoints() {
    let f = Fixture::new();
    let req = f.request();
    let errors = [
        DocumentPromotionError::Transport(Box::new(SshTransportError {
            root: "/private/path/sentinel".into(),
            remote_name: "private-endpoint-sentinel".into(),
            direction: SshDirection::Push,
            selected_key_id: None,
            authority: None,
            kind: SshTransportErrorKind::TransportUnavailable,
        })),
        DocumentPromotionError::ContextSynchronization(Box::new(SynchronizationError::Repository(
            RepositoryError::new(
                RepositoryOperation::Read,
                Some("/private/path/sentinel".into()),
                RepositoryErrorKind::Git,
                "private body sentinel",
            ),
        ))),
    ];
    let text = format!("{req:?} {errors:?}");
    for sentinel in [
        "private title sentinel",
        "private body sentinel",
        "private-endpoint-sentinel",
        "/private/path/sentinel",
        f.root.path().to_str().unwrap(),
    ] {
        assert!(!text.contains(sentinel));
    }
}

const EXIT_POINTS: [FailurePoint; 12] = [
    FailurePoint::AfterConfirmed,
    FailurePoint::AfterContextSync,
    FailurePoint::BeforeIntegrationRef,
    FailurePoint::AfterIntegrationRef,
    FailurePoint::AfterPublication,
    FailurePoint::BeforeRemoteDeletion,
    FailurePoint::AfterRemoteDeletion,
    FailurePoint::DuringWorktreeRemoval,
    FailurePoint::BeforeBranchDeletion,
    FailurePoint::BeforeCompletion,
    FailurePoint::AfterCompletion,
    FailurePoint::RefreshFailure,
];
#[test]
fn promotion_exit_child() {
    let Ok(root) = std::env::var("MANYHANDS_PROMOTION_EXIT_ROOT") else {
        return;
    };
    let data = std::env::var("MANYHANDS_PROMOTION_EXIT_DATA").unwrap();
    let point: usize = std::env::var("MANYHANDS_PROMOTION_EXIT_POINT")
        .unwrap()
        .parse()
        .unwrap();
    let op = OperationId::parse(&std::env::var("MANYHANDS_PROMOTION_EXIT_OP").unwrap()).unwrap();
    let digest = std::env::var("MANYHANDS_PROMOTION_EXIT_EXPECTATION").unwrap();
    let expected: Vec<u8> = digest.split(',').map(|v| v.parse().unwrap()).collect();
    let service = RepositoryService::open_at_with_exit_point_for_testing(
        std::path::Path::new(&data),
        EXIT_POINTS[point.min(11)],
    )
    .unwrap();
    if point == 12 {
        TRACKING_HOOKS.with(|h| {
            h.borrow_mut().insert(
                root.clone().into(),
                Box::new(|_| std::process::exit(FAILURE_POINT_EXIT_STATUS)),
            )
        });
    }
    if point == 13 {
        COMMIT_HOOKS.with(|h| {
            h.borrow_mut().insert(
                root.clone().into(),
                Box::new(|_| std::process::exit(FAILURE_POINT_EXIT_STATUS)),
            )
        });
    }
    let request = PromoteDocumentRequest {
        root: root.into(),
        operation_id: op,
        item_id: ITEM.parse().unwrap(),
        expected: PromotionExpectation::from_bytes(expected.try_into().unwrap()),
        approval: None,
        confirmed_identity: None,
    };
    run::promote_with_io(&service, request, &mut FakeIo::default()).unwrap();
    panic!("exit point was not reached")
}
#[test]
fn process_exit_f1_through_f12_and_inside_f10_reopens_without_repeated_effects() {
    for point in 0..14 {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "repository::promotion::run_tests::promotion_exit_child",
            ])
            .env("MANYHANDS_PROMOTION_EXIT_ROOT", f.root.path())
            .env("MANYHANDS_PROMOTION_EXIT_DATA", f.data.path())
            .env("MANYHANDS_PROMOTION_EXIT_POINT", point.to_string())
            .env("MANYHANDS_PROMOTION_EXIT_OP", req.operation_id.to_string())
            .env(
                "MANYHANDS_PROMOTION_EXIT_EXPECTATION",
                req.expected
                    .to_bytes()
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(
            status.code(),
            Some(FAILURE_POINT_EXIT_STATUS),
            "boundary {point}"
        );
        if point == 12 {
            assert_eq!(
                f.stored(req.operation_id).step,
                record::PromotionStep::BranchDeleted
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT publication_evidence FROM remote_context_states",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                "observed_published"
            );
            assert_eq!(
                f.db()
                    .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            let lock = f.repo.commondir().join(format!(
                "refs/remotes/origin/manyhands/document/{ITEM}.lock"
            ));
            assert_eq!(
                fs::read(&lock).unwrap(),
                b"",
                "libgit2's surviving absence guard has no ownership token"
            );
            assert!(
                f.repo
                    .find_reference(&format!("refs/remotes/origin/manyhands/document/{ITEM}"))
                    .is_err()
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                        [req.operation_id.to_string()],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                "advertising"
            );
        }
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        let mut io = FakeIo::default();
        if point == 12 {
            let protected = directory_image(f.root.path());
            let effects = effect_counts(&f);
            let history = history_image(&f);
            let pending = run::promote_with_io(&reopened, req.clone(), &mut io).unwrap();
            assert!(matches!(pending, DocumentPromotionResult::IndexPending(_)));
            assert_eq!(
                authoritative(pending).cleanup(),
                PromotionCleanup::Pending {
                    reason: PromotionCleanupPending::RecoveryRequired
                }
            );
            assert!(
                protected == directory_image(f.root.path()),
                "ambiguous lock refusal changed protected Git/worktree bytes"
            );
            assert!(
                history == history_image(&f),
                "ambiguous lock refusal changed publication/retirement history"
            );
            assert_eq!(effect_counts(&f), effects);
            assert_eq!(
                f.stored(req.operation_id).step,
                record::PromotionStep::BranchDeleted
            );
            assert!(
                reopened
                    .repository_snapshot(f.root.path())
                    .unwrap()
                    .refresh_required
            );
            assert_eq!(io.calls.borrow().children.len(), 0);
            assert_eq!(io.calls.borrow().deletes, 0);
            let lock = f.repo.commondir().join(format!(
                "refs/remotes/origin/manyhands/document/{ITEM}.lock"
            ));
            // Simulated operator ONLY on this parent-owned disposable fixture:
            // child process reaped above, no active writers, exact expected
            // absent ref, retained SQL/history/intent and regular empty lock
            // verified. Production never adopts/removes this backend lock.
            let metadata = fs::symlink_metadata(&lock).unwrap();
            assert!(metadata.is_file() && !metadata.file_type().is_symlink());
            assert_eq!(fs::read(&lock).unwrap(), b"");
            fs::remove_file(&lock).unwrap();
        }
        if point == 13 {
            assert_eq!(
                f.stored(req.operation_id).step,
                record::PromotionStep::Complete
            );
            assert_eq!(
                f.db()
                    .query_row("SELECT count(*) FROM remote_context_states", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert_eq!(
                f.db()
                    .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                1
            );
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                        [req.operation_id.to_string()],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                "completed"
            );
            assert!(
                f.repo
                    .commondir()
                    .join(format!(
                        "refs/remotes/origin/manyhands/document/{ITEM}.lock"
                    ))
                    .is_file()
            );
        }
        let retried = run::promote_with_io(&reopened, req.clone(), &mut io).unwrap();
        assert!(
            matches!(retried, DocumentPromotionResult::Complete(_)),
            "boundary {point}; stored {:?}; tracking lock present {}; retry {retried:?}",
            f.stored(req.operation_id).step,
            f.repo
                .commondir()
                .join(format!(
                    "refs/remotes/origin/manyhands/document/{ITEM}.lock"
                ))
                .exists()
        );
        let result = outcome(retried);
        if point == 13 {
            // Actual committed completion is replay authority. A stray backend
            // lock is still preserved; replay never adopts/deletes it.
            assert!(
                f.repo
                    .commondir()
                    .join(format!(
                        "refs/remotes/origin/manyhands/document/{ITEM}.lock"
                    ))
                    .is_file()
            );
        }
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Complete,
            "boundary {point}"
        );
        assert_eq!(promotion_commits(&f.repo), 1, "boundary {point}");
        for effect in [
            "context_push",
            "primary_push",
            "remote_delete",
            "worktree_remove",
            "branch_delete",
        ] {
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM promotion_test_effects WHERE effect=?1",
                        [effect],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                1,
                "boundary {point}/{effect}"
            );
        }
        f.assert_cleaned();
        assert_eq!(
            outcome(run::promote_with_io(&reopened, req, &mut io).unwrap()),
            result
        );
    }
}
fn directory_image(root: &std::path::Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(
        base: &std::path::Path,
        path: &std::path::Path,
        image: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(base, &path, image);
            } else {
                image.insert(
                    path.strip_prefix(base).unwrap().into(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut image = BTreeMap::new();
    walk(root, root, &mut image);
    image
}
fn effect_counts(f: &Fixture) -> Vec<(String, i64)> {
    f.db()
        .prepare(
            "SELECT effect,count(*) FROM promotion_test_effects GROUP BY effect ORDER BY effect",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
fn history_image(f: &Fixture) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
    [
        "remote_context_states",
        "retired_contexts",
        "remote_observation_batches",
        "remote_ref_observations",
    ]
    .iter()
    .map(|table| {
        let db = f.db();
        let mut q = db
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = q.column_count();
        let rows = q
            .query_map([], |r| {
                (0..columns)
                    .map(|c| r.get(c))
                    .collect::<Result<Vec<rusqlite::types::Value>, _>>()
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        ((*table).to_owned(), rows)
    })
    .collect()
}

fn commit_on(repo: &git2::Repository, parent: Oid, path: &str, bytes: &[u8]) -> Oid {
    let parent = repo.find_commit(parent).unwrap();
    let mut index = git2::Index::new().unwrap();
    index.read_tree(&parent.tree().unwrap()).unwrap();
    let mut entry = index.get_path(std::path::Path::new(path), 0).unwrap();
    entry.id = repo.blob(bytes).unwrap();
    entry.file_size = bytes.len() as u32;
    index.add(&entry).unwrap();
    let tree = repo.find_tree(index.write_tree_to(repo).unwrap()).unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    repo.commit(None, &sig, &sig, "incoming fixture", &tree, &[&parent])
        .unwrap()
}
struct ConflictIo {
    inner: FakeIo,
    incoming: Oid,
    role_primary: bool,
}
impl run::PromotionIo for ConflictIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        if (req.target == SynchronizationTarget::Primary) == self.role_primary {
            // Execute the real child's local integration/reconciliation stack;
            // only transport/publication is injected by the enclosing test seam.
            let before = self.inner.calls.borrow().children.len();
            self.inner.calls.borrow_mut().children.push((
                req.target.clone(),
                req.operation_id,
                req.restart,
            ));
            remote::sync::promotion_test_drive_local_bound(s, &req, self.incoming, execution)?;
            self.inner.calls.borrow_mut().children.truncate(before);
        }
        self.inner.synchronize(s, req, observer, execution)
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        config: &ObservationConfiguration,
        oid: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.inner.delete(s, req, plan, owner, config, oid, item)
    }
}
#[test]
fn genuine_context_conflict_resolves_canonical_and_external_and_reuses_owned_resolution() {
    for mode in [
        "canonical",
        "external",
        "owned_resolution",
        "interrupted_conflict",
    ] {
        let f = Fixture::new();
        f.remote();
        let base = f.context.head().unwrap().target().unwrap();
        let path = if mode == "external" {
            "ordinary.txt"
        } else {
            DOC
        };
        let base_bytes = fs::read(f.context.workdir().unwrap().join(path)).unwrap();
        let local_bytes = if mode == "external" {
            b"local code\n".to_vec()
        } else {
            String::from_utf8(base_bytes.clone())
                .unwrap()
                .replace("private body sentinel", "local body")
                .into_bytes()
        };
        let incoming_bytes = if mode == "external" {
            b"remote code\n".to_vec()
        } else {
            String::from_utf8(base_bytes)
                .unwrap()
                .replace("private body sentinel", "remote body")
                .into_bytes()
        };
        write(&f.context, path, &local_bytes);
        let local = commit(&f.context);
        let incoming = commit_on(&f.repo, base, path, &incoming_bytes);
        let req = f.request();
        let primary = f.primary();
        let mut io = ConflictIo {
            inner: FakeIo::default(),
            incoming,
            role_primary: false,
        };
        let error = f.run_io(req.clone(), &mut io).unwrap_err();
        assert!(
            matches!(error,DocumentPromotionError::ContextSynchronization(ref e) if matches!(**e,SynchronizationError::ConflictPending { .. })),
            "{mode}: {error:?}"
        );
        let child = f.stored(req.operation_id).context_sync_id.unwrap();
        assert_eq!(f.primary(), primary);
        assert!(
            git2::Repository::open(f.context.workdir().unwrap())
                .unwrap()
                .index()
                .unwrap()
                .has_conflicts()
        );
        let inspection = f
            .service
            .inspect_synchronization_recovery(f.root.path(), child)
            .unwrap();
        assert_eq!(inspection.paths.len(), 1);
        if mode == "interrupted_conflict" {
            f.db().execute("UPDATE remote_operation_records SET phase='interrupted' WHERE operation_ulid=?1",[child.to_string()]).unwrap();
            assert!(
                matches!(f.run_io(req.clone(),&mut io),Err(DocumentPromotionError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::ConflictPending { .. }))
            );
            assert_eq!(f.stored(req.operation_id).context_sync_id, Some(child));
        }
        if mode == "external" {
            assert_eq!(
                inspection.paths[0].eligibility,
                ConflictEligibility::ExternalResolutionRequired
            );
            f.service
                .read_synchronization_conflict(&inspection.paths[0].token)
                .unwrap();
            let resolution = ResolveSynchronizationRequest::new(
                f.root.path().into(),
                child,
                OperationId::new(),
                inspection.observation.clone(),
                vec![],
                None,
            );
            assert!(matches!(
                f.service.resolve_synchronization(resolution),
                Err(SynchronizationError::ExternalResolutionRequired { .. })
            ));
            write(&f.context, path, &local_bytes);
            let mut index = f.context.index().unwrap();
            index.read(true).unwrap();
            index.add_path(std::path::Path::new(path)).unwrap();
            index.write().unwrap();
            assert!(!index.has_conflicts());
            let tree = f
                .repo
                .find_tree(index.write_tree_to(&f.repo).unwrap())
                .unwrap();
            let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
            let a = f.repo.find_commit(local).unwrap();
            let b = f.repo.find_commit(incoming).unwrap();
            let merged = f
                .repo
                .commit(
                    None,
                    &sig,
                    &sig,
                    "external whole-merge repair",
                    &tree,
                    &[&a, &b],
                )
                .unwrap();
            checkout(&f.context, merged);
            f.context.cleanup_state().unwrap();
        } else {
            f.service
                .read_synchronization_conflict(&inspection.paths[0].token)
                .unwrap();
            let resolution = ResolveSynchronizationRequest::new(
                f.root.path().into(),
                child,
                OperationId::new(),
                inspection.observation.clone(),
                vec![(
                    inspection.paths[0].token.clone(),
                    RedactedConflictBytes::from_bytes(local_bytes.clone()),
                )],
                None,
            );
            if mode == "owned_resolution" {
                let failed = RepositoryService::open_at_with_failure_point_for_testing(
                    f.data.path(),
                    FailurePoint::ResolutionAfterCandidatePrepared,
                )
                .unwrap();
                assert!(failed.resolve_synchronization(resolution.clone()).is_err());
                f.service
                    .cancel_remote_operation(f.root.path(), child)
                    .unwrap();
                let pending = f.run_io(req.clone(), &mut io).unwrap_err();
                assert!(matches!(
                    pending,
                    DocumentPromotionError::ContextSynchronization(_)
                ));
                assert_eq!(f.stored(req.operation_id).context_sync_id, Some(child));
            }
            assert!(matches!(
                f.service.resolve_synchronization(resolution).unwrap(),
                ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
            ));
        }
        let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
        assert_eq!(result.cleanup(), PromotionCleanup::Complete);
        assert_eq!(f.stored(req.operation_id).context_sync_id, Some(child));
        assert_eq!(promotion_commits(&f.repo), 1);
        f.assert_cleaned();
    }
}

#[test]
fn genuine_primary_owned_resolution_stays_pending_on_recovery_then_publishes_same_child() {
    let f = Fixture::new();
    checkout(&f.repo, f.context.head().unwrap().target().unwrap());
    f.remote();
    let base = f.primary();
    let source = fs::read_to_string(f.context.workdir().unwrap().join(DOC)).unwrap();
    let local = source.replace("private body sentinel", "local primary body");
    write(&f.context, DOC, &local);
    commit(&f.context);
    let incoming = commit_on(
        &f.repo,
        base,
        DOC,
        source
            .replace("private body sentinel", "remote primary body")
            .as_bytes(),
    );
    let req = f.request();
    let mut io = ConflictIo {
        inner: FakeIo::default(),
        incoming,
        role_primary: true,
    };
    let first = authoritative(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        first.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::Synchronization(
                PromotionSynchronizationCategory::Conflict
            )
        }
    );
    let child = f.stored(req.operation_id).primary_sync_id.unwrap();
    let inspection = f
        .service
        .inspect_synchronization_recovery(f.root.path(), child)
        .unwrap();
    let resolution = ResolveSynchronizationRequest::new(
        f.root.path().into(),
        child,
        OperationId::new(),
        inspection.observation,
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(local.into_bytes()),
        )],
        None,
    );
    let stopped = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::ResolutionAfterCandidatePrepared,
    )
    .unwrap();
    assert!(stopped.resolve_synchronization(resolution.clone()).is_err());
    f.service
        .cancel_remote_operation(f.root.path(), child)
        .unwrap();
    for _ in 0..2 {
        let result = authoritative(f.run_io(req.clone(), &mut io).unwrap());
        assert_eq!(
            result.publication(),
            PromotionPublication::Pending {
                reason: PromotionPublicationPending::Synchronization(
                    PromotionSynchronizationCategory::RecoveryRequired
                )
            }
        );
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::AwaitingPublication
            }
        );
        assert_eq!(f.stored(req.operation_id).primary_sync_id, Some(child));
        assert!(f.context.workdir().unwrap().exists());
        assert_eq!(io.inner.calls.borrow().deletes, 0);
    }
    assert!(matches!(
        f.service.resolve_synchronization(resolution).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        result.publication(),
        PromotionPublication::Published { oid: f.primary() }
    );
    assert_eq!(f.stored(req.operation_id).primary_sync_id, Some(child));
    assert_eq!(outcome(f.run_io(req, &mut io).unwrap()), result);
    assert_eq!(promotion_commits(&f.repo), 1);
}
#[test]
fn valid_configuration_changes_after_integration_stop_before_any_cleanup_effect() {
    for mode in ["primary_retry", "remote_midstep", "endpoint_midstep"] {
        let f = Fixture::new();
        if mode == "endpoint_midstep" {
            f.remote();
        }
        let req = f.request();
        let root = f.root.path().to_owned();
        let config_path = root.join(".manyhands/config.toml");
        if mode == "primary_retry" {
            let s = RepositoryService::open_at_with_failure_point_for_testing(
                f.data.path(),
                FailurePoint::AfterPublication,
            )
            .unwrap();
            s.promote_document(req.clone(), &mut SessionCredentials::new(NoCredentials))
                .unwrap();
            let config = fs::read_to_string(&config_path)
                .unwrap()
                .replace("main", "other");
            fs::write(&config_path, config).unwrap();
        } else {
            let mode = mode.to_owned();
            hook(&f, move |_, p| {
                if p == FailurePoint::AfterPublication {
                    if mode == "endpoint_midstep" {
                        git2::Repository::open(&root)
                            .unwrap()
                            .remote_set_url("origin", "ssh://fixture@127.0.0.1:2/changed")
                            .unwrap();
                    } else {
                        let config = fs::read_to_string(&config_path).unwrap()
                            + "\npublication_remote = \"changed\"\n";
                        fs::write(&config_path, config).unwrap();
                    }
                }
            });
        }
        let mut io = FakeIo::default();
        let result = authoritative(f.run_io(req, &mut io).unwrap());
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::RecoveryRequired
            },
            "{mode}"
        );
        assert!(f.context.workdir().unwrap().join(DOC).exists(), "{mode}");
        assert_eq!(io.calls.borrow().deletes, 0, "{mode}");
    }
}
#[test]
fn final_sql_commit_failure_rolls_back_all_history_and_retry_finishes() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let old = history_image(&f);
    f.db().execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE promotion_commit_fault(value INTEGER REFERENCES repositories(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER promotion_commit_fault_trigger AFTER INSERT ON retired_contexts BEGIN INSERT INTO promotion_commit_fault VALUES(-1); END;").unwrap();
    let mut io = FakeIo::default();
    let result = authoritative(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert_eq!(
        f.stored(req.operation_id).step,
        record::PromotionStep::BranchDeleted
    );
    assert!(old == history_image(&f));
    assert!(
        f.repo
            .find_reference(&format!("refs/remotes/origin/manyhands/document/{ITEM}"))
            .is_err()
    );
    assert_ne!(
        f.db()
            .query_row(
                "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                [req.operation_id.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "completed"
    );
    f.db()
        .execute_batch(
            "DROP TRIGGER promotion_commit_fault_trigger; DROP TABLE promotion_commit_fault;",
        )
        .unwrap();
    assert_eq!(
        outcome(f.run_io(req, &mut io).unwrap()).cleanup(),
        PromotionCleanup::Complete
    );
    assert_eq!(io.calls.borrow().deletes, 1);
}
#[test]
fn lost_completion_commit_result_reopens_actual_complete_authority() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let root = f.root.path().to_owned();
    let op = req.operation_id;
    let fired = Rc::new(std::cell::Cell::new(false));
    let observed = fired.clone();
    COMMIT_HOOKS.with(|h| {
        h.borrow_mut().insert(
            root.clone(),
            Box::new(move |s| {
                observed.set(true);
                assert_eq!(
                    load_for_test(s, &root, op).step,
                    record::PromotionStep::Complete
                );
            }),
        )
    });
    let mut io = FakeIo::default();
    let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
    assert!(
        fired.get(),
        "uncertain commit-result seam must actually execute"
    );
    assert_eq!(result.cleanup(), PromotionCleanup::Complete);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                [op.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "completed"
    );
    assert_eq!(
        f.db()
            .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let counts = effect_counts(&f);
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(
        outcome(run::promote_with_io(&reopened, req, &mut io).unwrap()),
        result
    );
    assert_eq!(effect_counts(&f), counts);
}

#[test]
fn supersession_finishes_new_committed_context_and_fences_old_remote_changed_owner() {
    for remote_changed in [false, true] {
        let f = Fixture::new();
        f.remote();
        let first = f.request();
        let mut io = FakeIo::default();
        if remote_changed {
            io.deletion = Some(DeleteObservation::Changed(f.primary()));
        } else {
            let root = f.root.path().to_owned();
            hook(&f, move |_, p| {
                if p == FailurePoint::AfterPublication {
                    write(
                        &git2::Repository::open(root.join(".manyhands/worktrees").join(ITEM))
                            .unwrap(),
                        "untracked",
                        "keep",
                    );
                }
            });
        }
        let pending = authoritative(f.run_io(first.clone(), &mut io).unwrap());
        assert!(matches!(
            pending.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::ContextChanged
                    | PromotionCleanupPending::RemoteContextChanged
            }
        ));
        HOOKS.with(|h| h.borrow_mut().remove(f.root.path()));
        write(&f.context, "next", "new confirmed content");
        commit(&f.context);
        let next = f.request();
        let mut synced = ConflictIo {
            inner: io,
            incoming: f.context.head().unwrap().target().unwrap(),
            role_primary: false,
        };
        outcome(f.run_io(next.clone(), &mut synced).unwrap());
        assert_eq!(
            f.stored(first.operation_id).step,
            record::PromotionStep::Superseded
        );
        assert_eq!(
            f.stored(next.operation_id).step,
            record::PromotionStep::Complete
        );
        if remote_changed {
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                        [first.operation_id.to_string()],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                "cancelled"
            );
        }
        assert_eq!(promotion_commits(&f.repo), 2);
        assert!(f.root.path().join("next").exists());
        let calls = synced.inner.calls.borrow().children.len();
        let deletes = synced.inner.calls.borrow().deletes;
        assert_eq!(
            authoritative(f.run_io(first, &mut synced).unwrap()).cleanup(),
            pending.cleanup()
        );
        assert_eq!(synced.inner.calls.borrow().children.len(), calls);
        assert_eq!(synced.inner.calls.borrow().deletes, deletes);
    }
}
#[test]
fn early_remote_delete_keeps_history_and_explicit_deleted_publication_barrier() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let history = history_image(&f);
    let stopped = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterRemoteDeletion,
    )
    .unwrap();
    let mut io = FakeIo::default();
    let pending = authoritative(run::promote_with_io(&stopped, req.clone(), &mut io).unwrap());
    assert_eq!(
        pending.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert!(history == history_image(&f));
    let tracking = format!("refs/remotes/origin/manyhands/document/{ITEM}");
    f.repo.find_reference(&tracking).unwrap().delete().unwrap(); // Fixture-only later observation of remote absence.
    let check = remote::sync::promotion_test_publication_plan(
        &f.service,
        f.root.path(),
        &SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: ITEM.parse().unwrap(),
        },
    );
    assert!(matches!(
        check,
        Err(SynchronizationError::RemoteContextDeleted)
    ));
    assert_eq!(io.calls.borrow().deletes, 1);
    outcome(f.run_io(req, &mut io).unwrap());
    assert_eq!(io.calls.borrow().deletes, 1);
}
#[test]
fn cancel_at_pre_ref_and_each_local_boundary_stops_before_the_whole_step() {
    for point in [
        FailurePoint::BeforeIntegrationRef,
        FailurePoint::DuringWorktreeRemoval,
        FailurePoint::BeforeBranchDeletion,
        FailurePoint::BeforeCompletion,
    ] {
        let f = Fixture::new();
        let req = f.request();
        let op = req.operation_id;
        let root = f.root.path().to_owned();
        let mut once = false;
        let primary = f.primary();
        hook(&f, move |s, p| {
            if p == point && !once {
                once = true;
                assert!(s.cancel_document_promotion(&root, op).unwrap());
            }
        });
        let result = f.run(req.clone());
        if point == FailurePoint::BeforeIntegrationRef {
            assert!(matches!(result, Err(DocumentPromotionError::Interrupted)));
            assert_eq!(f.primary(), primary);
        } else {
            assert_eq!(
                authoritative(result.unwrap()).cleanup(),
                PromotionCleanup::Pending {
                    reason: PromotionCleanupPending::Cancelled
                }
            );
            if point == FailurePoint::DuringWorktreeRemoval {
                assert!(f.context.workdir().unwrap().join(DOC).exists());
            }
            if point == FailurePoint::BeforeBranchDeletion {
                assert!(
                    f.repo
                        .find_reference(&format!("refs/heads/manyhands/document/{ITEM}"))
                        .is_ok()
                );
            }
            assert_eq!(
                f.db()
                    .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
        outcome(f.run(req).unwrap());
        f.assert_cleaned();
        assert_eq!(promotion_commits(&f.repo), 1);
    }
}
#[test]
fn post_ref_retry_lease_failure_and_cancel_clear_failure_never_become_errors() {
    let f = Fixture::new();
    let req = f.request();
    let stopped = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterIntegrationRef,
    )
    .unwrap();
    stopped
        .promote_document(req.clone(), &mut SessionCredentials::new(NoCredentials))
        .unwrap();
    let lease = repository_lease(&f.repo, f.root.path(), RepositoryOperation::Read).unwrap();
    let result = authoritative(f.run(req.clone()).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    drop(lease);
    assert!(
        f.service
            .cancel_document_promotion(f.root.path(), req.operation_id)
            .unwrap()
    );
    f.db().execute_batch("CREATE TRIGGER cancel_clear_fault BEFORE UPDATE OF cancel_requested ON document_promotions WHEN OLD.cancel_requested=1 AND NEW.cancel_requested=0 BEGIN SELECT RAISE(ABORT,'fixed fault'); END;").unwrap();
    let result = authoritative(f.run(req.clone()).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    f.db()
        .execute_batch("DROP TRIGGER cancel_clear_fault")
        .unwrap();
    outcome(f.run(req).unwrap());
    assert_eq!(promotion_commits(&f.repo), 1);
}
struct DeleteErrorIo {
    inner: FakeIo,
    kind: Option<SshTransportErrorKind>,
}
impl run::PromotionIo for DeleteErrorIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        o: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        self.inner.synchronize(s, req, o, execution)
    }
    fn delete(
        &mut self,
        _: &RepositoryService,
        _: VerifySshTransportRequest,
        _: &RemoteRefPlan,
        _: &RemoteReservation,
        _: &ObservationConfiguration,
        _: Oid,
        _: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        Err(transport_error(self.kind.take().unwrap()))
    }
}
#[test]
fn deletion_failures_are_fixed_redacted_categories_and_keep_history() {
    for (kind, category) in [
        (
            SshTransportErrorKind::ConfigurationInvalid,
            PromotionRemoteDeletionCategory::Configuration,
        ),
        (
            SshTransportErrorKind::KeyMissing,
            PromotionRemoteDeletionCategory::Key,
        ),
        (
            SshTransportErrorKind::UnlockCancelled,
            PromotionRemoteDeletionCategory::Unlock,
        ),
        (
            SshTransportErrorKind::HostVerificationUnavailable,
            PromotionRemoteDeletionCategory::Host,
        ),
        (
            SshTransportErrorKind::TransportUnavailable,
            PromotionRemoteDeletionCategory::Transport,
        ),
        (
            SshTransportErrorKind::PushRejected,
            PromotionRemoteDeletionCategory::Rejected,
        ),
    ] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let before = history_image(&f);
        let mut io = DeleteErrorIo {
            inner: FakeIo::default(),
            kind: Some(kind),
        };
        let result = authoritative(f.run_io(req, &mut io).unwrap());
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::RemoteDeletionFailed(category)
            }
        );
        assert!(before == history_image(&f));
        assert!(f.context.workdir().unwrap().exists());
        assert!(!format!("{result:?}").contains("private-endpoint-sentinel"));
    }
}
#[test]
fn contained_remote_cache_recovery_skips_deleted_context_child_and_does_not_remerge() {
    let f = Fixture::new();
    f.remote();
    checkout(&f.repo, f.context.head().unwrap().target().unwrap());
    f.repo
        .reference("refs/remotes/origin/main", f.primary(), true, "fixture")
        .unwrap();
    f.repo
        .find_reference(&format!("refs/remotes/origin/manyhands/document/{ITEM}"))
        .unwrap()
        .delete()
        .unwrap();
    f.db()
        .execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    let req = f.request();
    let mut io = FakeIo::default();
    let result = outcome(f.run_io(req, &mut io).unwrap());
    assert!(matches!(
        result.integration(),
        PromotionIntegration::AlreadyContained { .. }
    ));
    assert_eq!(promotion_commits(&f.repo), 0);
    assert_eq!(io.calls.borrow().deletes, 0);
    assert!(
        io.calls
            .borrow()
            .children
            .iter()
            .all(|(t, _, _)| *t == SynchronizationTarget::Primary)
    );
}

#[test]
fn f6_rechecks_dirty_or_ignored_context_under_lease_before_deletion() {
    for ignored in [false, true] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let root = f.root.path().to_owned();
        hook(&f, move |_, p| {
            if p == FailurePoint::BeforeRemoteDeletion {
                let path = if ignored {
                    "late-ignored"
                } else {
                    "late-untracked"
                };
                if ignored {
                    fs::write(
                        git2::Repository::open(&root)
                            .unwrap()
                            .path()
                            .join("info/exclude"),
                        "late-ignored\n",
                    )
                    .unwrap();
                }
                write(
                    &git2::Repository::open(root.join(".manyhands/worktrees").join(ITEM)).unwrap(),
                    path,
                    "preserve",
                );
            }
        });
        let mut io = FakeIo::default();
        let result = authoritative(f.run_io(req, &mut io).unwrap());
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: if ignored {
                    PromotionCleanupPending::IgnoredFiles
                } else {
                    PromotionCleanupPending::ContextChanged
                }
            }
        );
        assert_eq!(io.calls.borrow().deletes, 0);
        assert!(f.context.workdir().unwrap().exists());
    }
}
struct ConsentIo {
    inner: FakeIo,
    approval: HostApproval,
    identity: ConfirmedCommitIdentity,
    parent: OperationId,
    cancel: Option<bool>,
}
impl run::PromotionIo for ConsentIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        o: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        assert_eq!(req.approval.as_ref(), Some(&self.approval));
        assert_eq!(
            req.confirmed_identity.as_ref().unwrap().confirmation_id,
            self.identity.confirmation_id
        );
        let repo = git2::Repository::open(&req.root).unwrap();
        drop(repository_lease(&repo, &req.root, RepositoryOperation::Read).unwrap());
        if self.cancel == Some(req.target == SynchronizationTarget::Primary) {
            self.cancel = None;
            assert!(s.cancel_document_promotion(&req.root, self.parent).unwrap());
            assert_eq!(
                o.safe_point(RemoteOperationSafePoint::BeforeFetch),
                SafePointRequest::Cancel
            );
            return Err(SynchronizationError::Interrupted);
        }
        assert_eq!(
            o.safe_point(RemoteOperationSafePoint::BeforeFetch),
            SafePointRequest::Continue
        );
        self.inner.synchronize(s, req, o, execution)
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        config: &ObservationConfiguration,
        oid: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        assert_eq!(req.approval.as_ref(), Some(&self.approval));
        let repo = git2::Repository::open(&req.root).unwrap();
        drop(repository_lease(&repo, &req.root, RepositoryOperation::Read).unwrap());
        self.inner.delete(s, req, plan, owner, config, oid, item)
    }
}
#[test]
fn consent_identity_cancellation_forwarding_and_network_seams_never_hold_a_lease() {
    for primary_cancel in [false, true] {
        let f = Fixture::new();
        f.remote();
        let mut req = f.request();
        let approval = HostApproval {
            authority: SshAuthority {
                host: "fixture.invalid".into(),
                port: 22,
            },
            expected: None,
            presented: HostKeyIdentity {
                algorithm: "ssh-ed25519".into(),
                sha256: "fixture-fingerprint".into(),
            },
        };
        let identity = ConfirmedCommitIdentity {
            confirmation_id: OperationId::new(),
            identity: CommitIdentity {
                name: "Fixture".into(),
                email: "fixture@example.invalid".into(),
            },
            expected_configuration: ExpectedConfiguration::for_testing(),
        };
        req.approval = Some(approval.clone());
        req.confirmed_identity = Some(identity.clone());
        let mut io = ConsentIo {
            inner: FakeIo::default(),
            approval,
            identity,
            parent: req.operation_id,
            cancel: Some(primary_cancel),
        };
        let first = f.run_io(req.clone(), &mut io);
        if primary_cancel {
            assert_eq!(
                authoritative(first.unwrap()).publication(),
                PromotionPublication::Pending {
                    reason: PromotionPublicationPending::Cancelled
                }
            );
        } else {
            assert!(matches!(first, Err(DocumentPromotionError::Interrupted)));
        }
        let before = f.stored(req.operation_id);
        outcome(f.run_io(req.clone(), &mut io).unwrap());
        let after = f.stored(req.operation_id);
        assert_ne!(
            if primary_cancel {
                before.primary_sync_id
            } else {
                before.context_sync_id
            },
            if primary_cancel {
                after.primary_sync_id
            } else {
                after.context_sync_id
            }
        );
        assert_eq!(promotion_commits(&f.repo), 1);
    }
}
#[test]
fn actual_primary_identity_required_keeps_original_child_for_confirmed_continuation() {
    let f = Fixture::new();
    f.remote();
    let base = f.primary();
    let incoming = commit_on(&f.repo, base, "ordinary.txt", b"remote unrelated change\n");
    let mut req = f.request();
    let root = f.root.path().to_owned();
    let mut fired = false;
    hook(&f, move |_, p| {
        if p == FailurePoint::AfterIntegrationRef && !fired {
            fired = true;
            let repo = git2::Repository::open(&root).unwrap();
            let mut c = repo.config().unwrap();
            c.set_str("user.name", "").unwrap();
            c.set_str("user.email", "").unwrap();
        }
    });
    let mut io = ConflictIo {
        inner: FakeIo::default(),
        incoming,
        role_primary: true,
    };
    let first = authoritative(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        first.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::Synchronization(
                PromotionSynchronizationCategory::Identity
            )
        }
    );
    let child = f.stored(req.operation_id).primary_sync_id;
    let expected_configuration = match remote::sync::select_committing_identity(
        &f.repo,
        None,
        &SynchronizationTarget::Primary,
    ) {
        Err(SynchronizationError::IdentityRequired {
            expected_configuration,
            ..
        }) => expected_configuration,
        _ => panic!("missing real identity boundary"),
    };
    req.confirmed_identity = Some(ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Confirmed".into(),
            email: "confirmed@example.invalid".into(),
        },
        expected_configuration,
    });
    let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        result.publication(),
        PromotionPublication::Published { oid: f.primary() }
    );
    assert_eq!(f.stored(req.operation_id).primary_sync_id, child);
    assert_eq!(
        f.repo.config().unwrap().get_string("user.name").unwrap(),
        "Confirmed"
    );
    assert_eq!(promotion_commits(&f.repo), 1);
}
fn assert_primary_discovery(f: &Fixture, only: bool) {
    let snapshot = f.service.repository_snapshot(f.root.path()).unwrap();
    assert!(!snapshot.refresh_required);
    let items = snapshot
        .items
        .iter()
        .filter(|i| i.id.to_string() == ITEM)
        .collect::<Vec<_>>();
    assert!(
        snapshot
            .contexts
            .iter()
            .any(|c| c.kind == DiscoveryContextKind::Primary && c.head_oid == Some(f.primary()))
    );
    assert!(
        matches!(crate::canonical::parse_item(std::path::Path::new(DOC),&fs::read_to_string(f.root.path().join(DOC)).unwrap()).unwrap(),crate::canonical::CanonicalItem::Document(d) if d.id.to_string()==ITEM)
    );
    // The existing public list prefers an active authoring context while it
    // survives (observe_root leaves active items to their contexts); primary
    // head metadata and the active item projection are refreshed. Completion
    // removes that override, so the single public item is then primary-only.
    if only {
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].context, f.root.path());
    }
}
#[test]
fn partial_refresh_failure_preserves_outcome_and_later_refresh_shows_integrated_primary() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::RefreshFailure,
    )
    .unwrap();
    let mut io = FakeIo {
        failure: Some((true, SynchronizationError::PushRejected)),
        ..Default::default()
    };
    let result = run::promote_with_io(&service, req.clone(), &mut io).unwrap();
    let DocumentPromotionResult::IndexPending(p) = result else {
        panic!("expected partial index pending")
    };
    assert_eq!(
        p.authoritative.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::AwaitingPublication
        }
    );
    assert!(
        f.service
            .repository_snapshot(f.root.path())
            .unwrap()
            .refresh_required
    );
    f.service
        .refresh_repository(RefreshRepositoryRequest {
            root: f.root.path().into(),
            operation_id: OperationId::new(),
        })
        .unwrap();
    assert_primary_discovery(&f, false);
    outcome(f.run_io(req, &mut io).unwrap());
    assert_primary_discovery(&f, true);
}
#[test]
fn later_edit_after_runner_completion_provisions_fresh_context_and_first_publication() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let mut io = FakeIo::default();
    outcome(f.run_io(req, &mut io).unwrap());
    assert_primary_discovery(&f, true);
    let primary = f.primary();
    f.service
        .prepare_context(AuthoringTarget {
            root: f.root.path().into(),
            kind: AuthoringKind::Document,
            item_id: ITEM.parse().unwrap(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        })
        .unwrap();
    let fresh =
        git2::Repository::open(f.root.path().join(".manyhands/worktrees").join(ITEM)).unwrap();
    assert_eq!(fresh.head().unwrap().target(), Some(primary));
    f.db()
        .execute("UPDATE remote_polling_state SET history_unknown=1", [])
        .unwrap();
    assert!(
        remote::sync::promotion_test_publication_plan(
            &f.service,
            f.root.path(),
            &SynchronizationTarget::Context {
                kind: AuthoringKind::Document,
                item_id: ITEM.parse().unwrap()
            }
        )
        .is_ok()
    );
}

#[test]
fn actual_applied_child_candidate_is_reused_after_lost_checkpoint_for_both_roles() {
    for primary in [false, true] {
        let f = Fixture::new();
        f.remote();
        let base = if primary {
            f.primary()
        } else {
            f.context.head().unwrap().target().unwrap()
        };
        write(&f.context, "ordinary.txt", "same independent edit\n");
        commit(&f.context);
        let incoming = commit_on(&f.repo, base, "ordinary.txt", b"same independent edit\n");
        let req = f.request();
        let mut io = ConflictIo {
            inner: FakeIo::default(),
            incoming,
            role_primary: primary,
        };
        f.db().execute_batch("CREATE TRIGGER child_apply_fault BEFORE UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied' BEGIN SELECT RAISE(ABORT,'fixed checkpoint fault'); END;").unwrap();
        let first = f.run_io(req.clone(), &mut io);
        if primary {
            assert_eq!(
                authoritative(first.unwrap()).publication(),
                PromotionPublication::Pending {
                    reason: PromotionPublicationPending::Synchronization(
                        PromotionSynchronizationCategory::RecoveryRequired
                    )
                }
            );
        } else {
            assert!(matches!(
                first,
                Err(DocumentPromotionError::ContextSynchronization(_))
            ));
        }
        let r = f.stored(req.operation_id);
        let child = if primary {
            r.primary_sync_id.unwrap()
        } else {
            r.context_sync_id.unwrap()
        };
        let candidate=f.db().query_row("SELECT s.candidate_oid FROM remote_integration_steps s JOIN remote_operation_records r ON r.id=s.operation_record_id WHERE r.operation_ulid=?1 AND s.candidate_oid IS NOT NULL",[child.to_string()],|r|r.get::<_,String>(0)).unwrap();
        assert_eq!(
            git2::Repository::open(if primary {
                f.root.path()
            } else {
                f.context.workdir().unwrap()
            })
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap()
            .to_string(),
            candidate
        );
        f.db()
            .execute_batch("DROP TRIGGER child_apply_fault")
            .unwrap();
        outcome(f.run_io(req.clone(), &mut io).unwrap());
        let r = f.stored(req.operation_id);
        assert_eq!(
            if primary {
                r.primary_sync_id
            } else {
                r.context_sync_id
            },
            Some(child)
        );
        assert_eq!(f.db().query_row("SELECT s.candidate_oid FROM remote_integration_steps s JOIN remote_operation_records r ON r.id=s.operation_record_id WHERE r.operation_ulid=?1 AND s.candidate_oid IS NOT NULL",[child.to_string()],|r|r.get::<_,String>(0)).unwrap(),candidate);
        assert_eq!(promotion_commits(&f.repo), 1);
    }
}
#[test]
fn runner_local_divergence_preserves_prepared_primary_tree_and_complete_discovery() {
    let f = Fixture::new();
    write(&f.repo, "ordinary.txt", "independent primary\n");
    commit(&f.repo);
    let result = outcome(f.run(f.request()).unwrap());
    assert!(matches!(
        result.integration(),
        PromotionIntegration::Merged { .. }
    ));
    assert_eq!(
        fs::read(f.root.path().join("ordinary.txt")).unwrap(),
        b"independent primary\n"
    );
    assert_primary_discovery(&f, true);
}

#[test]
fn integrating_retry_with_partial_checkout_is_pending_until_verified_baseline_repair() {
    let f = Fixture::new();
    let req = f.request();
    let base = f.primary();
    let stopped = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::BeforeIntegrationRef,
    )
    .unwrap();
    assert!(
        stopped
            .promote_document(req.clone(), &mut SessionCredentials::new(NoCredentials))
            .is_err()
    );
    let candidate = f.stored(req.operation_id).integration_oid.unwrap();
    f.repo
        .checkout_tree(
            &f.repo.find_object(candidate, None).unwrap(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )
        .unwrap();
    assert_eq!(f.primary(), base);
    let before = directory_image(f.root.path());
    let pending = authoritative(f.run(req.clone()).unwrap());
    assert_eq!(
        pending.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert!(before == directory_image(f.root.path()));
    // Disposable fixture's verified operator baseline repair; never performed
    // automatically by the production retry path.
    checkout(&f.repo, base);
    outcome(f.run(req).unwrap());
    assert_eq!(promotion_commits(&f.repo), 1);
}
#[test]
fn already_current_publication_is_stored_and_replayed_without_children_or_deletion() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let mut io = FakeIo {
        already_current: true,
        ..Default::default()
    };
    let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        result.publication(),
        PromotionPublication::AlreadyCurrent { oid: f.primary() }
    );
    let calls = io.calls.borrow().children.len();
    let deletes = io.calls.borrow().deletes;
    assert_eq!(outcome(f.run_io(req, &mut io).unwrap()), result);
    assert_eq!(io.calls.borrow().children.len(), calls);
    assert_eq!(io.calls.borrow().deletes, deletes);
}
struct AmbiguousDeleteIo {
    inner: FakeIo,
    deleted: bool,
    observations: usize,
}
impl run::PromotionIo for AmbiguousDeleteIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        o: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        self.inner.synchronize(s, req, o, execution)
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        _: &RemoteRefPlan,
        _: &RemoteReservation,
        _: &ObservationConfiguration,
        _: Oid,
        _: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.observations += 1;
        if self.deleted {
            return Ok(DeleteObservation::Deleted);
        }
        self.deleted = true;
        local_effect(s, &req.root, "remote_delete");
        Err(transport_error(SshTransportErrorKind::TransportUnavailable))
    }
}
#[test]
fn ambiguous_delete_result_is_pending_then_observe_only_retry_finishes() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let mut io = AmbiguousDeleteIo {
        inner: FakeIo::default(),
        deleted: false,
        observations: 0,
    };
    let before = history_image(&f);
    let result = authoritative(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RemoteDeletionFailed(
                PromotionRemoteDeletionCategory::Transport
            )
        }
    );
    assert!(before == history_image(&f));
    assert!(f.context.workdir().unwrap().exists());
    outcome(f.run_io(req, &mut io).unwrap());
    assert_eq!(io.observations, 2);
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM promotion_test_effects WHERE effect='remote_delete'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

struct CancelReleasedIo {
    inner: ConflictIo,
    cancel_continuation: bool,
    cancelled_id: Option<OperationId>,
    reached_before_fetch: Rc<std::cell::Cell<bool>>,
}
impl run::PromotionIo for CancelReleasedIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let selected = (req.target == SynchronizationTarget::Primary) == self.inner.role_primary;
        if selected && (self.cancel_continuation || self.cancelled_id == Some(req.operation_id)) {
            struct CancelBeforeFetch {
                cancel: bool,
                reached: Rc<std::cell::Cell<bool>>,
            }
            impl SynchronizationObserver for CancelBeforeFetch {
                fn safe_point(&mut self, p: RemoteOperationSafePoint) -> SafePointRequest {
                    if p == RemoteOperationSafePoint::BeforeFetch {
                        self.reached.set(true);
                        if self.cancel {
                            return SafePointRequest::Cancel;
                        }
                    }
                    SafePointRequest::Continue
                }
            }
            let cancel = self.cancel_continuation;
            self.cancel_continuation = false;
            if cancel {
                self.cancelled_id = Some(req.operation_id);
            }
            return s.synchronize_promotion_child_observed(
                req,
                &mut SessionCredentials::new(NoCredentials),
                &mut CancelBeforeFetch {
                    cancel,
                    reached: self.reached_before_fetch.clone(),
                },
                execution,
            );
        }
        self.inner.synchronize(s, req, observer, execution)
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        config: &ObservationConfiguration,
        oid: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.inner.delete(s, req, plan, owner, config, oid, item)
    }
}
fn spec3_cancel_after_released_resolution(primary: bool) {
    let f = Fixture::new();
    if primary {
        checkout(&f.repo, f.context.head().unwrap().target().unwrap());
    }
    f.remote();
    let base = if primary {
        f.primary()
    } else {
        f.context.head().unwrap().target().unwrap()
    };
    let source = fs::read_to_string(f.context.workdir().unwrap().join(DOC)).unwrap();
    let local = source.replace("private body sentinel", "local released resolution");
    write(&f.context, DOC, &local);
    commit(&f.context);
    let incoming = commit_on(
        &f.repo,
        base,
        DOC,
        source
            .replace("private body sentinel", "incoming released resolution")
            .as_bytes(),
    );
    let req = f.request();
    let reached = Rc::new(std::cell::Cell::new(false));
    let mut io = CancelReleasedIo {
        inner: ConflictIo {
            inner: FakeIo::default(),
            incoming,
            role_primary: primary,
        },
        cancel_continuation: false,
        cancelled_id: None,
        reached_before_fetch: reached.clone(),
    };
    let first = f.run_io(req.clone(), &mut io);
    if primary {
        assert_eq!(
            authoritative(first.unwrap()).publication(),
            PromotionPublication::Pending {
                reason: PromotionPublicationPending::Synchronization(
                    PromotionSynchronizationCategory::Conflict
                )
            }
        );
    } else {
        assert!(
            matches!(first,Err(DocumentPromotionError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::ConflictPending {..}))
        );
    }
    let parent = f.stored(req.operation_id);
    let role = if primary {
        record::ChildRole::Primary
    } else {
        record::ChildRole::Context
    };
    let old = if primary {
        parent.primary_sync_id.unwrap()
    } else {
        parent.context_sync_id.unwrap()
    };
    let inspection = f
        .service
        .inspect_synchronization_recovery(f.root.path(), old)
        .unwrap();
    assert!(matches!(
        f.service
            .resolve_synchronization(ResolveSynchronizationRequest::new(
                f.root.path().into(),
                old,
                OperationId::new(),
                inspection.observation,
                vec![(
                    inspection.paths[0].token.clone(),
                    RedactedConflictBytes::from_bytes(local.into_bytes())
                )],
                None
            ))
            .unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
    state::with_transaction(&f.service, f.root.path(), |tx, _| {
        assert!(matches!(
            record::child_state(tx, &parent, role)?,
            record::ChildState::Unowned(_)
        ));
        Ok(())
    })
    .unwrap();
    io.cancel_continuation = true;
    let cancelled = f.run_io(req.clone(), &mut io);
    if primary {
        assert_eq!(
            authoritative(cancelled.unwrap()).publication(),
            PromotionPublication::Pending {
                reason: PromotionPublicationPending::Cancelled
            }
        );
    } else {
        assert!(
            matches!(cancelled,Err(DocumentPromotionError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::Interrupted))
        );
    }
    assert!(
        reached.get(),
        "real child must cancel at BeforeFetch after fully releasing resolution"
    );
    assert_eq!(
        f.db()
            .query_row(
                "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                [old.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "cancelled"
    );
    state::with_transaction(&f.service, f.root.path(), |tx, _| {
        assert!(matches!(
            record::child_state(tx, &parent, role)?,
            record::ChildState::Unowned(_)
        ));
        Ok(())
    })
    .unwrap();
    let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Complete,
        "terminal cancelled released child must not strand its parent"
    );
    let parent = f.stored(req.operation_id);
    assert_ne!(
        if primary {
            parent.primary_sync_id.unwrap()
        } else {
            parent.context_sync_id.unwrap()
        },
        old
    );
    assert_eq!(promotion_commits(&f.repo), 1);
    f.assert_cleaned();
}
#[test]
fn spec3_terminal_cancelled_released_resolution_context_uses_new_attempt() {
    spec3_cancel_after_released_resolution(false);
}
#[test]
fn spec3_terminal_cancelled_released_resolution_primary_uses_new_attempt() {
    spec3_cancel_after_released_resolution(true);
}

fn operation_image(
    f: &Fixture,
    op: OperationId,
) -> Vec<(String, Vec<Vec<rusqlite::types::Value>>)> {
    [
        ("document_promotions", "operation_id"),
        ("remote_operation_records", "operation_ulid"),
    ]
    .iter()
    .map(|(table, column)| {
        let db = f.db();
        let mut q = db
            .prepare(&format!(
                "SELECT * FROM {table} WHERE {column}=?1 ORDER BY rowid"
            ))
            .unwrap();
        let columns = q.column_count();
        let rows = q
            .query_map([op.to_string()], |r| {
                (0..columns)
                    .map(|c| r.get(c))
                    .collect::<Result<Vec<rusqlite::types::Value>, _>>()
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        ((*table).to_owned(), rows)
    })
    .collect()
}
fn spec3_refused_replacement(blocked: bool) {
    let f = Fixture::new();
    f.remote();
    let old = f.request();
    let mut io = FakeIo {
        deletion: Some(DeleteObservation::Changed(f.primary())),
        ..Default::default()
    };
    assert_eq!(
        authoritative(f.run_io(old.clone(), &mut io).unwrap()).cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RemoteContextChanged
        }
    );
    write(&f.context, "confirmed-replacement", "new context content\n");
    commit(&f.context);
    let mut replacement = f.request();
    if blocked {
        write(&f.context, "unconfirmed-blocker", "must stay\n");
    } else {
        replacement.expected = PromotionExpectation::from_bytes([0; 32]);
    }
    let prior = operation_image(&f, old.operation_id);
    let git = directory_image(f.root.path());
    let history = history_image(&f);
    let counts = effect_counts(&f);
    let refused = f.run_io(replacement.clone(), &mut io);
    if blocked {
        assert!(matches!(refused, Err(DocumentPromotionError::Blocked(_))));
    } else {
        assert!(matches!(
            refused,
            Err(DocumentPromotionError::ExternalChange)
        ));
    }
    assert!(
        prior == operation_image(&f, old.operation_id),
        "refused consent must preserve the entire old parent/reservation including owner epoch"
    );
    assert!(
        git == directory_image(f.root.path()),
        "refused replacement must preserve all Git/worktree bytes"
    );
    assert!(history == history_image(&f));
    assert_eq!(counts, effect_counts(&f));
    assert_eq!(
        f.stored(replacement.operation_id).step,
        record::PromotionStep::Abandoned
    );
    assert_eq!(
        f.stored(old.operation_id).step,
        record::PromotionStep::Published
    );
    if blocked {
        fs::remove_file(f.context.workdir().unwrap().join("unconfirmed-blocker")).unwrap();
    }
    // The original operation is still a live continuation, not a superseded
    // stored outcome. Its expected old context now observes our new commit.
    assert_eq!(
        authoritative(f.run_io(old.clone(), &mut io).unwrap()).cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::ContextChanged
        }
    );
    let valid = f.request();
    let mut synced = ConflictIo {
        inner: io,
        incoming: f.context.head().unwrap().target().unwrap(),
        role_primary: false,
    };
    assert_eq!(
        outcome(f.run_io(valid, &mut synced).unwrap()).cleanup(),
        PromotionCleanup::Complete
    );
    assert_eq!(
        f.stored(old.operation_id).step,
        record::PromotionStep::Superseded
    );
    assert_eq!(promotion_commits(&f.repo), 2);
}
#[test]
fn spec3_stale_replacement_consent_preserves_old_parent_and_reservation() {
    spec3_refused_replacement(false);
}
#[test]
fn spec3_blocked_replacement_consent_preserves_old_parent_and_reservation() {
    spec3_refused_replacement(true);
}

#[test]
fn spec3_branch_deleted_reopen_preserves_reappeared_local_state_before_retirement() {
    for variant in [
        "old_branch",
        "moved_branch",
        "symbolic_branch",
        "path_only",
        "admin_only",
        "registered_worktree",
    ] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let mut io = FakeIo::default();
        let stopped = RepositoryService::open_at_with_failure_point_for_testing(
            f.data.path(),
            FailurePoint::BeforeCompletion,
        )
        .unwrap();
        run::promote_with_io(&stopped, req.clone(), &mut io).unwrap();
        let old = f.stored(req.operation_id);
        assert_eq!(old.step, record::PromotionStep::BranchDeleted);
        let branch = format!("refs/heads/manyhands/document/{ITEM}");
        let path = f.root.path().join(".manyhands/worktrees").join(ITEM);
        let admin = f.repo.commondir().join("worktrees").join(ITEM);
        match variant {
            "old_branch" => {
                f.repo
                    .reference(
                        &branch,
                        old.context_oid.unwrap(),
                        true,
                        "fixture new branch",
                    )
                    .unwrap();
            }
            "moved_branch" => {
                f.repo
                    .reference(&branch, f.primary(), true, "fixture moved branch")
                    .unwrap();
            }
            "symbolic_branch" => {
                f.repo
                    .reference_symbolic(&branch, "refs/heads/main", true, "fixture indirect branch")
                    .unwrap();
            }
            "path_only" => {
                fs::create_dir_all(&path).unwrap();
                fs::write(path.join("new-owner-data"), "preserve local work\n").unwrap();
            }
            "admin_only" => {
                fs::create_dir_all(&admin).unwrap();
                fs::write(admin.join("HEAD"), format!("ref: {branch}\n")).unwrap();
            }
            _ => {
                let reference = f
                    .repo
                    .reference(&branch, f.primary(), true, "fixture fresh context")
                    .unwrap();
                let mut options = git2::WorktreeAddOptions::new();
                options.reference(Some(&reference));
                f.repo.worktree(ITEM, &path, Some(&options)).unwrap();
            }
        }
        let git = directory_image(f.root.path());
        let history = history_image(&f);
        let counts = effect_counts(&f);
        let reopened = RepositoryService::open_at(f.data.path()).unwrap();
        let result = authoritative(run::promote_with_io(&reopened, req.clone(), &mut io).unwrap());
        assert!(
            matches!(
                result.cleanup(),
                PromotionCleanup::Pending {
                    reason: PromotionCleanupPending::ContextChanged
                        | PromotionCleanupPending::RecoveryRequired
                }
            ),
            "{variant}: {result:?}"
        );
        assert_eq!(
            f.stored(req.operation_id).step,
            record::PromotionStep::BranchDeleted
        );
        assert!(
            git == directory_image(f.root.path()),
            "reappeared local cleanup state must be preserved: {variant}"
        );
        assert!(history == history_image(&f));
        assert_eq!(counts, effect_counts(&f));
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_ne!(
            f.db()
                .query_row(
                    "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                    [req.operation_id.to_string()],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "completed"
        );
    }
}
#[test]
fn spec3_reappeared_local_branch_after_tracking_delete_stops_final_sql() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let root = f.root.path().to_owned();
    let oid = f.primary();
    let history = history_image(&f);
    TRACKING_HOOKS.with(|h| {
        h.borrow_mut().insert(
            root.clone(),
            Box::new(move |_| {
                git2::Repository::open(&root)
                    .unwrap()
                    .reference(
                        &format!("refs/heads/manyhands/document/{ITEM}"),
                        oid,
                        true,
                        "fixture late local state",
                    )
                    .unwrap();
            }),
        )
    });
    let mut io = FakeIo::default();
    let result = authoritative(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert_eq!(
        f.stored(req.operation_id).step,
        record::PromotionStep::BranchDeleted
    );
    assert!(history == history_image(&f));
    assert_eq!(
        f.repo
            .refname_to_id(&format!("refs/heads/manyhands/document/{ITEM}"))
            .unwrap(),
        oid
    );
    assert!(
        f.repo
            .find_reference(&format!("refs/remotes/origin/manyhands/document/{ITEM}"))
            .is_err()
    );
}
#[test]
fn spec3_complete_replay_preserves_a_fresh_context_created_after_completion() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let mut io = FakeIo::default();
    let done = outcome(f.run_io(req.clone(), &mut io).unwrap());
    f.service
        .prepare_context(AuthoringTarget {
            root: f.root.path().into(),
            kind: AuthoringKind::Document,
            item_id: ITEM.parse().unwrap(),
            intent: ContextIntent::Edit,
            operation_id: OperationId::new(),
        })
        .unwrap();
    let git = directory_image(f.root.path());
    let counts = effect_counts(&f);
    let reopened = RepositoryService::open_at(f.data.path()).unwrap();
    assert_eq!(
        outcome(run::promote_with_io(&reopened, req, &mut io).unwrap()),
        done
    );
    assert!(git == directory_image(f.root.path()));
    assert_eq!(counts, effect_counts(&f));
    assert!(
        f.root
            .path()
            .join(".manyhands/worktrees")
            .join(ITEM)
            .join(DOC)
            .exists()
    );
}

fn spec4_assert_unleased_scans(f: &Fixture) -> Rc<std::cell::Cell<usize>> {
    let count = Rc::new(std::cell::Cell::new(0));
    for root in [
        f.root.path().to_owned(),
        f.root.path().join(".manyhands/worktrees").join(ITEM),
    ] {
        let data = f.data.path().to_owned();
        let checked = root.clone();
        let count = count.clone();
        discovery::set_scan_hook(root, move || {
            assert!(
                coordination::leases_are_free(&data, &checked),
                "actual filesystem discovery entry must not run under the common lease"
            );
            count.set(count.get() + 1);
        });
    }
    count
}
#[test]
fn spec4_new_success_digest_refusal_and_blocker_scan_without_common_lease() {
    for mode in ["success", "digest", "blocker"] {
        let f = Fixture::new();
        let mut req = f.request();
        if mode == "digest" {
            req.expected = PromotionExpectation::from_bytes([0; 32]);
        }
        if mode == "blocker" {
            write(&f.context, "untracked", "preserve");
        }
        let count = spec4_assert_unleased_scans(&f);
        let result = f.run(req);
        match mode {
            "success" => {
                outcome(result.unwrap());
            }
            "digest" => assert!(matches!(
                result,
                Err(DocumentPromotionError::ExternalChange)
            )),
            _ => assert!(matches!(result, Err(DocumentPromotionError::Blocked(_)))),
        }
        assert!(count.get() > 0, "actual discovery entry probe must execute");
    }
}
#[test]
fn spec4_integration_preparation_scans_before_taking_its_common_lease() {
    let f = Fixture::new();
    let input = integrate::IntegrationRequest::from_preview(&f.preview());
    let count = spec4_assert_unleased_scans(&f);
    let prepared = integrate::prepare_integration(&f.service, f.root.path(), &input, None).unwrap();
    assert!(count.get() > 0);
    drop(prepared);
}
#[test]
fn spec4_real_context_child_scans_without_common_lease_before_its_safe_point() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let count = spec4_assert_unleased_scans(&f);
    let mut io = CancelReleasedIo {
        inner: ConflictIo {
            inner: FakeIo::default(),
            incoming: f.context.head().unwrap().target().unwrap(),
            role_primary: false,
        },
        cancel_continuation: true,
        cancelled_id: None,
        reached_before_fetch: Rc::new(std::cell::Cell::new(false)),
    };
    assert!(
        matches!(f.run_io(req,&mut io),Err(DocumentPromotionError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::Interrupted))
    );
    assert!(io.reached_before_fetch.get());
    assert!(count.get() > 0);
}
#[test]
fn spec4_integration_outside_inside_changes_refuse_without_owned_objects_or_effects() {
    for mode in ["context_ref", "status", "config", "identity"] {
        let f = Fixture::new();
        let input = integrate::IntegrationRequest::from_preview(&f.preview());
        let root = f.root.path().to_owned();
        let saved = Rc::new(RefCell::new(None));
        let observed = saved.clone();
        let count = spec4_assert_unleased_scans(&f);
        PREPARED_OBSERVATION_HOOKS.with(|h| {
            h.borrow_mut().insert(
                root.clone(),
                Box::new(move |_| {
                    let path = root.join(".manyhands/worktrees").join(ITEM);
                    match mode {
                        "context_ref" => {
                            let linked = git2::Repository::open(&path).unwrap();
                            write(&linked, "raced-content", "preserve");
                            commit(&linked);
                        }
                        "status" => {
                            fs::write(path.join("raced-untracked"), "preserve").unwrap();
                        }
                        "config" => {
                            let path = root.join(".manyhands/config.toml");
                            fs::write(
                                &path,
                                fs::read_to_string(&path).unwrap().replace("main", "other"),
                            )
                            .unwrap();
                        }
                        _ => {
                            fs::write(path.join(".git"), "gitdir: unavailable-fixture-admin\n")
                                .unwrap();
                        }
                    }
                    *observed.borrow_mut() = Some(directory_image(&root));
                }),
            )
        });
        assert!(
            matches!(
                integrate::prepare_integration(&f.service, f.root.path(), &input, None),
                Err(DocumentPromotionError::ExternalChange)
            ),
            "{mode}"
        );
        assert!(count.get() > 0);
        assert!(saved.borrow().as_ref().unwrap() == &directory_image(f.root.path()));
        assert_eq!(promotion_commits(&f.repo), 0);
    }
}
#[test]
fn spec4_integration_reobserves_tracking_primary_after_unleased_discovery() {
    let f = Fixture::new();
    f.remote();
    let input = integrate::IntegrationRequest::from_preview(&f.preview());
    let root = f.root.path().to_owned();
    let saved = Rc::new(RefCell::new(None));
    let observed = saved.clone();
    let count = spec4_assert_unleased_scans(&f);
    PREPARED_OBSERVATION_HOOKS.with(|h| {
        h.borrow_mut().insert(
            root.clone(),
            Box::new(move |_| {
                let repo = git2::Repository::open(&root).unwrap();
                let base = repo.refname_to_id("refs/remotes/origin/main").unwrap();
                let advanced = commit_on(&repo, base, "ordinary.txt", b"primary-only race\n");
                repo.reference("refs/remotes/origin/main", advanced, true, "fixture poll")
                    .unwrap();
                *observed.borrow_mut() = Some(directory_image(&root));
            }),
        )
    });
    assert!(matches!(
        integrate::prepare_integration(&f.service, f.root.path(), &input, None),
        Err(DocumentPromotionError::PrimaryAdvanced)
    ));
    assert!(count.get() > 0);
    assert!(saved.borrow().as_ref().unwrap() == &directory_image(f.root.path()));
    assert_eq!(promotion_commits(&f.repo), 0);
}
#[test]
fn spec4_outside_inside_races_refuse_without_superseding_the_old_owner() {
    for mode in [
        "primary_ref",
        "context_ref",
        "status",
        "config",
        "identity",
        "record",
        "owner",
    ] {
        let f = Fixture::new();
        f.remote();
        let old = f.request();
        let mut io = FakeIo {
            deletion: Some(DeleteObservation::Changed(f.primary())),
            ..Default::default()
        };
        f.run_io(old.clone(), &mut io).unwrap();
        write(&f.context, "new-confirmed", "new content");
        commit(&f.context);
        let req = f.request();
        let observed = Rc::new(RefCell::new(None));
        let saved = observed.clone();
        let root = f.root.path().to_owned();
        let data = f.data.path().to_owned();
        let op = old.operation_id;
        PREPARED_OBSERVATION_HOOKS.with(|h|h.borrow_mut().insert(root.clone(),Box::new(move |s| {
            let repo=git2::Repository::open(&root).unwrap();let context=root.join(".manyhands/worktrees").join(ITEM);
            match mode {
                "primary_ref"=>{write(&repo,"race-primary","external advance");commit(&repo);},
                "context_ref"=>{let linked=git2::Repository::open(&context).unwrap();write(&linked,"race-context","external advance");commit(&linked);},
                "status"=>{fs::write(context.join("race-untracked"),"external data").unwrap();},
                "config"=>{let path=root.join(".manyhands/config.toml");fs::write(&path,fs::read_to_string(&path).unwrap().replace("origin","changed")).unwrap();},
                "identity"=>{fs::write(context.join(".git"),"gitdir: unavailable-fixture-admin\n").unwrap();},
                "record"=>{rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap().execute("UPDATE document_promotions SET cleanup_category='context_changed' WHERE operation_id=?1",[op.to_string()]).unwrap();},
                _=>{rusqlite::Connection::open(data.join(REGISTRY_FILE)).unwrap().execute("UPDATE remote_operation_records SET owner_epoch=owner_epoch+1 WHERE operation_ulid=?1",[op.to_string()]).unwrap();},
            }
            let row=state::with_transaction(s,&root,|tx,id|Ok((record::read_record(tx,id,op)?.unwrap().step,state::read_operation(tx,id,op)?.unwrap().owner_epoch))).unwrap();
            *saved.borrow_mut()=Some((directory_image(&root),row));
        })));
        let counts = effect_counts(&f);
        let history = history_image(&f);
        let result = f.run_io(req, &mut io);
        assert!(
            observed.borrow().is_some(),
            "outside/inside race seam must execute"
        );
        assert!(
            matches!(result, Err(DocumentPromotionError::ExternalChange)),
            "{mode}: {result:?}"
        );
        let saved = observed.borrow();
        let (git, row) = saved.as_ref().unwrap();
        assert!(
            *git == directory_image(f.root.path()),
            "raced Git/local bytes must be preserved: {mode}"
        );
        let current = state::with_transaction(&f.service, f.root.path(), |tx, id| {
            Ok((
                record::read_record(tx, id, old.operation_id)?.unwrap().step,
                state::read_operation(tx, id, old.operation_id)?
                    .unwrap()
                    .owner_epoch,
            ))
        })
        .unwrap();
        assert_eq!(
            current, *row,
            "raced predecessor/owner must not be superseded: {mode}"
        );
        assert_eq!(counts, effect_counts(&f));
        assert!(history == history_image(&f));
    }
}

fn spec5_context_entry_mode(f: &Fixture, mode: u32) -> (Oid, Vec<u8>) {
    let source = fs::read(f.context.workdir().unwrap().join(DOC)).unwrap();
    let parent = f.context.head().unwrap().peel_to_commit().unwrap();
    let mut index = git2::Index::new().unwrap();
    index.read_tree(&parent.tree().unwrap()).unwrap();
    let mut entry = index.get_path(std::path::Path::new(DOC), 0).unwrap();
    entry.mode = mode;
    if mode == 0o160000 {
        entry.id = parent.id();
    }
    index.add(&entry).unwrap();
    let tree = f
        .context
        .find_tree(index.write_tree_to(&f.context).unwrap())
        .unwrap();
    let sig = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let oid = f
        .context
        .commit(
            None,
            &sig,
            &sig,
            "fixture canonical entry shape",
            &tree,
            &[&parent],
        )
        .unwrap();
    let mut config = f.repo.config().unwrap();
    config.set_bool("core.symlinks", cfg!(unix)).unwrap();
    config.set_bool("core.filemode", cfg!(unix)).unwrap();
    let fresh = git2::Repository::open(f.context.workdir().unwrap()).unwrap();
    checkout(&fresh, oid);
    let actual = fresh
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(std::path::Path::new(DOC))
        .unwrap();
    assert_eq!(actual.filemode() as u32, mode);
    assert_eq!(
        actual.kind(),
        Some(if mode == 0o160000 {
            git2::ObjectType::Commit
        } else {
            git2::ObjectType::Blob
        })
    );
    if mode == 0o120000 {
        assert!(
            matches!(crate::canonical::parse_item(std::path::Path::new(DOC),std::str::from_utf8(&source).unwrap()).unwrap(),crate::canonical::CanonicalItem::Document(d) if d.id.to_string()==ITEM)
        );
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let path = f.context.workdir().unwrap().join(DOC);
            assert!(
                fs::symlink_metadata(&path)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
            assert_eq!(fs::read_link(path).unwrap().as_os_str().as_bytes(), source);
        }
    }
    (oid, source)
}
type ModeImage = (Oid, Vec<u8>, Option<(bool, bool, Vec<u8>)>);

fn spec5_mode_image(f: &Fixture) -> ModeImage {
    let linked = git2::Repository::open(f.context.workdir().unwrap()).unwrap();
    let path = linked.workdir().unwrap().join(DOC);
    let payload = fs::symlink_metadata(&path).ok().map(|m| {
        let bytes = if m.file_type().is_symlink() {
            fs::read_link(&path)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes()
                .to_vec()
        } else if m.is_file() {
            fs::read(&path).unwrap()
        } else {
            Vec::new()
        };
        (m.file_type().is_symlink(), m.is_dir(), bytes)
    });
    (
        linked.head().unwrap().target().unwrap(),
        fs::read(linked.path().join("index")).unwrap(),
        payload,
    )
}
#[test]
fn spec5_cleanup_symlink_front_matter_and_gitlink_cannot_fulfill_target() {
    for mode in [0o120000, 0o160000] {
        let f = Fixture::new();
        let (oid, _) = spec5_context_entry_mode(&f, mode);
        let before = spec5_mode_image(&f);
        let cleanup = crate::repository::cleanup::ContextCleanup::acquire(f.root.path()).unwrap();
        assert_eq!(
            cleanup.precheck_context(AuthoringKind::Document, &ITEM.parse().unwrap(), oid),
            crate::repository::cleanup::Precheck::Changed,
            "mode {mode:o}"
        );
        assert_eq!(
            cleanup.remove_context_worktree(AuthoringKind::Document, &ITEM.parse().unwrap(), oid),
            crate::repository::cleanup::WorktreeRemoval::Changed
        );
        assert!(before == spec5_mode_image(&f));
        assert!(f.repo.find_worktree(ITEM).is_ok());
    }
}
#[test]
fn spec5_sync_symlink_front_matter_and_gitlink_cannot_fulfill_target() {
    for mode in [0o120000, 0o160000] {
        let f = Fixture::new();
        f.remote();
        let (oid, _) = spec5_context_entry_mode(&f, mode);
        let before = spec5_mode_image(&f);
        let req = SynchronizeRemoteRequest {
            root: f.root.path().into(),
            operation_id: OperationId::new(),
            target: SynchronizationTarget::Context {
                kind: AuthoringKind::Document,
                item_id: ITEM.parse().unwrap(),
            },
            approval: None,
            confirmed_identity: None,
            restart: false,
        };
        assert!(
            matches!(
                remote::sync::promotion_test_drive_local(&f.service, &req, oid),
                Err(SynchronizationError::TargetNotMaterialized)
            ),
            "mode {mode:o}"
        );
        assert!(before == spec5_mode_image(&f));
        assert_eq!(promotion_commits(&f.repo), 0);
    }
}
#[test]
fn spec5_cleanup_and_sync_accept_regular_and_executable_canonical_blobs() {
    for mode in [0o100644, 0o100755] {
        let f = Fixture::new();
        f.remote();
        let (oid, _) = spec5_context_entry_mode(&f, mode);
        let before = spec5_mode_image(&f);
        let cleanup = crate::repository::cleanup::ContextCleanup::acquire(f.root.path()).unwrap();
        assert_eq!(
            cleanup.precheck_context(AuthoringKind::Document, &ITEM.parse().unwrap(), oid),
            crate::repository::cleanup::Precheck::Ok,
            "mode {mode:o}"
        );
        drop(cleanup);
        let req = SynchronizeRemoteRequest {
            root: f.root.path().into(),
            operation_id: OperationId::new(),
            target: SynchronizationTarget::Context {
                kind: AuthoringKind::Document,
                item_id: ITEM.parse().unwrap(),
            },
            approval: None,
            confirmed_identity: None,
            restart: false,
        };
        assert_eq!(
            remote::sync::promotion_test_drive_local(&f.service, &req, oid).unwrap(),
            oid
        );
        assert!(before == spec5_mode_image(&f));
    }
}

struct Quality1BarrierIo {
    inner: ConflictIo,
    ready: Option<std::sync::mpsc::Sender<OperationId>>,
    release: Option<std::sync::mpsc::Receiver<()>>,
    at_reservation: bool,
}
impl run::PromotionIo for Quality1BarrierIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        o: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        if (req.target == SynchronizationTarget::Primary) == self.inner.role_primary
            && let (Some(ready), Some(release)) = (self.ready.take(), self.release.take())
        {
            let id = req.operation_id;
            let pause = move || {
                ready.send(id).unwrap();
                release
                    .recv_timeout(std::time::Duration::from_secs(120))
                    .unwrap();
            };
            if self.at_reservation {
                crate::repository::remote::reservation::set_promotion_reservation_hook(id, pause);
            } else {
                pause();
            }
        }
        self.inner.synchronize(s, req, o, execution)
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        config: &ObservationConfiguration,
        oid: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.inner.delete(s, req, plan, owner, config, oid, item)
    }
}
fn quality1_binding_race(primary: bool, candidate: bool, at_reservation: bool) {
    let f = Fixture::new();
    if primary {
        checkout(&f.repo, f.context.head().unwrap().target().unwrap());
    }
    f.remote();
    let base = if primary {
        f.primary()
    } else {
        f.context.head().unwrap().target().unwrap()
    };
    let source = fs::read_to_string(f.context.workdir().unwrap().join(DOC)).unwrap();
    let local = source.replace("private body sentinel", "current local contribution");
    write(&f.context, DOC, &local);
    commit(&f.context);
    let incoming_source = if candidate {
        local.clone()
    } else {
        source.replace("private body sentinel", "incoming conflicting contribution")
    };
    let incoming = commit_on(&f.repo, base, DOC, incoming_source.as_bytes());
    let req = f.request();
    if candidate {
        f.db().execute_batch("CREATE TRIGGER quality1_apply_fault BEFORE UPDATE OF phase ON remote_integration_steps WHEN NEW.phase='applied' BEGIN SELECT RAISE(ABORT,'fixed checkpoint fault'); END;").unwrap();
    }
    let (a_ready_tx, a_ready) = std::sync::mpsc::channel();
    let (a_release, a_release_rx) = std::sync::mpsc::channel();
    let (b_ready_tx, b_ready) = std::sync::mpsc::channel();
    let (b_release, b_release_rx) = std::sync::mpsc::channel();
    let spawn = |request: PromoteDocumentRequest, data: PathBuf, ready, release, at_reservation| {
        std::thread::spawn(move || {
            let service = RepositoryService::open_at(&data).unwrap();
            let mut io = Quality1BarrierIo {
                inner: ConflictIo {
                    inner: FakeIo::default(),
                    incoming,
                    role_primary: primary,
                },
                ready: Some(ready),
                release: Some(release),
                at_reservation,
            };
            let _result = run::promote_with_io(&service, request, &mut io);
        })
    };
    let a = spawn(
        req.clone(),
        f.data.path().into(),
        a_ready_tx,
        a_release_rx,
        at_reservation,
    );
    let c1 = a_ready
        .recv_timeout(std::time::Duration::from_secs(120))
        .unwrap();
    assert!(coordination::leases_are_free(f.data.path(), f.root.path()));
    let b = spawn(
        req.clone(),
        f.data.path().into(),
        b_ready_tx,
        b_release_rx,
        false,
    );
    let c2 = b_ready
        .recv_timeout(std::time::Duration::from_secs(120))
        .unwrap();
    assert_ne!(c1, c2);
    assert!(coordination::leases_are_free(f.data.path(), f.root.path()));
    let before = directory_image(f.root.path());
    a_release.send(()).unwrap();
    a.join().unwrap();
    let stale_row = f
        .db()
        .query_row(
            "SELECT count(*) FROM remote_operation_records WHERE operation_ulid=?1",
            [c1.to_string()],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
    let unchanged = before == directory_image(f.root.path());
    b_release.send(()).unwrap();
    b.join().unwrap(); // Always reap both executors before assertions.
    assert_eq!(
        stale_row, 0,
        "a stale formerly-bound attempt must not become an ordinary synchronization"
    );
    assert!(
        unchanged,
        "stale executor must create no Git objects/ref/worktree effects"
    );
    let parent = f.stored(req.operation_id);
    let bound = if primary {
        parent.primary_sync_id
    } else {
        parent.context_sync_id
    };
    assert_eq!(bound, Some(c2));
    let owned=state::with_transaction(&f.service,f.root.path(),|tx,id| {
        let ids=tx.prepare("SELECT operation_ulid FROM remote_operation_records WHERE action IN ('synchronize_context','synchronize_primary') ORDER BY operation_ulid").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        let mut owned=Vec::new();for op in ids {let op=OperationId::parse(&op).unwrap();let r=state::read_operation(tx,id,op)?.unwrap();if crate::repository::remote::reservation::retains_promotion_child(tx,&r)? {owned.push(op);}}Ok(owned)
    }).unwrap();
    assert_eq!(
        owned,
        vec![c2],
        "all real retained recovery must remain reachable through the parent"
    );
    let mut io = ConflictIo {
        inner: FakeIo::default(),
        incoming,
        role_primary: primary,
    };
    if candidate {
        f.db()
            .execute_batch("DROP TRIGGER quality1_apply_fault")
            .unwrap();
    } else {
        assert!(
            f.service
                .cancel_document_promotion(f.root.path(), req.operation_id)
                .unwrap()
        );
        assert!(f.stored(req.operation_id).cancel_requested);
        // Installed conflicts are already stopped (Interrupted), so the
        // existing cancellation API need not flag an inactive child owner.
        // The retained recovery remains reachable by the parent's current ID.
        let parent = f.stored(req.operation_id);
        state::with_transaction(&f.service, f.root.path(), |tx, _| {
            assert!(matches!(
                record::child_state(
                    tx,
                    &parent,
                    if primary {
                        record::ChildRole::Primary
                    } else {
                        record::ChildRole::Context
                    }
                )?,
                record::ChildState::Retained(_)
            ));
            Ok(())
        })
        .unwrap();
        let _ack = f.run_io(req.clone(), &mut io);
        let inspection = f
            .service
            .inspect_synchronization_recovery(f.root.path(), c2)
            .unwrap();
        assert!(matches!(
            f.service
                .resolve_synchronization(ResolveSynchronizationRequest::new(
                    f.root.path().into(),
                    c2,
                    OperationId::new(),
                    inspection.observation,
                    vec![(
                        inspection.paths[0].token.clone(),
                        RedactedConflictBytes::from_bytes(local.into_bytes())
                    )],
                    None
                ))
                .unwrap(),
            ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
        ));
    }
    assert_eq!(
        outcome(f.run_io(req.clone(), &mut io).unwrap()).cleanup(),
        PromotionCleanup::Complete
    );
    assert_eq!(promotion_commits(&f.repo), 1);
    let parent = f.stored(req.operation_id);
    assert_eq!(
        if primary {
            parent.primary_sync_id
        } else {
            parent.context_sync_id
        },
        Some(c2)
    );
}
#[test]
fn quality1_context_conflict_rejects_stale_unstarted_attempt() {
    quality1_binding_race(false, false, false);
}
#[test]
fn quality1_primary_conflict_rejects_stale_unstarted_attempt() {
    quality1_binding_race(true, false, false);
}
#[test]
fn quality1_context_candidate_rejects_stale_unstarted_attempt() {
    quality1_binding_race(false, true, false);
}
#[test]
fn quality1_primary_candidate_rejects_stale_unstarted_attempt() {
    quality1_binding_race(true, true, false);
}
#[test]
fn quality1_context_atomic_reservation_rechecks_after_early_validation() {
    quality1_binding_race(false, false, true);
}
#[test]
fn quality1_primary_atomic_reservation_rechecks_after_early_validation() {
    quality1_binding_race(true, true, true);
}

#[test]
fn quality1_parent_cancellation_fences_unstarted_children_at_both_handoff_windows() {
    for primary in [false, true] {
        for at_reservation in [false, true] {
            let f = Fixture::new();
            f.remote();
            let req = f.request();
            let data = f.data.path().to_owned();
            let request = req.clone();
            let incoming = f.context.head().unwrap().target().unwrap();
            let (ready_tx, ready) = std::sync::mpsc::channel();
            let (release, release_rx) = std::sync::mpsc::channel();
            let worker = std::thread::spawn(move || {
                let s = RepositoryService::open_at(&data).unwrap();
                let mut io = Quality1BarrierIo {
                    inner: ConflictIo {
                        inner: FakeIo::default(),
                        incoming,
                        role_primary: primary,
                    },
                    ready: Some(ready_tx),
                    release: Some(release_rx),
                    at_reservation,
                };
                let _ = run::promote_with_io(&s, request, &mut io);
            });
            let child = ready
                .recv_timeout(std::time::Duration::from_secs(120))
                .unwrap();
            assert!(coordination::leases_are_free(f.data.path(), f.root.path()));
            assert!(
                f.service
                    .cancel_document_promotion(f.root.path(), req.operation_id)
                    .unwrap()
            );
            let before = directory_image(f.root.path());
            release.send(()).unwrap();
            worker.join().unwrap();
            assert_eq!(
                f.db()
                    .query_row(
                        "SELECT count(*) FROM remote_operation_records WHERE operation_ulid=?1",
                        [child.to_string()],
                        |r| r.get::<_, i64>(0)
                    )
                    .unwrap(),
                0
            );
            assert!(before == directory_image(f.root.path()));
            assert!(f.stored(req.operation_id).cancel_requested);
            let mut io = FakeIo::default();
            assert_eq!(
                outcome(f.run_io(req.clone(), &mut io).unwrap()).cleanup(),
                PromotionCleanup::Complete
            );
            let parent = f.stored(req.operation_id);
            assert_ne!(
                if primary {
                    parent.primary_sync_id
                } else {
                    parent.context_sync_id
                },
                Some(child)
            );
            assert_eq!(promotion_commits(&f.repo), 1);
        }
    }
}
#[test]
fn quality1_confirmed_supersession_fences_late_children_without_ordinary_fallthrough() {
    for primary in [false, true] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let request = req.clone();
        let data = f.data.path().to_owned();
        let incoming = f.context.head().unwrap().target().unwrap();
        let (ready_tx, ready) = std::sync::mpsc::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let s = RepositoryService::open_at(&data).unwrap();
            let mut io = Quality1BarrierIo {
                inner: ConflictIo {
                    inner: FakeIo::default(),
                    incoming,
                    role_primary: primary,
                },
                ready: Some(ready_tx),
                release: Some(release_rx),
                at_reservation: false,
            };
            let _ = run::promote_with_io(&s, request, &mut io);
        });
        let child = ready
            .recv_timeout(std::time::Duration::from_secs(120))
            .unwrap();
        let mut b = FakeIo {
            deletion: Some(DeleteObservation::Changed(f.primary())),
            ..Default::default()
        };
        assert_eq!(
            authoritative(f.run_io(req.clone(), &mut b).unwrap()).cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::RemoteContextChanged
            }
        );
        let replacement = f.request();
        assert_eq!(
            outcome(f.run_io(replacement, &mut b).unwrap()).cleanup(),
            PromotionCleanup::Complete
        );
        assert_eq!(
            f.stored(req.operation_id).step,
            record::PromotionStep::Superseded
        );
        let before = directory_image(f.root.path());
        let counts = effect_counts(&f);
        release.send(()).unwrap();
        worker.join().unwrap();
        assert!(before == directory_image(f.root.path()));
        assert_eq!(counts, effect_counts(&f));
        assert_eq!(
            f.db()
                .query_row(
                    "SELECT count(*) FROM remote_operation_records WHERE operation_ulid=?1",
                    [child.to_string()],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}
#[test]
fn quality1_current_authority_handoff_and_ordinary_synchronization_remain_valid() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let mut io = FakeIo::default();
    outcome(f.run_io(req.clone(), &mut io).unwrap());
    let parent = f.stored(req.operation_id);
    let before = directory_image(f.root.path());
    let counts = effect_counts(&f);
    struct CancelBeforeFetch;
    impl SynchronizationObserver for CancelBeforeFetch {
        fn safe_point(&mut self, p: RemoteOperationSafePoint) -> SafePointRequest {
            if p == RemoteOperationSafePoint::BeforeFetch {
                SafePointRequest::Cancel
            } else {
                SafePointRequest::Continue
            }
        }
    }
    for role in [record::ChildRole::Context, record::ChildRole::Primary] {
        let execution = record::child_execution(&parent, role).unwrap();
        let request = SynchronizeRemoteRequest {
            root: f.root.path().into(),
            operation_id: if role == record::ChildRole::Context {
                parent.context_sync_id.unwrap()
            } else {
                parent.primary_sync_id.unwrap()
            },
            target: if role == record::ChildRole::Context {
                SynchronizationTarget::Context {
                    kind: AuthoringKind::Document,
                    item_id: ITEM.parse().unwrap(),
                }
            } else {
                SynchronizationTarget::Primary
            },
            approval: None,
            confirmed_identity: None,
            restart: false,
        };
        assert!(matches!(
            f.service
                .synchronize_promotion_child_observed(
                    request.clone(),
                    &mut SessionCredentials::new(NoCredentials),
                    &mut CancelBeforeFetch,
                    &execution
                )
                .unwrap(),
            SynchronizationResult::Complete(SynchronizationOutcome::Published { .. })
        ));
        let mut wrong = request;
        wrong.operation_id = OperationId::new();
        assert!(matches!(
            f.service.synchronize_promotion_child_observed(
                wrong,
                &mut SessionCredentials::new(NoCredentials),
                &mut CancelBeforeFetch,
                &execution
            ),
            Err(SynchronizationError::Repository(RepositoryError {
                kind: RepositoryErrorKind::OperationMismatch,
                ..
            }))
        ));
    }
    assert!(before == directory_image(f.root.path()));
    assert_eq!(counts, effect_counts(&f));
    let ordinary = OperationId::new();
    assert!(matches!(
        f.service.synchronize_remote_observed(
            SynchronizeRemoteRequest {
                root: f.root.path().into(),
                operation_id: ordinary,
                target: SynchronizationTarget::Primary,
                approval: None,
                confirmed_identity: None,
                restart: false
            },
            &mut SessionCredentials::new(NoCredentials),
            SynchronizationSettings::default(),
            &mut CancelBeforeFetch
        ),
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(
        f.db()
            .query_row(
                "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                [ordinary.to_string()],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "cancelled"
    );
}

struct Quality2LostReturnCancelIo {
    inner: FakeIo,
    parent: OperationId,
    primary: bool,
    completed: Option<(OperationId, Oid)>,
}
impl run::PromotionIo for Quality2LostReturnCancelIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let selected = (req.target == SynchronizationTarget::Primary) == self.primary;
        let lose_return = selected && self.completed.is_none();
        if lose_return && self.primary {
            // A real later primary tip lets the assertion distinguish the
            // stored publication OID from the parent's integration commit.
            let repo = git2::Repository::open(&req.root).unwrap();
            write(&repo, "ordinary.txt", "publication child contribution\n");
            commit(&repo);
        }
        let id = req.operation_id;
        let root = req.root.clone();
        let result = self.inner.synchronize(s, req, observer, execution)?;
        if lose_return {
            let completed = state::with_transaction(s, &root, |tx, repository_id| {
                let record = state::read_operation(tx, repository_id, id)?
                    .ok_or_else(state::recovery_required)?;
                assert_eq!(record.phase, RemoteOperationPhase::Completed);
                Ok(record.authority.ok_or_else(state::recovery_required)?.oid())
            })
            .unwrap();
            self.completed = Some((id, completed));
            assert!(s.cancel_document_promotion(&root, self.parent).unwrap());
            // Existing lost-result seam: the durable authority above wins
            // over both this injected transport error and parent cancellation.
            return Err(transport_error(SshTransportErrorKind::TransportUnavailable));
        }
        Ok(result)
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        config: &ObservationConfiguration,
        oid: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.inner.delete(s, req, plan, owner, config, oid, item)
    }
}
fn quality2_completion_before_cancel(primary: bool, already_current: bool) {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let base = f.primary();
    let mut io = Quality2LostReturnCancelIo {
        inner: FakeIo {
            already_current,
            ..Default::default()
        },
        parent: req.operation_id,
        primary,
        completed: None,
    };
    let first = f.run_io(req.clone(), &mut io);
    let (child, oid) = io
        .completed
        .expect("durable-completion/cancellation seam must execute");
    let parent = f.stored(req.operation_id);
    assert!(parent.cancel_requested);
    assert_eq!(
        if primary {
            parent.primary_sync_id
        } else {
            parent.context_sync_id
        },
        Some(child)
    );
    let stored = state::with_transaction(&f.service, f.root.path(), |tx, id| {
        state::read_operation(tx, id, child)
    })
    .unwrap()
    .unwrap();
    assert!(
        matches!(stored.authority,Some(state::SynchronizationAuthority::Published(value)) if !already_current && value==oid)
            || matches!(stored.authority,Some(state::SynchronizationAuthority::AlreadyCurrent(value)) if already_current && value==oid)
    );
    if primary {
        let result = authoritative(first.unwrap());
        assert_eq!(
            result.publication(),
            if already_current {
                PromotionPublication::AlreadyCurrent { oid }
            } else {
                PromotionPublication::Published { oid }
            }
        );
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::Cancelled
            }
        );
        assert_eq!(parent.step, record::PromotionStep::Published);
        assert_ne!(
            parent.integration_oid,
            Some(oid),
            "publication must report the exact stored later tip, not integration fallback"
        );
    } else {
        assert!(matches!(first, Err(DocumentPromotionError::Interrupted)));
        assert_eq!(
            parent.step,
            record::PromotionStep::ContextSynchronized,
            "completed context authority must be recorded before cancellation stops integration"
        );
        assert_eq!(parent.context_oid, Some(oid));
        assert_eq!(f.primary(), base);
    }
    assert!(f.context.workdir().unwrap().exists());
    assert_eq!(io.inner.calls.borrow().deletes, 0);
    let calls = io
        .inner
        .calls
        .borrow()
        .children
        .iter()
        .filter(|(target, _, _)| (*target == SynchronizationTarget::Primary) == primary)
        .count();
    let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(result.cleanup(), PromotionCleanup::Complete);
    if primary {
        assert_eq!(
            result.publication(),
            if already_current {
                PromotionPublication::AlreadyCurrent { oid }
            } else {
                PromotionPublication::Published { oid }
            }
        );
    }
    let parent = f.stored(req.operation_id);
    assert_eq!(
        if primary {
            parent.primary_sync_id
        } else {
            parent.context_sync_id
        },
        Some(child)
    );
    assert_eq!(
        io.inner
            .calls
            .borrow()
            .children
            .iter()
            .filter(|(target, _, _)| (*target == SynchronizationTarget::Primary) == primary)
            .count(),
        calls
    );
    let effect = if primary {
        "primary_push"
    } else {
        "context_push"
    };
    assert_eq!(
        f.db()
            .query_row(
                "SELECT count(*) FROM promotion_test_effects WHERE effect=?1",
                [effect],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        i64::from(!already_current)
    );
    assert_eq!(promotion_commits(&f.repo), 1);
    f.assert_cleaned();
}
#[test]
fn quality2_primary_published_completion_precedes_lost_return_and_parent_cancel() {
    quality2_completion_before_cancel(true, false);
}
#[test]
fn quality2_primary_current_completion_precedes_lost_return_and_parent_cancel() {
    quality2_completion_before_cancel(true, true);
}
#[test]
fn quality2_context_published_completion_precedes_lost_return_and_parent_cancel() {
    quality2_completion_before_cancel(false, false);
}
#[test]
fn quality2_context_current_completion_precedes_lost_return_and_parent_cancel() {
    quality2_completion_before_cancel(false, true);
}

fn quality3_fail_outcome_read(f: &Fixture) -> Rc<std::cell::Cell<bool>> {
    let fired = Rc::new(std::cell::Cell::new(false));
    let observed = fired.clone();
    OUTCOME_READ_HOOKS.with(|h| {
        h.borrow_mut().insert(
            f.root.path().into(),
            Box::new(move || {
                observed.set(true);
                Err(state::recovery_required())
            }),
        )
    });
    fired
}
fn quality3_unavailable_replay_authority(already_current: bool) {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let mut io = Quality2LostReturnCancelIo {
        inner: FakeIo {
            already_current,
            ..Default::default()
        },
        parent: req.operation_id,
        primary: true,
        completed: None,
    };
    f.run_io(req.clone(), &mut io).unwrap();
    let (child, published) = io.completed.unwrap();
    let integration = f.stored(req.operation_id).integration_oid.unwrap();
    assert_ne!(published, integration);
    let complete = outcome(f.run_io(req.clone(), &mut io).unwrap());
    assert_eq!(complete.cleanup(), PromotionCleanup::Complete);
    let counts = effect_counts(&f);
    let calls = io.inner.calls.borrow().children.len();
    let fired = quality3_fail_outcome_read(&f);
    let unavailable = authoritative(f.run_io(req.clone(), &mut io).unwrap());
    assert!(
        fired.get(),
        "actual outcome-time authority lookup must fail in this case"
    );
    match unavailable.publication() {
        PromotionPublication::Published { oid } => {
            assert!(!already_current);
            assert_eq!(
                oid, published,
                "integration OID is not a substitute for unavailable publication evidence"
            );
        }
        PromotionPublication::AlreadyCurrent { oid } => {
            assert!(already_current);
            assert_eq!(
                oid, published,
                "integration OID is not a substitute for unavailable publication evidence"
            );
        }
        PromotionPublication::Pending {
            reason:
                PromotionPublicationPending::Synchronization(
                    PromotionSynchronizationCategory::RecoveryRequired,
                ),
        } => (),
        _ => panic!(
            "unavailable publication authority must be exact known authority or explicit recovery pending"
        ),
    }
    assert_eq!(
        f.stored(req.operation_id).step,
        record::PromotionStep::Complete
    );
    assert_eq!(outcome(f.run_io(req.clone(), &mut io).unwrap()), complete);
    assert_eq!(f.stored(req.operation_id).primary_sync_id, Some(child));
    assert_eq!(io.inner.calls.borrow().children.len(), calls);
    assert_eq!(effect_counts(&f), counts);
}
#[test]
fn quality3_published_replay_read_failure_never_uses_integration_oid() {
    quality3_unavailable_replay_authority(false);
}
#[test]
fn quality3_current_replay_read_failure_never_uses_integration_oid() {
    quality3_unavailable_replay_authority(true);
}
#[test]
fn quality3_live_known_authority_does_not_depend_on_a_second_outcome_lookup() {
    for already_current in [false, true] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let fired = quality3_fail_outcome_read(&f);
        let mut io = Quality2LostReturnCancelIo {
            inner: FakeIo {
                already_current,
                ..Default::default()
            },
            parent: req.operation_id,
            primary: true,
            completed: None,
        };
        let result = authoritative(f.run_io(req.clone(), &mut io).unwrap());
        let (child, oid) = io.completed.unwrap();
        assert_ne!(f.stored(req.operation_id).integration_oid, Some(oid));
        assert_eq!(
            result.publication(),
            if already_current {
                PromotionPublication::AlreadyCurrent { oid }
            } else {
                PromotionPublication::Published { oid }
            }
        );
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::Cancelled
            }
        );
        assert!(
            !fired.get(),
            "carry already known immutable child authority instead of requiring a redundant query"
        );
        OUTCOME_READ_HOOKS.with(|h| h.borrow_mut().remove(f.root.path()));
        let calls = io.inner.calls.borrow().children.len();
        outcome(f.run_io(req.clone(), &mut io).unwrap());
        assert_eq!(f.stored(req.operation_id).primary_sync_id, Some(child));
        assert_eq!(io.inner.calls.borrow().children.len(), calls);
    }
}
struct Quality3LostExecutorIo {
    inner: FakeIo,
    completed: Option<(OperationId, Oid)>,
}
impl run::PromotionIo for Quality3LostExecutorIo {
    fn synchronize(
        &mut self,
        s: &RepositoryService,
        req: SynchronizeRemoteRequest,
        o: &mut dyn SynchronizationObserver,
        e: &record::ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let primary = req.target == SynchronizationTarget::Primary;
        let id = req.operation_id;
        if primary && self.completed.is_none() {
            let repo = git2::Repository::open(&req.root).unwrap();
            write(&repo, "ordinary.txt", "later primary publication tip\n");
            commit(&repo);
        }
        let result = self.inner.synchronize(s, req, o, e)?;
        if primary && self.completed.is_none() {
            let oid = match &result {
                SynchronizationResult::Complete(
                    SynchronizationOutcome::Published { oid, .. }
                    | SynchronizationOutcome::AlreadyCurrent { oid, .. },
                ) => *oid,
                _ => panic!("fixture requires durable primary completion"),
            };
            self.completed = Some((id, oid));
            panic!("disposable fixture executor lost after durable child completion");
        }
        Ok(result)
    }
    fn delete(
        &mut self,
        s: &RepositoryService,
        req: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        c: &ObservationConfiguration,
        oid: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.inner.delete(s, req, plan, owner, c, oid, item)
    }
}
fn quality3_retry_cancel_after_completion(already_current: bool) {
    for publication_pending in [false, true] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        if publication_pending {
            let mut first = FakeIo {
                failure: Some((true, SynchronizationError::PushRejected)),
                ..Default::default()
            };
            f.run_io(req.clone(), &mut first).unwrap();
        }
        let mut io = Quality3LostExecutorIo {
            inner: FakeIo {
                already_current,
                ..Default::default()
            },
            completed: None,
        };
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || f.run_io(req.clone(), &mut io)
            ))
            .is_err()
        );
        let (child, oid) = io
            .completed
            .expect("lost executor must leave actual durable authority");
        let parent = f.stored(req.operation_id);
        assert_eq!(
            parent.step,
            if publication_pending {
                record::PromotionStep::PublicationPending
            } else {
                record::PromotionStep::Integrated
            }
        );
        assert_ne!(parent.integration_oid, Some(oid));
        state::with_transaction(&f.service, f.root.path(), |tx, id| {
            let child = state::read_operation(tx, id, child)?.unwrap();
            assert_eq!(child.authority.unwrap().oid(), oid);
            Ok(())
        })
        .unwrap();
        assert!(
            f.service
                .cancel_document_promotion(f.root.path(), req.operation_id)
                .unwrap()
        );
        // This trigger deterministically posts a second cancellation after the
        // identical retry clears its first stop, before finish_steps can run.
        // Completed child rows are inactive, so a real cancellation in this
        // interval changes only this parent flag as well.
        f.db().execute_batch(&format!("CREATE TRIGGER quality3_recancel AFTER UPDATE OF cancel_requested ON document_promotions WHEN NEW.operation_id='{}' AND OLD.cancel_requested=1 AND NEW.cancel_requested=0 BEGIN UPDATE document_promotions SET cancel_requested=1 WHERE operation_id=NEW.operation_id; END;",req.operation_id)).unwrap();
        let calls = io.inner.calls.borrow().children.len();
        let result = authoritative(f.run_io(req.clone(), &mut io).unwrap());
        assert!(
            f.stored(req.operation_id).cancel_requested,
            "retry-window cancellation must be observed"
        );
        assert_eq!(
            result.publication(),
            if already_current {
                PromotionPublication::AlreadyCurrent { oid }
            } else {
                PromotionPublication::Published { oid }
            }
        );
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::Cancelled
            }
        );
        assert_eq!(
            f.stored(req.operation_id).step,
            record::PromotionStep::Published
        );
        assert_eq!(f.stored(req.operation_id).primary_sync_id, Some(child));
        assert_eq!(io.inner.calls.borrow().children.len(), calls);
        assert_eq!(io.inner.calls.borrow().deletes, 0);
        assert!(f.context.workdir().unwrap().exists());
        f.db()
            .execute_batch("DROP TRIGGER quality3_recancel")
            .unwrap();
        let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
        assert_eq!(
            result.publication(),
            if already_current {
                PromotionPublication::AlreadyCurrent { oid }
            } else {
                PromotionPublication::Published { oid }
            }
        );
        assert_eq!(result.cleanup(), PromotionCleanup::Complete);
        assert_eq!(f.stored(req.operation_id).primary_sync_id, Some(child));
        assert_eq!(io.inner.calls.borrow().children.len(), calls);
        assert_eq!(promotion_commits(&f.repo), 1);
    }
}
#[test]
fn quality3_retry_window_cancel_preserves_published_authority() {
    quality3_retry_cancel_after_completion(false);
}
#[test]
fn quality3_retry_window_cancel_preserves_current_authority() {
    quality3_retry_cancel_after_completion(true);
}
#[test]
fn execution_only_local_conflict_abandons_without_primary_effect() {
    let f = Fixture::new();
    let context = f.context.head().unwrap().target().unwrap();
    checkout(&f.repo, context);
    write(&f.context, "same.txt", "context\n");
    commit(&f.context);
    let req = f.request();
    let root = f.root.path().to_owned();
    hook(&f, move |_, p| {
        if p == FailurePoint::AfterConfirmed {
            let repo = git2::Repository::open(&root).unwrap();
            write(&repo, "same.txt", "primary\n");
            commit(&repo);
        }
    });
    assert!(matches!(
        f.run(req.clone()),
        Err(DocumentPromotionError::ReconfirmationRequired)
    ));
    assert_eq!(
        f.stored(req.operation_id).step,
        record::PromotionStep::Abandoned
    );
    assert_eq!(promotion_commits(&f.repo), 0);
}
#[test]
fn integrated_configuration_failure_is_ok_pending_even_when_configuration_is_invalid() {
    let f = Fixture::new();
    let req = f.request();
    let stopped = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterPublication,
    )
    .unwrap();
    stopped
        .promote_document(req.clone(), &mut SessionCredentials::new(NoCredentials))
        .unwrap();
    fs::write(
        f.root.path().join(".manyhands/config.toml"),
        "invalid configuration",
    )
    .unwrap();
    let result = authoritative(f.run(req).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert_eq!(promotion_commits(&f.repo), 1);
}
#[test]
fn partial_worktree_prune_requires_recovery_and_never_finishes_a_stub() {
    let f = Fixture::new();
    let req = f.request();
    let root = f.root.path().to_owned();
    hook(&f, move |_, p| {
        if p == FailurePoint::DuringWorktreeRemoval {
            fs::remove_file(root.join(".manyhands/worktrees").join(ITEM).join(".git")).unwrap();
        }
    });
    let result = authoritative(f.run(req.clone()).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert_eq!(
        f.stored(req.operation_id).step,
        record::PromotionStep::WorktreeRemoving
    );
    assert!(f.context.workdir().unwrap().join(DOC).exists());
    HOOKS.with(|h| h.borrow_mut().remove(f.root.path()));
    assert_eq!(
        authoritative(f.run(req).unwrap()).cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
}
fn outcome(result: DocumentPromotionResult) -> DocumentPromotionOutcome {
    match result {
        DocumentPromotionResult::Complete(o) => o,
        other => panic!("index not refreshed: {other:?}"),
    }
}
fn authoritative(result: DocumentPromotionResult) -> DocumentPromotionOutcome {
    match result {
        DocumentPromotionResult::Complete(o) => o,
        DocumentPromotionResult::IndexPending(p) => p.authoritative,
    }
}
#[test]
fn local_full_lifecycle_and_exact_replay() {
    let f = Fixture::new();
    let req = f.request();
    let base = f.primary();
    let context = f.context.head().unwrap().target().unwrap();
    let result = outcome(f.run(req.clone()).unwrap());
    let merged = f.primary();
    assert_eq!(result.operation_id(), req.operation_id);
    assert_eq!(
        result.integration(),
        PromotionIntegration::Merged { oid: merged }
    );
    assert_eq!(
        result.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::NoPublicationRemote
        }
    );
    assert_eq!(result.cleanup(), PromotionCleanup::Complete);
    let c = f.repo.find_commit(merged).unwrap();
    assert_eq!(
        c.summary(),
        Some(format!("Promote document {ITEM}").as_str())
    );
    assert_eq!(
        [c.parent_id(0).unwrap(), c.parent_id(1).unwrap()],
        [base, context]
    );
    f.assert_cleaned();
    assert_eq!(
        f.stored(req.operation_id).step,
        record::PromotionStep::Complete
    );
    assert_eq!(outcome(f.run(req).unwrap()), result);
    assert_eq!(f.primary(), merged);
    assert!(f.root.path().join(DOC).is_file());
    let text = format!("{result:?}");
    for secret in [
        "private title",
        "private body",
        f.root.path().to_str().unwrap(),
    ] {
        assert!(!text.contains(secret));
    }
}
#[test]
fn contained_cleanup_needs_no_identity_or_new_merge() {
    let f = Fixture::new();
    let tip = f.context.head().unwrap().target().unwrap();
    checkout(&f.repo, tip);
    f.repo.config().unwrap().set_str("user.name", "").unwrap();
    f.repo.config().unwrap().set_str("user.email", "").unwrap();
    let result = outcome(f.run(f.request()).unwrap());
    assert_eq!(
        result.integration(),
        PromotionIntegration::AlreadyContained { oid: tip }
    );
    assert_eq!(f.primary(), tip);
    f.assert_cleaned();
}
#[test]
fn changed_digest_is_durably_abandoned_and_new_id_succeeds() {
    let f = Fixture::new();
    let req = f.request();
    let base = f.primary();
    write(&f.context, "new.txt", "another path");
    commit(&f.context);
    assert!(matches!(
        f.run(req.clone()),
        Err(DocumentPromotionError::ExternalChange)
    ));
    assert_eq!(f.primary(), base);
    assert_eq!(
        f.stored(req.operation_id).step,
        record::PromotionStep::Abandoned
    );
    assert!(matches!(
        f.run(req),
        Err(DocumentPromotionError::ExternalChange)
    ));
    outcome(f.run(f.request()).unwrap());
    f.assert_cleaned();
}
#[test]
fn new_blocker_is_stored_and_replayed_before_any_child() {
    let f = Fixture::new();
    let req = f.request();
    write(&f.repo, "untracked", "preserve");
    assert!(
        matches!(f.run(req.clone()), Err(DocumentPromotionError::Blocked(b)) if b.contains(&PromotionBlocker::PrimaryDirty))
    );
    fs::remove_file(f.root.path().join("untracked")).unwrap();
    assert!(matches!(
        f.run(req),
        Err(DocumentPromotionError::Blocked(_))
    ));
    outcome(f.run(f.request()).unwrap());
}
#[test]
fn identity_and_mismatch_do_not_strand_the_item() {
    let f = Fixture::new();
    f.repo.config().unwrap().set_str("user.name", "").unwrap();
    f.repo.config().unwrap().set_str("user.email", "").unwrap();
    let mut req = f.request();
    let expected_configuration = match f.run(req.clone()) {
        Err(DocumentPromotionError::IdentityRequired {
            expected_configuration,
        }) => expected_configuration,
        other => panic!("expected identity: {other:?}"),
    };
    let mut wrong = req.clone();
    wrong.expected = PromotionExpectation::from_bytes([0; 32]);
    assert!(matches!(
        f.run(wrong),
        Err(DocumentPromotionError::OperationMismatch)
    ));
    req.confirmed_identity = Some(ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: CommitIdentity {
            name: "Confirmed".into(),
            email: "confirmed@example.invalid".into(),
        },
        expected_configuration,
    });
    outcome(f.run(req).unwrap());
    assert_eq!(
        f.repo.config().unwrap().get_string("user.name").unwrap(),
        "Confirmed"
    );
}

#[test]
fn remote_children_publication_cleanup_and_replay_are_once_only() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let mut io = FakeIo::default();
    let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
    assert!(matches!(
        result.publication(),
        PromotionPublication::Published { .. }
    ));
    assert_eq!(result.cleanup(), PromotionCleanup::Complete);
    assert_eq!(io.calls.borrow().children.len(), 2);
    assert_eq!(io.calls.borrow().deletes, 1);
    assert_eq!(outcome(f.run_io(req, &mut io).unwrap()), result);
    assert_eq!(io.calls.borrow().children.len(), 2);
    assert_eq!(io.calls.borrow().deletes, 1);
    f.assert_cleaned();
}
#[test]
fn publication_rejection_is_pending_and_retry_only_publishes_and_cleans() {
    let f = Fixture::new();
    f.remote();
    let req = f.request();
    let mut io = FakeIo {
        failure: Some((true, SynchronizationError::PushRejected)),
        ..Default::default()
    };
    let result = outcome(f.run_io(req.clone(), &mut io).unwrap());
    let oid = f.primary();
    assert_eq!(
        result.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::Synchronization(
                PromotionSynchronizationCategory::Rejected
            )
        }
    );
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::AwaitingPublication
        }
    );
    assert!(f.context.workdir().unwrap().exists());
    assert_eq!(io.calls.borrow().deletes, 0);
    assert_eq!(
        outcome(f.run_io(req, &mut io).unwrap()).cleanup(),
        PromotionCleanup::Complete
    );
    assert_eq!(f.primary(), oid);
    assert_eq!(io.calls.borrow().children.len(), 3);
}
#[test]
fn deletion_changed_and_still_present_keep_context_and_retry() {
    for changed in [false, true] {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let oid = f.context.head().unwrap().target().unwrap();
        let mut io = FakeIo {
            deletion: Some(if changed {
                DeleteObservation::Changed(f.primary())
            } else {
                DeleteObservation::StillPresent(oid)
            }),
            ..Default::default()
        };
        let partial = f.run_io(req.clone(), &mut io).unwrap();
        assert!(
            matches!(partial, DocumentPromotionResult::IndexPending(_)),
            "held promote reservation defers index claim"
        );
        let result = authoritative(partial);
        let reason = if changed {
            PromotionCleanupPending::RemoteContextChanged
        } else {
            PromotionCleanupPending::RemoteDeletionFailed(PromotionRemoteDeletionCategory::Rejected)
        };
        assert_eq!(result.cleanup(), PromotionCleanup::Pending { reason });
        assert!(f.context.workdir().unwrap().exists());
        assert_eq!(
            f.db()
                .query_row("SELECT count(*) FROM retired_contexts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        if !changed {
            assert_eq!(
                outcome(f.run_io(req, &mut io).unwrap()).cleanup(),
                PromotionCleanup::Complete
            );
        }
    }
}
#[test]
fn cleanup_precheck_preserves_dirty_untracked_moved_and_ignored_before_remote_delete() {
    for mode in 0..4 {
        let f = Fixture::new();
        f.remote();
        let req = f.request();
        let root = f.root.path().to_owned();
        hook(&f, move |_, point| {
            if point == FailurePoint::AfterPublication {
                let repo =
                    git2::Repository::open(root.join(".manyhands/worktrees").join(ITEM)).unwrap();
                match mode {
                    0 => write(&repo, DOC, "dirty"),
                    1 => write(&repo, "untracked", "preserve"),
                    2 => {
                        write(&repo, "moved", "preserve");
                        commit(&repo);
                    }
                    _ => {
                        fs::write(
                            git2::Repository::open(&root)
                                .unwrap()
                                .path()
                                .join("info/exclude"),
                            "ignored\n",
                        )
                        .unwrap();
                        write(&repo, "ignored", "preserve");
                    }
                }
            }
        });
        let mut io = FakeIo::default();
        let result = outcome(f.run_io(req, &mut io).unwrap());
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: if mode == 3 {
                    PromotionCleanupPending::IgnoredFiles
                } else {
                    PromotionCleanupPending::ContextChanged
                }
            }
        );
        assert_eq!(io.calls.borrow().deletes, 0);
        assert!(f.context.workdir().unwrap().exists());
    }
}
#[test]
fn cancellation_before_and_after_integration_preserves_and_retry_finishes() {
    for integrated in [false, true] {
        let f = Fixture::new();
        let req = f.request();
        let op = req.operation_id;
        let base = f.primary();
        let root = f.root.path().to_owned();
        let mut fired = false;
        hook(&f, move |service, p| {
            let point = if integrated {
                FailurePoint::AfterIntegrationRef
            } else {
                FailurePoint::AfterConfirmed
            };
            if p == point && !fired {
                fired = true;
                assert!(service.cancel_document_promotion(&root, op).unwrap());
            }
        });
        let result = f.run(req.clone());
        if integrated {
            assert_eq!(
                outcome(result.unwrap()).cleanup(),
                PromotionCleanup::Pending {
                    reason: PromotionCleanupPending::Cancelled
                }
            );
        } else {
            assert!(matches!(result, Err(DocumentPromotionError::Interrupted)));
            assert_eq!(f.primary(), base);
        }
        assert!(f.context.workdir().unwrap().exists());
        outcome(f.run(req).unwrap());
        f.assert_cleaned();
    }
}
#[test]
fn refresh_failure_keeps_identical_authoritative_outcome_and_replay_repairs_index() {
    let f = Fixture::new();
    let req = f.request();
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::RefreshFailure,
    )
    .unwrap();
    let result = service
        .promote_document(req.clone(), &mut SessionCredentials::new(NoCredentials))
        .unwrap();
    let DocumentPromotionResult::IndexPending(pending) = result else {
        panic!("expected index pending")
    };
    assert!(
        f.service
            .repository_snapshot(f.root.path())
            .unwrap()
            .refresh_required
    );
    assert_eq!(outcome(f.run(req).unwrap()), pending.authoritative);
    assert!(
        !f.service
            .repository_snapshot(f.root.path())
            .unwrap()
            .refresh_required
    );
}
