//! `document promote` and `ticket close`: the design's prepare rows, the rows
//! of a first run (Task 2a), and its re-entry, standing and answers tables
//! (Task 2b), a `c5_lifecycle_` case per row and verb.
//!
//! Cancellation is 2c's, resume 3's and the made-ready report 4's.

use std::{
    cell::OnceCell,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};

use super::*;
use lifecycle_world::*;
use results::{CheckpointEffect, CleanupEffect, IntegrationEffect};
use time::OffsetDateTime;

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
    "prepare_stale_index" => prepare_stale_index,
    "prepare_index_not_marked" => prepare_index_not_marked,
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
    "crash_before_record" => crash_before_record,
    "crash_before_record_stale" => crash_before_record_stale,
    "crash_before_record_blocked" => crash_before_record_blocked,
    "crash_before_record_past_ten_minutes" => crash_before_record_past_ten_minutes,
    "deleted_record_past_ten_minutes" => deleted_record_past_ten_minutes,
    "expired_confirmation" => expired_confirmation,
    "unfinished_continues" => unfinished_continues,
    "kept_record_past_ten_minutes" => kept_record_past_ten_minutes,
    "stopped_past_integration" => stopped_past_integration,
    "complete_with_record_accepted" => complete_with_record_accepted,
    "refused_with_record_accepted" => refused_with_record_accepted,
    "consent_refused_at_run" => consent_refused_at_run,
    "superseded" => superseded,
    "cleanup_ignored_files" => cleanup_ignored_files,
    "primary_advanced" => primary_advanced,
    "push_rejected_continues" => push_rejected_continues,
    "deletion_rejected" => deletion_rejected,
    "index_pending" => index_pending,
    "busy_with_no_parent" => busy_with_no_parent,
    "local_sync_pending" => local_sync_pending,
    "unlock_stop" => unlock_stop,
    "child_host_stop" => child_host_stop,
    "child_transport_stop" => child_transport_stop,
    "child_remote_branch_deleted" => child_remote_branch_deleted,
    "publication_transport" => publication_transport,
    "deletion_transport" => deletion_transport,
    "deletion_unlock" => deletion_unlock,
    "deletion_host" => deletion_host,
    "cache_loss_request" => cache_loss_request,
    "cache_loss_integrated" => cache_loss_integrated,
    "cache_loss_remote_gone" => cache_loss_remote_gone,
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
    (
        "c5_lifecycle_identity_removed_after_prepare_close",
        identity_removed_after_prepare,
    ),
    ("c5_lifecycle_prepare_slug_close", prepare_slug),
    (
        "c5_lifecycle_stop_with_written_checkpoint_close",
        stop_with_written_checkpoint,
    ),
    (
        "c5_lifecycle_refused_checkpoint_merge_close",
        refused_checkpoint_merge,
    ),
    ("c5_lifecycle_child_conflict_close", child_conflict),
    (
        "c5_lifecycle_comment_publication_pending_close",
        comment_publication_pending,
    ),
    (
        "c5_lifecycle_cache_loss_checkpoint_close",
        cache_loss_checkpoint,
    ),
];

const fn joined<const N: usize>(
    first: &[ssh_harness::Case],
    second: &[ssh_harness::Case],
) -> [ssh_harness::Case; N] {
    let mut all = [first[0]; N];
    let mut at = 0;
    while at < first.len() {
        all[at] = first[at];
        at += 1;
    }
    while at < N {
        all[at] = second[at - first.len()];
        at += 1;
    }
    all
}

pub(crate) const CASES: &[ssh_harness::Case] = &joined::<{ BOTH.len() + ONE.len() }>(BOTH, ONE);

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
    ssh_privacy::clean(
        &rendered,
        &[BODY.as_bytes().to_vec(), TITLE.as_bytes().to_vec()],
    )
    .expect("neither source nor title in a preview");
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
            TITLE.as_bytes().to_vec(),
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
        ("/effects/commit_oid", "<integration>"),
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

/// The world of the two rows below: an item on primary whose context was
/// retired outside Manyhands, which the index has not looked at since.
fn retired_unrefreshed(verb: Verb) -> Result<Local, FixtureError> {
    let local = Local::holding(verb)?;
    let repo = local.repo()?;
    fast_forward_primary(&repo, verb)?;
    retire_context(&repo, &local.root, verb)?;
    Ok(local)
}

/// An index marked behind cannot say an ID is unknown: the blockers stand.
fn prepare_stale_index(verb: Verb) -> Result<(), FixtureError> {
    let local = retired_unrefreshed(verb)?;
    fixed(
        local
            .db()?
            .execute("UPDATE repositories SET refresh_required = 1", []),
    )?;
    let envelope = prepared(&local.service, verb, &local.root, verb.id());
    assert_eq!(envelope.code, ResultCode::RecoveryRequired);
    assert_eq!(blockers_of(&envelope), [LifecycleBlocker::ContextMissing]);
    Ok(())
}

/// The design's known limit: an index not marked behind is believed, until
/// `index refresh`.
fn prepare_index_not_marked(verb: Verb) -> Result<(), FixtureError> {
    let local = retired_unrefreshed(verb)?;
    let envelope = prepared(&local.service, verb, &local.root, verb.id());
    assert_eq!(envelope.code, ResultCode::ItemNotFound);
    assert!(envelope.data.is_none());
    Ok(())
}

/// A ticket's short code, as the index holds it, is shown.
fn prepare_slug() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let local = Local::holding(verb)?;
    let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    let worktree = verb.worktree(&local.root);
    let source = fixed(std::fs::read_to_string(worktree.join(&path)))?;
    let slugged = source.replacen("---\n", "---\nslug: mh-vh-k9x2b\n", 1);
    commit_in(&worktree, verb.branch(), &path, slugged.as_bytes())?;
    let refreshed = called(
        &local.service,
        SECOND,
        None,
        Mutation::IndexRefresh(IndexRefreshInput {
            root: local.root.clone(),
        }),
    );
    assert_eq!(refreshed.code, ResultCode::Ok);
    let envelope = prepared(&local.service, verb, &local.root, ITEM);
    let preview = envelope
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?;
    let PreviewDetailsDto::Lifecycle(details) = &preview.details else {
        return Err(FixtureError);
    };
    assert_eq!(details.slug.as_deref(), Some("mh-vh-k9x2b"));
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

/// R-22: an identity removed since the preview is the one thing that stands
/// in the way, and the answer says so, with no record and no domain call.
fn identity_removed_after_prepare() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let local = Local::holding(verb)?;
    let repo = local.repo()?;
    let main = fixed(repo.refname_to_id(MAIN))?;
    let tip = fixed(repo.refname_to_id(verb.branch()))?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, ITEM));
    forget_identity(&repo)?;
    let envelope = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, ITEM),
    );
    assert_eq!(envelope.code, ResultCode::IdentityRequired);
    assert_eq!(actions(&envelope), [RecoveryActionKind::RepoIdentitySet]);
    assert_eq!(
        lifecycle_of(&envelope).ok_or(FixtureError)?.blockers,
        [LifecycleBlocker::IdentityRequired]
    );
    assert!(
        local
            .service
            .request_record(request(FIRST))
            .unwrap()
            .is_none()
    );
    let parents: i64 = fixed(local.db()?.query_row(
        "SELECT COUNT(*) FROM ticket_closures",
        [],
        |row| row.get(0),
    ))?;
    assert_eq!(parents, 0);
    assert_eq!(fixed(repo.refname_to_id(MAIN))?, main);
    assert_eq!(fixed(repo.refname_to_id(verb.branch()))?, tip);
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
    let repo = local.repo()?;
    assert_eq!(
        data.integration_oid,
        Some(fixed(repo.refname_to_id(MAIN))?.to_string())
    );
    assert!(repo.refname_to_id(verb.branch()).is_err());
    assert!(!verb.worktree(&local.root).exists());
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

// Re-entry, standing and answers.

/// A clock a row moves past a confirmation's ten minutes.
struct RowClock(AtomicI64);

impl RowClock {
    fn now() -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(
            OffsetDateTime::now_utc().unix_timestamp(),
        )))
    }

    fn past_ten_minutes(&self) {
        self.0.fetch_add(3600, Ordering::SeqCst);
    }
}

impl Clock for RowClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst)).unwrap()
    }
}

fn clean_service(data: &Path) -> Result<RepositoryService, FixtureError> {
    fixed(RepositoryService::open_at(data))
}

fn failing_service(data: &Path, point: FailurePoint) -> Result<RepositoryService, FixtureError> {
    fixed(RepositoryService::open_at_with_failure_point_for_testing(
        data, point,
    ))
}

fn local_data(local: &Local) -> std::path::PathBuf {
    local.directory.path().join("data")
}

/// The request `id`, resent with what its record holds.
fn resent(
    service: &RepositoryService,
    id: &str,
    verb: Verb,
    root: &Path,
) -> Envelope<MutationDataDto> {
    called(service, id, None, verb.mutation(root, verb.id()))
}

/// A preview, then the request stopped by `point`, on a remote world.
fn stopped_at(
    w: &mut World,
    verb: Verb,
    point: FailurePoint,
) -> Result<(String, Envelope<MutationDataDto>), FixtureError> {
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    let failing = failing_service(&w.data(), point)?;
    let envelope = called(
        &failing,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    w.service = clean_service(&w.data())?;
    Ok((confirmation, envelope))
}

fn state_of(w: &World, id: &str) -> Option<RequestState> {
    record(w, id).map(|record| record.state)
}

fn local_state(local: &Local, id: &str) -> Option<RequestState> {
    local
        .service
        .request_record(request(id))
        .unwrap()
        .map(|record| record.state)
}

fn table(verb: Verb) -> &'static str {
    match verb {
        Verb::Promote => "document_promotions",
        Verb::Close => "ticket_closures",
    }
}

fn parents(db: &rusqlite::Connection, verb: Verb) -> Result<i64, FixtureError> {
    fixed(db.query_row(
        &format!("SELECT COUNT(*) FROM {}", table(verb)),
        [],
        |row| row.get(0),
    ))
}

/// The commits on primary that merged a context: one, however many calls.
fn merges_on_primary(repo: &git2::Repository) -> Result<usize, FixtureError> {
    let mut walk = fixed(repo.revwalk())?;
    fixed(walk.push(fixed(repo.refname_to_id(MAIN))?))?;
    // Primary's own line: a context's synchronization merges primary into it,
    // on the side.
    fixed(walk.simplify_first_parent())?;
    let mut merges = 0;
    for oid in walk {
        if fixed(repo.find_commit(fixed(oid)?))?.parent_count() == 2 {
            merges += 1;
        }
    }
    Ok(merges)
}

fn other_verb_mutation(verb: Verb, root: &Path) -> Mutation {
    verb.mutation(root, UNKNOWN)
}

/// A commit on the verb's context, which changes what a preview shows.
fn edit_context(verb: Verb, root: &Path) -> Result<git2::Oid, FixtureError> {
    commit_in(
        &verb.worktree(root),
        verb.branch(),
        "edited-context.txt",
        b"edited\n",
    )
}

fn poll(w: &World) -> Result<(), FixtureError> {
    fixed(w.service.poll_publication_remote(
        PollPublicationRemoteRequest {
            root: w.root.clone(),
            operation_id: OperationId::new(),
            invocation: RemotePollInvocation::Explicit,
            approval: None,
            restart: false,
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    Ok(())
}

/// A peer publishes a commit on primary, and this clone's poll moves its own.
fn peer_advances_primary(w: &World) -> Result<git2::Oid, FixtureError> {
    let advanced = advance(&w.peer, MAIN, "peer-primary")?;
    push_peer(&w.peer, &w.server, MAIN)?;
    poll(w)?;
    Ok(advanced)
}

/// The registry is lost and rebuilt, as a corrupt cache is: its publication
/// history is unknown, and its key and host pin are chosen again.
fn lose_registry(w: &mut World) -> Result<(), FixtureError> {
    for entry in fixed(std::fs::read_dir(w.data()))? {
        let path = fixed(entry)?.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(REGISTRY_FILE))
        {
            fixed(std::fs::remove_file(path))?;
        }
    }
    fixed(std::fs::write(
        w.data().join(REGISTRY_FILE),
        b"lost registry",
    ))?;
    w.service = clean_service(&w.data())?;
    fixed(w.service.rebuild_repository(RebuildRepositoryRequest {
        root: w.root.clone(),
        operation_id: OperationId::new(),
    }))?;
    let RegisterSharedKeyOutcome::Registered(key) =
        fixed(w.service.register_shared_key(RegisterSharedKeyRequest {
            label: "acceptance-selected".into(),
            ownership: SharedKeyOwnership::Imported,
            private_key_path: w.server.client_key_path().into(),
            public_key_path: None,
        }))?
    else {
        return Err(FixtureError);
    };
    fixed(w.service.select_shared_key(key.id))?;
    trust(w)?;
    poll(w)
}

fn preview_details(
    envelope: &Envelope<PrepareDataDto>,
) -> Result<&LifecyclePreviewDto, FixtureError> {
    match &envelope
        .data
        .as_ref()
        .and_then(PrepareDataDto::preview)
        .ok_or(FixtureError)?
        .details
    {
        PreviewDetailsDto::Lifecycle(details) => Ok(details),
        _ => Err(FixtureError),
    }
}

// The re-entry table: no parent record.

/// A local request stopped between its acceptance and its record.
fn accepted_with_no_parent(
    verb: Verb,
) -> Result<(Local, String, Envelope<MutationDataDto>), FixtureError> {
    let mut local = Local::holding(verb)?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    let failing = failing_service(&local_data(&local), FailurePoint::BeforeRequestDomainCall)?;
    let envelope = called(
        &failing,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    local.service = clean_service(&local_data(&local))?;
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Accepted));
    assert_eq!(parents(&local.db()?, verb)?, 0);
    Ok((local, confirmation, envelope))
}

/// What was previewed is still there: it runs as a first call.
fn crash_before_record(verb: Verb) -> Result<(), FixtureError> {
    let (local, _, _) = accepted_with_no_parent(verb)?;
    let done = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.integration, IntegrationEffect::Complete);
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Finished));
    assert_eq!(merges_on_primary(&local.repo()?)?, 1);
    Ok(())
}

/// What was previewed has moved: `external_change`, and the record is deleted.
fn crash_before_record_stale(verb: Verb) -> Result<(), FixtureError> {
    let (local, _, _) = accepted_with_no_parent(verb)?;
    edit_context(verb, &local.root)?;
    let refused = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestPrepare]);
    assert_eq!(local_state(&local, FIRST), None);
    assert_eq!(parents(&local.db()?, verb)?, 0);
    Ok(())
}

/// A preview now blocked answers as a prepare does, and the record is deleted.
fn crash_before_record_blocked(verb: Verb) -> Result<(), FixtureError> {
    let (local, _, _) = accepted_with_no_parent(verb)?;
    dirty_primary(&local.root)?;
    let refused = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
    assert_eq!(
        lifecycle_of(&refused).ok_or(FixtureError)?.blockers,
        [LifecycleBlocker::PrimaryDirty]
    );
    assert_eq!(local_state(&local, FIRST), None);
    assert_eq!(parents(&local.db()?, verb)?, 0);
    Ok(())
}

/// A kept record's consent is the request's, however old: it runs.
fn crash_before_record_past_ten_minutes(verb: Verb) -> Result<(), FixtureError> {
    let (local, _, _) = accepted_with_no_parent(verb)?;
    let clock = RowClock::now();
    clock.past_ten_minutes();
    let later = clean_service(&local_data(&local))?.with_clock_for_testing(clock);
    let done = resent(&later, FIRST, verb, &local.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Finished));
    Ok(())
}

/// A deleted record's resend is a new request: its confirmation is released,
/// and past ten minutes it has expired.
fn deleted_record_past_ten_minutes(verb: Verb) -> Result<(), FixtureError> {
    let (local, confirmation, _) = accepted_with_no_parent(verb)?;
    edit_context(verb, &local.root)?;
    let refused = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(local_state(&local, FIRST), None);
    let clock = RowClock::now();
    clock.past_ten_minutes();
    let later = clean_service(&local_data(&local))?.with_clock_for_testing(clock);
    let expired = called(
        &later,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(expired.code, ResultCode::ConfirmationExpired);
    assert_eq!(actions(&expired), [RecoveryActionKind::RequestPrepare]);
    Ok(())
}

/// A confirmation no request has accepted lasts ten minutes.
fn expired_confirmation(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let main = fixed(local.repo()?.refname_to_id(MAIN))?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    let clock = RowClock::now();
    clock.past_ten_minutes();
    let later = clean_service(&local_data(&local))?.with_clock_for_testing(clock);
    let expired = called(
        &later,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(expired.code, ResultCode::ConfirmationExpired);
    assert_eq!(local_state(&local, FIRST), None);
    assert_eq!(fixed(local.repo()?.refname_to_id(MAIN))?, main);
    Ok(())
}

// The re-entry table: a parent record.

/// A stop before any durable effect leaves the record, and the same request
/// goes on with nothing compared.
fn unfinished_continues(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let (_, stopped) = stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    assert_eq!(stopped.code, ResultCode::InternalError);
    assert_ne!(stopped.outcome, Outcome::Partial, "nothing durable yet");
    assert_eq!(actions(&stopped)[0], RecoveryActionKind::RequestRetry);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    assert_eq!(parents(&w.db()?, verb)?, 1);
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.operation_id, stopped.operation_id);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Finished));
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// An `Owed` request's consent stands however old it is.
fn kept_record_past_ten_minutes(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    let clock = RowClock::now();
    clock.past_ten_minutes();
    let later = clean_service(&w.data())?.with_clock_for_testing(clock);
    let done = resent(&later, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Finished));
    Ok(())
}

/// A stop in cleanup, and what the parent shows of it to the same ID sent
/// for another item: `recorded_effects`, read past integration.
fn stopped_past_integration(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let (_, stopped) = stopped_at(&mut w, verb, FailurePoint::BeforeRemoteDeletion)?;
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    let main = fixed(w.repo()?.refname_to_id(MAIN))?;
    let mismatch = called(&w.service, FIRST, None, other_verb_mutation(verb, &w.root));
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(mismatch.outcome, Outcome::Partial);
    assert_eq!(mismatch.effects.integration, IntegrationEffect::Complete);
    assert_eq!(mismatch.effects.publication, PublicationEffect::Published);
    assert_eq!(mismatch.effects.cleanup, CleanupEffect::Pending);
    assert_eq!(mismatch.effects.commit_oid, Some(main.to_string()));
    if verb == Verb::Close {
        assert_eq!(mismatch.effects.checkpoint, CheckpointEffect::Committed);
    }
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, main);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// A complete parent whose result was lost: the domain gives its stored
/// outcome, with no transport.
fn complete_with_record_accepted(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let (_, lost) = stopped_at(&mut w, verb, FailurePoint::BeforeRequestSettlement)?;
    assert_eq!(
        state_of(&w, FIRST),
        Some(RequestState::Accepted),
        "{lost:?}"
    );
    let pushes = accepted(&w.server);
    let connections = connections(&w);
    let main = fixed(w.repo()?.refname_to_id(MAIN))?;
    let mismatch = called(&w.service, FIRST, None, other_verb_mutation(verb, &w.root));
    assert_eq!(mismatch.effects.cleanup, CleanupEffect::Complete);
    assert_eq!(mismatch.effects.publication, PublicationEffect::Published);
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.integration, IntegrationEffect::Complete);
    assert_eq!(done.effects.publication, PublicationEffect::Published);
    assert_eq!(accepted(&w.server), pushes);
    assert_eq!(super::connections(&w), connections);
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, main);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Finished));
    Ok(())
}

/// A parent a refusal ended, whose result was lost: the stored refusal, final,
/// whatever has been put right since.
fn refused_with_record_accepted(verb: Verb) -> Result<(), FixtureError> {
    let mut local = Local::holding(verb)?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    let failing = failing_service(&local_data(&local), FailurePoint::BeforeRequestSettlement)?;
    let root = local.root.clone();
    failing.set_request_hook_for_testing(move || dirty_primary(&root).expect("a dirty primary"));
    called(
        &failing,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    local.service = clean_service(&local_data(&local))?;
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Accepted));
    fixed(std::fs::remove_file(local.root.join("dirty-primary.txt")))?;
    let refused = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(refused.code, ResultCode::RecoveryRequired);
    assert_eq!(
        lifecycle_of(&refused).ok_or(FixtureError)?.blockers,
        [LifecycleBlocker::PrimaryDirty]
    );
    assert_eq!(actions(&refused), [RecoveryActionKind::RepoInspect]);
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Finished));
    Ok(())
}

/// What the request consented to moved before its record began: the record
/// ends with the consent's refusal, which a new preview answers.
fn consent_refused_at_run(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    let root = local.root.clone();
    local.service.set_request_hook_for_testing(move || {
        edit_context(verb, &root).expect("an edited context");
    });
    let refused = called(
        &local.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    assert_eq!(refused.code, ResultCode::ExternalChange);
    assert_eq!(refused.outcome, Outcome::Blocked);
    assert_eq!(actions(&refused), [RecoveryActionKind::RequestPrepare]);
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Finished));
    let replayed = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(replayed.code, refused.code);
    Ok(())
}

/// Review Focus 1: cleanup stopped on a supersedable reason; a new request
/// replaces the old, whose request then answers `request_mismatch` for good.
fn superseded(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    let first_merge = fixed(w.repo()?.refname_to_id(MAIN))?;
    let untracked = verb.worktree(&w.root).join("untracked.txt");
    fixed(std::fs::write(&untracked, b"the owner's\n"))?;
    let changed = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(
        changed.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        changed.data
    );
    assert_eq!(changed.outcome, Outcome::Partial);
    assert_eq!(
        actions(&changed),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(
        lifecycle_of(&changed).ok_or(FixtureError)?.cleanup_pending,
        Some(LifecycleCleanupPendingReason::ContextChanged)
    );
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    // The owner's file moves on, and a poll observes the published primary:
    // the push itself does not advance the tracking reference.
    fixed(std::fs::remove_file(&untracked))?;
    poll(&w)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    let replaced = called(
        &w.service,
        SECOND,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(replaced.code, ResultCode::Ok, "{:?}", replaced.data);
    let old = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(old.code, ResultCode::RequestMismatch);
    assert_eq!(old.recovery, []);
    assert_eq!(old.outcome, Outcome::Partial, "its integration stands");
    assert_eq!(old.effects.commit_oid, Some(first_merge.to_string()));
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Finished));
    let again = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(again.code, ResultCode::RequestMismatch);
    assert_eq!(again.effects, old.effects);
    golden_at(
        &w.root,
        &format!("{}_superseded", verb.golden()),
        None,
        &old,
    );
    Ok(())
}

/// Ignored files in the worktree stop cleanup, and the same request finishes
/// once they are gone.
fn cleanup_ignored_files(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    fixed(std::fs::write(
        w.root.join(".git/info/exclude"),
        b"fixture-ignored\n",
    ))?;
    let ignored = verb.worktree(&w.root).join("fixture-ignored");
    fixed(std::fs::write(&ignored, b"ignored\n"))?;
    let stopped = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(
        lifecycle_of(&stopped).ok_or(FixtureError)?.cleanup_pending,
        Some(LifecycleCleanupPendingReason::IgnoredFiles)
    );
    fixed(std::fs::remove_file(&ignored))?;
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    Ok(())
}

/// Primary moved under a synchronized context: the parent stays unfinished,
/// and the same request starts again from the context.
fn primary_advanced(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let (_, stopped) = stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    let advanced = peer_advances_primary(&w)?;
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, advanced);
    let moved = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(moved.code, ResultCode::ExternalChange, "{:?}", moved.data);
    assert_eq!(moved.outcome, Outcome::Blocked);
    assert_eq!(
        actions(&moved),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(moved.operation_id, stopped.operation_id);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.operation_id, stopped.operation_id);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// The remote refuses primary, and takes it once it no longer does. The
/// `discovery` a stop reports is what the index holds.
fn push_rejected_continues(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    w.server.reject_primary_updates(true)?;
    let stopped = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(stopped.code, ResultCode::PushRejected);
    let snapshot = fixed(w.service.repository_snapshot(&w.root))?;
    assert_eq!(
        stopped.effects.discovery == DiscoveryEffect::Pending,
        snapshot.refresh_required,
        "a stop's discovery is what the index holds"
    );
    let main = fixed(w.repo()?.refname_to_id(MAIN))?;
    let again = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(again.code, ResultCode::PushRejected);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    w.server.reject_primary_updates(false)?;
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.publication, PublicationEffect::Published);
    assert_eq!(done.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, main);
    assert_eq!(fixed(w.bare()?.refname_to_id(MAIN))?, main);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// The remote refuses the context's deletion: published, cleanup pending.
fn deletion_rejected(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    fixed(w.sync(w.request(context_target(verb)?)))?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    w.server.reject_context_deletion(true)?;
    // The fixture's native refusal applies to the receiver's current branch.
    fixed(fixed(w.bare())?.set_head(verb.branch()))?;
    let stopped = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(stopped.code, ResultCode::PushRejected, "{:?}", stopped.data);
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    assert_eq!(stopped.effects.publication, PublicationEffect::Published);
    assert_eq!(stopped.effects.cleanup, CleanupEffect::Pending);
    assert_eq!(
        lifecycle_of(&stopped).ok_or(FixtureError)?.cleanup_pending,
        Some(LifecycleCleanupPendingReason::RemoteDeletionRejected)
    );
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    golden_at(
        &w.root,
        &format!("{}_deletion_rejected", verb.golden()),
        Some(MUTATION),
        &stopped,
    );
    w.server.reject_context_deletion(false)?;
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert!(w.bare()?.refname_to_id(verb.branch()).is_err());
    Ok(())
}

/// Everything is done but the index: owed, and the same request finishes it.
fn index_pending(verb: Verb) -> Result<(), FixtureError> {
    let mut local = Local::holding(verb)?;
    let confirmation = confirmation_of(&prepared(&local.service, verb, &local.root, verb.id()));
    let failing = failing_service(&local_data(&local), FailurePoint::RefreshFailure)?;
    let pending = called(
        &failing,
        FIRST,
        Some(&confirmation),
        verb.mutation(&local.root, verb.id()),
    );
    local.service = clean_service(&local_data(&local))?;
    assert_eq!(
        pending.code,
        ResultCode::DiscoveryPending,
        "{:?}",
        pending.data
    );
    assert_eq!(pending.outcome, Outcome::Partial);
    assert_eq!(actions(&pending), [RecoveryActionKind::RequestRetry]);
    assert_eq!(pending.effects.discovery, DiscoveryEffect::Pending);
    assert_eq!(pending.effects.cleanup, CleanupEffect::Complete);
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Accepted));
    golden_at(
        &local.root,
        &format!("{}_discovery_pending", verb.golden()),
        Some(MUTATION),
        &pending,
    );
    let done = resent(&local.service, FIRST, verb, &local.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.discovery, DiscoveryEffect::Current);
    assert_eq!(local_state(&local, FIRST), Some(RequestState::Finished));
    Ok(())
}

/// A new ID while another request's parent is unfinished: no parent of its
/// own, so its record is deleted and a new prepare is what follows.
fn busy_with_no_parent(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    let busy = called(
        &w.service,
        SECOND,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(busy.code, ResultCode::Busy, "{:?}", busy.data);
    assert_eq!(busy.effects, Effects::not_requested());
    assert_eq!(actions(&busy), [RecoveryActionKind::RequestPrepare]);
    assert_eq!(state_of(&w, SECOND), None);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    assert_eq!(parents(&w.db()?, verb)?, 1);
    Ok(())
}

/// R-38: a pending local synchronization blocks a closure's prepare; a
/// promotion's domain refuses it at its start, its confirmation released.
fn local_sync_pending(verb: Verb) -> Result<(), FixtureError> {
    let local = Local::holding(verb)?;
    if verb == Verb::Promote {
        Verb::Close.save(&local.service, &local.root)?;
    }
    let source = format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ITEM}\n\
         title: Primary\ntype: task\nstatus: open\n---\nBody of Primary.\n"
    );
    commit_in(
        &local.root,
        MAIN,
        &format!(".manyhands/tickets/{ITEM}/ticket.md"),
        source.as_bytes(),
    )?;
    let id: canonical::ItemId = fixed(ITEM.parse())?;
    let observed = fixed(local.service.observe_local_context_sync(
        &fixed(local.service.resolve_repository(&local.root))?,
        AuthoringKind::Ticket,
        &id,
    ))?;
    let stopped = local.service.execute(
        MutationCall {
            request_id: request(SECOND),
            confirmation: None,
            mutation: Mutation::LocalContextSync(LocalContextSyncInput {
                root: local.root.clone(),
                kind: AuthoringKind::Ticket,
                id,
                observation: observed.observation,
            }),
        },
        &mut SessionCredentials::new(Provider),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(stopped.code, ResultCode::RecoveryRequired);
    let preview = prepared(&local.service, verb, &local.root, verb.id());
    match verb {
        Verb::Close => {
            assert_eq!(preview.code, ResultCode::RecoveryRequired);
            assert!(
                blockers_of(&preview).contains(&LifecycleBlocker::LocalOperationPending),
                "{:?}",
                blockers_of(&preview)
            );
        }
        Verb::Promote => {
            let confirmation = confirmation_of(&preview);
            let refused = called(
                &local.service,
                FIRST,
                Some(&confirmation),
                verb.mutation(&local.root, verb.id()),
            );
            assert_eq!(
                refused.code,
                ResultCode::RecoveryRequired,
                "{:?}",
                refused.data
            );
            assert_eq!(refused.outcome, Outcome::Blocked);
            assert_eq!(actions(&refused)[0], RecoveryActionKind::RepoInspect);
            assert_eq!(local_state(&local, FIRST), None);
            assert_eq!(parents(&local.db()?, verb)?, 0);
        }
    }
    Ok(())
}

fn context_target(verb: Verb) -> Result<SynchronizationTarget, FixtureError> {
    Ok(SynchronizationTarget::Context {
        kind: match verb {
            Verb::Promote => AuthoringKind::Document,
            Verb::Close => AuthoringKind::Ticket,
        },
        item_id: fixed(verb.id().parse())?,
    })
}

/// A protected key with nobody to ask stops at the context's synchronization,
/// and the same request goes on with one.
fn unlock_stop(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    let stopped = presenting_with(&w, FIRST, &confirmation, verb, NoPassphrase);
    assert_eq!(
        stopped.code,
        ResultCode::UnlockRequired,
        "{:?}",
        stopped.data
    );
    assert_eq!(stopped.outcome, Outcome::Blocked);
    assert_eq!(actions(&stopped)[0], RecoveryActionKind::RequestRetry);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    Ok(())
}

/// `execute` with a provider of the row's own.
fn presenting_with(
    w: &World,
    id: &str,
    confirmation: &str,
    verb: Verb,
    provider: impl SessionCredentialProvider,
) -> Envelope<MutationDataDto> {
    w.service.execute(
        MutationCall {
            request_id: request(id),
            confirmation: Some(ConfirmationId::parse(confirmation).unwrap()),
            mutation: verb.mutation(&w.root, verb.id()),
        },
        &mut SessionCredentials::new(provider),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

/// A host presenting another key stops at the context's synchronization. The
/// approval is the owner's, and the same request then continues.
fn child_host_stop(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    w.server.rotate_host_key()?;
    let stopped = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(
        stopped.code,
        ResultCode::HostReplacementRequired,
        "{:?}",
        stopped.data
    );
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    fixed(w.service.approve_host_pin_for_testing(
        &SshAuthority {
            host: "127.0.0.1".into(),
            port: w.server.address().port(),
        },
        &w.server.host_identity(),
    ))?;
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    Ok(())
}

fn child_transport_stop(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    w.server.clear_fault();
    assert_eq!(
        stopped.code,
        ResultCode::TransportUnavailable,
        "{:?}",
        stopped.data
    );
    assert_ne!(stopped.outcome, Outcome::Partial);
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    Ok(())
}

/// The context's branch deleted at the remote, found by its synchronization:
/// the parent stands, so its request is offered, not the child's cancellation.
fn child_remote_branch_deleted(verb: Verb) -> Result<(), FixtureError> {
    let w = remote(verb)?;
    fixed(w.sync(w.request(context_target(verb)?)))?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    fixed(fixed(w.bare()?.find_reference(verb.branch()))?.delete())?;
    let stopped = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(
        stopped.code,
        ResultCode::RemoteBranchDeleted,
        "{:?}",
        stopped.data
    );
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    assert_eq!(parents(&w.db()?, verb)?, 1);
    Ok(())
}

/// Publication's transport stop: partial, the same request first.
fn publication_transport(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterIntegrationRef)?;
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = resent(&w.service, FIRST, verb, &w.root);
    w.server.clear_fault();
    assert_eq!(
        stopped.code,
        ResultCode::TransportUnavailable,
        "{:?}",
        stopped.data
    );
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    assert_eq!(
        lifecycle_of(&stopped)
            .ok_or(FixtureError)?
            .publication_pending,
        Some(LifecyclePublicationPendingReason::Transport)
    );
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// The deletion's transport stop, after publication.
fn deletion_transport(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let stopped = resent(&w.service, FIRST, verb, &w.root);
    w.server.clear_fault();
    assert_eq!(
        stopped.code,
        ResultCode::TransportUnavailable,
        "{:?}",
        stopped.data
    );
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    assert_eq!(
        lifecycle_of(&stopped).ok_or(FixtureError)?.cleanup_pending,
        Some(LifecycleCleanupPendingReason::RemoteDeletionTransport)
    );
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    Ok(())
}

/// The deletion's unlock stop: `unlock_required`, partial.
fn deletion_unlock(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let (confirmation, _) = stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    let stopped = presenting_with(&w, FIRST, &confirmation, verb, NoPassphrase);
    assert_eq!(
        stopped.code,
        ResultCode::UnlockRequired,
        "{:?}",
        stopped.data
    );
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(actions(&stopped), [RecoveryActionKind::RequestRetry]);
    assert_eq!(
        lifecycle_of(&stopped).ok_or(FixtureError)?.cleanup_pending,
        Some(LifecycleCleanupPendingReason::RemoteDeletionUnlock)
    );
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    Ok(())
}

/// The deletion's host stop. The category carries no authority, so the
/// repository is what is looked at after the same request.
fn deletion_host(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterPublication)?;
    w.server.rotate_host_key()?;
    let stopped = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(
        stopped.code,
        ResultCode::HostApprovalRequired,
        "{:?}",
        stopped.data
    );
    assert_eq!(stopped.outcome, Outcome::Partial);
    assert_eq!(
        actions(&stopped),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    assert_eq!(
        lifecycle_of(&stopped).ok_or(FixtureError)?.cleanup_pending,
        Some(LifecycleCleanupPendingReason::RemoteDeletionHost)
    );
    Ok(())
}

// Cache loss.

/// The record went with the registry: the confirmation it held is not found.
fn cache_loss_request(verb: Verb) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    let (confirmation, _) = stopped_at(&mut w, verb, FailurePoint::AfterContextSync)?;
    lose_registry(&mut w)?;
    let answer = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(answer.code, ResultCode::ConfirmationNotFound);
    assert_eq!(actions(&answer), [RecoveryActionKind::RequestPrepare]);
    Ok(())
}

/// Integrated, then lost, with the remote branch present: a new prepare reads
/// Git and gives cleanup-only, and the first merge stands.
fn cache_loss_integrated(verb: Verb) -> Result<(), FixtureError> {
    cache_loss_after(verb, FailurePoint::AfterPublication, true)
}

/// The same, with the remote branch already gone.
fn cache_loss_remote_gone(verb: Verb) -> Result<(), FixtureError> {
    cache_loss_after(verb, FailurePoint::AfterRemoteDeletion, false)
}

/// The ticket's closure fields on primary or the context.
fn closure_fields(repo: &git2::Repository, reference: &str) -> Result<Vec<String>, FixtureError> {
    let tree = fixed(fixed(fixed(repo.find_reference(reference))?.peel_to_commit())?.tree())?;
    let entry = fixed(tree.get_path(Path::new(&format!(".manyhands/tickets/{ITEM}/ticket.md"))))?;
    let blob = fixed(repo.find_blob(entry.id()))?;
    Ok(String::from_utf8_lossy(blob.content())
        .lines()
        .filter(|line| line.starts_with("closed_at:") || line.starts_with("closed_by:"))
        .map(str::to_owned)
        .collect())
}

fn cache_loss_after(
    verb: Verb,
    point: FailurePoint,
    remote_branch: bool,
) -> Result<(), FixtureError> {
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, point)?;
    let main = fixed(w.repo()?.refname_to_id(MAIN))?;
    let fields = match verb {
        Verb::Close => closure_fields(&w.repo()?, MAIN)?,
        Verb::Promote => Vec::new(),
    };
    assert_eq!(
        w.bare()?.refname_to_id(verb.branch()).is_ok(),
        remote_branch
    );
    lose_registry(&mut w)?;
    let preview = prepared(&w.service, verb, &w.root, verb.id());
    let details = preview_details(&preview)?;
    assert_eq!(
        details.integration,
        LifecyclePlannedIntegration::AlreadyContained
    );
    assert_eq!(details.cleanup.remote_branch, remote_branch);
    if verb == Verb::Close {
        assert_eq!(
            details.closure.as_ref().map(|closure| closure.recording),
            Some(LifecycleRecording::Existing)
        );
    }
    let confirmation = confirmation_of(&preview);
    let done = called(
        &w.service,
        SECOND,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.effects.cleanup, CleanupEffect::Complete);
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, main);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    if verb == Verb::Close {
        assert_eq!(closure_fields(&w.repo()?, MAIN)?, fields);
    }
    Ok(())
}

/// A written, unintegrated checkpoint, then lost: an existing closure, whose
/// `closed_at` and `closed_by` keep their first values.
fn cache_loss_checkpoint() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterClosureCheckpoint)?;
    let fields = closure_fields(&w.repo()?, verb.branch())?;
    assert_eq!(fields.len(), 2);
    lose_registry(&mut w)?;
    let preview = prepared(&w.service, verb, &w.root, verb.id());
    let details = preview_details(&preview)?;
    assert_eq!(
        details.closure.as_ref().map(|closure| closure.recording),
        Some(LifecycleRecording::Existing)
    );
    assert_eq!(
        details.integration,
        LifecyclePlannedIntegration::MergeCommit
    );
    let confirmation = confirmation_of(&preview);
    let done = called(
        &w.service,
        SECOND,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(closure_fields(&w.repo()?, MAIN)?, fields);
    Ok(())
}

// Closure's own rows.

/// A stop with a written checkpoint and no remote row keeps the record, and
/// the effects are the parent's to report.
fn stop_with_written_checkpoint() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let mut w = remote(verb)?;
    let (_, stopped) = stopped_at(&mut w, verb, FailurePoint::AfterClosureCheckpoint)?;
    assert_eq!(stopped.code, ResultCode::InternalError);
    assert_eq!(
        stopped.outcome,
        Outcome::Partial,
        "the checkpoint is durable"
    );
    assert_eq!(stopped.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(stopped.effects.integration, IntegrationEffect::NotRequested);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    let operation = stopped.operation_id.clone().ok_or(FixtureError)?;
    let rows: i64 = fixed(w.db()?.query_row(
        "SELECT COUNT(*) FROM remote_operation_records WHERE operation_ulid=?1",
        [&operation],
        |row| row.get(0),
    ))?;
    assert_eq!(rows, 0, "a parent with no journal row");
    let mismatch = called(&w.service, FIRST, None, other_verb_mutation(verb, &w.root));
    assert_eq!(mismatch.code, ResultCode::RequestMismatch);
    assert_eq!(mismatch.outcome, Outcome::Partial);
    assert_eq!(mismatch.effects.checkpoint, CheckpointEffect::Committed);
    let fields = closure_fields(&w.repo()?, verb.branch())?;
    let done = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(done.code, ResultCode::Ok, "{:?}", done.data);
    assert_eq!(done.operation_id, Some(operation));
    assert_eq!(closure_fields(&w.repo()?, MAIN)?, fields);
    assert_eq!(merges_on_primary(&w.repo()?)?, 1);
    Ok(())
}

/// The peer's primary conflicts with a written checkpoint: nothing merges,
/// the record stands, and the data names the state.
fn refused_checkpoint_merge() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let mut w = remote(verb)?;
    stopped_at(&mut w, verb, FailurePoint::AfterClosureCheckpoint)?;
    let conflicting = fixed(w.peer.workdir().ok_or(FixtureError).map(Path::to_owned))?;
    commit_in(
        &conflicting,
        MAIN,
        &format!(".manyhands/tickets/{ITEM}/ticket.md"),
        b"conflicting\n",
    )?;
    push_peer(&w.peer, &w.server, MAIN)?;
    poll(&w)?;
    let main = fixed(w.repo()?.refname_to_id(MAIN))?;
    let refused = resent(&w.service, FIRST, verb, &w.root);
    assert_eq!(
        refused.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        refused.data
    );
    assert_eq!(refused.outcome, Outcome::Partial);
    assert_eq!(refused.effects.checkpoint, CheckpointEffect::Committed);
    assert_eq!(refused.effects.integration, IntegrationEffect::NotRequested);
    assert_eq!(
        actions(&refused),
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    let data = lifecycle_of(&refused).ok_or(FixtureError)?;
    assert!(
        data.blockers
            .contains(&LifecycleBlocker::IntegrationConflict),
        "{:?}",
        data.blockers
    );
    assert_eq!(fixed(w.repo()?.refname_to_id(MAIN))?, main);
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    golden_at(
        &w.root,
        "ticket_close_checkpoint_merge_refused",
        Some(MUTATION),
        &refused,
    );
    Ok(())
}

/// A conflict the context's synchronization stops on, before any durable
/// effect: `recovery_required`, naming the state, with the record kept.
fn child_conflict() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let w = remote(verb)?;
    fixed(w.sync(w.context_request()))?;
    let path = format!(".manyhands/tickets/{ITEM}/ticket.md");
    let peer = fixed(w.peer.workdir().ok_or(FixtureError).map(Path::to_owned))?;
    let mut fetch = git2::FetchOptions::new();
    fetch.remote_callbacks(callbacks(&w.server));
    fixed(fixed(w.peer.find_remote("origin"))?.fetch(
        &[format!("+{0}:{0}", verb.branch())],
        Some(&mut fetch),
        None,
    ))?;
    commit_in(&peer, verb.branch(), &path, b"the peer's\n")?;
    push_peer(&w.peer, &w.server, verb.branch())?;
    let worktree = verb.worktree(&w.root);
    let source = fixed(std::fs::read_to_string(worktree.join(&path)))?;
    commit_in(
        &worktree,
        verb.branch(),
        &path,
        source.replacen("task", "bug", 1).as_bytes(),
    )?;
    let confirmation = confirmation_of(&prepared(&w.service, verb, &w.root, verb.id()));
    let stopped = called(
        &w.service,
        FIRST,
        Some(&confirmation),
        verb.mutation(&w.root, verb.id()),
    );
    assert_eq!(
        stopped.code,
        ResultCode::RecoveryRequired,
        "{:?}",
        stopped.data
    );
    assert_ne!(stopped.outcome, Outcome::Partial);
    assert_eq!(
        actions(&stopped)[..2],
        [
            RecoveryActionKind::RequestRetry,
            RecoveryActionKind::RepoInspect
        ]
    );
    let data = lifecycle_of(&stopped).ok_or(FixtureError)?;
    assert!(!data.blockers.is_empty());
    assert_eq!(state_of(&w, FIRST), Some(RequestState::Accepted));
    Ok(())
}

/// A comment whose publication is pending blocks a closure's prepare.
fn comment_publication_pending() -> Result<(), FixtureError> {
    let verb = Verb::Close;
    let w = remote(verb)?;
    w.server.disconnect_at(FixtureBoundary::Advertisement);
    let saved = fixed(w.service.submit_comment(
        PublishCommentRequest {
            comment: SubmitCommentRequest {
                target: AuthoringTarget {
                    root: w.root.clone(),
                    kind: AuthoringKind::Ticket,
                    item_id: fixed(ITEM.parse())?,
                    intent: ContextIntent::Edit,
                    operation_id: OperationId::new(),
                },
                comment_id: fixed("01CRZ3NDEKTSV4RRFFQ69G5FAV".parse())?,
                parent_id: None,
                body: "a comment\n".into(),
                expected_destination: ExpectedPathObservation::Missing,
            },
            approval: None,
            confirmed_identity: None,
        },
        &mut SessionCredentials::new(Provider),
    ))?;
    w.server.clear_fault();
    assert!(matches!(saved, CommentSubmissionOutcome::Saved { .. }));
    let preview = prepared(&w.service, verb, &w.root, verb.id());
    assert_eq!(preview.code, ResultCode::RecoveryRequired);
    assert_eq!(
        blockers_of(&preview),
        [LifecycleBlocker::CommentPublicationPending]
    );
    assert_eq!(prepare_actions(&preview), [RecoveryActionKind::RepoInspect]);
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
