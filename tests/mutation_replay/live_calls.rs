//! Two calls of one request at once: the boundary does not serialize
//! them. The second raises the record's attempt and calls the domain
//! under the same operation. Where the first has handed off and is
//! indexing, with no lease held, its row is left to it.
//!
//! The first call is stood in for by its row: claimed for indexing, with
//! a heartbeat that is fresh. No hook runs a second call inside the
//! first's refresh, and two threads would race.

use manyhands::{
    repository::{FailurePoint, Mutation, RemoteAddInput, RemoteSelectInput, RequestState},
    results::{DiscoveryEffect, Outcome, ResultCode},
};

use crate::support::{
    FailOnce,
    items::TICKET_A,
    mutation::{DOCUMENT, REQUEST_1, World, recovery_actions},
    operations::LiveIndexOwner,
};

/// Saves `id` through `save`, with the save's hand-off failing; stands a
/// live index owner on its row; and sends the request again.
fn a_save_beside_a_live_index_owner(id: &str, save: impl Fn(&World, &str) -> Mutation) {
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    let token = world.token(id);
    let pending = world.execute(REQUEST_1, save(&world, &token));
    assert_eq!(pending.code, ResultCode::DiscoveryPending);
    assert_eq!(world.branch_commits(id), 1);
    let operation = pending.operation_id.clone().unwrap();
    let owner = LiveIndexOwner::on(world.data.path(), &operation);
    world.reopen();

    let beside = world.execute(REQUEST_1, save(&world, &token));

    // The commit is the request's; the index is the other call's to
    // bring up to date, and its row is as it was.
    assert_eq!(beside.code, ResultCode::DiscoveryPending);
    assert_eq!(beside.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(beside.effects.commit_oid, pending.effects.commit_oid);
    // What is owed needs no input: the operation is resumed.
    assert_eq!(beside.recovery[0].action.as_str(), "operation.resume");
    assert_eq!(beside.recovery[0].operation_id, beside.operation_id);
    owner.assert_left_alone();
    assert_eq!(world.branch_commits(id), 1);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    // The other call died: its heartbeat is old. The same request takes
    // the hand-off up and finishes.
    owner.die();
    let done = world.execute(REQUEST_1, save(&world, &token));
    assert_eq!(done.code, ResultCode::Ok);
    assert_eq!(owner.row().0, "completed");
    assert_eq!(world.branch_commits(id), 1);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
}

#[test]
fn a_document_save_s_row_another_call_is_indexing_under_is_left_as_it_is() {
    a_save_beside_a_live_index_owner(DOCUMENT, |world, token| {
        world.document_save(DOCUMENT, token)
    });
}

#[test]
fn a_ticket_save_s_row_another_call_is_indexing_under_is_left_as_it_is() {
    a_save_beside_a_live_index_owner(TICKET_A, |world, token| world.save(TICKET_A, token));
}

const LOCATION: &str = "git@example.invalid:team/project.git";

/// Sends `mutation` with its hand-off failing, stands a live index owner
/// on its row, and sends the request again: the second call records
/// nothing on the row and takes it from no one, says the hand-off is
/// pending, and is never told to recover. Once the owner is dead the same
/// request finishes.
fn a_settings_command_beside_a_live_index_owner(
    arrange: impl Fn(&World),
    mutation: impl Fn(&World) -> Mutation,
) {
    let mut world = World::with_service(|data| {
        FailOnce::at(FailurePoint::BeforeIndexTransactionCommit).open_service(data)
    });
    arrange(&world);
    let pending = world.execute(REQUEST_1, mutation(&world));
    assert_eq!(pending.code, ResultCode::DiscoveryPending);
    let operation = pending.operation_id.clone().unwrap();
    let owner = LiveIndexOwner::on(world.data.path(), &operation);
    let head = world.primary_head();
    world.reopen();

    let beside = world.execute(REQUEST_1, mutation(&world));

    assert_eq!(
        (beside.outcome, beside.code),
        (Outcome::Partial, ResultCode::DiscoveryPending)
    );
    assert_eq!(beside.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(beside.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(beside.effects.commit_oid, pending.effects.commit_oid);
    assert_eq!(beside.data, pending.data);
    // A row a call is at work under is not offered to be abandoned.
    assert_eq!(recovery_actions(&beside), ["operation.resume"]);
    owner.assert_left_alone();
    assert_eq!(world.primary_head(), head);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    owner.die();
    let done = world.execute(REQUEST_1, mutation(&world));
    assert_eq!(
        (done.outcome, done.code),
        (Outcome::Success, ResultCode::Ok)
    );
    assert_eq!(done.effects.commit_oid, pending.effects.commit_oid);
    assert_eq!(owner.row().0, "completed");
    assert_eq!(world.primary_head(), head);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Finished
    );
}

#[test]
fn a_remote_add_s_row_another_call_is_indexing_under_is_left_as_it_is() {
    a_settings_command_beside_a_live_index_owner(
        |_| {},
        |world| {
            Mutation::RemoteAdd(RemoteAddInput {
                root: world.root.clone(),
                name: "origin".to_owned(),
                url: LOCATION.to_owned(),
            })
        },
    );
}

#[test]
fn a_remote_selection_s_row_another_call_is_indexing_under_is_left_as_it_is() {
    a_settings_command_beside_a_live_index_owner(
        |world| {
            git2::Repository::open(&world.root)
                .unwrap()
                .remote("origin", LOCATION)
                .unwrap();
        },
        |world| {
            Mutation::RemoteSelect(RemoteSelectInput {
                root: world.root.clone(),
                name: Some("origin".to_owned()),
            })
        },
    );
}
