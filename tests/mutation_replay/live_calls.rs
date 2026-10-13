//! Two calls of one request at once: the boundary does not serialize
//! them. The second raises the record's attempt and calls the domain
//! under the same operation. Where the first has handed off and is
//! indexing, with no lease held, its row is left to it.
//!
//! The first call is stood in for by its row: claimed for indexing, with
//! a heartbeat that is fresh. No hook runs a second call inside the
//! first's refresh, and two threads would race.

use manyhands::{
    repository::{FailurePoint, Mutation, RequestState},
    results::ResultCode,
};

use crate::support::{
    FailOnce,
    items::{self, TICKET_A},
    mutation::{DOCUMENT, REQUEST_1, World},
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
    let index = items::index(world.data.path());
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    index
        .execute(
            "UPDATE operation_records SET state = 'indexing', observed_at = ?2
              WHERE operation_ulid = ?1",
            rusqlite::params![operation, now + 60],
        )
        .unwrap();
    let row = || -> (String, Option<String>, i64) {
        index
            .query_row(
                "SELECT state, completed_step, observed_at FROM operation_records
                  WHERE operation_ulid = ?1",
                [&operation],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
    };
    let claimed = row();
    assert_eq!(claimed.0, "indexing");
    world.reopen();

    let beside = world.execute(REQUEST_1, save(&world, &token));

    // The commit is the request's; the index is the other call's to
    // bring up to date, and its row is as it was.
    assert_eq!(beside.code, ResultCode::DiscoveryPending);
    assert_eq!(beside.operation_id.as_deref(), Some(operation.as_str()));
    assert_eq!(beside.effects.commit_oid, pending.effects.commit_oid);
    assert_eq!(row(), claimed);
    assert_eq!(world.branch_commits(id), 1);
    assert_eq!(
        world.record(REQUEST_1).unwrap().state,
        RequestState::Accepted
    );

    // The other call died: its heartbeat is old. The same request takes
    // the hand-off up and finishes.
    index
        .execute(
            "UPDATE operation_records SET observed_at = 0 WHERE operation_ulid = ?1",
            [&operation],
        )
        .unwrap();
    let done = world.execute(REQUEST_1, save(&world, &token));
    assert_eq!(done.code, ResultCode::Ok);
    assert_eq!(row().0, "completed");
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
