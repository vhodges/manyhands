//! `operation abandon`: a pending local operation is closed, with
//! nothing else touched, and the request that began it released.
//!
//! Each of the three sequences that strand an operation ends here with
//! another operation on the repository succeeding, and with every file,
//! reference and worktree byte-identical to before the abandon.

use manyhands::repository::{
    OperationAbandonInput, OperationAbandonPreviewDto, OperationAction, OperationId,
    RefreshRepositoryRequest, RelationshipWrite, RemoteOperationTarget, RemoteRefPlan,
    RemoteReservationOutcome, RepositoryErrorKind, TicketSaveInput,
};

use super::{
    support::{
        mutation::{BODY, TICKET_NEW, TITLE, operation_data},
        repository_and_worktree_snapshot,
    },
    *,
};

const REQUEST_4: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX4";
const REQUEST_5: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX5";

const STRANDED_ROOT: &str = "MANYHANDS_STRANDED_ROOT";
const STRANDED_DATA: &str = "MANYHANDS_STRANDED_DATA";
const STRANDED_TOKEN: &str = "MANYHANDS_STRANDED_TOKEN";

/// The save a killed child makes: the open ticket, with a new title and
/// body.
fn save_at(root: &Path, token: &str) -> Mutation {
    Mutation::TicketSave(TicketSaveInput {
        root: root.to_owned(),
        id: items::item_id(TICKET_A),
        draft: World::draft(TITLE, BODY),
        deps: RelationshipWrite::Unchanged,
        parent: RelationshipWrite::Unchanged,
        observation: token.to_owned(),
    })
}

/// The process `stranded_save` starts: it submits the save as
/// `REQUEST_1`, and ends after the ticket's file is written and before
/// that step is recorded, with nothing after it run.
#[test]
fn stranded_save_child() {
    let Some(root) = std::env::var_os(STRANDED_ROOT) else {
        return;
    };
    let service = RepositoryService::open_at_with_exit_point_for_testing(
        Path::new(&std::env::var_os(STRANDED_DATA).unwrap()),
        FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence,
    )
    .unwrap();
    execute_with(
        &service,
        REQUEST_1,
        save_at(Path::new(&root), &std::env::var(STRANDED_TOKEN).unwrap()),
    );
    // Not reached: the process ended at the fault point.
}

/// A save of the open ticket whose process was killed after it wrote the
/// ticket's file. Its record is accepted and its operation pending.
struct Stranded {
    world: World,
    /// The token the killed save was made with.
    token: String,
    /// The operation the killed save left pending.
    operation_id: String,
}

impl Stranded {
    fn new() -> Self {
        let mut world = World::new();
        let token = world.token(TICKET_A);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "abandon::stranded_save_child", "--nocapture"])
            .env(STRANDED_ROOT, &world.root)
            .env(STRANDED_DATA, world.data.path())
            .env(STRANDED_TOKEN, &token)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(
            status.code(),
            Some(manyhands::repository::FAILURE_POINT_EXIT_STATUS),
            "the child ended at the fault point"
        );
        world.reopen();
        let record = world.record(REQUEST_1).expect("the record stays");
        assert_eq!(record.state, RequestState::Accepted);
        let operation_id = record.operations[0].operation_id.to_string();
        assert_eq!(world.pending_journal_rows(), 1);
        assert!(world.worktree_source(TICKET_A).contains(TITLE));
        assert_eq!(world.branch_commits(TICKET_A), 0);
        Self {
            world,
            token,
            operation_id,
        }
    }

    fn abandon(&self) -> Mutation {
        abandon(&self.world.root, &self.operation_id)
    }

    /// The path of the ticket's file, as an abandon names it.
    fn written_path(&self) -> String {
        format!(
            ".manyhands/worktrees/{TICKET_A}/{}",
            items::ticket_path(TICKET_A)
        )
    }

    fn confirmations(&self) -> i64 {
        items::index(self.world.data.path())
            .query_row("SELECT COUNT(*) FROM confirmation_records", [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    /// A preview that is given, and the confirmation it gives.
    fn preview(&self, mutation: &Mutation) -> (String, PreviewDto) {
        let envelope = prepare_with(&self.world.service, mutation);
        assert_eq!(envelope.code, ResultCode::Ok, "{}", envelope.message);
        assert_eq!(envelope.outcome, Outcome::Success);
        let preview = envelope.data.expect("a preview");
        (preview.confirmation_id.clone(), preview)
    }

    fn execute(&self, request_id: &str, confirmation: &str) -> Envelope<MutationDataDto> {
        execute_confirmed(
            &self.world.service,
            request_id,
            confirmation,
            self.abandon(),
        )
    }

    /// The journal's rows, as they are stored.
    fn journal(&self) -> Vec<(String, String, Option<String>)> {
        journal(self.world.data.path())
    }
}

fn journal(data: &Path) -> Vec<(String, String, Option<String>)> {
    let connection = items::index(data);
    let mut statement = connection
        .prepare("SELECT operation_ulid, state, completed_step FROM operation_records ORDER BY id")
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn abandon(root: &Path, operation_id: &str) -> Mutation {
    Mutation::OperationAbandon(OperationAbandonInput {
        root: root.to_owned(),
        operation_id: OperationId::parse(operation_id).unwrap(),
    })
}

fn abandon_preview(preview: &PreviewDto) -> &OperationAbandonPreviewDto {
    match &preview.details {
        PreviewDetailsDto::OperationAbandon(details) => details,
        other => panic!("not a preview of an abandon: {other:?}"),
    }
}

/// What another request on the repository is answered with while the
/// operation is pending: it is refused, and offered the abandon of the
/// operation that is in its way.
fn assert_blocked_and_offered_the_abandon(stranded: &Stranded, request_id: &str) {
    let world = &stranded.world;
    let blocked = world.execute(request_id, world.create(TICKET_NEW));
    assert_eq!(blocked.code, ResultCode::RecoveryRequired);
    assert_eq!(blocked.outcome, Outcome::Blocked);
    assert_eq!(
        recovery_actions(&blocked),
        ["repo.inspect", "operation.abandon"]
    );
    let offered = &blocked.recovery[1];
    assert_eq!(
        offered.operation_id.as_deref(),
        Some(stranded.operation_id.as_str())
    );
    assert_eq!(
        offered.arguments.get("root").and_then(|root| root.as_str()),
        world.root.to_str()
    );
    assert_eq!(world.record(request_id), None);
    assert!(!world.root.join(items::ticket_path(TICKET_NEW)).exists());
}

/// The abandon of the stranded save, confirmed as `request_id`: it
/// succeeds, shows no effect, names what was left and releases the
/// save's request, and no file, reference or worktree differs.
fn assert_abandoned(stranded: &Stranded, request_id: &str, left: &[String]) {
    let world = &stranded.world;
    let before = repository_and_worktree_snapshot(&world.fixture);
    let (confirmation, preview) = stranded.preview(&stranded.abandon());
    let details = abandon_preview(&preview);
    assert_eq!(details.paths, left);

    let abandoned = stranded.execute(request_id, &confirmation);

    assert_eq!(abandoned.code, ResultCode::Ok, "{}", abandoned.message);
    assert_eq!(abandoned.outcome, Outcome::Success);
    assert_eq!(abandoned.effects, Effects::not_requested());
    // The request has no operation of its own: the one it closed is in
    // its data.
    assert_eq!(abandoned.operation_id, None);
    assert_eq!(recovery_actions(&abandoned), [] as [&str; 0]);
    let data = operation_data(&abandoned);
    assert_eq!(data.operation_id, stranded.operation_id);
    assert_eq!(data.action, Some(OperationAction::SaveTicket));
    assert_eq!(data.item_id.as_deref(), Some(TICKET_A));
    assert_eq!(
        data.worktree,
        Some(format!(".manyhands/worktrees/{TICKET_A}"))
    );
    assert_eq!(data.paths, left);
    assert!(data.paths_complete);
    assert_eq!(data.released_request_id.as_deref(), Some(REQUEST_1));

    assert_eq!(
        before,
        repository_and_worktree_snapshot(&world.fixture),
        "the abandon touched a file, a reference or a worktree"
    );
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(world.record(REQUEST_1), None, "the save's request");
    assert_eq!(
        world.record(request_id).map(|record| record.state),
        Some(RequestState::Finished)
    );
}

// ---------------------------------------------------------------------
// The three sequences
// ---------------------------------------------------------------------

/// Open case 1, lost input: the caller no longer has the body the save
/// was made with, so nothing can repeat it.
#[test]
fn a_save_whose_input_is_lost_is_abandoned_and_the_repository_goes_on() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    assert_blocked_and_offered_the_abandon(&stranded, REQUEST_2);

    assert_abandoned(&stranded, REQUEST_3, &[stranded.written_path()]);

    // A save of another item succeeds at once.
    let other = world.execute(REQUEST_2, world.create(TICKET_NEW));
    assert_eq!(other.code, ResultCode::Ok, "{}", other.message);

    // A save of the same item, by what its caller read before: the file
    // on disk is not what that caller observed.
    let stale = world.execute(REQUEST_4, world.save(TICKET_A, &stranded.token));
    assert_eq!(stale.code, ResultCode::ExternalChange);
    assert_eq!(world.record(REQUEST_4), None);
    // The request ID the abandon released is free: the same request under
    // it runs as new, and is answered as the one before it.
    let again = world.execute(REQUEST_1, world.save(TICKET_A, &stranded.token));
    assert_eq!(again.code, ResultCode::ExternalChange);
    assert_ne!(
        again.operation_id.as_deref(),
        Some(stranded.operation_id.as_str())
    );
    assert_eq!(world.record(REQUEST_1), None);
    assert_eq!(world.branch_commits(TICKET_A), 0);

    // A fresh read and save commits the file, once.
    let fresh = world.token(TICKET_A);
    assert_ne!(fresh, stranded.token);
    let saved = world.execute(REQUEST_1, world.save(TICKET_A, &fresh));
    assert_eq!(saved.code, ResultCode::Ok, "{}", saved.message);
    assert_eq!(saved.effects.checkpoint, CheckpointEffect::Committed);
    assert_ne!(
        saved.operation_id.as_deref(),
        Some(stranded.operation_id.as_str())
    );
    assert_eq!(world.branch_commits(TICKET_A), 1);
    let replayed = world.execute(REQUEST_1, world.save(TICKET_A, &fresh));
    assert_eq!(replayed.effects.commit_oid, saved.effects.commit_oid);
    assert_eq!(world.branch_commits(TICKET_A), 1);
    assert_eq!(world.pending_journal_rows(), 0);
}

/// Open case 1, overtaken: someone else has since committed other
/// content to the file, so the same save is refused for good.
#[test]
fn a_save_that_was_overtaken_is_abandoned_and_the_repository_goes_on() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    world.commit_in_context(
        TICKET_A,
        &items::ticket_path(TICKET_A),
        &items::ticket_source(TICKET_A, "Someone else's title", ""),
    );
    // The same request, with the same input, cannot clear its row.
    let refused = world.execute(REQUEST_1, world.save(TICKET_A, &stranded.token));
    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(world.pending_journal_rows(), 1);
    assert_blocked_and_offered_the_abandon(&stranded, REQUEST_2);

    // Nothing of the item's differs from its branch: the other commit
    // replaced what the save wrote.
    assert_abandoned(&stranded, REQUEST_3, &[]);
    assert_eq!(world.branch_commits(TICKET_A), 1);

    let other = world.execute(REQUEST_2, world.create(TICKET_NEW));
    assert_eq!(other.code, ResultCode::Ok, "{}", other.message);

    let stale = world.execute(REQUEST_4, world.save(TICKET_A, &stranded.token));
    assert_eq!(stale.code, ResultCode::ExternalChange);
    assert_eq!(world.record(REQUEST_4), None);
    assert_eq!(world.branch_commits(TICKET_A), 1);

    let fresh = world.token(TICKET_A);
    let saved = world.execute(REQUEST_4, world.save(TICKET_A, &fresh));
    assert_eq!(saved.code, ResultCode::Ok, "{}", saved.message);
    assert_eq!(saved.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(world.branch_commits(TICKET_A), 2);
    assert!(world.worktree_source(TICKET_A).contains(TITLE));
    assert_eq!(world.pending_journal_rows(), 0);
}

/// Open case 2: an enablement killed after it wrote the configuration
/// file leaves the worktree dirty, and its own repeat is refused for
/// that.
#[test]
fn an_enablement_that_left_its_configuration_file_is_abandoned_and_the_file_is_named() {
    let mut desk = Desk::new();
    let plain = Plain::new();
    let enable = || killed_enable(&plain.root);
    let (confirmation, _) = desk.preview(&enable());
    killed_at(
        &desk,
        &plain.root,
        true,
        &confirmation,
        FailurePoint::BeforeInitializationCommit,
    );
    desk.reopen();
    assert!(plain.configuration().is_some(), "the file is written");
    assert_eq!(plain.initializations(), 0);
    assert_eq!(desk.pending_journal_rows(), 1);
    let operation_id = desk.record(REQUEST_1).unwrap().operations[0]
        .operation_id
        .to_string();

    // The same request is refused for the file its own process wrote,
    // for as long as the file is there. Its operation has recorded the
    // step, so it is the domain that refuses it.
    let refused = desk.execute(REQUEST_1, &confirmation, enable());
    assert_eq!(refused.code, ResultCode::WorktreeNotClean);
    assert_eq!(recovery_actions(&refused), ["request.retry"]);
    assert_eq!(desk.pending_journal_rows(), 1);
    // The result names the operation that is in the way.
    assert_eq!(refused.operation_id.as_deref(), Some(operation_id.as_str()));

    let before = tree(&plain.root);
    let (abandon_confirmation, preview) = desk.preview(&abandon(&plain.root, &operation_id));
    let details = abandon_preview(&preview);
    assert_eq!(details.action, Some(OperationAction::Enable));
    assert_eq!(details.worktree.as_deref(), Some("."));
    assert_eq!(details.paths, [canonical::CONFIG_PATH]);
    assert_eq!(details.request_id.as_deref(), Some(REQUEST_1));

    let abandoned = desk.execute(
        REQUEST_2,
        &abandon_confirmation,
        abandon(&plain.root, &operation_id),
    );

    assert_eq!(abandoned.code, ResultCode::Ok, "{}", abandoned.message);
    let data = operation_data(&abandoned);
    // The file is still there and still uncommitted, and is named.
    assert_eq!(data.paths, [canonical::CONFIG_PATH]);
    assert_eq!(data.item_id, None);
    assert_eq!(data.released_request_id.as_deref(), Some(REQUEST_1));
    assert_eq!(before, tree(&plain.root), "the abandon touched a file");
    assert!(plain.configuration().is_some());
    assert_eq!(desk.pending_journal_rows(), 0);
    assert_eq!(desk.record(REQUEST_1), None);
    assert_eq!(desk.accepted_confirmations(), 1, "the abandon's own");

    // Restored with Git, the repository is enabled by the same request
    // ID, which is free, with a new preview.
    let head = plain
        .fixture
        .repository
        .head()
        .unwrap()
        .peel(git2::ObjectType::Commit)
        .unwrap();
    plain
        .fixture
        .repository
        .reset(&head, git2::ResetType::Hard, None)
        .unwrap();
    let _ = fs::remove_file(plain.root.join(canonical::CONFIG_PATH));
    let (confirmation, _) = desk.preview(&enable());
    let enabled = desk.execute(REQUEST_1, &confirmation, enable());
    assert_enabled(&enabled, &plain.root);
    assert_ne!(enabled.operation_id.as_deref(), Some(operation_id.as_str()));
    assert_eq!(plain.initializations(), 1);
    assert!(desk.registered(&plain.root));
}

// ---------------------------------------------------------------------
// The preview and the confirmation
// ---------------------------------------------------------------------

#[test]
fn a_preview_of_an_abandon_shows_what_the_operation_left_and_changes_nothing() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    let before = repository_and_worktree_snapshot(&world.fixture);
    let journal = stranded.journal();

    let (_, preview) = stranded.preview(&stranded.abandon());

    assert_eq!(preview.command, "operation abandon");
    assert_eq!(
        effects(&preview),
        [
            PreviewEffect::OperationClosed,
            PreviewEffect::RequestReleased
        ]
    );
    let details = abandon_preview(&preview);
    assert_eq!(details.root, world.root.to_str().unwrap());
    assert_eq!(details.operation_id, stranded.operation_id);
    assert_eq!(details.action, Some(OperationAction::SaveTicket));
    assert_eq!(details.item_id.as_deref(), Some(TICKET_A));
    assert_eq!(details.paths, [stranded.written_path()]);
    assert!(details.paths_complete);
    assert_eq!(details.request_id.as_deref(), Some(REQUEST_1));
    assert!(!details.files_changed);
    assert!(!details.git_state_changed);
    // No content of the file is in the preview.
    let text = serde_json::to_string(&preview).unwrap();
    assert!(!text.contains(TITLE) && !text.contains(BODY.trim()));

    // One confirmation, and nothing else.
    assert_eq!(stranded.confirmations(), 1);
    assert_eq!(before, repository_and_worktree_snapshot(&world.fixture));
    assert_eq!(journal, stranded.journal());
    assert_eq!(world.request_rows(), 1);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    assert_eq!(world.pending_journal_rows(), 1);
}

#[test]
fn an_abandon_without_a_confirmation_is_confirmation_required_and_closes_nothing() {
    let stranded = Stranded::new();

    let refused = stranded.world.execute(REQUEST_3, stranded.abandon());

    assert_refused(
        &refused,
        ResultCode::ConfirmationRequired,
        &["request.prepare"],
    );
    assert_eq!(stranded.world.pending_journal_rows(), 1);
    assert_eq!(stranded.world.record(REQUEST_3), None);
}

/// The operation is completed by its own request after the preview: the
/// confirmation is of a pending row, and what is there is not that.
#[test]
fn an_operation_that_completed_between_preview_and_confirmation_is_external_change() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    let finished = world.execute(REQUEST_1, world.save(TICKET_A, &stranded.token));
    assert_eq!(finished.code, ResultCode::Ok, "{}", finished.message);
    let journal = stranded.journal();

    let refused = stranded.execute(REQUEST_3, &confirmation);

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    assert_eq!(world.record(REQUEST_3), None);
    assert_eq!(journal, stranded.journal());
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Finished),
        "the save's record is not released"
    );
    // A new preview says what is true now: nothing is left to abandon.
    let noop = prepare_with(&world.service, &stranded.abandon());
    assert_eq!(noop.code, ResultCode::AlreadyApplied);
    assert_eq!(noop.outcome, Outcome::Noop);
    assert_eq!(noop.data, None);
}

/// A step recorded after the preview is a change as well: the
/// confirmation binds the row's state and step.
#[test]
fn an_operation_that_advanced_between_preview_and_confirmation_is_external_change() {
    let stranded = Stranded::new();
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    items::index(stranded.world.data.path())
        .execute(
            "UPDATE operation_records SET completed_step = 'authoring_checkpoint_observed'
              WHERE state != 'completed'",
            [],
        )
        .unwrap();

    let refused = stranded.execute(REQUEST_3, &confirmation);

    assert_refused(&refused, ResultCode::ExternalChange, &["request.prepare"]);
    assert_eq!(stranded.world.pending_journal_rows(), 1);
    assert_eq!(stranded.world.record(REQUEST_3), None);
    assert_eq!(
        stranded.world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
}

#[test]
fn an_unknown_operation_is_operation_not_found_and_an_ended_one_is_a_no_op() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    let prepare =
        |operation_id: &str| prepare_with(&world.service, &abandon(&world.root, operation_id));

    let unknown = prepare(&OperationId::new().to_string());
    assert_eq!(unknown.code, ResultCode::OperationNotFound);
    assert_eq!(unknown.outcome, Outcome::Error);
    assert_eq!(unknown.data, None);

    // Close the stranded one, so that others can run.
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    assert_eq!(
        stranded.execute(REQUEST_3, &confirmation).code,
        ResultCode::Ok
    );
    let completed = OperationId::new();
    world
        .service
        .add_remote(manyhands::repository::AddRemoteRequest {
            root: world.root.clone(),
            name: "backup".to_owned(),
            url: "https://example.invalid/backup.git".to_owned(),
            operation_id: completed,
        })
        .unwrap();
    let rejected = OperationId::new();
    world
        .service
        .add_remote(manyhands::repository::AddRemoteRequest {
            root: world.root.clone(),
            name: "not a name".to_owned(),
            url: "https://example.invalid/backup.git".to_owned(),
            operation_id: rejected,
        })
        .unwrap_err();
    let journal = stranded.journal();
    let confirmations = stranded.confirmations();

    for (what, operation_id) in [
        ("completed", completed.to_string()),
        ("rejected", rejected.to_string()),
        ("abandoned", stranded.operation_id.clone()),
    ] {
        let noop = prepare(&operation_id);
        assert_eq!(noop.code, ResultCode::AlreadyApplied, "{what}");
        assert_eq!(noop.outcome, Outcome::Noop, "{what}");
        assert_eq!(noop.data, None, "{what}");
    }
    // An operation of another repository's journal is not this one's.
    let elsewhere = Plain::new();
    let foreign = prepare_with(
        &world.service,
        &abandon(&elsewhere.root, &stranded.operation_id),
    );
    assert_eq!(foreign.code, ResultCode::OperationNotFound);

    assert_eq!(journal, stranded.journal());
    assert_eq!(confirmations, stranded.confirmations(), "no confirmation");
}

/// Only local operations are abandoned. A synchronization's reservation
/// has its own cancellation, and a key operation completes on its next
/// call.
#[test]
fn a_remote_or_a_key_operation_is_refused_with_nothing_changed() {
    let world = World::new();
    let connection = items::index(world.data.path());
    connection
        .execute_batch("UPDATE remote_polling_state SET remote_name='origin',primary_branch='main'")
        .unwrap();
    let remote = OperationId::new();
    let reserved = world
        .service
        .reserve_remote_operation(
            &world.root,
            remote,
            &RemoteOperationTarget::for_primary_synchronization(
                &RemoteRefPlan::from_configuration("origin", "main").unwrap(),
            ),
        )
        .unwrap();
    assert!(matches!(reserved, RemoteReservationOutcome::Reserved(_)));
    let home = tempfile::tempdir().unwrap();
    let generated = world.execute(
        REQUEST_1,
        mutation::generate_in(home.path(), "Work key", None),
    );
    assert_eq!(generated.code, ResultCode::Ok, "{}", generated.message);
    let key = world.record(REQUEST_1).unwrap().operations[0].operation_id;
    let count = |table: &str| -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    };
    let before = (
        repository_and_worktree_snapshot(&world.fixture),
        journal(world.data.path()),
        count("remote_operation_records"),
        count("confirmation_records"),
        count("request_records"),
    );

    for (what, operation_id) in [("remote", remote), ("key", key)] {
        let mutation = abandon(&world.root, &operation_id.to_string());
        let refused = prepare_with(&world.service, &mutation);
        assert_eq!(refused.code, ResultCode::InvalidInput, "{what}");
        assert_eq!(refused.outcome, Outcome::Error, "{what}");
        assert_eq!(refused.data, None, "{what}");
        // And with no preview there is no confirmation to run it with.
        let unconfirmed = world.execute(REQUEST_2, mutation);
        assert_eq!(unconfirmed.code, ResultCode::ConfirmationRequired, "{what}");
    }

    assert_eq!(
        before,
        (
            repository_and_worktree_snapshot(&world.fixture),
            journal(world.data.path()),
            count("remote_operation_records"),
            count("confirmation_records"),
            count("request_records"),
        )
    );
    let row = world
        .service
        .journal_row(
            manyhands::repository::OperationFamily::Remote,
            &manyhands::repository::request_store::ScopeKey::for_repository(&world.root).unwrap(),
            remote,
        )
        .unwrap();
    assert_eq!(
        row,
        manyhands::repository::request_store::JournalRow::Pending(
            manyhands::repository::request_store::PendingOperation::Remote {
                phase: manyhands::repository::RemoteOperationPhase::Reserved
            }
        )
    );
}

// ---------------------------------------------------------------------
// The abandoned operation and the requests around it
// ---------------------------------------------------------------------

#[test]
fn a_repeat_of_an_abandoned_operation_is_refused_and_writes_nothing() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    assert_eq!(
        stranded.execute(REQUEST_3, &confirmation).code,
        ResultCode::Ok
    );
    let before = repository_and_worktree_snapshot(&world.fixture);
    let journal = stranded.journal();
    let abandoned = OperationId::parse(&stranded.operation_id).unwrap();

    let refused = world
        .service
        .refresh_repository(RefreshRepositoryRequest {
            root: world.root.clone(),
            operation_id: abandoned,
        })
        .unwrap_err();

    assert_eq!(refused.kind, RepositoryErrorKind::OperationAbandoned);
    assert_eq!(before, repository_and_worktree_snapshot(&world.fixture));
    assert_eq!(journal, stranded.journal());
    assert!(
        journal.contains(&(
            stranded.operation_id.clone(),
            "completed".to_owned(),
            Some("abandoned".to_owned())
        )),
        "{journal:?}"
    );
    let repo = world.resolved();
    let shown = world.service.show_operation(&repo, abandoned).unwrap();
    assert_eq!(shown.state, "completed");
    assert_eq!(shown.completed_step.as_deref(), Some("abandoned"));
}

#[test]
fn a_retry_of_an_abandon_returns_its_stored_result_and_another_operation_is_a_mismatch() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    let abandoned = stranded.execute(REQUEST_3, &confirmation);
    assert_eq!(abandoned.code, ResultCode::Ok);
    let journal = stranded.journal();
    let rows = world.request_rows();

    // With its confirmation, with none, and long after: the stored
    // result, and nothing written.
    for retry in [
        stranded.execute(REQUEST_3, &confirmation),
        world.execute(REQUEST_3, stranded.abandon()),
    ] {
        assert_eq!(retry, abandoned);
    }
    assert_eq!(journal, stranded.journal());
    assert_eq!(world.request_rows(), rows);

    // The same request ID for another operation.
    let other = execute_confirmed(
        &world.service,
        REQUEST_3,
        &confirmation,
        abandon(&world.root, &OperationId::new().to_string()),
    );
    assert_eq!(other.code, ResultCode::RequestMismatch);
    assert_eq!(other.data, None);
    assert_eq!(journal, stranded.journal());
}

/// The abandon's own result is lost after it closed the row. The row,
/// closed as abandoned, is what shows the request's work to be done.
#[test]
fn an_abandon_whose_result_was_lost_is_a_no_op_and_not_external_change() {
    let mut stranded = Stranded::new();
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    stranded.world.service = RepositoryService::open_at_with_failure_point_for_testing(
        stranded.world.data.path(),
        FailurePoint::BeforeRequestSettlement,
    )
    .unwrap();
    stranded.execute(REQUEST_3, &confirmation);
    assert_eq!(stranded.world.pending_journal_rows(), 0);
    assert_eq!(
        stranded.world.record(REQUEST_3).map(|record| record.state),
        Some(RequestState::Accepted)
    );

    stranded.world.reopen();
    let retried = stranded.execute(REQUEST_3, &confirmation);

    assert_eq!(retried.code, ResultCode::AlreadyApplied);
    assert_eq!(retried.outcome, Outcome::Noop);
    assert_eq!(
        stranded.world.record(REQUEST_3).map(|record| record.state),
        Some(RequestState::Finished)
    );
    assert_eq!(stranded.world.record(REQUEST_1), None);
}

/// The abandon is cut off before its transaction. Its retry finds the
/// row as it was previewed and closes it, however old its confirmation.
#[test]
fn an_abandon_interrupted_before_its_transaction_is_completed_by_its_retry() {
    let mut stranded = Stranded::new();
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    stranded.world.service = RepositoryService::open_at_with_failure_point_for_testing(
        stranded.world.data.path(),
        FailurePoint::BeforeRequestDomainCall,
    )
    .unwrap();
    stranded.execute(REQUEST_3, &confirmation);
    assert_eq!(stranded.world.pending_journal_rows(), 1);

    stranded.world.reopen();
    let retried = stranded.execute(REQUEST_3, &confirmation);

    assert_eq!(retried.code, ResultCode::Ok, "{}", retried.message);
    assert_eq!(
        operation_data(&retried).released_request_id.as_deref(),
        Some(REQUEST_1)
    );
    assert_eq!(stranded.world.pending_journal_rows(), 0);
    assert_eq!(stranded.world.record(REQUEST_1), None);
}

/// A call that holds the repository's lease may be the one working under
/// the row. The abandon is busy, and the same request closes the row
/// once the lease is free.
#[test]
fn an_abandon_is_busy_while_a_call_holds_the_repository_lease() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    let held = RepositoryService::hold_lease_for_testing(
        &world.root,
        world.data.path(),
        LeaseKind::Repository,
    )
    .unwrap();

    let busy = stranded.execute(REQUEST_3, &confirmation);

    assert_eq!(busy.code, ResultCode::Busy);
    assert_eq!(recovery_actions(&busy), ["request.retry"]);
    assert_eq!(world.pending_journal_rows(), 1);
    assert_eq!(
        world.record(REQUEST_1).map(|record| record.state),
        Some(RequestState::Accepted)
    );
    drop(held);

    let abandoned = stranded.execute(REQUEST_3, &confirmation);
    assert_eq!(abandoned.code, ResultCode::Ok, "{}", abandoned.message);
    assert_eq!(world.pending_journal_rows(), 0);
}

/// The request that began the operation is entered again at the moment
/// its operation is abandoned: it has read its row as pending, and then
/// calls the domain under an ID that is spent. It is refused there, and
/// nothing of it is left.
#[test]
fn a_call_of_the_owning_request_that_races_the_abandon_is_refused_and_leaves_nothing() {
    let stranded = Stranded::new();
    let world = &stranded.world;
    let (confirmation, _) = stranded.preview(&stranded.abandon());
    let (data, root, operation_id) = (
        world.data.path().to_owned(),
        world.root.clone(),
        stranded.operation_id.clone(),
    );
    world.service.set_request_hook_for_testing(move || {
        let other = RepositoryService::open_at(&data).unwrap();
        let abandoned = execute_confirmed(
            &other,
            REQUEST_3,
            &confirmation,
            abandon(&root, &operation_id),
        );
        assert_eq!(abandoned.code, ResultCode::Ok, "{}", abandoned.message);
    });
    let before = world.worktree_source(TICKET_A);

    let raced = world.execute(REQUEST_1, world.save(TICKET_A, &stranded.token));

    assert_eq!(raced.code, ResultCode::ExternalChange);
    assert_eq!(world.record(REQUEST_1), None);
    assert_eq!(world.pending_journal_rows(), 0);
    assert_eq!(world.branch_commits(TICKET_A), 0);
    assert_eq!(world.worktree_source(TICKET_A), before);
    let other = world.execute(REQUEST_5, world.create(TICKET_NEW));
    assert_eq!(other.code, ResultCode::Ok, "{}", other.message);
}

/// A create killed after it made its directory, into which something
/// else was then put: its retry is refused for what is there, with its
/// own operation in the way of any new preview. The abandon it is
/// offered is the way out, for a root that is no repository yet, and
/// for one that is no longer there at all.
#[test]
fn a_create_that_cannot_go_on_is_abandoned_by_the_action_its_refusal_offers() {
    for removed in [false, true] {
        let what = format!("removed {removed}");
        let mut desk = Desk::new();
        let site = Site::new();
        let create = || killed_create(site.parent.path());
        let (confirmation, _) = desk.preview(&create());
        killed_at(
            &desk,
            site.parent.path(),
            false,
            &confirmation,
            FailurePoint::BeforeRepositoryInitialization,
        );
        fs::write(site.root.join("notes.txt"), "mine\n").unwrap();
        desk.reopen();
        let refused = desk.execute(REQUEST_1, &confirmation, create());
        assert_eq!(refused.code, ResultCode::ExternalChange, "{what}");
        assert_offered_its_own_abandon(&refused, &what);
        let offered = &refused.recovery[1];
        let operation_id = offered.operation_id.clone().unwrap();
        let root = PathBuf::from(offered.arguments["root"].as_str().unwrap());
        if removed {
            fs::remove_dir_all(&site.root).unwrap();
        }
        let before = tree(site.parent.path());

        let (abandon_confirmation, preview) = desk.preview(&abandon(&root, &operation_id));
        let details = abandon_preview(&preview);
        assert_eq!(
            details.action,
            Some(OperationAction::CreateAndEnable),
            "{what}"
        );
        assert_eq!(details.worktree, None, "{what}");
        assert_eq!(details.paths, [] as [&str; 0], "{what}");
        assert_eq!(details.request_id.as_deref(), Some(REQUEST_1), "{what}");
        let abandoned = desk.execute(
            REQUEST_2,
            &abandon_confirmation,
            abandon(&root, &operation_id),
        );

        assert_eq!(
            abandoned.code,
            ResultCode::Ok,
            "{what}: {}",
            abandoned.message
        );
        assert_eq!(before, tree(site.parent.path()), "{what}");
        assert_eq!(desk.pending_journal_rows(), 0, "{what}");
        assert_eq!(desk.record(REQUEST_1), None, "{what}");

        // With the target cleared, the same request ID creates the
        // repository, with a new preview.
        if !removed {
            fs::remove_dir_all(&site.root).unwrap();
        }
        let (confirmation, _) = desk.preview(&create());
        let created = desk.execute(REQUEST_1, &confirmation, create());
        assert_enabled(&created, &site.root);
        assert!(desk.registered(&site.root), "{what}");
    }
}
