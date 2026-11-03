//! A failed composer's actions (design, P7, P11; R-3; R-19): Retry, Edit
//! text, Post as a new comment, Check again and Set identity; and the
//! refusal of those that would move or check a post while it is in
//! flight, sent from anywhere.

use gpui_kit::{Context, EntityId, Window};
use manyhands::results::ResultCode;

use super::retarget::After;
use crate::desktop::{
    services::submissions::SubmissionState,
    view::item::{
        ItemPane, ItemRoute,
        composer::{ComposerState, FailedAction},
    },
};

impl ItemPane {
    /// Whether the post of `composer`'s comment is in flight: it is then
    /// followed, and Check again, a re-target and Post as a new comment
    /// refuse (R-19). Says so in the composer.
    pub(super) fn refused_in_flight(
        &mut self,
        composer: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let root = self.post_root();
        let submissions = self.services.submissions().clone();
        let Some((_, slot)) = self.composers.find(composer) else {
            return true;
        };
        let Some(target) = &slot.target else {
            return false;
        };
        if !submissions.read(cx).in_flight(&root, &target.comment_id) {
            return false;
        }
        slot.composer.clone().update(cx, |composer, cx| {
            composer.set_state(ComposerState::Posting, window, cx);
            composer.refuse_while_posting(cx);
        });
        true
    }

    /// An action of a failed composer.
    pub(super) fn composer_failed_action(
        &mut self,
        composer: EntityId,
        action: FailedAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            FailedAction::Retry => self.retry_post(composer, window, cx),
            FailedAction::EditText => self.retarget(composer, After::Edit, window, cx),
            FailedAction::PostAsNew => {
                // After decision 27's "posted meanwhile", the text is
                // already under a new ID of its own: it is sent as it is.
                let as_is = self
                    .composers
                    .find(composer)
                    .is_some_and(|(_, slot)| std::mem::take(&mut slot.post.send_as_is));
                if as_is {
                    self.start_post(composer, false, window, cx);
                } else {
                    self.retarget(composer, After::Post, window, cx);
                }
            }
            FailedAction::CheckAgain => self.check_again(composer, window, cx),
            FailedAction::SetIdentity => cx.emit(ItemRoute::SetIdentity(self.post_root())),
        }
    }

    /// Retry: the same request. An adopted draft that was submitted sends
    /// its stored body again (P11); a post of this session is sent again
    /// by `Submissions`, its declined key cleared first after an unlock
    /// code (R-3).
    fn retry_post(&mut self, composer: EntityId, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.post_root();
        let worker = self.services.worker().clone();
        let submissions = self.services.submissions().clone();
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        if slot.post.busy {
            return;
        }
        let Some(target) = slot.target.clone() else {
            return;
        };
        let post = submissions
            .read(cx)
            .get(&root, &target.comment_id)
            .map(|post| {
                let unlock = matches!(
                    &post.state,
                    SubmissionState::Finished { envelope, .. }
                        if matches!(envelope.code, ResultCode::UnlockRequired | ResultCode::UnlockFailed)
                );
                (post.request, unlock)
            });
        let retried = match post {
            Some((request, unlock)) if !slot.post.resend_pending => {
                if unlock {
                    worker.clear_declined_for(request);
                }
                submissions.update(cx, |posts, _| posts.retry(&root, &target.comment_id))
            }
            _ => false,
        };
        if retried {
            slot.post.sent = true;
            let entity = slot.composer.clone();
            entity.update(cx, |composer, cx| {
                composer.set_state(ComposerState::Posting, window, cx)
            });
        } else if submissions.read(cx).in_flight(&root, &target.comment_id) {
            // Sent from elsewhere meanwhile: followed, its stale answer
            // never applied.
            self.refused_in_flight(composer, window, cx);
        } else {
            slot.post.sent = true;
            self.start_post(composer, false, window, cx);
        }
    }
}
