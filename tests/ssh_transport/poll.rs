#![allow(clippy::result_large_err)]
use crate::{
    repository::{keys::*, transport::*, *},
    session::*,
    ssh_remote::*,
};
use std::{
    cell::Cell,
    str::FromStr,
    time::{Duration, Instant},
};

pub const CASES: &[crate::ssh_harness::Case] = &[
    ("poll_fetch_set", fetch_set),
    ("poll_absent_primary_and_empty_set", absent_and_empty),
    ("poll_tracking_race", tracking_race),
    ("poll_noncommit_tip", noncommit_tip),
    ("poll_progress_stop", progress_stop),
    ("poll_frozen_policy", frozen_policy),
    ("poll_automatic_blocked_unlock", automatic_blocked_unlock),
    ("poll_download_advertisement", download_advertisement),
    ("poll_partial_guarded_write", partial_guarded_write),
];
fn plan() -> RemoteRefPlan {
    RemoteRefPlan::from_configuration("origin", "main").unwrap()
}
fn context(kind: AuthoringKind) -> RemoteRefTarget {
    plan().context(
        kind,
        &crate::canonical::ItemId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap(),
    )
}
fn no_push(case: &Case) -> Result<(), FixtureError> {
    case.fixture.wait_for_helper_audits()?;
    assert!(case.fixture.receive_updates().is_empty());
    assert_eq!(case.fixture.receive_advertisements(), 0);
    assert!(
        case.fixture
            .commands()
            .iter()
            .all(|command| !command.starts_with(b"git-receive-pack"))
    );
    let probes = vec![
        PASSWORD.as_bytes().to_vec(),
        case.fixture.url().into_bytes(),
    ];
    crate::ssh_privacy::save(&probes)?;
    crate::ssh_privacy::scan(&case.directory.path().join("data"), &probes)?;
    Ok(())
}
fn commit(repository: &git2::Repository, name: &str, seed: u64) -> Result<git2::Oid, FixtureError> {
    // Distinct, deterministic, incompressible graphs make exclusions observable.
    let mut state = seed;
    let bytes: Vec<_> = (0..320_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    let blob = fixed(repository.blob(&bytes))?;
    let mut builder = fixed(repository.treebuilder(None))?;
    fixed(builder.insert("payload", blob, 0o100644))?;
    let tree = fixed(repository.find_tree(fixed(builder.write())?))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let oid = fixed(repository.commit(None, &signature, &signature, "fixture", &tree, &[]))?;
    fixed(repository.reference(name, oid, true, "fixture"))?;
    Ok(oid)
}
fn fetch_set() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let local = fixed(git2::Repository::open(&case.root))?;
    let old = crate::transfer::commit(&case)?;
    let document = context(AuthoringKind::Document);
    let ticket = context(AuthoringKind::Ticket);
    let doc_oid = commit(&server, document.remote_ref(), 21)?;
    let ticket_oid = commit(&server, ticket.remote_ref(), 22)?;
    let bad = "refs/heads/manyhands/ticket/bad";
    let outside = "refs/heads/unrelated";
    let bad_oid = commit(&server, bad, 23)?;
    let outside_oid = commit(&server, outside, 24)?;
    fixed(server.reference("refs/tags/remote-tag", outside_oid, true, "fixture"))?;
    for name in [
        "refs/remotes/origin/main",
        "refs/remotes/origin/unrelated",
        "refs/remotes/origin/stale",
        "refs/tags/local-tag",
        "refs/heads/main",
    ] {
        fixed(local.reference(name, old, true, "fixture"))?;
    }
    fixed(fixed(local.config())?.set_bool("fetch.prune", true))?;
    fixed(fixed(local.config())?.set_str("remote.origin.tagOpt", "--tags"))?;
    let fetchhead = local.path().join("FETCH_HEAD");
    fixed(std::fs::write(&fetchhead, b"preserved"))?;
    let (mut session, requests) = session(vec![]);
    let advertised = fixed(case.service.with_authenticated_remote(
        case.request(),
        &mut session,
        |remote| remote.fetch_poll_set(&plan(), |_| true),
    ))?;
    for (target, oid) in [
        (plan().primary().clone(), case.fixture.commit_id()),
        (document.clone(), doc_oid),
        (ticket.clone(), ticket_oid),
    ] {
        assert!(advertised.contains(&(target.remote_ref().to_owned(), oid)));
        assert_eq!(fixed(local.refname_to_id(target.tracking_ref()))?, oid);
        assert!(local.find_commit(oid).is_ok());
    }
    assert!(advertised.contains(&(bad.into(), bad_oid)));
    for oid in [bad_oid, outside_oid] {
        assert!(local.find_object(oid, None).is_err());
        let tree = fixed(fixed(server.find_commit(oid))?.tree())?;
        assert!(local.find_object(tree.id(), None).is_err());
        for entry in tree.iter() {
            assert!(local.find_object(entry.id(), None).is_err());
        }
    }
    for name in [
        "refs/remotes/origin/manyhands/ticket/bad",
        "refs/tags/remote-tag",
    ] {
        assert!(local.find_reference(name).is_err());
    }
    for name in [
        "refs/remotes/origin/unrelated",
        "refs/remotes/origin/stale",
        "refs/tags/local-tag",
        "refs/heads/main",
        "refs/heads/pushed",
    ] {
        assert_eq!(fixed(local.refname_to_id(name))?, old);
    }
    // Rewind is genuine ancestry, and affects only the tracking ref.
    let parent = fixed(local.find_commit(doc_oid))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let newer = fixed(local.commit(
        Some(document.remote_ref()),
        &signature,
        &signature,
        "newer",
        &fixed(parent.tree())?,
        &[&parent],
    ))?;
    fixed(local.reference(document.tracking_ref(), newer, true, "fixture"))?;
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                remote.fetch_poll_set(&plan(), |_| true)
            }),
    )?;
    assert_eq!(
        fixed(local.refname_to_id(document.tracking_ref()))?,
        doc_oid
    );
    assert_eq!(fixed(local.refname_to_id(document.remote_ref()))?, newer);
    assert_eq!(fixed(std::fs::read(fetchhead))?, b"preserved");
    assert!(requests.borrow().is_empty());
    no_push(&case)
}
fn absent_and_empty() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    fixed(fixed(server.find_reference("refs/heads/main"))?.delete())?;
    let ticket = context(AuthoringKind::Ticket);
    fixed(server.reference(
        ticket.remote_ref(),
        case.fixture.commit_id(),
        true,
        "fixture",
    ))?;
    let local = fixed(git2::Repository::open(&case.root))?;
    let old = crate::transfer::commit(&case)?;
    fixed(local.reference("refs/remotes/origin/main", old, true, "fixture"))?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                remote.fetch_poll_set(&plan(), |_| true)
            }),
    )?;
    assert_eq!(
        fixed(local.refname_to_id(ticket.tracking_ref()))?,
        case.fixture.commit_id()
    );
    assert_eq!(fixed(local.refname_to_id("refs/remotes/origin/main"))?, old);
    fixed(fixed(server.find_reference(ticket.remote_ref()))?.delete())?;
    let outside = commit(&server, "refs/heads/outside", 25)?;
    let helpers = case.fixture.helper_invocations();
    let calls = Cell::new(0);
    let advertised = fixed(case.service.with_authenticated_remote(
        case.request(),
        &mut session,
        |remote| {
            remote.fetch_poll_set(&plan(), |_| {
                calls.set(calls.get() + 1);
                true
            })
        },
    ))?;
    assert!(advertised.contains(&("refs/heads/outside".into(), outside)));
    assert_eq!(case.fixture.helper_invocations() - helpers, 2); // scope + poll advertisement, no download reconnect
    assert_eq!(calls.get(), 0);
    assert!(local.find_commit(outside).is_err());
    assert_eq!(
        fixed(local.refname_to_id(ticket.tracking_ref()))?,
        case.fixture.commit_id()
    );
    assert_eq!(fixed(local.refname_to_id("refs/remotes/origin/main"))?, old);
    assert!(!local.path().join("FETCH_HEAD").exists());
    no_push(&case)
}
fn tracking_race() -> Result<(), FixtureError> {
    for existing in [false, true] {
        let case = Case::new(false)?;
        let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
        let ticket = context(AuthoringKind::Ticket);
        let incoming = commit(&server, ticket.remote_ref(), 26)?;
        let local = fixed(git2::Repository::open(&case.root))?;
        let old = crate::transfer::commit(&case)?;
        let moved = commit(&local, "refs/heads/external", 28)?;
        fixed(local.reference("refs/remotes/origin/main", old, true, "fixture"))?;
        if existing {
            fixed(local.reference(ticket.tracking_ref(), old, true, "fixture"))?;
        }
        let changed = Cell::new(false);
        let (mut session, _) = session(vec![]);
        let error = case
            .service
            .with_authenticated_remote(case.request(), &mut session, |remote| {
                remote.fetch_poll_set(&plan(), |_| {
                    if !changed.replace(true) {
                        local
                            .reference(ticket.tracking_ref(), old, true, "external change")
                            .unwrap();
                        if existing {
                            local
                                .reference(ticket.tracking_ref(), moved, true, "external change")
                                .unwrap();
                        }
                    }
                    true
                })
            })
            .unwrap_err();
        assert!(changed.get());
        assert_eq!(error.kind, SshTransportErrorKind::ProtocolFailure);
        assert_eq!(fixed(local.refname_to_id("refs/remotes/origin/main"))?, old); // validate ALL before first write
        assert_eq!(
            fixed(local.refname_to_id(ticket.tracking_ref()))?,
            if existing { moved } else { old }
        );
        assert!(local.find_commit(incoming).is_ok());
        assert_eq!(fixed(local.refname_to_id("refs/heads/pushed"))?, old);
        assert!(!local.path().join("FETCH_HEAD").exists());
        no_push(&case)?;
    }
    Ok(())
}
fn noncommit_tip() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let blob = fixed(server.blob(b"noncommit"))?;
    let ticket = context(AuthoringKind::Ticket);
    fixed(server.reference(ticket.remote_ref(), blob, true, "fixture"))?;
    let local = fixed(git2::Repository::open(&case.root))?;
    let (mut session, _) = session(vec![]);
    let error = case
        .service
        .with_authenticated_remote(case.request(), &mut session, |remote| {
            remote.fetch_poll_set(&plan(), |_| true)
        })
        .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::ProtocolFailure);
    assert!(local.find_blob(blob).is_ok());
    assert!(local.find_reference("refs/remotes/origin/main").is_err());
    assert!(local.find_reference(ticket.tracking_ref()).is_err());
    assert!(!local.path().join("FETCH_HEAD").exists());
    no_push(&case)
}
fn progress_stop() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let incoming = commit(&server, "refs/heads/main", 27)?;
    case.fixture.pace_transfer(Duration::from_millis(50));
    let calls = Cell::new(0);
    let stopped = Cell::new(None);
    let local = fixed(git2::Repository::open(&case.root))?;
    let (mut session, _) = session(vec![]);
    let error = case
        .service
        .with_authenticated_remote(case.request(), &mut session, |remote| {
            remote.fetch_poll_set(&plan(), |stats| {
                calls.set(calls.get() + 1);
                // Public lease probe proves the transport invokes the hook outside its lease.
                let guard = RepositoryService::hold_lease_for_testing(
                    &case.root,
                    &case.directory.path().join("data"),
                    LeaseKind::Repository,
                )
                .unwrap();
                drop(guard);
                if stats.received_bytes() >= 16_384 {
                    stopped.set(Some(Instant::now()));
                    false
                } else {
                    true
                }
            })
        })
        .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::TransportUnavailable);
    assert!(calls.get() > 0);
    assert!(stopped.get().unwrap().elapsed() < Duration::from_secs(2));
    assert!(local.find_commit(incoming).is_err());
    assert_eq!(fixed(local.references())?.count(), 0);
    assert!(!local.path().join("FETCH_HEAD").exists());
    assert_eq!(case.fixture.helper_invocations(), 2); // no post-stop reconnect
    no_push(&case)
}
fn frozen_policy() -> Result<(), FixtureError> {
    for mode in 0..4 {
        let case = Case::new(false)?;
        let (mut session, requests) = session(vec![]);
        let error = crate::repository::transport::operation_tests::poll_policy(
            &case.service,
            case.request(),
            &mut session,
            true,
            || match mode {
                0 => {
                    let local = git2::Repository::open(&case.root).unwrap();
                    local
                        .remote_set_url("origin", "ssh://fixture@127.0.0.1:1/changed")
                        .unwrap();
                }
                1 => {
                    case.service.clear_shared_key_selection().unwrap();
                }
                2 => {
                    std::fs::write(case.fixture.client_key_path(), b"changed source").unwrap();
                }
                _ => {
                    let key_path = case.directory.path().join("alt");
                    let key = generate_key().unwrap();
                    std::fs::write(
                        &key_path,
                        key.to_openssh(ssh_key::LineEnding::LF).unwrap().as_bytes(),
                    )
                    .unwrap();
                    let RegisterSharedKeyOutcome::Registered(key) = case
                        .service
                        .register_shared_key(RegisterSharedKeyRequest {
                            label: "other".into(),
                            ownership: SharedKeyOwnership::Imported,
                            private_key_path: key_path,
                            public_key_path: None,
                        })
                        .unwrap()
                    else {
                        panic!("fixture registration");
                    };
                    case.service.select_shared_key(key.id).unwrap();
                }
            },
        )
        .unwrap_err();
        assert_eq!(
            error.kind,
            match mode {
                0 => SshTransportErrorKind::EndpointChanged,
                1 => SshTransportErrorKind::NoSelectedKey,
                2 => SshTransportErrorKind::KeySourceChanged,
                _ => SshTransportErrorKind::SelectionChanged,
            }
        );
        assert_eq!(case.fixture.helper_invocations(), 0);
        assert!(requests.borrow().is_empty());
        no_push(&case)?;
    }
    Ok(())
}
fn automatic_blocked_unlock() -> Result<(), FixtureError> {
    let case = Case::new(true)?;
    let (mut session, requests) = session(vec![PassphraseResponse::Cancelled]);
    assert_eq!(
        case.verify(&mut session).unwrap_err().kind,
        SshTransportErrorKind::UnlockCancelled
    );
    let before = requests.borrow().len();
    let error = crate::repository::transport::operation_tests::poll_policy(
        &case.service,
        case.request(),
        &mut session,
        true,
        || {},
    )
    .unwrap_err();
    assert_eq!(error.kind, SshTransportErrorKind::UnlockCancelled);
    assert_eq!(requests.borrow().len(), before);
    assert_eq!(case.fixture.helper_invocations(), 0);
    no_push(&case)
}

// Already-green hardening; not another new-behavior RED cycle.
fn download_advertisement() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let ticket = context(AuthoringKind::Ticket);
    let local = fixed(git2::Repository::open(&case.root))?;
    let (mut session, _) = session(vec![]);
    let mut changed = None;
    let advertised = fixed(case.service.with_authenticated_remote(
        case.request(),
        &mut session,
        |remote| {
            let prior = remote.advertisement()?;
            assert!(!prior.iter().any(|(name, _)| name == ticket.remote_ref()));
            changed = Some(commit(&server, ticket.remote_ref(), 29).unwrap());
            remote.fetch_poll_set(&plan(), |_| true)
        },
    ))?;
    assert!(advertised.contains(&(ticket.remote_ref().into(), changed.unwrap())));
    assert_eq!(
        fixed(local.refname_to_id(ticket.tracking_ref()))?,
        changed.unwrap()
    );
    assert_eq!(case.fixture.helper_invocations(), 2);
    no_push(&case)
}
fn partial_guarded_write() -> Result<(), FixtureError> {
    let case = Case::new(false)?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let ticket = context(AuthoringKind::Ticket);
    commit(&server, ticket.remote_ref(), 30)?;
    let local = fixed(git2::Repository::open(&case.root))?;
    let old = crate::transfer::commit(&case)?;
    for name in ["refs/remotes/origin/main", ticket.tracking_ref()] {
        fixed(local.reference(name, old, true, "fixture"))?;
    }
    let lock = local.path().join(format!("{}.lock", ticket.tracking_ref()));
    let called = Cell::new(false);
    let (mut session, _) = session(vec![]);
    let error = case
        .service
        .with_authenticated_remote(case.request(), &mut session, |remote| {
            remote.fetch_poll_set(&plan(), |_| {
                if !called.replace(true) {
                    std::fs::write(&lock, b"fixture lock").unwrap();
                }
                true
            })
        })
        .unwrap_err();
    assert!(called.get());
    assert_eq!(error.kind, SshTransportErrorKind::ProtocolFailure);
    assert_eq!(
        fixed(local.refname_to_id("refs/remotes/origin/main"))?,
        case.fixture.commit_id()
    );
    assert_eq!(fixed(local.refname_to_id(ticket.tracking_ref()))?, old);
    assert_eq!(fixed(local.refname_to_id("refs/heads/pushed"))?, old);
    assert!(!local.path().join("FETCH_HEAD").exists());
    no_push(&case)
}
