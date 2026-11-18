//! The new comment's composer while it is collapsed (decision 22): one
//! tab stop, Write a comment…, which opens it with focus in its editor.
//! It stays open while it holds text or a draft, an adopted one too; an
//! empty one collapses again once focus has left it, and so does one
//! emptied of the text whose draft it began, which is discarded. Open, Tab indents in
//! it and F6 leaves (decision 19), as in the body editor.

use gpui_kit::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use super::{ComposerKind, SlotDraft, WRITE_A_COMMENT};
use crate::desktop::view::item::{ItemPane, tokens::item_colors};

impl ItemPane {
    /// Whether the new comment's composer is collapsed to Write a
    /// comment… (decision 22): not opened, and holding no text and no
    /// draft.
    pub(super) fn new_collapsed(&self, cx: &gpui_kit::App) -> bool {
        !self.composers.open
            && self.composers.new.as_ref().is_some_and(|slot| {
                matches!(slot.draft, SlotDraft::None) && slot.composer.read(cx).text(cx).is_empty()
            })
    }

    /// As each frame begins: an opened new-comment composer that holds no
    /// text and no draft collapses again once focus has left it, as does
    /// one emptied of its text (`emptied_draft`), whose draft goes; one just
    /// opened takes focus in its editor once this frame is drawn, and one
    /// collapsed by an empty Cancel gives it to Write a comment….
    pub(super) fn follow_new_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(slot) = &self.composers.new else {
            return;
        };
        let composer = slot.composer.read(cx);
        let empty = matches!(slot.draft, SlotDraft::None) && composer.text(cx).is_empty();
        let left = !composer.contains_focus(window, cx);
        let blank = composer.text(cx).trim().is_empty();
        let editor = composer.editor().clone();
        let leaving = self.composers.open && !self.composers.focus_new && left;
        if leaving
            && blank
            && let Some(id) = self.emptied_draft(cx)
        {
            self.services
                .drafts()
                .update(cx, |registry, cx| registry.discard_comment(id, cx));
            let fresh = self.slot(ComposerKind::New, "", None, SlotDraft::None, window, cx);
            self.composers.new = Some(fresh);
            self.composers.open = false;
            return;
        }
        if leaving && empty {
            self.composers.open = false;
        }
        if std::mem::take(&mut self.composers.focus_start)
            && let Some(start) = self.composers.start.clone()
        {
            window.focus(&start, cx);
        }
        if std::mem::take(&mut self.composers.focus_new) {
            self.composers.open = true;
            cx.defer_in(window, move |_, window, cx| {
                editor.update(cx, |editor, cx| editor.focus(window, cx));
            });
        }
    }

    /// The new comment's draft, when this tab began it and nothing posted
    /// its comment: an emptied composer may let it go. Never an adopted
    /// draft, nor one with a post in `Submissions`.
    fn emptied_draft(&self, cx: &gpui_kit::App) -> Option<manyhands::drafts::DraftId> {
        let slot = self.composers.new.as_ref()?;
        let SlotDraft::Begun(id) = slot.draft else {
            return None;
        };
        let comment = &slot.target.as_ref()?.comment_id;
        if !self.composers.begun.contains(comment) {
            return None;
        }
        let repo = self.item_key()?.repo;
        let posted = self
            .services
            .submissions()
            .read(cx)
            .get(std::path::Path::new(&repo), comment)
            .is_some();
        (!posted).then_some(id)
    }

    /// Write a comment…: the new comment's composer opens, with focus in
    /// its editor.
    pub(super) fn open_new_composer(&mut self, cx: &mut Context<Self>) {
        self.composers.open = true;
        self.composers.focus_new = true;
        cx.notify();
    }

    /// The collapsed new-comment composer (decision 22): one tab stop that
    /// says Write a comment…, opened by click, Enter or Space, ringed while
    /// focused and revealed when focus reaches it.
    pub(super) fn write_a_comment(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let start = self.composers.start.as_ref()?;
        let colors = item_colors(cx);
        let border = if start.is_focused(window) {
            colors.page_teal
        } else {
            colors.page_border_strong
        };
        Some(
            div()
                .id("new-comment-start")
                .test_support()
                .role(Role::Button)
                .aria_label(WRITE_A_COMMENT)
                .track_focus(start)
                .relative()
                .w_full()
                .px(px(12.))
                .py(px(10.))
                .bg(colors.page)
                .border_1()
                .border_color(border)
                .rounded(px(8.))
                .text_size(px(14.))
                .text_color(colors.page_muted)
                .cursor_text()
                .on_click(cx.listener(|pane, _: &ClickEvent, _, cx| pane.open_new_composer(cx)))
                .child(WRITE_A_COMMENT)
                .child(self.scroll.marker(start))
                .into_any_element(),
        )
    }

    /// The composers, at the discussion's end, after the threads and the
    /// posts being sent: the new comment's, or Write a comment… while it
    /// is collapsed.
    pub(in crate::desktop::view::item) fn composers_view(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let slot = self.composers.new.as_ref()?;
        if self.new_collapsed(cx) {
            let start = self.write_a_comment(window, cx)?;
            return Some(
                div()
                    .id("new-comment-composer")
                    .test_support()
                    .w_full()
                    .child(start)
                    .into_any_element(),
            );
        }
        Some(
            div()
                .id("new-comment-composer")
                .test_support()
                .w_full()
                .child(slot.composer.clone())
                .into_any_element(),
        )
    }
}
