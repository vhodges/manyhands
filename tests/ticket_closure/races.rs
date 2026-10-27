//! Double closure over two clones: a sequential second closure and a true
//! race with distinct identities (C11).
use super::*;

pub(super) const CASES: &[ssh_harness::Case] = &[
    ("sequential_second_closure_stops_before_writing", sequential),
    (
        "true_race_distinct_identities_needs_external_recovery",
        true_race,
    ),
];

/// B confirms its closure before A publishes, then runs after A published.
fn sequential() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let edited = p.edit(&p.a, BODY)?;
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    assert_eq!(oid(&p.b, CONTEXT)?, edited);
    // B's own unpublished work keeps its context ahead of A's.
    let own = commit_files(&linked(&p.b)?, &[("peer-local", b"peer work\n")])?;
    let b_request = p.request(&p.b)?;
    assert!(matches!(
        p.preview(&p.b)?.closure(),
        PlannedClosure::Record { closed_by } if *closed_by == p.b.closed_by()
    ));
    // A's closure publishes its primary and keeps the remote context branch.
    let a_request = p.request(&p.a)?;
    stopped(&p, &p.a, &a_request, FailurePoint::AfterPublication)?;
    assert_eq!(step(&p.a, a_request.operation_id)?, "published");
    let closed = server_oid(&p, MAIN)?;
    assert_eq!(server_oid(&p, CONTEXT)?, edited);
    // B's context child merges A's closed primary into B's context and
    // pushes it; B's re-check then stops before writing anything.
    assert!(matches!(
        p.execute(&p.b.service, b_request.clone()),
        Err(TicketClosureError::ReconfirmationRequired)
    ));
    assert_eq!(step(&p.b, b_request.operation_id)?, "abandoned");
    assert_eq!(stored_closure(&p.b, b_request.operation_id)?, (None, None));
    let context = oid(&p.b, CONTEXT)?;
    let repo = p.b.repo()?;
    assert!(fixed(repo.graph_descendant_of(context, own))?);
    assert!(fixed(repo.graph_descendant_of(context, closed))?);
    assert_eq!(server_oid(&p, CONTEXT)?, context);
    assert_eq!(commits(&p.b, &close_subject())?, 1);
    assert_eq!(reachable(&p.b, context, &close_subject())?.len(), 1);
    assert_eq!(commits(&p.b, &merge_subject())?, 1);
    // B's context now carries A's closure: preserved, refused, AlreadyClosed.
    let ticket = ticket_at(&p.b, context)?;
    assert_eq!(ticket.closed_by, Some(p.a.closed_by()));
    assert_eq!(p.blockers(&p.b)?, [ClosureBlocker::AlreadyClosed]);
    assert!(p.b.worktree().exists());
    let image_before = image(&p.b.worktree())?;
    let bytes = fixed(std::fs::read(p.b.worktree().join(TICKET_PATH)))?;
    let refused = p.b.service.save_ticket(SaveTicketRequest {
        target: p.b.target(ContextIntent::Edit),
        draft: draft("refused peer edit\n"),
        expected_path: ExpectedPathObservation::from_bytes(&bytes),
    });
    assert!(matches!(refused, Err(ref e) if e.kind == RepositoryErrorKind::TicketClosed));
    assert_eq!(image(&p.b.worktree())?, image_before);
    assert_eq!(oid(&p.b, CONTEXT)?, context);
    // A's guarded delete meets B's push and keeps it.
    let finished = p.run(&p.a, &a_request)?;
    assert_eq!(
        finished.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RemoteContextChanged
        }
    );
    assert_eq!(server_oid(&p, CONTEXT)?, context);
    assert_eq!(server_oid(&p, MAIN)?, closed);
    p.privacy()
}

/// Both clones are held after their context children (each stops at F2),
/// so both pass the base-closed check before either publishes; then A, and
/// after it B, are released. This is equivalent to a barrier: the
/// `context_synchronized` resume makes no network call before its lease, so
/// B's base-closed check reads B's stale T(P), exactly as in a concurrent run.
fn true_race() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let edited = p.edit(&p.a, BODY)?;
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    assert_eq!(oid(&p.b, CONTEXT)?, edited);
    let a_request = p.request(&p.a)?;
    let b_request = p.request(&p.b)?;
    stopped(&p, &p.a, &a_request, FailurePoint::AfterContextSync)?;
    stopped(&p, &p.b, &b_request, FailurePoint::AfterContextSync)?;
    assert_eq!(step(&p.a, a_request.operation_id)?, "context_synchronized");
    assert_eq!(step(&p.b, b_request.operation_id)?, "context_synchronized");
    let a = p.run(&p.a, &a_request)?;
    assert_eq!(a.cleanup(), PromotionCleanup::Complete);
    let PromotionIntegration::Merged { oid: a_merge } = a.integration() else {
        return Err(FixtureError);
    };
    let b = p.run(&p.b, &b_request)?;
    let PromotionIntegration::Merged { oid: b_merge } = b.integration() else {
        return Err(FixtureError);
    };
    // The later clone: publication pending a conflict, cleanup waiting.
    assert_eq!(
        b.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::Synchronization(
                PromotionSynchronizationCategory::Conflict
            )
        }
    );
    assert_eq!(
        b.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::AwaitingPublication
        }
    );
    // Both closures stay in history, each with its own identity.
    assert_eq!(server_oid(&p, MAIN)?, a_merge);
    assert_eq!(oid(&p.b, MAIN)?, b_merge);
    assert_eq!(oid(&p.b, TRACKING_MAIN)?, a_merge);
    assert_eq!(ticket_at(&p.b, a_merge)?.closed_by, Some(p.a.closed_by()));
    assert_eq!(ticket_at(&p.b, b_merge)?.closed_by, Some(p.b.closed_by()));
    assert_eq!(reachable(&p.b, a_merge, &close_subject())?.len(), 1);
    assert_eq!(reachable(&p.b, b_merge, &close_subject())?.len(), 1);
    assert_ne!(
        reachable(&p.b, a_merge, &close_subject())?,
        reachable(&p.b, b_merge, &close_subject())?
    );
    // The existing guidance: two different closures cannot be resolved in
    // the product; the conflict needs external recovery.
    let child = fixed(OperationId::parse(
        &primary_child(&p.b, b_request.operation_id)?
            .0
            .ok_or(FixtureError)?,
    ))?;
    let inspection = fixed(
        p.b.service
            .inspect_synchronization_recovery(&p.b.root, child),
    )?;
    p.diagnostics(&format!("{inspection:?}"))?;
    // The conflicted ticket holds a different closure on each side.
    let mut closures = 0;
    for path in &inspection.paths {
        let sides = fixed(p.b.service.read_synchronization_conflict(&path.token))?;
        let holds = |bytes: &Option<RedactedConflictBytes>, by: &str| {
            bytes
                .as_ref()
                .is_some_and(|b| ssh_privacy::clean(b.bytes(), &[by.as_bytes().to_vec()]).is_err())
        };
        if holds(&sides.local, &p.b.closed_by()) && holds(&sides.incoming, &p.a.closed_by()) {
            closures += 1;
            assert_eq!(
                path.eligibility,
                ConflictEligibility::ExternalResolutionRequired
            );
        }
    }
    assert_eq!(closures, 1);
    let main_image = image(&p.b.root)?;
    // Neither side's closure may replace the other's: each in-product
    // resolution is refused and writes nothing.
    for incoming in [false, true] {
        let mut results = Vec::new();
        for path in &inspection.paths {
            let sides = fixed(p.b.service.read_synchronization_conflict(&path.token))?;
            let side = if incoming {
                sides.incoming
            } else {
                sides.local
            };
            results.push((path.token.clone(), side.ok_or(FixtureError)?));
        }
        let resolution =
            p.b.service
                .resolve_synchronization(ResolveSynchronizationRequest::new(
                    p.b.root.clone(),
                    child,
                    OperationId::new(),
                    inspection.observation.clone(),
                    results,
                    None,
                ));
        p.diagnostics(&format!("{resolution:?}"))?;
        // Fresh classification refuses before accepting a resolution attempt.
        assert!(matches!(
            resolution,
            Err(SynchronizationError::ExternalResolutionRequired { .. })
        ));
        assert_eq!(image(&p.b.root)?, main_image);
    }
    assert_eq!(oid(&p.b, MAIN)?, b_merge);
    assert_eq!(server_oid(&p, MAIN)?, a_merge);
    // After the refused resolutions the same ID stays pending a conflict,
    // with both closures still in history; a later closure in B is blocked
    // by B's diverged primary.
    let again = p.run(&p.b, &b_request)?;
    assert_eq!(
        again.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::Synchronization(
                PromotionSynchronizationCategory::Conflict
            )
        }
    );
    assert_eq!(again.cleanup(), b.cleanup());
    assert_eq!(ticket_at(&p.b, a_merge)?.closed_by, Some(p.a.closed_by()));
    assert_eq!(
        ticket_at(&p.b, oid(&p.b, MAIN)?)?.closed_by,
        Some(p.b.closed_by())
    );
    assert_eq!(
        reachable(&p.b, oid(&p.b, MAIN)?, &close_subject())?.len(),
        1
    );
    assert_eq!(reachable(&p.b, a_merge, &close_subject())?.len(), 1);
    assert_eq!(server_oid(&p, MAIN)?, a_merge);
    let other: canonical::ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FAY".parse().unwrap();
    fixed(p.b.service.save_ticket(SaveTicketRequest {
        target: AuthoringTarget {
            root: p.b.root.clone(),
            kind: AuthoringKind::Ticket,
            item_id: other.clone(),
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        },
        draft: draft(BODY),
        expected_path: ExpectedPathObservation::Missing,
    }))?;
    let preflight = fixed(
        p.b.service
            .preview_ticket_closure(PreviewTicketClosureRequest {
                root: p.b.root.clone(),
                item_id: other,
                confirmed_identity: None,
            }),
    )?;
    p.diagnostics(&format!("{preflight:?}"))?;
    assert!(matches!(
        preflight,
        TicketClosurePreflight::Blocked { ref blockers, .. }
            if blockers.contains(&ClosureBlocker::PrimaryAheadOfRemote)
    ));
    p.privacy()
}
