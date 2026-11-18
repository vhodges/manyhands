//! The three closure checkpoint seams by real child-process exit, each with
//! an advanced and with a conflicting published primary before the retry.
use super::exits::{AFTER_CHECKPOINT, CLOSING_RECORDED, DURING_CHECKOUT, launch};
use super::*;

pub(super) const CASES: &[ssh_harness::Case] = &[
    ("seam_closing_recorded_advanced_primary_abandons", || {
        seam(CLOSING_RECORDED, false)
    }),
    ("seam_closing_recorded_conflicting_primary_abandons", || {
        seam(CLOSING_RECORDED, true)
    }),
    ("seam_during_checkout_advanced_primary_merges", || {
        seam(DURING_CHECKOUT, false)
    }),
    ("seam_during_checkout_conflicting_primary_recovers", || {
        seam(DURING_CHECKOUT, true)
    }),
    ("seam_after_checkpoint_advanced_primary_merges", || {
        seam(AFTER_CHECKPOINT, false)
    }),
    ("seam_after_checkpoint_conflicting_primary_recovers", || {
        seam(AFTER_CHECKPOINT, true)
    }),
];

/// The user's deliberate synchronization over a written checkpoint (C9,
/// R-T6-1): the tracking primary merged into the ticket context with the
/// user's own Git, keeping the context's closed ticket.
fn synchronize_by_hand(side: &Side) -> Result<git2::Oid, FixtureError> {
    let repo = linked(side)?;
    let ours = fixed(fixed(repo.head())?.peel_to_commit())?;
    let theirs = fixed(repo.find_commit(oid(side, TRACKING_MAIN)?))?;
    let mut index = fixed(repo.merge_commits(&ours, &theirs, None))?;
    if index.has_conflicts() {
        let closed = blob(&repo, ours.id(), TICKET_PATH)?;
        let entry = git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: closed.len() as u32,
            id: fixed(repo.blob(&closed))?,
            flags: 0,
            flags_extended: 0,
            path: TICKET_PATH.as_bytes().to_vec(),
        };
        fixed(index.conflict_remove(Path::new(TICKET_PATH)))?;
        fixed(index.add(&entry))?;
    }
    assert!(!index.has_conflicts());
    let tree = fixed(repo.find_tree(fixed(index.write_tree_to(&repo))?))?;
    let sig = fixed(git2::Signature::now(
        "Collaborator",
        "fixture@example.invalid",
    ))?;
    let merged = fixed(repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Deliberate context synchronization",
        &tree,
        &[&ours, &theirs],
    ))?;
    fixed(repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())))?;
    Ok(merged)
}

fn seam(point: usize, conflicting: bool) -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let edited = p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    launch(&p, &p.a, &request, point)?;
    assert_eq!(step(&p.a, request.operation_id)?, "closing");
    let stopped = stored_closure(&p.a, request.operation_id)?;
    let checkpoint = fixed(git2::Oid::from_str(
        stopped.1.as_deref().ok_or(FixtureError)?,
    ))?;
    let closed = blob(&p.a.repo()?, checkpoint, TICKET_PATH)?;
    match point {
        CLOSING_RECORDED => assert_eq!(oid(&p.a, CONTEXT)?, edited),
        DURING_CHECKOUT => {
            assert_eq!(oid(&p.a, CONTEXT)?, edited);
            assert_eq!(
                fixed(std::fs::read(p.a.worktree().join(TICKET_PATH)))?,
                closed
            );
        }
        _ => assert_eq!(oid(&p.a, CONTEXT)?, checkpoint),
    }
    let context = oid(&p.a, CONTEXT)?;
    // A collaborator publishes a primary advance; A's ordinary poll observes
    // it and never moves the context of the unfinished closure.
    if conflicting {
        commit_files(
            &p.b.repo()?,
            &[(
                TICKET_PATH,
                ticket_source("conflicting primary body\n", None).as_bytes(),
            )],
        )?;
    } else {
        commit_files(&p.b.repo()?, &[("peer-main", b"primary advance\n")])?;
    }
    fixed(p.sync_target(&p.b, SynchronizationTarget::Primary))?;
    let advanced = server_oid(&p, MAIN)?;
    poll(&p, &p.a)?;
    assert_eq!(oid(&p.a, TRACKING_MAIN)?, advanced);
    assert_eq!(oid(&p.a, CONTEXT)?, context);
    let pushes = p.server.receive_updates().len();
    p.a.service = fixed(RepositoryService::open_at(&p.a.data))?;
    if point == CLOSING_RECORDED {
        return nothing_written(&p, &request, checkpoint, advanced, conflicting);
    }
    if conflicting {
        // C9 step 5: the in-memory merge conflicts; nothing is written and
        // the record is kept for the same ID.
        assert!(matches!(
            p.execute(&p.a.service, request.clone()),
            Err(TicketClosureError::RecoveryRequired)
        ));
        assert_eq!(step(&p.a, request.operation_id)?, "closing");
        assert_eq!(stored_closure(&p.a, request.operation_id)?, stopped);
        assert_eq!(oid(&p.a, CONTEXT)?, checkpoint);
        assert_eq!(server_oid(&p, MAIN)?, advanced);
        assert_eq!(commits(&p.a, &merge_subject())?, 0);
        p.privacy()?;
    }
    let tip = if conflicting {
        synchronize_by_hand(&p.a)?
    } else {
        checkpoint
    };
    let outcome = p.run(&p.a, &request)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    assert_eq!(
        outcome.closure(),
        ClosureRecording::Recorded {
            checkpoint_oid: checkpoint
        }
    );
    let PromotionIntegration::Merged { oid: merged } = outcome.integration() else {
        return Err(FixtureError);
    };
    // The checkpoint's own authority: the same `closed_at`, one checkpoint,
    // merged over the advanced primary.
    assert_eq!(stored_closure(&p.a, request.operation_id)?, stopped);
    let repo = p.a.repo()?;
    let merge = fixed(repo.find_commit(merged))?;
    assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [advanced, tip]);
    assert_eq!(merge.author().email(), Some(EMAIL));
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    assert_eq!(server_oid(&p, MAIN)?, merged);
    let ticket = ticket_at(&p.a, merged)?;
    assert_eq!(ticket.closed_by, Some(p.a.closed_by()));
    assert_eq!(
        ticket.closed_at.map(|t| t
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()),
        stopped.0
    );
    if !conflicting {
        assert_eq!(
            blob(&p.a.repo()?, merged, "peer-main")?,
            b"primary advance\n"
        );
    }
    // No context child ran after the checkpoint: the remote context branch
    // received only its deletion.
    assert!(
        p.server.receive_updates()[pushes..]
            .iter()
            .all(|u| u.reference != CONTEXT || u.new_oid.is_zero())
    );
    context_never_held(&p, cursor, checkpoint)?;
    assert!(server_oid(&p, CONTEXT).is_err());
    assert!(!p.a.worktree().exists());
    p.privacy()
}

/// `closing` with nothing moved: the consent is checked again, and the
/// advanced base is a refusal that abandons (R-T4-3), with only the dangling
/// unreferenced checkpoint left. A new confirmation then closes once.
fn nothing_written(
    p: &Pair,
    request: &CloseTicketRequest,
    dangling: git2::Oid,
    advanced: git2::Oid,
    conflicting: bool,
) -> Result<(), FixtureError> {
    let context = oid(&p.a, CONTEXT)?;
    assert!(matches!(
        p.execute(&p.a.service, request.clone()),
        Err(TicketClosureError::ReconfirmationRequired)
    ));
    assert_eq!(step(&p.a, request.operation_id)?, "abandoned");
    assert_eq!(oid(&p.a, CONTEXT)?, context);
    assert_eq!(server_oid(p, MAIN)?, advanced);
    let repo = p.a.repo()?;
    assert!(
        fixed(repo.references())?
            .filter_map(|r| r.ok().and_then(|r| r.target()))
            .all(|target| target != dangling)
    );
    assert_eq!(commits(&p.a, &merge_subject())?, 0);
    if conflicting {
        // The primary's ticket now conflicts with the context's: the next
        // preview refuses until the user synchronizes the context.
        assert_eq!(p.blockers(&p.a)?, [ClosureBlocker::IntegrationConflict]);
        let Err(SynchronizationError::ConflictPending {
            operation_id: child,
            ..
        }) = p.sync(&p.a)
        else {
            return Err(FixtureError);
        };
        resolve_local(p, &p.a, child)?;
        fixed(p.sync(&p.a))?;
    }
    let outcome = p.close(&p.a)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    let PromotionIntegration::Merged { oid: merged } = outcome.integration() else {
        return Err(FixtureError);
    };
    let ClosureRecording::Recorded { checkpoint_oid } = outcome.closure() else {
        return Err(FixtureError);
    };
    assert_ne!(checkpoint_oid, dangling);
    assert_eq!(
        fixed(repo.find_commit(merged))?.parent_id(0).ok(),
        Some(advanced)
    );
    assert_eq!(reachable(&p.a, merged, &close_subject())?, [checkpoint_oid]);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    let ticket = ticket_at(&p.a, merged)?;
    assert_eq!(ticket.closed_by, Some(p.a.closed_by()));
    assert_eq!(ticket.body, BODY);
    assert!(server_oid(p, CONTEXT).is_err());
    p.privacy()
}
