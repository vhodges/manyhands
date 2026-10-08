//! The status reads: one registration's index, its polling policy and the
//! operations its three stores hold.

// A read error carries its whole scope by value, as the contract has it.
#![allow(clippy::result_large_err)]

use std::fs;

use manyhands::{
    repository::{
        AuthoringKind, IndexState, IndexStatusState, OperationAction, OperationDto,
        OperationFamily, OperationId, OperationNextAction, OperationOwner, PollingInterval,
        PollingOutcome, RecoveryInspection, RefreshOutcome, RemoteOperationAction,
        RemoteOperationSafePoint, RemoteOperationTarget, RemoteOutcomeCategory, RemoteRefPlan,
        RemoteReservation, RemoteReservationOutcome, RemoteSafePointOutcome, ResolvedRepository,
        keys::{KeyMaterialAction, RecoveryAction},
    },
    results::{OperationFailureCode, Outcome, ProblemCode, ResultCode},
};
use serde_json::{Value, json};
use support::{
    items::{
        self, DOCUMENT_A, TICKET_A, contract_repository, degraded_service, document_source,
        never_refreshed_repository, refresh, refresh_completely, write,
    },
    operations::{
        self, KEY_MATERIAL_SENTINEL, OPERATION_A, OPERATION_ABSENT, OPERATION_B, OPERATION_C,
        STORED_AT_TEXT, configure_remote, insert_key_material, insert_local,
        insert_remote_index_pending, insert_remote_poll, insert_remote_synchronization,
        poll_target,
    },
};

mod support;

/// Planted in text the stores hold beside a code, where it must not be
/// published.
const SENTINEL: &str = "SENTINEL-b83f";

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
}

fn enabled() -> (
    support::TestRepository,
    support::EnabledRepository,
    ResolvedRepository,
) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    (fixture, enabled, repo)
}

fn reserved(outcome: RemoteReservationOutcome) -> RemoteReservation {
    match outcome {
        RemoteReservationOutcome::Reserved(token) => token,
        other => panic!("expected a reservation, got {other:?}"),
    }
}

fn operation_ids(operations: &[OperationDto]) -> Vec<Option<&str>> {
    operations
        .iter()
        .map(|operation| operation.operation_id.as_deref())
        .collect()
}

#[test]
fn index_status_of_a_refreshed_repository_is_current_and_counts_what_it_holds() {
    let (fixture, enabled) = contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let status = enabled.service.index_status(&repo).unwrap();

    assert_eq!(status.state, IndexStatusState::Current);
    assert!(status.refreshed_at.is_some());
    assert_eq!(status.context_count, Some(1));
    assert_eq!(status.item_count, Some(4));
    assert_eq!(status.problem_count, Some(0));
    assert!(status.problems.is_empty());
    assert!(status.pending_operations.is_empty());
    // The state and time are the ones every list reports.
    let list = enabled.service.list_documents(&repo).unwrap();
    assert_eq!(status.state.as_str(), list.index.state.as_str());
    assert_eq!(status.refreshed_at, list.index.refreshed_at);
    assert_git_transport_uninitialized();
}

#[test]
fn index_status_counts_an_item_once_however_many_worktrees_hold_it() {
    let (fixture, enabled) = contract_repository();
    items::create_document_context(
        &enabled.service,
        &fixture.root,
        items::DOCUMENT_C,
        "docs/c.md",
    );
    refresh_completely(&enabled.service, &fixture.root);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let status = enabled.service.index_status(&repo).unwrap();

    // The item worktree's checkout holds the four primary items as well.
    assert_eq!(status.state, IndexStatusState::Current);
    assert_eq!(status.context_count, Some(2));
    assert_eq!(status.item_count, Some(5));
    assert_git_transport_uninitialized();
}

#[test]
fn index_status_of_a_registration_never_refreshed_names_the_operation_that_would() {
    let never = never_refreshed_repository();
    let repo = never
        .service
        .resolve_repository(&never.fixture.root)
        .unwrap();

    let status = never.service.index_status(&repo).unwrap();

    assert_eq!(status.state, IndexStatusState::NeverRefreshed);
    assert_eq!(status.refreshed_at, None);
    assert_eq!(status.context_count, Some(0));
    assert_eq!(status.item_count, Some(0));
    let id = never.operation_id.to_string();
    assert_eq!(operation_ids(&status.pending_operations), [Some(&*id)]);
    let pending = &status.pending_operations[0];
    assert_eq!(pending.family, OperationFamily::Local);
    assert_eq!(pending.owner, OperationOwner::Repository);
    assert_eq!(pending.action, OperationAction::Enable);
    assert_ne!(pending.state, "completed");
    assert_eq!(pending.next_action, Some(OperationNextAction::Resume));
    assert!(pending.updated_at.is_some());
    assert_eq!(pending.failure_code, None);

    // It is the operation recovery reports, and the one the list holds.
    let inspected = never
        .service
        .recovery_inspection(&never.fixture.root)
        .unwrap();
    let [
        RecoveryInspection::Pending {
            operation_id,
            completed_step,
            ..
        },
    ] = &inspected[..]
    else {
        panic!("{inspected:?}");
    };
    assert_eq!(*operation_id, never.operation_id);
    assert_eq!(&pending.completed_step, completed_step);
    let list = never.service.list_operations(&repo).unwrap();
    assert_eq!(list.items, status.pending_operations);
    assert_eq!(
        &never
            .service
            .show_operation(&repo, never.operation_id)
            .unwrap(),
        pending
    );
    assert_git_transport_uninitialized();
}

#[test]
fn index_status_of_an_interrupted_refresh_is_stale_and_says_why() {
    let (fixture, enabled, repo) = enabled();
    let document = write(
        &fixture.root,
        "docs/a.md",
        &document_source(DOCUMENT_A, "A", ""),
    );
    refresh_completely(&enabled.service, &fixture.root);
    enabled
        .service
        .set_observation_hook_for_testing(move || fs::write(document, "plain\n").unwrap());
    assert!(matches!(
        refresh(&enabled.service, &fixture.root),
        RefreshOutcome::RetryRequired { .. }
    ));

    let status = enabled.service.index_status(&repo).unwrap();

    assert_eq!(status.state, IndexStatusState::Stale);
    assert!(status.refreshed_at.is_some());
    assert_eq!(status.problem_count, Some(status.problems.len() as u64));
    assert!(
        status
            .problems
            .iter()
            .any(|problem| problem.code == ProblemCode::RetryRequired),
        "{:?}",
        status.problems
    );
    let [pending] = &status.pending_operations[..] else {
        panic!("{:?}", status.pending_operations);
    };
    assert_eq!(pending.action, OperationAction::Refresh);
    assert_eq!(pending.next_action, Some(OperationNextAction::Resume));
    assert_git_transport_uninitialized();
}

#[test]
fn index_status_succeeds_when_the_index_cannot_be_read() {
    let unavailable = json!({
        "state": "unavailable",
        "refreshed_at": null,
        "context_count": null,
        "item_count": null,
        "problem_count": null,
        "problems": [],
        "pending_operations": [],
    });

    // An index that was already unreadable when the service opened it.
    let (fixture, enabled) = contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let (_data, service) = degraded_service(enabled);
    let status = service.index_status(&repo).unwrap();
    assert_eq!(status.state, IndexStatusState::Unavailable);
    assert_eq!(serde_json::to_value(&status).unwrap(), unavailable);
    // Every other read of it fails.
    assert_eq!(
        service.list_documents(&repo).unwrap_err().code(),
        ResultCode::IndexUnavailable
    );
    for error in [
        service.polling_status(&repo).map(drop).unwrap_err(),
        service.list_operations(&repo).map(drop).unwrap_err(),
        service
            .show_operation(&repo, operations::operation_id(OPERATION_A))
            .map(drop)
            .unwrap_err(),
    ] {
        assert_eq!(error.code(), ResultCode::IndexUnavailable);
        assert_eq!(error.scope.repository.as_deref(), repo.root().to_str());
        assert_eq!(error.recovery.len(), 1);
        assert_eq!(error.recovery[0].action.as_str(), "index.rebuild");
    }

    // An index that went away under a service that had opened it.
    let (fixture, enabled) = contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    for entry in fs::read_dir(enabled.data_directory.path()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        if name.starts_with("manyhands.sqlite3") && !name.ends_with(".lock") {
            fs::remove_file(path).unwrap();
        }
    }
    let status = enabled.service.index_status(&repo).unwrap();
    assert_eq!(serde_json::to_value(&status).unwrap(), unavailable);
    assert_git_transport_uninitialized();
}

#[test]
fn index_status_lists_every_stored_problem_with_fixed_guidance() {
    let (fixture, enabled) = contract_repository();
    let root = fs::canonicalize(&fixture.root).unwrap();
    write(
        &fixture.root,
        "docs/marker.md",
        "---\nmanyhands_managed: true\n---\n",
    );
    refresh_completely(&enabled.service, &fixture.root);
    let index = items::index(enabled.data_directory.path());
    // A problem about the registration as a whole, and one whose code this
    // build does not know; both stored with text that is not published.
    for code in ["retry-required", "future-code"] {
        index
            .execute(
                "INSERT INTO problems (repository_id, code, guidance, observed_at)
                 SELECT id, ?1, ?2, 0 FROM repositories",
                [code, SENTINEL],
            )
            .unwrap();
    }
    index
        .execute("UPDATE problems SET guidance = ?1", [SENTINEL])
        .unwrap();
    drop(index);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let status = enabled.service.index_status(&repo).unwrap();

    // Those with no working tree first, then by path and stored code.
    let listed: Vec<_> = status
        .problems
        .iter()
        .map(|problem| {
            (
                problem.code,
                problem.path.as_deref(),
                problem.worktree.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            (ProblemCode::UnknownProblem, None, None),
            (ProblemCode::RetryRequired, None, None),
            (
                ProblemCode::MissingField,
                Some("docs/marker.md"),
                root.to_str()
            ),
        ]
    );
    assert_eq!(status.problem_count, Some(3));
    let text = serde_json::to_string(&status).unwrap();
    assert!(!text.contains(SENTINEL), "{text}");
    assert_eq!(
        serde_json::to_value(&status.problems[1]).unwrap(),
        json!({
            "code": "retry_required",
            "path": null,
            "worktree": null,
            "guidance": ProblemCode::RetryRequired.guidance(),
        })
    );
    // The item lists report only the file that is not an item.
    let documents = enabled.service.list_documents(&repo).unwrap();
    assert_eq!(
        documents
            .items
            .iter()
            .filter(|item| item.id.is_none())
            .count(),
        1
    );
    assert_git_transport_uninitialized();
}

#[test]
fn polling_status_reports_the_stored_policy_and_the_latest_outcome() {
    let (fixture, enabled, repo) = enabled();
    let (service, root) = (&enabled.service, &fixture.root);

    // What a registration starts with.
    assert_eq!(
        serde_json::to_value(service.polling_status(&repo).unwrap()).unwrap(),
        json!({
            "enabled": true,
            "paused": false,
            "interval_seconds": 300,
            "backoff_seconds": null,
            "recovery_suspended": false,
            "latest_outcome": null,
            "latest_observed_at": null,
            "active_operation_id": null,
            "next_eligible_at": null,
        })
    );

    service
        .set_remote_polling(
            root,
            true,
            true,
            PollingInterval::from_seconds(600).unwrap(),
        )
        .unwrap();
    configure_remote(enabled.data_directory.path());
    let id = OperationId::new();
    let token = reserved(
        service
            .reserve_remote_operation(root, id, &poll_target())
            .unwrap(),
    );
    let reserved = service.polling_status(&repo).unwrap();
    assert!(reserved.enabled && reserved.paused);
    assert_eq!(reserved.interval_seconds, 600);
    assert_eq!(reserved.active_operation_id, Some(id.to_string()));
    assert_eq!(reserved.latest_outcome, None);

    assert_eq!(
        service
            .remote_safe_point(root, &token, RemoteOperationSafePoint::BeforeTransport)
            .unwrap(),
        RemoteSafePointOutcome::Continue
    );
    service
        .finish_remote_operation(root, &token, RemoteOutcomeCategory::TransportUnavailable)
        .unwrap();
    let failed = service.polling_status(&repo).unwrap();
    assert_eq!(
        failed.latest_outcome,
        Some(PollingOutcome::TransportUnavailable)
    );
    assert_eq!(failed.backoff_seconds, Some(60));
    assert_eq!(failed.active_operation_id, None);
    // The attempt observed nothing, and no time is stored for an outcome.
    assert_eq!(failed.latest_observed_at, None);
    assert_eq!(failed.next_eligible_at, None);

    operations::insert_current_observation(enabled.data_directory.path());
    let observed = service.polling_status(&repo).unwrap();
    assert_eq!(observed.latest_observed_at.as_deref(), Some(STORED_AT_TEXT));
    assert_eq!(observed.next_eligible_at, None);
    assert_eq!(
        PollingStatusFields::of(&observed),
        PollingStatusFields::of(&failed)
    );
    assert_git_transport_uninitialized();
}

/// The polling status fields an observation does not change.
#[derive(Debug, PartialEq)]
struct PollingStatusFields(bool, bool, u64, Option<u64>, bool, Option<PollingOutcome>);

impl PollingStatusFields {
    fn of(status: &manyhands::repository::PollingStatusDto) -> Self {
        Self(
            status.enabled,
            status.paused,
            status.interval_seconds,
            status.backoff_seconds,
            status.recovery_suspended,
            status.latest_outcome,
        )
    }
}

#[test]
fn the_three_stores_appear_in_one_list_ordered_by_operation_id() {
    let (fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    configure_remote(data);
    // An active remote reservation, made as polling makes one.
    let remote_id = operations::operation_id(OPERATION_B);
    reserved(
        service
            .reserve_remote_operation(&fixture.root, remote_id, &poll_target())
            .unwrap(),
    );
    // Stored out of ID order, and the two with no ID in this order.
    insert_local(data, None, "rebuild", "created", None);
    insert_local(
        data,
        Some(OPERATION_C),
        "save_ticket",
        "authoring_checkpoint_observed",
        Some("authoring_checkpoint_observed"),
    );
    insert_local(data, None, "refresh", "failed", None);
    let key_id = insert_key_material(
        data,
        OPERATION_A,
        "generate",
        "retained-for-inspection",
        Some("source-missing"),
    );
    // Finished operations are not listed.
    insert_local(
        data,
        Some("01ARZ3NDEKTSV4RRFFQ69G5FA3"),
        "refresh",
        "completed",
        Some("completed"),
    );
    insert_key_material(
        data,
        "01ARZ3NDEKTSV4RRFFQ69G5FA4",
        "delete",
        "completed",
        None,
    );

    let list = service.list_operations(&repo).unwrap();

    assert!(list.complete);
    assert_eq!(
        operation_ids(&list.items),
        [
            Some(OPERATION_A),
            Some(OPERATION_B),
            Some(OPERATION_C),
            None,
            None
        ]
    );
    let [key, remote, local, rebuild, refresh] = &list.items[..] else {
        unreachable!()
    };
    assert_eq!(
        serde_json::to_value(key).unwrap(),
        json!({
            "operation_id": OPERATION_A,
            "family": "key_material",
            "owner": "application",
            "action": "generate_key",
            "state": "retained_for_inspection",
            "completed_step": null,
            "next_action": "inspect_retained_files",
            "item_id": null,
            "key_id": key_id,
            "worktree": null,
            "updated_at": null,
            "failure_code": "source_missing",
        })
    );
    assert_eq!(remote.family, OperationFamily::Remote);
    assert_eq!(remote.owner, OperationOwner::Repository);
    assert_eq!(remote.action, OperationAction::Poll);
    assert_eq!(remote.state, "reserved");
    assert_eq!(remote.completed_step, None);
    assert_eq!(remote.next_action, None);
    assert!(remote.updated_at.is_some());
    assert_eq!(remote.failure_code, None);
    assert_eq!(
        serde_json::to_value(local).unwrap(),
        json!({
            "operation_id": OPERATION_C,
            "family": "local",
            "owner": "repository",
            "action": "save_ticket",
            "state": "authoring_checkpoint_observed",
            "completed_step": "authoring_checkpoint_observed",
            "next_action": "resume",
            "item_id": null,
            "key_id": null,
            "worktree": null,
            "updated_at": STORED_AT_TEXT,
            "failure_code": null,
        })
    );
    assert_eq!(
        (rebuild.action, rebuild.state.as_str()),
        (OperationAction::Rebuild, "created")
    );
    assert_eq!(
        (refresh.action, refresh.state.as_str()),
        (OperationAction::Refresh, "failed")
    );
    assert_eq!(refresh.family, OperationFamily::Local);

    // Each one that has an ID is found by it, as the list has it.
    for operation in &list.items[..3] {
        let id = operations::operation_id(operation.operation_id.as_deref().unwrap());
        assert_eq!(&service.show_operation(&repo, id).unwrap(), operation);
    }
    // The index status holds the local ones, in the order they were stored.
    let status = service.index_status(&repo).unwrap();
    assert_eq!(
        operation_ids(&status.pending_operations),
        [None, Some(OPERATION_C), None]
    );
    assert_eq!(status.pending_operations[0], *rebuild);

    let error = service
        .show_operation(&repo, operations::operation_id(OPERATION_ABSENT))
        .unwrap_err();
    assert_eq!(error.code(), ResultCode::OperationNotFound);
    assert_eq!(error.scope.repository.as_deref(), repo.root().to_str());
    assert!(error.recovery.is_empty());
    assert_eq!(
        error.to_envelope::<Value>("operation show").outcome,
        Outcome::Error
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_finished_operation_is_found_by_its_id_and_carries_its_failure_code() {
    let (fixture, enabled, repo) = enabled();
    let (service, root, data) = (
        &enabled.service,
        &fixture.root,
        enabled.data_directory.path(),
    );
    configure_remote(data);
    // A poll that failed before it reached the remote.
    let remote_id = OperationId::new();
    let token = reserved(
        service
            .reserve_remote_operation(root, remote_id, &poll_target())
            .unwrap(),
    );
    service
        .remote_safe_point(root, &token, RemoteOperationSafePoint::BeforeTransport)
        .unwrap();
    service
        .finish_remote_operation(root, &token, RemoteOutcomeCategory::HostApprovalRequired)
        .unwrap();
    // A key generation that completed and then lost its files, one that
    // completed, and one that failed for a reason this build does not know.
    insert_key_material(
        data,
        OPERATION_A,
        "generate",
        "completed",
        Some("source-changed"),
    );
    insert_key_material(data, OPERATION_B, "generate", "completed", None);
    let unknown = format!("future-failure {SENTINEL}");
    insert_key_material(
        data,
        OPERATION_C,
        "delete",
        "private-removed",
        Some(&unknown),
    );

    let remote = service.show_operation(&repo, remote_id).unwrap();
    assert_eq!(
        (remote.family, remote.action, remote.state.as_str()),
        (OperationFamily::Remote, OperationAction::Poll, "failed")
    );
    assert_eq!(remote.completed_step.as_deref(), Some("before_transport"));
    assert_eq!(
        remote.failure_code,
        Some(OperationFailureCode::HostApprovalRequired)
    );
    assert_eq!(remote.next_action, None);

    let show = |id: &str| {
        service
            .show_operation(&repo, operations::operation_id(id))
            .unwrap()
    };
    let lost = show(OPERATION_A);
    assert_eq!(lost.state, "completed");
    assert_eq!(lost.failure_code, Some(OperationFailureCode::SourceChanged));
    assert_eq!(
        lost.next_action,
        Some(OperationNextAction::InspectRetainedFiles)
    );
    let completed = show(OPERATION_B);
    assert_eq!(
        (completed.state.as_str(), completed.failure_code),
        ("completed", None)
    );
    assert_eq!(completed.next_action, None);
    assert_eq!(completed.owner, OperationOwner::Application);
    assert_eq!(completed.updated_at, None);
    let deleting = show(OPERATION_C);
    assert_eq!(deleting.action, OperationAction::DeleteKey);
    assert_eq!(deleting.state, "private_removed");
    assert_eq!(
        deleting.failure_code,
        Some(OperationFailureCode::UnknownFailure)
    );
    assert_eq!(
        deleting.next_action,
        Some(OperationNextAction::ReviewDeletionAgain)
    );

    // The failed poll and the completed generation are not listed; the two
    // key-material operations that need attention are.
    let list = service.list_operations(&repo).unwrap();
    assert_eq!(
        operation_ids(&list.items),
        [Some(OPERATION_A), Some(OPERATION_C)]
    );
    // Neither a key's label or paths nor a stored failure's text is read.
    let text = serde_json::to_string(&(list, remote, lost, completed, deleting)).unwrap();
    assert!(!text.contains(SENTINEL), "{text}");
    assert!(!text.contains(KEY_MATERIAL_SENTINEL), "{text}");
    assert_git_transport_uninitialized();
}

#[test]
fn key_material_operations_are_the_ones_key_recovery_lists() {
    let (_fixture, enabled, repo) = enabled();
    let data = enabled.data_directory.path();
    let rows = [
        ("01ARZ3NDEKTSV4RRFFQ69G5FB5", "generate", "reserved", None),
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FB4",
            "generate",
            "private-written",
            Some("storage-unavailable"),
        ),
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FB3",
            "generate",
            "pair-written",
            None,
        ),
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FB2",
            "generate",
            "retained-for-inspection",
            Some("ownership-unverified"),
        ),
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FB1",
            "generate",
            "completed",
            Some("source-missing"),
        ),
        ("01ARZ3NDEKTSV4RRFFQ69G5FB0", "generate", "completed", None),
        ("01ARZ3NDEKTSV4RRFFQ69G5FB6", "delete", "prepared", None),
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FB7",
            "delete",
            "private-removed",
            Some("unsafe-path"),
        ),
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FB8",
            "delete",
            "files-removed",
            None,
        ),
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FB9",
            "delete",
            "retained-for-inspection",
            None,
        ),
        ("01ARZ3NDEKTSV4RRFFQ69G5FBA", "delete", "completed", None),
    ];
    for (id, action, phase, failure) in rows {
        insert_key_material(data, id, action, phase, failure);
    }

    let mut recovery = enabled.service.list_key_material_recovery().unwrap();
    let list = enabled.service.list_operations(&repo).unwrap();

    recovery.sort_by_key(|operation| operation.operation_id.to_string());
    assert_eq!(recovery.len(), 9);
    assert_eq!(list.items.len(), recovery.len());
    for (read, existing) in list.items.iter().zip(&recovery) {
        assert_eq!(
            read.operation_id,
            Some(existing.operation_id.to_string()),
            "{read:?}"
        );
        assert_eq!(read.family, OperationFamily::KeyMaterial);
        assert_eq!(
            read.action,
            match existing.action {
                KeyMaterialAction::Generate => OperationAction::GenerateKey,
                KeyMaterialAction::Delete => OperationAction::DeleteKey,
                other => panic!("{other:?}"),
            }
        );
        assert_eq!(
            read.next_action,
            Some(match existing.recovery_action {
                RecoveryAction::RetryGeneration => OperationNextAction::RetryGeneration,
                RecoveryAction::ReviewDeletionAgain => OperationNextAction::ReviewDeletionAgain,
                RecoveryAction::InspectRetainedFiles => OperationNextAction::InspectRetainedFiles,
            }),
            "{read:?}"
        );
        // The stored code, under its contract name.
        assert_eq!(
            read.failure_code
                .map(|code| code.as_str().replace('_', "-")),
            existing
                .failure_code
                .as_ref()
                .map(|code| code.as_str().to_owned()),
            "{read:?}"
        );
    }
    assert_git_transport_uninitialized();
}

#[test]
fn an_operation_id_two_stores_hold_is_listed_twice_and_shown_as_the_unfinished_one() {
    let (_fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    let id = operations::operation_id(OPERATION_A);
    insert_key_material(data, OPERATION_A, "generate", "reserved", None);
    insert_local(data, Some(OPERATION_A), "refresh", "created", None);

    let list = service.list_operations(&repo).unwrap();
    let families: Vec<_> = list.items.iter().map(|item| item.family).collect();
    assert_eq!(
        families,
        [OperationFamily::Local, OperationFamily::KeyMaterial]
    );
    assert_eq!(
        service.show_operation(&repo, id).unwrap().family,
        OperationFamily::Local
    );

    // Once the local one has completed, the other is what remains to show.
    items::index(data)
        .execute("UPDATE operation_records SET state = 'completed'", [])
        .unwrap();
    assert_eq!(
        service.show_operation(&repo, id).unwrap().family,
        OperationFamily::KeyMaterial
    );
    // And when both have, the local one again.
    items::index(data)
        .execute("UPDATE key_material_operations SET phase = 'completed'", [])
        .unwrap();
    let shown = service.show_operation(&repo, id).unwrap();
    assert_eq!(shown.family, OperationFamily::Local);
    assert_eq!(shown.next_action, None);
    assert!(service.list_operations(&repo).unwrap().items.is_empty());
    assert_git_transport_uninitialized();
}

#[test]
fn operations_of_another_repository_are_not_this_ones() {
    let (_fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    let other = support::born_repository();
    let other_root = fs::canonicalize(&other.root).unwrap();
    items::index(data)
        .execute(
            "INSERT INTO operation_records (root_path, operation_ulid, action, state, observed_at)
             VALUES (?1, ?2, 'refresh', 'created', 0)",
            [other_root.to_str().unwrap(), OPERATION_A],
        )
        .unwrap();
    // A key-material operation belongs to the application.
    insert_key_material(data, OPERATION_B, "generate", "reserved", None);

    let list = service.list_operations(&repo).unwrap();

    assert_eq!(operation_ids(&list.items), [Some(OPERATION_B)]);
    assert_eq!(list.items[0].owner, OperationOwner::Application);
    assert_eq!(
        service
            .show_operation(&repo, operations::operation_id(OPERATION_A))
            .unwrap_err()
            .code(),
        ResultCode::OperationNotFound
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_stored_operation_that_cannot_be_valid_fails_the_read() {
    let (_fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    let state = format!("failed: {SENTINEL}");
    let damaged: [&dyn Fn(&rusqlite::Connection); 5] = [
        &|index| operations::insert_local_into(index, Some(OPERATION_A), "other", "created", None),
        &|index| operations::insert_local_into(index, Some(OPERATION_A), "refresh", &state, None),
        &|index| {
            operations::insert_local_into(
                index,
                Some(OPERATION_A),
                "refresh",
                "created",
                Some("Not A Step"),
            )
        },
        &|index| {
            operations::insert_local_into(index, Some("not-a-ulid"), "refresh", "created", None)
        },
        &|index| {
            index
                .execute(
                    "INSERT INTO operation_records
                        (root_path, operation_ulid, action, item_id, state, observed_at)
                     SELECT root_path, ?1, 'refresh', 'not-an-item', 'created', 0
                       FROM repositories",
                    [OPERATION_A],
                )
                .unwrap();
        },
    ];
    for (case, damage) in damaged.iter().enumerate() {
        let index = items::index(data);
        damage(&index);
        drop(index);

        let errors = [
            service.list_operations(&repo).map(drop).unwrap_err(),
            service.index_status(&repo).map(drop).unwrap_err(),
        ];
        for error in errors {
            assert_eq!(error.code(), ResultCode::InternalError, "case {case}");
            let envelope = serde_json::to_string(&error.to_envelope::<Value>("operation list"));
            assert!(!envelope.unwrap().contains(SENTINEL));
        }

        items::index(data)
            .execute("DELETE FROM operation_records", [])
            .unwrap();
        assert!(service.list_operations(&repo).unwrap().items.is_empty());
    }

    // A key-material operation whose ID is not one, and one in a phase its
    // action does not have.
    insert_key_material(data, "not-a-ulid", "generate", "reserved", None);
    assert_eq!(
        service.list_operations(&repo).unwrap_err().code(),
        ResultCode::InternalError
    );
    let index = items::index(data);
    index
        .execute_batch("DELETE FROM key_material_operations; PRAGMA ignore_check_constraints = ON;")
        .unwrap();
    index
        .execute(
            "INSERT INTO key_material_operations
                (operation_id, key_id, action, private_key_path, public_key_path, phase)
             VALUES (?1, '01ARZ3NDEKTSV4RRFFQ69G5FK0', 'delete', 'private', 'public',
                     'pair-written')",
            [OPERATION_A],
        )
        .unwrap();
    drop(index);
    for error in [
        service.list_operations(&repo).map(drop).unwrap_err(),
        service
            .show_operation(&repo, operations::operation_id(OPERATION_A))
            .map(drop)
            .unwrap_err(),
    ] {
        assert_eq!(error.code(), ResultCode::InternalError);
    }
    // And one for a key whose ID is not one.
    items::index(data)
        .execute(
            "UPDATE key_material_operations SET phase = 'prepared', key_id = 'not-a-key'",
            [],
        )
        .unwrap();
    assert_eq!(
        service.list_operations(&repo).unwrap_err().code(),
        ResultCode::InternalError
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_registration_removed_since_it_was_resolved_is_not_registered() {
    let (_fixture, enabled, repo) = enabled();
    let index = items::index(enabled.data_directory.path());
    index
        .execute_batch("PRAGMA foreign_keys = ON; DELETE FROM repositories;")
        .unwrap();
    drop(index);
    let service = &enabled.service;

    for error in [
        service.index_status(&repo).map(drop).unwrap_err(),
        service.polling_status(&repo).map(drop).unwrap_err(),
        service.list_operations(&repo).map(drop).unwrap_err(),
        service
            .show_operation(&repo, operations::operation_id(OPERATION_A))
            .map(drop)
            .unwrap_err(),
    ] {
        assert_eq!(error.code(), ResultCode::RepositoryNotRegistered);
        assert_eq!(error.scope.repository.as_deref(), repo.root().to_str());
    }
    assert_git_transport_uninitialized();
}

#[test]
fn a_remote_operation_shown_by_id_reports_each_stored_phase_and_step() {
    let (_fixture, enabled, repo) = enabled();
    let data = enabled.data_directory.path();
    configure_remote(data);
    insert_remote_poll(
        data,
        OPERATION_A,
        "interrupted",
        Some("after_advertisement"),
        None,
    );
    insert_remote_poll(
        data,
        OPERATION_B,
        "cancelled",
        Some("between_observations"),
        Some("cancelled"),
    );
    insert_remote_poll(
        data,
        OPERATION_C,
        "persisting",
        Some("before_batch_commit"),
        None,
    );
    let show = |id: &str| {
        enabled
            .service
            .show_operation(&repo, operations::operation_id(id))
            .unwrap()
    };

    let interrupted = show(OPERATION_A);
    assert_eq!(interrupted.state, "interrupted");
    assert_eq!(
        interrupted.completed_step.as_deref(),
        Some("after_advertisement")
    );
    assert_eq!(interrupted.failure_code, None);
    let cancelled = show(OPERATION_B);
    assert_eq!(cancelled.state, "cancelled");
    assert_eq!(
        cancelled.failure_code,
        Some(OperationFailureCode::Cancelled)
    );
    let active = show(OPERATION_C);
    assert_eq!(active.state, "persisting");
    assert_eq!(
        active.updated_at.as_deref(),
        Some("2023-11-14T22:15:00Z"),
        "the stored update time, not the creation time"
    );

    // Only the one that holds the reservation is listed, and it is the one
    // polling status names.
    let list = enabled.service.list_operations(&repo).unwrap();
    assert_eq!(list.items, [active]);
    assert_eq!(
        enabled
            .service
            .polling_status(&repo)
            .unwrap()
            .active_operation_id
            .as_deref(),
        Some(OPERATION_C)
    );
    assert_git_transport_uninitialized();
}

#[test]
fn index_status_is_stale_when_the_index_holds_an_item_twice() {
    let (fixture, enabled) = contract_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    assert_eq!(
        enabled.service.index_status(&repo).unwrap().state,
        IndexStatusState::Current
    );
    // What a refresh leaves part of the way through: the same item in the
    // primary context and in an item worktree's.
    let index = items::index(enabled.data_directory.path());
    index
        .execute(
            "INSERT INTO contexts (repository_id, kind, branch, worktree_path, head_oid)
             SELECT repository_id, 'active', 'manyhands/document/x',
                    worktree_path || '/.manyhands/worktrees/x', head_oid
               FROM contexts",
            [],
        )
        .unwrap();
    index
        .execute(
            "INSERT INTO discovered_items (
                context_id, item_id, kind, canonical_path, title, activity_at, activity_source
             ) SELECT (SELECT MAX(id) FROM contexts), item_id, kind, canonical_path, title,
                      activity_at, activity_source
                 FROM discovered_items WHERE item_id = ?1",
            [DOCUMENT_A],
        )
        .unwrap();
    drop(index);

    let status = enabled.service.index_status(&repo).unwrap();

    assert_eq!(status.state, IndexStatusState::Stale);
    assert_eq!(status.context_count, Some(2));
    // Still four items: the one held twice is one item.
    assert_eq!(status.item_count, Some(4));
    assert_eq!(
        enabled.service.list_documents(&repo).unwrap().index.state,
        IndexState::Stale
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_remote_operation_that_can_be_resumed_is_listed_and_a_poll_that_ended_is_not() {
    let (_fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    configure_remote(data);
    let id = |last: char| format!("01ARZ3NDEKTSV4RRFFQ69G5FR{last}");
    // Synchronizations a restart is given the reservation for.
    insert_remote_synchronization(
        data,
        &id('2'),
        Some(("ticket", TICKET_A)),
        "interrupted",
        Some("before_fetch"),
        None,
    );
    insert_remote_synchronization(
        data,
        &id('1'),
        None,
        "failed",
        Some("before_fetch"),
        Some("transport_unavailable"),
    );
    // One that published and has not reached the index.
    insert_remote_index_pending(data, &id('0'));
    // One that was cancelled, which is final, and the same with
    // reconciliation left to do.
    insert_remote_synchronization(
        data,
        &id('3'),
        None,
        "cancelled",
        Some("before_fetch"),
        Some("cancelled"),
    );
    insert_remote_synchronization(
        data,
        &id('4'),
        Some(("document", DOCUMENT_A)),
        "cancelled",
        Some("before_fetch"),
        Some("cancelled"),
    );
    items::index(data)
        .execute(
            "UPDATE remote_operation_records SET reconciliation_required = 1
              WHERE operation_ulid = ?1",
            [id('4')],
        )
        .unwrap();
    // Polls that ended, of each kind.
    insert_remote_poll(
        data,
        &id('5'),
        "failed",
        Some("before_transport"),
        Some("transport_unavailable"),
    );
    insert_remote_poll(
        data,
        &id('6'),
        "interrupted",
        Some("after_advertisement"),
        None,
    );
    insert_remote_poll(
        data,
        &id('7'),
        "completed",
        Some("after_batch_commit"),
        Some("completed"),
    );

    let list = service.list_operations(&repo).unwrap();

    let listed: Vec<_> = list
        .items
        .iter()
        .map(|operation| {
            (
                operation.operation_id.clone().unwrap(),
                operation.action,
                operation.state.as_str(),
                operation.item_id.as_deref(),
                operation.next_action,
            )
        })
        .collect();
    let resume = Some(OperationNextAction::Resume);
    assert_eq!(
        listed,
        [
            (
                id('0'),
                OperationAction::SynchronizePrimary,
                "completed",
                None,
                resume
            ),
            (
                id('1'),
                OperationAction::SynchronizePrimary,
                "failed",
                None,
                resume
            ),
            (
                id('2'),
                OperationAction::SynchronizeContext,
                "interrupted",
                Some(TICKET_A),
                resume
            ),
            (
                id('4'),
                OperationAction::SynchronizeContext,
                "cancelled",
                Some(DOCUMENT_A),
                None
            ),
        ]
    );
    assert!(
        list.items
            .iter()
            .all(|operation| operation.family == OperationFamily::Remote
                && operation.worktree.is_none()
                && operation.key_id.is_none())
    );
    assert_eq!(
        list.items[1].failure_code,
        Some(OperationFailureCode::TransportUnavailable)
    );
    assert_eq!(
        list.items[0].completed_step.as_deref(),
        Some("before_discovery")
    );
    // The commit a synchronization published is not an operation's to give.
    let text = serde_json::to_string(&list).unwrap();
    assert!(!text.contains(operations::PUBLISHED_OID), "{text}");
    // None of them holds the reservation.
    assert_eq!(
        service.polling_status(&repo).unwrap().active_operation_id,
        None
    );
    // Those not listed are still found, and offer nothing.
    for last in ['3', '5', '6', '7'] {
        let shown = service
            .show_operation(&repo, operations::operation_id(&id(last)))
            .unwrap();
        assert_eq!(shown.next_action, None, "{shown:?}");
    }

    // However many polls end, the list is what it was.
    for number in 0..40 {
        insert_remote_poll(
            data,
            &format!("01ARZ3NDEKTSV4RRFFQ69G5F{number:02}"),
            if number % 2 == 0 {
                "failed"
            } else {
                "interrupted"
            },
            Some("before_transport"),
            (number % 2 == 0).then_some("protocol_rejected"),
        );
    }
    assert_eq!(service.list_operations(&repo).unwrap(), list);
    assert_git_transport_uninitialized();
}

#[test]
fn a_remote_operation_about_an_item_names_it() {
    let (fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    configure_remote(data);
    let plan = RemoteRefPlan::from_configuration("origin", "main").unwrap();
    let cases = [
        (RemoteOperationAction::Promote, OperationAction::Promote),
        (RemoteOperationAction::Close, OperationAction::Close),
        (
            RemoteOperationAction::SynchronizeContext,
            OperationAction::SynchronizeContext,
        ),
    ];
    for (remote_action, action) in cases {
        let id = OperationId::new();
        let target = RemoteOperationTarget::for_context(
            &plan,
            remote_action,
            AuthoringKind::Document,
            items::item_id(DOCUMENT_A),
        )
        .unwrap();
        reserved(
            service
                .reserve_remote_operation(&fixture.root, id, &target)
                .unwrap(),
        );

        let list = service.list_operations(&repo).unwrap();
        let [operation] = &list.items[..] else {
            panic!("{list:?}");
        };
        assert_eq!(operation.operation_id, Some(id.to_string()));
        assert_eq!(operation.action, action);
        assert_eq!(operation.item_id.as_deref(), Some(DOCUMENT_A));
        assert_eq!(operation.state, "reserved");
        // It holds the reservation; whether anything runs it is not stored.
        assert_eq!(operation.next_action, None);
        assert_eq!(&service.show_operation(&repo, id).unwrap(), operation);

        items::index(data)
            .execute(
                "UPDATE remote_operation_records SET phase = 'cancelled'",
                [],
            )
            .unwrap();
        assert!(service.list_operations(&repo).unwrap().items.is_empty());
    }
    assert_git_transport_uninitialized();
}

#[test]
fn a_synchronization_and_its_index_refresh_share_an_id_and_are_both_listed() {
    let (_fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    configure_remote(data);
    let id = operations::operation_id(OPERATION_A);
    insert_remote_index_pending(data, OPERATION_A);
    // Until the hand-off begins there is only the remote operation.
    let alone = service.list_operations(&repo).unwrap();
    assert_eq!(alone.items.len(), 1);
    assert_eq!(alone.items[0].family, OperationFamily::Remote);
    assert_eq!(service.show_operation(&repo, id).unwrap(), alone.items[0]);

    // The refresh that hands it to the index, as synchronization begins it.
    insert_local(data, Some(OPERATION_A), "refresh", "indexing", None);
    items::index(data)
        .execute("UPDATE operation_records SET target = ''", [])
        .unwrap();

    let list = service.list_operations(&repo).unwrap();
    let pair: Vec<_> = list
        .items
        .iter()
        .map(|operation| {
            (
                operation.operation_id.as_deref(),
                operation.family,
                operation.action,
                operation.next_action,
            )
        })
        .collect();
    let resume = Some(OperationNextAction::Resume);
    assert_eq!(
        pair,
        [
            (
                Some(OPERATION_A),
                OperationFamily::Local,
                OperationAction::Refresh,
                resume
            ),
            (
                Some(OPERATION_A),
                OperationFamily::Remote,
                OperationAction::SynchronizePrimary,
                resume
            ),
        ]
    );
    // Both are listed, so the local one is shown.
    assert_eq!(
        service.show_operation(&repo, id).unwrap().family,
        OperationFamily::Local
    );

    // The refresh completed and the remote record has not been told.
    items::index(data)
        .execute("UPDATE operation_records SET state = 'completed'", [])
        .unwrap();
    let shown = service.show_operation(&repo, id).unwrap();
    assert_eq!(shown.family, OperationFamily::Remote);
    assert_eq!(shown.next_action, resume);
    assert_eq!(service.list_operations(&repo).unwrap().items, [shown]);

    // Told, nothing is left: the local record is shown, as finished.
    items::index(data)
        .execute("UPDATE remote_operation_records SET index_pending = 0", [])
        .unwrap();
    assert!(service.list_operations(&repo).unwrap().items.is_empty());
    let shown = service.show_operation(&repo, id).unwrap();
    assert_eq!(
        (shown.family, shown.next_action),
        (OperationFamily::Local, None)
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_local_synchronization_is_reported_as_one_and_its_target_is_not_published() {
    let (_fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    let oid = operations::PUBLISHED_OID;
    let cases = [
        (
            OPERATION_A,
            format!("synchronization-local-v1/primary/{oid}"),
            OperationAction::SynchronizePrimary,
            None,
        ),
        (
            OPERATION_B,
            format!("synchronization-local-v1/ticket/{TICKET_A}/{oid}"),
            OperationAction::SynchronizeContext,
            Some(TICKET_A),
        ),
        (
            OPERATION_C,
            format!("synchronization-local-v1/document/{DOCUMENT_A}/{oid}"),
            OperationAction::SynchronizeContext,
            Some(DOCUMENT_A),
        ),
        // Any other target leaves the record what it says it is.
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FA3",
            format!("{SENTINEL}/docs/a.md"),
            OperationAction::Refresh,
            None,
        ),
        (
            "01ARZ3NDEKTSV4RRFFQ69G5FA4",
            String::new(),
            OperationAction::Refresh,
            None,
        ),
    ];
    for (id, target, _, _) in &cases {
        insert_local(data, Some(id), "refresh", "created", None);
        items::index(data)
            .execute(
                "UPDATE operation_records SET target = ?1 WHERE operation_ulid = ?2",
                [target.as_str(), id],
            )
            .unwrap();
    }

    let list = service.list_operations(&repo).unwrap();

    assert_eq!(list.items.len(), cases.len());
    for (operation, (id, _, action, item)) in list.items.iter().zip(&cases) {
        assert_eq!(operation.operation_id.as_deref(), Some(*id));
        assert_eq!(operation.family, OperationFamily::Local);
        assert_eq!(operation.action, *action, "{id}");
        assert_eq!(operation.item_id.as_deref(), *item, "{id}");
        assert_eq!(operation.next_action, Some(OperationNextAction::Resume));
        assert_eq!(
            &service
                .show_operation(&repo, operations::operation_id(id))
                .unwrap(),
            operation
        );
    }
    let text = serde_json::to_string(&list).unwrap();
    for hidden in [oid, SENTINEL, "synchronization-local"] {
        assert!(!text.contains(hidden), "{text}");
    }

    // A target under that prefix that synchronization did not write, and
    // one on a record that is not a refresh.
    let damaged = [
        (
            "refresh",
            "synchronization-local-v1/primary/not-an-oid".to_owned(),
        ),
        ("refresh", format!("synchronization-local-v1/ticket/{oid}")),
        (
            "refresh",
            format!("synchronization-local-v1/comment/{TICKET_A}/{oid}"),
        ),
        ("refresh", "synchronization-local-v1/".to_owned()),
        (
            "save_ticket",
            format!("synchronization-local-v1/primary/{oid}"),
        ),
    ];
    for (action, target) in damaged {
        items::index(data)
            .execute(
                "UPDATE operation_records SET action = ?1, target = ?2
                  WHERE operation_ulid = ?3",
                [action, target.as_str(), OPERATION_A],
            )
            .unwrap();
        for error in [
            service.list_operations(&repo).map(drop).unwrap_err(),
            service
                .show_operation(&repo, operations::operation_id(OPERATION_A))
                .map(drop)
                .unwrap_err(),
        ] {
            assert_eq!(error.code(), ResultCode::InternalError, "{target}");
        }
    }
    assert_git_transport_uninitialized();
}

#[test]
fn a_local_operation_carries_the_item_and_working_tree_its_record_holds() {
    let (_fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    let worktree = repo.root().join(".manyhands/worktrees").join(TICKET_A);
    insert_local(
        data,
        Some(OPERATION_A),
        "save_ticket",
        "worktree_observed",
        Some("worktree_observed"),
    );
    items::index(data)
        .execute(
            "UPDATE operation_records SET item_id = ?1, context_path = ?2",
            [TICKET_A, worktree.to_str().unwrap()],
        )
        .unwrap();

    let list = service.list_operations(&repo).unwrap();

    let [operation] = &list.items[..] else {
        panic!("{list:?}");
    };
    assert_eq!(operation.item_id.as_deref(), Some(TICKET_A));
    assert_eq!(operation.worktree.as_deref(), worktree.to_str());
    assert_eq!(operation.key_id, None);
    let value = serde_json::to_value(operation).unwrap();
    assert_eq!(value["worktree"], json!(worktree.to_str().unwrap()));
    assert!(value.get("context").is_none());
    assert_git_transport_uninitialized();
}

#[test]
fn polling_status_reads_only_the_policy_and_the_current_observation() {
    let (_fixture, enabled, repo) = enabled();
    let (service, data) = (&enabled.service, enabled.data_directory.path());
    configure_remote(data);
    // An observation that is no longer the current one is not the latest.
    items::index(data)
        .execute(
            "INSERT INTO remote_observation_batches (
                repository_id, remote_name, primary_branch, configuration_generation,
                observed_at, is_current
             ) SELECT id, 'origin', 'main', 0, 5, 0 FROM repositories",
            [],
        )
        .unwrap();
    assert_eq!(
        service.polling_status(&repo).unwrap().latest_observed_at,
        None
    );

    // A current observation made for a remote the policy no longer names
    // is one the snapshot refuses whole. Its time is still what is stored.
    operations::insert_current_observation(data);
    items::index(data)
        .execute(
            "UPDATE remote_observation_batches SET remote_name = 'elsewhere'
              WHERE is_current = 1",
            [],
        )
        .unwrap();
    assert!(service.remote_snapshot(repo.root()).is_err());
    let status = service.polling_status(&repo).unwrap();
    assert_eq!(status.latest_observed_at.as_deref(), Some(STORED_AT_TEXT));
    assert_eq!(status.interval_seconds, 300);

    // A policy that cannot be one fails the read, and says nothing more.
    let invalid = [
        "interval_seconds = 5",
        "automatic_backoff_seconds = 7",
        "enabled = 2",
        "latest_outcome = 'gone fishing'",
    ];
    for change in invalid {
        let index = items::index(data);
        index
            .execute_batch(&format!(
                "PRAGMA ignore_check_constraints = ON;
                 UPDATE remote_polling_state SET {change};"
            ))
            .unwrap();
        drop(index);
        let error = service.polling_status(&repo).unwrap_err();
        assert_eq!(error.code(), ResultCode::InternalError, "{change}");
        assert_eq!(error.scope.repository.as_deref(), repo.root().to_str());
        assert!(error.recovery.is_empty());
        items::index(data)
            .execute_batch(
                "UPDATE remote_polling_state
                    SET interval_seconds = 300, automatic_backoff_seconds = NULL,
                        enabled = 1, latest_outcome = NULL;",
            )
            .unwrap();
        service.polling_status(&repo).unwrap();
    }

    // So does a registration with no policy at all.
    items::index(data)
        .execute("DELETE FROM remote_polling_state", [])
        .unwrap();
    assert_eq!(
        service.polling_status(&repo).unwrap_err().code(),
        ResultCode::InternalError
    );
    assert_git_transport_uninitialized();
}

#[test]
fn an_index_problem_with_an_empty_stored_path_has_none() {
    let (_fixture, enabled, repo) = enabled();
    items::index(enabled.data_directory.path())
        .execute(
            "INSERT INTO problems (repository_id, context_id, path, code, guidance, observed_at)
             SELECT repository_id, id, '', 'context', ?1, 0 FROM contexts",
            [SENTINEL],
        )
        .unwrap();

    let status = enabled.service.index_status(&repo).unwrap();

    let [problem] = &status.problems[..] else {
        panic!("{:?}", status.problems);
    };
    assert_eq!(problem.code, ProblemCode::ContextProblem);
    assert_eq!(problem.path, None);
    assert_eq!(problem.worktree.as_deref(), repo.root().to_str());
    assert_git_transport_uninitialized();
}
