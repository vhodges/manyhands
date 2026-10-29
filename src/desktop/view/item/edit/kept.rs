//! A session kept after Done (decision 15; R-58, R-68): the tab views the
//! saved copy while the edit editor waits with its text, caret and undo.
//!
//! - **What shows.** The page holds the read editor, which shows the saved
//!   copy through its own reload (R-79), and the header, chips, sheet and
//!   tab title show the saved copy too: they read the item, never the
//!   draft. The pill reads "Viewing · unsaved changes kept", and the tab
//!   keeps its unsaved mark: the draft is attached to it. Nothing is
//!   released.
//! - **The kept-draft banner** offers Continue editing and Discard…; Edit,
//!   `e` and `secondary-shift-e` continue too. Continuing shows the same
//!   editor again, focused, with nothing prepared again.
//! - **A refresh** that classifies the draft `Changed` shows the change
//!   banner, with Compare…, which continues into the review (R-68).
//!   Nothing opens by itself.
//! - **Discard…** asks first; it ends the session and discards the draft,
//!   and release on leave applies.
//!
//! The banners name no draft text.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Styled, Window,
    component::{Disableable, Sizable, button::Button, h_flex},
    div,
};
use manyhands::drafts::TargetState;
use zorite_editor::EditorState;

use super::leave::Elsewhere;
use crate::desktop::{
    model::edit::{
        COMPARE, CONTINUE_EDITING, DRAFT_DISCARDED, KEPT_AFTER_DONE, Phase,
        save_state::CHANGED_OUTSIDE,
    },
    view::{
        drafts::actions::open_discard_saying,
        item::{DraftDiscarded, ItemPane, banners::Banner, draft_actions::DISCARD, text},
    },
};

/// The kept-draft banner's Discard… asks first, in these words.
pub(crate) const DISCARD_KEPT: &str =
    "Discard your unsaved changes? The draft is deleted and cannot be recovered.";

impl ItemPane {
    /// Done kept the session (R-58): the tab is viewing the saved copy. Done
    /// gives the page's read editor the focus (`done`).
    pub(super) fn keep_session(&mut self, cx: &mut Context<Self>) {
        let Some(session) = &mut self.edit.session else {
            return;
        };
        session.kept = true;
        self.edit.phase = Phase::Viewing;
        self.edit.changed = None;
        self.sync_status(cx);
        cx.notify();
    }

    /// Continue editing a kept session: the same editor again, focused,
    /// with its text, caret and undo. False when no session is kept.
    pub(super) fn resume_kept(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(editor) = self.unkeep(cx) else {
            return false;
        };
        // Once the frame that draws it again is drawn.
        cx.defer_in(window, move |_, window, cx| {
            editor.update(cx, |editor, cx| editor.focus(window, cx));
        });
        true
    }

    /// The kept session is editing again, with its text, caret and undo,
    /// and the focus where it was: the close dialog's Save saves it (Task
    /// 18 fix 1). Gives its editor; `None` when no session is kept.
    pub(super) fn unkeep(&mut self, cx: &mut Context<Self>) -> Option<Entity<EditorState>> {
        let session = self.edit.session.as_mut().filter(|session| session.kept)?;
        session.kept = false;
        let editor = session.host.read(cx).editor().clone();
        self.edit.phase = Phase::Editing;
        self.sync_status(cx);
        cx.notify();
        Some(editor)
    }

    /// As each frame is drawn: whether the item changed under the session's
    /// draft, kept or shown, by D2's classification of the tab's read, its
    /// body as the editor loads it (R-11). The change banner says so
    /// (R-68): the kept session's here, the shown one's with the saves'
    /// banners. It is computed again for a new revision, or once a read
    /// lands.
    pub(in crate::desktop::view::item) fn sync_changed(&mut self, cx: &App) {
        let Some(id) = self.live_draft(cx) else {
            self.edit.changed = None;
            return;
        };
        // M1: a shown session's own save, in flight, kept for its Retry, or
        // still being settled by D2, moves the item before the draft's base
        // follows it: that is no one else's change.
        if !self.session_kept() && (self.edit.save.run.is_some() || self.store_settling(cx)) {
            self.edit.changed = None;
            return;
        }
        let registry = self.services.drafts().read(cx);
        let Some(revision) = registry.session(id).map(|session| session.revision()) else {
            return;
        };
        if matches!(self.edit.changed, Some((at, seen, _)) if at == id && seen == revision) {
            return;
        }
        let Some(item) = self.model.item_read() else {
            return;
        };
        let changed = matches!(
            registry.classify(id, item),
            Some(TargetState::Changed { .. })
        );
        self.edit.changed = Some((id, revision, changed));
    }

    /// The kept session's banners: the kept draft, with Continue editing
    /// and Discard…, and the change banner, with Compare…, when a refresh
    /// found the item changed; and, once a discard elsewhere ended the
    /// session, that it was discarded (R-71).
    pub(super) fn kept_banners(&self, cx: &mut Context<Self>) -> Vec<(Banner, AnyElement)> {
        let mut banners = Vec::new();
        if self.edit.discarded == Some(Elsewhere::Discarded) && self.edit.session.is_none() {
            banners.push((
                Banner::DraftDiscarded,
                text("item-draft-discarded", DRAFT_DISCARDED).into_any_element(),
            ));
        }
        if !self.session_kept() {
            return banners;
        }
        let available = self.edit_available(cx);
        let continue_editing = Button::new("item-kept-continue")
            .label(CONTINUE_EDITING)
            .small()
            .disabled(!available)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.edit_requested(window, cx);
            }));
        let discard = div()
            .id("item-kept-discard-focus")
            .track_focus(&self.edit.kept_discard)
            .child(
                Button::new("item-kept-discard")
                    .label(DISCARD)
                    .small()
                    .disabled(self.save_in_flight(cx))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.discard_kept(window, cx)
                    })),
            );
        banners.push((
            Banner::KeptAfterDone,
            h_flex()
                .flex_wrap()
                .gap_3()
                .items_center()
                .child(text("item-kept", KEPT_AFTER_DONE))
                .child(continue_editing)
                .child(discard)
                .into_any_element(),
        ));
        if matches!(self.edit.changed, Some((_, _, true))) {
            let compare = Button::new("item-kept-compare")
                .label(COMPARE)
                .small()
                .disabled(!available)
                .on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.compare_kept(window, cx)),
                );
            banners.push((
                Banner::Changed,
                h_flex()
                    .flex_wrap()
                    .gap_3()
                    .items_center()
                    .child(text("item-kept-changed", CHANGED_OUTSIDE))
                    .child(compare)
                    .into_any_element(),
            ));
        }
        banners
    }

    /// Compare…: the kept session is continued, into the review (R-68).
    fn compare_kept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.edit_available(cx) && self.resume_kept(window, cx) {
            self.open_review(cx);
        }
    }

    /// Discard…: asks first. Confirm ends the session and discards its
    /// draft; the tab's content then takes the focus, the banner having
    /// gone with its Discard….
    fn discard_kept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.save_in_flight(cx) {
            return;
        }
        let pane = cx.entity().downgrade();
        let origin = self.edit.kept_discard.clone();
        open_discard_saying(
            DISCARD_KEPT.to_owned(),
            origin,
            &self.services,
            window,
            cx,
            move |_, cx| {
                let _ = pane.update(cx, |pane, cx| {
                    if !pane.session_kept() {
                        return;
                    }
                    pane.discard_session(None, cx).detach();
                    cx.emit(DraftDiscarded);
                });
            },
        );
    }
}
