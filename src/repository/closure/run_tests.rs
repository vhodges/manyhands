//! `close_ticket` over the generic runner, end to end: consent, the closure
//! checkpoint and its seams, retry, cancellation, refresh, cleanup and the
//! read API. The fixtures are the closure's shared ones; the children, the
//! exact delete, the boundary hooks and the effect counters are promotion's
//! (`promotion::run_tests`), called, not copied.
use super::{
    lifecycle::{TicketClosureLifecycle, lifecycle_request},
    observe::closed_ticket_bytes,
    record::{self, ClosureKind, ClosureRecord, ClosureStep},
    test_support::{self as support, ID, checkout, commit, id, parse_ticket, read, ticket, write},
    *,
};
use crate::repository::{
    AuthoringKind, ClosureDto, ClosureState, FAILURE_POINT_EXIT_STATUS, FailurePoint,
    ItemContextKind, PromotionCleanup, PromotionCleanupPending, PromotionIntegration,
    PromotionPublication, PromotionPublicationPending, RemoteOperationAction,
    RemoteOperationTarget, RemoteRefPlan, RemoteReservationOutcome, RepositoryErrorKind,
    RepositoryOperation, SynchronizationError, SynchronizationObserver, SynchronizationResult,
    SynchronizationTarget, SynchronizeRemoteRequest, TicketFilter,
    keys::SessionCredentials,
    promotion::{
        lifecycle::LifecycleError,
        record::ChildExecution,
        run::{self, PromotionIo},
        run_tests::{COMMIT_HOOKS, FakeIo, HOOKS, NoCredentials, TRACKING_HOOKS},
    },
    remote::{observation::ObservationConfiguration, reservation::RemoteReservation, state},
    transport::{DeleteObservation, VerifySshTransportRequest},
};
use git2::Oid;
use std::{cell::RefCell, fs, path::Path, rc::Rc};

const CLOSED_BY: &str = "Fixture <fixture@example.invalid>";

fn context_ref() -> String {
    format!("refs/heads/manyhands/ticket/{ID}")
}
fn tracking_context_ref() -> String {
    format!("refs/remotes/origin/manyhands/ticket/{ID}")
}
fn close_subject() -> String {
    format!("Close ticket {ID}")
}
fn merge_subject() -> String {
    format!("Integrate closed ticket {ID}")
}

/// The shared closure fixture, with the runner's own helpers. Dropping it
/// clears the boundary hooks it installed.
struct Fixture(support::Fixture);
impl std::ops::Deref for Fixture {
    type Target = support::Fixture;
    fn deref(&self) -> &support::Fixture {
        &self.0
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let root = self.root.path().to_owned();
        HOOKS.with(|h| h.borrow_mut().remove(&root));
        TRACKING_HOOKS.with(|h| h.borrow_mut().remove(&root));
        COMMIT_HOOKS.with(|h| h.borrow_mut().remove(&root));
    }
}
impl Fixture {
    /// No remote: the ticket context is one commit ahead of the primary.
    fn new() -> Self {
        Self(support::Fixture::new(false))
    }
    /// A publication remote whose context branch holds the context's parent:
    /// the context child publishes the newer local tip, so the delete guard
    /// (the child's publication) and the tracking OID differ.
    fn remote() -> Self {
        let f = Self(support::Fixture::new(true));
        let published = f
            .repo
            .find_commit(f.context_oid())
            .unwrap()
            .parent_id(0)
            .unwrap();
        f.repo
            .reference(&tracking_context_ref(), published, true, "fixture")
            .unwrap();
        f
    }
    fn root(&self) -> &Path {
        self.root.path()
    }
    fn context_oid(&self) -> Oid {
        self.repo.refname_to_id(&context_ref()).unwrap()
    }
    fn primary(&self) -> Oid {
        self.repo.refname_to_id("refs/heads/main").unwrap()
    }
    fn preview(&self) -> TicketClosurePreview {
        match self
            .service
            .preview_ticket_closure(PreviewTicketClosureRequest {
                root: self.root().into(),
                item_id: id(),
                confirmed_identity: None,
            })
            .unwrap()
        {
            TicketClosurePreflight::Ready(p) => p,
            other => panic!("not ready: {other:?}"),
        }
    }
    fn request(&self) -> CloseTicketRequest {
        CloseTicketRequest {
            root: self.root().into(),
            operation_id: OperationId::new(),
            item_id: id(),
            expected: self.preview().expectation(),
            approval: None,
            confirmed_identity: None,
        }
    }
    /// The public operation; without a remote no child or transport runs.
    fn close(
        &self,
        service: &RepositoryService,
        request: CloseTicketRequest,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        service.close_ticket(request, &mut SessionCredentials::new(NoCredentials))
    }
    /// The runner with injected children and exact delete.
    fn run_io(
        &self,
        service: &RepositoryService,
        request: CloseTicketRequest,
        io: &mut impl PromotionIo,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        run::run_with_io(
            &TicketClosureLifecycle,
            service,
            lifecycle_request(request),
            io,
        )
    }
    fn failing(&self, point: FailurePoint) -> RepositoryService {
        RepositoryService::open_at_with_failure_point_for_testing(self.data.path(), point).unwrap()
    }
    fn reopened(&self) -> RepositoryService {
        RepositoryService::open_at(self.data.path()).unwrap()
    }
    fn stored(&self, op: OperationId) -> ClosureRecord {
        state::with_transaction(&self.service, self.root(), |tx, rid| {
            record::read_record(tx, rid, op)?.ok_or_else(state::recovery_required)
        })
        .unwrap()
    }
    /// Commits of `subject` anywhere in the object database.
    fn commits_in_odb(&self, subject: &str) -> usize {
        let mut count = 0;
        self.repo
            .odb()
            .unwrap()
            .foreach(|oid| {
                if self
                    .repo
                    .find_commit(*oid)
                    .is_ok_and(|c| c.summary() == Some(subject))
                {
                    count += 1;
                }
                true
            })
            .unwrap();
        count
    }
    /// Commits of `subject` reachable from the primary.
    fn reachable(&self, subject: &str) -> Vec<Oid> {
        let mut walk = self.repo.revwalk().unwrap();
        walk.push(self.primary()).unwrap();
        walk.map(Result::unwrap)
            .filter(|oid| self.repo.find_commit(*oid).unwrap().summary() == Some(subject))
            .collect()
    }
    fn merge(&self) -> git2::Commit<'_> {
        let merges = self.reachable(&merge_subject());
        assert_eq!(merges.len(), 1, "one integration merge");
        self.repo.find_commit(merges[0]).unwrap()
    }
    fn primary_ticket(&self) -> crate::canonical::Ticket {
        parse_ticket(&support::tree_bytes(&self.repo, self.primary(), &ticket()))
    }
    fn effects(&self, effect: &str) -> i64 {
        let db = self.db();
        let exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='promotion_test_effects')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        if !exists {
            return 0;
        }
        db.query_row(
            "SELECT count(*) FROM promotion_test_effects WHERE effect=?1",
            [effect],
            |r| r.get(0),
        )
        .unwrap()
    }
    /// A boundary hook for every runner boundary of this fixture's root.
    fn hook(&self, callback: impl FnMut(&RepositoryService, FailurePoint) + 'static) {
        HOOKS.with(|h| {
            h.borrow_mut()
                .insert(self.root().into(), Box::new(callback))
        });
    }
    fn assert_cleaned(&self) {
        assert!(!self.root().join(".manyhands/worktrees").join(ID).exists());
        assert!(self.repo.find_reference(&context_ref()).is_err());
    }
    /// Freezes the endpoint configuration a manual remote reservation needs.
    fn configure_endpoints(&self) -> RemoteRefPlan {
        let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
        let frozen = self
            .service
            .observation_configuration(self.root(), &plan)
            .unwrap();
        state::with_transaction(&self.service, self.root(), |tx, rid| {
            state::configure_endpoints(tx, rid, &plan, &frozen.endpoint_digest())
        })
        .unwrap();
        plan
    }
    fn manual_context_sync(&self, plan: &RemoteRefPlan) -> RemoteOperationTarget {
        RemoteOperationTarget::for_context(
            plan,
            RemoteOperationAction::SynchronizeContext,
            AuthoringKind::Ticket,
            id(),
        )
        .unwrap()
    }
}

fn outcome(result: TicketClosureResult) -> TicketClosureOutcome {
    match result {
        TicketClosureResult::Complete(o) => o,
        other => panic!("not complete: {other:?}"),
    }
}
fn authoritative(result: TicketClosureResult) -> TicketClosureOutcome {
    match result {
        TicketClosureResult::Complete(o) => o,
        TicketClosureResult::IndexPending(p) => p.authoritative,
    }
}
/// Every ref and its target.
fn refs(repo: &git2::Repository) -> Vec<(String, Oid)> {
    let mut refs: Vec<_> = repo
        .references()
        .unwrap()
        .filter_map(|r| {
            let r = r.ok()?;
            Some((r.name()?.to_owned(), r.target()?))
        })
        .collect();
    refs.sort();
    refs
}
/// Commits `edit` of the ticket text on a worktree.
fn edit_ticket(repo: &git2::Repository, edit: impl Fn(String) -> String) -> Oid {
    let source = String::from_utf8(read(repo, &ticket())).unwrap();
    write(repo, &ticket(), edit(source));
    commit(repo)
}
/// A primary commit that changes another file only.
fn advance_primary(root: &Path, file: &str) -> Oid {
    let repo = git2::Repository::open(root).unwrap();
    write(&repo, file, format!("{file}\n"));
    commit(&repo)
}
/// A deliberate synchronization the user ran over a written checkpoint: the
/// primary merged into the context, keeping the context's closed ticket.
fn synchronize_context_by_hand(f: &Fixture) -> Oid {
    let checkpoint = f.context_oid();
    let primary = f.primary();
    let ours = f.repo.find_commit(checkpoint).unwrap();
    let theirs = f.repo.find_commit(primary).unwrap();
    let mut index = f.repo.merge_commits(&ours, &theirs, None).unwrap();
    if index.has_conflicts() {
        let closed = support::tree_bytes(&f.repo, checkpoint, &ticket());
        let entry = git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: closed.len() as u32,
            id: f.repo.blob(&closed).unwrap(),
            flags: 0,
            flags_extended: 0,
            path: ticket().into_bytes(),
        };
        index.conflict_remove(Path::new(&ticket())).unwrap();
        index.add(&entry).unwrap();
    }
    let tree = f
        .repo
        .find_tree(index.write_tree_to(&f.repo).unwrap())
        .unwrap();
    let sig = git2::Signature::now("Collaborator", "collaborator@example.invalid").unwrap();
    let merged = f
        .repo
        .commit(
            None,
            &sig,
            &sig,
            "deliberate context synchronization",
            &tree,
            &[&ours, &theirs],
        )
        .unwrap();
    checkout(&f.context, merged);
    merged
}

// --- Happy path, replay and consent ------------------------------------

#[test]
fn no_remote_closure_records_merges_and_cleans_with_the_status_kept() {
    let f = Fixture::new();
    let base = f.primary();
    let req = f.request();
    let result = outcome(f.close(&f.service, req.clone()).unwrap());
    let r = f.stored(req.operation_id);
    let checkpoint = r.closure_oid.unwrap();
    assert_eq!(
        result.closure(),
        ClosureRecording::Recorded {
            checkpoint_oid: checkpoint
        }
    );
    assert_eq!(
        result.integration(),
        PromotionIntegration::Merged { oid: f.primary() }
    );
    assert_eq!(
        result.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::NoPublicationRemote
        }
    );
    assert_eq!(result.cleanup(), PromotionCleanup::Complete);
    assert_eq!(result.operation_id(), req.operation_id);
    assert_eq!(r.step, ClosureStep::Complete);
    let merge = f.merge();
    assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [base, checkpoint]);
    assert_eq!(f.reachable(&close_subject()), [checkpoint]);
    let closed = f.primary_ticket();
    assert_eq!(closed.status, "open");
    assert_eq!(closed.closed_by.as_deref(), Some(CLOSED_BY));
    assert_eq!(closed.closed_at, r.closed_at);
    assert!(closed.body.contains("context body"));
    f.assert_cleaned();
}

/// R-T1-4: the hook writes `closing` (with its checkpoint and `closed_at`)
/// inside the lease; the runner must reload before recording `integrating`,
/// or its stale snapshot would overwrite those fields.
#[test]
fn runner_reloads_the_record_the_checkpoint_hook_wrote() {
    let f = Fixture::new();
    let req = f.request();
    let failing = f.failing(FailurePoint::BeforeIntegrationRef);
    assert!(f.close(&failing, req.clone()).is_err());
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::Integrating);
    assert_eq!(r.closure_kind, Some(ClosureKind::Record));
    assert!(r.closed_at.is_some());
    let checkpoint = r
        .closure_oid
        .expect("the hook's checkpoint survives the save");
    assert_eq!(
        f.repo.find_commit(checkpoint).unwrap().summary(),
        Some(close_subject().as_str())
    );
    let candidate = f.repo.find_commit(r.integration_oid.unwrap()).unwrap();
    assert_eq!(candidate.parent_id(1).unwrap(), checkpoint);
    outcome(f.close(&f.reopened(), req).unwrap());
}

#[test]
fn completed_id_replays_the_stored_outcome_and_refuses_another_item_or_digest() {
    let f = Fixture::new();
    let req = f.request();
    let first = outcome(f.close(&f.service, req.clone()).unwrap());
    let before = refs(&f.repo);
    let close_commits = f.commits_in_odb(&close_subject());
    assert_eq!(outcome(f.close(&f.service, req.clone()).unwrap()), first);
    assert_eq!(refs(&f.repo), before);
    assert_eq!(f.commits_in_odb(&close_subject()), close_commits);
    let mut other_digest = req.clone();
    other_digest.expected = ClosureExpectation::from_bytes([9; 32]);
    assert!(matches!(
        f.close(&f.service, other_digest),
        Err(TicketClosureError::OperationMismatch)
    ));
    let mut other_item = req;
    other_item.item_id = crate::canonical::ItemId::generate();
    assert!(matches!(
        f.close(&f.service, other_item),
        Err(TicketClosureError::OperationMismatch)
    ));
    assert_eq!(refs(&f.repo), before);
}

#[test]
fn pre_checkpoint_refusals_abandon_with_nothing_written_replay_and_a_new_id_works() {
    let f = Fixture::new();
    let context = f.context_oid();
    let primary = f.primary();
    let assert_unwritten = |op| {
        let r = f.stored(op);
        assert_eq!(r.step, ClosureStep::Abandoned);
        assert_eq!(r.closure_oid, None);
        assert_eq!(f.context_oid(), context);
        assert_eq!(f.commits_in_odb(&close_subject()), 0);
    };
    // Blocked: an untracked file in the context.
    let blocked = f.request();
    write(&f.context, "draft.txt", "draft\n");
    assert!(matches!(
        f.close(&f.service, blocked.clone()),
        Err(TicketClosureError::Blocked(b)) if b == [ClosureBlocker::ContextDirty]
    ));
    assert_unwritten(blocked.operation_id);
    assert!(matches!(
        f.close(&f.service, blocked.clone()),
        Err(TicketClosureError::Blocked(b)) if b == [ClosureBlocker::ContextDirty]
    ));
    fs::remove_file(f.context.workdir().unwrap().join("draft.txt")).unwrap();
    // ExternalChange: a digest that is not the current preview's.
    let mut changed = f.request();
    changed.expected = ClosureExpectation::from_bytes([7; 32]);
    assert!(matches!(
        f.close(&f.service, changed.clone()),
        Err(TicketClosureError::ExternalChange)
    ));
    assert_unwritten(changed.operation_id);
    assert!(matches!(
        f.close(&f.service, changed),
        Err(TicketClosureError::ExternalChange)
    ));
    // ReconfirmationRequired: the primary moves after confirmation.
    let reconfirm = f.request();
    let root = f.root().to_owned();
    f.hook(move |_, point| {
        if point == FailurePoint::AfterContextSync {
            advance_primary(&root, "late.txt");
        }
    });
    assert!(matches!(
        f.close(&f.service, reconfirm.clone()),
        Err(TicketClosureError::ReconfirmationRequired)
    ));
    HOOKS.with(|h| h.borrow_mut().remove(f.root()));
    assert_unwritten(reconfirm.operation_id);
    assert_ne!(f.primary(), primary);
    assert!(matches!(
        f.close(&f.service, reconfirm),
        Err(TicketClosureError::ReconfirmationRequired)
    ));
    // A new ID with the current consent closes the ticket.
    let fresh = f.request();
    outcome(f.close(&f.service, fresh).unwrap());
    assert!(f.primary_ticket().closed_at.is_some());
}

/// "Identity errors at execution": a `Record` closure whose identity changed
/// or went away since its preview is `ExternalChange`, never
/// `Blocked([IdentityRequired])`, and nothing is written.
#[test]
fn a_changed_or_missing_identity_since_the_preview_is_external_change() {
    for (name, email) in [("Fixture", "changed@example.invalid"), ("", "")] {
        let f = Fixture::new();
        let req = f.request();
        // Empty local values also hide any identity configured elsewhere.
        let mut config = f.repo.config().unwrap();
        config.set_str("user.name", name).unwrap();
        config.set_str("user.email", email).unwrap();
        assert!(
            matches!(
                f.close(&f.service, req.clone()),
                Err(TicketClosureError::ExternalChange)
            ),
            "{email:?}"
        );
        let r = f.stored(req.operation_id);
        assert_eq!(r.step, ClosureStep::Abandoned);
        assert_eq!(r.refusal, Some(record::Refusal::ExternalChange));
        assert_eq!(f.commits_in_odb(&close_subject()), 0);
    }
}

/// R-T2-1: a new record carries the observed closure effect; only a refusal
/// recorded before any observation has none.
#[test]
fn new_records_carry_the_observed_closure_kind() {
    let f = Fixture::new();
    let confirmed = f.request();
    let _ = f.close(&f.failing(FailurePoint::AfterConfirmed), confirmed.clone());
    assert_eq!(
        f.stored(confirmed.operation_id).closure_kind,
        Some(ClosureKind::Record)
    );
    // A refusal after an observation (the digest differs) keeps its effect.
    let f = Fixture::new();
    let mut changed = f.request();
    changed.expected = ClosureExpectation::from_bytes([3; 32]);
    let _ = f.close(&f.service, changed.clone());
    let refused = f.stored(changed.operation_id);
    assert_eq!(refused.step, ClosureStep::Abandoned);
    assert_eq!(refused.closure_kind, Some(ClosureKind::Record));
    // A blocked revalidation observed no effect.
    let f = Fixture::new();
    let blocked = f.request();
    write(&f.context, "draft.txt", "draft\n");
    let _ = f.close(&f.service, blocked.clone());
    let refused = f.stored(blocked.operation_id);
    assert_eq!(refused.step, ClosureStep::Abandoned);
    assert_eq!(refused.closure_kind, None);

    let g = Fixture::new();
    let closed = closed_ticket_bytes(
        &read(&g.context, &ticket()),
        support::at(1_800_000_000),
        "Earlier <earlier@example.invalid>",
    )
    .unwrap();
    write(&g.context, &ticket(), closed);
    commit(&g.context);
    let existing = g.request();
    assert!(matches!(
        g.preview().closure(),
        PlannedClosure::Existing { .. }
    ));
    let _ = g.close(&g.failing(FailurePoint::AfterConfirmed), existing.clone());
    assert_eq!(
        g.stored(existing.operation_id).closure_kind,
        Some(ClosureKind::Existing)
    );
    let result = outcome(g.close(&g.reopened(), existing).unwrap());
    assert_eq!(result.closure(), ClosureRecording::Existing);
    assert_eq!(g.commits_in_odb(&close_subject()), 0);
    assert_eq!(
        g.primary_ticket().closed_by.as_deref(),
        Some("Earlier <earlier@example.invalid>")
    );
}

/// The error mapping never turns a repository error into a consent refusal:
/// the runner abandons consent refusals, and an error from inside the lease
/// may follow a written checkpoint.
#[test]
fn repository_errors_of_every_kind_stay_repository_errors() {
    for kind in [
        RepositoryErrorKind::ExternalChange,
        RepositoryErrorKind::RecoveryRequired,
        RepositoryErrorKind::RepositoryBusy,
        RepositoryErrorKind::OperationMismatch,
        RepositoryErrorKind::InjectedFailure,
    ] {
        let error = TicketClosureError::from(RepositoryError::new(
            RepositoryOperation::RepositorySnapshot,
            None,
            kind,
            "fixture",
        ));
        assert!(matches!(&error, TicketClosureError::Repository(e) if e.kind == kind));
        assert!(!error.is_consent_refusal(), "{kind:?}");
    }
}

// --- The checkpoint seams -----------------------------------------------

#[test]
fn checkpoint_seams_finish_once_with_the_same_id_and_a_new_id_is_busy() {
    for point in [
        FailurePoint::ClosureAfterClosingRecorded,
        FailurePoint::ClosureDuringCheckpointCheckout,
        FailurePoint::AfterClosureCheckpoint,
    ] {
        let f = Fixture::new();
        let req = f.request();
        assert!(
            matches!(
                f.close(&f.failing(point), req.clone()),
                Err(TicketClosureError::Repository(e)) if e.kind == RepositoryErrorKind::InjectedFailure
            ),
            "{point:?}"
        );
        let stopped = f.stored(req.operation_id);
        assert_eq!(stopped.step, ClosureStep::Closing, "{point:?}");
        let mut other = req.clone();
        other.operation_id = OperationId::new();
        assert!(
            matches!(f.close(&f.reopened(), other), Err(TicketClosureError::Busy)),
            "{point:?}"
        );
        let result = outcome(f.close(&f.reopened(), req.clone()).unwrap());
        let r = f.stored(req.operation_id);
        assert_eq!(r.closed_at, stopped.closed_at, "{point:?}");
        assert_eq!(r.closure_oid, stopped.closure_oid, "{point:?}");
        assert_eq!(f.commits_in_odb(&close_subject()), 1, "{point:?}");
        assert_eq!(f.reachable(&close_subject()).len(), 1, "{point:?}");
        assert_eq!(f.commits_in_odb(&merge_subject()), 1, "{point:?}");
        assert_eq!(f.primary_ticket().closed_at, stopped.closed_at, "{point:?}");
        assert_eq!(result.cleanup(), PromotionCleanup::Complete, "{point:?}");
    }
}

/// "Exit after `closing` recorded, before checkout": consent is re-checked
/// and a refusal abandons, with nothing referenced. Collaborator content on
/// the primary changes the consent (no remote: the local OIDs are bound).
#[test]
fn collaborator_change_after_closing_recorded_abandons_with_nothing_written() {
    let f = Fixture::new();
    let req = f.request();
    let context = f.context_oid();
    let _ = f.close(
        &f.failing(FailurePoint::ClosureAfterClosingRecorded),
        req.clone(),
    );
    let dangling = f.stored(req.operation_id).closure_oid.unwrap();
    edit_ticket(&f.repo, |s| s.replace("original body", "collaborator body"));
    let primary = f.primary();
    assert!(matches!(
        f.close(&f.reopened(), req.clone()),
        Err(TicketClosureError::ReconfirmationRequired)
    ));
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(f.context_oid(), context);
    assert_eq!(f.primary(), primary);
    assert!(f.reachable(&close_subject()).is_empty());
    assert!(
        refs(&f.repo).iter().all(|(_, oid)| *oid != dangling),
        "the checkpoint stays unreferenced"
    );
    assert!(matches!(
        f.close(&f.reopened(), req),
        Err(TicketClosureError::ReconfirmationRequired)
    ));
}

/// The design's reconciliation table: a `closing` context that is neither
/// untouched nor the checkpoint (here a hand commit on the context) is
/// `RecoveryRequired` with the record kept, never abandoned.
#[test]
fn hand_commit_on_the_context_after_closing_recorded_is_recovery_required_and_kept() {
    let f = Fixture::new();
    let req = f.request();
    let _ = f.close(
        &f.failing(FailurePoint::ClosureAfterClosingRecorded),
        req.clone(),
    );
    let stopped = f.stored(req.operation_id);
    edit_ticket(&f.context, |s| {
        s.replace("context body", "collaborator body")
    });
    assert!(matches!(
        f.close(&f.reopened(), req.clone()),
        Err(TicketClosureError::RecoveryRequired)
    ));
    assert_eq!(f.stored(req.operation_id), stopped);
}

#[test]
fn changed_email_after_the_checkpoint_finishes_with_the_checkpoint_authority() {
    let f = Fixture::new();
    let req = f.request();
    let _ = f.close(
        &f.failing(FailurePoint::AfterClosureCheckpoint),
        req.clone(),
    );
    f.repo
        .config()
        .unwrap()
        .set_str("user.email", "changed@example.invalid")
        .unwrap();
    outcome(f.close(&f.reopened(), req).unwrap());
    let merge = f.merge();
    assert_eq!(merge.author().email(), Some("fixture@example.invalid"));
    assert_eq!(merge.committer().email(), Some("fixture@example.invalid"));
    assert_eq!(f.primary_ticket().closed_by.as_deref(), Some(CLOSED_BY));
}

#[test]
fn advanced_primary_after_the_checkpoint_merges_in_memory() {
    let f = Fixture::new();
    let req = f.request();
    let _ = f.close(
        &f.failing(FailurePoint::AfterClosureCheckpoint),
        req.clone(),
    );
    let checkpoint = f.stored(req.operation_id).closure_oid.unwrap();
    let advanced = advance_primary(f.root(), "primary.txt");
    outcome(f.close(&f.reopened(), req).unwrap());
    let merge = f.merge();
    assert_eq!(
        merge.parent_ids().collect::<Vec<_>>(),
        [advanced, checkpoint]
    );
    assert!(f.root().join("primary.txt").is_file());
    assert!(f.primary_ticket().closed_at.is_some());
}

#[test]
fn conflicting_advance_after_the_checkpoint_needs_a_deliberate_synchronization() {
    let f = Fixture::new();
    let req = f.request();
    let _ = f.close(
        &f.failing(FailurePoint::AfterClosureCheckpoint),
        req.clone(),
    );
    let stopped = f.stored(req.operation_id);
    let advanced = edit_ticket(&f.repo, |s| s.replace("original body", "primary body"));
    assert!(matches!(
        f.close(&f.reopened(), req.clone()),
        Err(TicketClosureError::RecoveryRequired)
    ));
    assert_eq!(f.stored(req.operation_id), stopped);
    let synchronized = synchronize_context_by_hand(&f);
    outcome(f.close(&f.reopened(), req.clone()).unwrap());
    let merge = f.merge();
    assert_eq!(
        merge.parent_ids().collect::<Vec<_>>(),
        [advanced, synchronized]
    );
    let r = f.stored(req.operation_id);
    assert_eq!(r.adopted_context_oid, Some(synchronized));
    assert_eq!(r.closure_oid, stopped.closure_oid);
    assert_eq!(f.primary_ticket().closed_at, stopped.closed_at);
}

/// R-P2: once the checkpoint ref moved (a finished checkout, the checkpoint,
/// or an adopted descendant) a consent change never abandons the record.
fn never_abandons_once_the_checkpoint_ref_moved(point: FailurePoint, adopt: bool) {
    let f = Fixture::new();
    let req = f.request();
    let _ = f.close(&f.failing(point), req.clone());
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::Closing);
    if adopt {
        advance_primary(f.root(), "before-sync.txt");
        synchronize_context_by_hand(&f);
    }
    // Each would refuse consent if it were re-checked.
    advance_primary(f.root(), "after-stop.txt");
    f.repo
        .config()
        .unwrap()
        .set_str("user.email", "changed@example.invalid")
        .unwrap();
    let result = f.close(&f.reopened(), req.clone());
    assert!(
        matches!(&result, Ok(TicketClosureResult::Complete(_))),
        "{point:?} adopt={adopt}: {result:?}"
    );
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::Complete);
    assert!(f.primary_ticket().closed_at.is_some());
}
#[test]
fn never_abandons_after_the_checkout() {
    never_abandons_once_the_checkpoint_ref_moved(
        FailurePoint::ClosureDuringCheckpointCheckout,
        false,
    );
}
#[test]
fn never_abandons_after_the_checkpoint() {
    never_abandons_once_the_checkpoint_ref_moved(FailurePoint::AfterClosureCheckpoint, false);
}
#[test]
fn never_abandons_with_an_adopted_descendant() {
    never_abandons_once_the_checkpoint_ref_moved(FailurePoint::AfterClosureCheckpoint, true);
}

/// The adopted context end to end: the runner's merged context drives the
/// merge (`[base, adopted]`), the `integrating` crash reconciliation after F4
/// and the retirement.
#[test]
fn adopted_context_is_merged_reconciled_after_f4_and_retired() {
    let f = Fixture::new();
    let req = f.request();
    let _ = f.close(
        &f.failing(FailurePoint::AfterClosureCheckpoint),
        req.clone(),
    );
    let advanced = advance_primary(f.root(), "primary.txt");
    let adopted = synchronize_context_by_hand(&f);
    let stopped = f.close(&f.failing(FailurePoint::AfterIntegrationRef), req.clone());
    assert!(
        matches!(&stopped, Ok(r) if authoritative(r.clone()).cleanup() == PromotionCleanup::Pending { reason: PromotionCleanupPending::RecoveryRequired }),
        "{stopped:?}"
    );
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::Integrating);
    assert_eq!(r.adopted_context_oid, Some(adopted));
    outcome(f.close(&f.reopened(), req.clone()).unwrap());
    let merge = f.merge();
    assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [advanced, adopted]);
    assert_eq!(Some(merge.id()), f.stored(req.operation_id).integration_oid);
    assert_eq!(f.commits_in_odb(&merge_subject()), 1);
    let retired: (String, String) = f
        .db()
        .query_row(
            "SELECT kind,retired_oid FROM retired_contexts WHERE operation_id=?1",
            [req.operation_id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(retired, ("ticket".into(), adopted.to_string()));
}

// --- Process exits at every boundary ------------------------------------

/// Closure's own seams in their lifecycle order among Cycle 09's F1–F12;
/// index 15 exits inside F10 (tracking deleted), 16 inside F11 (completion
/// committed, its result lost).
const EXITS: [FailurePoint; 15] = [
    FailurePoint::AfterConfirmed,
    FailurePoint::AfterContextSync,
    FailurePoint::ClosureAfterClosingRecorded,
    FailurePoint::ClosureDuringCheckpointCheckout,
    FailurePoint::AfterClosureCheckpoint,
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
const F10_SEAM: usize = 15;
const F11_SEAM: usize = 16;

#[test]
fn closure_exit_child() {
    let Ok(root) = std::env::var("MANYHANDS_CLOSURE_EXIT_ROOT") else {
        return;
    };
    let data = std::env::var("MANYHANDS_CLOSURE_EXIT_DATA").unwrap();
    let point: usize = std::env::var("MANYHANDS_CLOSURE_EXIT_POINT")
        .unwrap()
        .parse()
        .unwrap();
    let op = OperationId::parse(&std::env::var("MANYHANDS_CLOSURE_EXIT_OP").unwrap()).unwrap();
    let digest: Vec<u8> = std::env::var("MANYHANDS_CLOSURE_EXIT_EXPECTATION")
        .unwrap()
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    let service = if point < EXITS.len() {
        RepositoryService::open_at_with_exit_point_for_testing(Path::new(&data), EXITS[point])
            .unwrap()
    } else {
        let hooks = if point == F10_SEAM {
            &TRACKING_HOOKS
        } else {
            &COMMIT_HOOKS
        };
        hooks.with(|h| {
            h.borrow_mut().insert(
                root.clone().into(),
                Box::new(|_| std::process::exit(FAILURE_POINT_EXIT_STATUS)),
            )
        });
        RepositoryService::open_at(Path::new(&data)).unwrap()
    };
    let request = CloseTicketRequest {
        root: root.into(),
        operation_id: op,
        item_id: id(),
        expected: ClosureExpectation::from_bytes(digest.try_into().unwrap()),
        approval: None,
        confirmed_identity: None,
    };
    let _ = run::run_with_io(
        &TicketClosureLifecycle,
        &service,
        lifecycle_request(request),
        &mut FakeIo::default(),
    );
    panic!("exit point was not reached")
}

/// Runs the closure in a child process that exits at `point`.
fn exit_at(f: &Fixture, req: &CloseTicketRequest, point: usize) {
    // `module_path!` names this module inside whichever test crate holds it.
    let module = module_path!().split_once("::").unwrap().1;
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("{module}::closure_exit_child")])
        .env("MANYHANDS_CLOSURE_EXIT_ROOT", f.root())
        .env("MANYHANDS_CLOSURE_EXIT_DATA", f.data.path())
        .env("MANYHANDS_CLOSURE_EXIT_POINT", point.to_string())
        .env("MANYHANDS_CLOSURE_EXIT_OP", req.operation_id.to_string())
        .env(
            "MANYHANDS_CLOSURE_EXIT_EXPECTATION",
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
}

#[test]
fn process_exit_at_every_boundary_reopens_without_repeated_effects() {
    for point in 0..=F11_SEAM {
        let f = Fixture::remote();
        let req = f.request();
        exit_at(&f, &req, point);
        let reopened = f.reopened();
        let mut io = FakeIo::default();
        let lock = f
            .repo
            .commondir()
            .join(format!("{}.lock", tracking_context_ref()));
        if point == F10_SEAM {
            // The surviving backend lock is refused, never adopted: the
            // retry changes nothing and is pending recovery.
            let before = refs(&f.repo);
            let pending = authoritative(f.run_io(&reopened, req.clone(), &mut io).unwrap());
            assert_eq!(
                pending.cleanup(),
                PromotionCleanup::Pending {
                    reason: PromotionCleanupPending::RecoveryRequired
                }
            );
            assert_eq!(refs(&f.repo), before);
            assert_eq!(f.stored(req.operation_id).step, ClosureStep::BranchDeleted);
            // The simulated operator, on this disposable fixture only.
            assert_eq!(fs::read(&lock).unwrap(), b"");
            fs::remove_file(&lock).unwrap();
        }
        if EXITS.get(point) == Some(&FailurePoint::ClosureDuringCheckpointCheckout) {
            // The checkout ran before any ref lock was taken: the exit leaves
            // no lock, and the same ID finishes below with no operator.
            assert!(
                !f.repo
                    .commondir()
                    .join(format!("{}.lock", context_ref()))
                    .exists()
            );
            assert_eq!(f.stored(req.operation_id).step, ClosureStep::Closing);
        }
        if point == F11_SEAM {
            assert_eq!(f.stored(req.operation_id).step, ClosureStep::Complete);
        }
        let retried = f
            .run_io(&reopened, req.clone(), &mut io)
            .unwrap_or_else(|e| panic!("boundary {point}: {e:?}"));
        assert!(
            matches!(retried, TicketClosureResult::Complete(_)),
            "boundary {point}: {retried:?}"
        );
        let result = outcome(retried);
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Complete,
            "boundary {point}"
        );
        assert_eq!(f.commits_in_odb(&close_subject()), 1, "boundary {point}");
        assert_eq!(f.commits_in_odb(&merge_subject()), 1, "boundary {point}");
        for effect in [
            "context_push",
            "primary_push",
            "remote_delete",
            "worktree_remove",
            "branch_delete",
        ] {
            assert_eq!(f.effects(effect), 1, "boundary {point}/{effect}");
        }
        f.assert_cleaned();
        let children = io.calls.borrow().children.len();
        assert_eq!(
            outcome(f.run_io(&reopened, req, &mut io).unwrap()),
            result,
            "boundary {point}"
        );
        assert_eq!(io.calls.borrow().children.len(), children);
    }
}

/// The adopted context through the F10 and F11 seams: the retirement and
/// the completion use the merged context, once.
#[test]
fn adopted_context_survives_the_f10_and_f11_seams() {
    for point in [F10_SEAM, F11_SEAM] {
        let f = Fixture::new();
        let req = f.request();
        let _ = f.close(
            &f.failing(FailurePoint::AfterClosureCheckpoint),
            req.clone(),
        );
        let advanced = advance_primary(f.root(), "primary.txt");
        let adopted = synchronize_context_by_hand(&f);
        exit_at(&f, &req, point);
        outcome(f.close(&f.reopened(), req.clone()).unwrap());
        assert_eq!(
            f.merge().parent_ids().collect::<Vec<_>>(),
            [advanced, adopted],
            "{point}"
        );
        let retired: String = f
            .db()
            .query_row(
                "SELECT retired_oid FROM retired_contexts WHERE kind='ticket'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(retired, adopted.to_string(), "{point}");
        assert_eq!(f.effects("branch_delete"), 1, "{point}");
        assert_eq!(f.effects("worktree_remove"), 1, "{point}");
    }
}

// --- Cancellation and refresh -------------------------------------------

#[test]
fn cancellation_before_integration_is_interrupted_and_the_same_id_continues() {
    let f = Fixture::new();
    let req = f.request();
    let (root, op) = (f.root().to_owned(), req.operation_id);
    f.hook(move |service, point| {
        if point == FailurePoint::AfterContextSync {
            assert!(service.cancel_ticket_closure(&root, op).unwrap());
        }
    });
    assert!(matches!(
        f.close(&f.service, req.clone()),
        Err(TicketClosureError::Interrupted)
    ));
    HOOKS.with(|h| h.borrow_mut().remove(f.root()));
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::ContextSynchronized);
    assert!(r.cancel_requested);
    assert_eq!(f.commits_in_odb(&close_subject()), 0);
    outcome(f.close(&f.service, req).unwrap());
}

#[test]
fn cancellation_after_integration_leaves_publication_and_cleanup_cancelled() {
    let f = Fixture::new();
    let req = f.request();
    let (root, op) = (f.root().to_owned(), req.operation_id);
    f.hook(move |service, point| {
        if point == FailurePoint::AfterIntegrationRef {
            assert!(service.cancel_ticket_closure(&root, op).unwrap());
        }
    });
    let result = authoritative(f.close(&f.service, req.clone()).unwrap());
    assert_eq!(
        result.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::Cancelled
        }
    );
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::Cancelled
        }
    );
    assert!(matches!(
        result.integration(),
        PromotionIntegration::Merged { .. }
    ));
    assert!(
        !f.service
            .cancel_ticket_closure(f.root(), OperationId::new())
            .unwrap()
    );
}

/// The primary child completes durably, then the parent is cancelled and
/// the child's return is lost: its authority is still consumed.
struct CancelAfterPrimary {
    inner: FakeIo,
    parent: OperationId,
}
impl PromotionIo for CancelAfterPrimary {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let root = request.root.clone();
        let primary = request.target == SynchronizationTarget::Primary;
        let result = self
            .inner
            .synchronize(service, request, observer, execution)?;
        if !primary {
            return Ok(result);
        }
        assert!(service.cancel_ticket_closure(&root, self.parent).unwrap());
        Err(SynchronizationError::Interrupted)
    }
    fn delete(
        &mut self,
        service: &RepositoryService,
        request: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        configuration: &ObservationConfiguration,
        expected: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.inner
            .delete(service, request, plan, owner, configuration, expected, item)
    }
}

#[test]
fn completed_child_authority_is_consumed_before_the_cancellation() {
    let f = Fixture::remote();
    let req = f.request();
    let mut io = CancelAfterPrimary {
        inner: FakeIo::default(),
        parent: req.operation_id,
    };
    let result = authoritative(f.run_io(&f.service, req.clone(), &mut io).unwrap());
    assert_eq!(
        result.publication(),
        PromotionPublication::Published { oid: f.primary() }
    );
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::Cancelled
        }
    );
    assert_eq!(io.inner.calls.borrow().deletes, 0);
}

#[test]
fn every_ok_after_integration_refreshes_or_is_index_pending() {
    let f = Fixture::new();
    let req = f.request();
    let pending = f
        .close(&f.failing(FailurePoint::RefreshFailure), req.clone())
        .unwrap();
    assert!(matches!(pending, TicketClosureResult::IndexPending(_)));
    assert!(
        f.service
            .repository_snapshot(f.root())
            .unwrap()
            .refresh_required
    );
    let replay = f.close(&f.service, req).unwrap();
    assert_eq!(authoritative(replay.clone()), authoritative(pending));
    assert!(matches!(replay, TicketClosureResult::Complete(_)));
    assert!(
        !f.service
            .repository_snapshot(f.root())
            .unwrap()
            .refresh_required
    );
}

#[test]
fn a_completed_closure_reads_closed_from_the_primary_only() {
    let f = Fixture::new();
    let req = f.request();
    outcome(f.close(&f.service, req.clone()).unwrap());
    let closed_at = f.stored(req.operation_id).closed_at.unwrap();
    let repository = f.service.resolve_repository(f.root()).unwrap();
    let item = f.service.show_item(&repository, &id()).unwrap();
    assert_eq!(item.context.kind, ItemContextKind::Primary);
    assert_eq!(item.status.as_deref(), Some("open"));
    assert_eq!(
        item.closure,
        Some(ClosureDto {
            state: ClosureState::Closed,
            closed_at: Some(
                closed_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap()
            ),
            closed_by: Some(CLOSED_BY.into()),
        })
    );
    let listed = f
        .service
        .list_tickets(&repository, &TicketFilter::default())
        .unwrap();
    let copies: Vec<_> = listed
        .items
        .iter()
        .filter(|i| i.id.as_deref() == Some(ID))
        .collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].context.kind, ItemContextKind::Primary);
    assert_eq!(copies[0].closure, item.closure);
}

// --- Cleanup with a remote ----------------------------------------------

/// Records the guard each exact delete was given.
#[derive(Default)]
struct GuardIo {
    inner: FakeIo,
    guards: Rc<RefCell<Vec<Oid>>>,
}
impl PromotionIo for GuardIo {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        self.inner
            .synchronize(service, request, observer, execution)
    }
    fn delete(
        &mut self,
        service: &RepositoryService,
        request: VerifySshTransportRequest,
        plan: &RemoteRefPlan,
        owner: &RemoteReservation,
        configuration: &ObservationConfiguration,
        expected: Oid,
        item: &ItemId,
    ) -> Result<DeleteObservation, SynchronizationError> {
        self.guards.borrow_mut().push(expected);
        self.inner
            .delete(service, request, plan, owner, configuration, expected, item)
    }
}
impl GuardIo {
    fn child_targets(&self) -> Vec<SynchronizationTarget> {
        self.inner
            .calls
            .borrow()
            .children
            .iter()
            .map(|(t, _, _)| t.clone())
            .collect()
    }
}
fn close_reservations(f: &Fixture, op: OperationId) -> i64 {
    f.db()
        .query_row(
            "SELECT count(*) FROM remote_operation_records WHERE operation_ulid=?1",
            [op.to_string()],
            |r| r.get(0),
        )
        .unwrap()
}

/// (a) A context child ran: the delete is guarded by its publication, not
/// by the older tracking OID; (d) the retirement is the merged context.
#[test]
fn a_child_publication_guards_the_delete_and_the_checkpoint_is_retired() {
    let f = Fixture::remote();
    let tracking = f.repo.refname_to_id(&tracking_context_ref()).unwrap();
    let context = f.context_oid();
    assert_ne!(tracking, context);
    let req = f.request();
    let mut io = GuardIo::default();
    outcome(f.run_io(&f.service, req.clone(), &mut io).unwrap());
    let r = f.stored(req.operation_id);
    assert_eq!(r.remote_context_oid, Some(tracking));
    assert_eq!(*io.guards.borrow(), [context]);
    let retired: (String, String, String) = f
        .db()
        .query_row(
            "SELECT kind,item_id,retired_oid FROM retired_contexts WHERE operation_id=?1",
            [req.operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        retired,
        (
            "ticket".into(),
            ID.into(),
            r.closure_oid.unwrap().to_string()
        )
    );
    assert!(f.repo.find_reference(&tracking_context_ref()).is_err());
    f.assert_cleaned();
}

/// A context merged into the primary by hand, without closure fields
/// ("contained, open, open"): no context child runs.
fn contained(tracking: bool) -> Fixture {
    let f = Fixture(support::Fixture::new(true));
    let context = f.context_oid();
    checkout(&f.repo, context);
    f.tracking("main", context);
    if tracking {
        // Published once before its last local commit: the guard must be
        // this OID, never the context tip.
        let published = f.repo.find_commit(context).unwrap().parent_id(0).unwrap();
        f.repo
            .reference(&tracking_context_ref(), published, true, "fixture")
            .unwrap();
    }
    f
}

/// (b) No context child ran: the pre-checkpoint tracking OID guards it.
#[test]
fn without_a_child_the_tracking_oid_guards_the_delete() {
    let f = contained(true);
    let context = f.context_oid();
    let tracking = f.repo.refname_to_id(&tracking_context_ref()).unwrap();
    assert_ne!(tracking, context);
    let req = f.request();
    let mut io = GuardIo::default();
    let result = outcome(f.run_io(&f.service, req.clone(), &mut io).unwrap());
    assert!(matches!(
        result.closure(),
        ClosureRecording::Recorded { .. }
    ));
    assert_eq!(io.child_targets(), [SynchronizationTarget::Primary]);
    assert_eq!(*io.guards.borrow(), [tracking]);
    let r = f.stored(req.operation_id);
    assert_eq!(r.remote_context_oid, Some(tracking));
    assert_eq!(r.context_attempt, 0);
    assert_eq!(close_reservations(&f, req.operation_id), 1);
    f.assert_cleaned();
}

/// (c) No child and no tracking context: nothing to delete, no reservation.
#[test]
fn without_a_child_or_tracking_context_nothing_is_reserved_or_deleted() {
    let f = contained(false);
    let req = f.request();
    let mut io = GuardIo::default();
    outcome(f.run_io(&f.service, req.clone(), &mut io).unwrap());
    assert_eq!(io.child_targets(), [SynchronizationTarget::Primary]);
    assert!(io.guards.borrow().is_empty());
    assert_eq!(io.inner.calls.borrow().deletes, 0);
    assert_eq!(close_reservations(&f, req.operation_id), 0);
    f.assert_cleaned();
}

/// R-T5-2: only a closure that minted no context attempt falls back to the
/// tracking OID, the same proxy as the required-delete rule. A minted
/// attempt that never reserved (its child state is absent) has no guard:
/// cleanup stops `RecoveryRequired` before any delete, and the remote and
/// local context are kept.
///
/// The test mints the attempt while the context is already contained, an
/// order the runner never uses. Production reaches the same row when the
/// attempt is minted first and the primary then absorbs a context whose net
/// change was already empty, so the consent digest stays equal.
#[test]
fn a_minted_context_attempt_that_never_reserved_has_no_delete_guard() {
    let f = contained(true);
    let req = f.request();
    let mut io = GuardIo::default();
    let _ = f.run_io(
        &f.failing(FailurePoint::AfterConfirmed),
        req.clone(),
        &mut io,
    );
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::Confirmed);
    let minted = state::with_transaction(&f.service, f.root(), |tx, rid| {
        record::next_attempt(tx, rid, req.operation_id, record::ChildRole::Context)
    })
    .unwrap();
    assert_eq!(minted.context_attempt, 1);
    let result = authoritative(f.run_io(&f.reopened(), req.clone(), &mut io).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    // The contained context ran no child; the minted one never reserved.
    assert_eq!(io.child_targets(), [SynchronizationTarget::Primary]);
    assert!(io.guards.borrow().is_empty(), "no tracking fallback");
    assert_eq!(io.inner.calls.borrow().deletes, 0);
    let r = f.stored(req.operation_id);
    assert_eq!(r.context_attempt, 1);
    assert_eq!(r.step, ClosureStep::Published);
    assert!(f.root().join(".manyhands/worktrees").join(ID).exists());
    assert!(f.repo.find_reference(&tracking_context_ref()).is_ok());
}

/// (e) After a `PrimaryAdvanced` reset, a collaborator merges the context
/// into the primary and no tracking context is left: the cleanup set the
/// consent binds changes (`remote_branch` no longer planned), so the same ID
/// stops `ReconfirmationRequired` before any write and never reaches a
/// cleanup that would need an absent guard.
#[test]
fn a_reset_that_leaves_the_context_contained_without_tracking_reconfirms() {
    let f = Fixture::remote();
    let req = f.request();
    let context = f.context_oid();
    let root = f.root().to_owned();
    f.hook(move |_, point| {
        if point == FailurePoint::AfterContextSync {
            let repo = git2::Repository::open(&root).unwrap();
            let base = repo.refname_to_id("refs/remotes/origin/main").unwrap();
            let advanced = support::tree_commit(
                &repo,
                base,
                &[("collaborator.txt", Some((&b"collaborator\n"[..], 0o100644)))],
            );
            repo.reference("refs/remotes/origin/main", advanced, true, "fixture")
                .unwrap();
        }
    });
    let mut io = FakeIo::default();
    assert!(matches!(
        f.run_io(&f.service, req.clone(), &mut io),
        Err(TicketClosureError::PrimaryAdvanced)
    ));
    HOOKS.with(|h| h.borrow_mut().remove(f.root()));
    let reset = f.stored(req.operation_id);
    assert_eq!(reset.step, ClosureStep::Confirmed);
    assert_eq!(reset.context_attempt, 2);
    // The collaborator merges the context into the remote primary; the
    // remote context branch is gone from this clone's view.
    let advanced = f.repo.refname_to_id("refs/remotes/origin/main").unwrap();
    let ours = f.repo.find_commit(advanced).unwrap();
    let theirs = f.repo.find_commit(context).unwrap();
    let mut index = f.repo.merge_commits(&ours, &theirs, None).unwrap();
    let tree = f
        .repo
        .find_tree(index.write_tree_to(&f.repo).unwrap())
        .unwrap();
    let sig = git2::Signature::now("Collaborator", "collaborator@example.invalid").unwrap();
    let merged = f
        .repo
        .commit(
            None,
            &sig,
            &sig,
            "collaborator merge",
            &tree,
            &[&ours, &theirs],
        )
        .unwrap();
    f.tracking("main", merged);
    f.repo
        .find_reference(&tracking_context_ref())
        .unwrap()
        .delete()
        .unwrap();
    let preview = f.preview();
    assert!(!preview.cleanup().remote_branch);
    assert_ne!(preview.expectation(), req.expected);
    assert!(matches!(
        f.run_io(&f.service, req.clone(), &mut io),
        Err(TicketClosureError::ReconfirmationRequired)
    ));
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(r.closure_oid, None);
    assert_eq!(f.context_oid(), context);
    assert_eq!(f.commits_in_odb(&close_subject()), 0);
    assert_eq!(io.calls.borrow().deletes, 0);
}

// --- The remote context branch never receives the checkpoint (C1, C9) ---

/// A written checkpoint refuses a manual synchronization of the ticket
/// context, which would push the checkpoint to the remote context branch.
#[test]
fn a_written_checkpoint_refuses_a_manual_context_synchronization() {
    let f = Fixture::remote();
    let req = f.request();
    let mut io = FakeIo::default();
    let _ = f.run_io(
        &f.failing(FailurePoint::AfterClosureCheckpoint),
        req.clone(),
        &mut io,
    );
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::Closing);
    let plan = f.configure_endpoints();
    let sync = OperationId::new();
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root(), sync, &f.manual_context_sync(&plan))
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    assert_eq!(close_reservations(&f, sync), 0);
    // The closure itself still finishes, with no further context child.
    let children = io.calls.borrow().children.len();
    outcome(f.run_io(&f.reopened(), req, &mut io).unwrap());
    assert_eq!(io.calls.borrow().children.len(), children + 1);
    assert_eq!(f.effects("context_push"), 1);
}

/// The other order: a manual context synchronization reserved before the
/// checkpoint holds the checkpoint off (`Busy`, nothing written) until it
/// ends; the same ID then finishes.
#[test]
fn an_active_manual_context_synchronization_holds_off_the_checkpoint() {
    let f = Fixture::remote();
    let req = f.request();
    let context = f.context_oid();
    let plan = f.configure_endpoints();
    let target = f.manual_context_sync(&plan);
    let sync = OperationId::new();
    let root = f.root().to_owned();
    f.hook(move |service, point| {
        if point == FailurePoint::AfterContextSync {
            assert!(matches!(
                service
                    .reserve_remote_operation(&root, sync, &target)
                    .unwrap(),
                RemoteReservationOutcome::Reserved(_)
            ));
        }
    });
    let mut io = FakeIo::default();
    assert!(matches!(
        f.run_io(&f.service, req.clone(), &mut io),
        Err(TicketClosureError::Busy)
    ));
    HOOKS.with(|h| h.borrow_mut().remove(f.root()));
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::ContextSynchronized);
    assert_eq!(r.closure_oid, None);
    assert_eq!(f.context_oid(), context);
    // Step 5 built its objects unreferenced; no ref reaches them.
    assert!(f.reachable(&close_subject()).is_empty());
    f.db()
        .execute(
            "UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled' WHERE operation_ulid=?1",
            [sync.to_string()],
        )
        .unwrap();
    outcome(f.run_io(&f.service, req, &mut io).unwrap());
    assert!(f.primary_ticket().closed_at.is_some());
}

// --- Fix round 1 (R-T6-3, R-T6-4 and minors) -----------------------------

/// One automatic poll after a fetch that left `advertised` as the remote's
/// refs (the tracking refs are the test's).
fn poll(f: &Fixture, advertised: Vec<(String, Oid)>) -> crate::repository::RemotePollReport {
    use crate::repository::{
        RemoteOperationPriority, RemoteOperationSafePoint, RemotePollOutcome, RemotePollResult,
        remote::reservation::RemotePollReservationOutcome,
    };
    let plan = f.configure_endpoints();
    let configuration = f
        .service
        .observation_configuration(f.root(), &plan)
        .unwrap();
    let owner = match f
        .service
        .reserve_remote_poll(
            f.root(),
            OperationId::new(),
            &RemoteOperationTarget::for_poll(&plan),
            RemoteOperationPriority::Poll,
        )
        .unwrap()
    {
        RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Reserved(o)) => o,
        other => panic!("{other:?}"),
    };
    f.service
        .remote_safe_point(f.root(), &owner, RemoteOperationSafePoint::BeforeTransport)
        .unwrap();
    match f
        .service
        .poll_after_fetch(f.root(), &owner, &plan, &configuration, advertised)
        .unwrap()
    {
        RemotePollResult::Complete(RemotePollOutcome::Polled(r)) => r,
        RemotePollResult::IndexPending(p) => match p.authoritative {
            RemotePollOutcome::Polled(r) => r,
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

/// R-T6-4, the brief's "collaborator content arriving" with a remote: a
/// poll in the `closing` window updates tracking but never fast-forwards the
/// ticket context, so the same ID re-checks consent and abandons with
/// nothing written; after that a poll may fast-forward and a new ID closes.
#[test]
fn a_poll_while_closing_updates_tracking_but_never_moves_the_context() {
    let f = Fixture::remote();
    let req = f.request();
    let mut io = FakeIo::default();
    let _ = f.run_io(
        &f.failing(FailurePoint::ClosureAfterClosingRecorded),
        req.clone(),
        &mut io,
    );
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::Closing);
    let context = f.context_oid();
    // A collaborator pushes to the ticket context.
    let pushed = support::tree_commit(
        &f.repo,
        context,
        &[("collaborator.txt", Some((&b"collaborator\n"[..], 0o100644)))],
    );
    f.repo
        .reference(&tracking_context_ref(), pushed, true, "fixture")
        .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let remote_ref = plan
        .context(AuthoringKind::Ticket, &id())
        .remote_ref()
        .to_owned();
    let main = f.repo.refname_to_id("refs/remotes/origin/main").unwrap();
    let advertised = vec![("refs/heads/main".to_owned(), main), (remote_ref, pushed)];
    poll(&f, advertised.clone());
    assert_eq!(f.context_oid(), context, "the closing context is not moved");
    assert_eq!(
        f.repo.refname_to_id(&tracking_context_ref()).unwrap(),
        pushed
    );
    assert!(matches!(
        f.run_io(&f.reopened(), req.clone(), &mut io),
        Err(TicketClosureError::ReconfirmationRequired)
    ));
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(f.context_oid(), context);
    assert!(f.reachable(&close_subject()).is_empty());
    // The closure is over: a poll may move the context, and a new ID closes.
    poll(&f, advertised);
    assert_eq!(f.context_oid(), pushed);
    outcome(f.run_io(&f.service, f.request(), &mut io).unwrap());
    assert!(f.primary_ticket().closed_at.is_some());
}

/// Poll materialization (6B), the remote-deletion case: from the start of
/// cleanup the closure holds its close slot until completion, and a stop
/// keeps it, so no poll runs (`Busy`) while the context is half removed.
#[test]
fn a_poll_cannot_run_while_the_closure_holds_its_close_slot() {
    use crate::repository::{
        RemoteOperationPriority, remote::reservation::RemotePollReservationOutcome,
    };
    let f = Fixture::remote();
    let req = f.request();
    let mut io = FakeIo::default();
    let _ = f.run_io(
        &f.failing(FailurePoint::BeforeCompletion),
        req.clone(),
        &mut io,
    );
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::BranchDeleted);
    assert_eq!(close_reservations(&f, req.operation_id), 1);
    let plan = f.configure_endpoints();
    assert!(matches!(
        f.reopened()
            .reserve_remote_poll(
                f.root(),
                OperationId::new(),
                &RemoteOperationTarget::for_poll(&plan),
                RemoteOperationPriority::Poll,
            )
            .unwrap(),
        RemotePollReservationOutcome::Reservation(RemoteReservationOutcome::Busy)
    ));
    outcome(f.run_io(&f.reopened(), req, &mut io).unwrap());
    f.assert_cleaned();
}

/// Poll materialization (6B), the reachable case: a closure that deletes
/// no remote branch (contained, no tracking context) holds no slot. Stopped
/// at `branch_deleted`, its worktree and branch are gone but not retired. A
/// collaborator then pushes the ticket context branch; the poll must not
/// rebuild the context, as R-T6-4 fences the fast-forward.
#[test]
fn a_poll_never_rebuilds_the_context_while_closure_cleanup_is_unfinished() {
    let f = contained(false);
    let req = f.request();
    let mut io = GuardIo::default();
    let _ = f.run_io(
        &f.failing(FailurePoint::BeforeCompletion),
        req.clone(),
        &mut io,
    );
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::BranchDeleted);
    assert_eq!(close_reservations(&f, req.operation_id), 0);
    f.assert_cleaned();
    // The collaborator's push reaches tracking.
    let pushed = r.merged_context().unwrap();
    f.repo
        .reference(&tracking_context_ref(), pushed, true, "fixture")
        .unwrap();
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let remote_ref = plan
        .context(AuthoringKind::Ticket, &id())
        .remote_ref()
        .to_owned();
    let main = f.repo.refname_to_id("refs/remotes/origin/main").unwrap();
    let report = poll(
        &f,
        vec![("refs/heads/main".to_owned(), main), (remote_ref, pushed)],
    );
    let target = report
        .targets()
        .iter()
        .find(|t| t.item_id() == Some(&id()))
        .expect("the ticket context is a poll target");
    assert_eq!(
        target.disposition(),
        crate::repository::PollDisposition::BlockedByLocalState
    );
    f.assert_cleaned();
    assert!(f.repo.find_worktree(ID).is_err());
    // The same ID re-checks under the lease. The tracking context it would
    // retire has changed, so its guarded tracking delete fails closed: it
    // stays at `branch_deleted`, recovery required, and recreates nothing.
    let result = authoritative(f.run_io(&f.reopened(), req.clone(), &mut io).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::BranchDeleted);
    assert_eq!(
        f.repo.refname_to_id(&tracking_context_ref()).unwrap(),
        pushed
    );
    f.assert_cleaned();
}

/// Obligation 9, restart: a manual context synchronization interrupted
/// before the checkpoint cannot be restarted once it is written.
#[test]
fn an_interrupted_manual_context_synchronization_cannot_restart_after_the_checkpoint() {
    let f = Fixture::remote();
    let plan = f.configure_endpoints();
    let target = f.manual_context_sync(&plan);
    let sync = OperationId::new();
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root(), sync, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    f.db()
        .execute(
            "UPDATE remote_operation_records SET phase='interrupted' WHERE operation_ulid=?1",
            [sync.to_string()],
        )
        .unwrap();
    let req = f.request();
    let mut io = FakeIo::default();
    let _ = f.run_io(
        &f.failing(FailurePoint::AfterClosureCheckpoint),
        req.clone(),
        &mut io,
    );
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::Closing);
    assert!(matches!(
        f.service
            .restart_remote_synchronization(f.root(), sync, &target)
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
}

/// Obligation 9 after integration: the fence lasts until retirement.
#[test]
fn a_manual_context_synchronization_is_refused_while_publication_is_pending() {
    let f = Fixture::remote();
    let req = f.request();
    let mut io = FakeIo {
        failure: Some((true, SynchronizationError::PushRejected)),
        ..FakeIo::default()
    };
    let result = authoritative(f.run_io(&f.service, req.clone(), &mut io).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::AwaitingPublication
        }
    );
    assert_eq!(
        f.stored(req.operation_id).step,
        ClosureStep::PublicationPending
    );
    let plan = f.configure_endpoints();
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root(), OperationId::new(), &f.manual_context_sync(&plan))
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
}

/// Minor 2: the hold-off comes before the identity step, so a confirmed
/// identity is not persisted when the closure is held off.
#[test]
fn a_held_off_checkpoint_persists_no_confirmed_identity() {
    let f = Fixture::remote();
    {
        let mut config = f.repo.config().unwrap();
        config.set_str("user.name", "").unwrap();
        config.set_str("user.email", "").unwrap();
    }
    let expected = match f
        .service
        .preview_ticket_closure(PreviewTicketClosureRequest {
            root: f.root().into(),
            item_id: id(),
            confirmed_identity: None,
        })
        .unwrap()
    {
        TicketClosurePreflight::Blocked {
            identity_configuration: Some(expected),
            ..
        } => expected,
        other => panic!("expected the identity boundary: {other:?}"),
    };
    let confirmation = ConfirmedCommitIdentity {
        confirmation_id: OperationId::new(),
        identity: crate::repository::CommitIdentity {
            name: "Confirmed".into(),
            email: "confirmed@example.invalid".into(),
        },
        expected_configuration: expected,
    };
    let preview = match f
        .service
        .preview_ticket_closure(PreviewTicketClosureRequest {
            root: f.root().into(),
            item_id: id(),
            confirmed_identity: Some(confirmation.clone()),
        })
        .unwrap()
    {
        TicketClosurePreflight::Ready(p) => p,
        other => panic!("not ready: {other:?}"),
    };
    let req = CloseTicketRequest {
        root: f.root().into(),
        operation_id: OperationId::new(),
        item_id: id(),
        expected: preview.expectation(),
        approval: None,
        confirmed_identity: Some(confirmation),
    };
    let plan = f.configure_endpoints();
    let target = f.manual_context_sync(&plan);
    let root = f.root().to_owned();
    f.hook(move |service, point| {
        if point == FailurePoint::AfterContextSync {
            assert!(matches!(
                service
                    .reserve_remote_operation(&root, OperationId::new(), &target)
                    .unwrap(),
                RemoteReservationOutcome::Reserved(_)
            ));
        }
    });
    let config = fs::read(f.repo.path().join("config")).unwrap();
    let mut io = FakeIo::default();
    assert!(matches!(
        f.run_io(&f.service, req.clone(), &mut io),
        Err(TicketClosureError::Busy)
    ));
    assert_eq!(fs::read(f.repo.path().join("config")).unwrap(), config);
    assert_eq!(f.stored(req.operation_id).closure_oid, None);
}

/// Minor 6: read after closure while the context still exists (cleanup
/// cancelled). The read API lists one effective copy, the kept context's
/// while it exists; it is closed, as is the primary's ticket.
#[test]
fn a_closed_ticket_with_its_context_kept_lists_one_closed_copy() {
    let f = Fixture::new();
    let req = f.request();
    let (root, op) = (f.root().to_owned(), req.operation_id);
    f.hook(move |service, point| {
        if point == FailurePoint::AfterIntegrationRef {
            assert!(service.cancel_ticket_closure(&root, op).unwrap());
        }
    });
    let result = authoritative(f.close(&f.service, req.clone()).unwrap());
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::Cancelled
        }
    );
    assert!(f.root().join(".manyhands/worktrees").join(ID).exists());
    let repository = f.service.resolve_repository(f.root()).unwrap();
    let listed = f
        .service
        .list_tickets(&repository, &TicketFilter::default())
        .unwrap();
    let copies: Vec<_> = listed
        .items
        .iter()
        .filter(|i| i.id.as_deref() == Some(ID))
        .collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(
        copies[0].closure.as_ref().map(|c| c.state),
        Some(ClosureState::Closed)
    );
    assert_eq!(copies[0].status.as_deref(), Some("open"));
    let primary = f.primary_ticket();
    assert!(primary.closed_at.is_some());
    assert_eq!(primary.status, "open");
}
