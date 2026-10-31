//! `document promote` and `ticket close`: the design's prepare rows and the
//! rows of a first run (Task 2a), a `c5_lifecycle_` case per row and verb.
//!
//! Re-entry and the rest of the answers table are Task 2b's, cancellation
//! 2c's, resume 3's and the made-ready report 4's.

use std::cell::OnceCell;

use super::*;
use lifecycle_world::*;
use results::{CheckpointEffect, CleanupEffect, IntegrationEffect};

macro_rules! both {
    ($( $name:literal => $row:ident ),+ $(,)?) => {
        &[$(
            (concat!("c5_lifecycle_", $name, "_promote"), || $row(Verb::Promote)),
            (concat!("c5_lifecycle_", $name, "_close"), || $row(Verb::Close)),
        )+]
    };
}

const BOTH: &[ssh_harness::Case] = both![
    "prepare_ready" => prepare_ready,
    "prepare_primary_dirty" => prepare_primary_dirty,
    "prepare_mixed_blockers" => prepare_mixed_blockers,
    "prepare_context_missing" => prepare_context_missing,
    "prepare_unknown_id" => prepare_unknown_id,
    "prepare_wrong_kind" => prepare_wrong_kind,
    "prepare_not_enabled" => prepare_not_enabled,
    "prepare_not_registered" => prepare_not_registered,
    "stale_at_confirm" => stale_at_confirm,
    "published" => published,
    "local_only" => local_only,
    "already_current" => already_current,
    "publication_rejected" => publication_rejected,
    "blocked_at_run" => blocked_at_run,
    "conformance" => conformance_of,
];

const ONE: &[ssh_harness::Case] = &[
    (
        "c5_lifecycle_prepare_identity_required_close",
        prepare_identity_required,
    ),
    (
        "c5_lifecycle_prepare_already_closed_close",
        prepare_already_closed,
    ),
];

pub(crate) const CASES: &[ssh_harness::Case] = &[
    BOTH[0], BOTH[1], BOTH[2], BOTH[3], BOTH[4], BOTH[5], BOTH[6], BOTH[7], BOTH[8], BOTH[9],
    BOTH[10], BOTH[11], BOTH[12], BOTH[13], BOTH[14], BOTH[15], BOTH[16], BOTH[17], BOTH[18],
    BOTH[19], BOTH[20], BOTH[21], BOTH[22], BOTH[23], BOTH[24], BOTH[25], BOTH[26], BOTH[27],
    BOTH[28], BOTH[29], ONE[0], ONE[1],
];

const FIRST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ1";
const SECOND: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ2";
const UNKNOWN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FJ9";

const PREVIEW: &str = "lifecycle_preview.schema.json";
const BLOCKED: &str = "lifecycle_blocked.schema.json";
const MUTATION: &str = "lifecycle_mutation.schema.json";

/// `prepare`, checked against the published schemas, naming no request.
fn prepared(
    service: &RepositoryService,
    verb: Verb,
    root: &std::path::Path,
    id: &str,
) -> Envelope<PrepareDataDto> {
    let envelope = service.prepare(&verb.mutation(root, id));
    let rendered = serde_json::to_vec(&envelope).expect("an envelope serializes");
    ssh_privacy::clean(&rendered, &[BODY.as_bytes().to_vec()]).expect("no source in a preview");
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    match &envelope.data {
        Some(PrepareDataDto::Preview(_)) => {
            schema::check_published(PREVIEW, &value["data"]).expect("the preview schema");
        }
        Some(PrepareDataDto::LifecycleBlocked(_)) => {
            schema::check_published(BLOCKED, &value["data"]).expect("the blocked schema");
        }
        Some(PrepareDataDto::HostReplacement(_)) => panic!("no host is named"),
        None => {}
    }
    assert_eq!(envelope.command, verb.command());
    assert_eq!(envelope.request_id, None);
    assert_eq!(envelope.operation_id, None);
    envelope
}

/// One call of `execute`, checked against the published schemas, with what
/// a lifecycle's result may never hold scanned for.
fn called(
    service: &RepositoryService,
    id: &str,
    confirmation: Option<&str>,
    mutation: Mutation,
) -> Envelope<MutationDataDto> {
    let envelope = service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: confirmation.map(|id| ConfirmationId::parse(id).unwrap()),
            mutation,
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    let rendered = serde_json::to_vec(&envelope).expect("an envelope serializes");
    ssh_privacy::clean(
        &rendered,
        &[
            BODY.as_bytes().to_vec(),
            b"fixture@example.invalid".to_vec(),
        ],
    )
    .expect("neither source nor closed_by in a result");
    let value = serde_json::to_value(&envelope).expect("an envelope serializes");
    schema::check_published("envelope.schema.json", &value).expect("the envelope schema");
    if !value["data"].is_null() {
        schema::check_published(MUTATION, &value["data"]).expect("the lifecycle data schema");
    }
    envelope
}

/// The confirmation of a preview that must be given.
fn confirmation_of(envelope: &Envelope<PrepareDataDto>) -> String {
    assert_eq!(
        envelope.code,
        ResultCode::Ok,
        "a preview: {:?}",
        envelope.data
    );
    envelope
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .expect("a preview")
        .confirmation_id
        .clone()
}

fn blockers_of(envelope: &Envelope<PrepareDataDto>) -> Vec<LifecycleBlocker> {
    match &envelope.data {
        Some(PrepareDataDto::LifecycleBlocked(data)) => data.blockers.clone(),
        _ => Vec::new(),
    }
}

fn lifecycle_of(envelope: &Envelope<MutationDataDto>) -> Option<&LifecycleMutationDto> {
    match envelope.data.as_ref() {
        Some(MutationDataDto::Lifecycle(data)) => Some(data),
        _ => None,
    }
}

fn prepare_actions(envelope: &Envelope<PrepareDataDto>) -> Vec<RecoveryActionKind> {
    envelope
        .recovery
        .iter()
        .map(|action| action.action)
        .collect()
}

/// Checks an envelope against its golden fixture, with the commits, the
/// confirmation and the time of a run replaced.
fn golden_at(
    root: &std::path::Path,
    name: &str,
    data_schema: Option<&str>,
    envelope: &impl serde::Serialize,
) {
    let value = serde_json::to_value(envelope).unwrap();
    let mut replaced: Vec<(String, &str)> = Vec::new();
    for (pointer, placeholder) in [
        ("/data/confirmation_id", "<confirmation>"),
        ("/data/expires_at", "<expires>"),
        ("/data/details/context_oid", "<context>"),
        ("/data/details/primary_oid", "<primary>"),
        ("/data/checkpoint_oid", "<checkpoint>"),
        ("/data/integration_oid", "<integration>"),
        ("/data/publication_oid", "<publication>"),
    ] {
        if let Some(found) = value.pointer(pointer).and_then(|found| found.as_str()) {
            replaced.push((found.to_owned(), placeholder));
        }
    }
    let replaced: Vec<(&str, &str)> = replaced
        .iter()
        .map(|(found, placeholder)| (found.as_str(), *placeholder))
        .collect();
    let root = root.canonicalize().unwrap();
    mutation_golden::assert_contract_at(
        name,
        root.to_str().unwrap(),
        data_schema,
        &replaced,
        &value,
    );
}

fn confirmations(db: &rusqlite::Connection) -> Result<i64, FixtureError> {
    fixed(
        db.query_row("SELECT COUNT(*) FROM confirmation_records", [], |row| {
            row.get(0)
        }),
    )
}

// Prepare.

/// A preview shows what the verb will do, changes nothing, and holds no
/// title, body or content.
fn prepare_ready(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let repo = w.repo()?;
    let main = fixed(repo.refname_to_id(MAIN))?;
    let tip = fixed(repo.refname_to_id(verb.branch()))?;
    let envelope = prepared(&w.service, verb, &w.root, verb.id());
    assert_eq!(envelope.code, ResultCode::Ok);
    assert_eq!(envelope.outcome, Outcome::Success);
    let preview = envelope
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?;
    assert_eq!(preview.command, verb.command());
    assert_eq!(preview.scope.item_id.as_deref(), Some(verb.id()));
    let effects: Vec<PreviewEffect> = preview.effects.iter().map(|effect| effect.effect).collect();
    let mut planned = vec![PreviewEffect::ContextSynchronized];
    if verb == Verb::Close {
        planned.push(PreviewEffect::ClosureRecorded);
    }
    planned.extend([
        PreviewEffect::PrimaryIntegrated,
        PreviewEffect::PrimaryPublished,
        PreviewEffect::RemoteBranchDeleted,
        PreviewEffect::WorktreeRemoved,
        PreviewEffect::BranchDeleted,
    ]);
    assert_eq!(effects, planned);
    let PreviewDetailsDto::Lifecycle(details) = &preview.details else {
        return Err(FixtureError);
    };
    assert_eq!(details.item_id, verb.id());
    assert_eq!(
        details.branch,
        verb.branch().strip_prefix("refs/heads/").unwrap()
    );
    assert_eq!(details.primary_branch, "main");
    assert_eq!(details.remote.as_deref(), Some("origin"));
    assert_eq!(details.context_oid, tip.to_string());
    assert_eq!(details.primary_oid, main.to_string());
    assert_eq!(
        details.integration,
        LifecyclePlannedIntegration::MergeCommit
    );
    assert_eq!(details.publication, LifecyclePlannedPublication::Publish);
    assert_eq!(details.changes.len(), 1, "{:?}", details.changes);
    assert_eq!(details.changes[0].change, LifecyclePathChangeKind::Added);
    match verb {
        Verb::Promote => {
            assert_eq!(details.kind, ItemDtoKind::Document);
            assert_eq!(details.changes[0].path, DOCUMENT_PATH);
            assert_eq!(details.closure, None);
        }
        Verb::Close => {
            assert_eq!(details.kind, ItemDtoKind::Ticket);
            assert_eq!(
                details.closure,
                Some(LifecyclePlannedClosureDto {
                    recording: LifecycleRecording::Record,
                    closed_by: "Fixture <fixture@example.invalid>".into(),
                })
            );
        }
    }
    // Nothing but the one confirmation.
    assert_eq!(fixed(repo.refname_to_id(MAIN))?, main);
    assert_eq!(fixed(repo.refname_to_id(verb.branch()))?, tip);
    let db = w.db()?;
    assert_eq!(confirmations(&db)?, 1);
    golden_at(
        &w.root,
        &format!("{}_preview", verb.golden()),
        Some(PREVIEW),
        &envelope,
    );
    Ok(())
}

/// A dirty primary is `recovery_required`, naming it, with no confirmation.
fn prepare_primary_dirty(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    dirty_primary(&local.root)?;
    let envelope = prepared(&local.service, verb, &local.root, verb.id());
    assert_eq!(envelope.code, ResultCode::RecoveryRequired);
    assert_eq!(blockers_of(&envelope), [LifecycleBlocker::PrimaryDirty]);
    assert_eq!(
        prepare_actions(&envelope),
        [RecoveryActionKind::RepoInspect]
    );
    assert_eq!(confirmations(&local.db()?)?, 0);
    golden_at(
        &local.root,
        &format!("{}_blocked", verb.golden()),
        Some(BLOCKED),
        &envelope,
    );
    Ok(())
}

/// Every blocker is named, whatever else is among them.
fn prepare_mixed_blockers(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    dirty_primary(&local.root)?;
    fixed(std::fs::write(
        verb.worktree(&local.root).join("dirty-context.txt"),
        b"dirty\n",
    ))?;
    let envelope = prepared(&local.service, verb, &local.root, verb.id());
    assert_eq!(envelope.code, ResultCode::RecoveryRequired);
    assert_eq!(
        blockers_of(&envelope),
        [
            LifecycleBlocker::ContextDirty,
            LifecycleBlocker::PrimaryDirty
        ]
    );
    assert_eq!(
        prepare_actions(&envelope),
        [RecoveryActionKind::RepoInspect]
    );
    Ok(())
}

/// An item on primary with no context: `ticket save` or `document save`
/// makes one, and the repository is what is looked at.
fn prepare_context_missing(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let repo = local.repo()?;
    fast_forward_primary(&repo, verb)?;
    retire_context(&repo, &local.root, verb)?;
    // The index tells a missing context from an unknown ID, and it reads the
    // item from primary once it has looked.
    let refreshed = called(
        &local.service,
        SECOND,
        None,
        Mutation::IndexRefresh(IndexRefreshInput {
            root: local.root.clone(),
        }),
    );
    assert_eq!(refreshed.code, ResultCode::Ok);
    let envelope = prepared(&local.service, verb, &local.root, verb.id());
    assert_eq!(envelope.code, ResultCode::RecoveryRequired);
    assert_eq!(blockers_of(&envelope), [LifecycleBlocker::ContextMissing]);
    assert_eq!(
        prepare_actions(&envelope),
        [RecoveryActionKind::RepoInspect]
    );
    Ok(())
}

/// An ID no item holds, as `item republish` answers it.
fn prepare_unknown_id(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::empty()?;
    let envelope = prepared(&local.service, verb, &local.root, UNKNOWN);
    assert_eq!(envelope.code, ResultCode::ItemNotFound);
    assert!(envelope.data.is_none());
    if verb == Verb::Close {
        golden_at(&local.root, "ticket_close_item_not_found", None, &envelope);
    }
    Ok(())
}

/// The other verb's item is not this verb's to act on.
fn prepare_wrong_kind(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb.other())?;
    let envelope = prepared(&local.service, verb, &local.root, verb.other().id());
    assert_eq!(envelope.code, ResultCode::ItemNotFound);
    assert!(envelope.data.is_none());
    Ok(())
}

fn prepare_not_enabled(verb: Verb) -> Result<(), FixtureError> {
    let directory = fixed(tempfile::tempdir())?;
    let root = directory.path().join("never");
    initialized(&root)?;
    let service = fixed(RepositoryService::open_at(&directory.path().join("data")))?;
    let envelope = prepared(&service, verb, &root, verb.id());
    assert_eq!(envelope.code, ResultCode::RepositoryNotEnabled);
    Ok(())
}

fn prepare_not_registered(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let other = fixed(RepositoryService::open_at(
        &local.directory.path().join("other-data"),
    ))?;
    let envelope = prepared(&other, verb, &local.root, verb.id());
    assert_eq!(envelope.code, ResultCode::RepositoryNotRegistered);
    Ok(())
}

/// `[IdentityRequired]` alone: nothing but an identity stands in the way.
fn prepare_identity_required() -> Result<(), FixtureError> {
    let local = Local::holding(Verb::Close)?;
    forget_identity(&local.repo()?)?;
    let envelope = prepared(&local.service, Verb::Close, &local.root, ITEM);
    assert_eq!(envelope.code, ResultCode::IdentityRequired);
    assert_eq!(blockers_of(&envelope), [LifecycleBlocker::IdentityRequired]);
    assert_eq!(
        prepare_actions(&envelope),
        [RecoveryActionKind::RepoIdentitySet]
    );
    golden_at(
        &local.root,
        "ticket_close_identity_required",
        Some(BLOCKED),
        &envelope,
    );
    Ok(())
}

/// `[AlreadyClosed]`: the verb's own earlier closure.
fn prepare_already_closed() -> Result<(), FixtureError> {
    let local = Local::holding(Verb::Close)?;
    let confirmation = confirmation_of(&prepared(&local.service, Verb::Close, &local.root, ITEM));
    let done = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        Verb::Close.mutation(&local.root, ITEM),
    );
    assert_eq!(done.code, ResultCode::Ok);
    let envelope = prepared(&local.service, Verb::Close, &local.root, ITEM);
    assert_eq!(envelope.code, ResultCode::TicketClosed);
    assert_eq!(blockers_of(&envelope), [LifecycleBlocker::AlreadyClosed]);
    golden_at(
        &local.root,
        "ticket_close_ticket_closed",
        Some(BLOCKED),
        &envelope,
    );
    Ok(())
}

// Confirm.

/// What the preview showed has changed before the request is accepted:
/// refused with no record, and a new preview names what stands in the way.
fn stale_at_confirm(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let main = fixed(local.repo()?.refname_to_id(MAIN))?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    dirty_primary(&local.root)?;
    let envelope = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(envelope.code, ResultCode::ExternalChange);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RequestPrepare]);
    assert!(
        local
            .service
            .request_record(request(FIRST))
            .unwrap()
            .is_none()
    );
    assert_eq!(fixed(local.repo()?.refname_to_id(MAIN))?, main);
    let again = prepared(&local.service, verb, &local.root, verb.id());
    assert_eq!(blockers_of(&again), [LifecycleBlocker::PrimaryDirty]);
    Ok(())
}

// A first run.

/// Everything done: merged, published, the remote branch deleted and the
/// context retired. The result is final.
fn published(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    let envelope = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(envelope.code, ResultCode::Ok, "{:?}", envelope.data);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.recovery, []);
    assert_eq!(envelope.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(envelope.effects.publication, PublicationEffect::Published);
    assert_eq!(envelope.effects.integration, IntegrationEffect::Complete);
    assert_eq!(envelope.effects.cleanup, CleanupEffect::Complete);
    let data = lifecycle_of(&envelope).ok_or(FixtureError)?;
    let merged = fixed(w.repo()?.refname_to_id(MAIN))?;
    assert_eq!(data.item_id, verb.id());
    assert_eq!(data.integration, Some(LifecycleIntegrationKind::Merged));
    assert_eq!(data.integration_oid, Some(merged.to_string()));
    assert_eq!(data.publication, Some(LifecyclePublicationState::Published));
    assert_eq!(data.publication_oid, Some(merged.to_string()));
    assert_eq!(data.publication_pending, None);
    assert_eq!(data.cleanup, Some(LifecycleCleanupState::Complete));
    assert_eq!(data.cleanup_pending, None);
    assert_eq!(data.blockers, []);
    match verb {
        Verb::Promote => {
            assert_eq!(data.recording, None);
            assert_eq!(data.checkpoint_oid, None);
            assert_eq!(envelope.effects.checkpoint, CheckpointEffect::NotRequested);
        }
        Verb::Close => {
            assert_eq!(data.recording, Some(LifecycleRecording::Record));
            assert!(data.checkpoint_oid.is_some());
            assert_eq!(envelope.effects.checkpoint, CheckpointEffect::Committed);
        }
    }
    assert_eq!(fixed(w.bare()?.refname_to_id(MAIN))?, merged);
    assert!(w.bare()?.refname_to_id(verb.branch()).is_err());
    assert!(w.repo()?.refname_to_id(verb.branch()).is_err());
    let record = record(&w, FIRST).ok_or(FixtureError)?;
    assert_eq!(record.state, RequestState::Finished);
    golden_at(
        &w.root,
        &format!("{}_ok", verb.golden()),
        Some(MUTATION),
        &envelope,
    );
    Ok(())
}

/// No publication remote: `ok`, with the publication pending, and final.
fn local_only(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let preview = prepared(&local.service, verb, &local.root, verb.id());
    let details = preview
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?;
    let effects: Vec<PreviewEffect> = details.effects.iter().map(|effect| effect.effect).collect();
    assert!(!effects.contains(&PreviewEffect::ContextSynchronized));
    assert!(!effects.contains(&PreviewEffect::PrimaryPublished));
    assert!(!effects.contains(&PreviewEffect::RemoteBranchDeleted));
    let PreviewDetailsDto::Lifecycle(shown) = &details.details else {
        return Err(FixtureError);
    };
    assert_eq!(shown.remote, None);
    assert_eq!(
        shown.publication,
        LifecyclePlannedPublication::PendingNoRemote
    );
    let confirmation = confirmation_of(&preview);
    let envelope = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(envelope.code, ResultCode::Ok, "{:?}", envelope.data);
    assert_eq!(envelope.outcome, Outcome::Success);
    assert_eq!(envelope.failure_class(), None);
    assert_eq!(envelope.recovery, []);
    assert_eq!(envelope.effects.publication, PublicationEffect::Pending);
    assert_eq!(envelope.effects.integration, IntegrationEffect::Complete);
    assert_eq!(envelope.effects.cleanup, CleanupEffect::Complete);
    let data = lifecycle_of(&envelope).ok_or(FixtureError)?;
    assert_eq!(data.publication, Some(LifecyclePublicationState::Pending));
    assert_eq!(
        data.publication_pending,
        Some(LifecyclePublicationPendingReason::NoPublicationRemote)
    );
    assert_eq!(data.publication_oid, None);
    let held = local
        .service
        .request_record(request(FIRST))
        .unwrap()
        .ok_or(FixtureError)?;
    assert_eq!(held.state, RequestState::Finished);
    golden_at(
        &local.root,
        &format!("{}_local_only", verb.golden()),
        Some(MUTATION),
        &envelope,
    );
    Ok(())
}

/// A context primary already holds, on a primary the remote already holds:
/// the run is cleanup, and `ok`.
fn already_current(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    if verb == Verb::Close {
        // A closure already recorded on the branch.
        let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
        let worktree = verb.worktree(&w.root);
        let source = fixed(std::fs::read_to_string(worktree.join(&path)))?;
        let closed = source.replacen(
            "---\n",
            "---\nclosed_at: 2027-01-15T08:00:00Z\nclosed_by: Fixture <fixture@example.invalid>\n",
            1,
        );
        commit_in(&worktree, verb.branch(), &path, closed.as_bytes())?;
    }
    let repo = w.repo()?;
    let tip = fast_forward_primary(&repo, verb)?;
    push_peer(&repo, &w.server, MAIN)?;
    fixed(w.sync(w.primary()))?;
    let pushes = accepted(&w.server);
    let envelope = prepared(&w.service, verb, &w.root, verb.id());
    let PreviewDetailsDto::Lifecycle(shown) = &envelope
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?
        .details
    else {
        return Err(FixtureError);
    };
    assert_eq!(
        shown.integration,
        LifecyclePlannedIntegration::AlreadyContained
    );
    if verb == Verb::Close {
        assert_eq!(
            shown.closure.as_ref().map(|closure| closure.recording),
            Some(LifecycleRecording::Existing)
        );
    }
    let confirmation = confirmation_of(&envelope);
    let done = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.publication, PublicationEffect::Current);
    assert_eq!(done.effects.integration, IntegrationEffect::Complete);
    assert_eq!(done.effects.cleanup, CleanupEffect::Complete);
    let data = lifecycle_of(&done).ok_or(FixtureError)?;
    assert_eq!(
        data.integration,
        Some(LifecycleIntegrationKind::AlreadyContained)
    );
    assert_eq!(
        data.publication,
        Some(LifecyclePublicationState::AlreadyCurrent)
    );
    assert_eq!(data.publication_oid, Some(tip.to_string()));
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, tip);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(
        record(&w, FIRST).ok_or(FixtureError)?.state,
        RequestState::Finished
    );
    if verb == Verb::Close {
        assert_eq!(data.recording, Some(LifecycleRecording::Existing));
        assert_eq!(data.checkpoint_oid, None);
        assert_eq!(done.effects.checkpoint, CheckpointEffect::NotRequested);
        golden_at(
            &w.root,
            "ticket_close_already_current",
            Some(MUTATION),
            &done,
        );
    }
    Ok(())
}

/// The remote refuses primary: merged here and not published, so `partial`
/// with `push_rejected`, the request left for the same request to continue.
fn publication_rejected(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    w.server.reject_primary_updates(true)?;
    let envelope = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(
        envelope.code,
        ResultCode::PushRejected,
        "{:?}",
        envelope.data
    );
    assert_eq!(envelope.outcome, Outcome::Partial);
    assert_eq!(actions(&envelope)[0], RecoveryActionKind::RequestRetry);
    assert_eq!(envelope.effects.integration, IntegrationEffect::Complete);
    assert_eq!(envelope.effects.publication, PublicationEffect::Pending);
    assert_eq!(envelope.effects.cleanup, CleanupEffect::Pending);
    let data = lifecycle_of(&envelope).ok_or(FixtureError)?;
    assert_eq!(
        data.publication_pending,
        Some(LifecyclePublicationPendingReason::Rejected)
    );
    assert_eq!(
        data.cleanup_pending,
        Some(LifecycleCleanupPendingReason::AwaitingPublication)
    );
    assert_eq!(
        record(&w, FIRST).ok_or(FixtureError)?.state,
        RequestState::Accepted
    );
    golden_at(
        &w.root,
        &format!("{}_push_rejected", verb.golden()),
        Some(MUTATION),
        &envelope,
    );
    Ok(())
}

/// A blocker that appears once the request is accepted is the domain's to
/// refuse, as a prepare answers it. The refusal is final: its record holds
/// the blockers, and a replay gives them back.
fn blocked_at_run(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let main = fixed(local.repo()?.refname_to_id(MAIN))?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    let root = local.root.clone();
    local
        .service
        .set_request_hook_for_testing(move || dirty_primary(&root).expect("a dirty primary"));
    let envelope = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(envelope.code, ResultCode::RecoveryRequired);
    assert_eq!(
        lifecycle_of(&envelope).ok_or(FixtureError)?.blockers,
        [LifecycleBlocker::PrimaryDirty]
    );
    assert_eq!(fixed(local.repo()?.refname_to_id(MAIN))?, main);
    let held = local
        .service
        .request_record(request(FIRST))
        .unwrap()
        .ok_or(FixtureError)?;
    assert_eq!(held.state, RequestState::Finished);
    let replayed = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(replayed.data, envelope.data);
    assert_eq!(replayed.code, envelope.code);
    golden_at(
        &local.root,
        &format!("{}_blocked_at_run", verb.golden()),
        Some(MUTATION),
        &envelope,
    );
    Ok(())
}

// Conformance.

fn conforming(verb: Verb, rule: &mut dyn FnMut(&conformance::Subject<'_>)) -> Result<(), String> {
    use conformance::{Ask, NO_SUCH_CONFIRMATION, Recorded as Held, Subject};
    let w = remote(verb).map_err(|_| "a world holding the verb's item".to_owned())?;
    let confirmation = OnceCell::new();
    let preview = || -> String {
        confirmation
            .get_or_init(|| confirmation_of(&prepared(&w.service, verb, &w.root, verb.id())))
            .clone()
    };
    rule(&Subject {
        outdate: Some(&|| {
            preview();
            dirty_primary(&w.root).expect("a dirty primary");
        }),
        call: &|id, ask| {
            let presented = match ask {
                Ask::Same | Ask::Changed => Some(preview()),
                Ask::Refused => Some(NO_SUCH_CONFIRMATION.to_owned()),
                Ask::Unconsented => None,
            };
            let mutation = match ask {
                // Another item: a changed input under a finished request's ID.
                Ask::Changed => verb.mutation(&w.root, UNKNOWN),
                Ask::Same | Ask::Refused | Ask::Unconsented => verb.mutation(&w.root, verb.id()),
            };
            let envelope = presenting(&w, id, presented.as_deref(), mutation);
            serde_json::to_vec(&envelope).expect("an envelope serializes")
        },
        record: &|id| {
            record(&w, id).map(|record| Held {
                finished: record.state == RequestState::Finished,
                image: format!("{record:?}"),
            })
        },
        request_rows: &|| {
            w.db()
                .expect("the database")
                .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
                .expect("the request records")
        },
    });
    Ok(())
}

pub(super) fn conforming_promote(
    rule: &mut dyn FnMut(&conformance::Subject<'_>),
) -> Result<(), String> {
    conforming(Verb::Promote, rule)
}

pub(super) fn conforming_close(
    rule: &mut dyn FnMut(&conformance::Subject<'_>),
) -> Result<(), String> {
    conforming(Verb::Close, rule)
}

fn conformance_of(verb: Verb) -> Result<(), FixtureError> {
    conformance_row(verb.command())
}
