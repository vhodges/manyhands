//! A re-target overtaken by its old comment (R-19, Task 13 re-review):
//! Edit text or Post as a new comment moved the composer from comment X
//! to Y, but X was sent again since, from the operation area say. Y is
//! then never sent: X's answer stands on the composer. Saved, Y's draft
//! goes and the composer clears (X's draft is already retired); not
//! saved, the text goes back to X, with a draft of its own again.

use gpui_kit::{Context, EntityId, Window};
use manyhands::{canonical::ItemId, drafts::CommentTarget};

use super::SlotDraft;
use crate::desktop::{
    model::publication::CommentOutcome,
    services::{submissions::SubmissionState, worker::SendMark},
    view::item::{ItemPane, composer::ComposerState},
};

impl ItemPane {
    /// The last attempt of this session's post of `comment`, if any.
    pub(super) fn attempt_mark(&self, comment: &ItemId, cx: &Context<Self>) -> Option<SendMark> {
        let request = self
            .services
            .submissions()
            .read(cx)
            .get(&self.post_root(), comment)?
            .request;
        self.services.worker().sent_mark(request)
    }

    /// Whether `old`, the comment a composer was re-targeted from, was
    /// sent again since `mark`, is in flight, or is saved.
    pub(super) fn overtaken(
        &self,
        old: &ItemId,
        mark: Option<SendMark>,
        cx: &Context<Self>,
    ) -> bool {
        let root = self.post_root();
        let submissions = self.services.submissions().read(cx);
        let saved = submissions.get(&root, old).is_some_and(|post| {
            matches!(&post.state, SubmissionState::Finished { result, .. }
                if result.outcome.is_saved()
                    && !matches!(result.outcome, CommentOutcome::Mismatched { .. }))
        });
        submissions.in_flight(&root, old) || saved || self.attempt_mark(old, cx) != mark
    }

    /// Undoes the re-target of `composer` to its old comment `old`: the
    /// new comment's draft is discarded, never posted. A saved `old`
    /// clears the composer; otherwise it follows `old`'s post, whose
    /// answer it then shows, with a draft of `old` begun again.
    pub(super) fn undo_retarget(
        &mut self,
        composer: EntityId,
        old: ItemId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.item_key() else {
            return;
        };
        let drafts = self.services.drafts().clone();
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        if let SlotDraft::Begun(new) = slot.draft {
            drafts.update(cx, |registry, cx| registry.discard_comment(new, cx));
        }
        let parent_id = slot
            .target
            .as_ref()
            .and_then(|target| target.parent_id.clone());
        let target = CommentTarget {
            comment_id: old.clone(),
            parent_id,
        };
        slot.target = Some(target.clone());
        slot.draft = SlotDraft::None;
        slot.post.from = None;
        slot.post.from_mark = None;
        slot.post.busy = false;
        slot.post.sent = true;
        let text = slot.composer.read(cx).text(cx);
        let root = self.post_root();
        let saved = self
            .services
            .submissions()
            .read(cx)
            .get(&root, &old)
            .is_some_and(|post| {
                matches!(&post.state, SubmissionState::Finished { result, .. }
                    if result.outcome.is_saved()
                        && !matches!(result.outcome, CommentOutcome::Mismatched { .. }))
            });
        if saved {
            self.clear_slot(composer, window, cx);
            return;
        }
        // Followed: `apply_posts` shows its answer, or keeps it posting.
        self.set_composer(composer, ComposerState::Posting, window, cx);
        let begin = drafts.update(cx, |registry, cx| {
            registry.begin_comment(item, target, text, cx)
        });
        cx.spawn(async move |pane, cx| {
            let Ok(draft) = begin.await else {
                return;
            };
            let _ = pane.update(cx, |pane, cx| {
                pane.composers.known.insert(draft);
                match pane.composers.find(composer) {
                    Some((_, slot))
                        if slot.target.as_ref().map(|target| &target.comment_id) == Some(&old)
                            && matches!(slot.draft, SlotDraft::None) =>
                    {
                        slot.draft = SlotDraft::Begun(draft);
                    }
                    _ => pane
                        .services
                        .drafts()
                        .update(cx, |registry, cx| registry.discard_comment(draft, cx)),
                }
            });
        })
        .detach();
        cx.notify();
    }
}
