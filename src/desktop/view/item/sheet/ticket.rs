//! A ticket's details (design, Details sheet, Ticket): Title, Type and
//! Status, required; Project and Team, each a control while editing
//! (`fields`); then its relationships: Parent, Waiting on with the
//! readiness note, Dependents and Children, each D1's rows, and the
//! relationship problems (R-54), Parent and Waiting on holding the
//! session's relationships while it is shown (`relations_edit`); then
//! Other fields, the Facts, the Problems and the Inspect link.

use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Div, ElementId, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Role, Stateful, StatefulInteractiveElement,
    Styled, TestSupportExt, Window,
    component::{ActiveTheme, Icon, h_flex, v_flex},
    div, px,
};
use manyhands::repository::{ItemDto, ItemDtoKind, ReadinessState};

use super::{super::chips::waiting_on, BOX_RADIUS, GAP, LABEL_SIZE, Row, value};
use crate::desktop::{
    assets::icon_path,
    model::{
        edit::fields::Field,
        relations_edit::{
            ADD, CHANGE, CLEAR, NOT_FOUND_KEPT, Proposed, REMOVE, RelationField, SAVE_CHECKS_AGAIN,
        },
        relationships::{NONE, RELATIONSHIPS, RESOLVED, Relationships},
        tickets::Lifecycle,
    },
    view::{
        item::{ChipField, ItemPane, text},
        tickets::lifecycle_label,
    },
};

const TYPE: Row = Row::required("sheet-type", "Type");
const STATUS: Row = Row::required("sheet-status", "Status");
const PROJECT: Row = Row::new("sheet-project", "Project");
const TEAM: Row = Row::new("sheet-team", "Team");
const PARENT: Row = Row::new("sheet-parent", "Parent");
const WAITING_ON: Row = Row::new("sheet-waiting-on", "Waiting on");
const DEPENDENTS: Row = Row::new("sheet-dependents", "Dependents");
const CHILDREN: Row = Row::new("sheet-children", "Children");

/// A ready ticket that depends on nothing.
pub(crate) const WAITS_ON_NOTHING: &str =
    "This ticket waits on no other ticket, so it is ready to work on.";

/// What the ticket's readiness says of its dependencies: the closed ones a
/// ready ticket depended on, or what a blocked one waits on, by short code
/// once the tree named them, else by ID. A closed ticket has no note.
pub(super) fn readiness_note(
    item: &ItemDto,
    relationships: Option<&Relationships>,
) -> Option<String> {
    let readiness = item.readiness.as_ref()?;
    match readiness.state {
        ReadinessState::Blocked => Some(format!(
            "This ticket waits on {}, so it is blocked.",
            waiting_on(item, relationships)?
        )),
        ReadinessState::Ready => {
            let names: Vec<String> = relationships
                .map(Relationships::dependency_rows)
                .unwrap_or_default()
                .iter()
                .map(|row| row.name().short_or(&row.id).to_owned())
                .collect();
            Some(match names.as_slice() {
                [] => WAITS_ON_NOTHING.to_owned(),
                [one] => format!("{one} is closed, so this ticket is ready to work on."),
                many => format!(
                    "{} are closed, so this ticket is ready to work on.",
                    many.join(", ")
                ),
            })
        }
        ReadinessState::Closed => None,
    }
}

/// A chip's icon.
const ICON_SIZE: f32 = 12.;

/// A chip or control, as the tests find it.
type Drawn = gpui_kit::base::ObservedElement<Stateful<Div>>;

/// The element IDs of a chip and its cells.
struct ChipIds {
    chip: ElementId,
    name: ElementId,
    state: ElementId,
    id: ElementId,
}

impl ChipIds {
    /// The dependency chip at `ix`'s.
    fn dependency(ix: usize) -> Self {
        Self {
            chip: ("rel-dep-chip", ix).into(),
            name: ("rel-dep-name", ix).into(),
            state: ("rel-dep-state", ix).into(),
            id: ("rel-dep-id", ix).into(),
        }
    }

    /// The parent chip's.
    fn parent() -> Self {
        Self {
            chip: "rel-parent-chip".into(),
            name: "rel-parent-name".into(),
            state: "rel-parent-state".into(),
            id: "rel-parent-id".into(),
        }
    }
}

/// A focusable control of the rows: `label`, as a button, with `focus`. A
/// click, Enter or Space activates it.
fn action(
    id: impl Into<ElementId>,
    label: &'static str,
    focus: &FocusHandle,
    window: &Window,
    cx: &App,
) -> Drawn {
    let theme = cx.theme();
    let focused = focus.is_focused(window);
    let element = div()
        .id(id)
        .test_support()
        .track_focus(focus)
        .role(Role::Button)
        .aria_label(label)
        .relative()
        .flex_none()
        .px_2()
        .py(px(2.))
        .rounded(px(4.))
        .border_1()
        .border_color(if focused { theme.ring } else { theme.border })
        .text_xs()
        .cursor_pointer();
    if focused { element.border_2() } else { element }
}

/// A chip's icon: its lifecycle, or not found.
fn state_icon(lifecycle: Option<Lifecycle>, unresolved: bool) -> Icon {
    let name = match (unresolved, lifecycle) {
        (true, _) => "circle-alert",
        (false, Some(Lifecycle::Closed)) => "circle-check",
        (false, _) => "ticket",
    };
    Icon::default()
        .path(icon_path(name))
        .w(px(ICON_SIZE))
        .h(px(ICON_SIZE))
}

impl ItemPane {
    pub(super) fn ticket_rows(
        &mut self,
        item: &ItemDto,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let focus = |pane: &Self, field: ChipField| pane.sheet_focus[field.index()].clone();
        let mut rows = vec![self.title_row(item, window, cx)];
        for (row, id, chip, field, read) in [
            (
                TYPE,
                "item-type",
                ChipField::Type,
                Field::Type,
                &item.ticket_type,
            ),
            (
                STATUS,
                "item-status",
                ChipField::Status,
                Field::Status,
                &item.status,
            ),
            (
                PROJECT,
                "item-project",
                ChipField::Project,
                Field::Project,
                &item.project,
            ),
            (TEAM, "item-team", ChipField::Team, Field::Team, &item.team),
        ] {
            // While editing, a control; else read only, the session's value
            // during the review (R-74).
            if let Some(control) = self.control_row(row, field, window, cx) {
                rows.push(control);
                continue;
            }
            let shown = self.named_value(field, read.as_ref());
            let focus = focus(self, chip);
            rows.push(self.row(row, value(id, shown.as_ref()), Some(&focus), window, cx));
        }
        rows.extend(self.relationship_rows(item, window, cx));
        rows.push(self.other_fields(item, &[], cx));
        rows.push(self.facts(item, ItemDtoKind::Ticket, window, cx));
        rows.extend(self.file_rows(item, cx));
        rows
    }

    /// Parent, Waiting on, Dependents and Children, each holding its group
    /// of D1's rows, then the relationship problems, under one name. While
    /// the session is shown, Parent and Waiting on hold its relationships
    /// instead (`relations_edit`), and "Save checks again" may follow;
    /// Dependents and Children stay D1's (R-54).
    fn relationship_rows(
        &mut self,
        item: &ItemDto,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let groups = self.relationship_groups(window, cx)?;
        let muted = cx.theme().muted_foreground;
        let note = readiness_note(item, self.model.relationships()).map(|note| {
            text("sheet-readiness", note)
                .text_xs()
                .text_color(muted)
                .into_any_element()
        });
        let parent_focus = self.sheet_focus[ChipField::Parent.index()].clone();
        let waiting_focus = self.sheet_focus[ChipField::WaitingOn.index()].clone();
        let (parent, waiting) = if self.editing() {
            let parent = self.parent_edit(window, cx);
            let waiting = self.waiting_on_edit(note, window, cx);
            (
                self.edit_row(PARENT, parent, &parent_focus, window, cx),
                self.edit_row(WAITING_ON, waiting, &waiting_focus, window, cx),
            )
        } else {
            let waiting = v_flex()
                .gap_1()
                .child(groups.dependencies)
                .children(note)
                .into_any_element();
            (
                self.row(PARENT, Some(groups.parent), Some(&parent_focus), window, cx),
                self.row(WAITING_ON, Some(waiting), Some(&waiting_focus), window, cx),
            )
        };
        // While the session is shown: a kept one views the saved copy.
        let index_note = self.editing().then(|| self.relations_note(cx)).flatten();
        Some(
            v_flex()
                .id("relationships")
                .test_support()
                .aria_label(RELATIONSHIPS)
                .gap(px(GAP))
                .child(parent)
                .child(waiting)
                .child(self.row(DEPENDENTS, Some(groups.dependents), None, window, cx))
                .child(self.row(CHILDREN, Some(groups.children), None, window, cx))
                .children(groups.problems)
                .children(index_note)
                .into_any_element(),
        )
    }

    /// `row` while the session is shown: its label over `content`, its
    /// relationship controls, outside the read-only box. The row is a tab
    /// stop, ringed while focused and revealed, as the chip that opens the
    /// sheet on it focuses it (fact 80).
    fn edit_row(
        &self,
        row: Row,
        content: AnyElement,
        focus: &FocusHandle,
        window: &Window,
        cx: &App,
    ) -> AnyElement {
        let theme = cx.theme();
        let focused = focus.is_focused(window);
        let label = div()
            .text_size(px(LABEL_SIZE))
            .font_weight(FontWeight::MEDIUM)
            .child(row.label);
        v_flex()
            .id(row.id)
            .test_support()
            .role(Role::Group)
            .aria_label(row.name())
            .track_focus(focus)
            .relative()
            .gap(px(6.))
            .child(label)
            .child(
                div()
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(BOX_RADIUS))
                    .border_1()
                    .border_color(if focused { theme.ring } else { theme.input })
                    .bg(theme.background)
                    .child(content),
            )
            .child(self.sheet_scroll.marker(focus))
            .into_any_element()
    }

    /// A related ticket's chip: its icon, its name, its state and its ID,
    /// as `ids` name them.
    fn relation_chip(&self, id: &str, ids: ChipIds, cx: &App) -> Drawn {
        let candidate = self.relation_candidate(id);
        let unresolved = self.relation_unresolved(id);
        let lifecycle = candidate.as_ref().and_then(|candidate| candidate.lifecycle);
        let state = match (unresolved, lifecycle) {
            (true, _) => NOT_FOUND_KEPT,
            (false, Some(lifecycle)) => lifecycle_label(lifecycle),
            (false, None) => RESOLVED,
        };
        let shown = candidate.map_or_else(|| id.to_owned(), |candidate| candidate.named());
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        h_flex()
            .id(ids.chip)
            .test_support()
            .aria_label(format!("{shown}, {state}"))
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded_sm()
            .border_1()
            .border_color(theme.border)
            .child(state_icon(lifecycle, unresolved))
            .child(text(ids.name, shown).text_sm())
            .child(text(ids.state, state).text_xs().text_color(muted))
            .child(text(ids.id, id.to_owned()).text_xs().text_color(muted))
    }

    /// The rejection that holds, in words, under `field`, as `id`.
    fn relation_problem(
        &self,
        field: RelationField,
        id: &'static str,
        cx: &App,
    ) -> Option<AnyElement> {
        let words = self.relation_explanation(field)?;
        Some(
            text(id, words)
                .text_xs()
                .text_color(cx.theme().danger)
                .into_any_element(),
        )
    }

    /// Waiting on while the session is shown: its chips, Add…, the picker
    /// opened from it, the rejection under it, then `note`, the readiness
    /// note.
    fn waiting_on_edit(
        &mut self,
        note: Option<AnyElement>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editable = self.controls_editable();
        let deps = self
            .session_meta()
            .and_then(Proposed::of)
            .map(|proposed| proposed.deps)
            .unwrap_or_default();
        while self.relations.remove.len() < deps.len() {
            self.relations.remove.push(cx.focus_handle().tab_stop(true));
        }
        let chips: Vec<AnyElement> = deps
            .iter()
            .enumerate()
            .map(|(ix, id)| {
                let chip = self.relation_chip(id, ChipIds::dependency(ix), cx);
                let remove = editable.then(|| {
                    let focus = self.relations.remove[ix].clone();
                    action(("rel-dep-remove", ix), REMOVE, &focus, window, cx)
                        .child(
                            Icon::default()
                                .path(icon_path("close"))
                                .w(px(10.))
                                .h(px(10.)),
                        )
                        .child(self.sheet_scroll.marker(&focus))
                        .on_click(cx.listener(move |pane, _: &ClickEvent, window, cx| {
                            pane.remove_dependency(ix, window, cx)
                        }))
                });
                chip.children(remove).into_any_element()
            })
            .collect();
        let muted = cx.theme().muted_foreground;
        let empty = deps
            .is_empty()
            .then(|| text("rel-deps-none", NONE).text_xs().text_color(muted));
        let add = editable.then(|| {
            let focus = self.relations.add.clone();
            action("rel-deps-add", ADD, &focus, window, cx)
                .child(ADD)
                .child(self.sheet_scroll.marker(&focus))
                .on_click(cx.listener(|pane, _: &ClickEvent, window, cx| {
                    pane.open_picker(RelationField::WaitingOn, window, cx)
                }))
        });
        let picker = self.picker_view(RelationField::WaitingOn, window, cx);
        let problem = self.relation_problem(RelationField::WaitingOn, "rel-deps-problem", cx);
        v_flex()
            .gap_1()
            .children(chips)
            .children(empty)
            .child(h_flex().children(add))
            .children(picker)
            .children(problem)
            .children(note)
            .into_any_element()
    }

    /// Parent while the session is shown: its chip, Change… and Clear, the
    /// picker opened from it, and the rejection under it.
    fn parent_edit(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let editable = self.controls_editable();
        let parent = self
            .session_meta()
            .and_then(Proposed::of)
            .and_then(|proposed| proposed.parent);
        let muted = cx.theme().muted_foreground;
        let chip = match &parent {
            Some(id) => self
                .relation_chip(id, ChipIds::parent(), cx)
                .into_any_element(),
            None => text("rel-parent-none", NONE)
                .text_xs()
                .text_color(muted)
                .into_any_element(),
        };
        let change = editable.then(|| {
            let focus = self.relations.change.clone();
            action("rel-parent-change", CHANGE, &focus, window, cx)
                .child(CHANGE)
                .child(self.sheet_scroll.marker(&focus))
                .on_click(cx.listener(|pane, _: &ClickEvent, window, cx| {
                    pane.open_picker(RelationField::Parent, window, cx)
                }))
        });
        let clear = (editable && parent.is_some()).then(|| {
            let focus = self.relations.clear.clone();
            action("rel-parent-clear", CLEAR, &focus, window, cx)
                .child(CLEAR)
                .child(self.sheet_scroll.marker(&focus))
                .on_click(
                    cx.listener(|pane, _: &ClickEvent, window, cx| pane.clear_parent(window, cx)),
                )
        });
        let picker = self.picker_view(RelationField::Parent, window, cx);
        let problem = self.relation_problem(RelationField::Parent, "rel-parent-problem", cx);
        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(chip)
                    .children(change)
                    .children(clear),
            )
            .children(picker)
            .children(problem)
            .into_any_element()
    }

    /// "Save checks again", under the relationships, while the answer that
    /// holds came from an index that is behind or incomplete.
    fn relations_note(&self, cx: &App) -> Option<AnyElement> {
        self.relations_partial().then(|| {
            text("rel-index-note", SAVE_CHECKS_AGAIN)
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .into_any_element()
        })
    }
}
