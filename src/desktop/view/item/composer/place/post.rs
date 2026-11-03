//! What a composer posts and how its outcome applies (design, Lane D,
//! Posting; P1-P18).
//!
//! - **Submit** sets the composer posting at once. The draft is flushed
//!   and its stored body read, and that body, byte for byte, is what is
//!   posted (R-16, change 9). An adopted draft with no record is posted
//!   after a refreshed listing (P4, `check`). The comment ID is the
//!   request ID; a comment sent before is a re-send.
//! - **Outcomes** come from `Submissions`, which the services drain
//!   updates. One applies only to the composer whose comment ID is its
//!   own and which is posting (I7, A17). A saved comment clears its
//!   composer, whose draft the drain retires; the old slot is dropped and
//!   its draft never edited again (change 10). Any other outcome leaves
//!   the text, read-only, with the actions the outcome offers.
//! - **Actions**: Retry sends the same request (clearing a declined key
//!   first on an unlock code, R-3), or, for an adopted draft that was
//!   submitted, its stored body again (P11); Edit text and Post as a new
//!   comment re-target (`retarget`); Check again checks (`check`).
//!
//! Nothing here derives `Debug`: a slot holds the user's text.

use gpui_kit::{Context, EntityId, Window};
use manyhands::{
    canonical::ItemId,
    drafts::{CommentTarget, DraftId},
    repository::{AuthoringKind, ItemDtoKind, MutationDataDto},
    results::{CheckpointEffect, Envelope, WriteEffect},
};

use super::{SlotDraft, Thread};
use crate::desktop::{
    model::publication::{CommentOffer, CommentOutcome, CommentResult},
    services::{
        requests::{CommentBody, Intent},
        submissions::SubmissionState,
        worker::RecordState,
    },
    view::item::{
        ItemPane,
        composer::{ComposerState, FailedActions},
    },
};

/// A post `Submissions` refused with nothing of it in flight.
const NOT_SENT: &str = "This comment could not be sent. Try again.";
/// What `MaybeSaved` says (P5).
pub(crate) const MAYBE_SAVED: &str = "This comment may already be saved.";
/// Discard's note when the post's record is open (P7).
pub(crate) const MAY_STILL_BE_SAVED: &str =
    "This comment may still be saved later if it was written";
/// Discard's note when its file was written and not committed (P18).
pub(crate) const FILE_STAYS: &str =
    "This comment's file stays on this computer and may still appear in the thread.";
const NOT_SAVED: &str = "This comment was not saved.";
const STOPPED: &str = "Posting was stopped before this comment was saved.";
const OWED: &str = "This comment is not saved yet. Retry to finish posting it.";
const NEEDS_IDENTITY: &str = "Set your identity, then retry, to save this comment.";
const OCCUPIED: &str = "Another file already holds this comment's place.";
const MISMATCH_SAVED: &str =
    "This comment is saved with the text first posted under it; this text differs.";
pub(super) const MISMATCH: &str = "This text differs from the comment first posted under its ID.";
/// An adopted draft whose post has a record: it was submitted (P11).
pub(crate) const RESEND: &str =
    "This comment was being posted when Manyhands closed. Retry to finish posting it.";
/// An adopted draft whose record could not be read (P17).
pub(crate) const RECORD_UNKNOWN: &str = "Manyhands could not tell whether this comment was posted.";
/// Check again found a record that did not save the comment (change 2).
pub(crate) const NOT_SAVED_FINAL: &str = "This comment was not saved.";

/// What a slot's posting stands at.
#[derive(Default)]
pub(in crate::desktop::view::item) struct SlotPost {
    /// An async step of the tab's is out: a flush, a check or a
    /// re-target. The slot's outcome waits for it.
    pub(super) busy: bool,
    /// Its comment was sent before: posting it again is a re-send.
    pub(super) sent: bool,
    /// Adopted with no record: P4's checks run before its first post.
    pub(super) unchecked: bool,
    /// Adopted with a record: read-only until Retry sends it (P11, P12).
    pub(super) resend_pending: bool,
    /// Re-targeted from this comment ID (P13): its post restarts.
    pub(super) from: Option<ItemId>,
    /// The old comment's last attempt as the re-target was pressed: any
    /// attempt since overtakes the re-target (`overtaken`).
    pub(super) from_mark: Option<crate::desktop::services::worker::SendMark>,
    /// The composer said once that the old comment was sent again and
    /// not saved: the next Submit sends the new one (decision 27).
    pub(super) noted: bool,
    /// The old comment was posted meanwhile with other text: Post as a
    /// new comment sends this one, as it is (decision 27).
    pub(super) send_as_is: bool,
    /// The actions its last failed state offered: a refused re-target
    /// keeps them.
    pub(super) actions: Option<FailedActions>,
}

/// The kind of the item a comment is of, as a command takes it.
fn authoring(kind: ItemDtoKind) -> AuthoringKind {
    match kind {
        ItemDtoKind::Document => AuthoringKind::Document,
        ItemDtoKind::Ticket => AuthoringKind::Ticket,
    }
}

/// The failed state an outcome leaves a composer in.
pub(super) fn failed_state(
    envelope: &Envelope<MutationDataDto>,
    result: &CommentResult,
) -> ComposerState {
    let mut actions = FailedActions::default();
    for offer in result.offers() {
        match offer {
            CommentOffer::Retry => actions.retry = true,
            CommentOffer::EditText => actions.edit_text = true,
            CommentOffer::SetIdentity => actions.set_identity = true,
            CommentOffer::Discard => actions.discard = true,
            CommentOffer::CopyText => actions.copy_text = true,
            CommentOffer::CheckAgain => actions.check_again = true,
            _ => {}
        }
    }
    let maybe = result.outcome == CommentOutcome::MaybeSaved;
    if maybe {
        // R-19: never trapped behind Check again.
        actions.copy_text = true;
        actions.discard = true;
    }
    let message = match &result.outcome {
        CommentOutcome::Mismatched { saved: true } => MISMATCH_SAVED,
        CommentOutcome::Mismatched { saved: false } => MISMATCH,
        CommentOutcome::NotSavedStopped => STOPPED,
        CommentOutcome::NotSavedIdentity => NEEDS_IDENTITY,
        CommentOutcome::NotSavedConflicting => OCCUPIED,
        CommentOutcome::MaybeSaved => MAYBE_SAVED,
        CommentOutcome::NotSavedOwed { .. } => OWED,
        _ => NOT_SAVED,
    };
    ComposerState::Failed {
        message: message.to_owned(),
        actions,
        discard_note: if maybe {
            Some(MAY_STILL_BE_SAVED.to_owned())
        } else {
            discard_note(envelope, result.attempt.record)
        },
    }
}

/// What Discard says of a post that may still come to be saved.
pub(super) fn discard_note(
    envelope: &Envelope<MutationDataDto>,
    record: RecordState,
) -> Option<String> {
    let written = envelope.effects.write == WriteEffect::Written
        && envelope.effects.checkpoint != CheckpointEffect::Committed;
    if written {
        Some(FILE_STAYS.to_owned())
    } else if record == RecordState::Accepted {
        Some(MAY_STILL_BE_SAVED.to_owned())
    } else {
        None
    }
}

impl ItemPane {
    /// Submit: posts the composer's comment; an adopted draft with no
    /// record after P4's checks. An adopted draft that was submitted
    /// waits for Retry.
    pub(super) fn composer_submitted(
        &mut self,
        composer: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        if slot.post.busy || slot.post.resend_pending {
            return;
        }
        let checks = slot.post.unchecked;
        self.start_post(composer, checks, window, cx);
    }

    /// Sends the post of `composer`: posting at once, its draft flushed
    /// and its stored body read, then, when `checks`, P4's listing.
    pub(super) fn start_post(
        &mut self,
        composer: EntityId,
        checks: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drafts = self.services.drafts().clone();
        let Some((thread, slot)) = self.composers.find(composer) else {
            return;
        };
        if slot.target.is_none() {
            // Typed before the listing: an ID now, and no draft.
            let parent_id = match thread_root(&thread) {
                Ok(parent) => parent,
                Err(()) => return,
            };
            slot.target = Some(CommentTarget {
                comment_id: ItemId::generate(),
                parent_id,
            });
        }
        let Some(target) = slot.target.clone() else {
            return;
        };
        let draft = match slot.draft {
            SlotDraft::Begun(id) => Some(id),
            _ => None,
        };
        slot.post.busy = true;
        let entity = slot.composer.clone();
        let text = entity.read(cx).text(cx);
        let state = if checks {
            ComposerState::Checking
        } else {
            ComposerState::Posting
        };
        entity.update(cx, |composer, cx| composer.set_state(state, window, cx));
        cx.spawn_in(window, async move |pane, cx| {
            let body = match draft {
                Some(id) => stored_body(&drafts, id, cx).await.unwrap_or(text),
                None => text,
            };
            if checks {
                let Ok(look) = pane.update(cx, |pane, cx| {
                    pane.look(target.comment_id.clone(), false, cx)
                }) else {
                    return;
                };
                let found = look.await;
                if found.listed_body.as_deref() == Some(body.as_str()) {
                    // Saved by the session that ended: retired, not posted.
                    let _ = pane.update_in(cx, |pane, window, cx| {
                        if let Some(id) = draft {
                            pane.services
                                .drafts()
                                .update(cx, |registry, cx| registry.retire_comment(id, cx))
                                .detach();
                        }
                        pane.clear_slot(composer, window, cx);
                    });
                    return;
                }
            }
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.send_post(composer, target, body, draft, window, cx)
            });
        })
        .detach();
    }

    /// Hands the post to `Submissions`: a restart when re-targeted (P13),
    /// else the comment ID's request, a re-send when sent or adopted.
    fn send_post(
        &mut self,
        composer: EntityId,
        target: CommentTarget,
        body: String,
        draft: Option<DraftId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.item_key() else {
            return;
        };
        let root = self.post_root();
        let submissions = self.services.submissions().clone();
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        let resend = slot.post.sent || slot.post.unchecked || slot.post.resend_pending;
        let from = slot.post.from.clone();
        let mark = slot.post.from_mark;
        let entity = slot.composer.clone();
        // The old comment was sent again since it was re-targeted, from
        // outside this app: the new one waits, or is never sent
        // (decision 27).
        if let Some(old) = &from
            && self.overtaken(composer, old, mark, &body, window, cx)
        {
            return;
        }
        let intent = Intent::CommentAdd {
            root,
            kind: authoring(item.kind),
            id: item.id,
            comment_id: target.comment_id.clone(),
            parent_id: target.parent_id,
            body: CommentBody::new(body),
        };
        let sent = submissions.update(cx, |submissions, _| match &from {
            Some(old) => submissions.restart(old, &target.comment_id, intent, draft),
            None => submissions.submit(intent, draft, resend),
        });
        let Some((_, slot)) = self.composers.find(composer) else {
            return;
        };
        if sent.is_none() {
            // Refused: nothing changes, and the old target is kept. A post
            // of this comment in flight is followed (`apply_posts`); else
            // the text is editable again.
            slot.post.busy = false;
            let following = submissions
                .read(cx)
                .in_flight(&self.post_root(), &target.comment_id);
            entity.update(cx, |composer, cx| {
                if following {
                    composer.set_state(ComposerState::Posting, window, cx);
                    composer.refuse_while_posting(cx);
                } else {
                    composer.set_state(ComposerState::Idle, window, cx);
                    composer.say_refused(NOT_SENT, cx);
                }
            });
            cx.notify();
            return;
        }
        slot.post = SlotPost {
            sent: true,
            ..Default::default()
        };
        entity.update(cx, |composer, cx| {
            composer.set_state(ComposerState::Posting, window, cx)
        });
        cx.notify();
    }

    /// Applies each post's outcome that landed to its own composer, as
    /// the frame begins.
    pub(super) fn apply_posts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.post_root();
        let submissions = self.services.submissions().read(cx);
        let mut landed = Vec::new();
        let mut follow = Vec::new();
        for slot in self.composers.slots() {
            let composer = slot.composer.read(cx);
            let Some(target) = &slot.target else {
                continue;
            };
            if slot.post.busy {
                continue;
            }
            let id = slot.composer.entity_id();
            let posting = matches!(composer.state, ComposerState::Posting);
            // R-19: the composer follows its post, sent from anywhere.
            if submissions.in_flight(&root, &target.comment_id) {
                if !composer.is_posting() {
                    follow.push(id);
                }
                continue;
            }
            if !posting {
                continue;
            }
            match submissions
                .get(&root, &target.comment_id)
                .map(|post| &post.state)
            {
                None => landed.push((id, None)),
                Some(SubmissionState::Posting) => {}
                Some(SubmissionState::Finished { envelope, result }) => {
                    landed.push((id, Some((envelope.clone(), result.clone()))))
                }
            }
        }
        for composer in follow {
            self.set_composer(composer, ComposerState::Posting, window, cx);
        }
        for (composer, outcome) in landed {
            match outcome {
                // Never sent, or removed before it ran.
                None => self.set_composer(composer, ComposerState::Idle, window, cx),
                Some((envelope, result)) => {
                    self.apply_outcome(composer, &envelope, &result, window, cx)
                }
            }
        }
    }

    /// One outcome on its composer: cleared once saved, else failed.
    fn apply_outcome(
        &mut self,
        composer: EntityId,
        envelope: &Envelope<MutationDataDto>,
        result: &CommentResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mismatched = matches!(result.outcome, CommentOutcome::Mismatched { .. });
        if result.outcome.is_saved() && !mismatched {
            self.clear_slot(composer, window, cx);
            return;
        }
        self.set_composer(composer, failed_state(envelope, result), window, cx);
        if mismatched && result.attempt.record == RecordState::Finished {
            self.mismatch_record(composer, window, cx);
        }
    }

    /// Sets the state of `composer`.
    pub(super) fn set_composer(
        &mut self,
        composer: EntityId,
        state: ComposerState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some((_, slot)) = self.composers.find(composer) {
            if let ComposerState::Failed { actions, .. } = &state {
                slot.post.actions = Some(*actions);
            }
            slot.composer
                .clone()
                .update(cx, |composer, cx| composer.set_state(state, window, cx));
        }
    }

    /// A composer whose comment is saved, or retired, starts again: the
    /// new comment's empty, the reply's closed. Its old slot and draft
    /// session are let go of, never edited again (change 10).
    pub(super) fn clear_slot(
        &mut self,
        composer: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((thread, _)) = self.composers.find(composer) else {
            return;
        };
        match thread {
            None => {
                let slot = self.slot(
                    super::super::ComposerKind::New,
                    "",
                    None,
                    SlotDraft::None,
                    window,
                    cx,
                );
                self.composers.new = Some(slot);
                self.composers.open = false;
            }
            Some(root) => {
                if matches!(&self.composers.reply, Some((at, _)) if *at == root) {
                    self.composers.reply = None;
                    self.composers.reply_from = None;
                } else {
                    self.composers.kept.remove(&root);
                }
            }
        }
        cx.notify();
    }
}

/// The parent a comment of `thread` answers: none for a new comment, the
/// thread's root for a reply; `Err` when the root's ID cannot be read.
fn thread_root(thread: &Thread) -> Result<Option<ItemId>, ()> {
    match thread {
        None => Ok(None),
        Some(root) => root.parse().map(Some).map_err(|_| ()),
    }
}

/// The draft `id` flushed, and its stored body: what is posted (R-16,
/// change 9). `None` when it cannot be read.
pub(super) async fn stored_body(
    drafts: &gpui_kit::Entity<crate::desktop::services::drafts::DraftRegistry>,
    id: DraftId,
    cx: &mut gpui_kit::AsyncWindowContext,
) -> Option<String> {
    let flush = drafts.update(cx, |registry, cx| registry.flush_now(id, cx));
    let _ = flush.await;
    let adopted = drafts.update(cx, |registry, cx| registry.adopt_comment(id, cx));
    adopted.await.ok().map(|adopted| adopted.body)
}
