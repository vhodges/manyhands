//! A re-target overtaken by its old comment (decision 27): Edit text or
//! Post as a new comment moved the composer from comment X to Y, and X
//! was sent again since, from outside this app (the operations panel
//! offers no re-send of an unsaved post, decision 26). The edited text is
//! never put under X's ID and never discarded: Y stays as it is.
//!
//! - X in flight: Y waits, editable, and says so; Submit checks again.
//! - X saved: Y's text the same as X's, byte for byte, is X: Y's draft
//!   retires and the composer clears. Other text says "Your earlier text
//!   was posted meanwhile", with Post as a new comment (sends Y), Copy
//!   text and Discard.
//! - X answered and not saved: Y says so once; the next Submit sends Y.

use gpui_kit::{Context, EntityId, Window};
use manyhands::canonical::ItemId;

use super::SlotDraft;
use crate::desktop::{
    model::publication::CommentOutcome,
    services::{requests::Intent, submissions::SubmissionState, worker::SendMark},
    view::item::{
        ItemPane,
        composer::{ComposerState, FailedActions},
    },
};

const STILL_POSTING: &str = "Your earlier text is still being posted.";
const SENT_AGAIN: &str = "Your earlier text was sent again and not saved. Post to send this text.";
const POSTED_MEANWHILE: &str = "Your earlier text was posted meanwhile.";

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

    /// Whether sending `composer`'s new comment, re-targeted from `old`
    /// at `mark`, with `body`, is held back; it then says why, or clears.
    pub(super) fn overtaken(
        &mut self,
        composer: EntityId,
        old: &ItemId,
        mark: Option<SendMark>,
        body: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let root = self.post_root();
        let submissions = self.services.submissions().read(cx);
        let in_flight = submissions.in_flight(&root, old);
        let post = submissions.get(&root, old);
        let saved = post.is_some_and(|post| {
            matches!(&post.state, SubmissionState::Finished { result, .. }
                if result.outcome.is_saved()
                    && !matches!(result.outcome, CommentOutcome::Mismatched { .. }))
        });
        let same = post.is_some_and(|post| {
            matches!(
                self.services.operations().read(cx).request(post.request).map(|request| &request.intent),
                Some(Intent::CommentAdd { body: posted, .. }) if posted.text() == body
            )
        });
        let moved = self.attempt_mark(old, cx) != mark;
        let Some((_, slot)) = self.composers.find(composer) else {
            return true;
        };
        if saved {
            slot.post.from = None;
            slot.post.from_mark = None;
            slot.post.busy = false;
            if same {
                // X is this text: Y's draft goes, and the composer clears.
                if let SlotDraft::Begun(draft) = slot.draft {
                    self.services
                        .drafts()
                        .update(cx, |registry, cx| registry.retire_comment(draft, cx))
                        .detach();
                }
                self.clear_slot(composer, window, cx);
            } else {
                slot.post.send_as_is = true;
                let state = ComposerState::Failed {
                    message: POSTED_MEANWHILE.to_owned(),
                    actions: FailedActions {
                        post_as_new: true,
                        copy_text: true,
                        discard: true,
                        ..Default::default()
                    },
                    discard_note: None,
                };
                self.set_composer(composer, state, window, cx);
            }
            return true;
        }
        let note = if in_flight {
            STILL_POSTING
        } else if moved && !slot.post.noted {
            slot.post.noted = true;
            SENT_AGAIN
        } else {
            return false;
        };
        slot.post.busy = false;
        slot.composer.clone().update(cx, |composer, cx| {
            composer.set_state(ComposerState::Idle, window, cx);
            composer.say_refused(note, cx);
        });
        true
    }
}
