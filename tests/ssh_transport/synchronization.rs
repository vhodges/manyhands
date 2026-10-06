//! Narrow service-consumption smoke cases; full Cycle acceptance is Task 5.
#![allow(clippy::result_large_err)]
use crate::{
    repository::{transport::*, *},
    session::*,
    ssh_remote::*,
};
use std::{cell::Cell, path::Path};
pub const CASES: &[crate::ssh_harness::Case] = &[
    ("synchronization_primary_service", primary_service),
    (
        "synchronization_cancel_before_transfer",
        cancel_before_transfer,
    ),
    (
        "synchronization_push_objects_distinct",
        push_objects_distinct,
    ),
    ("synchronization_push_intent_restart", push_intent_restart),
];
fn prepare() -> Result<Case, FixtureError> {
    let case = Case::new(false)?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    fixed(fixed(repo.config())?.set_str("user.name", "Fixture"))?;
    fixed(fixed(repo.config())?.set_str("user.email", "fixture@example.invalid"))?;
    let plan = fixed(RemoteRefPlan::from_configuration("origin", "main"))?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .with_authenticated_remote(case.request(), &mut session, |r| {
                r.fetch_exact(&plan, &SynchronizationTarget::Primary)
            }),
    )?;
    fixed(repo.reference(
        "refs/heads/main",
        case.fixture.commit_id(),
        false,
        "fixture",
    ))?;
    fixed(repo.set_head("refs/heads/main"))?;
    fixed(repo.checkout_head(Some(git2::build::CheckoutBuilder::new().safe())))?;
    commit_configuration(&repo)?;
    assert!(matches!(
        fixed(case.service.enable(EnableRepositoryRequest {
            root: case.root.clone(),
            primary_branch: "main".into(),
            identity: None,
            operation_id: OperationId::new()
        }))?,
        EnableRepositoryOutcome::AlreadyEnabled
    ));
    Ok(case)
}
fn commit_configuration(repo: &git2::Repository) -> Result<git2::Oid, FixtureError> {
    let mut index = fixed(repo.index())?;
    fixed(index.add_path(Path::new(".manyhands/config.toml")))?;
    fixed(index.write())?;
    let tree = fixed(repo.find_tree(fixed(index.write_tree())?))?;
    let parent = fixed(fixed(repo.head())?.peel_to_commit())?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture configuration",
        &tree,
        &[&parent],
    ))
}
fn request(case: &Case) -> SynchronizeRemoteRequest {
    SynchronizeRemoteRequest {
        root: case.root.clone(),
        operation_id: OperationId::new(),
        target: SynchronizationTarget::Primary,
        approval: case.request().approval,
        restart: false,
    }
}
fn primary_service() -> Result<(), FixtureError> {
    let case = prepare()?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    let oid = fixed(fixed(repo.head())?.peel_to_commit())?.id();
    let (mut session, _) = session(vec![]);
    let req = request(&case);
    assert_eq!(
        fixed(case.service.synchronize_remote(req.clone(), &mut session))?,
        SynchronizationResult::Complete(SynchronizationOutcome::Published {
            target: SynchronizationTarget::Primary,
            oid
        })
    );
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, oid);
    let effects = case.fixture.accepted_keys().len();
    assert_eq!(
        fixed(case.service.synchronize_remote(req, &mut session))?,
        SynchronizationResult::Complete(SynchronizationOutcome::Published {
            target: SynchronizationTarget::Primary,
            oid
        })
    );
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    let next = request(&case);
    assert_eq!(
        fixed(case.service.synchronize_remote(next, &mut session))?,
        SynchronizationResult::Complete(SynchronizationOutcome::AlreadyCurrent {
            target: SynchronizationTarget::Primary,
            oid
        })
    );
    // A real server commit changes worktree bytes; the service performs one FF.
    let parent = fixed(server.find_commit(oid))?;
    let mut builder = fixed(server.treebuilder(Some(&fixed(parent.tree())?)))?;
    fixed(builder.insert("advanced.txt", fixed(server.blob(b"advanced\n"))?, 0o100644))?;
    let tree = fixed(server.find_tree(fixed(builder.write())?))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let advanced = fixed(server.commit(
        Some("refs/heads/main"),
        &sig,
        &sig,
        "advanced",
        &tree,
        &[&parent],
    ))?;
    let req = request(&case);
    // Discovery failure preserves the verified authority and exact-ID replay.
    let db = fixed(rusqlite::Connection::open(
        case.directory.path().join("data").join(REGISTRY_FILE),
    ))?;
    fixed(db.execute_batch("CREATE TRIGGER fail_refresh BEFORE UPDATE ON operation_records WHEN NEW.state='completed' AND NEW.action='refresh' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    let result = fixed(case.service.synchronize_remote(req.clone(), &mut session))?;
    assert_eq!(
        result,
        SynchronizationResult::IndexPending(IndexPending::new(
            SynchronizationOutcome::AlreadyCurrent {
                target: SynchronizationTarget::Primary,
                oid: advanced
            }
        ))
    );
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, advanced);
    assert_eq!(
        fixed(std::fs::read(case.root.join("advanced.txt")))?,
        b"advanced\n"
    );
    fixed(db.execute_batch("DROP TRIGGER fail_refresh"))?;
    let effects = case.fixture.accepted_keys().len();
    assert_eq!(
        fixed(case.service.synchronize_remote(req, &mut session))?,
        SynchronizationResult::Complete(SynchronizationOutcome::AlreadyCurrent {
            target: SynchronizationTarget::Primary,
            oid: advanced
        })
    );
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    Ok(())
}
fn cancel_before_transfer() -> Result<(), FixtureError> {
    let case = prepare()?;
    let req = request(&case);
    let operation = req.operation_id;
    let root = case.root.clone();
    let service = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let called = Cell::new(false);
    let hook = crate::repository::transport::operation_tests::install_hook(move |point| {
        if point == crate::repository::transport::operation_tests::Checkpoint::Reconnected
            && !called.replace(true)
        {
            service.cancel_remote_operation(&root, operation).unwrap();
        }
    });
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    drop(hook);
    let effects = case.fixture.accepted_keys().len();
    let mut restart = req;
    restart.restart = true;
    assert!(matches!(
        case.service.synchronize_remote(restart, &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    let repo = fixed(git2::Repository::open(&case.root))?;
    assert_eq!(
        fixed(repo.refname_to_id("refs/remotes/origin/main"))?,
        case.fixture.commit_id()
    );
    Ok(())
}
fn push_objects_distinct() -> Result<(), FixtureError> {
    let case = prepare()?;
    let destination = SshRemoteFixture::start()?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &case.fixture.allowed_client_public_key(),
    ))?);
    let server = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    let tree = fixed(server.find_tree(fixed(fixed(server.treebuilder(None))?.write())?))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    let oid = fixed(server.commit(Some("refs/heads/unique"), &sig, &sig, "unique", &tree, &[]))?;
    fixed(server.reference("refs/heads/main", oid, true, "fixture"))?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    assert!(repo.find_commit(oid).is_err());
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
    let fetch_oid = fixed(repo.refname_to_id("refs/remotes/origin/main"))?;
    let local_oid = fixed(repo.refname_to_id("refs/heads/main"))?;
    let fetchhead = repo.path().join("FETCH_HEAD");
    fixed(std::fs::write(&fetchhead, b"preserved"))?;
    let mut transport = case.request();
    transport.direction = SshDirection::Push;
    transport.approval = Some(HostApproval {
        authority: SshAuthority {
            host: "127.0.0.1".into(),
            port: destination.address().port(),
        },
        expected: None,
        presented: destination.host_identity(),
    });
    let (mut session, _) = session(vec![]);
    let plan = fixed(RemoteRefPlan::from_configuration("origin", "main"))?;
    assert_eq!(
        fixed(
            case.service
                .with_authenticated_remote(transport, &mut session, |r| r
                    .download_push_target(&plan, &SynchronizationTarget::Primary))
        )?,
        Some(oid)
    );
    assert!(repo.find_commit(oid).is_ok());
    assert_eq!(
        fixed(repo.refname_to_id("refs/remotes/origin/main"))?,
        fetch_oid
    );
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, local_oid);
    assert_eq!(fixed(std::fs::read(fetchhead))?, b"preserved");
    Ok(())
}
fn push_intent_restart() -> Result<(), FixtureError> {
    let case = prepare()?;
    let req = request(&case);
    let dbpath = case.directory.path().join("data").join(REGISTRY_FILE);
    let db = fixed(rusqlite::Connection::open(&dbpath))?;
    fixed(db.execute_batch("CREATE TRIGGER fail_verify BEFORE UPDATE ON remote_operation_records WHEN NEW.sync_checkpoint='push_verified' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    let (mut session, _) = session(vec![]);
    assert!(
        case.service
            .synchronize_remote(req.clone(), &mut session)
            .is_err()
    );
    let repo = fixed(git2::Repository::open(&case.root))?;
    let candidate = fixed(repo.refname_to_id("refs/heads/main"))?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, candidate);
    let effects = case.fixture.accepted_keys().len();
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    fixed(db.execute_batch("DROP TRIGGER fail_verify"))?;
    let mut restart = req;
    restart.restart = true;
    assert_eq!(
        fixed(case.service.synchronize_remote(restart, &mut session))?,
        SynchronizationResult::Complete(SynchronizationOutcome::Published {
            target: SynchronizationTarget::Primary,
            oid: candidate
        })
    );
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, candidate);
    assert_eq!(
        fixed(
            case.service
                .synchronize_remote(request(&case), &mut session)
        )?,
        SynchronizationResult::Complete(SynchronizationOutcome::AlreadyCurrent {
            target: SynchronizationTarget::Primary,
            oid: candidate
        })
    );
    Ok(())
}
