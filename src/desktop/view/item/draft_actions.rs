//! The restored draft section's Export… and Discard…, through the Drafts
//! page's own actions (`view/drafts/actions.rs`).
//!
//! Export… writes the session's newest text as Markdown where the save
//! dialog says, and the section says what it came to. Discard… asks
//! first, naming the draft; Confirm ends the session and deletes the
//! draft, and the section goes with the session: the tab's content takes
//! the focus (`DraftDiscarded`). A discard that fails has released the
//! draft, still stored: the banner says so, and offers the draft again
//! once the store is listed.

use gpui_kit::{
    AnyElement, ClickEvent, Context, ElementId, EventEmitter, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Task, TestSupportExt, Window,
    component::{button::Button, h_flex},
    div,
};
use manyhands::drafts::{DraftId, Held, markdown};

use super::{ItemPane, draft::button, text};
use crate::desktop::{
    model::drafts::DraftRow,
    view::drafts::actions::{EXPORT, draft_name, export, may_open, open_discard, suggested_name},
};

pub(crate) const DISCARD: &str = "Discard…";

/// The section's Discard… ended the session: the tab's content is to take
/// the focus, the section and its Discard… being gone.
pub(crate) struct DraftDiscarded;

impl EventEmitter<DraftDiscarded> for ItemPane {}

impl ItemPane {
    /// Export… and Discard… for the session `id`, and what the last
    /// export came to.
    pub(super) fn draft_actions(&mut self, id: DraftId, cx: &mut Context<Self>) -> AnyElement {
        let focus = self
            .draft
            .discard_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let row = self
            .draft
            .actions_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let exported = self
            .draft
            .exported
            .clone()
            .map(|exported| text("draft-export-result", exported).text_sm());
        // Tab reaches the Kit button; the focus Discard's dialog gives back
        // to the wrapper is pressed by Enter or Space there too.
        let discard = div()
            .id("draft-discard")
            .test_support()
            .aria_label(DISCARD)
            .track_focus(&focus)
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                if event.is_keyboard() {
                    this.discard_draft(id, window, cx)
                }
            }))
            .child(
                Button::new(ElementId::from("draft-discard-button"))
                    .label(DISCARD)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.discard_draft(id, window, cx)
                    })),
            );
        // Below a long section and its comparison: the focus in the row
        // scrolls it into view.
        h_flex()
            .id("draft-actions")
            .relative()
            .track_focus(&row)
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
            .child(self.scroll.marker_within(&row))
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

    /// Discard…: asks first, naming the draft as its row on the Drafts
    /// page does: its title, else its kind.
    fn discard_draft(&mut self, id: DraftId, window: &mut Window, cx: &mut Context<Self>) {
        let registry = self.services.drafts().read(cx);
        let Some(session) = registry.session(id) else {
            return;
        };
        let row = DraftRow::readable(&session.draft(), Held::Here, String::new());
        let name = draft_name(&row);
        let Some(origin) = self.draft.discard_focus.clone() else {
            return;
        };
        let pane = cx.entity().downgrade();
        open_discard(name, origin, &self.services, window, cx, move |_, cx| {
            let _ = pane.update(cx, |pane, cx| pane.discard_confirmed(id, cx));
        });
    }

    /// Ends the session `id` and deletes its draft; the tab's content then
    /// takes the focus. A failure is said in the banner: the draft is
    /// released, still stored.
    fn discard_confirmed(&mut self, id: DraftId, cx: &mut Context<Self>) {
        let discard = self
            .services
            .drafts()
            .update(cx, |registry, cx| registry.discard(id, cx));
        self.draft.exported = None;
        cx.spawn(async move |pane, cx| {
            let discarded = discard.await;
            let _ = pane.update(cx, |pane, cx| match discarded {
                Ok(()) => cx.emit(DraftDiscarded),
                Err(kind) => pane.discard_failed(kind, cx),
            });
        })
        .detach();
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "draft_actions_tests.rs"]
mod tests;
