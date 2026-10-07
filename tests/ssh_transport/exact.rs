// Match the scoped transport's owned, redacted error contract.
#![allow(clippy::result_large_err)]
use crate::{
    repository::{transport::*, *},
    session::*,
    ssh_remote::*,
};
use std::{cell::Cell, str::FromStr};

pub const CASES: &[crate::ssh_harness::Case] = &[
    ("exact_fetch_scope", fetch_scope),
    ("exact_direction_and_plan", direction_and_plan),
    ("exact_distinct_push_endpoint", distinct_push_endpoint),
    ("exact_reconnect_rechecks", reconnect_rechecks),
    ("exact_push_rejection_redaction", push_rejection_redaction),
    ("exact_tracking_boundaries", tracking_boundaries),
    (
        "exact_reference_status_rejection",
        reference_status_rejection,
    ),
];
fn plan() -> RemoteRefPlan {
    RemoteRefPlan::from_configuration("origin", "main").unwrap()
}
fn context() -> SynchronizationTarget {
    SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: crate::canonical::ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
    }
}
fn target_ref() -> &'static str {
    "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV"
}
fn tracking_ref() -> &'static str {
    "refs/remotes/origin/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV"
}
fn fetch_scope() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let oid = case.fixture.commit_id();
    for name in [target_ref(), "refs/heads/unrelated", "refs/tags/untouched"] {
        fixed(server.reference(name, oid, true, "fixture"))?;
    }
    let local = fixed(git2::Repository::open(&case.root))?;
    let old = crate::transfer::commit(&case)?;
    fixed(local.reference("refs/remotes/origin/main", old, true, "fixture"))?;
    fixed(local.reference("refs/remotes/origin/unrelated", old, true, "fixture"))?;
    fixed(fixed(local.config())?.set_bool("fetch.prune", true))?;
    fixed(fixed(local.config())?.set_str("remote.origin.tagOpt", "--tags"))?;
    let fetchhead = local.path().join("FETCH_HEAD");
    fixed(std::fs::write(&fetchhead, b"preserved fetch head"))?;
    let (mut session, requests) = session(vec![secret(PASSWORD)]);
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                let before = remote.fresh_advertisement()?;
                remote.fetch_exact(&plan(), &context())?;
                let after = remote.fresh_advertisement()?;
                assert_eq!(before, after);
                Ok(())
            }),
    )?;
    for name in ["refs/remotes/origin/main", tracking_ref()] {
        assert_eq!(fixed(local.refname_to_id(name))?, oid);
    }
    assert_eq!(
        fixed(local.refname_to_id("refs/remotes/origin/unrelated"))?,
        old
    );
    assert_eq!(fixed(local.refname_to_id("refs/heads/pushed"))?, old);
    assert!(local.find_reference("refs/tags/untouched").is_err());
    assert_eq!(fixed(std::fs::read(&fetchhead))?, b"preserved fetch head");
    assert_eq!(requests.borrow().len(), 1);
    assert_eq!(
        case.fixture.accepted_keys(),
        vec![case.fixture.allowed_client_public_key(); 5]
    );
    // A true tracking rewind is allowed, but its local source branch is untouched.
    let parent = fixed(local.find_commit(oid))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let newer = fixed(local.commit(
        Some("refs/heads/rewind-marker"),
        &signature,
        &signature,
        "newer",
        &fixed(parent.tree())?,
        &[&parent],
    ))?;
    fixed(local.reference("refs/remotes/origin/main", newer, true, "fixture"))?;
    // Primary-only transfer must not update the context; absence must not prune it.
    fixed(local.reference(tracking_ref(), old, true, "fixture"))?;
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                remote.fetch_exact(&plan(), &SynchronizationTarget::Primary)
            }),
    )?;
    assert_eq!(fixed(local.refname_to_id(tracking_ref()))?, old);
    assert_eq!(fixed(local.refname_to_id("refs/remotes/origin/main"))?, oid);
    assert_eq!(
        fixed(local.refname_to_id("refs/heads/rewind-marker"))?,
        newer
    );
    fixed(fixed(server.find_reference(target_ref()))?.delete())?;
    fixed(fixed(server.find_reference("refs/heads/main"))?.delete())?;
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                remote.fetch_exact(&plan(), &context())
            }),
    )?;
    assert_eq!(fixed(local.refname_to_id(tracking_ref()))?, old);
    assert_eq!(fixed(local.refname_to_id("refs/remotes/origin/main"))?, oid);
    assert_eq!(fixed(std::fs::read(&fetchhead))?, b"preserved fetch head");
    Ok(())
}
fn direction_and_plan() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                for invalid in [
                    RemoteRefPlan::from_configuration("other", "main").unwrap(),
                    RemoteRefPlan::from_configuration("origin", "other").unwrap(),
                ] {
                    assert_eq!(
                        remote.fetch_exact(&invalid, &context()).unwrap_err().kind,
                        SshTransportErrorKind::ConfigurationInvalid
                    );
                }
                assert_eq!(
                    remote.push_exact(&plan(), &context()).unwrap_err().kind,
                    SshTransportErrorKind::ConfigurationInvalid
                );
                Ok(())
            }),
    )?;
    let mut request = case.request();
    request.direction = SshDirection::Push;
    fixed(
        case.service
            .with_authenticated_remote(request, &mut session, |remote| {
                assert_eq!(
                    remote.fetch_exact(&plan(), &context()).unwrap_err().kind,
                    SshTransportErrorKind::ConfigurationInvalid
                );
                Ok(())
            }),
    )?;
    Ok(())
}
fn distinct_push_endpoint() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let destination = SshRemoteFixture::start()?;
    let push_server = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    let push_primary = root_commit(&push_server, "refs/heads/different")?;
    fixed(push_server.reference("refs/heads/main", push_primary, true, "fixture"))?;
    assert_ne!(push_primary, case.fixture.commit_id());
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &case.fixture.allowed_client_public_key(),
    ))?);
    let local = fixed(git2::Repository::open(&case.root))?;
    fixed(local.remote_set_pushurl("origin", Some(&destination.url())))?;
    let candidate = crate::transfer::commit(&case)?;
    fixed(local.reference(target_ref(), candidate, true, "fixture"))?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                remote.fetch_exact(&plan(), &context())
            }),
    )?;
    let fetch_oid = fixed(local.refname_to_id("refs/remotes/origin/main"))?;
    let mut request = case.request();
    request.direction = SshDirection::Push;
    request.approval = Some(HostApproval {
        authority: SshAuthority {
            host: "127.0.0.1".into(),
            port: destination.address().port(),
        },
        expected: None,
        presented: destination.host_identity(),
    });
    fixed(
        case.service
            .with_authenticated_remote(request, &mut session, |remote| {
                let before = remote.fresh_advertisement()?;
                assert!(
                    before
                        .iter()
                        .any(|(name, oid)| name == "refs/heads/main" && *oid == push_primary)
                );
                assert!(!before.iter().any(|(name, _)| name == target_ref()));
                remote.push_exact(&plan(), &context())?;
                let after = remote.fresh_advertisement()?;
                assert!(
                    after
                        .iter()
                        .any(|(name, oid)| name == target_ref() && *oid == candidate)
                );
                Ok(())
            }),
    )?;
    assert_eq!(
        fixed(local.refname_to_id("refs/remotes/origin/main"))?,
        fetch_oid
    );
    assert!(local.find_reference(tracking_ref()).is_err());
    let source = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert!(source.find_reference(target_ref()).is_err());
    assert_eq!(
        destination.accepted_keys(),
        vec![case.fixture.allowed_client_public_key(); 4]
    );
    Ok(())
}
fn reconnect_rechecks() -> Result<(), FixtureError> {
    for mode in 0..5 {
        let case = Case::new(false)?;
        let (mut session, _) = session(vec![]);
        let error = case
            .service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                // These caller-owned checks/changes happen after a libgit2 call returns.
                let checked = Cell::new(false);
                match mode {
                    0 => {
                        fixed(git2::Repository::open(&case.root))
                            .unwrap()
                            .remote_set_url(
                                "origin",
                                "ssh://fixture@hostile.invalid/secret-endpoint",
                            )
                            .unwrap();
                    }
                    1 => {
                        case.service.clear_shared_key_selection().unwrap();
                    }
                    2 => {
                        std::fs::write(case.fixture.client_key_path(), b"changed key source")
                            .unwrap();
                    }
                    3 => {
                        let db = rusqlite::Connection::open(
                            case.directory.path().join("data/manyhands.sqlite3"),
                        )
                        .unwrap();
                        db.execute("DELETE FROM ssh_host_pins", []).unwrap();
                    }
                    _ => {
                        case.fixture.rotate_host_key().unwrap();
                    }
                }
                checked.set(true);
                let result = if mode % 2 == 0 {
                    remote.fresh_advertisement().map(|_| ())
                } else {
                    remote.fetch_exact(&plan(), &context())
                };
                assert!(checked.get());
                result
            })
            .unwrap_err();
        assert!(match mode {
            0 => error.kind == SshTransportErrorKind::EndpointChanged,
            1 => error.kind == SshTransportErrorKind::SelectionChanged,
            2 => error.kind == SshTransportErrorKind::KeySourceChanged,
            3 => error.kind == SshTransportErrorKind::HostTrustChanged,
            _ => matches!(
                error.kind,
                SshTransportErrorKind::HostReplacementRequired { .. }
                    | SshTransportErrorKind::HostTrustChanged
                    | SshTransportErrorKind::HostApprovalRequired { .. }
            ),
        });
        assert!(!format!("{error:?} {error}").contains("secret-endpoint"));
    }
    Ok(())
}
fn push_rejection_redaction() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    fixed(fixed(server.config())?.set_bool("receive.denyNonFastForwards", true))?;
    case.fixture.hostile_rejection("LEAK-SERVER-TEXT");
    let candidate = crate::transfer::commit(&case)?;
    let local = fixed(git2::Repository::open(&case.root))?;
    fixed(local.reference("refs/heads/main", candidate, true, "fixture"))?;
    let (mut session, _) = session(vec![]);
    let mut request = case.request();
    request.direction = SshDirection::Push;
    let error = case
        .service
        .with_authenticated_remote(request, &mut session, |remote| {
            remote.push_exact(&plan(), &SynchronizationTarget::Primary)
        })
        .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::PushRejected);
    let formatted = format!("{error:?} {error}");
    for sentinel in ["LEAK-SERVER-TEXT", &case.fixture.url(), PASSWORD] {
        assert!(!formatted.contains(sentinel));
    }
    assert_eq!(
        fixed(server.refname_to_id("refs/heads/main"))?,
        case.fixture.commit_id()
    );
    Ok(())
}

fn root_commit(repository: &git2::Repository, name: &str) -> Result<git2::Oid, FixtureError> {
    let tree = fixed(fixed(repository.treebuilder(None))?.write())?;
    let tree = fixed(repository.find_tree(tree))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repository.commit(
        Some(name),
        &signature,
        &signature,
        "different fixture history",
        &tree,
        &[],
    ))
}
fn tracking_boundaries() -> Result<(), FixtureError> {
    use crate::repository::transport::operation_tests::*;
    for mode in 0..5 {
        let case = Case::new(false)?;
        let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
        let oid = case.fixture.commit_id();
        fixed(server.reference(target_ref(), oid, true, "fixture"))?;
        let changed = root_commit(&server, "refs/heads/different")?;
        let local = fixed(git2::Repository::open(&case.root))?;
        let old = crate::transfer::commit(&case)?;
        fixed(local.reference("refs/remotes/origin/main", old, true, "fixture"))?;
        fixed(local.reference(tracking_ref(), old, true, "fixture"))?;
        let root = case.root.clone();
        let server_path = case.fixture.repository_path().to_owned();
        let mut reconnects = 0;
        let mut first = true;
        let hook = install_hook(move |point| {
            let local = git2::Repository::open(&root).unwrap();
            if point == Checkpoint::Reconnected {
                reconnects += 1;
                if reconnects == 1 && mode <= 1 {
                    let server = git2::Repository::open_bare(&server_path).unwrap();
                    if mode == 0 {
                        server
                            .reference("refs/heads/main", changed, true, "fixture")
                            .unwrap();
                    } else {
                        server
                            .find_reference("refs/heads/main")
                            .unwrap()
                            .delete()
                            .unwrap();
                    }
                }
            }
            if (mode == 2 && point == Checkpoint::TrackingDownloaded)
                || (mode == 3 && point == Checkpoint::BeforeTrackingWrite && first)
            {
                local
                    .reference("refs/remotes/origin/main", oid, true, "external change")
                    .unwrap();
                first = false;
            }
            if mode == 4 && point == Checkpoint::TrackingWritten {
                let lock = local.path().join(format!("{}.lock", tracking_ref()));
                std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
                std::fs::write(lock, b"owned fixture lock").unwrap();
            }
        });
        let (mut session, _) = session(vec![]);
        let result =
            case.service
                .with_authenticated_remote(case.request(), &mut session, |remote| {
                    remote.fetch_exact(&plan(), &context())
                });
        drop(hook);
        if mode == 0 {
            fixed(result)?;
            // This download's advertisement, not the previous connection's old OID.
            assert_eq!(
                fixed(local.refname_to_id("refs/remotes/origin/main"))?,
                changed
            );
            assert_eq!(fixed(local.refname_to_id(tracking_ref()))?, oid);
        } else {
            assert_eq!(
                result.unwrap_err().kind,
                SshTransportErrorKind::ProtocolFailure
            );
            assert_eq!(fixed(local.refname_to_id(tracking_ref()))?, old);
            let expected = if mode == 1 { old } else { oid };
            assert_eq!(
                fixed(local.refname_to_id("refs/remotes/origin/main"))?,
                expected
            );
        }
        assert!(!local.path().join("FETCH_HEAD").exists());
        assert_eq!(fixed(local.refname_to_id("refs/heads/pushed"))?, old);
    }
    Ok(())
}
fn reference_status_rejection() -> Result<(), FixtureError> {
    use crate::repository::transport::operation_tests::*;
    let case = Case::new(false)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    fixed(fixed(server.config())?.set_bool("receive.denyNonFastForwards", true))?;
    case.fixture.hostile_rejection("LEAK-SERVER-TEXT");
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                remote.fetch_exact(&plan(), &SynchronizationTarget::Primary)
            }),
    )?;
    let local = fixed(git2::Repository::open(&case.root))?;
    let parent = fixed(local.find_commit(case.fixture.commit_id()))?;
    let tree = fixed(parent.tree())?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(local.commit(
        Some("refs/heads/main"),
        &signature,
        &signature,
        "candidate",
        &tree,
        &[&parent],
    ))?;
    let changed = root_commit(&server, "refs/heads/different")?;
    let path = case.fixture.repository_path().to_owned();
    let hook = install_hook(move |point| {
        if point == Checkpoint::Reconnected {
            git2::Repository::open_bare(&path)
                .unwrap()
                .reference("refs/heads/main", changed, true, "fixture race")
                .unwrap();
        }
    });
    let mut request = case.request();
    request.direction = SshDirection::Push;
    let error = case
        .service
        .with_authenticated_remote(request, &mut session, |remote| {
            remote.push_exact(&plan(), &SynchronizationTarget::Primary)
        })
        .unwrap_err();
    drop(hook);
    assert_eq!(error.kind, SshTransportErrorKind::PushRejected);
    let formatted = format!("{error:?} {error}");
    assert!(!formatted.contains("LEAK-SERVER-TEXT"));
    assert!(!formatted.contains(&case.fixture.url()));
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, changed);
    Ok(())
}
