//! The status reads: one registration's index, its polling policy and the
//! operations its three stores hold.

// A read error carries its whole scope by value, as the contract has it.
#![allow(clippy::result_large_err)]

use std::fs;

use manyhands::{
    repository::{
        IndexStatusState, OperationAction, OperationDto, OperationFamily, OperationId,
        OperationNextAction, OperationScope, PollingInterval, PollingOutcome, RecoveryInspection,
        RefreshOutcome, RemoteOperationSafePoint, RemoteOutcomeCategory, RemoteReservation,
        RemoteReservationOutcome, RemoteSafePointOutcome, ResolvedRepository,
        keys::{KeyMaterialAction, RecoveryAction},
    },
    results::{OperationFailureCode, Outcome, ProblemCode, ResultCode},
};
use serde_json::{Value, json};
use support::{
    items::{
        self, DOCUMENT_A, contract_repository, degraded_service, document_source,
        never_refreshed_repository, refresh, refresh_completely, write,
    },
    operations::{
        self, KEY_MATERIAL_SENTINEL, OPERATION_A, OPERATION_ABSENT, OPERATION_B, OPERATION_C,
        STORED_AT_TEXT, configure_remote, insert_key_material, insert_local, insert_remote_poll,
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
    assert_eq!(pending.scope, OperationScope::Repository);
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
        assert_eq!(error.recovery[0].action, "index.rebuild");
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
    insert_key_material(
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
            "scope": "application",
            "action": "generate_key",
            "state": "retained_for_inspection",
            "completed_step": null,
            "next_action": "inspect_retained_files",
            "item_id": null,
            "context": null,
            "updated_at": null,
            "failure_code": "source_missing",
        })
    );
    assert_eq!(remote.family, OperationFamily::Remote);
    assert_eq!(remote.scope, OperationScope::Repository);
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
            "scope": "repository",
            "action": "save_ticket",
            "state": "authoring_checkpoint_observed",
            "completed_step": "authoring_checkpoint_observed",
            "next_action": "resume",
            "item_id": null,
            "context": null,
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
    assert_eq!(completed.scope, OperationScope::Application);
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
    assert_eq!(list.items[0].scope, OperationScope::Application);
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
             VALUES (?1, 'key', 'delete', 'private', 'public', 'pair-written')",
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
