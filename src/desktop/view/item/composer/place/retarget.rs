//! Moving a composer's comment to a new comment ID (design, P6, P13, P16;
//! R-18; changes 4 and 5): Edit text after a finished failure, and Post
//! as a new comment. At the press: a new ID, the new draft begun with the
//! text and awaited, then the old draft retired and awaited. Only then
//! does the slot write the new comment, so a crash in between leaves both
//! drafts, never neither, and relaunch never adopts the old ID with new
//! text. A begin that fails refuses, keeping the old ID and text with
//! nothing retired; a retirement that fails discards the new draft, as far
//! as it can, and refuses too. The post that follows restarts under the
//! new ID (`Submissions::restart`).

use gpui_kit::{Context, EntityId, Window};
use manyhands::{
    canonical::ItemId,
    drafts::{CommentTarget, DraftErrorKind},
};

use super::{SlotDraft, post::failed_state};
use crate::desktop::{
    services::submissions::SubmissionState,
    view::item::{ItemPane, composer::ComposerState},
};

/// What a refused re-target says, before the actions it had.
pub(crate) const NOT_MOVED: &str =
    "Manyhands could not prepare the comment to be posted again. Try again.";

/// What follows a re-target.
#[derive(Clone, Copy)]
pub(super) enum After {
    /// Edit text: the text is editable, under the new ID.
    Edit,
    /// Post as a new comment: the same text posts under the new ID.
    Post,
}

impl ItemPane {
    /// Re-targets `composer`'s comment to a new comment ID, then edits or
    /// posts it.
    pub(super) fn retarget(
        &mut self,
        composer: EntityId,
        after: After,
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
        let Some(old) = slot.target.clone() else {
            return;
        };
        if slot.post.busy {
            return;
        }
        slot.post.busy = true;
        let old_draft = match slot.draft {
            SlotDraft::Begun(id) => Some(id),
            _ => None,
        };
        let text = slot.composer.read(cx).text(cx);
        let target = CommentTarget {
            comment_id: ItemId::generate(),
            parent_id: old.parent_id.clone(),
        };
        self.composers.begun.insert(target.comment_id.clone());
        self.set_composer(composer, ComposerState::Checking, window, cx);
        let begin = drafts.update(cx, |registry, cx| {
            registry.begin_comment(item, target.clone(), text, cx)
        });
        cx.spawn_in(window, async move |pane, cx| {
            let Ok(new_draft) = begin.await else {
                let _ = pane.update_in(cx, |pane, window, cx| {
                    pane.refuse_retarget(composer, &old.comment_id, window, cx)
                });
                return;
            };
            if let Some(old_draft) = old_draft {
                let retire =
                    drafts.update(cx, |registry, cx| registry.retire_comment(old_draft, cx));
                if let Err(kind) = retire.await
                    && kind != DraftErrorKind::NotFound
                {
                    drafts.update(cx, |registry, cx| registry.discard_comment(new_draft, cx));
                    let _ = pane.update_in(cx, |pane, window, cx| {
                        pane.refuse_retarget(composer, &old.comment_id, window, cx)
                    });
                    return;
                }
            }
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.composers.known.insert(new_draft);
                let Some((_, slot)) = pane.composers.find(composer) else {
                    // Gone meanwhile: the new draft stays for the Drafts
                    // page, as an ended session's would.
                    return;
                };
                slot.target = Some(target);
                slot.draft = SlotDraft::Begun(new_draft);
                slot.post.busy = false;
                slot.post.sent = false;
                slot.post.unchecked = false;
                slot.post.resend_pending = false;
                slot.post.from = Some(old.comment_id);
                let entity = slot.composer.clone();
                match after {
                    After::Edit => {
                        pane.set_composer(composer, ComposerState::Idle, window, cx);
                        let editor = entity.read(cx).editor().clone();
                        editor.update(cx, |editor, cx| editor.focus(window, cx));
                    }
                    After::Post => pane.start_post(composer, false, window, cx),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A re-target that could not be made: the old comment ID and text
    /// stay, with the failed post's actions, and say so (change 5).
    fn refuse_retarget(
        &mut self,
        composer: EntityId,
        old: &ItemId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let root = self.post_root();
        let state = self
            .services
            .submissions()
            .read(cx)
            .get(&root, old)
            .and_then(|post| match &post.state {
                SubmissionState::Finished { envelope, result } => {
                    Some(failed_state(envelope, result))
                }
                SubmissionState::Posting => None,
            });
        let state = match state {
            Some(ComposerState::Failed {
                actions,
                discard_note,
                ..
            }) => ComposerState::Failed {
                message: NOT_MOVED.to_owned(),
                actions,
                discard_note,
            },
            _ => ComposerState::Failed {
                message: NOT_MOVED.to_owned(),
                actions: crate::desktop::view::item::composer::FailedActions {
                    copy_text: true,
                    discard: true,
                    ..Default::default()
                },
                discard_note: None,
            },
        };
        if let Some((_, slot)) = self.composers.find(composer) {
            slot.post.busy = false;
        }
        self.set_composer(composer, state, window, cx);
    }
}
