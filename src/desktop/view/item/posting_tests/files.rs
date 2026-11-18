//! The posting tests' repository files, request records, stored drafts
//! and constructed answers (`support`'s, re-exported there).

use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui_kit::{Entity, TestAppContext};
use manyhands::{
    canonical::ItemId,
    drafts::{CommentTarget, DraftFields, DraftId, DraftTarget, ItemKind},
    repository::{
        AuthoringKind, CancellationToken, CommentAddInput, CommentMutationDto, CommentMutationType,
        CommentPendingReason, ConflictNextAction, ConflictNextDto, Mutation, MutationCall,
        MutationDataDto, NoProgress, RequestId, keys::SessionCredentials,
    },
    results::{
        CheckpointEffect, Effects, Envelope, PublicationEffect, ResultCode, Scope, WriteEffect,
    },
};

use super::super::super::discussion_tests::{TICKET, new_composer, pane};
use crate::desktop::{
    model::publication::CommentOutcome,
    test_support::fixtures::{Fixture, NoPassphrase, item_id},
    view::{
        harness::{Shell, key},
        item::composer::Composer,
    },
};

use super::support::{KEPT, focus_end, outcome, submit, until_answered, until_post_runs};

/// The item's editing context, where its comments are written.
pub(super) fn context(root: &Path) -> PathBuf {
    root.join(".manyhands/worktrees").join(TICKET)
}

/// The comment files of `TICKET` in its editing context, by name.
pub(super) fn comment_files(root: &Path) -> Vec<String> {
    let dir = context(root).join(".manyhands/comments").join(TICKET);
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// The text of the comment file of `comment` in the editing context.
pub(super) fn comment_source(root: &Path, comment: &ItemId) -> Option<String> {
    fs::read_to_string(
        context(root)
            .join(".manyhands/comments")
            .join(TICKET)
            .join(format!("{comment}.md")),
    )
    .ok()
}

/// How many comment checkpoints the editing context's branch holds.
pub(super) fn checkpoints(root: &Path) -> usize {
    let Ok(repository) = git2::Repository::open(context(root)) else {
        return 0;
    };
    let mut walk = repository.revwalk().unwrap();
    walk.push_head().unwrap();
    walk.filter(|oid| {
        repository
            .find_commit(*oid.as_ref().unwrap())
            .unwrap()
            .summary()
            .is_some_and(|summary| summary.starts_with("Checkpoint comment "))
    })
    .count()
}

/// The paths the editing context's newest comment checkpoint changed.
pub(super) fn checkpoint_paths(root: &Path) -> Vec<String> {
    let repository = git2::Repository::open(context(root)).unwrap();
    let head = repository.head().unwrap().peel_to_commit().unwrap();
    assert!(
        head.summary()
            .is_some_and(|summary| summary.starts_with("Checkpoint comment ")),
        "the context's head is a comment checkpoint"
    );
    let parent = head.parent(0).unwrap().tree().unwrap();
    let diff = repository
        .diff_tree_to_tree(Some(&parent), Some(&head.tree().unwrap()), None)
        .unwrap();
    diff.deltas()
        .filter_map(|delta| delta.new_file().path()?.to_str().map(str::to_owned))
        .collect()
}

/// Stores a comment draft of `TICKET` in `root` and lets go of its claim,
/// as a session that ended would.
pub(super) fn store_comment(
    fixture: &Fixture,
    root: &Path,
    comment: &str,
    parent: Option<&str>,
    body: &str,
) -> DraftId {
    let target = DraftTarget {
        repository: key(root),
        item_kind: ItemKind::Ticket,
        item_id: item_id(TICKET),
        comment: Some(CommentTarget {
            comment_id: item_id(comment),
            parent_id: parent.map(item_id),
        }),
    };
    match fixture.draft_store().create(
        target,
        None,
        DraftFields::Comment {
            body: body.to_owned(),
        },
    ) {
        Ok(claim) => claim.draft().id,
        Err(error) => panic!("the draft must be stored: {error}"),
    }
}

/// Posts `body` as the root comment `comment` of `TICKET` through the
/// fixture's own service, under the comment ID as its request ID (R-16):
/// what the session that ended had sent.
pub(super) fn post_through(
    fixture: &Fixture,
    root: &Path,
    comment: &str,
    body: &str,
) -> Envelope<MutationDataDto> {
    fixture.service().execute(
        MutationCall {
            request_id: RequestId::parse(comment).unwrap(),
            confirmation: None,
            mutation: Mutation::CommentAdd(CommentAddInput {
                root: root.to_owned(),
                kind: AuthoringKind::Ticket,
                id: item_id(TICKET),
                comment_id: item_id(comment),
                parent_id: None,
                body: body.to_owned(),
                created_by: None,
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    )
}

/// Waits until the new comment's composer holds `text`, an adopted draft,
/// and its record has been looked at.
pub(super) async fn until_adopted(
    shell: &Shell,
    text: &str,
    cx: &mut TestAppContext,
) -> Entity<Composer> {
    let wanted = text.to_owned();
    shell
        .until_drawn(cx, "the draft adopted", move |shell, cx| {
            pane(shell, cx)
                .read_with(cx, |pane, _| pane.new_composer())
                .is_some_and(|composer| {
                    composer.read_with(cx, |composer, cx| {
                        composer.text(cx) == wanted && !composer.is_checking()
                    })
                })
        })
        .await;
    new_composer(shell, cx)
}

fn effects(write: WriteEffect, checkpoint: CheckpointEffect) -> Effects {
    Effects {
        write,
        checkpoint,
        ..Effects::not_requested()
    }
}

/// A post's constructed answer: `code` with `effects`.
fn answer(code: ResultCode, effects: Effects) -> Envelope<MutationDataDto> {
    Envelope::classified_mutation(
        "comment add",
        Scope::default(),
        code == ResultCode::Cancelled,
        code,
        effects,
        false,
    )
}

/// Row 1: written, committed and published.
pub(super) fn shared(comment: &ItemId) -> Envelope<MutationDataDto> {
    let mut effects = effects(WriteEffect::Written, CheckpointEffect::Committed);
    effects.publication = PublicationEffect::Published;
    answer(ResultCode::Ok, effects).with_data(MutationDataDto::Comment(CommentMutationDto {
        r#type: CommentMutationType::Comment,
        item_id: TICKET.to_owned(),
        comment_id: comment.to_string(),
        parent_id: None,
        created_by: None,
        synchronization_id: "01M4CC0VN1KTQFSVR99378TDHC".to_owned(),
        publication: PublicationEffect::Published,
        pending_reason: None::<CommentPendingReason>,
        conflict: None,
    }))
}

/// Row 4: written and committed, its synchronization stopped on a
/// conflict.
pub(super) fn saved_conflict(comment: &ItemId) -> Envelope<MutationDataDto> {
    let mut effects = effects(WriteEffect::Written, CheckpointEffect::Committed);
    effects.publication = PublicationEffect::Pending;
    answer(ResultCode::RecoveryRequired, effects).with_data(MutationDataDto::Comment(
        CommentMutationDto {
            r#type: CommentMutationType::Comment,
            item_id: TICKET.to_owned(),
            comment_id: comment.to_string(),
            parent_id: None,
            created_by: None,
            synchronization_id: "01M4CC0VN1KTQFSVR99378TDHC".to_owned(),
            publication: PublicationEffect::Pending,
            pending_reason: Some(CommentPendingReason::Synchronization),
            conflict: Some(ConflictNextDto {
                operation_id: "01M4CC0VN1KTQFSVR99378TDHV".to_owned(),
                next_action: ConflictNextAction::ALL[0],
            }),
        },
    ))
}

/// Row 6: written and committed, its publication needs the key unlocked.
pub(super) fn saved_needs_unlock() -> Envelope<MutationDataDto> {
    let mut effects = effects(WriteEffect::Written, CheckpointEffect::Committed);
    effects.publication = PublicationEffect::Pending;
    answer(ResultCode::UnlockRequired, effects)
}

/// Nothing written or committed, answered `code`.
pub(super) fn not_saved(code: ResultCode) -> Envelope<MutationDataDto> {
    answer(
        code,
        effects(WriteEffect::NotRequested, CheckpointEffect::NotRequested),
    )
}

/// The comment's file written, its checkpoint not committed, answered
/// `code`.
pub(super) fn written(code: ResultCode) -> Envelope<MutationDataDto> {
    answer(
        code,
        effects(WriteEffect::Written, CheckpointEffect::NotRequested),
    )
}

/// `internal_error` with nothing written: what a re-sent comment ID whose
/// binding is found but not its record answers (P15).
pub(super) fn internal_error() -> Envelope<MutationDataDto> {
    Envelope::failure(
        "comment add",
        Scope::default(),
        ResultCode::InternalError,
        vec![],
    )
}

/// The draft this session's post of `comment` still has to retire.
pub(super) fn post_draft(
    shell: &Shell,
    root: &Path,
    comment: &ItemId,
    cx: &mut TestAppContext,
) -> Option<Option<DraftId>> {
    let submissions = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().submissions().clone()
    });
    submissions.read_with(cx, |submissions, _| {
        submissions.get(root, comment).map(|post| post.draft)
    })
}

/// The adopted draft `KEPT`, stored by the build as `body`, posted and
/// stopped before its acceptance, so no record holds its ID; its re-send
/// answers `internal_error`, which may be saved (P15, P16).
pub(super) async fn maybe_saved_without_record(
    shell: &Shell,
    root: &Path,
    body: &str,
    cx: &mut TestAppContext,
) -> Entity<Composer> {
    let comment = item_id(KEPT);
    let composer = until_adopted(shell, body, cx).await;
    let held = shell
        .backend
        .hold_job(crate::desktop::services::JobKind::Comment);
    let returned = shell
        .backend
        .hold_job_return_answering(crate::desktop::services::JobKind::Comment, internal_error());
    focus_end(shell, &composer, cx);
    submit(shell, cx);
    let id = until_post_runs(shell, cx).await;
    held.wait_reached();
    let worker = shell
        .workspace
        .read_with(cx, |workspace, _| workspace.services().worker().clone());
    assert_eq!(
        worker.cancel(id),
        crate::desktop::services::worker::CancelResult::Signalled
    );
    held.release();
    returned.release();
    until_answered(shell, root, &comment, cx).await;
    assert_eq!(
        outcome(shell, root, &comment, cx).map(|(outcome, ..)| outcome),
        Some(CommentOutcome::MaybeSaved)
    );
    composer
}
