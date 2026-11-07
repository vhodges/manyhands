//! Real process termination over the public exit-point API and real SSH children.
use super::*;

const POINTS: [FailurePoint; 14] = [
    FailurePoint::AfterConfirmed,
    FailurePoint::AfterContextSync,
    FailurePoint::BeforeIntegrationRef,
    FailurePoint::AfterIntegrationRef,
    FailurePoint::AfterPublication,
    FailurePoint::BeforeRemoteDeletion,
    FailurePoint::AfterRemoteDeletion,
    FailurePoint::DuringWorktreeRemoval,
    FailurePoint::BeforeBranchDeletion,
    FailurePoint::BeforeCompletion,
    FailurePoint::AfterCompletion,
    FailurePoint::RefreshFailure,
    FailurePoint::PromotionAfterTrackingDeletion,
    FailurePoint::PromotionAfterCompletionCommit,
];
pub(super) fn maybe_child() -> bool {
    if std::env::args().nth(1).as_deref() != Some("--promotion-exit") {
        return false;
    }
    std::panic::set_hook(Box::new(|info| {
        eprintln!("promotion exit child assertion");
        if let Some(l) = info.location() {
            ssh_harness::observation(&[l.line() as u128]);
        }
    }));
    let result = child_body();
    if result.is_err() {
        eprintln!("promotion exit child failed");
    }
    std::process::exit(1)
}
fn child_body() -> Result<(), FixtureError> {
    let args: Vec<_> = std::env::args().skip(2).collect();
    if args.len() != 5 {
        return Err(FixtureError);
    }
    let point: usize = fixed(args[0].parse())?;
    let root = PathBuf::from(&args[1]);
    let data = PathBuf::from(&args[2]);
    let operation_id = fixed(OperationId::parse(&args[3]))?;
    let digest = fixed(
        args[4]
            .split(',')
            .map(str::parse::<u8>)
            .collect::<Result<Vec<_>, _>>(),
    )?;
    let service = fixed(RepositoryService::open_at_with_exit_point_for_testing(
        &data,
        *POINTS.get(point).ok_or(FixtureError)?,
    ))?;
    let request = PromoteDocumentRequest {
        root,
        operation_id,
        item_id: DOC.parse().unwrap(),
        expected: PromotionExpectation::from_bytes(fixed(digest.try_into())?),
        approval: None,
        confirmed_identity: None,
    };
    fixed(service.promote_document(
        request,
        &mut SessionCredentials::new(remote_world::Provider),
    ))?;
    Err(FixtureError) // Reaching here means the actual checkpoint did not execute.
}
pub(super) fn launch(
    p: &Pair,
    request: &PromoteDocumentRequest,
    point: usize,
) -> Result<(), FixtureError> {
    p.privacy()?;
    let output = fixed(
        std::process::Command::new(fixed(std::env::current_exe())?)
            .arg("--promotion-exit")
            .arg(point.to_string())
            .arg(&p.a.root)
            .arg(&p.a.data)
            .arg(request.operation_id.to_string())
            .arg(
                request
                    .expected
                    .to_bytes()
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .stdin(std::process::Stdio::null())
            .output(),
    )?;
    let probes = ssh_privacy::load(&PathBuf::from(
        std::env::var_os(ssh_privacy::PROBES).ok_or(FixtureError)?,
    ))?;
    ssh_privacy::clean(&output.stdout, &probes)?;
    ssh_privacy::clean(&output.stderr, &probes)?;
    assert_eq!(output.status.code(), Some(FAILURE_POINT_EXIT_STATUS));
    p.server.wait_for_helper_audits()?;
    Ok(())
}
fn receives(p: &Pair, cursor: usize) -> Result<[usize; 3], FixtureError> {
    p.server.wait_for_helper_audits()?;
    let updates = p.server.receive_updates();
    let mut counts = [0; 3];
    for u in &updates[cursor..] {
        assert!(u.accepted);
        if u.reference == MAIN {
            counts[1] += 1;
        } else if u.reference == CONTEXT {
            counts[if u.new_oid.is_zero() { 2 } else { 0 }] += 1;
        } else {
            return Err(FixtureError);
        }
    }
    Ok(counts)
}
fn pending_history(p: &Pair, request: &PromoteDocumentRequest) -> Result<[u8; 32], FixtureError> {
    assert_eq!(step(&p.a, request.operation_id)?, "branch_deleted");
    assert_eq!(count(&p.a, "retired_contexts")?, 0);
    assert_eq!(fixed(p.a.db()?.query_row("SELECT COUNT(*) FROM remote_context_states WHERE publication_evidence='observed_published'",[],|r|r.get::<_,i64>(0)))?,1);
    assert_eq!(fixed(p.a.db()?.query_row("SELECT COUNT(*) FROM remote_operation_records WHERE operation_ulid=?1 AND action='promote' AND phase IN ('reserved','advertising','persisting')",[request.operation_id.to_string()],|r|r.get::<_,i64>(0)))?,1);
    let db = p.a.db()?;
    let mut q=fixed(db.prepare("SELECT remote_ref,tracking_ref,kind,item_id,last_advertised_oid,tracking_oid,publication_evidence,state,observed_at FROM remote_context_states ORDER BY remote_ref"))?;
    let rows = fixed(
        fixed(q.query_map([], |r| {
            (0..9)
                .map(|i| r.get::<_, rusqlite::types::Value>(i))
                .collect::<Result<Vec<_>, _>>()
        }))?
        .collect::<Result<Vec<_>, _>>(),
    )?;
    Ok(*blake3::hash(format!("{rows:?}").as_bytes()).as_bytes())
}
fn tracking_lock(p: &Pair) -> Result<PathBuf, FixtureError> {
    Ok(p.a
        .repo()?
        .commondir()
        .join(format!("refs/remotes/origin/manyhands/document/{DOC}.lock")))
}
fn published_tracking(p: &Pair, request: &PromoteDocumentRequest) -> Result<(), FixtureError> {
    stopped(p, request, FailurePoint::AfterPublication)?;
    poll(p, &p.a)?;
    // These split-state tests exercise deletion of an EXISTING fetched ref,
    // separately from the verified-absence path covered by the normal exits.
    let tracking = format!(
        "refs/remotes/origin/{}",
        CONTEXT.trim_start_matches("refs/heads/")
    );
    assert_eq!(oid(&p.a, &tracking)?, oid(&p.a, CONTEXT)?);
    Ok(())
}
pub(super) fn boundary(point: usize) -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let base = oid(&p.a, MAIN)?;
    let context = p.save(&p.a)?;
    let request = p.request()?;
    if point >= 12 {
        published_tracking(&p, &request)?;
    }
    launch(&p, &request, point)?;
    let stages = [
        "confirmed",
        "context_synchronized",
        "integrating",
        "integrating",
        "published",
        "published",
        "remote_deleted",
        "worktree_removing",
        "worktree_removed",
        "branch_deleted",
        "complete",
        "complete",
        "branch_deleted",
        "complete",
    ];
    assert_eq!(step(&p.a, request.operation_id)?, stages[point]);
    let before = receives(&p, cursor)?;
    let expected = if point == 0 {
        [0, 0, 0]
    } else if point <= 3 {
        [1, 0, 0]
    } else if point <= 5 {
        [1, 1, 0]
    } else {
        [1, 1, 1]
    };
    assert_eq!(before, expected);
    assert_eq!(p.a.worktree().exists(), point <= 7);
    assert_eq!(p.a.repo()?.find_reference(CONTEXT).is_ok(), point <= 8);
    assert_eq!(server_oid(&p, CONTEXT).is_ok(), (1..=5).contains(&point));
    assert_eq!(promotion_merges(&p.a)?, usize::from(point >= 2));
    if point <= 2 {
        assert_eq!(oid(&p.a, MAIN)?, base);
    } else {
        assert_ne!(oid(&p.a, MAIN)?, base);
    }
    if point <= 3 {
        assert_eq!(server_oid(&p, MAIN)?, base);
    } else {
        assert_eq!(server_oid(&p, MAIN)?, oid(&p.a, MAIN)?);
    }
    if point <= 8 {
        assert_eq!(oid(&p.a, CONTEXT)?, context);
    }
    p.privacy()?;
    p.a.service = fixed(RepositoryService::open_at(&p.a.data))?;
    if point == 12 {
        let history = pending_history(&p, &request)?;
        let lock = tracking_lock(&p)?;
        let bytes = fixed(std::fs::read(&lock))?;
        assert!(bytes.is_empty());
        let outcome = run(&p, &request)?;
        assert_eq!(
            outcome.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::RecoveryRequired
            }
        );
        assert_eq!(pending_history(&p, &request)?, history);
        assert_eq!(receives(&p, cursor)?, before);
        assert_eq!(fixed(std::fs::read(&lock))?, bytes);
        assert!(!p.a.worktree().exists() && p.a.repo()?.find_reference(CONTEXT).is_err());
        p.privacy()?;
        // T6-1: child is joined, helper audits drained, no other Git writers.
        // Only this exact verified empty backend lock in this disposable fixture
        // is removed to simulate the operator's quiesced repair. No production
        // auto-removal or inferred lock ownership is introduced.
        assert_eq!(p.server.active_helpers(), 0);
        assert!(fixed(std::fs::symlink_metadata(&lock))?.is_file());
        fixed(std::fs::remove_file(&lock))?;
    }
    let surviving = if point == 13 {
        Some((
            tracking_lock(&p)?,
            fixed(std::fs::read(tracking_lock(&p)?))?,
        ))
    } else {
        None
    };
    let outcome = run(&p, &request)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    assert_eq!(step(&p.a, request.operation_id)?, "complete");
    assert_eq!(promotion_merges(&p.a)?, 1);
    assert_eq!(receives(&p, cursor)?, [1, 1, 1]);
    assert!(!p.a.worktree().exists() && p.a.repo()?.find_reference(CONTEXT).is_err());
    assert_eq!(server_oid(&p, MAIN)?, oid(&p.a, MAIN)?);
    assert!(server_oid(&p, CONTEXT).is_err());
    assert_eq!(count(&p.a, "retired_contexts")?, 1);
    let complete = run(&p, &request)?;
    assert_eq!(complete, outcome);
    assert_eq!(receives(&p, cursor)?, [1, 1, 1]);
    if let Some((path, bytes)) = surviving {
        assert_eq!(fixed(std::fs::read(path))?, bytes);
    }
    discovery_primary_once(&p)?;
    p.privacy()
}
pub(super) fn f7_loss() -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    launch(&p, &request, 6)?;
    let primary = oid(&p.a, MAIN)?;
    let updates = p.server.receive_updates();
    assert_eq!(promotion_merges(&p.a)?, 1);
    lose_registry(&mut p)?;
    unknown(&p.a)?;
    // T(P) may be pre-publication. A real poll reobserves the already-published
    // primary without recreating an absent context; no receive command is sent.
    poll(&p, &p.a)?;
    let preview = p.preview()?;
    assert_eq!(preview.integration(), PlannedIntegration::AlreadyContained);
    assert!(!preview.cleanup().remote_branch);
    let new = p.request()?;
    assert_ne!(new.operation_id, request.operation_id);
    let result = run(&p, &new)?;
    assert_eq!(
        result.integration(),
        PromotionIntegration::AlreadyContained { oid: primary }
    );
    assert_eq!(result.cleanup(), PromotionCleanup::Complete);
    assert_eq!(promotion_merges(&p.a)?, 1);
    assert_eq!(p.server.receive_updates(), updates);
    assert_eq!(count(&p.a, "retired_contexts")?, 1);
    unknown(&p.a)?;
    // This newly confirmed cleanup earned new proof, not reconstructed old proof.
    let saved = p.save_body(&p.a, &(BODY.to_owned() + "earned after F7 recovery\n"))?;
    assert_eq!(published(fixed(p.sync(&p.a))?)?, saved);
    unknown(&p.a)?;
    p.privacy()
}
pub(super) fn sql_failure(commit: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    published_tracking(&p, &request)?;
    stopped(&p, &request, FailurePoint::BeforeCompletion)?;
    assert!(
        oid(
            &p.a,
            &format!(
                "refs/remotes/origin/{}",
                CONTEXT.trim_start_matches("refs/heads/")
            )
        )
        .is_ok()
    );
    let history = pending_history(&p, &request)?;
    let updates = p.server.receive_updates();
    let db = p.a.db()?;
    if commit {
        fixed(db.execute_batch("CREATE TABLE remote_commit_fault(parent INTEGER REFERENCES repositories(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER promotion_sql_fault AFTER UPDATE OF step ON document_promotions WHEN NEW.step='complete' BEGIN INSERT INTO remote_commit_fault VALUES(-1); END;"))?;
    } else {
        fixed(db.execute_batch("CREATE TRIGGER promotion_sql_fault BEFORE INSERT ON retired_contexts BEGIN SELECT RAISE(ABORT,'fixed'); END;"))?;
    }
    let o = run(&p, &request)?;
    assert_eq!(
        o.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RecoveryRequired
        }
    );
    assert_eq!(pending_history(&p, &request)?, history);
    assert_eq!(p.server.receive_updates(), updates);
    assert!(
        p.a.repo()?
            .find_reference(&format!(
                "refs/remotes/origin/{}",
                CONTEXT.trim_start_matches("refs/heads/")
            ))
            .is_err()
    );
    p.privacy()?;
    fixed(db.execute_batch("DROP TRIGGER promotion_sql_fault"))?;
    if commit {
        fixed(db.execute_batch("DROP TABLE remote_commit_fault"))?;
    }
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(p.server.receive_updates(), updates);
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}
pub(super) fn uncertain_commit() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    published_tracking(&p, &request)?;
    let service = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &p.a.data,
        FailurePoint::PromotionAfterCompletionCommit,
    ))?;
    let o = authoritative(fixed(p.execute(&service, request.clone()))?);
    assert_eq!(o.cleanup(), PromotionCleanup::Complete);
    assert_eq!(step(&p.a, request.operation_id)?, "complete");
    let updates = p.server.receive_updates();
    assert_eq!(run(&p, &request)?, o);
    assert_eq!(p.server.receive_updates(), updates);
    p.privacy()
}
