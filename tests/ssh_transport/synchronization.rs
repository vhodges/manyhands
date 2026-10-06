//! Narrow service-consumption smoke cases; full Cycle acceptance is Task 5.
#![allow(clippy::result_large_err)]
use crate::{
    repository::{keys::SessionCredentials, transport::*, *},
    session::*,
    ssh_remote::*,
};
use std::{cell::Cell, path::Path};
pub const CASES: &[crate::ssh_harness::Case] = &[
    ("synchronization_primary_service", primary_service),
    (
        "synchronization_scope_prepare_endpoint_race",
        scope_prepare_endpoint_race,
    ),
    (
        "synchronization_initial_snapshot_not_rebased",
        initial_snapshot_not_rebased,
    ),
    (
        "synchronization_authenticated_action_snapshot",
        authenticated_action_snapshot,
    ),
    (
        "synchronization_prompt_action_snapshot_and_cancel",
        prompt_action_snapshot_and_cancel,
    ),
    (
        "synchronization_same_id_restart_endpoint_changed",
        same_id_restart_endpoint_changed,
    ),
    (
        "synchronization_before_push_endpoint_changed",
        before_push_endpoint_changed,
    ),
    (
        "synchronization_after_push_return_endpoint_changed",
        after_push_return_endpoint_changed,
    ),
    (
        "synchronization_cancelled_replay_before_config",
        cancelled_replay_before_config,
    ),
    (
        "synchronization_cancel_before_transfer",
        cancel_before_transfer,
    ),
    (
        "synchronization_push_objects_distinct",
        push_objects_distinct,
    ),
    ("synchronization_push_intent_restart", push_intent_restart),
    (
        "synchronization_context_inherited_absent_push",
        context_inherited_absent_push,
    ),
    (
        "synchronization_cancelled_context_equal_candidate",
        cancelled_context_equal_candidate,
    ),
    (
        "synchronization_context_deleted_before_push",
        context_deleted_before_push,
    ),
    (
        "synchronization_fetch_observed_context_deleted",
        fetch_observed_context_deleted,
    ),
    (
        "synchronization_distinct_push_context_deleted",
        distinct_push_context_deleted,
    ),
    (
        "synchronization_push_history_generation_fencing",
        push_history_generation_fencing,
    ),
    (
        "synchronization_ambiguous_cancelled_context_absent",
        ambiguous_cancelled_context_absent,
    ),
    (
        "synchronization_context_identity_preservation",
        context_identity_preservation,
    ),
    (
        "synchronization_context_absence_boundaries",
        context_absence_boundaries,
    ),
    (
        "synchronization_missing_remote_primary",
        missing_remote_primary,
    ),
    (
        "synchronization_service_divergence_preservation",
        service_divergence_preservation,
    ),
    ("synchronization_cancel_after_fetch", cancel_after_fetch),
    (
        "synchronization_local_prepared_mismatch_restart",
        local_prepared_mismatch_restart,
    ),
    (
        "synchronization_completed_refresh_index_flag_replay",
        completed_refresh_index_flag_replay,
    ),
    (
        "synchronization_cancelled_push_new_id_proof",
        cancelled_push_new_id_proof,
    ),
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

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const CONTEXT_REF: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV";
fn database(case: &Case) -> Result<rusqlite::Connection, FixtureError> {
    fixed(rusqlite::Connection::open(
        case.directory.path().join("data").join(REGISTRY_FILE),
    ))
}
fn publish_primary(case: &Case) -> Result<(), FixtureError> {
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        fixed(case.service.synchronize_remote(request(case), &mut session))?,
        SynchronizationResult::Complete(SynchronizationOutcome::Published { .. })
    ));
    Ok(())
}
fn context_fixture() -> Result<(Case, ItemContext), FixtureError> {
    let case = prepare()?;
    publish_primary(&case)?;
    let context = author_context(&case)?;
    Ok((case, context))
}
fn author_context(case: &Case) -> Result<ItemContext, FixtureError> {
    let target = AuthoringTarget {
        root: case.root.clone(),
        kind: AuthoringKind::Ticket,
        item_id: fixed(ITEM.parse())?,
        intent: ContextIntent::Create,
        operation_id: OperationId::new(),
    };
    let saved = fixed(case.service.save_ticket(SaveTicketRequest {
        target,
        draft: TicketDraft {
            title: "Fixture".into(),
            body: "fixture".into(),
            ticket_type: "task".into(),
            status: "open".into(),
            project: None,
            team: None,
        },
        expected_path: ExpectedPathObservation::Missing,
    }))?;
    let context = match saved {
        SaveOutcome::Saved { context, .. } | SaveOutcome::IndexPending { context, .. } => context,
        _ => return Err(FixtureError),
    };
    let linked = fixed(git2::Repository::open(&context.worktree))?;
    let mut index = fixed(linked.index())?;
    fixed(index.read_tree(&fixed(fixed(linked.head())?.peel_to_tree())?))?;
    fixed(index.write())?;
    Ok(context)
}
fn context_request(case: &Case) -> Result<SynchronizeRemoteRequest, FixtureError> {
    let mut req = request(case);
    req.target = SynchronizationTarget::Context {
        kind: AuthoringKind::Ticket,
        item_id: fixed(ITEM.parse())?,
    };
    Ok(req)
}
// Digests compare actual bytes without printing/persisting canonical contents or
// worktree paths. Tracking refs may legitimately change during completed fetch.
#[derive(Debug, PartialEq, Eq)]
struct TargetState {
    local_refs: [u8; 32],
    head: [u8; 32],
    index: [u8; 32],
    files: [u8; 32],
    status: [u8; 32],
}
fn target_state(root: &Path, worktree: &Path) -> Result<TargetState, FixtureError> {
    let repository = fixed(git2::Repository::open(root))?;
    let linked = fixed(git2::Repository::open(worktree))?;
    let mut references = fixed(repository.references_glob("refs/heads/*"))?
        .map(|r| {
            let r = fixed(r)?;
            Ok((r.name().unwrap().to_owned(), r.target()))
        })
        .collect::<Result<Vec<_>, FixtureError>>()?;
    references.sort();
    let local_refs = *blake3::hash(format!("{references:?}").as_bytes()).as_bytes();
    let head = *blake3::hash(&fixed(std::fs::read(linked.path().join("HEAD")))?).as_bytes();
    let index_bytes = fixed(std::fs::read(linked.path().join("index")))?;
    let index = *blake3::hash(&index_bytes).as_bytes();
    let mut files = blake3::Hasher::new();
    for entry in fixed(linked.index())?.iter() {
        let path = fixed(std::str::from_utf8(&entry.path))?;
        files.update(&entry.path);
        files.update(&fixed(std::fs::read(worktree.join(path)))?);
    }
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let mut statuses = Vec::new();
    for entry in fixed(linked.statuses(Some(&mut options)))?.iter() {
        statuses.push((entry.path().unwrap().to_owned(), entry.status().bits()));
    }
    statuses.sort();
    Ok(TargetState {
        local_refs,
        head,
        index,
        files: *files.finalize().as_bytes(),
        status: *blake3::hash(format!("{statuses:?}").as_bytes()).as_bytes(),
    })
}
fn child_commit(
    repository: &git2::Repository,
    parent: git2::Oid,
    path: &str,
    bytes: &[u8],
) -> Result<git2::Oid, FixtureError> {
    let parent = fixed(repository.find_commit(parent))?;
    let mut builder = fixed(repository.treebuilder(Some(&fixed(parent.tree())?)))?;
    fixed(builder.insert(path, fixed(repository.blob(bytes))?, 0o100644))?;
    let tree = fixed(repository.find_tree(fixed(builder.write())?))?;
    let sig = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(repository.commit(None, &sig, &sig, "fixture child", &tree, &[&parent]))
}
fn remote_advance(case: &Case) -> Result<git2::Oid, FixtureError> {
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let old = fixed(server.refname_to_id("refs/heads/main"))?;
    let new = child_commit(&server, old, "advanced.txt", b"remote advance\n")?;
    fixed(server.reference("refs/heads/main", new, true, "fixture"))?;
    Ok(new)
}
fn context_identity_preservation() -> Result<(), FixtureError> {
    let (case, context) = context_fixture()?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    let linked = fixed(git2::Repository::open(&context.worktree))?;
    let oid = fixed(fixed(linked.head())?.peel_to_commit())?.id();
    fixed(linked.reference("refs/heads/wrong-context", oid, false, "fixture"))?;
    fixed(linked.set_head("refs/heads/wrong-context"))?;
    let before = target_state(&case.root, &context.worktree)?;
    let effects = case.fixture.accepted_keys().len();
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service
            .synchronize_remote(context_request(&case)?, &mut session),
        Err(SynchronizationError::TargetNotMaterialized)
    ));
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    fixed(linked.set_head(CONTEXT_REF))?;
    // Keep a real deterministic worktree but register its stable name elsewhere.
    let alternate = case.root.join(".manyhands/worktrees/registered-elsewhere");
    fixed(std::fs::create_dir(&alternate))?;
    fixed(std::fs::write(alternate.join(".git"), b"fixture decoy"))?;
    fixed(std::fs::write(
        repo.commondir().join("worktrees").join(ITEM).join("gitdir"),
        format!("{}\n", alternate.join(".git").display()),
    ))?;
    assert_eq!(fixed(repo.find_worktree(ITEM))?.path(), alternate);
    let before = target_state(&case.root, &context.worktree)?;
    assert!(matches!(
        case.service
            .synchronize_remote(context_request(&case)?, &mut session),
        Err(SynchronizationError::TargetNotMaterialized)
    ));
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    assert_eq!(
        fixed(std::fs::read(alternate.join(".git")))?,
        b"fixture decoy"
    );
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    Ok(())
}
fn context_absence_boundaries() -> Result<(), FixtureError> {
    // Fresh complete absence ignores stale tracking and permits first publication.
    let (case, context) = context_fixture()?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    let local = fixed(repo.refname_to_id(CONTEXT_REF))?;
    fixed(repo.reference(
        &format!("refs/remotes/origin/manyhands/ticket/{ITEM}"),
        case.fixture.commit_id(),
        false,
        "stale fixture tracking",
    ))?;
    let (mut session, _) = session(vec![]);
    let result = fixed(
        case.service
            .synchronize_remote(context_request(&case)?, &mut session),
    )?;
    assert_eq!(
        result,
        SynchronizationResult::Complete(SynchronizationOutcome::Published {
            target: context_request(&case)?.target,
            oid: local
        })
    );
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert_eq!(fixed(server.refname_to_id(CONTEXT_REF))?, local);
    fixed(fixed(server.find_reference(CONTEXT_REF))?.delete())?;
    // A new primary descendant could otherwise FF this context before noticing
    // Push deletion; the publication guard must run before either local effect.
    let advanced = child_commit(
        &server,
        local,
        "publication-guard.txt",
        b"must not integrate\n",
    )?;
    fixed(server.reference("refs/heads/main", advanced, true, "fixture"))?;
    let before = target_state(&case.root, &context.worktree)?;
    assert!(matches!(
        case.service
            .synchronize_remote(context_request(&case)?, &mut session),
        Err(SynchronizationError::RemoteContextDeleted)
    ));
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    assert!(server.find_reference(CONTEXT_REF).is_err());
    assert_eq!(
        fixed(case.service.remote_snapshot(&case.root))?
            .publication_evidence_for(AuthoringKind::Ticket, &fixed(ITEM.parse())?),
        RemotePublicationEvidence::NeverPublished
    ); // Push evidence must not manufacture a Fetch-present observation.
    // Cache history loss cannot manufacture first-publication evidence.
    let (unknown, context) = context_fixture()?;
    fixed(database(&unknown)?.execute_batch("UPDATE remote_polling_state SET history_unknown=1"))?;
    let before = target_state(&unknown.root, &context.worktree)?;
    assert!(matches!(
        unknown
            .service
            .synchronize_remote(context_request(&unknown)?, &mut session),
        Err(SynchronizationError::HistoryUnknown)
    ));
    assert_eq!(target_state(&unknown.root, &context.worktree)?, before);
    let server = fixed(git2::Repository::open_bare(
        unknown.fixture.repository_path(),
    ))?;
    assert!(server.find_reference(CONTEXT_REF).is_err());
    Ok(())
}
fn missing_remote_primary() -> Result<(), FixtureError> {
    for context in [false, true] {
        let (case, worktree, target) = if context {
            let (case, ctx) = context_fixture()?;
            let target = context_request(&case)?;
            (case, ctx.worktree, target)
        } else {
            let case = prepare()?;
            publish_primary(&case)?;
            let target = request(&case);
            let worktree = case.root.clone();
            (case, worktree, target)
        };
        let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
        fixed(fixed(server.find_reference("refs/heads/main"))?.delete())?;
        let before = target_state(&case.root, &worktree)?;
        let (mut session, _) = session(vec![]);
        assert!(matches!(
            case.service.synchronize_remote(target, &mut session),
            Err(SynchronizationError::PrimaryMissing)
        ));
        assert_eq!(target_state(&case.root, &worktree)?, before);
        assert!(server.find_reference("refs/heads/main").is_err());
    }
    Ok(())
}
fn service_divergence_preservation() -> Result<(), FixtureError> {
    // Remote-context divergence and virtual-primary divergence after a valid
    // prospective context FF must both fail before any local branch update.
    for virtual_primary in [false, true] {
        let (case, context) = context_fixture()?;
        let (mut session, _) = session(vec![]);
        fixed(
            case.service
                .synchronize_remote(context_request(&case)?, &mut session),
        )?;
        let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
        let base = fixed(server.refname_to_id("refs/heads/main"))?;
        let local = fixed(server.refname_to_id(CONTEXT_REF))?;
        let remote = child_commit(
            &server,
            if virtual_primary { local } else { base },
            "context-remote.txt",
            b"remote context\n",
        )?;
        fixed(server.reference(CONTEXT_REF, remote, true, "fixture"))?;
        if virtual_primary {
            let primary = child_commit(&server, base, "primary-remote.txt", b"primary diverged\n")?;
            fixed(server.reference("refs/heads/main", primary, true, "fixture"))?;
        }
        let before = target_state(&case.root, &context.worktree)?;
        assert!(matches!(
            case.service
                .synchronize_remote(context_request(&case)?, &mut session),
            Err(SynchronizationError::MergeRequired { .. })
        ));
        assert_eq!(target_state(&case.root, &context.worktree)?, before);
        let repo = fixed(git2::Repository::open(&case.root))?;
        assert_eq!(fixed(repo.refname_to_id(CONTEXT_REF))?, local);
        assert_eq!(
            fixed(repo.refname_to_id(&format!("refs/remotes/origin/manyhands/ticket/{ITEM}")))?,
            remote
        );
    }
    let case = prepare()?;
    publish_primary(&case)?;
    let repository = fixed(git2::Repository::open(&case.root))?;
    let base = fixed(repository.refname_to_id("refs/heads/main"))?;
    let local = child_commit(&repository, base, "local.txt", b"local divergence\n")?;
    fixed(repository.checkout_tree(
        fixed(repository.find_commit(local))?.as_object(),
        Some(git2::build::CheckoutBuilder::new().safe()),
    ))?;
    fixed(repository.reference("refs/heads/main", local, true, "fixture"))?;
    remote_advance(&case)?;
    let before = target_state(&case.root, &case.root)?;
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service
            .synchronize_remote(request(&case), &mut session),
        Err(SynchronizationError::MergeRequired { .. })
    ));
    assert_eq!(target_state(&case.root, &case.root)?, before);
    Ok(())
}
fn cancel_after_fetch() -> Result<(), FixtureError> {
    let case = prepare()?;
    publish_primary(&case)?;
    let new = remote_advance(&case)?;
    let req = request(&case);
    let id = req.operation_id;
    let root = case.root.clone();
    let other = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let before = target_state(&case.root, &case.root)?;
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::AfterFetch {
            other.cancel_remote_operation(&root, id).unwrap();
        }
    });
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    drop(hook);
    assert_eq!(target_state(&case.root, &case.root)?, before);
    let repo = fixed(git2::Repository::open(&case.root))?;
    assert_eq!(fixed(repo.refname_to_id("refs/remotes/origin/main"))?, new);
    assert!(
        fixed(case.service.remote_snapshot(&case.root))?
            .observations()
            .iter()
            .any(|row| row.remote_ref() == Some("refs/heads/main")
                && row.advertised_oid() == new
                && row.tracking_oid() == Some(new))
    );
    let effects = case.fixture.accepted_keys().len();
    let mut restart = req;
    restart.restart = true;
    assert!(matches!(
        case.service.synchronize_remote(restart, &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    Ok(())
}
fn local_prepared_mismatch_restart() -> Result<(), FixtureError> {
    let case = prepare()?;
    publish_primary(&case)?;
    let new = remote_advance(&case)?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    let old = fixed(repo.refname_to_id("refs/heads/main"))?;
    let root = case.root.clone();
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforeLocalMutation {
            git2::Repository::open(&root)
                .unwrap()
                .set_head_detached(old)
                .unwrap();
        }
    });
    let req = request(&case);
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(hook);
    fixed(repo.set_head("refs/heads/main"))?; // Restore symbolic identity only, not index/content/ref.
    assert_eq!(fixed(repo.refname_to_id("refs/heads/main"))?, old);
    assert_eq!(
        fixed(fixed(repo.index())?.write_tree())?,
        fixed(repo.find_commit(new))?.tree_id()
    );
    assert_eq!(
        fixed(std::fs::read(case.root.join("advanced.txt")))?,
        b"remote advance\n"
    );
    let before = target_state(&case.root, &case.root)?;
    let record = fixed(case.service.active_remote_operation(&case.root))?.ok_or(FixtureError)?;
    assert_eq!(record.phase(), RemoteOperationPhase::LocalPrepared);
    assert_eq!(
        format!("{:?}", record.sync_checkpoint()),
        "Some(LocalPrepared)"
    );
    assert_eq!(record.sync_evidence().expected_oid, Some(old));
    assert_eq!(record.sync_evidence().local_oid, Some(new));
    let effects = case.fixture.accepted_keys().len();
    let mut restart = req;
    restart.restart = true;
    assert!(matches!(
        case.service
            .synchronize_remote(restart.clone(), &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(target_state(&case.root, &case.root)?, before);
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    assert!(matches!(
        case.service.synchronize_remote(restart, &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(target_state(&case.root, &case.root)?, before);
    Ok(())
}
fn completed_refresh_index_flag_replay() -> Result<(), FixtureError> {
    let case = prepare()?;
    publish_primary(&case)?;
    let new = remote_advance(&case)?;
    let db = database(&case)?;
    fixed(db.execute_batch("CREATE TRIGGER fail_index_flag BEFORE UPDATE OF index_pending ON remote_operation_records WHEN NEW.index_pending=0 BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    let req = request(&case);
    let (mut session, _) = session(vec![]);
    let expected = SynchronizationOutcome::AlreadyCurrent {
        target: SynchronizationTarget::Primary,
        oid: new,
    };
    assert_eq!(
        fixed(case.service.synchronize_remote(req.clone(), &mut session))?,
        SynchronizationResult::IndexPending(IndexPending::new(expected.clone()))
    );
    let (refresh_state,pending):(String,i64)=fixed(db.query_row("SELECT o.state,r.index_pending FROM operation_records o JOIN remote_operation_records r ON o.operation_ulid=r.operation_ulid WHERE o.operation_ulid=?1",[req.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))))?;
    assert_eq!(refresh_state, "completed");
    assert_eq!(pending, 1);
    let effects = case.fixture.accepted_keys().len();
    let before = target_state(&case.root, &case.root)?;
    case.service
        .set_observation_hook_for_testing(|| panic!("index-only replay scanned discovery"));
    assert_eq!(
        fixed(case.service.synchronize_remote(req.clone(), &mut session))?,
        SynchronizationResult::IndexPending(IndexPending::new(expected.clone()))
    );
    fixed(db.execute_batch("DROP TRIGGER fail_index_flag"))?;
    assert_eq!(
        fixed(case.service.synchronize_remote(req, &mut session))?,
        SynchronizationResult::Complete(expected)
    );
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    assert_eq!(target_state(&case.root, &case.root)?, before);
    Ok(())
}
fn cancelled_push_new_id_proof() -> Result<(), FixtureError> {
    let case = prepare()?;
    let req = request(&case);
    let id = req.operation_id;
    let root = case.root.clone();
    let other = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::AfterPushReturn {
            other.cancel_remote_operation(&root, id).unwrap();
        }
    });
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    drop(hook);
    let repo = fixed(git2::Repository::open(&case.root))?;
    let candidate = fixed(repo.refname_to_id("refs/heads/main"))?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, candidate);
    let effects = case.fixture.accepted_keys().len();
    let mut old = req;
    old.restart = true;
    assert!(matches!(
        case.service.synchronize_remote(old, &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    let push_calls = std::rc::Rc::new(Cell::new(0));
    let push_observations = std::rc::Rc::new(Cell::new(0));
    let calls = push_calls.clone();
    let observations = push_observations.clone();
    let hook = crate::repository::transport::operation_tests::install_hook(move |point| {
        use crate::repository::transport::operation_tests::Checkpoint as C;
        match point {
            C::ExactPushStarted => calls.set(calls.get() + 1),
            C::PushAdvertisementObserved => observations.set(observations.get() + 1),
            _ => {}
        }
    });
    let before = target_state(&case.root, &case.root)?;
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
    drop(hook);
    assert_eq!(push_calls.get(), 0);
    assert!(push_observations.get() > 0);
    assert!(case.fixture.accepted_keys().len() > effects);
    assert_eq!(fixed(server.refname_to_id("refs/heads/main"))?, candidate);
    assert_eq!(target_state(&case.root, &case.root)?, before);
    Ok(())
}
fn fetch_observed_context_deleted() -> Result<(), FixtureError> {
    let (case, context) = context_fixture()?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .synchronize_remote(context_request(&case)?, &mut session),
    )?;
    fixed(case.service.observe_publication_remote(
        ObservePublicationRemoteRequest {
            root: case.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            approval: case.request().approval,
            restart: false,
        },
        &mut session,
    ))?;
    assert_eq!(
        fixed(case.service.remote_snapshot(&case.root))?
            .publication_evidence_for(AuthoringKind::Ticket, &fixed(ITEM.parse())?),
        RemotePublicationEvidence::ObservedPublished
    );
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    fixed(fixed(server.find_reference(CONTEXT_REF))?.delete())?;
    let before = target_state(&case.root, &context.worktree)?;
    assert!(matches!(
        case.service
            .synchronize_remote(context_request(&case)?, &mut session),
        Err(SynchronizationError::RemoteContextDeleted)
    ));
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    assert!(server.find_reference(CONTEXT_REF).is_err());
    Ok(())
}
fn trust_push_destination(
    case: &Case,
    destination: &SshRemoteFixture,
    session: &mut SessionCredentials<Provider>,
) -> Result<(), FixtureError> {
    let repo = fixed(git2::Repository::open(&case.root))?;
    destination.allow_client_public_key(fixed(russh::keys::PublicKey::from_bytes(
        &case.fixture.allowed_client_public_key(),
    ))?);
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
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
    fixed(case.service.verify_ssh_transport(transport, session))?;
    Ok(())
}
fn distinct_push_context_deleted() -> Result<(), FixtureError> {
    let case = prepare()?;
    let destination = SshRemoteFixture::start()?;
    let (mut session, _) = session(vec![]);
    trust_push_destination(&case, &destination, &mut session)?;
    let server = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    fixed(fixed(server.find_reference("refs/heads/main"))?.delete())?;
    let mut primary = request(&case);
    primary.approval = None; // Both endpoint pins were independently approved.
    fixed(case.service.synchronize_remote(primary, &mut session))?;
    let context = author_context(&case)?;
    let mut first = context_request(&case)?;
    first.approval = None;
    let result = fixed(case.service.synchronize_remote(first, &mut session))?;
    assert!(matches!(
        result,
        SynchronizationResult::Complete(SynchronizationOutcome::Published { .. })
    ));
    let fetch = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert!(fetch.find_reference(CONTEXT_REF).is_err());
    assert!(server.find_reference(CONTEXT_REF).is_ok());
    assert_eq!(
        fixed(case.service.remote_snapshot(&case.root))?
            .publication_evidence_for(AuthoringKind::Ticket, &fixed(ITEM.parse())?),
        RemotePublicationEvidence::NeverPublished
    );
    fixed(fixed(server.find_reference(CONTEXT_REF))?.delete())?;
    // Recreated service + new ID must retain terminal direction-qualified proof.
    let reopened = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let before = target_state(&case.root, &context.worktree)?;
    let mut next = context_request(&case)?;
    next.approval = None;
    assert!(matches!(
        reopened.synchronize_remote(next, &mut session),
        Err(SynchronizationError::RemoteContextDeleted)
    ));
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    assert!(server.find_reference(CONTEXT_REF).is_err());
    assert!(fetch.find_reference(CONTEXT_REF).is_err());
    assert_eq!(
        fixed(reopened.remote_snapshot(&case.root))?
            .publication_evidence_for(AuthoringKind::Ticket, &fixed(ITEM.parse())?),
        RemotePublicationEvidence::NeverPublished
    );
    Ok(())
}
fn push_history_generation_fencing() -> Result<(), FixtureError> {
    let (case, context) = context_fixture()?;
    let (mut session, _) = session(vec![]);
    let published = context_request(&case)?;
    fixed(
        case.service
            .synchronize_remote(published.clone(), &mut session),
    )?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    fixed(fixed(server.find_reference(CONTEXT_REF))?.delete())?;
    let destination = SshRemoteFixture::start()?;
    trust_push_destination(&case, &destination, &mut session)?;
    let before = target_state(&case.root, &context.worktree)?;
    let mut next = context_request(&case)?;
    next.approval = None;
    assert!(matches!(
        case.service.synchronize_remote(next, &mut session),
        Err(SynchronizationError::HistoryUnknown)
    ));
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    let db = database(&case)?;
    let old: i64 = fixed(db.query_row(
        "SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
        [published.operation_id.to_string()],
        |r| r.get(0),
    ))?;
    let current: i64 = fixed(db.query_row(
        "SELECT configuration_generation FROM remote_polling_state",
        [],
        |r| r.get(0),
    ))?;
    assert!(current > old);
    let push = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    assert!(push.find_reference(CONTEXT_REF).is_err());
    Ok(())
}
fn ambiguous_cancelled_context_absent() -> Result<(), FixtureError> {
    let (case, context) = context_fixture()?;
    let req = context_request(&case)?;
    let id = req.operation_id;
    let root = case.root.clone();
    let other = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::AfterPushReturn {
            other.cancel_remote_operation(&root, id).unwrap();
        }
    });
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    drop(hook);
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert!(server.find_reference(CONTEXT_REF).is_ok()); // Actual acceptance before cancellation, not row inference.
    fixed(fixed(server.find_reference(CONTEXT_REF))?.delete())?;
    let reopened = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let before = target_state(&case.root, &context.worktree)?;
    let pushes = std::rc::Rc::new(Cell::new(0));
    let observations = std::rc::Rc::new(Cell::new(0));
    let calls = pushes.clone();
    let lists = observations.clone();
    let hook = crate::repository::transport::operation_tests::install_hook(move |point| {
        use crate::repository::transport::operation_tests::Checkpoint as C;
        match point {
            C::ExactPushStarted => calls.set(calls.get() + 1),
            C::PushAdvertisementObserved => lists.set(lists.get() + 1),
            _ => {}
        }
    });
    assert!(matches!(
        reopened.synchronize_remote(context_request(&case)?, &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(hook);
    assert_eq!(pushes.get(), 0);
    assert!(observations.get() > 0);
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    assert!(server.find_reference(CONTEXT_REF).is_err());
    // Cancelled exact-ID restart remains terminal, not republish confirmation.
    let effects = case.fixture.accepted_keys().len();
    let mut restart = req;
    restart.restart = true;
    assert!(matches!(
        reopened.synchronize_remote(restart, &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    Ok(())
}
fn context_inherited_absent_push() -> Result<(), FixtureError> {
    let (case, context) = context_fixture()?;
    let db = database(&case)?;
    fixed(db.execute_batch("CREATE TRIGGER fail_verify BEFORE UPDATE ON remote_operation_records WHEN NEW.sync_checkpoint='push_verified' BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    let req = context_request(&case)?;
    let (mut session, _) = session(vec![]);
    assert!(
        case.service
            .synchronize_remote(req.clone(), &mut session)
            .is_err()
    );
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    assert!(server.find_reference(CONTEXT_REF).is_ok());
    fixed(fixed(server.find_reference(CONTEXT_REF))?.delete())?;
    fixed(db.execute_batch("DROP TRIGGER fail_verify"))?;
    let before = target_state(&case.root, &context.worktree)?;
    let reopened = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let mut restart = req;
    restart.restart = true;
    assert!(matches!(
        reopened.synchronize_remote(restart, &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    assert!(server.find_reference(CONTEXT_REF).is_err());
    Ok(())
}
fn cancelled_context_equal_candidate() -> Result<(), FixtureError> {
    let (case, context) = context_fixture()?;
    let req = context_request(&case)?;
    let id = req.operation_id;
    let root = case.root.clone();
    let other = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::AfterPushReturn {
            other.cancel_remote_operation(&root, id).unwrap();
        }
    });
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service.synchronize_remote(req, &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    drop(hook);
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let oid = fixed(server.refname_to_id(CONTEXT_REF))?;
    let before = target_state(&case.root, &context.worktree)?;
    let calls = std::rc::Rc::new(Cell::new(0));
    let lists = std::rc::Rc::new(Cell::new(0));
    let pushes = calls.clone();
    let observations = lists.clone();
    let hook = crate::repository::transport::operation_tests::install_hook(move |point| {
        use crate::repository::transport::operation_tests::Checkpoint as C;
        match point {
            C::ExactPushStarted => pushes.set(pushes.get() + 1),
            C::PushAdvertisementObserved => observations.set(observations.get() + 1),
            _ => {}
        }
    });
    let reopened = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    assert_eq!(
        fixed(reopened.synchronize_remote(context_request(&case)?, &mut session))?,
        SynchronizationResult::Complete(SynchronizationOutcome::AlreadyCurrent {
            target: context_request(&case)?.target,
            oid
        })
    );
    drop(hook);
    assert_eq!(calls.get(), 0);
    assert!(lists.get() > 0);
    assert_eq!(fixed(server.refname_to_id(CONTEXT_REF))?, oid);
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    Ok(())
}
fn context_deleted_before_push() -> Result<(), FixtureError> {
    let (case, context) = context_fixture()?;
    let (mut session, _) = session(vec![]);
    fixed(
        case.service
            .synchronize_remote(context_request(&case)?, &mut session),
    )?;
    let server = fixed(git2::Repository::open_bare(case.fixture.repository_path()))?;
    let path = case.fixture.repository_path().to_owned();
    let deleted = Cell::new(false);
    let hook = crate::repository::transport::operation_tests::install_hook(move |point| {
        if point
            == crate::repository::transport::operation_tests::Checkpoint::PushAdvertisementObserved
            && !deleted.replace(true)
        {
            git2::Repository::open_bare(&path)
                .unwrap()
                .find_reference(CONTEXT_REF)
                .unwrap()
                .delete()
                .unwrap();
        }
    });
    let before = target_state(&case.root, &context.worktree)?;
    assert!(matches!(
        case.service
            .synchronize_remote(context_request(&case)?, &mut session),
        Err(SynchronizationError::RemoteContextDeleted)
    ));
    drop(hook);
    assert_eq!(target_state(&case.root, &context.worktree)?, before);
    assert!(server.find_reference(CONTEXT_REF).is_err());
    Ok(())
}

// Both endpoints are independently preapproved. Tests compare owned refs/bytes
// without persisting or formatting raw endpoints or server text.
fn alternate_push(case: &Case) -> Result<SshRemoteFixture, FixtureError> {
    let destination = SshRemoteFixture::start()?;
    let (mut session, _) = session(vec![]);
    trust_push_destination(case, &destination, &mut session)?;
    let server = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    fixed(fixed(server.find_reference("refs/heads/main"))?.delete())?;
    fixed(fixed(git2::Repository::open(&case.root))?.remote_set_pushurl("origin", None))?;
    Ok(destination)
}
fn same_id_restart_endpoint_changed() -> Result<(), FixtureError> {
    let case = prepare()?;
    let destination = alternate_push(&case)?;
    let req = request(&case);
    let db = database(&case)?;
    fixed(db.execute_batch("CREATE TRIGGER fail_fetch_batch BEFORE INSERT ON remote_observation_batches BEGIN SELECT RAISE(ABORT,'fixed failure'); END"))?;
    let before = target_state(&case.root, &case.root)?;
    let (mut session, _) = session(vec![]);
    assert!(
        case.service
            .synchronize_remote(req.clone(), &mut session)
            .is_err()
    );
    let old: i64 = fixed(db.query_row(
        "SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
        [req.operation_id.to_string()],
        |r| r.get(0),
    ))?;
    let record = fixed(case.service.active_remote_operation(&case.root))?.ok_or(FixtureError)?;
    assert_eq!(record.phase(), RemoteOperationPhase::FetchPrepared);
    fixed(db.execute_batch("DROP TRIGGER fail_fetch_batch"))?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
    let effects = case.fixture.accepted_keys().len();
    let other_effects = destination.accepted_keys().len();
    let mut restart = req.clone();
    restart.restart = true;
    restart.approval = None;
    assert!(
        case.service
            .synchronize_remote(restart, &mut session)
            .is_err()
    );
    let current: i64 = fixed(db.query_row(
        "SELECT configuration_generation FROM remote_polling_state",
        [],
        |r| r.get(0),
    ))?;
    let retained: i64 = fixed(db.query_row(
        "SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
        [req.operation_id.to_string()],
        |r| r.get(0),
    ))?;
    assert!(current > old);
    assert_eq!(retained, old);
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    assert_eq!(destination.accepted_keys().len(), other_effects);
    assert_eq!(target_state(&case.root, &case.root)?, before);
    assert!(
        fixed(git2::Repository::open_bare(destination.repository_path()))?
            .find_reference("refs/heads/main")
            .is_err()
    );
    Ok(())
}
fn before_push_endpoint_changed() -> Result<(), FixtureError> {
    let case = prepare()?;
    let destination = alternate_push(&case)?;
    let root = case.root.clone();
    let endpoint = destination.url();
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforePush {
            git2::Repository::open(&root)
                .unwrap()
                .remote_set_pushurl("origin", Some(&endpoint))
                .unwrap();
        }
    });
    let before = target_state(&case.root, &case.root)?;
    let (mut session, _) = session(vec![]);
    let mut req = request(&case);
    req.approval = None;
    assert!(
        case.service
            .synchronize_remote(req.clone(), &mut session)
            .is_err()
    );
    drop(hook);
    let server = fixed(git2::Repository::open_bare(destination.repository_path()))?;
    assert!(server.find_reference("refs/heads/main").is_err());
    assert_eq!(
        fixed(
            fixed(git2::Repository::open_bare(case.fixture.repository_path()))?
                .refname_to_id("refs/heads/main")
        )?,
        case.fixture.commit_id()
    );
    assert_eq!(target_state(&case.root, &case.root)?, before);
    let db = database(&case)?;
    let (checkpoint,authority):(String,Option<String>)=fixed(db.query_row("SELECT sync_checkpoint,authoritative_kind FROM remote_operation_records WHERE operation_ulid=?1",[req.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))))?;
    assert_eq!(checkpoint, "push_prepared");
    assert!(authority.is_none());
    Ok(())
}
fn after_push_return_endpoint_changed() -> Result<(), FixtureError> {
    let case = prepare()?;
    let destination = alternate_push(&case)?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    let candidate = fixed(repo.refname_to_id("refs/heads/main"))?;
    fixed(repo.remote_set_pushurl("origin", Some(&destination.url())))?;
    let plan = fixed(RemoteRefPlan::from_configuration("origin", "main"))?;
    let (mut session, _) = session(vec![]);
    let mut transport = case.request();
    transport.direction = SshDirection::Push;
    transport.approval = None;
    fixed(
        case.service
            .with_authenticated_remote(transport, &mut session, |r| {
                r.push_exact(&plan, &SynchronizationTarget::Primary)
            }),
    )?;
    fixed(repo.remote_set_pushurl("origin", None))?;
    let root = case.root.clone();
    let source = case.fixture.repository_path().to_owned();
    let endpoint = destination.url();
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::AfterPushReturn {
            let server = git2::Repository::open_bare(&source).unwrap();
            assert_eq!(server.refname_to_id("refs/heads/main").unwrap(), candidate);
            server
                .find_reference("refs/heads/main")
                .unwrap()
                .delete()
                .unwrap();
            git2::Repository::open(&root)
                .unwrap()
                .remote_set_pushurl("origin", Some(&endpoint))
                .unwrap();
        }
    });
    let before = target_state(&case.root, &case.root)?;
    let mut req = request(&case);
    req.approval = None;
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(hook);
    assert!(
        fixed(git2::Repository::open_bare(case.fixture.repository_path()))?
            .find_reference("refs/heads/main")
            .is_err()
    );
    assert_eq!(
        fixed(
            fixed(git2::Repository::open_bare(destination.repository_path()))?
                .refname_to_id("refs/heads/main")
        )?,
        candidate
    );
    assert_eq!(target_state(&case.root, &case.root)?, before);
    let (checkpoint,authority,oid):(String,Option<String>,Option<String>)=fixed(database(&case)?.query_row("SELECT sync_checkpoint,authoritative_kind,push_oid FROM remote_operation_records WHERE operation_ulid=?1",[req.operation_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))))?;
    assert_eq!(checkpoint, "push_returned");
    assert!(authority.is_none());
    assert_eq!(oid, Some(candidate.to_string()));
    Ok(())
}
fn cancelled_replay_before_config() -> Result<(), FixtureError> {
    let case = prepare()?;
    let req = request(&case);
    let id = req.operation_id;
    let root = case.root.clone();
    let other = fixed(RepositoryService::open_at(
        &case.directory.path().join("data"),
    ))?;
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforeFetch {
            other.cancel_remote_operation(&root, id).unwrap();
        }
    });
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    drop(hook);
    fixed(std::fs::write(
        case.root.join(".manyhands/config.toml"),
        b"invalid configuration",
    ))?;
    let effects = case.fixture.accepted_keys().len();
    let mut restart = req;
    restart.restart = true;
    assert!(matches!(
        case.service.synchronize_remote(restart, &mut session),
        Err(SynchronizationError::Interrupted)
    ));
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    Ok(())
}
fn initial_snapshot_not_rebased() -> Result<(), FixtureError> {
    let case = prepare()?;
    let destination = alternate_push(&case)?;
    let root = case.root.clone();
    let endpoint = destination.url();
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforeFetch {
            git2::Repository::open(&root)
                .unwrap()
                .remote_set_pushurl("origin", Some(&endpoint))
                .unwrap();
        }
    });
    let req = request(&case);
    let before = target_state(&case.root, &case.root)?;
    let effects = case.fixture.accepted_keys().len();
    let other_effects = destination.accepted_keys().len();
    let (mut session, _) = session(vec![]);
    assert!(matches!(
        case.service.synchronize_remote(req.clone(), &mut session),
        Err(SynchronizationError::ExternalChange)
    ));
    drop(hook);
    assert_eq!(case.fixture.accepted_keys().len(), effects);
    assert_eq!(destination.accepted_keys().len(), other_effects);
    let db = database(&case)?;
    let old: i64 = fixed(db.query_row(
        "SELECT configuration_generation FROM remote_operation_records WHERE operation_ulid=?1",
        [req.operation_id.to_string()],
        |r| r.get(0),
    ))?;
    let mut restart = req;
    restart.restart = true;
    restart.approval = None;
    assert!(matches!(
        case.service.synchronize_remote(restart, &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    let new: i64 = fixed(db.query_row(
        "SELECT configuration_generation FROM remote_polling_state",
        [],
        |r| r.get(0),
    ))?;
    assert!(new > old);
    assert_eq!(target_state(&case.root, &case.root)?, before);
    assert!(
        fixed(git2::Repository::open_bare(destination.repository_path()))?
            .find_reference("refs/heads/main")
            .is_err()
    );
    Ok(())
}
fn authenticated_action_snapshot() -> Result<(), FixtureError> {
    let case = prepare()?;
    let destination = alternate_push(&case)?;
    let repo = fixed(git2::Repository::open(&case.root))?;
    // Keep Push explicitly pinned to A; alter only Fetch during Push auth. The
    // per-call Push endpoint remains valid, but the whole action snapshot does not.
    fixed(repo.remote_set_pushurl("origin", Some(&case.fixture.url())))?;
    let pushing = std::rc::Rc::new(Cell::new(false));
    let stage = pushing.clone();
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforePush {
            stage.set(true);
        }
    });
    let root = case.root.clone();
    let endpoint = destination.url();
    let changed = Cell::new(false);
    let transport_hook =
        crate::repository::transport::operation_tests::install_hook(move |point| {
            if point == crate::repository::transport::operation_tests::Checkpoint::Authenticated
                && pushing.get()
                && !changed.replace(true)
            {
                git2::Repository::open(&root)
                    .unwrap()
                    .remote_set_url("origin", &endpoint)
                    .unwrap();
            }
        });
    let before = target_state(&case.root, &case.root)?;
    let (mut session, _) = session(vec![]);
    let mut req = request(&case);
    req.approval = None;
    assert!(matches!(
        case.service.synchronize_remote(req, &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(transport_hook);
    drop(hook);
    assert_eq!(
        fixed(
            fixed(git2::Repository::open_bare(case.fixture.repository_path()))?
                .refname_to_id("refs/heads/main")
        )?,
        case.fixture.commit_id()
    );
    assert!(
        fixed(git2::Repository::open_bare(destination.repository_path()))?
            .find_reference("refs/heads/main")
            .is_err()
    );
    assert_eq!(target_state(&case.root, &case.root)?, before);
    Ok(())
}
fn prompt_action_snapshot_and_cancel() -> Result<(), FixtureError> {
    for cancel in [false, true] {
        let case = prepare()?;
        let destination = alternate_push(&case)?;
        let key = fixed(ssh_key::PrivateKey::read_openssh_file(
            case.fixture.client_key_path(),
        ))?;
        let encrypted = fixed(key.encrypt(&mut ssh_key::rand_core::OsRng, PASSWORD))?;
        fixed(std::fs::write(
            case.fixture.client_key_path(),
            fixed(encrypted.to_openssh(ssh_key::LineEnding::LF))?.as_bytes(),
        ))?;
        let req = request(&case);
        let id = req.operation_id;
        let root = case.root.clone();
        let endpoint = destination.url();
        let other = fixed(RepositoryService::open_at(
            &case.directory.path().join("data"),
        ))?;
        let hook = crate::repository::transport::operation_tests::install_hook(move |point| {
            if point == crate::repository::transport::operation_tests::Checkpoint::ProviderReturned
            {
                if cancel {
                    other.cancel_remote_operation(&root, id).unwrap();
                } else {
                    git2::Repository::open(&root)
                        .unwrap()
                        .remote_set_pushurl("origin", Some(&endpoint))
                        .unwrap();
                }
            }
        });
        let before = target_state(&case.root, &case.root)?;
        let (mut session, prompts) = session(vec![secret(PASSWORD)]);
        let result = case.service.synchronize_remote(req, &mut session);
        if cancel {
            assert!(matches!(result, Err(SynchronizationError::Interrupted)));
        } else {
            assert!(matches!(result, Err(SynchronizationError::ExternalChange)));
        }
        drop(hook);
        assert_eq!(prompts.borrow().len(), 1);
        assert_eq!(target_state(&case.root, &case.root)?, before);
        assert!(
            fixed(git2::Repository::open_bare(destination.repository_path()))?
                .find_reference("refs/heads/main")
                .is_err()
        );
    }
    Ok(())
}
fn scope_prepare_endpoint_race() -> Result<(), FixtureError> {
    let case = prepare()?;
    let destination = alternate_push(&case)?;
    let pushing = std::rc::Rc::new(Cell::new(false));
    let stage = pushing.clone();
    let hook = crate::repository::observation_tests::install_hook(move |point| {
        if point == RemoteOperationSafePoint::BeforePush {
            stage.set(true);
        }
    });
    let root = case.root.clone();
    let endpoint = destination.url();
    let changed = Cell::new(false);
    let attempts = std::rc::Rc::new(Cell::new(0));
    let pushes = attempts.clone();
    let transport_hook =
        crate::repository::transport::operation_tests::install_hook(move |point| {
            use crate::repository::transport::operation_tests::Checkpoint as C;
            if point == C::ActionSnapshotChecked && pushing.get() && !changed.replace(true) {
                git2::Repository::open(&root)
                    .unwrap()
                    .remote_set_pushurl("origin", Some(&endpoint))
                    .unwrap();
            }
            if point == C::ExactPushStarted {
                pushes.set(pushes.get() + 1);
            }
        });
    let before = target_state(&case.root, &case.root)?;
    let effects = destination.accepted_keys().len();
    let (mut session, prompts) = session(vec![]);
    let mut req = request(&case);
    req.approval = None;
    assert!(matches!(
        case.service.synchronize_remote(req, &mut session),
        Err(SynchronizationError::RecoveryRequired)
    ));
    drop(transport_hook);
    drop(hook);
    assert_eq!(attempts.get(), 0);
    assert_eq!(prompts.borrow().len(), 0);
    assert_eq!(destination.accepted_keys().len(), effects);
    assert_eq!(
        fixed(
            fixed(git2::Repository::open_bare(case.fixture.repository_path()))?
                .refname_to_id("refs/heads/main")
        )?,
        case.fixture.commit_id()
    );
    assert!(
        fixed(git2::Repository::open_bare(destination.repository_path()))?
            .find_reference("refs/heads/main")
            .is_err()
    );
    assert_eq!(target_state(&case.root, &case.root)?, before);
    Ok(())
}
