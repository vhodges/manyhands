//! The comment drafts this tab's composers hold, as the Drafts page sees
//! them (Task 12 review): the registry is told which they are, so the
//! page's Show reaches this tab, beside the tab's own draft; and a draft
//! the page discards resets the composer holding it, so its later text
//! begins a new draft rather than editing one that is gone. Nothing here
//! derives `Debug`.

use std::collections::BTreeSet;

use gpui_kit::{Context, Window};
use manyhands::drafts::DraftId;

use super::SlotDraft;
use crate::desktop::{
    services::drafts::CommentDiscarded,
    view::item::{ItemPane, composer::ComposerKind},
};

impl ItemPane {
    /// Tells the registry which comment drafts this tab's composers hold
    /// now, shown or kept.
    pub(super) fn hold_comment_drafts(&mut self, cx: &mut Context<Self>) {
        let held: BTreeSet<DraftId> = self
            .composers
            .slots()
            .filter_map(|slot| match slot.draft {
                SlotDraft::Begun(id) => Some(id),
                _ => None,
            })
            .collect();
        let tab = self.tab_key();
        self.services
            .drafts()
            .update(cx, |registry, cx| registry.hold_comments(&tab, held, cx));
    }

    /// The Drafts page discarded the comment draft of `event`: the
    /// composer holding it starts again as its next frame is drawn.
    pub(in crate::desktop::view::item) fn comment_discarded(
        &mut self,
        event: &CommentDiscarded,
        cx: &mut Context<Self>,
    ) {
        self.composers.discarded.push(event.0.clone());
        cx.notify();
    }

    /// Each composer whose draft the Drafts page discarded starts again,
    /// as an empty Cancel leaves it but with no focus moved, unless it
    /// holds focus: the new comment's empty and collapsed, a reply's
    /// closed. Its draft is gone already; nothing more is discarded.
    pub(super) fn reset_discarded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for ids in std::mem::take(&mut self.composers.discarded) {
            let composers = &mut self.composers;
            // Any ID that reached the discarded session: a slot keeps the
            // one it began or adopted.
            let holds =
                |draft: &SlotDraft| matches!(draft, SlotDraft::Begun(held) if ids.contains(held));
            if composers
                .new
                .as_ref()
                .is_some_and(|slot| holds(&slot.draft))
            {
                let focused = composers
                    .new
                    .as_ref()
                    .is_some_and(|slot| slot.composer.read(cx).contains_focus(window, cx));
                let slot = self.slot(ComposerKind::New, "", None, SlotDraft::None, window, cx);
                self.composers.new = Some(slot);
                self.composers.open = false;
                self.composers.focus_start = focused;
                continue;
            }
            if composers
                .reply
                .as_ref()
                .is_some_and(|(_, slot)| holds(&slot.draft))
            {
                let focused = composers
                    .reply
                    .as_ref()
                    .is_some_and(|(_, slot)| slot.composer.read(cx).contains_focus(window, cx));
                composers.reply = None;
                let from = composers.reply_from.take();
                if focused
                    && let Some(handle) = from.and_then(|id| self.discussion.reply_focus(&id))
                {
                    window.focus(&handle, cx);
                }
                continue;
            }
            composers.kept.retain(|_, slot| !holds(&slot.draft));
        }
    }
}
