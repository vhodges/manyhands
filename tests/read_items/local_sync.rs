use super::support::{
    golden::{self, ContractCase},
    items, mutation,
};
use manyhands::{
    repository::*,
    results::{Envelope, ResultCode, Scope},
};

fn conflict() -> (mutation::World, OperationId) {
    let world = mutation::World::new();
    let id = items::TICKET_A;
    world.execute(mutation::REQUEST_1, world.save(id, &world.token(id)));
    let path = items::ticket_path(id);
    items::write(
        &world.root,
        &path,
        &items::ticket_source(id, "Incoming", ""),
    );
    items::commit(&world.fixture, &[&path], items::COMMITTED_AT);
    let observed = world
        .service
        .observe_local_context_sync(
            &world.resolved(),
            AuthoringKind::Ticket,
            &items::item_id(id),
        )
        .unwrap();
    let result = world.execute(
        mutation::REQUEST_2,
        Mutation::LocalContextSync(LocalContextSyncInput {
            root: world.root.clone(),
            kind: AuthoringKind::Ticket,
            id: items::item_id(id),
            observation: observed.observation,
        }),
    );
    assert_eq!(result.code, ResultCode::RecoveryRequired);
    let id = OperationId::parse(result.operation_id.as_deref().unwrap()).unwrap();
    (world, id)
}
#[test]
fn local_sync_public_reads_are_read_only_and_explicit_content_is_ephemeral() {
    let (world, id) = conflict();
    let before = super::support::repository_and_worktree_snapshot(&world.fixture);
    let requests = world.request_rows();
    let inspect = world
        .service
        .inspect_local_context_sync(&world.resolved(), id)
        .unwrap();
    assert_eq!(inspect.next_action, LocalContextSyncNextAction::Resolve);
    assert!(inspect.can_start_resolution);
    let content = world
        .service
        .read_local_context_conflict(&world.resolved(), id, &inspect.paths[0].token)
        .unwrap();
    assert!(content.local.as_deref().unwrap().contains(mutation::TITLE));
    let printed = serde_json::to_string(&inspect).unwrap();
    assert!(!printed.contains(mutation::TITLE));
    assert!(
        !serde_json::to_value(&content)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("current")
    );
    assert_eq!(world.request_rows(), requests);
    assert_eq!(
        super::support::repository_and_worktree_snapshot(&world.fixture),
        before
    );
    mutation::assert_request_stores_exclude(world.data.path(), &mutation::SENTINELS);
}
#[test]
fn local_sync_missing_wrong_kind_and_absent_context_refuse() {
    let world = mutation::World::new();
    for kind in [AuthoringKind::Document, AuthoringKind::Ticket] {
        let error = world
            .service
            .observe_local_context_sync(&world.resolved(), kind, &items::item_id(items::TICKET_A))
            .unwrap_err();
        assert_eq!(error.code(), ResultCode::ItemNotFound);
    }
    assert!(!world.worktree(items::TICKET_A).exists());
}
#[test]
fn authority_only_change_is_seen_by_fresh_read_session() {
    for mode in [0, 1, 2] {
        let (world, id) = conflict();
        let before = world
            .service
            .inspect_local_context_sync(&world.resolved(), id)
            .unwrap();
        let data = world.data.path().to_owned();
        world.service.set_observation_hook_for_testing(move||{
            items::index(&data).execute(match mode {0=>"UPDATE local_context_sync_records SET semantic_revision=semantic_revision+1",1=>"UPDATE repositories SET enabled_at=enabled_at+1",_=>"UPDATE local_context_sync_records SET owner_epoch=owner_epoch+1"},[]).unwrap();
        });
        let result = world
            .service
            .inspect_local_context_sync(&world.resolved(), id);
        if mode != 2 {
            assert_eq!(result.unwrap_err().code(), ResultCode::ExternalChange);
        } else {
            assert_eq!(result.unwrap().observation, before.observation);
            assert!(
                world
                    .service
                    .read_local_context_conflict(&world.resolved(), id, &before.paths[0].token)
                    .is_ok()
            );
        }
    }
}
#[test]
fn side_read_permission_failure_is_read_only() {
    let (mut world, id) = conflict();
    let inspect = world
        .service
        .inspect_local_context_sync(&world.resolved(), id)
        .unwrap();
    let before = super::support::repository_and_worktree_snapshot(&world.fixture);
    world.service = super::support::FailOnce::at(FailurePoint::LocalSyncBeforeSideRead)
        .open_service(world.data.path());
    let error = match world.service.read_local_context_conflict(
        &world.resolved(),
        id,
        &inspect.paths[0].token,
    ) {
        Err(e) => e,
        Ok(_) => panic!("permission denial must not return side bytes"),
    };
    assert_eq!(error.code(), ResultCode::RepositoryInaccessible);
    assert_eq!(
        super::support::repository_and_worktree_snapshot(&world.fixture),
        before
    );
}

#[test]
fn local_sync_historical_sides_keep_original_tokens_after_owned_path_write() {
    let (mut world, id) = conflict();
    let view = world
        .service
        .inspect_local_context_sync(&world.resolved(), id)
        .unwrap();
    let token = view.paths[0].token.clone();
    let original = world
        .service
        .read_local_context_conflict(&world.resolved(), id, &token)
        .unwrap();
    world.service = super::support::FailOnce::at(FailurePoint::LocalSyncAfterResolutionPath)
        .open_service(world.data.path());
    let result = world.execute(
        mutation::REQUEST_3,
        Mutation::LocalContextResolve(LocalContextResolveInput {
            root: world.root.clone(),
            operation_id: id,
            observation: view.observation.unwrap(),
            resolutions: vec![LocalConflictResolutionInput {
                token: token.clone(),
                source: items::ticket_source(items::TICKET_A, "Resolved", ""),
            }],
        }),
    );
    assert_ne!(result.code, ResultCode::Ok);
    world.reopen();
    let historical = world
        .service
        .read_local_context_conflict(&world.resolved(), id, &token)
        .unwrap();
    assert!(historical.historical);
    assert_eq!(historical.base, original.base);
    assert_eq!(historical.local, original.local);
    assert_eq!(historical.incoming, original.incoming);
    let inspection = world
        .service
        .inspect_local_context_sync(&world.resolved(), id)
        .unwrap();
    assert!(!inspection.can_start_resolution);
    assert_eq!(
        inspection.required_request_id.as_deref(),
        Some(mutation::REQUEST_3)
    );
    assert_eq!(
        inspection.next_action,
        LocalContextSyncNextAction::RetryResolution
    );
}
#[test]
fn local_sync_read_schemas_and_goldens() {
    let (world, id) = conflict();
    let inspect = world
        .service
        .inspect_local_context_sync(&world.resolved(), id)
        .unwrap();
    let content = world
        .service
        .read_local_context_conflict(&world.resolved(), id, &inspect.paths[0].token)
        .unwrap();
    let operation = id.to_string();
    let token = inspect.paths[0].token.clone();
    let observation = inspect.observation.clone().unwrap();
    let parents = [
        inspect.context_parent.clone(),
        inspect.primary_parent.clone(),
    ];
    let placeholders = [
        (world.root.to_str().unwrap(), "<root>"),
        (operation.as_str(), "<operation>"),
        (token.as_str(), "<token>"),
        (observation.as_str(), "<observation>"),
        (parents[0].as_str(), "<context-parent>"),
        (parents[1].as_str(), "<primary-parent>"),
    ];
    golden::assert_contract(
        &ContractCase {
            name: "local_context_sync_inspection",
            data_schema: Some("local_context_sync_inspection.schema.json"),
            placeholders: &placeholders,
            sentinels: &mutation::SENTINELS,
        },
        &Envelope::read_success("context inspect-local", Scope::default(), inspect),
    );
    golden::assert_contract(
        &ContractCase {
            name: "local_context_conflict_content",
            data_schema: Some("local_context_conflict_content.schema.json"),
            placeholders: &placeholders,
            sentinels: &[],
        },
        &Envelope::read_success("context read-local-conflict", Scope::default(), content),
    );
}
