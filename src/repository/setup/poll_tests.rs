//! The setup poll (design Part G): what it decides on the fetched base, what
//! activation leaves, and what a crash in the middle of it leaves. Local
//! repositories on disk and a stubbed channel; no network, no waits.

use std::time::Duration;

use git2::Oid;

use super::*;
use crate::{
    canonical::Integration,
    repository::{
        EnableRepositoryRequest, OperationId, RepositoryService, SetPublicationRemoteRequest,
        mutation::Mutation,
        setup::{
            record::{self, SetupKind},
            test_world::{
                CURRENT, NOW, OTHER, PRIMARY, SETUP, Stub, World, configuration, configuration_for,
                dump,
            },
        },
        transport::SshTransportErrorKind,
    },
    results::ResultCode,
};

/// A commit a case arranges and has no more use for.
fn done(_: Oid) {}

fn poll(world: &World, stub: &mut Stub) -> Result<SetupPollReport, SetupPollError> {
    world
        .service
        .poll_setup_with(&world.root, false, stub, None, Some(Duration::ZERO))
}

/// What the registry says of the repository's polling: the attempt, its
/// outcome and the next eligible time.
fn attempt(world: &World) -> (Option<i64>, Option<String>, Option<i64>) {
    world
        .db()
        .query_row(
            "SELECT latest_attempt_at, latest_outcome, next_eligible_at FROM remote_polling_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
}

/// A row of the table: the state the case is about, and what the poll
/// decides on it.
struct Row {
    name: &'static str,
    kind: SetupKind,
    published: bool,
    arrange: fn(&World),
    outcome: SetupPollOutcome,
    /// Connections the poll opens.
    fetches: usize,
}

fn merged(world: &World) -> Vec<u8> {
    world.record().unwrap().configuration
}

fn rows() -> Vec<Row> {
    use SetupKind::{Change, Enable};
    use SetupPollOutcome as O;
    fn nothing(_: &World) {}
    vec![
        // G1
        Row {
            name: "G1 a recorded setup is not published",
            kind: Enable,
            published: false,
            arrange: nothing,
            outcome: O::NotPublished,
            fetches: 0,
        },
        Row {
            name: "G1 a recorded change is not published",
            kind: Change,
            published: false,
            arrange: nothing,
            outcome: O::NotPublished,
            fetches: 0,
        },
        // G2
        Row {
            name: "G2 the base holds a configuration that cannot be read",
            kind: Enable,
            published: true,
            arrange: |w| done(w.squash(b"primary_branch = [")),
            outcome: O::ConfigurationInvalid,
            fetches: 1,
        },
        Row {
            name: "G2 the base's configuration names another primary",
            kind: Enable,
            published: true,
            arrange: |w| done(w.squash(&configuration_for("trunk", OTHER, "pull_request"))),
            outcome: O::PrimaryDiffers,
            fetches: 1,
        },
        Row {
            name: "G2 a change's base names another primary",
            kind: Change,
            published: true,
            arrange: |w| done(w.squash(&configuration_for("trunk", OTHER, "pull_request"))),
            outcome: O::PrimaryDiffers,
            fetches: 1,
        },
        // G3
        Row {
            name: "G3 no configuration on the base and the setup branch is gone",
            kind: Enable,
            published: true,
            arrange: |w| w.delete_remote_branch(),
            outcome: O::BranchDeleted,
            fetches: 1,
        },
        Row {
            name: "G3 the change's starting blob on the base and the setup branch is gone",
            kind: Change,
            published: true,
            arrange: |w| w.delete_remote_branch(),
            outcome: O::BranchDeleted,
            fetches: 1,
        },
        // G4
        Row {
            name: "G4 no configuration on the base yet",
            kind: Enable,
            published: true,
            arrange: nothing,
            outcome: O::WaitingForMerge,
            fetches: 1,
        },
        Row {
            name: "G4 the change's starting blob still on the base",
            kind: Change,
            published: true,
            arrange: nothing,
            outcome: O::WaitingForMerge,
            fetches: 1,
        },
        Row {
            name: "G4 the base moved on without the setup",
            kind: Enable,
            published: true,
            arrange: |w| done(w.remote_moves_on()),
            outcome: O::WaitingForMerge,
            fetches: 1,
        },
        // A remote whose primary is gone holds nothing the setup could be in.
        Row {
            name: "G4 the remote has no primary any more, and the setup branch is there",
            kind: Enable,
            published: true,
            arrange: |w| w.delete_remote_primary(),
            outcome: O::WaitingForMerge,
            fetches: 1,
        },
        Row {
            name: "G3 the remote has no primary any more, and the setup branch is gone too",
            kind: Enable,
            published: true,
            arrange: |w| {
                w.delete_remote_primary();
                w.delete_remote_branch();
            },
            outcome: O::BranchDeleted,
            fetches: 1,
        },
        // G5
        Row {
            name: "G5 merged, and the root is on another branch",
            kind: Enable,
            published: true,
            arrange: |w| {
                done(w.squash(&merged(w)));
                w.check_out_another_branch();
            },
            outcome: O::WaitingForCheckout,
            fetches: 1,
        },
        Row {
            name: "G5 merged, and the root has an untracked file",
            kind: Enable,
            published: true,
            arrange: |w| {
                done(w.squash(&merged(w)));
                w.leave_an_untracked_file();
            },
            outcome: O::WaitingForCheckout,
            fetches: 1,
        },
        // G6
        Row {
            name: "G6 merged, and the local primary has a commit the base lacks",
            kind: Enable,
            published: true,
            arrange: |w| {
                done(w.squash(&merged(w)));
                done(w.local_commit());
            },
            outcome: O::Diverged,
            fetches: 1,
        },
        // G7
        Row {
            name: "G7 a merge commit",
            kind: Enable,
            published: true,
            arrange: |w| done(w.merge_commit(&merged(w))),
            outcome: O::Activated,
            fetches: 1,
        },
        Row {
            name: "G7 a squash",
            kind: Enable,
            published: true,
            arrange: |w| done(w.squash(&merged(w))),
            outcome: O::Activated,
            fetches: 1,
        },
        Row {
            name: "G7 a fast-forward of the setup commit",
            kind: Enable,
            published: true,
            arrange: |w| done(w.fast_forward()),
            outcome: O::Activated,
            fetches: 1,
        },
        Row {
            name: "G7 a squash after other work reached the base",
            kind: Enable,
            published: true,
            arrange: |w| {
                done(w.remote_moves_on());
                done(w.squash(&merged(w)));
            },
            outcome: O::Activated,
            fetches: 1,
        },
        Row {
            name: "G7 a change merged",
            kind: Change,
            published: true,
            arrange: |w| done(w.squash(&merged(w))),
            outcome: O::Activated,
            fetches: 1,
        },
    ]
}

#[test]
fn g1_to_g7_the_setup_poll_decides_on_the_base() {
    for row in rows() {
        let what = row.name;
        let world = World::new(row.kind, OTHER, row.published);
        (row.arrange)(&world);
        let before_primary = world.local_primary();
        let before_refs = world.refs();
        let mut stub = world.stub();

        let report = poll(&world, &mut stub).unwrap_or_else(|error| panic!("{what}: {error:?}"));

        assert_eq!(report.outcome, row.outcome, "{what}");
        assert_eq!(report.setup_id.to_string(), SETUP, "{what}");
        assert_eq!(stub.fetches, row.fetches, "{what}: connections");
        // The link is the pull request of the setup branch into the primary.
        assert_eq!(
            report.link.source,
            format!("manyhands/setup/{SETUP}"),
            "{what}"
        );
        assert_eq!(report.link.target, PRIMARY, "{what}");
        if row.outcome == SetupPollOutcome::Activated {
            assert_eq!(report.integration, Some(Integration::PullRequest), "{what}");
            assert!(!report.differs, "{what}");
            assert!(world.record().is_none(), "{what}: the record is gone");
            assert_eq!(
                world.local_primary(),
                world.remote_primary(),
                "{what}: the primary is the base"
            );
            // The attempt is the repository's own, as a direct enable leaves it.
            assert_eq!(attempt(&world).0, None, "{what}: no attempt on activation");
        } else {
            assert_eq!(report.integration, None, "{what}");
            assert!(!report.differs, "{what}");
            // Nothing local changed: the record, the primary and every ref but
            // the private base ref a fetch writes.
            assert!(world.record().is_some(), "{what}: the record is kept");
            assert_eq!(world.local_primary(), before_primary, "{what}");
            let mut after = world.refs();
            after.remove(&world.base_ref());
            let mut before = before_refs;
            before.remove(&world.base_ref());
            assert_eq!(after, before, "{what}: a ref changed");
            assert_eq!(
                world.request().state,
                crate::repository::RequestState::Accepted,
                "{what}"
            );
            // A first-time setup records its attempt where the desktop reads
            // it, with the outcome of an answered advertisement; a change on
            // another remote records nothing (today's poll's record
            // schedules both); no connection, no attempt.
            let (at, outcome, next) = attempt(&world);
            if row.kind == SetupKind::Enable && row.fetches > 0 {
                assert_eq!(at, Some(NOW), "{what}");
                assert_eq!(outcome.as_deref(), Some("completed"), "{what}");
                let interval: i64 = world
                    .db()
                    .query_row(
                        "SELECT interval_seconds FROM remote_polling_state",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert_eq!(next, Some(NOW + interval), "{what}");
            } else {
                assert_eq!((at, outcome, next), (None, None, None), "{what}");
            }
        }
    }
}

/// A key, host or network stop of the fetch is the poll's stop, and an
/// attempt a first-time setup records with the transport's category.
#[test]
fn a_stopped_fetch_is_the_polls_stop_and_a_first_time_setup_records_it() {
    for (kind, code, category) in [
        (
            SshTransportErrorKind::NoSelectedKey,
            ResultCode::NoSelectedKey,
            "selected_key_unavailable",
        ),
        (
            SshTransportErrorKind::TransportUnavailable,
            ResultCode::TransportUnavailable,
            "transport_unavailable",
        ),
    ] {
        let world = World::new(SetupKind::Enable, OTHER, true);
        let mut stub = world.stub();
        stub.fails = Some(kind.clone());

        let error = poll(&world, &mut stub).unwrap_err();

        assert_eq!(error.code(), code, "{kind:?}");
        let (at, outcome, _) = attempt(&world);
        assert_eq!(at, Some(NOW), "{kind:?}");
        assert_eq!(outcome.as_deref(), Some(category), "{kind:?}");
        assert!(world.record().is_some(), "{kind:?}: the record is kept");
    }
}

/// `differs` is true when the merged file is not the one the setup intended,
/// and the merged mode is the one in effect.
#[test]
fn the_merged_file_wins_over_the_intended_one() {
    let world = World::new(SetupKind::Enable, OTHER, true);
    world.squash(&configuration(OTHER, "direct"));
    let mut stub = world.stub();

    let report = poll(&world, &mut stub).unwrap();

    assert_eq!(report.outcome, SetupPollOutcome::Activated);
    assert!(report.differs);
    assert_eq!(report.integration, Some(Integration::Direct));
    // The file in the working tree is the merged one.
    assert_eq!(
        std::fs::read(world.root.join(crate::canonical::CONFIG_PATH)).unwrap(),
        configuration(OTHER, "direct")
    );
}

/// After `Activated` the repository is registered as a direct enable plus a
/// remote selection of the same configuration leaves it, and the setup is
/// gone: its record, its local branch and the private ref. The setup's
/// request, still accepted, is finished.
#[test]
fn activation_leaves_the_repository_as_a_direct_enable_would() {
    // A direct enable, then the selection of `origin`, as the merged file
    // would be written.
    let direct_data = tempfile::tempdir().unwrap();
    let direct_root = tempfile::tempdir().unwrap();
    let direct_root = direct_root.keep().canonicalize().unwrap();
    {
        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head(PRIMARY);
        let repository = git2::Repository::init_opts(&direct_root, &options).unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config
            .set_str("user.email", "fixture@example.invalid")
            .unwrap();
        std::fs::write(direct_root.join("README.md"), "readme").unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = git2::Signature::now("Fixture", "fixture@example.invalid").unwrap();
        repository
            .commit(Some("HEAD"), &signature, &signature, "Base", &tree, &[])
            .unwrap();
        repository
            .remote(CURRENT, "ssh://git@github.com/team/repo.git")
            .unwrap();
        // The setup world has the same two remotes.
        repository
            .remote(OTHER, "ssh://git@github.com/other/repo.git")
            .unwrap();
    }
    let direct = RepositoryService::open_at(direct_data.path()).unwrap();
    direct
        .enable(EnableRepositoryRequest {
            root: direct_root.clone(),
            primary_branch: PRIMARY.into(),
            identity: None,
            operation_id: OperationId::new(),
        })
        .unwrap();
    direct
        .set_publication_remote(SetPublicationRemoteRequest {
            root: direct_root.clone(),
            name: Some(CURRENT.into()),
            operation_id: OperationId::new(),
        })
        .unwrap();
    let intended = std::fs::read(direct_root.join(crate::canonical::CONFIG_PATH)).unwrap();

    // The setup, on a remote that is not the current one: a first-time
    // setup is always on another remote.
    let world = World::with_configuration(SetupKind::Enable, CURRENT, true, intended.clone());
    let setup_commit = world.setup_commit();
    world.squash(&intended);
    let mut stub = world.stub();

    let report = poll(&world, &mut stub).unwrap();

    assert_eq!(report.outcome, SetupPollOutcome::Activated);
    assert!(!report.differs);
    // The setup is gone.
    assert!(world.record().is_none());
    assert_eq!(
        world.local_ref(&world.branch_ref()),
        None,
        "the local branch"
    );
    assert_eq!(world.local_ref(&world.base_ref()), None, "the private ref");
    assert_eq!(
        world.remote_ref(&world.branch_ref()),
        Some(setup_commit),
        "the remote's branch is the forge's to delete"
    );
    // The remote-tracking ref of the setup's remote is the base.
    assert_eq!(
        world.local_ref(&format!("refs/remotes/{CURRENT}/{PRIMARY}")),
        Some(world.remote_primary())
    );
    // The setup's request is finished, as its merged file is the intended
    // one: `already_applied`, storing the setup.
    let request = world.request();
    assert_eq!(request.state, crate::repository::RequestState::Finished);
    assert_eq!(
        request.result.as_ref().unwrap().code,
        ResultCode::AlreadyApplied
    );

    // The rows a direct enable and selection leave.
    let db = world.db();
    let direct_db =
        rusqlite::Connection::open(direct_data.path().join(crate::repository::REGISTRY_FILE))
            .unwrap();
    for (table, skip) in [
        (
            "repositories",
            &["id", "root_path", "enabled_at", "refreshed_at"][..],
        ),
        (
            "configuration_observations",
            &["id", "repository_id", "root_path", "observed_at"][..],
        ),
        ("remote_polling_state", &["repository_id"][..]),
    ] {
        assert_eq!(
            dump(&db, table, skip),
            dump(&direct_db, table, skip),
            "{table}"
        );
    }

    // `repo inspect`, apart from the setup and the roots.
    let mut after = world.service.inspect_repository(&world.root).unwrap();
    let mut direct_inspection = direct.inspect_repository(&direct_root).unwrap();
    assert!(after.setup.is_none());
    for inspection in [&mut after, &mut direct_inspection] {
        inspection.selected_path.clear();
        inspection.root.clear();
        inspection.operations.clear();
    }
    assert_eq!(
        serde_json::to_value(&after).unwrap(),
        serde_json::to_value(&direct_inspection).unwrap()
    );

    // Writes are not refused: a ticket can be created.
    let mut session = crate::repository::keys::SessionCredentials::new(NoPassphrase);
    let created = world.service.execute(
        crate::repository::MutationCall {
            request_id: crate::repository::RequestId::new(),
            confirmation: None,
            mutation: Mutation::TicketCreate(crate::repository::TicketCreateInput {
                root: world.root.clone(),
                id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap(),
                draft: crate::repository::TicketDraft {
                    title: "A title".to_owned(),
                    ticket_type: "task".to_owned(),
                    status: "open".to_owned(),
                    project: None,
                    team: None,
                    body: String::new(),
                },
                deps: Vec::new(),
                parent: None,
                initials: None,
            }),
        },
        &mut session,
        &mut crate::repository::NoProgress,
        &crate::repository::CancellationToken::new(),
    );
    assert_eq!(created.code, ResultCode::Ok, "{}", created.message);
}

/// A session whose credentials are never asked for.
struct NoPassphrase;

impl crate::repository::keys::SessionCredentialProvider for NoPassphrase {
    fn request_passphrase(
        &mut self,
        _: &crate::repository::keys::UnlockRequest,
    ) -> crate::repository::keys::PassphraseResponse {
        panic!("an activation asks for no passphrase")
    }
}

/// A change on another remote: activation does the same on the new remote,
/// and the poll of the current remote is today's, and not the setup poll's:
/// the setup poll records no attempt (it is `g1_to_g7`'s), and the stored plan
/// is the new remote's after completion.
#[test]
fn a_change_on_another_remote_still_polls_the_current_remote() {
    use setup_path::*;
    let first_time = World::new(SetupKind::Enable, OTHER, true);
    let change = World::new(SetupKind::Change, OTHER, true);
    let same_remote = World::with_configuration(
        SetupKind::Change,
        CURRENT,
        true,
        configuration(CURRENT, "pull_request"),
    );
    // The poll a request takes, by the record: a first-time setup instead of
    // today's, a change on another remote after today's, a change on the same
    // remote today's alone.
    assert_eq!(path(&first_time), Some(SetupPollPath::Replace));
    assert_eq!(path(&change), Some(SetupPollPath::After));
    assert_eq!(path(&same_remote), None);
    // No record: today's poll.
    let none = World::new(SetupKind::Enable, OTHER, true);
    {
        let mut db = none.db();
        let tx = db.transaction().unwrap();
        record::delete(&tx, none.repository_id).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(path(&none), None);
    // A complete setup is none: the primary already holds it, as after a
    // `git pull`.
    let complete = World::new(SetupKind::Change, OTHER, true);
    let pulled = complete.squash(&merged(&complete));
    complete.pull(pulled);
    assert_eq!(path(&complete), None);

    // Activation of the change on the new remote: the stored plan names it.
    let mut stub = change.stub();
    change.squash(&merged(&change));
    let report = poll(&change, &mut stub).unwrap();
    assert_eq!(report.outcome, SetupPollOutcome::Activated);
    let remote: Option<String> = change
        .db()
        .query_row("SELECT remote_name FROM remote_polling_state", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(remote.as_deref(), Some(OTHER));
}

mod setup_path {
    use super::*;

    pub(super) fn path(world: &World) -> Option<SetupPollPath> {
        path_of(&world.service, &world.root).unwrap()
    }
}

/// A crash after the fast-forward leaves a record the next poll finishes,
/// and the next gate too, with the primary at the base and no second move.
#[test]
fn a_crash_after_the_fast_forward_is_finished_by_the_next_poll_or_the_gate() {
    for by_the_gate in [false, true] {
        let what = if by_the_gate {
            "the gate"
        } else {
            "the next poll"
        };
        let world = World::new(SetupKind::Enable, OTHER, true);
        let squashed = world.squash(&merged(&world));
        // The state of a crash after the fast-forward and the tracking ref,
        // before the record went: the local primary is the base, in the
        // working tree too; everything of the setup is still there.
        world.pull(squashed);
        assert!(world.record().is_some());

        if by_the_gate {
            let finished =
                crate::repository::setup::complete::finish(&world.service, &world.root).unwrap();
            assert!(finished, "{what}");
        } else {
            let mut stub = world.stub();
            let report = poll(&world, &mut stub).unwrap();
            assert_eq!(report.outcome, SetupPollOutcome::Activated, "{what}");
        }

        assert!(world.record().is_none(), "{what}");
        assert_eq!(world.local_primary(), squashed, "{what}: no second move");
        assert_eq!(world.local_ref(&world.branch_ref()), None, "{what}");
        assert_eq!(world.local_ref(&world.base_ref()), None, "{what}");
        assert_eq!(
            world.request().state,
            crate::repository::RequestState::Finished,
            "{what}"
        );
        let _: Oid = squashed;
    }
}
