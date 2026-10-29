//! A ticket's details (design, Details sheet, Ticket): Title, Type and
//! Status, required; Project and Team, each a control while editing
//! (`fields`); then its relationships: Parent, Waiting on with the
//! readiness note, Dependents and Children, each D1's rows, and the
//! relationship problems (R-54); then Other fields, the Facts, the
//! Problems and the Inspect link.

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{ActiveTheme, v_flex},
    px,
};
use manyhands::repository::{ItemDto, ItemDtoKind, ReadinessState};

use super::{super::chips::waiting_on, GAP, Row, value};
use crate::desktop::{
    model::{
        edit::fields::Field,
        relationships::{RELATIONSHIPS, Relationships},
    },
    view::item::{ChipField, ItemPane, text},
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
        rows.extend(self.problems_section(item, cx));
        rows.push(self.inspect_link(item, cx));
        rows
    }

    /// Parent, Waiting on, Dependents and Children, each holding its group
    /// of D1's rows, then the relationship problems, under one name.
    fn relationship_rows(
        &mut self,
        item: &ItemDto,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let groups = self.relationship_groups(window, cx)?;
        let muted = cx.theme().muted_foreground;
        let note = readiness_note(item, self.model.relationships())
            .map(|note| text("sheet-readiness", note).text_xs().text_color(muted));
        let waiting = v_flex()
            .gap_1()
            .child(groups.dependencies)
            .children(note)
            .into_any_element();
        let parent_focus = self.sheet_focus[ChipField::Parent.index()].clone();
        let waiting_focus = self.sheet_focus[ChipField::WaitingOn.index()].clone();
        Some(
            v_flex()
                .id("relationships")
                .test_support()
                .aria_label(RELATIONSHIPS)
                .gap(px(GAP))
                .child(self.row(PARENT, Some(groups.parent), Some(&parent_focus), window, cx))
                .child(self.row(WAITING_ON, Some(waiting), Some(&waiting_focus), window, cx))
                .child(self.row(DEPENDENTS, Some(groups.dependents), None, window, cx))
                .child(self.row(CHILDREN, Some(groups.children), None, window, cx))
                .children(groups.problems)
                .into_any_element(),
        )
    }
}
