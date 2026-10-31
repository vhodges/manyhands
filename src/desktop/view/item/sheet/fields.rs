//! The details sheet's named controls while editing (design, Named
//! controls in the details sheet; R-27, R-38, R-57, R-74; decision 14).
//!
//! - **Which.** A document's Title; a ticket's Title, Type, Status,
//!   Project and Team (`model::edit::fields`). Everything else the sheet
//!   shows stays a row, never a control.
//! - **When.** While the tab is Editing or Saving, the named rows are Kit
//!   inputs that write into the session's fields through `change_fields`,
//!   which marks the tab dirty and feeds the draft exactly as typing does;
//!   a field put back to the saved copy's value leaves the tab clean. While
//!   it is Preparing, Loading or in the review, and in a session kept after
//!   Done, they are the read-only rows (R-74): the review's show the
//!   session's fields, the others the item as read.
//! - **Their values** are the session's fields, set into each input as a
//!   frame is drawn wherever they differ: a clean session that takes a late
//!   read, or the review's choices used. A dirty session is never
//!   reloaded, so a changed control is never replaced by a late read.
//! - **Opening.** A chip's field focuses its control (`sheet_target`).
//! - **Required.** An empty Title, Type or Status draws a red border and
//!   its message under it; every save waits until it is filled
//!   (`edit::save_view::save_blocked`).
//! - **Suggestions** (R-57) under a focused Type, Status, Project or Team:
//!   the values on the tickets the workspace's ticket list loaded for this
//!   tab's repository, with the item's own as read, given to the tab each
//!   frame the sheet is open (`take_loaded_tickets`). A suggestion chosen
//!   is the field's value; any typed value is kept.
//! - **Saving.** `secondary-s` inside the sheet, which renders in Root's
//!   layer outside the tab, saves the tab; during the review it does
//!   nothing (R-74).
//!
//! The controls hold the user's text, so nothing here derives `Debug`.

use gpui_kit::{
    AnyElement, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled,
    Subscription, TestSupportExt, Window,
    component::{
        ActiveTheme, Sizable,
        button::Button,
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div, px,
};

use super::{LABEL_SIZE, Row};
use crate::desktop::{
    keys::Save,
    model::{
        edit::{
            Phase,
            fields::{self, Field, Suggestions},
        },
        tickets::Tickets,
    },
    view::item::{ChipField, ItemPane, text},
};

/// The named field `chip` opens the sheet on, if it has one.
fn field_of(chip: ChipField) -> Option<Field> {
    match chip {
        ChipField::All => Some(Field::Title),
        ChipField::Type => Some(Field::Type),
        ChipField::Status => Some(Field::Status),
        ChipField::Project => Some(Field::Project),
        ChipField::Team => Some(Field::Team),
        _ => None,
    }
}

/// The element IDs of `field`'s control, its message and, but for the
/// Title, which has none, its suggestions.
fn ids(field: Field) -> (&'static str, &'static str, Option<&'static str>) {
    match field {
        Field::Title => ("sheet-title-input", "sheet-title-required", None),
        Field::Type => (
            "sheet-type-input",
            "sheet-type-required",
            Some("sheet-type-suggestion"),
        ),
        Field::Status => (
            "sheet-status-input",
            "sheet-status-required",
            Some("sheet-status-suggestion"),
        ),
        Field::Project => (
            "sheet-project-input",
            "sheet-project-required",
            Some("sheet-project-suggestion"),
        ),
        Field::Team => (
            "sheet-team-input",
            "sheet-team-required",
            Some("sheet-team-suggestion"),
        ),
    }
}

/// One named field's control.
struct Control {
    field: Field,
    input: Entity<InputState>,
    /// Around the control and its suggestions: the row is revealed, and its
    /// suggestions shown, while focus is anywhere in it. It is no tab stop:
    /// the control is.
    holder: FocusHandle,
    _changed: Subscription,
}

/// The sheet's controls of the tab's session, and the values it suggests.
#[derive(Default)]
pub(in crate::desktop::view::item) struct SheetFields {
    controls: Vec<Control>,
    suggestions: Suggestions,
}

impl SheetFields {
    fn control(&self, field: Field) -> Option<&Control> {
        self.controls.iter().find(|control| control.field == field)
    }
}

impl ItemPane {
    /// Whether the sheet's named fields are controls now: the session is
    /// shown, Editing or Saving; not while Preparing, Loading or in the
    /// review, nor while the session is kept (R-74).
    pub(in crate::desktop::view::item) fn controls_editable(&self) -> bool {
        self.editing() && matches!(self.edit.phase(), Phase::Editing | Phase::Saving)
    }

    /// As the sheet is drawn: the controls are made, once per session, and
    /// each takes the session's value where it holds another. With no
    /// session they go.
    pub(super) fn sync_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_session() {
            self.fields.controls.clear();
            return;
        }
        if !self.controls_editable() {
            return;
        }
        let Some(meta) = self.session_meta().cloned() else {
            return;
        };
        if self.fields.controls.is_empty() {
            self.fields.controls = Field::of(&meta)
                .iter()
                .map(|field| Self::control_for(*field, window, cx))
                .collect();
        }
        for control in &self.fields.controls {
            let wanted = fields::value(&meta, control.field).unwrap_or_default();
            if control.input.read(cx).value().as_ref() != wanted {
                let wanted = wanted.to_owned();
                control
                    .input
                    .update(cx, |input, cx| input.set_value(wanted, window, cx));
            }
        }
    }

    fn control_for(field: Field, window: &mut Window, cx: &mut Context<Self>) -> Control {
        let input = cx.new(|cx| InputState::new(window, cx));
        let changed = cx.subscribe(&input, move |pane, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                pane.field_typed(field, cx);
            }
        });
        Control {
            field,
            input,
            holder: cx.focus_handle(),
            _changed: changed,
        }
    }

    /// What was typed into `field`'s control goes into the session's
    /// fields, exactly as typed; an emptied Project or Team is absent
    /// (R-27).
    fn field_typed(&mut self, field: Field, cx: &mut Context<Self>) {
        if !self.controls_editable() {
            return;
        }
        let Some(control) = self.fields.control(field) else {
            return;
        };
        let typed = control.input.read(cx).value();
        let Some(meta) = self.session_meta() else {
            return;
        };
        let changed = fields::with_value(meta, field, &typed);
        if changed == *meta {
            cx.notify();
        } else {
            self.change_fields(changed, cx);
        }
    }

    /// A suggestion chosen: the field's value, focus kept in its control.
    /// The input replaces its text as one edit it can undo (Task 20 fix
    /// 1). The value reaches the session now, through `field_typed`, before
    /// any frame can set the control back from the session's fields; the
    /// input's own change event, which follows, then finds nothing to
    /// change.
    fn take_suggestion(
        &mut self,
        field: Field,
        value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(input) = self
            .fields
            .control(field)
            .map(|control| control.input.clone())
        else {
            return;
        };
        input.update(cx, |input, cx| {
            input.replace_all(value, window, cx);
            input.focus(window, cx);
        });
        self.field_typed(field, cx);
    }

    /// What `field`'s chip focuses as the sheet opens: its control while the
    /// fields are controls, else its row (fact 80).
    pub(super) fn sheet_target(
        &mut self,
        field: ChipField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> FocusHandle {
        self.sync_controls(window, cx);
        let control = field_of(field)
            .filter(|_| self.controls_editable())
            .and_then(|named| self.fields.control(named));
        match control {
            Some(control) => control.input.read(cx).focus_handle(cx),
            None => self.sheet_focus[field.index()].clone(),
        }
    }

    /// `field`'s value in its read-only row: the session's while one is
    /// shown, as in the review, else the item's as read, `read`.
    pub(super) fn named_value(&self, field: Field, read: Option<&String>) -> Option<String> {
        match self.session_meta() {
            Some(meta) => fields::value(meta, field)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            None => read.cloned(),
        }
    }

    /// `row`, the row of `field`, as a control: its label, the Kit input,
    /// its message while it is required and empty, and its suggestions
    /// while focus is in it. `None` while the fields are not controls.
    pub(super) fn control_row(
        &self,
        row: Row,
        field: Field,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.controls_editable() {
            return None;
        }
        let control = self.fields.control(field)?;
        let meta = self.session_meta()?;
        let (input_id, required_id, suggestion_id) = ids(field);
        let danger = cx.theme().danger;
        let message = fields::required_message(meta, field);
        let label = h_flex()
            .gap(px(2.))
            .text_size(px(LABEL_SIZE))
            .font_weight(gpui_kit::FontWeight::MEDIUM)
            .child(row.label)
            .children(row.required.then(|| div().text_color(danger).child("*")));
        let mut input = Input::new(&control.input).aria_label(row.label);
        if message.is_some() {
            input = input.border_color(danger);
        }
        let shown = field.suggested() && control.holder.contains_focused(window, cx);
        let typed = control.input.read(cx).value();
        let offered = if shown {
            self.fields.suggestions.offered(field, &typed)
        } else {
            Vec::new()
        };
        let suggestions = suggestion_id
            .filter(|_| !offered.is_empty())
            .map(|suggestion_id| {
                h_flex()
                    .flex_wrap()
                    .gap_1()
                    .children(offered.into_iter().enumerate().map(|(ix, value)| {
                        let chosen = value.clone();
                        Button::new((suggestion_id, ix))
                            .label(value)
                            .xsmall()
                            .outline()
                            .on_click(cx.listener(move |pane, _: &ClickEvent, window, cx| {
                                pane.take_suggestion(field, chosen.clone(), window, cx)
                            }))
                    }))
            });
        let element = v_flex()
            .id(row.id)
            .test_support()
            .role(Role::Group)
            .aria_label(row.name())
            .track_focus(&control.holder)
            .relative()
            .gap(px(6.))
            .child(label)
            .child(
                div()
                    .id(input_id)
                    .test_support()
                    .aria_label(row.label)
                    .child(input),
            )
            .children(
                message.map(|message| text(required_id, message).text_xs().text_color(danger)),
            )
            .children(suggestions)
            .child(self.sheet_scroll.marker_within(&control.holder));
        Some(element.into_any_element())
    }

    /// The tickets the workspace has loaded, given as each frame with the
    /// sheet open is drawn: while the fields are controls, the values on
    /// those of this tab's repository, and the item's own as read, are
    /// what the sheet suggests (R-57).
    pub(in crate::desktop::view) fn take_loaded_tickets(&mut self, tickets: &Tickets) {
        if !self.controls_editable() {
            return;
        }
        let rows = tickets
            .state()
            .rows()
            .filter(|_| tickets.repo().map(String::as_str) == Some(self.repo()));
        let mut suggestions = Suggestions::of_rows(rows.into_iter().flatten());
        if let Some(item) = self.model.item().rows() {
            suggestions.add(Field::Type, item.ticket_type.as_deref());
            suggestions.add(Field::Status, item.status.as_deref());
            suggestions.add(Field::Project, item.project.as_deref());
            suggestions.add(Field::Team, item.team.as_deref());
        }
        self.fields.suggestions = suggestions;
    }

    /// `secondary-s` inside the sheet: the tab's Save, but nothing during
    /// the review (R-74). It goes no further: no workspace key acts behind
    /// the sheet (R-60).
    pub(super) fn sheet_save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        if self.edit.phase() == Phase::Review {
            return;
        }
        self.save_requested(window, cx);
    }

    /// The control of `field`, once made.
    #[cfg(all(test, feature = "desktop-test-support"))]
    pub(crate) fn sheet_input(&self, field: Field) -> Option<Entity<InputState>> {
        Some(self.fields.control(field)?.input.clone())
    }
}
