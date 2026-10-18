//! Real Git/SSH closure journeys over two clones, the blockers with full
//! preservation snapshots, poll priority, pending comment publication, the
//! C5 guard and discoverability.
use super::*;

pub(super) const CASES: &[ssh_harness::Case] = &[
    ("happy_collaborator_poll_and_context_never_held", happy),
    ("local_only_then_primary_publication", local_only),
    ("existing_closure_fields_merge_without_checkpoint", existing),
    ("contained_closed_collaborator_cleanup_only", || {
        contained_closed(false)
    }),
    (
        "contained_closed_cleanup_after_cache_loss_remote_gone",
        || contained_closed(true),
    ),
    (
        "contained_open_closes_without_context_child",
        contained_open,
    ),
    ("context_conflict_resolve_same_id_closes_once", conflict),
    ("poll_advances_primary_mid_closure_retry_once", poll_advance),
    ("primary_only_advance_preserves_consent", || {
        changed_context(true)
    }),
    ("changed_remote_context_reconfirms", || {
        changed_context(false)
    }),
    ("primary_rejection_retry_cleans_up_only", primary_rejection),
    ("deletion_rejection_retry_deletes_only", deletion_rejection),
    (
        "remote_context_race_at_deletion_preserves_peer",
        deletion_race,
    ),
    ("offline_before_integration_identical_retry", || {
        offline(false)
    }),
    ("offline_after_integration_identical_retry", || {
        offline(true)
    }),
    ("preserve_dirty_context_after_integration", || {
        preservation(0)
    }),
    ("preserve_untracked_context_after_integration", || {
        preservation(1)
    }),
    ("preserve_ignored_context_after_integration", || {
        preservation(2)
    }),
    ("preserve_moved_context_after_integration", || {
        preservation(3)
    }),
    ("automatic_poll_advertisement_yields_to_closure", priority),
    (
        "pending_comment_publication_blocks_then_closes",
        comment_pending,
    ),
    (
        "guard_spares_own_checkpoint_and_refuses_later_writes",
        guard,
    ),
    ("closed_ticket_discoverable_from_both_clones", discoverable),
];

pub(super) const BLOCKERS: &[ssh_harness::Case] = &[
    ("blocker_not_a_ticket", || blocker(0)),
    ("blocker_context_missing", || blocker(1)),
    ("blocker_context_nonconforming", || blocker(2)),
    ("blocker_context_dirty", || blocker(3)),
    ("blocker_context_conflicted", || blocker(4)),
    ("blocker_conflict_pending", || blocker(5)),
    ("blocker_local_operation_pending", || blocker(6)),
    ("blocker_comment_publication_pending", || blocker(7)),
    ("blocker_primary_missing", || blocker(8)),
    ("blocker_primary_not_checked_out", || blocker(9)),
    ("blocker_primary_dirty", || blocker(10)),
    ("blocker_primary_conflicted", || blocker(11)),
    ("blocker_primary_ahead_of_remote", || blocker(12)),
    ("blocker_context_behind_remote", || blocker(13)),
    ("blocker_integration_conflict", || blocker(14)),
    ("blocker_remote_context_deleted", || blocker(15)),
    ("blocker_history_unknown", || blocker(16)),
    ("blocker_already_closed", || blocker(17)),
    ("blocker_primary_reopened", || blocker(18)),
    ("blocker_ticket_invalid_after_closure", || blocker(19)),
    ("blocker_identity_required", || blocker(20)),
];

const DOC: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";

fn merged(outcome: &TicketClosureOutcome) -> Result<git2::Oid, FixtureError> {
    match outcome.integration() {
        PromotionIntegration::Merged { oid } => Ok(oid),
        _ => Err(FixtureError),
    }
}
fn checkpoint_of(outcome: &TicketClosureOutcome) -> Result<git2::Oid, FixtureError> {
    match outcome.closure() {
        ClosureRecording::Recorded { checkpoint_oid } => Ok(checkpoint_oid),
        ClosureRecording::Existing => Err(FixtureError),
    }
}
/// The primary ticket of `side` carries exactly `by`, with the status kept.
fn closed_on(side: &Side, tip: git2::Oid, by: &str) -> Result<canonical::Ticket, FixtureError> {
    let ticket = ticket_at(side, tip)?;
    assert_eq!(ticket.closed_by.as_deref(), Some(by));
    assert!(ticket.closed_at.is_some());
    assert_eq!(ticket.status, "open");
    assert_eq!(ticket.title, TITLE);
    Ok(ticket)
}
fn cleaned(p: &Pair, side: &Side) -> Result<(), FixtureError> {
    assert!(!side.worktree().exists());
    assert!(side.repo()?.find_reference(CONTEXT).is_err());
    assert!(server_oid(p, CONTEXT).is_err());
    Ok(())
}
fn zero_request(p: &Pair, side: &Side, id: canonical::ItemId) -> CloseTicketRequest {
    CloseTicketRequest {
        root: side.root.clone(),
        operation_id: OperationId::new(),
        item_id: id,
        expected: ClosureExpectation::from_bytes([0; 32]),
        approval: Some(p.approval()),
        confirmed_identity: None,
    }
}
fn preflight_of(
    p: &Pair,
    side: &Side,
    id: canonical::ItemId,
) -> Result<TicketClosurePreflight, FixtureError> {
    let preflight = fixed(
        side.service
            .preview_ticket_closure(PreviewTicketClosureRequest {
                root: side.root.clone(),
                item_id: id,
                confirmed_identity: None,
            }),
    )?;
    p.diagnostics(&format!("{preflight:?}"))?;
    Ok(preflight)
}

fn happy() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let base = oid(&p.a, MAIN)?;
    let edited = p.edit(&p.a, BODY)?;
    let outcome = p.close(&p.a)?;
    let checkpoint = checkpoint_of(&outcome)?;
    let merged = merged(&outcome)?;
    assert_eq!(
        outcome.publication(),
        PromotionPublication::Published { oid: merged }
    );
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    let repo = p.a.repo()?;
    let merge = fixed(repo.find_commit(merged))?;
    assert_eq!(merge.summary(), Some(merge_subject().as_str()));
    assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [base, checkpoint]);
    let close = fixed(repo.find_commit(checkpoint))?;
    assert_eq!(close.summary(), Some(close_subject().as_str()));
    assert_eq!(close.parent_ids().collect::<Vec<_>>(), [edited]);
    assert_eq!(close.author().name(), Some(NAME));
    assert_eq!(close.author().email(), Some(EMAIL));
    let ticket = closed_on(&p.a, merged, &p.a.closed_by())?;
    assert_eq!(ticket.body, BODY);
    let (closed_at, _) = stored_closure(&p.a, outcome.operation_id())?;
    assert_eq!(
        closed_at,
        ticket.closed_at.map(|t| t
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap())
    );
    assert_eq!(server_oid(&p, MAIN)?, merged);
    assert_eq!(oid(&p.a, MAIN)?, merged);
    cleaned(&p, &p.a)?;
    assert_eq!(receives(&p, cursor)?, [1, 1, 1]);
    // C1: the remote context branch only ever held the pre-closure context.
    context_never_held(&p, cursor, checkpoint)?;
    assert!(
        p.server.receive_updates()[cursor..]
            .iter()
            .any(|u| u.reference == CONTEXT && u.new_oid == edited && u.accepted)
    );
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    assert_eq!(count(&p.a, "retired_contexts")?, 1);
    // B sees the closed primary and reports nothing for the deleted context.
    let report = poll(&p, &p.b)?;
    assert_eq!(ticket_disposition(&report), None);
    assert_eq!(oid(&p.b, MAIN)?, merged);
    assert!(!p.b.worktree().exists());
    closed_on(&p.b, oid(&p.b, MAIN)?, &p.a.closed_by())?;
    let again = poll(&p, &p.a)?;
    assert_eq!(ticket_disposition(&again), None);
    assert!(!p.a.worktree().exists());
    p.privacy()
}

fn local_only() -> Result<(), FixtureError> {
    let p = Pair::new_with_remote(false)?;
    fixed(p.a.service.remove_remote(RemoveRemoteRequest {
        root: p.a.root.clone(),
        name: "origin".into(),
        operation_id: OperationId::new(),
    }))?;
    assert!(fixed(p.a.repo()?.remotes())?.is_empty());
    let connections = p.server.accepted_sockets();
    let base = oid(&p.a, MAIN)?;
    p.edit(&p.a, BODY)?;
    let preview = p.preview(&p.a)?;
    assert_eq!(preview.publication_remote(), None);
    let outcome = p.close(&p.a)?;
    assert_eq!(p.server.accepted_sockets(), connections);
    assert_eq!(
        outcome.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::NoPublicationRemote
        }
    );
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    let merged = merged(&outcome)?;
    assert_eq!(oid(&p.a, MAIN)?, merged);
    assert_eq!(server_oid(&p, MAIN)?, base);
    assert!(!p.a.worktree().exists());
    fixed(p.a.service.add_remote(AddRemoteRequest {
        root: p.a.root.clone(),
        name: "origin".into(),
        url: p.server.url(),
        operation_id: OperationId::new(),
    }))?;
    fixed(
        p.a.service
            .set_publication_remote(SetPublicationRemoteRequest {
                root: p.a.root.clone(),
                name: Some("origin".into()),
                operation_id: OperationId::new(),
            }),
    )?;
    // Publication selection keeps the live index (Cycle 09 local_only): the
    // user's Git aligns only that committed configuration entry.
    let repo = p.a.repo()?;
    let mut index = fixed(repo.index())?;
    fixed(index.add_path(Path::new(".manyhands/config.toml")))?;
    fixed(index.write())?;
    fixed(p.sync_target(&p.a, SynchronizationTarget::Primary))?;
    let published = server_oid(&p, MAIN)?;
    assert_eq!(published, oid(&p.a, MAIN)?);
    assert!(published == merged || fixed(repo.graph_descendant_of(published, merged))?);
    assert_eq!(reachable(&p.a, published, &merge_subject())?, [merged]);
    closed_on(&p.a, published, &p.a.closed_by())?;
    assert!(server_oid(&p, CONTEXT).is_err());
    p.privacy()
}

fn existing() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = oid(&p.a, MAIN)?;
    p.edit(&p.a, BODY)?;
    // The context already carries both closure fields, written by hand.
    let by = p.b.closed_by();
    let tip = commit_files(
        &linked(&p.a)?,
        &[(
            TICKET_PATH,
            ticket_source(BODY, Some((1_800_000_000, &by))).as_bytes(),
        )],
    )?;
    let preview = p.preview(&p.a)?;
    assert_eq!(
        preview.closure(),
        &PlannedClosure::Existing {
            closed_at: "2027-01-15T08:00:00Z".into(),
            closed_by: by.clone(),
        }
    );
    assert_eq!(preview.integration(), PlannedIntegration::MergeCommit);
    let outcome = p.close(&p.a)?;
    assert_eq!(outcome.closure(), ClosureRecording::Existing);
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    let merged = merged(&outcome)?;
    let repo = p.a.repo()?;
    let merge = fixed(repo.find_commit(merged))?;
    assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [base, tip]);
    assert_eq!(commits(&p.a, &close_subject())?, 0);
    let ticket = closed_on(&p.a, merged, &by)?;
    assert_eq!(
        ticket.closed_at,
        Some(time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap())
    );
    assert_eq!(stored_closure(&p.a, outcome.operation_id())?, (None, None));
    assert_eq!(server_oid(&p, MAIN)?, merged);
    cleaned(&p, &p.a)?;
    p.privacy()
}

fn contained_closed(cache_loss: bool) -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    let edited = p.edit(&p.a, BODY)?;
    if cache_loss {
        // A's closure stops after the remote deletion (F7); the registry is
        // then lost, so the old record cannot finish the local cleanup.
        let request = p.request(&p.a)?;
        stopped(&p, &p.a, &request, FailurePoint::AfterRemoteDeletion)?;
        assert_eq!(step(&p.a, request.operation_id)?, "remote_deleted");
        assert!(server_oid(&p, CONTEXT).is_err());
        let primary = oid(&p.a, MAIN)?;
        let updates = p.server.receive_updates();
        lose_registry(&p.server, &mut p.a)?;
        poll(&p, &p.a)?;
        let preview = p.preview(&p.a)?;
        assert_eq!(preview.integration(), PlannedIntegration::AlreadyContained);
        assert!(!preview.cleanup().remote_branch);
        assert!(matches!(
            preview.closure(),
            PlannedClosure::Existing { closed_by, .. } if *closed_by == p.a.closed_by()
        ));
        let outcome = p.close(&p.a)?;
        assert_ne!(outcome.operation_id(), request.operation_id);
        assert_eq!(
            outcome.integration(),
            PromotionIntegration::AlreadyContained { oid: primary }
        );
        assert_eq!(outcome.closure(), ClosureRecording::Existing);
        assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
        assert_eq!(p.server.receive_updates(), updates);
        assert_eq!(commits(&p.a, &close_subject())?, 1);
        assert_eq!(commits(&p.a, &merge_subject())?, 1);
        assert_eq!(count(&p.a, "retired_contexts")?, 1);
        cleaned(&p, &p.a)?;
        return p.privacy();
    }
    // B's context is the published pre-closure context; A closes and deletes
    // the remote branch; B's poll keeps B's context, now contained.
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    assert_eq!(oid(&p.b, CONTEXT)?, edited);
    let closed = p.close(&p.a)?;
    let primary = merged(&closed)?;
    let report = poll(&p, &p.b)?;
    assert_eq!(
        ticket_disposition(&report),
        Some(PollDisposition::RemotelyDeleted)
    );
    assert!(p.b.worktree().exists());
    // The collaborator's kept context is still open while its primary is
    // closed. C5 refuses a new write there (the primary-tip arm) and nothing
    // changes; the listing reads the kept context's open copy (R-T6-6).
    let image_before = image(&p.b.root)?;
    let context = oid(&p.b, CONTEXT)?;
    let bytes = fixed(std::fs::read(p.b.worktree().join(TICKET_PATH)))?;
    let refused = p.b.service.save_ticket(SaveTicketRequest {
        target: p.b.target(ContextIntent::Edit),
        draft: draft("refused collaborator edit\n"),
        expected_path: ExpectedPathObservation::from_bytes(&bytes),
    });
    assert!(matches!(refused, Err(ref e) if e.kind == RepositoryErrorKind::TicketClosed));
    assert_eq!(image(&p.b.root)?, image_before);
    assert_eq!(oid(&p.b, CONTEXT)?, context);
    let repository = fixed(p.b.service.resolve_repository(&p.b.root))?;
    let listed = fixed(
        p.b.service
            .list_tickets(&repository, &TicketFilter::default()),
    )?;
    let copies: Vec<_> = listed
        .items
        .iter()
        .filter(|i| i.id.as_deref() == Some(TICKET))
        .collect();
    assert_eq!(copies.len(), 1);
    ssh_harness::observation(&[
        920,
        match copies[0].context.kind {
            ItemContextKind::Primary => 0,
            ItemContextKind::Unverified => 1,
            ItemContextKind::Active => 2,
        },
        match copies[0].closure.as_ref().map(|c| c.state) {
            None => 0,
            Some(ClosureState::Open) => 1,
            Some(ClosureState::Closed) => 2,
        },
    ]);
    assert_eq!(copies[0].context.kind, ItemContextKind::Active);
    assert_eq!(copies[0].status.as_deref(), Some("open"));
    assert_eq!(
        copies[0].closure,
        Some(ClosureDto {
            state: ClosureState::Open,
            closed_at: None,
            closed_by: None,
        })
    );
    assert_eq!(oid(&p.b, MAIN)?, primary);
    let preview = p.preview(&p.b)?;
    assert_eq!(preview.integration(), PlannedIntegration::AlreadyContained);
    assert!(matches!(
        preview.closure(),
        PlannedClosure::Existing { closed_by, .. } if *closed_by == p.a.closed_by()
    ));
    let updates = p.server.receive_updates();
    let outcome = p.close(&p.b)?;
    assert_eq!(
        outcome.integration(),
        PromotionIntegration::AlreadyContained { oid: primary }
    );
    assert_eq!(outcome.closure(), ClosureRecording::Existing);
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    assert!(
        p.server.receive_updates()[updates.len()..]
            .iter()
            .all(|u| u.reference != MAIN)
    );
    assert_eq!(commits(&p.b, &merge_subject())?, 1);
    assert_eq!(count(&p.b, "retired_contexts")?, 1);
    cleaned(&p, &p.b)?;
    p.privacy()
}

fn contained_open() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let context = p.edit(&p.a, BODY)?;
    // The context is merged into the primary by hand, without closure fields.
    let repo = p.a.repo()?;
    fixed(repo.reference(MAIN, context, true, "fixture hand merge"))?;
    fixed(repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())))?;
    fixed(p.sync_target(&p.a, SynchronizationTarget::Primary))?;
    assert_eq!(server_oid(&p, MAIN)?, context);
    // Publication does not advance T(P): an ordinary poll observes it.
    poll(&p, &p.a)?;
    let preview = p.preview(&p.a)?;
    assert_eq!(preview.integration(), PlannedIntegration::MergeCommit);
    assert!(matches!(preview.closure(), PlannedClosure::Record { .. }));
    let outcome = p.close(&p.a)?;
    let checkpoint = checkpoint_of(&outcome)?;
    let merged = merged(&outcome)?;
    assert_eq!(
        fixed(repo.find_commit(checkpoint))?
            .parent_ids()
            .collect::<Vec<_>>(),
        [context]
    );
    assert_eq!(
        fixed(repo.find_commit(merged))?
            .parent_ids()
            .collect::<Vec<_>>(),
        [context, checkpoint]
    );
    assert_eq!(context_child(&p.a, outcome.operation_id())?, (None, 0));
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    // No context child ran: the remote context branch was never written.
    assert!(
        p.server.receive_updates()[cursor..]
            .iter()
            .all(|u| u.reference != CONTEXT)
    );
    closed_on(&p.a, server_oid(&p, MAIN)?, &p.a.closed_by())?;
    cleaned(&p, &p.a)?;
    p.privacy()
}

fn conflict() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    let left = p.edit(&p.a, &(BODY.to_owned() + "left\n"))?;
    let right = p.edit(&p.b, &(BODY.to_owned() + "right\n"))?;
    fixed(p.sync(&p.b))?;
    let request = p.request(&p.a)?;
    let Err(TicketClosureError::ContextSynchronization(error)) =
        p.execute(&p.a.service, request.clone())
    else {
        return Err(FixtureError);
    };
    let SynchronizationError::ConflictPending {
        operation_id: child,
        ..
    } = *error
    else {
        return Err(FixtureError);
    };
    assert!(fixed(linked(&p.a)?.index())?.has_conflicts());
    assert_eq!(commits(&p.a, &close_subject())?, 0);
    let resolved = resolve_local(&p, &p.a, child)?;
    assert_eq!(
        fixed(p.a.repo()?.find_commit(resolved))?
            .parent_ids()
            .collect::<Vec<_>>(),
        [left, right]
    );
    let outcome = p.run(&p.a, &request)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    let checkpoint = checkpoint_of(&outcome)?;
    assert_eq!(
        fixed(p.a.repo()?.find_commit(checkpoint))?
            .parent_ids()
            .collect::<Vec<_>>(),
        [resolved]
    );
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    assert_eq!(context_child(&p.a, request.operation_id)?.1, 1);
    p.privacy()
}

fn poll_advance() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    let base = oid(&p.a, MAIN)?;
    stopped(&p, &p.a, &request, FailurePoint::AfterContextSync)?;
    assert_eq!(step(&p.a, request.operation_id)?, "context_synchronized");
    let (old_child, _) = context_child(&p.a, request.operation_id)?;
    commit_files(&p.b.repo()?, &[("poll-primary", b"published peer\n")])?;
    fixed(p.sync_target(&p.b, SynchronizationTarget::Primary))?;
    fixed(p.a.service.set_remote_polling(
        &p.a.root,
        true,
        false,
        PollingInterval::from_seconds(120).unwrap(),
    ))?;
    let mut session = SessionCredentials::new(remote_world::Provider);
    fixed(p.a.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: p.a.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(p.approval()),
        },
        &mut session,
    ))?;
    let automatic = p.a.service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: p.a.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Automatic,
            approval: Some(p.approval()),
            restart: false,
        },
        &mut session,
    );
    p.diagnostics(&format!("{automatic:?}"))?;
    assert!(
        matches!(fixed(automatic)?,RemotePollResult::Complete(RemotePollOutcome::Polled(ref report)) if report.completion()==PollCompletion::Completed)
    );
    let primary = oid(&p.a, MAIN)?;
    assert_ne!(primary, base);
    assert!(matches!(
        p.execute(&p.a.service, request.clone()),
        Err(TicketClosureError::PrimaryAdvanced)
    ));
    assert_eq!(oid(&p.a, MAIN)?, primary);
    assert_eq!(commits(&p.a, &close_subject())?, 0);
    let outcome = p.run(&p.a, &request)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    let (new_child, attempt) = context_child(&p.a, request.operation_id)?;
    assert_ne!(new_child, old_child);
    assert!(attempt > 1);
    assert_eq!(
        fixed(p.a.db()?.query_row(
            "SELECT COUNT(*) FROM remote_operation_records WHERE action='synchronize_context'",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        2
    );
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    let published = server_oid(&p, MAIN)?;
    assert_eq!(
        blob(&p.a.repo()?, published, "poll-primary")?,
        b"published peer\n"
    );
    closed_on(&p.a, published, &p.a.closed_by())?;
    p.privacy()
}

fn changed_context(primary: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    let request = p.request(&p.a)?;
    let base = oid(&p.a, MAIN)?;
    if primary {
        commit_files(&p.b.repo()?, &[("peer-main", b"primary advance\n")])?;
        fixed(p.sync_target(&p.b, SynchronizationTarget::Primary))?;
        let outcome = p.run(&p.a, &request)?;
        assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
        assert_eq!(
            blob(&p.a.repo()?, oid(&p.a, MAIN)?, "peer-main")?,
            b"primary advance\n"
        );
    } else {
        commit_files(&linked(&p.b)?, &[("peer-context", b"context advance\n")])?;
        fixed(p.sync(&p.b))?;
        assert!(matches!(
            p.execute(&p.a.service, request.clone()),
            Err(TicketClosureError::ReconfirmationRequired)
        ));
        assert_eq!(step(&p.a, request.operation_id)?, "abandoned");
        assert_eq!(oid(&p.a, MAIN)?, base);
        assert_eq!(commits(&p.a, &close_subject())?, 0);
        let preview = p.preview(&p.a)?;
        assert!(
            preview
                .changes()
                .iter()
                .any(|c| c.path() == Path::new("peer-context"))
        );
        assert_ne!(preview.expectation(), request.expected);
        p.close(&p.a)?;
    }
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    p.privacy()
}

fn primary_rejection() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    let server = server_oid(&p, MAIN)?;
    p.server.reject_primary_updates(true)?;
    let result = p.run(&p.a, &request)?;
    assert_eq!(
        result.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::Synchronization(
                PromotionSynchronizationCategory::Rejected
            )
        }
    );
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::AwaitingPublication
        }
    );
    assert_eq!(server_oid(&p, MAIN)?, server);
    assert!(p.a.worktree().exists());
    let checkpoint = checkpoint_of(&result)?;
    assert_eq!(oid(&p.a, CONTEXT)?, checkpoint);
    assert_ne!(server_oid(&p, CONTEXT)?, checkpoint);
    p.privacy()?;
    p.server.reject_primary_updates(false)?;
    let retried = p.run(&p.a, &request)?;
    assert_eq!(retried.cleanup(), PromotionCleanup::Complete);
    assert_eq!(retried.closure(), result.closure());
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    // Only the unfinished publication and cleanup ran on the retry.
    assert_eq!(receives_accepted(&p, cursor)?, [1, 1, 1]);
    context_never_held(&p, cursor, checkpoint)?;
    p.privacy()
}
/// Accepted updates only (rejected attempts are audit records too).
fn receives_accepted(p: &Pair, cursor: usize) -> Result<[usize; 3], FixtureError> {
    p.server.wait_for_helper_audits()?;
    let mut counts = [0; 3];
    for u in p.server.receive_updates()[cursor..]
        .iter()
        .filter(|u| u.accepted)
    {
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

fn deletion_rejection() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let edited = p.edit(&p.a, BODY)?;
    fixed(p.sync(&p.a))?;
    let request = p.request(&p.a)?;
    p.server.reject_context_deletion(true)?;
    // The fixture's native refusal applies to the receiver's current branch.
    fixed(fixed(git2::Repository::open_bare(p.server.repository_path()))?.set_head(CONTEXT))?;
    let result = p.run(&p.a, &request)?;
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RemoteDeletionFailed(
                PromotionRemoteDeletionCategory::Rejected
            )
        }
    );
    let before = image(&p.a.worktree())?;
    assert_eq!(server_oid(&p, CONTEXT)?, edited);
    assert_eq!(server_oid(&p, MAIN)?, merged(&result)?);
    p.privacy()?;
    p.server.reject_context_deletion(false)?;
    assert_eq!(image(&p.a.worktree())?, before);
    let retried = p.run(&p.a, &request)?;
    assert_eq!(retried.cleanup(), PromotionCleanup::Complete);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    assert_eq!(receives_accepted(&p, cursor)?, [1, 1, 1]);
    cleaned(&p, &p.a)?;
    p.privacy()
}

fn deletion_race() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let old = p.edit(&p.a, BODY)?;
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    let peer = commit_files(&linked(&p.b)?, &[("peer-race", b"peer\n")])?;
    // The peer object reaches the server without moving the context; the
    // fixture's advertise/receive race then moves it.
    let b = p.b.repo()?;
    fixed(b.reference("refs/heads/fixture-peer", peer, true, "fixture"))?;
    remote_world::push_peer(&b, &p.server, "refs/heads/fixture-peer")?;
    let request = p.request(&p.a)?;
    stopped(&p, &p.a, &request, FailurePoint::AfterPublication)?;
    assert_eq!(step(&p.a, request.operation_id)?, "published");
    p.server
        .race_context_update(ssh_remote::ObservationRef::RemoteTicket, old, peer)?;
    let before = image(&p.a.worktree())?;
    let result = p.run(&p.a, &request)?;
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RemoteContextChanged
        }
    );
    assert_eq!(server_oid(&p, CONTEXT)?, peer);
    assert_eq!(image(&p.a.worktree())?, before);
    assert_eq!(oid(&p.a, TRACKING)?, peer);
    p.privacy()
}

fn transport_attempt(
    p: &Pair,
    request: &CloseTicketRequest,
    primary: bool,
) -> Result<(String, i64), FixtureError> {
    let (id, attempt) = if primary {
        primary_child(&p.a, request.operation_id)?
    } else {
        context_child(&p.a, request.operation_id)?
    };
    Ok((id.ok_or(FixtureError)?, attempt))
}
fn offline(integrated: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let base = server_oid(&p, MAIN)?;
    let context = p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    if integrated {
        stopped(&p, &p.a, &request, FailurePoint::AfterIntegrationRef)?;
        assert_eq!(step(&p.a, request.operation_id)?, "integrating");
    }
    let primary = oid(&p.a, MAIN)?;
    let local_context = oid(&p.a, CONTEXT)?;
    let worktree = image(&p.a.worktree())?;
    let endpoint = p.server.url();
    let host = p.server.host_identity();
    let receipts = p.server.receive_updates();
    // A persistent real disconnect at this same endpoint; clearing it
    // restores the same listener, repository and host key.
    p.server
        .disconnect_at(ssh_remote::FixtureBoundary::Advertisement);
    let result = p.execute(&p.a.service, request.clone());
    if integrated {
        let result = authoritative(fixed(result)?);
        assert_eq!(
            result.publication(),
            PromotionPublication::Pending {
                reason: PromotionPublicationPending::Synchronization(
                    PromotionSynchronizationCategory::Transport
                ),
            }
        );
        assert_eq!(
            result.cleanup(),
            PromotionCleanup::Pending {
                reason: PromotionCleanupPending::AwaitingPublication,
            }
        );
        assert_eq!(server_oid(&p, CONTEXT)?, context);
    } else {
        assert!(
            matches!(result, Err(TicketClosureError::ContextSynchronization(ref e))
            if matches!(**e, SynchronizationError::Transport(_)))
        );
        assert!(server_oid(&p, CONTEXT).is_err());
        assert_eq!(commits(&p.a, &close_subject())?, 0);
    }
    let (old_child, old_attempt) = transport_attempt(&p, &request, integrated)?;
    assert_eq!(old_attempt, 1);
    assert_eq!(server_oid(&p, MAIN)?, base);
    assert_eq!(oid(&p.a, MAIN)?, primary);
    assert_eq!(oid(&p.a, CONTEXT)?, local_context);
    assert_eq!(image(&p.a.worktree())?, worktree);
    assert_eq!(commits(&p.a, &merge_subject())?, usize::from(integrated));
    p.server.wait_for_helper_audits()?;
    assert_eq!(p.server.receive_updates(), receipts);
    p.privacy()?;

    p.server.clear_fault();
    assert!(p.server.url() == endpoint && p.server.host_identity() == host);
    // The identical request resumes only unfinished work.
    let outcome = p.run(&p.a, &request)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    let (new_child, new_attempt) = transport_attempt(&p, &request, integrated)?;
    assert_ne!(new_child, old_child);
    assert_eq!(new_attempt, 2);
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    assert_eq!(server_oid(&p, MAIN)?, oid(&p.a, MAIN)?);
    cleaned(&p, &p.a)?;
    assert_eq!(receives(&p, cursor)?, [1, 1, 1]);
    context_never_held(&p, cursor, checkpoint_of(&outcome)?)?;
    p.privacy()
}

fn preservation(mode: usize) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    stopped(&p, &p.a, &request, FailurePoint::AfterPublication)?;
    assert_eq!(step(&p.a, request.operation_id)?, "published");
    let remote = server_oid(&p, CONTEXT)?;
    let ticket = p.a.worktree().join(TICKET_PATH);
    let closed = fixed(std::fs::read(&ticket))?;
    let moved = match mode {
        0 => {
            let mut bytes = closed.clone();
            bytes.extend_from_slice(b"user edit\n");
            fixed(std::fs::write(&ticket, bytes))?;
            None
        }
        1 => {
            fixed(std::fs::write(p.a.worktree().join("u"), b"user\n"))?;
            None
        }
        2 => {
            let repo = p.a.repo()?;
            let path = repo.commondir().join("info/exclude");
            let mut bytes = fixed(std::fs::read(&path))?;
            bytes.extend_from_slice(b"\nprivate-ignored\n");
            fixed(std::fs::write(path, bytes))?;
            fixed(std::fs::write(
                p.a.worktree().join("private-ignored"),
                b"user\n",
            ))?;
            None
        }
        3 => Some(commit_files(
            &linked(&p.a)?,
            &[("moved.txt", b"user commit\n")],
        )?),
        _ => return Err(FixtureError),
    };
    let context = oid(&p.a, CONTEXT)?;
    let before = image(&p.a.worktree())?;
    let administration = image(linked(&p.a)?.path())?;
    let outcome = p.run(&p.a, &request)?;
    assert_eq!(
        outcome.cleanup(),
        PromotionCleanup::Pending {
            reason: if mode == 2 {
                PromotionCleanupPending::IgnoredFiles
            } else {
                PromotionCleanupPending::ContextChanged
            }
        }
    );
    assert_eq!(server_oid(&p, CONTEXT)?, remote);
    assert_eq!(image(&p.a.worktree())?, before);
    assert_eq!(image(linked(&p.a)?.path())?, administration);
    assert_eq!(oid(&p.a, CONTEXT)?, context);
    p.privacy()?;
    if let Some(moved) = moved {
        // Further work over a closed ticket is never merged: the context is
        // kept and the next preview reports the closure.
        assert_eq!(context, moved);
        poll(&p, &p.a)?;
        assert_eq!(p.blockers(&p.a)?, [ClosureBlocker::AlreadyClosed]);
        assert_eq!(oid(&p.a, CONTEXT)?, moved);
        return p.privacy();
    }
    match mode {
        0 => fixed(std::fs::write(&ticket, &closed))?,
        1 => fixed(std::fs::remove_file(p.a.worktree().join("u")))?,
        2 => fixed(std::fs::remove_file(p.a.worktree().join("private-ignored")))?,
        _ => unreachable!(),
    }
    // A new confirmation observes the published primary; only the cleanup
    // remains, and the stopped record is superseded.
    poll(&p, &p.a)?;
    let next = p.request(&p.a)?;
    assert_ne!(next.operation_id, request.operation_id);
    let finished = p.run(&p.a, &next)?;
    assert_eq!(finished.cleanup(), PromotionCleanup::Complete);
    assert_eq!(finished.closure(), ClosureRecording::Existing);
    assert!(matches!(
        finished.integration(),
        PromotionIntegration::AlreadyContained { .. }
    ));
    assert_eq!(step(&p.a, request.operation_id)?, "superseded");
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    assert_eq!(commits(&p.a, &merge_subject())?, 1);
    cleaned(&p, &p.a)?;
    p.privacy()
}

fn priority() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    fixed(p.a.service.set_remote_polling(
        &p.a.root,
        true,
        false,
        PollingInterval::from_seconds(120).unwrap(),
    ))?;
    let mut session = SessionCredentials::new(remote_world::Provider);
    fixed(p.a.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: p.a.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(p.approval()),
        },
        &mut session,
    ))?;
    let second = fixed(RepositoryService::open_at(&p.a.data))?;
    let gate = p.server.hold_advertisement()?;
    let poll_request = PollPublicationRemoteRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
        invocation: RemotePollInvocation::Automatic,
        approval: Some(p.approval()),
        restart: false,
    };
    let cursor = p.server.receive_updates().len();
    let primary = oid(&p.a, MAIN)?;
    std::thread::scope(|scope| -> Result<(), FixtureError> {
        let worker = scope.spawn(|| {
            p.a.service
                .poll_publication_remote(poll_request, &mut session)
        });
        gate.wait_until_held()?;
        let result = p.execute(&second, request.clone());
        assert!(
            matches!(result,Err(TicketClosureError::ContextSynchronization(ref e)) if matches!(**e,SynchronizationError::PollYielding))
        );
        assert_eq!(oid(&p.a, MAIN)?, primary);
        gate.release()?;
        let result = fixed(worker.join())?;
        p.diagnostics(&format!("{result:?}"))?;
        assert!(
            matches!(result, Err(RemotePollError::Interrupted))
                || matches!(result,Ok(RemotePollResult::Complete(RemotePollOutcome::Polled(ref r))) if r.completion()==PollCompletion::Interrupted)
        );
        Ok(())
    })?;
    assert_eq!(commits(&p.a, &close_subject())?, 0);
    let outcome = p.run(&p.a, &request)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    assert_eq!(receives_accepted(&p, cursor)?, [1, 1, 1]);
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    p.privacy()
}

fn comment(p: &Pair, side: &Side) -> Result<CommentSubmissionOutcome, FixtureError> {
    let id: canonical::ItemId = "01CRZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    let result = side.service.submit_comment(
        PublishCommentRequest {
            comment: SubmitCommentRequest {
                target: side.target(ContextIntent::Edit),
                comment_id: id,
                parent_id: None,
                body: COMMENT.into(),
                expected_destination: ExpectedPathObservation::Missing,
            },
            approval: Some(p.approval()),
            confirmed_identity: None,
        },
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{:?}", result.as_ref().map(|_| ())))?;
    fixed(result)
}
/// A comment checkpointed on the ticket whose publication stays pending.
fn pending_comment(p: &Pair) -> Result<OperationId, FixtureError> {
    p.server
        .disconnect_at(ssh_remote::FixtureBoundary::Advertisement);
    let outcome = comment(p, &p.a)?;
    p.server.clear_fault();
    let CommentSubmissionOutcome::Saved {
        receipt,
        publication,
        ..
    } = outcome
    else {
        return Err(FixtureError);
    };
    p.diagnostics(&format!("{receipt:?} {publication:?}"))?;
    assert!(matches!(
        publication,
        CommentPublicationState::Pending { .. }
    ));
    Ok(receipt.operation_id)
}
fn comment_pending() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    let comment = pending_comment(&p)?;
    let path = PathBuf::from(format!(
        ".manyhands/comments/{TICKET}/01CRZ3NDEKTSV4RRFFQ69G5FAV.md"
    ));
    assert!(
        p.blockers(&p.a)?
            .contains(&ClosureBlocker::CommentPublicationPending)
    );
    let refused = zero_request(&p, &p.a, item());
    assert!(matches!(
        p.execute(&p.a.service, refused),
        Err(TicketClosureError::Blocked(ref b)) if b.contains(&ClosureBlocker::CommentPublicationPending)
    ));
    assert_eq!(commits(&p.a, &close_subject())?, 0);
    // The comment's own publication finishes; closure is then ready.
    // The failed child is restarted explicitly by a reopened service, as the
    // comment publication contract requires (Cycle 08 interrupted_restart).
    let reopened = fixed(RepositoryService::open_at(&p.a.data))?;
    let retried = reopened.retry_comment_publication(
        RetryCommentPublicationRequest {
            root: p.a.root.clone(),
            operation_id: comment,
            approval: Some(p.approval()),
            confirmed_identity: None,
            restart: true,
        },
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{:?}", retried.as_ref().map(|_| ())))?;
    let CommentSubmissionOutcome::Saved { publication, .. } = fixed(retried)? else {
        return Err(FixtureError);
    };
    assert!(matches!(
        publication,
        CommentPublicationState::Published { .. } | CommentPublicationState::AlreadyCurrent { .. }
    ));
    let preview = p.preview(&p.a)?;
    assert!(preview.changes().iter().any(|c| c.path() == path));
    let outcome = p.close(&p.a)?;
    let merged = merged(&outcome)?;
    assert!(
        blob(&p.a.repo()?, merged, path.to_str().ok_or(FixtureError)?)?
            .windows(COMMENT.len())
            .any(|w| w == COMMENT.as_bytes())
    );
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    p.privacy()
}

fn refused_closed<T>(result: Result<T, RepositoryError>) -> Result<(), FixtureError> {
    match result {
        Err(error) => {
            assert_eq!(error.kind, RepositoryErrorKind::TicketClosed);
            Ok(())
        }
        Ok(_) => Err(FixtureError),
    }
}
fn guard() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let context = p.edit(&p.a, BODY)?;
    let request = p.request(&p.a)?;
    stopped(
        &p,
        &p.a,
        &request,
        FailurePoint::ClosureAfterClosingRecorded,
    )?;
    assert_eq!(step(&p.a, request.operation_id)?, "closing");
    assert_eq!(oid(&p.a, CONTEXT)?, context);
    // The same ID from `closing` writes its own checkpoint despite the guard.
    stopped(&p, &p.a, &request, FailurePoint::AfterClosureCheckpoint)?;
    assert_eq!(step(&p.a, request.operation_id)?, "closing");
    let (closed_at, closure) = stored_closure(&p.a, request.operation_id)?;
    let checkpoint = oid(&p.a, CONTEXT)?;
    assert_eq!(closure, Some(checkpoint.to_string()));
    // The context copy now reads closed: a product save there is refused
    // and changes nothing.
    let worktree = image(&p.a.worktree())?;
    let closed = fixed(std::fs::read(p.a.worktree().join(TICKET_PATH)))?;
    refused_closed(p.a.service.save_ticket(SaveTicketRequest {
        target: p.a.target(ContextIntent::Edit),
        draft: draft("refused edit\n"),
        expected_path: ExpectedPathObservation::from_bytes(&closed),
    }))?;
    assert_eq!(oid(&p.a, CONTEXT)?, checkpoint);
    assert_eq!(image(&p.a.worktree())?, worktree);
    let outcome = p.run(&p.a, &request)?;
    assert_eq!(outcome.cleanup(), PromotionCleanup::Complete);
    assert_eq!(
        outcome.closure(),
        ClosureRecording::Recorded {
            checkpoint_oid: checkpoint
        }
    );
    assert_eq!(
        stored_closure(&p.a, request.operation_id)?,
        (closed_at, Some(checkpoint.to_string()))
    );
    assert_eq!(commits(&p.a, &close_subject())?, 1);
    // After the closure: a new save, context and comment are refused, and
    // nothing is provisioned.
    let refs = fixed(p.a.repo()?.references())?.count();
    let primary = fixed(std::fs::read(p.a.root.join(TICKET_PATH)))?;
    refused_closed(p.a.service.save_ticket(SaveTicketRequest {
        target: p.a.target(ContextIntent::Edit),
        draft: draft("refused edit\n"),
        expected_path: ExpectedPathObservation::from_bytes(&primary),
    }))?;
    refused_closed(p.a.service.prepare_context(p.a.target(ContextIntent::Edit)))?;
    let submitted = p.a.service.submit_comment(
        PublishCommentRequest {
            comment: SubmitCommentRequest {
                target: p.a.target(ContextIntent::Edit),
                comment_id: canonical::ItemId::generate(),
                parent_id: None,
                body: COMMENT.into(),
                expected_destination: ExpectedPathObservation::Missing,
            },
            approval: Some(p.approval()),
            confirmed_identity: None,
        },
        &mut SessionCredentials::new(remote_world::Provider),
    );
    assert!(matches!(submitted, Err(ref e) if e.kind == RepositoryErrorKind::TicketClosed));
    assert!(!p.a.worktree().exists());
    assert!(p.a.repo()?.find_reference(CONTEXT).is_err());
    assert_eq!(fixed(p.a.repo()?.references())?.count(), refs);
    assert_eq!(fixed(std::fs::read(p.a.root.join(TICKET_PATH)))?, primary);
    p.privacy()
}

fn discoverable() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.edit(&p.a, BODY)?;
    let outcome = p.close(&p.a)?;
    let merged = merged(&outcome)?;
    poll(&p, &p.b)?;
    poll(&p, &p.a)?;
    for side in [&p.a, &p.b] {
        assert_eq!(oid(side, MAIN)?, merged);
        let repository = fixed(side.service.resolve_repository(&side.root))?;
        let listed = |closure| -> Result<Vec<ItemDto>, FixtureError> {
            let list = fixed(side.service.list_tickets(
                &repository,
                &TicketFilter {
                    closure,
                    ..TicketFilter::default()
                },
            ))?;
            Ok(list
                .items
                .into_iter()
                .filter(|i| i.id.as_deref() == Some(TICKET))
                .collect())
        };
        let closed = listed(ClosureFilter::Closed)?;
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].status.as_deref(), Some("open"));
        assert_eq!(closed[0].context.kind, ItemContextKind::Primary);
        let closure = closed[0].closure.clone().ok_or(FixtureError)?;
        assert_eq!(closure.state, ClosureState::Closed);
        assert_eq!(closure.closed_by.as_deref(), Some(p.a.closed_by().as_str()));
        assert!(listed(ClosureFilter::Open)?.is_empty());
        assert_eq!(listed(ClosureFilter::All)?.len(), 1);
        let shown = fixed(side.service.show_item(&repository, &item()))?;
        assert_eq!(shown.status.as_deref(), Some("open"));
        assert_eq!(shown.closure, Some(closure));
    }
    p.privacy()
}

fn blocker(mode: usize) -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    if !matches!(mode, 0 | 1) {
        p.edit(&p.a, BODY)?;
    }
    let ready = !matches!(mode, 0 | 1 | 17 | 18);
    let mut request = if ready {
        p.request(&p.a)?
    } else {
        zero_request(&p, &p.a, item())
    };
    let repo = p.a.repo()?;
    let expected = match mode {
        0 => {
            let document = fixed(canonical::serialize_item(
                &canonical::CanonicalItem::Document(canonical::Document {
                    id: DOC.parse().unwrap(),
                    title: "Shared document".into(),
                    body: "Seed\n".into(),
                    unknown: serde_yaml::Mapping::new(),
                }),
            ))?;
            commit_files(&repo, &[("docs/a.md", document.as_bytes())])?;
            fixed(p.sync_target(&p.a, SynchronizationTarget::Primary))?;
            // Publication does not advance T(P): an ordinary poll observes it.
            poll(&p, &p.a)?;
            request = zero_request(&p, &p.a, DOC.parse().unwrap());
            ClosureBlocker::NotATicket
        }
        1 => ClosureBlocker::ContextMissing,
        2 => {
            commit_removal(&linked(&p.a)?, TICKET_PATH)?;
            ClosureBlocker::ContextNonconforming
        }
        3 => {
            fixed(std::fs::write(p.a.worktree().join("draft.txt"), b"draft\n"))?;
            ClosureBlocker::ContextDirty
        }
        4 => {
            fixed(std::fs::write(
                linked(&p.a)?.path().join("MERGE_HEAD"),
                format!("{}\n", oid(&p.a, MAIN)?),
            ))?;
            ClosureBlocker::ContextConflicted
        }
        5 => {
            fixed(p.sync(&p.a))?;
            poll(&p, &p.b)?;
            p.edit(&p.a, &(BODY.to_owned() + "left\n"))?;
            p.edit(&p.b, &(BODY.to_owned() + "right\n"))?;
            fixed(p.sync(&p.b))?;
            request = p.request(&p.a)?;
            assert!(matches!(
                p.sync(&p.a),
                Err(SynchronizationError::ConflictPending { .. })
            ));
            ClosureBlocker::ConflictPending
        }
        6 => {
            let failing = fixed(RepositoryService::open_at_with_failure_point_for_testing(
                &p.a.data,
                FailurePoint::BeforeCheckpointCommit,
            ))?;
            let current = fixed(std::fs::read(p.a.worktree().join(TICKET_PATH)))?;
            let interrupted = failing.save_ticket(SaveTicketRequest {
                target: p.a.target(ContextIntent::Edit),
                draft: draft(&(BODY.to_owned() + "interrupted\n")),
                expected_path: ExpectedPathObservation::from_bytes(&current),
            });
            assert!(interrupted.is_err());
            ClosureBlocker::LocalOperationPending
        }
        7 => {
            pending_comment(&p)?;
            ClosureBlocker::CommentPublicationPending
        }
        8 => {
            fixed(fixed(repo.find_reference(TRACKING_MAIN))?.delete())?;
            ClosureBlocker::PrimaryMissing
        }
        9 => {
            fixed(repo.reference("refs/heads/other", oid(&p.a, MAIN)?, false, "fixture"))?;
            fixed(repo.set_head("refs/heads/other"))?;
            ClosureBlocker::PrimaryNotCheckedOut
        }
        10 => {
            fixed(std::fs::write(p.a.root.join("fixture.txt"), b"dirty\n"))?;
            ClosureBlocker::PrimaryDirty
        }
        11 => {
            let mut index = fixed(repo.index())?;
            let mut entry = index
                .get_path(Path::new("fixture.txt"), 0)
                .ok_or(FixtureError)?;
            fixed(index.remove(Path::new("fixture.txt"), 0))?;
            for stage in 1..=3 {
                entry.flags = (entry.flags & 0x0fff) | (stage << 12);
                fixed(index.add(&entry))?;
            }
            fixed(index.write())?;
            assert!(index.has_conflicts());
            ClosureBlocker::PrimaryConflicted
        }
        12 => {
            commit_files(&repo, &[("ahead", b"ahead\n")])?;
            ClosureBlocker::PrimaryAheadOfRemote
        }
        13 => {
            fixed(p.sync(&p.a))?;
            poll(&p, &p.b)?;
            commit_files(&linked(&p.b)?, &[("peer", b"peer\n")])?;
            fixed(p.sync(&p.b))?;
            peer_fetch(&p, &p.a, CONTEXT, TRACKING)?;
            ClosureBlocker::ContextBehindRemote
        }
        14 => {
            commit_files(
                &p.b.repo()?,
                &[(
                    TICKET_PATH,
                    ticket_source("conflicting primary body\n", None).as_bytes(),
                )],
            )?;
            fixed(p.sync_target(&p.b, SynchronizationTarget::Primary))?;
            poll(&p, &p.a)?;
            ClosureBlocker::IntegrationConflict
        }
        15 => {
            // An already-current second synchronization fetches T(C).
            fixed(p.sync(&p.a))?;
            fixed(p.sync(&p.a))?;
            poll(&p, &p.b)?;
            peer_delete(&p)?;
            poll(&p, &p.a)?;
            // D5 is conditional on an absent T(C); polls keep tracking and
            // the user's ordinary prune removes it.
            fixed(fixed(repo.find_reference(TRACKING))?.delete())?;
            ClosureBlocker::RemoteContextDeleted
        }
        16 => {
            lose_registry(&p.server, &mut p.a)?;
            ClosureBlocker::HistoryUnknown
        }
        17 => {
            p.close(&p.a)?;
            poll(&p, &p.a)?;
            request = zero_request(&p, &p.a, item());
            ClosureBlocker::AlreadyClosed
        }
        18 => {
            // A closed context merged by hand, then its closure removed on
            // the primary by hand.
            let by = p.a.closed_by();
            let closed = commit_files(
                &linked(&p.a)?,
                &[(
                    TICKET_PATH,
                    ticket_source(BODY, Some((1_800_000_000, &by))).as_bytes(),
                )],
            )?;
            fixed(repo.reference(MAIN, closed, true, "fixture hand merge"))?;
            fixed(repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())))?;
            commit_files(
                &repo,
                &[(TICKET_PATH, ticket_source(BODY, None).as_bytes())],
            )?;
            fixed(p.sync_target(&p.a, SynchronizationTarget::Primary))?;
            // Publication does not advance T(P): an ordinary poll observes it.
            poll(&p, &p.a)?;
            ClosureBlocker::PrimaryReopened
        }
        19 => {
            commit_files(
                &linked(&p.a)?,
                &[(".manyhands/worktrees/stray", b"stray\n")],
            )?;
            ClosureBlocker::TicketInvalidAfterClosure
        }
        20 => {
            let mut config = fixed(repo.config())?;
            fixed(config.remove("user.name"))?;
            fixed(config.remove("user.email"))?;
            ClosureBlocker::IdentityRequired
        }
        _ => return Err(FixtureError),
    };
    p.server.wait_for_helper_audits()?;
    let before = all_images(&p)?;
    let connections = p.server.accepted_sockets();
    let preflight = preflight_of(&p, &p.a, request.item_id.clone())?;
    let TicketClosurePreflight::Blocked {
        blockers,
        identity_configuration,
    } = preflight
    else {
        return Err(FixtureError);
    };
    for b in &blockers {
        ssh_harness::observation(&[800, *b as u128]);
    }
    // The exact set: the two modes whose fixture really adds a second fact
    // report it; every other mode reports its one blocker only.
    let exact = match mode {
        5 => vec![
            ClosureBlocker::ContextConflicted,
            ClosureBlocker::ConflictPending,
            ClosureBlocker::ContextBehindRemote,
        ],
        6 => vec![
            ClosureBlocker::ContextDirty,
            ClosureBlocker::LocalOperationPending,
        ],
        _ => vec![expected],
    };
    assert_eq!(blockers, exact);
    let sorted = {
        let mut sorted = blockers.clone();
        sorted.sort();
        sorted
    };
    assert_eq!(sorted, blockers);
    if expected == ClosureBlocker::IdentityRequired {
        assert_eq!(blockers, [ClosureBlocker::IdentityRequired]);
        assert!(identity_configuration.is_some());
        // The identity went away after the preview: execution's consent check
        // (C2) refuses it as a change, never as `Blocked([IdentityRequired])`.
        assert!(matches!(
            p.execute(&p.a.service, request.clone()),
            Err(TicketClosureError::ExternalChange)
        ));
    } else {
        assert!(identity_configuration.is_none());
        assert!(matches!(
            p.execute(&p.a.service, request.clone()),
            Err(TicketClosureError::Blocked(ref b)) if *b == exact
        ));
    }
    assert_eq!(step(&p.a, request.operation_id)?, "abandoned");
    assert_eq!(p.server.accepted_sockets(), connections);
    assert_eq!(all_images(&p)?, before);
    p.privacy()
}
