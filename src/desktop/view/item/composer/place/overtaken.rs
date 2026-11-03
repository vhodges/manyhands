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
//! - X answered: when its answer is final and not saved (the rows that
//!   offer Edit text) Y says so once, and the next Submit sends Y; any
//!   other answer holds Y, "Your earlier text may still be saved", with
//!   Copy text, Discard and Check again, which settles it from X's record.

use gpui_kit::{Context, EntityId, Window};
use manyhands::canonical::ItemId;

use manyhands::results::ResultCode;

use super::{SlotDraft, post::stored_body};
use crate::desktop::{
    model::publication::{
        CommentOffer, CommentOutcome,
        record::{Checked, RecordRead, check_again, envelope_of_record},
    },
    services::{requests::Intent, submissions::SubmissionState, worker::SendMark},
    view::item::{
        ItemPane,
        composer::{ComposerState, FailedActions},
    },
};

const STILL_POSTING: &str = "Your earlier text is still being posted.";
const SENT_AGAIN: &str = "Your earlier text was sent again and not saved. Post to send this text.";
const POSTED_MEANWHILE: &str = "Your earlier text was posted meanwhile.";
const MAY_STILL_BE_SAVED_EARLIER: &str = "Your earlier text may still be saved.";

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
    #[allow(clippy::too_many_arguments)]
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
        let finished = post.and_then(|post| match &post.state {
            SubmissionState::Finished { result, .. } => Some(result.clone()),
            SubmissionState::Posting => None,
        });
        let saved = finished.as_ref().is_some_and(|result| {
            result.outcome.is_saved()
                && !matches!(result.outcome, CommentOutcome::Mismatched { .. })
        });
        // Final and not saved: the rows that offer Edit text.
        let final_unsaved = finished
            .as_ref()
            .is_some_and(|result| result.offers().contains(&CommentOffer::EditText));
        let same = self.posted_text_is(old, body, cx);
        let moved = self.attempt_mark(old, cx) != mark;
        let Some((_, slot)) = self.composers.find(composer) else {
            return true;
        };
        slot.post.busy = false;
        if saved {
            self.old_saved_meanwhile(composer, same, window, cx);
            return true;
        }
        if in_flight {
            slot.composer.clone().update(cx, |composer, cx| {
                composer.set_state(ComposerState::Idle, window, cx);
                composer.say_refused(STILL_POSTING, cx);
            });
            return true;
        }
        if !moved && !slot.post.held {
            return false;
        }
        if !final_unsaved {
            // It may still come to be saved: Y waits for Check again.
            slot.post.held = true;
            self.set_composer(composer, held_state(), window, cx);
            return true;
        }
        if slot.post.noted {
            return false;
        }
        slot.post.noted = true;
        slot.composer.clone().update(cx, |composer, cx| {
            composer.set_state(ComposerState::Idle, window, cx);
            composer.say_refused(SENT_AGAIN, cx);
        });
        true
    }

    /// Whether the text `old` was posted with is `body`, byte for byte.
    fn posted_text_is(&self, old: &ItemId, body: &str, cx: &Context<Self>) -> bool {
        let root = self.post_root();
        self.services
            .submissions()
            .read(cx)
            .get(&root, old)
            .is_some_and(|post| {
                matches!(
                    self.services.operations().read(cx).request(post.request).map(|request| &request.intent),
                    Some(Intent::CommentAdd { body: posted, .. }) if posted.text() == body
                )
            })
    }

    /// The old comment was saved meanwhile: the same text clears the
    /// composer, retiring the new draft; other text stays, posted only as
    /// a new comment by choice.
    pub(super) fn old_saved_meanwhile(
        &mut self,
        composer: EntityId,
        same: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        slot.post.from = None;
        slot.post.from_mark = None;
        slot.post.held = false;
        slot.post.busy = false;
        if same {
            if let SlotDraft::Begun(draft) = slot.draft {
                self.services
                    .drafts()
                    .update(cx, |registry, cx| registry.retire_comment(draft, cx))
                    .detach();
            }
            self.clear_slot(composer, window, cx);
            return;
        }
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

    /// Check again on a composer held for its old comment `old`: the old
    /// comment's record and listing settle it (change 2, R-19). Saved, or
    /// listed, it is the saved-meanwhile case; final with nothing of it
    /// written, the new text may be sent; otherwise it stays held.
    pub(super) fn check_overtaken(
        &mut self,
        composer: EntityId,
        old: ItemId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        let drafts = self.services.drafts().clone();
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        if slot.post.busy {
            return;
        }
        slot.post.busy = true;
        let text = slot.composer.read(cx).text(cx);
        let draft = match slot.draft {
            SlotDraft::Begun(id) => Some(id),
            _ => None,
        };
        let last = self
            .services
            .submissions()
            .read(cx)
            .get(&root, &old)
            .and_then(|post| match &post.state {
                SubmissionState::Finished { envelope, .. } => Some(envelope.code),
                SubmissionState::Posting => None,
            })
            .unwrap_or(ResultCode::Ok);
        self.set_composer(composer, ComposerState::Checking, window, cx);
        let look = self.look(old.clone(), true, cx);
        cx.spawn_in(window, async move |pane, cx| {
            let found = look.await;
            let body = match draft {
                Some(id) => stored_body(&drafts, id, cx).await.unwrap_or(text),
                None => text,
            };
            let record = found.record.unwrap_or(RecordRead::Unknown);
            let checked = check_again(&record, found.listing, last);
            let _ = pane.update_in(cx, |pane, window, cx| {
                if let Some((_, slot)) = pane.composers.find(composer) {
                    slot.post.busy = false;
                }
                match checked {
                    Checked::Saved(result) => {
                        if let RecordRead::Finished(stored) = &record {
                            let envelope = Box::new(envelope_of_record(stored));
                            pane.services
                                .submissions()
                                .update(cx, |posts, _| posts.settle(&root, &old, envelope, result));
                        }
                        let same = pane.posted_text_is(&old, &body, cx);
                        pane.old_saved_meanwhile(composer, same, window, cx);
                    }
                    Checked::Listed => {
                        let same = found.listed_body.as_deref() == Some(body.as_str());
                        pane.old_saved_meanwhile(composer, same, window, cx);
                    }
                    Checked::Final {
                        post_as_new: true, ..
                    }
                    | Checked::Maybe { post_as_new: true }
                        if !pane.services.submissions().read(cx).in_flight(&root, &old) =>
                    {
                        // Nothing of the old comment exists, and none of
                        // it is on its way: the new text may be sent.
                        let mark = pane.attempt_mark(&old, cx);
                        if let Some((_, slot)) = pane.composers.find(composer) {
                            slot.post.held = false;
                            slot.post.noted = true;
                            slot.post.from_mark = mark;
                        }
                        pane.set_composer(composer, ComposerState::Idle, window, cx);
                    }
                    _ => pane.set_composer(composer, held_state(), window, cx),
                }
            });
        })
        .detach();
    }
}

/// A new comment held while its old one may still be saved.
fn held_state() -> ComposerState {
    ComposerState::Failed {
        message: MAY_STILL_BE_SAVED_EARLIER.to_owned(),
        actions: FailedActions {
            check_again: true,
            copy_text: true,
            discard: true,
            ..Default::default()
        },
        discard_note: None,
    }
}
