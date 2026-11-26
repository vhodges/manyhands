//! The pull-request close through the runner: the consent a confirm
//! re-observes in the configuration's mode, and the branch runner after it:
//! the context's child, the closure written on the context, the publication
//! child and the answers, each of the record's and none of the primary's.
use super::*;
use crate::{
    canonical::Integration,
    repository::{
        SynchronizationOutcome, SynchronizationStage,
        promotion::{
            lifecycle::{Lifecycle, LifecycleRequest},
            record::CleanupCategory,
            run_tests::complete_child_row,
        },
        pull_request_links::{link_of, linked_items_for_test},
        remote::sync,
    },
};
use record::ChildRole;
use std::collections::VecDeque;
use time::OffsetDateTime;

const DIRECT_CONFIG: &str =
    "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";
const PULL_REQUEST_CONFIG: &str = "format_version = 2\nintegration = \"pull_request\"\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";

/// Commits the repository's configuration for `mode` on the primary, and
/// moves the tracking primary with it, so only the mode differs.
fn switch(f: &Fixture, mode: Integration) -> Oid {
    let source = match mode {
        Integration::Direct => DIRECT_CONFIG,
        Integration::PullRequest => PULL_REQUEST_CONFIG,
    };
    write(&f.repo, ".manyhands/config.toml", source);
    let oid = commit(&f.repo);
    f.tracking("main", oid);
    oid
}

/// What a confirm that was refused for its consent leaves behind: one
/// abandoned row of the confirm's path with no answer, no commit, no child
/// and no ref moved.
fn assert_refused_untouched(
    f: &Fixture,
    req: &CloseTicketRequest,
    io: &FakeIo,
    path: (Integration, ClosureAction),
    refs_before: &[(String, Oid)],
    case: &str,
) {
    let r = f.stored(req.operation_id);
    assert_eq!(r.step, ClosureStep::Abandoned, "{case}");
    assert_eq!(r.refusal, Some(record::Refusal::ExternalChange), "{case}");
    assert_eq!((r.integration_mode, r.action), path, "{case}");
    assert_eq!(r.answer_code, None, "{case}");
    assert_eq!(r.closure_oid, None, "{case}");
    assert_eq!(f.commits_in_odb(&close_subject()), 0, "{case}");
    assert!(io.calls.borrow().children.is_empty(), "{case}");
    assert_eq!(io.calls.borrow().deletes, 0, "{case}");
    assert_eq!(refs(&f.repo), refs_before, "{case}");
}

/// D17: a preview made in one mode is `external_change` when confirmed in
/// the other, either way, and the refused row is the confirm's mode's.
#[test]
fn d17_a_mode_switch_between_prepare_and_confirm_is_external_change() {
    for (prepared, confirmed) in [
        (Integration::Direct, Integration::PullRequest),
        (Integration::PullRequest, Integration::Direct),
    ] {
        let case = format!("{prepared:?} to {confirmed:?}");
        let f = Fixture::remote();
        if prepared == Integration::PullRequest {
            switch(&f, prepared);
        }
        let req = f.request();
        switch(&f, confirmed);
        let refs_before = refs(&f.repo);
        let mut io = FakeIo::default();
        let result = run::run_with_io(
            &TicketClosureLifecycle,
            &f.service,
            lifecycle_request(req.clone()),
            &mut io,
        );
        assert!(
            matches!(result, Err(TicketClosureError::ExternalChange)),
            "{case}: {result:?}"
        );
        assert_refused_untouched(
            &f,
            &req,
            &io,
            (confirmed, ClosureAction::Close),
            &refs_before,
            &case,
        );
    }
}

/// D18: the identity changed, or went, between prepare and confirm: the
/// digest binds `closed_by`, so the confirm is `external_change`, never
/// `identity_required`.
#[test]
fn d18_an_identity_change_between_prepare_and_confirm_is_external_change() {
    for (name, email) in [("Fixture", "changed@example.invalid"), ("", "")] {
        let case = format!("{name:?} {email:?}");
        let f = Fixture::remote();
        switch(&f, Integration::PullRequest);
        let req = f.request();
        // Empty local values also hide any identity configured elsewhere.
        let mut config = f.repo.config().unwrap();
        config.set_str("user.name", name).unwrap();
        config.set_str("user.email", email).unwrap();
        let refs_before = refs(&f.repo);
        let mut io = FakeIo::default();
        let result = run::run_with_io(
            &TicketClosureLifecycle,
            &f.service,
            lifecycle_request(req.clone()),
            &mut io,
        );
        assert!(
            matches!(result, Err(TicketClosureError::ExternalChange)),
            "{case}: {result:?}"
        );
        assert_refused_untouched(
            &f,
            &req,
            &io,
            (Integration::PullRequest, ClosureAction::Close),
            &refs_before,
            &case,
        );
    }
}

/// R25: a new request observes in the configuration's mode, and a
/// continuation of a record observes in the record's. A direct record at a
/// supersedable cleanup stop, met after a switch to pull-request mode, is a
/// closed ticket to the new request, and the old record is left as it was.
#[test]
fn a_new_request_observes_live_and_a_continuation_its_record_s_mode() {
    let f = Fixture::remote();
    let first = f.request();
    let mut io = FakeIo {
        deletion: Some(DeleteObservation::Changed(f.primary())),
        ..FakeIo::default()
    };
    let pending = authoritative(f.run_io(&f.service, first.clone(), &mut io).unwrap());
    assert!(matches!(
        pending.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RemoteContextChanged
        }
    ));
    let old = f.stored(first.operation_id);
    assert_eq!(old.cleanup, Some(CleanupCategory::RemoteContextChanged));
    assert_eq!(
        (old.integration_mode, old.action),
        (Integration::Direct, ClosureAction::Close)
    );
    // The direct digest the closed, merged ticket has, before the switch.
    let direct = f.preview().expectation();
    switch(&f, Integration::PullRequest);
    let request = PreviewTicketClosureRequest {
        root: f.root().into(),
        item_id: id(),
        confirmed_identity: None,
    };
    let observed = |mode| {
        observe::prepare_observation_in(&f.service, &request, mode)
            .unwrap()
            .observed
    };
    // A new request: the live mode, pull request. The tracking primary's
    // ticket holds a closure, and nothing else blocks.
    match observed(None) {
        Err(refusal) => assert_eq!(refusal.blockers, [ClosureBlocker::AlreadyClosed]),
        Ok(o) => panic!("expected blocked, observed {:?}", o.path),
    }
    // Its continuation: the record's mode, whatever the configuration says.
    match observed(Some(Integration::Direct)) {
        Ok(o) => {
            assert_eq!(o.path, (Integration::Direct, ClosureAction::Close));
            assert_eq!(o.expectation, direct);
        }
        Err(refusal) => panic!("expected ready: {:?}", refusal.blockers),
    }
    // The runner asks for the record's mode through the preview request.
    let request_of =
        |record| TicketClosureLifecycle::preview_request(f.root().into(), id(), None, record).mode;
    assert_eq!(request_of(Some(&old)), Some(Integration::Direct));
    assert_eq!(request_of(None), None);
    // Neither observation touched the old record.
    assert_eq!(
        format!("{:?}", f.stored(first.operation_id)),
        format!("{old:?}")
    );
}

/// R23: a synchronization of the ticket's context that is not a record's own
/// is `busy`. The binding's prepare names the row, and a new request's
/// confirm answers `RepositoryBusy` with the blocker, which is not mapped
/// to `Busy` and inserts nothing.
#[test]
fn a_foreign_context_synchronization_is_busy_at_prepare_and_confirm() {
    use crate::repository::{Mutation, TicketCloseInput};
    let f = Fixture::remote();
    switch(&f, Integration::PullRequest);
    let req = f.request();
    let plan = f.configure_endpoints();
    let target = f.manual_context_sync(&plan);
    let foreign = OperationId::new();
    assert!(matches!(
        f.service
            .reserve_remote_operation(f.root(), foreign, &target)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    let prepared = f.service.prepare(&Mutation::TicketClose(TicketCloseInput {
        root: f.root().to_owned(),
        id: id(),
    }));
    let value = serde_json::to_value(&prepared).unwrap();
    assert_eq!(value["code"], "busy", "{value}");
    assert_eq!(
        value["data"]["blocker"]["operation_id"],
        foreign.to_string(),
        "{value}"
    );
    let mut io = FakeIo::default();
    match f.run_io(&f.service, req.clone(), &mut io) {
        Err(TicketClosureError::Repository(e)) => {
            assert_eq!(e.kind, RepositoryErrorKind::RepositoryBusy);
            assert_eq!(e.blocker().map(|b| b.operation_id), Some(foreign));
        }
        other => panic!("expected a repository busy: {other:?}"),
    }
    let records: i64 = f
        .db()
        .query_row("SELECT count(*) FROM ticket_closures", [], |r| r.get(0))
        .unwrap();
    assert_eq!(records, 0);
    assert!(io.calls.borrow().children.is_empty());
}

// --- The branch runner, through the fixture below ------------------------

/// The clock a pull-request run is given: a branch record's `closed_at`.
const CLOCK: i64 = 1_795_000_000;
fn clock() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(CLOCK).unwrap()
}

/// A later clock than `CLOCK`: a withdraw, or a second close, run a day on.
fn later() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(CLOCK + 86_400).unwrap()
}

/// A ticket other than the one the fixture closes, that depends on it.
const DEPENDENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";

/// A repository in pull-request mode: the configuration is committed on the
/// primary, which a pull-request close never moves.
struct PullRequest {
    f: Fixture,
    primary_before: Oid,
}
impl PullRequest {
    fn new() -> Self {
        let f = Fixture::remote();
        switch(&f, Integration::PullRequest);
        let primary_before = f.primary();
        Self { f, primary_before }
    }
    /// A direct repository with a publication remote: its context is a
    /// branch record's too, for a withdraw.
    fn direct() -> Self {
        let f = Fixture::remote();
        switch(&f, Integration::Direct);
        let primary_before = f.primary();
        Self { f, primary_before }
    }
    /// A direct repository with no publication remote.
    fn local() -> Self {
        let f = Fixture::new();
        let primary_before = f.primary();
        Self { f, primary_before }
    }
    fn request(&self) -> CloseTicketRequest {
        self.f.request()
    }
    fn run(
        &self,
        req: CloseTicketRequest,
        io: &mut impl PromotionIo,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        self.run_in(&self.f.service, req, io)
    }
    /// A close run on `clock`.
    fn run_at(
        &self,
        req: CloseTicketRequest,
        io: &mut impl PromotionIo,
        clock: fn() -> OffsetDateTime,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        run::run_with_io(
            &TicketClosureLifecycle,
            &self.f.service,
            LifecycleRequest {
                clock,
                ..lifecycle_request(req)
            },
            io,
        )
    }
    /// What a withdraw of the ticket's closure is offered as.
    fn withdrawal(&self) -> ClosureWithdrawalPreflight {
        self.f
            .service
            .preview_closure_withdrawal(PreviewTicketClosureRequest {
                root: self.f.root().into(),
                item_id: id(),
                confirmed_identity: None,
            })
            .unwrap()
    }
    /// A withdraw request for the preview it is offered (panics when it is
    /// not offered).
    fn withdraw_request(&self) -> CloseTicketRequest {
        let ClosureWithdrawalPreflight::Ready(preview) = self.withdrawal() else {
            panic!("a withdraw is not offered: {:?}", self.withdrawal());
        };
        CloseTicketRequest {
            root: self.f.root().into(),
            operation_id: OperationId::new(),
            item_id: id(),
            expected: preview.expectation(),
            approval: None,
            confirmed_identity: None,
        }
    }
    fn withdraw(
        &self,
        req: CloseTicketRequest,
        io: &mut impl PromotionIo,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        self.withdraw_with(&self.f.service, req, io, clock)
    }
    /// A withdraw run on `clock`.
    fn withdraw_at(
        &self,
        req: CloseTicketRequest,
        io: &mut impl PromotionIo,
        clock: fn() -> OffsetDateTime,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        self.withdraw_with(&self.f.service, req, io, clock)
    }
    fn withdraw_with(
        &self,
        service: &RepositoryService,
        req: CloseTicketRequest,
        io: &mut impl PromotionIo,
        clock: fn() -> OffsetDateTime,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        run::run_with_io(
            &TicketWithdrawLifecycle,
            service,
            LifecycleRequest {
                clock,
                ..lifecycle_request(req)
            },
            io,
        )
    }
    fn run_in(
        &self,
        service: &RepositoryService,
        req: CloseTicketRequest,
        io: &mut impl PromotionIo,
    ) -> Result<TicketClosureResult, TicketClosureError> {
        run::run_with_io(
            &TicketClosureLifecycle,
            service,
            LifecycleRequest {
                clock,
                ..lifecycle_request(req)
            },
            io,
        )
    }
    /// The answer of a branch record, whichever way the index step went.
    fn branch(result: Result<TicketClosureResult, TicketClosureError>) -> BranchClosureOutcome {
        match result {
            Ok(TicketClosureResult::Branch(outcome)) => outcome,
            Ok(TicketClosureResult::BranchIndexPending(pending)) => pending.authoritative,
            other => panic!("not a branch answer: {other:?}"),
        }
    }
    /// A pull-request close never reaches the primary, and runs no primary
    /// child.
    fn assert_primary_untouched(&self, io: &PullRequestIo) {
        assert_eq!(self.f.primary(), self.primary_before);
        assert_eq!(io.primary_calls, 0);
    }
    /// A commit on `parent` whose ticket is closed as a remote clone, or a
    /// forge's merge, would have it.
    fn closed_commit_on(&self, parent: Oid, closed_at: i64, closed_by: &str) -> Oid {
        let open = support::tree_bytes(&self.f.repo, parent, &ticket());
        let closed = observe::closure_replaced_bytes(&open, support::at(closed_at), closed_by)
            .expect("a ticket");
        support::tree_commit(
            &self.f.repo,
            parent,
            &[(ticket().as_str(), Some((&closed, 0o100644)))],
        )
    }
    /// A commit on `parent` that opens the ticket as the original one, as a
    /// withdraw does.
    fn open_commit_on(&self, parent: Oid) -> Oid {
        let open = support::tree_bytes(&self.f.repo, self.f.base, &ticket());
        support::tree_commit(
            &self.f.repo,
            parent,
            &[(ticket().as_str(), Some((&open, 0o100644)))],
        )
    }
    /// A commit on `parent` with its ticket's closure removed, as another
    /// clone's withdraw leaves it.
    fn withdraw_commit_on(&self, parent: Oid) -> Oid {
        let closed = support::tree_bytes(&self.f.repo, parent, &ticket());
        let open = observe::withdrawn_ticket_bytes(&closed).expect("a closed ticket");
        support::tree_commit(
            &self.f.repo,
            parent,
            &[(ticket().as_str(), Some((&open, 0o100644)))],
        )
    }
    /// A close request run to `Published` with its own io: the closure
    /// commit `C`, the context's tip, published.
    fn close_first(&self) -> Oid {
        self.close_first_request().1
    }
    /// `close_first`, with the request that closed.
    fn close_first_request(&self) -> (CloseTicketRequest, Oid) {
        let mut io = PullRequestIo::default();
        let req = self.request();
        let outcome = PullRequest::branch(self.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::Published);
        self.assert_primary_untouched(&io);
        let closure = self.f.context_oid();
        assert_eq!(outcome.checkpoint_oid(), Some(closure));
        (req, closure)
    }
    /// A ticket on the primary that depends on the fixture's, indexed, so
    /// that a listing reads what the closure does to its dependents.
    fn add_dependent(&mut self) {
        let source = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {DEPENDENT}\ntitle: Dependent\n\
             type: task\nstatus: open\ndeps:\n- {ID}\n---\nDependent body.\n"
        );
        write(
            &self.f.repo,
            &format!(".manyhands/tickets/{DEPENDENT}/ticket.md"),
            source,
        );
        let oid = commit(&self.f.repo);
        self.f.tracking("main", oid);
        self.primary_before = self.f.primary();
        self.refresh();
    }
    /// The index, refreshed from what the contexts hold now.
    fn refresh(&self) {
        use crate::repository::{RefreshOutcome, RefreshRepositoryRequest};
        let refreshed = self.f.service.refresh_repository(RefreshRepositoryRequest {
            root: self.f.root().into(),
            operation_id: OperationId::new(),
        });
        assert!(
            matches!(refreshed, Ok(RefreshOutcome::Refreshed { .. })),
            "{refreshed:?}"
        );
    }
    /// Why the dependent ticket is blocked, as the stored index lists it.
    fn dependent_reasons(&self) -> Vec<crate::repository::ReadinessReasonCode> {
        let repository = self.f.service.resolve_repository(self.f.root()).unwrap();
        self.f
            .service
            .list_tickets(&repository, &TicketFilter::default())
            .unwrap()
            .items
            .iter()
            .find(|item| item.id.as_deref() == Some(DEPENDENT))
            .and_then(|item| item.readiness.as_ref())
            .map(|readiness| readiness.reasons.iter().map(|reason| reason.code).collect())
            .expect("the dependent is listed with its readiness")
    }
    /// When the item was last given its link, as the link record keeps it.
    fn link_given_at(&self) -> Option<i64> {
        self.f
            .db()
            .query_row(
                "SELECT given_at FROM pull_request_links WHERE kind='ticket' AND item_ulid=?1",
                [ID],
                |row| row.get(0),
            )
            .ok()
    }
    /// A commit on `parent` with the ticket's body written as `body`.
    fn body_commit_on(&self, parent: Oid, body: &str) -> Oid {
        let source = String::from_utf8(support::tree_bytes(&self.f.repo, parent, &ticket()))
            .unwrap()
            .replace("original body", body)
            .replace("context body", body);
        support::tree_commit(
            &self.f.repo,
            parent,
            &[(ticket().as_str(), Some((source.as_bytes(), 0o100644)))],
        )
    }
    /// The context's worktree holds `oid`: its closure is there before any
    /// request.
    fn hold(&self, oid: Oid) {
        checkout(&self.f.context, oid);
    }
    fn stored(&self, req: &CloseTicketRequest) -> ClosureRecord {
        self.f.stored(req.operation_id)
    }
    fn link(&self) -> Option<crate::forge::PullRequestLink> {
        let db = self.f.db();
        let repository: i64 = db
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap();
        link_of(&db, repository, "ticket", ID).unwrap()
    }
    fn assert_link(&self) {
        self.assert_link_into("main");
    }
    fn assert_link_into(&self, target: &str) {
        let link = self.link().expect("the item has its link");
        assert_eq!(link.source, format!("manyhands/ticket/{ID}"));
        assert_eq!(link.target, target);
        assert_eq!(
            linked_items_for_test(&self.f.db()),
            [("ticket".to_owned(), ID.to_owned())]
        );
    }
    /// How the ticket reads in a listing: the context holds the closure, and
    /// the primary does not.
    fn listed_state(&self) -> Option<ClosureState> {
        let repository = self.f.service.resolve_repository(self.f.root()).unwrap();
        self.f
            .service
            .list_tickets(&repository, &TicketFilter::default())
            .unwrap()
            .items
            .iter()
            .find(|item| item.id.as_deref() == Some(ID))
            .and_then(|item| item.closure.as_ref().map(|closure| closure.state))
    }
    /// A child's row phase, when it has a row.
    fn phase(&self, child: OperationId) -> Option<String> {
        self.f
            .db()
            .query_row(
                "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                [child.to_string()],
                |row| row.get(0),
            )
            .ok()
    }
    /// No child of the record is left holding the context: each row it has is
    /// ended.
    fn assert_no_active_child(&self, r: &ClosureRecord, case: &str) {
        for child in [r.context_sync_id, r.primary_sync_id].into_iter().flatten() {
            let phase = self.phase(child);
            assert!(
                phase.as_ref().is_none_or(|phase| matches!(
                    phase.as_str(),
                    "completed" | "interrupted" | "cancelled" | "failed"
                )),
                "{case}: {child} is {phase:?}"
            );
        }
    }
    fn held(&self) -> bool {
        let db = self.f.db();
        let repository: i64 = db
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap();
        record::written_closure_unfinished(&db, repository, &id()).unwrap()
    }
    /// The unfinished record of the ticket that a new request cannot
    /// supersede, as the binding's `held` reads it.
    fn holder(&self) -> Option<OperationId> {
        let db = self.f.db();
        let repository: i64 = db
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap();
        crate::repository::promotion::unfinished_holder::<ClosureRecord>(&db, repository, &id())
            .unwrap()
    }
}

/// The children and the exact delete of a pull-request close, scripted by
/// role. It never touches the primary: a primary child is a failure.
#[derive(Default)]
struct PullRequestIo {
    inner: FakeIo,
    /// `refs/remotes/origin/main` as each role's fetch leaves it, written
    /// first, before any stop.
    fetched: Vec<(ChildRole, Oid)>,
    /// Scripted stops by role, first in first out; `kept` reserves the
    /// child and leaves its row active.
    stops: VecDeque<(ChildRole, SynchronizationError, bool)>,
    /// One real local update, taken once: the role and the incoming commit.
    drive: Option<(ChildRole, Oid)>,
    /// A kept stop also records what a child that stopped after its local
    /// update has: something to lose, so that letting it go takes the
    /// cancel-and-acknowledge path, not the release alone.
    prepared: bool,
    primary_calls: usize,
    /// Puts a driven update to R-30's push decision, as production's
    /// `push_unless_unclosed` puts the head it publishes: a head that lacks
    /// the closure the fetched primary holds is `TicketClosed`, and the row
    /// is left as the drive left it.
    decide_push: bool,
}
impl PullRequestIo {
    fn children(&self) -> usize {
        self.inner.calls.borrow().children.len()
    }
    /// The tracking context ref, as a publication leaves it.
    fn tracking_context(&self, root: &Path, oid: Oid) {
        git2::Repository::open(root)
            .unwrap()
            .reference(&tracking_context_ref(), oid, true, "fixture publication")
            .unwrap();
    }
}
impl PromotionIo for PullRequestIo {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        req: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        self.inner.calls.borrow_mut().children.push((
            req.target.clone(),
            req.operation_id,
            req.restart,
        ));
        if req.target == SynchronizationTarget::Primary {
            self.primary_calls += 1;
            return Err(SynchronizationError::RecoveryRequired);
        }
        let (root, role) = (req.root.clone(), execution.role());
        // The acknowledgement of a cancelled row ends it, as a real child
        // does at its first safe point.
        let cancelling = state::with_transaction(service, &root, |tx, id| {
            Ok(state::read_operation(tx, id, req.operation_id)?
                .is_some_and(|row| row.cancel_requested))
        })
        .unwrap();
        if req.restart && cancelling {
            state::with_transaction(service, &root, |tx, _| {
                tx.execute(
                    "UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled',cancel_requested=0 WHERE operation_ulid=?1",
                    [req.operation_id.to_string()],
                )
                .map_err(|_| state::recovery_required())?;
                Ok(())
            })
            .unwrap();
            return Err(SynchronizationError::Interrupted);
        }
        // The fetch, which a stop after it leaves.
        if let Some(at) = self.fetched.iter().position(|(r, _)| *r == role) {
            let (_, oid) = self.fetched.remove(at);
            git2::Repository::open(&root)
                .unwrap()
                .reference("refs/remotes/origin/main", oid, true, "fixture fetch")
                .unwrap();
        }
        if self.stops.front().is_some_and(|(r, ..)| *r == role) {
            let (_, error, kept) = self.stops.pop_front().unwrap();
            if kept {
                let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
                let new = state::with_transaction(service, &root, |tx, id| {
                    Ok(state::read_operation(tx, id, req.operation_id)?.is_none())
                })
                .unwrap();
                let target = req.target.operation_target(&plan);
                assert!(matches!(
                    service
                        .reserve_promotion_child(&root, req.operation_id, &target, execution, new)
                        .unwrap(),
                    RemoteReservationOutcome::Reserved(_)
                ));
                if self.prepared {
                    let oid = git2::Repository::open(&root)
                        .unwrap()
                        .refname_to_id("refs/heads/main")
                        .unwrap();
                    state::with_transaction(service, &root, |tx, _| {
                        tx.execute(
                            "UPDATE remote_operation_records SET phase='local_prepared',sync_checkpoint='local_prepared',expected_oid=?2,local_oid=?2,primary_tracking_oid=?2 WHERE operation_ulid=?1",
                            [req.operation_id.to_string(), oid.to_string()],
                        )
                        .map_err(|_| state::recovery_required())?;
                        Ok(())
                    })
                    .unwrap();
                }
            }
            return Err(error);
        }
        if self.drive.as_ref().is_some_and(|(r, _)| *r == role) {
            let (_, incoming) = self.drive.take().unwrap();
            let merged =
                sync::promotion_test_drive_local_bound(service, &req, incoming, execution)?;
            if self.decide_push {
                let repo = git2::Repository::open(&root).unwrap();
                let primary = repo.refname_to_id("refs/remotes/origin/main").unwrap();
                if crate::repository::integration::prepare::lacks_primary_closure(
                    &repo,
                    merged,
                    primary,
                    &observe::ticket_path(&id()),
                )
                .unwrap()
                {
                    return Err(SynchronizationError::TicketClosed);
                }
            }
            complete_child_row(
                service,
                &root,
                execution,
                req.operation_id,
                merged,
                "published",
            );
            self.tracking_context(&root, merged);
            return Ok(SynchronizationResult::Complete(
                SynchronizationOutcome::Published {
                    target: req.target,
                    oid: merged,
                },
            ));
        }
        // Not a second record of the same call.
        self.inner.calls.borrow_mut().children.pop();
        let result = self.inner.synchronize(service, req, observer, execution)?;
        if let SynchronizationResult::Complete(
            SynchronizationOutcome::Published { oid, .. }
            | SynchronizationOutcome::AlreadyCurrent { oid, .. },
        ) = &result
        {
            self.tracking_context(&root, *oid);
        }
        Ok(result)
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

fn transport(kind: crate::repository::transport::SshTransportErrorKind) -> SynchronizationError {
    SynchronizationError::Transport(crate::repository::transport::SshTransportError {
        root: "/private/path/sentinel".into(),
        remote_name: "private-endpoint-sentinel".into(),
        direction: crate::repository::transport::SshDirection::Push,
        selected_key_id: None,
        authority: None,
        kind,
    })
}

/// D1: a `record` close synchronizes its context, writes one closure commit
/// on it, publishes it, and answers `published`, never touching the primary.
#[test]
fn d1_records_publishes_and_completes_without_touching_the_primary() {
    let p = PullRequest::new();
    let req = p.request();
    let context = p.f.context_oid();
    let mut io = PullRequestIo::default();
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    let r = p.stored(&req);
    // One commit `Close ticket <ULID>` over the synchronized head, as the
    // clock and the identity gave it.
    assert_eq!(p.f.commits_in_odb(&close_subject()), 1);
    let closure = p.f.context_oid();
    let commit = p.f.repo.find_commit(closure).unwrap();
    assert_eq!(commit.summary(), Some(close_subject().as_str()));
    assert_eq!(commit.parent_id(0).unwrap(), context);
    assert_eq!(commit.time().seconds(), CLOCK);
    let ticket = parse_ticket(&support::tree_bytes(&p.f.repo, closure, &ticket()));
    assert_eq!(ticket.closed_by.as_deref(), Some(CLOSED_BY));
    assert_eq!(ticket.closed_at, Some(clock()));
    // The record, the answer and the tracking context.
    assert_eq!(r.step, ClosureStep::Complete);
    assert_eq!(r.answer_code, Some(BranchAnswer::Published));
    assert_eq!(r.closure_oid, Some(closure));
    assert_eq!(r.closed_at, Some(clock()));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    assert_eq!(outcome.action(), ClosureAction::Close);
    assert_eq!(outcome.checkpoint_oid(), Some(closure));
    assert_eq!(
        outcome.publication(),
        Some(PromotionPublication::Published { oid: closure })
    );
    assert_eq!(
        p.f.repo.refname_to_id(&tracking_context_ref()).unwrap(),
        closure
    );
    p.assert_link();
    assert!(!p.held());
    p.assert_primary_untouched(&io);
    // The same request again gives the same answer, and runs nothing.
    let children = io.children();
    let again = PullRequest::branch(p.run(req, &mut io));
    assert_eq!(again, outcome);
    assert_eq!(io.children(), children);
    assert_eq!(p.f.commits_in_odb(&close_subject()), 1);
}

/// D2: a closure the context already holds is published with the pending
/// work, answers `already_closing`, and writes no commit.
#[test]
fn d2_an_existing_closure_publishes_pending_work_and_is_already_closing() {
    let p = PullRequest::new();
    let held = p.closed_commit_on(
        p.f.context_oid(),
        1_700_000_000,
        "Earlier <earlier@example.invalid>",
    );
    p.hold(held);
    let req = p.request();
    assert!(matches!(
        p.f.preview().closure(),
        PlannedClosure::Existing { .. }
    ));
    let mut io = PullRequestIo::default();
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Existing);
    assert_eq!(outcome.checkpoint_oid(), None);
    assert_eq!(
        outcome.publication(),
        Some(PromotionPublication::Published { oid: held })
    );
    assert_eq!(p.f.commits_in_odb(&close_subject()), 0);
    assert_eq!(p.f.context_oid(), held);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Complete);
    assert_eq!(r.closure_oid, None);
    assert_eq!(r.answer_code, Some(BranchAnswer::Existing));
    p.assert_link();
    p.assert_primary_untouched(&io);
}

/// D3: the context child brings in the closure a remote clone made: the
/// record ends as `already_closing` with nothing written.
#[test]
fn d3_a_closure_the_context_child_brings_in_abandons_as_already_closing() {
    let p = PullRequest::new();
    let req = p.request();
    let incoming = p.closed_commit_on(
        p.f.context_oid(),
        1_700_000_000,
        "Remote <remote@example.invalid>",
    );
    let mut io = PullRequestIo {
        drive: Some((ChildRole::Context, incoming)),
        ..PullRequestIo::default()
    };
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::AlreadyClosing);
    assert_eq!(outcome.checkpoint_oid(), None);
    assert_eq!(outcome.publication(), None);
    assert_eq!(p.f.commits_in_odb(&close_subject()), 0);
    // The context is as the child left it: it holds the closure, whole.
    let held = parse_ticket(&support::tree_bytes(
        &p.f.repo,
        p.f.context_oid(),
        &ticket(),
    ));
    assert_eq!(
        held.closed_by.as_deref(),
        Some("Remote <remote@example.invalid>")
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(r.refusal, Some(record::Refusal::ExternalChange));
    assert_eq!(r.answer_code, Some(BranchAnswer::AlreadyClosing));
    assert_eq!(r.closure_oid, None);
    p.assert_link();
    assert!(!p.held());
    p.assert_primary_untouched(&io);
    // The same request again answers the same, from its record.
    let children = io.children();
    assert_eq!(PullRequest::branch(p.run(req, &mut io)), outcome);
    assert_eq!(io.children(), children);
}

/// D4: the tracking primary holds a closure after the context child: the
/// record ends as `already_closed`, with nothing written.
#[test]
fn d4_a_closure_the_tracking_primary_holds_after_the_context_child_abandons_as_ticket_closed() {
    let p = PullRequest::new();
    let req = p.request();
    let closed = p.closed_commit_on(
        p.f.primary(),
        1_700_000_000,
        "Remote <remote@example.invalid>",
    );
    let mut io = PullRequestIo {
        fetched: vec![(ChildRole::Context, closed)],
        ..PullRequestIo::default()
    };
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::AlreadyClosed);
    assert_eq!(outcome.checkpoint_oid(), None);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(
        r.refusal,
        Some(record::Refusal::Blocked(vec![
            ClosureBlocker::AlreadyClosed
        ]))
    );
    assert_eq!(r.answer_code, Some(BranchAnswer::AlreadyClosed));
    assert_eq!(r.closure_oid, None);
    assert_eq!(p.f.commits_in_odb(&close_subject()), 0);
    assert_eq!(io.children(), 1);
    p.assert_primary_untouched(&io);
}

/// D5: a context child that stops leaves the record `confirmed`, and its
/// child as it stands, for the same request. A synchronization of the
/// context that no record owns holds step 3 off, named, with nothing written.
#[test]
fn d5_a_stopped_context_child_keeps_the_record_confirmed_and_the_same_request_continues() {
    use crate::repository::transport::SshTransportErrorKind as K;
    type Stop = fn() -> SynchronizationError;
    let table: [(&str, Stop, bool); 7] = [
        ("transport", || transport(K::TransportUnavailable), false),
        ("key", || transport(K::NoSelectedKey), false),
        ("host", || transport(K::HostTrustChanged), false),
        ("rejected", || SynchronizationError::PushRejected, false),
        ("busy", || SynchronizationError::Busy, false),
        ("yielding", || SynchronizationError::PollYielding, false),
        ("kept", || transport(K::TransportUnavailable), true),
    ];
    for (case, error, kept) in table {
        let p = PullRequest::new();
        let req = p.request();
        let mut io = PullRequestIo {
            stops: VecDeque::from([(ChildRole::Context, error(), kept)]),
            ..PullRequestIo::default()
        };
        let result = p.run(req.clone(), &mut io);
        assert!(
            matches!(result, Err(TicketClosureError::ContextSynchronization(_))),
            "{case}: {result:?}"
        );
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Confirmed, "{case}");
        let child = r.context_sync_id.expect("the child was minted");
        let phase = |child: OperationId| -> Option<String> {
            p.f.db()
                .query_row(
                    "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
                    [child.to_string()],
                    |row| row.get(0),
                )
                .ok()
        };
        // A kept child is not let go: its row still holds the reservation.
        assert_eq!(phase(child).is_some(), kept, "{case}");
        // A new request's confirm: its own child is excluded from the
        // observation, so the record's hold answers.
        let other = p.request();
        let mut other_io = PullRequestIo::default();
        let refused = p.run(other, &mut other_io);
        assert!(
            matches!(refused, Err(TicketClosureError::Busy)),
            "{case}: {refused:?}"
        );
        assert_eq!(other_io.children(), 0, "{case}");
        // The same request goes on.
        let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::Published, "{case}");
        p.assert_primary_untouched(&io);
    }
    // A synchronization of the context that is nobody's record's holds step
    // 3 off: `busy`, with the row named, and nothing written.
    let p = PullRequest::new();
    let req = p.request();
    let plan = p.f.configure_endpoints();
    let target = p.f.manual_context_sync(&plan);
    let foreign = OperationId::new();
    let root = p.f.root().to_owned();
    p.f.hook(move |service, point| {
        if point == FailurePoint::AfterContextSync {
            assert!(matches!(
                service
                    .reserve_remote_operation(&root, foreign, &target)
                    .unwrap(),
                RemoteReservationOutcome::Reserved(_)
            ));
        }
    });
    let mut io = PullRequestIo::default();
    match p.run(req.clone(), &mut io) {
        Err(TicketClosureError::Repository(e)) => {
            assert_eq!(e.kind, RepositoryErrorKind::RepositoryBusy);
            assert_eq!(e.blocker().map(|b| b.operation_id), Some(foreign));
        }
        other => panic!("expected a repository busy: {other:?}"),
    }
    HOOKS.with(|h| h.borrow_mut().remove(p.f.root()));
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::ContextSynchronized);
    assert_eq!(r.closure_oid, None);
    assert_eq!(p.f.commits_in_odb(&close_subject()), 0);
    p.f.db()
        .execute(
            "UPDATE remote_operation_records SET phase='cancelled',outcome='cancelled' WHERE operation_ulid=?1",
            [foreign.to_string()],
        )
        .unwrap();
    let outcome = PullRequest::branch(p.run(req, &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    p.assert_primary_untouched(&io);
}

/// The context child's conflict, retained: `incoming` changes the body line
/// the context changed.
fn conflicted_context(
    p: &PullRequest,
    req: &CloseTicketRequest,
    io: &mut PullRequestIo,
) -> (OperationId, Vec<u8>) {
    let parent =
        p.f.repo
            .find_commit(p.f.context_oid())
            .unwrap()
            .parent_id(0)
            .unwrap();
    let local = read(&p.f.context, &ticket());
    io.drive = Some((ChildRole::Context, p.body_commit_on(parent, "remote body")));
    let error = p.run(req.clone(), io).unwrap_err();
    assert!(
        matches!(&error, TicketClosureError::ContextSynchronization(e) if matches!(**e, SynchronizationError::ConflictPending { .. })),
        "{error:?}"
    );
    (p.stored(req).context_sync_id.expect("the child"), local)
}

/// Resolves a retained conflict of `child` with `local` bytes, through the
/// conflict verbs' library call.
fn resolve(p: &PullRequest, child: OperationId, local: Vec<u8>) {
    use crate::repository::{
        RedactedConflictBytes, ResolveSynchronizationOutcome, ResolveSynchronizationRequest,
    };
    let inspection =
        p.f.service
            .inspect_synchronization_recovery(p.f.root(), child)
            .unwrap();
    assert_eq!(inspection.paths.len(), 1);
    p.f.service
        .read_synchronization_conflict(&inspection.paths[0].token)
        .unwrap();
    let resolution = ResolveSynchronizationRequest::new(
        p.f.root().into(),
        child,
        OperationId::new(),
        inspection.observation.clone(),
        vec![(
            inspection.paths[0].token.clone(),
            RedactedConflictBytes::from_bytes(local),
        )],
        None,
    );
    assert!(matches!(
        p.f.service.resolve_synchronization(resolution).unwrap(),
        ResolveSynchronizationOutcome::LocalCheckpointComplete { .. }
    ));
}

/// D7: a conflict of the context child is retained, and the same request
/// goes on once it is resolved.
#[test]
fn d7_a_context_conflict_is_retained_and_the_same_request_continues() {
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo::default();
    let (child, local) = conflicted_context(&p, &req, &mut io);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Confirmed);
    assert_eq!(r.closure_oid, None);
    assert_eq!(p.f.commits_in_odb(&close_subject()), 0);
    resolve(&p, child, local);
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    assert_eq!(p.stored(&req).context_sync_id, Some(child));
    p.assert_primary_untouched(&io);
}

/// D8: a publication child that stops leaves the record at
/// `publication_pending` with its category, holding its context, for the
/// same request.
#[test]
fn d8_a_stopped_publication_child_is_held_and_the_same_request_continues() {
    use crate::repository::promotion::record::PublicationCategory as P;
    use crate::repository::transport::SshTransportErrorKind as K;
    type Stop = fn() -> SynchronizationError;
    let table: [(&str, Stop, bool, P); 6] = [
        (
            "transport",
            || transport(K::TransportUnavailable),
            false,
            P::Transport,
        ),
        ("key", || transport(K::NoSelectedKey), false, P::Transport),
        (
            "host",
            || transport(K::HostTrustChanged),
            false,
            P::Transport,
        ),
        ("busy", || SynchronizationError::Busy, false, P::Busy),
        (
            "yielding",
            || SynchronizationError::PollYielding,
            false,
            P::Yielding,
        ),
        (
            "kept",
            || transport(K::TransportUnavailable),
            true,
            P::Transport,
        ),
    ];
    for (case, error, kept, category) in table {
        let p = PullRequest::new();
        let req = p.request();
        let mut io = PullRequestIo {
            stops: VecDeque::from([(ChildRole::Publication, error(), kept)]),
            ..PullRequestIo::default()
        };
        let result = p.run(req.clone(), &mut io);
        assert!(
            matches!(result, Err(TicketClosureError::ContextSynchronization(_))),
            "{case}: {result:?}"
        );
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::PublicationPending, "{case}");
        assert_eq!(r.publication, Some(category), "{case}");
        assert_eq!(r.answer_code, None, "{case}");
        assert!(r.closure_oid.is_some(), "{case}");
        assert!(p.held(), "{case}");
        assert_eq!(p.listed_state(), Some(ClosureState::Closing), "{case}");
        assert_eq!(p.f.commits_in_odb(&close_subject()), 1, "{case}");
        let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::Published, "{case}");
        assert_eq!(p.f.commits_in_odb(&close_subject()), 1, "{case}");
        assert!(!p.held(), "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// D9: a publication the remote rejected is replaced by a new attempt, which
/// merges the closure another clone published first and keeps it.
#[test]
fn d9_a_rejected_publication_is_replaced_and_keeps_the_published_closure() {
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo {
        stops: VecDeque::from([(
            ChildRole::Publication,
            SynchronizationError::PushRejected,
            true,
        )]),
        ..PullRequestIo::default()
    };
    let result = p.run(req.clone(), &mut io);
    assert!(matches!(
        result,
        Err(TicketClosureError::ContextSynchronization(_))
    ));
    assert_eq!(p.stored(&req).step, ClosureStep::PublicationPending);
    // Another clone closed the ticket first, and published it.
    let ours = p.f.context_oid();
    let incoming = p.closed_commit_on(
        p.f.repo.find_commit(ours).unwrap().parent_id(0).unwrap(),
        1_700_000_000,
        "Remote <remote@example.invalid>",
    );
    io.drive = Some((ChildRole::Publication, incoming));
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::KeptPublished);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Complete);
    assert_eq!(r.answer_code, Some(BranchAnswer::KeptPublished));
    p.assert_link();
    p.assert_primary_untouched(&io);
}

/// The record's closure is written and its first publication stops; the
/// next one's merge conflicts on the body line the context changed, and is
/// retained.
fn conflicted_publication(
    p: &PullRequest,
    req: &CloseTicketRequest,
    io: &mut PullRequestIo,
) -> (OperationId, Vec<u8>) {
    // The record is written, and its child is stopped, before the context
    // moves under it.
    io.stops.push_back((
        ChildRole::Publication,
        SynchronizationError::PushRejected,
        false,
    ));
    let _ = p.run(req.clone(), io);
    let ours = p.f.context_oid();
    let local = read(&p.f.context, &ticket());
    let parent = p.f.repo.find_commit(ours).unwrap().parent_id(0).unwrap();
    // The context's own ancestor, with the line the context changed changed
    // another way: the closure the record wrote does not take it away.
    let parent = p.f.repo.find_commit(parent).unwrap().parent_id(0).unwrap();
    let incoming = p.body_commit_on(parent, "remote body");
    io.drive = Some((ChildRole::Publication, incoming));
    let error = p.run(req.clone(), io).unwrap_err();
    assert!(
        matches!(&error, TicketClosureError::ContextSynchronization(e) if matches!(**e, SynchronizationError::ConflictPending { .. })),
        "{error:?}"
    );
    (p.stored(req).primary_sync_id.expect("the child"), local)
}

/// D10: a conflict of the publication child is retained, and the same
/// request goes on once it is resolved.
#[test]
fn d10_a_publication_conflict_is_retained_and_the_same_request_continues() {
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo::default();
    let (child, local) = conflicted_publication(&p, &req, &mut io);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::PublicationPending);
    assert_eq!(
        r.publication,
        Some(crate::repository::promotion::record::PublicationCategory::Conflict)
    );
    assert!(p.held());
    resolve(&p, child, local);
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    assert_eq!(p.stored(&req).primary_sync_id, Some(child));
    assert!(!p.held());
    p.assert_primary_untouched(&io);
}

/// D12: a process lost before the context ref moved to the closure rebuilds
/// the same commit, from the `closed_at` the record stored.
#[test]
fn d12_a_crash_before_the_ref_moved_rebuilds_the_same_commit() {
    for point in [2, 3] {
        let p = PullRequest::new();
        let req = p.request();
        exit_at(&p.f, &req, point);
        let before = p.stored(&req);
        assert_eq!(before.step, ClosureStep::Closing, "{point}");
        let (closed_at, commit) = (before.closed_at, before.closure_oid);
        assert!(closed_at.is_some() && commit.is_some());
        let mut io = PullRequestIo::default();
        let outcome = PullRequest::branch(p.run_in(&p.f.reopened(), req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::Published, "{point}");
        let r = p.stored(&req);
        assert_eq!((r.closed_at, r.closure_oid), (closed_at, commit), "{point}");
        assert_eq!(p.f.commits_in_odb(&close_subject()), 1, "{point}");
        assert_eq!(p.f.context_oid(), commit.unwrap(), "{point}");
        p.assert_primary_untouched(&io);
    }
}

/// A publication child that updates the context for real, and is lost
/// before its row is completed: reserved after the local update and before
/// any push, as a lost process leaves it.
struct DriveThenStop {
    inner: PullRequestIo,
    incoming: Oid,
    done: bool,
}
impl PromotionIo for DriveThenStop {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        req: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        if execution.role() == ChildRole::Publication && !self.done {
            self.done = true;
            sync::promotion_test_drive_local_bound(service, &req, self.incoming, execution)?;
            return Err(SynchronizationError::Interrupted);
        }
        self.inner.synchronize(service, req, observer, execution)
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

/// D13: a process lost after the context ref moved never runs step 3 again:
/// the record is past it, and its child replays or restarts.
#[test]
fn d13_a_crash_after_the_ref_moved_never_runs_the_hook_again() {
    use run::RunEvent;
    for point in [4, 7, 12] {
        let p = PullRequest::new();
        let req = p.request();
        exit_at(&p.f, &req, point);
        let mut io = PullRequestIo::default();
        let mut told = Told(Vec::new());
        let outcome = PullRequest::branch(run::run_with_io_observed(
            &TicketClosureLifecycle,
            &p.f.reopened(),
            LifecycleRequest {
                clock,
                ..lifecycle_request(req.clone())
            },
            &mut io,
            &mut told,
        ));
        assert_eq!(outcome.answer(), BranchAnswer::Published, "{point}");
        assert_eq!(p.f.commits_in_odb(&close_subject()), 1, "{point}");
        // Once the publication child is minted, the record is past step 3.
        assert_eq!(
            told.0.contains(&RunEvent::Recording),
            point == 4,
            "{point}: {:?}",
            told.0
        );
        p.assert_primary_untouched(&io);
    }
    // Lost in the child, after its local update: its row is reserved, and the
    // same request restarts it.
    let p = PullRequest::new();
    let req = p.request();
    let ours = p.f.context_oid();
    let incoming = p.closed_commit_on(
        p.f.repo.find_commit(ours).unwrap().parent_id(0).unwrap(),
        1_700_000_000,
        "Remote <remote@example.invalid>",
    );
    let mut io = DriveThenStop {
        inner: PullRequestIo::default(),
        incoming,
        done: false,
    };
    assert!(p.run(req.clone(), &mut io).is_err());
    assert_eq!(p.stored(&req).step, ClosureStep::PublicationPending);
    let commits = p.f.commits_in_odb(&close_subject());
    let mut told = Told(Vec::new());
    let outcome = PullRequest::branch(run::run_with_io_observed(
        &TicketClosureLifecycle,
        &p.f.service,
        LifecycleRequest {
            clock,
            ..lifecycle_request(req.clone())
        },
        &mut io,
        &mut told,
    ));
    assert_eq!(outcome.answer(), BranchAnswer::KeptPublished);
    assert!(!told.0.contains(&run::RunEvent::Recording));
    assert_eq!(p.f.commits_in_odb(&close_subject()), commits);
    p.assert_primary_untouched(&io.inner);
}

/// D14: while a record is unfinished, a new request is held: the prepare
/// answers `recovery_required` naming the holder, not `busy`, and a new
/// request's confirm is `Busy` and writes nothing.
#[test]
fn d14_a_new_request_while_unfinished_is_held_and_busy() {
    use crate::repository::transport::SshTransportErrorKind as K;
    use crate::repository::{Mutation, TicketCloseInput};
    for (case, role, step) in [
        (
            "publication",
            ChildRole::Publication,
            ClosureStep::PublicationPending,
        ),
        ("context", ChildRole::Context, ClosureStep::Confirmed),
    ] {
        let p = PullRequest::new();
        let req = p.request();
        let mut io = PullRequestIo {
            stops: VecDeque::from([(role, transport(K::TransportUnavailable), true)]),
            ..PullRequestIo::default()
        };
        assert!(
            matches!(
                p.run(req.clone(), &mut io),
                Err(TicketClosureError::ContextSynchronization(_))
            ),
            "{case}"
        );
        assert_eq!(p.stored(&req).step, step, "{case}");
        assert_eq!(p.holder(), Some(req.operation_id), "{case}");
        let prepared =
            p.f.service
                .prepare(&Mutation::TicketClose(TicketCloseInput {
                    root: p.f.root().to_owned(),
                    id: id(),
                }));
        let value = serde_json::to_value(&prepared).unwrap();
        assert_eq!(value["code"], "recovery_required", "{case}: {value}");
        assert!(
            value["recovery"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["operation_id"] == req.operation_id.to_string()),
            "{case}: {value}"
        );
        let other = p.request();
        let mut other_io = PullRequestIo::default();
        assert!(
            matches!(p.run(other, &mut other_io), Err(TicketClosureError::Busy)),
            "{case}"
        );
        assert_eq!(other_io.children(), 0, "{case}");
        p.assert_primary_untouched(&io);
        p.assert_primary_untouched(&other_io);
        let records: i64 =
            p.f.db()
                .query_row("SELECT count(*) FROM ticket_closures", [], |r| r.get(0))
                .unwrap();
        assert_eq!(records, 1, "{case}");
    }
}

/// A cancellation asked at the publication child, which returns before it
/// reserves anything.
struct CancelAtPublication {
    inner: PullRequestIo,
    /// The parent to cancel, once.
    parent: Option<OperationId>,
}
impl PromotionIo for CancelAtPublication {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        if execution.role() == ChildRole::Publication
            && let Some(parent) = self.parent.take()
        {
            assert!(
                service
                    .cancel_ticket_closure(&request.root, parent)
                    .unwrap()
            );
            return Err(SynchronizationError::Interrupted);
        }
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
        self.inner
            .delete(service, request, plan, owner, configuration, expected, item)
    }
}

/// D15: a cancelled publication child is let go, and the same request
/// replaces it.
#[test]
fn d15_a_cancelled_publication_child_is_replaced_by_the_same_request() {
    let p = PullRequest::new();
    let req = p.request();
    let mut io = CancelAtPublication {
        inner: PullRequestIo::default(),
        parent: Some(req.operation_id),
    };
    let result = p.run(req.clone(), &mut io);
    assert!(
        matches!(result, Err(TicketClosureError::Interrupted)),
        "{result:?}"
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::PublicationPending);
    assert_eq!(
        r.publication,
        Some(crate::repository::promotion::record::PublicationCategory::Cancelled)
    );
    let first = r.primary_sync_id.unwrap();
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    assert_ne!(p.stored(&req).primary_sync_id, Some(first));
    p.assert_primary_untouched(&io.inner);
}

/// D21: a closure the context's tip took on since the consent is not the
/// one consented to: the record ends with `external_change`, and writes
/// nothing, whatever the change was.
#[test]
fn d21_a_changed_existing_closure_before_step_three_abandons_as_external_change() {
    for (case, other) in [("another closure", true), ("a withdraw", false)] {
        let p = PullRequest::new();
        let held = p.closed_commit_on(
            p.f.context_oid(),
            1_700_000_000,
            "Earlier <earlier@example.invalid>",
        );
        p.hold(held);
        let req = p.request();
        assert!(matches!(
            p.f.preview().closure(),
            PlannedClosure::Existing { .. }
        ));
        let incoming = if other {
            p.closed_commit_on(held, 1_750_000_000, "Another <another@example.invalid>")
        } else {
            p.open_commit_on(held)
        };
        let mut io = PullRequestIo {
            drive: Some((ChildRole::Context, incoming)),
            ..PullRequestIo::default()
        };
        let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::ExternalChange, "{case}");
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Abandoned, "{case}");
        assert_eq!(r.refusal, Some(record::Refusal::ExternalChange), "{case}");
        assert_eq!(r.closure_oid, None, "{case}");
        assert_eq!(p.f.commits_in_odb(&close_subject()), 0, "{case}");
        assert!(!p.held(), "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// E-23: a branch record's own publication child reserves through the hold
/// that holds the ticket's context against everything else, and the registry
/// audit reads the reserved child as a record's own. An attempt the record
/// replaced is refused by the child fence before the hold.
#[test]
fn a_branch_record_s_own_publication_child_passes_the_fence() {
    use crate::repository::transport::SshTransportErrorKind as K;
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo {
        stops: VecDeque::from([(
            ChildRole::Publication,
            transport(K::TransportUnavailable),
            false,
        )]),
        ..PullRequestIo::default()
    };
    assert!(p.run(req.clone(), &mut io).is_err());
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::PublicationPending);
    let child = r.primary_sync_id.unwrap();
    let stale = record::child_execution(&r, ChildRole::Publication).unwrap();
    // The record replaces its attempt, which never reserved.
    let replaced = state::with_transaction(&p.f.service, p.f.root(), |tx, id| {
        record::next_attempt(tx, id, req.operation_id, ChildRole::Publication)
    })
    .unwrap();
    let (child, stale_child) = (replaced.primary_sync_id.unwrap(), child);
    let own = record::child_execution(&replaced, ChildRole::Publication).unwrap();
    let plan = p.f.configure_endpoints();
    let target = p.f.manual_context_sync(&plan);
    let mismatch =
        p.f.service
            .reserve_promotion_child(p.f.root(), stale_child, &target, &stale, true)
            .unwrap_err();
    assert_eq!(mismatch.kind, RepositoryErrorKind::OperationMismatch);
    assert!(matches!(
        p.f.service
            .reserve_promotion_child(p.f.root(), child, &target, &own, true)
            .unwrap(),
        RemoteReservationOutcome::Reserved(_)
    ));
    // A manual synchronization of the same target is held off.
    assert!(matches!(
        p.f.service
            .reserve_remote_operation(p.f.root(), OperationId::new(), &target)
            .unwrap(),
        RemoteReservationOutcome::Busy
    ));
    // The reserved child is the record's own to the audit.
    state::with_transaction(&p.f.service, p.f.root(), |tx, _| record::audit(tx)).unwrap();
    p.assert_primary_untouched(&io);
}

/// D20: the operations a branch record has are listed as a direct close's
/// are: the record once, as a `close` that offers resume, and its child's row
/// on its own as the synchronization it is, with no next action of its own.
#[test]
fn d20_operations_list_a_branch_record_as_a_close_and_its_child_as_its_own_row() {
    use crate::repository::{OperationAction, OperationNextAction};
    let p = PullRequest::new();
    let req = p.request();
    // A publication that stopped with its row reserved, as a stop that keeps
    // its child leaves it.
    let mut io = PullRequestIo {
        stops: VecDeque::from([(
            ChildRole::Publication,
            transport(crate::repository::transport::SshTransportErrorKind::TransportUnavailable),
            true,
        )]),
        ..PullRequestIo::default()
    };
    // The record's request is the one the registry holds for its operation.
    p.f.db()
        .execute_batch(&format!(
            "INSERT INTO request_records
                 (request_ulid, attempt, scope_key, command, target, intent_digest, state, accepted_at)
             VALUES ('01ARZ3NDEKTSV4RRFFQ69G5FAX', 1, 'scope', 'ticket close', 'ticket', '{}', 'accepted', 1);
             INSERT INTO request_operations VALUES ('01ARZ3NDEKTSV4RRFFQ69G5FAX', 0, 'remote', '{}')",
            "0".repeat(64),
            req.operation_id
        ))
        .unwrap();
    assert!(p.run(req.clone(), &mut io).is_err());
    p.assert_primary_untouched(&io);
    let child = p.stored(&req).primary_sync_id.unwrap();
    // The child's row is attributed to that request, whose command it is.
    let attributed: (Option<String>, Option<String>) = p
        .f
        .db()
        .query_row(
            "SELECT request_ulid, command FROM remote_operation_records WHERE operation_ulid=?1",
            [child.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        attributed,
        (
            Some("01ARZ3NDEKTSV4RRFFQ69G5FAX".to_owned()),
            Some("ticket close".to_owned())
        )
    );
    let repository = p.f.service.resolve_repository(p.f.root()).unwrap();
    let list = p.f.service.list_operations(&repository).unwrap();
    let of = |operation: OperationId| {
        list.items
            .iter()
            .filter(|row| row.operation_id.as_deref() == Some(operation.to_string().as_str()))
            .collect::<Vec<_>>()
    };
    let [parent] = of(req.operation_id)[..] else {
        panic!("{list:?}");
    };
    assert_eq!(parent.action, OperationAction::Close);
    assert_eq!(parent.state, "publication_pending");
    assert_eq!(parent.next_action, Some(OperationNextAction::Resume));
    // The child holds the reservation, and is the one row of its operation.
    let [row] = of(child)[..] else {
        panic!("{list:?}");
    };
    assert_eq!(row.action, OperationAction::SynchronizeContext);
    assert_eq!(row.next_action, None);
    // Its abandon is refused: the record is its owner.
    let abandonable = state::with_transaction(&p.f.service, p.f.root(), |tx, id| {
        let stored = state::read_operation(tx, id, child)?.unwrap();
        crate::repository::remote::reservation::abandonable(tx, &stored)
    })
    .unwrap();
    assert!(!abandonable);
}

/// A context child after which the context branch is gone.
struct GoneAfterChild {
    inner: PullRequestIo,
}
impl PromotionIo for GoneAfterChild {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        req: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let root = req.root.clone();
        let result = self.inner.synchronize(service, req, observer, execution)?;
        git2::Repository::open(root)
            .unwrap()
            .find_reference(&context_ref())
            .unwrap()
            .delete()
            .unwrap();
        Ok(result)
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

/// A context branch that is gone after its child ran cannot be consented:
/// the record ends with `reconfirmation_required`, and writes nothing.
#[test]
fn a_context_branch_gone_after_its_child_abandons_as_reconfirmation_required() {
    let p = PullRequest::new();
    let req = p.request();
    let mut io = GoneAfterChild {
        inner: PullRequestIo::default(),
    };
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::ReconfirmationRequired);
    assert_eq!(outcome.checkpoint_oid(), None);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(r.refusal, Some(record::Refusal::ReconfirmationRequired));
    assert_eq!(r.answer_code, Some(BranchAnswer::ReconfirmationRequired));
    assert_eq!(r.closure_oid, None);
    assert_eq!(p.f.commits_in_odb(&close_subject()), 0);
    assert!(!p.held());
    assert_eq!(PullRequest::branch(p.run(req, &mut io)), outcome);
    p.assert_primary_untouched(&io.inner);
}

/// A publication child that answers with nothing published (no head to read
/// an answer from): `finish_branch` owns that case, and the record stays at
/// `publication_pending` with its category, for the same request.
struct PublishesNothing {
    inner: PullRequestIo,
    once: bool,
}
impl PromotionIo for PublishesNothing {
    fn synchronize(
        &mut self,
        service: &RepositoryService,
        request: SynchronizeRemoteRequest,
        observer: &mut dyn SynchronizationObserver,
        execution: &ChildExecution,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        if execution.role() == ChildRole::Publication && std::mem::take(&mut self.once) {
            return Ok(SynchronizationResult::Complete(
                SynchronizationOutcome::PublishPending {
                    target: request.target,
                    local_oid: Oid::zero(),
                    reason: sync::PublishPendingReason::NoPublicationRemote,
                },
            ));
        }
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
        self.inner
            .delete(service, request, plan, owner, configuration, expected, item)
    }
}

#[test]
fn a_publication_that_published_nothing_is_recovery_required_and_the_same_request_continues() {
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PublishesNothing {
        inner: PullRequestIo::default(),
        once: true,
    };
    let result = p.run(req.clone(), &mut io);
    assert!(
        matches!(result, Err(TicketClosureError::RecoveryRequired)),
        "{result:?}"
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::PublicationPending);
    assert_eq!(
        r.publication,
        Some(crate::repository::promotion::record::PublicationCategory::RecoveryRequired)
    );
    assert_eq!(r.answer_code, None);
    let outcome = PullRequest::branch(p.run(req, &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    p.assert_primary_untouched(&io.inner);
}

// --- Slice D: the stops that end a record, and the answer table ----------

/// A record whose first publication child stopped with `error`, its row kept
/// and active: the record is at `publication_pending` for the same request.
fn stopped_at_publication(
    p: &PullRequest,
    req: &CloseTicketRequest,
    io: &mut PullRequestIo,
    error: SynchronizationError,
) {
    io.stops.push_back((ChildRole::Publication, error, true));
    let result = p.run(req.clone(), io);
    assert!(
        matches!(result, Err(TicketClosureError::ContextSynchronization(_))),
        "{result:?}"
    );
    assert_eq!(p.stored(req).step, ClosureStep::PublicationPending);
}

/// The closure the record writes, as the forge's merge of it would hold it on
/// the primary.
fn own_closure_on_primary(p: &PullRequest) -> Oid {
    p.closed_commit_on(p.f.primary(), CLOCK, CLOSED_BY)
}

/// Another closure, on the primary.
fn another_closure_on_primary(p: &PullRequest) -> Oid {
    p.closed_commit_on(
        p.f.primary(),
        1_700_000_000,
        "Remote <remote@example.invalid>",
    )
}

/// What another clone published, over the context the record's closure was
/// written on: its own closure, or the record's closure withdrawn after it
/// was published.
fn published_by_another_clone(p: &PullRequest, withdrawn: bool) -> Oid {
    let ours = p.f.context_oid();
    if withdrawn {
        return p.open_commit_on(ours);
    }
    let parent = p.f.repo.find_commit(ours).unwrap().parent_id(0).unwrap();
    p.closed_commit_on(parent, 1_700_000_000, "Remote <remote@example.invalid>")
}

/// The repository's configuration, as the working tree holds it now.
fn configure(p: &PullRequest, remote: Option<&str>, primary: &str) {
    let remote = remote
        .map(|remote| format!("publication_remote = \"{remote}\"\n"))
        .unwrap_or_default();
    write(
        &p.f.repo,
        ".manyhands/config.toml",
        format!(
            "format_version = 2\nintegration = \"pull_request\"\nprimary_branch = \"{primary}\"\n{remote}"
        ),
    );
}

/// D6: a context child that cannot get past its stop with nothing written (a
/// deleted remote branch, an unresolved history, a closed ticket) is let go,
/// and the record is abandoned with the child's own code, nothing written.
#[test]
fn d6_a_deleted_or_ended_context_child_abandons_with_its_code() {
    type Stop = fn() -> SynchronizationError;
    let table: [(&str, Stop, BranchAnswer); 3] = [
        (
            "deleted",
            || SynchronizationError::RemoteContextDeleted,
            BranchAnswer::ContextRemoteBranchDeleted,
        ),
        (
            "unresolved",
            || SynchronizationError::HistoryUnresolved,
            BranchAnswer::ContextHistoryUnresolved,
        ),
        (
            "closed",
            || SynchronizationError::TicketClosed,
            BranchAnswer::ContextTicketClosed,
        ),
    ];
    for (case, error, answer) in table {
        let p = PullRequest::new();
        let req = p.request();
        let context = p.f.context_oid();
        // The row is kept: it holds the reservation until it is let go.
        let mut io = PullRequestIo {
            stops: VecDeque::from([(ChildRole::Context, error(), true)]),
            ..PullRequestIo::default()
        };
        let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), answer, "{case}");
        assert_eq!(outcome.checkpoint_oid(), None, "{case}");
        assert_eq!(outcome.publication(), None, "{case}");
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Abandoned, "{case}");
        assert_eq!(
            r.refusal,
            Some(record::Refusal::ReconfirmationRequired),
            "{case}"
        );
        assert_eq!(r.answer_code, Some(answer), "{case}");
        // The child's row ended: nothing to lose, so released without a
        // connection.
        let child = r.context_sync_id.expect("the child was minted");
        assert_eq!(p.phase(child).as_deref(), Some("cancelled"), "{case}");
        p.assert_no_active_child(&r, case);
        assert!(
            io.inner.calls.borrow().children.iter().all(|c| !c.2),
            "{case}"
        );
        // Nothing was written, and nothing holds the ticket.
        assert_eq!(r.closure_oid, None, "{case}");
        assert_eq!(p.f.commits_in_odb(&close_subject()), 0, "{case}");
        assert_eq!(p.f.context_oid(), context, "{case}");
        assert!(!p.held(), "{case}");
        assert_eq!(p.holder(), None, "{case}");
        // The same request again gives the same answer, and runs nothing.
        let children = io.children();
        assert_eq!(PullRequest::branch(p.run(req, &mut io)), outcome, "{case}");
        assert_eq!(io.children(), children, "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// D11: a publication child that answers `remote_branch_deleted` after the
/// closure is answered by the tracking primary as the child left it, and its
/// child is let go first: by release when it has nothing to lose, by its
/// cancellation's acknowledgement when it has something.
#[test]
fn d11_a_deleted_remote_branch_after_the_closure_answers_by_the_tracking_primary() {
    type Fetched = Option<fn(&PullRequest) -> Oid>;
    let table: [(&str, Fetched, bool, BranchAnswer); 4] = [
        (
            "this closure",
            Some(own_closure_on_primary),
            false,
            BranchAnswer::Merged,
        ),
        (
            "another closure",
            Some(another_closure_on_primary),
            false,
            BranchAnswer::ClosedElsewhere,
        ),
        ("open", None, false, BranchAnswer::RemoteBranchDeleted),
        (
            "something to lose",
            None,
            true,
            BranchAnswer::RemoteBranchDeleted,
        ),
    ];
    for (case, fetched, prepared, answer) in table {
        let p = PullRequest::new();
        let req = p.request();
        let mut io = PullRequestIo {
            fetched: fetched
                .map(|oid| vec![(ChildRole::Publication, oid(&p))])
                .unwrap_or_default(),
            stops: VecDeque::from([(
                ChildRole::Publication,
                SynchronizationError::RemoteContextDeleted,
                true,
            )]),
            prepared,
            ..PullRequestIo::default()
        };
        let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), answer, "{case}");
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Complete, "{case}");
        assert_eq!(r.answer_code, Some(answer), "{case}");
        // The child ended before the record completed, whichever way.
        let child = r.primary_sync_id.expect("the child was minted");
        assert_eq!(p.phase(child).as_deref(), Some("cancelled"), "{case}");
        p.assert_no_active_child(&r, case);
        let acknowledged = io
            .inner
            .calls
            .borrow()
            .children
            .iter()
            .any(|(_, op, restart)| *op == child && *restart);
        assert_eq!(acknowledged, prepared, "{case}");
        // Nothing was published, and the closure stays on the local branch.
        assert_eq!(outcome.publication(), None, "{case}");
        assert_eq!(outcome.checkpoint_oid(), r.closure_oid, "{case}");
        assert_eq!(p.f.context_oid(), r.closure_oid.unwrap(), "{case}");
        p.assert_link();
        assert!(!p.held(), "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// The answer table, row by row: each row is chosen by what the publication
/// child and the tracking primary say, in the table's order, and the record
/// completes with it, its link given and no child left holding the context.
/// A row that follows a stopped child lets it go first (its row is kept).
/// Rows 6 to 9 pin the answers of the child that completed.
#[test]
fn answer_table_chooses_each_row_in_order() {
    use crate::repository::transport::SshTransportErrorKind as K;
    type Prepare = fn(&PullRequest);
    type Script = fn(&PullRequest, &CloseTicketRequest, &mut PullRequestIo);
    fn nothing(_: &PullRequest) {}
    fn unscripted(_: &PullRequest, _: &CloseTicketRequest, _: &mut PullRequestIo) {}
    // Row 5's three transport kinds, each scripted as the child's stop.
    fn remote_missing(_: &PullRequest, _: &CloseTicketRequest, io: &mut PullRequestIo) {
        io.stops.push_back((
            ChildRole::Publication,
            transport(K::PublicationRemoteMissing),
            true,
        ));
    }
    fn endpoint_changed(_: &PullRequest, _: &CloseTicketRequest, io: &mut PullRequestIo) {
        io.stops
            .push_back((ChildRole::Publication, transport(K::EndpointChanged), true));
    }
    fn configuration_invalid(_: &PullRequest, _: &CloseTicketRequest, io: &mut PullRequestIo) {
        io.stops.push_back((
            ChildRole::Publication,
            transport(K::ConfigurationInvalid),
            true,
        ));
    }
    // Row 5's runner check: the record's child stopped, and then the
    // configuration moved.
    fn remote_renamed(p: &PullRequest, req: &CloseTicketRequest, io: &mut PullRequestIo) {
        stopped_at_publication(p, req, io, SynchronizationError::Busy);
        configure(p, Some("other"), "main");
    }
    // A pull-request configuration with no publication remote is not valid.
    fn remote_removed(p: &PullRequest, req: &CloseTicketRequest, io: &mut PullRequestIo) {
        stopped_at_publication(p, req, io, SynchronizationError::Busy);
        configure(p, None, "main");
    }
    fn primary_renamed(p: &PullRequest, req: &CloseTicketRequest, io: &mut PullRequestIo) {
        stopped_at_publication(p, req, io, SynchronizationError::Busy);
        configure(p, Some("origin"), "trunk");
    }
    fn configuration_gone(p: &PullRequest, req: &CloseTicketRequest, io: &mut PullRequestIo) {
        stopped_at_publication(p, req, io, SynchronizationError::Busy);
        write(
            &p.f.repo,
            ".manyhands/config.toml",
            "this is not a configuration {{{",
        );
    }
    let rows: [(&str, Prepare, Script, BranchAnswer, &str); 15] = [
        (
            "1 merged",
            nothing,
            |p, _, io| {
                io.fetched
                    .push((ChildRole::Publication, own_closure_on_primary(p)))
            },
            BranchAnswer::Merged,
            "main",
        ),
        (
            "2 closed elsewhere",
            nothing,
            |p, _, io| {
                io.fetched
                    .push((ChildRole::Publication, another_closure_on_primary(p)))
            },
            BranchAnswer::ClosedElsewhere,
            "main",
        ),
        (
            "3 remote branch deleted",
            nothing,
            |_, _, io| {
                io.stops.push_back((
                    ChildRole::Publication,
                    SynchronizationError::RemoteContextDeleted,
                    true,
                ))
            },
            BranchAnswer::RemoteBranchDeleted,
            "main",
        ),
        (
            "4 ticket closed",
            nothing,
            |_, _, io| {
                io.stops.push_back((
                    ChildRole::Publication,
                    SynchronizationError::TicketClosed,
                    true,
                ))
            },
            BranchAnswer::TicketClosed,
            "main",
        ),
        (
            "5 remote renamed",
            nothing,
            remote_renamed,
            BranchAnswer::RemoteChanged,
            "main",
        ),
        (
            "5 remote removed",
            nothing,
            remote_removed,
            BranchAnswer::RemoteChanged,
            "main",
        ),
        (
            "5 primary renamed",
            nothing,
            primary_renamed,
            BranchAnswer::RemoteChanged,
            "trunk",
        ),
        (
            "5 no valid configuration",
            nothing,
            configuration_gone,
            BranchAnswer::RemoteChanged,
            "main",
        ),
        (
            "5 publication remote missing",
            nothing,
            remote_missing,
            BranchAnswer::RemoteChanged,
            "main",
        ),
        (
            "5 endpoint changed",
            nothing,
            endpoint_changed,
            BranchAnswer::RemoteChanged,
            "main",
        ),
        (
            "5 configuration invalid",
            nothing,
            configuration_invalid,
            BranchAnswer::RemoteChanged,
            "main",
        ),
        (
            "6 existing",
            |p| {
                let held = p.closed_commit_on(
                    p.f.context_oid(),
                    1_700_000_000,
                    "Earlier <earlier@example.invalid>",
                );
                p.hold(held);
            },
            unscripted,
            BranchAnswer::Existing,
            "main",
        ),
        (
            "7 published",
            nothing,
            unscripted,
            BranchAnswer::Published,
            "main",
        ),
        (
            "8 kept published",
            nothing,
            |p, req, io| {
                stopped_at_publication(p, req, io, SynchronizationError::PushRejected);
                io.drive = Some((ChildRole::Publication, published_by_another_clone(p, false)));
            },
            BranchAnswer::KeptPublished,
            "main",
        ),
        (
            "9 withdrawn since",
            nothing,
            |p, req, io| {
                stopped_at_publication(p, req, io, SynchronizationError::PushRejected);
                io.drive = Some((ChildRole::Publication, published_by_another_clone(p, true)));
            },
            BranchAnswer::WithdrawnSince,
            "main",
        ),
    ];
    for (case, prepare, script, answer, link) in rows {
        let p = PullRequest::new();
        prepare(&p);
        let req = p.request();
        let mut io = PullRequestIo::default();
        script(&p, &req, &mut io);
        let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), answer, "{case}");
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Complete, "{case}");
        assert_eq!(r.answer_code, Some(answer), "{case}");
        p.assert_no_active_child(&r, case);
        // The link is into the primary the configuration names now.
        p.assert_link_into(link);
        assert!(!p.held(), "{case}");
        if answer == BranchAnswer::RemoteChanged {
            // Its commit is kept on the local branch, and nothing published.
            assert_eq!(outcome.publication(), None, "{case}");
            assert_eq!(outcome.checkpoint_oid(), r.closure_oid, "{case}");
            assert_eq!(p.f.context_oid(), r.closure_oid.unwrap(), "{case}");
            assert_ne!(
                p.f.repo.refname_to_id(&tracking_context_ref()).ok(),
                r.closure_oid,
                "{case}"
            );
        }
        // The same request again gives the same answer, and runs nothing.
        let children = io.children();
        assert_eq!(PullRequest::branch(p.run(req, &mut io)), outcome, "{case}");
        assert_eq!(io.children(), children, "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// A child that completed published already: a configuration that moved
/// since does not take its answer from it.
#[test]
fn a_completed_publication_answers_by_its_child_under_a_changed_remote() {
    use crate::repository::FailurePoint;
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo::default();
    let failing = p.f.failing(FailurePoint::AfterPublication);
    assert!(p.run_in(&failing, req.clone(), &mut io).is_err());
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Published);
    configure(&p, Some("other"), "main");
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    assert!(matches!(
        outcome.publication(),
        Some(PromotionPublication::Published { .. })
    ));
    p.assert_primary_untouched(&io);
}

/// A retained conflict is the user's, and a row that would end the record
/// cannot let it go: the child's own stop stands, with its category, and the
/// record stays at `publication_pending` for the conflict verbs and the same
/// request. The realistic case: the forge merged the closure while the
/// conflict was retained, so the tracking primary holds it, and the child
/// meets its conflict again.
#[test]
fn a_retained_conflict_stands_as_the_stop_when_the_tracking_primary_holds_the_closure() {
    use crate::repository::promotion::record::PublicationCategory as P;
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo::default();
    let (child, local) = conflicted_publication(&p, &req, &mut io);
    let target = io.inner.calls.borrow().children.last().unwrap().0.clone();
    io.fetched
        .push((ChildRole::Publication, own_closure_on_primary(&p)));
    io.stops.push_back((
        ChildRole::Publication,
        SynchronizationError::ConflictPending {
            target,
            operation_id: child,
            stage: SynchronizationStage::Context,
        },
        false,
    ));
    let result = p.run(req.clone(), &mut io);
    assert!(
        matches!(&result, Err(TicketClosureError::ContextSynchronization(e)) if matches!(**e, SynchronizationError::ConflictPending { .. })),
        "{result:?}"
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::PublicationPending);
    assert_eq!(r.publication, Some(P::Conflict));
    assert_eq!(r.answer_code, None);
    assert_eq!(r.primary_sync_id, Some(child));
    assert!(p.held());
    // Once the conflict is resolved, the same request answers by the row.
    resolve(&p, child, local);
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Merged);
    p.assert_no_active_child(&p.stored(&req), "resolved");
    p.assert_primary_untouched(&io);
}

/// The same for a stop that would end the record by another row: a retained
/// publication child that meets a deleted remote branch keeps that stop.
#[test]
fn a_retained_child_s_stop_stands_and_the_record_stays_unfinished() {
    use crate::repository::promotion::record::PublicationCategory as P;
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo::default();
    let (child, _) = conflicted_publication(&p, &req, &mut io);
    io.stops.push_back((
        ChildRole::Publication,
        SynchronizationError::RemoteContextDeleted,
        false,
    ));
    let result = p.run(req.clone(), &mut io);
    assert!(
        matches!(
            result,
            Err(TicketClosureError::ContextSynchronization(ref e)) if matches!(**e, SynchronizationError::RemoteContextDeleted)
        ),
        "{result:?}"
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::PublicationPending);
    assert_eq!(r.publication, Some(P::RecoveryRequired));
    assert_eq!(r.answer_code, None);
    assert_eq!(r.primary_sync_id, Some(child));
    assert!(p.held());
    p.assert_primary_untouched(&io);
}

/// A retained context child (D7's conflict) that then meets a D6 end keeps
/// that stop too: the record stays `confirmed`, abandoned over nothing.
#[test]
fn a_retained_context_child_s_end_stands_and_the_record_stays_confirmed() {
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo::default();
    let (child, _) = conflicted_context(&p, &req, &mut io);
    io.stops.push_back((
        ChildRole::Context,
        SynchronizationError::RemoteContextDeleted,
        false,
    ));
    let result = p.run(req.clone(), &mut io);
    assert!(
        matches!(
            result,
            Err(TicketClosureError::ContextSynchronization(ref e)) if matches!(**e, SynchronizationError::RemoteContextDeleted)
        ),
        "{result:?}"
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Confirmed);
    assert_eq!(r.answer_code, None);
    assert_eq!(r.context_sync_id, Some(child));
    assert!(p.holder().is_some());
    p.assert_primary_untouched(&io);
}

/// A failure after a completion still answers from the record the completion
/// stored, as it does for every finished branch record.
#[test]
fn a_failure_after_a_stop_s_completion_answers_from_the_record() {
    use crate::repository::FailurePoint;
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo {
        stops: VecDeque::from([(
            ChildRole::Publication,
            SynchronizationError::RemoteContextDeleted,
            true,
        )]),
        ..PullRequestIo::default()
    };
    let failing = p.f.failing(FailurePoint::AfterCompletion);
    let outcome = PullRequest::branch(p.run_in(&failing, req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::RemoteBranchDeleted);
    assert_eq!(p.stored(&req).step, ClosureStep::Complete);
    p.assert_primary_untouched(&io);
}

/// The forge's merge of the record's pull request: the primary and the
/// context's tip as its parents, so the primary holds the closure and the
/// context's content.
fn forge_merge(p: &PullRequest) -> Oid {
    let (primary, tip) = (p.f.primary(), p.f.context_oid());
    let config = support::tree_bytes(&p.f.repo, primary, ".manyhands/config.toml");
    let tree =
        p.f.repo
            .find_commit(support::tree_commit(
                &p.f.repo,
                tip,
                &[(".manyhands/config.toml", Some((&config, 0o100644)))],
            ))
            .unwrap()
            .tree()
            .unwrap();
    let signature = git2::Signature::now("Forge", "forge@example.invalid").unwrap();
    p.f.repo
        .commit(
            None,
            &signature,
            &signature,
            "Merge pull request",
            &tree,
            &[
                &p.f.repo.find_commit(primary).unwrap(),
                &p.f.repo.find_commit(tip).unwrap(),
            ],
        )
        .unwrap()
}

/// What the hold (R-17) answers a writer of the ticket: `recovery_required`
/// naming the record's request.
fn assert_held<T>(
    case: &str,
    writer: &str,
    record: OperationId,
    result: Result<T, RepositoryError>,
) {
    let error = result
        .err()
        .unwrap_or_else(|| panic!("{case}: {writer} was not refused"));
    assert_eq!(
        error.kind,
        RepositoryErrorKind::RecoveryRequired,
        "{case}: {writer}"
    );
    assert_eq!(
        error.blocker().map(|blocker| blocker.operation_id),
        Some(record),
        "{case}: {writer}"
    );
}

/// D16: while a branch record holds its ticket (at `closing` and at
/// `publication_pending`), every writer of the ticket answers
/// `recovery_required` naming the record's request, and a manual context
/// synchronization is `busy`. The record's own publication child is not
/// refused: the same request then publishes. A direct record keeps today's
/// answer, the guard's `ticket_closed`.
#[test]
fn d16_the_hold_refuses_every_writer_but_its_own_publication_child() {
    use crate::repository::transport::SshTransportErrorKind as K;
    use crate::repository::{
        AuthoringTarget, ContextIntent, ExpectedPathObservation, ItemRepublishInput, Mutation,
        RepairItemRequest, SaveTicketRequest, SubmitCommentRequest, TicketDraft,
        TicketWriteOptions,
    };
    use crate::results::{RecoveryAction, RecoveryActionKind, ResultCode};
    // A direct record is no hold: the guard's own answer stands.
    let f = Fixture::remote();
    let req = f.request();
    let _ = f.run_io(
        &f.failing(FailurePoint::AfterClosureCheckpoint),
        req.clone(),
        &mut FakeIo::default(),
    );
    assert_eq!(f.stored(req.operation_id).step, ClosureStep::Closing);
    let path = f.root().join(".manyhands/worktrees").join(ID);
    let source =
        fs::read_to_string(path.join(".manyhands/tickets").join(ID).join("ticket.md")).unwrap();
    let error = f
        .service
        .save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: f.root().into(),
                kind: AuthoringKind::Ticket,
                item_id: id(),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            },
            draft: TicketDraft {
                title: "Ticket".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
                body: "Edited body.\n".into(),
            },
            expected_path: ExpectedPathObservation::from_bytes(source.as_bytes()),
        })
        .err()
        .expect("a closed ticket was saved");
    assert_eq!(error.kind, RepositoryErrorKind::TicketClosed);
    for case in ["closing", "publication_pending"] {
        let p = PullRequest::new();
        let req = p.request();
        let record = req.operation_id;
        let mut io = PullRequestIo::default();
        if case == "closing" {
            let failing = p.f.failing(FailurePoint::AfterClosureCheckpoint);
            assert!(p.run_in(&failing, req.clone(), &mut io).is_err(), "{case}");
            assert_eq!(p.stored(&req).step, ClosureStep::Closing, "{case}");
        } else {
            // The child's row is not kept, so the hold alone refuses; a kept
            // row is refused earlier, by the live row, as it always was.
            io.stops.push_back((
                ChildRole::Publication,
                transport(K::TransportUnavailable),
                false,
            ));
            assert!(
                matches!(
                    p.run(req.clone(), &mut io),
                    Err(TicketClosureError::ContextSynchronization(_))
                ),
                "{case}"
            );
            assert_eq!(p.stored(&req).step, ClosureStep::PublicationPending);
        }
        assert!(p.held(), "{case}");
        let context = p.f.context_oid();
        let path = p.f.root().join(".manyhands/worktrees").join(ID);
        let ticket_file = path.join(".manyhands/tickets").join(ID).join("ticket.md");
        let source = fs::read_to_string(&ticket_file).unwrap();
        let target = |operation_id| AuthoringTarget {
            root: p.f.root().into(),
            kind: AuthoringKind::Ticket,
            item_id: id(),
            intent: ContextIntent::Edit,
            operation_id,
        };
        let save = || SaveTicketRequest {
            target: target(OperationId::new()),
            draft: TicketDraft {
                title: "Ticket".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
                body: "Edited body.\n".into(),
            },
            expected_path: ExpectedPathObservation::from_bytes(source.as_bytes()),
        };
        let service = &p.f.service;
        assert_held(case, "save", record, service.save_ticket(save()));
        assert_held(
            case,
            "slug",
            record,
            service.save_ticket_with(
                save(),
                TicketWriteOptions {
                    slug: Some("ab-1".into()),
                    ..TicketWriteOptions::default()
                },
            ),
        );
        assert_held(
            case,
            "context prepare",
            record,
            service.prepare_context(target(OperationId::new())),
        );
        assert_held(
            case,
            "comment",
            record,
            service.checkpoint_comment_locally(
                SubmitCommentRequest {
                    target: target(OperationId::new()),
                    comment_id: "01BX5ZZKBKACTAV9WEVGEMMVS0".parse().unwrap(),
                    parent_id: None,
                    body: "A comment.\n".into(),
                    expected_destination: ExpectedPathObservation::Missing,
                },
                None,
            ),
        );
        assert_held(
            case,
            "ticket repair",
            record,
            service.repair_item(RepairItemRequest {
                root: p.f.root().into(),
                kind: AuthoringKind::Ticket,
                path: std::path::PathBuf::from(ticket()),
                source: source.clone(),
                expected: ExpectedPathObservation::from_bytes(source.as_bytes()),
                operation_id: OperationId::new(),
            }),
        );
        // The bindings: a refusal that names the record by its request.
        let named = |writer: &str, value: serde_json::Value| {
            assert_eq!(
                value["code"], "recovery_required",
                "{case}: {writer}: {value}"
            );
            assert_eq!(
                value["data"]["blocker"]["operation_id"],
                record.to_string(),
                "{case}: {writer}: {value}"
            );
        };
        // `context retire` names the record too: the blocker takes the place
        // of the retirement report in the envelope, as for every named answer.
        named(
            "context retire",
            execute(
                &p.f,
                Mutation::ContextRetire(crate::repository::ContextRetireInput {
                    root: p.f.root().into(),
                    kind: AuthoringKind::Ticket,
                    id: id(),
                }),
            ),
        );
        named(
            "item republish",
            serde_json::to_value(
                service.prepare(&Mutation::ItemRepublish(ItemRepublishInput {
                    root: p.f.root().into(),
                    kind: AuthoringKind::Ticket,
                    id: id(),
                })),
            )
            .unwrap(),
        );
        // The ticket's own `context sync-local`: a read error whose recovery
        // action names the record, not a domain error with a blocker.
        let resolved = service.resolve_repository(p.f.root()).unwrap();
        let observation = service
            .observe_local_context_sync(&resolved, AuthoringKind::Ticket, &id())
            .unwrap();
        let refused = service
            .begin_local_context_sync(
                &resolved,
                OperationId::new(),
                AuthoringKind::Ticket,
                &id(),
                &observation.observation,
            )
            .expect_err(&format!("{case}: sync-local was admitted"));
        assert_eq!(refused.code(), ResultCode::RecoveryRequired, "{case}");
        assert_eq!(
            refused.recovery,
            [RecoveryAction::for_operation(
                RecoveryActionKind::OperationShow,
                record.to_string(),
                []
            )],
            "{case}"
        );
        // A manual synchronization of the context is `busy`, as before.
        let plan = p.f.configure_endpoints();
        assert!(
            matches!(
                service
                    .reserve_remote_operation(
                        p.f.root(),
                        OperationId::new(),
                        &p.f.manual_context_sync(&plan)
                    )
                    .unwrap(),
                RemoteReservationOutcome::Busy
            ),
            "{case}"
        );
        // Nothing of the ticket moved, and the record is as it was.
        assert_eq!(p.f.context_oid(), context, "{case}");
        assert!(p.held(), "{case}");
        // The record's own publication child is not refused.
        let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::Published, "{case}");
        assert!(!p.held(), "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// Ruling 19: `item resolve-history` passes a branch record's hold. It
/// changes the history record, not the context, and the same request then
/// continues, so a publication child stopped by unresolved history can be
/// cleared. (A pin of what the writer already does: nothing refuses it.)
#[test]
fn item_resolve_history_passes_a_branch_record_s_hold() {
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo {
        stops: VecDeque::from([(
            ChildRole::Publication,
            SynchronizationError::HistoryUnresolved,
            false,
        )]),
        ..PullRequestIo::default()
    };
    assert!(matches!(
        p.run(req.clone(), &mut io),
        Err(TicketClosureError::ContextSynchronization(_))
    ));
    assert_eq!(p.stored(&req).step, ClosureStep::PublicationPending);
    assert!(p.held());
    // The repository lost its history: the context's row says so, no fetch
    // ever tracked the branch, and the current batch does not advertise it.
    let plan = p.f.configure_endpoints();
    p.f.repo
        .find_reference(&tracking_context_ref())
        .unwrap()
        .delete()
        .unwrap();
    let target = plan.context(AuthoringKind::Ticket, &id());
    state::with_transaction(&p.f.service, p.f.root(), |tx, rid| {
        for table in [
            "remote_context_states",
            "remote_ref_observations",
            "remote_observation_batches",
        ] {
            tx.execute(&format!("DELETE FROM {table}"), [])
                .map_err(|_| state::recovery_required())?;
        }
        tx.execute(
            "INSERT INTO remote_context_states(repository_id,remote_ref,tracking_ref,kind,item_id,publication_evidence,state,observed_at) VALUES(?1,?2,?3,'ticket',?4,'history_unknown','history_unknown',123)",
            rusqlite::params![rid, target.remote_ref(), target.tracking_ref(), ID],
        )
        .map_err(|_| state::recovery_required())?;
        let generation = state::generation(tx, rid)?;
        state::complete_batch(tx, rid, &plan, generation, &[], 100)
    })
    .unwrap();
    let observed =
        p.f.service
            .history_observation(p.f.root(), AuthoringKind::Ticket, &id())
            .unwrap();
    assert!(observed.resolvable());
    // The binding's prepare passes the hold too: a preview, not a refusal.
    let prepared =
        p.f.service
            .prepare(&crate::repository::Mutation::ItemResolveHistory(
                crate::repository::ItemResolveHistoryInput {
                    root: p.f.root().into(),
                    kind: AuthoringKind::Ticket,
                    id: id(),
                    choice: crate::repository::ResolveHistoryChoice::Publish,
                },
            ));
    let value = serde_json::to_value(&prepared).unwrap();
    assert_eq!(value["code"], "ok", "{value}");
    p.f.service
        .resolve_history(p.f.root(), AuthoringKind::Ticket, &id(), &observed)
        .expect("the hold refused the resolution");
    assert!(p.held());
    // The same request then continues.
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    p.assert_primary_untouched(&io);
}

/// R-17's poll row for a branch record (rulings 22 and 26): the publication
/// child was let go, so no row holds the reservation and a poll runs. The
/// forge merged the closure, which the local and the tracking primary hold
/// (here moved by hand), and the context's remote branch is gone: the poll
/// observes the context `Closed`, and its retirement must leave the context
/// alone while the record holds it. The same request then answers `Merged`.
#[test]
fn a_poll_does_not_retire_a_context_a_branch_record_holds() {
    let mut p = PullRequest::new();
    let req = p.request();
    let mut io = CancelAtPublication {
        inner: PullRequestIo::default(),
        parent: Some(req.operation_id),
    };
    assert!(matches!(
        p.run(req.clone(), &mut io),
        Err(TicketClosureError::Interrupted)
    ));
    assert_eq!(p.stored(&req).step, ClosureStep::PublicationPending);
    assert!(p.held());
    let merged = forge_merge(&p);
    checkout(&p.f.repo, merged);
    p.primary_before = merged;
    let context = p.f.context_oid();
    let worktree = p.f.root().join(".manyhands/worktrees").join(ID);
    let value = retiring_poll(&p.f, merged);
    let target = value["data"]["poll"]["targets"]
        .as_array()
        .and_then(|targets| targets.iter().find(|t| t["item_id"] == ID))
        .unwrap_or_else(|| panic!("the ticket context is a poll target: {value}"));
    assert_eq!(target["disposition"], "closed", "{value}");
    assert_candidate_refused(&p.f);
    assert_eq!(
        p.f.repo.refname_to_id(&context_ref()).ok(),
        Some(context),
        "{value}"
    );
    assert!(worktree.exists(), "{value}");
    assert_eq!(p.stored(&req).step, ClosureStep::PublicationPending);
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Merged);
    p.assert_primary_untouched(&io.inner);
}

/// A repair that began before a branch record reached `closing` is refused
/// under the lease: the check before the lease found no hold (the record is
/// out of the item's reach, as it was before it wrote), and the record holds
/// the item once the repair has the lease and its row. The repair writes
/// nothing.
#[test]
fn a_repair_checks_the_hold_again_under_the_lease() {
    use crate::repository::{
        ExpectedPathObservation, LifecycleLeasePhase, REGISTRY_FILE, RepairItemRequest,
    };
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo::default();
    let failing = p.f.failing(FailurePoint::AfterClosureCheckpoint);
    assert!(p.run_in(&failing, req.clone(), &mut io).is_err());
    assert_eq!(p.stored(&req).step, ClosureStep::Closing);
    let context = p.f.context_oid();
    let worktree = p.f.root().join(".manyhands/worktrees").join(ID);
    let ticket_file = worktree
        .join(".manyhands/tickets")
        .join(ID)
        .join("ticket.md");
    let source = fs::read_to_string(&ticket_file).unwrap();
    // The record is not the item's yet: the check before the lease finds
    // nothing. The hook gives it back once the repair holds the lease.
    const ELSEWHERE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    let registry = p.f.data.path().join(REGISTRY_FILE);
    let set_item = move |item: &'static str| {
        rusqlite::Connection::open(&registry)
            .unwrap()
            .execute("UPDATE ticket_closures SET item_id=?1", [item])
            .unwrap();
    };
    let restore = set_item.clone();
    set_item(ELSEWHERE);
    assert!(!p.held());
    p.f.service
        .set_lifecycle_lease_hook_for_testing(LifecycleLeasePhase::RepairBegun, move || {
            restore(ID)
        });
    let result = p.f.service.repair_item(RepairItemRequest {
        root: p.f.root().into(),
        kind: AuthoringKind::Ticket,
        path: std::path::PathBuf::from(ticket()),
        source: source.clone(),
        expected: ExpectedPathObservation::from_bytes(source.as_bytes()),
        operation_id: OperationId::new(),
    });
    assert_held(
        "closing, under the lease",
        "ticket repair",
        req.operation_id,
        result,
    );
    assert_eq!(p.f.context_oid(), context);
    assert_eq!(fs::read_to_string(&ticket_file).unwrap(), source);
    assert!(p.held());
}

/// The poll's retirement claims a candidate under the fence too: a record
/// that reached the item after the candidate was captured (the candidate
/// holds a selected context, so the capture saw no hold) is met by the
/// claim, and the context is left alone.
#[test]
fn a_candidate_captured_before_the_record_held_is_refused_at_its_claim() {
    use crate::repository::REGISTRY_FILE;
    let mut p = PullRequest::new();
    let req = p.request();
    let mut io = CancelAtPublication {
        inner: PullRequestIo::default(),
        parent: Some(req.operation_id),
    };
    assert!(matches!(
        p.run(req.clone(), &mut io),
        Err(TicketClosureError::Interrupted)
    ));
    let merged = forge_merge(&p);
    checkout(&p.f.repo, merged);
    p.primary_before = merged;
    let context = p.f.context_oid();
    let worktree = p.f.root().join(".manyhands/worktrees").join(ID);
    // Until the pass has frozen its candidates, the record is not the
    // item's; then it is.
    const ELSEWHERE: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    let registry = p.f.data.path().join(REGISTRY_FILE);
    let set_item = move |item: &'static str| {
        rusqlite::Connection::open(&registry)
            .unwrap()
            .execute("UPDATE ticket_closures SET item_id=?1", [item])
            .unwrap();
    };
    let restore = set_item.clone();
    set_item(ELSEWHERE);
    crate::repository::context_retirement::handoff::observe_eligible(move |_| restore(ID));
    let value = retiring_poll(&p.f, merged);
    assert!(p.held(), "{value}");
    let retirement = &value["data"]["retirements"][0];
    assert_eq!(retirement["state"], "refused", "{value}");
    assert_eq!(retirement["code"], "recovery_required", "{value}");
    assert_eq!(
        p.f.repo.refname_to_id(&context_ref()).ok(),
        Some(context),
        "{value}"
    );
    assert!(worktree.exists(), "{value}");
    let outcome = PullRequest::branch(p.run(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Merged);
    p.assert_primary_untouched(&io.inner);
}

/// W6 (Tasks 3 and 6, pinned here for the withdraw): a clone that holds the
/// closure `C` unchanged, with work of its own over it, synchronizes with
/// another clone's withdraw of `C`, and the merge keeps the withdraw. The
/// conflicting shape (both sides changed the body) is the integration
/// table's, `provisional_closures_may_change_primary_ones_may_not`.
#[test]
fn w6_another_clone_s_synchronization_keeps_the_withdraw() {
    let p = PullRequest::new();
    let c = p.close_first();
    // Another clone's withdraw of `C`, and work of our own over it.
    let withdrawn = p.withdraw_commit_on(c);
    support::write(&p.f.context, "notes.txt", "notes\n");
    let own = support::commit(&p.f.context);
    assert_eq!(p.f.context_oid(), own);
    // A manual synchronization of the context, bound to no record.
    p.f.configure_endpoints();
    let req = SynchronizeRemoteRequest {
        root: p.f.root().into(),
        operation_id: OperationId::new(),
        target: SynchronizationTarget::Context {
            kind: AuthoringKind::Ticket,
            item_id: id(),
        },
        approval: None,
        confirmed_identity: None,
        restart: false,
    };
    let merged = sync::promotion_test_drive_local(&p.f.service, &req, withdrawn).unwrap();
    // The merge keeps the withdraw and the work, and descends from both.
    let repo = &p.f.repo;
    assert_eq!(p.f.context_oid(), merged);
    assert!(matches!(
        observe::file_closure(&support::tree_bytes(repo, merged, &ticket())),
        observe::FileClosure::Open
    ));
    assert_eq!(
        support::tree_bytes(repo, merged, "notes.txt"),
        b"notes\n".to_vec()
    );
    assert!(repo.graph_descendant_of(merged, own).unwrap());
    assert!(repo.graph_descendant_of(merged, withdrawn).unwrap());
    // It never touches the primary.
    assert_eq!(p.f.primary(), p.primary_before);
}

// --- Task 8, slice B: the withdraw --------------------------------------

/// A repository whose ticket holds a closure to withdraw, and that closure's
/// commit: a pull-request close run to `Published`, or, in direct mode, a
/// closure the context already holds, never published. Either way it is
/// indexed, so a listing reads what the closure does.
fn withdrawable(direct: bool) -> (PullRequest, Oid) {
    if direct {
        let p = PullRequest::direct();
        let held = p.closed_commit_on(p.f.context_oid(), 1_700_000_000, CLOSED_BY);
        p.hold(held);
        p.refresh();
        (p, held)
    } else {
        let p = PullRequest::new();
        let closed = p.close_first();
        (p, closed)
    }
}

/// A record whose first publication child stopped with `error`, its row kept
/// and active: the withdraw is at `publication_pending` for the same request.
fn withdraw_stopped_at_publication(
    p: &PullRequest,
    req: &CloseTicketRequest,
    io: &mut PullRequestIo,
    error: SynchronizationError,
) {
    io.stops.push_back((ChildRole::Publication, error, true));
    let result = p.withdraw_at(req.clone(), io, later);
    assert!(
        matches!(result, Err(TicketClosureError::ContextSynchronization(_))),
        "{result:?}"
    );
    assert_eq!(p.stored(req).step, ClosureStep::PublicationPending);
}

/// A commit on the primary whose ticket is `tip`'s, with one parent: the
/// forge's squash of the pull request.
fn squash_merge(p: &PullRequest, tip: Oid) -> Oid {
    let closed = support::tree_bytes(&p.f.repo, tip, &ticket());
    support::tree_commit(
        &p.f.repo,
        p.f.primary(),
        &[(ticket().as_str(), Some((&closed, 0o100644)))],
    )
}

/// A save of the ticket, which the hold of `record` refuses.
fn assert_save_held(p: &PullRequest, record: OperationId, case: &str) {
    use crate::repository::{
        AuthoringTarget, ContextIntent, ExpectedPathObservation, SaveTicketRequest, TicketDraft,
    };
    let ticket_file =
        p.f.root()
            .join(".manyhands/worktrees")
            .join(ID)
            .join(".manyhands/tickets")
            .join(ID)
            .join("ticket.md");
    let source = fs::read_to_string(ticket_file).unwrap();
    assert_held(
        case,
        "save",
        record,
        p.f.service.save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: p.f.root().into(),
                kind: AuthoringKind::Ticket,
                item_id: id(),
                intent: ContextIntent::Edit,
                operation_id: OperationId::new(),
            },
            draft: TicketDraft {
                title: "Ticket".into(),
                ticket_type: "task".into(),
                status: "open".into(),
                project: None,
                team: None,
                body: "Edited body.\n".into(),
            },
            expected_path: ExpectedPathObservation::from_bytes(source.as_bytes()),
        }),
    );
}

/// W1: a withdraw of a closing ticket writes one commit that removes the
/// closure, publishes it, reopens the ticket and keeps its link, in a
/// pull-request repository, in a direct one with a publication remote, and
/// when only the tracking context holds the closure (its context child brings
/// it in).
#[test]
fn w1_a_withdraw_publishes_and_reopens_in_either_mode() {
    use crate::repository::ReadinessReasonCode as Reason;
    // (case, direct, only the tracking context holds the closure)
    for (case, direct, tracking_only) in [
        ("pull request", false, false),
        ("direct", true, false),
        ("tracking only", false, true),
    ] {
        let mode = if direct {
            Integration::Direct
        } else {
            Integration::PullRequest
        };
        let mut io = PullRequestIo::default();
        let p = if tracking_only {
            let p = PullRequest::new();
            let closed = p.closed_commit_on(p.f.context_oid(), 1_700_000_000, CLOSED_BY);
            p.f.repo
                .reference(&tracking_context_ref(), closed, true, "fixture")
                .unwrap();
            io.drive = Some((ChildRole::Context, closed));
            p
        } else {
            // The index is read after the hold: the ticket is closing and its
            // dependent waits on it.
            let (mut p, _) = withdrawable(direct);
            p.add_dependent();
            assert_eq!(p.listed_state(), Some(ClosureState::Closing), "{case}");
            assert_eq!(p.dependent_reasons(), [Reason::ClosingDependency], "{case}");
            p
        };
        let req = p.withdraw_request();
        let outcome = PullRequest::branch(p.withdraw_at(req.clone(), &mut io, later));
        let r = p.stored(&req);
        let w = p.f.context_oid();
        assert_eq!(outcome.answer(), BranchAnswer::Withdrawn, "{case}");
        assert_eq!(outcome.action(), ClosureAction::Withdraw, "{case}");
        assert_eq!(outcome.checkpoint_oid(), Some(w), "{case}");
        assert_eq!(
            outcome.publication(),
            Some(PromotionPublication::Published { oid: w }),
            "{case}"
        );
        // One commit over the synchronized head, as the run's clock and the
        // identity gave it, with the head's ticket and no closure.
        assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 1, "{case}");
        let commit = p.f.repo.find_commit(w).unwrap();
        assert_eq!(
            commit.summary(),
            Some(withdraw_subject().as_str()),
            "{case}"
        );
        let parent = commit.parent_id(0).unwrap();
        assert_eq!(Some(parent), r.context_oid, "{case}");
        assert_eq!(commit.time().seconds(), later().unix_timestamp(), "{case}");
        assert_eq!(
            support::tree_bytes(&p.f.repo, w, &ticket()),
            observe::withdrawn_ticket_bytes(&support::tree_bytes(&p.f.repo, parent, &ticket()))
                .unwrap(),
            "{case}"
        );
        // The record, in the mode it began in.
        assert_eq!(r.step, ClosureStep::Complete, "{case}");
        assert_eq!(r.answer_code, Some(BranchAnswer::Withdrawn), "{case}");
        assert_eq!(r.withdraw_oid, Some(w), "{case}");
        assert_eq!((r.closure_oid, r.closed_at), (None, None), "{case}");
        assert_eq!(
            (r.integration_mode, r.action),
            (mode, ClosureAction::Withdraw)
        );
        assert_eq!(
            p.f.repo.refname_to_id(&tracking_context_ref()).unwrap(),
            w,
            "{case}"
        );
        // The ticket reads open, and its dependent waits on an open one.
        assert_eq!(p.listed_state(), Some(ClosureState::Open), "{case}");
        if !tracking_only {
            assert_eq!(p.dependent_reasons(), [Reason::OpenDependency], "{case}");
        }
        // The link record is kept as it was: a pull-request close gave it at
        // `CLOCK`, the withdraw at `later` gave none.
        let linked = !direct && !tracking_only;
        assert_eq!(p.link().is_some(), linked, "{case}");
        assert_eq!(p.link_given_at(), linked.then_some(CLOCK), "{case}");
        assert!(!p.held(), "{case}");
        // The same request again gives the same answer, and runs nothing.
        let children = io.children();
        assert_eq!(
            PullRequest::branch(p.withdraw_at(req, &mut io, later)),
            outcome,
            "{case}"
        );
        assert_eq!(io.children(), children, "{case}");
        assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 1, "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// W2: with no closure to withdraw the answer is `already_applied` and
/// nothing is written: at prepare; at a confirm whose closure is gone since
/// the preview (the confirm's refusal); at step 3, after another clone's
/// withdraw arrived with the context child; and for a withdraw made here and
/// never published, whose tracking ref still holds the closure (R-32).
#[test]
fn w2_nothing_to_withdraw_is_already_applied_with_nothing_written() {
    // Prepare.
    let p = PullRequest::new();
    assert_eq!(
        p.withdrawal(),
        ClosureWithdrawalPreflight::NothingToWithdraw
    );
    // Confirm: the closure is gone since the preview.
    let p = PullRequest::new();
    p.hold(p.closed_commit_on(p.f.context_oid(), 1_700_000_000, CLOSED_BY));
    let req = p.withdraw_request();
    p.hold(p.open_commit_on(p.f.context_oid()));
    let refs_before = refs(&p.f.repo);
    let mut io = PullRequestIo::default();
    let result = p.withdraw(req.clone(), &mut io);
    assert!(
        matches!(result, Err(TicketClosureError::ExternalChange)),
        "{result:?}"
    );
    assert_refused_untouched(
        &p.f,
        &req,
        &io.inner,
        (Integration::PullRequest, ClosureAction::Withdraw),
        &refs_before,
        "confirm",
    );
    assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0);
    p.assert_primary_untouched(&io);
    // Step 3: another clone's withdraw arrived with the context child.
    let p = PullRequest::new();
    let closed = p.close_first();
    let req = p.withdraw_request();
    let incoming = p.withdraw_commit_on(closed);
    let mut io = PullRequestIo {
        drive: Some((ChildRole::Context, incoming)),
        ..PullRequestIo::default()
    };
    let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::AlreadyApplied);
    assert_eq!(outcome.checkpoint_oid(), None);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(r.refusal, Some(record::Refusal::ExternalChange));
    assert_eq!(r.answer_code, Some(BranchAnswer::AlreadyApplied));
    assert_eq!(r.withdraw_oid, None);
    assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0);
    // The context is as the child left it: with the other clone's withdraw.
    assert_eq!(Some(p.f.context_oid()), r.context_oid);
    assert!(
        p.f.repo
            .graph_descendant_of(p.f.context_oid(), incoming)
            .unwrap()
    );
    assert!(matches!(
        observe::file_closure(&support::tree_bytes(
            &p.f.repo,
            p.f.context_oid(),
            &ticket()
        )),
        observe::FileClosure::Open
    ));
    assert!(!p.held());
    let children = io.children();
    assert_eq!(PullRequest::branch(p.withdraw(req, &mut io)), outcome);
    assert_eq!(io.children(), children);
    p.assert_primary_untouched(&io);
    // R-32: a withdraw made here and never published. The tracking ref holds
    // the closure, so a withdraw is offered, and its context child publishes
    // the withdraw: nothing is left to write.
    let p = PullRequest::new();
    let closed = p.close_first();
    let local = p.withdraw_commit_on(closed);
    p.hold(local);
    let req = p.withdraw_request();
    let mut io = PullRequestIo::default();
    let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::AlreadyApplied);
    assert_eq!(outcome.publication(), None);
    assert_eq!(
        p.f.repo.refname_to_id(&tracking_context_ref()).unwrap(),
        local
    );
    assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0);
    assert_eq!(p.stored(&req).step, ClosureStep::Abandoned);
    p.assert_primary_untouched(&io);
}

/// W3: a closure the tracking primary holds at step 3 is `ticket_closed`,
/// the withdrawn closure or another: withdrawing it would be a reopen.
#[test]
fn w3_a_closure_the_tracking_primary_holds_at_step_three_is_ticket_closed() {
    type Fetched = fn(&PullRequest) -> Oid;
    for (case, fetched) in [
        ("own", own_closure_on_primary as Fetched),
        ("another", another_closure_on_primary as Fetched),
    ] {
        let p = PullRequest::new();
        p.close_first();
        let req = p.withdraw_request();
        let mut io = PullRequestIo {
            fetched: vec![(ChildRole::Context, fetched(&p))],
            ..PullRequestIo::default()
        };
        let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::AlreadyClosed, "{case}");
        assert_eq!(outcome.checkpoint_oid(), None, "{case}");
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Abandoned, "{case}");
        assert_eq!(
            r.refusal,
            Some(record::Refusal::Blocked(vec![
                ClosureBlocker::AlreadyClosed
            ])),
            "{case}"
        );
        assert_eq!(r.withdraw_oid, None, "{case}");
        assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0, "{case}");
        assert_eq!(io.children(), 1, "{case}");
        assert!(!p.held(), "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// W4: an unfinished closure record holds the ticket: a withdraw is offered
/// and a new confirm of it is `busy`, whichever verb the record is of, and
/// the holder is found for the binding's `held`. A withdraw lost at
/// `BeforeCompletion`, with its commit pushed and both contexts open, is a
/// holder, and its ticket is not "nothing to withdraw" (ruling 35).
#[test]
fn w4_an_unfinished_closure_record_holds_a_withdraw_and_a_withdraw_holds_a_close() {
    use crate::repository::transport::SshTransportErrorKind as K;
    use crate::repository::{Mutation, TicketCloseInput};
    // A close stopped at `publication_pending` holds a new withdraw.
    let p = PullRequest::new();
    let req = p.request();
    let mut io = PullRequestIo {
        stops: VecDeque::from([(
            ChildRole::Publication,
            transport(K::TransportUnavailable),
            true,
        )]),
        ..PullRequestIo::default()
    };
    assert!(matches!(
        p.run(req.clone(), &mut io),
        Err(TicketClosureError::ContextSynchronization(_))
    ));
    assert_eq!(p.stored(&req).step, ClosureStep::PublicationPending);
    assert_eq!(p.holder(), Some(req.operation_id));
    // The record's own child is left out of the observation (ruling 28).
    let other = p.withdraw_request();
    let mut other_io = PullRequestIo::default();
    let refused = p.withdraw(other, &mut other_io);
    assert!(
        matches!(refused, Err(TicketClosureError::Busy)),
        "{refused:?}"
    );
    assert_eq!(other_io.children(), 0);
    let records: i64 =
        p.f.db()
            .query_row("SELECT count(*) FROM ticket_closures", [], |r| r.get(0))
            .unwrap();
    assert_eq!(records, 1);
    p.assert_primary_untouched(&io);
    p.assert_primary_untouched(&other_io);
    // A withdraw stopped at `publication_pending` holds a new close.
    let p = PullRequest::new();
    p.close_first();
    let req = p.withdraw_request();
    let mut io = PullRequestIo::default();
    withdraw_stopped_at_publication(&p, &req, &mut io, transport(K::TransportUnavailable));
    assert_eq!(p.holder(), Some(req.operation_id));
    let prepared =
        p.f.service
            .prepare(&Mutation::TicketClose(TicketCloseInput {
                root: p.f.root().to_owned(),
                id: id(),
            }));
    let value = serde_json::to_value(&prepared).unwrap();
    assert_eq!(value["code"], "recovery_required", "{value}");
    assert!(
        value["recovery"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["operation_id"] == req.operation_id.to_string()),
        "{value}"
    );
    let close = p.request();
    let mut other_io = PullRequestIo::default();
    assert!(matches!(
        p.run(close, &mut other_io),
        Err(TicketClosureError::Busy)
    ));
    assert_eq!(other_io.children(), 0);
    let records: i64 =
        p.f.db()
            .query_row("SELECT count(*) FROM ticket_closures", [], |r| r.get(0))
            .unwrap();
    assert_eq!(records, 2);
    p.assert_primary_untouched(&io);
    p.assert_primary_untouched(&other_io);
    // A withdraw lost before its completion, with its commit pushed: both
    // contexts hold the open ticket, and the record still holds the ticket.
    let p = PullRequest::new();
    let closed = p.close_first();
    let req = p.withdraw_request();
    exit_at_with(&p.f, &req, 12, "withdraw");
    let w = p.f.context_oid();
    assert_ne!(w, closed);
    p.f.repo
        .reference(&tracking_context_ref(), w, true, "the withdraw is pushed")
        .unwrap();
    assert_ne!(p.stored(&req).step, ClosureStep::Complete);
    assert_eq!(p.holder(), Some(req.operation_id));
    assert!(
        matches!(p.withdrawal(), ClosureWithdrawalPreflight::Ready(_)),
        "{:?}",
        p.withdrawal()
    );
    p.assert_primary_untouched(&PullRequestIo::default());
}

/// W5: a child that stops answers as a close's does (D5 to D10), and a
/// publication child's deleted remote branch answers `ticket_closed` when the
/// tracking primary holds a closure, `remote_branch_deleted` otherwise, the
/// withdraw kept on the local branch.
#[test]
fn w5_a_stopped_child_answers_as_a_close_s_and_a_deleted_branch_by_the_tracking_primary() {
    use crate::repository::transport::SshTransportErrorKind as K;
    // D5: a context child that stops, kept, leaves the record `confirmed`
    // for the same request.
    let (p, _) = withdrawable(false);
    let req = p.withdraw_request();
    let mut io = PullRequestIo {
        stops: VecDeque::from([(ChildRole::Context, transport(K::TransportUnavailable), true)]),
        ..PullRequestIo::default()
    };
    let result = p.withdraw(req.clone(), &mut io);
    assert!(
        matches!(result, Err(TicketClosureError::ContextSynchronization(_))),
        "{result:?}"
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Confirmed);
    assert!(p.phase(r.context_sync_id.unwrap()).is_some());
    let outcome = PullRequest::branch(p.withdraw(req, &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Withdrawn);
    p.assert_primary_untouched(&io);
    // D6: a context child that meets a deleted remote branch is let go, and
    // the record is abandoned with its code, nothing written.
    let (p, closed) = withdrawable(false);
    let req = p.withdraw_request();
    let mut io = PullRequestIo {
        stops: VecDeque::from([(
            ChildRole::Context,
            SynchronizationError::RemoteContextDeleted,
            true,
        )]),
        ..PullRequestIo::default()
    };
    let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::ContextRemoteBranchDeleted);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(r.refusal, Some(record::Refusal::ReconfirmationRequired));
    assert_eq!(
        p.phase(r.context_sync_id.unwrap()).as_deref(),
        Some("cancelled")
    );
    p.assert_no_active_child(&r, "D6");
    assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0);
    assert_eq!(p.f.context_oid(), closed);
    assert!(!p.held());
    p.assert_primary_untouched(&io);
    // D8: a publication child that stops holds the ticket, which reads open
    // meanwhile, and refuses a save; the same request goes on. In both modes.
    for (case, direct) in [("pull request", false), ("direct", true)] {
        let (p, _) = withdrawable(direct);
        let req = p.withdraw_request();
        // The child's row is not kept, so the hold alone refuses a save; a
        // kept row is refused earlier, by the live row, as it always was.
        let mut io = PullRequestIo {
            stops: VecDeque::from([(
                ChildRole::Publication,
                transport(K::TransportUnavailable),
                false,
            )]),
            ..PullRequestIo::default()
        };
        let result = p.withdraw_at(req.clone(), &mut io, later);
        assert!(
            matches!(result, Err(TicketClosureError::ContextSynchronization(_))),
            "{case}: {result:?}"
        );
        assert_eq!(
            p.stored(&req).step,
            ClosureStep::PublicationPending,
            "{case}"
        );
        assert!(p.held(), "{case}");
        assert_eq!(p.listed_state(), Some(ClosureState::Open), "{case}");
        assert_save_held(&p, req.operation_id, case);
        let outcome = PullRequest::branch(p.withdraw_at(req.clone(), &mut io, later));
        assert_eq!(outcome.answer(), BranchAnswer::Withdrawn, "{case}");
        assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 1, "{case}");
        assert!(!p.held(), "{case}");
        p.assert_primary_untouched(&io);
    }
    // D11: a publication child's deleted remote branch.
    type Fetched = Option<fn(&PullRequest) -> Oid>;
    let table: [(&str, Fetched, BranchAnswer); 2] = [
        (
            "the tracking primary holds a closure",
            Some(own_closure_on_primary),
            BranchAnswer::WithdrawClosed,
        ),
        (
            "the tracking primary is open",
            None,
            BranchAnswer::RemoteBranchDeleted,
        ),
    ];
    for (case, fetched, answer) in table {
        let (p, closed) = withdrawable(false);
        let req = p.withdraw_request();
        let mut io = PullRequestIo {
            fetched: fetched
                .map(|oid| vec![(ChildRole::Publication, oid(&p))])
                .unwrap_or_default(),
            stops: VecDeque::from([(
                ChildRole::Publication,
                SynchronizationError::RemoteContextDeleted,
                true,
            )]),
            ..PullRequestIo::default()
        };
        let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
        assert_eq!(outcome.answer(), answer, "{case}");
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Complete, "{case}");
        assert_eq!(r.answer_code, Some(answer), "{case}");
        let child = r.primary_sync_id.expect("the child was minted");
        assert_eq!(p.phase(child).as_deref(), Some("cancelled"), "{case}");
        p.assert_no_active_child(&r, case);
        // Nothing was published: the withdraw stays on the local branch.
        let w = r.withdraw_oid.expect("the withdraw was written");
        assert_eq!(outcome.publication(), None, "{case}");
        assert_eq!(outcome.checkpoint_oid(), Some(w), "{case}");
        assert_eq!(p.f.context_oid(), w, "{case}");
        assert_eq!(
            p.f.repo.refname_to_id(&tracking_context_ref()).unwrap(),
            closed,
            "{case}"
        );
        assert!(!p.held(), "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// W7: a close after a withdraw is Part D's close: it observes the head open,
/// records a new closure over the withdraw commit and a new `closed_at`.
#[test]
fn w7_a_close_after_a_withdraw_records_a_new_closed_at() {
    let p = PullRequest::new();
    p.close_first();
    let mut io = PullRequestIo::default();
    let req = p.withdraw_request();
    let withdrawn = PullRequest::branch(p.withdraw(req, &mut io));
    assert_eq!(withdrawn.answer(), BranchAnswer::Withdrawn);
    let w = p.f.context_oid();
    let again = p.request();
    let outcome = PullRequest::branch(p.run_at(again.clone(), &mut io, later));
    assert_eq!(outcome.answer(), BranchAnswer::Published);
    assert_eq!(p.f.commits_in_odb(&close_subject()), 2);
    let closure = p.f.context_oid();
    let commit = p.f.repo.find_commit(closure).unwrap();
    assert_eq!(commit.summary(), Some(close_subject().as_str()));
    assert_eq!(commit.parent_id(0).unwrap(), w);
    let ticket = parse_ticket(&support::tree_bytes(&p.f.repo, closure, &ticket()));
    assert_eq!(ticket.closed_at, Some(later()));
    let r = p.stored(&again);
    assert_eq!(r.closed_at, Some(later()));
    assert_eq!(r.closure_oid, Some(closure));
    assert_eq!(p.listed_state(), Some(ClosureState::Closing));
    assert_eq!(p.link_given_at(), Some(later().unix_timestamp()));
    p.assert_primary_untouched(&io);
}

/// W8: a process lost after the publication child was minted never runs the
/// hook again: the same request answers `withdrawn` with the one commit.
#[test]
fn w8_a_crash_after_the_publication_child_never_runs_the_hook_again() {
    use run::RunEvent;
    // `AfterPublication`, and `BeforeCompletion`.
    for point in [7, 12] {
        let p = PullRequest::new();
        p.close_first();
        let req = p.withdraw_request();
        exit_at_with(&p.f, &req, point, "withdraw");
        let mut io = PullRequestIo::default();
        let mut told = Told(Vec::new());
        let outcome = PullRequest::branch(run::run_with_io_observed(
            &TicketWithdrawLifecycle,
            &p.f.reopened(),
            LifecycleRequest {
                clock,
                ..lifecycle_request(req.clone())
            },
            &mut io,
            &mut told,
        ));
        assert_eq!(outcome.answer(), BranchAnswer::Withdrawn, "{point}");
        assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 1, "{point}");
        assert!(
            !told.0.contains(&RunEvent::Recording),
            "{point}: {:?}",
            told.0
        );
        p.assert_primary_untouched(&io);
    }
}

/// W9: the forge merges the closure between the two children. A fast-forward
/// leaves the withdraw on the local branch only, as the publication child
/// stops with `ticket_closed` (R-30); a merge commit or a squash is merged
/// into the withdraw with the primary's closure, and published. Either way
/// the answer is `ticket_closed`, by the tracking primary.
#[test]
fn w9_a_forge_merge_between_the_children_answers_ticket_closed_in_every_style() {
    type Forge = fn(&PullRequest, Oid) -> Oid;
    let table: [(&str, Forge, bool); 3] = [
        ("fast-forward", |_, closed| closed, true),
        ("merge commit", |p, _| forge_merge(p), false),
        ("squash", |p, closed| squash_merge(p, closed), false),
    ];
    for (case, forge, fast_forward) in table {
        let (p, closed) = withdrawable(false);
        let forged = forge(&p, closed);
        let req = p.withdraw_request();
        // `decide_push` is R-30's decision as production's
        // `push_unless_unclosed` makes it, which Task 6 unit-tests: a head
        // that lacks the closure the fetched primary holds is not pushed.
        let mut io = PullRequestIo {
            fetched: vec![(ChildRole::Publication, forged)],
            drive: Some((ChildRole::Publication, closed)),
            decide_push: true,
            ..PullRequestIo::default()
        };
        let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
        assert_eq!(outcome.answer(), BranchAnswer::WithdrawClosed, "{case}");
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Complete, "{case}");
        assert_eq!(r.answer_code, Some(BranchAnswer::WithdrawClosed), "{case}");
        p.assert_no_active_child(&r, case);
        let w = r.withdraw_oid.expect("the withdraw was written");
        assert_eq!(outcome.checkpoint_oid(), Some(w), "{case}");
        let tracking = p.f.repo.refname_to_id(&tracking_context_ref()).unwrap();
        if fast_forward {
            // Nothing was pushed: the withdraw is on the local branch only.
            assert_eq!(p.f.context_oid(), w, "{case}");
            assert_eq!(tracking, closed, "{case}");
            assert_eq!(outcome.publication(), None, "{case}");
        } else {
            // The merge was given the primary's closure, and published.
            let Some(PromotionPublication::Published { oid: head }) = outcome.publication() else {
                panic!("{case}: {:?}", outcome.publication());
            };
            assert_eq!(tracking, head, "{case}");
            assert!(p.f.repo.graph_descendant_of(head, w).unwrap(), "{case}");
            let ticket = parse_ticket(&support::tree_bytes(&p.f.repo, head, &ticket()));
            assert_eq!(ticket.closed_at, Some(clock()), "{case}");
            assert_eq!(ticket.closed_by.as_deref(), Some(CLOSED_BY), "{case}");
        }
        assert!(!p.held(), "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// W10: a cancelled publication child is let go, and the same request
/// replaces it, as D15 does for a close.
#[test]
fn w10_a_cancelled_withdraw_is_continued_by_the_same_request() {
    let p = PullRequest::new();
    p.close_first();
    let req = p.withdraw_request();
    let mut io = CancelAtPublication {
        inner: PullRequestIo::default(),
        parent: Some(req.operation_id),
    };
    let result = p.withdraw(req.clone(), &mut io);
    assert!(
        matches!(result, Err(TicketClosureError::Interrupted)),
        "{result:?}"
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::PublicationPending);
    assert_eq!(
        r.publication,
        Some(crate::repository::promotion::record::PublicationCategory::Cancelled)
    );
    let first = r.primary_sync_id.unwrap();
    let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Withdrawn);
    assert_ne!(p.stored(&req).primary_sync_id, Some(first));
    p.assert_primary_untouched(&io.inner);
}

/// W11: another closure than the consented one on the head at step 3 is
/// `external_change`, with nothing written.
#[test]
fn w11_another_closure_on_the_head_at_step_three_is_external_change() {
    let p = PullRequest::new();
    let closed = p.close_first();
    let req = p.withdraw_request();
    let incoming = p.closed_commit_on(closed, 1_750_000_000, "Another <another@example.invalid>");
    let mut io = PullRequestIo {
        drive: Some((ChildRole::Context, incoming)),
        ..PullRequestIo::default()
    };
    let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::ExternalChange);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(r.refusal, Some(record::Refusal::ExternalChange));
    assert_eq!(r.withdraw_oid, None);
    assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0);
    // The context is as the child left it, holding the other closure.
    assert_eq!(Some(p.f.context_oid()), r.context_oid);
    let held = parse_ticket(&support::tree_bytes(
        &p.f.repo,
        p.f.context_oid(),
        &ticket(),
    ));
    assert_eq!(
        held.closed_by.as_deref(),
        Some("Another <another@example.invalid>")
    );
    assert!(!p.held());
    p.assert_primary_untouched(&io);
}

/// 3.1 step 5: a head that names a closure that does not read (its
/// `closed_by` is gone) is not a closure to withdraw: `external_change`, with
/// nothing written.
#[test]
fn a_head_naming_a_closure_it_does_not_hold_is_external_change_at_step_three() {
    let p = PullRequest::new();
    let closed = p.close_first();
    let req = p.withdraw_request();
    let text = String::from_utf8(support::tree_bytes(&p.f.repo, closed, &ticket())).unwrap();
    let malformed = text
        .lines()
        .filter(|line| !line.starts_with("closed_by"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_ne!(text, malformed);
    let incoming = support::tree_commit(
        &p.f.repo,
        closed,
        &[(ticket().as_str(), Some((malformed.as_bytes(), 0o100644)))],
    );
    let mut io = PullRequestIo {
        drive: Some((ChildRole::Context, incoming)),
        ..PullRequestIo::default()
    };
    let outcome = PullRequest::branch(p.withdraw(req.clone(), &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::ExternalChange);
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(r.refusal, Some(record::Refusal::ExternalChange));
    assert_eq!(r.withdraw_oid, None);
    assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0);
    // The context is as the child left it.
    assert_eq!(Some(p.f.context_oid()), r.context_oid);
    assert!(
        p.f.repo
            .graph_descendant_of(p.f.context_oid(), incoming)
            .unwrap()
    );
    assert!(!p.held());
    p.assert_primary_untouched(&io);
}

/// W12: with no publication remote a withdraw is refused in direct mode, at
/// prepare and at a confirm, which is stored as a withdraw's own row.
#[test]
fn w12_without_a_publication_remote_a_withdraw_is_refused_in_direct_mode() {
    let p = PullRequest::local();
    p.hold(p.closed_commit_on(p.f.context_oid(), 1_700_000_000, CLOSED_BY));
    assert_eq!(
        p.withdrawal(),
        ClosureWithdrawalPreflight::PublicationRemoteRequired
    );
    let req = CloseTicketRequest {
        root: p.f.root().into(),
        operation_id: OperationId::new(),
        item_id: id(),
        expected: ClosureExpectation::from_bytes([7; 32]),
        approval: None,
        confirmed_identity: None,
    };
    let refs_before = refs(&p.f.repo);
    let mut io = PullRequestIo::default();
    let result = p.withdraw(req.clone(), &mut io);
    assert!(
        matches!(result, Err(TicketClosureError::ExternalChange)),
        "{result:?}"
    );
    assert_refused_untouched(
        &p.f,
        &req,
        &io.inner,
        (Integration::Direct, ClosureAction::Withdraw),
        &refs_before,
        "no remote",
    );
    assert_eq!(p.stored(&req).remote_name, None);
    assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0);
    p.assert_primary_untouched(&io);
}

/// W14: a ticket with no context is `context_missing`, at prepare and at a
/// confirm, which is stored as the withdraw's refusal with nothing run.
#[test]
fn w14_a_ticket_with_no_context_is_context_missing() {
    let (p, _) = withdrawable(false);
    let req = p.withdraw_request();
    p.f.repo
        .find_worktree(ID)
        .unwrap()
        .prune(Some(
            git2::WorktreePruneOptions::new()
                .valid(true)
                .working_tree(true),
        ))
        .unwrap();
    p.f.repo
        .find_reference(&context_ref())
        .unwrap()
        .delete()
        .unwrap();
    assert_eq!(
        p.withdrawal(),
        ClosureWithdrawalPreflight::Blocked {
            blockers: vec![ClosureBlocker::ContextMissing],
            identity_configuration: None,
        }
    );
    let mut io = PullRequestIo::default();
    let result = p.withdraw(req.clone(), &mut io);
    assert!(
        matches!(&result, Err(TicketClosureError::Blocked(b)) if *b == [ClosureBlocker::ContextMissing]),
        "{result:?}"
    );
    let r = p.stored(&req);
    assert_eq!(r.step, ClosureStep::Abandoned);
    assert_eq!(
        r.refusal,
        Some(record::Refusal::Blocked(vec![
            ClosureBlocker::ContextMissing
        ]))
    );
    assert_eq!(
        (r.integration_mode, r.action),
        (Integration::PullRequest, ClosureAction::Withdraw)
    );
    assert_eq!(io.children(), 0);
    assert_eq!(p.f.commits_in_odb(&withdraw_subject()), 0);
    p.assert_primary_untouched(&io);
}

/// The withdraw's answer table, row by row, as the close's is: each row is
/// chosen by what the tracking primary and the publication child say, in the
/// table's order, and the record completes with it, no child left holding the
/// context and the link record kept as the close gave it. The withdraws run
/// at `later`, so a link given again would show.
#[test]
fn withdraw_answers_choose_each_row_in_order() {
    use crate::repository::transport::SshTransportErrorKind as K;
    type Script = fn(&PullRequest, &CloseTicketRequest, &mut PullRequestIo);
    let rows: [(&str, Script, BranchAnswer); 8] = [
        (
            "1 own closure fetched",
            |p, _, io| {
                io.fetched
                    .push((ChildRole::Publication, own_closure_on_primary(p)))
            },
            BranchAnswer::WithdrawClosed,
        ),
        (
            "1 another closure fetched",
            |p, _, io| {
                io.fetched
                    .push((ChildRole::Publication, another_closure_on_primary(p)))
            },
            BranchAnswer::WithdrawClosed,
        ),
        (
            "3 ticket closed",
            |_, _, io| {
                io.stops.push_back((
                    ChildRole::Publication,
                    SynchronizationError::TicketClosed,
                    true,
                ))
            },
            BranchAnswer::WithdrawClosed,
        ),
        (
            "4 remote branch deleted",
            |_, _, io| {
                io.stops.push_back((
                    ChildRole::Publication,
                    SynchronizationError::RemoteContextDeleted,
                    true,
                ))
            },
            BranchAnswer::RemoteBranchDeleted,
        ),
        (
            "2 remote removed",
            |p, req, io| {
                withdraw_stopped_at_publication(p, req, io, SynchronizationError::Busy);
                configure(p, None, "main");
            },
            BranchAnswer::RemoteChanged,
        ),
        (
            "5 publication remote missing",
            |_, _, io| {
                io.stops.push_back((
                    ChildRole::Publication,
                    transport(K::PublicationRemoteMissing),
                    true,
                ))
            },
            BranchAnswer::RemoteChanged,
        ),
        (
            "6 closed since",
            |p, req, io| {
                // The remote's publication was rejected; the retry merges what
                // another clone published since: a closure on the head.
                withdraw_stopped_at_publication(p, req, io, SynchronizationError::PushRejected);
                let w = p.f.context_oid();
                io.drive = Some((
                    ChildRole::Publication,
                    p.closed_commit_on(w, 1_750_000_000, "Another <another@example.invalid>"),
                ));
            },
            BranchAnswer::ClosedSince,
        ),
        ("7 withdrawn", |_, _, _| {}, BranchAnswer::Withdrawn),
    ];
    for (case, script, answer) in rows {
        let (p, _) = withdrawable(false);
        let req = p.withdraw_request();
        let mut io = PullRequestIo::default();
        script(&p, &req, &mut io);
        let outcome = PullRequest::branch(p.withdraw_at(req.clone(), &mut io, later));
        assert_eq!(outcome.answer(), answer, "{case}");
        let r = p.stored(&req);
        assert_eq!(r.step, ClosureStep::Complete, "{case}");
        assert_eq!(r.answer_code, Some(answer), "{case}");
        p.assert_no_active_child(&r, case);
        assert_eq!(p.link_given_at(), Some(CLOCK), "{case}");
        assert!(!p.held(), "{case}");
        if answer == BranchAnswer::RemoteChanged {
            // Its commit is kept on the local branch, and nothing published.
            assert_eq!(outcome.publication(), None, "{case}");
            assert_eq!(outcome.checkpoint_oid(), r.withdraw_oid, "{case}");
            assert_eq!(p.f.context_oid(), r.withdraw_oid.unwrap(), "{case}");
            assert_ne!(
                p.f.repo.refname_to_id(&tracking_context_ref()).ok(),
                r.withdraw_oid,
                "{case}"
            );
        }
        // The same request again gives the same answer, and runs nothing.
        let children = io.children();
        assert_eq!(
            PullRequest::branch(p.withdraw_at(req, &mut io, later)),
            outcome,
            "{case}"
        );
        assert_eq!(io.children(), children, "{case}");
        p.assert_primary_untouched(&io);
    }
}

/// A request continues only a record of its own verb: another verb's record
/// of its ID and digest is not answered from, not run on and not refused
/// into, whatever its step. A direct close's completed record is the row
/// that only the runner's guard refuses.
#[test]
fn a_request_continues_only_its_own_verb_s_record() {
    use crate::repository::{ConfigurationInspection, read_configuration};
    // A completed close, sent as a withdraw.
    let p = PullRequest::new();
    let (close_req, _) = p.close_first_request();
    let before = format!("{:?}", p.stored(&close_req));
    let mut io = PullRequestIo::default();
    let result = p.withdraw(close_req.clone(), &mut io);
    assert!(
        matches!(result, Err(TicketClosureError::OperationMismatch)),
        "{result:?}"
    );
    assert_eq!(io.children(), 0);
    assert_eq!(format!("{:?}", p.stored(&close_req)), before);
    // A refused request that meets the record at its insertion: either
    // verb's refusal finds the other's record.
    let ConfigurationInspection::Valid(config) = read_configuration(p.f.root()).unwrap() else {
        panic!("a valid configuration")
    };
    let refusal = record::Refusal::ExternalChange;
    let refused = state::with_transaction(&p.f.service, p.f.root(), |tx, rid| {
        Ok(<TicketWithdrawLifecycle as Lifecycle>::refuse_record(
            tx,
            rid,
            close_req.operation_id,
            &id(),
            close_req.expected,
            &config,
            None,
            &refusal,
        ))
    })
    .unwrap();
    assert!(
        matches!(&refused, Err(e) if e.kind == RepositoryErrorKind::OperationMismatch),
        "{refused:?}"
    );
    // A completed direct close, sent as a withdraw: the runner's guard.
    let f = Fixture::new();
    let direct = f.request();
    f.close(&f.service, direct.clone()).unwrap();
    let before = format!("{:?}", f.stored(direct.operation_id));
    let result = run::run_with_io(
        &TicketWithdrawLifecycle,
        &f.service,
        lifecycle_request(direct.clone()),
        &mut FakeIo::default(),
    );
    assert!(
        matches!(result, Err(TicketClosureError::OperationMismatch)),
        "{result:?}"
    );
    assert_eq!(format!("{:?}", f.stored(direct.operation_id)), before);
    // A withdraw stopped at `publication_pending`, sent as a close.
    let p = PullRequest::new();
    p.close_first();
    let req = p.withdraw_request();
    let mut io = PullRequestIo::default();
    withdraw_stopped_at_publication(
        &p,
        &req,
        &mut io,
        transport(crate::repository::transport::SshTransportErrorKind::TransportUnavailable),
    );
    let before = format!("{:?}", p.stored(&req));
    let mut other = PullRequestIo::default();
    let result = p.run(req.clone(), &mut other);
    assert!(
        matches!(result, Err(TicketClosureError::OperationMismatch)),
        "{result:?}"
    );
    assert_eq!(other.children(), 0);
    assert_eq!(format!("{:?}", p.stored(&req)), before);
    let refused = state::with_transaction(&p.f.service, p.f.root(), |tx, rid| {
        Ok(<TicketClosureLifecycle as Lifecycle>::refuse_record(
            tx,
            rid,
            req.operation_id,
            &id(),
            req.expected,
            &config,
            None,
            &refusal,
        ))
    })
    .unwrap();
    assert!(
        matches!(&refused, Err(e) if e.kind == RepositoryErrorKind::OperationMismatch),
        "{refused:?}"
    );
    // The withdraw's own request goes on.
    let outcome = PullRequest::branch(p.withdraw(req, &mut io));
    assert_eq!(outcome.answer(), BranchAnswer::Withdrawn);
    p.assert_primary_untouched(&io);
    p.assert_primary_untouched(&other);
}
