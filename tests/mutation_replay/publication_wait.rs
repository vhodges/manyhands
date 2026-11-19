//! A publication behind another writer's synchronization: a `comment add`,
//! an `item sync` and a `repo sync` whose reservation another operation
//! holds wait while that operation's holder works, as a writer waits for a
//! live peer's row (`reenter.rs`), and are answered as before once the
//! wait is over, or at once when the holder is gone.
//!
//! No server runs, and none is needed: the reservation is taken before any
//! transport, and a call that gets past it stops at the transport, which
//! this test process never sets up. A publication that completes needs the
//! SSH targets.

use std::sync::Arc;

use manyhands::{
    repository::{
        AuthoringKind, CommentAddInput, ItemSyncInput, Mutation, MutationDataDto, RemoteAddInput,
        RemoteSelectInput, RepoSyncInput, RequestState,
    },
    results::{Envelope, Outcome, ResultCode},
};

use crate::support::{
    items::{self, TICKET_A},
    mutation::{self, REQUEST_1, REQUEST_2, REQUEST_3, StepClock, World, recovery_actions},
    operations,
};

/// The request each case sends, and the peer's whose synchronization holds
/// the reservation.
const REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX7";
const PEER_REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FX8";

/// A pid the waiting service's process table holds, and one it does not.
const LIVE_PID: u32 = 4242;
const DEAD_PID: u32 = 4243;

/// The three ways a publication is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    CommentAdd,
    ItemSync,
    RepoSync,
}

const ENTRIES: [Entry; 3] = [Entry::CommentAdd, Entry::ItemSync, Entry::RepoSync];

impl Entry {
    fn mutation(self, world: &World) -> Mutation {
        match self {
            Self::CommentAdd => Mutation::CommentAdd(CommentAddInput {
                root: world.root.clone(),
                kind: AuthoringKind::Ticket,
                id: items::item_id(TICKET_A),
                comment_id: items::item_id(items::COMMENT_A),
                parent_id: None,
                body: "A comment published behind another writer.\n".to_owned(),
                created_by: None,
            }),
            Self::ItemSync => Mutation::ItemSync(ItemSyncInput {
                root: world.root.clone(),
                kind: AuthoringKind::Ticket,
                id: items::item_id(TICKET_A),
            }),
            Self::RepoSync => Mutation::RepoSync(RepoSyncInput {
                root: world.root.clone(),
            }),
        }
    }

    /// The outcome of its `busy`: a comment is saved all the same, and only
    /// its publication is refused.
    fn busy_outcome(self) -> Outcome {
        match self {
            Self::CommentAdd => Outcome::Partial,
            Self::ItemSync | Self::RepoSync => Outcome::Error,
        }
    }
}

/// A world with a publication remote, ready for `entry`, whose reservation
/// a peer's publication of a comment on another item, the document, holds:
/// its synchronization reserved and stopped at the transport, and its row,
/// which holds the reservation, names the process `pid` of this host. Its
/// service waits on the clock returned, five looks 50 ms apart, and finds
/// only `LIVE_PID` in the process table. Returns the peer's
/// synchronization as well.
fn held(entry: Entry, pid: u32) -> (World, Arc<StepClock>, String) {
    let mut world = World::new();
    let added = world.execute(
        REQUEST_1,
        Mutation::RemoteAdd(RemoteAddInput {
            root: world.root.clone(),
            name: "origin".to_owned(),
            url: "ssh://git@example.invalid/team/project.git".to_owned(),
        }),
    );
    assert_eq!(added.code, ResultCode::Ok, "{added:?}");
    let selected = world.execute(
        REQUEST_2,
        Mutation::RemoteSelect(RemoteSelectInput {
            root: world.root.clone(),
            name: Some("origin".to_owned()),
        }),
    );
    assert_eq!(selected.code, ResultCode::Ok, "{selected:?}");
    if entry == Entry::ItemSync {
        // The ticket's branch and worktree, which an item sync publishes.
        let token = world.token(TICKET_A);
        let saved = world.execute(REQUEST_3, world.save(TICKET_A, &token));
        assert_eq!(saved.code, ResultCode::Ok, "{saved:?}");
    }
    let peer = world.execute(
        PEER_REQUEST,
        Mutation::CommentAdd(CommentAddInput {
            root: world.root.clone(),
            kind: AuthoringKind::Document,
            id: items::item_id(mutation::DOCUMENT),
            comment_id: items::item_id(items::COMMENT_B),
            parent_id: None,
            body: "The peer's comment.\n".to_owned(),
            created_by: None,
        }),
    );
    assert_eq!(
        (peer.outcome, peer.code),
        (Outcome::Partial, ResultCode::InternalError),
        "{peer:?}"
    );
    let Some(MutationDataDto::Comment(comment)) = peer.data.as_ref() else {
        panic!("the peer's comment: {peer:?}");
    };
    let holder = comment.synchronization_id.clone();
    assert_eq!(
        left(&world, &holder).remote,
        [(
            true,
            "synchronize_context".to_owned(),
            "fetch_prepared".to_owned()
        )]
    );
    operations::attribute_remote(
        world.data.path(),
        &holder,
        &operations::Attribution {
            request: Some(PEER_REQUEST),
            command: Some("comment add"),
            author: None,
            host: Some(&manyhands::repository::this_host()),
            pid: Some(i64::from(pid)),
            started_at: Some(operations::STORED_AT),
        },
    );
    let clock = StepClock::new();
    world.service = mutation::waiting_service(world.data.path(), &clock)
        .with_process_probe_for_testing(Arc::new(operations::ProcessTable(vec![LIVE_PID])));
    (world, clock, holder)
}

/// The blocker an envelope's data names.
fn blocker_of(envelope: &Envelope<MutationDataDto>) -> serde_json::Value {
    serde_json::to_value(envelope).unwrap()["data"]["blocker"].clone()
}

/// What a call left behind, with the IDs it minted left out so that two
/// worlds compare: the request's record, each local row's action, state and
/// step, each remote row's action and phase and whether it is `holder`'s,
/// the comment bindings, and the checkpoints on `TICKET_A`'s branch.
#[derive(Debug, PartialEq, Eq)]
struct Left {
    record: Option<RequestState>,
    local: Vec<(String, String, Option<String>)>,
    remote: Vec<(bool, String, String)>,
    bindings: i64,
    commits: usize,
}

fn left(world: &World, holder: &str) -> Left {
    let index = items::index(world.data.path());
    let mut local = index
        .prepare("SELECT action, state, completed_step FROM operation_records")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    local.sort();
    let mut remote = index
        .prepare("SELECT operation_ulid = ?1, action, phase FROM remote_operation_records")
        .unwrap()
        .query_map([holder], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    remote.sort();
    Left {
        record: world.record(REQUEST).map(|record| record.state),
        local,
        remote,
        bindings: index
            .query_row(
                "SELECT COUNT(*) FROM comment_publication_bindings",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        commits: world.branch_commits(TICKET_A),
    }
}

/// Behind a live holder of the reservation a publication waits the whole
/// budget, a poll at a time, with nothing held, and then answers `busy`
/// naming the holder, live. Behind a holder that is gone it does not wait,
/// and answers the same `busy` at once, naming it stale, as every such
/// publication answered before it waited. The wait writes nothing: the two
/// answers, and what each call leaves, are the same.
#[test]
fn a_publication_waits_for_a_live_reservation_holder_and_answers_as_at_once_behind_a_stale_one() {
    for entry in ENTRIES {
        let (live_world, live_clock, live_holder) = held(entry, LIVE_PID);
        let live = live_world.execute(REQUEST, entry.mutation(&live_world));
        let (stale_world, stale_clock, stale_holder) = held(entry, DEAD_PID);
        let stale = stale_world.execute(REQUEST, entry.mutation(&stale_world));

        assert_eq!(
            (live_clock.slept(), stale_clock.slept()),
            (5, 0),
            "{entry:?}: {live:?}"
        );
        for (answer, holder, live, pid) in [
            (&live, &live_holder, true, LIVE_PID),
            (&stale, &stale_holder, false, DEAD_PID),
        ] {
            assert_eq!(
                (answer.outcome, answer.code),
                (entry.busy_outcome(), ResultCode::Busy),
                "{entry:?}: {answer:?}"
            );
            assert_eq!(
                recovery_actions(answer),
                ["request.retry", "operation.show"],
                "{entry:?}"
            );
            assert_eq!(
                answer.recovery[1].operation_id.as_deref(),
                Some(holder.as_str()),
                "{entry:?}"
            );
            let blocker = blocker_of(answer);
            assert_eq!(blocker["operation_id"], holder.as_str(), "{entry:?}");
            assert_eq!(blocker["family"], "remote", "{entry:?}");
            assert_eq!(blocker["action"], "synchronize_context", "{entry:?}");
            assert_eq!(blocker["request_id"], PEER_REQUEST, "{entry:?}");
            assert_eq!(blocker["pid"], pid, "{entry:?}");
            assert_eq!(blocker["live"], live, "{entry:?}");
        }
        // The comment's commit is in each world's own repository.
        let without_commit = |effects: &manyhands::results::Effects| manyhands::results::Effects {
            commit_oid: effects.commit_oid.as_ref().map(|_| String::new()),
            ..effects.clone()
        };
        assert_eq!(
            without_commit(&live.effects),
            without_commit(&stale.effects),
            "{entry:?}"
        );
        let left_live = left(&live_world, &live_holder);
        assert_eq!(left_live, left(&stale_world, &stale_holder), "{entry:?}");
        // Nothing was reserved for the request: the holder's row is the
        // only remote row, and still holds the reservation.
        assert_eq!(
            left_live.remote,
            [(
                true,
                "synchronize_context".to_owned(),
                "fetch_prepared".to_owned()
            )],
            "{entry:?}"
        );
    }
}

/// A holder that finishes while the publication waits lets it go on: it
/// sleeps once, with nothing held, the holder ends meanwhile (here its
/// cancellation is acknowledged, as its next safe point records one), and
/// its next look reserves. It then goes on to the transport, which this
/// test process never set up: it got past the reservation, which no `busy`
/// publication does.
#[test]
fn a_publication_whose_reservation_holder_finishes_inside_the_wait_goes_on() {
    for entry in ENTRIES {
        let (world, clock, holder) = held(entry, LIVE_PID);
        let data = world.data.path().to_owned();
        let ended = holder.clone();
        clock.at_first_sleep(move || {
            let changed = items::index(&data)
                .execute(
                    "UPDATE remote_operation_records
                        SET phase = 'cancelled', outcome = 'cancelled', cancel_requested = 0
                      WHERE operation_ulid = ?1",
                    [&ended],
                )
                .unwrap();
            assert_eq!(changed, 1);
        });
        let answer = world.execute(REQUEST, entry.mutation(&world));
        assert_eq!(clock.slept(), 1, "{entry:?}: {answer:?}");
        assert_eq!(
            answer.code,
            ResultCode::InternalError,
            "{entry:?}: {answer:?}"
        );
        assert!(blocker_of(&answer).is_null(), "{entry:?}: {answer:?}");
        // The request's own synchronization was reserved, and stopped where
        // the peer's had: at the transport.
        let action = match entry {
            Entry::CommentAdd | Entry::ItemSync => "synchronize_context",
            Entry::RepoSync => "synchronize_primary",
        };
        assert_eq!(
            left(&world, &holder).remote,
            [
                (false, action.to_owned(), "fetch_prepared".to_owned()),
                (
                    true,
                    "synchronize_context".to_owned(),
                    "cancelled".to_owned()
                ),
            ],
            "{entry:?}"
        );
    }
}
