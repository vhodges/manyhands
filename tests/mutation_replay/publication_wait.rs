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

use std::{path::Path, sync::Arc};

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

/// Whose synchronization holds the reservation a case meets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Peer {
    /// A peer's publication of a comment on another item, the document.
    Document,
    /// A peer's synchronization of the entry's own item: a comment's on
    /// `TICKET_A`, or for a repo sync the peer's own repo sync.
    OwnItem,
}

impl Peer {
    /// The action of the peer's synchronization, for `entry`.
    fn action(self, entry: Entry) -> &'static str {
        match (self, entry) {
            (Self::OwnItem, Entry::RepoSync) => "synchronize_primary",
            _ => "synchronize_context",
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
    held_by(entry, pid, Peer::Document)
}

/// `held`, with the reservation held by `peer`'s synchronization.
fn held_by(entry: Entry, pid: u32, peer: Peer) -> (World, Arc<StepClock>, String) {
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
    let comment_on = |kind, id: &str| {
        Mutation::CommentAdd(CommentAddInput {
            root: world.root.clone(),
            kind,
            id: items::item_id(id),
            comment_id: items::item_id(items::COMMENT_B),
            parent_id: None,
            body: "The peer's comment.\n".to_owned(),
            created_by: None,
        })
    };
    let (mutation, command) = match (peer, entry) {
        (Peer::Document, _) => (
            comment_on(AuthoringKind::Document, mutation::DOCUMENT),
            "comment add",
        ),
        (Peer::OwnItem, Entry::RepoSync) => (
            Mutation::RepoSync(RepoSyncInput {
                root: world.root.clone(),
            }),
            "repo sync",
        ),
        (Peer::OwnItem, _) => (comment_on(AuthoringKind::Ticket, TICKET_A), "comment add"),
    };
    let peer_answer = world.execute(PEER_REQUEST, mutation);
    assert_eq!(
        peer_answer.code,
        ResultCode::InternalError,
        "{peer_answer:?}"
    );
    let holder: String = items::index(world.data.path())
        .query_row(
            "SELECT operation_ulid FROM remote_operation_records",
            [],
            |row| row.get(0),
        )
        .unwrap();
    if command == "comment add" {
        assert_eq!(peer_answer.outcome, Outcome::Partial, "{peer_answer:?}");
        let Some(MutationDataDto::Comment(comment)) = peer_answer.data.as_ref() else {
            panic!("the peer's comment: {peer_answer:?}");
        };
        assert_eq!(comment.synchronization_id, holder);
    }
    assert_eq!(
        left(&world, &holder).remote,
        [(
            true,
            peer.action(entry).to_owned(),
            "fetch_prepared".to_owned()
        )]
    );
    operations::attribute_remote(
        world.data.path(),
        &holder,
        &operations::Attribution {
            request: Some(PEER_REQUEST),
            command: Some(command),
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

/// The peer's synchronization lets the reservation go and its index
/// hand-off is at work, as a synchronization that published leaves them:
/// its remote row is `completed` with its outcome and its index hand-off
/// owed (as `classify_synchronization` writes it), and its local
/// `refresh` row under the same operation is `indexing` under its index
/// owner's first epoch, with a heartbeat fresh by the waiting service's
/// clock, for the process `pid` of this host. The row is the
/// synchronization's item's, or the primary's, at the published commit,
/// and owes only its hand-off (P-4). `data` is the world's data directory.
fn hand_off(data: &Path, holder: &str, pid: u32) {
    let changed = items::index(data)
        .execute(
            "UPDATE remote_operation_records
                SET phase = 'completed', outcome = 'completed',
                    sync_checkpoint = 'discovery_pending', completed_step = 'before_discovery',
                    expected_oid = ?2, local_oid = ?2, tracking_oid = ?2,
                    primary_tracking_oid = ?2, push_oid = ?2, push_advertised_oid = ?2,
                    authoritative_kind = 'published', authoritative_oid = ?2, index_pending = 1
              WHERE operation_ulid = ?1",
            [holder, operations::PUBLISHED_OID],
        )
        .unwrap();
    assert_eq!(changed, 1);
    let (kind, item): (Option<String>, Option<String>) = items::index(data)
        .query_row(
            "SELECT kind, item_id FROM remote_operation_records WHERE operation_ulid = ?1",
            [holder],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let (target, owed) = match (&kind, &item) {
        (Some(kind), Some(item)) => (format!("{kind}/{item}"), item.clone()),
        _ => ("primary".to_owned(), "primary".to_owned()),
    };
    operations::insert_local(
        data,
        Some(holder),
        "refresh",
        "indexing",
        Some("authoritative_observed"),
    );
    let claimed = items::index(data)
        .execute(
            "UPDATE operation_records
                SET target = ?2, item_id = ?3, owed_contexts = ?4, index_owner_epoch = 1
              WHERE operation_ulid = ?1",
            rusqlite::params![
                holder,
                format!(
                    "synchronization-index-v1/{target}/{}",
                    operations::PUBLISHED_OID
                ),
                item,
                format!(r#"["{owed}"]"#),
            ],
        )
        .unwrap();
    assert_eq!(claimed, 1);
    operations::attribute_local(
        data,
        holder,
        &operations::Attribution {
            request: Some(PEER_REQUEST),
            command: Some("comment add"),
            author: None,
            host: Some(&manyhands::repository::this_host()),
            pid: Some(i64::from(pid)),
            started_at: Some(operations::STORED_AT),
        },
        None,
    );
}

/// `hand_off`, to run inside a sleep of the waiting call.
fn hand_off_begins(world: &World, holder: &str, pid: u32) -> impl FnOnce() + Send + 'static {
    let data = world.data.path().to_owned();
    let holder = holder.to_owned();
    move || hand_off(&data, &holder, pid)
}

/// What ends the peer's hand-off, to run inside a sleep of the waiting call:
/// its row completes, and its remote row owes its hand-off no more, as
/// `finish_synchronization_index` records.
fn hand_off_ends(world: &World, holder: &str) -> impl FnOnce() + Send + 'static {
    let data = world.data.path().to_owned();
    let holder = holder.to_owned();
    move || {
        let changed = items::index(&data)
            .execute(
                "UPDATE operation_records SET state = 'completed'
                  WHERE operation_ulid = ?1 AND action = 'refresh'",
                [&holder],
            )
            .unwrap();
        assert_eq!(changed, 1);
        let told = items::index(&data)
            .execute(
                "UPDATE remote_operation_records SET index_pending = 0 WHERE operation_ulid = ?1",
                [&holder],
            )
            .unwrap();
        assert_eq!(told, 1);
    }
}

/// Asserts that `answer` names the peer's hand-off row as its blocker, held
/// by `pid`, `live` as given.
fn assert_names_the_hand_off(
    answer: &Envelope<MutationDataDto>,
    holder: &str,
    pid: u32,
    live: bool,
    what: &str,
) {
    let blocker = blocker_of(answer);
    assert_eq!(blocker["operation_id"], holder, "{what}: {answer:?}");
    assert_eq!(blocker["family"], "local", "{what}");
    assert_eq!(blocker["action"], "refresh", "{what}");
    assert_eq!(blocker["request_id"], PEER_REQUEST, "{what}");
    assert_eq!(blocker["pid"], pid, "{what}");
    assert_eq!(blocker["live"], live, "{what}");
}

/// A publication whose reservation is refused because a peer's local row is
/// unfinished, here the index hand-off of a peer's synchronization of the
/// publication's own item just after it let the reservation go (Table B's
/// first column: the hand-off is a row of its item, or of the primary,
/// P-4), waits for it as a writer's begin waits for a live row: it goes on
/// once the hand-off ends inside its first sleep; while the hand-off stays
/// live it answers `busy` naming the row after the whole budget, reserving
/// nothing; and behind a row whose holder is gone it is answered
/// `recovery_required` at once, naming it stale. Behind another item's
/// hand-off, the document's, it does not wait: it goes on at once.
#[test]
fn a_publication_refused_for_a_peer_s_hand_off_waits_while_its_holder_is_live() {
    for entry in [Entry::ItemSync, Entry::RepoSync] {
        let action = match entry {
            Entry::CommentAdd | Entry::ItemSync => "synchronize_context",
            Entry::RepoSync => "synchronize_primary",
        };
        let peer = Peer::OwnItem.action(entry);
        let holder_only = |world: &World, holder: &str| {
            assert_eq!(
                left(world, holder).remote,
                [(true, peer.to_owned(), "completed".to_owned())],
                "{entry:?}: nothing reserved"
            );
        };

        // The hand-off ends inside the first sleep: the call goes on, past
        // the reservation, to the transport.
        let (world, clock, holder) = held_by(entry, LIVE_PID, Peer::OwnItem);
        hand_off(world.data.path(), &holder, LIVE_PID);
        clock.at_sleep(1, hand_off_ends(&world, &holder));
        let answer = world.execute(REQUEST, entry.mutation(&world));
        assert_eq!(clock.slept(), 1, "{entry:?}: {answer:?}");
        assert_eq!(
            answer.code,
            ResultCode::InternalError,
            "{entry:?}: {answer:?}"
        );
        assert_eq!(
            left(&world, &holder).remote,
            [
                (false, action.to_owned(), "fetch_prepared".to_owned()),
                (true, peer.to_owned(), "completed".to_owned()),
            ],
            "{entry:?}"
        );

        // The hand-off stays live: `busy` after the whole budget.
        let (world, clock, holder) = held_by(entry, LIVE_PID, Peer::OwnItem);
        hand_off(world.data.path(), &holder, LIVE_PID);
        let busy = world.execute(REQUEST, entry.mutation(&world));
        assert_eq!(clock.slept(), 5, "{entry:?}: {busy:?}");
        assert_eq!(
            (busy.outcome, busy.code),
            (Outcome::Error, ResultCode::Busy),
            "{entry:?}: {busy:?}"
        );
        assert_eq!(
            recovery_actions(&busy),
            ["request.retry", "operation.show"],
            "{entry:?}"
        );
        assert_names_the_hand_off(&busy, &holder, LIVE_PID, true, "live");
        assert_eq!(world.record(REQUEST).map(|record| record.state), None);
        holder_only(&world, &holder);

        // The hand-off's holder is gone: `recovery_required` at once.
        let (world, clock, holder) = held_by(entry, LIVE_PID, Peer::OwnItem);
        hand_off(world.data.path(), &holder, DEAD_PID);
        let refused = world.execute(REQUEST, entry.mutation(&world));
        assert_eq!(clock.slept(), 0, "{entry:?}: {refused:?}");
        assert_eq!(
            (refused.outcome, refused.code),
            (Outcome::Blocked, ResultCode::RecoveryRequired),
            "{entry:?}: {refused:?}"
        );
        assert!(
            recovery_actions(&refused).contains(&"operation.show"),
            "{entry:?}: {refused:?}"
        );
        assert_names_the_hand_off(&refused, &holder, DEAD_PID, false, "stale");
        holder_only(&world, &holder);

        // The document's hand-off, live, is another item's: the call goes
        // on at once, past it, to the transport.
        let (world, clock, holder) = held(entry, LIVE_PID);
        hand_off(world.data.path(), &holder, LIVE_PID);
        let answer = world.execute(REQUEST, entry.mutation(&world));
        assert_eq!(clock.slept(), 0, "{entry:?}: {answer:?}");
        assert_eq!(
            answer.code,
            ResultCode::InternalError,
            "{entry:?}: {answer:?}"
        );
        assert_eq!(
            left(&world, &holder).remote,
            [
                (false, action.to_owned(), "fetch_prepared".to_owned()),
                (
                    true,
                    "synchronize_context".to_owned(),
                    "completed".to_owned()
                ),
            ],
            "{entry:?}"
        );
    }
}

/// The incident, in full: two writers comment on two items, and the second
/// one's publication meets the first one's synchronization. It waits while
/// that synchronization holds the reservation; inside its first sleep the
/// peer lets the reservation go and begins its index hand-off, a row of the
/// document (P-4), and the next look goes on past it to the transport: a
/// publication waits for no other item's hand-off, live as it is. The
/// comment is written once; sent again (as the CLI's retry, or a person,
/// sends it), the same request writes nothing again.
#[test]
fn a_comment_s_publication_waits_for_a_peer_s_synchronization_and_not_its_hand_off() {
    let entry = Entry::CommentAdd;
    let (mut world, clock, holder) = held(entry, LIVE_PID);
    clock.at_sleep(1, hand_off_begins(&world, &holder, LIVE_PID));
    let answer = world.execute(REQUEST, entry.mutation(&world));
    assert_eq!(clock.slept(), 1, "{answer:?}");
    assert_eq!(
        (answer.outcome, answer.code),
        (Outcome::Partial, ResultCode::InternalError),
        "{answer:?}"
    );
    let saved = left(&world, &holder);
    assert_eq!(
        saved.remote,
        [
            (
                false,
                "synchronize_context".to_owned(),
                "fetch_prepared".to_owned()
            ),
            (
                true,
                "synchronize_context".to_owned(),
                "completed".to_owned()
            ),
        ]
    );
    assert_eq!(saved.commits, 1, "the comment is saved");
    // The peer's hand-off is still at work: nothing ended it.
    assert!(
        saved.local.contains(&(
            "refresh".to_owned(),
            "indexing".to_owned(),
            Some("authoritative_observed".to_owned())
        )),
        "{saved:?}"
    );

    // Sent again: nothing is written again.
    let again = StepClock::new();
    world.service = mutation::waiting_service(world.data.path(), &again)
        .with_process_probe_for_testing(Arc::new(operations::ProcessTable(vec![LIVE_PID])));
    let resent = world.execute(REQUEST, entry.mutation(&world));
    assert_eq!(again.slept(), 0, "{resent:?}");
    assert_eq!(
        (resent.outcome, resent.code),
        (Outcome::Partial, ResultCode::InternalError),
        "{resent:?}"
    );
    let after = left(&world, &holder);
    assert_eq!(
        (after.commits, after.bindings),
        (saved.commits, saved.bindings)
    );
    assert_eq!(after.remote.len(), 2, "{after:?}");
}
