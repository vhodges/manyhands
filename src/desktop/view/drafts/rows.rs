//! The Drafts page's body: why the store cannot be listed, that it is
//! being listed, that it keeps nothing, or a row per draft.
//!
//! A row is a focusable stop of its own, then its actions, each a Kit
//! button in a focus-tracking wrapper with the content region's scroll
//! marker. Show and Retry act here; Restore, Inspect and Discard are wired
//! by the page's actions (Task 11).

use std::sync::Arc;

use gpui_kit::{
    AnyElement, ClickEvent, Context, ElementId, FocusHandle, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, button::Button, h_flex, v_flex},
    div,
};
use manyhands::drafts::{DraftError, DraftId, Held, TargetState};

use super::{
    ACTIONS, CANNOT_READ, CHECKING, COMMENT_LATER, DraftsPage, LOADING_DRAFTS, NO_DRAFTS,
    OPEN_ELSEWHERE, RESTORED_HERE, SAVED_AT, Slot, row_id,
};
use crate::desktop::{
    model::{
        drafts::{DraftRow, RowActions, RowState, rows::unreadable_label},
        list_state::ListState,
        read_state::Read,
    },
    view::draft_comparison::target_label,
};

/// A row's state in words: who holds it first, then where its item
/// stands.
pub(crate) fn state_label(row: &DraftRow) -> String {
    match &row.state {
        RowState::Unreadable(reason) => format!("{CANNOT_READ}: {}", unreadable_label(*reason)),
        RowState::Readable {
            held: Held::Elsewhere,
            ..
        } => OPEN_ELSEWHERE.to_owned(),
        RowState::Readable {
            held: Held::Here, ..
        } => RESTORED_HERE.to_owned(),
        RowState::Readable {
            target: Read::Ready(target),
            ..
        } => target_label(target).to_owned(),
        RowState::Readable { .. } => CHECKING.to_owned(),
    }
}

fn offered(actions: RowActions, part: &str) -> bool {
    match part {
        "restore" => actions.restore,
        "show" => actions.show,
        "inspect" => actions.inspect,
        "discard" => actions.discard,
        "retry" => actions.retry,
        _ => false,
    }
}

type Text = gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>>;

fn text(id: impl Into<ElementId>, value: impl Into<String>) -> Text {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

impl DraftsPage {
    pub(super) fn body(&self, window: &Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let registry = self.services.drafts().read(cx);
        let reason = match (registry.unavailable(), registry.listing_failure()) {
            (Some(unavailable), _) => Some(unavailable.to_string()),
            (None, Some(kind)) => Some(DraftError { kind }.to_string()),
            (None, None) => None,
        };
        if let Some(reason) = reason {
            return vec![text("drafts-reason", reason).text_sm().into_any_element()];
        }
        if self.slots.is_empty() {
            let note = match registry.listing() {
                ListState::Loading => text("drafts-loading", LOADING_DRAFTS),
                _ => text("drafts-empty", NO_DRAFTS),
            };
            return vec![note.text_sm().into_any_element()];
        }
        self.slots
            .iter()
            .map(|slot| self.row(slot, window, cx))
            .collect()
    }

    fn row(&self, slot: &Slot, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let row = &slot.row;
        let id = row.id;
        let theme = cx.theme();
        let (muted, border, ring) = (theme.muted_foreground, theme.border, theme.ring);
        let focus = &self.focus[&id];
        let mut details = h_flex().gap_2().flex_wrap().items_center();
        if !row.kind_label.is_empty() {
            details = details.child(text(row_id(id, "kind"), row.kind_label.clone()).text_sm());
        }
        if !row.title.is_empty() {
            details = details.child(
                text(row_id(id, "title"), row.title.clone())
                    .text_sm()
                    .font_weight(gpui_kit::FontWeight::BOLD),
            );
        }
        if !row.repository.is_empty() {
            details = details.child(
                text(row_id(id, "repository"), row.repository.clone())
                    .text_xs()
                    .text_color(muted),
            );
        }
        if !row.flushed_at.is_empty() {
            details = details.child(
                text(
                    row_id(id, "saved"),
                    format!("{SAVED_AT} {}", row.flushed_at),
                )
                .text_xs()
                .text_color(muted),
            );
        }
        let comment = slot
            .draft
            .as_ref()
            .is_some_and(|draft| draft.target.comment.is_some());
        let note = (comment && matches!(row.state, RowState::Readable { held: Held::No, .. }))
            .then(|| {
                text(row_id(id, "note"), COMMENT_LATER)
                    .text_xs()
                    .text_color(muted)
            });
        // Nothing would be lost: Discard comes first.
        let mut order: Vec<usize> = (0..ACTIONS.len()).collect();
        if matches!(row.target(), Some(TargetState::AlreadySaved)) {
            order.sort_by_key(|ix| ACTIONS[*ix].0 != "discard");
        }
        let actions = order
            .into_iter()
            .filter(|ix| offered(row.actions, ACTIONS[*ix].0))
            .map(|ix| self.action(id, ix, &focus.actions[ix], cx));
        let mut element = v_flex()
            .id(row_id(id, "row"))
            .test_support()
            .aria_label(state_label(row))
            .relative()
            .track_focus(&focus.row)
            .gap_1()
            .p_2()
            .rounded_sm()
            .border_1()
            .border_color(border)
            .child(details)
            .child(text(row_id(id, "state"), state_label(row)).text_sm())
            .children(note)
            .child(h_flex().gap_2().flex_wrap().children(actions))
            .child(self.scroll.marker(&focus.row));
        if focus.row.is_focused(window) {
            element = element.border_2().border_color(ring);
        }
        element.into_any_element()
    }

    /// The action `ACTIONS[ix]` of row `id`.
    fn action(
        &self,
        id: DraftId,
        ix: usize,
        focus: &FocusHandle,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (part, label) = ACTIONS[ix];
        let wrapper = ElementId::from(row_id(id, part));
        let mut button = Button::new(ElementId::NamedChild(
            Arc::new(wrapper.clone()),
            "button".into(),
        ))
        .label(label);
        button = match part {
            "show" => button
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_tab_of(id, cx))),
            "retry" => {
                button.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.retry(id, cx)))
            }
            // Restore, Inspect and Discard arrive with the page's actions.
            _ => button,
        };
        div()
            .id(wrapper)
            .test_support()
            .aria_label(label)
            .relative()
            .track_focus(focus)
            .child(button)
            .child(self.scroll.marker_within(focus))
            .into_any_element()
    }
}
