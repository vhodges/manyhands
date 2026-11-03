//! What the tab settles of a post after its answer (design, P12, P17;
//! changes 1 and 6): a saved post's draft retired once, a mismatch said
//! saved only by its record, a saved post's unknown record read again, and
//! the posts being sent, drawn pending until their outcomes.

use gpui_kit::{Context, EntityId, Window};
use manyhands::{canonical::ItemId, drafts::DraftId};

use super::{ComposerState, post::failed_state};
use crate::desktop::{
    model::publication::{
        Attempt, CommentOutcome, comment_outcome,
        record::{RecordRead, comment_outcome_of_record, envelope_of_record, mismatch_saved},
    },
    services::{submissions::SubmissionState, worker::RecordState},
    view::item::ItemPane,
};

impl ItemPane {
    /// The posts of `thread`'s composers being sent, with their texts:
    /// drawn pending until their outcomes.
    pub(in crate::desktop::view::item) fn pending_posts(
        &self,
        thread: Option<&str>,
        cx: &gpui_kit::App,
    ) -> Vec<(ItemId, String)> {
        let shown = self
            .composers
            .new
            .iter()
            .map(|slot| (None, slot))
            .chain(
                self.composers
                    .reply
                    .iter()
                    .map(|(root, slot)| (Some(root.as_str()), slot)),
            )
            .chain(
                self.composers
                    .kept
                    .iter()
                    .map(|(root, slot)| (Some(root.as_str()), slot)),
            );
        shown
            .filter(|(at, _)| *at == thread)
            .filter_map(|(_, slot)| {
                let composer = slot.composer.read(cx);
                if !matches!(composer.state, ComposerState::Posting) {
                    return None;
                }
                Some((slot.target.as_ref()?.comment_id.clone(), composer.text(cx)))
            })
            .collect()
    }

    /// Retires the draft of a saved post of this tab's, and tells
    /// `Submissions` once it is gone (change 6).
    pub(super) fn retire_post_draft(
        &mut self,
        comment: ItemId,
        draft: DraftId,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        let submissions = self.services.submissions().clone();
        let retiring = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.retire_comment(draft, cx));
        cx.spawn(async move |_, cx| {
            if matches!(
                retiring.await,
                Ok(()) | Err(manyhands::drafts::DraftErrorKind::NotFound)
            ) {
                submissions.update(cx, |posts, cx| {
                    posts.retired(&root, &comment, draft);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// A mismatch whose record is finished: saved when its stored result
    /// committed the comment (change 1); the composer says which.
    pub(super) fn mismatch_record(
        &mut self,
        composer: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        let Some(comment) = self
            .composers
            .find(composer)
            .and_then(|(_, slot)| slot.target.as_ref().map(|target| target.comment_id.clone()))
        else {
            return;
        };
        let read = self.read_post(comment.clone(), false, true, cx);
        cx.spawn_in(window, async move |pane, cx| {
            let record = read.await.ok().and_then(|found| found.record);
            let _ = pane.update_in(cx, |pane, window, cx| {
                let submissions = pane.services.submissions().clone();
                let Some((envelope, mut result)) = submissions
                    .read(cx)
                    .get(&root, &comment)
                    .and_then(|post| match &post.state {
                        SubmissionState::Finished { envelope, result } => {
                            Some((envelope.clone(), result.clone()))
                        }
                        SubmissionState::Posting => None,
                    })
                else {
                    return;
                };
                let saved = mismatch_saved(result.attempt.record, false, record.as_ref());
                result.outcome = CommentOutcome::Mismatched { saved };
                let state = failed_state(&envelope, &result);
                submissions.update(cx, |posts, _| {
                    posts.settle(&root, &comment, envelope, result)
                });
                if pane
                    .composers
                    .find(composer)
                    .is_some_and(|(_, slot)| slot.composer.read(cx).is_failed())
                {
                    pane.set_composer(composer, state, window, cx);
                }
            });
        })
        .detach();
    }

    /// The chip's Check again on a saved post whose record was unknown
    /// (P17): its record classifies it afresh.
    pub(in crate::desktop::view::item) fn recheck_saved(
        &mut self,
        comment: ItemId,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        let read = self.read_post(comment.clone(), false, true, cx);
        cx.spawn(async move |pane, cx| {
            let record = read.await.ok().and_then(|found| found.record);
            let _ = pane.update(cx, |pane, cx| {
                let submissions = pane.services.submissions().clone();
                let Some((envelope, result)) =
                    submissions
                        .read(cx)
                        .get(&root, &comment)
                        .and_then(|post| match &post.state {
                            SubmissionState::Finished { envelope, result } => {
                                Some((envelope.clone(), result.clone()))
                            }
                            SubmissionState::Posting => None,
                        })
                else {
                    return;
                };
                let attempt = |record| Attempt {
                    record,
                    ..result.attempt
                };
                let (envelope, result) = match record {
                    Some(RecordRead::Finished(stored)) => (
                        Box::new(envelope_of_record(&stored)),
                        comment_outcome_of_record(&stored, attempt(RecordState::Finished)),
                    ),
                    Some(RecordRead::Accepted) => {
                        let result = comment_outcome(&envelope, attempt(RecordState::Accepted));
                        (envelope, result)
                    }
                    _ => return,
                };
                submissions.update(cx, |posts, cx| {
                    posts.settle(&root, &comment, envelope, result);
                    cx.notify();
                });
            });
        })
        .detach();
    }
}
