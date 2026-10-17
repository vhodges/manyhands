//! Real Git/SSH journeys; only the public library and owned fixture are used.
use super::*;
#[path = "advanced.rs"]
mod advanced;
#[path = "exits.rs"]
mod exits;
#[path = "quality.rs"]
mod quality;

pub(super) const CASES: &[ssh_harness::Case] = &[
    (
        "quality2_image_name_payload_boundaries",
        quality::image_boundaries,
    ),
    #[cfg(unix)]
    ("quality2_image_type_and_git_mode", quality::image_types),
    ("quality2_f11_fences_older_refresh", || {
        quality::refresh_f11(false)
    }),
    ("quality2_f11_preserves_active_child_handoff", || {
        quality::refresh_f11(true)
    }),
    (
        "quality2_f11_does_not_adopt_reclaimed_owner",
        quality::refresh_takeover,
    ),
    ("quality2_r1_distinct_reconcile_verified", || {
        quality::verified_routes(0)
    }),
    ("quality2_r1_distinct_prepare_already_current", || {
        quality::verified_routes(1)
    }),
    ("quality2_r1_distinct_settlement_verified", || {
        quality::verified_routes(2)
    }),
    (
        "quality2_publication_index_additive_audited",
        quality::projection_index,
    ),
    (
        "quality2_late_classification_keeps_new_retirement",
        quality::late_classification,
    ),
    ("happy_comment_history_and_collaborator_poll", happy),
    ("local_only_then_primary_publication", local_only),
    ("contained_and_fresh_second_promotion", fresh_second),
    ("blocker_dirty_primary", || blocker(0)),
    ("blocker_conflicted_primary", || blocker(1)),
    ("blocker_primary_not_checked_out", || blocker(2)),
    ("blocker_primary_ahead", || blocker(3)),
    ("blocker_dirty_context", || blocker(4)),
    ("blocker_context_behind_fetched", || blocker(5)),
    ("blocker_remote_deleted", || blocker(6)),
    ("preserve_edited_context_then_supersede", || preservation(0)),
    ("preserve_untracked_then_supersede", || preservation(1)),
    ("preserve_ignored_then_supersede", || preservation(2)),
    ("preserve_locked_then_supersede", || preservation(3)),
    ("retained_r1_unknown_history_and_consumption", retained_r1),
    ("r1_verified_push_consumes_before_authority", || {
        advanced::retired_push_guard(true)
    }),
    ("r1_new_ambiguous_push_is_not_old_retired_history", || {
        advanced::retired_push_guard(false)
    }),
    ("new_context_content_reconfirms", || changed_content(false)),
    ("primary_only_advance_preserves_consent", || {
        changed_content(true)
    }),
    ("real_context_conflict_public_resolution", conflict),
    ("primary_rejection_retry_only_unfinished", primary_rejection),
    (
        "offline_before_context_is_preintegration_error",
        offline_before,
    ),
    (
        "offline_after_integration_is_publication_pending",
        offline_after,
    ),
    (
        "delete_rejection_retry_preserves_local_state",
        deletion_rejection,
    ),
    (
        "delete_context_race_preserves_peer_and_tracking",
        deletion_race,
    ),
    (
        "poll_f2_primary_advanced_retry_integrates_once",
        integration_base,
    ),
    ("cancelled_context_attempt_replaced_once", || {
        advanced::cancelled(false)
    }),
    ("cancelled_primary_attempt_replaced_once", || {
        advanced::cancelled(true)
    }),
    ("interrupted_retained_context_keeps_child_id", || {
        advanced::retained(false)
    }),
    ("interrupted_retained_primary_keeps_child_id", || {
        advanced::retained(true)
    }),
    (
        "automatic_poll_advertisement_yields_to_promotion",
        advanced::priority,
    ),
    (
        "poll_between_publication_and_f6",
        advanced::between_publication_and_f6,
    ),
    ("poll_busy_at_f6", || {
        advanced::busy(FailurePoint::BeforeRemoteDeletion)
    }),
    ("poll_busy_at_f7", || {
        advanced::busy(FailurePoint::AfterRemoteDeletion)
    }),
    ("poll_busy_at_f8", || {
        advanced::busy(FailurePoint::DuringWorktreeRemoval)
    }),
    ("poll_busy_at_f9", || {
        advanced::busy(FailurePoint::BeforeBranchDeletion)
    }),
    ("poll_busy_at_f10", || {
        advanced::busy(FailurePoint::BeforeCompletion)
    }),
    ("remote_rename_before_integration_fences", || {
        advanced::configuration(false)
    }),
    ("remote_rename_after_integration_is_pending", || {
        advanced::configuration(true)
    }),
    ("exit_f1_confirmed", || exits::boundary(0)),
    ("exit_f2_context_synchronized", || exits::boundary(1)),
    ("exit_f3_before_integration_ref", || exits::boundary(2)),
    ("exit_f4_after_integration_ref", || exits::boundary(3)),
    ("exit_f5_after_publication", || exits::boundary(4)),
    ("exit_f6_before_remote_deletion", || exits::boundary(5)),
    ("exit_f7_after_remote_deletion", || exits::boundary(6)),
    ("exit_f8_worktree_removing", || exits::boundary(7)),
    ("exit_f9_before_branch_deletion", || exits::boundary(8)),
    ("exit_f10_before_completion", || exits::boundary(9)),
    ("exit_f11_after_completion", || exits::boundary(10)),
    ("exit_f12_before_refresh", || exits::boundary(11)),
    ("exit_inside_f10_git_sql_split_operator_repair", || {
        exits::boundary(12)
    }),
    ("exit_after_sql_commit_replay_preserves_lock", || {
        exits::boundary(13)
    }),
    (
        "f7_cache_loss_new_confirmed_cleanup_earns_retirement",
        exits::f7_loss,
    ),
    ("completion_statement_rollback_preserves_history", || {
        exits::sql_failure(false)
    }),
    ("completion_commit_rollback_preserves_history", || {
        exits::sql_failure(true)
    }),
    (
        "completion_unknown_commit_result_reconciles",
        exits::uncertain_commit,
    ),
];
pub(super) fn maybe_exit_child() -> bool {
    exits::maybe_child()
}

/// All directory entries and file bytes, including indexes, refs, administration,
/// ignored/untracked files and registrations. Never follows symbolic links.
pub(super) fn image(root: &Path) -> Result<[u8; 32], FixtureError> {
    fn frame(hash: &mut blake3::Hasher, bytes: &[u8]) -> Result<(), FixtureError> {
        hash.update(&fixed(u64::try_from(bytes.len()))?.to_be_bytes());
        hash.update(bytes);
        Ok(())
    }
    fn metadata(hash: &mut blake3::Hasher, meta: &std::fs::Metadata) -> Result<u8, FixtureError> {
        let (kind, base_mode) = if meta.file_type().is_symlink() {
            (b'l', 0o120000u32)
        } else if meta.is_dir() {
            (b'd', 0o040000)
        } else if meta.is_file() {
            (b'f', 0o100644)
        } else {
            // Never open a FIFO/device/socket as an ordinary file payload.
            return Err(FixtureError);
        };
        #[cfg(unix)]
        let git_mode = {
            use std::os::unix::fs::PermissionsExt;
            let permissions = meta.permissions().mode();
            hash.update(&(permissions & 0o7777).to_be_bytes());
            if kind == b'f' && permissions & 0o100 != 0 {
                0o100755
            } else {
                base_mode
            }
        };
        #[cfg(not(unix))]
        let git_mode = base_mode;
        hash.update(&[kind]);
        hash.update(&git_mode.to_be_bytes());
        hash.update(&[u8::from(meta.permissions().readonly())]);
        Ok(kind)
    }
    fn add(root: &Path, hash: &mut blake3::Hasher) -> Result<(), FixtureError> {
        let mut paths = fixed(std::fs::read_dir(root))?
            .map(|e| fixed(e).map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        hash.update(&fixed(u64::try_from(paths.len()))?.to_be_bytes());
        for path in paths {
            frame(
                hash,
                path.file_name().ok_or(FixtureError)?.as_encoded_bytes(),
            )?;
            let meta = fixed(std::fs::symlink_metadata(&path))?;
            match metadata(hash, &meta)? {
                b'l' => frame(
                    hash,
                    fixed(std::fs::read_link(path))?
                        .as_os_str()
                        .as_encoded_bytes(),
                )?,
                b'd' => add(&path, hash)?,
                b'f' => frame(hash, &fixed(std::fs::read(path))?)?,
                _ => unreachable!(),
            }
        }
        Ok(())
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands fixture image v2");
    if metadata(&mut hash, &fixed(std::fs::symlink_metadata(root))?)? != b'd' {
        return Err(FixtureError);
    }
    add(root, &mut hash)?;
    Ok(*hash.finalize().as_bytes())
}

fn oid(side: &Side, reference: &str) -> Result<git2::Oid, FixtureError> {
    fixed(side.repo()?.refname_to_id(reference))
}
fn server_oid(p: &Pair, reference: &str) -> Result<git2::Oid, FixtureError> {
    fixed(fixed(git2::Repository::open_bare(p.server.repository_path()))?.refname_to_id(reference))
}
fn authoritative(result: DocumentPromotionResult) -> DocumentPromotionOutcome {
    match result {
        DocumentPromotionResult::Complete(o) => o,
        DocumentPromotionResult::IndexPending(p) => p.authoritative,
    }
}
fn run(
    p: &Pair,
    request: &PromoteDocumentRequest,
) -> Result<DocumentPromotionOutcome, FixtureError> {
    Ok(authoritative(fixed(
        p.execute(&p.a.service, request.clone()),
    )?))
}
fn stopped(
    p: &Pair,
    request: &PromoteDocumentRequest,
    point: FailurePoint,
) -> Result<(), FixtureError> {
    let service = fixed(RepositoryService::open_at_with_failure_point_for_testing(
        &p.a.data, point,
    ))?;
    let result = p.execute(&service, request.clone());
    assert!(
        result.is_err()
            || matches!(result, Ok(ref r) if authoritative(r.clone()).cleanup() != PromotionCleanup::Complete)
    );
    Ok(())
}
fn count(side: &Side, table: &str) -> Result<i64, FixtureError> {
    fixed(
        side.db()?
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)),
    )
}
fn step(side: &Side, id: OperationId) -> Result<String, FixtureError> {
    fixed(side.db()?.query_row(
        "SELECT step FROM document_promotions WHERE operation_id=?1",
        [id.to_string()],
        |r| r.get(0),
    ))
}
fn promotion_merges(side: &Side) -> Result<usize, FixtureError> {
    let repo = side.repo()?;
    let odb = fixed(repo.odb())?;
    let mut n = 0;
    fixed(odb.foreach(|id| {
        if repo
            .find_commit(*id)
            .is_ok_and(|c| c.summary() == Some(format!("Promote document {DOC}").as_str()))
        {
            n += 1;
        }
        true
    }))?;
    Ok(n)
}
pub(super) fn poll(p: &Pair, side: &Side) -> Result<RemotePollReport, FixtureError> {
    let result = side.service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: side.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            approval: Some(p.approval()),
            restart: false,
        },
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{result:?}"))?;
    let result = match fixed(result)? {
        RemotePollResult::Complete(r) => r,
        RemotePollResult::IndexPending(i) => i.authoritative,
    };
    match result {
        RemotePollOutcome::Polled(r) => Ok(r),
        _ => Err(FixtureError),
    }
}
fn linked(side: &Side) -> Result<git2::Repository, FixtureError> {
    fixed(git2::Repository::open(side.worktree()))
}
fn blob(repo: &git2::Repository, id: git2::Oid, path: &str) -> Result<Vec<u8>, FixtureError> {
    let tree = fixed(fixed(repo.find_commit(id))?.tree())?;
    Ok(
        fixed(repo.find_blob(fixed(tree.get_path(Path::new(path)))?.id()))?
            .content()
            .to_vec(),
    )
}
fn peer_fetch(
    p: &Pair,
    side: &Side,
    reference: &str,
    tracking: &str,
) -> Result<git2::Oid, FixtureError> {
    let repo = side.repo()?;
    let mut remote = fixed(repo.remote_anonymous(&p.server.url()))?;
    let mut options = git2::FetchOptions::new();
    options.remote_callbacks(remote_world::callbacks(&p.server));
    fixed(remote.fetch(
        &[&format!("+{reference}:{tracking}")],
        Some(&mut options),
        None,
    ))?;
    fixed(repo.refname_to_id(tracking))
}
fn peer_delete(p: &Pair) -> Result<(), FixtureError> {
    let old = server_oid(p, CONTEXT)?;
    let before = p.server.receive_updates().len();
    let repo = p.b.repo()?;
    let mut remote = fixed(repo.remote_anonymous(&p.server.url()))?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(remote_world::callbacks(&p.server));
    fixed(remote.push(&[&format!(":{CONTEXT}")], Some(&mut options)))?;
    p.server.fixture_push_transport_returned()?;
    p.server.wait_for_receive_update(
        before,
        &ssh_remote::ReceiveUpdate {
            reference: CONTEXT.into(),
            old_oid: old,
            new_oid: git2::Oid::zero(),
            accepted: true,
        },
    )
}
fn discovery_primary_once(p: &Pair) -> Result<(), FixtureError> {
    let snapshot = fixed(p.a.service.repository_snapshot(&p.a.root))?;
    assert!(!snapshot.refresh_required);
    let items: Vec<_> = snapshot
        .items
        .iter()
        .filter(|i| i.id.to_string() == DOC)
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(
        fixed(std::fs::canonicalize(&items[0].context))?,
        fixed(std::fs::canonicalize(&p.a.root))?
    );
    Ok(())
}
pub(super) fn comment(p: &Pair) -> Result<(git2::Oid, PathBuf), FixtureError> {
    let id: canonical::ItemId = "01CRZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
    let path = PathBuf::from(format!(".manyhands/comments/{DOC}/{id}.md"));
    let result = p.a.service.submit_comment(
        PublishCommentRequest {
            comment: SubmitCommentRequest {
                target: p.a.target(),
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
    match fixed(result)? {
        CommentSubmissionOutcome::Saved {
            receipt,
            publication,
            indexing,
            ..
        } => {
            p.diagnostics(&format!("{receipt:?} {publication:?} {indexing:?}"))?;
            assert!(matches!(
                publication,
                CommentPublicationState::Published { .. }
                    | CommentPublicationState::AlreadyCurrent { .. }
            ));
            Ok((receipt.checkpoint_oid, path))
        }
        _ => Err(FixtureError),
    }
}
fn happy() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let base = oid(&p.a, MAIN)?;
    let edited = p.save(&p.a)?;
    let (context, path) = comment(&p)?;
    poll(&p, &p.b)?;
    assert!(p.b.worktree().exists());
    let before = image(&p.b.worktree())?;
    let result = p.promote()?;
    let PromotionIntegration::Merged { oid: merged } = result.integration() else {
        return Err(FixtureError);
    };
    assert_eq!(result.cleanup(), PromotionCleanup::Complete);
    assert_eq!(server_oid(&p, MAIN)?, merged);
    let repo = p.a.repo()?;
    let merge = fixed(repo.find_commit(merged))?;
    assert_eq!(
        merge.summary(),
        Some(format!("Promote document {DOC}").as_str())
    );
    assert_eq!(merge.parent_ids().collect::<Vec<_>>(), [base, context]);
    assert!(
        blob(&repo, merged, DOCUMENT_PATH)?
            .windows(BODY.len())
            .any(|w| w == BODY.as_bytes())
    );
    assert!(
        blob(&repo, merged, path.to_str().ok_or(FixtureError)?)?
            .windows(COMMENT.len())
            .any(|w| w == COMMENT.as_bytes())
    );
    // Actual Git log traversal from the retained second parent, not a fabricated
    // history list: the edit and comment checkpoints remain reachable.
    let mut log = fixed(repo.revwalk())?;
    fixed(log.push(fixed(merge.parent(1))?.id()))?;
    let history = fixed(log.collect::<Result<Vec<_>, _>>())?;
    assert!(history.contains(&edited) && history.contains(&context));
    let log = fixed(
        std::process::Command::new("git")
            .arg("-C")
            .arg(&p.a.root)
            .args(["log", "--format=%H", &format!("{merged}^2")])
            .output(),
    )?;
    ssh_privacy::clean(&log.stdout, &p.probes)?;
    ssh_privacy::clean(&log.stderr, &p.probes)?;
    assert!(log.status.success());
    let history = fixed(String::from_utf8(log.stdout))?;
    assert!(
        history.lines().any(|s| s == edited.to_string())
            && history.lines().any(|s| s == context.to_string())
    );
    assert!(!p.a.worktree().exists() && repo.find_reference(CONTEXT).is_err());
    assert!(server_oid(&p, CONTEXT).is_err());
    discovery_primary_once(&p)?;
    let report = poll(&p, &p.b)?;
    assert_eq!(oid(&p.b, MAIN)?, merged);
    assert!(
        report
            .targets()
            .iter()
            .any(|r| r.item_id().is_some_and(|id| id.to_string() == DOC)
                && r.disposition() == PollDisposition::RemotelyDeleted)
    );
    assert_eq!(image(&p.b.worktree())?, before);
    let after = poll(&p, &p.a)?;
    assert!(
        !after
            .targets()
            .iter()
            .any(|r| r.item_id().is_some_and(|id| id.to_string() == DOC))
    );
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
    p.save(&p.a)?;
    let result = p.promote()?;
    assert_eq!(p.server.accepted_sockets(), connections);
    assert_eq!(
        result.publication(),
        PromotionPublication::Pending {
            reason: PromotionPublicationPending::NoPublicationRemote
        }
    );
    assert_eq!(result.cleanup(), PromotionCleanup::Complete);
    let merged = oid(&p.a, MAIN)?;
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
    // Publication selection deliberately preserves the live index (the existing
    // publication_repeated_changes_accept_the_expected_stale_live_index contract).
    // The user's Git tooling aligns only that committed config entry before sync.
    let repo = p.a.repo()?;
    let mut index = fixed(repo.index())?;
    fixed(index.add_path(Path::new(".manyhands/config.toml")))?;
    fixed(index.write())?;
    fixed(p.sync_target(&p.a, SynchronizationTarget::Primary))?;
    assert_eq!(server_oid(&p, MAIN)?, oid(&p.a, MAIN)?);
    assert!(
        fixed(p.a.repo()?.graph_descendant_of(oid(&p.a, MAIN)?, merged))?
            || oid(&p.a, MAIN)? == merged
    );
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}
fn fresh_second() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    p.promote()?;
    poll(&p, &p.a)?;
    let first = oid(&p.a, MAIN)?;
    fixed(p.a.service.prepare_context(p.a.target()))?;
    assert_eq!(oid(&p.a, CONTEXT)?, first);
    assert_eq!(
        p.preview()?.integration(),
        PlannedIntegration::AlreadyContained
    );
    let contained = p.promote()?;
    assert_eq!(
        contained.integration(),
        PromotionIntegration::AlreadyContained { oid: first }
    );
    assert_eq!(promotion_merges(&p.a)?, 1);
    let saved = p.save_body(&p.a, &(BODY.to_owned() + "second promotion\n"))?;
    assert_eq!(
        fixed(linked(&p.a)?.find_commit(saved))?.parent_id(0).ok(),
        Some(first)
    );
    assert_eq!(published(fixed(p.sync(&p.a))?)?, saved);
    p.promote()?;
    assert_eq!(promotion_merges(&p.a)?, 2);
    discovery_primary_once(&p)?;
    p.privacy()
}
fn all_images(p: &Pair) -> Result<[[u8; 32]; 3], FixtureError> {
    Ok([
        image(&p.a.root)?,
        image(&p.b.root)?,
        image(p.server.repository_path())?,
    ])
}
fn blocker(mode: usize) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    let repo = p.a.repo()?;
    let expected = match mode {
        0 => {
            fixed(std::fs::write(p.a.root.join("fixture.txt"), b"dirty\n"))?;
            PromotionBlocker::PrimaryDirty
        }
        1 => {
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
            PromotionBlocker::PrimaryConflicted
        }
        2 => {
            fixed(repo.reference("refs/heads/other", oid(&p.a, MAIN)?, false, "fixture"))?;
            fixed(repo.set_head("refs/heads/other"))?;
            PromotionBlocker::PrimaryNotCheckedOut
        }
        3 => {
            commit_files(&repo, &[("ahead", b"ahead\n")])?;
            PromotionBlocker::PrimaryAheadOfRemote
        }
        4 => {
            fixed(std::fs::write(
                p.a.worktree().join(DOCUMENT_PATH),
                b"dirty\n",
            ))?;
            PromotionBlocker::ContextDirty
        }
        5 | 6 => {
            fixed(p.sync(&p.a))?;
            fixed(p.sync(&p.a))?;
            poll(&p, &p.b)?;
            if mode == 5 {
                commit_files(&linked(&p.b)?, &[("peer", b"peer\n")])?;
                fixed(p.sync(&p.b))?;
                peer_fetch(
                    &p,
                    &p.a,
                    CONTEXT,
                    &format!(
                        "refs/remotes/origin/{}",
                        CONTEXT.trim_start_matches("refs/heads/")
                    ),
                )?;
                PromotionBlocker::ContextBehindRemote
            } else {
                peer_delete(&p)?;
                poll(&p, &p.a)?;
                // D5 is explicitly conditional on absent T(C). Cycle 08 polls
                // preserve tracking; the user's ordinary prune removes it.
                let tracking = format!(
                    "refs/remotes/origin/{}",
                    CONTEXT.trim_start_matches("refs/heads/")
                );
                fixed(fixed(repo.find_reference(&tracking))?.delete())?;
                PromotionBlocker::RemoteContextDeleted
            }
        }
        _ => return Err(FixtureError),
    };
    p.server.wait_for_helper_audits()?;
    let before = all_images(&p)?;
    let connections = p.server.accepted_sockets();
    let preflight = fixed(p.a.service.preview_document_promotion(
        PreviewDocumentPromotionRequest {
            root: p.a.root.clone(),
            item_id: DOC.parse().unwrap(),
        },
    ))?;
    p.diagnostics(&format!("{preflight:?}"))?;
    assert!(
        matches!(preflight,DocumentPromotionPreflight::Blocked {ref blockers} if blockers.contains(&expected))
    );
    assert!(
        matches!(p.execute(&p.a.service,request),Err(DocumentPromotionError::Blocked(ref b)) if b.contains(&expected))
    );
    assert_eq!(p.server.accepted_sockets(), connections);
    assert_eq!(all_images(&p)?, before);
    p.privacy()
}
fn preservation(mode: usize) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    stopped(&p, &request, FailurePoint::AfterPublication)?;
    let context = oid(&p.a, CONTEXT)?;
    match mode {
        0 => {
            fixed(std::fs::write(
                p.a.worktree().join(DOCUMENT_PATH),
                fixed(std::fs::read(p.a.worktree().join(DOCUMENT_PATH)))?
                    .into_iter()
                    .chain(b"user edit\n".iter().copied())
                    .collect::<Vec<_>>(),
            ))?;
        }
        1 => fixed(std::fs::write(p.a.worktree().join("u"), b"user\n"))?,
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
        }
        3 => {
            fixed(p.a.repo()?.find_worktree(DOC))?
                .lock(Some("owned fixture lock"))
                .map_err(|_| FixtureError)?;
        }
        _ => return Err(FixtureError),
    }
    let before = image(&p.a.worktree())?;
    let administration = image(linked(&p.a)?.path())?;
    let outcome = run(&p, &request)?;
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
    assert_eq!(server_oid(&p, CONTEXT)?, context);
    assert_eq!(image(&p.a.worktree())?, before);
    assert_eq!(image(linked(&p.a)?.path())?, administration);
    assert_eq!(oid(&p.a, CONTEXT)?, context);
    p.privacy()?;
    match mode {
        0 => {
            p.save_body(&p.a, &(BODY.to_owned() + "resolved user edit\n"))?;
        }
        1 => fixed(std::fs::remove_file(p.a.worktree().join("u")))?,
        2 => fixed(std::fs::remove_file(p.a.worktree().join("private-ignored")))?,
        3 => fixed(fixed(p.a.repo()?.find_worktree(DOC))?.unlock())?,
        _ => unreachable!(),
    }
    // A new confirmation observes the published primary (the push itself does
    // not advance T(P)); this ordinary inter-step poll is allowed before F6.
    poll(&p, &p.a)?;
    let next = p.request()?;
    assert_ne!(next.operation_id, request.operation_id);
    assert_eq!(run(&p, &next)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(step(&p.a, request.operation_id)?, "superseded");
    p.privacy()
}
fn lose_registry(p: &mut Pair) -> Result<(), FixtureError> {
    for entry in fixed(std::fs::read_dir(&p.a.data))? {
        let path = fixed(entry)?.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(REGISTRY_FILE))
        {
            fixed(std::fs::remove_file(path))?;
        }
    }
    fixed(std::fs::write(
        p.a.data.join(REGISTRY_FILE),
        b"lost registry",
    ))?;
    p.a.service = fixed(RepositoryService::open_at(&p.a.data))?;
    fixed(p.a.service.rebuild_repository(RebuildRepositoryRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
    }))?;
    select_key(&p.a.service, &p.server)?;
    Ok(())
}
fn unknown(side: &Side) -> Result<(), FixtureError> {
    assert_eq!(
        fixed(side.db()?.query_row(
            "SELECT history_unknown FROM remote_polling_state",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        1
    );
    Ok(())
}
fn retained_r1() -> Result<(), FixtureError> {
    let mut p = Pair::new()?;
    p.save(&p.a)?;
    fixed(p.sync(&p.a))?;
    lose_registry(&mut p)?;
    unknown(&p.a)?;
    // Public observation of the still-published branch restores its real history.
    fixed(p.sync(&p.a))?;
    p.promote()?;
    assert_eq!(count(&p.a, "retired_contexts")?, 1);
    p.a.service = fixed(RepositoryService::open_at(&p.a.data))?;
    unknown(&p.a)?;
    let base = oid(&p.a, MAIN)?;
    fixed(p.a.service.prepare_context(p.a.target()))?;
    assert_eq!(oid(&p.a, CONTEXT)?, base);
    let saved = p.save_body(&p.a, &(BODY.to_owned() + "fresh retained proof\n"))?;
    assert_eq!(
        fixed(p.a.service.remote_snapshot(&p.a.root))?
            .publication_evidence_for(AuthoringKind::Document, &DOC.parse().unwrap()),
        RemotePublicationEvidence::NeverPublished
    );
    assert_eq!(published(fixed(p.sync(&p.a))?)?, saved);
    unknown(&p.a)?;
    assert_eq!(count(&p.a, "retired_contexts")?, 0);
    let other: canonical::ItemId = "01ARZ3NDEKTSV4RRFFQ69G5FAY".parse().unwrap();
    // A separate unknown item in this same registry gets no R1 permission.
    fixed(p.a.service.save_document(SaveDocumentRequest {
        target: AuthoringTarget {
            root: p.a.root.clone(),
            kind: AuthoringKind::Document,
            item_id: other.clone(),
            intent: ContextIntent::Create,
            operation_id: OperationId::new(),
        },
        source_path: None,
        destination_path: "docs/other.md".into(),
        draft: DocumentDraft {
            title: TITLE.into(),
            body: BODY.into(),
        },
        expected_source: None,
        expected_destination: ExpectedPathObservation::Missing,
    }))?;
    let receives = p.server.receive_updates();
    let mut other_request = SynchronizeRemoteRequest {
        root: p.a.root.clone(),
        operation_id: OperationId::new(),
        target: SynchronizationTarget::Context {
            kind: AuthoringKind::Document,
            item_id: other,
        },
        restart: false,
        confirmed_identity: None,
        approval: Some(p.approval()),
    };
    let result = p.a.service.synchronize_remote(
        other_request.clone(),
        &mut SessionCredentials::new(remote_world::Provider),
    );
    p.diagnostics(&format!("{result:?}"))?;
    assert!(matches!(result, Err(SynchronizationError::HistoryUnknown)));
    assert_eq!(p.server.receive_updates(), receives);
    fixed(
        p.a.service
            .cancel_remote_operation(&p.a.root, other_request.operation_id),
    )?;
    other_request.restart = true;
    let ack = p.a.service.synchronize_remote(
        other_request,
        &mut SessionCredentials::new(remote_world::Provider),
    );
    assert!(matches!(
        ack,
        Err(SynchronizationError::Interrupted | SynchronizationError::RecoveryRequired)
    ));
    peer_delete(&p)?;
    assert!(matches!(
        p.sync(&p.a),
        Err(SynchronizationError::RemoteContextDeleted)
    ));
    unknown(&p.a)?;
    p.privacy()
}
fn changed_content(primary: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    let request = p.request()?;
    let base = oid(&p.a, MAIN)?;
    if primary {
        commit_files(&p.b.repo()?, &[("peer-main", b"primary advance\n")])?;
        fixed(p.sync_target(&p.b, SynchronizationTarget::Primary))?;
    } else {
        commit_files(&linked(&p.b)?, &[("peer-context", b"context advance\n")])?;
        fixed(p.sync(&p.b))?;
    }
    if primary {
        assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
        assert_eq!(
            blob(&p.a.repo()?, oid(&p.a, MAIN)?, "peer-main")?,
            b"primary advance\n"
        );
    } else {
        assert!(matches!(
            p.execute(&p.a.service, request.clone()),
            Err(DocumentPromotionError::ReconfirmationRequired)
        ));
        assert_eq!(oid(&p.a, MAIN)?, base);
        assert_eq!(promotion_merges(&p.a)?, 0);
        let preview = p.preview()?;
        assert!(
            preview
                .changes()
                .iter()
                .any(|c| c.path() == Path::new("peer-context"))
        );
        assert_ne!(preview.expectation(), request.expected);
        p.promote()?;
    }
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}
fn resolve_local(p: &Pair, child: OperationId) -> Result<git2::Oid, FixtureError> {
    let inspection = fixed(
        p.a.service
            .inspect_synchronization_recovery(&p.a.root, child),
    )?;
    p.diagnostics(&format!("{inspection:?}"))?;
    let mut results = Vec::new();
    for path in &inspection.paths {
        let sides = fixed(p.a.service.read_synchronization_conflict(&path.token))?;
        p.diagnostics(&format!("{sides:?}"))?;
        results.push((path.token.clone(), sides.local.ok_or(FixtureError)?));
    }
    let request = ResolveSynchronizationRequest::new(
        p.a.root.clone(),
        child,
        OperationId::new(),
        inspection.observation,
        results,
        None,
    );
    p.diagnostics(&format!("{request:?}"))?;
    let result = fixed(p.a.service.resolve_synchronization(request))?;
    p.diagnostics(&format!("{result:?}"))?;
    match result {
        ResolveSynchronizationOutcome::LocalCheckpointComplete { commit_oid } => Ok(commit_oid),
        _ => Err(FixtureError),
    }
}
fn conflict() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    let left = p.save_body(&p.a, &(BODY.to_owned() + "left\n"))?;
    let right = p.save_body(&p.b, &(BODY.to_owned() + "right\n"))?;
    fixed(p.sync(&p.b))?;
    let request = p.request()?;
    let Err(DocumentPromotionError::ContextSynchronization(error)) =
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
    let resolved = resolve_local(&p, child)?;
    assert_eq!(
        fixed(p.a.repo()?.find_commit(resolved))?
            .parent_ids()
            .collect::<Vec<_>>(),
        [left, right]
    );
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}
fn primary_rejection() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    p.save(&p.a)?;
    let request = p.request()?;
    let server = server_oid(&p, MAIN)?;
    p.server.reject_primary_updates(true)?;
    let result = run(&p, &request)?;
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
    assert_eq!(server_oid(&p, CONTEXT)?, oid(&p.a, CONTEXT)?);
    p.privacy()?;
    p.server.reject_primary_updates(false)?;
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(promotion_merges(&p.a)?, 1);
    assert_eq!(
        p.server.receive_updates()[cursor..]
            .iter()
            .filter(|u| u.accepted)
            .count(),
        3
    );
    p.privacy()
}
fn offline_before() -> Result<(), FixtureError> {
    offline_retry(false)
}
fn offline_after() -> Result<(), FixtureError> {
    offline_retry(true)
}
fn transport_attempt(
    p: &Pair,
    request: &PromoteDocumentRequest,
    primary: bool,
) -> Result<(String, i64), FixtureError> {
    let sql = if primary {
        "SELECT primary_sync_id,primary_attempt FROM document_promotions WHERE operation_id=?1"
    } else {
        "SELECT context_sync_id,context_attempt FROM document_promotions WHERE operation_id=?1"
    };
    fixed(
        p.a.db()?
            .query_row(sql, [request.operation_id.to_string()], |r| {
                Ok((r.get(0)?, r.get(1)?))
            }),
    )
}
fn offline_retry(integrated: bool) -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let cursor = p.server.receive_updates().len();
    let base = server_oid(&p, MAIN)?;
    let context = p.save(&p.a)?;
    let request = p.request()?;
    if integrated {
        stopped(&p, &request, FailurePoint::AfterIntegrationRef)?;
    }
    let primary = oid(&p.a, MAIN)?;
    let worktree = image(&p.a.worktree())?;
    let endpoint = p.server.url();
    let host = p.server.host_identity();
    let receipts = p.server.receive_updates();
    // A persistent real SSH disconnect prevents Git advertisements at this SAME
    // endpoint. Clearing it restores the same listener/repository/server key;
    // no child result, endpoint substitution, mock or sleep is involved.
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
            matches!(result, Err(DocumentPromotionError::ContextSynchronization(ref e))
            if matches!(**e, SynchronizationError::Transport(_)))
        );
        assert!(server_oid(&p, CONTEXT).is_err());
    }
    let (old_child, old_attempt) = transport_attempt(&p, &request, integrated)?;
    assert_eq!(old_attempt, 1);
    // The failed actual transport child has no retained integration/resolution.
    let db = p.a.db()?;
    for table in ["remote_integration_steps", "remote_resolution_attempts"] {
        assert_eq!(fixed(db.query_row(&format!(
            "SELECT COUNT(*) FROM {table} WHERE operation_record_id=(SELECT id FROM remote_operation_records WHERE operation_ulid=?1)"
        ), [&old_child], |r| r.get::<_, i64>(0)))?, 0);
    }
    assert_eq!(server_oid(&p, MAIN)?, base);
    assert_eq!(oid(&p.a, MAIN)?, primary);
    assert_eq!(oid(&p.a, CONTEXT)?, context);
    assert_eq!(image(&p.a.worktree())?, worktree);
    assert_eq!(promotion_merges(&p.a)?, usize::from(integrated));
    p.server.wait_for_helper_audits()?;
    assert_eq!(p.server.receive_updates(), receipts);
    p.privacy()?;

    p.server.clear_fault();
    assert!(p.server.url() == endpoint && p.server.host_identity() == host);
    // The identical public parent identity/digest resumes only unfinished work.
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    let (new_child, new_attempt) = transport_attempt(&p, &request, integrated)?;
    assert_ne!(new_child, old_child);
    assert_eq!(new_attempt, 2);
    let phase: String = fixed(db.query_row(
        "SELECT phase FROM remote_operation_records WHERE operation_ulid=?1",
        [&old_child],
        |r| r.get(0),
    ))?;
    assert!(matches!(
        phase.as_str(),
        "cancelled" | "failed" | "interrupted"
    ));
    assert_eq!(promotion_merges(&p.a)?, 1);
    assert_eq!(server_oid(&p, MAIN)?, oid(&p.a, MAIN)?);
    assert!(server_oid(&p, CONTEXT).is_err());
    assert!(!p.a.worktree().exists() && p.a.repo()?.find_reference(CONTEXT).is_err());
    p.server.wait_for_helper_audits()?;
    let updates = p.server.receive_updates();
    let lifecycle = &updates[cursor..];
    assert_eq!(lifecycle.len(), 3);
    assert!(lifecycle.iter().all(|u| u.accepted));
    assert_eq!(lifecycle.iter().filter(|u| u.reference == MAIN).count(), 1);
    assert_eq!(
        lifecycle
            .iter()
            .filter(|u| u.reference == CONTEXT && !u.new_oid.is_zero())
            .count(),
        1
    );
    assert_eq!(
        lifecycle
            .iter()
            .filter(|u| u.reference == CONTEXT && u.new_oid.is_zero())
            .count(),
        1
    );
    discovery_primary_once(&p)?;
    p.privacy()
}
fn deletion_rejection() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    fixed(p.sync(&p.a))?;
    let request = p.request()?;
    p.server.reject_context_deletion(true)?;
    let before = image(&p.a.worktree())?;
    let result = run(&p, &request)?;
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RemoteDeletionFailed(
                PromotionRemoteDeletionCategory::Rejected
            )
        }
    );
    assert_eq!(image(&p.a.worktree())?, before);
    assert_eq!(server_oid(&p, CONTEXT)?, oid(&p.a, CONTEXT)?);
    p.privacy()?;
    p.server.reject_context_deletion(false)?;
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(promotion_merges(&p.a)?, 1);
    p.privacy()
}
fn deletion_race() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    let old = p.save(&p.a)?;
    fixed(p.sync(&p.a))?;
    poll(&p, &p.b)?;
    let peer = commit_files(&linked(&p.b)?, &[("peer-race", b"peer\n")])?;
    // Make the peer object available on the server without moving the selected
    // context, then use Task 1's genuine advertise/receive race.
    let b = p.b.repo()?;
    fixed(b.reference("refs/heads/fixture-peer", peer, true, "fixture"))?;
    remote_world::push_peer(&b, &p.server, "refs/heads/fixture-peer")?;
    let request = p.request()?;
    stopped(&p, &request, FailurePoint::AfterPublication)?;
    p.server
        .race_context_update(ssh_remote::ObservationRef::RemoteDocument, old, peer)?;
    let before = image(&p.a.worktree())?;
    let result = run(&p, &request)?;
    assert_eq!(
        result.cleanup(),
        PromotionCleanup::Pending {
            reason: PromotionCleanupPending::RemoteContextChanged
        }
    );
    assert_eq!(server_oid(&p, CONTEXT)?, peer);
    assert_eq!(image(&p.a.worktree())?, before);
    let tracking = format!(
        "refs/remotes/origin/{}",
        CONTEXT.trim_start_matches("refs/heads/")
    );
    assert_eq!(oid(&p.a, &tracking)?, peer);
    p.privacy()
}
fn integration_base() -> Result<(), FixtureError> {
    let p = Pair::new()?;
    p.save(&p.a)?;
    let request = p.request()?;
    let base = oid(&p.a, MAIN)?;
    stopped(&p, &request, FailurePoint::AfterContextSync)?;
    let old_child: String = fixed(p.a.db()?.query_row(
        "SELECT context_sync_id FROM document_promotions WHERE operation_id=?1",
        [request.operation_id.to_string()],
        |r| r.get(0),
    ))?;
    commit_files(&p.b.repo()?, &[("poll-primary", b"published peer\n")])?;
    fixed(p.sync_target(&p.b, SynchronizationTarget::Primary))?;
    fixed(p.a.service.set_remote_polling(
        &p.a.root,
        true,
        false,
        PollingInterval::from_seconds(120).unwrap(),
    ))?;
    let mut automatic_session = SessionCredentials::new(remote_world::Provider);
    fixed(p.a.service.verify_ssh_transport(
        VerifySshTransportRequest {
            root: p.a.root.clone(),
            direction: SshDirection::Fetch,
            approval: Some(p.approval()),
        },
        &mut automatic_session,
    ))?;
    let automatic = p.a.service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: p.a.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Automatic,
            approval: Some(p.approval()),
            restart: false,
        },
        &mut automatic_session,
    );
    p.diagnostics(&format!("{automatic:?}"))?;
    assert!(
        matches!(fixed(automatic)?,RemotePollResult::Complete(RemotePollOutcome::Polled(ref report)) if report.completion()==PollCompletion::Completed)
    );
    let primary = oid(&p.a, MAIN)?;
    assert_ne!(primary, base);
    assert!(matches!(
        p.execute(&p.a.service, request.clone()),
        Err(DocumentPromotionError::PrimaryAdvanced)
    ));
    assert_eq!(oid(&p.a, MAIN)?, primary);
    assert_eq!(promotion_merges(&p.a)?, 0);
    assert_eq!(run(&p, &request)?.cleanup(), PromotionCleanup::Complete);
    assert_eq!(promotion_merges(&p.a)?, 1);
    let (new_child, attempt): (String, i64) = fixed(p.a.db()?.query_row(
        "SELECT context_sync_id,context_attempt FROM document_promotions WHERE operation_id=?1",
        [request.operation_id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    assert_ne!(new_child, old_child);
    // T0 permits replacing a persisted-but-unrun ID; attempt numbers need not
    // be contiguous executions. Exactly two real context children must run.
    assert!(attempt > 1);
    assert_eq!(
        fixed(p.a.db()?.query_row(
            "SELECT COUNT(*) FROM remote_operation_records WHERE action='synchronize_context'",
            [],
            |r| r.get::<_, i64>(0)
        ))?,
        2
    );
    assert_eq!(
        blob(&p.a.repo()?, server_oid(&p, MAIN)?, "poll-primary")?,
        b"published peer\n"
    );
    p.privacy()
}
