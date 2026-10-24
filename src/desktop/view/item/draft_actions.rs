//! The restored draft section's Export… and Discard…, through the Drafts
//! page's own actions (`view/drafts/actions.rs`).
//!
//! Export… writes the session's newest text as Markdown where the save
//! dialog says, and the section says what it came to. Discard… asks
//! first, naming the draft; Confirm ends the session and deletes the
//! draft, and the section goes with the session. A discard that fails
//! has released the draft, still stored: the banner says so, and offers
//! the draft again once the store is listed.

use gpui_kit::{
    AnyElement, ClickEvent, Context, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Task, TestSupportExt, Window,
    component::{button::Button, h_flex},
    div,
};
use manyhands::drafts::{DraftFields, DraftId, markdown};

use super::{ItemPane, draft::button, text};
use crate::desktop::view::drafts::actions::{
    EXPORT, export, may_open, open_discard, suggested_name,
};

pub(crate) const DISCARD: &str = "Discard…";

impl ItemPane {
    /// Export… and Discard… for the session `id`, and what the last
    /// export came to.
    pub(super) fn draft_actions(&mut self, id: DraftId, cx: &mut Context<Self>) -> AnyElement {
        let focus = self
            .draft
            .discard_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let exported = self
            .draft
            .exported
            .clone()
            .map(|exported| text("draft-export-result", exported).text_sm());
        let discard = div()
            .id("draft-discard")
            .test_support()
            .aria_label(DISCARD)
            .track_focus(&focus)
            .child(
                Button::new(ElementId::from("draft-discard-button"))
                    .label(DISCARD)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.discard_draft(id, window, cx)
                    })),
            );
        h_flex()
            .flex_wrap()
            .gap_2()
            .child(button(
                "draft-export",
                EXPORT,
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.export_draft(id, window, cx)
                }),
            ))
            .child(discard)
            .children(exported)
            .into_any_element()
    }

    /// Export…: the session's newest text, as Markdown.
    fn export_draft(&mut self, id: DraftId, window: &mut Window, cx: &mut Context<Self>) {
        if !may_open(&self.services, window, cx) {
            return;
        }
        let Some(draft) = self
            .services
            .drafts()
            .read(cx)
            .session(id)
            .map(|session| session.draft())
        else {
            return;
        };
        self.draft.exported = None;
        let bytes = markdown(&draft).into_bytes();
        let export = export(suggested_name(&draft), Task::ready(Ok(bytes)), cx);
        cx.spawn(async move |pane, cx| {
            let exported = export.await;
            let _ = pane.update(cx, |pane, cx| {
                pane.draft.exported = exported.label();
                cx.notify();
            });
        })
        .detach();
    }

    /// Discard…: asks first, naming the draft by its own title, as its
    /// own section does.
    fn discard_draft(&mut self, id: DraftId, window: &mut Window, cx: &mut Context<Self>) {
        let registry = self.services.drafts().read(cx);
        let Some(session) = registry.session(id) else {
            return;
        };
        let name = match session.current() {
            DraftFields::Document { title, .. } | DraftFields::Ticket { title, .. } => {
                title.clone()
            }
            DraftFields::Comment { .. } => session.item_title().to_owned(),
        };
        let Some(origin) = self.draft.discard_focus.clone() else {
            return;
        };
        let pane = cx.entity().downgrade();
        open_discard(
            Some(name),
            origin,
            &self.services,
            window,
            cx,
            move |_, cx| {
                let _ = pane.update(cx, |pane, cx| pane.discard_confirmed(id, cx));
            },
        );
    }

    /// Ends the session `id` and deletes its draft. A failure is said in
    /// the banner: the draft is released, still stored.
    fn discard_confirmed(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let discard = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.discard(id, cx));
        self.draft.exported = None;
        cx.spawn(async move |pane, cx| {
            if let Err(kind) = discard.await {
                let _ = pane.update(cx, |pane, cx| pane.discard_failed(kind, cx));
            }
        })
        .detach();
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "draft_actions_tests.rs"]
mod tests;
