//! The pull-request close's consent, through the runner: a confirm that
//! re-observes in the configuration's mode, and what it stores when the
//! consent no longer holds. Until the branch runner exists, a consent that
//! holds writes nothing and stops, so these rows are the refusals.
use super::*;
use crate::{
    canonical::Integration,
    repository::promotion::{lifecycle::Lifecycle, record::CleanupCategory},
};

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
