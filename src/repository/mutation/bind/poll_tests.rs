//! The poll binding's hand-off to the setup poll (Wave 06 Cycle 02, design
//! Part G and L12), driven through the mutation boundary over a `Session`
//! whose polls are scripted, so that no remote is reached and no key is
//! needed.

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
};

use serde_json::Value;

use crate::{
    forge,
    repository::{
        Mutation, MutationDataDto, PollOnceInput, ReadError, RemotePollError, RepositoryError,
        RepositoryService, RequestId, RequestState, SetupPollError, SetupPollOutcome,
        SetupPollReport,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
        mutation::{Accepted, CancellationToken, progress::Session},
        open_registry,
        promotion::lifecycle::CancellationProbe,
        promotion::run::RunObserver,
        setup::{
            publish::{Published, SetupError},
            record::{self, SetupKind},
            test_world::{CURRENT, OTHER, PRIMARY, SETUP, World},
        },
        transport::{SshDirection, SshTransportError, SshTransportErrorKind},
    },
    results::{Envelope, ResultCode},
};

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";

/// What the scripted `poll_setup` does at one call.
enum Step {
    /// The setup poll saw this.
    Saw(SetupPollOutcome),
    /// The call stops with this.
    Fail(fn() -> SetupPollError),
    /// The process is killed in the middle of the call.
    Killed,
}

/// A session whose two polls are scripted, and that logs which it was asked
/// for, in order. A call the script has no step for is a call the test did
/// not expect.
struct Scripted {
    setup: RefCell<VecDeque<Step>>,
    log: RefCell<Vec<&'static str>>,
    /// What today's poll answers; the same each time it is asked.
    poll_once: Cell<bool>,
    /// Today's poll is the domain's, over a synthetic fetch, as production
    /// runs it; otherwise it answers `busy`.
    real_poll: Cell<bool>,
}

impl Scripted {
    fn new(steps: Vec<Step>) -> Self {
        Self {
            setup: RefCell::new(steps.into()),
            log: RefCell::new(Vec::new()),
            poll_once: Cell::new(true),
            real_poll: Cell::new(false),
        }
    }

    fn log(&self) -> Vec<&'static str> {
        self.log.borrow().clone()
    }
}

/// Credentials that are never asked for: the fetch is synthetic.
struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("a synthetic fetch asks for no passphrase")
    }
}

fn report(service: &RepositoryService, root: &Path, outcome: SetupPollOutcome) -> SetupPollReport {
    let db = open_registry(&service.registry_path, &mut |_| {}).unwrap();
    let id: i64 = db
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root.to_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    let record = record::read(&db, id).unwrap().expect("a setup");
    SetupPollReport {
        setup_id: record.setup_id.clone(),
        outcome,
        integration: None,
        differs: false,
        link: forge::pull_request_link(
            None,
            &format!("manyhands/setup/{}", record.setup_id),
            &record.primary_branch,
        ),
    }
}

impl Session for Scripted {
    fn poll_setup(
        &self,
        service: &RepositoryService,
        root: &Path,
        _: RequestId,
    ) -> Option<Result<SetupPollReport, SetupPollError>> {
        self.log.borrow_mut().push("poll_setup");
        let step = self
            .setup
            .borrow_mut()
            .pop_front()
            .expect("poll_setup was called more often than the test expected");
        match step {
            Step::Saw(outcome) => Some(Ok(report(service, root, outcome))),
            Step::Fail(make) => Some(Err(make())),
            Step::Killed => panic!("killed in the middle of the setup poll"),
        }
    }

    fn poll_once(
        &self,
        service: &RepositoryService,
        request: crate::repository::PollPublicationRemoteRequest,
        asked_of: RequestId,
    ) -> Option<Result<crate::repository::RemotePollResult, RemotePollError>> {
        self.log.borrow_mut().push("poll_once");
        assert!(self.poll_once.get(), "today's poll was not expected");
        if self.real_poll.get() {
            let mut credentials = SessionCredentials::new(NoPassphrase);
            let probe = CancellationProbe::new(CancellationToken::new(), asked_of);
            return Some(service.poll_publication_remote_observed(
                request,
                &mut credentials,
                probe,
            ));
        }
        // Another's short local work: a plain answer with no report to build.
        Some(Err(RemotePollError::Busy))
    }

    fn poll_once_retiring(
        &self,
        _: &RepositoryService,
        _: crate::repository::PollPublicationRemoteRequest,
        _: &Accepted,
    ) -> Option<
        Result<
            (
                crate::repository::RemotePollResult,
                Option<crate::repository::context_retirement::handoff::Live>,
            ),
            RemotePollError,
        >,
    > {
        self.log.borrow_mut().push("poll_once_retiring");
        None
    }

    fn cancelled(&self) -> bool {
        false
    }

    fn report(&self, _: &crate::repository::ProgressEvent) {}

    fn retire_context(
        &self,
        _: &RepositoryService,
        _: crate::repository::RetireContextRequest,
        _: &Accepted,
        _: bool,
        _: Option<crate::repository::context_retirement::handoff::InitialInput>,
    ) -> Option<Result<crate::repository::ContextRetirementOutcome, RepositoryError>> {
        None
    }

    fn synchronize(
        &self,
        _: &RepositoryService,
        _: crate::repository::SynchronizeRemoteRequest,
        _: crate::repository::SynchronizationSettings,
        _: &mut dyn crate::repository::SynchronizationObserver,
    ) -> Option<
        Result<crate::repository::SynchronizationResult, crate::repository::SynchronizationError>,
    > {
        None
    }

    fn republish(
        &self,
        _: &RepositoryService,
        _: crate::repository::SynchronizeRemoteRequest,
        _: &crate::repository::remote::sync::RepublishConsent,
        _: &mut dyn crate::repository::SynchronizationObserver,
    ) -> Option<
        Result<crate::repository::SynchronizationResult, crate::repository::SynchronizationError>,
    > {
        None
    }

    fn promote_document(
        &self,
        _: &RepositoryService,
        _: crate::repository::PromoteDocumentRequest,
        _: RequestId,
        _: &mut dyn RunObserver,
    ) -> Option<
        Result<
            crate::repository::DocumentPromotionResult,
            crate::repository::DocumentPromotionError,
        >,
    > {
        None
    }

    fn close_ticket(
        &self,
        _: &RepositoryService,
        _: crate::repository::CloseTicketRequest,
        _: RequestId,
        _: &mut dyn RunObserver,
    ) -> Option<Result<crate::repository::TicketClosureResult, crate::repository::TicketClosureError>>
    {
        None
    }

    fn verify_transport(
        &self,
        _: &RepositoryService,
        _: crate::repository::transport::VerifySshTransportRequest,
    ) -> Option<
        Result<
            crate::repository::transport::SshTransportVerified,
            crate::repository::transport::SshTransportError,
        >,
    > {
        None
    }

    fn verify_setup_transport(
        &self,
        _: &RepositoryService,
        _: crate::repository::transport::VerifySshTransportRequest,
        _: &crate::repository::transport::SetupScope,
    ) -> Option<
        Result<
            crate::repository::transport::SshTransportVerified,
            crate::repository::transport::SshTransportError,
        >,
    > {
        None
    }

    fn publish_setup(
        &self,
        _: &RepositoryService,
        _: &Path,
        _: RequestId,
    ) -> Option<Result<Published, SetupError>> {
        None
    }
}

fn run(world: &World, session: &dyn Session, retiring: bool) -> Envelope<MutationDataDto> {
    let input = PollOnceInput {
        root: world.root.clone(),
    };
    world.service.execute_binding(
        RequestId::parse(R1).unwrap(),
        None,
        if retiring {
            Mutation::PollOnceRetiring(input)
        } else {
            Mutation::PollOnce(input)
        }
        .into_binding(),
        session,
    )
}

fn data_of(envelope: &Envelope<MutationDataDto>) -> Value {
    serde_json::to_value(&envelope.data).unwrap()
}

fn request_state(world: &World) -> Option<RequestState> {
    world
        .service
        .request_record(RequestId::parse(R1).unwrap())
        .unwrap()
        .map(|record| record.state)
}

fn remote_rows(world: &World) -> i64 {
    world.count("SELECT COUNT(*) FROM remote_operation_records")
}

fn transport(kind: SshTransportErrorKind) -> SetupPollError {
    SetupPollError::Transport(SshTransportError {
        root: "/nowhere".into(),
        remote_name: OTHER.into(),
        direction: SshDirection::Fetch,
        selected_key_id: None,
        authority: None,
        kind,
    })
}

/// L12: a setup poll request killed mid-run and sent again takes the setup
/// path again, and is finished on its answer, with no remote row; a change
/// on another remote polls the current remote first, and its answer is
/// today's poll's; a change on the same remote has today's poll alone.
#[test]
fn a_setup_poll_rerun_takes_the_setup_path() {
    for retiring in [false, true] {
        let what = if retiring {
            "--retire-completed"
        } else {
            "poll once"
        };
        let world = World::new(SetupKind::Enable, OTHER, true);

        // Killed in the middle of the setup poll: the request stays accepted.
        let killed = Scripted::new(vec![Step::Killed]);
        let result = catch_unwind(AssertUnwindSafe(|| run(&world, &killed, retiring)));
        assert!(result.is_err(), "{what}: the call was killed");
        assert_eq!(killed.log(), ["poll_setup"], "{what}");
        assert_eq!(
            request_state(&world),
            Some(RequestState::Accepted),
            "{what}"
        );

        // Sent again: `rerun` takes the setup path again, today's poll is
        // not run, and the request is finished on the answer.
        let again = Scripted::new(vec![Step::Saw(SetupPollOutcome::WaitingForMerge)]);
        let answer = run(&world, &again, retiring);
        assert_eq!(answer.code, ResultCode::Ok, "{what}: {}", answer.message);
        assert_eq!(again.log(), ["poll_setup"], "{what}");
        let data = data_of(&answer);
        assert_eq!(data["type"], "setup_poll", "{what}");
        assert_eq!(data["setup_id"], SETUP, "{what}");
        assert_eq!(data["outcome"], "waiting_for_merge", "{what}");
        assert_eq!(data["integration"], Value::Null, "{what}");
        assert_eq!(data["differs"], false, "{what}");
        assert_eq!(data["pull_request"]["target"], PRIMARY, "{what}");
        assert_eq!(
            request_state(&world),
            Some(RequestState::Finished),
            "{what}"
        );
        assert_eq!(
            remote_rows(&world),
            0,
            "{what}: a setup poll reserves no remote row"
        );

        // Sent a third time: the stored envelope, read back by its tag, and
        // no poll at all.
        let replay = Scripted::new(vec![]);
        let replayed = run(&world, &replay, retiring);
        assert_eq!(replay.log(), Vec::<&str>::new(), "{what}");
        assert_eq!(replayed.code, ResultCode::Ok, "{what}");
        assert_eq!(data_of(&replayed), data, "{what}");
    }

    // A change on another remote: today's poll of the current remote runs
    // first and its answer is the request's; the setup poll follows it and
    // reports nothing.
    let world = World::new(SetupKind::Change, OTHER, true);
    let session = Scripted::new(vec![Step::Saw(SetupPollOutcome::WaitingForMerge)]);
    let answer = run(&world, &session, false);
    assert_eq!(session.log(), ["poll_once", "poll_setup"]);
    assert_eq!(answer.code, ResultCode::Busy, "today's poll's answer");
    assert_ne!(data_of(&answer)["type"], "setup_poll");

    // A retiring request on a change on another remote, entered again: the
    // setup poll follows today's poll once, not once in the ordinary rerun
    // and again after it.
    let world = World::new(SetupKind::Change, OTHER, true);
    let first = Scripted::new(vec![]);
    let owed = run(&world, &first, true);
    assert_eq!(owed.code, ResultCode::InternalError);
    assert_eq!(first.log(), ["poll_once_retiring"]);
    assert_eq!(request_state(&world), Some(RequestState::Accepted));
    let again = Scripted::new(vec![Step::Saw(SetupPollOutcome::WaitingForMerge)]);
    let answer = run(&world, &again, true);
    assert_eq!(answer.code, ResultCode::Busy, "today's poll's answer");
    assert_eq!(again.log(), ["poll_once", "poll_setup"]);

    // A setup that ended between the choice of the poll and the lock has
    // nothing to poll and nothing to recover: today's poll runs.
    let world = World::new(SetupKind::Enable, OTHER, true);
    let session = Scripted::new(vec![Step::Fail(|| SetupPollError::Gone)]);
    let answer = run(&world, &session, false);
    assert_eq!(session.log(), ["poll_setup", "poll_once"]);
    assert_eq!(answer.code, ResultCode::Busy, "today's poll's answer");

    // A change on the same remote: today's poll alone.
    let world = World::new(SetupKind::Change, CURRENT, true);
    let session = Scripted::new(vec![]);
    let answer = run(&world, &session, false);
    assert_eq!(session.log(), ["poll_once"]);
    assert_eq!(answer.code, ResultCode::Busy);
}

/// A key, unlock, network or Ctrl-C stop of the setup poll is final: the
/// request is finished with that code, a new request polls again, and the
/// same request again is that answer, with no second poll.
#[test]
fn the_setup_poll_s_stops_are_final() {
    type Row = (&'static str, fn() -> SetupPollError, ResultCode);
    let rows: Vec<Row> = vec![
        (
            "key",
            || transport(SshTransportErrorKind::NoSelectedKey),
            ResultCode::NoSelectedKey,
        ),
        (
            "unlock",
            || transport(SshTransportErrorKind::UnlockCancelled),
            ResultCode::UnlockRequired,
        ),
        (
            "network",
            || transport(SshTransportErrorKind::TransportUnavailable),
            ResultCode::TransportUnavailable,
        ),
        (
            "Ctrl-C",
            || SetupPollError::Cancelled,
            ResultCode::Cancelled,
        ),
        ("held lock", || SetupPollError::Busy, ResultCode::Busy),
        (
            "a registry that cannot be read",
            || SetupPollError::Registry(ReadError::new(ResultCode::IndexUnavailable)),
            ResultCode::IndexUnavailable,
        ),
    ];
    for (what, fail, code) in rows {
        let world = World::new(SetupKind::Enable, OTHER, true);
        let session = Scripted::new(vec![Step::Fail(fail)]);

        let answer = run(&world, &session, false);

        assert_eq!(answer.code, code, "{what}");
        assert_eq!(session.log(), ["poll_setup"], "{what}");
        assert_eq!(
            request_state(&world),
            Some(RequestState::Finished),
            "{what}: a stop is final"
        );
        assert_eq!(remote_rows(&world), 0, "{what}");
        // The record is the setup's own and is kept.
        assert!(world.record().is_some(), "{what}");
        let replay = Scripted::new(vec![]);
        let replayed = run(&world, &replay, false);
        assert_eq!(replay.log(), Vec::<&str>::new(), "{what}");
        assert_eq!(replayed.code, code, "{what}");
        // A new request polls again.
        let next = Scripted::new(vec![Step::Saw(SetupPollOutcome::WaitingForMerge)]);
        let fresh = world.service.execute_binding(
            RequestId::new(),
            None,
            Mutation::PollOnce(PollOnceInput {
                root: world.root.clone(),
            })
            .into_binding(),
            &next,
        );
        assert_eq!(fresh.code, ResultCode::Ok, "{what}");
        assert_eq!(next.log(), ["poll_setup"], "{what}");
    }
}

/// A crash between a first-time setup's fast-forward and its finish leaves a
/// setup the local primary holds, in a registration that is not reconciled
/// yet. `poll once`, through the binding as production runs it, takes
/// today's poll (`path_of` gives none for a complete setup): its answer is the
/// poll's own, with no internal error, and the end of the poll finishes the
/// setup.
#[test]
fn a_poll_after_a_crash_between_the_fast_forward_and_the_finish_ends_the_setup() {
    let world = World::new(SetupKind::Enable, OTHER, true);
    let merged = world.record().unwrap().configuration;
    let tip = world.squash(&merged);
    world.pull(tip);
    crate::repository::remote::synthetic_fetch_error(SshTransportError {
        root: world.root.clone(),
        remote_name: OTHER.into(),
        direction: SshDirection::Fetch,
        selected_key_id: None,
        authority: None,
        kind: SshTransportErrorKind::TransportUnavailable,
    });
    let session = Scripted::new(vec![]);
    session.real_poll.set(true);

    let answer = run(&world, &session, false);

    assert_eq!(
        session.log(),
        ["poll_once"],
        "today's poll, not the setup poll"
    );
    assert_ne!(answer.code, ResultCode::InternalError, "{}", answer.message);
    assert_eq!(
        answer.code,
        ResultCode::TransportUnavailable,
        "{}",
        answer.message
    );
    assert_eq!(request_state(&world), Some(RequestState::Finished));
    // The end of the poll finished the setup, and its request.
    assert!(world.record().is_none());
    assert_eq!(world.request().state, RequestState::Finished);
    assert_eq!(world.local_ref(&world.branch_ref()), None);
    assert_eq!(
        world.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'"),
        0
    );
}
