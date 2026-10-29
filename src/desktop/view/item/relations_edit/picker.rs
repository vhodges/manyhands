//! The ticket picker that Add… and Change… open (design, Relationships
//! (contract 10)): a filter box over `list_tickets` of the tab's own
//! repository, never another's, read when it opens. A title holding the
//! filter is offered, ignoring case; a short code is matched only
//! exactly, through `find_tickets_by_slug`, and offered first. The ticket
//! itself is never offered, nor, from Add…, a ticket it already waits on,
//! nor, from Change…, its parent; the check still explains a cycle the
//! user makes.
//!
//! It is reached by keyboard: the filter takes focus as it opens, Enter in
//! it takes the first offer, Tab reaches each offer, and a click, Enter or
//! Space takes one. Escape closes it, not the sheet, and focus goes back
//! to the Add… or Change… that opened it, as it does once one is taken.
//! Its text is drawn in the sheet's body, a `draft-section` while editing
//! (R-38).

use std::path::PathBuf;

use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled, Subscription, Task,
    TestSupportExt, Window,
    component::{
        ActiveTheme, Sizable,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div, px,
};
use manyhands::repository::{ItemListDto, TicketFilter};

use super::super::{ItemPane, text};
use crate::desktop::{
    keys::{ClosePicker, RELATION_PICKER_CONTEXT},
    model::{
        list_state::LABEL_LOADING,
        relations_edit::{
            Candidate, FILTER, NO_MATCH, NOT_LISTED, Proposed, RelationField, offered,
        },
        stamp::Stamp,
    },
    services::{ReadKind, ReadResult},
};

/// What the picker's list read gave.
enum Listed {
    Loading,
    Ready(Vec<Candidate>),
    Failed,
}

/// The open picker.
pub(super) struct Picker {
    field: RelationField,
    input: Entity<InputState>,
    listed: Listed,
    /// The library's tickets whose short code is a filter, and that filter.
    exact: Option<(String, Vec<Candidate>)>,
    /// Held so the read is not dropped; replacing the picker drops it.
    _list_read: Option<Task<()>>,
    slug_read: Option<Task<()>>,
    /// One per offer, by its place.
    rows: Vec<FocusHandle>,
    _input: Subscription,
}

impl ItemPane {
    /// Add… or Change…: the picker opens under `field`, its filter focused,
    /// and the tab's repository's tickets are read.
    #[allow(clippy::result_large_err)]
    pub(in crate::desktop::view::item) fn open_picker(
        &mut self,
        field: RelationField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.controls_editable() {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(FILTER));
        let events =
            cx.subscribe_in(
                &input,
                window,
                |pane, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => pane.picker_filtered(cx),
                    InputEvent::PressEnter { .. } => {
                        if let Some(first) = pane.picker_offers(cx).into_iter().next() {
                            pane.picker_take(first, window, cx);
                        }
                    }
                    InputEvent::Focus | InputEvent::Blur => {}
                },
            );
        input.update(cx, |input, cx| input.focus(window, cx));
        let stamp = self.relations.lists.next(self.relations.scope.clone());
        let root = PathBuf::from(self.repo());
        let read = self.services.read(
            ReadKind::Tickets,
            stamp,
            move |service| {
                let repo = service.resolve_repository(&root)?;
                service.list_tickets(&repo, &TicketFilter::default())
            },
            cx,
            |pane: &mut Self, stamp, result, cx| pane.picker_listed(stamp, result, cx),
        );
        self.relations.picker = Some(Picker {
            field,
            input,
            listed: Listed::Loading,
            exact: None,
            _list_read: Some(read),
            slug_read: None,
            rows: Vec::new(),
            _input: events,
        });
        cx.notify();
    }

    /// The list read landed: its tickets, which also name them elsewhere.
    fn picker_listed(
        &mut self,
        stamp: Stamp,
        result: ReadResult<ItemListDto>,
        cx: &mut Context<Self>,
    ) {
        let relations = &mut self.relations;
        let Some(picker) = &mut relations.picker else {
            return;
        };
        if !relations.lists.is_current(&stamp) {
            return;
        }
        picker.listed = match result {
            ReadResult::Ok(list) => {
                let listed = Candidate::of_list(&list);
                for candidate in &listed {
                    relations
                        .known
                        .insert(candidate.id.clone(), candidate.clone());
                }
                Listed::Ready(listed)
            }
            ReadResult::Busy | ReadResult::Failed(_) => Listed::Failed,
        };
        cx.notify();
    }

    /// The filter changed: the tickets whose short code is exactly it are
    /// read.
    #[allow(clippy::result_large_err)]
    fn picker_filtered(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = &self.relations.picker else {
            return;
        };
        let filter = picker.input.read(cx).value().trim().to_owned();
        let stamp = self.relations.slugs.next(self.relations.scope.clone());
        let read = (!filter.is_empty()).then(|| {
            let root = PathBuf::from(self.repo());
            let slug = filter.clone();
            self.services.read(
                ReadKind::FindSlug,
                stamp,
                move |service| {
                    let repo = service.resolve_repository(&root)?;
                    service.find_tickets_by_slug(&repo, &slug)
                },
                cx,
                move |pane: &mut Self, stamp, result, cx| {
                    pane.picker_matched(stamp, filter, result, cx)
                },
            )
        });
        if let Some(picker) = &mut self.relations.picker {
            picker.slug_read = read;
        }
        cx.notify();
    }

    /// The exact short-code read for `filter` landed.
    fn picker_matched(
        &mut self,
        stamp: Stamp,
        filter: String,
        result: ReadResult<ItemListDto>,
        cx: &mut Context<Self>,
    ) {
        let relations = &mut self.relations;
        let Some(picker) = &mut relations.picker else {
            return;
        };
        if !relations.slugs.is_current(&stamp) {
            return;
        }
        let matches = match result {
            ReadResult::Ok(list) => Candidate::of_list(&list),
            ReadResult::Busy | ReadResult::Failed(_) => Vec::new(),
        };
        for candidate in &matches {
            relations
                .known
                .insert(candidate.id.clone(), candidate.clone());
        }
        picker.exact = Some((filter, matches));
        cx.notify();
    }

    /// What the picker offers now: never the ticket itself, nor what its
    /// field already holds.
    fn picker_offers(&self, cx: &App) -> Vec<Candidate> {
        let Some(picker) = &self.relations.picker else {
            return Vec::new();
        };
        let filter = picker.input.read(cx).value();
        let listed = match &picker.listed {
            Listed::Ready(listed) => listed.as_slice(),
            Listed::Loading | Listed::Failed => &[],
        };
        let exact = picker
            .exact
            .as_ref()
            .filter(|(answered, _)| answered == filter.trim())
            .map_or(&[][..], |(_, matches)| matches.as_slice());
        let proposed = self
            .session_meta()
            .and_then(Proposed::of)
            .unwrap_or_default();
        let own = self.own_id().map(ToString::to_string);
        let mut left_out: Vec<&str> = own.as_deref().into_iter().collect();
        match picker.field {
            RelationField::WaitingOn => left_out.extend(proposed.deps.iter().map(String::as_str)),
            RelationField::Parent => left_out.extend(proposed.parent.as_deref()),
        }
        offered(listed, exact, &filter, &left_out)
    }

    /// `candidate` taken: it is a dependency, or the parent, the picker
    /// closes, and focus goes back to its opener.
    fn picker_take(&mut self, candidate: Candidate, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picker) = self.relations.picker.take() else {
            return;
        };
        let Some(proposed) = self.session_meta().and_then(Proposed::of) else {
            return;
        };
        self.relations
            .known
            .insert(candidate.id.clone(), candidate.clone());
        let (next, opener) = match picker.field {
            RelationField::WaitingOn => (
                proposed.with_dependency(&candidate.id),
                self.relations.add.clone(),
            ),
            RelationField::Parent => (
                proposed.with_parent(Some(&candidate.id)),
                self.relations.change.clone(),
            ),
        };
        self.change_relations(next, picker.field, cx);
        window.focus(&opener, cx);
        cx.notify();
    }

    /// Escape: the picker closes, and focus goes back to its opener.
    fn close_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picker) = self.relations.picker.take() else {
            return;
        };
        let opener = match picker.field {
            RelationField::WaitingOn => &self.relations.add,
            RelationField::Parent => &self.relations.change,
        };
        window.focus(opener, cx);
        cx.notify();
    }

    /// The picker, under `field`'s row while it is open from there. It goes
    /// once the fields are no longer controls (R-74).
    pub(in crate::desktop::view::item) fn picker_view(
        &mut self,
        field: RelationField,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.controls_editable() {
            self.relations.picker = None;
            return None;
        }
        if self.relations.picker.as_ref()?.field != field {
            return None;
        }
        let offers = self.picker_offers(cx);
        let picker = self.relations.picker.as_mut()?;
        while picker.rows.len() < offers.len() {
            picker.rows.push(cx.focus_handle().tab_stop(true));
        }
        let rows = picker.rows.clone();
        let input = picker.input.clone();
        let status = match (&picker.listed, offers.is_empty()) {
            (_, false) => None,
            (Listed::Loading, true) => Some(LABEL_LOADING),
            (Listed::Failed, true) => Some(NOT_LISTED),
            (Listed::Ready(_), true) => Some(NO_MATCH),
        };
        let theme = cx.theme();
        let (muted, border, ring) = (theme.muted_foreground, theme.border, theme.ring);
        let offers: Vec<AnyElement> = offers
            .into_iter()
            .zip(rows)
            .enumerate()
            .map(|(ix, (candidate, focus))| {
                let label = candidate.named();
                let focused = focus.is_focused(window);
                div()
                    .id(("rel-pick", ix))
                    .test_support()
                    .track_focus(&focus)
                    .role(Role::Button)
                    .aria_label(label.clone())
                    .relative()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(if focused { ring } else { border })
                    .text_sm()
                    .cursor_pointer()
                    .child(label)
                    .child(self.sheet_scroll.marker(&focus))
                    .on_click(cx.listener(move |pane, _: &ClickEvent, window, cx| {
                        pane.picker_take(candidate.clone(), window, cx)
                    }))
                    .into_any_element()
            })
            .collect();
        Some(
            v_flex()
                .id("rel-picker")
                .test_support()
                .key_context(RELATION_PICKER_CONTEXT)
                .on_action(
                    cx.listener(|pane, _: &ClosePicker, window, cx| pane.close_picker(window, cx)),
                )
                .gap_1()
                .p_2()
                .rounded(px(6.))
                .border_1()
                .border_color(border)
                .child(
                    div()
                        .id("rel-picker-filter")
                        .test_support()
                        .aria_label(FILTER)
                        .child(Input::new(&input).small().aria_label(FILTER)),
                )
                .children(offers)
                .children(
                    status
                        .map(|status| text("rel-picker-none", status).text_xs().text_color(muted)),
                )
                .into_any_element(),
        )
    }

    /// The picker's filter box, while it is open.
    #[cfg(test)]
    pub(crate) fn picker_filter(&self) -> Option<Entity<InputState>> {
        Some(self.relations.picker.as_ref()?.input.clone())
    }
}
