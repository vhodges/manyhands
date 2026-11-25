//! The request lifecycle of a pull-request `repo enable` (Wave 06 Cycle 02,
//! design Part K, rules K1 to K7 and outcomes L1 to L14), driven through
//! the mutation boundary over a `Session` whose `publish_setup` is scripted,
//! so that no remote is reached and no key is needed.

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    panic::{AssertUnwindSafe, catch_unwind},
    path::{Path, PathBuf},
    sync::mpsc,
};

use git2::{Oid, Repository, RepositoryInitOptions, Signature, Time};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

use crate::{
    canonical::Integration,
    forge,
    repository::{
        CommitIdentity, ConfirmationId, Mutation, MutationDataDto, OperationResumeInput,
        PrepareDataDto, ReadError, RepoEnableInput, RepositoryError, RepositoryErrorKind,
        RepositoryOperation, RepositoryService, RequestId, RequestState,
        mutation::{command::Command, finish_setup_request, progress::Session},
        open_registry,
        promotion::run::RunObserver,
        setup::{
            publish::{Published, SetupError},
            record::{self, SetupRecord},
        },
        transport::{SshDirection, SshTransportError, SshTransportErrorKind},
    },
    results::{CheckpointEffect, Envelope, Outcome, PublicationEffect, ResultCode, WriteEffect},
};

use super::RepoBinding;
use crate::repository::mutation::{
    Accepted,
    bind::{Binding, CallContext, Entered, Standing},
};

const R1: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX1";
const R2: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX2";
const REMOTE_URL: &str = "ssh://git@example.invalid/team/repo.git";

fn ada() -> CommitIdentity {
    CommitIdentity {
        name: "Ada Example".into(),
        email: "ada@example.invalid".into(),
    }
}

fn grace() -> CommitIdentity {
    CommitIdentity {
        name: "Grace Example".into(),
        email: "grace@example.invalid".into(),
    }
}

/// What a scripted call does, and returns.
type Runs = dyn FnOnce(&RepositoryService, &Path, RequestId) -> Result<Published, SetupError>;

/// What `publish_setup` does at one call.
enum Step {
    /// The setup is published: what `continue_setup` returns once it has
    /// recorded the remote's branch.
    Published,
    /// The call stops with this.
    Fail(fn() -> SetupError),
    /// The session has no credentials: nothing was called.
    NoCredentials,
    /// Anything else: the call does `run` and returns what it does.
    Then(Box<Runs>),
}

/// A session whose publishing is scripted, and that counts its calls. A
/// call the script has no step for is a call the test did not expect.
struct Scripted {
    steps: RefCell<VecDeque<Step>>,
    calls: Cell<usize>,
}

impl Scripted {
    fn new(steps: Vec<Step>) -> Self {
        Self {
            steps: RefCell::new(steps.into()),
            calls: Cell::new(0),
        }
    }

    /// A session that must not be asked to publish.
    fn none() -> Self {
        Self::new(Vec::new())
    }

    fn calls(&self) -> usize {
        self.calls.get()
    }
}

/// The setup the registry holds for the repository at `root`.
fn read_setup(service: &RepositoryService, root: &Path) -> Option<SetupRecord> {
    let db = open_registry(&service.registry_path, &mut |_| {}).unwrap();
    let id: Option<i64> = db
        .query_row(
            "SELECT id FROM repositories WHERE root_path = ?1",
            [root.to_str().unwrap()],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    record::read(&db, id?).unwrap()
}

fn branch_of(record: &SetupRecord) -> String {
    format!("manyhands/setup/{}", record.setup_id)
}

/// What `continue_setup` returns for the recorded setup once its branch is
/// on the remote.
fn published(service: &RepositoryService, root: &Path) -> Published {
    let record = read_setup(service, root).expect("a recorded setup");
    Published {
        setup_id: record.setup_id.clone(),
        commit_oid: Oid::from_str(&"1".repeat(40)).unwrap(),
        link: forge::pull_request_link(None, &branch_of(&record), "main"),
    }
}

impl Session for Scripted {
    fn publish_setup(
        &self,
        service: &RepositoryService,
        root: &Path,
        request_id: RequestId,
    ) -> Option<Result<Published, SetupError>> {
        self.calls.set(self.calls.get() + 1);
        let step = self
            .steps
            .borrow_mut()
            .pop_front()
            .expect("publish_setup was called more often than the test expected");
        match step {
            Step::Published => Some(Ok(published(service, root))),
            Step::Fail(make) => Some(Err(make())),
            Step::NoCredentials => None,
            Step::Then(run) => Some(run(service, root, request_id)),
        }
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

    fn poll_once(
        &self,
        _: &RepositoryService,
        _: crate::repository::PollPublicationRemoteRequest,
        _: RequestId,
    ) -> Option<Result<crate::repository::RemotePollResult, crate::repository::RemotePollError>>
    {
        None
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
            crate::repository::RemotePollError,
        >,
    > {
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

    fn poll_setup(
        &self,
        _: &RepositoryService,
        _: &Path,
        _: RequestId,
    ) -> Option<Result<crate::repository::SetupPollReport, crate::repository::SetupPollError>> {
        None
    }
}

/// A repository on `main` with one commit, the remote `origin` at an
/// SSH-shaped address nothing contacts, and a service over a data
/// directory of its own, for a pull-request enable.
struct World {
    _local: tempfile::TempDir,
    _data: tempfile::TempDir,
    root: PathBuf,
    service: RepositoryService,
    head: Oid,
}

impl World {
    /// With an identity in the repository's own configuration.
    fn new() -> Self {
        Self::build(true)
    }

    /// With no identity anywhere: a request that wants to commit supplies
    /// its own.
    fn without_identity() -> Self {
        Self::build(false)
    }

    fn build(identity: bool) -> Self {
        let local = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let root = local.path().canonicalize().unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = Repository::init_opts(&root, &options).unwrap();
        if identity {
            let mut config = repository.config().unwrap();
            config.set_str("user.name", "Fixture").unwrap();
            config
                .set_str("user.email", "fixture@example.invalid")
                .unwrap();
        }
        std::fs::write(root.join("README.md"), "readme\n").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature =
            Signature::new("Fixture", "fixture@example.invalid", &Time::new(0, 0)).unwrap();
        let head = repository
            .commit(Some("HEAD"), &signature, &signature, "Base", &tree, &[])
            .unwrap();
        drop(tree);
        repository.remote("origin", REMOTE_URL).unwrap();
        let data = tempfile::tempdir().unwrap();
        // No test waits out the lease's real timeout.
        let service = RepositoryService::open_at(data.path())
            .unwrap()
            .with_lease_wait_for_testing(std::time::Duration::from_millis(1));
        Self {
            _local: local,
            _data: data,
            root,
            service,
            head,
        }
    }

    fn input(&self, identity: Option<CommitIdentity>) -> RepoEnableInput {
        RepoEnableInput {
            root: self.root.clone(),
            primary_branch: "main".into(),
            identity,
            slug_prefix: Some("mh".into()),
            publication_remote: Some("origin".into()),
            integration: Some(Integration::PullRequest),
            forge: None,
        }
    }

    fn mutation(&self, identity: Option<CommitIdentity>) -> Mutation {
        Mutation::RepoEnable(self.input(identity))
    }

    /// The confirmation `prepare` gives for the request.
    fn confirmation(&self, identity: Option<CommitIdentity>) -> ConfirmationId {
        let envelope = self.service.prepare(&self.mutation(identity));
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        match envelope.data {
            Some(PrepareDataDto::Preview(preview)) => {
                ConfirmationId::parse(&preview.confirmation_id).unwrap()
            }
            other => panic!("no preview: {other:?}"),
        }
    }

    /// One call of the request, through the boundary.
    fn run(
        &self,
        request: &str,
        confirmation: Option<ConfirmationId>,
        identity: Option<CommitIdentity>,
        session: &dyn Session,
    ) -> Envelope<MutationDataDto> {
        self.service.execute_binding(
            RequestId::parse(request).unwrap(),
            confirmation,
            self.mutation(identity).into_binding(),
            session,
        )
    }

    fn record(&self, request: &str) -> crate::repository::request_store::RequestRecord {
        self.service
            .request_record(RequestId::parse(request).unwrap())
            .unwrap()
            .expect("a request record")
    }

    fn setup(&self) -> Option<SetupRecord> {
        read_setup(&self.service, &self.root)
    }

    fn count(&self, sql: &str) -> i64 {
        open_registry(&self.service.registry_path, &mut |_| {})
            .unwrap()
            .query_row(sql, [], |row| row.get(0))
            .unwrap()
    }

    /// The step of the one local row of the enable, and its state.
    fn row(&self) -> (String, Option<String>) {
        open_registry(&self.service.registry_path, &mut |_| {})
            .unwrap()
            .query_row(
                "SELECT state, completed_step FROM operation_records WHERE action = 'enable'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }

    fn local_config(&self, key: &str) -> Option<String> {
        Repository::open(&self.root)
            .unwrap()
            .config()
            .unwrap()
            .open_level(git2::ConfigLevel::Local)
            .unwrap()
            .get_string(key)
            .ok()
    }

    /// The local primary, HEAD and the working tree are as they were: no
    /// configuration was written, nothing was committed.
    fn assert_local_primary_untouched(&self, what: &str) {
        let repository = Repository::open(&self.root).unwrap();
        assert_eq!(
            repository.head().unwrap().target(),
            Some(self.head),
            "{what}: HEAD moved"
        );
        assert_eq!(
            repository.refname_to_id("refs/heads/main").unwrap(),
            self.head,
            "{what}: the primary moved"
        );
        assert!(
            !self.root.join(".manyhands/config.toml").exists(),
            "{what}: a configuration was written: the direct path ran"
        );
        assert!(
            repository
                .references_glob("refs/heads/manyhands/setup/*")
                .unwrap()
                .next()
                .is_none(),
            "{what}: the enable made a setup branch itself"
        );
    }

    /// One registration, one record, one local row, one request: nothing
    /// was made twice. `setup` is the record as the first call made it.
    fn assert_one_setup(&self, setup: &SetupRecord, what: &str) {
        assert_eq!(self.count("SELECT COUNT(*) FROM repositories"), 1, "{what}");
        assert_eq!(
            self.count("SELECT COUNT(*) FROM repository_setups"),
            1,
            "{what}"
        );
        assert_eq!(
            self.count("SELECT COUNT(*) FROM operation_records WHERE action = 'enable'"),
            1,
            "{what}"
        );
        assert_eq!(
            self.count("SELECT COUNT(*) FROM request_records"),
            1,
            "{what}"
        );
        let now = self.setup().expect("the record is kept");
        assert_eq!(now.setup_id, setup.setup_id, "{what}: a second setup");
        assert_eq!(now.request_id, setup.request_id, "{what}");
        assert_eq!(now.created_at, setup.created_at, "{what}");
    }
}

fn data_of(envelope: &Envelope<MutationDataDto>) -> Value {
    serde_json::to_value(&envelope.data).unwrap()
}

/// The `setup` data of design Part I, for the record in `state`, as the
/// pull request link of an unrecognised remote and the primary `main` has it.
fn setup_data(record: &SetupRecord, state: &str) -> Value {
    let branch = branch_of(record);
    json!({
        "type": "setup",
        "id": record.setup_id.to_string(),
        "kind": "enable",
        "state": state,
        "request_id": record.request_id.to_string(),
        "branch": branch,
        "remote": "origin",
        "integration": "pull_request",
        "pull_request": {"source": branch, "target": "main", "forge": null, "url": null},
    })
}

fn transport(kind: SshTransportErrorKind) -> SetupError {
    SetupError::Transport(SshTransportError {
        root: PathBuf::from("/nowhere"),
        remote_name: "origin".into(),
        direction: SshDirection::Fetch,
        selected_key_id: None,
        authority: None,
        kind,
    })
}

// L1: a first call that publishes.
fn l1_a_first_call_that_publishes() {
    let w = World::new();
    let session = Scripted::new(vec![Step::Published]);
    let confirmation = w.confirmation(None);

    let answer = w.run(R1, Some(confirmation), None, &session);

    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(answer.outcome, Outcome::Success);
    assert_eq!(session.calls(), 1);
    let setup = w.setup().expect("the record is kept until the merge");
    assert_eq!(data_of(&answer), setup_data(&setup, "published"));
    // Part E step 4 and K7: written, no checkpoint, published, and no
    // commit: the setup commit is in the data, and never on the primary.
    assert_eq!(answer.effects.write, WriteEffect::Written);
    assert_eq!(answer.effects.checkpoint, CheckpointEffect::NotRequested);
    assert_eq!(answer.effects.publication, PublicationEffect::Published);
    assert_eq!(answer.effects.commit_oid, None);
    assert_eq!(w.record(R1).state, RequestState::Finished);
    assert_eq!(
        w.row(),
        ("completed".to_owned(), Some("setup_recorded".to_owned()))
    );
    w.assert_one_setup(&setup, "L1");
    w.assert_local_primary_untouched("L1");
    // The registration holds no configuration yet: `repo list` must not
    // read the setup as complete.
    assert_eq!(
        w.count("SELECT COUNT(*) FROM repositories WHERE config_blob_oid IS NULL"),
        1
    );
    // The intended file is the record's, and holds the prefix.
    let configuration = String::from_utf8(setup.configuration.clone()).unwrap();
    assert!(
        configuration.contains("format_version = 2"),
        "{configuration}"
    );
    assert!(configuration.contains("publication_remote = \"origin\""));
    assert!(
        configuration.contains(&format!("{} = \"mh\"", crate::canonical::SLUG_PREFIX_KEY)),
        "{configuration}"
    );
}

// L2: a first call that stops after it recorded, once for each answer K2
// leaves owed.
fn l2_each_owed_answer_keeps_the_request_and_the_record() {
    let rows: Vec<(&str, Step, ResultCode)> = vec![
        ("busy", Step::Fail(|| SetupError::Busy), ResultCode::Busy),
        (
            "recovery_required",
            Step::Fail(|| SetupError::RecoveryRequired),
            ResultCode::RecoveryRequired,
        ),
        (
            "the setup remote changed (S11)",
            Step::Fail(|| transport(SshTransportErrorKind::EndpointChanged)),
            ResultCode::RecoveryRequired,
        ),
        (
            "identity_required",
            Step::Fail(|| SetupError::IdentityRequired),
            ResultCode::IdentityRequired,
        ),
        (
            "no_selected_key",
            Step::Fail(|| transport(SshTransportErrorKind::NoSelectedKey)),
            ResultCode::NoSelectedKey,
        ),
        (
            "unlock_required",
            Step::Fail(|| transport(SshTransportErrorKind::UnlockCancelled)),
            ResultCode::UnlockRequired,
        ),
        (
            "transport_unavailable",
            Step::Fail(|| transport(SshTransportErrorKind::TransportUnavailable)),
            ResultCode::TransportUnavailable,
        ),
        (
            "a push that returned and left no branch",
            Step::Fail(|| SetupError::PushNotSeen),
            ResultCode::TransportUnavailable,
        ),
        (
            "push_rejected",
            Step::Fail(|| transport(SshTransportErrorKind::PushRejected)),
            ResultCode::PushRejected,
        ),
        (
            "a Git error",
            Step::Fail(|| {
                SetupError::Repository(RepositoryError::new(
                    RepositoryOperation::Enable,
                    None,
                    RepositoryErrorKind::Io,
                    "a read failed",
                ))
            }),
            crate::repository::mutation::repository_error_code(RepositoryErrorKind::Io),
        ),
        (
            "a registry error",
            Step::Fail(|| SetupError::Registry(ReadError::new(ResultCode::IndexUnavailable))),
            ResultCode::IndexUnavailable,
        ),
        (
            "a session with no credentials",
            Step::NoCredentials,
            ResultCode::InternalError,
        ),
    ];
    for (what, step, code) in rows {
        let w = World::new();
        let session = Scripted::new(vec![step]);
        let confirmation = w.confirmation(None);

        let answer = w.run(R1, Some(confirmation), None, &session);

        assert_eq!(answer.code, code, "{what}");
        // The registry rows and the record are durable changes no effect
        // shows: the request is partial, never a plain refusal.
        assert_eq!(answer.outcome, Outcome::Partial, "{what}");
        assert_eq!(session.calls(), 1, "{what}");
        let setup = w
            .setup()
            .unwrap_or_else(|| panic!("{what}: the record is kept"));
        // Every answer after recording carries the setup, a stop included.
        assert_eq!(data_of(&answer), setup_data(&setup, "recorded"), "{what}");
        // K2: owed. The request stays accepted for the same request to
        // continue; a settled or deleted request would free its ID for a
        // second setup.
        let record = w.record(R1);
        assert_eq!(record.state, RequestState::Accepted, "{what}");
        assert_eq!(record.result, None, "{what}");
        assert_eq!(
            w.row(),
            ("completed".to_owned(), Some("setup_recorded".to_owned())),
            "{what}"
        );
        w.assert_one_setup(&setup, what);
        w.assert_local_primary_untouched(what);
    }
}

// A host the setup's remote presents that is not trusted: the stop offers
// the approval of that host (ruling R-W), though the setup's data names none.
fn a_host_stop_offers_the_approval_of_its_host() {
    let w = World::new();
    let confirmation = w.confirmation(None);
    let session = Scripted::new(vec![Step::Fail(|| {
        SetupError::Transport(SshTransportError {
            root: PathBuf::from("/nowhere"),
            remote_name: "origin".into(),
            direction: SshDirection::Fetch,
            selected_key_id: None,
            authority: Some(crate::repository::transport::SshAuthority {
                host: "example.invalid".into(),
                port: 22,
            }),
            kind: SshTransportErrorKind::HostApprovalRequired {
                presented: crate::repository::transport::HostKeyIdentity {
                    algorithm: "ssh-ed25519".into(),
                    sha256: "SHA256:abc".into(),
                },
            },
        })
    })]);

    let answer = w.run(R1, Some(confirmation), None, &session);

    assert_eq!(answer.code, ResultCode::HostApprovalRequired);
    let actions = serde_json::to_value(&answer.recovery).unwrap();
    assert_eq!(actions[0]["action"], "host.approve", "{actions}");
    assert_eq!(actions[0]["arguments"]["authority"], "example.invalid:22");
    assert_eq!(actions[0]["arguments"]["root"], w.root.to_str().unwrap());
    // The same request is offered too, and the data names no host.
    assert!(actions.to_string().contains("request.retry"), "{actions}");
    assert_eq!(data_of(&answer)["type"], "setup");
    assert_eq!(w.record(R1).state, RequestState::Accepted);
}

// L3: the same request again, record present: the setup continues.
fn l3_the_same_request_again_continues_the_recorded_setup() {
    let w = World::new();
    let confirmation = w.confirmation(None);
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(
        w.run(R1, Some(confirmation), None, &stopped).code,
        ResultCode::Busy
    );
    let setup = w.setup().unwrap();

    // The re-entry presents no confirmation: the record's consent is the
    // request's, and `Reentry::of` takes the row as recorded (K1).
    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, None, None, &again);

    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(again.calls(), 1, "published once, by this call");
    assert_eq!(data_of(&answer), setup_data(&setup, "published"));
    assert_eq!(answer.effects.commit_oid, None);
    assert_eq!(w.record(R1).state, RequestState::Finished);
    // No second registration, record or setup branch.
    w.assert_one_setup(&setup, "L3");
    w.assert_local_primary_untouched("L3");

    // L7: the request again after it finished, or after its answer was
    // lost: the stored envelope, through `from_stored`, and nothing is
    // published, nothing checked against the repository.
    let lost = Scripted::none();
    let replayed = w.run(R1, None, None, &lost);
    assert_eq!(lost.calls(), 0);
    assert_eq!(replayed.code, answer.code);
    assert_eq!(replayed.outcome, answer.outcome);
    assert_eq!(replayed.effects, answer.effects);
    assert_eq!(data_of(&replayed), data_of(&answer));
    w.assert_one_setup(&setup, "L7");
}

// L3, the identity: a supplied identity is written again on re-entry, from
// the request's input.
fn l3_an_identity_that_is_missing_is_written_again_on_reentry() {
    let w = World::without_identity();
    let confirmation = w.confirmation(Some(ada()));
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(
        w.run(R1, Some(confirmation), Some(ada()), &stopped).code,
        ResultCode::Busy
    );
    assert_eq!(w.local_config("user.name").as_deref(), Some("Ada Example"));

    // As after a crash before the identity was written.
    let mut config = Repository::open(&w.root).unwrap().config().unwrap();
    config.remove("user.name").unwrap();
    config.remove("user.email").unwrap();
    assert_eq!(w.local_config("user.name"), None);

    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, None, Some(ada()), &again);

    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(w.local_config("user.name").as_deref(), Some("Ada Example"));
    assert_eq!(
        w.local_config("user.email").as_deref(),
        Some("ada@example.invalid")
    );
}

// L5, L4: a crash between the removal of the record and the request's
// settle: the removal finished the request in its own transaction.
fn l5_a_removal_finishes_the_request_in_its_own_transaction() {
    // What the ending of a refused setup (R-7) does, as `publish.rs` does
    // it: the request, then the record, then the registration of a first-time
    // setup, in one transaction.
    fn end(service: &RepositoryService, root: &Path, request: RequestId) {
        let record = read_setup(service, root).unwrap();
        let data =
            MutationDataDto::from_stored(Command::RepoEnable, &setup_data(&record, "recorded"))
                .expect("the setup data of a stored enable is read back");
        let mut db = open_registry(&service.registry_path, &mut |_| {}).unwrap();
        let id: i64 = db
            .query_row("SELECT id FROM repositories", [], |row| row.get(0))
            .unwrap();
        let tx = db.transaction().unwrap();
        assert!(
            finish_setup_request(
                &tx,
                id,
                request,
                ResultCode::RemoteAlreadySetUp,
                Some(data),
                1_700_000_000,
            )
            .unwrap()
        );
        record::delete(&tx, id).unwrap();
        tx.execute("DELETE FROM repositories WHERE id = ?1", [id])
            .unwrap();
        tx.commit().unwrap();
    }

    // (a) The call answers what ended the setup.
    let w = World::new();
    let confirmation = w.confirmation(None);
    let setup_before = {
        let session = Scripted::new(vec![Step::Then(Box::new(|service, root, request| {
            end(service, root, request);
            Err(SetupError::RemoteAlreadySetUp)
        }))]);
        let answer = w.run(R1, Some(confirmation), None, &session);
        // The request was finished in the removal's transaction, so the
        // answer is the stored envelope, with the setup in its data.
        assert_eq!(answer.code, ResultCode::RemoteAlreadySetUp);
        assert_eq!(data_of(&answer)["type"], "setup");
        assert_eq!(data_of(&answer)["state"], "recorded");
        assert_eq!(w.record(R1).state, RequestState::Finished);
        assert_eq!(w.setup().map(|_| ()), None);
        assert_eq!(w.count("SELECT COUNT(*) FROM repositories"), 0);
        // Settling a request the domain finished is not a second answer.
        assert_eq!(
            w.record(R1).result.as_ref().unwrap().code,
            ResultCode::RemoteAlreadySetUp
        );
        answer
    };
    // The same request again: the stored envelope, nothing run (L4).
    let none = Scripted::none();
    let replayed = w.run(R1, None, None, &none);
    assert_eq!(none.calls(), 0);
    assert_eq!(replayed.code, setup_before.code);
    assert_eq!(replayed.effects, setup_before.effects);
    assert_eq!(data_of(&replayed), data_of(&setup_before));

    // (b) The process dies after the removal and before the settle (L5).
    let w = World::new();
    let confirmation = w.confirmation(None);
    let session = Scripted::new(vec![Step::Then(Box::new(|service, root, request| {
        end(service, root, request);
        panic!("the process died after the removal")
    }))]);
    let crashed = catch_unwind(AssertUnwindSafe(|| {
        w.run(R1, Some(confirmation), None, &session)
    }));
    assert!(crashed.is_err());
    // Nothing settled it: it was finished in the removal's transaction.
    let record = w.record(R1);
    assert_eq!(record.state, RequestState::Finished);
    let none = Scripted::none();
    let answer = w.run(R1, None, None, &none);
    assert_eq!(none.calls(), 0);
    assert_eq!(answer.code, ResultCode::RemoteAlreadySetUp);
    assert_eq!(data_of(&answer)["type"], "setup");
    // The stored envelope names no commit: nothing is checked against the
    // repository (K7), and the data is read back by `from_stored`.
    assert_eq!(answer.effects.commit_oid, None);
}

// L6: a crash before the recording transaction, then the same request: the
// pending row has no step, and the request runs afresh as a setup request.
fn l6_a_crash_before_recording_runs_afresh_as_a_setup_request() {
    let w = World::new();
    let confirmation = w.confirmation(None);
    // The recording path calls the registration hook before it writes, as
    // `reconcile_registration` does.
    w.service
        .set_registration_git_hook_for_testing(|| panic!("the process died before recording"));
    let session = Scripted::none();
    let crashed = catch_unwind(AssertUnwindSafe(|| {
        w.run(R1, Some(confirmation), None, &session)
    }));
    assert!(crashed.is_err());
    assert_eq!(session.calls(), 0);
    assert_eq!(w.record(R1).state, RequestState::Accepted);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
    assert_eq!(w.count("SELECT COUNT(*) FROM repositories"), 0);
    let (state, step) = w.row();
    assert_ne!(state, "completed");
    assert_eq!(step, None);

    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, None, None, &again);

    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(again.calls(), 1);
    let setup = w.setup().expect("recorded by the second call");
    assert_eq!(data_of(&answer), setup_data(&setup, "published"));
    assert_eq!(
        w.row(),
        ("completed".to_owned(), Some("setup_recorded".to_owned()))
    );
    w.assert_one_setup(&setup, "L6");
    // Not the direct path: no configuration file, no initialization commit.
    w.assert_local_primary_untouched("L6");
}

// L10 and L11: two calls at once. One thread is held inside the first call's
// recording, with the repository lease and its row pending; the second call
// meets the lease and answers `busy`, writes nothing and records nothing
// (a repository that is not registered yet has no live holder to wait for).
// Once the first has gone on, the second goes on from its own request or is
// refused for the first's record, and nothing is made twice.
fn races(same_request: bool) {
    let (entered_tx, entered_rx) = mpsc::channel::<()>();
    let (proceed_tx, proceed_rx) = mpsc::channel::<()>();
    let w = World::without_identity();
    let first_identity = Some(ada());
    let second_identity = if same_request {
        Some(ada())
    } else {
        Some(grace())
    };
    let first = w.confirmation(first_identity.clone());
    let second = if same_request {
        None
    } else {
        Some(w.confirmation(second_identity.clone()))
    };
    w.service.set_registration_git_hook_for_testing(move || {
        entered_tx.send(()).unwrap();
        proceed_rx.recv().unwrap();
    });

    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn({
            let w = &w;
            let identity = first_identity.clone();
            move || {
                let session = Scripted::new(vec![Step::Published]);
                w.run(R1, Some(first), identity, &session)
            }
        });
        // The first call is inside its recording, holding the lease.
        entered_rx.recv().unwrap();
        let session = Scripted::none();
        let b = if same_request {
            w.run(R1, None, second_identity.clone(), &session)
        } else {
            w.run(R2, second, second_identity.clone(), &session)
        };
        assert_eq!(session.calls(), 0);
        // Nothing of the second was written: no registration, no record, and
        // the identity it supplied is nowhere.
        assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
        assert_eq!(w.count("SELECT COUNT(*) FROM repositories"), 0);
        assert_eq!(w.local_config("user.name"), None);
        proceed_tx.send(()).unwrap();
        (a.join().unwrap(), b)
    });
    assert_eq!(b.code, ResultCode::Busy, "{}", b.message);
    assert_eq!(a.code, ResultCode::Ok, "{}", a.message);
    let setup = w.setup().expect("one record");
    assert_eq!(setup.request_id, RequestId::parse(R1).unwrap());
    // The first call's supplied identity is the one that stands.
    assert_eq!(w.local_config("user.name").as_deref(), Some("Ada Example"));
    w.assert_local_primary_untouched(if same_request { "L11" } else { "L10" });
    if same_request {
        // L11: the second call entered the request, so the first call's
        // settle did not apply: the request is still accepted, and the
        // second call, sent again, finds its own record and continues. Both
        // answer alike, and one setup branch is published.
        assert_eq!(w.record(R1).state, RequestState::Accepted);
        let again = Scripted::new(vec![Step::Published]);
        let b = w.run(R1, None, second_identity, &again);
        assert_eq!(b.code, ResultCode::Ok, "{}", b.message);
        assert_eq!(data_of(&a), data_of(&b));
        assert_eq!(data_of(&b), setup_data(&setup, "published"));
        assert_eq!(w.record(R1).state, RequestState::Finished);
        w.assert_one_setup(&setup, "L11");
    } else {
        // L10: the second request, sent again, is refused for the first's
        // record before it is accepted, and the identity it supplied was
        // never written.
        let again = Scripted::none();
        let refused = w.run(R2, None, second_identity, &again);
        assert_eq!(refused.code, ResultCode::SetupPending);
        assert_eq!(again.calls(), 0);
        assert_eq!(
            w.local_config("user.email").as_deref(),
            Some("ada@example.invalid")
        );
        assert_eq!(w.count("SELECT COUNT(*) FROM request_records"), 1);
        w.assert_one_setup(&setup, "L10");
    }
    assert_eq!(
        w.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'"),
        0,
        "no operation is left in flight"
    );
}

// L10, the domain's own check (K1): a request that gets past the gate while
// another's setup is recorded, as in the window between the gate's look and
// the lease, is refused for that record, finally, with its row closed, its
// identity unwritten and nothing published.
fn l10_another_requests_record_refuses_the_domain_call() {
    let w = World::without_identity();
    let confirmation = w.confirmation(Some(ada()));
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(
        w.run(R1, Some(confirmation), Some(ada()), &stopped).code,
        ResultCode::Busy
    );
    let setup = w.setup().unwrap();

    let input = w.input(Some(grace()));
    let intended = crate::repository::setup::intent::for_enable(
        "main",
        Some("origin"),
        Some(Integration::PullRequest),
        None,
        Some(REMOTE_URL),
    )
    .unwrap();
    let outcome = w
        .service
        .enable_with_setup(
            crate::repository::EnableRepositoryRequest {
                root: w.root.clone(),
                primary_branch: "main".into(),
                identity: input.identity.clone(),
                operation_id: crate::repository::OperationId::new(),
            },
            crate::repository::setup::enable::SetupRequest {
                request_id: RequestId::parse(R2).unwrap(),
                prefix: None,
                remote_name: "origin".into(),
                intended: Some(intended),
                recorded: false,
            },
            &|_, _| panic!("another's setup is not published by this request"),
        )
        .unwrap();

    assert!(matches!(
        outcome,
        crate::repository::setup::enable::SetupOutcome::AnotherSetup
    ));
    assert_eq!(w.local_config("user.name").as_deref(), Some("Ada Example"));
    assert_eq!(w.count("SELECT COUNT(*) FROM repositories"), 1);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 1);
    assert_eq!(w.setup().unwrap().setup_id, setup.setup_id);
    assert_eq!(w.setup().unwrap().request_id, setup.request_id);
    assert_eq!(
        w.count("SELECT COUNT(*) FROM operation_records WHERE state != 'completed'"),
        0
    );
}

// L11, K2, R-20: a call of a request that read its row pending, and answers
// after another call of the same request recorded the setup. Its domain call
// is refused, by a check, but the request is recorded by now: it is owed,
// with the setup, and the same request goes on. Settled as a refusal before
// the recording, from the row it reads after the call, it would be deleted,
// and its ID freed for a second setup. (A call that waits for the lease on a
// root registered beforehand has the same standing; this is that call's
// answer, made after the recording, with no race to win.)
fn l11_a_call_refused_after_another_recorded_the_setup_leaves_the_request_owed() {
    let w = World::new();
    let confirmation = w.confirmation(None);
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(
        w.run(R1, Some(confirmation), None, &stopped).code,
        ResultCode::Busy
    );
    let setup = w.setup().unwrap();
    // The worktree is not clean by the time the late call makes its checks.
    let dirt = w.root.join("untracked.txt");
    std::fs::write(&dirt, "not clean\n").unwrap();
    let record = w.record(R1);
    let accepted = Accepted::of_record(&record, record.attempt).unwrap();
    let binding = RepoBinding::from(w.input(None));
    let session = Scripted::none();
    let call = CallContext {
        service: &w.service,
        session: &session,
        confirmation: None,
    };
    let intended = crate::repository::setup::intent::for_enable(
        "main",
        Some("origin"),
        Some(Integration::PullRequest),
        None,
        Some(REMOTE_URL),
    )
    .unwrap();

    // It made its domain call as a first one: `recorded` was false.
    let ran = binding.call_setup(&call, &accepted, Some(&intended), false);

    assert_eq!(ran.answer.code, ResultCode::WorktreeNotClean);
    assert!(matches!(ran.standing, Standing::Owed));
    assert_eq!(
        serde_json::to_value(&ran.answer.data).unwrap(),
        setup_data(&setup, "recorded")
    );
    assert_eq!(w.record(R1).state, RequestState::Accepted);
    // The same request goes on from its record, once the worktree is as it was.
    std::fs::remove_file(dirt).unwrap();
    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, None, None, &again);
    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    assert_eq!(w.record(R1).state, RequestState::Finished);
    assert_eq!(data_of(&answer), setup_data(&setup, "published"));
    w.assert_one_setup(&setup, "L11 refused");
    w.assert_local_primary_untouched("L11 refused");
}

// R-Y: a binding that no observation gave a route is not run as a direct
// enable. A request whose row is gone or has no step of its own, entered
// again by `rerun` with no route, would otherwise commit "Initialize
// Manyhands" on the primary.
fn ry_a_binding_with_no_route_is_never_run_as_a_direct_enable() {
    let w = World::new();
    let confirmation = w.confirmation(None);
    w.service
        .set_registration_git_hook_for_testing(|| panic!("the process died before recording"));
    let session = Scripted::none();
    let crashed = catch_unwind(AssertUnwindSafe(|| {
        w.run(R1, Some(confirmation), None, &session)
    }));
    assert!(crashed.is_err());
    let record = w.record(R1);
    let accepted = Accepted::of_record(&record, record.attempt).unwrap();
    let binding = RepoBinding::from(w.input(None));
    let call = CallContext {
        service: &w.service,
        session: &session,
        confirmation: None,
    };

    let ran = binding.rerun(&call, &accepted, &Entered::Unfinished);

    assert_eq!(ran.answer.code, ResultCode::RecoveryRequired);
    assert!(matches!(ran.standing, Standing::Owed));
    w.assert_local_primary_untouched("R-Y");
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 0);
    assert_eq!(w.record(R1).state, RequestState::Accepted);
}

// L13: `operation resume` of a setup request's operation.
fn l13_resume_names_the_original_request() {
    let w = World::new();
    let confirmation = w.confirmation(None);
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    let first = w.run(R1, Some(confirmation), None, &stopped);
    let operation = first.operation_id.clone().expect("the request's operation");

    let resumed = w.service.execute_binding(
        RequestId::parse(R2).unwrap(),
        None,
        Mutation::OperationResume(OperationResumeInput {
            root: w.root.clone(),
            operation_id: crate::repository::OperationId::parse(&operation).unwrap(),
        })
        .into_binding(),
        &Scripted::none(),
    );

    assert_eq!(resumed.code, ResultCode::OriginalRequestRequired);
    assert!(
        resumed
            .recovery
            .iter()
            .any(|action| serde_json::to_string(action).unwrap().contains(R1)),
        "the recovery names the request: {:?}",
        resumed.recovery
    );
    // Nothing was run: the setup is as it was.
    assert_eq!(w.record(R1).state, RequestState::Accepted);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 1);
}

// L14: a registry read that fails on re-entry is owed: it never falls
// through to the direct path, and no second setup is made.
fn l14_an_unreadable_record_is_owed_not_the_direct_path() {
    let w = World::new();
    let confirmation = w.confirmation(None);
    let stopped = Scripted::new(vec![Step::Fail(|| SetupError::Busy)]);
    assert_eq!(
        w.run(R1, Some(confirmation), None, &stopped).code,
        ResultCode::Busy
    );
    let setup = w.setup().unwrap();
    // A row this build cannot read: the table's checks are what keep one out.
    {
        let db = open_registry(&w.service.registry_path, &mut |_| {}).unwrap();
        db.execute_batch(
            "PRAGMA ignore_check_constraints = ON;
             UPDATE repository_setups SET kind = 'unreadable';
             PRAGMA ignore_check_constraints = OFF;",
        )
        .unwrap();
    }

    let session = Scripted::none();
    let answer = w.run(R1, None, None, &session);

    assert_ne!(answer.code, ResultCode::Ok);
    assert_eq!(session.calls(), 0);
    let record = w.record(R1);
    assert_eq!(record.state, RequestState::Accepted, "owed, not settled");
    assert_eq!(record.result, None);
    assert_eq!(w.count("SELECT COUNT(*) FROM repositories"), 1);
    assert_eq!(w.count("SELECT COUNT(*) FROM repository_setups"), 1);
    assert_eq!(
        w.row(),
        ("completed".to_owned(), Some("setup_recorded".to_owned()))
    );
    w.assert_local_primary_untouched("L14");

    // Read again, it continues.
    open_registry(&w.service.registry_path, &mut |_| {})
        .unwrap()
        .execute("UPDATE repository_setups SET kind = 'enable'", [])
        .unwrap();
    let again = Scripted::new(vec![Step::Published]);
    let answer = w.run(R1, None, None, &again);
    assert_eq!(answer.code, ResultCode::Ok, "{}", answer.message);
    w.assert_one_setup(&setup, "L14");
}

/// Design Part K for `repo enable` in pull-request mode: the standing of
/// each answer, the route a re-entered request takes, what it answers and
/// that no second registration, record or setup branch is made.
#[test]
fn the_enable_request_lifecycle_of_design_part_k() {
    l1_a_first_call_that_publishes();
    l2_each_owed_answer_keeps_the_request_and_the_record();
    a_host_stop_offers_the_approval_of_its_host();
    l3_the_same_request_again_continues_the_recorded_setup();
    l3_an_identity_that_is_missing_is_written_again_on_reentry();
    l5_a_removal_finishes_the_request_in_its_own_transaction();
    l6_a_crash_before_recording_runs_afresh_as_a_setup_request();
    races(false);
    races(true);
    l10_another_requests_record_refuses_the_domain_call();
    l11_a_call_refused_after_another_recorded_the_setup_leaves_the_request_owed();
    ry_a_binding_with_no_route_is_never_run_as_a_direct_enable();
    l13_resume_names_the_original_request();
    l14_an_unreadable_record_is_owed_not_the_direct_path();
}
