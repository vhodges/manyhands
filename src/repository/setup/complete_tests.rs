//! Completion (design Part B, R-17): a setup is finished wherever the local
//! primary already holds it, by the gate and at the end of a poll, with no
//! poll of the setup's own. Local repositories on disk; no network, no waits.

use std::time::Duration;

use git2::Oid;

use super::*;
use crate::{
    repository::{
        Mutation, OperationId, PollPublicationRemoteRequest, RemotePollError, RemotePollInvocation,
        RepoEnableInput, RepositoryOperation, RequestState,
        coordination::setup_lock,
        keys::{PassphraseResponse, SessionCredentialProvider, SessionCredentials, UnlockRequest},
        setup::{
            record::SetupKind,
            test_world::{CURRENT, OTHER, PRIMARY, World, edited_configuration},
        },
        transport::{SshDirection, SshTransportError, SshTransportErrorKind},
    },
    results::ResultCode,
};

/// A session whose credentials are never asked for: the poll's fetch is
/// replaced by a synthetic one.
struct NoPassphrase;

impl SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        panic!("a synthetic fetch asks for no passphrase")
    }
}

/// How the local primary comes to hold the setup: the remote merges it and
/// the root pulls, as a person does after a pull request merges.
#[derive(Clone, Copy, Debug)]
enum Way {
    MergeCommit,
    Squash,
    FastForward,
}

impl Way {
    const ALL: [Self; 3] = [Self::MergeCommit, Self::Squash, Self::FastForward];

    /// Merges on the remote with the configuration `merged`. A fast-forward
    /// holds the setup commit's own file.
    fn merge(self, world: &World, merged: &[u8]) -> Oid {
        match self {
            Self::MergeCommit => world.merge_commit(merged),
            Self::Squash => world.squash(merged),
            Self::FastForward => world.fast_forward(),
        }
    }
}

/// What the gate answers for `repo enable`, which every pending setup
/// refuses: a prepare is gated, and finishes a complete setup first.
fn gate(world: &World) -> ResultCode {
    world
        .service
        .prepare(&Mutation::RepoEnable(RepoEnableInput {
            root: world.root.clone(),
            primary_branch: PRIMARY.into(),
            identity: None,
            slug_prefix: None,
            publication_remote: None,
            integration: None,
            forge: None,
        }))
        .code
}

fn finish_without_waiting(world: &World) -> bool {
    finish_waiting(&world.service, &world.root, Some(Duration::ZERO)).unwrap()
}

#[test]
fn a_setup_completes_when_the_local_primary_holds_it() {
    // A first-time setup and a change, each on another remote, and a change
    // on the same remote; each merged as the setup intended, and edited in
    // review.
    for (kind, remote) in [
        (SetupKind::Enable, OTHER),
        (SetupKind::Change, OTHER),
        (SetupKind::Change, CURRENT),
    ] {
        for way in Way::ALL {
            for edited in [false, true] {
                let what = format!("{kind:?} on {remote}, {way:?}, edited {edited}");
                let world = World::new(kind, remote, true);
                let intended = world.record().unwrap().configuration;
                // A fast-forward holds the setup commit's own file, so it
                // cannot be edited in review.
                let edited = edited && !matches!(way, Way::FastForward);
                let merged = if edited {
                    edited_configuration(remote, "pull_request")
                } else {
                    intended
                };

                // Not before: the primary does not hold the setup, so nothing
                // is finished and the gate refuses.
                assert!(!finish_without_waiting(&world), "{what}: before");
                assert!(world.record().is_some(), "{what}: before");
                assert_eq!(gate(&world), ResultCode::SetupPending, "{what}: before");

                let tip = way.merge(&world, &merged);
                world.pull(tip);

                // Not with the setup branch checked out: HEAD must name the
                // primary.
                let local = world.git();
                local.set_head(&world.branch_ref()).unwrap();
                assert!(!finish_without_waiting(&world), "{what}: setup branch");
                assert!(world.record().is_some(), "{what}: setup branch");
                assert!(world.local_ref(&world.branch_ref()).is_some(), "{what}");
                local.set_head(&format!("refs/heads/{PRIMARY}")).unwrap();

                // A held lock skips the finish and changes no answer: a
                // complete setup refuses nothing.
                {
                    let held =
                        setup_lock(&local, &world.root, RepositoryOperation::Enable).unwrap();
                    assert!(!finish_without_waiting(&world), "{what}: lock held");
                    assert_ne!(gate(&world), ResultCode::SetupPending, "{what}: lock held");
                    assert!(world.record().is_some(), "{what}: lock held");
                    assert_eq!(world.request().state, RequestState::Accepted, "{what}");
                    drop(held);
                }

                // The gate finishes it, before it refuses anything.
                assert_ne!(gate(&world), ResultCode::SetupPending, "{what}: gate");
                assert!(world.record().is_none(), "{what}: finished by the gate");
                assert_eq!(world.local_ref(&world.branch_ref()), None, "{what}");
                assert_eq!(world.local_ref(&world.base_ref()), None, "{what}");
                assert_eq!(
                    world.local_primary(),
                    tip,
                    "{what}: the primary is not moved"
                );
                // The setup request, still accepted, was finished in the same
                // transaction: L4, L5.
                let request = world.request();
                assert_eq!(request.state, RequestState::Finished, "{what}");
                let result = request.result.unwrap();
                assert_eq!(
                    result.code,
                    if edited {
                        ResultCode::ExternalChange
                    } else {
                        ResultCode::AlreadyApplied
                    },
                    "{what}"
                );
                assert_eq!(
                    result.data.as_ref().and_then(|data| data["type"].as_str()),
                    Some("setup"),
                    "{what}"
                );
                // A setup on another remote leaves the stored plan of remote
                // selection: the new remote's.
                if remote == OTHER {
                    let stored: Option<String> = world
                        .db()
                        .query_row("SELECT remote_name FROM remote_polling_state", [], |row| {
                            row.get(0)
                        })
                        .unwrap();
                    assert_eq!(stored.as_deref(), Some(OTHER), "{what}: the plan");
                }
            }
        }
    }
}

/// The setup branch is deleted only at the commit the record names, and a
/// peer's commits on it are kept (K-15).
#[test]
fn a_setup_branch_moved_since_publishing_is_kept() {
    let world = World::new(SetupKind::Enable, OTHER, true);
    let merged = world.record().unwrap().configuration;
    let tip = world.squash(&merged);
    world.pull(tip);
    // The local setup branch is somewhere the record does not name.
    world
        .git()
        .reference(&world.branch_ref(), world.c0, true, "moved")
        .unwrap();

    assert!(finish_without_waiting(&world));

    assert!(world.record().is_none());
    assert_eq!(world.local_ref(&world.branch_ref()), Some(world.c0));
    assert_eq!(world.local_ref(&world.base_ref()), None);
}

/// Reconciliation stopped by an operation in the way: the record stays and
/// the next look finishes it. Nothing is half done: the request is not
/// finished.
#[test]
fn an_operation_in_the_way_stops_the_finish_and_the_next_look_finishes_it() {
    let world = World::new(SetupKind::Enable, OTHER, true);
    let merged = world.record().unwrap().configuration;
    let tip = world.squash(&merged);
    world.pull(tip);
    // Another operation of the repository has not finished.
    world
        .db()
        .execute(
            "INSERT INTO operation_records
                (repository_id,root_path,operation_ulid,action,target,state,completed_step,observed_at)
             VALUES (?1,?2,?3,'enable','','created',NULL,1)",
            rusqlite::params![
                world.repository_id,
                world.root.to_str().unwrap(),
                OperationId::new().to_string()
            ],
        )
        .unwrap();

    let stopped = finish_without_waiting(&world);

    assert!(!stopped);
    assert!(world.record().is_some());
    assert_eq!(world.request().state, RequestState::Accepted);

    // The operation ends; the next look finishes the setup.
    world
        .db()
        .execute(
            "UPDATE operation_records SET state = 'completed' WHERE state = 'created'",
            [],
        )
        .unwrap();
    assert!(finish_without_waiting(&world));
    assert!(world.record().is_none());
    assert_eq!(world.request().state, RequestState::Finished);
}

/// The end of a poll finishes a setup the local primary holds, whatever the
/// poll's own answer, and only after the poll's own remote row is final, so
/// the plan store never fences the calling poll.
#[test]
fn the_end_of_a_poll_finishes_a_complete_setup() {
    // A change on the same remote, merged and pulled: today's poll is the
    // only poll there is, and it finds the remote unreachable.
    let world = World::new(SetupKind::Change, CURRENT, true);
    let merged = world.record().unwrap().configuration;
    let tip = world.squash(&merged);
    world.pull(tip);
    crate::repository::remote::synthetic_fetch_error(SshTransportError {
        root: world.root.clone(),
        remote_name: CURRENT.into(),
        direction: SshDirection::Fetch,
        selected_key_id: None,
        authority: None,
        kind: SshTransportErrorKind::TransportUnavailable,
    });
    let mut session = SessionCredentials::new(NoPassphrase);

    let answer = world.service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: world.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            approval: None,
            restart: false,
        },
        &mut session,
    );

    // The poll's own answer is the transport's, unchanged by the finish.
    assert!(
        matches!(&answer, Err(RemotePollError::Transport(error)) if error.kind == SshTransportErrorKind::TransportUnavailable),
        "{answer:?}"
    );
    assert!(world.record().is_none(), "finished at the end of the poll");
    assert_eq!(world.request().state, RequestState::Finished);
    assert_eq!(world.local_ref(&world.branch_ref()), None);
}
