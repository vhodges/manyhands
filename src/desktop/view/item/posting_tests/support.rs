//! What the posting recovery tests share (design, Required outcomes of
//! posting recovery): a repository holding `TICKET` and comment `A`, with
//! or without a publication remote, in an open window; posting from a
//! composer; waiting on a post's answer through `Submissions`; the
//! editing context's checkpoints and comment files; constructed answers
//! for rows no fixture reaches; and the leftovers of a session that ended
//! (a stored comment draft, a request record of its comment ID).
//!
//! The elements Task 13 draws are named here, so the tests and the design
//! checkpoint agree on them: a post's pending row, its "not shared yet"
//! chip and the chip's controls, and the "Saved on this computer" note,
//! each keyed by the comment's ID.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
};

use gpui_kit::{Entity, TestAppContext, px, size};
use manyhands::{
    canonical::{CONFIG_PATH, ItemId},
    drafts::{CommentTarget, DraftFields, DraftId, DraftTarget, ItemKind},
    repository::{
        AuthoringKind, CancellationToken, CommentAddInput, CommentMutationDto, CommentMutationType,
        CommentPendingReason, ItemDtoKind, Mutation, MutationCall, MutationDataDto, NoProgress,
        RequestId, keys::SessionCredentials,
    },
    results::{
        CheckpointEffect, Effects, Envelope, PublicationEffect, ResultCode, Scope, WriteEffect,
    },
};

use super::super::super::discussion_tests::{
    A, ADA, TICKET, commit_comment, composer_target, flushed, focus_editor, new_composer,
    open_new_composer, pane, type_into,
};
use crate::desktop::{
    model::{navigation::OpenTarget, publication::CommentOutcome},
    services::{
        requests::{DesktopRequest, DesktopRequestId, Intent, RequestState},
        submissions::SubmissionState,
        worker::RecordState,
    },
    test_support::fixtures::{Fixture, NoPassphrase, item_id},
    view::{
        harness::{Shell, WAIT, key, wait_until},
        item::composer::Composer,
    },
};

/// The chip of a saved comment that is not shared (design, Posting).
pub(super) const CHIP: &str = "Saved on this computer \u{b7} not shared yet";
/// The note of a comment saved with no publication remote.
pub(super) const SAVED_HERE: &str = "Saved on this computer";
/// `MaybeSaved`'s words (P5).
pub(super) const MAYBE_SAVED: &str = "This comment may already be saved.";
/// What a refused Submit says (P10).
pub(super) const ALREADY_POSTING: &str = "Already posting";
/// The discard note of a post whose record is open (P7).
pub(super) const MAY_STILL_BE_SAVED: &str =
    "This comment may still be saved later if it was written";
/// Reply elsewhere while the reply is posting (design, Placement).
pub(super) const REPLY_POSTING: &str = "Finish or cancel the reply being posted";

/// A second comment ID for a session that ended.
pub(super) const KEPT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP0";
/// A reply's comment ID for a session that ended.
pub(super) const KEPT_REPLY: &str = "01ARZ3NDEKTSV4RRFFQ69G5FP1";

/// The failed composer's controls, in the order they are drawn.
const FAILED_CONTROLS: [&str; 7] = [
    "composer-retry",
    "composer-set-identity",
    "composer-check-again",
    "composer-post-as-new",
    "composer-edit-text",
    "composer-copy-text",
    "composer-discard",
];

/// Repository `a` holding `TICKET` and comment `A` by Ada, then `build`,
/// published over SSH with its selection committed when `remote`, indexed,
/// in a window with `TICKET` open. Nothing is refreshed at launch.
pub(super) fn posting_shell(
    remote: bool,
    cx: &mut TestAppContext,
    build: impl FnOnce(&Fixture, &Path),
) -> (Shell, PathBuf) {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Discussed", "");
    commit_comment(&fixture, &root, A, None, 1, Some(ADA), "First.\n");
    if remote {
        fixture.publish_over_ssh(&root);
        let config = fs::read(root.join(CONFIG_PATH)).unwrap();
        fixture.commit_file(&root, CONFIG_PATH, &config);
    }
    build(&fixture, &root);
    fixture.refresh(&root);
    let shell = Shell::open(fixture, cx);
    shell.visual(cx).simulate_resize(size(px(1440.), px(1400.)));
    shell.run(cx);
    open_ticket(&shell, &root, cx);
    (shell, root)
}

/// Opens `TICKET` of `root`, or shows its tab again.
pub(super) fn open_ticket(shell: &Shell, root: &Path, cx: &mut TestAppContext) {
    let target = OpenTarget::Item {
        repo: key(root),
        kind: ItemDtoKind::Ticket,
        id: item_id(TICKET),
    };
    shell
        .workspace
        .update(cx, |workspace, cx| workspace.open_item(target, cx));
    shell.run(cx);
    shell.frame(cx, |_, _| ());
}

/// Writes `text` in the new comment's composer, opened first, and lets
/// its draft be stored: the composer and the comment ID it allocated.
pub(super) fn write_new(
    shell: &Shell,
    text: &str,
    cx: &mut TestAppContext,
) -> (Entity<Composer>, ItemId) {
    open_new_composer(shell, cx);
    let composer = new_composer(shell, cx);
    type_into(shell, &composer, text, cx);
    flushed(shell, cx);
    let target = composer_target(shell, &composer, cx).expect("an ID on the first keystroke");
    (composer, target.comment_id)
}

/// Puts focus at the end of `composer`'s text, a frame drawn.
pub(super) fn focus_end(shell: &Shell, composer: &Entity<Composer>, cx: &mut TestAppContext) {
    let editor = composer.read_with(cx, |composer, _| composer.editor().clone());
    let end = editor.read_with(cx, |editor, _| editor.text().len());
    focus_editor(shell, &editor, end, cx);
}

/// Ctrl Enter in the focused composer: Submit.
pub(super) fn submit(shell: &Shell, cx: &mut TestAppContext) {
    shell.keys("secondary-enter", cx);
}

/// The composer's text, and whether it is read-only.
pub(super) fn composer_state(
    composer: &Entity<Composer>,
    cx: &mut TestAppContext,
) -> (String, bool) {
    composer.read_with(cx, |composer, cx| {
        (composer.text(cx), composer.is_read_only())
    })
}

/// The controls the failed composer shows.
pub(super) fn failed_actions(shell: &Shell, cx: &mut TestAppContext) -> Vec<&'static str> {
    FAILED_CONTROLS
        .into_iter()
        .filter(|id| shell.present(*id, cx))
        .collect()
}

/// The element of `part` of the comment `comment`, as Task 13 draws it.
pub(super) fn of(part: &str, comment: &ItemId) -> String {
    format!("{part}-{comment}")
}

/// A post's pending row, drawn from its text until its outcome.
pub(super) fn pending_row(comment: &ItemId) -> String {
    of("pending-comment", comment)
}

/// A saved comment's "not shared yet" chip.
pub(super) fn chip(comment: &ItemId) -> String {
    of("comment-chip", comment)
}

/// One of the chip's controls: `retry-sharing`, `unlock`, `give-up`,
/// `share`, `open-keys`, `check-host` or `set-identity`.
pub(super) fn chip_control(comment: &ItemId, control: &str) -> String {
    format!("{}-{control}", chip(comment))
}

/// The "Saved on this computer" note of a comment saved with no remote.
pub(super) fn saved_here(comment: &ItemId) -> String {
    of("comment-saved-here", comment)
}

/// How many comments the discussion lists.
pub(super) fn listed(shell: &Shell, cx: &mut TestAppContext) -> usize {
    (0..)
        .take_while(|ix: &usize| shell.present(("comment", *ix), cx))
        .count()
}

/// Waits until this session's post of `comment` has an answer, then
/// draws frames until the reads it started have landed.
pub(super) async fn until_answered(
    shell: &Shell,
    root: &Path,
    comment: &ItemId,
    cx: &mut TestAppContext,
) {
    let submissions = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().submissions().clone()
    });
    let (root, comment) = (root.to_owned(), comment.clone());
    wait_until(cx, &submissions, move |submissions, _| {
        submissions
            .get(&root, &comment)
            .is_some_and(|post| matches!(post.state, SubmissionState::Finished { .. }))
    })
    .await;
    shell.run(cx);
    shell.frame(cx, |_, _| ());
    shell.run(cx);
}

/// Waits until the `attempt`th attempt of the post of `comment` has
/// ended, and the session's post has its answer.
pub(super) async fn until_attempt(
    shell: &Shell,
    root: &Path,
    comment: &ItemId,
    attempt: u32,
    cx: &mut TestAppContext,
) {
    let id = post_request(shell, comment, cx).expect("the comment was posted");
    shell
        .until(cx, move |operations| {
            operations.request(id).is_some_and(|request| {
                request.attempt == attempt
                    && !matches!(
                        request.state,
                        RequestState::Queued | RequestState::Running { .. }
                    )
            })
        })
        .await;
    until_answered(shell, root, comment, cx).await;
}

/// The session's post of `comment` in `root`: its outcome, the record it
/// was classified under and whether it was a re-send.
pub(super) fn outcome(
    shell: &Shell,
    root: &Path,
    comment: &ItemId,
    cx: &mut TestAppContext,
) -> Option<(CommentOutcome, RecordState, bool)> {
    let submissions = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().submissions().clone()
    });
    submissions.read_with(cx, |submissions, _| {
        match &submissions.get(root, comment)?.state {
            SubmissionState::Finished { result, .. } => Some((
                result.outcome.clone(),
                result.attempt.record,
                result.attempt.resend,
            )),
            SubmissionState::Posting => None,
        }
    })
}

/// Whether this session holds a post of `comment` in `root`, whatever its
/// state: a comment ID it reserves.
pub(super) fn reserved(
    shell: &Shell,
    root: &Path,
    comment: &ItemId,
    cx: &mut TestAppContext,
) -> bool {
    let submissions = shell.workspace.read_with(cx, |workspace, _| {
        workspace.services().submissions().clone()
    });
    submissions.read_with(cx, |submissions, _| {
        submissions.get(root, comment).is_some()
    })
}

/// Whether `request` posts the comment `comment`.
fn posts(request: &DesktopRequest, comment: &ItemId) -> bool {
    matches!(&request.intent, Intent::CommentAdd { comment_id, .. } if comment_id == comment)
}

/// The worker request posting `comment`, newest first.
pub(super) fn post_request(
    shell: &Shell,
    comment: &ItemId,
    cx: &mut TestAppContext,
) -> Option<DesktopRequestId> {
    let comment = comment.clone();
    shell
        .requests(cx, move |request| posts(request, &comment))
        .into_iter()
        .next()
}

/// Every post of this session: its comment ID, library request ID and
/// attempt, oldest first.
pub(super) fn comment_posts(
    shell: &Shell,
    cx: &mut TestAppContext,
) -> Vec<(ItemId, RequestId, u32)> {
    let operations = shell.operations(cx);
    operations.read_with(cx, |operations, _| {
        let mut posts: Vec<_> = operations
            .requests()
            .iter()
            .filter_map(|request| match &request.intent {
                Intent::CommentAdd { comment_id, .. } => Some((
                    request.id,
                    comment_id.clone(),
                    request.request_id,
                    request.attempt,
                )),
                _ => None,
            })
            .collect();
        posts.sort_by_key(|(id, ..)| *id);
        posts
            .into_iter()
            .map(|(_, comment, request, attempt)| (comment, request, attempt))
            .collect()
    })
}

/// Whether any request of this session shares or synchronizes.
pub(super) fn shares(shell: &Shell, cx: &mut TestAppContext) -> usize {
    shell
        .requests(cx, |request| {
            matches!(
                request.intent,
                Intent::ItemSync { .. } | Intent::RepoSync { .. }
            )
        })
        .len()
}

/// Waits until the worker runs a post: Started, before its library call.
pub(super) async fn until_post_runs(shell: &Shell, cx: &mut TestAppContext) -> DesktopRequestId {
    shell
        .until(cx, |operations| {
            operations.requests().iter().any(|request| {
                matches!(request.intent, Intent::CommentAdd { .. })
                    && matches!(request.state, RequestState::Running { .. })
            })
        })
        .await;
    shell.requests(cx, |request| {
        matches!(request.intent, Intent::CommentAdd { .. })
            && matches!(request.state, RequestState::Running { .. })
    })[0]
}

/// Holds the next request after its acceptance, before its domain call
/// (fact 16): the receiver hears it is held, and dropping the sender lets
/// it go on.
pub(super) fn hold_after_acceptance(shell: &Shell) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (reached_tx, reached) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    shell
        .fixture
        .service()
        .set_request_hook_for_testing(move || {
            let _ = reached_tx.send(());
            let _ = released.recv_timeout(WAIT);
        });
    (reached, release)
}

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

/// Waits until the new comment's composer holds `text`: an adopted draft.
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
                    composer.read_with(cx, |composer, cx| composer.text(cx)) == wanted
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
