//! The Drafts page's Inspect panel: one row's draft, its own fields
//! read-only and, when its item changed or may have, the comparison view,
//! whose body shows its three blocks with no merge.
//!
//! It opens under its row, focused, and closes on Escape or Close, the
//! focus going back to the row's Inspect. It shows the draft's text, so it
//! is drawn only in the draft's own row.

use std::sync::Arc;

use gpui_kit::{
    AnyElement, ClickEvent, Context, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, button::Button, h_flex, v_flex},
    div,
};
use manyhands::drafts::{DraftId, TargetState};

use super::{DraftsPage, INSPECT, Slot, row_actions::action_ix};
use crate::desktop::{
    keys::{CloseInspect, DRAFT_INSPECT_CONTEXT},
    view::draft_comparison::{DraftComparisonView, FieldIds, draft_fields},
};

pub(crate) const CLOSE: &str = "Close";

const INSPECT_FIELDS: FieldIds = FieldIds {
    title: "drafts-inspect-title",
    field: "drafts-inspect-field",
    body: "drafts-inspect-body",
};

impl DraftsPage {
    /// Opens the panel on row `id`, focused, in place of any other.
    pub(super) fn inspect(&mut self, id: DraftId, window: &mut Window, cx: &mut Context<Self>) {
        if self.slot(id).and_then(|slot| slot.draft.as_ref()).is_none() {
            return;
        }
        self.inspecting = Some(id);
        window.focus(&self.inspect_focus, cx);
        cx.notify();
    }

    /// Closes the panel; the focus goes back to its row's Inspect.
    pub(super) fn close_inspect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.inspecting.take() else {
            return;
        };
        if let Some(focus) = self.focus.get(&id) {
            window.focus(&focus.actions[action_ix("inspect")], cx);
        }
        cx.notify();
    }

    /// The panel, when it is open on `slot`'s row.
    pub(super) fn inspect_panel(&self, slot: &Slot, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.inspecting != Some(slot.row.id) {
            return None;
        }
        let draft = slot.draft.as_ref()?;
        let comparison = match slot.row.target() {
            Some(
                TargetState::Changed {
                    theirs, comparison, ..
                }
                | TargetState::Unverified { theirs, comparison },
            ) => {
                // With no base, the comparison was made from the item.
                let base = draft
                    .base
                    .as_ref()
                    .map(|base| &base.fields)
                    .unwrap_or(theirs);
                Some(DraftComparisonView::new(comparison, base, theirs, &draft.current).render(cx))
            }
            _ => None,
        };
        let wrapper = ElementId::from("drafts-inspect-close");
        let close = div()
            .id(wrapper.clone())
            .test_support()
            .aria_label(CLOSE)
            .child(
                Button::new(ElementId::NamedChild(Arc::new(wrapper), "button".into()))
                    .label(CLOSE)
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.close_inspect(window, cx)
                    })),
            );
        Some(
            v_flex()
                .id("drafts-inspect")
                .test_support()
                .aria_label(INSPECT)
                .track_focus(&self.inspect_focus)
                .key_context(DRAFT_INSPECT_CONTEXT)
                .on_action(
                    cx.listener(|this, _: &CloseInspect, window, cx| {
                        this.close_inspect(window, cx)
                    }),
                )
                .gap_2()
                .p_2()
                .border_1()
                .border_color(cx.theme().border)
                .child(draft_fields(INSPECT_FIELDS, &draft.current, cx))
                .children(comparison)
                .child(h_flex().child(close))
                .into_any_element(),
        )
    }
}
