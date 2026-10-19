//! Real process termination at every closure boundary (F1–F12, the three
//! checkpoint seams and both F10/F11 seams) over real SSH children, then
//! reopen, the identical retry and the effect counters.
use super::*;

/// Lifecycle order: closure's seams sit between F2 and F3.
pub(super) const POINTS: [FailurePoint; 17] = [
    FailurePoint::AfterConfirmed,
    FailurePoint::AfterContextSync,
    FailurePoint::ClosureAfterClosingRecorded,
    FailurePoint::ClosureDuringCheckpointCheckout,
    FailurePoint::AfterClosureCheckpoint,
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
const STAGES: [&str; 17] = [
    "confirmed",
    "context_synchronized",
    "closing",
    "closing",
    "closing",
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
pub(super) const CLOSING_RECORDED: usize = 2;
pub(super) const DURING_CHECKOUT: usize = 3;
pub(super) const AFTER_CHECKPOINT: usize = 4;
const F10_SEAM: usize = 15;
const F11_SEAM: usize = 16;

pub(super) const CASES: &[ssh_harness::Case] = &[
    ("exit_f1_confirmed", || boundary(0)),
    ("exit_f2_context_synchronized", || boundary(1)),
    ("exit_seam_closing_recorded", || boundary(2)),
    ("exit_seam_during_checkpoint_checkout", || boundary(3)),
    ("exit_seam_after_closure_checkpoint", || boundary(4)),
    ("exit_f3_before_integration_ref", || boundary(5)),
    ("exit_f4_after_integration_ref", || boundary(6)),
    ("exit_f5_after_publication", || boundary(7)),
    ("exit_f6_before_remote_deletion", || boundary(8)),
    ("exit_f7_after_remote_deletion", || boundary(9)),
    ("exit_f8_worktree_removing", || boundary(10)),
    ("exit_f9_before_branch_deletion", || boundary(11)),
    ("exit_f10_before_completion", || boundary(12)),
    ("exit_f11_after_completion", || boundary(13)),
    ("exit_f12_before_refresh", || boundary(14)),
    ("exit_inside_f10_git_sql_split_operator_repair", || {
        boundary(F10_SEAM)
    }),
    ("exit_after_sql_commit_replay_preserves_lock", || {
        boundary(F11_SEAM)
    }),
];

pub(super) fn maybe_child() -> bool {
    if std::env::args().nth(1).as_deref() != Some("--closure-exit") {
        return false;
    }
    std::panic::set_hook(Box::new(|info| {
        eprintln!("closure exit child assertion");
        if let Some(l) = info.location() {
            ssh_harness::observation(&[l.line() as u128]);
        }
    }));
    if child_body().is_err() {
        eprintln!("closure exit child failed");
    }
    std::process::exit(1)
}
fn child_body() -> Result<(), FixtureError> {
    let args: Vec<_> = std::env::args().skip(2).collect();
    if args.len() != 5 {
        return Err(FixtureError);
    }
    let point: usize = fixed(args[0].parse())?;
    let digest = fixed(
        args[4]
            .split(',')
            .map(str::parse::<u8>)
            .collect::<Result<Vec<_>, _>>(),
    )?;
    let service = fixed(RepositoryService::open_at_with_exit_point_for_testing(
        Path::new(&args[2]),
        *POINTS.get(point).ok_or(FixtureError)?,
    ))?;
    let request = CloseTicketRequest {
        root: PathBuf::from(&args[1]),
        operation_id: fixed(OperationId::parse(&args[3]))?,
        item_id: item(),
        expected: ClosureExpectation::from_bytes(fixed(digest.try_into())?),
        approval: None,
        confirmed_identity: None,
    };
    let _ = service.close_ticket(
        request,
        &mut SessionCredentials::new(remote_world::Provider),
    );
    Err(FixtureError) // Reaching here means the boundary did not execute.
}
/// Run `request` in a child process that exits at `POINTS[point]`.
pub(super) fn launch(
    p: &Pair,
    side: &Side,
    request: &CloseTicketRequest,
    point: usize,
) -> Result<(), FixtureError> {
    p.privacy()?;
    let output = fixed(
        std::process::Command::new(fixed(std::env::current_exe())?)
            .arg("--closure-exit")
            .arg(point.to_string())
            .arg(&side.root)
            .arg(&side.data)
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
fn tracking_lock(side: &Side) -> Result<PathBuf, FixtureError> {
    Ok(side.repo()?.commondir().join(format!(
        "refs/remotes/origin/manyhands/ticket/{TICKET}.lock"
    )))
}
/// The remote history rows and the unfinished `close` reservation.
fn pending_history(side: &Side, request: &CloseTicketRequest) -> Result<[u8; 32], FixtureError> {
    assert_eq!(step(side, request.operation_id)?, "branch_deleted");
    assert_eq!(count(side, "retired_contexts")?, 0);
    let db = side.db()?;
    assert_eq!(
        fixed(db.query_row(
            "SELECT COUNT(*) FROM remote_operation_records WHERE operation_ulid=?1 AND action='close' AND phase IN ('reserved','advertising','persisting')",
            [request.operation_id.to_string()],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    let mut q = fixed(db.prepare(
        "SELECT remote_ref,tracking_ref,kind,item_id,last_advertised_oid,tracking_oid,publication_evidence,state,observed_at FROM remote_context_states ORDER BY remote_ref",
    ))?;
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
/// The F10/F11 seams exercise deletion of an existing fetched tracking ref:
/// stop after publication and let an ordinary poll fetch the context.
fn published_tracking(p: &Pair, request: &CloseTicketRequest) -> Result<(), FixtureError> {
    stopped(p, &p.a, request, FailurePoint::AfterPublication)?;
    let context = oid(&p.a, CONTEXT)?;
    poll(p, &p.a)?;
    // The poll updates tracking only; the written closure's context is kept.
    assert_eq!(oid(&p.a, TRACKING)?, server_oid(p, CONTEXT)?);
    assert_eq!(oid(&p.a, CONTEXT)?, context);
    Ok(())
}

pub(super) fn boundary(point: usize) -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let base = oid(&p.a, MAIN)?;
    let edited = p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    if point >= F10_SEAM {
        published_tracking(&p, &request)?;
    }
    launch(&p, &p.a, &request, point)?;
    assert_eq!(step(&p.a, request.operation_id)?, STAGES[point]);
    let before = receives(&p, cursor)?;
    let expected = match point {
        0 => [0, 0, 0],
        1..=6 => [1, 0, 0],
        7 | 8 => [1, 1, 0],
        _ => [1, 1, 1],
    };
    assert_eq!(before, expected);
    assert_eq!(p.a.worktree().exists(), point <= 10);
    assert_eq!(p.a.repo()?.find_reference(CONTEXT).is_ok(), point <= 11);
    assert_eq!(server_oid(&p, CONTEXT).is_ok(), (1..=8).contains(&point));
    assert_eq!(commits(&p.a, &close_subject())?, usize::from(point >= 2));
    assert_eq!(commits(&p.a, &merge_subject())?, usize::from(point >= 5));
    if point <= 5 {
        assert_eq!(oid(&p.a, MAIN)?, base);
    } else {
        assert_ne!(oid(&p.a, MAIN)?, base);
    }
    if point <= 6 {
        assert_eq!(server_oid(&p, MAIN)?, base);
    } else {
        assert_eq!(server_oid(&p, MAIN)?, oid(&p.a, MAIN)?);
    }
    let (closed_at, closure) = stored_closure(&p.a, request.operation_id)?;
    assert_eq!(closure.is_some(), point >= 2);
    if point <= DURING_CHECKOUT {
        assert_eq!(oid(&p.a, CONTEXT)?, edited);
    } else if point <= 11 {
        assert_eq!(Some(oid(&p.a, CONTEXT)?.to_string()), closure.clone());
    }
    if (1..=8).contains(&point) {
        // C1: the remote context holds the pre-closure context only.
        assert_eq!(server_oid(&p, CONTEXT)?, edited);
    }
    if point == DURING_CHECKOUT {
        // The checkout ran before any ref lock: no lock survives the exit.
        assert!(
            !p.a.repo()?
                .commondir()
                .join(format!("refs/heads/manyhands/ticket/{TICKET}.lock"))
                .exists()
        );
    }
    p.privacy()?;
    p.a.service = fixed(RepositoryService::open_at(&p.a.data))?;
    if point == F10_SEAM {
        let history = pending_history(&p.a, &request)?;
        let lock = tracking_lock(&p.a)?;
        let bytes = fixed(std::fs::read(&lock))?;
        assert!(bytes.is_empty());
        let outcome = p.run(&p.a, &request)?;
        assert_eq!(
            outcome.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::RecoveryRequired
            }
        );
        assert_eq!(pending_history(&p.a, &request)?, history);
        assert_eq!(receives(&p, cursor)?, before);
        assert_eq!(fixed(std::fs::read(&lock))?, bytes);
        assert!(!p.a.worktree().exists() && p.a.repo()?.find_reference(CONTEXT).is_err());
        p.privacy()?;
        // T6-1: the child is joined, helper audits are drained and nothing
        // else writes Git. Only this verified empty backend lock in this
        // disposable fixture is removed, as the operator's quiesced repair.
        assert_eq!(p.server.active_helpers(), 0);
        assert!(fixed(std::fs::symlink_metadata(&lock))?.is_file());
        fixed(std::fs::remove_file(&lock))?;
    }
    let surviving = if point == F11_SEAM {
        let lock = tracking_lock(&p.a)?;
        let bytes = fixed(std::fs::read(&lock))?;
        Some((lock, bytes))
    } else {
        None
    };
    let outcome = p.run(&p.a, &request)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    assert_eq!(step(&p.a, request.operation_id)?, "complete");
    // A checkpoint written before the exit is finished, never rebuilt: the
    // same `closed_at` and commit.
    let after = stored_closure(&p.a, request.operation_id)?;
    if point >= CLOSING_RECORDED {
        assert_eq!(after, (closed_at, closure));
    }
    assert_eq!(after.1, Some(checkpoint(&outcome)?.to_string()));
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    assert_eq!(receives(&p, cursor)?, [1, 1, 1]);
    assert!(!p.a.worktree().exists() && p.a.repo()?.find_reference(CONTEXT).is_err());
    assert_eq!(server_oid(&p, MAIN)?, oid(&p.a, MAIN)?);
    assert!(server_oid(&p, CONTEXT).is_err());
    assert_eq!(count(&p.a, "retired_contexts")?, 1);
    context_never_held(&p, cursor, checkpoint(&outcome)?)?;
    let main = oid(&p.a, MAIN)?;
    assert_eq!(reachable(&p.a, main, &close_subject())?.len(), 1);
    let complete = p.run(&p.a, &request)?;
    assert_eq!(complete, outcome);
    assert_eq!(receives(&p, cursor)?, [1, 1, 1]);
    if let Some((path, bytes)) = surviving {
        assert_eq!(fixed(std::fs::read(path))?, bytes);
    }
    closed_on_primary(&p)?;
    p.privacy()
}
fn checkpoint(outcome: &TicketClosureOutcome) -> Result<git2::Oid, FixtureError> {
    match outcome.closure() {
        ClosureRecording::Recorded { checkpoint_oid } => Ok(checkpoint_oid),
        ClosureRecording::Existing => Err(FixtureError),
    }
}
fn closed_on_primary(p: &Pair) -> Result<(), FixtureError> {
    let ticket = ticket_at(&p.a, oid(&p.a, MAIN)?)?;
    assert_eq!(ticket.closed_by, Some(p.a.closed_by()));
    assert!(ticket.closed_at.is_some());
    assert_eq!(ticket.status, "open");
    Ok(())
}
